> **ARCHIVED (2026-07-06).** Historical record — the build it tracked is complete; its durable layout ledger and house rules were extracted to ../rom/README.md. Note: some in-progress headers below were overtaken before archiving (M4→M8 all landed).

# Console ROM rewrite — execution progress & resume notes

Living status of [`ROM-REWRITE-PLAN.md`](./ROM-REWRITE-PLAN.md) execution, so a
fresh session resumes without re-deriving state. Sibling of the GROM track's
`../QUALITY-ASSESSMENT-PROGRESS.md`. **Read order:** `../README.md` →
`../STATUS.md` → this plan → [`RECON.md`](../rom/RECON.md) (the interface spec) →
this file → the chunk you're picking up.

**House rules** (plan §13 + P9): probes/facts from `RECON.md`; a differential
microtest per element (the authentic ROM is the oracle — drive any element
directly, no cart needed to call it); **completeness is the bar, not usage**
(§0/P9 — implement every authentic element regardless of the census); `AORG`
every routine at its authentic address (P8), the `check_layout` gate enforces
it; rebuild + commit `console-rom.bin` once the artifact is committed (deferred
until M1 is substantially complete); `cargo test -p libre99-asm` (+ `-p libre99-gpl`
for matrix gates) + `cargo clippy` green; commit per increment.

---

## Done

- **R-1 — toolchain** ✅ (2026-07-03). `libre99asm`: absolute/`AORG` mode + overlap
  guard, `Assembly::check_layout` layout gate, `expand_includes`, listing +
  symbols, `build_console_rom()`, `libre99asm rom`/`dis`, TMS9900 disassembler
  (round-trips). Gate `tests/rom.rs`.
- **R-2 — D1 static dossier** ✅ (2026-07-04). `RECON.md` (interface contract,
  verified against the binary with our own `dis`) + `SURFACE-MAP.md` (8 KiB
  classified + frozen-address table). **`RECON.md` §15 is the complete element
  enumeration — the P9 source of truth for every gate.**
- **R-3 — D2 entry census** ✅ (2026-07-04). `ENTRY-CENSUS.md` + the
  `entry_census` P8 tripwire: real ML carts enter the ROM only at documented
  public addresses (`>000E` KSCAN). Heavier censuses deferred with rationale.
