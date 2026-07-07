> **ARCHIVED (2026-07-02).** This review's playbook was executed to completion.
> Its recon results (§2) live on in [`../RECON.md`](../RECON.md); its traps and
> testing mechanics (F14) in [`../DEBUGGING.md`](../DEBUGGING.md). Kept for
> provenance; do not work from it — several statements were later corrected
> (see [`README.md`](./README.md) in this folder, notably the `>8300` dispatch
> cell, the `OP dst,src` operand order, and the §8 "OURS" scratchpad rows).

# GROM Rewrite — senior engineering review & implementation playbook

A review of **[`GROM-REWRITE-PLAN.md`](./GROM-REWRITE-PLAN.md)** (which this
document deliberately does **not** modify), written from the perspective of an
engineer who has spent serious time inside the TI-99/4A's firmware. It has two
jobs:

1. **Review** — what the plan gets right, what it gets wrong, what it's missing,
   and the concrete deltas to apply.
2. **Playbook** — a step-by-step implementation guide precise enough for an AI
   model (or any engineer new to GPL) to execute the whole plan in one sitting,
   with the traps identified *before* they're stepped in.

### How this review was produced, and how to read the evidence tags

This is not an armchair review. The claims below were tested **live against this
repository's own emulator** (2026-07-01, on the Windows machine, fresh
`rustup` install, `cargo test -p libre99-core` green first) using a recon probe now
committed at **`crates/libre99-core/examples/recon_probe.rs`** — run it yourself
with `cargo run -p libre99-core --example recon_probe` from the repo root. Every
factual claim carries one of three tags:

- ✅ **VERIFIED** — reproduced in this repo, on this emulator, with the real
  ROM/GROM images, during this review. Treat as ground truth.
