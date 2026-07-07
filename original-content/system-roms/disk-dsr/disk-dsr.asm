* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â see LICENSE.md at the repository root.
*
*  disk-dsr.asm ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the rewritten TI Disk Controller DSR ROM (CPU >4000..>5FEF)
*  =========================================================================
*  ORIGINAL WORK, assembled by this repo's own `libre99asm` (crate libre99-asm)
*  into `disk-dsr.bin`, executed by our emulator's FD1771 disk card in place
*  of TI's `Disk.Bin`. Phase 3 of the system-ROM project; the plan is
*  DSR-REWRITE-PLAN.md (this folder), the resume ledger is PROGRESS.md, and
*  the interface spec is RECON.md ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â every behaviour here is implemented from
*  RECON's probe-pinned facts (differentially gated against the genuine DSR
*  as oracle), never from TI's bytes. Clean-room per plan P5.
*
*  STATUS: M1-M4 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â card plumbing, the FD1771 driver, the subprograms
*  (SECTOR, FORMAT, PROTECT, RENAME, FILEIN, FILEOUT, FILES/named-FILES),
*  and the full PAB file system: OPEN (all modes, create/truncate/append)
*  / CLOSE / READ / WRITE (FIX + VAR) / RESTORE / LOAD / SAVE / DELETE /
*  STATUS / the catalog ("DSKn."). SCRATCH returns the authentic error 6.
*  FORMAT is the stock single-density subset (in place, via Write Sector
*  ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the Write-Track substitution, plan ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§0 exception 2), pending the
*  deferred scope decision (plan ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§10.3).
*
*  Calling convention (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§1): entered via BL *R9 on the GPL workspace
*  >83E0 with R12 = the card CRU base (>1100, ROM enabled), R13/R15 = the
*  GROM/VDP ports, LIMI 0. Inputs: >8355 = device-name length, >834A.. =
*  the staged device name, >8356 = VDP address of the char past the device
*  name in the PAB; subprograms: >834C..>8351 parameter block. Handled
*  requests return to R11+2 (skip); power-up returns plainly. We preserve
*  R1 (saved/restored) and R11..R15; R0,R2..R10 are scratch.
*
*  ---- call-level discipline (the register/return-cell contract) --------
*  Leaves   VSETW/VSETR      return B *R11; clobber R0 only; no nesting.
*  FDPOS                     under a driver: plain R11 (driver holds R7).
*  Drivers  RDSEC/WRSEC      return R7;  clobber R0..R5; call leaves+FDPOS.
*  L3 (R8)  CHAIN SLTOA VVCOPY RDFDR8 SLF8 SLFIND SLFREE SLINIT SLMODE
*           WRCNT WRREC CATNAM CATNUM VNCNT   return R8; call leaves only.
*  L3b      SLBASE (R9, under SLFIND/SLFREE); WINFO (R10, under FNDFDR).
*  L2 (@RETF) FNDFDR SLSEC VARNXT VARSKP VOLRES SETBUF ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â call drivers/L3.
*  L2b      VARPEEK (@RETV, under VARNXT/VARSKP); VNLOAD (@T2, under
*           VARPEEK and SLSEC-adjacent flows).
*  Handlers (@RETH) may call everything; loop state in LV0..LV3 (the FNAME
*  cells, free once the name is resolved). T0/T1 = shared data scratch
*  (never return addresses). R1 is saved at entry and restored at exit.
*
*  VRAM layout (ours; the 5-byte header shape, the top formula, and the
*  top+12 info-record address are contract ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the interior is ours):
*    top = [>8370] = >3DEF - 518*n - 6 after power-up (n=3) / FILES(n)
*    top+1..top+5   header: >AA >3F >FF >11 n          (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§2)
*    top+12..+37    the info record (>8356 side-effect target, RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§1)
*    top+40 + 510*i slot i: [PAB word][FDR-sector word][drive b][state b]
*                   [+6: 248-byte FDR copy (chains past 72 clusters are
*                   out of bound ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â far beyond any real file)][+254:
*                   256-byte data buffer]. Ext state rides the FDR copy's
*                   reserved bytes: +6+>14 cursec word (>FFFF = none),
*                   +6+>16 curoff byte, +6+>17 mode (>80 = catalog).
*    >3E00..>3EFF   buffer A (VIB / candidate FDR / LOAD bounce)
*    >3F00..>3FFF   buffer B (the FDIR during lookups)

*  ---- the >AA peripheral-card header (>4000) --------------------------
*  Chain structure and names mirror the authentic card (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§1b).
        AORG >4000
HDR     BYTE >AA                   * valid-DSR marker
        BYTE >02                   * version
        BYTE >00                   * number of programs
        BYTE >00                   * reserved
        DATA PUPCH                 * >4004: power-up chain
        DATA >0000                 * >4006: program chain
        DATA DEVCH                 * >4008: device (DSR) chain
        DATA SUBCH                 * >400A: subprogram chain
        DATA >0000                 * >400C: interrupt chain
        DATA >0000                 * >400E: reserved

PUPCH   DATA >0000                 * one unnamed power-up node
        DATA DPWRUP
        BYTE >00

DEVCH   DATA DEVC1                 * DSK -> DSK1 -> DSK2 -> DSK3
        DATA DEVENT
        BYTE 3
        TEXT 'DSK'
DEVC1   DATA DEVC2
        DATA DEVENT
        BYTE 4
        TEXT 'DSK1'
DEVC2   DATA DEVC3
        DATA DEVENT
        BYTE 4
        TEXT 'DSK2'
DEVC3   DATA >0000
        DATA DEVENT
        BYTE 4
        TEXT 'DSK3'

SUBCH   DATA SUBC1                 * >10 SECTOR
        DATA SECIO
        BYTE 1
        BYTE >10
SUBC1   DATA SUBC2                 * >11 FORMAT
        DATA SFMT
        BYTE 1
        BYTE >11
SUBC2   DATA SUBC3                 * >12 PROTECT
        DATA SPROT
        BYTE 1
        BYTE >12
SUBC3   DATA SUBC4                 * >13 RENAME
        DATA SREN
        BYTE 1
        BYTE >13
SUBC4   DATA SUBC5                 * >14 FILEIN
        DATA SFIN
        BYTE 1
        BYTE >14
SUBC5   DATA SUBC6                 * >15 FILEOUT
        DATA SFOUT
        BYTE 1
        BYTE >15
SUBC6   DATA SUBC7                 * >16 FILES(n)
        DATA SFILES
        BYTE 1
        BYTE >16