- **R-4 / M1 — kernel + GPL core — IN PROGRESS.**
  - **Increment 1** ✅ (2026-07-04). The interpreter **skeleton + first opcode**,
    all address-exact (`AORG`): reset kernel `>0024` (RECON §1), the main loop
    `>0070` (fetch + first-nibble dispatch `>0C36`), the specials sub-dispatch
    `>0270` + special table `>0C3E`, `BACK` `>029E`, the VDP-register helper
    `>089A`, and a `HALT` stop for not-yet-implemented entries. Gate
    (`tests/rom.rs::our_rom_interprets_gpl_and_back_sets_the_backdrop`): our ROM
    boots a trivial GROM (`BACK >07` at `>0020`), interprets it, and sets VDP R7
    — for two colours (proving the operand is read). The reset→fetch→dispatch→
    opcode architecture is proven end to end.
  - **Increment 2** ✅ (2026-07-04). **The core GPL ISA, oracle-pinned and
    differentially verified.** The oracle probe (`examples/gpl_oracle.rs`)
    extracted the exact `>837C` status model + stack/RAND/edge semantics from
    the authentic ROM (now RECON **§16**); `console.asm` implements the GAS
    operand engine (`OPGET` at its authentic `>077A`), the format-1 families
    (ADD SUB MUL DIV AND OR XOR ST EX CH CHE CGT CGE CEQ CLOG SRA SLL SRL SRC,
    byte/word, imm/mem), format-5 (ABS NEG INV CLR FETCH CASE PUSH CZ INC DEC
    INCT DECT), BR/BS/B/CALL/RTN/RTNC (authentic sub-stack mechanics incl. the
    word-op pointer quirk), RAND (the authentic LCG), H/GT/CARRY/OVF, EXIT,
    the public `>0016/>001C/>006A` entries, the vestigial `>0036–004D`
    vectors/data, and loud breadcrumb stubs (`>837D`) for everything else.
    Gate: **`tests/gpl_core.rs` — 79 differential microtests** (identical GPL
    under authentic + ours; scratchpad `>8300–83DF` + VDP regs + VRAM compared;
    masks documented in RECON §16). Toolchain gained out-of-order `AORG` with
    per-byte overlap guarding en route. Full libre99-asm suite (114) + clippy
    green; libre99-gpl untouched and green.
    *Test-authoring gotcha (RECON §16): `BR $` is not a self-loop when the
    condition bit is set — terminate GPL test programs with the double-BR
    `halt()` idiom.*
  - **Increment 3a — MOVE** ✅ (2026-07-04). The `MOVE` (`>20–3F`) block-move
    family: the `>0CCE` sub-dispatch table (authentic home, our handler
    addresses) selects one of three source loaders (CPU/VDP/GROM) and four
    dest storers (CPU/VDP/GRAM/VDP-register); the handler decodes
    `001 G R V C N` and reads the stream in the pinned order **count, dest,
    source**. Live combos: source GROM-immediate / CPU / VDP; dest CPU / VDP /
    VDP-register; count immediate **or** from-memory (pinned a **word** read —
    RECON §17). Ascending byte-at-a-time copy makes the CPU↔CPU and VDP↔VDP
    overlap-fill idioms work; GROM sources re-address per byte (P4) with the
    interpreter position saved/restored around the copy. GRAM dest (`G=0`) and
    computed-GROM source (`C=1`) stay loud stubs (M4). Gate: **7 `move_*`
    differential microtests** in `tests/gpl_core.rs` (86 total, all green);
    semantics in RECON **§17**. Placed at `>1000` (P8: opcode-handler bodies
    are not address-pinned).
  - **Increment 3b — ALL** ✅ (2026-07-04). `ALL` (`>07`) at its authentic home
    `>05A2` fills the 768-cell name table (VRAM `>0000..>02FF`, base hardcoded —
    oracle-pinned that VDP reg2 does not move it, RECON §18); status unchanged.
    Wired `SPCTAB[7]`. Gate: `all_fills_screen` / `all_clears_screen` (88 total).
  - **Increment 3c — IO (CRU output)** ✅ (2026-07-04). `IO` (`>F4–F7`): the
    driver now routes `>=>EC` opcodes past the format-1 source-parse (the byte
    after the dest is a **function code by value**, not a GAS source); the
    function indexes the `>0CEC` sub-table. **Function 3 (CRU output)** is live —
    the four-field list `{addr word, count, data-addr}` drives `count` CRU bits
    LSB-first from `>8300+data-addr` (`SBO`/`SBZ`), so the boot's `IO @>8302,#3`
    arms 9901 VDP-interrupt bit 2 (RECON §19). Sound 0/1 → M2; CRU-in 2 → deferred
    stub; cassette 4/5/6 → hardware-gated stub. Gate:
    `io_cru_output_arms_vdp_interrupt` via `tms9901.int_mask()` (89 total).
  - **Increment 3d — minimal ISR + the M1 title gate** ✅ (2026-07-04). Two
    unblocks discovered by booting our GROM's real title under our ROM
    (`libre99-gpl/examples/rom_title_probe.rs`):
    1. **A `SPEC` dispatch bug** (latent since increment 1): the specials index
       was `SLA R9,3 / SRL R9,10`, contaminated by **R9's low byte** — and the
       MOVE handler parks a storer address in R9, so after any MOVE the low byte
       has bit 7 set, making the *next* special-op's index odd (misaligned) → a
       garbage dispatch. The title path (MOVEs, then `XML >19`) derailed into
       blank ROM. Fixed to isolate `(opcode & >1F)*2` cleanly
       (`ANDI R4,>1F00 / SRL R4,7`), preserving R9. No microtest caught it (none
       followed a MOVE with a special-op); the 89 microtests still pass.
    2. **A minimal VBLANK ISR** at the pinned home `>0900` (frozen). Acknowledge-
       only: run on GPLWS, read the VDP status (clears the interrupt), `CLR R8`,
       `RTWP`. Without it the still-stubbed vector wedged the CPU once the boot
       armed the 9901 interrupt, so the title's `DISPON` never ran. The full duty
       list (sprite motion, sound/beep, QUIT, screen-timeout blank, SPEED timer,
       user hook — RECON §6) stays M2.
    With both, our ROM paints the whole visible title (bars, banner, chip logo,
    prompt, copyright, fonts, colours) and enables the display, then halts
    cleanly at the unimplemented `XML >19` (PUSCAN) with a loud breadcrumb.
    **M1 exit gate:** `libre99-gpl/tests/rom_title.rs` — our GROM's title paints
    **pixel-identically** (all 8 VDP registers + all 16 KiB of VRAM) under our
    ROM vs the authentic ROM. Full libre99-asm + libre99-gpl suites + clippy green.
  - **Increment 3e — XML + the no-card device scan** ✅ (2026-07-04). `XML`
    (`>0F`) table-of-tables dispatch (handler `>1200`) + the master table
    (`>0CFA`, byte-identical to authentic) + FLTAB (`>0D1A`, FP stubs) + XTAB
    (`>12A0`). **SROM** (`>19`, pinned home `>0AC0`) does the real no-card CRU
    scan (returns not-found: `>83D0=0`, cond clear via the soft entry);
    **SGROM** (`>1A`, `>0B24`) is minimal not-found for M1 (the authentic 16-base
    GROM power-up walk is M3). The boot now falls through the power-up scan
    (PUSCAN) to the title's key-wait `SCAN` (RECON §21; no-card contract traced
    in `examples/puscan_trace.rs`). Gate: `xml_srom_no_card` (90 microtests) +
    rom_title still green.

- **M2 — VBLANK ISR core** ✅ (2026-07-04). The `>0900` ISR upgraded from
  acknowledge-only to the full duty structure (RECON §6/§20): cassette fork
  (hardware-gated), VDP/card source test, the four `>83C2`-gated duties, VDP
  status read, **screen-timeout blank** (`>83D6`/`>83D4`), **SPEED timer**
  (`>8379`), and the **`>83C4` user hook**; **QUIT** (`>004C` `CZC` → `BLWP
  @>0000`) is live. **Sprite auto-motion** (the `>0780` velocity math) and
  **sound-list processing** (the boot beep) are gated-off scoped follow-ups.
  Gate: `isr_advances_timer_and_timeout` — with the interrupt armed and nothing
  else, our ISR advances `>8379`/`>83D6`/`>837B` byte-identically to authentic
  (91 microtests). **Remaining M2:** the sprite-velocity math, the sound list,
  and full KSCAN (a clean-room spec is being drafted in `rom/KSCAN-SPEC.md`).

