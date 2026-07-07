> **ARCHIVED (2026-07-06).** Historical record — executed in full (M1–M5/M7/M8 done; M6 deferred by policy) and archived per its own §13 instruction. Live successors: ../rom/README.md (status, principles summary, maintenance notes) and ../rom/RECON.md.

# Console ROM Rewrite — exhaustive plan (Phase 2 of the system-ROM project)

A from-scratch, original-content reimplementation of the TI-99/4A **console ROM**
(`994aROM.Bin`, 8 KiB of TMS9900 machine code at CPU `>0000–1FFF`) — the reset
kernel, the **GPL interpreter**, the VBLANK ISR, the keyboard scanner, the XML
dispatch tables, and the floating-point package — written in TMS9900 assembly in
**our own dialect**, assembled by **our own `libre99-asm`** toolchain (upgrading it
is part of this plan), and proven against the genuine ROM as a differential
oracle. It is the firmware counterpart to the completed GROM rewrite
([`../STATUS.md`](../STATUS.md)) and closes the loop the parent
[`../README.md`](../README.md) opened: after this, **no Texas Instruments bytes
remain anywhere in the boot path**.

Everything for this sub-project — this plan, the assembly source, the committed
binary, its dossier and README — lives in this folder,
`original-content/system-roms/rom/`, mirroring [`../grom/`](../grom/).

*Written 2026-07-02, planning session. Status: **APPROVED by Joel 2026-07-02 —
in execution; chunk R-1 (toolchain) landed 2026-07-03; completeness
mandate added 2026-07-04 (§0 overriding goal + P9); chunks R-2 (D1 dossier —
[`RECON.md`](../rom/RECON.md) + [`SURFACE-MAP.md`](../rom/SURFACE-MAP.md)) and R-3 (D2
entry census — [`ENTRY-CENSUS.md`](./ROM-ENTRY-CENSUS.md)) landed 2026-07-04;
chunk R-4 (M1 kernel + GPL core) COMPLETE 2026-07-04 — the full core GPL ISA
(oracle-pinned, RECON §16), MOVE/ALL/IO CRU-out/XML dispatch + the no-card
device scan, and the M1 title gate met; chunk R-5 (M2 input + ISR) COMPLETE
2026-07-05 (bare-console scope) — the **full VBLANK ISR** (every §6 duty:
sound-list, sprite auto-motion, timer, timeout, QUIT, user hook), **IO
functions 0/1** (sound-list arming), **KSCAN all modes 0–5** incl. split
keyboard + joysticks (RECON §§20–23), the CLEAR test, the end-to-end MENU gate,
the **firmware matrix** (plan §8), and the **early bounded differential fuzz**
(which caught a real M1 shift-count-mask bug); chunk R-6 (M3 XML + dispatch +
device I/O) COMPLETE 2026-07-05 — **SROM found+call** (the disk power-up lowers
`>8370` identically), the **full SGROM 16-base power-up walk** (PUSCAN parity
with authentic), and **`XML >F0`** ML-cart launch verified under our ROM
(RECON §24); the *Tunnels of Doom* / Parsec device flows carry a documented
FMT-opcode (M4) dependency and the ISR card-chain is hardware-gated (no card
interrupts in the emulator). The committed artifact
[`console-rom.bin`](../rom/console-rom.bin) ships and runs via `--system-rom`.**
Live execution status + the resume point (M4) are
[`PROGRESS.md`](./ROM-PROGRESS.md); the 2026-07-04 review pass added the
**execution amendments** below and PROGRESS's **layout ledger** + hazard house
rules. All §10 decisions are recorded and settled. ⚠ **Read §0's overriding
goal and P9 before any implementation milestone** — this is a *functionally
complete* rewrite (every authentic element implemented and differentially
verified regardless of usage evidence), deliberately **not** the GROM track's
implement-on-demand strategy. The GROM ledger-closure track
([`../QUALITY-ASSESSMENT.md`](./QUALITY-ASSESSMENT.md) §7) is complete-with-
one-open-entry and quiescent; §12's file discipline still applies. House
methods carry over: read [`../README.md`](../README.md) →
[`../STATUS.md`](../STATUS.md) → [`../RECON.md`](../RECON.md), and before any
debugging, [`../GROM-DEBUGGING-GUIDE.md`](../GROM-DEBUGGING-GUIDE.md) +
[`../DEBUGGING.md`](../DEBUGGING.md).*