- 📖 **DOCUMENTED** — established TI-99 community/TI literature (Nouspikel,
  the E/A manual, the GPL Programmer's Guide). Almost certainly right, but
  verify at the step indicated before depending on it.
- ❓ **HYPOTHESIS** — this reviewer's inference. The playbook step that
  confirms or refutes it is always named. Never hardcode a ❓ fact.

---

## 1. Verdict

**The plan's architecture is correct and the project is very achievable.**
Keeping the console ROM and rewriting only the GROM is exactly right — this
review *confirmed empirically* that the entire boot/menu/dispatch machinery the
rewrite must reproduce is GPL in GROM 0, entered at one fixed address, and that
every facility our GPL needs (including launching machine-code cartridges) is
reachable from GPL under the unmodified ROM.

But the plan as written would fail its own Milestone 3 gate, and would stall at
Milestone 0 on this machine. Four **blocking** findings:

| # | Blocker | One-line fix |
|---|---------|--------------|
| **F1** | The menu contract in plan §2.3 omits **CPU `>6000` ROM-header cartridges — 33 of the 137 bundled carts have no GROM at all** and would vanish from the menu. | Scan CPU `>6000` for `>AA` too; dispatch ML entries via `>8380` + `XML >F0` (mechanism ✅ verified below). |
| **F2** | **No font.** The console GROM owns every glyph on screen; a rewritten GROM with no character patterns draws nothing, including cartridge names. | Add an original 8×8 font (~96 glyphs) as an explicit M1 deliverable. |
| **F3** | The **ISR/scratchpad contract** (plan §2.5) is a two-line sketch, but the console ROM's interrupt service routine runs every VBLANK and consumes ~a dozen scratchpad cells our GPL must initialize. | §8's cell map is the contract; init code specified in Phase 6. |
| **F4** | **M0's validation gate needs R1's answer** (the fixed entry address). Classic99's `gpl.cpp` is the ISA reference and is now checked out on both workstations (PC + Mac). | R1 is *already done* (§2.1: the entry is GROM `>0020` ✅); §7's reference card + the fetch-differential oracle cross-check Classic99 (consult, never copy). |

With the deltas in §3 applied and the playbook in §6 followed, every milestone
gate in the plan is reachable. Much of the reconnaissance the plan schedules
(R1, and the core of R2) was performed *during this review* and its results are
recorded in §2 — the implementer starts several squares ahead.

---

## 2. New facts established for this review

These are the recon results the plan asked for (its R1, and most of R2),
plus a census the plan never thought to take. All ✅ unless tagged.

### 2.1 The fixed GPL boot entry — R1 is answered: it is GROM `>0020`

Probe: enable `grom_record(true)` immediately after `Machine::new`, then `step()`
until the log is non-empty.

- The first GROM activity comes after only **6 CPU instructions** (PC `>005E`).
- Log entry 0 is a throwaway data read (`>1FFF=00`) — the ROM strobes the data
  port once before ever setting an address, so **"first read = entry" (plan §6
  R1) is off by one**. The entry is the first read *after an address write*.
- Reads 1–2: `>0020=40`, `>0021=52` — the ROM sets the GROM address to
  **`>0020`** and fetches. Read 3 is `>0052=87`: the interpreter *executed*
  `40 52` as **`BR >0052`** and branched. So:
  - the fixed entry is **`>0020`** (matching 📖 lore), and
  - TI's own first instruction is a `BR` used as an unconditional branch
    (condition bit is clear at power-up) — see finding F6 on `BR` semantics.
- **Machine state at first GPL fetch** (the entry contract our GPL inherits):
  - Every VDP register is `>00` — display off, interrupts off. **GPL does all
    VDP setup.**
  - Scratchpad is all zero **except**: `>83E0/1 = >0020` (GPL workspace **R0 =
    the entry address**), `>83FA/B = >9800` (R13 = GROM read port), `>83FC/D =
    >0100` (R14 = interpreter flags), `>83FE/F = >8C02` (R15 = VDP write port).
  - ⚠ The zeros are the **emulator's** RAM init, *not* a hardware guarantee —
    real scratchpad powers up random. Firmware must explicitly initialize every
    cell it (or the ISR) reads; do not lean on the zeros.

### 2.2 The title screen, as the real GROM leaves it

After 180 frames: VDP registers `R0=>00 R1=>E0 R2=>F0 R3=>0E R4=>F9 R5=>86
R6=>F8 R7=>F7`. The 9918A masks unused high bits, so the **effective layout**
is: name table `>0000`, pattern table `>0800`, color table `>0380`, sprite
attributes `>0300`, sprite patterns `>0000`, backdrop cyan (`R7` low nibble
`7`). `R1=>E0` = 16K + display on + interrupt enable. During boot the GPL
`MOVE`s an 8-byte register-init table straight out of GROM `>0451–>0458`
(`00 20 F0 0E F9 86 F8 F7` — note R1 starts as `>20`, display *off*, and is
raised to `>E0` only after drawing).

Name-table facts that shape our M1/M2 code and tests:

- Character codes are **plain ASCII** (no `>60` offset): row 9 reads
  `TEXAS INSTRUMENTS`, row 11 `HOME COMPUTER`, row 16
  `READY-PRESS ANY KEY TO BEGIN`, row 22 `©1981  TEXAS INSTRUMENTS` (the `©` is
  custom glyph `>0A`).
- The color bars (rows 0–2 and 18–20) and the TI logo (rows 5–7) use custom
  glyphs: codes `>60–>DF` for the bars, `>01–>09` for the logo.
- Scratchpad at title time (full snapshot in §8): `>8370/1=>3FFF` (top of free
  VDP RAM), `>8372=>FE` / `>8373=>7E` (GPL data / subroutine stack pointers),
  `>8374=>00` (keyboard mode 0), `>83CC/D=>0484` (**a sound list pointer into
  GROM 0** — the key beep), `>83D4=>E0` (the ISR's VDP-R1 image), `>83D6/7`
  ticking (screen-blank timeout), GPL workspace R13/R14/R15 as at entry
  (R14 now `>0108`).

### 2.3 The menu, and how a machine-code cartridge is launched (core of R2)

Probe: mount `centipe.ctg` (**ROM-only**, single bank), boot, press Space at the
title, dump the name table; then hold `2` and step frames until the CPU lands in
`>6000–>7FFF`.

- The menu lists **both** GPL and ML programs, numbered from 1:
  row 5 `PRESS`, row 7 `1 FOR TI BASIC`, row 9 `2 FOR CENTIPEDE`. This
  **proves the real menu scans CPU `>6000` ROM headers** (Centipede has no
  GROM), which plan §2.3 omits — finding F1.
- Menu scan state visible in scratchpad while the menu is up: `>8302/3=>6010`
  (the ROM cart's program-list pointer, read from CPU `>6006`), `>8306/7=>214D`
  (console GROM 1's program list). The scan walks GROM bases *and* the CPU
  `>6000` header, collecting program-list entries.
- On keypress the menu starts a **beep**: `>83CC/D` points at a sound list *in
  GROM* (`>0484`), and the VBLANK ISR fetches the sound bytes out of GROM
  itself (interleaved reads at `>0481+` appear mid-trace — the ISR saves and
  restores the GROM address counter around them, which is exactly the
  `>9802`-flip-flop behavior `docs/STATUS.md` documents as the historic boot
  bug). The menu then **polls until the beep finishes**: a tight GPL loop at
  `>033C` (`8F 80 CE / 43 3C` = test `@>83CE` — sound-bytes-remaining — and
  `BR >033C`). Two consequences:
  - any test that injects a key **must keep pumping `run_frame()`** — the first
    version of this probe stepped the CPU without VBLANKs and hung forever in
    that loop, precisely reproducing what a missing ISR contract does (F3/F14);
  - if our menu never *starts* a sound list, it must also never wait on one —
    the simple v0 posture is a silent menu with `>83CE` held at zero (F16).
- **The ML dispatch mechanism** (⚠ CORRECTED 2026-07-02: the vector cell is
  **`>8300`**, not `>8380` — `>8380` is the GPL sub-stack slot the menu copies
  *from*; see `RECON.md` "VERIFIED MECHANISMS" §2, execution-verified): the
  trace ends `DST @>8300,@>8380 ; XML >F0` — the menu GPL writes the program's
  entry address (`>6056`, from the ROM header's program list) into CPU `>8300`
  and executes **`XML >F0`** ("vector in >8300", per Nouspikel). One instruction later the CPU is at the
  cartridge's entry (observed at PC `>7E04` a frame later, game running). This
  is the GPL→ML trampoline our menu will use verbatim: `DST @>8380, #entry ;
  XML >F0`. (Which XML slot the GROM-cart *GPL* dispatch uses was not traced —
  Phase 3 of the playbook closes that with a two-line probe change. ❓ until
  then; likely a ROM helper that reloads the GROM address counter.)

### 2.4 Cartridge census (all 137 bundled `.ctg` images, parsed offline)

| Class | Count | Consequence for the rewrite |
|---|---|---|
| GROM-bearing (GPL menu entries) | 104 (48 GROM-only + 56 GROM+ROM) | listed via GROM `>6000+` headers |
| **ROM-only (ML menu entries)** | **33** | **invisible unless CPU `>6000` is scanned** — includes DigDug, Moon Patrol, Ms. Pac-Man, Donkey Kong, Centipede, Defender, Pole Position, Frogger†, Shamus… |
| Bank-switched ROM (2 banks) | 21 | header must be read from the **power-on bank**; the scan must not write into `>6000–>7FFF` (a write flips banks — `machine.rs:250-254`) |
| Multi-program menus | 10 (et=7, mine=7, Soccer=5, HuntTheWumpus=3, VideoGames1/2=3…) | the program-list **walk must follow `next` pointers to the end**, not take the first entry |
| Declare a power-up list (header `>04`) | **0** | the boot-time power-up walk is *not* needed for the bundled gate (keep it as a documented gap or a cheap add) |
| No `>AA` header at GROM or ROM `>6000` | 11 (connect, fantasy, frogger†, germptl, hangman×2, popeye, qbert, rtpirat, sxba, zerozap) | triage in M3: first check whether they even list on the **authentic** GROM in this emulator; if not, they're outside the compatibility gate. († frogger appears in both lists because two dumps differ.) |

### 2.5 The three console GROMs are interconnected — "GROM 2 is free" is wrong as stated

Reading `roms/994AGROM.Bin` directly:

- GROM 0 `>0010–>0037` is a **vector table of twenty GROM addresses, every one
  of them in `>4000–>5FFF`** (GROM 2): `>43DC >443C >49A9 >4396 >439E >4446
  >4449 >444C >4052 >51FE >4C82 >4D59 >4DB4 >4E64 >4EF9 >4F01 >4F5F >4F80
  >43CE >43D6`.
- `>0038` is `B >4D12` — a long branch into GROM 2 — followed by more short
  branch stubs (`>003B: BR >125E`, `>003D: BR >0417`, `>003F: B >2844`). These
  low fixed addresses are the 📖 **GPLLNK service entries** the E/A ecosystem
  calls (e.g. `>0016`/`>003D` = character-set loaders, `>0018` = power-up).
- GROM 1's header points its **subprogram list at `>4D1A`** — inside GROM 2.

So GROM 2 is not merely "BASIC continuation": it hosts the console's shared GPL
library, reached through the fixed low-address vectors in GROM 0. The *slots*
(`>0010–>004F`) are the fixed interface; the *targets* are ours to place.
Plan §1's claim should be read as "GROM 2's *addresses* are free once R3 shows
no cartridge branches into them directly" — that's exactly what R3 must now
measure (playbook Phase 3).

### 2.6 Environment facts an implementer must know

- **Classic99 is available on both workstations** — `C:\ClaudeShared\classic99`
  on the PC (sibling of this repo) and `/Users/Shared/classic99` on the Mac. Its
  `gpl.cpp`/`Tiemul.cpp` are the reference for the plan's citations (consult,
  never copy); §7's fetch-differential oracle is the execution-anchored
  cross-check that made the ISA work even before the PC checkout existed.
- Rust was **not** installed for this Windows sandbox user; it is now (rustup,
  stable 1.96.1, MSVC — VS Build Tools are present system-wide). Baseline
  `cargo test -p libre99-core` passes.
- Git needed `safe.directory` for this repo under the sandbox user (already
  added to the sandbox's global git config).
- Useful emulator surface for all recon/tests (all public today, no core
  changes needed): `Machine::new/run_frame/step/render/set_key/mount_cartridge/
  reset/cpu()/vdp()/bus()/bus_mut()`; `Tms9900Bus::peek/peek_word/poke/
  grom_record/grom_log/grom_address` (`machine.rs:476-518`); `Vdp::register/
  vram` (`vdp.rs:137-148`); `cartridge::write_v1(title, cru, rom,
  &[(u16, Vec<u8>)])` (`cartridge.rs:156`).

---

## 3. Findings — deltas to apply to the plan

Ordered by severity. Each names the plan section it amends.

### F1 — BLOCKER · plan §2.3/§8-M3: the menu must scan **CPU `>6000`** and dispatch ML programs

As written, §2.3 scans GROM bases and "DSR ROM bases" only. **33/137 bundled
carts are ROM-only** (§2.4) and would never list; M3's gate ("all bundled
cartridges must still appear and launch") is unmeetable. Amend the contract:

1. After walking GROM bases, probe **CPU `>6000`** for `>AA` (via GPL extended
   CPU addressing) and walk its program list identically — pointers in ROM
   headers are CPU addresses in `>6000–>7FFF`.
2. Dispatch for ROM entries is the verified trampoline (§2.3):
   `DST @>8380, #entry` then `XML >F0`. No return — the game owns the machine.
3. The scan must be **read-only** in `>6000–>7FFF` (bank-switch hazard, §2.4).
4. M2's gate gains a second case: a ROM-only cart (use `centipe.ctg`) must
   list and launch, alongside the plan's GROM example.

### F2 — BLOCKER · plan §3/§8-M1: the fonts are ours to supply

The plan replaces title text and logo but never mentions that **every glyph on
the TI screen comes out of console GROM** — TI's patterns are TI's expression
and cannot ship. Deliverable added to M1: an **original 8×8 font** covering at
least `>20–>5F` (96 glyphs: space, punctuation, digits, A–Z — cartridge names
like `DIG-DUG` and `HUNT THE WUMPUS!` need punctuation), stored as `BYTE` rows
in GROM 0 and `MOVE`d to VDP `>0800 + 8·code` during init. Map lowercase
(`>60–>7A`) to the uppercase glyphs in v0. Design guidance in Phase 6. The
GPLLNK charset-loader vectors (`>0016`, `>003D` 📖) should eventually serve
this font — Phase 10.

### F3 — MAJOR · plan §2.5: the ISR contract, made concrete

The console ROM's VBLANK ISR runs whenever GPL has interrupts enabled (which is
"always" during SCAN/menu loops) and reads/writes a fixed set of cells: it
processes **sound lists** (`>83CC/D` pointer, `>83CE` bytes-remaining, source
GROM-vs-VDP flag in R14's image at `>83FC/D` ❓bit), moves **auto-sprites**
(count `>837A`), maintains the **VDP-status copy** (`>837B`), the **VDP R1
image** (`>83D4`) and **screen-blank timeout** (`>83D6/7`), calls a **user ISR
hook** if `>83C4` ≠ 0, honours **disable flags** at `>83C2` 📖, and implements
**QUIT (FCTN-=) as `BLWP @>0000`** — a full reboot into our title. ✅ evidence:
the beep fetch interleave and the `>033C` sound-wait (§2.3), `>83D4=>E0`
tracking R1, `>83D6/7` ticking, and QUIT-reboot is 📖 (verify in Phase 8's
sweep — one keystroke).

Consequences (all folded into Phase 6's init block):

- Our power-up GPL **must zero**: `>837A`, `>83C2`, `>83C4/5`, `>83CC–>83CF`,
  and set `>83D4` to the R1 value we program, *before* enabling the display —
  because real scratchpad powers up random (§2.1 ⚠).
- **QUIT comes for free** — plan §8-M3's "QUIT returns to the title" needs no
  code at all, only a test.
- v0 menus are **silent** (F16): never start a sound list, never wait on one.

### F4 — MAJOR · plan §5.6/§6/§8: ordering — and R1 is already done

M0's own gate ("assemble a trivial GROM … at the fixed entry") needs R1's
answer. The plan lists R1 *after* M0. Resolution: R1's core question is
answered in §2.1 (**entry `>0020`**), and the probe is committed. The playbook
reorders the remaining work: recon-anchored decoder first (it needs no
assembler), then the encoder. Also amend R1's method note: the first log entry
is a dummy read (§2.1), and the ISR's sound fetches interleave with
instruction fetches (§2.3) — trace analysis must tolerate both.

### F5 — RESOLVED · plan §5.1/§12: Classic99 dependency backed by three sources

M0 cites `classic99/addons/gpl.cpp` as the opcode source of truth. Classic99 is
now checked out on **both** workstations — `C:\ClaudeShared\classic99` (PC) and
`/Users/Shared/classic99` (Mac) — so the citation resolves directly (consult,
never copy). The full 256-entry table has since been extracted into
`crates/libre99-gpl/src/isa.rs`. Even independent of Classic99, the playbook rests
on three cross-checking sources, any one of which suffices:

1. **§7's reference card** (this document) — the ISA as this reviewer knows it,
   every row tagged with confidence;
2. the **fetch-differential oracle** (§7.4) — the real interpreter running the
   real GROM *is* the encoding ground truth, available on every machine this
   repo builds on; it converts any ISA-table mistake into a loud, localized
   test failure;
3. online 📖 references (§7.5) when connectivity allows, and Classic99 on either
   workstation (`C:\ClaudeShared\classic99` on the PC, `/Users/Shared/classic99`
   on the Mac) — consult, never copy.

### F6 — CORRECTION · plan §5.1: `BR`/`BS` are *slot-absolute 13-bit*, not "relative"

✅ Proven at the very first instruction: `40 52` at `>0020` branched to
`>0052` — the low 5 opcode bits are the **top bits of a 13-bit address within
the current 8 KiB slot** (`addr = slot_base | ((op & >1F) << 8) | operand`).
Encoder rules: reject a `BR`/`BS` whose target is outside the current slot;
remember `BR` branches when the condition bit is **clear** (TI itself uses `BR`
as the boot "unconditional" — our source should use explicit `B` for
unconditional jumps and keep `BR`/`BS` for real conditionals). `B`/`CALL` take
16-bit absolute GROM addresses (✅ `>0038: 05 4D 12` = `B >4D12`).

### F7 — CORRECTION · plan §5.1: the operand ("general address") grammar is under-specified

The plan's five-form table compresses the second byte away. The actual grammar
(📖, partially ✅ — `80 CE` decoding to `@>83CE` is confirmed by the sound-wait
loop; the 16-bit form appears at boot as `8F 11 00` ≈ `@>9400`, the speech-port
write ❓):

```
first byte 0aaaaaaa                 → direct scratchpad >8300+a          (1 byte)
first byte 1 X V I nnnn, second aa  → 12-bit address >8300+(nnnn:aa)     (2 bytes)
        …if nnnn == 1111: two more bytes follow = 16-bit address,
         still biased +>8300 (wraps mod 64K: CPU >6000 encodes as >DD00) (3 bytes)
X=1 → indexed: ONE index byte (a scratchpad cell) appended LAST
V=1 → VDP RAM instead of CPU;  I=1 → indirect through the addressed word
```

Every combination must get a golden test (Phase 4) and is cross-checked by the
differential oracle. The `+>8300` bias on the 16-bit form is the single easiest
thing to get wrong — verify it first (❓→✅ via one decoded `MOVE` from the
boot trace, whose source address `>0451` is known from §2.2).

### F8 — SIMPLIFICATION · plan §5.1/§5.3: defer `FMT`; hand-lay the headers

- **`FMT` (>08)**: TI's title uses it (✅ — the `READY…` text sits in a
  count-prefixed FMT run ending in `>FB` FEND, and the count byte `>1B` for a
  28-char string shows FMT encodes **count−1**). But *our* screens don't have
  to: `MOVE` to the name table does everything we need. Decision: the
  **disassembler must decode FMT** (recon reads TI's code); the **assembler
  defers FMT entirely** (error "FMT not supported in v0"). Cuts M0 meaningfully.
- **Assisted `HEADER`/`PROGRAM` directives** (plan §5.3): unnecessary — we
  author exactly one image with two headers. Hand-lay them with
  `BYTE`/`DATA`/labels (16 bytes each; sketch in Phase 6). Revisit only if GPL
  cartridge authoring (plan §5.4) becomes real.

### F9 — DECISION · plan §5.2: new crate `libre99-gpl`, **no** shared-crate extraction

Concur with the new-sibling-crate recommendation, but reject the
`libre99-asm-syntax` extraction: `libre99-asm/src/lib.rs:14` declares `mod expr;`
privately — make it `pub mod expr;` (one line), keep `pub mod lex;` as is, and
have `libre99-gpl` depend on `libre99-asm` directly (both are pure-`std`; the
dependency is free and reversible). Smallest possible blast radius for a
single-prompt implementation.

### F10 — CORRECTION · plan §1/§2.4: GROM 2 and the fixed low vectors (see §2.5)

R3's scope is sharpened: record, across the cartridge sweep, (a) any GROM fetch
in `>0000–>005F` *not* caused by our own code — a cartridge invoking a fixed
vector — and (b) any fetch in `>4000–>5FFF` — a cartridge branching into TI's
library directly. Category (a) we support by pointing the same vector slots at
our own routines; category (b) is per-address and, if it exists at all in the
bundled set, is documented as a known gap per title. Expectation ❓: (b) is
empty for games (TI's own GPL games are self-contained; third-party ML games
never execute GPL).

### F11 — DOWNGRADE · plan §2.1/§11: the "menu program in GROM 0 vs GROM 1" worry is moot

The ROM hardcodes exactly one GROM address: `>0020` (✅ §2.1). Everything
else — scan order, numbering, which GROM hosts the program list — is *our* GPL
reading structures *we* define. Keep TI PYTHON's program list in GROM 1 purely
so the menu numbering matches tradition (entry 1), not because any ROM revision
demands it.

### F12 — RISK REFINEMENT · plan §11: quantified compatibility exposure

- Power-up lists: **zero** bundled carts use them (§2.4) — risk retired.
- Multi-program lists: ten carts, up to 7 entries — the walk must be a real
  linked-list traversal; M2's test should include `VideoGames1.ctg` (3 entries).
- The 11 header-less images (§2.4) get a triage step (Phase 8), not silent
  failure. `sxba.ctg` (Super Extended BASIC ❓) likely *extends console BASIC*
  and is a **principled incompatibility** for a BASIC-less GROM — document it.
- Speech: several carts speak (Parsec, Alpiner). The console GROM's role in
  speech is nil (speech is a DSR + cart-side GPL talking to `>9000/>9400`), and
  this emulator stubs speech anyway (`machine.rs:156,216`) — no GROM work.

### F13 — SPECIFICATION · plan §9: TI PYTHON v0 decisions pinned

So the implementation and its tests are deterministic:

- **Division/modulo truncate toward zero** (C semantics, *not* CPython floor):
  `10/3=3`, `-10/3=-3`, `10%3=1`, `-10%3=-1`, `10%-3=1`. Rationale: GPL
  `DIV` is unsigned 📖; sign-fixup yields truncation naturally. Document the
  deviation from Python in the REPL's README; revisit in v1.
- 16-bit two's-complement wraparound documented (`32767+1 → -32768`).
- `-32768` needs a special print path (negation overflows).
- **The screen row is the line buffer** (TI BASIC's own trick): echo keys into
  the name table; on ENTER, tokenize by *reading the row back from VDP RAM*.
  Saves a scratchpad buffer entirely.
- Variables: 16 slots × 4 bytes (two significant name chars + 16-bit value) at
  `>8300–>833F`; parser temps `>8340–>8349`; **nothing above `>8349`** (the
  ROM's floating-point workspace begins at `>834A` 📖 and the system cells at
  `>8370` — see §8).
- Expression nesting capped (depth counter, error `SYNTAX ERROR` beyond ~8) —
  the GPL subroutine stack lives in scratchpad (`>8373` pointed at `>7E` ✅)
  and must not be driven into the interpreter's workspace at `>83E0`.
- Keyboard: mode 5 📖 ("BASIC", full ASCII incl. shifted symbols) — verify the
  mode byte early in Phase 9 by echoing `SHIFT-8` etc.; fall back to mode 0 +
  manual shift tables only if mode 5 disappoints. `FCTN-S` (`>08`) = backspace;
  `ENTER` (`>0D`) = evaluate; QUIT handled by the ISR as everywhere else.

### F14 — TESTING MECHANICS · plan §10: the assertions, concretely

- Screen text: `let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;` then
  compare `m.vdp().vram(base + 32*row + col)` bytes against ASCII (our charset
  is identity-mapped). ⚠ mask register 2 — the real firmware leaves it `>F0`
  and the raw value overflows the multiply (this bit the probe during review).
- Keys: `set_key(key, true)`, **≥ 2 `run_frame()`s held**, release, more
  frames. Always drive time with `run_frame()`, never bare `step()` loops — the
  sound-wait hang (§2.3) is the canonical failure.
- "Cartridge launched": ROM carts → `(0x6000..0x8000).contains(&m.cpu().pc())`
  within ~120 frames of the digit. GROM carts → `grom_record` and assert
  fetches at/above `>6000` (their GPL executing). Both helpers in Phase 8.
- The full-137 sweep runs `#[ignore]`d (minutes of wall clock); the default
  gate is a 9-cart representative sample (list in Phase 8).

### F15 — MINOR corrections and confirmations

- Plan §12's "tests/titris.rs" lives at `crates/libre99-asm/tests/titris.rs`.
- The system-GROM artifact: emit the full **24 KiB** (three slots, zero-filled)
  so it drops in wherever `994AGROM.Bin` goes; keep code within the first
  6 KiB of each slot when convenient (real TMC0430s are 6 KiB — `grom.rs:43-48`
  models 8 KiB slots, so this is hardware-fidelity polish, not correctness).
- `--system-grom` should bypass the save-state resume path exactly as the
  existing media flags do (plan already says this; confirm in `app.rs` when
  wiring — follow `--cartridge-file`'s pattern end to end, `cli.rs:15-17`).
- Keep the printed name-table/text conventions ASCII-identity so F14's
  assertions stay trivial.

### F16 — SCOPE · sound: ship v0 silent

TI's menu beeps via a GROM-hosted sound list serviced by the ISR (§2.3).
Replicating that is real work (sound-list format 📖, the GROM-source flag bit
❓) for a chirp. v0: initialize `>83CC–>83CF` to zero, never start a list,
never poll one. File the beep as a v1 nicety with the sound-list format to be
lifted from the R2 trace (the list TI plays at `>0484` is 3 bytes + terminator).

---

## 4. What the plan gets right (and this review verified)

- **Keep-the-ROM strategy** — vindicated end to end: the unmodified ROM
  interpreted, from the real GROM, everything our rewrite needs to emit,
  including the ML trampoline (`XML >F0`) our menu will reuse (§2.3).
- **The GROM tracer is the right recon instrument** — `grom_log`'s
  address-of-byte bookkeeping (`grom.rs:126-137`) held up perfectly against
  prefetch and the ISR's mid-instruction GROM excursions.
- **In-memory assemble→boot→assert tests** (plan §10) — the pattern already
  proven by `crates/libre99-asm/tests/titris.rs` transfers unchanged.
- **`write_v1` already takes GROM pages** — plan §5.4's cartridge-GROM bonus is
  real (`cartridge.rs:156-193`), just deferred.
- **The IP stance** (§4) is the right line: interface facts (headers, entry
  address, port protocol, scratchpad cells) reproduced; TI's *expression*
  (text, glyphs, BASIC, their GPL code) replaced wholesale. This review's
  disassembly evidence was used exactly that way — to learn *where the
  interfaces are*, never as source to transcribe.

---

## 5. Revised milestone order

| Step | What | Gate | Plan § |
|---|---|---|---|
| **P0** | Preflight: env, baseline tests | suite green | — |
| **P1** (≈R1) | Recon results adopted (§2); probe re-run | `RECON.md` committed | §6-R1 |
| **P2** (M0a) | `libre99-gpl` crate: ISA table + **decoder** | fetch-differential oracle passes over the boot trace | §5 |
| **P3** (R1b/R2/R3) | Disassembler; GROM-cart dispatch trace; vector/subprogram sweep | recon appendix in `RECON.md` | §6 |
| **P4** (M0b) | **Encoder**, directives, image builder | goldens + round-trip | §5 |
| **P5** (M0 gate) | Trivial GROM boots on real ROM | `BACK` reaches VDP R7 | §5.6 |
| **P6** (M1) | Header + init + **font** + title | `boots_to_rewrite_title_screen` | §8-M1 |
| **P7** (M2) | Menu: scan GROM **and CPU `>6000`**, list, dispatch both kinds | GROM cart + ROM cart list & launch | §8-M2 |
| **P8** (M3) | Compatibility sweep + QUIT | 9-cart sample green; 137 `#[ignore]` sweep; headerless triage | §8-M3 |
| **P9** (M4) | TI PYTHON v0 | plan §9 session + F13 cases | §8-M4 |
| **P10** (M5/M6) | Vector stubs per R3; CLI flags; embed+toggle; docs; artifact | full suite green, docs synced | §8-M5/6 |

Stop-points: every phase leaves the repo shippable. If effort must be cut,
P0–P5 is the toolchain spine (commit it), P6–P7 is the visible product,
P8 the proof, P9 the showpiece, P10 the polish.

---

## 6. Implementation playbook

Written to be executed top-to-bottom in a single session by an AI implementer.
Rules of engagement:

- **Gates are mandatory.** A phase's tests must pass before the next phase
  starts. Never weaken or delete a gate to proceed.
- **TDD, clippy-clean, zero third-party deps** in `libre99-gpl` (workspace rule,
  `Cargo.toml:2-16`). Repo conventions: commit straight to `main`
  (`CLAUDE.md`), keep README/help in sync when flags change.
- **IP guardrails**: TI's GROM may be *executed* (tests boot `roms/994AGROM.Bin`)
  and *decoded for interface facts* (addresses, cell usage, header layouts).
  Never copy decoded TI GPL into our source; never embed TI ROM/GROM bytes as
  literals in source or fixtures (tests `include_bytes!` the `roms/` files at
  build time, as `tests/boot.rs:17-18` already does). Our GPL is written fresh
  from the behavioral spec in this document.
- When a ❓ fact fails verification, stop, re-derive from a trace, update
  `RECON.md`, then continue.

### Phase 0 — Preflight

1. `cargo test -p libre99-core` and `cargo test -p libre99-asm` — both green before
   any change (verified during this review).
2. Confirm `roms/994aROM.Bin` (8 KiB) and `roms/994AGROM.Bin` (24 KiB) exist.
3. Read, in order: `GROM-REWRITE-PLAN.md`, this file, `crates/libre99-asm/src/lib.rs`
   (the driver you'll mirror), `crates/libre99-core/examples/recon_probe.rs`.

### Phase 1 — Adopt the recon

1. Create `original-content/system-roms/RECON.md` recording §2.1–§2.5 of this
   file (copy the tables; they are the project's interface dossier), plus the
   raw first-400-reads listing from a fresh probe run.
2. Re-run `cargo run -p libre99-core --example recon_probe` and diff against §2 —
   if anything moved, the emulator changed since this review; investigate
   before proceeding.

### Phase 2 — `libre99-gpl` crate + decoder + the oracle

1. Scaffold `crates/libre99-gpl/` (add to workspace `members`): `Cargo.toml` with
   `[dependencies] libre99-core = { path = "../libre99-core" }` and
   `libre99-asm = { path = "../libre99-asm" }`; change `libre99-asm/src/lib.rs:14` to
   `pub mod expr;`.
2. `src/isa.rs`: the opcode table from §7.3 — a `struct GplInsn { name, opcode,
   operands: OperandPattern }` array. Encode *shapes*, not behavior.
3. `src/decode.rs`: `pub fn decode(image: &[u8], addr: u16) -> Result<Decoded, DecodeError>`
   returning mnemonic, operand values, **length**, and successor kind
   (fallthrough / branch{target} / stop). Implement the §F7 operand grammar and
   §F6 branch addressing. FMT: decode sub-ops enough to find FEND (§F8).
4. **The fetch-differential oracle** (`tests/oracle.rs`) — this is the phase
   gate and the project's keel:
   - Boot `Machine::new(CONSOLE_ROM, CONSOLE_GROM)` with `grom_record(true)`;
     run ~10 frames; take the log.
   - Split the `(addr, byte)` stream into **contiguous runs** (address
     increments within a slot). Drop run #0's dummy read (§2.1) and any run in
     `>0480–>04A0` while a sound list is active (§2.3) — in practice: keep runs
     that *begin* at a decoded instruction boundary.
   - Walk each run with `decode()`: starting at the run's first address, each
     decoded instruction must consume exactly the bytes the interpreter
     fetched, ending precisely where the run ends (the next run begins at the
     decoded branch target when the last instruction branches).
   - Gate: the boot trace (≥ 2,000 instructions) decodes with **zero**
     desynchronizations. Any ISA-table error dies here, loudly, at a named
     address. Extend to a menu trace (cart mounted, key pressed) for coverage
     of SCAN/MOVE/compare groups.

### Phase 3 — Disassembler + close the recon gaps

1. `src/disasm.rs`: linear + flow-following pretty-printer over `decode()`
   (labels for branch targets, `G@/V@/@` operand syntax as §7.2).
2. **GROM-cart dispatch** (the one ❓ left in the menu contract): edit a copy
   of the probe to mount `cartridges/amazing.ctg` (GROM-only) instead of
   centipede, select it, and capture the final 100 fetches before console-GROM
   reads stop and cart-GROM (`>6000+`) fetches begin. Identify the mechanism
   (expected: a ROM XML helper that loads the GROM address counter from a
   scratchpad cell — find *which* cell and *which* XML operand). Record in
   `RECON.md`; our menu uses the same sequence.
3. **R3 sweep**: for each of ~20 carts across the census classes, boot + launch
   + run 600 frames with recording on; bucket console-GROM fetches
   (`< >6000`) that occur *after* dispatch: `>0000–>005F` (vector use),
   `>0060–>3FFF` (GROM 0/1 code), `>4000–>5FFF` (GROM 2 library). Write the
   histogram into `RECON.md`. Expected ❓: empty or tiny; whatever shows up
   defines Phase 10's stub list.

### Phase 4 — Encoder + assembler

1. `src/encode.rs`: inverse of `decode` — property test
   `decode(encode(i)) == i` across the full operand matrix (every form ×
   direct/indirect × CPU/VDP × indexed, boundary addresses `>8300`, `>837F`,
   `>92FF`, `>0000`, `>FFFF`).
2. `src/asm.rs`: two-pass driver **mirroring `libre99-asm`'s** (`lib.rs:90-203`):
   reuse `libre99_asm::lex` for lines/labels and `libre99_asm::expr` for operand
   arithmetic. Directives: `GROM >addr` (absolute origin, gap-fill `>00`),
   `BYTE/DATA/TEXT/BSS/EVEN/EQU` with `libre99-asm` semantics. Errors: `BR/BS`
   cross-slot target; any code/data crossing a `>2000` slot boundary
   (mirroring the bank-crossing warning philosophy); FMT mnemonic → "not
   supported in v0".
3. Operand syntax (§7.2), source order **`OP src, dst`** — deliberate local
   convention matching `libre99-asm` muscle memory; TI's historical GPL syntax
   differs, which is irrelevant since no TI GPL source exists to assemble.
4. Output: `pub fn assemble(src: &str) -> Result<GromAssembly, Vec<Diag>>` with
   `GromAssembly { image: Vec<u8> /* padded to 24 KiB */, symbols, entry_used: bool }`.
5. Golden encoding tests hand-derived from §7.3 (the libre99-asm pattern,
   `lib.rs:654-739`) — **not** derived from TI GROM bytes (IP guardrail); the
   oracle already cross-validates against reality.

### Phase 5 — M0 gate: a trivial GROM on the real ROM

`crates/libre99-gpl/tests/boot_trivial.rs`:

```text
        GROM >0000
        BYTE >AA,>02,>00,>00      ; valid header, version 2
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020                 ; the ROM's fixed GPL entry (§2.1)
START   BACK >17                   ; VDP R7 := >17
LOOP    B    LOOP
```

Assemble in-memory, `Machine::new(CONSOLE_ROM, &image)`, run 5 frames, assert
`m.vdp().register(7) == 0x17`. **This single test proves the entire premise**:
our bytes, TI's interpreter, observable effect. If it fails: check the header
is present (❓ whether the ROM validates `>AA` before jumping — if it does, the
failure mode is no GROM fetch at `>0020`; the trace shows either way).

### Phase 6 — M1: title screen

Source at `original-content/system-roms/grom/console.gpl` (committed), plus an
in-memory boot test. Structure, in order:

1. **Init block** (before touching the VDP): zero `>837A`, `>83C2`, `>83C4/5`,
   `>83CC–>83CF`, `>8374` (keyboard mode 0), `>8375` := `>FF`; leave `>8372/3`
   alone (the ROM set them ✅ §2.1-adjacent — verify values `>FE/>7E` in the
   entry snapshot before relying on stack ops ❓→check in P5's trace).
2. **VDP register block**: mirror TI's values (§2.2) — set R1 `>20` first
   (display off), name `>0000` / pattern `>0800` / color `>0380` / sprites
   `>0300`, backdrop = our choice.
3. **Font load**: `MOVE 768, G@FONT, V@>0900` (chars `>20–>7F` land at
   `>0800+8·code`). The font is 96 original 8×8 glyphs as `BYTE` rows —
   design once (a clean 5×7-in-8×8; hand-author or generate with a
   `build.rs`-free one-off and paste as source; the glyphs are original
   expression, part of the deliverable).
4. **Color table**: `MOVE 32, G@COLORS, V@>0380` — 32 bytes of `>17`-style
   (fg/bg) entries, our palette.
5. **Screen paint**: `ALL >20` (clear), then `MOVE`s of `TEXT` strings from
   GROM into name-table rows — `JOEL ODOM ROM REWRITE` (21 cols) centered,
   `(WWW.JOELODOM.COM)` below, our banner glyphs above. Raise R1 to `>E0`.
6. **Key wait**: `SCAN` / `BR`-loop until a fresh key (condition bit 📖 set on
   new key; verify against the traced title loop from P3's listing), then fall
   into the menu (Phase 7; until then, loop forever).

Gate `boots_to_rewrite_title_screen`: assemble, boot, 60 frames, assert both
strings via F14's vram check, `register(1) & 0x40 != 0`, and (like
`tests/boot.rs:68-73`) ≥ 3 distinct framebuffer colors.

### Phase 7 — M2: selection list

GPL, all in GROM 0:

1. **Scan**: for base in `>0000,>2000,>4000,>6000,…,>E000`: read GROM byte at
   `base` (via `MOVE 1, G@base, @tmp` — GROM source addressing of a computed
   base needs the idiom recovered in P3 ❓; a static unrolled scan of the eight
   bases is the fallback and is fine for v0), require `>AA`, then walk the
   program list at `base+>06`: entries are `{next(2), entry(2), len(1),
   name(len)}` — copy the name into the next menu row, record `(kind=GPL,
   entry)` in a table at `>8320+` (our cells, §8). Then probe **CPU `>6000`**:
   read `@>DD00`-style extended operands (16-bit form, bias per §F7) for `>AA`,
   walk its list identically with `kind=ML`. Numbering starts at 1; our GROM 1
   program list (`TI PYTHON`) is found by the same scan — no special case.
2. **Render**: `n FOR NAME` rows, exactly the real spacing (§2.3 dump: rows
   7,9,11…), because it's tasteful *and* it keeps F14 assertions simple.
3. **Select**: `SCAN` loop; accept ASCII `'1'..'9'` ≤ count.
4. **Dispatch**: `kind=GPL` → the P3-recovered mechanism (set GPL PC to entry).
   `kind=ML` → `DST @>8380, entry ; XML >F0` (✅ §2.3).

Gate (two tests): mount `amazing.ctg` (GROM-only) → lists at 2, launches
(cart-GROM fetches begin); mount `centipe.ctg` (ROM-only) → lists at 2,
launches (`PC ∈ >6000–>7FFF` within 120 frames). Plus `VideoGames1.ctg` lists
**three** entries (F12).

### Phase 8 — M3: compatibility sweep

1. Helpers from F14; a table-driven test over the representative sample:
   `amazing` (GROM-only), `HuntTheWumpus` (GROM, 3 programs), `Parsec`,
   `TI-Invaders` (GROM+ROM), `centipe` (ROM plain), `DigDug`, `MoonPatrol`
   (ROM banked), `VideoGames1` (multi), `et` (7 programs, banked). Assert: every
   declared program listed, first program of each launches.
2. QUIT test: launch one cart, `Fctn`+`Equals`, assert our title strings return
   within 60 frames (ISR reboot, F3).
3. `#[ignore]`d full sweep over `cartridges/*.ctg`: parse, count expected menu
   entries (the §2.4 census logic), boot our GROM, compare listed count,
   dispatch entry 2 (or 1 where only one), assert launch signal; emit a
   pass/fail table. Triage the 11 headerless images first against the
   **authentic** GROM (same harness, TI image): whatever fails there too is
   out of scope — record the list in `RECON.md`.

### Phase 9 — M4: TI PYTHON v0

In GROM 1 (`>2000` header: `AA 02 01 00`, program list → `{0, REPL, 9,
"TI PYTHON"}`), implementing plan §9 with F13's decisions. Build order, each
with its boot-and-type test (drive keys per F14, assert screen rows):

1. Banner + prompt + echo loop (mode 5 ❓ verification happens here: type
   `(`, `+`, `=`; if wrong codes arrive, drop to mode 0 + shift table).
2. ENTER → read row back from VDP → integer literal echo (tokenizer + decimal
   print via repeated `DIV` by 10; `-32768` special case).
3. `+`/`-` left-assoc; then `*`/`/`/`%` precedence; then parens (recursive
   descent via `CALL`; depth cap → `SYNTAX ERROR`).
4. Variables (16-slot table, §F13), `NAME ERROR`.
5. `ZERO DIVISION ERROR`; stray chars → `SYNTAX ERROR`; wrap semantics test.
6. The plan §9 session transcript as one end-to-end test, plus
   `10/3=3`, `-10/3=-3`, `-10%3=-1`, `32767+1=-32768`.

### Phase 10 — M5/M6: vectors, flags, embed, docs

1. Point the fixed low vectors (`>0010–>0037` slots, and branch stubs at
   `>0038+`) at our routines for whatever P3's R3 histogram actually showed;
   everything unused gets a stub branching to the title (safe failure mode).
   Implement `>0016`/`>003D` (charset loaders 📖) against our font if any
   sampled cart calls them.
2. `--system-grom <path>` (and `--system-rom <path>`) in `cli.rs` + `USAGE` +
   `app.rs` wiring + help overlay + README, following `--cartridge-file`
   end-to-end; overrides skip save-state resume.
3. Build `console-grom.bin` (24 KiB), commit as an artifact beside its source
   (titris convention, `original-content/cartridges/titris/`), write
   `grom/README.md` (how it works, how to rebuild, address map).
4. Embed + preference toggle (rewrite vs authentic) once the sweep is green —
   default per Joel's call (§10 Q1).
5. Disk regression (plan M5): boot the disk-based title on our GROM.

---

## 7. GPL reference card (for implementation without Classic99)

### 7.1 Execution model

The GPL "program counter" *is* the GROM chip's auto-incrementing address
counter; the ROM interpreter fetches via `>9800`, executes, and only rewrites
the counter (`>9C02`) on branches — that's why `grom_log` *is* an instruction
fetch trace (✅ exploited throughout §2). `CALL` pushes the return address on a
scratchpad stack (`>8373` = stack byte pointer ✅); the ISR may move the GROM
address mid-instruction and restores it (§2.3). Interrupts are effectively
always enabled between GPL instructions.

### 7.2 Operand grammar and our assembler syntax

Encoding per §F7. Source syntax (ours, by fiat):

| Syntax | Meaning | Encodes as |
|---|---|---|
| `@>83CE` / `@CELL` | CPU direct | form 1 if `>8300–>837F` (`>CE` → `4E`… **no**: single byte is the *offset* `>4E`? — the offset byte is `addr->8300`, so `@>834E`→`4E`, `@>83CE`→ 12-bit `80 CE` ✅) |
| `@>2000` | CPU direct, far | 16-bit form, value `addr->8300` (mod 64K) |
| `V@>0380` | VDP RAM direct | 12/16-bit form with V=1 (VDP addresses are **not** biased ❓ — verify with one traced title `MOVE`; adjust encoder + this row) |
| `*@>83xx` / `V*@…` | indirect through CPU word | I=1 |
| `@>8300(@>83xx)` | indexed | X=1, index byte last |
| `G@LABEL` | GROM address (MOVE src/dst only) | per MOVE's operand layout |
| `#>17` / `#N` | immediate byte/word where the op takes one | raw byte(s) |

Rows marked ❓ are exactly the ones the Phase 2 oracle nails down for free.

### 7.3 Opcode map

Confidence: ✅ anchored by this review's traces · 📖 literature · ❓ verify via
oracle before trusting the row. The oracle (7.4) is the arbiter for *all* rows.

| Range | Mnemonics | Notes |
|---|---|---|
| `>00` RTN, `>01` RTNC | return (clear / preserve condition) 📖 |
| `>02` RAND, `>03` SCAN | 📖; SCAN → key at `>8375`, mode `>8374`, condition = new key |
| `>04` BACK imm8 | ✅ shape (Phase 5 proves); sets VDP R7 |
| `>05` B addr16 | ✅ (`05 4D 12` at `>0038`) |
| `>06` CALL addr16 | 📖 |
| `>07` ALL imm8 | fill screen with char 📖 |
| `>08` FMT … `>FB` FEND | sub-language; decode-only (F8); text counts are **count−1** ✅ |
| `>09–>0D` H, GT, EXIT(`>0B`), CARRY, OVF | copy status bit → condition; EXIT 📖 |
| `>0E` PARSE, `>0F` XML op8 | XML ✅ (`0F F0` dispatch); operand = table:entry nibbles; table `>F` = RAM vectors at `>8380` ✅ |
| `>20–>3F` MOVE | length(word operand) + dst + src; variant bits in low 5 opcode bits select GROM/CPU/VDP/register targets; `39 00 08 00 04 51` = 8 bytes GROM→VDP-regs ✅ observed shape; exact bit meanings ❓→oracle |
| `>40–>5F` BR, `>60–>7F` BS | 13-bit slot-absolute ✅ (F6) |
| `>80–>9F` single-operand group | ABS/NEG/INV/CLR/FETCH/CASE/INC/DEC/INCT/DECT + `D`-word twins 📖; `8F @cell` observed as a test-style op in the sound-wait ✅usage |
| `>A0–>AF` ADD/SUB/MUL/DIV (byte/word/imm variants) 📖 |
| `>B0–>BF` AND/OR/XOR/ST/EX family | `BE/BF` = ST/DST immediate-ish ✅usage at boot (stack-pointer setup) |
| `>C0–>DF` compares (CEQ/CH/CHE/CGT/CGE/CLOG…) | set condition 📖; `CF`/`CD` observed in menu loops ✅usage |
| `>E0–>EF` shifts | 📖 |
| `>10–>1F`, `>F0–>FF` | unassigned/rare ❓ — decoder should error loudly; the oracle tells us if the real firmware uses any |

Byte-vs-word and immediate-vs-memory variants ride on low opcode bits within
each group 📖 — lay the table out per-opcode (256 rows) so the oracle can
correct individual entries without structural rework.

### 7.4 The fetch-differential oracle (the ground truth that replaces Classic99)

Because `grom_log` records every byte the *real interpreter* consumed, in
order (§7.1), a decoder that walks the same image must reproduce the fetch
stream exactly: instruction lengths, branch targets, MOVE data reads. Protocol
in Phase 2 step 4. Properties: needs no external reference, runs on every
machine the repo builds on, localizes an ISA error to the first wrong address,
and covers exactly the opcodes the firmware actually uses (which is exactly the
set our menu/REPL must coexist with). Extend coverage by tracing more
scenarios (menu with carts, BASIC keyed in on the authentic GROM, a GPL game).

### 7.5 External references (secondary)

- Thierry Nouspikel's TI-99/4A Tech Pages — GPL instruction set & console
  internals (the community bible; fetch when online).
- TI *GPL Programmer's Guide* (1979) and the E/A manual (GPLLNK tables) — the
  repo already carries `assembler/Editor_Assembler_Manual.pdf`.
- Classic99 `addons/gpl.cpp`, `console/Tiemul.cpp` — checked out on both
  workstations (`C:\ClaudeShared\classic99` on the PC, `/Users/Shared/classic99`
  on the Mac); plan §12's citations resolve on either. Consult, never copy.

---

## 8. Scratchpad map (`>8300–>83FF`)

Ownership legend: **SYS** = ROM interpreter/ISR/SCAN — initialize as noted,
never repurpose; **OURS** = free for our GPL. Values ✅ from the §2 snapshots.

| Cells | Owner | Contents / required init |
|---|---|---|
| `>8300–>833F` | OURS | TI PYTHON variable table (F13); menu scratch before that |
| `>8320+` (menu time) | OURS | menu entry table `(kind, entry)` (Phase 7) |
| `>8340–>8349` | OURS | parser temps (F13) |
| `>834A–>836F` | avoid | ROM floating-point workspace (FAC) 📖 — unused by v0 but stay out |
| `>8370/1` | SYS | top of free VDP RAM; ROM/GPL sets `>3FFF` ✅ |
| `>8372` / `>8373` | SYS | GPL data / subroutine stack pointers (`>FE` / `>7E` ✅) |
| `>8374` / `>8375` | SYS | SCAN: keyboard mode / key code (init `0` / `>FF`) |
| `>8376/7` | SYS | joystick Y/X (SCAN modes 1–2) |
| `>8378` | SYS | random byte (ISR-stirred) |
| `>8379` | SYS | VDP-interrupt timer (free-running counter — cursor blink source) |
| `>837A` | SYS | auto-sprite count — **init 0** |
| `>837B` | SYS | VDP status copy |
| `>837C` | SYS | GPL status byte (condition/carry/…) |
| `>837D–>837F` | SYS | interpreter internals (`>10`,`>1E` observed ✅ — leave alone) |
| `>8380–>839F` | SYS | **XML table `>F` — RAM branch vectors; `>8380/1` is the ML-dispatch slot** ✅ |
| `>83A0–>83BF` | ❓ | unknown; treat as SYS until R3 says otherwise |
| `>83C0–>83DF` | SYS | ISR workspace & cells: `>83C2` disable flags (**init 0**), `>83C4/5` user hook (**init 0**), `>83C8–>83CA` scan debounce ✅, `>83CC/D` sound list ptr + `>83CE` count (**init 0**, F16), `>83D4` VDP-R1 image (**init = our R1**), `>83D6/7` blank timeout (**init 0**) |
| `>83E0–>83FF` | SYS | GPL interpreter workspace (R0–R15); R13=`>9800`, R14=flags, R15=`>8C02` ✅ |

---

## 9. Consolidated test matrix

| Test (crate) | Asserts | Phase |
|---|---|---|
| `oracle::boot_trace_decodes` (libre99-gpl) | zero decode desync over real boot | P2 |
| `oracle::menu_trace_decodes` | ditto w/ cart + keypress | P2 |
| `encode::roundtrip_matrix` | `decode∘encode = id` | P4 |
| golden encodings | hand-derived §7.3 bytes | P4 |
| `boot_trivial` | `BACK` reaches VDP R7 under real ROM | P5 |
| `boots_to_rewrite_title_screen` (libre99-core or libre99-gpl) | our strings in VRAM, display on, ≥3 colors | P6 |
| `menu_lists_and_launches_grom_cart` | amazing @2 launches | P7 |
| `menu_lists_and_launches_rom_cart` | centipe @2, PC∈`>6000+` | P7 |
| `menu_walks_multi_program_lists` | VideoGames1 ×3 | P7 |
| `sweep_representative` (9 carts) | list + dispatch | P8 |
| `quit_returns_to_title` | FCTN-= reboot | P8 |
| `sweep_all_137` `#[ignore]` | census-vs-menu diff table | P8 |
| `ti_python_*` (6 tests) | F13 + plan §9 transcript | P9 |
| `disk_title_boots` | plan M5 | P10 |

---

## 10. Questions for Joel (none block P0–P8)

1. **Embed default** (plan §11-open): once the sweep is green, boot the rewrite
   by default with `--authentic-grom` opt-out, or keep TI default? Reviewer
   leans rewrite-default — it's the point of the project — behind a preference.
2. **Menu beep**: accept the silent v0 (F16), or is the beep part of the
   "feels like a TI" bar for M6?
3. **TI PYTHON division**: truncating `/` and `%` documented as a deviation
   from CPython (F13) — acceptable for v0?
4. **The 11 headerless carts** (§2.4): if they don't list on the authentic
   GROM either, may they be excluded from the M3 gate with a note?

---

*Review artifacts: the recon probe lives at
`crates/libre99-core/examples/recon_probe.rs`; run it from the repo root. The
cartridge census used the `.ctg` layout in `cartridge.rs:9-28` directly. No
plan text was modified; apply §3's deltas during implementation and record
recon updates in `original-content/system-roms/RECON.md`.*
