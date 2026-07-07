* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
*  console.asm — the rewritten TI-99/4A console ROM (CPU >0000..>1FFF)
*  =====================================================================
*  ORIGINAL WORK, assembled by this repo's own `libre99asm` (crate libre99-asm)
*  into `console-rom.bin`, executed by our emulator in place of TI's
*  `994aROM.Bin` — and booted BY DEFAULT since 2026-07-06. Phase 2 of the
*  system-ROM project; the interface spec is RECON.md (this folder), the
*  executed plan is archived at ../history/ROM-REWRITE-PLAN.md, and the
*  front door (status, test estate, layout ledger, house rules) is
*  README.md here. No Texas Instruments code appears here: every behaviour
*  is implemented from the dossier's specs and the oracle-probe
*  measurements (RECON §16), clean-room per plan P5.
*
*  STATUS: COMPLETE — M1-M5, M7, M8 (2026-07-05). The full non-BASIC GPL
*  interpreter (every opcode incl. MOVE/ALL/IO/COINC/SWGR/RTGR and indexed
*  GAS), the VBLANK ISR, KSCAN in all modes, XML dispatch + device linkage
*  (SROM/SGROM), FMT, the cassette modem layer, and the bit-exact radix-100
*  floating-point package — all differentially verified against the
*  authentic ROM. M6 (the TI BASIC half) is deferred indefinitely by
*  policy; its dispatch entries land on the loud STUB (breadcrumb >837D),
*  and a tripwire test enforces the deferral paperwork.
*
*  Register conventions on the GPL workspace >83E0 (RECON §3): R13=>9800
*  (GROM ports as offsets), R14=>0100 (SPEED/FLAGS), R15=>8C02 (VDP write-
*  address port). Interior register use (ours): R9 = current opcode (high
*  byte); R5 = opcode flag copy (>0100 word, >0200 immediate source);
*  R7/R8 = destination address/space; R3/R4 = source address/space;
*  R2 = destination value; R0 = source value; R1,R6 = scratch;
*  R10 = parked DIV divisor; R11 = BL linkage; R12 = second-level BL
*  parking inside helpers (free until the CRU milestones).
*
*  Values are byte-in-HIGH-byte for byte ops (MOVB-natural, low byte
*  zeroed) and full words for word ops, so the CPU's own byte/word
*  instructions produce exactly the status flags the GPL status byte
*  copies (RECON §16).

*  ---- vectors + fixed data/stubs (>0000-0023) ------------------------
        AORG >0000
        DATA >83E0                  * reset WP  = GPL workspace
        DATA START                  * reset PC  = >0024
        DATA >83C0                  * level-1 interrupt WP = INTWS
        DATA ISR                    * level-1 interrupt PC (minimal ISR; M2 adds duties)
        DATA >83C0                  * level-2 vector (unreachable; present)
        DATA ISTUB
        DATA >30AA                  * >000C: clock byte + the >AA header marker
        B    @KSCAN                 * >000E: KSCAN public entry
        DATA >0008                  * >0012: fixed data word (harvested)
        SBZ  0                      * >0014: prologue of the >0016 entry
        B    @R9ENT                 * >0016: enter interpreter, opcode in R9
        SBZ  0                      * >001A: prologue of the >001C entry
        B    @FETCH                 * >001C: enter interpreter, fetch opcode
        B    @CLEARH                * >0020: CLEAR/BREAK (FCTN-4) test

*  ---- reset / power-up (>0024) ---------------------------------------
*  RECON §1: the vector already set WP=>83E0. Establish the GPLWS port
*  registers, strobe the GROM once, set the boot address >0020, clear the
*  condition bit, enter the loop. The word at >0032 (the immediate of the
*  LI R0 below) doubles as the ISR's cassette-flag COC mask — a NASTY
*  harvested constant pinned to this exact address (RECON §13).
        AORG >0024
START   LI   R13,>9800              * GROM read-port base
        LI   R14,>0100              * SPEED=1 / FLAGS=0
        LI   R15,>8C02              * VDP write-address port
        LI   R0,>0020               * boot GROM address (word @>0032 = >0020)
        JMP  KERN2                  * continue at >005C (fixed data between)

*  ---- vestigial + fixed words (>0036-004D) ---------------------------
*  The extended-GPL-card return stub and the XOP vectors: never-shipped
*  hardware, reproduced behaviour-faithfully (plan §0; RECON §2).
        AORG >0036
        JMP  EXTRET
EXTRET  SBZ  0
        LWPI >280A
        RTWP
        AORG >0040
        DATA >280A,EXTGX0           * XOP 0 vector -> the ext-GPL trampoline (>0C1C)
        DATA >FFD8,>FFF8            * XOP 1 vector (user-defined)
        DATA >83A0,>8300            * XOP 2 vector (user-defined)
        DATA >1100                  * >004C: the ISR's QUIT row mask (NASTY)

*  ---- kernel continuation (>005C-006F) --------------------------------
        AORG >005C
KERN2   MOVB *R13,R4                * dummy GROM strobe (settle the prefetch)
        MOV  R0,R1
        MOVB R1,@>0402(R13)         * GROM address, high byte (>00)
        MOVB @>83E3,@>0402(R13)     * low byte via R1's workspace alias (>20)

*  ---- >006A: the public soft entry — clear the condition bit, run -----
SOFT    SZCB @MASK20,@>837C         * (mask byte pinned at >011B)

*  ---- GPL interpreter main loop (>0070) -------------------------------
*  Public geometry (RECON §3): >0070 loop with the one interrupt window
*  per instruction; >0078 the fetch; >007A the opcode-in-R9 entry.
        AORG >0070
LOOP    LIMI >0002
        LIMI >0000
FETCH   MOVB *R13,R9                * fetch the opcode byte from GROM
R9ENT   JLT  HI80                   * >=>80 -> the two-operand path
        MOVB R9,R4
        SRL  R4,12                  * top nibble 0..7
        MOV  @NIBTAB(R4),R5         * word access pairs the nibbles
        B    *R5

*  ---- the >=>80 driver -------------------------------------------------
*  Every >=>80 opcode starts with a destination GAS: parse it, load its
*  value, then dispatch — directly for single-operand ops, or after
*  fetching/loading the source for the two-operand families.
HI80    MOV  R9,R5                  * flag copy: >0100 word, >0200 imm src
        BL   @OPGET                 * -> R3 addr, R4 space
        MOV  R3,R7                  * destination kept in R7/R8
        MOV  R4,R8
        BL   @LDR0                  * (R3/R4 still name the dest)
        MOV  R0,R2                  * R2 := destination value
        MOV  R9,R0
        SRL  R0,8
        CI   R0,>00A0
        JHE  TWOOP
        MOV  R0,R1
        ANDI R1,>001E               * single-op table offset = op & >1E
        MOV  @TAB7E(R1),R1
        B    *R1
TWOOP   COC  @UBIT,R5               * the uniform source discipline (RECON §25):
        JNE  TWOMEM                 * imm bit >0200 set -> inline; clear -> GAS.
        BL   @IMMSRC                * immediate source -> R0
        JMP  TWOGO
TWOMEM  BL   @OPGET                 * source GAS -> R3/R4
        BL   @LDR0                  * source value -> R0
TWOGO   MOV  R9,R1
        SRL  R1,8
        AI   R1,->00A0
        ANDI R1,>00FC               * two-op offset = 32 + (op->A0 & >FC)/2
        SRL  R1,1
        AI   R1,32
        MOV  @TAB7E(R1),R1
        B    *R1

*  ---- the NASTY status mask (>011B) -----------------------------------
        AORG >011B
MASK20  BYTE >20

*  ============================================================================
*  Zone A (>0120-026F): control flow + specials
*  ============================================================================
        AORG >0120

*  BR (>40-5F) / BS (>60-7F): 13-bit slot-absolute branches on the
*  condition bit clear / set. Both consume (reset) the bit, taken or not
*  (oracle, RECON §16); the operand byte is consumed either way.
BRH     MOVB *R13,R2                * target low byte
        MOVB @>837C,R6
        COC  @C2000,R6
        JEQ  BSKIP                  * BR: condition set -> fall through
        JMP  BTAKE
BSH     MOVB *R13,R2
        MOVB @>837C,R6
        COC  @C2000,R6
        JEQ  BTAKE                  * BS: condition set -> take
BSKIP   SZCB @MASK20,@>837C
        B    @LOOP
BTAKE   SZCB @MASK20,@>837C
        BL   @GPCRD                 * R6 := counter (= next byte + 1)
        DEC  R6
        ANDI R6,>E000               * the current 8 KiB slot
        MOV  R9,R0
        SRL  R0,8
        ANDI R0,>001F
        SLA  R0,8
        SOC  R0,R6                  * | opcode low 5 bits -> bits 12..8
        SRL  R2,8
        SOC  R2,R6                  * | the operand byte
        BL   @GSETA
        B    @LOOP

*  B (>05): 16-bit absolute GROM target; resets the condition bit.
BH      BL   @ADDR16
        SZCB @MASK20,@>837C
        BL   @GSETA
        B    @LOOP