> **Execution amendments (2026-07-04 review pass — the plan as adjusted by the
> M1/M2 build-out; these refine sequencing and method, not §10 decisions):**
> 1. **The GPL-fetch-stream lockstep gate moves M1 → M3.** M1 shipped the
>    sanctioned minimal not-found SGROM (the M1 row's sequencing note), but the
>    authentic power-up scan walks 16 GROM bases, so the boot fetch streams
>    differ *by design* until M3's full walk. M1/M2 equivalence was gated
>    instead on VRAM + VDP-register **pixel-identity at the title AND the
>    menu**, plus a standing no-loud-stub (`>837D`) tripwire — adopt that pair
>    as the per-increment end-to-end gate from here on.
> 2. **A layout ledger governs body placement** (PROGRESS.md). Several handler
>    bodies temporarily occupy regions M4/M5/M6 must claim at pinned addresses
>    (Zone H + MOVE/IO/XML handlers in the FP/conversion region; KSCANB in the
>    BASIC half). The `AORG` per-byte overlap guard makes each debt
>    self-enforcing; **relocating the squatters is part of the displacing
>    milestone's definition of done**, and M5/M6 sizing must budget for it.
> 3. **Two hazard house rules** (PROGRESS.md Concerns, each from a real bug):
>    GPLWS byte-alias discipline (`>83E0–83FF` cells ARE registers — the SPEC
>    and KSCAN bugs), and sequence-coupling coverage (single-element microtests
>    miss inter-handler register leakage — run the boot/menu gates every
>    increment, and **pull a bounded differential fuzz over the implemented
>    opcode subset forward from M4 to the M2 close-out**).
> 4. **Stand up the §8 firmware matrix at M2 close-out** (parameterize the
>    existing libre99-gpl estate over `[TI_ROM, OUR_ROM]`) instead of accreting
>    per-flow `rom_*` twins.
> 5. **Exported findings:** the authentic unshifted keytab is lowercase where
>    our GROM's is uppercase (GROM-track ledger; RECON §23), and the functional
>    alpha-lock switch is emulator work (`docs/ROADMAP.md` §6) — the ROM-side
>    read is already written and differentially gated.

Confidence tags, house-style: ✅ verified on this emulator/its binaries during
planning · 📖 literature (TI Intern, Nouspikel, E/A manual — consult, never
copy) · ❓ to be pinned by the dossier (D1/D2) before code relies on it.

---

## 0. Scope, goals, and definition of done

> **★ OVERRIDING GOAL — functional completeness over usage evidence
> (Joel, 2026-07-04). This clause governs every other statement in this
> document; where anything below reads as usage-gated, this wins.**
>
> This is a **complete** reimplementation of the console ROM, not a
> minimal-viable one. **Every functional element the authentic 8 KiB ROM
> implements is implemented here and verified correct against the authentic
> oracle — whether or not any bundled cartridge, TI BASIC, or observed flow is
> known to exercise it.** Absence of evidence that something is used is **never**
> a reason to omit it, stub it, or leave it untested.
>
> The GROM rewrite used the *opposite* strategy — implement the console's
> de-facto interface **on demand**, as software was seen to need it
> ([`../QUALITY-ASSESSMENT.md`](./QUALITY-ASSESSMENT.md) §1). That strategy
> produced its five field bugs (things nobody had enumerated, discovered only in
> play). It is **explicitly rejected here.** The census/coverage tooling (D2,
> §6) exists to **order** the work and **aim extra adversarial scrutiny** at hot
> paths; it may **never** be read as licensing omission or as a filter on *what*
> to implement or test. Every principle, milestone, and gate below is subordinate
> to this (see P9, §4).
>
> **The only two exceptions** — both recorded decisions, neither a completeness
> shortcut:
> 1. **Hardware the emulator lacks** — the cassette tape *transport* (the ROM's
>    bit engines are written and their *observable* behavior is correct; only the
>    byte transfer is inert until tape hardware exists, §10.2). The code is
>    present and complete; it simply has no device to talk to.
> 2. **Functions not in this ROM at all** — the transcendentals (SIN/COS/LOG/EXP/
>    …) live in **GROM-0 GPL**, not the 8 KiB ROM (§8/M5). Reimplementing them
>    here would be scope *inflation*, not completeness.
>
> Genuinely **vestigial** authentic content that *is* in the ROM — the
> extended-GPL-board trampolines to the never-shipped card at CRU `>1B00`, the
> XTAB `>1C–1F` past-the-table entries — is still reproduced **behavior-faithfully**
> (it branches/decodes exactly as the authentic ROM does, even though nothing
> sane calls it). "Dead" in this plan is only ever a statement about **layout
> freedom** (P8), never permission to leave a function unimplemented.

**In scope:**

1. An **original 8 KiB console ROM image** (`rom/console-rom.bin`) with no TI
   bytes, assembled from TMS9900 source in this folder by our `libre99-asm`.
2. **Full GPL interpreter** — the complete 256-opcode ISA as the authentic ROM
   implements it (including `FMT`, `PARSE`, and the constructs our own GROM
   avoids — cartridges use them even though we don't).
3. The **service surface** machine-language software depends on: reset/interrupt
   vectors, the fixed entry stubs at `>000E–>0023`, KSCAN, the VBLANK ISR, the
   XML table-of-tables + tables 0/1 (floating point, DSR search/execute), and
   the scratchpad conventions of [`../RECON.md`](../RECON.md).
4. **Both GROMs must run on it**: our rewritten GROM *and* the authentic TI
   GROM (title, menu, **TI BASIC**) — the acid test that the interpreter is
   real, and the heart of Joel's mix-and-match requirement.
5. **Toolchain upgrades** to `libre99-asm` needed to build a raw ROM at `>0000`
   (absolute origin, raw binary output, a 9900 disassembler for recon).
6. A **production-quality committed binary**: the full gate matrix green, no
   crash/hang/corruption reachable from console user flows (the adopted
   "production ready by 1981 TI standards" bar), perf at parity, docs complete.

**Out of scope (recorded dispositions):**

- **Cassette tape transport — DEFERRED by decision (Joel, 2026-07-02; §10.2).**
  The emulator has no cassette hardware (`crates/libre99-core/src/cru.rs` leaves
  motor/level "not wired"; ROADMAP §6 tracks building it). The authentic ROM's
  tape read/write code is therefore dead in-emulator. We ship the *interface*
  (device error semantics — a CS1 PAB must fail the authentic way, never hang)
  and defer the transport; when the emulator grows cassette hardware, the
  transport becomes a follow-on work item here, commissioned together with the
  hardware. Same disposition as the GROM track (QUALITY-ASSESSMENT §7.4);
  durable documentation homes listed in §10.2.
- **TI-99/4 (non-A) ROM variants, ROM revisions other than our dump.** The
  oracle is pinned: `roms/994aROM.Bin`, 8192 bytes, sha256
  `599da51e9e1968a806871d681f17b5acbb617accf07191891265aee44ebec2b6` ✅.
- **A "turbo" ROM** (faster-than-authentic block moves etc.) — a tempting
  original improvement, but it breaks apples-to-apples comparison and timing
  compatibility. Stretch idea only, as a third, clearly-labeled build flavor
  (§10 decision 3).

**Definition of done** (all four firmware combinations, from the repo root):

```sh
# 1  TI ROM   × TI GROM    — the authentic baseline (unchanged)
cargo run -p libre99-app -- --no-cartridge
# 2  TI ROM   × our GROM   — today's shipping configuration (unchanged)
cargo run -p libre99-app -- --system-grom original-content/system-roms/grom/console-grom.bin --no-cartridge
# 3  our ROM  × TI GROM    — authentic title/menu/TI BASIC on our interpreter
cargo run -p libre99-app -- --system-rom original-content/system-roms/rom/console-rom.bin --no-cartridge
# 4  our ROM  × our GROM   — the all-original console
cargo run -p libre99-app -- --system-rom original-content/system-roms/rom/console-rom.bin --system-grom original-content/system-roms/grom/console-grom.bin --no-cartridge
```

- Combos **3** and **4** boot to their correct title screens; menus list and
  launch the bundled cartridges (same 137-cart gates as the GROM track); TI
  BASIC is usable in combo 3 (differential smoke scripts pass); Tunnels of Doom
  loads QUEST from disk in combos 3 and 4; QUIT, sound, sprites, joysticks work.
- Add `--cartridge <name>` / `--disk <name>` to any line to mix media in; the
  flags already exist and compose (`crates/libre99-app/src/cli.rs:36-37`) ✅.
  Firmware overrides deliberately skip save-state resume
  (`crates/libre99-app/src/main.rs:121-122`) ✅ — states saved under one firmware
  are never restored under another.
- The **performance comparison** Joel asked for is a committed, re-runnable
  report: frames-to-title / frames-to-menu / wall-clock frame throughput for
  all four combos (§8's performance report, landed with chunk R-9), plus the
  honest note that GPL-visible speed is dominated by the ROM's per-byte GROM
  addressing behavior, which we reproduce (§4 policy P4).
- `cargo test` green across the workspace including the new `rom` gates;
  `cargo clippy` clean; docs current (`rom/README.md`, parent README/STATUS).
- **Functional-completeness checklist (the P9 acceptance bar) — every box a
  test-enforced completeness assertion, not a spot check.** The build is not
  done until each is green:
  - [ ] **All 256 GPL opcodes** + every addressing form: a differential
    microtest each; a test asserts the microsuite's opcode set equals the full
    ISA (no gaps).
  - [ ] **`FMT`**: every sub-op of the `>0CDC` sub-language gated.
  - [ ] **XML**: the master table + **every** table-0 and table-1 entry live
    and gated (incl. the vestigial `>1C–1F` reproduced behavior-faithfully).
  - [ ] **Floating point**: every FP/conversion routine + every `>8354` error
    path, bit-exact vs authentic.
  - [ ] **KSCAN**: all modes 0–5 and every translation state gated.
  - [ ] **ISR**: every duty in authentic order + every `>83C2` gate bit.
  - [ ] **Service/dispatch**: `XML >F0`, the `>000E`/`>0016`/`>001C`/`>0020`
    stubs, SROM/SGROM, the powerup scan, XMLLNK conventions.
  - [ ] **BASIC ROM half**: `PARSE/CONT/EXEC/RTNB`, statement entries, the
    `>1C9C` jump tables, the symbol/value-stack package.
  - [ ] **Cassette bit engines** present + behavior-correct (transport
    hardware-gated, §10.2 — the one place "can't fully run" is allowed, and
    only because the emulator has no tape).
  - [ ] **Vestigial content** (extended-GPL trampolines → CRU `>1B00`; the
    `>1FFC` tail words if any reader) placed behavior-faithfully.
  A final **coverage-completeness test** cross-checks the D1 dossier's element
  enumeration against the set of gates and **fails if any enumerated element
  lacks a differential test** — the mechanical guarantee that no element was
  silently dropped (the anti-regression to the GROM's on-demand tail).

---

## 1. Background — what the console ROM actually is

Component map of the authentic 8 KiB, from this session's own dumps of
`roms/994aROM.Bin` ✅ cross-checked against TI Intern / Nouspikel 📖 (planning
research verified every table value below against the binary; D1 re-derives and
formalizes the per-routine map in `rom/RECON.md`). Note a documentation trap
found during this research: where Nouspikel and the binary disagree (e.g. XML
`>03` = `>0FA4` in the binary vs `>0F4A` on the roms page), **the binary
wins** — pin every literature claim against the dump:

| CPU range | Component | Evidence |
|---|---|---|
| `>0000–000D` | **Vectors**: reset `WP=>83E0 PC=>0024`; level-1 interrupt `WP=>83C0 PC=>0900`; level-2 `WP=>83C0 PC=>0A92`; word `>30AA` at `>000C` (❓ data) | ✅ dump; matches `crates/libre99-core/src/cpu.rs` reset |
| `>000E–0023` | **Fixed public entry stubs**: `>000E: B @>02B2` — the documented **KSCAN entry** (`BL @>000E`, WP must be `>83E0`); `>0016: B @>007A` — enter the GPL interpreter with the opcode already in R9; `>001C: B @>0078` — enter the interpreter, fetch next opcode, interrupts *not* re-enabled; `>0020: B @>04B2` — the documented **CLEAR/BREAK test** (E/A: `BL @>0020`, returns EQ set if FCTN-4 is down). `>0010–>0015` are data words (`02B2 0008 1E00`) — the `>1E00`s at `>0014/>001A` are either constants or `SBZ 0` prologues (❓ D1 pins which) | ✅ dump + 📖 E/A manual, TI Intern |
| `>0024–006F` | **Reset kernel + fixed data/vectors**: `>0024` (also the GPL `EXIT` target — warm reset ≡ cold reset) does `LI R13,>9800 / LI R14,>0100 / LI R15,>8C02`, one dummy GROM data read, writes GROM address `>0020`, clears the GPL status byte (`>006A: SZCB`, itself a documented soft entry), and falls into `>0070`. **That is the whole boot** — no self-test, no RAM/VDP/9901 init; everything else is GROM-side GPL 📖. Also here: the extended-GPL-board return stub `>0036`; **XOP 0/1/2 vectors `>0040–004B`** (`>0040`: WP `>280A`, PC `>0C1C` → trampoline to the never-released extended-GPL card at CRU `>1B00`; `>0044/>0048` point at RAM, "user defined"); the QUIT row-mask word `>004C = >1100`; the `SWGR` handler `>004E` | ✅ dump + 📖 TI Intern |
| `>0070–08FF` | **GPL interpreter**: main loop `>0070` (`LIMI 2 / LIMI 0` — the *only* interrupt window, one per GPL instruction), fetch `>0078`, opcode-in-R9 entry `>007A`; opcode bodies with known addresses (compare/logic `>00CC+`, branches `B >0104 / BS >010E / BR >011A`, unary `>0136+`, `CASE >0162`, `PUSH >016E`, arith `ADD >0188 / SUB >0186` (INC/DEC dispatch to these), logic `>0190+`, `ST >019E`, `EX >01A2`, shifts `>01B0+`, `MPY >01CE`, `DIV >01EA`, special-op second dispatch `>0270`, `RAND >027A` (seed `>83C0`), `BACK >029E`, `SCAN >02AE` shim → **KSCAN `>02B2`**, CLEAR test `>04B2`, **FMT sub-interpreter `>04DE–05A1`**, `ALL >05A2–05C7`, I/O dispatch `>05C8` (sound `>05D6`, CRU in/out `>05E8/>05EA`), **XML dispatch `>0608`**, `MOVE >061E–06D1`, `COINC >06D2`, the GAS operand engine (`>077A` fetch, modes `~>07B0`), interpreter service/`RTN`/`CALL`/sub-stack `>0800–08FF` (`CALL >085A`, GROM-address write helper `>0864`) | ✅ dumps + 📖 TI Intern per-routine map (D1 re-derives) |
| `>0900–0ABF` | **Level-1 ISR** (entry `>0900`, WP `>83C0`, immediately `LIMI 0` + `LWPI >83E0`): cassette-timer-mode check first (FLAGS bit `>20` → `B @>1404`), then `TB 2` — if not the VDP, walk **peripheral-card interrupt chains** (CRU `>1000–>1F00`, header byte `>4000`=`>AA` compared against `@>000D`, chain ptr at card `>400C`); if VDP: sprite motion → sound list → QUIT test (`CZC @>004C`) → **VDP status read into `>837B`** (this clears the interrupt) → screen-timeout `INCT` at `>83D6` (blank via VDP-R1 copy `>83D4`, code at `>0A92` — which is also the level-2 vector target) → `AB R14,@>8379` (timer += SPEED) → user hook `>83C4` (`BL *R12`, return `B *R11`) → `CLR R8` → `RTWP`. First three duties gated by `>83C2` bits `>80/>40/>20/>10` (all/motion/sound/QUIT) | ✅ dump of `>0900–094F` + 📖 TI Intern/Nouspikel ints (order binary-verified by research) |
| `>0AC0–0C35` | **SROM `>0AC0`** = `XML >19` — search peripheral-card ROM headers (DSR/subprogram/power-up chains; name length `>8354/>836D`, name ptr `>8356`, cursor cells `>83D0` CRU / `>83D2` entry); **SGROM `>0B24`** = `XML >1A` — same over GROM headers; extended-GPL trampolines `>0C0C/>0C14/>0C1C` | 📖 TI Intern + our device_io trace ✅ |
| `>0C36–0CF9` | **The dispatch tables**: first-nibble table `>0C36`; special-op (`>00–1F`) table `>0C3E` (…`EXIT→>0024`, `PARSE→>18C8`, `XML→>0608`, `CONT→>1920`, `EXEC→>1968`, `RTNB→>19F0`…); `>80–FF` op table `>0C7E`; MOVE table `>0CCE`; FMT table `>0CDC`; I/O table `>0CEC` (0/1 sound-list GROM/VDP, 2 CRU-in, 3 CRU-out, 4/5/6 cassette write/read/verify) | 📖 TI Intern, values binary-verified |
| `>0CFA–0D19` | **XML table-of-tables** — 16 words: `0D1A 12A0 2000 3FC0 3FE0 4010 4030 6010 6030 7000 8000 A000 B000 C000 D000 8300`. Table F = `>8300` is why `XML >F0` branches through the RAM vector at `>8300` (RECON §2); tables 2–E point at RAM/card/cartridge homes | ✅ dump |
| `>0D1A–0D39` | **XML table 0 (FLTAB)**: `>01 ROUND1 >0F54`, `>02 ROUND >0FB2`, `>03 STST >0FA4`, `>04 OVEXP >0FC2`, `>05 OV >0FCC`, `>06 FADD >0D80`, `>07 FSUB >0D7C`, `>08 FMUL >0E88`, `>09 FDIV >0FF4`, `>0A FCOMP >0D3A`, `>0B–0F` stack variants `SADD/SSUB/SMUL/SDIV/SCOMP` (`>0D84/>0D74/>0E8C/>0FF8/>0D46`); entry `>00` = `>0000` (an `XML >00` resets!) | ✅ dump + 📖 names |
| `>0D3A–~11A1` | **Floating-point package** (radix-100 8-byte reals; FAC `>834A–8351`, ARG `>835C–8363`, error byte `>8354`, VDP value-stack ptr `>836E`, sign/exponent scratch `>8375/>8376`): add/sub/mul/div/compare, rounding, overflow paths. Transcendentals (SIN/COS/LOG/EXP/SQR/PWR) are **GROM-0 GPL, not ROM** | ✅ table-0 targets + 📖 |
| `>11A2–1345` | **String↔number conversions**: `CSN >11AE` (`XML >10`), `CSNGR >11A2` (`XML >11`), `CFI >12B8` (`XML >12`); **XML table 1 (XTAB) at `>12A0` — exactly 12 entries** (`>10–>1B`): `>10 CSN`, `>11 CSNGR`, `>12 CFI`, `>13–>17` symbol-table trampolines (`SYM/SMB/ASSGNV/`search`/VPUSH`), `>18 VPOP >1F2E`, **`>19 SROM >0AC0`, `>1A SGROM >0B24`**, `>1B PGMCH >1868`. `XML >1C–>1F` index **past the table** into CFI's first instruction words — undefined behavior (D1/D2 decide reproduce-vs-document) | ✅ dump + 📖 |
| `>1346–15D3` | **Cassette bit engines**: write `>1346` (GPL `I/O 4`), **cassette timer-ISR `>1404–1422`** (the FLAGS-`>20` interrupt regime; 9901 decrementer loaded with `>0011` ≈ 363.6 µs half-bit), verify `>1426` (`I/O 6`), read `>142E` (`I/O 5`). Reachable **only via GPL I/O** — the user-visible CS1/CS2 DSRs are GPL in GROM 0 (`>1320–16DC`) 📖 | 📖 TI Intern/Nouspikel cassette |
| `>15D6–18C7` | **BASIC-support XML package**: symbol-table search/assignment (`SMB >1670`, `SYM >176A`, `ASSIGNV >1788`), `PGMCH >1868`, trampolines `>163C–164E` for `XML >13–>17` | 📖 TI Intern |
| `>18C8–1FFB` | **The TI BASIC ROM half**: `PARSE >18C8`, `CONT >1920`, `EXEC >1968`, `RTNB >19F0`, statement entries `>19E6–1A2C`, jump tables `>1C9C–1DE2` (MSB-set entries = handlers in BASIC GROMs 1–2 — ROM and GROMs are co-designed), support subroutines, `VPUSH >1EAA` (`XML >17` alt `>1E9C`), `VPOP >1F2E` (`XML >18`) | 📖 TI Intern |
| `>1FFC–1FFF` | trailing words `2A61 A38A` (Nouspikel labels them "checksum"; ❓ never referenced by code — D2's read census confirms). **Our tail ships all-zero** (no watermark — decision §10.5); the two words join the byte-identical set only if D2 finds a reader | ✅ dump |

Two structural facts drive the whole plan:

1. **The ROM is ~100% executable code + dispatch tables.** Unlike the GROM
   (fonts, keytabs — data cartridges read *by address*), almost nothing in the
   ROM is data an outside program reads. The compatibility surface is
   *behavioral*: fixed **entry addresses** + **register/scratchpad conventions**
   + **observable effects**. That makes the rewrite more demanding to get right
   (an interpreter, not screens) but clean IP-wise: original code honoring an
   uncopyrightable interface, with only a small, enumerable set of
   byte-identical interface data (vectors + dispatch tables — §3).
2. **We hold a perfect oracle and already own the probe rig.** `Machine::new`
   takes the ROM as a parameter (`crates/libre99-core/src/machine.rs:83`, `:319`
   ✅), the GROM tracer records every GPL fetch (`grom_record`/`grom_log`), and
   the entire libre99-gpl gate suite (title, menu, sweep, device I/O, interrupts,
   char-set, keyboard, TI PYTHON) boots `Machine::new(CONSOLE_ROM, grom)`
   with `CONSOLE_ROM = include_bytes!(roms/994aROM.Bin)`
   (e.g. `crates/libre99-gpl/tests/boot_trivial.rs:12` ✅). Point that constant at
   our image and the whole existing regression estate becomes the ROM's gate
   matrix — for free.
3. **Software enters the ROM's *interior*, not just its vectors.** TI Intern
   (p.7) warns that Extended BASIC branches directly into interior ROM
   subroutines, and TI's own GROMs reach handlers through the dispatch tables.
   Worse, the authentic code **harvests its own instruction words as data
   constants** in at least five places (the "NASTY" list: byte `@>0012` as a
   `LDCR` column selector, word `@>0032` (= the `LI R0,>0020` immediate) as
   the ISR's cassette-flag `COC` mask, word `@>0036` (a `JMP` opcode) as a
   `CZC` row mask, word `@>0072` / byte `@>0074` (the `LIMI` words) in
   KSCAN/CLEAR, byte `@>011B` as the status-clear `SZCB` mask — found by Lee
   Stewart's AtariAge OS project 📖, several binary-confirmed by our research).
   Consequence: §4 policy **P8 — address-exact routine entries** — and D1 must
   turn every harvested constant into an *explicit* named constant pinned at
   the identical address.

One more planning-relevant research result: **no open-source clean-room
replacement of this ROM exists anywhere** — only disassemblies (TI Intern),
the recovered original TI source (Lee Stewart's AtariAge thread — TI's
copyright, consult its *findings* only, exactly like Classic99), and hardware
substitutes that still carry TI's image. This rewrite would be a first.

---

## 2. The compatibility contract (what MUST be reproduced)

Three client classes consume the console ROM. Everything below is the
interface; the dossier (D1/D2) turns every ❓ into ✅ before the implementing
milestone starts.

### 2.1 The hardware/CPU contract (client: the chips)

- **Vectors** at `>0000` (reset) and `>0004` (level 1). All 99/4A interrupts
  arrive at level 1; the level-2 vector (`>83C0/>0A92`) points into the ISR's
  screen-blank fragment and is unreachable on this machine — under P8 it keeps
  the same value by construction.
- **Reset flow — deliberately tiny, match it exactly**: establish GPLWS
  `>83E0` with R13=`>9800` (GROM read port), R14=`>0100` (SPEED=1 / FLAGS=0),
  R15=`>8C02` (VDP write port); **one dummy GROM data read** (the `>1FFF`
  throwaway RECON R1 observed ✅); write GROM address `>0020`; clear the GPL
  status byte (`>006A`, itself a documented soft entry); fall into the `>0070`
  loop. **The authentic ROM does *no* self-test and touches *no* RAM, VDP, or
  9901 at reset** — scratchpad clearing, VDP setup, and interrupt arming are
  all GROM-side GPL (📖 TI Intern; consistent with RECON R1's warning that
  hardware scratchpad powers up random). Our kernel must be equally minimal or
  the conformance diffs and real-GROM boot will diverge. GPL `EXIT` and the
  ISR's QUIT path both re-enter `>0024` — warm reset ≡ cold reset. The machine
  state at first GPL fetch must match RECON R1's entry contract **exactly**.
- **GROM port discipline**: data `>9800`, address readback `>9802` (destructive
  read; resets the write flip-flop), data write `>9C00`, address write `>9C02`
  high-then-low; a data read costs a prefetch advance. The ISR's sound-list
  fetching must save/restore the GROM address around its reads, adjusting for
  prefetch (❓ pin the -1 adjustment by probe; the emulator model is
  `crates/libre99-core/src/grom.rs`, hardened by the historic boot bug —
  `docs/STATUS.md` "GROM address-port read corrupted the next write").
- **VDP discipline**: `>8800/>8802` read data/status, `>8C00/>8C02` write
  data/address; even-address-only decode (word writes reach the chip once —
  the disk-title bug, `docs/STATUS.md`); a status read returns the flags,
  **clears the top status bits, and resets the address byte-latch** (📖
  Classic99 `Tiemul.cpp:5152-5225`, "tested on hardware") — the ISR's status
  read is what clears the interrupt condition; address writes are
  LSB-then-MSB with prefetch inhibited for non-read setups.
- **9901/CRU**: keyboard **column select at CRU bits 18–20** (addresses
  `>0024/>0026/>0028`), row reads at bits 3–10, **alpha lock = bit 21**;
  ⚠ sources number the joystick columns differently (Classic99's tables say
  joy1/joy2 = columns 4/0, Nouspikel says 6/7 — a column-numbering-convention
  clash, not a hardware dispute; our `keyboard.rs` + the binary are the
  operative truth, D1 reconciles); VDP interrupt = CRU bit 2, and the interrupt
  reaches the CPU only while the ST mask ≥ 1 (level-1 vector `>0004`);
  peripheral-card CRU windows `>1000–>1F00` for the ISR card scan and DSR
  search; the 9901 interval timer (bit 0 = clock mode; decrements every 64
  CPU cycles 📖 `Tiemul.cpp:3599-3645,6240-6273`) is only minimally modeled in
  the emulator (`cru.rs:14-16` ✅) — the rewrite must not depend on unemulated
  timer behavior (only cassette needs it, and cassette is deferred).

### 2.2 The GPL contract (clients: our GROM, the authentic GROMs, 104 GROM-bearing cartridges)

- **The full 256-entry GPL ISA** — semantics per the authentic interpreter, not
  per our assembler's supported subset. The complete opcode map is already in
  the repo (`crates/libre99-gpl/src/isa.rs`, extracted from Classic99's table and
  cross-verified; RECON §8 ✅). Critical inclusions **our own GROM never uses
  but cartridges/BASIC do**:
  - **`FMT` (`>08`)** — the screen-format sub-language (own sub-ISA via its
    dispatch table at authentic `>0CDC`: HTEX/VTEX/HCHA/VCHA/ROW/COL/RPTB/
    ROW+/COL+/FEND; cursor kept at `>837E/>837F`; regular GPL opcodes are
    invalid inside FMT). Our toolchain deliberately rejects *emitting* it; the
    interpreter must *execute* it. 📖 TI Intern/Nouspikel gpl2; D2's opcode
    census measures which carts exercise it; pinned by differential probes.
  - **`PARSE` (`>0E`) / `CONT` (`>10`) / `EXEC` (`>11`) / `RTNB` (`>12`)** —
    dispatch into the BASIC ROM half (`>18C8/>1920/>1968/>19F0`), co-designed
    with BASIC's GROMs. Required for TI BASIC (and XB) on our ROM — scoped to
    M6, spec from TI Intern + probes.
  - **Indexed GAS addressing and `MOVE`'s C=1 (computed GROM source) form** —
    the GROM track *banned emitting* these after probes failed to match the
    documented story (RECON §7), but whatever the authentic ROM actually does
    **is the spec**; real cartridges may rely on it. D1 disassembles the
    authentic operand decoder to recover the true semantics; the probes then
    become *conformance* tests of our implementation against the real ROM.
  - `RAND` (`>02`, handler `>027A`) — result to `>8378`, seed at `>83C0`
    (which doubles as INTWS R0). Match the authentic algorithm so differential
    runs stay lockstep-comparable (§10 decision 4). ❓ algorithm from TI
    Intern.
  - `IO` (`>F4–F7`) — sub-ops per the authentic `>0CEC` table: 0/1 = arm a
    sound list in GROM/VDP (`>05D6` — sets `>83CC/>83CE` and the FLAGS
    `>01` bit); 2 = CRU input (`>05E8`); 3 = CRU output (`>05EA`,
    execution-verified RECON §11 ✅ with its scratchpad list format); 4/5/6 =
    cassette write/read/verify (`>1346/>142E/>1426`) — transport deferred,
    interface-correct behavior required.
- **Dispatch mechanics**: `BR`/`BS` 13-bit slot-absolute; `B`/`CALL` 16-bit;
  `CALL`/`RTN` sub-stack at `>8373`/`>8380+` (pre-increment store / read
  post-decrement); `CASE` PC += 2×value; `FETCH`; the GPL status byte `>837C`
  condition-bit rules per opcode (❓ per-opcode set/clear table from TI Intern +
  probe suite — subtle and load-bearing, e.g. the menu's `BS` after `SCAN`).
- **`MOVE` in all live variants** (GROM→CPU, GROM→VDP, CPU↔CPU, VDP↔CPU,
  VDP→VDP, →VDP registers, count-from-memory), **including the authentic
  per-byte GROM address rewrite on GROM-source moves** — behavior compatibility
  policy P4 (§4): cartridge-visible timing, the menu scan's 512-byte-window
  design, and LIMITATIONS L5's parity note all assume it.
- **`SCAN` (`>03`)** → shim `>02AE` (`LI R11,>0070`) falling into KSCAN: mode
  `>8374` — 0 = re-use the state saved at `>83C6`; 1/2 = left/right split
  keyboard + joystick 1/2 (column masks `>0FFF`/`>F0FF`); **3/4/5 = full scan
  in the 99/4 / Pascal / 99-4A-native translation states — the code subtracts
  3 and stores the result back into `>8374` *and* `>83C6`** 📖; result
  `>8375` (`>FF` none), joystick Y/X `>8376/>8377`, condition bit only on a
  **new** key (debounce via scan-code cells `>83C7–>83CA`). Side effects: on a
  detected key KSCAN **reloads VDP R1 from the `>83D4` copy** (un-blank) and
  resets the `>83D6` timeout. Translation is table-driven from **GROM 0**
  (bases `>16E0` deflection / `>1730` shift / `>1760` FCTN / `>17C0` split 📖
  TI Intern — reconcile with RECON §9's observed first-entry offsets
  `>16EA/>1705/>1735/>1765/>1795/>17C8` in D1; both GROMs already ship these
  tables ✅, the ROM must index them identically).
- **`XML` (`>0F`)** through the table-of-tables semantics of §1: high nibble
  picks the table pointer at `>0CFA+2n`, low nibble the entry. Tables 2–E point
  at RAM/cartridge addresses (`>2000`, `>3FC0`, `>4010`, `>6010`, `>7000`, …)
  ✅ — so E/A low-memory utilities and cartridge XML tables keep working with
  **no ROM content behind them**; only tables 0/1 (and the `>8300` vector
  table F) are ROM-resident work.
- **The ISR's GPL-visible duties, in the authentic order** (§1's `>0900` row;
  📖 order binary-verified): cassette-flag check → card-chain walk (non-VDP) →
  sprite auto-motion (count `>837A`, motion table at VDP `>0780`) → sound list
  (`>83CC/>83CE`, GROM-vs-VDP source per FLAGS bit `>01`; format RECON ✅) →
  QUIT (`CZC` mask `>1100`) → VDP status read → `>837B` (clears the interrupt)
  → screen-timeout `>83D6` (+2 per tick, blanks at 0 via `>83D4`) → `>8379` +=
  SPEED (`>83FC`) → user hook `>83C4` (runs on WS `>83E0`, `BL *R12`/`B *R11`)
  → **`CLR R8`** (GPLWS R8 is zeroed on *every* interrupt — interpreter code
  must not keep state there) → `RTWP`. **`>83C2` disable bits: `>80` skip all
  VDP duties, `>40` no sprite motion, `>20` no sound, `>10` no QUIT** 📖.
  Conformance harness pins all of it against the oracle.
- **Scratchpad ownership** exactly per RECON's map (`>8370–83FF` SYS cells) —
  now read from the *other side*: the ROM is the party that initializes and
  maintains them.

### 2.3 The machine-language contract (clients: 33 ROM-only carts, hybrid carts, E/A-style software, DSRs)

- **Fixed entries**: `BLWP @>0000` soft reset; the `>000E` scan entry
  (`B @>02B2` behind it ✅; the documented ML caller idiom is
  `MOVB <mode>,@>8374` / `LWPI >83E0` / `BL @>000E` / `LWPI <own WS>` — 📖
  Classic99 `addons/makecart.cpp:1156-1168`; KSCAN saves R11 at `>83D8` ✅ and
  returns `B *R11`); the interpreter re-entries `>0016` (opcode in R9) and
  `>001C` (fetch, no interrupt window) plus soft entries `>006A`/`>0070` 📖;
  `>0020` = the CLEAR/BREAK test (`BL @>0020`, EQ = FCTN-4 down) 📖. **Public
  addresses are frozen** — D1 re-verifies each contract; our image places
  compatible code at the *same addresses*.
- **`XML >F0` dispatch** (vector at `>8300`) — how the menu launches ML carts
  (RECON §2 ✅), and the general ML↔GPL trampoline.
- **XMLLNK return conventions** — the `>83FA`/`>8372` cells our own DSRLNK
  already leans on (RECON "Console device I/O" ✅), and the documented `>0070`-
  family re-entry points.
- **DSR calling convention**: the `XML >19` device search walks CRU bases,
  enables card ROMs, matches names in card DSR headers; `XML >1A` calls the
  DSR **on the GPL workspace `>83E0` with R12 (`>83F8`) = the card's CRU base,
  R13/R15 = GROM/VDP ports intact, `>83D2` = DSR entry, and interrupts masked
  (`LIMI 0`) for the duration** — the exact set of invariants the genuine disk
  DSR asserts (📖 Classic99 `disk/disk.cpp:327-367`: "Calling DSR with
  interrupts enabled will randomly crash on hw!", "MUST store the CRU base…");
  powerup scan protocol via `>836D = >04` / done-flag `>83D0` / `>8370`
  VRAM-top (all ✅ from the GROM track's M7 work). D1 pins the remaining
  details (header walk order, error return path).
- **The ISR peripheral protocol**: card-ISR scan order and the `>83C4` user
  hook (E/A convention: hook runs with which workspace/registers ❓).
- **Data reads of ROM addresses by external code** — presumed none 📖, but D2's
  read-census verifies (if some cart reads ROM bytes as data, those bytes join
  a tiny DATA-MUST-MATCH set, GROM-track style).

### 2.4 What is explicitly NOT contract

- Register allocation inside routines, instruction-by-instruction expression,
  and cycle-exact timing of individual routines (frame-level timing parity is
  the bar, not cycle parity). *Interior routine **addresses** are, however,
  contract by default* — policy P8 (§4): TI Intern warns XB branches into ROM
  interiors, so entry addresses of named routines are frozen; only truly-dead
  interior space (per D2's census) is free layout.
- Fixed *interior* PCs that third-party tools hook — e.g. Classic99's
  paste-injection intercept fires at the authentic KSCAN's store-to-`>8375`
  instruction (PC `>0478`, `Tiemul.cpp:4003-4008`). Our ROM runs on our
  emulator; such tool hooks are not a compatibility surface.
- TI's expression: comments, style, exact instruction sequences. We never copy
  them — clean-room per §4.
- The cassette transport (deferred; interface errors must match).

---

## 3. What we replace vs. preserve (original-content policy)

- **Replaced (everything):** all 8 KiB of executable content is written new, in
  our own source dialect, from the *behavioral* spec in the dossier. There is no
  creative on-screen content in the ROM (that all lives GROM-side), so unlike
  the GROM there is no artwork to re-imagine — the originality lives in the
  code itself.
- **Preserved (the uncopyrightable interface):** entry addresses, vector
  values, scratchpad cell meanings, table *formats* and *locations* (`>0CFA`
  table-of-tables layout, XML table addresses), register conventions
  (R13/R14/R15 GPLWS roles), and observable behaviors — functional facts
  required for interoperability, same policy as the GROM's header/keytab stance
  (`../grom/README.md` "Provenance / interface-data policy").
- **Byte-identical content (policy APPROVED by Joel, 2026-07-02; scoped
  2026-07-04 with P8's scoping):** small and enumerable — the vectors, the
  fixed data words (`>000C`, `>004C`, `>0010–0015`), the **XML master table**,
  and the NASTY constants are byte-identical by construction (address lists +
  functional constants: uncopyrightable interface data, enumerated in
  `rom/SURFACE-MAP.md` with identity gates, exactly like the GROM's
  DATA-MUST-MATCH set). The six **dispatch tables** and XML tables 0/1 keep the
  authentic *location, size, and structure* but carry **our** handler addresses
  wherever a body is not address-pinned (P8 scoping) — structurally identical,
  value-equivalent dispatch. Everything else — all actual code — is original
  expression at pinned entry addresses. If D2/M7 evidence ever shows software
  reading unpinned table values or entering unpinned interiors, those entries
  join the pinned set.
- **No provenance watermark (DECIDED — Joel, 2026-07-02, §10.5):** TI shipped
  **no** string of its own in the console ROM (scanned: zero printable runs;
  its © text is GROM-side, already replaced by our GROM). Per Joel: no string
  if TI doesn't have one to replace — our image carries none; authorship is
  recorded by this repo, the source headers, and git history. Our tail stays
  all-zero unless D2 finds a reader of the authentic `>1FFC` words.

---

## 4. Strategy & method

- **P1 — Dossier before code.** No component is implemented before its
  contract row in `rom/RECON.md` (the ROM dossier, sibling of `../RECON.md`)
  is ✅-tagged with probe evidence or disassembly-derived fact. The GROM
  track's five field bugs all came from surface nobody had enumerated; here the
  surface is enumerated *first* (D1/D2 are the first milestones, and M7's
  tripwires keep it honest).
- **P2 — Differential-first, always.** The authentic ROM under the same
  emulator is a perfect oracle. Every behavior question becomes: run the same
  GPL/ML stimulus under `Machine::new(TI_ROM, g)` and `Machine::new(OUR_ROM,
  g)`, diff the observables (scratchpad, VDP regs, VRAM, PSG, cart RAM, the
  GROM fetch stream, PC-region traces). The GROM fetch log doubles as a
  **GPL-semantics trace**: identical GROM + identical inputs ⇒ the fetch
  streams must match to the instruction (modulo ISR excursions, which the
  harness filters — `../RECON.md` "GPL execution model").
- **P3 — Behavior-compatible, bug-for-bug.** Where the authentic ROM's behavior
  is observable by software, we reproduce it — including its quirks (the boot's
  `>1FFF` throwaway GROM strobe, per-byte GROM addressing, KSCAN debounce
  idiosyncrasies, the condition-bit rules exactly as implemented). "The
  authentic probes' observed behavior IS the spec."
- **P4 — Authentic performance envelope.** GROM-source `MOVE`s rewrite the GROM
  address per byte, like the real ROM (RECON §10) — menu-scan timing, L5's
  parity claim, and fair perf comparisons all depend on it. Faster designs are
  a labeled stretch flavor, never the default (§10 decision 3).
- **P5 — Clean-room IP discipline.** Consult Classic99 + literature + the
  authentic binary's *behavior*; disassemble the authentic ROM **only to
  extract interface facts** (addresses, table formats, semantics) recorded in
  the dossier as specifications; write our implementation from the dossier.
  Never transcribe TI instruction sequences. (The dossier is the firewall: spec
  in, original code out — the exact discipline `../RECON.md` established.)
- **P6 — Gates are cargo tests; artifacts are committed.** Every milestone
  lands with `cargo test`-runnable gates (fast pre-commit tier + `#[ignore]`d
  deep tier), and `rom/console-rom.bin` is rebuilt + committed whenever source
  changes, like `grom/console-grom.bin`.
- **P7 — One source of truth per fact.** ROM facts live in `rom/RECON.md`;
  shared scratchpad/GPL facts stay in `../RECON.md` (cross-referenced, not
  duplicated); build/run facts in `rom/README.md`.
- **P8 — Address-exact routine entries.** Every named routine and table in the
  authentic map (§1) starts at the **same address** in our image, enforced by
  `AORG` + the layout-assertion gate. Rationale: XB and TI's GROMs enter ROM
  interiors directly (structural fact 3); the dispatch tables then carry
  identical values, which keeps differential traces aligned; and the NASTY
  harvested constants stay where their consumers look — as explicit, named
  `DATA`/`EQU` constants in our source, never as accidental instruction
  encodings. Escape hatch: if an original routine can't fit its authentic
  slot, its entry keeps the address and trampolines to a continuation in
  known-dead space (D2-verified) — the entry contract survives, the interior
  moves. Only D2-proven-dead regions are free layout. (**"Dead" here means only
  that no code *enters* the region, so its bytes may be relocated — it never
  means the region's *function* may be skipped; see P9.**)
  **Scoping (2026-07-04, from R-3's evidence + M1 implementation reality):**
  P8 pins, at authentic addresses: (i) every *externally-entered* address (the
  D2-validated public set: vectors, `>000E/>0016/>001C/>0020/>0024/>006A/
  >0070`, and the `>0078/>007A` warm-entry geometry inside the loop); (ii) all
  **table locations** (`>0C36/>0C3E/>0C7E/>0CCE/>0CDC/>0CEC/>0CFA/>0D1A/
  >12A0`), the XML **master-table values**, the vectors, and the NASTY
  constants; (iii) the literature-documented routine entries external ML is
  known to reach or that XB plausibly enters (KSCAN `>02B2`, CLEAR `>04B2`,
  the ISR `>0900`, SROM `>0AC0`/SGROM `>0B24`, FMT `>04DE`, the specials
  `>0270`, the operand engine `>077A`, RTN/RTNC/CALL `>0838/>083E/>085A`, the
  GROM-push helper `>0864`, `>089A`, and the BASIC four + FP entries in M5/M6).
  **Individual GPL opcode-handler *bodies* are NOT address-pinned**: TI packed
  them with shared-tail tricks into slots as small as 4 bytes, so pinning them
  would either coerce TI's internal structure (against P5) or burn ~40
  trampolines (a fit risk); no software enters them (the R-3 census: ML enters
  only `>000E`), and the GPL-fetch-stream gates are blind to CPU-internal
  addresses. Our dispatch tables therefore keep the authentic *structure and
  location* but carry **our** handler addresses where TI aliased or packed.
  The tripwire: the `entry_census` P8 test (and its M7 full-corpus form) fails
  if any software is ever observed entering an unpinned interior — that address
  then joins the frozen set and gets pinned.
- **P9 — Functional completeness is the acceptance bar, not usage** (§0's
  overriding goal, as a working rule). The unit of "done" for a milestone is
  *every functional element of the authentic ROM in that milestone's scope
  implemented **and** differentially verified* — **not** "everything the corpus
  was observed to touch." The authentic ROM is a total oracle: a microtest can
  drive **any** opcode, addressing form, XML entry, KSCAN mode, ISR duty, FP
  operation, or BASIC-ROM entry *directly* under both ROMs and diff the result,
  with **no cartridge needed to call it**. So there is never an excuse to leave
  an implemented element untested "because nothing exercises it." Each
  milestone's gate therefore requires a passing differential microtest for
  **every** element in its scope, enumerated from the D1 dossier (not from the
  D2 census). The D2 census only **orders** the work and **targets extra**
  adversarial/fuzz scrutiny at hot paths; it is **not** a checklist of what to
  build or verify. This principle **overrides** any "prioritized / observed-
  subset / at lower priority / on demand" phrasing anywhere in this document —
  such phrasing means *sequence and emphasis*, never *omission*.

---

## 5. Milestone T1 — the toolchain (assembler upgrades + recon tools)

`libre99-asm` today: full TMS9900 ISA, two-pass, E/A-style source, `BYTE/DATA/
TEXT/BSS/EVEN/EQU`, but **cartridge-shaped**: origin forced to `>6000`,
auto-header, `.ctg` packaging, and `AORG` explicitly rejected
(`crates/libre99-asm/src/lib.rs:367-370` ✅). Upgrades (all fit the existing
architecture; `Options { base, auto_header }` already exists ✅):

1. **Raw-ROM mode**: `base = >0000`, `auto_header = false`, output = the 8 KiB
   zero-padded image (the existing `Assembly::rom` bank pad is already exactly
   8 KiB ✅). No `.ctg`, no header synthesis, entry = the reset vector itself.
2. **`AORG >addr`** (absolute origin, multiple regions): required to pin public
   addresses (`>0000` vectors, `>000E` stubs, `>0024` reset, `>0070` family,
   `>02B2` KSCAN, `>0900` ISR, `>0CFA` tables — every frozen address gets its
   own `AORG`d region). Semantics: forward-only placement with zero-fill gaps;
   **error on overlap** (two regions colliding is always a bug); keep `$`/label
   semantics absolute. E/A's `AORG` is the model (`assembler/ASSEMBLER.md`
   FR-6/§6 specifies it; the E/A manual PDF is in `assembler/`).
3. **Layout assertions**: a directive or builder-side check asserting
   `label == >addr` for the frozen-address table (so a size regression in an
   interior routine can never silently shift a public entry). Cheapest form: a
   Rust-side check in the builder against `Assembly::symbols` ✅ (already
   emitted, `lib.rs:78`).
4. **`COPY`/include** (FR-9) — the source will be well over a thousand lines;
   split by component (`kernel.asm`, `gpl-core.asm`, `gpl-move.asm`,
   `gpl-fmt.asm`, `kscan.asm`, `isr.asm`, `xml.asm`, `fp.asm`, `dsr.asm`,
   `tables.asm`) with one top-level `console.asm`. (Alternative if COPY proves
   heavy: the builder concatenates a fixed file list — decide in T1; either
   satisfies "all source in this folder".)
5. **Listing + symbol map output** (FR-5 subset): address/object/source listing
   and a symbols JSON — indispensable for debugging an 8 KiB image and for the
   layout-assertion gate.
6. **`build_console_rom()`** — a `pub fn` in `libre99-asm` (mirroring
   `libre99_gpl::system_grom::build_console_grom()`) that assembles the sources
   from this folder and returns the 8 KiB image; plus a CLI verb
   (`libre99asm rom <out.bin>`, mirroring `libre99gpl console`). `libre99-gpl` already
   depends on `libre99-asm` (`crates/libre99-gpl/Cargo.toml:23` ✅), so the matrix
   gates in `libre99-gpl/tests/` can build both firmware images in-memory.
7. **A TMS9900 disassembler** (`libre99asm dis`, inverse of `isa.rs`, exactly like
   `libre99gpl dis`): needed by D1 to map the authentic ROM and by debugging
   sessions forever after. Round-trip tests against our own assembler; spot-
   validated against Classic99's debugger output (consulted, never copied).

**Gate T1:** golden encodings still green; `AORG`/raw-mode unit tests; a
**tracer-bullet ROM** — vectors + a reset routine that sets a VDP backdrop
color and spins — assembles to 8 KiB, boots in
`Machine::new(&tracer_rom, any_grom)`, and the test observes the backdrop
change: proof that *our code executes from `>0000` on our emulator*. (The ROM
analogue of M0's `BACK`-reaches-VDP-R7 gate.)

---

## 6. Milestones D1/D2 — the dossier (recon before code)

### D1 — the static dossier: `rom/RECON.md` + `rom/SURFACE-MAP.md`

Using `libre99asm dis` on the authentic image (consult TI Intern/Nouspikel to
*identify*, never to copy):

- **Component map**: every byte of the authentic 8 KiB classified
  (`VECTORS / ENTRY-STUB / KERNEL / GPL-CORE / GPL-op-<name> / KSCAN / ISR /
  XML-TABLE / FP / DSR-LINK / CASSETTE / DATA / DEAD`) — the ROM sibling of the
  GROM census, seeded from §1's table. Enumerate: the exact `>000E/>0014/
  >0016/>001A/>001C/>0020` stub contracts; the `>0070` warm-entry family; the
  full XML table 1 decode; the FP entry list with per-routine scratchpad
  contracts (FAC/ARG/status/error cells); the ISR duty order; KSCAN's mode
  table and debounce cells; the `>83C2` bit map; the interpreter's
  condition-bit rules per opcode; the authentic indexed-GAS and MOVE-C=1
  semantics (resolving RECON §7's "banned" mystery with the real decoder in
  hand); the RAND algorithm; the FMT sub-ISA (authentic home `>04DE–05A1`);
  which side (ROM kernel vs GROM boot) writes the `>8359` version cell (99/4A
  = `>02` 📖 Classic99 `makecart.cpp:1540`) and any sibling cells; **the
  complete NASTY harvested-constant list** (structural fact 3 — each becomes a
  named constant at its exact address in our source, with a doc row); the
  KSCAN GROM-table base-vs-first-entry reconciliation (`>16E0` vs `>16EA`
  etc.) and joystick column numbering; **XTAB's true 12-entry extent** (`XML
  >1C–>1F` index past the table into CFI's first instruction words — decide
  reproduce-the-accident vs document-as-undefined, informed by D2).
- **The frozen-address table**: every address external software may enter, with
  its contract — this becomes the `AORG` skeleton of our source and the
  layout-assertion gate's input.
- **The complete element enumeration** (the P9 spine): an explicit,
  machine-checkable list of *every functional element* the ROM must implement —
  each of the 256 opcodes + addressing forms, each FMT sub-op, each XML table-0/1
  entry, each KSCAN mode, each ISR duty + `>83C2` bit, each FP/conversion
  routine + error code, each BASIC-ROM entry, each service stub, and the
  vestigial items. This list is the **authoritative source of the M-gates'
  completeness assertions** (§0's checklist, §7 M4/M5): the coverage-completeness
  test cross-checks it against the set of differential gates and fails on any
  element without one. Keep it as data (a table/const the tests read), so
  "complete" is enforced mechanically, not by eyeball.
- Record everything as **specification prose + tables** in `rom/RECON.md`
  (✅-tagged with dump/disasm evidence), never as TI code.

### D2 — the dynamic dossier: censuses over the real corpus (real ROM installed)

New diagnostics-only instruments in `libre99-core` (same pattern as `grom_log` and
the GROM track's read-bitmap; all behind existing "instruments" conventions,
`../DEBUGGING.md` §Instruments).

⚠ **What the censuses are for (and are not).** Per P9 / §0's overriding goal,
these measure *usage* only to **order the work** and to **aim extra
adversarial/fuzz scrutiny** at the paths real software hammers. They are **not**
a list of what to implement, and **not** the source of the per-element test
matrix — that list comes from the **D1 dossier's complete enumeration** (every
opcode, XML entry, KSCAN mode, ISR duty, FP op, BASIC entry), and **every** item
on it gets implemented and differentially gated regardless of whether any census
ever counts it. A census returning "count 0" for an element changes its test
*priority*, never its *inclusion*.

1. **ROM entry census**: log every PC transition from ≥ `>2000` into
   `>0000–1FFF` (entry address + source region), and every vectored entry
   through `>0000`/`>0004`. Run the 137-cart sweep + launches + input mash (reuse the
   GROM track's coverage-sweep driver) + a TI BASIC session + ToD disk load.
   Output: the **empirical public-entry set** → `rom/SURFACE-MAP.md`. Expected:
   vectors, `>000E`, the `>0070` family, XML targets — anything else is a
   finding.
2. **XML census**: log every `XML` dispatch (opcode byte → table/entry →
   target). Shows which table-0/1 entries the corpus + BASIC exercise, to
   *order* the work. **All** table-0/1 entries are implemented and gated
   regardless (M5/M3); an unexercised entry is verified by a directly-driven
   `XML` microtest, not deferred.
3. **GPL opcode census**: count executed GPL opcodes per cart (dynamic
   complement of the other track's static census), to *order* the M4 work by
   real-world load. The M4 gate requires a differential microtest for **every
   one of the 256 opcodes** and every addressing form (driven directly under
   both ROMs), plus implementation of all — the census picks the order and where
   to add fuzz weight, not which ones get a test.
4. **KSCAN mode census**: which `>8374` modes the corpus uses, to *order* the
   work — **all** of modes 0–5 and every translation state are implemented and
   gated by directly-driven `SCAN` microtests regardless.
5. **ROM data-read census**: CPU data reads of `>0000–1FFF` from PC ≥ `>2000`
   (expected empty; guards §3's byte-identical claim and settles whether the
   authentic `>1FFC` tail words have any reader).

**Gate D1/D2:** `rom/RECON.md` + `rom/SURFACE-MAP.md` committed; every §2 ❓
either ✅ or explicitly deferred with rationale; censuses committed as a
re-runnable `#[ignore]`d test + a report (`rom/ENTRY-CENSUS.md`); instruments
merged without disturbing the GROM track's instruments.

---

## 7. Milestones M1–M6 — implementation ladder

Each milestone: implement from the dossier → per-component differential probes
→ land gates → update `rom/RECON.md`/`STATUS`. House rules apply (probes before
fixes, regression test per fix, never copy TI bytes).

| # | Milestone | Contents | Gate (all `cargo test`-runnable) |
|---|---|---|---|
| **M1** | **Kernel + GPL core** | Reset kernel (RECON-R1-exact entry state); GROM/VDP port drivers; interpreter fetch/dispatch skeleton; format-1 (`ADD…SRC` byte/word, imm/mem) + format-5 (`ABS…DECT`) ops; `B/CALL/RTN/RTNC/BR/BS/CASE/FETCH/PUSH`; `ST/EX` family; `BACK/ALL`; core `MOVE` variants (GROM→CPU, GROM→VDP w/ per-byte addressing, CPU↔CPU, →VDP-regs); condition-bit rules. *Because the title boot also arms the ISR and runs the power-up scan, M1 ships **minimal-correct early versions** of `IO` CRU-output, `XML >19/>1A`'s no-cards path, and a no-key `SCAN` — completed to full spec in M2/M3 (tracked in RECON §15; P9 still requires the full versions there, this is sequencing not omission)* | `boot_trivial.rs` passes on our ROM; **our GROM's title paints pixel-identical** under our ROM (`title_screen.rs` matrix run); GPL-fetch-stream diff vs authentic ROM through the title sequence is clean |
| **M2** | **Input + ISR** | KSCAN all modes + GROM keytab indexing + debounce + joysticks + alpha lock; `SCAN` opcode; full ISR (VDP status, `>83C2` gates, sound lists w/ GROM address save/restore, sprite motion, `>8378/>8379`, QUIT, screen timeout, user hook, card-ISR scan); `IO` CRU in/out | `interrupts.rs`, `menu.rs`, `keyboard.rs`, `char_set.rs` green on our ROM; key-wait/beep/QUIT behave; conformance checkpoint diffs (scratchpad+VDP+VRAM at title/menu/post-launch) whitelist-clean vs authentic |
| **M3** | **XML + dispatch + device I/O** | `>0CFA` table-of-tables (ours); table F/`>8300` vector; `XML >F0`; XML table 1: DSR search (`>19`) + DSR execute (`>1A`) + powerup protocol; XMLLNK return conventions; sub-stack trampoline behaviors | `device_io.rs` (ToD loads QUEST from disk) + M2-probe dispatch tests green on our ROM; DSR powerup lowers `>8370` identically; `sweep.rs` class samples launch |
| **M4** | **Interpreter completeness (non-BASIC)** | `FMT` sub-language **and every FMT sub-op** (authentic table `>0CDC`); authentic indexed-GAS + MOVE-C=1 semantics; `RAND` parity; **all** `I/O` sub-ops (cassette 4/5/6 = bit engines present, behavior-correct, transport hardware-gated §10.2); `COINC`, `SWGR/RTGR`, the "incompletely decoded" dispatch quirks reproduced faithfully; and **every** remaining opcode/addressing-form except the BASIC four (M6) | **Complete per-opcode differential microsuite — a passing test for EVERY one of the 256 opcodes and each addressing form** (driven directly vs authentic; census picks order + fuzz weight, not coverage); a completeness assertion that the microsuite's opcode set == the full ISA; **137-cart sweep + class launches under our ROM × our GROM**; random-GPL fuzz diff (§8) clean over N seeds |
| **M5** | **Floating point + conversions** | The **complete** FP package: FADD/FSUB/FMUL/FDIV/FCOMP + stack variants (S-ops on the `>836E` VDP value stack), ROUND/ROUND1/STST/OVEXP/OV, CSN/CSNGR/CFI — radix-100 format, FAC/ARG/error cells per dossier; **XML tables 0 and 1 fully wired, every entry live**. (Transcendentals are GROM-0 GPL, not this ROM — §0 exception 2, not a gap.) | FP differential microtests over **every** table-0/1 routine (planted FAC/ARG incl. edge cases: signs, zero, exponent extremes, rounding, all `>8354` error codes) match authentic bit-for-bit in FAC/ARG/status; a completeness assertion that every table-0/1 entry has a gate; BASIC-exercised routines get *extra* scrutiny (order/weight), none are skipped |
| **M6** | **The acid test: authentic GROM + the BASIC ROM half** | `PARSE/CONT/EXEC/RTNB` (`>18C8/>1920/>1968/>19F0`) + statement entries + the `>1C9C` jump tables (MSB-set = GROM-side handlers) + the symbol-table package (`SMB/SYM/ASSGNV/VPUSH/VPOP`, `PGMCH`) — co-designed with BASIC's GROMs 1–2; then fix whatever else the authentic console GROM exposes | `boots_to_master_title_screen` with **our ROM + TI GROM**; authentic menu lists/launches carts; **TI BASIC differential smoke scripts** (arithmetic incl. floats, strings, FOR/GOSUB, PRINT formatting, error messages) byte-identical on screen vs authentic ROM; 137-cart sweep under our ROM × TI GROM |

Two closing milestones ride on top of the ladder: **M7 — hardening** (the full
§8 matrix/coverage/robustness/perf gates turned on and green, the D2 censuses
re-run as permanent tripwires) and **M8 — production packaging** (§9's polish,
the committed artifact, `rom/README.md`, docs sync, and the embed decision to
Joel). They are specified by §8/§9 and packaged as chunks R-9/R-10.

> **The M6 deferral policy (Joel, 2026-07-05).** **M6 — TI BASIC — is
> deferred indefinitely.** The execution order is **M4 → M5 → M7 → M8**,
> skipping M6. Rationale: with M4 complete and M5 landing, everything
> *except* the built-in BASIC programming experience runs — game and tool
> cartridges (GPL and ML), disk device I/O, the full non-BASIC interpreter —
> and the FP package (M5) is justified independently of BASIC (the GROM-0
> transcendentals and ordinary GPL carts call the XML table-0/1 routines).
> What M6 alone gates: the authentic console GROMs' TI BASIC (and any
> cartridge that calls the BASIC-support surface — `PARSE/CONT/EXEC/RTNB` or
> `XML >13–>18`; Extended BASIC is the likely candidate).
>
> **Un-deferring M6 requires a written justification**: name the real
> program/flow that needs it. The instruments that surface such a program
> are already armed — the four opcodes and the `XML >13-18` entries are
> loud stubs (breadcrumb `>837D`), the 137-cart sweep would show the
> breadcrumb, and `gpl_opcode_sweep.rs::m6_deferred_opcodes_hit_the_loud_stub`
> fails the moment anyone implements the surface without first amending
> this note with the justification.
>
> **Sequencing note (the "parallel M4/M5" question):** M4's remaining work
> and M5's first step (the great squatter relocation) both edit
> `console.asm` and the same address ledger, so they were executed
> **sequentially in one session**, not as parallel branches — same
> throughput, no merge risk. The plan's milestone numbering is unchanged;
> only the execution order moves M6 to the indefinite tail.

Sizing note (measured from the authentic map, §1): kernel + interpreter +
KSCAN + ISR + SROM/SGROM + dispatch tables = `>0000–0CF9` ≈ 3.3 K; FP +
conversions ≈ 1.5 K; BASIC-support XML package ≈ 0.75 K; the BASIC ROM half ≈
1.8 K; cassette engines ≈ 0.65 K (transport deferred — that space is our
slack). P8 pins entries to authentic addresses anyway, so each routine has an
authentic-sized budget with the trampoline escape hatch for overflows.

---

## 8. Testing strategy (the matrix, the microsuite, the fuzz, the perf report)

- **The firmware matrix.** Parameterize the existing libre99-gpl gates over
  `[TI_ROM, OUR_ROM] × [TI_GROM, OUR_GROM]` (a tiny helper: `for rom in
  firmware_roms() { … }`; our-ROM rows activate as milestones land). The full
  existing estate — title, menu, sweep, device I/O, interrupts, keyboard,
  char-set, TI PYTHON — becomes the ROM's regression net without writing new
  scenario logic.
- **Per-opcode conformance microsuite** (the ROM sibling of the GROM's probe
  files): for each GPL opcode/addressing form, a small GPL snippet assembled
  in-memory, run under both ROMs from a controlled machine state, with
  scratchpad/VDP/GROM-counter/status-byte diffs asserted empty. Ordered by the
  D2 census; kept fast (they're the pre-commit tier).
- **Random-GPL differential fuzz.** Generate bounded random GPL programs from
  `libre99_gpl::isa` (deterministic seeds; exclude `SCAN`/`IO`/`XML`/GROM-writes;
  `RAND` allowed once parity lands), run K instructions under both ROMs, diff
  full observable state. Cheap, brutal, catches operand-decode corner cases no
  hand-written test imagines. `#[ignore]`d deep-tier with committed seeds.
- **GPL-fetch-stream lockstep**: for scripted boots (title, menu, launch), the
  `grom_log` under our ROM must match the authentic ROM's log
  instruction-for-instruction (ISR excursions filtered) — the strongest
  whole-flow equivalence signal we have, and it's already built ✅.
- **Conformance checkpoints** (the GROM track's A3, reused): scratchpad
  `>8300–83FF` + VDP regs + VRAM tables diffed at title/menu/post-launch
  checkpoints across the matrix, with a commented whitelist of intended
  differences (ours ≈ none for combos 1v3 and 2v4 beyond timing counters).
- **Robustness probes** (1981 bar): CS1/garbage-device PABs error authentically
  (never hang); QUIT/reset storms; menu with ≥10 entries; ISR under pathological
  sound lists; all launched-cart input mash from the coverage sweep.
- **Performance report** (Joel's explicit ask): extend the planned
  `perf_parity` concept to the matrix — frames-to-settled-title,
  frames-to-menu-complete, and host wall-clock per emulated frame for all four
  combos + a cartridge load; assert our-ROM combos within ×1.25 of their
  authentic-ROM counterparts; commit the numbers to `rom/README.md` and keep
  the test as the regression tripwire. (Expectation: near-identical — P4
  reproduces the authentic per-byte GROM addressing that dominates.)
- **Two-tier gating**, house pattern: fast suite pre-commit (< ~1 min);
  `#[ignore]`d deep tier (full sweeps, fuzz, coverage, perf) before
  demos/releases and after multi-fix sessions.

---

## 9. Emulator & app integration

- **No new flags needed for the core ask** — `--system-rom`/`--system-grom`
  already exist, compose with all media flags, and skip save-state resume
  (`crates/libre99-app/src/{cli,main}.rs` ✅). §0's four commands are the
  user-facing matrix, to be documented in `rom/README.md` and the repo README
  when the artifact ships.
- **Nice-to-haves (M8 polish, small):** log the active firmware combo at boot
  (both paths already log overrides); show a firmware tag in the window title
  (like the media tags) so screenshots in comparisons are self-labeling;
  `--list` mentions the two committed original-firmware paths.
- **Instruments stay diagnostics-only** (entry/XML/opcode censuses, PC bitmap)
  — same discipline as `grom_record`: zero cost unless enabled, never wired
  into release behavior.
- **Embedding decision** (Joel's, at M8): keep TI images the embedded default
  with our firmware as committed artifacts + flags (today's GROM stance), or
  embed ours and make TI the flag. Plan recommends: revisit only after M7's
  matrix has soaked; flags are sufficient for the mix-and-match goal.

---

## 10. Decision record — ALL decided by Joel, 2026-07-02

Executing sessions: these are **settled** — do not re-litigate them; §8 of
QUALITY-ASSESSMENT-style triage does not reopen them. Joel also approved the
**byte-identical interface-data policy** (§3) and **P8 address-exact layout**
(§4) the same day, and approved this plan as final ("enshrined") pending
implementation. The only decision deliberately left for later is the M8
embed-default question (§9), which needs M7's soak data to answer.

1. **FP package: IN SCOPE — decided.** Joel: "We need this to be authentic."
   TI BASIC/XB and XMLLNK-using software require it; the M6 acid test depends
   on it. The M5→M6 ordering stands as written.
2. **Cassette transport: DEFERRED — decided, durably documented.** The
   emulator has no tape hardware; the rewrite ships **interface-correct
   CS1/CS2 error behavior only** (GPL `I/O 4/5/6` stubs + DSR error path that
   fails the authentic way, never hangs). The ROM-side transport (bit engines
   + `>1404` timer-ISR regime) is built **only when the emulator grows
   cassette hardware** — commission both together (ROADMAP §6). This decision
   is recorded in: this section; §0 out-of-scope; **`docs/ROADMAP.md` §6**;
   a source comment at the unwired CRU bits (`crates/libre99-core/src/cru.rs`);
   and — **mandatory for the executing sessions** — a comment block at the
   ROM source's I/O dispatch/cassette stubs and a row in `rom/README.md` +
   `rom/SURFACE-MAP.md` (`CASSETTE` regions classified `DEFERRED-BY-DECISION`,
   not `DEAD`), so the deferral survives every future doc/code pass.
3. **Turbo ROM flavor: NOT NOW — decided.** Exactly-authentic timing is the
   only build; a labeled `console-rom-turbo.bin` remains a recorded stretch
   idea *after* production, never the comparison baseline.
4. **RAND parity: MATCH EXACTLY — decided.** Joel: "required for functional
   equivalence." Reproduce the authentic PRNG algorithm + `>83C0` seed
   semantics bit-for-bit; document algorithm/constants in the dossier like
   the keytab layout (functional facts, uncopyrightable).
5. **Naming: DECIDED (proposal adopted); watermark: NO STRING — decided.**
   Source `rom/console.asm` + per-component includes; artifact
   `rom/console-rom.bin`. Research answer that settled the watermark
   (2026-07-02, scanned the binary): **TI shipped no provenance/ASCII string
   in the console ROM at all** — zero printable runs ≥ 4 chars; TI's © text
   lives in the GROM title screen (already replaced by our GROM). The ROM
   tail is 20 zero bytes + the unexplained words `2A61 A38A` at `>1FFC`.
   Joel: *no provenance string if TI doesn't have one to replace* — our
   image carries none; the tail ships all-zero (the `>1FFC` words join the
   byte-identical set only if D2 finds a reader). Authorship is recorded by
   the repo, the source headers, and git history.

---

## 11. Risks & mitigations

- **The interpreter's condition-bit/status subtleties** (per-opcode set/clear,
  compare bias, `RTN` vs `RTNC`) — *the* correctness hazard; a wrong bit sends
  every `BS/BR` in some cart down the wrong path. → D1 extracts the full rule
  table; per-opcode microsuite + fuzz diff hammer it (M1 gate includes the rule
  table's tests, not just happy paths).
- **FMT sub-language complexity** — a mini-ISA with its own operand forms. →
  D2 census scopes real usage; differential probes per sub-op; TI BASIC (M6)
  is the torture test.
- **FP bit-exactness** (radix-100 normalization, rounding, overflow status) —
  → bit-for-bit differential microtests; BASIC smoke scripts print values that
  expose formatting/rounding drift on screen where it's cheap to diff.
- **ISR re-entrancy and workspace discipline** (authentic ISR runs `LIMI 0`,
  borrows `>83E0` GPLWS mid-ISR ✅ dump) — subtle interactions with the
  interpreter's own state. → dossier the exact save/restore choreography; the
  lockstep fetch-stream gate catches drift immediately.
- **GROM address save/restore around ISR sound fetches** (prefetch off-by-one)
  — historic bug territory in *any* TI-99 work. → pinned probe + a dedicated
  regression test (the emulator's own GROM model is already hardened ✅).
- **Unknown ML entries into ROM interior** — a cart jumping somewhere we made
  interior-only. → P8 makes every *named* routine address-exact by default;
  D2's entry census over the whole corpus + BASIC verifies; M7's tripwire
  keeps the census green forever; any hit becomes a frozen address.
- **The NASTY harvested constants** (code words doubling as data — structural
  fact 3): moving *any* instruction in those regions silently breaks an
  unrelated routine. → D1 enumerates the full list from the disassembly +
  literature; our source expresses each as a named constant pinned at the
  authentic address; the fuzz + conformance gates would catch a miss.
- **Two tracks, one repo** — the GROM ledger-closure chunks are executing
  concurrently. → §12 sequencing; disjoint files; shared-instrument
  coordination in `libre99-core` is the only touchpoint (different modules).
- **Schedule risk: M4/M5 are big.** → chunk packaging (§13) splits them; the
  matrix keeps partial progress shippable (combo 4 can go green before combo 3).

---

## 12. Sequencing with the GROM ledger-closure track

- **Start now, collision-free:** T1 (all in `crates/libre99-asm`), D1 (new files
  in `rom/`), and most of D2 (new instruments in *new* `libre99-core` modules)
  touch nothing the GROM chunks 1–5 touch (`libre99-gpl` splices, `grom/*`,
  `tests/{census,conformance,coverage_sweep}.rs`).
- **Reuse, don't duplicate:** D2's sweep driver and M-gates reuse the GROM
  track's coverage-sweep input mash and conformance checkpoint harness once
  those land (GROM chunk 2). If the ROM track starts first, build the minimal
  driver and converge later.
- **The GROM stays the reference client:** any ROM-side ambiguity about GPL
  semantics gets a probe under the *authentic* ROM first (the GROM track's
  probes are the library to copy from — `../DEBUGGING.md` §Probe inventory).
- **Never edit `../grom/**` or the GROM-track docs from ROM sessions** while
  chunks 1–5 are in flight; cross-references go in `rom/` files and this plan.

---

## 13. Work packaging — hand-off chunks for executing sessions

**Ground rules (every chunk).** Read `../README.md` → `../STATUS.md` →
`rom/RECON.md` (once it exists) → this plan's sections for your chunk;
`../GROM-DEBUGGING-GUIDE.md` + `../DEBUGGING.md` before debugging. Probes
before fixes; a regression test per fix; consult-never-copy; rebuild + commit
`rom/console-rom.bin` when source changes (once M1 lands); update
`rom/RECON.md`/`rom/README.md`/`../STATUS.md` as you go; commit per completed
item to `main`. Fast gate: `cargo test -p libre99-asm -p libre99-gpl`; deep gate
where the chunk says so. **Do not touch `../grom/**` (concurrent track).**

| Chunk | Contents | Exit criterion |
|---|---|---|
| **R-1** | **T1 toolchain**: AORG/raw-ROM mode, layout assertions, COPY, listing+symbols, `build_console_rom()`, `libre99asm rom`, `libre99asm dis`, tracer-bullet ROM | T1 gate green; tracer-bullet boots; disassembler round-trips |
| **R-2** | **D1 static dossier**: disassemble + classify the authentic 8 KiB; `rom/RECON.md` + `rom/SURFACE-MAP.md`; frozen-address table; FMT/PARSE/FP/ISR/KSCAN/XML contracts | every §2 ❓ resolved or explicitly deferred; docs committed |
| **R-3** | **D2 censuses**: entry/XML/opcode/KSCAN/data-read instruments + corpus runs + reports | censuses + `rom/ENTRY-CENSUS.md` committed; re-runnable deep test |
| **R-4** | **M1 kernel + GPL core** | M1 gates: trivial GPL + our-GROM title pixel-identical + fetch-stream diff clean |
| **R-5** | **M2 input + ISR** | M2 gates: menu/QUIT/beep/keyboard/joystick matrix tests green |
| **R-6** ✅ | **M3 XML + device I/O** (COMPLETE 2026-07-05) | M3 gates green: SROM found+call peripheral power-up (`>8370` parity), SGROM 16-base PUSCAN walk parity, `XML >F0` ML-cart launch (sweep, both ROMs), dispatch probes. *Note:* the ToD full-load + Parsec/Invaders launches carry an FMT (M4) dependency — device linkage verified FMT-free; ISR card-chain hardware-gated (RECON §24) |
| **R-7** ⏳ | **M4 interpreter completeness** — IN PROGRESS (2026-07-05): FMT ✅ (`gpl_fmt.rs`, RECON §7 — Parsec/TI-Invaders launch under our ROM); MOVE **C=1** + **indexed-GAS** + **GRAM dest** ✅ (`gpl_core.rs`, RECON §15). Remaining: RAND parity close, IO sub-ops (CRU-in + cassette), COINC/SWGR/RTGR, ext-GPL vestige, the 256-opcode microsuite + the fuzz's MUL/DIV/EX/shift-status items | per-opcode microsuite + 137-sweep (our ROM × our GROM) + fuzz green |
| **R-8** | **M5 floating point** | FP differential microtests bit-exact |
| **R-9** | **M6 acid test + M7 hardening**: authentic-GROM boot, BASIC smoke, matrix conformance/coverage/perf gates, robustness probes | all-matrix deep gate green; perf report committed |
| **R-10** | **M8 production packaging**: committed artifact (all-zero tail — §10.5), `rom/README.md` (build/run/matrix/perf), app-side firmware labeling polish, docs sync, embed decision to Joel | §0 definition-of-done checklist fully checked |

Dependencies: R-1 → R-2 → R-3 is the spine (R-2/R-3 can interleave); R-4…R-8
are strictly ordered; R-9 needs all prior; R-10 last. R-1+R-2 pair well in one
session; so do R-2+R-3. Sizing honestly: R-4 and R-7 are the fat ones (R-7 may
take two prompts); everything else ≈ one prompt each, GROM-track pace.

---

## 14. The `rom/` folder (target layout)

```
original-content/system-roms/rom/
├─ ROM-REWRITE-PLAN.md    this document (archive to ../history/ when executed)
├─ RECON.md               the ROM interface dossier (D1/D2; ✅-tagged facts)
├─ SURFACE-MAP.md         authentic-image classification + frozen-address table
├─ ENTRY-CENSUS.md        D2's empirical entry/XML/opcode census report
├─ README.md              build/run, the 2×2 matrix commands, perf report, address map
├─ console.asm            top-level source (COPYs the component files)
├─ kernel.asm  gpl-core.asm  gpl-move.asm  gpl-fmt.asm  kscan.asm
├─ isr.asm  xml.asm  fp.asm  dsr.asm  tables.asm
└─ console-rom.bin        the committed 8 KiB artifact (build: libre99asm rom)
```

## 15. References

| Topic | Location |
|---|---|
| Machine takes ROM+GROM as parameters (the oracle seam) | `crates/libre99-core/src/machine.rs:83,319` |
| Existing gates that become the matrix | `crates/libre99-gpl/tests/{boot_trivial,title_screen,menu,sweep,interrupts,device_io,char_set,keyboard,ti_python}.rs` |
| Firmware override flags + resume-skip | `crates/libre99-app/src/cli.rs:19-57`, `crates/libre99-app/src/main.rs:34-79,121-122` |
| GROM tracer (fetch-stream oracle) | `crates/libre99-core/src/machine.rs` `grom_record`/`grom_log`; `../RECON.md` "GPL execution model" |
| GPL ISA already extracted | `crates/libre99-gpl/src/isa.rs`; `../RECON.md` §8 |
| Scratchpad map + ISR/SCAN/XML contracts (GROM side) | `../RECON.md` (R1, scratchpad map, §§1–11) |
| Assembler today (base/auto-header seam; AORG rejection) | `crates/libre99-asm/src/lib.rs:49-64,362-370`; spec `assembler/ASSEMBLER.md`; E/A manual PDF `assembler/` |
| 9901/CRU model (timer minimal; cassette unwired) | `crates/libre99-core/src/cru.rs` |
| CPU core (full ISA incl. XOP/IDLE; conformance-tested) | `crates/libre99-core/src/cpu.rs` |
| Authentic ROM dumps made for this plan (vectors, `>0CFA/>0D1A` tables, ISR/KSCAN heads, tail) | §1 ✅ rows; reproduce with `xxd` offsets given there |
| Classic99 (consult, never copy) — surveyed for this plan: | `C:\ClaudeShared\classic99` |
| · GROM port semantics (prefetch, destructive addr read, selector resets, data write to GRMADD−1, 8K-bank wrap) | `console/Tiemul.cpp:5766-6016` (`ReadValidGrom`/`WriteValidGrom`) |
| · VDP status/data/address semantics ("tested on hardware" notes), 4K/16K translation | `console/Tiemul.cpp:5080-5494`; `console/tivdp.cpp:958-965` |
| · 9901 timer/clock-mode/INTREQ + interrupt gate; CRU keyboard matrix + column bits + alpha lock + joystick columns; cassette CRU bits 22/24/25/27 | `console/Tiemul.cpp:6240-6273,3599-3645,285-314,6843-6878,6447-6452`; `console/tape.cpp` |
| · CPU quirks: post-increment ordering (canonical 6-step order), BLWP/RTWP/XOP register choreography, one-instruction interrupt grace, illegal-op behavior, X | `console/cpu9900.cpp:151-172,398-445,1044-1088,1498-1584,2628-2642`; conformance oracle `tests/TI9900_CPU_Test.txt` |
| · GPL opcode table + per-op docs (SCAN/RAND/FMT/PARSE/XML cells); KSCAN `>000E` + interpreter `>0070` + `XML >F0` usage; GPLWS register roles + DSR invariants | `addons/gpl.cpp:43-616` (file is `#if 0` — reference only); `addons/makecart.cpp:1156-1168,1544-1605`; `disk/disk.cpp:327-367` |
| · Classic99 never byte-patches the console CPU ROM (pure oracle; its patches are GROM-side App-Mode branding or PC-intercepts) | `console/Tiemul.cpp:2522-2695,3385-3497,3999-4122` |
| Literature 📖 (consult, never copy — key pages verified against our binary during planning) | **TI Intern** (Heiner Martin, 1985) — the full commented ROM disassembly: https://www.99er.net/files/tiintern.pdf , https://archive.org/details/tibook_ti994a-intern · **Nouspikel Tech Pages**: roms.htm (ROM map), padram.htm (scratchpad), gpl.htm/gpl2.htm (GPL + FMT/IO/XML), ints.htm (ISR), keyboard.htm + tutor1.htm (KSCAN), cassette.htm, reals.htm (FP), headers.htm, gplcall.htm — under https://www.unige.ch/medecine/nouspikel/ti99/ · **E/A manual** (`assembler/Editor_Assembler_Manual.pdf`) |
| Prior art (context; none is usable source) | **Lee Stewart's "The TI-99/4A Operating System"** (AtariAge) — TI's recovered original ROM source, rebuilds byte-identical; origin of the NASTY list; **TI's copyright — consult findings only**: https://forums.atariage.com/topic/250272-the-ti-994a-operating-system/ · Classic99 ships TI's ROMs under a TI license (precedent: TI still owns them) · no open-source clean-room console-ROM replacement exists (searched 2026-07) — this rewrite would be the first · useful tools: xdt99 (xas99/xga99), jedimatt42/9900dis |
| House method docs | `../GROM-DEBUGGING-GUIDE.md`, `../DEBUGGING.md`, `../QUALITY-ASSESSMENT.md` (chunk style §7.7) |