SUBC7   DATA >0000                 * named FILES (BASIC's CALL FILES)
        DATA SFILES
        BYTE 5
        TEXT 'FILES'

*  ---- scratch cell names ----------------------------------------------
ABS     EQU  >834A                 * absolute sector (driver arg; SECTOR echo)
DRIVE   EQU  >834C                 * drive 1..3
T3      EQU  >834D                 * byte scratch (SECTOR r/w flag input)
DMA     EQU  >834E                 * VDP address for sector transfers
T0      EQU  >8350                 * word scratch (SECTOR sector input/error)
T1      EQU  >8352                 * word scratch
PAB     EQU  >8354                 * PAB base (also the authentic side-effect)
INFO    EQU  >8356                 * in: name-follow ptr; out: top+12
OPC     EQU  >8358                 * PAB+0 copy
FLG     EQU  >8359                 * PAB+1 copy
BUF     EQU  >835A                 * PAB+2/3 copy
RCL     EQU  >835C                 * PAB+4 copy
CNT     EQU  >835D                 * PAB+5 copy
REC     EQU  >835E                 * PAB+6/7 copy
FNAME   EQU  >8360                 * 10-byte file name (then free: RETV/LVs)
RETV    EQU  >8360                 * VARPEEK return (record ops only)
LV0     EQU  >8362                 * handler loop state (record ops only)
LV1     EQU  >8364
LV2     EQU  >8366
LV3     EQU  >8368
SLOT    EQU  >836A                 * active slot VRAM address
T2      EQU  >836C                 * VNLOAD return cell
RETH    EQU  >83DA                 * handler return (the console's R11)
RETF    EQU  >83DC                 * L2 FS return
R1SV    EQU  >83DE                 * saved R1

VDPRD   EQU  >8800
VDPWD   EQU  >8C00
VDPWA   EQU  >8C02
FDSTA   EQU  >5FF0
FDDAT   EQU  >5FF6
FDCMD   EQU  >5FF8
FDTRK   EQU  >5FFA
FDSEC   EQU  >5FFC
FDDWR   EQU  >5FFE

*  ======================================================================
*  Power-up (plain return) and SETBUF (L2): reserve the region for n
*  buffers, write the header, zero the region, mark the slots free.
*  ======================================================================
DPWRUP  MOV  R11,@RETH
        MOV  R1,@R1SV
        LI   R0,3
        BL   @SETBUF
        MOV  @R1SV,R1
        MOV  @RETH,R11
        B    *R11                  * plain return: the scan continues

*  SETBUF: R0 = n (1..9). top = >3DEF - 518n - 6. Clobbers R0,R2..R6.
SETBUF  MOV  R11,@RETF
        MOV  R0,R6                 * n
        LI   R2,518
        MPY  R0,R2                 * R2:R3 = 518n (fits R3)
        LI   R2,>3DE9              * >3DEF - 6
        S    R3,R2                 * R2 = top
        MOV  R2,@>8370
        MOV  R2,R0                 * header at top+1
        INC  R0
        BL   @VSETW
        LI   R1,>AA00
        MOVB R1,@VDPWD
        LI   R1,>3F00
        MOVB R1,@VDPWD
        LI   R1,>FF00
        MOVB R1,@VDPWD
        LI   R1,>1100
        MOVB R1,@VDPWD
        MOV  R6,R1
        SWPB R1
        MOVB R1,@VDPWD             * n
        MOV  R2,R0                 * zero top+6 .. >3FFF
        AI   R0,6
        LI   R4,>4000
        S    R0,R4
        BL   @VSETW
        CLR  R1
SBZLP   MOVB R1,@VDPWD
        DEC  R4
        JNE  SBZLP
        MOV  @RETF,R11
        B    *R11

*  ======================================================================
*  Leaves: VDP address setup. R0 = address (clobbered).
*  ======================================================================
VSETW   ORI  R0,>4000
VSETR   SWPB R0
        MOVB R0,@VDPWA             * low byte first
        SWPB R0
        MOVB R0,@VDPWA             * then high (|>40 for write mode)
        B    *R11
*  R10-return I/O shorthands (they call the drivers; NOT callable from
*  routines holding R10 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â FNDFDR's bisect keeps its loads inline).
*  LDA/WRA: the FDR sector @T1 <-> buffer A.  LD0/WR0: the VIB <-> A.
*  LDB/WRB: the FDIR (sector 1) <-> buffer B.  RDW: read the big-endian
*  word at VDP address R0 into R2 (clobbers R0,R3).
LDA     MOV  R11,R10
        MOV  @T1,@ABS
LDAB    LI   R0,>3E00
        MOV  R0,@DMA
        BL   @RDSEC
        B    *R10
LD0     MOV  R11,R10
        CLR  @ABS
        JMP  LDAB
LDB     MOV  R11,R10
        LI   R0,1
        MOV  R0,@ABS
        LI   R0,>3F00
        MOV  R0,@DMA
        BL   @RDSEC
        B    *R10
WRA     MOV  R11,R10
        MOV  @T1,@ABS
WRAB    LI   R0,>3E00
        MOV  R0,@DMA
        BL   @WRSEC
        B    *R10
WR0     MOV  R11,R10
        CLR  @ABS
        JMP  WRAB
WRB     MOV  R11,R10
        LI   R0,1
        MOV  R0,@ABS
        LI   R0,>3F00
        MOV  R0,@DMA
        BL   @WRSEC
        B    *R10
RDW     MOV  R11,R10
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        SRL  R2,8
        SLA  R2,8
        SRL  R3,8
        SOC  R3,R2
        B    *R10
SLR     MOV  R11,R10               * VDP read addr := @SLOT + R0
        A    @SLOT,R0
        BL   @VSETR
        B    *R10
SLW     MOV  R11,R10               * VDP write addr := @SLOT + R0
        A    @SLOT,R0
        BL   @VSETW
        B    *R10
PUTNM   MOV  R11,R10               * FNAME -> the first 10 bytes of A
        LI   R0,>3E00
        BL   @VSETW
        CLR  R4
PUTNML  MOVB @FNAME(R4),R1
        MOVB R1,@VDPWD
        INC  R4
        CI   R4,10
        JL   PUTNML
        B    *R10

*  ======================================================================
*  The FD1771 driver (return R7; clobber R0..R5). Stock geometry: 9
*  sectors/track, 40 tracks, side 1 = absolute >= 360 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â no VIB read,
*  matching the authentic DSR (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§5). Register bytes are inverted.
*  RDSEC: sector @ABS of drive @DRIVE -> VDP @DMA. WRSEC: the reverse.
*  EQ set on success.
*  ======================================================================
RDSEC   MOV  R11,R7
        BL   @FDPOS
        LI   R1,>7F00              * >80 read sector, inverted
        MOVB R1,@FDCMD
        MOVB @FDSTA,R1
        INV  R1
        ANDI R1,>0100              * BUSY = data staged
        JEQ  FDFAIL
        MOV  @DMA,R0
        BL   @VSETW
        LI   R2,256
RDLP    MOVB @FDDAT,R1
        INV  R1
        MOVB R1,@VDPWD
        DEC  R2
        JNE  RDLP
        SZC  R1,R1                 * zero + EQ in one op
        B    *R7

WRSEC   MOV  R11,R7
        BL   @FDPOS
        MOV  @DMA,R0
        BL   @VSETR
        LI   R1,>5F00              * >A0 write sector, inverted
        MOVB R1,@FDCMD
        MOVB @FDSTA,R1
        INV  R1
        ANDI R1,>0100              * BUSY = accepting
        JEQ  FDFAIL
        LI   R2,256
WRLP    MOVB @VDPRD,R1
        INV  R1
        MOVB R1,@FDDWR
        DEC  R2
        JNE  WRLP
        SZC  R1,R1                 * zero + EQ in one op
        B    *R7
FDFAIL  SETO R1                    * EQ clear = failure
        MOV  R1,R1
        B    *R7

*  FDPOS: select drive/side, set track+sector registers for @ABS.
FDPOS   MOV  @ABS,R3
        MOVB @DRIVE,R2
        SRL  R2,8
        CI   R2,1
        JNE  FDP2
        SBO  4
        JMP  FDPSID
FDP2    CI   R2,2
        JNE  FDP3
        SBO  5
        JMP  FDPSID
FDP3    SBO  6
FDPSID  CI   R3,360
        JHE  FDPS1
        SBZ  7
        CLR  R2
        LI   R0,9
        DIV  R0,R2                 * R2 = track, R3 = sector-in-track
        JMP  FDPWR
FDPS1   SBO  7
        AI   R3,-360
        CLR  R2
        LI   R0,9
        DIV  R0,R2
        LI   R0,39                 * side-1 track runs outward
        S    R2,R0
        MOV  R0,R2
FDPWR   SWPB R2
        INV  R2
        MOVB R2,@FDTRK
        SWPB R3
        INV  R3
        MOVB R3,@FDSEC
        B    *R11

*  ======================================================================
*  Subprograms
*  ======================================================================

*  SECTOR (>10): unit @>834C, r/w flag @>834D (0 = write), VDP buffer
*  @>834E, sector = the word at >8350; echo in >834A, error byte >8350.
SECIO   MOV  R11,@RETH
        MOV  R1,@R1SV
        MOV  @T0,@ABS
        MOVB @T3,R1
        JEQ  SECWR
        BL   @RDSEC
        JMP  SECFIN
SECWR   BL   @WRSEC
SECFIN  JEQ  SECOK
        LI   R1,>0600              * device error
        JMP  SECST
SECOK   CLR  R1
SECST   MOVB R1,@T0
        MOV  @R1SV,R1
        MOV  @RETH,R11
        INCT R11
        B    *R11

*  FILES (>16 / named): n @>834C -> rebuild the buffer region.
SFILES  MOV  R11,@RETH
        MOV  R1,@R1SV
        MOVB @DRIVE,R0
        SRL  R0,8
        JEQ  SFBAD
        CI   R0,9
        JH   SFBAD
        BL   @SETBUF
        CLR  R1
        JMP  SFST
SFBAD   LI   R1,>0400              * out-of-range n
SFST    MOVB R1,@T0
        MOV  @R1SV,R1
        MOV  @RETH,R11
        INCT R11
        B    *R11

*  M3/M4 subprogram placeholders: graceful device-error, never a hang.
SSTUB   MOV  R11,@RETH
        LI   R1,>0600
        MOVB R1,@T0
        MOV  @RETH,R11
        INCT R11
        B    *R11

*  ======================================================================
*  The device entry: fetch + parse the PAB, resolve the drive, dispatch.
*  Always handled (skip return); errors OR'd into PAB+1 (sticky).
*  ======================================================================
DEVENT  MOV  R11,@RETH
        MOV  R1,@R1SV
        CLR  @SLOT
*  Drive from the staged name BEFORE >834A.. is reused: "DSKn" digit at
*  >834D; "DSK" (len 3) -> volume form, resolved after the parse.
        CLR  R4
        MOVB @>8355,R0
        SRL  R0,8
        MOV  R0,R5                 * R5 = device-name length
        CI   R0,3
        JEQ  DEVVOL
        MOVB @T3,R4
        SRL  R4,8
        AI   R4,->30               * '1'..'3' -> 1..3
DEVVOL  MOV  R4,@T1                * park the drive (0 = by volume)
*  PAB base = [>8356] - devlen - 10.
        MOV  @INFO,R0
        S    R5,R0
        AI   R0,-10
        MOV  R0,@PAB
*  Copy PAB+0..+7 to >8358.., skip +8, name length -> R6.
        BL   @VSETR
        LI   R2,OPC
        LI   R0,8
PABLP   MOVB @VDPRD,*R2+
        DEC  R0
        JNE  PABLP
        MOVB @VDPRD,R0             * +8 screen offset
        MOVB @VDPRD,R6
        SRL  R6,8                  * total name length
*  File-name length = total - devlen - 1 ('.'), or 0 for "DSKn.".
        MOV  R6,R8
        S    R5,R8
        JEQ  DEVPAD
        DEC  R8
DEVPAD  LI   R2,FNAME              * pre-fill with spaces
        LI   R1,>2000
        LI   R0,10
FNPAD   MOVB R1,*R2+
        DEC  R0
        JNE  FNPAD
        MOV  R8,R8
        JEQ  DEVDRV
        MOV  @PAB,R0               * copy the name text (past the '.')
        AI   R0,10
        A    R5,R0
        INC  R0
        BL   @VSETR
        LI   R2,FNAME
        MOV  R8,R4
        CI   R4,10
        JLE  FNCP0
        LI   R4,10
FNCP0   MOVB @VDPRD,*R2+
        DEC  R4
        JNE  FNCP0
DEVDRV  MOV  @T1,R4
        MOV  R4,R4
        JNE  DEVSET
        BL   @VOLRES               * volume form: sets DRIVE + FNAME
        JMP  DEVGO
DEVSET  SWPB R4
        MOVB R4,@DRIVE
DEVGO   MOVB @OPC,R2
        SRL  R2,8
        CI   R2,9
        JH   DEVBAD
        SLA  R2,1
        MOV  @OPTAB(R2),R2
        B    *R2
DEVBAD  LI   R0,3
        B    @ERREX

OPTAB   DATA HOPEN                 * 0
        DATA HCLOSE                * 1
        DATA HREAD                 * 2
        DATA HWRITE                * 3 (M3 placeholder)
        DATA HREST                 * 4
        DATA HLOAD                 * 5
        DATA HSAVE                 * 6 (M3 placeholder)
        DATA HDEL                  * 7 (M3 placeholder)
        DATA HSCR                  * 8 (authentic: error 6)
        DATA HSTAT                 * 9

*  ---- common exits ------------------------------------------------------
ERREX   SLA  R0,5                  * code -> bits 5-7
        SWPB R0
        MOVB @FLG,R1
        SOCB R1,R0                 * flags | code<<5
        MOV  R0,R2
        MOV  @PAB,R0
        INC  R0
        BL   @VSETW
        MOVB R2,@VDPWD
OKEX    MOV  @R1SV,R1
        MOV  @RETH,R11
        INCT R11
        B    *R11

HWRITE  B    @HWRITB               * write side lives past the read section
HSAVE   B    @HSAVEB
HDEL    B    @HDELB
HSCR    LI   R0,6                  * SCRATCH: the authentic error (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§3)
        B    @ERREX

*  ======================================================================
*  VOLRES (L2): FNAME holds "VOLNAME.FILE"; match a mounted drive's VIB
*  name, set DRIVE, rewrite FNAME to the file part. No match: error 7.
*  ======================================================================
VOLRES  MOV  R11,@RETF
        LI   R2,FNAME              * find the '.' (volume length -> T0)
        CLR  R4
VRSPL   CI   R4,10
        JEQ  VRSPD
        MOVB *R2+,R0
        SRL  R0,8
        CI   R0,>2E
        JEQ  VRSPD
        INC  R4
        JMP  VRSPL
VRSPD   MOV  R4,@T0
        LI   R5,1                  * try drives 1..3
VRDRV   MOV  R5,R0
        SWPB R0
        MOVB R0,@DRIVE
        BL   @LD0
        JNE  VRNXT
        CLR  R6                    * compare 10 bytes (name then spaces)
VRCMP   LI   R0,>3E00
        A    R6,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        C    R6,@T0
        JHE  VRSPC
        MOVB @FNAME(R6),R3
        CB   R3,R2
        JNE  VRNXT
        JMP  VRSTP
VRSPC   CI   R2,>2000
        JNE  VRNXT
VRSTP   INC  R6
        CI   R6,10
        JL   VRCMP
        JMP  VRHIT
VRNXT   INC  R5
        CI   R5,4
        JL   VRDRV
        LI   R0,7                  * unknown volume (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§3)
        B    @ERREX
VRHIT   MOV  @T0,R4                * shift the file part down, re-pad
        INC  R4
        LI   R2,FNAME
        LI   R5,10
VRSH    CI   R4,10
        JHE  VRSHSP
        MOVB @FNAME(R4),R0
        JMP  VRSHW
VRSHSP  LI   R0,>2000
VRSHW   MOVB R0,*R2+
        INC  R4
        DEC  R5
        JNE  VRSH
        MOV  @RETF,R11
        B    *R11

*  ======================================================================
*  L3 utilities (return R8 unless noted; call leaves only)
*  ======================================================================

*  VVCOPY: R4 bytes VDP[R2] -> VDP[R3]. Clobbers R0,R1,R2,R3,R4.
VVCOPY  MOV  R11,R8
VVLP    MOV  R2,R0
        BL   @VSETR
        MOVB @VDPRD,R1
        MOV  R3,R0
        BL   @VSETW
        MOVB R1,@VDPWD
        INC  R2
        INC  R3
        DEC  R4
        JNE  VVLP
        B    *R8

*  CHAIN: file-sector index R2 -> @ABS via the cluster list of the FDR in
*  buffer A. EQ ok; EQ clear past the end. Clobbers R0,R3..R6,R9,R10.
CHAIN   MOV  R11,R8
        SETO R9                    * previous end offset = -1
        LI   R10,>3E1C
CHLP    MOV  R10,R0
        BL   @VSETR
        MOVB @VDPRD,R3             * b0
        MOVB @VDPRD,R4             * b1
        MOVB @VDPRD,R5             * b2
        SRL  R3,8
        SRL  R4,8
        SRL  R5,8
        MOV  R3,R6
        SOC  R4,R6
        SOC  R5,R6
        JEQ  CHFAIL                * zero cluster = end of chain
        MOV  R4,R6                 * start = (b1 & >0F) << 8 | b0
        ANDI R6,>000F
        SLA  R6,8
        SOC  R3,R6
        MOV  R5,R0                 * end = b2 << 4 | b1 >> 4
        SLA  R0,4
        MOV  R4,R3
        SRL  R3,4
        SOC  R3,R0
        C    R2,R0
        JGT  CHNEXT
        MOV  R2,R3                 * abs = start + (idx - (prev+1))
        S    R9,R3
        DEC  R3
        A    R6,R3
        MOV  R3,@ABS
        SZC  R0,R0                 * zero + EQ in one op
        B    *R8
CHNEXT  MOV  R0,R9
        AI   R10,3
        JMP  CHLP
CHFAIL  SETO R0
        MOV  R0,R0
        B    *R8

*  SLTOA: copy the active slot's 256-byte FDR copy into buffer A.
SLTOA   MOV  R11,R8
        CLR  R3
SLTAL   MOV  @SLOT,R0
        AI   R0,6
        A    R3,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        LI   R0,>3E00
        A    R3,R0
        BL   @VSETW
        MOVB R2,@VDPWD
        INC  R3
        CI   R3,248                * the slot FDR copy is 248 bytes
        JL   SLTAL
        B    *R8

*  RDFDR8: FDR header fields out of buffer A:
*  R2 = flags, R3 = recs/sector, R4 = eof offset, R5 = reclen (all in the
*  LOW byte), R6 = the >12/13 count (byte-swap undone). Clobbers R0.
RDFDR8  MOV  R11,R8
        LI   R0,>3E0C
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        SRL  R2,8
        SRL  R3,8
        LI   R0,>3E10
        BL   @VSETR
        MOVB @VDPRD,R4
        MOVB @VDPRD,R5
        MOVB @VDPRD,R6             * count low byte (on-disk +12)
        MOVB @VDPRD,R0             * count high byte (on-disk +13)
        SRL  R4,8
        SRL  R5,8
        SRL  R6,8
        ANDI R0,>FF00
        SOC  R0,R6                 * R6 = count
        B    *R8

*  SLF8: the same fields from the active slot's FDR copy:
*  R3 = recs/sector, R5 = reclen, R6 = count. Clobbers R0,R2.
SLF8    MOV  R11,R8
        LI   R0,6+>0D
        BL   @SLR
        MOVB @VDPRD,R3
        SRL  R3,8
        LI   R0,6+>11
        BL   @SLR
        MOVB @VDPRD,R5
        MOVB @VDPRD,R6             * count low
        MOVB @VDPRD,R2             * count high
        SRL  R5,8
        SRL  R6,8
        ANDI R2,>FF00
        SOC  R2,R6
        B    *R8

*  SLBASE (return R9): R4 = first slot (top+7), R5 = n.
SLBASE  MOV  R11,R9
        MOV  @>8370,R4
        MOV  R4,R0
        AI   R0,5
        BL   @VSETR
        MOVB @VDPRD,R5
        SRL  R5,8
        AI   R4,40                 * slots start at top+40 (past the
        B    *R9                   *  top+12 info record ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§1)

*  SLFIND: the open slot whose PAB matches @PAB -> @SLOT (EQ). SLFREE: a
*  free slot -> @SLOT (EQ). Clobber R0,R2..R5,R9.
SLFIND  MOV  R11,R8
        BL   @SLBASE
SLFLP   MOV  R5,R5
        JEQ  SLNO
        MOV  R4,R0
        AI   R0,5
        BL   @VSETR
        MOVB @VDPRD,R2
        SRL  R2,8
        JEQ  SLFNX                 * free slot
        MOV  R4,R0
        BL   @RDW                 * the slot's PAB word
        C    R2,@PAB
        JNE  SLFNX
        MOV  R4,@SLOT
        SZC  R0,R0                 * zero + EQ in one op
        B    *R8
SLFNX   AI   R4,510                * our slot stride (6+248+256)
        DEC  R5
        JMP  SLFLP
SLNO    SETO R0
        MOV  R0,R0
        B    *R8

SLFREE  MOV  R11,R8
        BL   @SLBASE
SLRLP   MOV  R5,R5
        JEQ  SLNO
        MOV  R4,R0
        AI   R0,5
        BL   @VSETR
        MOVB @VDPRD,R2
        SRL  R2,8
        JEQ  SLRHIT
        AI   R4,510                * our slot stride
        DEC  R5
        JMP  SLRLP
SLRHIT  MOV  R4,@SLOT
        SZC  R0,R0                 * zero + EQ in one op
        B    *R8

*  SLINIT: claim @SLOT for @PAB/@DRIVE/@T1 (FDR sector); cursec >FFFF.
SLINIT  MOV  R11,R8
        MOV  @SLOT,R0
        BL   @VSETW
        MOVB @PAB,R1
        MOVB R1,@VDPWD
        MOVB @PAB+1,R1
        MOVB R1,@VDPWD
        MOVB @T1,R1
        MOVB R1,@VDPWD
        MOVB @T1+1,R1
        MOVB R1,@VDPWD
        MOVB @DRIVE,R1
        MOVB R1,@VDPWD
        LI   R1,>0100              * state = open
        MOVB R1,@VDPWD
        LI   R0,6+>14
        BL   @SLW
        SETO R1
        MOVB R1,@VDPWD             * >14/15 cursec = >FFFF
        MOVB R1,@VDPWD
        CLR  R1
        MOVB R1,@VDPWD             * >16 curoff = 0
        MOVB R1,@VDPWD             * >17 mode placeholder (SLMODE rewrites)
        MOVB R1,@VDPWD             * >18 TFLAG = 0 (buffer not chain-tail)
        MOVB R1,@VDPWD             * >19 spare
        SETO R1
        MOVB R1,@VDPWD             * >1A/1B LASTPH = >FFFE
        LI   R1,>FE00
        MOVB R1,@VDPWD
        B    *R8

*  SLMODE: mode byte (R1 high) -> slot ext +>17.
SLMODE  MOV  R11,R8
        MOV  R1,@T0
        LI   R0,6+>17
        BL   @SLW
        MOVB @T0,R1
        MOVB R1,@VDPWD
        B    *R8

*  WRCNT: R1 (high byte) -> PAB+5 (+ the @CNT copy).
WRCNT   MOV  R11,R8
        MOVB R1,@CNT
        MOV  @PAB,R0
        AI   R0,5
        BL   @VSETW
        MOVB @CNT,R1
        MOVB R1,@VDPWD
        B    *R8

*  WRREC: R2 -> PAB+6/7 (+ the @REC copy).
WRREC   MOV  R11,R8
        MOV  R2,@REC
        MOV  @PAB,R0
        AI   R0,6
        BL   @VSETW
        MOVB @REC,R1
        MOVB R1,@VDPWD
        MOVB @REC+1,R1
        MOVB R1,@VDPWD
        B    *R8

*  VNCNT: the file's >12/13 count from the slot FDR copy -> R5.
VNCNT   MOV  R11,R8
        LI   R0,6+>12
        BL   @SLR
        MOVB @VDPRD,R3             * low
        MOVB @VDPRD,R5             * high
        SRL  R3,8
        ANDI R5,>FF00
        SOC  R3,R5
        B    *R8

*  WINFO (return R10, under FNDFDR): the info record at top+12 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the name
*  with its first byte zeroed, then FDR bytes >0A..>13 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â and >8356.
WINFO   MOV  R11,R10
        MOV  @>8370,R5
        AI   R5,12
        MOV  R5,@INFO
        MOV  R5,R0
        BL   @VSETW
        CLR  R1
        MOVB R1,@VDPWD             * name[0] = 0
        LI   R4,1
WINML   MOVB @FNAME(R4),R1
        MOVB R1,@VDPWD
        INC  R4
        CI   R4,10
        JL   WINML
        CLR  R4                    * FDR >0A..>13 -> info+10..+19
WINCL   LI   R0,>3E0A
        A    R4,R0
        BL   @VSETR
        MOVB @VDPRD,R1
        MOV  R5,R0
        AI   R0,10
        A    R4,R0
        BL   @VSETW
        MOVB R1,@VDPWD
        INC  R4
        CI   R4,10
        JL   WINCL
        B    *R10

*  CATNAM: the 10-byte name at VDP R5, trailing-space-trimmed, emitted as
*  [len][chars] at the @T1 write cursor (starts at @BUF). CATNM0: empty.
CATNAM  MOV  R11,R8
        CLR  R4                    * trimmed length
        CLR  R3
CATNL   MOV  R5,R0
        A    R3,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        SRL  R2,8
        CI   R2,>20
        JEQ  CATNS
        MOV  R3,R4
        INC  R4
CATNS   INC  R3
        CI   R3,10
        JL   CATNL
        MOV  @T1,R0
        BL   @VSETW
        MOV  R4,R1
        SWPB R1
        MOVB R1,@VDPWD
        INC  @T1
        CLR  R3
CATNC   C    R3,R4
        JHE  CATND
        MOV  R5,R0
        A    R3,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        MOV  @T1,R0
        BL   @VSETW
        MOVB R2,@VDPWD
        INC  @T1
        INC  R3
        JMP  CATNC
CATND   B    *R8
CATNM0  MOV  R11,R8
        MOV  @T1,R0
        BL   @VSETW
        CLR  R1
        MOVB R1,@VDPWD
        INC  @T1
        B    *R8

*  CATNUM: R1 (signed, |v| < 10000) as a len-8 radix-100 real at @T1.
*  Negative: two's-complement the first word (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§3).
CATNUM  MOV  R11,R8
        MOV  R1,R6
        CLR  R7                    * first word (exp:d1)
        CLR  R5                    * d2
        CLR  R9                    * sign flag
        MOV  R6,R6
        JEQ  CATEMT
        JGT  CATPOS
        SETO R9
        NEG  R6
CATPOS  CI   R6,100
        JL   CAT1D
        CLR  R4                    * two digits: >41, v/100, v%100
        MOV  R6,R5
        LI   R0,100
        DIV  R0,R4                 * R4 = quotient, R5 = remainder
        LI   R7,>4100
        A    R4,R7
        JMP  CATSGN
CAT1D   LI   R7,>4000              * one digit: >40, v
        A    R6,R7
CATSGN  MOV  R9,R9
        JEQ  CATEMT
        NEG  R7
CATEMT  MOV  @T1,R0
        BL   @VSETW
        LI   R1,>0800              * [8]
        MOVB R1,@VDPWD
        MOV  R7,R1
        MOVB R1,@VDPWD             * exponent byte
        SWPB R1
        MOVB R1,@VDPWD             * first digit
        MOV  R5,R1
        SWPB R1
        MOVB R1,@VDPWD             * second digit (or 0)
        CLR  R1
        MOVB R1,@VDPWD
        MOVB R1,@VDPWD
        MOVB R1,@VDPWD
        MOVB R1,@VDPWD
        MOVB R1,@VDPWD             * 8 mantissa/pad bytes total
        MOV  @T1,R0
        AI   R0,9
        MOV  R0,@T1
        B    *R8

*  ======================================================================
*  L2 file-system routines (return @RETF)
*  ======================================================================

*  FNDFDR: look FNAME up on @DRIVE via the FDIR bisect (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§6). EQ:
*  @T1 = the FDR sector, the FDR in buffer A, info record written.
FNDFDR  MOV  R11,@RETF
        BL   @LDB
        JNE  FNDEV                 * unreadable FDIR: a device error (R0=1)
        CLR  R8                    * lo = 0
        LI   R9,254                * hi = 254 (byte offsets, 127 slots)
FNLOOP  C    R8,R9
        JHE  FNMISS
        MOV  R8,R10                * mid = ((lo+hi)/2) & ~1
        A    R9,R10
        SRL  R10,1
        ANDI R10,>FFFE
        LI   R0,>3F00
        A    R10,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        SRL  R2,8
        SLA  R2,8
        SRL  R3,8
        SOC  R3,R2                 * the FDR pointer at [mid]
        JEQ  FNHI                  * empty compares high
        MOV  R2,@ABS
        MOV  R2,@T1
        LI   R0,>3E00
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  FNMISS
        LI   R0,>3E00              * compare the 10-byte names
        BL   @VSETR
        CLR  R4
FNCMPL  MOVB @VDPRD,R2
        MOVB @FNAME(R4),R3
        CB   R3,R2                 * FNAME vs the entry
        JNE  FNDIF
        INC  R4
        CI   R4,10
        JL   FNCMPL
        BL   @WINFO                * found
        MOV  @RETF,R11
        SZC  R0,R0                 * zero + EQ in one op
        B    *R11
FNDIF   JL   FNLO
        MOV  R10,R8                * FNAME > entry: lo = mid+2
        INCT R8
        JMP  FNLOOP
FNLO    MOV  R10,R9                * FNAME < entry: hi = mid
        JMP  FNLOOP
FNHI    MOV  R10,R9                * empty slot: hi = mid
        JMP  FNLOOP
FNMISS  MOV  @RETF,R11
        SETO R0                    * NE + R0=-1: the name is absent
        MOV  R0,R0
        B    *R11
FNDEV   MOV  @RETF,R11
        LI   R0,1                  * NE + R0=1: the drive/FDIR failed
        B    *R11

*  SLSEC: make file-sector R2 resident in the slot's data buffer (via the
*  slot's FDR copy). EQ ok. Uses @T0 as its park.
SLSEC   MOV  R11,@RETF
        MOV  R2,@T0                * wanted
        MOV  @SLOT,R0
        AI   R0,6+>14
        BL   @RDW                 * cursec
        C    R2,@T0
        JNE  SLSLD
        BL   @SLTOA                * already resident: still publish @ABS
        MOV  @T0,R2                * (the FIX write-through needs it)
        BL   @CHAIN
        JNE  SLSNO
        JMP  SLSOK
SLSLD   BL   @SLTOA                * chain lives in buffer A
        MOV  @T0,R2
        BL   @CHAIN
        JNE  SLSNO
        MOV  @SLOT,R0
        AI   R0,254                * slot data buffer (6+248)
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  SLSNO
        LI   R0,6+>14
        BL   @SLW
        MOVB @T0,R1
        MOVB R1,@VDPWD
        MOVB @T0+1,R1
        MOVB R1,@VDPWD
SLSOK   MOV  @RETF,R11
        SZC  R0,R0                 * zero + EQ in one op
        B    *R11
SLSNO   MOV  @RETF,R11
        SETO R0
        MOV  R0,R0
        B    *R11

*  VNLOAD (return @T2): load file-sector R2 into the slot data buffer and
*  set cursec; used by the VAR path. Preserves the index in @LV0.
VNLOAD  MOV  R11,@T2
        MOV  R2,@LV0
        BL   @SLTOA
        MOV  @LV0,R2
        BL   @CHAIN
        JNE  VNLNO
        MOV  @SLOT,R0
        AI   R0,254                * slot data buffer (6+248)
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  VNLNO
        LI   R0,6+>14
        BL   @SLW
        MOVB @LV0,R1
        MOVB R1,@VDPWD             * big-endian: high byte first
        MOVB @LV0+1,R1
        MOVB R1,@VDPWD
        MOV  @T2,R11
        SZC  R0,R0                 * zero + EQ in one op
        B    *R11
VNLNO   MOV  @T2,R11
        SETO R0
        MOV  R0,R0
        B    *R11

*  VARPEEK (return @RETV): position at the next VAR record: ensure a
*  sector is loaded, skip >FF sector-ends, leave @LV0 = cursec and
*  @LV1 = curoff pointing at the length byte, R9 = the record length.
*  EQ ok; EQ clear = end of file.
VARPEEK MOV  R11,@RETV
        MOV  @SLOT,R0
        AI   R0,6+>14
        BL   @RDW                 * cursec
        MOVB @VDPRD,R4
        SRL  R4,8                  * curoff
        MOV  R4,@LV1
        CI   R2,>FFFF
        JNE  VPHAVE
        CLR  R2
        BL   @VNLOAD               * (VNLOAD leaves the index in @LV0)
        JNE  VPEOF
        CLR  @LV1
        JMP  VPLEN
VPHAVE  MOV  R2,@LV0
VPLEN   MOV  @SLOT,R0
        AI   R0,254                * slot data buffer (6+248)
        A    @LV1,R0
        BL   @VSETR
        MOVB @VDPRD,R9
        SRL  R9,8
        CI   R9,>00FF
        JNE  VPOK
        BL   @VNCNT                * sector-end: advance to the next
        MOV  @LV0,R2
        INC  R2
        C    R2,R5
        JHE  VPEOF
        BL   @VNLOAD
        JNE  VPEOF
        MOV  @LV0,R2               * (VNLOAD kept the index in LV0)
        CLR  @LV1
        JMP  VPLEN
VPOK    MOV  @RETV,R11
        SZC  R0,R0                 * zero + EQ in one op
        B    *R11
VPEOF   MOV  @RETV,R11
        SETO R0
        MOV  R0,R0
        B    *R11

*  VARNXT (L2): serve the next VAR record into the caller's buffer,
*  update the cursor + PAB+5. EQ ok; EQ clear = EOF.
VARNXT  MOV  R11,@RETF
        BL   @VARPEEK
        JNE  VNXEOF
        MOV  R9,@LV2               * record length
        MOV  R9,R9
        JEQ  VNXCNT                * zero-length record: nothing to copy
        MOV  @SLOT,R2              * src = slot data + curoff + 1
        AI   R2,254                * slot data buffer (6+248)
        A    @LV1,R2
        INC  R2
        MOV  @BUF,R3
        MOV  @LV2,R4
        BL   @VVCOPY
VNXCNT  MOV  @LV2,R1               * PAB+5 = len
        SWPB R1
        BL   @WRCNT
        MOV  @LV1,R0               * curoff += len + 1
        A    @LV2,R0
        INC  R0
        MOV  R0,@LV1
        LI   R0,6+>16
        BL   @SLW
        MOVB @LV1+1,R1             * low byte of the new offset
        MOVB R1,@VDPWD
        MOV  @RETF,R11
        SZC  R0,R0                 * zero + EQ in one op
        B    *R11
VNXEOF  MOV  @RETF,R11
        SETO R0
        MOV  R0,R0
        B    *R11

*  VARSKP (L2): skip one VAR record without serving it (RESTORE's loop).
VARSKP  MOV  R11,@RETF
        BL   @VARPEEK
        JNE  VNXEOF
        MOV  @LV1,R0
        A    R9,R0
        INC  R0
        MOV  R0,@LV1
        LI   R0,6+>16
        BL   @SLW
        MOVB @LV1+1,R1
        MOVB R1,@VDPWD
        MOV  @RETF,R11
        SZC  R0,R0                 * zero + EQ in one op
        B    *R11

*  ======================================================================
*  PAB opcode handlers
*  ======================================================================

*  OPEN (0).
HOPEN   BL   @SLFREE
        JNE  HOPFUL
        MOVB @FNAME,R0             * empty name = the catalog form
        SRL  R0,8
        CI   R0,>20
        JNE  HOPFIL
*  --- catalog open ---
        MOVB @RCL,R0
        SRL  R0,8
        JNE  HOPC1
        LI   R1,>2600              * default record length 38
        MOVB R1,@RCL
        MOV  @PAB,R0
        AI   R0,4
        BL   @VSETW
        MOVB @RCL,R1
        MOVB R1,@VDPWD
HOPC1   CLR  @T1                   * no FDR
        BL   @SLINIT
        LI   R1,>8000              * mode: catalog
        BL   @SLMODE
        CLR  R0
        B    @OKEX
HOPFUL  LI   R0,4                  * no free buffer
        B    @ERREX
*  --- normal open ---
HOPFIL  BL   @FNDFDR
        JEQ  HOPHIT
        CI   R0,1                  * an unreadable drive: device error 6
        JNE  HOPMIS
        LI   R0,6
        B    @ERREX
*  Missing: INPUT errors (2); UPDATE/OUTPUT/APPEND create (pinned by the
*  differential gates against the authentic DSR).
HOPMIS  MOVB @FLG,R0
        SRL  R0,8
        ANDI R0,>0006              * the mode bits
        CI   R0,>0004              * INPUT?
        JEQ  HOPE2
        B    @HCREAT               * create + finish the open (write side)
HOPHIT  BL   @RDFDR8               * R2 flags R3 rps R4 eof R5 rcl R6 count
        MOV  R2,R0                 * PROGRAM files cannot be record-opened
        ANDI R0,>0001
        JNE  HOPE2
        MOV  R2,R0                 * a protected file rejects write modes
        ANDI R0,>0008              * at OPEN with error 1 (pinned)
        JEQ  HOPTYP
        MOVB @FLG,R0
        SRL  R0,8
        ANDI R0,>0006
        CI   R0,>0004              * INPUT is allowed
        JEQ  HOPTYP
        LI   R0,1
        B    @ERREX
HOPTYP  MOV  R2,R0                 * VARIABLE bit must match the PAB's
        ANDI R0,>0080
        MOVB @FLG,R7
        SRL  R7,8
        ANDI R7,>0010
        SLA  R7,3
        C    R0,R7
        JNE  HOPE2
        MOVB @RCL,R0               * record length: 0 -> fill in; else match
        SRL  R0,8
        JEQ  HOPFRL
        C    R0,R5
        JNE  HOPE2
        JMP  HOPSLT
HOPFRL  MOV  R5,R1
        SWPB R1
        MOVB R1,@RCL
        MOV  @PAB,R0
        AI   R0,4
        BL   @VSETW
        MOVB @RCL,R1
        MOVB R1,@VDPWD
HOPSLT  MOVB @FLG,R0               * OUTPUT on an existing file: truncate
        SRL  R0,8
        ANDI R0,>0006
        CI   R0,>0002
        JNE  HOPCPY
        BL   @FREECH               * free the old chain bits (disk-based)
        BL   @LDA
        JNE  HOPE2B
        BL   @TRUNCA
        MOV  @T1,@ABS              * write the truncated FDR back
        BL   @WRSEC
        JNE  HOPE2B
HOPCPY  MOV  @SLOT,R3              * cache the FDR into the slot FIRST ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â
        AI   R3,6                  * SLINIT's cursor init must survive it
        LI   R2,>3E00
        LI   R4,248
        BL   @VVCOPY
        BL   @SLINIT
        MOVB @FLG,R1               * mode bits -> slot ext
        SRL  R1,8
        ANDI R1,>0006
        SLA  R1,8
        BL   @SLMODE
        B    @HOPFIN               * mode-specific positioning (write side)
HOPE2B  LI   R0,6
        B    @ERREX
HOPE2   LI   R0,2
        B    @ERREX

*  CLOSE (1): finalize write modes, then free the slot (write side).
HCLOSE  B    @HCLOSB

*  READ (2).
HREAD   BL   @SLFIND
        JNE  HRDE2
        LI   R0,6+>17
        BL   @SLR
        MOVB @VDPRD,R2
        SRL  R2,8
        CI   R2,>80
        JNE  HRDNC
        B    @HCATRD               * (out of relative-jump range)
HRDNC   MOVB @FLG,R0
        SRL  R0,8
        ANDI R0,>0010
        JNE  HRDVAR
*  --- FIX: record @REC via the cached FDR ---
        BL   @SLF8                 * R3 rps, R5 reclen, R6 count
        MOV  @REC,R2
        C    R2,R6
        JHE  HRDE5
        MOV  R5,@LV3               * park reclen
        CLR  R8                    * filesec = rec / rps
        MOV  R2,R9
        DIV  R3,R8                 * R8 = filesec, R9 = remainder
        MPY  R5,R9                 * R9:R10 ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â offset = rem * reclen (R10)
        MOV  R10,@LV2              * park the offset
        MOV  R8,R2
        BL   @SLSEC
        JNE  HRDE6
        MOV  @SLOT,R2              * copy the record to the caller
        AI   R2,254                * slot data buffer (6+248)
        A    @LV2,R2
        MOV  @BUF,R3
        MOV  @LV3,R4
        BL   @VVCOPY
        MOVB @RCL,R1               * PAB+5 = reclen
        BL   @WRCNT
        MOV  @REC,R2               * PAB+6/7 = rec + 1
        INC  R2
        BL   @WRREC
        CLR  R0
        B    @OKEX
HRDE2   LI   R0,7       * unopened PAB: error 7 (pinned)
        B    @ERREX
HRDE5   LI   R0,5
        B    @ERREX
HRDE6   LI   R0,6
        B    @ERREX
*  --- VAR sequential ---
HRDVAR  BL   @VARNXT
        JNE  HRDE5
        CLR  R0
        B    @OKEX

*  RESTORE (4): FIX ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â the PAB record number is the cursor (nothing to
*  do); VAR ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â rewind, then skip PAB+6/7 records forward.
HREST   BL   @SLFIND
        JNE  HRSE2
        MOVB @FLG,R0
        SRL  R0,8
        ANDI R0,>0010
        JEQ  HRSOK
        LI   R0,6+>14
        BL   @SLW
        SETO R1
        MOVB R1,@VDPWD
        MOVB R1,@VDPWD
        CLR  R1
        MOVB R1,@VDPWD
        MOV  @REC,R2
        MOV  R2,@LV3
        JMP  HRSTST
HRSKIP  BL   @VARSKP
        JNE  HRSE5
        DEC  @LV3
HRSTST  MOV  @LV3,R2
        MOV  R2,R2
        JNE  HRSKIP
HRSOK   CLR  R0
        B    @OKEX
HRSE2   LI   R0,7       * unopened PAB: error 7 (pinned)
        B    @ERREX
HRSE5   LI   R0,5
        B    @ERREX

*  STATUS (9): PAB+8 from the FDR (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§3).
HSTAT   BL   @FNDFDR
        JNE  HSTNO
        LI   R0,>3E0C
        BL   @VSETR
        MOVB @VDPRD,R2
        SRL  R2,8
        CLR  R3
        MOV  R2,R0
        ANDI R0,>0001              * program
        JEQ  HST1
        ORI  R3,>0008
HST1    MOV  R2,R0
        ANDI R0,>0002              * internal
        JEQ  HST2
        ORI  R3,>0010
HST2    MOV  R2,R0
        ANDI R0,>0008              * protected
        JEQ  HST3
        ORI  R3,>0040
HST3    MOV  R2,R0
        ANDI R0,>0080              * variable
        JEQ  HSTWR
        ORI  R3,>0004
        JMP  HSTWR
HSTNO   LI   R3,>0080              * no such file
HSTWR   MOV  @PAB,R0
        AI   R0,8
        BL   @VSETW
        SWPB R3
        MOVB R3,@VDPWD
        CLR  R0
        B    @OKEX

*  LOAD (5): PROGRAM image -> VDP at PAB+2; PAB+6/7 = max bytes (input,
*  left unchanged ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§3). Loop state: LV0 index, LV1 dest, LV2
*  sectors, LV3 eof.
HLOAD   BL   @FNDFDR
        JNE  HLDE7
        BL   @RDFDR8               * R2 flags, R4 eof
        MOV  R2,R0
        ANDI R0,>0001
        JEQ  HLDE7                 * not a program file
        MOV  R4,@LV3
        LI   R0,>3E0E              * sectors allocated (BE)
        BL   @RDW
        MOV  R2,@LV2
        MOV  R2,R3                 * size = (sectors-1)*256 + (eof|256)
        DEC  R3
        SLA  R3,8
        MOV  @LV3,R0
        MOV  R0,R0
        JNE  HLDSZ
        LI   R0,256
HLDSZ   A    R0,R3
        C    R3,@REC
        JH   HLDE4                 * larger than the caller's buffer
        CLR  @LV0                  * file-sector index
        MOV  @BUF,R0
        MOV  R0,@LV1               * running destination
HLDLP   MOV  @LV0,R2
        C    R2,@LV2
        JHE  HLDOK
        BL   @CHAIN                * buffer A still holds the FDR
        JNE  HLDE7
        MOV  @LV0,R2               * the last, partial sector?
        INC  R2
        C    R2,@LV2
        JNE  HLDFUL
        MOV  @LV3,R0
        MOV  R0,R0
        JEQ  HLDFUL
        LI   R0,>3F00              * bounce it through buffer B
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  HLDE6
        LI   R2,>3F00
        MOV  @LV1,R3
        MOV  @LV3,R4
        BL   @VVCOPY
        JMP  HLDNXT
HLDFUL  MOV  @LV1,R0
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  HLDE6
HLDNXT  MOV  @LV1,R0
        AI   R0,256
        MOV  R0,@LV1
        INC  @LV0
        JMP  HLDLP
HLDOK   CLR  R0
        B    @OKEX
HLDE7   LI   R0,7
        B    @ERREX
HLDE6   LI   R0,6
        B    @ERREX
HLDE4   LI   R0,4
        B    @ERREX

*  ======================================================================
*  The catalog READ (record @REC of "DSKn." ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€šÃ‚Â RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§3): record 0 =
*  (trimmed volume, 0, total-2, free); r >= 1 = FDIR entry r-1 =
*  (trimmed name, ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â±type, sectors+1, reclen); a zero FDIR entry emits the
*  all-empty end record; r-1 >= 127 = error 5. INTERNAL packing.
*  ======================================================================
HCATRD  MOV  @REC,R0
        MOV  R0,@LV0               * the requested record
        BL   @LDB
        JNE  HCERR6
        CLR  R0                    * VIB -> buffer A
        MOV  R0,@ABS
        LI   R0,>3E00
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  HCERR6
        JMP  HCGO
HCERR6  LI   R0,6                  * near error stubs (jump-range relays)
        B    @ERREX
HCERR5  LI   R0,5
        B    @ERREX
HCGO    MOV  @BUF,R0               * the record write cursor
        MOV  R0,@T1
        MOV  @LV0,R2
        MOV  R2,R2
        JNE  HCATF
        B    @HCATV                * (record 0 lives past jump range)
HCATF   DEC  R2                    * file record r-1
        CI   R2,127
        JHE  HCERR5
        SLA  R2,1
        LI   R0,>3F00
        A    R2,R0
        BL   @VSETR
        MOVB @VDPRD,R3
        MOVB @VDPRD,R4
        SRL  R3,8
        SLA  R3,8
        SRL  R4,8
        SOC  R4,R3                 * the FDR sector (0 = past the files)
        JNE  HCATG
        B    @HCATZ
HCATG   MOV  R3,@ABS
        LI   R0,>3E00
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  HCERR6
        LI   R5,>3E00              * name
        BL   @CATNAM
        BL   @RDFDR8               * R2 flags
        MOV  R2,@LV1               * park the flags
        LI   R3,5                  * type: PROGRAM = 5
        MOV  R2,R0
        ANDI R0,>0001
        JNE  HCATT
        LI   R3,1                  * 1 D/F  2 D/V  3 I/F  4 I/V
        MOV  R2,R0
        ANDI R0,>0002
        JEQ  HCATDV
        LI   R3,3
HCATDV  MOV  R2,R0
        ANDI R0,>0080
        JEQ  HCATT
        INC  R3
HCATT   MOV  @LV1,R0
        ANDI R0,>0008              * protected -> negative
        JEQ  HCATT2
        NEG  R3
HCATT2  MOV  R3,R1
        BL   @CATNUM
        LI   R0,>3E0E              * sectors + 1
        BL   @RDW
        INC  R2
        MOV  R2,R1
        BL   @CATNUM
        LI   R0,>3E11              * record length
        BL   @VSETR
        MOVB @VDPRD,R1
        SRL  R1,8
        BL   @CATNUM
        B    @HCATFN
HCATZ   BL   @CATNM0               * the end record
        CLR  R1
        BL   @CATNUM
        CLR  R1
        BL   @CATNUM
        CLR  R1
        BL   @CATNUM
        B    @HCATFN
HCATV   LI   R5,>3E00              * record 0: the volume
        BL   @CATNAM
        CLR  R1
        BL   @CATNUM
        LI   R0,>3E0A              * total (BE)
        BL   @RDW
        MOV  R2,@LV2               * park the total
        MOV  R2,R1
        AI   R1,-2                 * total - 2 (RECON ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§3)
        BL   @CATNUM
        CLR  R6                    * free = zero bits over `total`
        CLR  R7                    * bitmap byte index; total is a
        MOV  @LV2,R0               * multiple of 8 on every TI format
        SRL  R0,3
        MOV  R0,@LV1               * byte count
HCVFL   C    R7,@LV1
        JHE  HCVFD
        MOV  R7,R0
        AI   R0,>3E38
        BL   @VSETR
        MOVB @VDPRD,R2
        SRL  R2,8
        LI   R3,8
HCVBIT  MOV  R2,R0
        ANDI R0,1
        JNE  HCVB1
        INC  R6
HCVB1   SRL  R2,1
        DEC  R3
        JNE  HCVBIT
        INC  R7
        JMP  HCVFL
HCVFD   MOV  R6,R1
        BL   @CATNUM
HCATFN  LI   R1,>2600              * PAB+5 = 38
        BL   @WRCNT
        MOV  @LV0,R2               * PAB+6/7 = rec + 1
        INC  R2
        BL   @WRREC
        CLR  R0
        B    @OKEX
HCATE5  LI   R0,5
        B    @ERREX
HCATE6  LI   R0,6
        B    @ERREX

*  ======================================================================
*  ======================================================================
*  THE WRITE SIDE (M3/M4): allocation, create/truncate/append, WRITE,
*  SAVE, DELETE, CLOSE finalization, PROTECT, RENAME, FILEIN/FILEOUT,
*  FORMAT. Same call-level discipline; ALLOC-class routines return in R9
*  (may call drivers + R8 utilities; their callers hold nothing in R9).
*  ======================================================================
*  ======================================================================

*  BITSET/BITCLR (R8): set/clear the bitmap bit for sector R3 in the VIB
*  resident in buffer A. Clobbers R0,R2,R4,R5,R10.
BITSET  MOV  R11,R8
        BL   @BITPOS
        SOCB R4,R2                 * set the bit
        JMP  BITWR
BITCLR  MOV  R11,R8
        BL   @BITPOS
        SZCB R4,R2                 * clear the bit (SZCB is dst &= ~src)
BITWR   MOV  R5,R0
        BL   @VSETW
        MOVB R2,@VDPWD
        B    *R8

*  BITPOS (R10, under R8 holders): R3 = sector -> R2 = the bitmap byte
*  (high), R4 = the bit mask (high), R5 = the byte's VDP address.
BITPOS  MOV  R11,R10
        MOV  R3,R5
        SRL  R5,3
        AI   R5,>3E38              * bitmap byte address
        MOV  R5,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        MOV  R3,R4
        ANDI R4,7
        LI   R0,>0100              * 1 << (s & 7), built in the high byte
BITSH   MOV  R4,R4
        JEQ  BITSHD
        SLA  R0,1
        DEC  R4
        JMP  BITSH
BITSHD  MOV  R0,R4
        B    *R10

*  ALLOC (R9): allocate the first free sector >= R2 (wrapping to 2).
*  Loads the VIB, sets the bit, writes the VIB back. Out: R3 = sector,
*  EQ ok; NE = disk full. Clobbers R0..R7,R10 and @LV1.
ALLOC   MOV  R11,R9
        MOV  R2,R8                 * park the start (R8 survives the scan Ã¢â‚¬â€
        BL   @LD0                  *  NEVER an LV cell: those alias FNAME)
        JNE  ALFULL
        LI   R0,>3E0A              * total sectors
        BL   @VSETR
        MOVB @VDPRD,R6
        MOVB @VDPRD,R7
        SRL  R6,8
        SLA  R6,8
        SRL  R7,8
        SOC  R7,R6                 * R6 = total
        MOV  R8,R3                 * phase 1: start..total-1
ALSCAN  C    R3,R6
        JHE  ALWRAP
        BL   @ALTEST
        JEQ  ALGOT
        INC  R3
        JMP  ALSCAN
ALWRAP  LI   R3,2                  * phase 2: 2..start-1
ALSCN2  C    R3,R8
        JHE  ALFULL
        BL   @ALTEST
        JEQ  ALGOT
        INC  R3
        JMP  ALSCN2
ALGOT   BL   @BITSET
        MOV  R3,R6                 * the driver clobbers R3 (R6 survives it)
        BL   @WR0
        JNE  ALFULL
        MOV  R6,R3
        SZC  R0,R0                 * zero + EQ in one op
        B    *R9
ALFULL  SETO R0
        MOV  R0,R0
        B    *R9

*  ALTEST (R10, under R9 holders): EQ if sector R3 is free in the A-VIB.
ALTEST  MOV  R11,R10
        MOV  R3,R5
        SRL  R5,3
        AI   R5,>3E38
        MOV  R5,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        SRL  R2,8
        MOV  R3,R4
        ANDI R4,7
        JEQ  ALTNS
ALTSH   SRL  R2,1
        DEC  R4
        JNE  ALTSH
ALTNS   ANDI R2,1                  * EQ = free
        B    *R10

*  ZBUFA (R8): zero buffer A. ZSLOT (R8): zero the slot data buffer.
ZBUFA   MOV  R11,R8
        LI   R0,>3E00
        JMP  ZCOM
ZSLOT   MOV  R11,R8
        MOV  @SLOT,R0
        AI   R0,254                * slot data buffer (6+248)
ZCOM    BL   @VSETW
        LI   R2,256
        CLR  R1
ZLP     MOVB R1,@VDPWD
        DEC  R2
        JNE  ZLP
        B    *R8

*  CHAPP (R8): append the just-flushed sector R3 to the SLOT FDR copy's
*  cluster chain (extending the last run when contiguous), bump the
*  sectors-allocated count, and update LASTPH. Clobbers R0,R2,R4..R7,R10
*  and @T0,@T2.
CHAPP   MOV  R11,R8
        MOV  R3,@T0                * the new physical sector
*  sectors-so-far = the copy's >0E/0F word = the new sector's file index.
        LI   R0,6+>0E
        BL   @SLR
        MOVB @VDPRD,R6
        MOVB @VDPRD,R7
        SRL  R6,8
        SLA  R6,8
        SRL  R7,8
        SOC  R7,R6                 * R6 = file index of this sector
*  LASTPH + 1 == new? -> extend the last cluster.
        LI   R0,6+>1A
        BL   @SLR
        MOVB @VDPRD,R4
        MOVB @VDPRD,R5
        SRL  R4,8
        SLA  R4,8
        SRL  R5,8
        SOC  R5,R4                 * LASTPH
        INC  R4
        C    R4,@T0
        JNE  CHNEW
*  Extend: find the last (nonzero) cluster entry, rewrite its end offset.
        BL   @CHFIND               * R5 = offset of the terminator
        AI   R5,-3                 * -> the last entry
        MOV  @SLOT,R0
        AI   R0,6
        A    R5,R0
        MOV  R0,@T2                * entry address
        BL   @VSETR
        MOVB @VDPRD,R2             * b0 (keep)
        MOVB @VDPRD,R4             * b1: start-hi | end[3:0]<<4
        SRL  R4,8
        ANDI R4,>000F              * keep the start-hi nibble
        MOV  R6,R7                 * new end offset = R6
        MOV  R6,R0
        ANDI R0,>000F
        SLA  R0,4
        SOC  R0,R4                 * new b1
        MOV  @T2,R0
        INC  R0
        BL   @VSETW
        MOV  R4,R1
        SWPB R1
        MOVB R1,@VDPWD             * b1
        MOV  R7,R1
        SRL  R1,4
        SWPB R1
        MOVB R1,@VDPWD             * b2 = end >> 4
        JMP  CHCNT
*  New cluster entry at the terminator.
CHNEW   BL   @CHFIND
        MOV  @SLOT,R0
        AI   R0,6
        A    R5,R0
        BL   @VSETW
        MOV  @T0,R1
        SWPB R1
        MOVB R1,@VDPWD             * b0 = start low
        MOV  @T0,R2
        SRL  R2,8
        ANDI R2,>000F              * start hi nibble
        MOV  R6,R0
        ANDI R0,>000F
        SLA  R0,4
        SOC  R0,R2                 * | end[3:0] << 4
        SWPB R2
        MOVB R2,@VDPWD             * b1
        MOV  R6,R1
        SRL  R1,4
        SWPB R1
        MOVB R1,@VDPWD             * b2
CHCNT   INC  R6                    * sectors-allocated += 1
        LI   R0,6+>0E
        BL   @SLW
        MOV  R6,R1
        MOVB R1,@VDPWD
        SWPB R1
        MOVB R1,@VDPWD
        LI   R0,6+>1A
        BL   @SLW
        MOVB @T0,R1
        MOVB R1,@VDPWD
        MOVB @T0+1,R1
        MOVB R1,@VDPWD
        B    *R8

*  CHFIND (R10, under R8 holders): R5 = byte offset (>1C..) of the first
*  all-zero cluster entry in the SLOT FDR copy. Clobbers R0,R2,R4.
CHFIND  MOV  R11,R10
        LI   R5,>001C
CHFLP   MOV  @SLOT,R0
        AI   R0,6
        A    R5,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R4
        SOCB R4,R2
        MOVB @VDPRD,R4
        SOCB R4,R2
        SRL  R2,8
        JEQ  CHFD
        AI   R5,3
        CI   R5,>00F4              * the 248-byte copy bound
        JL   CHFLP
CHFD    B    *R10

*  FLUSHV (@RETF): write the slot data buffer out ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â rewriting the chain
*  tail when TFLAG says the buffer IS the tail, else allocating a fresh
*  sector and appending it to the chain. EQ ok.
FLUSHV  MOV  R11,@RETF
        LI   R0,6+>18
        BL   @SLR
        MOVB @VDPRD,R2
        SRL  R2,8
        JEQ  FLNEW
*  Tail rewrite: ABS := LASTPH; clear TFLAG.
        LI   R0,6+>1A
        BL   @SLR
        MOVB @VDPRD,R4
        MOVB @VDPRD,R5
        SRL  R4,8
        SLA  R4,8
        SRL  R5,8
        SOC  R5,R4
        MOV  R4,@ABS
        LI   R0,6+>18
        BL   @SLW
        CLR  R1
        MOVB R1,@VDPWD
        JMP  FLWR
FLNEW   LI   R2,>0022              * data sectors allocate from >22
        BL   @ALLOC
        JNE  FLBAD
        MOV  R3,@ABS
        BL   @CHAPP
FLWR    MOV  @SLOT,R0
        AI   R0,254                * slot data buffer (6+248)
        MOV  R0,@DMA
        BL   @WRSEC
        JNE  FLBAD
        CLR  R0
        JMP  FLRET
FLBAD   SETO R0
FLRET   MOV  @RETF,R11             * load the return FIRST Ã¢â‚¬â€ the flag test
        MOV  R0,R0                 * must be the LAST thing before B *R11
        B    *R11

*  WFDR (@RETF): write the slot's FDR copy back to its disk sector, with
*  the ext-state bytes (>14..>1B) and the uncopied tail (>F8..>FF) zeroed.
WFDR    MOV  R11,@RETF
        BL   @SLTOA                * copy -> buffer A (248 bytes)
        LI   R0,>3E14              * zero the ext bytes
        BL   @VSETW
        CLR  R1
        LI   R2,8
WFZ1    MOVB R1,@VDPWD
        DEC  R2
        JNE  WFZ1
        LI   R0,>3EF8              * zero the uncopied tail
        BL   @VSETW
        LI   R2,8
WFZ2    MOVB R1,@VDPWD
        DEC  R2
        JNE  WFZ2
        MOV  @SLOT,R0              * the FDR sector from the control block
        AI   R0,2
        BL   @RDW
        MOV  R2,@ABS
        LI   R0,>3E00
        MOV  R0,@DMA
        BL   @WRSEC
        JNE  WFBAD
        CLR  R0
        JMP  WFRET
WFBAD   SETO R0
WFRET   MOV  @RETF,R11             * return first, flag test last
        MOV  R0,R0
        B    *R11

*  FREECH (@RETV): free every cluster run of the FDR at @T1 in the VIB
*  (re-reading the FDR per cluster; each run's VIB update written back).
*  Owns LV1..LV3 + T0.
FREECH  MOV  R11,@RETF
        CLR  @T2                   * cluster index k (NOT an FNAME alias Ã¢â‚¬â€
        SETO @T0                   * previous end = -1   callers need FNAME)
FRLP    BL   @LDA                  * FDR -> A
        JNE  FRDONE
        MOV  @T2,R5                * cluster k -> byte offset 3k
        MOV  R5,R6
        SLA  R6,1
        A    R5,R6                 * R6 = 3k
        LI   R0,>3E1C
        A    R6,R0
        BL   @VSETR
        MOVB @VDPRD,R2             * b0
        MOVB @VDPRD,R3             * b1
        MOVB @VDPRD,R4             * b2
        SRL  R2,8
        SRL  R3,8
        SRL  R4,8
        MOV  R2,R5
        SOC  R3,R5
        SOC  R4,R5
        JEQ  FRDONE                * zero cluster = end
        MOV  R3,R5                 * start = (b1 & 0F)<<8 | b0
        ANDI R5,>000F
        SLA  R5,8
        SOC  R2,R5
        MOV  R4,R6                 * end = b2<<4 | b1>>4
        SLA  R6,4
        SRL  R3,4
        SOC  R3,R6
        MOV  R6,R9                 * R9 = last physical = start+(end-prev)-1
        S    @T0,R9
        DEC  R9
        A    R5,R9
        MOV  R6,@T0                * previous end := this end
        MOV  R5,R6                 * park start (R6/R9 survive the drivers)
        BL   @LD0
        JNE  FRDONE
        MOV  R6,R3
FRBITS  BL   @BITCLR
        INC  R3
        C    R3,R9
        JLE  FRBITS
        BL   @WR0
        INC  @T2
        JMP  FRLP
FRDONE  MOV  @RETF,R11
        B    *R11

*  TRUNCA (R8): reset the FDR in buffer A to an empty file ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â zero the
*  sectors count, eof, record count, and the whole cluster chain.
TRUNCA  MOV  R11,R8
        LI   R0,>3E0E
        BL   @VSETW
        CLR  R1
        MOVB R1,@VDPWD             * >0E/>0F sectors
        MOVB R1,@VDPWD
        MOVB R1,@VDPWD             * >10 eof
        LI   R0,>3E12
        BL   @VSETW
        MOVB R1,@VDPWD             * >12/>13 count
        MOVB R1,@VDPWD
        LI   R0,>3E1C
        BL   @VSETW
        LI   R2,228                * >1C..>FF
TRZ     MOVB R1,@VDPWD
        DEC  R2
        JNE  TRZ
        B    *R8

*  FDIINS (@RETV): insert @T1 into the FDIR, sorted by FNAME. The FDIR
*  works in B; candidate names are read through A. Owns @LV1.
FDIINS  MOV  R11,@RETF
        BL   @LDB
        CLR  R6                    * entry index
FDILP   MOV  R6,R0
        SLA  R0,1
        AI   R0,>3F00
        BL   @RDW                 * the entry's FDR sector
        JEQ  FDIAT                 * empty: insert here (at the end)
        MOV  R2,@ABS               * read its name (R6 survives the driver)
        LI   R0,>3E00
        MOV  R0,@DMA
        BL   @RDSEC
        LI   R0,>3E00
        BL   @VSETR
        CLR  R4
FDICMP  MOVB @VDPRD,R2
        MOVB @FNAME(R4),R3
        CB   R3,R2
        JL   FDIAT                 * FNAME < entry: insert before it
        JH   FDINXT
        INC  R4
        CI   R4,10
        JL   FDICMP
FDINXT  INC  R6                    * FNAME >= entry: keep scanning
        CI   R6,127
        JL   FDILP
FDIAT   MOV  R6,R7                 * R6 = insertion index; find the end
FDIEND  MOV  R7,R0
        SLA  R0,1
        AI   R0,>3F00
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        SOCB R3,R2
        SRL  R2,8
        JEQ  FDISHF
        INC  R7
        CI   R7,127
        JL   FDIEND
FDISHF  MOV  R7,R5                 * shift [ins..end) up one entry
FDISLP  C    R5,R6
        JLE  FDIPUT
        MOV  R5,R0
        SLA  R0,1
        AI   R0,>3EFE              * source = >3F00 + 2(i-1)
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        MOV  R5,R0
        SLA  R0,1
        AI   R0,>3F00
        BL   @VSETW
        MOVB R2,@VDPWD
        MOVB R3,@VDPWD
        DEC  R5
        JMP  FDISLP
FDIPUT  MOV  R6,R0
        SLA  R0,1
        AI   R0,>3F00
        BL   @VSETW
        MOVB @T1,R1
        MOVB R1,@VDPWD
        MOVB @T1+1,R1
        MOVB R1,@VDPWD
        BL   @WRB
        MOV  @RETF,R11
        B    *R11

*  FDIREM (@RETV): remove the FDIR entry equal to @T1.
FDIREM  MOV  R11,@RETF
        BL   @LDB
        CLR  R6
FDRLP   MOV  R6,R0
        SLA  R0,1
        AI   R0,>3F00
        BL   @RDW
        JEQ  FDRWRT                * hit the end without a match
        C    R2,@T1
        JEQ  FDRSH
        INC  R6
        CI   R6,127
        JL   FDRLP
        JMP  FDRWRT
FDRSH   MOV  R6,R5                 * shift everything after it down
FDRSLP  MOV  R5,R0
        SLA  R0,1
        AI   R0,>3F02              * source = next entry
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        MOV  R5,R0
        SLA  R0,1
        AI   R0,>3F00
        BL   @VSETW
        MOVB R2,@VDPWD
        MOVB R3,@VDPWD
        SOCB R3,R2
        SRL  R2,8
        JEQ  FDRWRT                * copied the terminator
        INC  R5
        CI   R5,127
        JL   FDRSLP
FDRWRT  LI   R0,1
        MOV  R0,@ABS
        LI   R0,>3F00
        MOV  R0,@DMA
        BL   @WRSEC
        MOV  @RETF,R11
        B    *R11

*  ======================================================================
*  HCREAT: OPEN's create path (OUTPUT/APPEND on a missing file). Build
*  the fresh FDR in A, allocate + write it, insert into the FDIR, then
*  join the normal open tail at HOPCPY.
*  ======================================================================
HCREAT  MOVB @RCL,R0               * default record length 80
        SRL  R0,8
        JNE  HCRL
        LI   R1,>5000
        MOVB R1,@RCL
        MOV  @PAB,R0
        AI   R0,4
        BL   @VSETW
        MOVB @RCL,R1
        MOVB R1,@VDPWD
HCRL    MOVB @FLG,R2               * flags: VAR (>10 -> >80), INT (>08 -> >02)
        SRL  R2,8
        CLR  R3
        MOV  R2,R0
        ANDI R0,>0010
        JEQ  HCRI
        ORI  R3,>0080
HCRI    MOV  R2,R0
        ANDI R0,>0008
        JEQ  HCRAL
        ORI  R3,>0002
HCRAL   MOV  R3,@T0                * park the flags
        CLR  R2                    * allocate the FDR sector (from 0)
        BL   @ALLOC
        JNE  HCRE4
        MOV  R3,@T1
*  Build the FDR in A (ALLOC left the VIB there ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â start clean).
        BL   @ZBUFA
        LI   R0,>3E00              * the name
        BL   @VSETW
        CLR  R4
HCRNM   MOVB @FNAME(R4),R1
        MOVB R1,@VDPWD
        INC  R4
        CI   R4,10
        JL   HCRNM
*  An UPDATE-mode create leaves every FDR field zero â€” name only (pinned:
*  the authentic DSR defers the attributes; dsr_open_modes_match).
        MOVB @FLG,R0
        SRL  R0,8
        ANDI R0,>0006
        JEQ  HCRSKF
        MOVB @RCL,R5               * rps = 256 / (rl [+1 for VAR])
        SRL  R5,8
        MOV  R5,R6
        MOV  @T0,R0
        ANDI R0,>0080
        JEQ  HCRDIV
        INC  R6
HCRDIV  CLR  R4
        LI   R5,256
        DIV  R6,R4                 * R4 = rps
        LI   R0,>3E0C
        BL   @VSETW
        MOVB @T0+1,R1              * flags (low byte of T0)
        MOVB R1,@VDPWD
        MOV  R4,R1
        SWPB R1
        MOVB R1,@VDPWD             * recs/sector
        LI   R0,>3E11
        BL   @VSETW
        MOVB @RCL,R1
        MOVB R1,@VDPWD             * record length
HCRSKF  BL   @WRA
        JNE  HCRE6
        BL   @FDIINS
*  The FDR must be back in A for the open tail (FDIINS clobbered it).
        BL   @LDA
        JNE  HCRE6
        B    @HOPCPY
HCRE4   LI   R0,4
        B    @ERREX
HCRE6   LI   R0,6
        B    @ERREX

*  HOPFIN: the open tail ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â mode-specific positioning, then done.
HOPFIN  MOV  @SLOT,R0              * seed LASTPH from the chain so a later
        AI   R0,6+>0E              * flush merges with the on-disk tail run
        BL   @RDW                  * R2 = sectors allocated
        MOV  R2,R6                 * (SLTOA clobbers R2 Ã¢â‚¬â€ park in R6)
        JEQ  HOPFM
        DEC  R6                    * physical of the last file sector
        BL   @SLTOA
        MOV  R6,R2
        BL   @CHAIN
        JNE  HOPFM
        LI   R0,6+>1A
        BL   @SLW
        MOVB @ABS,R1
        MOVB R1,@VDPWD
        MOVB @ABS+1,R1
        MOVB R1,@VDPWD
HOPFM   MOVB @FLG,R2
        SRL  R2,8
        MOV  R2,R3
        ANDI R3,>0006              * mode
        MOV  R2,R4
        ANDI R4,>0010              * VAR?
        JEQ  HOPFW                 * FIX: nothing to position
        CI   R3,>0006              * VAR + APPEND?
        JNE  HOPFW
*  APPEND: if the file has sectors, reopen the tail sector for filling.
        BL   @VNCNT                * R5 = the VAR sector count
        MOV  R5,R5
        JEQ  HOPFW
        MOV  R5,R2
        DEC  R2
        BL   @VNLOAD               * tail -> the slot buffer (sets cursec)
        JNE  HOPFE6
        LI   R0,6+>10
        BL   @SLR
        MOVB @VDPRD,R2
        SRL  R2,8
        MOV  R2,@T0
        LI   R0,6+>16
        BL   @SLW
        MOVB @T0+1,R1              * curoff (low byte of T0)
        MOVB R1,@VDPWD
        MOVB @FLG,R1               * >17 mode (rewritten in sequence)
        SRL  R1,8
        ANDI R1,>0006
        SLA  R1,8
        MOVB R1,@VDPWD
        LI   R1,>0100
        MOVB R1,@VDPWD             * >18 TFLAG = 1
        CLR  R1
        MOVB R1,@VDPWD             * >19
        MOVB @ABS,R1               * >1A/1B LASTPH = the tail's physical
        MOVB R1,@VDPWD
        MOVB @ABS+1,R1
        MOVB R1,@VDPWD
        JMP  HOPFOK
HOPFW   CI   R3,>0004              * write modes get a clean fill buffer
        JEQ  HOPFOK                * (INPUT keeps whatever loads later)
        BL   @ZSLOT
HOPFOK  CLR  R0
        B    @OKEX
HOPFE6  LI   R0,6
        B    @ERREX

*  ======================================================================
*  WRITE (3)
*  ======================================================================
HWRITB  BL   @SLFIND
        JNE  HWRE2
        LI   R0,6+>17
        BL   @SLR
        MOVB @VDPRD,R2
        SRL  R2,8
        CI   R2,>80
        JEQ  HWRE3
        MOVB @FLG,R0
        SRL  R0,8
        ANDI R0,>0010
        JEQ  HWRFX0
*  --- VAR append: length byte + data into the fill buffer ---
        MOVB @CNT,R2
        SRL  R2,8                  * record length
        MOV  R2,@LV2
        LI   R0,6+>16
        BL   @SLR
        MOVB @VDPRD,R3
        SRL  R3,8
        MOV  R3,@LV1
        A    R2,R3                 * fits? curoff + len + 2 <= 256
        AI   R3,2
        CI   R3,256
        JLE  HWVFIT
        MOV  @SLOT,R0              * seal the sector with >FF
        AI   R0,254                * slot data buffer (6+248)
        A    @LV1,R0
        BL   @VSETW
        LI   R1,>FF00
        MOVB R1,@VDPWD
        LI   R0,6+>10              * record this sector's eof (a close
        BL   @SLW                  *  after a sector-exact fill keeps it)
        MOVB @LV1+1,R1
        MOVB R1,@VDPWD
        BL   @FLUSHV
        JNE  HWRE6
        BL   @ZSLOT
        CLR  @LV1
HWVFIT  MOV  @SLOT,R0              * the length byte
        AI   R0,254                * slot data buffer (6+248)
        A    @LV1,R0
        MOV  R0,@T0
        BL   @VSETW
        MOVB @LV2+1,R1
        MOVB R1,@VDPWD
        MOV  @LV2,R4               * copy the record bytes
        JEQ  HWVUPD
        MOV  @T0,R3
        INC  R3
        MOV  @BUF,R2
        BL   @VVCOPY
HWVUPD  MOV  @LV1,R0               * curoff += len + 1
        A    @LV2,R0
        INC  R0
        MOV  R0,@LV1
        LI   R0,6+>16
        BL   @SLW
        MOVB @LV1+1,R1
        MOVB R1,@VDPWD
        CLR  R0
        B    @OKEX
HWRE2   LI   R0,7       * unopened PAB: error 7 (pinned)
        B    @ERREX
HWRE3   LI   R0,3
        B    @ERREX
HWRE6   LI   R0,6
        B    @ERREX
HWRE4   LI   R0,4
        B    @ERREX
HWRFX0  JMP  HWRFIX
*  --- FIX write-through: record @REC, extending the chain as needed ---
HWRFIX  BL   @SLF8                 * R3 rps, R5 reclen, R6 count(records)
        MOV  R5,@LV3               * reclen
        MOV  @REC,R2
        CLR  R8
        MOV  R2,R9
        DIV  R3,R8                 * R8 = file sector, R9 = remainder
        MPY  R5,R9                 * offset in R10
        MOV  R10,@LV2
        MOV  R8,@LV0               * needed file-sector index
*  Extend with zero sectors while the chain is short.
HWFEXT  MOV  @SLOT,R0              * sectors so far
        AI   R0,6+>0E
        BL   @RDW
        C    @LV0,R2
        JL   HWFRES                * resident range: load it
        BL   @ZSLOT
        LI   R2,>0022
        BL   @ALLOC
        JNE  HWRE4
        MOV  R3,@ABS
        BL   @CHAPP
        MOV  @SLOT,R0              * write the zero sector out
        AI   R0,254                * slot data buffer (6+248)
        MOV  R0,@DMA
        BL   @WRSEC
        JNE  HWRE6
        JMP  HWFEXT
HWFRES  MOV  @LV0,R2               * load/locate the target sector
        BL   @SLSEC
        JNE  HWRE6
        MOV  @SLOT,R2              * copy the record in
        AI   R2,254                * slot data buffer (6+248)
        A    @LV2,R2
        MOV  R2,R3
        MOV  @BUF,R2
        MOV  @LV3,R4
        BL   @VVCOPY
        MOV  @SLOT,R0              * write-through
        AI   R0,254                * slot data buffer (6+248)
        MOV  R0,@DMA
        BL   @WRSEC
        JNE  HWRE6
        BL   @VNCNT                * records = max(records, rec+1)
        MOV  @REC,R2
        INC  R2
        C    R2,R5
        JLE  HWFREC
        LI   R0,6+>12
        BL   @SLW
        MOV  R2,R1
        SWPB R1
        MOVB R1,@VDPWD             * low byte (LE on disk)
        SWPB R1
        MOVB R1,@VDPWD             * high byte
HWFREC  MOV  @REC,R2
        INC  R2
        BL   @WRREC
        CLR  R0
        B    @OKEX

*  ======================================================================
*  CLOSE (1), full version: finalize write modes, refresh the info
*  record, free the slot.
*  ======================================================================
HCLOSB  BL   @SLFIND
        JNE  HCLOK
        LI   R0,6+>17
        BL   @SLR
        MOVB @VDPRD,R2
        SRL  R2,8
        CI   R2,>0080              * catalog: nothing to finalize
        JEQ  HCLFRE
        CI   R2,>0004              * INPUT: nothing to finalize
        JEQ  HCLFRE
*  VAR OUTPUT/APPEND: seal the tail sector and set the EOF offset.
        LI   R0,6+>0C
        BL   @SLR
        MOVB @VDPRD,R3
        SRL  R3,8
        ANDI R3,>0080
        JEQ  HCLFDR                * FIX: the copy is already current
        CI   R2,>0002
        JEQ  HCLVAR
        CI   R2,>0006
        JNE  HCLFDR                * VAR UPDATE: nothing buffered
HCLVAR  MOV  @SLOT,R0              * eof := curoff
        AI   R0,6+>16
        BL   @VSETR
        MOVB @VDPRD,R4
        SRL  R4,8
        JEQ  HCLFDR                * nothing buffered (an empty file or a
        MOV  R4,@T0                *  sector-exact tail): skip the flush
        MOV  @SLOT,R0
        AI   R0,254                * slot data buffer (6+248)
        A    R4,R0
        BL   @VSETW
        LI   R1,>FF00              * the sector terminator
        MOVB R1,@VDPWD
        LI   R0,6+>10
        BL   @SLW
        MOVB @T0+1,R1              * eof (low byte of T0)
        MOVB R1,@VDPWD
        BL   @FLUSHV
        JNE  HCLE6
*  The VAR level-3 count (>12/13, LE) = the sector count (>0E/0F).
        MOV  @SLOT,R0
        AI   R0,6+>0E
        BL   @RDW                  * R2 = sectors allocated
        LI   R0,6+>12
        BL   @SLW
        MOV  R2,R1
        SWPB R1
        MOVB R1,@VDPWD             * low byte first (LE on disk)
        SWPB R1
        MOVB R1,@VDPWD
HCLFDR  BL   @WFDR                 * FDR back to disk (leaves A = the FDR)
        JNE  HCLE6
        BL   @WINFO                * refresh the >8356 info record
HCLFRE  MOV  @SLOT,R0              * free the slot
        AI   R0,5
        BL   @VSETW
        CLR  R1
        MOVB R1,@VDPWD
HCLOK   CLR  R0
        B    @OKEX
HCLE6   LI   R0,6
        B    @ERREX

*  ======================================================================
*  SAVE (6): create/replace a PROGRAM file of PAB+6/7 bytes from PAB+2.
*  Strategy (works on fragmented disks): place the FDR skeleton on disk
*  first (name, PROGRAM flags, eof, zero chain), then per data sector:
*  allocate, append/merge into the on-disk FDR's chain (HSVAPP), write
*  the data. The disk FDR is complete when the loop ends.
*  ======================================================================
HSAVEB  BL   @FNDFDR
        JNE  HSVNEW
        BL   @FREECH               * replace: free the old chain
        JMP  HSVSKL                * (@T1 keeps the reused FDR sector)
HSVNEW  CLR  R2                    * new FDR sector (from 0)
        BL   @ALLOC
        JNE  HSVE4M
        MOV  R3,@T1
        BL   @FDIINS               * join the directory (FNAME is set)
*  --- the FDR skeleton, written to disk up front ---
HSVSKL  BL   @ZBUFA
        LI   R0,>3E00
        BL   @VSETW
        CLR  R4
HSVNM   MOVB @FNAME(R4),R1
        MOVB R1,@VDPWD
        INC  R4
        CI   R4,10
        JL   HSVNM
        LI   R0,>3E0C
        BL   @VSETW
        LI   R1,>0100              * flags = PROGRAM
        MOVB R1,@VDPWD
        LI   R0,>3E10
        BL   @VSETW
        MOV  @REC,R1
        ANDI R1,>00FF              * eof = len % 256
        SWPB R1
        MOVB R1,@VDPWD
        BL   @WRA
        JNE  HSVE6M
*  --- the data loop ---
        MOV  @REC,R2               * sectors = ceil(len / 256)
        AI   R2,255
        SRL  R2,8
        MOV  R2,@LV2
        CLR  @LV0                  * file-sector index
        MOV  @BUF,R0
        MOV  R0,@LV1               * running source
        SETO @T2                   * last physical (merge tracking)
        JMP  HSVLP
HSVE4M  LI   R0,4                  * near relays for the loop below
        B    @ERREX
HSVE6M  LI   R0,6
        B    @ERREX
HSVLP   MOV  @LV0,R2
        C    R2,@LV2
        JHE  HSVOK
        LI   R2,>0022
        BL   @ALLOC
        JNE  HSVE4M
        MOV  R3,@LV3               * park the new physical sector
        BL   @LDA
        JNE  HSVE6M
        MOV  @LV3,R6               * append/merge + count bump
        MOV  @LV0,R7
        BL   @HSVAPP
        BL   @WRA
        JNE  HSVE6M
        MOV  @LV3,R2
        MOV  R2,@T2                * last physical := this one
        MOV  R2,@ABS               * now the data sector itself
*  Every sector â€” the final partial included â€” is written as a full 256
*  bytes straight from the VDP source (bytes past the image length come
*  from VRAM), matching the authentic SAVE (pinned: dsr_save_matches).
        MOV  @LV1,R0
        MOV  R0,@DMA
        BL   @WRSEC
        JNE  HSVE6M
        MOV  @LV1,R0
        AI   R0,256
        MOV  R0,@LV1
        INC  @LV0
        JMP  HSVLP
HSVOK   CLR  R0
        B    @OKEX
HSVE4   LI   R0,4
        B    @ERREX
HSVE6   LI   R0,6
        B    @ERREX

*  HSVAPP (R8): append physical sector R6 (file index R7) to the chain
*  of the FDR in buffer A, merging into the last run when contiguous
*  (@T2 = the previous physical sector), and bump the >0E/F count.
HSVAPP  MOV  R11,R8
        MOV  @T2,R2
        INC  R2
        C    R2,R6
        JNE  HSVANW
*  Contiguous: rewrite the last entry's end offset (find the terminator).
        LI   R5,>001C
HSVAF   LI   R0,>3E00
        A    R5,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        SOCB R3,R2
        MOVB @VDPRD,R3
        SOCB R3,R2
        SRL  R2,8
        JEQ  HSVAFD
        AI   R5,3
        CI   R5,>00F4
        JL   HSVAF
HSVAFD  AI   R5,-3                 * the last entry
        LI   R0,>3E00
        A    R5,R0
        INC  R0
        MOV  R0,R10                * park the b1 address
        BL   @VSETR
        MOVB @VDPRD,R2             * b1
        SRL  R2,8
        ANDI R2,>000F
        MOV  R7,R0
        ANDI R0,>000F
        SLA  R0,4
        SOC  R0,R2
        MOV  R10,R0
        BL   @VSETW
        MOV  R2,R1
        SWPB R1
        MOVB R1,@VDPWD             * b1
        MOV  R7,R1
        SRL  R1,4
        SWPB R1
        MOVB R1,@VDPWD             * b2
        JMP  HSVACN
*  New entry at the terminator.
HSVANW  LI   R5,>001C
HSVAN   LI   R0,>3E00
        A    R5,R0
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        SOCB R3,R2
        MOVB @VDPRD,R3
        SOCB R3,R2
        SRL  R2,8
        JEQ  HSVAND
        AI   R5,3
        CI   R5,>00F4
        JL   HSVAN
HSVAND  LI   R0,>3E00
        A    R5,R0
        BL   @VSETW
        MOV  R6,R1
        SWPB R1
        MOVB R1,@VDPWD             * b0
        MOV  R6,R2
        SRL  R2,8
        ANDI R2,>000F
        MOV  R7,R0
        ANDI R0,>000F
        SLA  R0,4
        SOC  R0,R2
        SWPB R2
        MOVB R2,@VDPWD             * b1
        MOV  R7,R1
        SRL  R1,4
        SWPB R1
        MOVB R1,@VDPWD             * b2
HSVACN  LI   R0,>3E0E              * sectors += 1
        BL   @RDW
        INC  R2
        LI   R0,>3E0E
        BL   @VSETW
        MOV  R2,R1
        MOVB R1,@VDPWD
        SWPB R1
        MOVB R1,@VDPWD
        B    *R8

*  ======================================================================
*  DELETE (7)
*  ======================================================================
HDELB   BL   @FNDFDR
        JNE  HDLOK                 * missing: silent success (pinned)
        LI   R0,>3E0C              * a protected file refuses deletion
        BL   @VSETR                * with error 1 (pinned)
        MOVB @VDPRD,R2
        SRL  R2,8
        ANDI R2,>0008
        JEQ  HDLGO
        LI   R0,1
        B    @ERREX
HDLGO   BL   @FREECH               * free the data chain (VIB rewritten)
        BL   @LD0
        JNE  HDLE6
        MOV  @T1,R3
        BL   @BITCLR
        BL   @WR0
        JNE  HDLE6
        BL   @FDIREM
HDLOK   CLR  R0
        B    @OKEX
HDLE6   LI   R0,6
        B    @ERREX

*  ======================================================================
*  Subprograms: PROTECT (>12), RENAME (>13), FILEIN (>14), FILEOUT (>15),
*  FORMAT (>11). Entered like SECIO; error byte to >8350; skip return.
*  ======================================================================

*  VNAME (R8): copy the 10-byte name at VDP R2 into FNAME.
VNAME   MOV  R11,R8
        MOV  R2,R0
        BL   @VSETR
        LI   R2,FNAME
        LI   R3,10
VNMLP   MOVB @VDPRD,*R2+
        DEC  R3
        JNE  VNMLP
        B    *R8

SUBOK   CLR  R1                    * shared subprogram exits
        JMP  SUBST
SUBER6  LI   R1,>0600
        JMP  SUBST
SUBER7  LI   R1,>0700
SUBST   MOVB R1,@T0
        MOV  @R1SV,R1
        MOV  @RETH,R11
        INCT R11
        B    *R11

*  PROTECT: flag @>834D (0 clear / else set), name at VDP @>834E.
SPROT   MOV  R11,@RETH
        MOV  R1,@R1SV
        MOVB @T3,R6
        SRL  R6,8                  * the flag (R6 survives the lookup)
        MOV  @DMA,R2               * the VDP name pointer
        BL   @VNAME
        BL   @FNDFDR
        JNE  SUBER7
        LI   R0,>3E0C              * RMW the protected bit
        BL   @VSETR
        MOVB @VDPRD,R2
        SRL  R2,8
        ANDI R2,>00F7
        MOV  R6,R6
        JEQ  SPRWR
        ORI  R2,>0008
SPRWR   LI   R0,>3E0C
        BL   @VSETW
        MOV  R2,R1
        SWPB R1
        MOVB R1,@VDPWD
        BL   @WRA
        JNE  SUBER6
        JMP  SUBOK

*  RENAME: old name at VDP @>8350, new name at VDP @>834E.
SREN    MOV  R11,@RETH
        MOV  R1,@R1SV
        MOV  @T0,R2                * the OLD name
        BL   @VNAME
        BL   @FNDFDR
        JNE  SUB7A
        MOV  @T1,R6                * park the FDR sector (R6 survives all)
        MOV  @DMA,R2               * the NEW name -> FNAME
        BL   @VNAME
        BL   @FNDFDR               * duplicate?
        JEQ  SUB6A
        MOV  R6,@T1
        BL   @LDA
        JNE  SUB6A
        LI   R0,>3E00
        BL   @VSETW
        CLR  R4
SRNNM   MOVB @FNAME(R4),R1
        MOVB R1,@VDPWD
        INC  R4
        CI   R4,10
        JL   SRNNM
        BL   @WRA
        JNE  SUB6A
        BL   @FDIREM               * re-sort: remove + insert
        BL   @FDIINS
        JMP  SUBOKA
SUB6A   LI   R1,>0600              * near subprogram exits (jump range)
        JMP  SUBSTA
SUB7A   LI   R1,>0700
        JMP  SUBSTA
SUBOKA  CLR  R1
SUBSTA  B    @SUBST              * shared subprogram exit

*  FILEIN (>14): N @>834D sectors of the file named at VDP @>834E into
*  the info block's VDP buffer; N = 0 returns the file info instead.
*  The info block is CPU RAM at >8300 + the byte at >8350.
SFIN    MOV  R11,@RETH
        MOV  R1,@R1SV
        BL   @SFBLK                * R7 = block base (parked in @T2)
        MOV  @DMA,R2
        BL   @VNAME
        BL   @FNDFDR
        JNE  SUB7A
        MOVB @T3,R2
        SRL  R2,8                  * N
        JNE  SFINRD
*  Info request: fill the block from the FDR.
        MOV  @T2,R7
        LI   R0,>3E0E              * +2/3 := sector count
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB @VDPRD,R3
        MOVB R2,@2(R7)
        MOVB R3,@3(R7)
        LI   R0,>3E0C              * +4 := flags, +5 := recs/sector
        BL   @VSETR
        MOVB @VDPRD,R2
        MOVB R2,@4(R7)
        MOVB @VDPRD,R2
        MOVB R2,@5(R7)
        LI   R0,>3E10
        BL   @VSETR
        MOVB @VDPRD,R2             * +6 := eof
        MOVB R2,@6(R7)
        MOVB @VDPRD,R2             * +7 := reclen
        MOVB R2,@7(R7)
        MOVB @VDPRD,R2             * +8/9 := records (LE undone -> BE)
        MOVB @VDPRD,R3
        MOVB R3,@8(R7)
        MOVB R2,@9(R7)
        JMP  SUBOKA
*  Sector read: N sectors from file-sector [blk+2] to VDP [blk+0].
SFINRD  MOV  R2,@LV0               * N
        MOV  @T2,R7
        MOV  *R7,R4                * VDP dest
        MOV  @2(R7),R5             * first file sector
        MOV  R4,@LV1
        MOV  R5,@LV2
SFINLP  MOV  @LV0,R0
        MOV  R0,R0
        JEQ  SUBOKA
        MOV  @LV2,R2
        BL   @CHAIN                * (the FDR is resident in A)
        JNE  SUB7A
        MOV  @LV1,R0
        MOV  R0,@DMA
        BL   @RDSEC
        JNE  SUB6A
        BL   @LDA
        JNE  SUB6A
        MOV  @LV1,R0
        AI   R0,256
        MOV  R0,@LV1
        INC  @LV2
        DEC  @LV0
        JMP  SFINLP

*  SFBLK (R8): R7 = >8300 + the byte at >8350; also parked in @T2.
SFBLK   MOV  R11,R8
        MOVB @T0,R7
        SRL  R7,8
        AI   R7,>8300
        MOV  R7,@T2
        B    *R8

*  FILEOUT (>15): N = 0 creates a file from the info block; N > 0 writes
*  N sectors from VDP [blk+0] over file sectors starting at [blk+2].
*  (Extension past the current end is bounded to the existing chain ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â
*  the M4 differential gates own the exact authentic envelope.)
SFOUT   MOV  R11,@RETH
        MOV  R1,@R1SV
        BL   @SFBLK
        MOV  @DMA,R2
        BL   @VNAME
        BL   @FNDFDR
        JNE  SFOCRT
        B    @SFOHIT
SUB6B   LI   R1,>0600              * near exits for the SFOUT body
        JMP  SUBSTB
SUB7B   LI   R1,>0700
        JMP  SUBSTB
SUBOKB  CLR  R1
SUBSTB  B    @SUBST              * shared subprogram exit
*  Missing: create it from the block info (the FDR is built once,
*  after ALLOC - ALLOC overwrites buffer A with the VIB).
SFOCRT  CLR  R2                    * allocate + write the FDR
        BL   @ALLOC
        JNE  SUB6B
*  ALLOC clobbered A with the VIB; the new FDR must be rebuilt ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â but a
*  created-empty file's FDR is name+block fields only: redo them.
        MOV  R3,@T1
        BL   @ZBUFA
        BL   @PUTNM
        MOV  @T2,R7
        LI   R0,>3E0C
        BL   @VSETW
        MOVB @4(R7),R1
        MOVB R1,@VDPWD
        MOVB @5(R7),R1
        MOVB R1,@VDPWD
        LI   R0,>3E10
        BL   @VSETW
        MOVB @6(R7),R1
        MOVB R1,@VDPWD
        MOVB @7(R7),R1
        MOVB R1,@VDPWD
        MOVB @9(R7),R1
        MOVB R1,@VDPWD
        MOVB @8(R7),R1
        MOVB R1,@VDPWD
        BL   @WRA
        JNE  SUB6C
        BL   @FDIINS
        BL   @LDA
        JNE  SUB6C
SFOHIT  MOVB @T3,R0                * N, re-read from its input cell (the
        SRL  R0,8                  * lookup left >834D untouched)
        MOV  R0,@LV0
        MOV  R0,R0
        JEQ  SUBOKC                 * N = 0: creation/lookup only
*  Overwrite N existing file sectors from VDP [blk+0].
        MOV  @T2,R7
        MOV  *R7,R4                * VDP source
        MOV  @2(R7),R5             * first file sector
        MOV  R4,@LV1
        MOV  R5,@LV2
SFOLP   MOV  @LV0,R0
        MOV  R0,R0
        JEQ  SUBOKC
        MOV  @LV2,R2
        BL   @CHAIN
        JNE  SUB7C                * past the end: out of the envelope
        MOV  @LV1,R0
        MOV  R0,@DMA
        BL   @WRSEC
        JNE  SUB6C
        BL   @LDA
        JNE  SUB6C
        MOV  @LV1,R0
        AI   R0,256
        MOV  R0,@LV1
        INC  @LV2
        DEC  @LV0
        JMP  SFOLP
SUB6C   LI   R1,>0600              * near exits for the SFOUT tail
        JMP  SUBSTC
SUB7C   LI   R1,>0700
        JMP  SUBSTC
SUBOKC  CLR  R1
SUBSTC  B    @SUBST              * shared subprogram exit

*  FORMAT (>11): the stock single-density subset (plan ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§10.3 pending):
*  tracks @>834D (35/40), sides @>8351, density forced SD. Re-initializes
*  the mounted image in place via Write Sector (the Write-Track
*  substitution ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã†â€™Ãƒâ€šÃ‚Â¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡Ãƒâ€šÃ‚Â¬ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â plan ÃƒÆ’Ã†â€™Ãƒâ€ Ã¢â‚¬â„¢ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬Ãƒâ€¦Ã‚Â¡ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã…Â¡ÃƒÆ’Ã¢â‚¬Å¡Ãƒâ€šÃ‚Â§0 exception 2). Returns total sectors in >834A,
*  errors in the >8350 byte.
SFMT    MOV  R11,@RETH
        MOV  R1,@R1SV
        MOVB @T3,R2                * tracks
        SRL  R2,8
        JNE  SFMT1
        LI   R2,40
SFMT1   MOV  R2,@LV0
        MOVB @>8351,R3             * sides
        SRL  R3,8
        JNE  SFMT2
        LI   R3,1
SFMT2   MOV  R3,@LV1
        MOV  @LV0,R2               * total = tracks * 9 * sides
        MPY  @SFMT9,R2             * R2:R3 = tracks * 9
        MOV  @LV1,R2
        MPY  R3,R2                 * R2:R3 = * sides
        MOV  R3,@LV2               * total sectors
*  Zero every sector.
        BL   @ZBUFA
        CLR  R5
SFMTZ   MOV  R5,@LV3
        MOV  R5,@ABS
        LI   R0,>3E00
        MOV  R0,@DMA
        BL   @WRSEC
        JNE  SFMTE
        MOV  @LV3,R5
        INC  R5
        C    R5,@LV2
        JL   SFMTZ
*  Build the VIB in A.
        BL   @ZBUFA
        LI   R0,>3E00              * 10-space volume name
        BL   @VSETW
        LI   R1,>2000
        LI   R2,10
SFMTN   MOVB R1,@VDPWD
        DEC  R2
        JNE  SFMTN
        MOV  @LV2,R1               * total (BE at >0A)
        MOVB R1,@VDPWD
        SWPB R1
        MOVB R1,@VDPWD
        LI   R1,>0900              * sectors/track
        MOVB R1,@VDPWD
        LI   R1,>4400              * 'D'
        MOVB R1,@VDPWD
        LI   R1,>5300              * 'S'
        MOVB R1,@VDPWD
        LI   R1,>4B00              * 'K'
        MOVB R1,@VDPWD
        LI   R1,>2000              * unprotected
        MOVB R1,@VDPWD
        MOVB @LV0+1,R1             * tracks
        MOVB R1,@VDPWD
        MOVB @LV1+1,R1             * sides
        MOVB R1,@VDPWD
        LI   R1,>0100              * density = SD
        MOVB R1,@VDPWD
*  Bitmap: sectors 0+1 allocated; tail past the total = >FF.
        LI   R0,>3E38
        BL   @VSETW
        LI   R1,>0300
        MOVB R1,@VDPWD
        MOV  @LV2,R2               * zero bytes: total/8 - 1
        SRL  R2,3
        DEC  R2
        CLR  R1
SFMTB   MOVB R1,@VDPWD
        DEC  R2
        JNE  SFMTB
        MOV  @LV2,R2               * >FF tail: 200 - total/8 bytes
        SRL  R2,3
        LI   R3,200
        S    R2,R3
        LI   R1,>FF00
SFMTF   MOVB R1,@VDPWD
        DEC  R3
        JNE  SFMTF
        BL   @WR0
        JNE  SFMTE
        MOV  @LV2,@>834A           * echo the total
        CLR  R1
        JMP  SFMTS
SFMTE   LI   R1,>0600
SFMTS   MOVB R1,@T0
        MOVB R1,@>8351             * secondary error byte (RECON A.2)
        MOV  @R1SV,R1
        MOV  @RETH,R11
        INCT R11
        B    *R11
SFMT9   DATA 9