*  RAND (>02): the authentic LCG (oracle, RECON §16): seed' = seed * >6FE5
*  + >7AB9 (word at >83C0); >8378 := byteswap(seed') mod (limit + 1).
RANDH   LI   R1,>6FE5
        MPY  @>83C0,R1              * R1:R2 = seed * >6FE5
        AI   R2,>7AB9
        MOV  R2,@>83C0
        MOVB *R13,R6                * the limit byte
        SRL  R6,8
        INC  R6
        CLR  R1
        SWPB R2
        DIV  R6,R1                  * remainder -> R2
        SWPB R2
        MOVB R2,@>8378
        B    @LOOP

*  H / GT / CARRY / OVF: condition := the named status bit (RECON §16:
*  H=>80, GT=>40, CARRY=>10, OVF=>08).
HH      LI   R1,>8000
        JMP  STTST
GTH     LI   R1,>4000
        JMP  STTST
CARRYH  LI   R1,>1000
        JMP  STTST
OVFH    LI   R1,>0800
STTST   MOVB @>837C,R6
        COC  R1,R6
        JEQ  STT1
        SZCB @MASK20,@>837C
        B    @LOOP
STT1    SOCB @MASK20,@>837C
        B    @LOOP

*  CASE (>8A/8B): GROM PC += 2 * value; resets the condition bit.
CASEH   BL   @NORM2
        SZCB @MASK20,@>837C
        BL   @GPCRD
        DEC  R6
        A    R2,R6
        A    R2,R6
        BL   @GSETA
        B    @LOOP

*  PUSH (>8C/8D): pre-increment the data-stack byte pointer (>8372), then
*  store one byte there — the value (>8D: the word's low byte); oracle.
PUSHH   BL   @NORM2
        SWPB R2                     * payload byte -> high
        MOVB @>8372,R1
        SRL  R1,8
        INC  R1
        MOVB R2,@>8300(R1)
        SWPB R1
        MOVB R1,@>8372
        B    @LOOP

*  CZ (>8E/8F): a full compare-to-zero copy: H/GT from the value, the
*  condition bit = (value == 0), C/OV cleared (oracle: CZ >5A -> >C0).
CZH     COC  @W1,R5
        JEQ  CZW
        MOVB R2,R2
        STST R6
        ANDI R6,>E400               * byte: H/GT/cond + natural parity
        MOVB R6,@>837C
        B    @LOOP
CZW     MOV  R2,R2
        STST R6
        ANDI R6,>E000
        ORI  R6,>0400               * word: >04 always (oracle: DCZ -> C4)
        MOVB R6,@>837C
        B    @LOOP

*  ============================================================================
*  The VBLANK ISR (>0900, pinned home) — M2. Duty order (RECON §6), each VDP
*  duty gated by a >83C2 bit (>80 all / >40 sprite / >20 sound / >10 QUIT):
*  cassette-timer fork (hardware-gated), VDP-vs-card source, sprite auto-motion,
*  the sound list, QUIT, the VDP status read (clears the interrupt), the
*  screen-timeout blank (>83D6/>83D4), the SPEED timer (>8379), and the >83C4
*  user hook. Runs on GPLWS >83E0 for the duties and INTWS >83C0 for the RTWP
*  frame + the timeout counter (INTWS R11=>83D6, R10=>83D4).
*
*  Implemented here: the full control structure + gates, QUIT, status read, the
*  timeout blank, the SPEED timer, the user hook, and CLR R8. **Sprite
*  auto-motion (the >0780 velocity math) and the sound-list processing (the boot
*  beep) are gated-off scoped follow-ups** — they touch only the SAT/PSG and
*  sound cells, never the interrupt-ack / timer / timeout the boot and idle
*  depend on. Cassette + the non-VDP card scan are unreachable on a bare console
*  (all interrupts are VDP) and acknowledge-only.
*  ============================================================================
        AORG >0900
ISR     LIMI >0000
        LWPI >83E0                  * run on the GPL workspace
        CLR  R12
        COC  @>0032,R14             * FLAGS >20 = cassette-timer mode ->
        JNE  ISRTB                  *   the cassette timer ISR (Zone K; the
        B    @CASTIM                *   authentic >0910 fork to >1404)
ISRTB   TB   2                      * did the VDP raise the interrupt?
        JNE  ISRVDP
        B    @ISRACK                * non-VDP (no cards modelled) -> acknowledge
ISRVDP  MOVB @>83C2,R1              * the ISR disable bits (MSB first)
        SLA  R1,1                   * >80: skip ALL VDP duties?
        JNC  ISRSPR                 *   (ISRSTAT now far -> absolute branch)
        B    @ISRSTAT
ISRSPR  SLA  R1,1                   * >40: sprite auto-motion enabled?
        JOC  ISRNSP                 *   disabled -> skip
*  --- sprite auto-motion (authentic >095C) ---
*  For each of the >837A auto-motion sprites, integrate its velocity. The motion
*  table (SMT) at VDP >0780 holds [Yvel, Xvel, Yacc, Xacc] per sprite; add the
*  signed (/16) velocity into the sub-pixel accumulator, add that into the SAT
*  Y/X position at VDP >0300, apply the vertical screen-edge wrap, then write both
*  the SAT and the SMT accumulators back. R8 walks the VDP addresses; >83F1 is the
*  GPLWS R8-low alias, used to write the address LSB-first without a SWPB.
        MOVB @>837A,R12             * R12 = the auto-motion sprite count
        JEQ  ISRNSP                 *   none -> skip
        SRL  R12,8                  * count as 0..255
        LI   R2,>8800               * VDP read-data port
        LI   R3,>8C00               * VDP write-data port
        LI   R8,>0780               * R8 = the SMT (motion table) base
SPRLP   MOVB @>83F1,*R15            * set VDP read addr = R8 (LSB first; >83F1 = R8-low)
        MOVB R8,*R15
        CLR  R4
        MOVB *R2,R4                 * Yvel
        CLR  R6
        MOVB *R2,R6                 * Xvel
        SRA  R4,4                   * Yvel, signed /16
        MOVB *R2,R5                 * Yacc
        SRA  R5,4                   * Yacc, signed /16
        A    R4,R5                  * R5 = Yacc + Yvel (the new Y accumulator)
        MOVB *R2,R7                 * Xacc
        SRA  R6,4                   * Xvel, signed /16
        SRA  R7,4                   * Xacc, signed /16
        A    R6,R7                  * R7 = Xacc + Xvel (the new X accumulator)
        AI   R8,>FB80               * R8 -> the SAT (>0780 - >0480 = >0300)
        MOVB @>83F1,*R15            * set VDP read addr = SAT Y
        MOVB R8,*R15
        CLR  R4
        MOVB *R2,R4                 * current SAT Y
        A    R5,R4                  * new Y = current Y + Y accumulator
        CI   R4,>C0FF               * vertical edge wrap only for Y in (>C0FF, >E000]
        JLE  SPRX
        CI   R4,>E000
        JH   SPRX
        MOV  R5,R5                  * moving up (accumulator <= 0)?
        JGT  SPRWR
        AI   R4,>C000               *   bias before the wrap
SPRWR   AI   R4,>2000               * wrap Y by +>2000
SPRX    CLR  R6
        MOVB *R2,R6                 * current SAT X (port auto-advanced Y -> X)
        A    R7,R6                  * new X = current X + X accumulator
        ORI  R8,>4000               * set the VDP write flag -> write the SAT
        MOVB @>83F1,*R15            * set VDP write addr = SAT Y
        MOVB R8,*R15
        MOVB R4,*R3                 * write the new Y
        AI   R8,>0482               * R8 -> the SMT accumulator words (write, >4782)
        MOVB R6,*R3                 * write the new X (port auto-advanced)
        SWPB R5                     * the Y accumulator's fraction to the high byte
        MOVB @>83F1,*R15            * set VDP write addr = SMT Yacc
        MOVB R8,*R15
        SRL  R5,4
        MOVB R5,*R3                 * write back the updated Yacc
        SWPB R7                     * the X accumulator's fraction to the high byte
        SRL  R7,4
        MOVB R7,*R3                 * write back the updated Xacc (port auto-advanced)
        AI   R8,>C002               * R8 -> the next sprite's SMT entry (+4, write flag off)
        DEC  R12
        JGT  SPRLP
ISRNSP  SLA  R1,1                   * >20: sound list enabled?
        JOC  ISRNSD                 *   disabled -> skip
*  --- sound-list processing (authentic >09EC; list format in ../RECON.md) ---
*  Countdown >83CE -= SPEED; on reaching zero, emit the next block — its N bytes
*  stream from the >83CC/D list (GROM, or VDP per FLAGS >01) to the sound chip
*  >8400 — then reload the countdown with the block's duration D. Control blocks:
*  N=0 reloads the pointer (a jump), N=>FF toggles the source then jumps, D=0
*  ends the list (its bytes still play). The pointer advances arithmetically
*  (R3 += N+2), so no GROM address-readback (hence no prefetch) is involved.
        MOVB @>83CE,R2              * the frame countdown
        JEQ  ISRNSD                 *   0 -> no sound active
        SB   R14,@>83CE             * countdown -= SPEED (R14 high byte)
        JNE  ISRNSD                 *   still counting down this block
        MOV  @>83CC,R3              * R3 = the list pointer
        MOV  R14,R5
        SRL  R5,1                   * FLAGS bit 0 (source select) -> carry
        JOC  SNDVDP
        BL   @GPUSH                 * GROM source: save the interpreter's GROM position
        LI   R5,>0402
        A    R13,R5                 * R5 = >9C02, the GROM address-write port
        MOVB R3,*R5                 * pointer high
        MOVB @>83E7,*R5             * pointer low (>83E7 = GPLWS R3-low alias)
        MOV  R13,R6                 * R6 = >9800, the GROM read-data port
        JMP  SNDRD
SNDVDP  LI   R5,>8C02               * VDP source: R5 = the address-write port
        MOVB @>83E7,*R5             * low byte first (VDP is LSB-then-MSB); >83E7 = R3-low
        MOVB R3,*R5                 * then the high byte
        LI   R6,>8800               * R6 = >8800, the VDP read-data port
SNDRD   MOVB *R6,R8                 * R8 high = N, the block's byte count
        JEQ  SNDNEW                 * N=0 -> reload the pointer (a jump)
        CB   @SNDESC,R8             * N=>FF -> switch source then jump
        JEQ  SNDSW
        SRL  R8,8                   * N as a 0..255 count
        A    R8,R3                  * advance the pointer past the N data bytes
SNDBYT  MOVB *R6,@>8400             * stream a byte to the sound chip
        DEC  R8
        JNE  SNDBYT
        INCT R3                     * advance past the count byte + the duration byte
        MOVB *R6,R2                 * R2 high = D, the block duration
        JEQ  SNDEND                 * D=0 -> the list ends after this block
        JMP  SNDSTOR
SNDSW   XOR  @SNDTOG,R14            * flip the FLAGS source bit, then reload the pointer
SNDNEW  MOVB *R6,R3                 * new pointer high
        LI   R2,>0100               * duration 1 -> process the new block next tick
        MOVB *R6,@>83E7             * new pointer low (>83E7 = R3-low alias)
        JMP  SNDSTOR
SNDEND  SB   R2,R2                  * R2 := 0 -> countdown 0, the list is now inactive
SNDSTOR MOV  R3,@>83CC              * store the advanced pointer
        MOVB R2,@>83CE              * store the reloaded countdown
        CI   R5,>8C02               * VDP path? (R5 still holds the port used)
        JEQ  ISRNSD                 *   yes -> no GROM position to restore
        BL   @GPOP                  * GROM path -> restore the interpreter's GROM position
ISRNSD  SLA  R1,1                   * >10: QUIT (FCTN + =)?
        JOC  ISRSTAT
        LI   R12,>0024              * keyboard column-select CRU base
        LDCR @>0012,3               * select column 0 (@>0012 = >00, NASTY)
        SRC  R12,7                  * settle
        LI   R12,>0006              * row-read CRU base
        STCR R5,8                   * read the 8 rows (active low)
        CZC  @>004C,R5              * FCTN + = down? (mask >1100, NASTY)
        JNE  ISRSTAT
        BLWP @>0000                 * QUIT -> soft reset
ISRSTAT MOVB @>8802,@>837B          * VDP status read -> clears the interrupt
        LWPI >83C0                  * INTWS (holds the timeout counter + frame)
        INCT R11                    * screen-timeout >83D6 += 2
        JNE  ISRTMR
*  timeout wrapped -> blank: rebuild VDP R1 from the >83D4 copy (INTWS R10) with
*  the display-enable bit (>40) cleared, and write register 1.
        MOVB R10,R12
        SRL  R12,8
        ORI  R12,>8160              * register-1 selector (>81) + bits
        ANDI R12,>FFBF              * clear the display-enable bit
        MOVB @>83D9,@>8C02          * R12 low (the R1 value) -> VDP
        MOVB R12,@>8C02             * R12 high (the >81 selector) -> VDP
ISRTMR  LWPI >83E0
        AB   R14,@>8379             * SPEED timer: >8379 += SPEED (R14 high)
        MOV  @>83C4,R12             * user hook
        JEQ  ISRDONE
        BL   *R12                   * runs on GPLWS, returns B *R11
ISRDONE CLR  R8                     * GPLWS R8 is zeroed every tick
        LWPI >83C0
        RTWP
ISRACK  MOVB @>8802,@>837B          * acknowledge (clear any VDP status) + return
        CLR  R8
        LWPI >83C0
        RTWP

*  ============================================================================
*  Zone B: the specials dispatch (>0270, pinned) + BACK (>029E, pinned)
*  ============================================================================
        AORG >0270
SPEC    MOV  R9,R4                  * isolate the opcode bits from R9's high byte;
        ANDI R4,>1F00               * R9's low byte is scratch (e.g. MOVE parks a
        SRL  R4,7                   * storer address there) -> (opcode & >1F) * 2
        MOV  @SPCTAB(R4),R4
        B    *R4

        AORG >029E
BACK    LI   R7,>8700               * VDP register-7 selector
        MOVB *R13,@>83EF            * operand -> the helper's value cell
        BL   @VDPRL
        B    @LOOP

*  SCAN opcode shim (>02AE) — return to the interpreter loop, then fall into the
*  pinned KSCAN entry. KSCAN (>02B2, pinned) trampolines to its body (Zone C
*  occupies the authentic >0300+ interior, so the body lives in free space; the
*  public entry address is preserved — P8 escape hatch).
        AORG >02AE
SCANH   LI   R11,>0070              * SCAN's "return" is the interpreter fetch
KSCAN   B    @KSCANB

*  ============================================================================
*  Zone C (>0300-04B1): format-1 handlers
*  ============================================================================
        AORG >0300

*  ST: dest := src. No status effect.
STH     MOV  R0,R2
        BL   @STOD
        B    @LOOP

*  EX: swap dest and source (memory source). No status effect. The immediate
*  forms are the authentic accident (§25): the immediate stores to the dest
*  normally, and the old dest value goes to the imm path's leftover pointer —
*  the speech-write region (>97FF/>9800), inert on this bus — so the
*  observable effect is dest := immediate.
EXH     COC  @UBIT,R5
        JNE  EXMEM
        MOV  R0,R2                  * immediate form: dest := the immediate...
        BL   @STOD
        B    @LOOP                  * ...the second store is inert (§25)
EXMEM   MOV  R2,R9                  * park the old dest in R9 (the authentic
        MOV  R0,R2                  * >01A2 park — STOD's >837D echo path
        BL   @STOD                  * clobbers R6); dest := source value
        MOV  R9,R2
        MOV  R3,R7                  * the source cell becomes the target
        MOV  R4,R8
        BL   @STOD                  * source := old dest value
        B    @LOOP

*  ADD/SUB (+ the INC family, below): real CPU arithmetic so the status
*  copy is exact — >837C := L>|A>|EQ|C|OV, plus >04 always for word ops
*  and the natural odd-parity bit for byte ops (oracle, RECON §16).
*  SUB is the authentic NEG-then-ADD (>0186 falls into >0188, §25): the
*  carry/overflow are an ADD's, which differs from a subtract's borrow
*  convention exactly at source 0 (C) and >8000 (OV) — the deep fuzz
*  caught the SB-based version.
SUBH    NEG  R0                     * SUB = ADD of the negation (authentic)
ADDH    COC  @W1,R5
        JEQ  ADDW
        AB   R0,R2
        JMP  STARIB
ADDW    A    R0,R2
STARIW  STST R6
        ANDI R6,>F800
        ORI  R6,>0400
        JMP  STARI2
STARIB  STST R6
        ANDI R6,>FC00
STARI2  MOVB R6,@>837C
        BL   @STOD
        B    @LOOP

*  AND/OR/XOR: result flags (L>/A>/EQ + byte parity); C/OV preserved from
*  the previous >837C; word logic leaves >04 clear (oracle).
ANDH    COC  @W1,R5
        JEQ  ANDW
        INV  R0
        SZCB R0,R2
        JMP  STLOGB
ANDW    INV  R0
        SZC  R0,R2
        JMP  STLOGW
ORH     COC  @W1,R5
        JEQ  ORW
        SOCB R0,R2
        JMP  STLOGB
ORW     SOC  R0,R2
        JMP  STLOGW
XORH    COC  @W1,R5
        JEQ  XORW
        ANDI R0,>FF00
        XOR  R0,R2
        MOVB R2,R2                  * byte flags + parity from the result
        JMP  STLOGB
XORW    XOR  R0,R2
STLOGW  STST R6
        ANDI R6,>E000
        ORI  R6,>1000               * word logic: C reads as set (oracle:
        JMP  STLOG2                 * DOR 0|0 -> >10; DAND after carry -> >90)
STLOGB  STST R6
        ANDI R6,>E400               * byte logic: C reads as clear (oracle)
STLOG2  MOVB R6,@>837C
        BL   @STOD
        B    @LOOP

*  MUL (>A8-AB): the authentic model (§25). Word: 32-bit product, high word
*  at D, low at D+2. Byte: the DEST byte is cleaned unsigned but the SOURCE
*  keeps its right-justified SIGN EXTENSION (the >07AA load discipline), so
*  the 16x16 product can exceed 16 bits — its LOW word stores at D,D+1
*  (e.g. >09*>96 -> >0009*>FF96 = >0008FC46 -> D=>FC, D+1=>46). No status.
MULH    COC  @W1,R5
        JEQ  MULGO                  * word: operands as-is
        SRA  R0,8                   * byte: source right-justified sign-extended
        SRL  R2,8                   * dest cleaned unsigned (authentic SB)
MULGO   MOV  R2,R1
        MPY  R0,R1                  * R1:R2 = the 32-bit product
        COC  @W1,R5
        JEQ  MULW
        BL   @STW2                  * byte: the product's LOW word at D,D+1
        B    @LOOP
MULW    MOV  R2,R0
        MOV  R1,R2
        BL   @STW2                  * high word at D
        INCT R7
        MOV  R0,R2
        BL   @STW2                  * low word at D+2
        B    @LOOP

*  DIV (>AC-AF): the authentic model (§25). >837C is PRESET wholesale to >01
*  (word) / >00 (byte) — DIV wipes the prior status. Overflow — the 9900's own
*  JNO condition, divide-by-zero included — ORs >08 (via the harvested >0013
*  byte, NASTY §13) and the UNCHANGED dividend halves still store back.
*  Byte: dividend = sext(D-byte)::(D-byte:(D+1)-byte), divisor sext(src);
*  q byte -> D, r byte -> D+1. Word: dividend = D-word::(D+2)-word, q -> D,
*  r -> D+2.
DIVH    COC  @W1,R5
        JEQ  DIVPW
        MOVB @CZERO,@>837C          * byte form: the wholesale preset >00
        SRA  R0,8                   * byte divisor: right-justified sign-extended
        MOV  R0,R6
        BL   @LDW2                  * dividend low word := the D:(D+1) bytes
        MOV  R2,R1
        SRA  R1,15                  * dividend high word := the dest byte's SIGN
        DIV  R6,R1                  * (R1:R2)/R6 -> q R1, r R2 (unchanged on ovf)
        JNO  DIVBS
        SOCB @>0013,@>837C          * overflow/zero: OR >08 (the NASTY harvest)
DIVBS   SWPB R1                     * build [q low byte : r low byte]...
        ANDI R1,>FF00
        ANDI R2,>00FF
        SOC  R2,R1
        MOV  R1,R2
        BL   @STW2                  * ...stored at D, D+1
        B    @LOOP
DIVPW   MOVB @W1,@>837C             * word form: the preset >01 (W1's high byte)
        MOV  R0,R10                 * park the divisor (LDR0W clobbers R0/R1)
        MOV  R7,R3
        INCT R3
        MOV  R8,R4
        BL   @LDR0W                 * the low word -> R0
        MOV  R2,R1                  * R1:R2 := the 32-bit dividend
        MOV  R0,R2
        MOV  R10,R6
        DIV  R6,R1                  * q -> R1, r -> R2 (unchanged on overflow)
        JNO  DIVWS
        SOCB @>0013,@>837C          * overflow/zero: OR >08 (-> >09 on word)
DIVWS   MOV  R2,R0
        MOV  R1,R2
        BL   @STW2                  * quotient (or the unchanged high) at D
        INCT R7
        MOV  R0,R2
        BL   @STW2                  * remainder (or the unchanged low) at D+2
        B    @LOOP

*  Compares: condition := (dest OP src). Only the condition bit changes —
*  the authentic tails SOCB/SZCB the >20 bit and PRESERVE the rest of >837C
*  (§25: an earlier op's H/GT/C/OV survive a compare; the M2 fuzz overturned
*  the old wholesale-replace reading once MUL/DIV/EX joined the pool).
CHH     BL   @CMP
        JH   CSET
        JMP  CCLR
CHEH    BL   @CMP
        JHE  CSET
        JMP  CCLR
CGTH    BL   @CMP
        JGT  CSET
        JMP  CCLR
CGEH    BL   @CMP
        JGT  CSET
        JEQ  CSET
        JMP  CCLR
*  (CEQ lives in Zone J — its authentic tail is the CZ raw-STST wholesale,
*  not the jump-family SOCB/SZCB; Zone C has no room for it.)
CLOGH   COC  @W1,R5
        JEQ  CLOGW
        ANDI R0,>FF00
        ANDI R2,>FF00
CLOGW   CZC  R0,R2                  * EQ := (src AND dest) == 0
        JEQ  CSET
CCLR    SZCB @MASK20,@>837C
        B    @LOOP
CSET    SOCB @MASK20,@>837C
        B    @LOOP

*  ============================================================================
*  CLEAR / BREAK test (>04B2, pinned) — behind the >0020 public entry. FCTN-4
*  (CLEAR) is a two-key chord: column 0 row 4 (mask >1000) AND column 3 row 4.
*  Probe column 0 first (via the NASTY selectors @>0012=>00 / @>0074=>03 and the
*  row mask @>0036=>1000); return EQ iff both are held. Called `BL @>0020` on
*  WP=>83E0; returns B *R11.
*  ============================================================================
        AORG >04B2
CLEARH  LI   R12,>0024              * keyboard column-select CRU base
        LDCR @>0012,3               * select column 0 (@>0012 = >00)
        SRC  R12,7                  * settle
        LI   R12,>0006              * row-read CRU base
        STCR R12,8                  * read the 8 rows (active low)
        CZC  @>0036,R12             * row 4 (mask >1000) down in column 0?
        JNE  CLEARR                 * no -> not CLEAR (return NE)
        LI   R12,>0024
        LDCR @>0074,3               * select column 3 (@>0074 = >03)
        SRC  R12,7
        LI   R12,>0006
        STCR R12,8
        CZC  @>0036,R12             * row 4 down in column 3?
CLEARR  B    *R11                   * EQ iff FCTN-4 (CLEAR) held

*  ---- FMT entry (>04DE, pinned — P8: literature-documented; XB plausibly
*  enters). The body is free-placed (Zone I), so the pinned entry trampolines
*  to it; the >0CDC dispatch table carries our handler addresses (P8 scoping).
        AORG >04DE
FMT     B    @FMTBODY

*  ============================================================================
*  Zone D (>0500-05A1): single-operand + INC-family handlers
*  ============================================================================
        AORG >0500

ABSH    MOV  R2,R2
        JLT  ABSN
        JMP  ABSST
ABSN    NEG  R2
ABSST   BL   @STOD
        B    @LOOP
NEGH    NEG  R2
        BL   @STOD
        B    @LOOP
INVH    INV  R2
        BL   @STOD
        B    @LOOP
CLRH    CLR  R2
        BL   @STOD
        B    @LOOP

*  INC/DEC/INCT/DECT: implied-operand arithmetic; ADD/SUB status rules.
INCH    LI   R0,>0100
        JMP  IARITH
DECH    LI   R0,>0100
        JMP  IARITS
INCTH   LI   R0,>0200
        JMP  IARITH
DECTH   LI   R0,>0200
        JMP  IARITS
IARITH  COC  @W1,R5
        JEQ  IARW
        AB   R0,R2
        B    @STARIB
IARW    SRL  R0,8
        A    R0,R2
        B    @STARIW
IARITS  COC  @W1,R5
        JEQ  IASW
        SB   R0,R2
        B    @STARIB
IASW    SRL  R0,8
        S    R0,R2
        B    @STARIW

*  ALL (>07): fill the 768-cell name table (VDP >0000..>02FF, base hardcoded —
*  oracle-pinned: reg2 does not move it) with the immediate operand character.
*  No status effect. Authentic home >05A2.
        AORG >05A2
ALLH    MOVB *R13,R0                * the fill character (consume the operand)
        CLR  R7
        BL   @VWR                   * VDP write address := >0000
        LI   R2,>0300               * 768 cells
ALLLP   MOVB R0,@>8C00
        DEC  R2
        JNE  ALLLP
        B    @LOOP

*  ============================================================================
*  Zone E (>0680-0779): stream + small helpers (leaf routines)
*  ============================================================================
        AORG >0680

*  GPCRD: R6 := the GROM address counter (two destructive reads: high then
*  low). The value is (next unconsumed byte) + 1. Clobbers R1.
GPCRD   MOVB @>0002(R13),R6
        MOVB @>0002(R13),R1
        SRL  R1,8
        ANDI R6,>FF00
        SOC  R1,R6
        B    *R11

*  GSETA: write R6 to the GROM address port, high byte then low.
GSETA   MOVB R6,@>0402(R13)
        SWPB R6
        MOVB R6,@>0402(R13)
        SWPB R6
        B    *R11

*  ADDR16: R6 := a 16-bit immediate from the instruction stream.
ADDR16  CLR  R6
        MOVB *R13,R6
        MOVB *R13,R1
        SRL  R1,8
        SOC  R1,R6
        B    *R11

*  IMMSRC: R0 := the immediate source (byte high-justified, or a word).
IMMSRC  CLR  R0
        MOVB *R13,R0
        COC  @W1,R5
        JNE  IMMB
        MOVB *R13,R1
        SRL  R1,8
        SOC  R1,R0
IMMB    B    *R11

*  NORM2: right-justify a byte dest value (CASE/PUSH); words pass through.
NORM2   COC  @W1,R5
        JEQ  NORM2W
        SRL  R2,8
NORM2W  B    *R11

*  SHCNT: shift count -> R6. The 9900 shift-by-R0 form authentic uses (`SRL
*  R2,0`, RECON §1 shift block) takes the count from R0's low **nibble** (0-15;
*  0 -> 16) — the count is masked to >000F, not >001F. The fuzz caught the
*  wider mask (a count with bit 4 set, e.g. 17, shifted a word to 0 where
*  authentic shifts by 1).
SHCNT   MOV  R0,R6
        COC  @W1,R5
        JEQ  SHCW
        SRL  R6,8
SHCW    ANDI R6,>000F
        JNE  SHOK
        LI   R6,16
SHOK    B    *R11

*  CMP: width-correct compare of dest (R2) against source (R0).
CMP     COC  @W1,R5
        JEQ  CMPW
        CB   R2,R0
        B    *R11
CMPW    C    R2,R0
        B    *R11

*  ============================================================================
*  Zone F (>077A+): the GAS operand engine (pinned at its authentic home)
*  ============================================================================
        AORG >077A

*  OPGET: parse one GAS operand from the instruction stream (RECON §3:
*  short = >8300-based; long = `1 X V I nnnn` with a 12-bit value or, for
*  nnnn=15, a 16-bit extension; CPU space is >8300-biased, VDP unbiased;
*  indirect goes through a CPU cell — a byte pointer for CPU, a word
*  pointer for VDP). -> R3 = address, R4 = space (0 CPU, 1 VDP). The indexed
*  form (X bit) adds the >8300-indexed word to the base (OPGIDX, M4). Leaf;
*  clobbers R0, R1, R6.
OPGET   CLR  R4
        MOVB *R13,R1
        JLT  OPGL
        SRL  R1,8
        MOV  R1,R3
        AI   R3,>8300
        JMP  OPGD7                  * every CPU form takes the >837D check
OPGL    MOV  R1,R6                  * the flag bits
        ANDI R1,>0F00
        CI   R1,>0F00
        JEQ  OPGX
        MOV  R1,R3                  * 12-bit: nibble<<8 ...
        MOVB *R13,R1
        SRL  R1,8
        SOC  R1,R3                  * ... | the next byte
        JMP  OPGF
OPGX    CLR  R3
        MOVB *R13,R3                * 16-bit: two extension bytes
        MOVB *R13,R1
        SRL  R1,8
        SOC  R1,R3
OPGF    COC  @X4000,R6
        JNE  OPGNX                  * not indexed
        MOV  R11,R12                * park our return across the nested BL
        BL   @OPGIDX                * indexed: base += the >8300-indexed word
        MOV  R12,R11
OPGNX   COC  @V2000,R6
        JNE  OPGCPU
        LI   R4,1                   * VDP space
        COC  @I1000,R6
        JNE  OPGDON
        AI   R3,>8300               * indirect: the pointer cell is CPU
        MOV  R3,R1
        CLR  R3
        MOVB *R1+,R3                * word pointer -> the VDP address
        SWPB R3
        MOVB *R1,R3
        SWPB R3
        JMP  OPGDON
OPGCPU  AI   R3,>8300
        COC  @I1000,R6
        JNE  OPGD7
        CI   R3,>837C               * indirect through >837C is the data-stack
        JNE  OPGIND                 * POP quirk (§25): the "pointer" is >8372,
        MOVB @>8372,R1              * read then post-decremented (R14 high = 1)
        SB   R14,@>8372
        JMP  OPGPOP
OPGIND  MOVB *R3,R1                 * byte pointer, >8300-based
OPGPOP  SRL  R1,8
        MOV  R1,R3
        AI   R3,>8300
OPGD7   CI   R3,>837D               * >837D is the character buffer (§25):
        JNE  OPGDON                 * fetch the screen byte at the cursor into
        MOV  R11,R12                * the cell before the normal load; CHBRD
        B    @CHBRD                 * returns straight to our caller via R12
OPGDON  B    *R11

*  OPGIDX (M4): the indexed-GAS offset. The index selector is the next stream
*  byte; it names a >8300-based cell whose **word** value is added to the base
*  address R3 (authentic `>07D2`/`>077E`). Reused by MOVE's C=1 computed-GROM
*  source. Clobbers R0, R1 (R6 = the flag bits is preserved for the V/I tests).
OPGIDX  CLR  R0
        MOVB *R13,R0                * the index selector byte
        SRL  R0,8                   * -> the >8300-based cell offset
        MOV  R0,R1
        CLR  R0
        MOVB @>8300(R1),R0          * the index value: high byte
        MOVB @>8301(R1),R1          * ... low byte
        SRL  R1,8
        SOC  R1,R0                  * R0 = the 16-bit index value
        A    R0,R3                  * base += index
        B    *R11

*  ============================================================================
*  RTN / RTNC (>0838/>083E) + CALL (>085A) + the GROM push (>0864) — pinned
*  ============================================================================
        AORG >0838
RTN     SZCB @MASK20,@>837C         * RTN clears the condition bit; RTNC
RTNC    B    @RTNCI                 * (falling in here) does not

*  GPOP (>0842, pinned) — pop a saved GROM position off the GPL sub-stack and
*  re-write the GROM address port, the inverse of GPUSH (>0864). Used by KSCAN
*  (and RTGR) to restore the interpreter's fetch position after reading tables.
        AORG >0842
GPOP    MOVB @>8373,R4
        SRL  R4,8
        DECT @>8373
        MOVB @>8300(R4),@>0402(R13)
        MOVB @>8301(R4),@>0402(R13)
        B    *R11

        AORG >085A
CALLH2  B    @CALLI

*  GPUSH (>0864): push the current GROM position (counter - 1) onto the
*  subroutine stack: bump the pointer byte by 2 (the word-op-at->8372
*  alias quirk, exactly as the authentic ROM behaves — RECON §16), then
*  store the adjusted counter. Preserves R6; clobbers R0, R1.
        AORG >0864
GPUSH   INCT @>8373
        MOVB @>8373,R1
        SRL  R1,8
        MOVB @>0002(R13),R0         * counter high (destructive read)
        MOVB R0,@>8300(R1)
        MOVB @>0002(R13),R0         * counter low
        MOVB R0,@>8301(R1)
        DEC  @>8300(R1)             * stored word -= 1 -> the resume address
        B    *R11

*  ---- the VDP register-load helper (>089A, pinned) ---------------------
        AORG >089A
VDPRL   MOVB @>83EF,*R15            * the value byte, then >80|register
        MOVB R7,*R15
        B    *R11

*  ============================================================================
*  Zone G (>08A4-08FF): value loads
*  ============================================================================
        AORG >08A4

*  LDR0: R0 := the value at (R3 addr, R4 space); byte (high-justified,
*  low zero) or word per R5. VDP reads ride the auto-incrementing port.
*  Nested BLs park R11 in R12.
LDR0    MOV  R4,R4
        JNE  LDR0V
        COC  @W1,R5
        JEQ  LDR0W1
        CLR  R0
        MOVB *R3,R0
        B    *R11
LDR0W1  MOV  R3,R1
        MOVB *R1+,R0
        SWPB R0
        MOVB *R1,R0
        SWPB R0
        B    *R11
LDR0V   MOV  R11,R12
        BL   @VRD
        COC  @W1,R5
        JEQ  LDR0VW
        CLR  R0
        MOVB @>8800,R0
        B    *R12
LDR0VW  MOVB @>8800,R0
        MOVB @>8800,R1
        SRL  R1,8
        ANDI R0,>FF00
        SOC  R1,R0
        B    *R12

*  LDR0W: a word at (R3,R4) regardless of R5 (DIV's low word).
LDR0W   MOV  R4,R4
        JNE  LDR0WV
        MOV  R3,R1
        MOVB *R1+,R0
        SWPB R0
        MOVB *R1,R0
        SWPB R0
        B    *R11
LDR0WV  MOV  R11,R12
        BL   @VRD
        JMP  LDR0VW

*  ============================================================================
*  M5 — the radix-100 floating-point package (RECON §9/§27). FAC = >834A-8351,
*  ARG = >835C-8363, guard bytes >8352-8353 / >8364-8365; exponent byte biased
*  >40, seven 0-99 digits; negatives carry the first WORD negated; zero =
*  first word >0000. Error byte >8354, sign >8375, exponent scratch >8376,
*  the FP-error GROM address >836C, the VDP value stack pointer >836E. The
*  pinned entries pack exactly as the authentic's (the inter-pin gaps force
*  it); internal exits flow through the shared round/normalize tails in the
*  ROUND1 cluster. Every routine is XMLLNK'd: R11 = the return, parked in R10.
*  ============================================================================
        AORG >0D3A

*  FCOMP (XML >0A): compare ARG against FAC through the >0FAA status tail.
FCOMP   MOV  R11,R10
        LI   R3,FPTAA
        JMP  FCBODY
*  The flags-only compare entry (authentic >0D42): pops ARG, compares, and
*  returns the raw 9900 flags straight to the caller (no status write).
FCPOPF  MOV  R11,R3
        JMP  FCPOP
*  SCOMP (XML >0F): pop ARG from the value stack, then compare.
SCOMP   LI   R3,FPTAA
        MOV  R11,R10
FCPOP   BL   @FPPOP
*  The shared compare body. The first words carry the sign (negatives are
*  word-negated), so one SIGNED word compare orders any mixed-sign pair;
*  equal first words with both negative swap the cursors (bigger magnitude =
*  smaller value) before the three magnitude words compare unsigned-ish.
FCBODY  LI   R7,>835C
        LI   R5,>834A
        C    *R7,*R5+
        JNE  FCOUT
        MOV  *R7+,R6
        JEQ  FCOUT                  * both zero -> equal
        JGT  FCMAG                  * positive pair -> compare as-is
        MOV  R5,R6                  * negative pair -> swap the cursors
        MOV  R7,R5
        MOV  R6,R7
FCMAG   C    *R7+,*R5+
        JNE  FCOUT
        C    *R7+,*R5+
        JNE  FCOUT
        C    *R7,*R5
FCOUT   B    *R3

*  SSUB (XML >0C): pop ARG, re-materialize R11 (FSUB/FADD park it again),
*  fall into FSUB.
SSUB    MOV  R11,R10
        BL   @FPPOP
        MOV  R10,R11
*  FSUB (XML >07): ARG - FAC = ARG + (-FAC) — negate FAC's first word and
*  fall into FADD (the NEG-then-ADD house pattern, RECON §25).
FSUB    NEG  @>834A
*  FADD (XML >06): ARG + FAC -> FAC.
FADD    MOV  R11,R10
        JMP  FABODY
*  SADD (XML >0B): pop ARG, fall into the add.
SADD    MOV  R11,R10
        BL   @FPPOP

*  The add body. Zero shortcuts: ARG == 0 -> the result is FAC as it stands
*  (out through the >0FA6 round/store tail); FAC == 0 -> FAC := ARG first.
FABODY  MOV  @>835C,R7
        JEQ  FADONE
        MOV  @>834A,R8
        JNE  FAGO
        LI   R1,>FFF8               * FAC := ARG (four words, indexed -8..-2)
FACPY   MOV  @>8364(R1),@>8352(R1)
        INCT R1
        JLT  FACPY
FADONE  B    @FPTA6
*  Both non-zero: R7 := sign-difference (the XOR's sign bit), magnitudes
*  compare word-by-word and the BIGGER operand swaps into FAC; the result
*  sign (>8375) is the bigger operand's original first-word high byte (the
*  XOR chain reconstructs it on the swap path, exactly as the authentic).
FAGO    XOR  R8,R7
        ABS  @>834A
        ABS  @>835C
        LI   R3,>FFF8
FAMCMP  C    @>8352(R3),@>8364(R3)
        JGT  FAORD                  * FAC bigger -> keep
        JLT  FASWP                  * ARG bigger -> swap it into FAC
        INCT R3
        JNE  FAMCMP
        JMP  FAORD                  * equal magnitudes -> keep
FASWP   MOV  @>8364(R3),R0
        MOV  @>8352(R3),@>8364(R3)
        MOV  R0,@>8352(R3)
        INCT R3
        JNE  FASWP
        XOR  R7,R8                  * the result sign follows the swap
FAORD   CLR  R5
        CLR  @>8352                 * clear both guard words
        CLR  @>8364
        MOVB R8,@>8375              * the result sign
        CLR  R6
        MOVB @>834A,@>83ED          * R6 low := FAC's exponent byte
        MOV  R6,@>8376              * the exponent scratch (right-justified)
        MOVB R5,@>834A              * FAC's exponent position becomes digit 0
        SB   @>835C,@>83ED          * R6 := the exponent difference
        CI   R6,>0007
        JGT  FABIGX                 * too far apart: the bigger operand wins
        MOV  R6,R0
        LI   R8,>0100               * the base-100 carry (as a high byte)
        LI   R9,>6400               * one hundred (as a high byte)
        LI   R5,>8353               * FAC cursor: the guard byte, descending
        LI   R6,>8365               * ARG cursor...
        S    R0,R6                  * ...aligned down by the exponent diff
        MOV  R0,R4
        AI   R4,>FFF7               * loop count: 9 - diff bytes
        MOV  R7,R1
        JLT  FSLOOP                 * signs differ -> the subtract loop
*  Same signs: digit-wise add, base-100 carry rippling up.
FALOOP  AB   *R6,*R5
        CB   *R5,R9
        JL   FANOC
        SB   R9,*R5                 * >= 100: subtract the base...
        AB   R8,@>FFFF(R5)          * ...and carry into the next digit up
FANOC   DEC  R5
        DEC  R6
        INC  R4
        JLT  FALOOP
        JMP  FATOP
*  The top-position carry: reduce it mod 100, counting overflows downward.
FACAR   DEC  R5
        AB   R8,*R5
FATOP   SB   R9,*R5
        JGT  FACAR
        JEQ  FACAR
        AB   R9,*R5                 * went negative: restore
        MOVB @>834A,R1              * an overflow digit at the top?
        JEQ  FARND
        INC  @>8376                 * yes: exponent++ and shift the whole
        LI   R1,>8352               * mantissa (incl. guards) right one digit
        LI   R2,>0009
FASHR   MOVB *R1,@>0001(R1)
        DEC  R1
        DEC  R2
        JNE  FASHR
FARND   JMP  RND1B                  * finish in ROUND1's body (R10 = return)
*  Different signs: digit-wise subtract with base-100 borrow.
FSLOOP  SB   *R6,*R5
        JGT  FSNOB
        JEQ  FSNOB
        AB   R9,*R5                 * borrow: add the base back...
        SB   R8,@>FFFF(R5)          * ...and take one from the next digit up
FSNOB   DEC  R5
        DEC  R6
        INC  R4
        JLT  FSLOOP
        JMP  FSTOP
FSBOR   AB   R9,*R5                 * the top went negative: restore and
        DEC  R5                     * push the borrow further down
        SB   R8,*R5
FSTOP   MOVB *R5,R4
        JLT  FSBOR
        JMP  FPNRM                  * the shared normalize (leading zeros)
FABIGX  B    @FPBIG                 * diff > 7: the bigger operand, rounded

*  The FP package's shared numeric constants (the authentic harvests its own
*  instruction immediates — @>1044/>1045/>0E59; ours are named data, P8).
        AORG >1AC0