**Verified design facts** (grounding for the next increments; ✅ on the binary):
- **First-nibble dispatch** (`>0070`): `MOVB *R13,R9` (fetch), `JLT` if `≥>80`,
  else `MOVB R9,R4 / SRL R4,12` (top nibble 0–7) `/ MOV @>0C36(R4),R5 / B *R5`.
  The 9900 **ignores the odd address bit** on word access, so nibbles pair:
  0-1→specials `>0270`, 2-3→MOVE `>061E`, 4-5→BR `>011A`, 6-7→BS `>010E`.
- **Specials sub-dispatch** (`>0270`): `SLA R9,3 / SRL R9,10` computes
  `(opcode & >1F)*2`, indexing the `>0C3E` special table (BACK=>04 → index 8 →
  `>0C46` → `>029E`).
- **VDP register write**: value byte then `>80|reg` to the write-address port
  `>8C02` (via R15); `BACK` sets R7 selector `>8700`, fetches the operand to
  `>83EF`, calls `>089A`.

---

## Next — M4 IN PROGRESS (FMT + MOVE/indexed-GAS ✅ 2026-07-05) → the long tail

**Where things stand (2026-07-05):** M1, M2, **and M3 are complete**. Our ROM
boots our GROM end-to-end (title + menu pixel-identical), the ISR runs every duty
(RECON §6), KSCAN covers all modes, and the **device linkage** now works: SROM's
found+call peripheral power-up (the disk card lowers `>8370` identically), the
full SGROM 16-base GROM power-up walk (PUSCAN parity with authentic), and the
`XML >F0` ML-cart launch (verified under our ROM by the sweep). ~95 gpl_core
microtests + gpl_fuzz + the boot-flow gates (`rom_title`/`rom_kscan`) + 4 matrix
gates + the parameterized `device_io`/`sweep` (both firmwares). Run the artifact:
`cargo run -p libre99-app -- --system-rom
original-content/system-roms/rom/console-rom.bin --no-cartridge`.

**M2 close-out ledger:**

1. ✅ **ISR sound-list processing** (661fde2) — the boot beep. Authentic `>09EC`
   engine: SPEED countdown, GROM/VDP source (FLAGS `>01`), all block forms (a
   normal block, `N=0` pointer-reload, `N=>FF` source-switch, `D=0` end). Two
   NASTY constants pinned (SNDTOG/SNDESC). Gate:
   `isr_sound_list_drains_to_the_chip_like_authentic` (cells + PSG state).
2. ✅ **ISR sprite auto-motion** (948c8f8) — authentic `>095C`: fixed-point
   velocity integration (SMT `>0780` → SAT `>0300`) with the vertical edge-wrap.
   Gate: `isr_sprite_motion_integrates_like_authentic` (SAT + SMT after N ticks).
   Added `Vdp::set_vram`/`Machine::vdp_mut` for VRAM-planting tests.
3. ✅ **IO functions 0/1** (4988f26) — arm a GROM/VDP sound list (authentic
   `>05D6`, source bit from the function code). Gates: `io_sound_arm_function0…`
   and `io_sound_function1_drains_from_vdp` (the VDP-source ISR path).
4. ✅ **KSCAN split modes 1/2 + joysticks** (1599d78) — mode dispatch, split
   mask (`SZC R0,R4`), `>17C0` split base, unit-indexed debounce, joystick
   deflections (`>16E0` → `>8376/77`). Gates: 4 new `rom_kscan.rs` tests
   (joystick 1/2 + split keyboard). **One documented deferral:** the full-scan
   split-cell coherence write to `>83C9/CA` (authentic `>03CA`) — no flow uses
   it; the bounded fuzz (item 6) is the instrument that would surface it.
5. ✅ **Firmware matrix** (4618392) — the `[TI_ROM, OUR_ROM]` conformance harness
   (`firmware_matrix.rs`). Title/menu/idle-ISR parity: VDP regs + VRAM match
   byte-for-byte; the scratchpad diff carries the documented whitelist (the
   M1-minimal-SGROM boot-timing cascade — `>8379`/`>83D6-7`/`>83CC-CE`/`>83D2` —
   which **realigns at M3**; the whitelist doubles as the M3 to-do list).
6. ✅ **Early bounded differential fuzz** (acf67ba) — `libre99-asm/tests/gpl_fuzz.rs`:
   deterministic SplitMix64 over the well-defined format-1/5 subset, full
   data-cell + VDP + VRAM diff. Fast tier (300) is the pre-commit gate; deep tier
   (20k, `#[ignore]`) soaks green (142s). **It immediately caught a real M1 bug:**
   SHCNT masked the shift count to `>001F` but the 9900 `SRL R2,0` form masks to
   the low nibble `>000F` — a count ≥16 shifted a word to 0 where authentic
   shifts by 1 (fixed; gpl_core's <16-count shift tests were blind to it).
   **Deferred to M4 (logged, not dropped):** MUL/DIV byte-form semantics
   (MUL byte `>09*>96` → authentic `>FC` vs ours `>05`), EX's undefined immediate
   forms, and the SRA/SLL/SRL/SRC handlers not mirroring result flags into `>837C`
   — per-opcode-*form* details, not sequence coupling.

**M2 close-out is COMPLETE (bare-console scope).** ~103 gpl_core microtests +
gpl_fuzz + 16 boot-flow gates + 3 matrix gates, all green.

**M3 — XML + dispatch + device I/O — COMPLETE (2026-07-05).** Semantics in
RECON **§24**. Three slices:

1. ✅ **SROM found+call + peripheral power-up** (d9d6912) — SROM (`>0AC0`)
   rewritten from the no-card stub to the authentic found+call+resume path (CRU
   scan → `>AA` header → chain walk at `>836D` → SNAME → `BL *R9` with the
   DSR-call invariants), shared SNAME helper in free space. Proven under our ROM:
   the boot peripheral power-up finds the disk card (routine `>4070`, CRU `>1100`)
   and lowers `>8370` to `>37D7` identically to authentic. Gate:
   `device_io::disk_power_up_reserves_vram` (both ROMs, FMT-free).
2. ✅ **SGROM full 16-base walk** (4358473) — SGROM (`>0B24`) rewritten to the
   authentic GROM-header power-up walk (trampoline → free-space body). The two
   harvested constants pinned as named DATA (`GR13M=>1FFF`, `PGMKEY=>06`, P8); the
   R1/R3/R9 low-byte GPLWS aliases as authentic. Gate:
   `firmware_matrix::matrix_puscan_walk_matches_authentic` (SROM ×1, SGROM ×16,
   `>83D0` sweep byte-identical). Realigns `>83D2`; the matrix whitelist tightens
   to the ISR-counter cycle-timing offset only.
3. ✅ **XML >F0 cart-launch matrix** (743aeb1) — `sweep.rs` parameterized over
   `[TI_ROM, OUR_ROM]`: 9/11 class samples list+launch under our ROM; the ML
   samples (centipe/MoonPatrol) are the `XML >F0` gate. Parsec/TI-Invaders list
   under both, launch oracle-only (their GPL startup needs FMT — M4).

**Cross-milestone finding (documented, not a gap):** the *ToD* end-to-end load
and two GPL sweep carts paint with the **FMT** opcode (`>08`, M4), so those
*flows* run on the oracle until M4; the device linkage itself is proven FMT-free.
The **ISR card-chain** scan is hardware-gated (the emulator raises only the VDP
interrupt line — `cru.rs`; an M2-scope duty deferred like the cassette transport,
RECON §24). The GPL-fetch-stream lockstep amendment is satisfied by the observable
PUSCAN-walk parity + pixel-identity (a byte-identical stream through the phantom
`>E000` power-up execution — undefined GPL off an absent GROM — is not a
meaningful bare-console gate).

**M4 — interpreter completeness (non-BASIC) — IN PROGRESS.** Element list:
RECON §15; pinned semantics §§16–24 + §7 (FMT).

1. ✅ **FMT — the screen-format sub-interpreter** (2026-07-05). Entry `>04DE`
   (pinned trampoline, P8) → free-placed body (Zone I, `>1440`); the `>0CDC`
   dispatch table wired at its authentic home with our handler addresses. All
   eight groups + thirteen sub-op behaviours — HTEXT/VTEXT/HCHAR/VCHAR,
   HMOVE/VMOVE, RPTB (the sub-stack loop), and the E/F control group
   (string-from-operand, FEND, BIAS immediate/GAS, ROW, COL) — with the cursor
   (`>837E/837F`, base VRAM 0) and the 768-cell wrap. Grammar pinned by
   disassembling authentic `>04DE-05B7` as a spec (P5), reproduced clean; RECON
   §7. Gate: `libre99-asm/tests/gpl_fmt.rs` (15 differential cases). **Payoff:
   Parsec + TI-Invaders now list *and launch* under our ROM** (`sweep.rs`,
   `check` both ROMs — the two `check_fmt_launch` exceptions retired); ToD's
   title/selection paints under our ROM (its LOAD path next needs MOVE C=1).

2. ✅ **MOVE C=1 + indexed-GAS + GRAM dest** (2026-07-05). Three intertwined
   operand-engine elements, all pinned by disassembling authentic `>0758`/
   `>077E`/`>07D2` as a spec: **indexed GAS** (`OPGET`'s `>4000` X-bit — adds the
   `>8300`-indexed **word** to the base, replacing the loud stop; `OPGIDX`);
   **MOVE C=1** (computed-GROM source = a 16-bit inline base + that same indexed
   offset, `MVSCP`); **MOVE GRAM dest** (`MVDGR`/`MDGRM` — a 16-bit inline
   GRAM address, written to `>9C00` with per-byte re-addressing; the MOVE
   position save/restore extended to cover a GRAM dest). Gates (`gpl_core.rs`,
   99 total): `indexed_gas_destination`, `move_computed_grom_to_cpu`,
   `move_cpu_to_gram`, `move_grom_to_gram`. The growing MOVE handler claimed
   `>1000-11FF`, so `IOH` relocated to the `>0EC0` FP-zone gap (layout ledger).
   These clear ToD's FMT + MOVE-C=1/indexed/GRAM-dest LOAD-path opcodes; ToD then
   still diverges at a GRAM `MOVE` (>20) whose per-op microtests match — the
   residual is most likely the emulator's no-op GRAM writes (mask ROMs, no GRAM),
   which ToD reads back, so ToD stays oracle-gated (`device_io.rs` note).

3. ✅ **COINC + SWGR/RTGR + IO CRU-in + the uniform source discipline + the
   ext-GPL vestige** (2026-07-05). Semantics pinned in RECON **§25** by
   disassembling authentic `>0086-00CA` / `>06D2` / `>004E` / `>082C` /
   `>05C8-0606` / `>0C0C-0C2E` as a spec. The big driver finding: there is
   **no `>=>EC` special routing** — every `≥A0` opcode takes the uniform
   imm(`>0200`)/mem source parse with **byte values right-justified
   sign-extended** (`>07AA`); our driver realigned (the old skip misread the
   `>F4/F5/F7/F9` mem/word forms), our high-justified internal convention kept,
   the three value-consuming handlers normalize at entry. **COINC** (`>EC-EF`):
   the full bitmap coincidence test (deltas, SRA scale, header offsets/limits,
   MSB-first bit probe, `>837C` overwritten wholesale, RTNC exit). **SWGR**
   (`>F8-FB`): switch GROM base (push ×2 + top-slot R13 overwrite, settle
   strobe, `>006A` exit); **RTGR** (`>13`): the inverse (pop-restore R13, the
   faithful GRAM-port poke, RTN exit). **IO functions 2+3** rebuilt as the
   authentic synthesized-`X` LDCR/STCR engine — settles the "CRU count > 1" ❓:
   the 4-bit count field (0→16), byte ≤8 / word >8 data access, LSB-first,
   STCR zero-fill. **Ext-GPL trampolines** `>0C0C/>0C14/>0C1C` + `>0C28` live
   at their pinned homes (byte-identical, address-forced); the XOP-0 vector
   `>0040` now points at real code; **XTAB `>1C-1F`** carries the harvested
   dispatch constants (`C120 834A 1342 04C0`). RAND parity confirmed closed
   (the M1 LCG + `rand_*` gates). Gates: **15 new** in `gpl_core.rs` (113
   total): 2 SWGR/RTGR (imm + mem forms), 7 COINC (hit/miss/outside/negative/
   scaled/offset/byte-form), 3 IO (in+readback, word counts, fn-from-memory),
   the bounded ext-GPL departure pair, and the byte-conformance check on the
   three pinned vestigial surfaces. New Zone J (`>1620+`, the BASIC-half span
   M6's deferral frees) holds the three bodies; IOH shrank in place. Kept
   divergences (documented, RECON §25): IO functions ≥7 / function bytes ≥`>80`
   hit our loud stub where the authentic garbage-dispatches.

4. ✅ **The per-opcode-form semantics the M2 fuzz owed** (2026-07-05, RECON
   §25 addendum). The fuzz pool widened to MUL/DIV/EX with `>837C & F8`
   compared live — it then drove an iterative disassemble→fix loop that
   landed far more than the three logged items: **SUB is NEG-then-ADD**
   (C/OV differ at source 0/>8000 — seed 306); **MUL byte** keeps the
   source's sign extension (>09*>96 → FC 46 at D,D+1); **DIV** presets
   >837C wholesale, ORs >08 via the harvested `@>0013` on the 9900's own
   overflow, and stores the unchanged halves even then; **EX's imm forms**
   store the immediate (the old dest goes to the inert speech-region
   accident); **the jump-family compares PRESERVE** the rest of >837C
   (the M1 wholesale-replace reading was wrong) while **CEQ/CZ are
   raw-STST wholesale** (CEQ's C bit = the opcode's word bit); and two
   whole *new* authentic behaviors surfaced from the >0780/>0232 reads:
   **>837D is the character buffer** (OPGET fetches the screen byte at the
   cursor into it on any CPU-space read resolving there — indirection
   included; stores ending on it paint back at the cursor, multicolour
   nibble RMW and all) and **`*@>837C` is the data-stack pop quirk**.
   Gates: 27 new microtests (140 gpl_core total) + the widened fuzz
   (fast tier in pre-commit; the 20k deep soak green in 41s). Layout:
   IOH moved >0EC0 → >0F00 (Zone H grew); CEQH + the character-buffer
   helpers live in Zone J.

5. ✅ **The cassette modem layer** (2026-07-05, RECON **§26**). The whole
   FSK layer written to the authentic structure: IO 4 (write: leader/sync/
   double-record/checksum, the X-executed mag-out toggle, inverted bytes),
   IO 5/6 (read/verify: leader hunt, timer-calibrated cell measurement,
   sync ride, per-record double-pass retry, VDP compare for verify), the
   shared setup (FLAGS >20, VDP-int off, the 9901 half-cell timer), the
   teardown, and the **cassette timer ISR** with the authentic JMP-$
   single-step idiom + the GPL-R6 (`>83EC`) timeout warp — the `>0900`
   ISR's FLAGS->20 fork now branches to it. Free-placed (Zone K `>1820+`;
   no external caller enters the authentic homes, so FMT keeps its squat).
   The emulator models no interval timer/tape line, so the engines park on
   their first half-cell wait identically under both ROMs — per §11's
   disposition: present and behavior-correct, inert only for want of the
   device. Gates: `cassette_write_arms_and_parks`, `cassette_read_and_
   verify_flags`, `cassette_timer_isr_fork_warps` (the fork's SBO-3
   signature both ROMs + the ladder ride on the authentic). **Bonus gate
   discovery:** the authentic `>0698` VDP-register storer mirrors a
   reg-1-starting MOVE's first byte into `>83D4` (the ISR's R1 copy, +the
   16K `>80` force) — our `MDREG` now reproduces it (a real M2-era gap the
   selective ISR asserts had missed).