F100W   DATA >0064                  * one hundred, as a word
F100B   BYTE >64                    * one hundred, as a byte
F01B    BYTE >01                    * a byte one (the add-back carry)
*  (ten — the authentic harvests its own CI immediate at >117A; ours named)
F10W    DATA >000A

*  FPPOP: pop the VDP value-stack top into ARG — eight bytes read at the
*  >836E pointer, then the pointer steps down one entry (authentic >1FA8;
*  ours free-placed — only BL reaches it). Needs R15 = >8C02 (the GPL VDP
*  convention). Clobbers R5/R6/R7.
        AORG >1420
FPPOP   LI   R5,>FFF8
        LI   R6,>835C
        MOVB @>836F,*R15            * VDP read address := the pointer (LSB,
        LI   R7,>8800               * then the MSB — read mode)
        MOVB @>836E,*R15
        A    R5,@>836E              * the pointer pops down one 8-byte entry
FPPOPL  MOVB *R7,*R6+               * eight data-port reads -> ARG
        INC  R5
        JNE  FPPOPL
        B    *R11

*  ============================================================================
*  FMUL (XML >08) / SMUL (XML >0D): FAC := ARG * FAC — the radix-100
*  schoolbook multiply (RECON §27; the fp-recon-fmul dossier). The sign is
*  the first words' XOR; the working exponent W = eF + eA - >3F accumulates
*  as a word at >8376; the product digits build IN PLACE across FAC and the
*  guard extension (>834B-8358): each FAC-digit row (last digit first) is
*  consumed-and-zeroed so its slot catches the row's carry-out, and each
*  column term facdigit*argdigit + acc splits by DIV 100 into the stored
*  digit and a lazy byte carry one column left (bounded 198, never wraps).
*  Ends in the >0F18 error-word clear and the shared normalize.
*  ============================================================================
        AORG >0E88
FMUL    MOV  R11,R10
        JMP  FMBODY
SMUL    MOV  R11,R10
        BL   @FPPOP
FMBODY  LI   R3,>834A
        LI   R5,>835C
        MOV  *R3,R8                 * FAC == 0 -> a zero product
        JEQ  FPZERO
        XOR  *R5,R8                 * R8 bit 15 := the product sign
        ABS  *R5                    * ARG == 0 -> a zero product
        JEQ  FPZERO
        ABS  *R3
        CLR  R9
        MOVB *R3,R9                 * W := eF + eA - >3F
        AB   *R5,R9
        SWPB R9
        AI   R9,>FFC1
        MOV  R9,@>8376
        MOVB R8,@>8375              * the sign byte
        LI   R5,>8352               * clear the guard/extension words
FMCLR   CLR  *R5+
        CI   R5,>835A
        JNE  FMCLR
        LI   R5,>8352               * R5 -> the last nonzero FAC byte
FMFSCN  DEC  R5
        MOVB *R5,R0
        JEQ  FMFSCN
        LI   R7,>0008               * R7 := the last nonzero ARG index
FMASCN  DEC  R7
        MOVB @>835C(R7),R0
        JEQ  FMASCN
        CLR  R0
        MPY  R0,R2                  * one-instruction clear of R2 AND R3
        MOV  R5,R6
        LI   R8,>83E1               * R8 -> R0's low byte (the WP alias)
        LI   R9,>0064               * the radix
FMROW   MOV  R7,R4                  * a := amax
        A    R7,R6                  * R6 -> this row's units column
        MOVB *R5,@>83E7             * R3 := the FAC digit (via R3-low)...
        MOVB R3,*R5                 * ...its slot zeroed for the carry-out