6. ✅ **The ToD residual — the SROM DSR skip-return exit** (2026-07-05,
   RECON §26 addendum). The old "GRAM readback" hypothesis was wrong (the
   `>837D=>20` "breadcrumb" was ToD writing a SPACE to the now-live
   character buffer). PC-histogram + stale-cell probing found the real
   gap: a DSR that HANDLES a request returns to `BL *R9`+2, and the
   authentic SROM's skip-exit turns the card off, **GPOPs the GPL
   DSRLNK's CALL frame** (resuming DSRLNK's caller directly) and clears
   the condition bit. Ours landed the skip on SRNONE — card on, the GPL
   stuck in DSRLNK's error tail, ToD hung at WORKING. With the exit in:
   **Tunnels of Doom's full disk load completes under our ROM** — the
   `device_io.rs` ToD flows (QUEST load + the CS1 graceful error) now run
   under BOTH ROMs; the module's cross-milestone caveat is retired.

7. ✅ **The 256-opcode differential sweep — the M4 exit gate**
   (2026-07-05, `gpl_opcode_sweep.rs`). Every opcode byte >00-FF runs a
   canonical well-formed program (stack ops ride real frames, branch forms
   get slot-planted halts, EXIT is the bare reset loop, the ext blocks park
   in the card march) bounded in steps under both ROMs — all 252
   non-deferred opcodes dispatch identically. The four M6-deferred opcodes
   (PARSE/CONT/EXEC/RTNB) are asserted to hit OUR loud stub — the
   **M6-deferral tripwire**: un-deferring M6 fails the gate and demands the
   policy's written justification. The sweep caught two real bugs en route:
   **MOVE's destination decode order** (the authentic peels G before R —
   `>28-2F` are GRAM-dest forms with R ignored; ours read R first and
   wrote VDP registers) and **FETCH's word form** (`>89` stores the inline
   byte sign-extended as a WORD through the R5-honoring store; ours stored
   a byte regardless).

**M5 — FP + conversions — COMPLETE (2026-07-05).** Two slices. Slice 1: the
great relocation cleared `>0D3A-1345` (the ledger below); the sweep's
`>2C/>2E` garbage-corner finding documented (RECON §26, an M7 probe).
Slice 2: **the whole radix-100 package, bit-exact** (RECON **§27**; three
parallel subagent disassembly dossiers + in-session decode): FADD/FSUB
(digit loop, base-100 carries/borrows, magnitude swap, alignment), FMUL
(the in-place schoolbook engine), FDIV (Knuth D in radix 100, nine digits),
FCOMP + the flags-only pop-compare, all five S-forms via the `>1FA8`-
protocol FPPOP, the shared normalize/round/pack/status/error cluster at its
authentic homes (round-half-up on the guard, ±`7F63…` saturation, overflow
`>01` / ÷0 `>02` / CFI `>03`, silent underflow, the raw-STST tails), CSN/
CSNGR (the two-pass VDP/GROM text parse, the full grammar, the silent-zero
and full-abort paths), CFI (floor(x+0.5), ties toward +∞), FLTAB + XTAB
fully wired incl. the `XML >00` = `>0000` reset accident, and **CFI's first
four words byte-identical** so the XML `>1C-1F` vestigial accident
reproduces exactly (the DATA tail retired). Every internal label landed at
its authentic address by natural flow. Gates: `gpl_fp.rs` — **58 tests**
(planted operands over every table-0/1 entry, 23 CSN grammar cases, 17 CFI
edges, a 160-case operand fuzz + an 80-case text fuzz), all bit-exact.
Layout: IOH split (`>17FC`/`>1AC8`/`>1FE8`), FPPOP `>1420`, the fetchers in
the SGROM-body span (`>0B28`), the named FP constants at `>1AC0`.

**M7 — hardening — COMPLETE (2026-07-05).** Every §8 gate exists and is
green: the firmware matrix (all our-ROM rows), the per-opcode microsuite +
the 256-opcode sweep, the random-GPL fuzz (fast tier pre-commit, the 20k
deep soak green) + the FP operand/text fuzzes, the GPL-fetch-stream
lockstep, the conformance checkpoints, the robustness probes (bad-device,
the QUIT/reset storm with a >8370 sentinel, the pathological self-looping
sound list, the many-entry menu stress — `rom_robust.rs`), the performance
parity report (`rom_perf.rs`: settle 22 vs 32 frames within the documented
frame-parity slack; wall-clock ratio 0.99-1.04 vs the 1.25 budget), and the
D2 censuses as permanent tripwires (`entry_census`). **One documented
residual:** pinning the authentic's `>2C/>2E` garbage-corner MOVE parse
(RECON §26; no real emitter, excluded with its own tripwire in the sweep).

**M8 — packaging — COMPLETE (2026-07-05).** `rom/README.md` (provenance,
method, status, the measured perf numbers, the test-estate map),
`docs/STATUS.md` gained the firmware-track pointer, the committed
`console-rom.bin` current (522 symbols). The §9 nice-to-haves (a firmware
tag in the window title, a `--list` mention) are left as optional app-side
polish; the boot path already logs firmware overrides. **The embed-default
decision is deliberately Joel's** (plan §9/§10): TI images stay the
embedded default with our firmware as committed artifacts + flags until he
rules otherwise.

**M4 — interpreter completeness (non-BASIC) — COMPLETE (2026-07-05).**
Seven slices: FMT; MOVE C=1 + indexed-GAS + GRAM dest; COINC/SWGR/RTGR +
IO CRU-in/out + the uniform source discipline + the ext-GPL vestige; the
STST-exact status model + MUL/DIV/EX forms + the >837D character buffer +
the *@>837C pop; the cassette modem layer + the timer ISR + the >83D4
mirror; the SROM DSR skip-return exit (ToD loads under our ROM); the
256-opcode sweep. RECON §§25-26 hold the pinned semantics. **Next: M5**
(FP + conversions — **displaces the Zone-H/MOVE/IO/XML/FMT + the
device-linkage squatters**, see the ledger), then M7 (hardening), M8
(packaging). **M6 (TI BASIC) is deferred indefinitely by policy** — only a
written justification (a real program tripping the PARSE/CONT/EXEC/RTNB
stubs or the XML >13-18 surface) un-defers it; the sweep's tripwire
enforces the paperwork.