FMCOL   MOVB @>835C(R4),*R8         * R0 := the ARG digit (via R0-low)
        MPY  R3,R0                  * R0:R1 := the digit product
        MOVB *R6,@>83E5             * R2 := the accumulator byte (R2-low)
        A    R2,R1                  * + acc
        DIV  R9,R0                  * R0 = the carry, R1 = the digit
        MOVB @>83E3,*R6             * store the digit (R1's low byte)
        DEC  R6
        AB   *R8,*R6                * the lazy carry, one column left
        DEC  R4
        JGT  FMCOL
        DEC  R6
        DEC  R5
        CI   R5,>834A
        JGT  FMROW
*  (the error-word clear — FDIV enters here; it also discards raw P10/P11)
FPNR18  CLR  @>8354

*  ============================================================================
*  FDIV (XML >09) / SDIV (XML >0E): FAC := ARG / FAC — Knuth's Algorithm D in
*  radix 100 (RECON §27; the fp-recon-round-fdiv dossier). The divisor (FAC)
*  relocates to >8354-835B and the dividend grows a zero spill digit plus an
*  8-byte zero extension (>8364-836B); when the divisor's leading digit is
*  under 50 both strings pre-scale by m = 100/(d1+1) (two passes of one
*  driver, 8-visit budget each); each of the NINE quotient digits comes from
*  a two-digit estimate against v1 with the Knuth D3 while-loop correction
*  (qhat = 100 special-cased), a byte-wise multiply-subtract with +100
*  borrow correction, and the rare add-back. The exponent W = (Ea-Ef) + 64.
*  Divide-by-zero exits through the >0FBC error->02 stub with the dividend's
*  sign; a zero dividend is a clean zero. Ends through >0F18's error-clear +
*  the shared normalize (which also discards the two rawest digits).
*  ============================================================================
        AORG >0FF4
FDIV    MOV  R11,R10
        JMP  FDBODY
SDIV    MOV  R11,R10
        BL   @FPPOP
FDBODY  LI   R3,>834A
        MOV  *R3,R8
        LI   R0,>835C
        XOR  *R0,R8
        MOVB R8,@>8375              * the result sign (dividend^divisor)
        ABS  *R3
        JEQ  FPESYN                 * divisor zero -> error >02, saturate
        ABS  *R0
        JEQ  FPZERO                 * dividend zero -> a clean zero
        MOVB *R0,R9                 * W := (Ea - Ef) + 64
        SB   *R3,R9
        SRA  R9,8
        AI   R9,>0040
        MOV  R9,@>8376
        LI   R4,>0004               * relocate the divisor to >8354-835B and
        LI   R5,>8364               * zero the dividend's 8-byte extension
FDRLOC  MOV  *R3+,@>0008(R3)        * (the post-increment lands the +8 dest)
        CLR  *R5+
        DEC  R4
        JGT  FDRLOC
        MOVB R4,@>835C              * the dividend's zero spill digit
        LI   R5,>83E1               * R5 -> R0's low byte (the WP alias)
        LI   R6,>83E3               * R6 -> R1's low byte
        LI   R7,>0064               * the radix
        CLR  R2
        MOVB @>8355,@>83E5          * R2 := the divisor's leading digit
        CI   R2,>0031
        JGT  FDLEN                  * d1 >= 50 -> no scaling
        INC  R2                     * m := 100 / (d1+1)
        CLR  R3
        MOV  R7,R4
        DIV  R2,R3
        LI   R9,>835C               * pass 1: the divisor digits...
FDSCPS  LI   R4,>0008               * (an 8-visit byte budget per pass)
FDSCSK  DEC  R4
        DEC  R9
        MOVB *R9,R0
        JEQ  FDSCSK                 * low-order zeros just spend budget
        CLR  R0
FDSCLP  MOV  R0,R2                  * the carry in
        MOVB *R9,*R5                * R0 := the digit (low-byte insert)
        MPY  R3,R0                  * * m
        A    R2,R1                  * + carry
        DIV  R7,R0                  * R0 = the carry out, R1 = the digit
        MOVB *R6,*R9                * store it
        DEC  R9
        DEC  R4
        JGT  FDSCLP
        CI   R9,>8354               * pass 1 ends exactly at >8354 ->
        JNE  FDSCEN                 * run pass 2 over the dividend
        LI   R9,>8364
        JMP  FDSCPS
FDSCEN  MOVB *R5,@>835C             * the dividend spill := the final carry
FDLEN   LI   R6,>0008               * the divisor's significant length
FDLENL  DEC  R6
        MOVB @>8354(R6),R0
        JEQ  FDLENL
        CLR  R7
        MOVB @>8355,@>83EF          * R7 := v1 (50..99 once scaled)
        MOV  R7,R8
        MPY  @F100W,R8              * R8:R9 := v1 * 100
        MOVB @>8356,@>83F1          * R8 := v2 (the product high was zero)
        A    R8,R9                  * R9 := V2 = v1*100 + v2
        LI   R5,>FFF7               * nine quotient digits -> >834B-8353
        LI   R11,>835C              * the remainder window base
FDDIG   CLR  R2
        MOVB *R11,@>83E5            * qhat estimate: (u0*100 + u1) / v1
        MPY  @F100W,R2
        CLR  R0
        MOVB @>0001(R11),@>83E1
        A    R0,R3
        DIV  R7,R2                  * R2 = qhat, R3 = rhat
        MPY  @F100W,R3              * R3:R4 := rhat*100...
        MOVB @>0002(R11),@>83E1
        A    R0,R4                  * ...+ u2
        MOV  R2,R0
        MPY  R8,R0                  * R0:R1 := qhat * v2
        C    R2,@F100W              * the qhat = 100 special case
        JEQ  FDQ100
        S    R4,R1                  * T := qhat*v2 - (rhat*100 + u2)
        JMP  FDQTST
FDQ100  S    R4,R1
FDQDN   DEC  R2                     * Knuth D3: while T > 0 { qhat--,
        S    R9,R1                  *            T -= V2 }
FDQTST  JGT  FDQDN
        MOV  R2,R2
        JEQ  FDSTOR                 * qhat = 0: the window is unchanged
        CLR  R3
        MOV  R6,R4                  * multiply-subtract: window -= qhat*divisor
        A    R6,R11
FDMSUB  MOV  R0,R3                  * the carry (the first pass rides the
        MOVB @>8354(R4),@>83E1      * dead product high = 0)
        MPY  R2,R0                  * v_i * qhat
        A    R3,R1                  * + carry
        DIV  @F100W,R0              * split: R0 = carry', R1 = the digit
        SB   @>83E3,*R11            * window byte -= the digit
        JGT  FDMSOK
        JEQ  FDMSOK
        AB   @F100B,*R11            * negative: += 100, borrow up
        INC  R0
FDMSOK  DEC  R11
        DEC  R4
        JGT  FDMSUB
        SB   @>83E1,*R11            * the leading byte -= the final carry
        JGT  FDSTOR
        JEQ  FDSTOR
        DEC  R2                     * the rare add-back (qhat one too big)
        MOV  R6,R4
        A    R6,R11
FDADBK  AB   @>8354(R4),*R11
        CB   *R11,@F100B
        JL   FDADB2
        SB   @F100B,*R11
        AB   @F01B,@>FFFF(R11)
FDADB2  DEC  R11
        DEC  R4
        JGT  FDADBK
FDSTOR  MOVB @>83E5,@>8354(R5)      * the quotient digit (R2's low byte)
        INC  R11                    * slide the window
        INC  R5
        JLT  FDDIG
        B    @FPNR18                * -> the error clear + normalize + round

*  ============================================================================
*  The round / normalize / status / error cluster (>0F1C-0FF3) — the shared
*  tails every FP routine exits through, at their authentic homes.
*  ============================================================================
        AORG >0F1C
*  FPNRM: the shared normalize — strip leading zero digits (guards included),
*  charging the exponent scratch; an all-zero mantissa clears FAC (a true
*  zero); no leading zeros falls straight into the round.
FPNRM   LI   R1,>FFF7
FPNSCN  MOVB @>8354(R1),R2          * scan >834B.. for the first non-zero
        JNE  FPNFND
        INC  R1
        JLT  FPNSCN
FPZERO  CLR  @>834A                 * all zero: FAC := 0 (first words clear)
        CLR  @>834C
        JMP  FPTA6
FPNFND  MOV  R1,R0
        AI   R0,>0009               * R0 := the leading-zero count
        JEQ  RND1B                  * none -> the guard round
        S    R0,@>8376              * exponent -= the shift
        LI   R2,>834B
FPNSHL  MOVB @>8354(R1),*R2+        * shift the digits up...
        INC  R1
        JLT  FPNSHL
FPNZFL  MOVB R1,*R2+                * ...and zero-fill the tail (R1 = 0 here)
        DEC  R0
        JGT  FPNZFL
        JMP  RND1B

*  ROUND1 (XML >01): round on the first guard digit, half up (>= 50), the
*  carry rippling through the seven digits; a full ripple (all 99s) bumps
*  the exponent and leaves 1.0. Falls into the store/status tail. FADD
*  enters at RND1B with R10 already parked.
        AORG >0F54
ROUND1  MOV  R11,R10
RND1B   LI   R0,>3200
        C    @>8352,R0              * the guard word: >= >3200 rounds up
        JLT  FPBIG
        LI   R1,>0007
RNDPOS  LI   R2,>0100
        LI   R0,>6400
RNDRIP  AB   R2,@>834A(R1)          * +1 at the digit...
        CB   @>834A(R1),R0
        JL   FPBIG                  * ...no carry -> done
        SB   R0,@>834A(R1)
        DEC  R1
        JGT  RNDRIP
        INC  @>8376                 * the ripple ran out the top: exponent++
        MOVB R2,@>834B              * and the mantissa becomes 1.00…
*  FPBIG — the store/status tail: exponent-range check (>= >80 is over/
*  underflow -> the OVEXP protocol), the exponent byte re-installed from the
*  scratch, the sign re-applied from >8375, then the FAC-value status.
FPBIG   MOV  @>8376,R3
        CI   R3,>0080
        JHE  OVBODY
        MOVB @>83E7,@>834A          * FAC[0] := the exponent (R3's low alias)
        MOVB @>8375,R2
        INV  R2
        JLT  FPTA6                  * positive -> done
        NEG  @>834A                 * negative -> negate the first word
        JMP  FPTA6

*  STST (XML >03) falls into the tails: FPTA6 sets the 9900 flags from FAC's
*  first word (sign/zero); FPTAA stores the RAW STST byte to >837C wholesale
*  (FCOMP arrives here with its compare flags live) and returns via R10.
        AORG >0FA4
STSTH   MOV  R11,R10
FPTA6   MOV  @>834A,R1
FPTAA   STST R2
        MOVB R2,@>837C
        B    *R10

*  ROUND (XML >02): round AT the digit position given in >8354 (the error
*  cell doubles as the parameter), unconditionally, with the same ripple.
*  Garbage-corner ledger: positions >= >96 walk the ripple through the LIVE
*  GPLWS itself, so the outcome depends on the interpreter's own transient
*  register file — which no reimplementation can share. Positions >AA/>AB/
*  >B1 (walks starting on the R10/R13 cells) diverge from the authentic;
*  kept as a garbage corner (RECON §27) with a tripwire in gpl_fp.rs. Every
*  other position byte 0-255 is differentially bit-exact.
        AORG >0FB2
ROUNDH  MOV  R11,R10
        MOVB @>8354,R1
        SRL  R1,8
        JMP  RNDPOS

*  The syntax-error entry (CSN's exit): error >02, saturate, status.
FPESYN  LI   R9,>0200
        JMP  OVSAT

*  OVEXP (XML >04): the over/underflow filter — a negative exponent scratch
*  underflows to a clean zero; positive overflows saturate. OV (XML >05):
*  saturate unconditionally with error >01.
        AORG >0FC2
OVEXPH  MOV  R11,R10
OVBODY  MOVB @>8376,R2
        JLT  FPZERO                 * underflow -> the zero exit
        JMP  OVGO
        AORG >0FCC
OVH     MOV  R11,R10
OVGO    LI   R9,>0100               * error >01 (overflow / divide-by-zero)
OVSAT   LI   R0,>809D               * the saturation first word: +>7F63 /
        MOVB @>8375,R2              * ->809D (its negation) per the sign
        JLT  OVNEG
        NEG  R0
OVNEG   LI   R2,>834A
        MOV  R0,*R2+
        LI   R0,>6363               * digits: all 99s
        MOV  R0,*R2+
        MOV  R0,*R2+
        MOV  R0,*R2
        MOVB R9,@>8354              * the error code
        JMP  FPTA6

*  ============================================================================
*  Zone H (>1CF0+): stores, shifts, VDP addressing, spilled impls, stubs.
*  (Relocated from >0D40 at M5 slice 1 — the FP package claims >0D3A+; this
*  now squats the M6 BASIC-tables region, free indefinitely per the M6
*  deferral policy.)
*  ============================================================================
        AORG >1CF0

*  STOD: store R2 at (R7 addr, R8 space), byte or word per R5. A CPU store
*  whose LAST byte lands at >837D also paints that byte at the cursor — the
*  character-buffer echo (§25, the authentic >0232 tail). On the echo path
*  R1/R6/R12 are clobbered and a swapped R2 stays swapped — every caller is
*  done with them. The echo preserves the whole scratchpad: >8300-830F is
*  the running program's space (see the §25 block), and MPY/DIV's parked R0
*  survives the standard path.
STOD    MOV  R8,R8
        JNE  STODV
        COC  @W1,R5
        JEQ  STODW1
        MOVB R2,*R7
        CI   R7,>837D
        JNE  STODRT
        B    @CHBWR                 * byte store at >837D -> echo (R2 high)
STODRT  B    *R11
STODW1  MOV  R7,R1
        MOVB R2,*R1+
        SWPB R2
        MOVB R2,*R1
        CI   R1,>837D
        JNE  STODW2
        B    @CHBWR                 * word store at >837C/D -> echo (R2 is
STODW2  SWPB R2                     * still swapped: the last byte is high)
        B    *R11
STODV   MOV  R11,R12
        BL   @VWR
        COC  @W1,R5
        JEQ  STODVW
        MOVB R2,@>8C00
        B    *R12
STODVW  MOVB R2,@>8C00
        SWPB R2
        MOVB R2,@>8C00
        SWPB R2
        B    *R12

*  STW2 / LDW2: word store/load of R2 at (R7,R8) regardless of R5.
*  (STW2 carries the >837D echo too — the authentic MUL/DIV stores run
*  through the same >0232 tail, §25.)
STW2    MOV  R8,R8
        JNE  STW2V
        MOV  R7,R1
        MOVB R2,*R1+
        SWPB R2
        MOVB R2,*R1
        CI   R1,>837D
        JNE  STW2R
        B    @CHBWR                 * last byte at >837D -> the cursor echo
STW2R   SWPB R2
        B    *R11
STW2V   MOV  R11,R12
        BL   @VWR
        JMP  STODVW
LDW2    MOV  R8,R8
        JNE  LDW2V
        MOV  R7,R1
        MOVB *R1+,R2
        SWPB R2
        MOVB *R1,R2
        SWPB R2
        B    *R11
LDW2V   MOV  R11,R12
        MOV  R7,R3
        BL   @VRD
        MOVB @>8800,R2
        MOVB @>8800,R1
        SRL  R1,8
        ANDI R2,>FF00
        SOC  R1,R2
        B    *R12

*  VRD / VWR: set the VDP read / write address from R3 / R7 (low byte
*  first, then the high byte with the mode bits). Clobbers R1.
VRD     MOV  R3,R1
        SWPB R1
        MOVB R1,@>8C02
        MOV  R3,R1
        ANDI R1,>3FFF
        MOVB R1,@>8C02
        B    *R11
VWR     MOV  R7,R1
        SWPB R1
        MOVB R1,@>8C02
        MOV  R7,R1
        ANDI R1,>3FFF
        ORI  R1,>4000
        MOVB R1,@>8C02
        B    *R11

*  Shifts (spilled from zone C): dest by count (0 -> 16); byte SRC rotates
*  in 8 bits via a doubled pattern. No status effect.
SRAH    BL   @SHCNT
SRA1    SRA  R2,1
        DEC  R6
        JNE  SRA1
        JMP  SHDONE
SLLH    BL   @SHCNT
SLL1    SLA  R2,1
        DEC  R6
        JNE  SLL1
        JMP  SHDONE
SRLH    BL   @SHCNT
SRL1    SRL  R2,1
        DEC  R6
        JNE  SRL1
        JMP  SHDONE
SRCH    BL   @SHCNT
        COC  @W1,R5
        JEQ  SRC1
        MOVB R2,R1                  * byte rotate: double into both halves
        SRL  R1,8
        SOC  R1,R2
SRC1    SRC  R2,1
        DEC  R6
        JNE  SRC1
SHDONE  BL   @STOD
        B    @LOOP

*  FETCH (>88/89): read the byte the sub-stack top points at (inline data
*  after a CALL) into the destination, and bump the stored address so the
*  caller resumes past it. Works through the sub-stack, as the authentic
*  interpreter does — our resume position is pushed as a frame and popped
*  after the excursion (the frame bytes remain above the pointer, exactly
*  the residue the authentic leaves; oracle-verified).
FETCHH  BL   @GPUSH                 * push our resume; pointer now at +2
        MOVB @>8373,R1
        SRL  R1,8
        MOV  R1,R4
        AI   R1,-2                  * R1 -> the caller's frame (stored addr)
        CLR  R6
        MOVB @>8300(R1),R6
        MOVB @>8301(R1),R0
        SRL  R0,8
        SOC  R0,R6                  * the inline-data address
        BL   @GSETA
        CLR  R2
        MOVB *R13,R2                * the data byte
        INC  @>8300(R1)             * bump the stored word past the data
        CLR  R6
        MOVB @>8300(R4),R6          * pop our own frame: the resume address
        MOVB @>8301(R4),R0
        SRL  R0,8
        SOC  R0,R6
        DECT @>8373
        BL   @GSETA                 * back to our stream
        COC  @W1,R5                 * the WORD form (>89) stores the byte
        JNE  FETCHB                 * SIGN-EXTENDED as a word (authentic
        SRA  R2,8                   * >0152 SRA + the R5-honoring store —
FETCHB  BL   @STOD                  * the 256-opcode sweep caught the
        B    @LOOP                  * store-a-byte-regardless reading)

*  RTNC implementation: pop the sub-stack word (post-decrementing the
*  pointer byte, word-quirk preserved) and resume there.
RTNCI   MOVB @>8373,R1
        SRL  R1,8
        CLR  R6
        MOVB @>8300(R1),R6
        MOVB @>8301(R1),R0
        SRL  R0,8
        SOC  R0,R6
        DECT @>8373
        BL   @GSETA
        B    @LOOP

*  CALL implementation: read the 16-bit target, reset the condition bit,
*  push the resume address, jump.
CALLI   BL   @ADDR16
        SZCB @MASK20,@>837C
        BL   @GPUSH                 * preserves R6 (the target)
        BL   @GSETA
        B    @LOOP

*  ---- loud stubs -------------------------------------------------------
*  Everything not implemented lands here with the opcode in the
*  breadcrumb cell >837D and spins visibly (peek >837D to identify).
*  With M1-M5/M7 complete, only the M6-deferred BASIC surface still
*  routes here (deferral policy: README.md / LIMITATIONS.md L9).
STUB    MOVB R9,@>837D
SHALT   JMP  SHALT
ISTUB   LIMI >0000                  * the (unreachable) level-2 vector target
IHALT   JMP  IHALT

*  ============================================================================
*  Zone I (>1440+): FMT (>08) — the screen-format sub-interpreter (M4). Entry
*  pinned at >04DE (trampoline above); this body is free-placed (a squatter in
*  the deferred-cassette span, layout ledger). FMT switches the interpreter
*  into an independent sub-language: each format byte's top three bits select a
*  group via >0CDC; the low five bits are a count (n -> n+1) or a control
*  selector. Groups: HTEXT/VTEXT (inline text, horizontal/vertical), HCHAR/VCHAR
*  (repeat a char), HMOVE/VMOVE (advance the cursor without output), RPTB (a
*  repeat block), and the E/F control group (string-from-operand, FEND, BIAS,
*  ROW, COL). The cursor is the linear name-table cell R7 (row*32 + col, base
*  VRAM 0 like ALL — RECON §18) with a single 768-cell wrap; the persistent
*  cursor cells are >837E (row) / >837F (col). R3 is a bias added to every
*  emitted char; R9 is the RPTB nesting depth; the GPL sub-stack (>8373) holds
*  the per-level loop counter. Grammar pinned by disassembling the authentic
*  >04DE-05B7 as a spec (P5); every sub-op is differentially gated (gpl_fmt.rs).
*  ============================================================================
        AORG >1440
*  Prologue: reset the bias + RPTB depth, load the cursor from its cells.
FMTBODY CLR  R9                     * RPTB nesting depth
        CLR  R3                     * character bias
        BL   @FLDCUR                * R7 := (>837E) * 32 + (>837F)
*  Fetch the next format byte and dispatch it by its top three bits.
FMTNXT  MOVB *R13,R8                * next format byte -> R8 high
        MOV  R8,R4
        SRL  R4,12                  * top nibble (word access pairs 0/1, 2/3, ...)
        MOV  @FMTTAB(R4),R4         * -> one of eight group handlers
        MOV  R8,R1
        ANDI R1,>1F00
        SRL  R1,8                   * R1 = the low-5-bit count / control selector
        B    *R4

*  0/1 HTEXT, 2/3 VTEXT: emit n+1 inline chars (+bias each), advancing the
*  cursor by one cell (HTEXT) or one row (VTEXT) with the 768-cell wrap.
FHTEX   LI   R4,>0001              * stride: one column
        JMP  FTEXT
FVTEX   LI   R4,>0020              * stride: one row (32 cells)
FTEXT   INC  R1                    * n+1 characters
FTEXTL  MOVB *R13,R6               * read one inline char
        A    R3,R6                 * += bias
        BL   @FEMIT                * write it, advance, wrap
        DEC  R1
        JNE  FTEXTL
        B    @FMTNXT

*  4/5 HCHAR, 6/7 VCHAR: emit ONE char (read + biased once) n+1 times.
FHCHA   LI   R4,>0001
        JMP  FCHAR
FVCHA   LI   R4,>0020
FCHAR   MOVB *R13,R6               * read the character once
        A    R3,R6                 * += bias once
        INC  R1
FCHARL  BL   @FEMIT
        DEC  R1
        JNE  FCHARL
        B    @FMTNXT

*  8/9 HMOVE, A/B VMOVE: advance the cursor n+1 columns / rows, no output;
*  the move path takes the single >=>0300 wrap (not the char-write range check).
FHMOV   INC  R1                    * n+1 columns
        A    R1,R7
        JMP  FMVWR
FVMOV   INC  R1
        SLA  R1,5                  * (n+1) * 32 = n+1 rows
        A    R1,R7
FMVWR   CI   R7,>0300
        JL   FMVN
        AI   R7,>FD00              * wrap by one screen (768 cells)
FMVN    B    @FMTNXT

*  C/D RPTB: open a repeat block of n+1 passes. Push the loop counter (255-n)
*  onto the GPL sub-stack at >8373; the matching FEND (>FB) adds SPEED each
*  pass until the byte wraps to 0. Bump the nesting depth so FEND loops.
FRPTB   INC  R9                    * nesting depth++
        INCT @>8373               * sub-stack pointer += 2
        MOVB @>8373,R6
        SRL  R6,8                  * R6 = the new pointer
        LI   R2,>FF00
        SLA  R1,8                  * n -> high byte
        S    R1,R2                 * R2 high = 255 - n = ~n as a byte
        MOVB R2,@>8300(R6)         * push the loop counter
        B    @FMTNXT

*  E/F control group: the sub-op is the format byte & >1F. 0x00-0x1A -> a
*  string from a GAS operand; 0x1B FEND / block-end; 0x1C/0x1D BIAS (immediate
*  / from a GAS operand); 0x1E ROW; 0x1F COL.
FCTRL   CI   R1,>001B
        JEQ  FEND
        JL   FMSTR
        CI   R1,>001C
        JEQ  FBIASI
        CI   R1,>001D
        JEQ  FBIASG
        CI   R1,>001E
        JEQ  FROW
        JMP  FCOL

*  FEND (>FB): outside a RPTB, store the cursor and rejoin the interpreter;
*  inside one, tick the loop counter and re-run the block or pop the level.
FEND    MOV  R9,R9
        JNE  FBLKE
        BL   @FSTCUR               * >837E/837F := R7's row/col
        B    @LOOP                 * back to the interpreter fetch (>0070)
FBLKE   MOVB *R13,R4               * loop-back GROM address hi
        MOVB *R13,R5               * loop-back GROM address lo
        MOVB @>8373,R6
        SRL  R6,8
        AB   R14,@>8300(R6)        * counter += SPEED (R14 high)
        JEQ  FBLKX                 * wrapped to 0 -> the block is done
        MOVB R4,@>0402(R13)        * else re-point the GROM at the block start
        MOVB R5,@>0402(R13)
        B    @FMTNXT
FBLKX   DECT @>8373               * pop the loop counter
        DEC  R9                    * nesting--
        B    @FMTNXT

*  BIAS (>FC): the next inline byte becomes the character bias.
FBIASI  MOVB *R13,R3               * bias := next byte (high-justified)
        B    @FMTNXT

*  BIAS from a GAS operand (>FD): the byte at the operand address is the bias.
*  (Our OPGET returns the address in R3 — save/restore the bias around it.)
FBIASG  BL   @OPGET                * -> R3 addr, R4 space (0 CPU / 1 VDP)
        MOV  R4,R4
        JNE  FBIGV
        CLR  R6
        MOVB *R3,R6                * CPU byte
        MOV  R6,R3                 * -> bias (high byte, low byte clear)
        B    @FMTNXT
FBIGV   MOV  R3,R1                 * VDP: point the read address at R3
        SWPB R1
        MOVB R1,@>8C02
        MOV  R3,R1
        ANDI R1,>3FFF
        MOVB R1,@>8C02
        CLR  R3
        MOVB @>8800,R3             * bias := the VDP byte
        B    @FMTNXT

*  ROW (>FE) / COL (>FF): set the cursor row / column from the next byte.
FROW    MOVB *R13,R6
        SRL  R6,8                  * new row
        SLA  R6,5                  * row * 32
        ANDI R7,>001F              * keep the current column
        A    R6,R7
        B    @FMTNXT
FCOL    MOVB *R13,R6
        SRL  R6,8                  * new column
        ANDI R7,>FFE0              * keep row * 32
        A    R6,R7
        B    @FMTNXT

*  String from a GAS operand (E0-FA): emit n+1 chars from the operand address
*  (+bias each), advancing horizontally. Source may be CPU or VDP (OPGET's R4).
FMSTR   INC  R1                    * n+1 characters
        MOV  R1,R10                * save the count (OPGET clobbers R1)
        MOV  R3,R0                 * save the bias (OPGET returns the addr in R3)
        BL   @OPGET                * -> R3 addr, R4 space
        MOV  R3,R2                 * R2 = source cursor
        MOV  R4,R8                 * R8 = source space (0 CPU / 1 VDP)
        MOV  R0,R3                 * restore the bias
        LI   R4,>0001              * horizontal stride
FMSTRL  MOV  R8,R8
        JNE  FMSTRV
        CLR  R6
        MOVB *R2+,R6               * CPU source byte
        JMP  FMSTRE
FMSTRV  MOV  R2,R1                 * VDP source byte at R2
        SWPB R1
        MOVB R1,@>8C02
        MOV  R2,R1
        ANDI R1,>3FFF
        MOVB R1,@>8C02
        CLR  R6
        MOVB @>8800,R6
        INC  R2
FMSTRE  A    R3,R6                 * += bias
        BL   @FEMIT
        DEC  R10
        JNE  FMSTRL
        B    @FMTNXT

*  FEMIT: write R6 (an already-biased char) at the cursor R7, then advance the
*  cursor by R4 and apply the char-write wrap ([>0300,>0320) -> one screen).
FEMIT   MOV  R7,R5                 * set the VDP write address = R7 | >4000
        SWPB R5
        MOVB R5,@>8C02
        MOV  R7,R5
        ANDI R5,>3FFF
        ORI  R5,>4000
        MOVB R5,@>8C02
        MOVB R6,@>8C00             * emit the character (VDP auto-increments)
        A    R4,R7                 * advance the cursor
        CI   R7,>0320
        JHE  FEMITR
        CI   R7,>0300
        JL   FEMITR
        AI   R7,>FD00
FEMITR  B    *R11

*  FLDCUR / FSTCUR: convert between the linear cursor R7 and the >837E (row) /
*  >837F (col) persistence cells.
FLDCUR  CLR  R7
        MOVB @>837E,R7             * row -> high byte
        SRL  R7,3                  * row * 32
        CLR  R6
        MOVB @>837F,R6
        SRL  R6,8                  * col
        A    R6,R7                 * R7 = row*32 + col
        B    *R11
FSTCUR  MOV  R7,R6
        SRL  R6,5                  * row = R7 / 32
        SWPB R6
        MOVB R6,@>837E
        MOV  R7,R6
        ANDI R6,>001F              * col = R7 & >1F
        SWPB R6
        MOVB R6,@>837F
        B    *R11

*  ============================================================================
*  Zone J (>1620+): the M4 interpreter completions — COINC, SWGR, RTGR.
*  Free-placed bodies (the >0C3E/>0C7E table entries carry our addresses, P8);
*  semantics disassembled-as-spec from the authentic >06D2/>004E/>082C
*  (RECON §25). Sits in the BASIC-half span the M6 deferral leaves free.
*  ============================================================================
        AORG >1620

*  COINC (>EC-EF): a bitmap coincidence test between two Y,X byte-pair points.
*  Stream: [dest GAS][source imm/mem][scale][table16]. Deltas (src - dest, byte
*  math), optionally SRA-scaled (count = scale's low nibble, 0 -> 16), are
*  offset and bounded by the 4-byte header [Ylim,Xlim,Yoff,Xoff] at table16
*  (GROM); inside the box, bit (dY*(Xlim+1) + dX) of the bitmap that follows
*  the header (MSB-first per byte) decides. >837C is overwritten wholesale
*  (>20 hit / >00 miss) — the authentic clobbers H/GT/CARRY/OV too. The fetch
*  position is pushed before re-addressing and restored by exiting through
*  RTNC (which preserves the fresh status).
COINCH  COC  @W1,R5
        JEQ  COIWD
        SRA  R2,8                   * byte forms: the authentic right-justified
        SRA  R0,8                   * sign-extended view (RECON §25)
COIWD   MOV  R0,R8
        MOV  R8,R3
        SB   R2,R3                  * R3 high := Y delta (src.Y - dest.Y)
        SWPB R8
        SWPB R2
        SB   R2,R8                  * R8 high := X delta
        CLR  R10
        MOVB *R13,R10               * the scale byte
        SRL  R10,8
        BL   @ADDR16                * the coincidence table's GROM address
        MOV  R6,R5                  * (R5's flag role is done; park the base)
        BL   @GPUSH                 * save the interpreter's fetch position
        MOV  R5,R6
        BL   @GSETA                 * re-address to the table header
        MOVB *R13,R2                * Y limit
        MOVB *R13,R1                * X limit
        MOVB *R13,R6                * Y offset
        MOVB *R13,R7                * X offset
        MOV  R10,R0
        JEQ  COIBX                  * scale 0 -> raw deltas
        SRA  R3,0                   * scale the deltas (count = R0 low nibble;
        SRA  R8,0                   * 0 -> 16, the 9900 rule)
COIBX   AB   R7,R8                  * X delta += X offset
        JLT  COINO                  * negative -> outside
        AB   R6,R3                  * Y delta += Y offset
        JLT  COINO
        CB   R3,R2                  * beyond the Y limit?
        JGT  COINO
        CB   R8,R1                  * beyond the X limit?
        JGT  COINO
        SRL  R1,8                   * bitmap stride = X limit + 1
        INC  R1
        SRL  R3,8
        MPY  R3,R1                  * R1:R2 := row * stride
        SRL  R8,8
        A    R8,R2                  * + column -> the bit index
        MOV  R2,R0
        ANDI R2,>FFF8
        S    R2,R0                  * R0 := the bit within the byte
        SRA  R2,3                   * R2 := the byte offset...
        A    R5,R2                  * ...+ the table base...
        INCT R2
        INCT R2                     * ...+ the 4-byte header
        MOVB R2,@>0402(R13)         * re-address to the bitmap byte
        INC  R0                     * shift count = bit + 1
        SWPB R2
        MOVB R2,@>0402(R13)
        LI   R2,>2000
        MOVB *R13,R3                * the bitmap byte
        SLA  R3,0                   * MSB-first: bit n carries out on shift n+1
        JOC  COIYES
COINO   CLR  R2
COIYES  MOVB R2,@>837C              * status := >20 / >00 wholesale (authentic)
        B    @RTNC                  * restore the fetch position, keep the status

*  SWGR (>F8-FB): switch GROM base. Dest value -> the new R13 (a >9800+4n port
*  base); source value -> the new GROM address. The return position is pushed
*  twice and the top slot overwritten with the old R13 (stack: retPC, oldR13 —
*  RTGR unwinds it); a settle read strobes the new base (the authentic
*  advance); the condition bit clears through the >006A soft entry.
SWGRH   COC  @W1,R5
        JEQ  SWGWD
        SRA  R2,8                   * byte forms: right-justified sign-extended
        SRA  R0,8
SWGWD   MOV  R0,R10                 * park the new address (GPUSH clobbers R0/R1)
        BL   @GPUSH                 * push the return position...
        BL   @GPUSH                 * ...twice (the authentic >004E shape)
        MOV  R13,@>8300(R1)         * overwrite the top slot with the old base
        MOV  R2,R13                 * R13 := the new GROM base
        MOVB *R13,R4                * settle strobe (advances the new base)
        MOV  R10,R6
        BL   @GSETA                 * the new fetch address
        B    @SOFT                  * clear the condition bit, run

*  RTGR (>13): return across a base switch — pop the saved R13 (the pop's
*  address-port write of that value is the authentic spurious side effect),
*  poke the restored base's GRAM data port (inert on mask ROMs, faithful),
*  then RTN pops the return position and clears the condition bit.
RTGRH   BL   @GPOP                  * R4 = the popped slot's index
        MOV  @>8300(R4),R13         * restore the saved GROM base
        MOVB R4,@>0400(R13)         * the authentic GRAM-port poke
        B    @RTN

*  CEQ (>D4-D7) is NOT a jump-family compare: its authentic tail is the CZ
*  raw-STST — >837C replaced wholesale with H/GT/EQ from the compare, C = the
*  opcode's word bit (the SRL R8,9 dispatch carry = opcode bit 0, §25), OV = 0
*  (the authentic's stale OV is deterministically clear on these paths).
CEQH    BL   @CMP
        STST R6
        ANDI R6,>E000               * H/GT/EQ from the compare
        COC  @W1,R5
        JNE  CEQST
        ORI  R6,>1000               * word forms: the authentic C bit
CEQST   MOVB R6,@>837C
        B    @LOOP

*  ---- the >837D character buffer (§25) ----------------------------------
*  >837D is a live window onto the screen cell at the cursor (>837E row /
*  >837F col): an operand read naming it fetches the screen byte into the
*  cell first (CHBRD, from OPGET — every CPU form, indirection included); a
*  store whose last byte lands on it also paints the byte at the cursor
*  (CHBWR, from STOD/STW2). In multicolour mode (FLAGS >02 — tested against
*  the NASTY @>0072 word, our own LIMI 2 operand like the authentic's) the
*  screen cell is the 4-bit pattern nibble at (row,col): reads extract it,
*  writes read-modify-write it, the column's low bit picking the half.
*
*  Register discipline (the Tunnels-of-Doom lesson): these paths run in the
*  middle of the USER'S instruction — >8300-830F is the running program's
*  own variable space, and real GPL uses the >837D window as a drawing
*  primitive (ToD's corridor renderer plots its floor-edge chars through it
*  with live state in >8306). The authentic tails preserve the entire pad;
*  ours therefore spill NOTHING to scratchpad. CHBRD clobbers R0/R1/R6
*  (OPGET's own declared scratch) and returns via R12; CHBWR clobbers
*  R1/R6/R12 and — multicolour only — R0/R2 (the authentic clobbers more
*  there); the standard write path preserves R0, which MPY/DIV hold their
*  second result word in across the first of two stores.

*  CURSM: the multicolour cursor-cell address, set on the VDP in READ form
*  (the mode bit is never applied on this first setup — authentic
*  >08AA-08CE). Cell = >0800 + (row/8)*256 + row%8 + (col/2)*8. In: R1 =
*  column, R6 = row. Out: R1 = the cell address, R6 = >0100|column (bit 0 =
*  the nibble parity). Uses R1/R6 only, rebuilding row/col from >837E/>837F
*  instead of parking them.
CURSM   SRL  R1,1
        SLA  R1,3                   * (col/2)*8
        ANDI R6,>0007
        A    R6,R1                  * + row%8
        CLR  R6
        MOVB @>837E,R6
        SRL  R6,8                   * the row again
        ANDI R6,>00F8
        SLA  R6,5
        A    R6,R1                  * + (row/8)*256
        AI   R1,>0800               * + the pattern-table base
        MOVB @>83E3,*R15            * LSB (R1's low-byte alias), then MSB
        MOVB R1,*R15
        CLR  R6
        MOVB @>837F,R6
        SRL  R6,8                   * the raw column again
        ORI  R6,>0100               * the mode/parity return
        B    *R11

*  CHBRD: fetch the screen byte at the cursor into >837D (multicolour: the
*  nibble, right-justified). Entered from OPGET by `MOV R11,R12; B @CHBRD`
*  — R12 is the way home (straight to OPGET's caller); R11 is free for the
*  CURSM linkage.
CHBRD   CLR  R1
        MOVB @>837F,R1
        SRL  R1,8                   * the column
        CLR  R6
        MOVB @>837E,R6
        SRL  R6,8                   * the row
        COC  @>0072,R14             * multicolour?
        JEQ  CHBRM
        SLA  R6,5
        A    R6,R1                  * row*32 + col, read form
        MOVB @>83E3,*R15
        MOVB R1,*R15
        CLR  R6                     * standard: no nibble select
        JMP  CHBRC
CHBRM   BL   @CURSM
CHBRC   MOVB @>8800,R0              * the screen byte
        MOV  R6,R6
        JEQ  CHBRS
        SRA  R6,1                   * multicolour: parity -> carry
        JNC  CHBRE
        SLA  R0,4                   * odd column: the low nibble
CHBRE   SRL  R0,4
CHBRS   MOVB R0,@>837D
        B    *R12

*  CHBWR: paint the byte just stored at >837D onto the screen at the cursor.
*  Jumped into from STOD/STW2 with the byte in R2's HIGH half and R11 = the
*  store's caller. Standard: one write-mode setup + the data write
*  (authentic >0264), R0/R2 preserved. Multicolour: read the pattern byte,
*  merge the new nibble from >837D by column parity, re-address in write
*  mode, store (authentic >08D6-08FE).
CHBWR   MOV  R11,R12                * the store's caller
        CLR  R1
        MOVB @>837F,R1
        SRL  R1,8                   * the column
        CLR  R6
        MOVB @>837E,R6
        SRL  R6,8                   * the row
        COC  @>0072,R14             * multicolour?
        JEQ  CHBWM0
        SLA  R6,5
        A    R6,R1                  * row*32 + col
        ORI  R1,>4000               * write form
        MOVB @>83E3,*R15
        MOVB R1,*R15
        MOVB R2,@>8C00              * the byte at the cursor cell
        B    *R12
CHBWM0  BL   @CURSM                 * multicolour: read-form setup
        MOVB @>8800,R0              * the current pattern byte
        MOVB @>837D,R2              * the just-stored nibble source
        ANDI R2,>0F00
        SRA  R6,1
        JOC  CHBWO
        ANDI R0,>0F00               * even column: keep low, new nibble high
        SLA  R2,4
        JMP  CHBWM
CHBWO   ANDI R0,>F000               * odd: keep high, new nibble low
CHBWM   A    R2,R0
        ORI  R1,>4000               * re-address in write mode
        MOVB @>83E3,*R15
        MOVB R1,*R15
        MOVB R0,@>8C00              * the merged byte
        B    *R12

*  ============================================================================
*  Zone K (>1820+): the cassette modem layer — IO 4 (write) / 5 (read) / 6
*  (verify) + the cassette timer ISR (M4 slice 5; RECON §26). Free-placed: no
*  external caller enters the authentic homes (>1346/>142E/>1426/>1404 are
*  IO-table / ISR-fork targets, both ours). The engines are present and
*  behavior-correct per §11's disposition; the emulator models no 9901
*  interval timer and no tape line, so the timed sections park on their first
*  half-cell wait identically under both ROMs — the observable surface (the
*  list parse, FLAGS, the VDP address, the 9901 programming, the parked warp
*  state, the fork) is differentially gated.
*
*  The JMP-$ timer idiom (authentic >13E2/>1404): each FSK half-cell is a
*  `JMP $` spin; every 9901 timer tick enters CASTIM, which — if the
*  interrupted instruction IS a `JMP $` (the CJMPS encoding) — INCTs the
*  saved PC past it (a hardware-timed single-step). During the edge HUNTS the
*  engines instead run with GPL R1's high byte set (>FF00), and CASTIM then
*  warps the PC to the resume address parked in GPL R6's cell (>83EC) — the
*  timeout/abort escape. The engines re-park R6 phase by phase.
*  ============================================================================
        AORG >1820

*  The cassette timer ISR (authentic >1404-1422). Entered from the >0900
*  ISR's FLAGS->20 fork, still on GPLWS (the R1 mode test), R12 = 0.
CASTIM  SBZ  0                      * I/O mode
        SBO  3                      * re-arm the timer interrupt
        MOV  R1,R1                  * GPL R1 < 0 -> the hunt phase: warp
        JLT  CASTW
        LWPI >83C0                  * the RTWP frame lives on INTWS
        C    *R14,@CJMPS            * interrupted at a JMP-$ half-cell wait?
        JNE  CASTW2
        INCT R14                    * step past it
        RTWP
CASTW   LWPI >83C0
CASTW2  MOV  @>83EC,R14             * warp to the parked resume (GPL R6)
        RTWP

CJMPS   DATA >10FF                  * the JMP-$ encoding the waits spin on
CV10    DATA >0010                  * FLAGS: the cassette-verify bit
CTOG    DATA >0300                  * SBZ 25 <-> SBO 25 (the mag-out toggle)
CFF     DATA >00FF                  * the polarity-memory mask
C21     BYTE >21                    * the record-phase >837C marker
        EVEN

*  ---- the shared setup (authentic >13BA) --------------------------------
*  R1 -> the operand list {byte-count word, VDP-address word}; R0 = the VDP
*  mode seed (0 = read source, >4000 = write dest); R3 = the timer word.
*  Leaves R5 = ceil(count/64) records, R10 = the VDP address; sets FLAGS >20,
*  kills the VDP interrupt, programs the 9901 half-cell timer, arms the
*  timer interrupt. R12 := 0 (the CRU base for the whole layer).
CASSU   MOV  *R1+,R5                * the byte count ->
        AI   R5,>003F
        SRL  R5,6                   * ... records of 64
        SOC  *R1,R0                 * | the VDP address
        MOV  R0,R10
        MOVB @>83E1,*R15            * set the VDP address (R0-low alias, MSB)
        CLR  R1
        CLR  R12
        MOVB R0,*R15
        SOC  @>0032,R14             * FLAGS |= >20: the timer-ISR fork on
        SBZ  2                      * the VDP interrupt off
        SBZ  12
        LDCR R3,15                  * timer mode + the interval...
        SBZ  0                      * ...and back to I/O mode
        SBZ  1
        SBO  3                      * the timer interrupt armed
        B    *R11

*  ---- function 4: write (authentic >1346) --------------------------------
*  The 768-zero-byte leader, a sync byte, the record count twice, then per
*  64-byte record (sent twice each): an 8-zero leader, sync, the bytes from
*  VDP, the additive checksum. Ends on a stepped JMP-$ into the teardown.
CASW    MOV  R7,R1                  * our driver parks the list in R7
        CLR  R0
        LI   R2,>0300
        LI   R8,>1E19               * the X-executed mag-out driver (SBZ 25)
        LI   R3,>0023               * timer mode + the write half-cell
        BL   @CASSU
        LI   R0,CASBIT
        LIMI >0001
CASWL   CLR  R4                     * the leader
        BL   *R0
        DEC  R2
        JNE  CASWL
        SETO R4                     * sync
        BL   *R0
        MOV  R5,R4                  * the record count, twice
        SWPB R4
        BL   *R0
        MOV  R5,R4
        SWPB R4
        BL   *R0
CASWR   CLR  R9                     * each record goes out twice (R9 toggles)
        LI   R2,>0008
CASWZ   CLR  R4                     * the record leader
        BL   *R0
        DEC  R2
        JNE  CASWZ
        SETO R4                     * the record sync
        BL   *R0
        MOVB @>83F5,*R15            * VDP read address := R10 (the R10-low
        LI   R2,>0040               * alias, then the MSB)
        MOVB R10,*R15
        CLR  R7
CASWB   CLR  R4
        MOVB @>FBFE(R15),R4         * the next VDP byte
        A    R4,R7                  * the checksum
        BL   *R0
        DEC  R2
        JNE  CASWB
        MOV  R7,R4                  * send the checksum
        BL   *R0
        INV  R9
        JNE  CASWR                  * the record's second pass
        AI   R10,>0040
        DEC  R5
        JNE  CASWR
CASWE   JMP  CASWE                  * the final half-cell, stepped past...
        B    @CASX                  * ...into the teardown

*  CASBIT: send R4's byte MSB-first, inverted (authentic >13E6), toggling
*  the mag-out line each half-cell; a 0 bit adds the mid-cell transition.
CASBIT  LI   R6,>0008
        INV  R4
CASB1   JMP  CASB1                  * half-cell
        X    R8                     * toggle the line
        XOR  @CTOG,R8
CASB2   JMP  CASB2                  * half-cell
        MOV  R4,R4
        JLT  CASB3
        X    R8                     * a 0 bit: the extra transition
        XOR  @CTOG,R8
CASB3   SLA  R4,1
        DEC  R6
        JNE  CASB1
        B    *R11

*  ---- functions 5/6: read / verify (authentic >142E/>1426) ---------------
CASV    SOC  @CV10,R14              * verify: FLAGS |= >10, VDP stays a source
        CLR  R0
        JMP  CASR2
CASR    SZC  @CV10,R14              * read: FLAGS &= ~>10...
        LI   R0,>4000               * ...and the VDP arms for writing
CASR2   MOV  R7,R1                  * our driver parks the list in R7
        LI   R3,>002B               * the read half-cell interval
        BL   @CASSU
        MOV  R10,R7                 * R7 := the VDP address (R10 is re-used
        CLR  R0                     *      as the helper-return forge below)
        MOVB @MASK20,@>837C         * the working preset (authentic >1442)
        JMP  CASRC                  * (the authentic's always-taken JHE)

*  The leader hunt + cell calibration (authentic >1448-14CA). GPL R1's high
*  byte marks the hunt phase for CASTIM; R6 parks the phase's warp target.
CASRS   LI   R8,>7530               * the edge-timeout budget
        LIMI >0001
        LI   R6,CASRHT              * a timer warp mid-hunt re-enters here
CASRH2  LI   R3,>002B
CASRHT  ANDI R1,>00FF
        DEC  R8
        JEQ  CASX                   * budget exhausted -> teardown (error)
        LI   R2,>0030               * the good-cell run: 48...
        MOV  R0,R0
        JNE  CASRH3
        A    R2,R2                  * ...or 96 when not writing to VDP
CASRH3  BL   @CASCEL                * one bit-cell
        JMP  CASRH4                 * (edge ok)
        JMP  CASRHT                 * (bad -> restart the hunt)
CASRH4  DEC  R2
        JNE  CASRH3
        LI   R9,>7FFF               * calibrate: load the timer long...
        LI   R8,>0008
        LDCR R9,15
        SBZ  0
        SBO  3
CASRC1  BL   @CASEDG                * ...ride 8 edges...
        JMP  CASRC2
        JMP  CASRC1
CASRC2  DEC  R8
        JNE  CASRC1
        SBO  0
        STCR R3,15                  * ...and read the elapsed count
        S    R3,R9
        MOV  R9,R3
        SLA  R9,2
        A    R9,R3
        SRL  R3,6                   * the measured half-cell interval
        ORI  R3,>0001
        LI   R10,CASRS0             * forge the cell helper's return
        CI   R3,>001F
        JLT  CASRH2                 * too fast = noise -> re-hunt
        B    @CASHNT                * hunt the sync edge with the new timer
CASRS0  BL   @CASCEL                * ride out the sync byte's leading 0s
        JMP  CASRS0
        LI   R2,>0007               * then its seven 1 bits
CASRS1  BL   @CASCEL
        JMP  CASRHT                 * (bad -> re-hunt)
        DEC  R2
        JNE  CASRS1
        LI   R6,CASRDW              * the data phase's warp target
        MOV  R0,R0
        JNE  CASRD                  * read mode -> straight to the record
*  The verify header (authentic >14CC-14EA): the tape's record count, twice.
CASRC   MOVB @C21,@>837C            * the record-phase marker
        MOV  R7,R0                  * R0 := the VDP cursor; R7 := the checksum
        CLR  R7
        BL   @CASBYT
        C    R5,R4                  * the tape claims fewer records? -> error
        JL   CASX
        MOV  R4,R5
        INC  R5
        NEG  R7
        BL   @CASBYT                * the count's second copy must balance
        JNE  CASX
        JMP  CASRN
*  The record checksum (authentic >14EC): the additive sum must return to 0.
CASRK   ANDI R7,>00FF
        NEG  R7
        BL   @CASBYT
        JEQ  CASRG
CASRDW  MOV  R5,R5                  * (also the data phase's warp target)
        JLT  CASX                   * failed both copies -> error out
        MOVB @>83E1,*R15            * re-address the VDP window...
        NEG  R5                     * ...mark "on the second copy"...
        MOVB R0,*R15
        JMP  CASRS                  * ...and hunt the record's second pass
CASRG   MOV  R5,R5                  * good: on the second copy already?
        JLT  CASRA
        LI   R2,>0049               * first copy good: time out the second
CASRG1  LI   R6,CASRG2              * (a warp mid-byte skips that byte)
        BL   @CASBYT
CASRG2  DEC  R2
        JNE  CASRG1
CASRA   AI   R0,>0040               * advance the VDP window
        MOVB @>83E1,*R15
        ABS  R5
        MOVB R0,*R15
CASRN   CLR  R7
        DEC  R5
        JNE  CASRS                  * the next record (leader + sync again)
        JMP  CASOK
*  The record body (authentic >152E): 64 bytes, written to VDP (read) or
*  compared against it (verify; a mismatch retries via the second copy).
CASRD   LI   R2,>0040
        CLR  R7
CASRD1  BL   @CASBYT
        SWPB R4
        COC  @CV10,R14              * verifying?
        JNE  CASRD2
        SB   @>FBFE(R15),R4         * compare the tape byte with the VDP byte
        JEQ  CASRD3
        CI   R5,>0001
        JEQ  CASRD3
        JMP  CASRDW                 * mismatch -> retry on the second copy
CASRD2  MOVB R4,@>FFFE(R15)         * read: the byte lands in VDP
CASRD3  DEC  R2
        JNE  CASRD1
        JMP  CASRK                  * -> the checksum byte
CASOK   MOVB @CZERO,@>837C          * success: the status clears
*  The teardown (authentic >155E): both cassette FLAGS bits off, the timer
*  interrupt off, the VDP interrupt back on, into the main loop.
CASX    SZC  @CV10,R14
        SZC  @>0032,R14
        SBZ  3
        SBO  12
        SBO  1
        SBO  2
        B    @LOOP

*  CASCEL: one FSK bit-cell (authentic >1572) — a stepped JMP-$ half-cell,
*  a mid-cell edge sample, then the polarity-conditional edge hunt with the
*  timer reloaded on the edge. Two-way return (the caller follows with a JMP
*  pair): a MID-CELL EDGE returns +0; none returns +2. R10 carries the
*  forged/derived return (the calibrator enters at CASHNT with R10 forged).
CASCEL  MOV  R11,R10
CASCW   JMP  CASCW                  * the stepped half-cell
        BL   @CASEDG
        INCT R10                    * (no mid-cell edge -> the +2 return)
        ORI  R1,>FF00               * the hunt phase: timer warps go to R6
CASHNT  CZC  @CFF,R1
        JEQ  CASHN2
CASHN1  TB   27                     * wait out the line at the old polarity
        JNE  CASHN3
        JMP  CASHN1
CASHN2  TB   27
        JNE  CASHN2
CASHN3  LDCR R3,15                  * reload the half-cell timer on the edge
        SBZ  0
        SBO  3
        ANDI R1,>00FF               * back to the stepped phase
        XOR  @CFF,R1                * flip the polarity memory
        B    *R10

*  CASBYT: assemble 8 FSK bits into R4 (a mid-cell edge = a 1), add the byte
*  into the running checksum R7; the caller's flags come from that add.
CASBYT  LI   R8,>0008
        CLR  R4
        MOV  R11,R9
*  (a mid-cell edge decodes as a 0 bit here — the writer inverts the byte,
*  so the senses cancel; no mid-cell edge = a 1)
CASBY1  SLA  R4,1
        BL   @CASCEL
        JMP  CASBY2
        INC  R4
CASBY2  DEC  R8
        JNE  CASBY1
        A    R4,R7
        B    *R9

*  CASEDG: sample the tape-in line against the polarity memory (R1 low).
*  Two-way return: +0 no change, +2 an edge (the polarity flips on the
*  no-change arcs exactly as the authentic >15BA block wires them).
CASEDG  TB   27
        JEQ  CASEG2
        CZC  @CFF,R1                * line high: memory low -> an edge
        JEQ  CASEG3
CASEG1  XOR  @CFF,R1
        B    *R11
CASEG2  CZC  @CFF,R1                * line low: memory low -> no change
        JEQ  CASEG1
CASEG3  INCT R11
        B    *R11

*  ============================================================================
*  MOVE (>20-3F) — the block-move family (M1 increment 3). Stream layout is
*  opcode, count, destination, source (execution-pinned in libre99-gpl's
*  m2_probe/move_probe). Opcode bits 001 G R V C N sit in R9's high byte:
*  N=imm-count, C=computed-GROM-source, V=RAM-source, R=VDP-register-dest,
*  G=non-GRAM-dest. All combinations are live (M4): source GROM-immediate /
*  computed-GROM (C=1, base + the >8300-indexed word) / CPU / VDP; destination
*  CPU / VDP / GRAM / VDP-register; count immediate or from a memory cell.
*  GROM sources (and GRAM destinations) are re-addressed per byte (P4) and the
*  interpreter's GROM position is saved before the copy and restored after, so
*  the next opcode is fetched from the instruction stream.
*  Registers across the copy: R2 count, R3 source addr, R4 source space
*  (0 CPU / 1 VDP / 2 GROM), R7 dest addr/reg-selector, R8 dest index
*  (0 CPU / 1 VDP / 3 reg), R5 source loader, R9 dest storer, R10 saved pos.
*  (Relocated from >1000 at M5 slice 1 — the FP interior claims it; now in
*  the deferred-M6 region.)
*  ============================================================================
        AORG >1E90

MOVEH   COC  @W1,R9                 * N: immediate count?
        JNE  MVCNTM
        BL   @ADDR16                * 16-bit immediate count -> R6
        MOV  R6,R2
        JMP  MVDST
MVCNTM  BL   @OPGET                 * count operand (a word from CPU/VDP RAM)
        BL   @LDR0W                 * word at (R3,R4) -> R0
        MOV  R0,R2

*  ---- destination ----
*  Destination decode order is G FIRST (authentic >063E peels G before R —
*  the 256-opcode sweep caught the reversed order): G=0 is a GRAM dest no
*  matter what R says; only G=1 forms consult R.
MVDST   COC  @MVG,R9                * G clear -> GRAM destination
        JNE  MVDGR
        COC  @MVR,R9                * G=1 + R: VDP-register destination?
        JNE  MVDNR
        CLR  R7
        MOVB *R13,R7                * starting register number (the >8000
        LI   R8,3                   * selector bit ORs in per byte — MDREG's
        JMP  MVSRC                  * >83D4 mirror keys off the raw first reg)
MVDNR   BL   @OPGET                 * dest GAS -> R3/R4
        MOV  R3,R7
        MOV  R4,R8                  * 0 CPU / 1 VDP
        JMP  MVSRC
*  G=0: GRAM destination — a 16-bit inline GRAM/GROM address (authentic `>0758`
*  G-clear path), stored via the GRAM storer with per-byte re-addressing (M4).
MVDGR   BL   @ADDR16                * 16-bit GRAM address -> R6
        MOV  R6,R7
        LI   R8,2                   * dest index 2 -> the GRAM storer (MDGRM)
        JMP  MVSRC

*  ---- source ----
MVSRC   COC  @MVV,R9                * V: RAM source (GAS)?
        JNE  MVSGR
        BL   @OPGET                 * src GAS -> R3/R4 (0 CPU / 1 VDP)
        JMP  MVGO
MVSGR   COC  @MVC,R9                * C: computed-GROM source?
        JEQ  MVSCP
        BL   @ADDR16                * 16-bit GROM source address -> R6
        MOV  R6,R3
        LI   R4,2                   * src index 2 -> the GROM loader
        JMP  MVGO
*  C=1: the computed-GROM source is a 16-bit inline base address *plus* an
*  indexed offset (the >8300-indexed word, authentic `>0758`->`>077E`) — the
*  same index mechanism as OPGIDX. (Vs the C=0 bare inline address.) M4.
MVSCP   BL   @ADDR16                * 16-bit base GROM address -> R6
        MOV  R6,R3
        BL   @OPGIDX                * R3 += the >8300-indexed word
        LI   R4,2                   * src index 2 -> the GROM loader
        JMP  MVGO

*  ---- copy loop ----
MVGO    MOV  R4,R1                  * source loader := MVTAB[src*2]
        SLA  R1,1
        MOV  @MVTAB(R1),R5
        MOV  R8,R1                  * dest storer := MVTAB[(3+dst)*2]
        AI   R1,3
        SLA  R1,1
        MOV  @MVTAB(R1),R9          * (opcode is no longer needed)
        CI   R4,2                   * GROM source, or...
        JEQ  MVSAVE
        CI   R8,2                   * ...GRAM dest? both drive the GROM addr counter
        JNE  MVLP
MVSAVE  BL   @GPCRD                 * -> save the interpreter's fetch position
        DEC  R6
        MOV  R6,R10
MVLP    MOV  R2,R2
        JEQ  MVEND
        BL   *R5                    * load one byte -> R0, advance the source
        BL   *R9                    * store one byte, advance the destination
        DEC  R2
        JMP  MVLP
MVEND   CI   R4,2                   * GROM source, or...
        JEQ  MVRST
        CI   R8,2                   * ...GRAM dest? -> restore the fetch position
        JNE  MVRET
MVRST   MOV  R10,R6
        BL   @GSETA
MVRET   B    @LOOP

*  ---- per-space byte loaders: R0 := next source byte, advance the cursor ----
MSCPU   MOVB *R3+,R0
        B    *R11
MSVDP   MOV  R11,R12
        BL   @VRD                   * VDP read address := R3
        CLR  R0
        MOVB @>8800,R0
        INC  R3
        B    *R12
MSGROM  MOV  R11,R12
        MOV  R3,R6                  * per-byte GROM re-addressing (P4)
        BL   @GSETA
        CLR  R0
        MOVB *R13,R0
        INC  R3
        B    *R12

*  ---- per-space byte storers: store R0, advance the cursor ----
MDCPU   MOVB R0,*R7+
        B    *R11
MDVDP   MOV  R11,R12
        BL   @VWR                   * VDP write address := R7
        MOVB R0,@>8C00
        INC  R7
        B    *R12
*  GRAM storer: write R0 to the GRAM address R7 via the GROM write-data port
*  >9C00, re-addressing per byte (the GROM address counter is shared, so each
*  byte re-sets it — like the GROM source loader; authentic `>0686`). M4.
MDGRM   MOV  R11,R12
        MOV  R7,R6
        BL   @GSETA                 * set the GROM/GRAM address = R7
        MOVB R0,@>0400(R13)         * write the byte to >9C00 (GRAM)
        INC  R7
        B    *R12
*  VDP-register storer. A MOVE STARTING at register 1 mirrors its FIRST byte
*  into >83D4 (the ISR's R1 copy, read by the screen-timeout blank/unblank);
*  only the pre-ORI selector can match — a multi-register MOVE passing
*  through R1 mid-copy does NOT mirror (authentic >0698: the CB fires before
*  the per-byte ORI >80 — RECON §26). With FLAGS >08 (16K) the >80 mode bit
*  is forced into the value first — the copy AND the register get it.
MDREG   CI   R7,>0100               * the raw starting register 1?
        JNE  MDRGO
        COC  @>0012,R14             * FLAGS >08 (16K)? (the NASTY word >0008)
        JNE  MDRG1
        ORI  R0,>8000               * force the 16K bit
MDRG1   MOVB R0,@>83D4              * the ISR's R1 copy
MDRGO   ORI  R7,>8000               * the >80|reg selector (idempotent)
        MOVB R0,*R15                * value byte to the VDP write-address port
        MOVB R7,*R15                * then the selector
        AI   R7,>0100              * advance to the next register
        B    *R11

*  ============================================================================
*  IO (>F4-F7) — GPL CRU / sound / cassette I/O. The driver delivers the
*  function through the uniform imm/mem source parse (RECON §25): R0 = the
*  function value, R5 = the flag copy, R7 = the destination GAS address (the
*  list pointer). Byte-form functions arrive high-justified (our internal
*  convention) — normalized here to the authentic right-justified view.
*  Functions 0/1 (sound-list arming), 2 (CRU input) and 3 (CRU output) are
*  live; 4/5/6 (cassette) are hardware-gated. Functions >= 7 are undefined:
*  the authentic indexes past >0CEC into the XML master table and executes it
*  as code — we keep the loud stub for diagnosability (documented divergence
*  on garbage input, RECON §25). Relocated to >1346 (the old cassette-span
*  head) at M5 slice 1 — ROUND1 claims >0F54.
        AORG >17FC

IOH     MOV  R0,R6
        COC  @W1,R5                 * word-form functions arrive right-justified
        JEQ  IOHRJ
        SRL  R6,8                   * byte form: right-justify (>= >80 hits the
IOHRJ   MOV  R6,R0                  * stub where the authentic garbage-dispatches)
        CI   R6,>0007
        JHE  IOFST
        SLA  R6,1
        MOV  @IOTAB(R6),R1
        B    *R1
IOFST   B    @STUB

*  CRU input (function 2) / output (function 3): the destination GAS points at
*  the list { CRU-address word, count byte, data-address byte } (RECON §25).
*  The authentic engine synthesizes the exact LDCR/STCR the list describes and
*  X-executes it, so the transfer semantics are the 9900's own: the count is
*  the 4-bit field (0 -> 16); count <= 8 accesses the BYTE at >8300+data-addr,
*  count > 8 the WORD (odd address -> the even pair); bits move LSB-first over
*  consecutive CRU addresses; STCR zero-fills the rest. We synthesize the same
*  instruction from LI skeletons — interface-identical by construction. The
*  console boot uses `IO @>8302,#3` (address 2, count 1, the byte-imm form
*  F6 02 03) to arm the 9901 VDP interrupt.
*  (Split placement at M5: the three IO bodies are independently
*  table-referenced, so they pack the exact-size pockets the layout leaves.)
        AORG >1AC8
IOCRIN  LI   R9,>3412               * STCR *R2 skeleton (function 2: CRU -> pad)
        JMP  IOCR
IOCROUT LI   R9,>3012               * LDCR *R2 skeleton (function 3: pad -> CRU)
IOCR    MOV  R7,R1                  * R1 -> the list
        MOV  *R1+,R12               * CRU bit address...
        A    R12,R12                * ...doubled into R12 (the 9900 convention)
        CLR  R2
        MOVB *R1+,R2                * count byte ->
        ANDI R2,>0F00               * the 4-bit LDCR/STCR count field (0 -> 16)
        SRL  R2,2                   * -> bits 9-6
        SOC  R2,R9
        CLR  R2
        MOVB *R1,R2                 * data-address offset byte ->
        SWPB R2
        AI   R2,>8300               * the >8300-based data cell
        X    R9                     * run the synthesized transfer
        B    @LOOP

*  Sound (functions 0/1): arm a sound list for the VBLANK ISR (authentic >05D6,
*  shared by both functions). The operand cell holds the list's GROM (function 0)
*  or VDP (function 1) address; store it in >83CC/D, select the source in FLAGS
*  bit 0 (the function's low bit — R0 is still the function code), and set the
*  countdown >83CE = SPEED so the ISR drains the first block next tick (§6).
        AORG >1FE8
IOSND   ANDI R14,>FFFE              * clear FLAGS bit 0 (the sound source select)
        SOC  R0,R14                * function bit 0 -> FLAGS bit 0 (0 = GROM, 1 = VDP)
        MOV  *R7,@>83CC             * >83CC/D = the sound-list pointer (word at the operand)
        MOVB R14,@>83CE             * >83CE = SPEED (R14 high) -> drain on the next tick
        B    @LOOP

*  ============================================================================
*  XML (>0F) — the table-of-tables dispatch + the device-linkage routines
*  (M1 increment 3e). Operand >XY: X picks a table pointer from the master
*  table (>0CFA), Y the entry word; XML calls the routine at that word (XMLLNK
*  via BL). Table F (>8300) is the XML >F0 ML-launch vector. The index math
*  reads the operand into a cleaned register (R9's low byte is opcode scratch —
*  the same hazard the >0270 SPEC fix addressed, RECON §20).
*  ============================================================================
        AORG >12A0
*  XML table 1 / XTAB (>12A0, authentic home): 12 words, >10-1B. The
*  conversions (>10-12) and SROM/SGROM (>19/>1A) are live; the symbol/
*  value-stack entries (>13-18, >1B) are deferred-M6 loud stubs. The
*  vestigial >1C-1F entries index PAST the table into CFI's first code
*  words — byte-identical to the authentic's (C120 834A 1342 04C0, the
*  zero exit pinned at >1342), so the accident reproduces exactly (RECON §27).
XTAB    DATA CSN,CSNGR,CFI,STUB       * >10 CSN >11 CSNGR >12 CFI >13
        DATA STUB,STUB,STUB,STUB      * >14 >15 >16 >17
        DATA STUB,SROM,SGROM,STUB     * >18 VPOP >19 SROM >1A SGROM >1B PGMCH

*  (XMLH relocated from >1200 at M5 slice 1 — the conversion package's
*  interior claims the >11A2-12B7 span.)
        AORG >1FB8
XMLH    CLR  R4
        MOVB *R13,R4                * operand >XY -> >XY00
        SRL  R4,8                   * -> >00XY (low byte clean)
        MOV  R4,R9
        ANDI R9,>00F0               * X in bits 7..4
        SRL  R9,3                   * -> X * 2  (master-table byte offset)
        ANDI R4,>000F               * Y
        SLA  R4,1                   * -> Y * 2
        A    @XMASTER(R9),R4        * master[X] + Y*2 = the entry address
        MOV  *R4,R4                 * -> the routine address
        BL   *R4                    * call it (XMLLNK)
        B    @LOOP                  * (routines that B @SOFT re-enter directly)

*  ============================================================================
*  The conversion package's character fetchers (free-placed in the SGROM-body
*  span — RECON §27; the authentic pair lives at >1FC8/>1FDA). Both keep the
*  text cursor in R6, return the character right-justified in R8, advance R6,
*  and RT — the callers park their real return in R9/R10 around BL *R3.
*  ============================================================================
        AORG >0B28
*  The VDP fetcher: re-latch the read address every character (LSB, a settle
*  jump, MSB), then one data-port read.
CSNVF   MOVB @>83ED,*R15            * the address LSB (R6's low-byte alias)
        JMP  CSNVF2                 * (the authentic's deliberate settle)
CSNVF2  MOVB R6,*R15                * the MSB — read mode
        INC  R6
        MOVB @>8800,R8
CSNVF3  SRL  R8,8
        B    *R11
*  The GROM fetcher: address MSB then LSB to >9C02, one >9800 read, then the
*  shared right-justify tail. (In GROM mode the text cursor IS the GPL PC —
*  re-aiming afterwards is the caller's job, exactly as the authentic.)
CSNGF   MOVB R6,@>0402(R13)
        MOVB @>83ED,@>0402(R13)
        INC  R6
        MOVB *R13,R8
        JMP  CSNVF3

*  ============================================================================
*  The number-conversion package (RECON §27; the fp-recon-conv dossier).
*  CSNGR (XML >11) / CSN (XML >10): string -> floating point. The text lives
*  in VDP RAM at the address in >8356 (CSNGR reads GROM instead when >8389 is
*  non-zero). Grammar: [+|-] zeros* digits* [. frac] [E[+|-]digits], uppercase
*  E, no blanks. Two passes over the device text: pass 1 classifies (sign,
*  the digit-count/leading-zero exponent adjust DADJ, the first-significant-
*  digit address, the explicit exponent via the shared decimal reader), pass
*  2 re-fetches the digits and builds EIGHT radix-100 bytes at >834B-8352
*  (the 8th is the round guard), paired by the parity of E10+128. Exits: the
*  >0F56 finisher (round/pack/status); a silent +0 for anything without a
*  significant digit (the ROM never reports syntax errors — callers compare
*  >8356); a FULL ABORT for 'E' with no digits (nothing written but >8375);
*  the huge-exponent paths (E- -> 0, E+ -> the ±9.9..E127 saturation + >01).
*  ============================================================================
        AORG >1158
CNV50   BYTE >32                    * the CFI tie threshold (authentic >1158)
CNVE3   BYTE >03                    * the CFI overflow code  (authentic >1159)
*  The unsigned decimal reader (the E-exponent): R3 = the fetcher, R6 = the
*  cursor; returns R4 = the value with the terminator consumed and pushed
*  into R8-30. No digits at all = the caller's FULL ABORT (B *R10). A value
*  overflowing 15 bits switches to huge-exponent mode: keep consuming digits,
*  then resolve through the over/underflow finisher.
CNVDEC  CLR  R4
        CLR  R0
        MOV  R11,R9                 * park (BL *R3 clobbers R11)
        JMP  CNVDF
CNVDA   MPY  @F10W,R4               * R4:R5 := 10 * the accumulator
        MOV  R4,R4
        JNE  CNVDH                  * >= 65536 -> huge mode
        INC  R0
        A    R8,R5
        MOV  R5,R4
        JLT  CNVDH                  * >= 32768 -> huge mode
CNVDF   BL   *R3
        AI   R8,>FFD0               * - '0'
        CI   R8,>000A
        JL   CNVDA
        MOV  R0,R0                  * no digits at all?
        JEQ  CNVABT                 * -> the full abort
        B    *R9
CNVDH   LI   R9,CNVHUG              * huge: redirect the completion...
        JMP  CNVDF                  * ...and keep consuming digits
CNVZRO  B    @FPZERO                * the shared zero exit
CNVABT  B    *R10                   * the full abort: nothing written
CNVHUG  DEC  R6                     * huge-exponent completion: push back
        MOV  R6,@>8356              * the terminator, update the pointer
        C    R12,R2                 * any significant mantissa digits?
        JEQ  CNVZRO                 * no -> zero
        MOV  R1,@>8376              * the E-sign flag word (>0000/>FFFF)
        B    @OVBODY                * E- -> 0; E+ -> saturation + error >01

*  CSNGR (XML >11): the source select, then the shared body.
        AORG >11A2
CSNGR   MOVB @>8389,R3
        JEQ  CSN
        LI   R3,CSNGF               * GROM text
        JMP  CSNB
*  CSN (XML >10): VDP text always.
        AORG >11AE
CSN     LI   R3,CSNVF
CSNB    MOV  R11,R10                * the XML return (R11 dies on BL *R3)
        MOV  @>8356,R6
        BL   *R3                    * the first character
        CLR  R7                     * the sign flag
        MOV  R6,R2                  * the no-chars-consumed reference mark
        CI   R8,>002B               * '+'
        JEQ  CSNSGN
        CI   R8,>002D               * '-'
        JNE  CSNZT                  * no sign: zero-test THIS character
        SETO R7
CSNSGN  INC  R2
CSNZF   BL   *R3
CSNZT   CI   R8,>0030               * skip (and don't count) leading zeros
        JEQ  CSNZF
        MOVB R7,@>8375              * the sign byte, on every path from here
        MOV  R6,R12
        DEC  R12                    * R12 -> the first significant char
        SETO R7                     * R7 = DADJ, starting at -1
        JMP  CSNIT
CSNIL   INC  R7                     * the integer-digit loop
        BL   *R3
CSNIT   CI   R8,>0030
        JL   CSNDOT
        CI   R8,>0039
        JLE  CSNIL
CSNDOT  CI   R8,>002E               * '.' ?
        JNE  CSNTRM
        INC  R2                     * count the '.' in the mark
        MOV  R7,R7                  * integer digits seen?
        JLT  CSNFZ0
        JMP  CSNFRC
CSNFZL  DEC  R7                     * leading fractional zeros charge DADJ
CSNFZ0  BL   *R3
        CI   R8,>0030
        JEQ  CSNFZL
        DEC  R6                     * push back the non-zero...
        MOV  R6,R12                 * ...it is the first significant digit
CSNFRC  BL   *R3                    * the fractional-digit loop
        CI   R8,>0030
        JL   CSNFEN
        CI   R8,>0039
        JLE  CSNFRC
CSNFEN  C    R6,R2                  * nothing at all consumed? (".", "+.")
        JEQ  CNVZRO                 * -> zero, >8356 NOT updated
CSNTRM  MOV  R6,R2
        CLR  R4                     * the explicit exponent
        DEC  R2                     * R2 = the terminator address
        CLR  R1                     * the E-sign flag
        CI   R8,>0045               * 'E' ?
        JNE  CSNEXP
        BL   *R3
        CI   R8,>002B               * 'E+'
        JEQ  CSNED
        CI   R8,>002D               * 'E-'
        JNE  CSNEPB
        DEC  R1
        JMP  CSNED
CSNEPB  DEC  R6                     * neither: push back for the reader
CSNED   BL   @CNVDEC                * the exponent digits -> R4
        MOVB R1,R1
        JEQ  CSNEXP
        NEG  R4
CSNEXP  DEC  R6                     * push back the terminator
        MOV  R6,@>8356              * the pointer-at-exit contract
        C    R12,R2                 * zero significant digits?
        JEQ  CNVZRO                 * ("0", "0.0", "E5", garbage) -> zero
        AI   R4,>0080               * the exponent/alignment math:
        CLR  R1                     * R4 = E10 + 128 (E10 = exp + DADJ)
        A    R7,R4
        MOV  R4,R7
        SRA  R4,1                   * the biased radix-100 exponent
        MOV  R4,@>8376
        SRC  R7,1                   * R7 bit 15 := the pairing parity
        LI   R5,>0008               * eight output bytes (7 + the guard)
        LI   R0,>834B
        MOV  R12,R6                 * ---- pass 2: re-read the digits ----
CSNP2   C    R6,R2
        JEQ  CSNFLU                 * the terminator position -> flush/pad
        BL   *R3
        CI   R8,>002E               * the single '.' inside skips
        JEQ  CSNP2
        AI   R8,>FFD0
        INV  R7                     * toggle the pairing parity
        JLT  CSNP2B
        MPY  @F10W,R8               * first of a pair: R1 high := 10*digit
        MOVB @>83F3,R1              * (the R9-low alias carries it up)
        JMP  CSNP2
CSNP2B  AB   @>83F1,R1              * second of a pair: += the digit (R8-low)
CSNFLU  MOVB R1,*R0+                * store the radix-100 byte
        CLR  R1
        DEC  R5
        JNE  CSNP2                  * (exhausted text zero-pads)
        B    @RND1B                 * the round/pack/status finisher

*  CFI (XML >12): floating -> the signed 16-bit integer at >834A, rounding
*  to nearest with exact halves toward +infinity; range -32768..+32767,
*  overflow = >8354 := >03 (via the CNVE3 byte) with no result stored and
*  the first word left ABS'd. The FIRST FOUR WORDS double as XML table 1's
*  vestigial >1C-1F entries (the index-past-the-table accident, RECON §8) —
*  C120 834A 1342 04C0, byte-identical by construction: the zero exit is
*  pinned at >1342 so the JEQ encodes exactly >1342.
        AORG >12B8
CFI     MOV  @>834A,R4              * (= >C120 >834A — the vestigial words)
        JEQ  CFIRT                  * zero in, zero out (= >1342)
        CLR  R0                     * (= >04C0)
        LI   R2,>834B
        CLR  R3
        ABS  @>834A
        CLR  R5
        MOVB @>834A,R5              * the exponent byte
        CI   R5,>3F00
        JLT  CFIST0                 * |x| < 0.01 -> 0
        JEQ  CFITIE                 * 0.01 <= |x| < 1 -> magnitude 0
        CI   R5,>4100
        JLT  CFID1                  * one integer digit
        JEQ  CFID2                  * two
        CI   R5,>4200
        JH   CFIOVF                 * |x| >= 10^6 -> overflow
        MOVB *R2+,@>83E1            * three: digit -> R0 (the low alias)
        MPY  @CNV100,R0
        MOV  R1,R0
CFID2   MOVB *R2+,@>83E7            * digit -> R3
        A    R3,R0
        MPY  @CNV100,R0
        MOV  R0,R0                  * the product high nonzero -> overflow
        JNE  CFIOVF
        MOV  R1,R0
        JLT  CFIOVF                 * >= 32800 before the last digit
CFID1   MOVB *R2+,@>83E7
        A    R3,R0                  * the final magnitude (32768 reachable)
CFITIE  CB   *R2+,@CNV50            * the first fractional pair vs 50
        JLT  CFINRD
        JGT  CFIRUP
        MOV  R4,R4                  * exactly 50: the original sign decides
        JGT  CFIRUP                 * positive -> up (ties toward +inf)
CFISCN  MOVB *R2+,R3                * negative: any residue -> not a tie
        JNE  CFIRUP
        CI   R2,>8352
        JL   CFISCN
        JMP  CFINRD                 * a true tie, negative -> truncate
CNV100  DATA >0064                  * one hundred, embedded mid-stream like
CFIRUP  INC  R0                     * the authentic's >1320
CFINRD  CI   R0,>8000               * the range check
        JL   CFISGN
        JH   CFIOVF
        MOV  R4,R4                  * exactly 32768: only -32768 is legal
        JLT  CFINEG
CFIOVF  MOVB @CNVE3,@>8354          * overflow: error >03, nothing stored
        B    *R11
CFISGN  INV  R4
        JLT  CFIST0
CFINEG  NEG  R0
CFIST0  MOV  R0,@>834A              * the 16-bit result over FAC 0-1
CFIRT   B    *R11
*  peripheral cards' >4000 ROMs for a matching DSR / subprogram / power-up chain
*  entry and CALL each match. Inputs (set by the GPL caller): the chain offset
*  >836D (>04 power-up / >06 program / >08 DSR / >0A subprogram), the search-name
*  length >8355 (0 = match every node, as the boot power-up scan does) and text
*  >834A..; the resume cursor >83D0 (0 = start a fresh CRU scan; non-0 = the card
*  base to resume this card's chain from the node saved in >83D2). Outputs: found
*  count R1, the current node in >83D2, the found card base (or 0) in >83D0, the
*  condition bit. Each match is called via `BL *R9` with R12 = the card's CRU
*  base and its ROM enabled, interrupts already masked by the interpreter's
*  per-instruction LIMI 0 — the DSR-call invariants (RECON §10/§24). Fits the
*  authentic 100-byte home directly; the name compare (SNAME) is shared with
*  SGROM out in free space. NASTY >000D = the >AA card marker.
        AORG >0AC0
SROM    CLR  R1                     * found count := 0
        MOV  @>83D0,R12            * R12 = the resume cursor (a found card base)
        JNE  SRWALK                * non-0 -> resume this card's chain at >83D2
        LI   R12,>0F00             * fresh scan: the CRU base just before >1000
SRSCAN  MOV  R12,R12
        JEQ  SRSEL                 * first pass -> nothing to turn off yet
        SBZ  0                     * disable the previous card's ROM
SRSEL   AI   R12,>0100             * advance to the next card CRU base
        CLR  @>83D0
        CI   R12,>2000             * scanned past >1F00?
        JEQ  SRNONE                * yes -> not found
        MOV  R12,@>83D0            * remember which card we're at
        SBO  0                     * enable this card's ROM at >4000
        LI   R2,>4000
        CB   *R2,@>000D            * a valid card? (header byte == >AA marker)
        JNE  SRSCAN                * no -> next base
        AB   @>836D,@>83E5         * R2-low += chain offset -> >4000+off  (>83E5 = GPLWS R2 low)
        JMP  SRLINK                * enter the chain walk at the follow-pointer step
SRWALK  MOV  @>83D2,R2             * resume: R2 = the last node we processed
        SBO  0                     * re-enable the card's ROM
SRLINK  MOV  *R2,R2                * follow the pointer -> the next node
        JEQ  SRSCAN                * a zero link ends this card's chain -> next base
        MOV  R2,@>83D2             * save the current node (the resume point)
        INCT R2                    * -> the node's routine-address field
        MOV  *R2+,R9               * R9 = the DSR / power-up routine; R2 -> name field
        BL   @SNAME                * compare the search name (a match skips the JMP)
        JMP  SRWALK                * name mismatch -> keep walking the chain
        INC  R1                    * a match: bump the found count
        BL   *R9                   * call it (R12 = CRU base, ROM on, LIMI 0)
        JMP  SRWALK                * a plain return -> continue the chain
*  The DSR skip-return (authentic >0B16): a routine that HANDLED the request
*  returns to R11+2. Turn the card ROM off, pop the GPL DSRLNK's CALL frame —
*  the interpreter resumes at DSRLNK's caller, short-circuiting its error
*  tail — and clear the condition bit through the soft entry. >83D0 keeps the
*  found base (the authentic leaves it; a later stale-resume search walks
*  ROM-as-GROM, a documented garbage accident — RECON §26).
        SBZ  0
        BL   @GPOP
        B    @SOFT
SRNONE  B    @SOFT                 * not found: clear cond + re-enter the loop

*  SGROM (XML >1A, pinned home >0B24): the GROM-header service search — the
*  console's power-up / program / DSR / subprogram walk over the standard GROM
*  headers, the GROM-side twin of SROM. Trampolines to SGROMB in free space (P8:
*  the body is bigger than the authentic 100-byte slot once SNAME is hoisted out,
*  and no software enters SGROM's interior — ENTRY-CENSUS). Full spec RECON §24.
        AORG >0B24
SGROM   B    @SGROMB

*  ============================================================================
*  Device-linkage bodies in free space (the M5/M6 conversion/BASIC region, not
*  yet claimed — layout-ledger debt, relocated when M5 lands). SNAME is the
*  DSR-name comparator shared by SROM (card-ROM nodes, R2 < >9800) and SGROM
*  (GROM nodes read through the port at R2 = >9800). It compares the search name
*  (length >8355, text >834A..) against a node's name; a length of 0 matches
*  every node (the power-up scan). On a match it does INCT R11 so the caller
*  falls through its "keep walking" JMP into the call; otherwise it returns to
*  that JMP. The R2>=>9800 test suppresses the pre-compare INC for a GROM source
*  (the port auto-advances on each read) — for a card pointer the INC always
*  fires, stepping past the node's length byte to its first name character.
*  (Relocated from >1300 at M5 slice 1 — the conversion package claims it.)
*  ============================================================================
        AORG >17D4
SNAME   MOVB @>8355,R5             * R5 (high byte) = the search-name length
        JEQ  SNMTCH                * length 0 -> match every node (power-up scan)
        CB   R5,*R2                * compare with the node's own name-length byte
        JNE  SNRET                * differs -> not a match
        SRL  R5,8                 * R5 = the length as a loop count
        LI   R6,>834A             * R6 -> our staged search-name buffer
SNCHR   CI   R2,>9800             * a GROM port source?
        JHE  SNCMP                * yes -> it auto-advanced; don't INC
        INC  R2                   * a card pointer -> step to the next char
SNCMP   CB   *R6+,*R2             * compare a name character
        JNE  SNRET
        DEC  R5
        JNE  SNCHR
SNMTCH  INCT R11                  * matched -> skip the caller's keep-walking JMP
SNRET   B    *R11

*  ============================================================================
*  SGROMB — the SGROM (>0B24) body: the console's GROM-header power-up / program
*  / DSR / subprogram walk. It scans the eight GROM bases (>E000 down to >0000,
*  stepping by -->2000), and at each base whose >AA header is present it follows
*  the header's chain field (at the >836D offset) node by node, SNAME-matching
*  each. A match pushes the found routine (or, for a program search, the node)
*  onto the GPL data stack and re-enters the interpreter with the condition bit
*  set, so PUSCAN's GPL loop runs it; a full pass with no more matches steps the
*  port cursor (>83D0) and, for the un-named power-up scan, returns with >83D0
*  still set so PUSCAN re-enters — the authentic 16-iteration walk arises from
*  the >9800..>9840 cursor sweep. All GROM addressing rides the R1/R3/R9 low-byte
*  GPLWS aliases (>83E3/>83E7/>83F3) exactly as the authentic ROM does. RECON §24.
*  (Relocated from >1340 at M5 slice 1; now in the deferred-M6 region.)
*  ============================================================================
        AORG >1346
*  Harvested constants, pinned as explicit named data (P8): the authentic ROM
*  reads these as its own instruction words (@>0128 = the ANDI mask, @>0C04 = a
*  DEC opcode byte, @>0030 = the reset LI-R0 opcode); our packed bodies differ
*  there, so we name the VALUES rather than harvest an accidental encoding.
GR13M   DATA >1FFF                 * low-13-bit mask: is R1 a clean GROM base? (@>0128)
PGMKEY  BYTE >06                   * the program-list offset key (@>0C04 byte)
DSTK2   BYTE >02                   * GPL data-stack slot size = one word (@>0030 hi)
        EVEN
SGROMB  LI   R7,>83D2              * R7 -> the entry/link cursor cell
        LI   R8,>83D0              * R8 -> the base/port cursor cell
        BL   @GPUSH                * save the interpreter's GROM fetch position
SGLOOP  MOV  *R7,R1                * R1 = the saved link (>83D2)
        MOV  *R8,R2                * R2 = the saved cursor (>83D0)
        JNE  SGBASE                * resuming mid-walk -> R2 is the live port cursor
        LI   R2,>9800             * fresh: R2 = the GROM data port
SGFRST  LI   R1,>E000             * R1 = the first GROM base to scan
SGBASE  CZC  @GR13M,R1            * is R1 a clean base (no low-13 bits set)?
        JNE  SGHDR2               *   no (a live link) -> skip the marker check
        MOV  R2,*R8              * remember the port cursor
        MOVB R1,@>0402(R2)       * GROM read address = base, high byte (R2+>0402 = write port)
        MOVB @>83E3,@>0402(R2)   * base low byte (>83E3 = GPLWS R1 low)
        AB   @>836D,@>83E3       * R1 low += the chain offset (>836D) for the next read
        MOVB R1,@>83CB          * stash the base high byte for the step
        CB   *R2,@>000D         * a valid header here? (marker byte == >AA)
        JNE  SGNEXT             *   no -> the next base
SGHDR2  MOVB R1,@>0402(R2)      * set the GROM address to the chain field (base+offset)
        MOVB @>83E3,@>0402(R2)
        SLA  R10,4              * a short settle before the GROM read (authentic)
        MOVB *R2,R3            * link word, high byte
        JMP  SGL1             * (prefetch-settle delay slot)
SGL1    MOVB *R2,@>83E7        * link low (>83E7 = GPLWS R3 low) -> R3 = the link
        MOV  R3,*R7           * save the link -> >83D2
        JEQ  SGNEXT           * a zero link ends this chain -> the next base
        INCT R3               * -> the node's routine-address field
        MOVB R3,@>0402(R2)    * set the GROM address = the routine field
        MOVB @>83E7,@>0402(R2)
        JMP  SGL2            * (settle)
SGL2    MOVB *R2,R9           * routine high
        SLA  R10,4            * settle
        MOVB *R2,@>83F3       * routine low (>83F3 = GPLWS R9 low) -> R9 = the routine
        BL   @SNAME           * compare the search name (a match skips the JMP)
        JMP  SGLOOP           * mismatch -> the next chain entry
        AB   @DSTK2,@>8372    * a match: make room on the GPL data stack (ptr += 2)
        AB   R14,@>836C       * advance the FP-error GROM cell by SPEED (authentic bookkeeping)
        MOVB @>8372,R4        * R4 = the bumped data-stack pointer
        SRL  R4,8
        DECT R3               * R3 -> back at the node (a program search pushes this)
        CB   @>836D,@PGMKEY   * a program-list search (offset == >06)?
        JNE  SGPUSH
        MOV  R3,R9           *   yes -> push the node pointer, not the routine
SGPUSH  MOVB R9,@>8300(R4)    * push the found address (high) onto the data stack
        MOVB @>83F3,@>8301(R4) * ... and its low byte (>83F3 = GPLWS R9 low)
        MOV  R2,R13          * hand the interpreter the GROM port at the found position
        BL   @GPOP           * restore the saved fetch position (balances the entry GPUSH)
        SOCB @MASK20,@>837C  * set the GPL condition bit (a routine was found)...
        B    @LOOP           * ...and re-enter the interpreter to run the pushed routine
SGNEXT  CLR  R1               * advance to the next GROM base
        MOVB @>83CB,R1       * R1 = the saved base high
        AI   R1,>E000        * next base = base - >2000 (>E000 steps downward, wraps)
        MOV  R1,*R7          * park it in >83D2
        CI   R1,>E000        * wrapped all the way back to >E000?
        JNE  SGBASE          *   no -> scan this base
        C    *R2+,*R2+       * a full pass done: step the port cursor by 4 (two reads)
        MOV  R2,*R8          * >83D0 = the port cursor
        CI   R2,>9840        * swept the whole >9800..>9840 window (16 passes)?
        JEQ  SGDONE          *   yes -> the walk is complete
        MOVB @>8355,R5       * a search with a name (length != 0)?
        JNE  SGFRST          *   yes -> restart the base scan for the next pass
        JMP  SGREXIT         *   power-up (no name): return, >83D0 still set to resume
SGDONE  CLR  *R8             * clear the cursor -> PUSCAN's PUDONE sees "done"
SGREXIT BL   @GPOP           * restore the interpreter's GROM position
        B    @SOFT           * clear cond + re-enter the loop

*  ============================================================================
*  KSCAN body — the keyboard/joystick scanner, all modes (spec: KSCAN-SPEC.md).
*  Mode dispatch (>8374): 0 full scan; 1/2 left/right split + joystick 1/2;
*  3-5 select the translation state then full-scan. The keyboard scan walks
*  columns 5..0 via the CRU (select @>0024 bits 18-20, read @>0006 bits 3-10
*  active-low), first-key-wins, computes raw = col*8 + (7-row), captures the
*  column-0 modifiers, applies the split mask (>0FFF/>F0FF), picks the GROM
*  translation base — >17C0 for a split unit, else by modifier priority
*  (CTRL>FCTN>SHIFT>none) — reads the character from GROM into >8375, debounces
*  against >83C8(unit) (condition bit only on a code change), normalizes full-scan
*  results per the >83C6 translation state (state 0 folds a-z; states 1/2 read the
*  alpha-lock switch — RECON §23), and un-blanks on a new key. Modes 1/2 also
*  read the joystick deflections into >8376/>8377 from the GROM table at >16E0.
*  Temporarily placed in the free tail (the M6 BASIC region, not yet claimed) —
*  like Zone H sits in the future FP region; relocate when BASIC lands.
*  ============================================================================
        AORG >1B00
KSCANB  MOV  R11,@>83D8             * save the caller's return
        BL   @GPUSH                 * save the interpreter's GROM position
        CLR  R12
        SBO  21                     * select the alpha-lock line (P5)
        MOVB @>8374,R5              * the mode byte
        SRL  R5,8
        MOV  R5,R6                  * R6 = working copy of the mode
        JEQ  KSFULL                 * mode 0 -> full keyboard scan (R0=0, R5=0)
        LI   R0,>0FFF               * mode 1: left split (keep rows 4-7)
        DEC  R6
        JEQ  KSJOY                  * mode 1 -> joystick 1 + left split (unit R5=1)
        LI   R0,>F0FF               * mode 2: right split (keep rows 0-3)
        DEC  R6
        JEQ  KSJOY                  * mode 2 -> joystick 2 + right split (unit R5=2)
        DEC  R6                     * modes 3-5: translation state = mode - 3
        CI   R6,>0002
        JH   KSNOK                  * mode >=6 -> no key
        MOVB R6,@>8374              * persist the state to >8374...
        SWPB R6
        MOVB R6,@>83C6              * ...and >83C6 (byte-swapped)
        CLR  R5                     * unit 0 (full keyboard)
KSFULL  CLR  R0                     * full scan: no split mask
        CLR  R2                     * no key found yet
        CLR  R7                     * no modifiers yet
        JMP  KSSCAN
*  ---- joystick scan (modes 1/2, authentic >02F4), then the split-keyboard scan.
*  Read the 5 joystick lines on column 6/7 and look up the (Y,X) deflection pair
*  in GROM (>16E0 + 2*index) into >8376/>8377; with no direction pressed, stage
*  the centered default code and translate it, else fall into the keyboard scan.
KSJOY   LI   R12,>0024             * column-select CRU base
        LDCR @JOYSEL(R5),3         * select column 6 (Joy1, R5=1) or 7 (Joy2, R5=2)
        LI   R12,>0006             * row-read CRU base
        CLR  R3
        SETO R4
        STCR R4,5                  * read 5 joystick lines (fire/L/R/D/U), active-low
        SRL  R4,9                  * form the deflection-table index
        JOC  KSJ1                  *   a direction/fire -> no centered default
        MOVB @JOYDEF(R5),@>83E7     * centered: stage the default code (>83E7 = R3-low)
KSJ1    SLA  R4,1
        AI   R4,>16E0              * GROM deflection address = >16E0 + 2*index
        MOVB R4,@>0402(R13)        * set the GROM read address, high...
        MOVB @>83E9,@>0402(R13)     * ...then low (>83E9 = GPLWS R4-low)
        MOVB *R13,@>8376           * >8376 = Y deflection
        MOVB *R13,@>8377           * >8377 = X deflection
        MOV  R3,R3                  * was a centered default staged?
        JNE  KSKEY                  *   yes -> translate it (split base >17C0)
        CLR  R2                     * else scan the split keyboard for a key
        CLR  R7
KSSCAN  LI   R1,>0005               * start at column 5
KSCOL   LI   R12,>0024              * column-select CRU base
        SWPB R1
        LDCR R1,3                   * output the 3-bit column number
        SWPB R1
        LI   R12,>0006              * row-read CRU base
        SETO R4
        STCR R4,8                   * read the 8 rows (active low, high byte)
        INV  R4                     * pressed -> 1
        MOV  R1,R1                  * column 0?
        JNE  KSNC0
        MOVB R4,R7                  * capture the modifier rows
        ANDI R4,>0F00               * keep rows 0-3 as keys
KSNC0   SZC  R0,R4                  * apply the split mask (0 = full scan, a no-op)
        JEQ  KSCNXT                 * no key in this column
        MOV  R1,R1                  * column 0...
        JNE  KSFND
        MOV  R5,R5                  * ...on a split unit? skip its keys (=, space, Enter)
        JNE  KSCNXT
KSFND   MOV  R2,R2                  * already found one?
        JNE  KSCNXT
        SETO R2                     * mark found
        MOV  R1,R3
        SLA  R3,3                   * column * 8
        DEC  R3
KSBIT   INC  R3
        SLA  R4,1                   * MSB set = 7 - row
        JNC  KSBIT                  * R3 = raw = col*8 + (7-row); >83E7 aliases R3 low
KSCNXT  DEC  R1
        JOC  KSCOL                  * columns 5..0
        MOV  R2,R2
        JNE  KSKEY                  * a key was found
KSNOK   CLR  R6                     * no key: >8375 := >FF, condition clear
        SETO R0
        MOVB R0,@>83C8              * unit-0 debounce cell := >FF
        MOVB R0,@>83C8(R5)          * this unit's debounce cell := >FF
        MOV  R5,R5                  * full scan?
        JNE  KSFIN
        MOVB R0,@>83C9              * full scan -> propagate >FF to the split cells
        MOVB R0,@>83CA
        JMP  KSFIN
KSKEY   MOV  R5,R5                  * split unit?
        JEQ  KSKMOD                 *   full scan -> modifier priority
        LI   R1,>17C0               * split/joystick translation base
        JMP  KSXL
KSKMOD  LI   R1,>1700               * pick the translation base (priority)
        MOV  R7,R6
        ANDI R6,>4000               * CTRL?
        JEQ  KSNCT
        LI   R1,>1790
        JMP  KSXL
KSNCT   MOV  R7,R6
        ANDI R6,>1000               * FCTN?
        JEQ  KSNFC
        LI   R1,>1760
        JMP  KSXL
KSNFC   MOV  R7,R6
        ANDI R6,>2000               * SHIFT?
        JEQ  KSXL
        LI   R1,>1730
KSXL    A    R3,R1                  * base + raw = GROM address (R3 = raw)
        MOVB R1,@>0402(R13)         * set the GROM read address, high then low
        MOVB @>83E3,@>0402(R13)
        CLR  R0
        MOVB *R13,R0                * the translated character
        CB   @>83E7,@>83C8(R5)      * same code as this unit's last scan? (>83E7 = R3 low)
        JEQ  KSHLD
        LI   R6,>2000               * new key -> condition bit
        MOVB @>83E7,@>83C8          * update the unit-0 debounce cell...
        MOVB @>83E7,@>83C8(R5)      * ...and this unit's cell (the full-scan split
*                                     coherence write to >83C9/>83CA — authentic
*                                     >03CA — is a deferred refinement; no flow uses it)
        JMP  KSSPL
KSHLD   CLR  R6                     * a held key -> no condition
KSSPL   MOV  R5,R5                  * split unit?
        JNE  KSFIN                  *   yes -> skip result normalization (§5)

*  ---- result normalization (RECON §23; the authentic >0422-0476 logic) ------
*  Gated by the >83C6 translation state: 0 = 99/4, 1 = Pascal, 2 = 99/4A
*  native. Key-found path only (the no-key path skips it, like the authentic
*  >0382). R3's raw code is dead here and its high byte is 0, so loading the
*  state through the >83E7 low-byte alias yields the state as a word.
*  Split units (R5!=0) skip this block (KSSPL above -> KSFIN), like authentic >0420.
KSNRM   MOVB @>83C6,@>83E7          * R3 := translation state
        CI   R0,>6100               * result in 'a'..'z'?
        JL   KSNAZ
        CI   R0,>7A00
        JH   KSNAZ
        MOV  R3,R3                  * state 0 (the 99/4 has no lowercase):
        JEQ  KSFOLD                 *   fold unconditionally, no switch read
        CLR  R12                    * states 1/2: read the alpha-lock switch
        SBZ  21                     * P5 low; a locked switch pulls bit 7 low.
        TB   7                      * (Our 9901 has no switch input; the line
        SBO  21                     * idles high = never locked — ROADMAP §6.)
        JEQ  KSFIN                  * line high -> not locked -> keep lowercase
KSFOLD  AI   R0,->2000              * fold: lowercase -> uppercase (>20 delta)
        JMP  KSFIN
KSNAZ   MOV  R3,R3                  * not a-z:
        JNE  KSN1
        CI   R0,>1000               * state 0 rejects the 4A-only codes —
        JL   KSFIN                  * >10..>1F and everything above >5F
        CI   R0,>1F00               * behave as "no key" (the authentic >0382)
        JLE  KSNOK
        CI   R0,>5F00
        JH   KSNOK
        JMP  KSFIN
KSN1    DEC  R3                     * state 2 (4A native): nothing more
        JNE  KSFIN
        CI   R0,>0D00               * state 1 (Pascal): Enter is exempt
        JEQ  KSFIN
        CI   R0,>0F00               * <= >0F -> set the >80 bit
        JH   KSN1B
        ORI  R0,>8000
        JMP  KSFIN
KSN1B   CI   R0,>8000               * >80..>9F -> clear the >80 bit
        JL   KSFIN
        CI   R0,>9F00
        JH   KSFIN
        ANDI R0,>7FFF
KSFIN   MOVB R0,@>8375             * the result key (>FF if none)
        BL   @GPOP                  * restore the interpreter's GROM position
        MOVB R6,@>837C              * condition byte (>20 new / >00)
        MOV  R6,R6
        JEQ  KSRET                  * no new key -> no un-blank
        MOVB @>83D4,*R15            * un-blank: reload VDP register 1 from >83D4
        LI   R4,>8100
        MOVB R4,*R15                * the register-1 write selector (>81)
        CLR  @>83D6                 * reset the screen-blank timeout
KSRET   MOV  @>83D8,R11
        B    *R11

*  ---- constants --------------------------------------------------------
W1      DATA >0100                  * opcode word bit (high-byte view)
UBIT    DATA >0200                  * opcode immediate-source bit
MVC     DATA >0200                  * MOVE: C (computed-GROM source)
MVV     DATA >0400                  * MOVE: V (RAM source)
MVR     DATA >0800                  * MOVE: R (VDP-register dest)
MVG     DATA >1000                  * MOVE: G (non-GRAM dest)
X4000   DATA >4000                  * GAS: indexed
V2000   DATA >2000                  * GAS: VDP space
I1000   DATA >1000                  * GAS: indirect
C2000   DATA >2000                  * the condition bit as a word mask
SNDTOG  DATA >0001                  * sound-list FLAGS source-toggle mask (authentic @>0378)
CZERO   BYTE >00
SNDESC  BYTE >FF                    * sound-list "switch source" count sentinel (authentic @>0A9C)
JOYSEL  BYTE >00,>06,>07            * split joystick column selectors (unit R5: 1->6, 2->7)
JOYDEF  BYTE >00,>29,>25            * centered "no direction" split codes (unit R5: 1->>29, 2->>25)
        EVEN

*  ============================================================================
*  Ext-GPL trampolines (>0C0C/>0C14/>0C1C, pinned; helper >0C28) — the
*  never-released extended-GPL card's dispatch targets, vestigial but present
*  (RECON §25). Each enables the card ROM at CRU >1B00 and branches into its
*  >4000 space; with no card the branch lands on empty bus — the authentic
*  accident, reproduced exactly. >0C1C is the XOP-0 vector target (>0040), so
*  the whole block is address-forced (P8), byte-identical like the vectors.
*  ============================================================================
        AORG >0C0C
EXTG14  BL   @EXTON                 * special ops >14-1E + the two-op >98-9F /
        B    @>4020                 * >F0-F3 / >FC-FF blocks -> card entry 0
EXTG1F  BL   @EXTON                 * special op >1F
        B    @>401C
EXTGX0  LWPI >2800                  * the XOP-0 entry (vector >0040)
        BL   @EXTON
        B    @>4028
EXTON   LI   R12,>1B00              * the ext-GPL card's CRU base
        SBO  0                      * card ROM on
        B    *R11

*  ============================================================================
*  Dispatch tables (pinned at their authentic homes)
*  ============================================================================
        AORG >0C36
NIBTAB  DATA SPEC,MOVEH,BRH,BSH     * specials / MOVE / BR / BS

        AORG >0C3E
SPCTAB  DATA RTN,RTNC,RANDH,SCANH   * >00 RTN   >01 RTNC  >02 RAND  >03 SCAN
        DATA BACK,BH,CALLH2,ALLH    * >04 BACK  >05 B     >06 CALL  >07 ALL
        DATA FMT,HH,GTH,START       * >08 FMT   >09 H     >0A GT    >0B EXIT
        DATA CARRYH,OVFH,STUB,XMLH  * >0C CARRY >0D OVF   >0E PARSE(M6) >0F XML
        DATA STUB,STUB,STUB,RTGRH   * >10 CONT  >11 EXEC  >12 RTNB(M6) >13 RTGR
        DATA EXTG14,EXTG14,EXTG14,EXTG14 * >14-1E: the ext-GPL card block
        DATA EXTG14,EXTG14,EXTG14,EXTG14 * (behaviour-faithful vestige,
        DATA EXTG14,EXTG14,EXTG14,EXTG1F * RECON §25); >1F its own entry

        AORG >0C7E
TAB7E   DATA ABSH,NEGH,INVH,CLRH    * >80 >82 >84 >86  (byte/word pairs)
        DATA FETCHH,CASEH,PUSHH,CZH * >88 >8A >8C >8E
        DATA INCH,DECH,INCTH,DECTH  * >90 >92 >94 >96
        DATA EXTG14,EXTG14,EXTG14,EXTG14 * >98-9F: the ext-GPL card block
        DATA ADDH,SUBH,MULH,DIVH    * >A0 >A4 >A8 >AC
        DATA ANDH,ORH,XORH,STH      * >B0 >B4 >B8 >BC
        DATA EXH,CHH,CHEH,CGTH      * >C0 >C4 >C8 >CC
        DATA CGEH,CEQH,CLOGH,SRAH   * >D0 >D4 >D8 >DC
        DATA SLLH,SRLH,SRCH,COINCH  * >E0 >E4 >E8 >EC COINC
        DATA EXTG14,IOH,SWGRH,EXTG14 * >F0 ext  >F4 IO  >F8 SWGR  >FC ext

*  MOVE sub-dispatch (>0CCE, authentic home; fills exactly to the >0CDC FMT
*  table). Structure mirrors the authentic table — three source loaders then
*  four destination storers — but carries our handler addresses (P8 scoping).
        AORG >0CCE
MVTAB   DATA MSCPU,MSVDP,MSGROM     * source loaders:  CPU / VDP / GROM
        DATA MDCPU,MDVDP,MDGRM,MDREG * dest storers:   CPU / VDP / GRAM / VDP-reg

*  FMT sub-dispatch (>0CDC, authentic home; fills exactly from MVTAB to the
*  >0CEC IO table). The format byte's top three bits pick one of eight group
*  handlers (the 9900 word access pairs the odd/even nibble). Structure +
*  location authentic; handler addresses ours (P8 scoping). See Zone I.
        AORG >0CDC
FMTTAB  DATA FHTEX,FVTEX,FHCHA,FVCHA * 0/1 HTEXT 2/3 VTEXT 4/5 HCHAR 6/7 VCHAR
        DATA FHMOV,FVMOV,FRPTB,FCTRL * 8/9 HMOVE A/B VMOVE C/D RPTB  E/F control

*  IO sub-dispatch (>0CEC, authentic home; fills exactly to the >0CFA XML
*  master table). Functions 0/1 (sound-list arming), 2 (CRU input) and 3 (CRU
*  output) are live; 4/5/6 (cassette) are hardware-gated stubs (plan §10.2,
*  M4 slice 5 writes the engines).
        AORG >0CEC
IOTAB   DATA IOSND,IOSND,IOCRIN,IOCROUT * 0 sound-G 1 sound-V 2 CRU-in 3 CRU-out
        DATA CASW,CASR,CASV         * 4 cass-wr  5 cass-rd  6 cass-vfy (Zone K)

*  XML master table-of-tables (>0CFA, byte-identical to authentic — the values
*  are uncopyrightable interface data). Table 0 = FLTAB (FP), 1 = XTAB (device/
*  BASIC), 2-E = RAM/card/cartridge homes with no ROM behind them, F = >8300
*  (the XML >F0 vector). FLTAB/XTAB live at their authentic homes >0D1A/>12A0.
        AORG >0CFA
XMASTER DATA FLTAB,XTAB,>2000,>3FC0   * >0 >1 >2 >3
        DATA >3FE0,>4010,>4030,>6010  * >4 >5 >6 >7
        DATA >6030,>7000,>8000,>A000  * >8 >9 >A >B
        DATA >B000,>C000,>D000,>8300  * >C >D >E >F

*  XML table 0 / FLTAB (>0D1A) — the floating-point dispatch (M5). Entry >00
*  is the authentic accident: the word >0000 sends XML >00 through the reset
*  vector's bytes — a crash/reset, reproduced faithfully (RECON §8).
        AORG >0D1A
FLTAB   DATA >0000,ROUND1,ROUNDH,STSTH * >00 (reset) >01 ROUND1 >02 ROUND >03 STST
        DATA OVEXPH,OVH,FADD,FSUB     * >04 OVEXP >05 OV >06 FADD >07 FSUB
        DATA FMUL,FDIV,FCOMP,SADD     * >08 FMUL >09 FDIV >0A FCOMP >0B SADD
        DATA SSUB,SMUL,SDIV,SCOMP     * >0C SSUB >0D SMUL >0E SDIV >0F SCOMP

        END  START