---

## Layout ledger — blocks placed outside their final homes

P8 pins *entries and tables*; handler *bodies* are free-placement — but several
of ours currently sit in regions later milestones must claim at pinned
addresses. **The `AORG` per-byte overlap guard makes every debt self-enforcing**
(the displacing milestone's first build fails loudly), so nothing here can rot
silently; this table exists so each displacement is *planned*, not discovered.

| Block (label) | Now at | Sits in the authentic… | Displaced by | Disposition |
|---|---|---|---|---|
*(Rewritten at M5 slice 1 — the great relocation. All addresses measured
from the symbol map, 2026-07-05.)*

| Block (label) | Now at | Sits in the authentic… | Displaced by | Disposition |
|---|---|---|---|---|
| Zone A (control-flow/specials bodies) | `>0120–026F` | opcode bodies (unpinned) | — | permanent |
| Zone C (format-1 bodies) | `>0300–04B1` | KSCAN interior (unpinned; our entry trampolines out) | — | permanent |
| Zone D (format-5 + INC bodies) | `>0500–05A1` | FMT body region (entry `>04DE` pinned, body free) | — | permanent (FMT trampolines) |
| Zone E (stream helpers) | `>0680–0779` | MOVE/COINC/GAS interior (unpinned) | — | permanent |
| Zone G (value loads) | `>08A4–08FF` | interpreter-service interior (unpinned) | — | permanent |
| `IOH`/`IOCRIN`/`IOCROUT`/`IOSND` | `>0E94–~0EF4` | the FMUL-interior gap (`>0E92-0F53` unpinned) | **M5** if FMUL's body wants it | one more hop if needed |
| `SGROMB` + linkage constants | `>1346–~141A` | the old cassette span (free — Zone K took the engines to `>1820`) | — | permanent-ish |
| Zone I (`FMTBODY`) | `>1440–~1608` | the cassette span + BASIC-support head | **M6** only | permanent under the deferral |
| `SNAME` | `>17D4–~17FA` | BASIC-support interior | **M6** only | permanent under the deferral |
| Zone J (COINC/SWGR/RTGR/CEQ + the >837D helpers) | `>1620–~17D2` | BASIC-support interior | **M6** only | permanent under the deferral |
| Zone K (the cassette engines + timer ISR) | `>1820–~1AC0` | BASIC-half interior | **M6** only | permanent under the deferral |
| `KSCANB` | `>1B00–~1CEC` | BASIC-half interior + the `>1C9C` tables span | **M6** only | permanent under the deferral |
| Zone H (stores/shifts/stubs) | `>1CF0–~1E8A` | the M6 tables/support region | **M6** only | permanent under the deferral |
| `MOVEH` + loaders/storers | `>1E90–~1FB4` | ditto | **M6** only | permanent under the deferral |
| `XMLH` | `>1FB8–~1FE6` | ditto | **M6** only | permanent under the deferral |
| XTAB `>1C-1F` tail (harvested constants) | `>12B8–12BF` | CFI's entry (`>12B8`, pinned at M5) | **M5 slice 2** | CFI's code displaces the constants; the accident then reproduces structurally (RECON §8/§25) |

Free slack (measured): `>141C-143F`, `>1609-161F`, `>17FC-181F`,
`>1AC0-1AFF`, `>1FE8-1FFB` ≈ 178 bytes + whatever M5's bodies leave inside
`>0D3A-1345`. **The M6 deferral is what makes this layout close** — every
"M6 only" row above is a displacement M6's justification note must budget.

Budget note: ≈3 K of the 8 K is used today; the deferred cassette transport's
`>1346–15D3` is the planned slack (plan §7 sizing note). Fit pressure arrives
at M6 — track bytes-used per region when M5/M6 land.

---

## Concerns / notes (come back to these)

- ~~**Per-opcode condition-bit rules**~~ **Resolved** — pinned by the oracle
  round + the differential microsuite (RECON §16).
- ~~**Indexed GAS + `MOVE` C=1** semantics ❓~~ **Resolved (M4 slice 2)** —
  pinned by disassembly + differential gates (RECON §15/§25).
- **The committed `console-rom.bin` artifact** is now committed (M1 complete,
  2026-07-04): `rom/console-rom.bin`, 8 KiB, built by
  `libre99asm rom rom/console-rom.bin`. Per P6, **rebuild + recommit it whenever
  `console.asm` changes** (`cargo run -p libre99-asm --bin libre99asm -- rom
  original-content/system-roms/rom/console-rom.bin`). Tests still build it
  in-memory (`build_console_rom()`), so a stale on-disk copy never fails a gate —
  keep it fresh for the emulator's `--system-rom` flag.
- **Loud stubs**: every unimplemented dispatch entry lands on `STUB`
  (breadcrumb `>837D` := the opcode byte, then a visible spin) — diagnosable,
  never a silent runaway. The boot/menu gates assert the breadcrumb stays
  clear (the standing "boot ran clean" tripwire).
- **⚠ House rule — GPLWS byte aliases.** The `>83E0–83FF` cells ARE the GPL
  workspace registers (`>83E7` = R3-low, `>83EF` = R7-low, `>83F9` = R12-low,
  …). Two real bugs came from forgetting this: the SPEC dispatch read R9's
  stale low byte (increment 3d), and KSCAN's `CLR R3` wiped `>83E7` — the raw
  scan code — before use. Never treat a `>83Ex/>83Fx` cell as independent
  storage; comment every such reference with its register identity. (The
  authentic ROM *exploits* the aliasing deliberately — e.g. KSCAN's state load
  and the `>04A2` bound helper — so reads of authentic code must watch for it
  too.)
- **⚠ House rule — sequence coupling.** Single-element differential microtests
  are blind to state leaked between handlers through shared registers: the
  MOVE→XML SPEC-dispatch bug survived 89 of them and was caught only by the
  end-to-end boot probe. Every increment must run the boot/menu gates, and the
  bounded fuzz (Next §6) covers the sequence space systematically.
- ~~**IO CRU-output count > 1** ❓~~ **Resolved (M4 slice 3)** — the authentic
  engine is a synthesized `X`-executed LDCR/STCR, so counts follow the 9900's
  own rules (4-bit field, 0→16, byte ≤8 / word >8); ours synthesizes the same
  instruction and the word-count + input forms are differentially gated
  (RECON §25).
- **`XML >00` is a reset in the authentic ROM** (FLTAB entry `>00` = `>0000` —
  it dispatches to the reset vector; RECON §8). Ours currently loud-stubs it;
  M5 must reproduce the authentic accident faithfully.
- **`>83C6` (the translation state) is never seeded by boot GPL** — authentic
  behavior (zeroed emulator RAM ⇒ state 0 = the 99/4 state; random on real
  hardware; F5 preserves a game's leftover). Both ROMs read the same cell, so
  differential integrity holds regardless; note for the GROM track's
  structure-handoff audit and the M7 conformance whitelists.
- **Findings exported to other tracks:** our GROM's unshifted keytab holds
  **uppercase** where the authentic holds **lowercase** (RECON §23 — masked in
  state 0 by the fold, visible in the 4A-native state TI BASIC uses; a
  GROM-track ledger item, not a ROM defect). The functional alpha-lock switch
  is emulator work — `docs/ROADMAP.md` §6.

---

## Done (continued) — the M2 entries

- **M1-3f / M2 — KSCAN + the CLEAR test** ✅ (2026-07-04). **KSCAN** (`>02B2`,
  pinned; the `SCAN` opcode + the `>000E` entry) — M2-1 mode-0 full-keyboard
  scan: columns 5→0 via the CRU (`LDCR`@`>0024` select, `STCR`@`>0006` rows),
  first-key-wins, **raw = col*8 + (7-row)**, column-0 modifiers, priority
  CTRL>FCTN>SHIFT>none → GROM translation base `>17xx`+raw → `>8375`, debounce
  vs `>83C8` (condition only on a code change), un-blank on a new key. The body
  trampolines from the pinned entry (Zone C holds the authentic interior — P8
  escape hatch); added the `>0842` `GPOP` helper. **CLEAR/BREAK test** (`>04B2`,
  behind `>0020`): the two-column FCTN-4 CRU probe. **The full boot now reaches
  and idles in the title's key-wait, matching authentic's `>0165-016A` loop, and
  a keypress reads correctly.** Gate: `libre99-gpl/tests/rom_kscan.rs` (5 tests:
  digit/letter/Enter/Space/no-key, differential vs authentic); RECON §22, spec
  in `rom/KSCAN-SPEC.md`. **Deferred (M2-2):** split/joystick modes, the
  alpha-lock case fold (a `cru.rs` gap), control-code fixups; and the ISR's
  sprite-motion + sound-list follow-ups.

- **M2 — KSCAN result normalization (the "alpha-lock blocker" dissolved)** ✅
  (2026-07-04). Pinned the authentic `>0422-0476` logic by disassembly (RECON
  §23): normalization is gated by the `>83C6` translation state — **state 0
  (the 99/4 state, the zeroed-RAM boot default) folds a-z to uppercase without
  ever reading the switch**; only states 1/2 read the alpha-lock line, which
  idles high ("not locked") on our switchless 9901 — identically under both
  ROMs, so differential integrity holds by construction. Implemented the full
  block (the fold, state-0 code rejection `>10-1F`/`>5F`, the Pascal `>80`-bit
  fixups with Enter exempt); constants pinned: `@>03B4=>20`, `@>0587=>5F`,
  `@>02CA=>0F`, `@>0025=>0D` (the last already correct in our ROM — it is our
  own `LI R13` low byte). Gates: `rom_kscan.rs` now **7 tests**, incl. two on
  the AUTHENTIC GROM's lowercase keytabs that exercise the fold and the
  switch-read differentially. A functional switch (host Caps Lock) is queued as
  emulator **ROADMAP §6**. **Finding for the GROM track:** our GROM's unshifted
  keytab holds uppercase where the authentic holds lowercase — masked in state
  0 by the fold; visible only in the 4A-native state (TI BASIC's mode 5).

- **M2 — the menu gate (end-to-end, two screens)** ✅ (2026-07-04).
  `rom_title.rs` gained `our_rom_reaches_the_menu_pixel_identically`: title →
  keypress+release → the master selection menu (chip logo, banner, the
  SCANNING pass over every GROM/cart base, `1 FOR TI PYTHON`) settles
  **pixel-identical** under our ROM vs authentic. Both title tests also assert
  the loud-stub breadcrumb `>837D` stayed clear — the standing "the boot ran
  clean" tripwire.