# DEBUGGING — strategy & playbook for the GROM rewrite (living document)

**This is a shared, frequently-updated knowledge base.** It is written so that
*any* session — including smaller/faster AI models — can debug the rewritten
console GROM effectively by following the protocol below literally. When you
debug, record what worked here: new traps, new instruments, and a short case
study for each real bug. Future sessions should **read this first** and
**append to it**. It complements [`RECON.md`](./RECON.md) (the *interface*
facts) with *how we discover* those facts.

This file is the **operational** half. Its companion,
**[`GROM-DEBUGGING-GUIDE.md`](./GROM-DEBUGGING-GUIDE.md)** ("A Software
Engineer's Guide to TI-99 GROM Debugging"), is the **architecture** half: the
machine's three-layer mental model, the boot-stage ladder, the spot-diagnosis
table, the GPL bug taxonomy, and the deep methodologies. Read the guide once
before your first bug; start every bug at its §5 spot-diagnosis table.

Bugs arrive as **fresh session reports from Joel** — there is no ticket file.
The report in the conversation is the spec; if a session ends without a fix,
persist everything learned in the **Open investigations** section below so the
next session starts where you stopped, not from zero.

---

## The golden rules

1. **Suspect our GPL first; the emulator last.** The same genuine console ROM
   runs both the authentic GROM and ours, so booting the authentic GROM through
   the identical scenario is a controlled experiment: if authentic behaves and
   ours doesn't, the bug is in `grom/console.gpl` (or the spliced font/keymap).
   Only touch `crates/libre99-core` if the symptom reproduces under the
   **authentic** GROM too — that's an emulator bug, a different playbook (see
   the last section).
2. **Never guess-and-patch.** Every hypothesis gets a cheap in-emulator
   experiment *before* any change to `console.gpl`. If you cannot name the
   observation that would falsify your hypothesis, you don't have a hypothesis
   yet — go collect more state.
3. **One change at a time**, re-running the reproducing probe after each.
4. **Never weaken, skip, or delete a test to get green.** Gates only ratchet.
5. **Trust order:** your own in-emulator experiment > a ✅ fact in `RECON.md`
   > 📖 literature/Classic99 > your inference. When a decode/disassembly and an
   execution result disagree, the execution result wins.
6. **Budget your attempts.** If three hypotheses die in a row, stop. Write what
   you observed, what you ruled out, and the exact probe commands into **Open
   investigations** (bottom of this file), and leave it for a fresh session. A
   precise writeup of a narrowed search space **is** a successful outcome.
7. **Finish the job:** root cause → regression test (fails before, passes
   after) → rebuild `console-grom.bin` → full test + clippy green → update
   `LIMITATIONS.md`/`RECON.md` if facts changed → case study here + a
   distilled lesson in the guide's §9 register if it taught something general
   → commit.

## The standard protocol

Follow these steps in order; do not skip Step 2 even when the cause seems
obvious.

- **Step 0 — Orient.** Read Joel's bug report carefully. Check
  [`LIMITATIONS.md`](./LIMITATIONS.md): is this a *known, documented gap*
  rather than a bug? Skim [`STATUS.md`](./STATUS.md) for what is supposed to
  work. Look the symptom up in the guide's spot-diagnosis table
  ([`GROM-DEBUGGING-GUIDE.md`](./GROM-DEBUGGING-GUIDE.md) §5) and check
  **Open investigations** and the case studies below for prior work on it.
- **Step 0.5 — Triage: does this bug earn a fix session?** (The anti-rabbit-hole
  rule — [`history/QUALITY-ASSESSMENT.md`](./history/QUALITY-ASSESSMENT.md) §8
  (archived).) Fix now **only**
  if one holds: **(Tier 1)** it is console behaviour — title, menu, dispatch, TI
  PYTHON, the ISR contract, or reset; **(Tier 2)** a bundled cartridge's *primary
  flow* (boot → menu → launch → play with keyboard/joystick → QUIT) is broken, ≥ 1
  cart is affected, and the cause is ours (Step 0 said it is not a known
  LIMITATION); or **(Tier 3)** the coverage sweep / stub log shows multiple carts
  touching the same gap. Everything else — exotic non-bundled carts,
  real-hardware-only fidelity, deep BASIC-era library calls, cosmetic timing —
  becomes a `LIMITATIONS.md` entry with a path forward, *by design and without
  guilt*, and is closed via the §7 ledger, not chased here. (The seven enumerated
  ledger entries L1–L7 are exempt — they are being driven to zero by §7.)
- **Step 1 — Reproduce in a probe.** Copy the nearest example from
  `crates/libre99-gpl/examples/` (inventory below) into a new example. Build our
  GROM in memory (`libre99_gpl::system_grom::build_console_grom()`), script the
  exact scenario from the bug report (cartridge, keys, frames), and print the
  observable that is wrong. No fix attempts until the probe shows the symptom.
- **Step 2 — Run the authentic GROM through the same scenario**
  (`roms/994AGROM.Bin`). If the symptom appears there too, it is an emulator
  or expectation bug — stop and re-read the report. If authentic is fine, the
  gap is ours. This one run halves the search space and proves the emulator
  innocent; it is mandatory.
- **Step 3 — First-divergence hunt.** Sample the health panel (next section)
  each frame under both GROMs; find the **first** frame and the **first**
  signal where they diverge. Diff scratchpad cells, VDP registers, screen
  rows — whatever the symptom implicates. Divergences downstream of the first
  one are noise; chase the first.
- **Step 4 — Localize to a GPL instruction.** Turn on
  `m.bus_mut().grom_record(true)` around the divergence — the GROM fetch log
  **is** the GPL instruction trace. Decode it with
  `libre99_gpl::decode::decode_at(img, off, addr)`. Identify which of our
  instructions ran wrong, or which authentic instruction has no counterpart in
  our GPL. `m.cpu().pc()` / `m.reg(n)` tell you which ROM routine was executing
  (R12 = CRU base).
- **Step 5 — Pin the fix empirically.** Assemble a *minimal* candidate GROM (or
  a sweep of candidates) and assert the observable now matches authentic —
  `examples/cru_experiment.rs` is the model for this. Only after a candidate
  wins do you write the clean, commented GPL into `console.gpl`.
- **Step 6 — Ship.** Regression test in `crates/libre99-gpl/tests/`; rebuild the
  committed artifact (`cargo run -p libre99-gpl --bin libre99gpl -- console
  original-content/system-roms/grom/console-grom.bin`); `cargo test -p
  libre99-core -p libre99-gpl` and `cargo clippy` green; update the docs per rule 7;
  commit.

## The health panel — first five minutes of any "console feels weird" bug

Check these in order; each takes one line in a probe:

| # | Check | Healthy | If not |
|---|---|---|---|
| 1 | `m.bus().peek(0x8379)` changes across frames | yes | **the VBLANK ISR is not running** — whole classes of symptoms (no sound, no sprite motion, no cursor blink, QUIT ignored) are this one bug. Go to check 2. Case study 1. |
| 2 | `m.bus().tms9901.vdp_interrupt_enabled()` | `true` | boot never enabled CRU bit 2 — the `IO` block in `START` (RECON §11) is broken/skipped |
| 3 | `m.bus().interrupt_line()` vs `m.vdp().interrupt_pending()` | line fires when pending | pending-but-no-line = 9901 mask; line-but-no-ISR = CPU mask (LIMI/ST) |
| 4 | `m.vdp().register(1)` | `>E0` (16K + display + int-enable) | our VDP init or the `>83D4` R1 image is wrong |
| 5 | `>83CC/D` (sound-list ptr) and `>83CE` (bytes remaining) | ptr advances while a tune plays | list installed but frozen = ISR (check 1); never installed = the program's own code path |
| 6 | screen text via the VRAM recipe (below) | expected rows | rendering/scan logic; diff rows against authentic |

## The setup, in one place

- **Run tools from the repo root.** `cargo` is not on PATH in the sandbox —
  prepend it: `export PATH="$HOME/.cargo/bin:$PATH"` (bash) or
  `$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"` (PowerShell).
- **Build our GROM in-memory:** `libre99_gpl::system_grom::build_console_grom()`
  assembles `console.gpl` + spliced font/keymap into the 24 KiB image. No need
  to rebuild the committed `console-grom.bin` while iterating — tests/probes
  call `build_console_grom()` directly.
- **Boot it:** `Machine::new(CONSOLE_ROM, &grom)` then `m.reset()`; drive time
  with `m.run_frame()` (never bare `m.step()` loops for behaviour — see traps).
- **The authentic GROM** is `roms/994AGROM.Bin`; **the console ROM** (the GPL
  interpreter we keep) is `roms/994aROM.Bin`.
- **Classic99** (`C:\ClaudeShared\classic99` on the PC,
  `/Users/Shared/classic99` on the Mac) is the ISA/hardware reference —
  *consult, never copy*. Its `addons/gpl.cpp` documents GPL opcode semantics
  (e.g. the `IO` function codes).

## Instruments (all in the emulator, diagnostics-only)

| Tool | What it gives you |
|---|---|
| `m.bus().peek(addr)` / `peek_word` | any RAM/scratchpad byte without side effects (⚠ not `>6000–7FFF` — see traps) |
| `m.bus_mut().grom_record(true)` + `grom_log()` | every GROM fetch `(addr, byte)` — **this is a GPL instruction trace** |
| `m.cpu().pc()`, `m.reg(n)` | CPU PC and workspace registers (R12 = CRU base!) |
| `m.vdp().register(n)`, `.vram(a)`, `.interrupt_pending()` | VDP state |
| `m.bus().psg.volume(ch)` / `.period(ch)` | sound: a channel is **audible** iff `volume < 0x0F` |
| `m.bus().tms9901.vdp_interrupt_enabled()` / `int_mask()` | is the 9901 VDP interrupt (CRU bit 2) armed? |
| `m.bus().interrupt_line()` | does a level-1 interrupt currently reach the CPU? |
| `libre99_gpl::decode::decode_at(img, off, addr)` | disassemble GPL bytes back to instructions |
| `m.set_key(k, pressed)` | inject keys (hold ≥ 2 frames — recipes below) |

## Probe inventory (copy the nearest one; don't start from scratch)

All in `crates/libre99-gpl/examples/` unless noted; run with
`cargo run -p libre99-gpl --example <name>` from the repo root.

| Probe | Demonstrates |
|---|---|
| `sound_probe` | **the differential-trace pattern**: boots ToD under authentic + ours, samples PSG/ISR/sound cells per frame |
| `invaders_probe` | **the glyph-coverage differential** (guide §4.8): launches TI Invaders under both GROMs, reports the name table as ASCII, a rendered non-backdrop pixel count, and — per on-screen code — whether its pattern-table glyph is loaded under each GROM (finds "text is blank because a font/char-set load didn't happen"); also traces the console-code call chain to pin which interconnect vector the cart called |
| `tod_load_probe` | **the user-session pattern** (guide §4.8): drives ToD as a real player to the "LOAD DATA FROM" screen under both GROMs, dumps each screen as ASCII + a health panel, then diffs the *cartridge's own* fetch stream to pin the console call that breaks a load |
| `tod_disk_probe` | **the device-I/O probe** (M7): mounts `Disk.Bin` + `Tunnels.Dsk`, drives ToD's disk-load of QUEST under both GROMs, and reports screen + disk `read_log` + CRU/FD1771 activity + interconnect slots + a scratchpad diff, and decodes the authentic DSRLNK vs. our reimplementation |
| `cru_experiment` | **the empirical fix-pinning pattern**: assembles candidate mini-GROMs, sweeps operand recipes, reads back chip state |
| `show_title` | prints our title screen name table as ASCII + a PPM screenshot |
| `tipython_probe` | fast REPL iteration: dispatches straight into TI PYTHON (skips the slow menu), types a line, reads the screen |
| `census_probe` | per-cartridge: programs the header declares vs. what our menu lists |
| `keymap_probe` | presses every key on the authentic GROM, records the KSCAN table offsets the ROM reads |
| `menu_scan_trace` | traces how the *authentic* menu reads cartridge headers (GROM fetch stream) |
| `scan_check` | isolates SCAN/keyboard behaviour, authentic vs. ours |
| `fctn_probe` | presses FCTN+key on both GROMs; recovers which >1700 block the ROM reads for the arrow/edit keys (mode-0 FCTN table) |
| `joystick_scan_check` | assembles a key-unit-1 SCAN loop carrying our keytab vs. none; shows the `>17C8` joystick table is what makes in-game directions register (dead `>00` without it) |
| `joy_column_trace` | steps the CPU through a key-unit-1 SCAN loop and histograms the 9901's selected column; proves the ROM reaches the joystick column (6/7) — rules the CRU layer in/out |
| `joy_gromtrace` | logs every GROM addr the ROM's SCAN reads per joystick direction; recovered the deflection table at `>16EA` (`(Y,X)` deflection pairs) that the joystick — not the keyboard — needs |
| `joy_gameplay_probe` | drives TI Invaders into LIVE gameplay (fire launches the wave), isolates the ship sprite, and reports its X move for joystick vs. keyboard, authentic vs. ours |
| `reset_artifacts_probe` | drives TI Invaders into gameplay, then `m.reset()` (F5) and re-boots; dumps the name-table dirty-cell count, sprite-list state, and row 23, authentic vs. ours — shows the boot must repaint the whole screen because reset leaves VRAM untouched |
| `m2_probe`, `m4_probe`, `move_probe`, `move_c_probe`, `menu_probe` | the mechanism-pinning probes that verified dispatch, MOVE forms, arithmetic (RECON §§1–8) |
| `recon_probe` (in `libre99-core`) | the original boot/title/menu recon against the authentic images |
| `f5_press2_probe` (in `libre99-core`) | **the F5-warm-state pattern**: runs Joel's exact flow — cold and F5-from-playing — under the **committed** `console-grom.bin` (the bytes the user ran) and authentic; reports launch, health, a cold-vs-warm scratchpad diff, and a post-keypress GROM-fetch histogram that names the stuck loop (found case study 9) |
| `f5_mask_bisect` (in `libre99-core`) | frame- then instruction-level bisect of a 9901 state flip after F5; watches a cell (`>8300`) around a suspect instruction and prints PC/R12/GPL-PC at the transition — pinned the `IO` list's data-address byte |

## Tools: use → extend → build

The strategy for tooling, in priority order:

1. **Use** an existing probe (table above). Most bugs need only a copied probe
   with a different cartridge/keys/cells.
2. **Extend** the nearest probe (add a knob, print one more cell) rather than
   forking a near-duplicate.
3. **Build** a new reusable tool when you notice yourself hand-writing the same
   scaffold for the *second bug in a row*. New tools live as examples in
   `crates/libre99-gpl/examples/` (zero third-party deps, runnable from the repo
   root), get a one-line entry in the inventory table above, and get mentioned
   in the case study of the bug that motivated them.

**Candidate tools** (build when a bug first needs them — not before):

- `diff_probe` — a general differential dashboard: boot authentic + ours
  through the same scenario (optional cartridge, scripted keys), print the
  health panel per frame plus the wholesale scratchpad diff (guide §4.1), flag
  the first divergence. Generalises `sound_probe`; would collapse protocol
  Steps 2–3 into one command. Likely the first tool worth building.
- `screen_diff` — dump/diff the name table as ASCII for both GROMs at frame N
  (today `show_title` prints ours only). *Partly realised:* `tod_load_probe`
  now dumps the screen as ASCII along a scripted keystroke path under both
  GROMs — promote its `screen()`/`health()` helpers into a shared tool the
  second time a bug needs them.
- a symbolized GPL trace printer — `grom_log` decoded via `decode_at` with
  labels resolved from `console.gpl`'s symbol table, so traces read like source.

## The core technique: differential trace (authentic vs. ours)

Because we run the **same console ROM** under both GROMs, any behavioural gap
is something *our GPL does or omits*. The workflow that has cracked every hard
bug so far is exactly protocol Steps 1–5: reproduce → authenticate → diff state
frame-by-frame to the first divergence → localise with the fetch trace → pin
the fix empirically before writing clean GPL.

**ISR-liveness check** (invaluable): the console ROM's VBLANK ISR stirs `>8379`
(and `>8378`) every frame. If `>8379` never changes across frames, *the ISR is
not running* — interrupts aren't reaching the CPU. That one signal
distinguishes "the ISR ran but did the wrong thing" from "the ISR never ran".

## Traps (things that will waste an afternoon)

- **Bare `step()` loops hang or lie.** Always advance with `run_frame()` so
  VBLANK fires; menus that wait on a sound list (`>83CE`) spin forever
  otherwise (RECON R2).
- **`bus().peek()` cannot see `>6000–7FFF`** (the cartridge window) — it reads
  zeros there. Use `cart.rom` in tests or `Bus::read_byte`.
- **`INC`/`DEC` are byte-only.** Use `DINC`/`DDEC`/`DADD` on word pointers — a
  classic silent corruption.
- **GROM→VDP `MOVE` is slow** (the ROM rewrites the GROM address per byte); a
  big copy *looks* like a hang. Give probes a generous frame budget (RECON §10).
- **The assembler rejects a construct → it is probably banned** (RECON §7:
  indexed GAS, `MOVE` C=1, FMT — all verified-failed). Redesign with verified
  forms; don't hand-encode via `BYTE` unless you pin the semantics with a probe
  first (that's how the `IO` fix was done — legitimately).
- **GPL misbehaves with no obvious cause → suspect an operand-encoding desync**:
  cut the program down, verify each instruction's bytes with
  `libre99_gpl::disasm`, and pin semantics with a 20-line probe on the real ROM.
- **Disassembler signatures can be approximate.** `decode_at` models `IO`
  (`>F4–F7`) as two GAS operands, but the second byte is really an *immediate
  function code*. Trust an **empirical** in-emulator test over a decode when
  they disagree.
- **Scratchpad powers up zero *in the emulator* but random on hardware.** A bug
  can hide because a cell happens to be zero. Initialise every cell the ROM/ISR
  reads (RECON scratchpad map), don't lean on the zeros.
- **Keys need hold-and-release**: `set_key(k, true)`, ≥ 2 `run_frame()`s, then
  release — and copy `>8375` out immediately after SCAN sees it (the ISR can
  overwrite it).

## Test recipes (for probes and regression gates)

- **Screen text:** `let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;`
  then compare `m.vdp().vram(base + 32*row + col)` against plain ASCII (our
  charset is identity-mapped). ⚠ Mask register 2 — the firmware leaves it
  `>F0`, and the raw value overflows the multiply.
- **Key injection:** `m.set_key(key, true)`; ≥ 2 frames; `m.set_key(key,
  false)`; more frames. Drive everything with `run_frame()`.
- **"Cartridge launched":** ROM carts → `(0x6000..0x8000).contains(&m.cpu().pc())`
  within ~120 frames of the digit. GROM carts → `grom_record` shows sustained
  fetches at/above `>6000`.
- **"Sound audible":** any `m.bus().psg.volume(ch) < 0x0F`.
- **Frame budgets:** menu build takes ~1–2 s of emulated frames per cartridge
  (the MOVE slowness); give menu tests 120+ frames of grace before asserting.
- **Sweep hygiene:** the full 137-cart sweep stays `#[ignore]`d; the
  representative sample is the default gate.

## If it's (maybe) an emulator bug

Reproduced the symptom under the **authentic** GROM too? Then and only then
suspect `crates/libre99-core`. Cross-check the behaviour against Classic99's
source (per the repo `CLAUDE.md`: when we disagree with Classic99, Classic99 is
almost certainly right), reproduce its behaviour in our core, and add a focused
regression test in `libre99-core`. Do not change core behaviour to make *our
GROM* work — that masks the real bug and breaks the authentic image.

---

# Open investigations

When a session ends **without** a fix, leave the next session a head start
here: the symptom, the exact repro, every hypothesis tested with its outcome
(especially the ruled-out ones), the probe code or commands used, and the
narrowed search space. Delete the entry when the bug is fixed (its narrative
becomes a case study below).

*(none currently)*

---

# Case studies

Add one per real bug: symptom → trace → root cause → fix → lesson (distilled
lessons also go in [`GROM-DEBUGGING-GUIDE.md`](./GROM-DEBUGGING-GUIDE.md) §9).
Newest last.

## Case study 1 — "no sound at the Tunnels of Doom splash" (2026-07, fixed)

**Symptom.** With our GROM, launching Tunnels of Doom played no splash tune;
with the authentic GROM (same emulator) it did.

**Trace.** `sound_probe.rs` launched ToD under both GROMs and sampled the PSG +
the ISR sound cells each frame:
- Authentic: `>83CC` (sound-list pointer) **advanced** every few frames and a
  channel was audible — the ISR was walking ToD's sound list (in 32K RAM at
  `>CAxx`).
- Ours: the cart *installed* a list (`>83CC=CA10, >83CE=01`) but it sat
  **frozen** and stayed silent — **the ISR was not advancing it**.

**Localise.** ISR-liveness told the real story: `>8379` changed on **317/380**
frames under authentic vs **0/380** under ours — the ISR *never ran*, and had
been dead since the title screen, not just after launch. `interrupt_line()`
after each frame was `None` while `vdp.interrupt_pending()` was `true`: the VDP
was asserting its interrupt but it wasn't reaching the CPU. Per `cru.rs`, that
means the **9901's VDP interrupt mask (CRU bit 2) was never set**. A CRU-write
log confirmed **551** enables under authentic, **0** under ours.

**Root cause.** The console ROM does *not* enable the VDP interrupt on its own —
the **console GROM's boot GPL does**, with a GPL `IO` (CRU-output) instruction.
Disassembling the authentic boot (`decode_at` over GROM `>0080–00B0`) showed a
chain of `ST @>8303,imm ; IO @>8302,#3` (opcode `>F6`); the one with bit
address `2` runs `SBO` at ROM `>0604` with `R12=>0004`. Our rewrite's `START`
issued no `IO` at all, so the ISR — and therefore sound, sprite motion, cursor
timers, and **QUIT** — was dead.

**Fix.** In `console.gpl` `START`, initialise the ISR scratchpad cells and
enable the VDP interrupt with a GPL `IO`, using the CRU-output list format
pinned empirically in `cru_experiment.rs`:

```
        ST   @>8300,>FF        ; data: output bit value 1
        DST  @>8302,>0002      ; CRU bit address 2 = VDP interrupt
        ST   @>8304,>01        ; count: one bit
        BYTE >F6,>02,>03       ; IO @>8302,#3  (CRU output = SBO bit 2)
```

The `IO` list is `{ data @>8300, CRU-address-word @>8302, bit-count @>8304 }`;
the source operand's byte (`>03`) is function 3 = CRU output. The **data byte at
`>8300` must be non-zero** (that was the missing piece — the authentic firmware
sets it with `INV @>8300`; a zero data byte does `SBZ` and silently clears the
bit instead of setting it). Full interface write-up: RECON §11.

**Payoff.** One fix restored sound *and* QUIT (previously `LIMITATIONS.md` L1).
Guards: `tests/interrupts.rs` (`isr_runs_after_boot`, `tunnels_of_doom_plays_sound`)
and the now-passing `tests/sweep.rs::quit_returns_to_our_title`.

**Lesson.** Whole classes of "the console feels dead" bugs (no sound, no sprite
motion, no cursor blink, QUIT ignored) share one root cause: the ISR isn't
running. Health-panel checks 1–3 catch it in a minute.

## Case study 2 — "Tunnels of Doom won't load from disk" → console device I/O (M7, 2026-07, fixed)

**Symptom (Joel).** At ToD's `LOAD DATA FROM: 1-CASSETTE / 2-DISK 1 / 3-OTHER`,
selecting any device made the display "go bonkers" (VRAM garbage, ISR dead).
Goal: load a QUEST scenario from **DSK1** and run it. (Cassette deferred — no
emulator hardware; ROADMAP §6.)

**Trace (`tod_disk_probe`, the user-session/differential method, guide §4.8).**
Drove ToD to the disk-load prompt under authentic and ours with `Disk.Bin` +
`Tunnels.Dsk` mounted. Two divergences, found in order:
1. **The load call.** ToD builds a PAB naming `DSK1.QUEST` and `CALL`s
   interconnect vector `>0010`. Authentic `>0010 = BR >03DC` → the console's
   **DSRLNK** GPL wrapper (parse device name → `XML >19`/`>1A` into the kept ROM,
   which does the CRU device search + `BLWP` into the disk DSR). Ours left the
   interconnect table `>0010-0037` **zero** → executed as `RTN`/garbage → crash.
2. **The stall.** After adding the table + DSRLNK the corruption vanished and the
   DSR *engaged* but read **0** sectors (authentic: 53). A wholesale scratchpad
   diff showed **`>8370` (top of free VRAM) = `0000` vs authentic `>37D7`** — our
   boot never ran the **peripheral DSR power-up**, so the disk DSR's VRAM buffer
   was never reserved (`docs/STATUS.md` "disk-title blocker" documents that the
   DSR power-up lowers `>8370` to `>37D7`). Poking `>8370` alone did *not* fix it.

**Root cause.** Our GROM shipped none of the console **device-I/O layer**
(LIMITATIONS L6): (a) the interconnect table + DSRLNK a loader `CALL`s, and
(b) the boot-time peripheral power-up scan that initialises each card's DSR.

**Fix (`grom/console.gpl`).** (a) `>0010-0037` = twenty executable `BR` stubs;
`>0010 → DSRLNK` (our original routine at `>1200`, byte-verified equivalent to
authentic `>03DC`, delegating to the ROM via `XML >19`/`>1A`); `>0020 → START`;
rest → clean `RTN`. (b) In `START`, before the key-wait, an inline **power-up
scan** (`>8370 := >3FFF`; `>836D := >04`; loop `XML >19`/`>1A` until `>83D0`
clears) — reproduces the authentic boot's inline scan at `>0183`, which runs the
disk card's power-up (reserving the VRAM buffer). Both reimplemented clean-room
from the traced interface (RECON "Console device I/O"), never TI's bytes.

**Payoff.** ToD loads QUEST from disk and reaches `GAME SELECTION: NEW DUNGEON`
under our GROM — identical to authentic (53 sectors read, ISR alive). Guard:
`tests/device_io.rs::tunnels_of_doom_loads_quest_scenario_from_disk_on_our_grom`.
The disk-I/O path of L6 is resolved; on-demand GPLLNK library routines remain.

**Lesson.** Device I/O is **two** console subsystems, not one: the DSRLNK a
loader calls *and* the boot's peripheral power-up that initialises the card. A
routine that "engages but does nothing" points upstream — to missing *init*, not
the routine itself; the wholesale scratchpad diff (guide §4.1) named the cell
(`>8370`) in one run. The heavy lifting (CRU scan, `BLWP` into the DSR) lives in
the **kept ROM** behind `XML >19`/`>1A`; our GROM only supplies the GPL wrappers
that invoke it — so "reimplement DSRLNK" was ~40 lines, not the whole linker.

## Case study 3 — "TI Invaders text doesn't draw" → console char-set loaders (2026-07, fixed)

**Symptom (Joel).** Pressing 2 launches TI Invaders and reaches the opening
screen, but the **text doesn't draw** under our GROM — the sprites are fine. It
renders correctly under the authentic GROM.

**Trace (`invaders_probe`, the user-session/glyph-coverage method, guide §4.8).**
Drove both GROMs to the opening screen and read the screen back. The **name
table was identical** under both (the cart writes the same character codes) — so
the codes were right; the pixels weren't. A rendered non-backdrop pixel count
made it objective: authentic **3785**, ours **354**. Per-code glyph coverage
named the cause: of the 72 codes on screen, **58 had all-zero (blank) glyphs
under ours vs 4 under authentic** — the text codes (the cart maps text to
`ASCII+>50` and `ASCII+>90`) had no patterns loaded. The console-code call chain
after launch showed the cart `CALL`ing interconnect slots **`>0016` and `>0018`**
(the closed R3 histogram had noted carts *read* these) — under authentic they
`BR >0396`/`>039E` and run for thousands of fetches loading fonts; under ours
they hit the `RTN` stub and returned instantly.

**Root cause.** `>0396`/`>039E` are the console's two **character-set loaders**:
a cart points `>834A` at a VDP pattern-table address and CALLs `>0016` (standard
set, from GROM `>04B4`) or `>0018` (thin "small" set, from GROM `>06B4`) to fill
64 glyphs. Our interconnect table stubbed both to `ILRTN` (a bare `RTN`), so no
font loaded and every text glyph stayed blank while cart-defined sprites showed.

**Fix (`grom/console.gpl` + `font.rs`).** Wire `>0016 → LDCSET` (a single
`MOVE >0200, G@FONT, V@*>834A` — our `FONT` *is* the `>04B4` set) and `>0018 →
LDTSET` (`MOVE >0200, G@FONT2, V@*>834A` then `DADD @>834A,>0200`). Added the
thin set as `font.rs::THIN_GLYPHS` (7 rows/glyph, byte-identical to `>06B4`),
spliced **pre-expanded** to 8 rows/glyph at GROM `>1800` so one flat `MOVE`
reproduces the authentic per-glyph loop's VRAM byte-for-byte; the trailing
`DADD` matches `>039E`'s destination-advance side effect. Interface in RECON
"Console character-set loaders"; reproduced clean-room, no TI GPL copied.

**Payoff.** Our GROM's TI Invaders opening screen is now pixel-identical to
authentic (non-backdrop pixels 3787 vs 3785; zero on-screen codes blank that
authentic draws). Guard: `tests/char_set.rs`
(`ti_invaders_opening_text_glyphs_load_on_our_grom`,
`char_set_loader_slots_are_wired`).

**Lesson.** "Text doesn't draw" with a *correct name table* is a **pattern-table
(font) bug, not a layout bug** — read pixels, not codes (the ASCII name-table
dump looks identical either way; a rendered pixel count and per-code glyph-
presence expose it). And a GROM cart's text often comes from **console** font-
load utilities reached through the interconnect table, not the cart's own code —
so an unimplemented console utility shows up as *missing glyphs*, downstream and
far from the stub that caused it.

## Case study 4 — "TI Invaders keyboard dead during gameplay" → missing keytab blocks (2026-07, fixed)

**Symptom (Joel).** Under our GROM the keyboard works to select the level and
enter gameplay, but **during actual play nothing responds** — no move, no shoot.
Under the authentic GROM the arrow keys move and fire the ship.

**Trace (`fctn_probe` then `invaders_play_probe`, differential-trace + GROM read
log).** The tell was *what worked*: menu and level select use **number keys**,
gameplay uses **arrow keys**. `fctn_probe` pressed FCTN+S/D/E/X on both GROMs and
logged the GROM address the ROM's `SCAN` read: authentic and ours both read
`>176A/>1772/>1771/>1768`, but ours returned `>00` there while authentic returned
`>08/>09/>0B/>0A` (the arrow codes). Driving into gameplay and logging keytab
reads while holding a direction showed the **in-game** path is different again:
`SCAN` runs in **key-unit 1/2** (mode cell `>8374` = 1/2, what joystick games set
during play) and reads a table at **`>17C8`** (LEFT→`>17CA`, RIGHT→`>17D2`,
UP→`>17D1`, DOWN→`>17C8`, JOY-FIRE→`>17E9`) — all `>00` under ours.

**Root cause.** The console ROM's `SCAN` (which we keep) looks up **four** 48-byte
ASCII blocks at `>1705/>1735/>1765/>1795` (unshifted/shifted/**FCTN**/CTRL) plus a
40-byte **joystick / split-keyboard** table at `>17C8`. Our `keymap.rs` emitted
only the first two blocks (ended at `>1760`), so everything from the FCTN block
on was zero-filled up to `FONT2` at `>1800`. Mode-0 arrows (title/editor nav) and
mode-1/2 arrows+fire (gameplay) both decoded to `>00`. Menu/level-select survived
because their number keys live in the unshifted block, which *was* present.

**Fix (`keymap.rs`).** Emit the full `>1700..>17EF` region: all four ASCII blocks
(FCTN carries the arrow/edit codes `>01..>0F` and the printed symbol legends; CTRL
is `>80+n` for letters) and the `>17C8` joystick table (two palindromic halves the
ROM walks from both ends). Values reconstructed from the documented TI-99/4A key
codes, not copied. Regenerated `grom/console-grom.bin`.

**Payoff.** The rebuilt image is byte-identical to authentic across `>1760..>17EF`
(the only remaining keytab diff vs. authentic is the pre-existing choice to store
unshifted letters uppercase — since eliminated: flipped to authentic lowercase
2026-07-06). Mode-0 FCTN arrows now decode `>08/>09/>0B/>0A`;
key-unit-1 SCAN returns live direction/fire codes instead of `>00`. Guards:
`tests/keyboard.rs` (`mode0_fctn_decodes_the_arrow_keys`,
`joystick_mode_needs_the_17c8_table`) and the `keymap` unit tests.

**Lesson.** "Some keys work, others are dead" is a **keytab-coverage** bug, not a
scan-engine bug — split it by *which block* each key lives in (number keys =
unshifted; arrows = FCTN; in-game arrows = the `>17C8` joystick table). The ROM's
`SCAN` reaches into **fixed GROM addresses** that a rewrite must fill; the failure
is silent (`>00`), downstream, and only on the keys whose block you omitted. When
menu input works but gameplay doesn't, check the *mode cell* `>8374` — games
switch to key-unit 1/2 and a whole different table.

## Case study 5 — "TI Invaders joystick isn't wired up" → missing deflection table (2026-07, fixed)

**Symptom (Joel).** After case study 4 the **keyboard** plays TI Invaders, but the
**arrow keys don't** — and the frontend maps the host arrows to *joystick 1*
(`crates/libre99-app/src/input.rs`: arrows → `Joy1{Up,Down,Left,Right}`, Right-Alt →
`Joy1Fire`). "The joystick isn't wired up right."

**Trace (`joy_column_trace`, `joy_gromtrace`, `joy_gameplay_probe`).** First ruled
out the CRU layer: stepping a key-unit-1 `SCAN` loop and histogramming the 9901's
selected column showed the ROM *does* select **column 6** for joystick 1 (and
parks there when a Joy1 key is held) — our matrix column is right, the joystick
reaches the ROM. Then the discriminator: in live gameplay (the wave only starts
once **fire** is pressed — before that sprites are frozen, which had masked every
earlier movement probe), holding Joy1Left set `JOYX>8377=>FC` under authentic but
**nothing** under ours, while the *keyboard* arrow (FCTN+S) set `KEY>8375=>02`
identically under both. So the keyboard path (→ `KEY`, via `>17C8`) worked; the
joystick path (→ `JOYX/JOYY`) was dead. `joy_gromtrace` logged every GROM address
`SCAN` reads per direction: holding a joystick direction reads a pair of bytes in
**`>16EA..>16FF`** (Left→`>16FD`, Right→`>16FB`, Up→`>16EE`, Down→`>16F6`) — all
`>00` under ours.

**Root cause.** Below the keytab, at GROM **`>16EA`**, sits a 22-byte joystick
**deflection** table: 11 `(Y, X)` pairs of signed deflections `+4/0/-4`
(`>04/>00/>FC`), indexed by `vert{up:0,down:4,none:8} + horiz{right:0,left:1,none:2}`.
`SCAN` in key-units 1–2 reads the joystick column, forms that index, and copies the
pair into `JOYY/JOYX`. Our rewrite left `>16EA..>16FF` in the zero gap between the
logo (`>1600`) and the keytab (`>1700`), so every joystick direction deflected by
zero — "wired but frozen". The keyboard arrows were unaffected because they take
the separate `>17C8` → `KEY` path.

**Fix (`keymap.rs`).** Emit the deflection table (`GROM >16EA`) ahead of the keytab,
reconstructed from the deflection spec above. Regenerated `grom/console-grom.bin`
(now byte-identical to authentic at `>16EA..>16FF`).

**Payoff.** Joystick directions now set `JOYX/JOYY` exactly as authentic
(Left `>FC`, Right `>04`, Up `>04`, Down `>FC`); in live gameplay the ship tracks
both the joystick and the keyboard arrows, matching authentic ship-X moves. Guards:
`tests/keyboard.rs::joystick_directions_produce_deflection` and
`keymap::joystick_deflection_table_lands_at_16ea`.

**Lesson.** The joystick and the keyboard arrows are **two different decode paths**
even in the same scan mode: arrows → `>17C8` → `KEY`, joystick → `>16EA` deflection
table → `JOYX/JOYY`. "Keyboard works but joystick doesn't" (or vice-versa) means
one table is missing, not a wiring fault — confirm the CRU column first (histogram
it), then diff the *result cells* (`>8375` vs `>8376/>8377`) to see which path is
dead. And driving a real-time game needs the game actually **running**: TI Invaders
freezes every sprite until fire launches the wave, so "nothing moves" proved
nothing until fire was in the script.

## Case study 6 — "F5 reset leaves game graphics on the title screen" (2026-07, fixed)

**Symptom (Joel).** After playing TI Invaders, pressing **F5** (reset) returns to
the title screen but with **graphic artifacts** from whatever was on screen before
the reset.

**Key fact.** F5 → `crates/libre99-app/src/app.rs` `KeyCode::F5 => self.machine.reset()`,
and `Machine::reset()` re-runs only the **CPU** reset sequence — it does **not**
touch VRAM. On real hardware reset behaves the same way; it is the GROM **boot**
that repaints (and thereby clears) the screen. So any cell the boot doesn't write
keeps the previous program's content.

**Trace (`reset_artifacts_probe`).** Drove both GROMs into TI Invaders gameplay,
then `m.reset()` and re-booted, and dumped the name table + sprite attribute table.
Authentic came back clean: name table = only the 274 title cells, **row 23 all
spaces**, sprite table zeroed (Y=`>00`, colour 0 = transparent → invisible). Ours
came back **dirty**: **759** non-space cells (row 23 still held Invaders tiles
`85 86…87` with `>FF` borders) and **13 live invader sprites** (entry 0 Y=`>6D`).

**Root cause.** `START` (the title boot in `console.gpl`) draws only the *specific*
title cells — the colour bars, emblem, banner, "PRESS ANY KEY", copyright — and
never clears the rest of the screen. On a **cold** boot VRAM is all zero, so the
gaps are blank and the title looks clean; on a **reset from a running cartridge**
the name table still holds that cart's tiles and the sprite attribute table its
sprites, which show through the gaps. (Contrast `MENU`, which already opens with
`ALL >20`.) The authentic GROM repaints the full screen: name table → spaces,
sprites neutralised.

**Fix (`console.gpl` `START`).** Before drawing the title — while the display is
still off (`R1=>A0`) — clear the whole screen: `ALL >20` (name table → spaces) and
`ST V@>0300,>D0` (write the sprite-list terminator to entry 0, which our VDP honors
at `vdp.rs` `render_sprites`, so no sprites are drawn). Regenerated
`grom/console-grom.bin`.

**Payoff.** After a reset-from-gameplay ours now matches authentic: 0 leftover
cells, row 23 all spaces, sprite list terminated. Guard:
`tests/title_screen.rs::reset_from_a_dirty_screen_repaints_it_clean` (dirties the
name table + a live sprite through the VDP ports, resets, asserts the screen comes
back clean and the title still renders).

**Lesson.** `reset()` is CPU-only by design — VRAM (and everything else the boot
doesn't re-init) persists across it, exactly as on hardware. Any boot/title code
must therefore **repaint the entire screen**, not just its own cells: clear the
name table *and* neutralise sprites. A bug that is invisible on a cold boot (blank
VRAM) but appears on reset is the tell — differential-trace the *reset* path, not
just the boot.

## Case study 7 — "F5: no fun beep and can't play TI Invaders" → stale sound on reset (2026-07, fixed)

**Symptom (Joel).** After my Chunk 1 GROM changes, pressing **F5** to restart gave
**no power-on beep** and it seemed like the menu was unresponsive to pressing **2**
to launch TI Invaders.

**First move — don't trust the attribution; diff the artifact.** The report blamed
a recent commit, so the first question was "did that commit change the code path?"
`cmp -l` on the old vs new `console-grom.bin` showed every changed byte was **data**
(fonts at `>04B4`/`>06B4`, a beep at `>0484`, FONT2 `>1800`→`>4000`); the console
**code** — `START`, the ISR arming, the menu, dispatch, `SND`/`KBEEP`, `SBWAIT` —
was **byte-identical**. So the menu/beep/ISR path could not have changed. A repro
harness (`examples/f5_repro`) confirmed it: across cold boot and every F5 depth,
the ISR runs, the menu lists entry 2, and pressing 2 launches with sound. The
"can't press 2" half did not reproduce — the ISR is alive, so `SBWAIT` drains.

**The real bug — the audio analog of case study 6.** `reset()` is CPU-only, so
just as VRAM survives a reset (case study 6), the **SN76489 keeps its latches**.
A game playing a multi-channel tune at F5 keeps **droning on channels 1-3** over
our title, drowning the power-on beep — "no fun beep." Differential probe
(`examples/reset_psg_probe`): inject a loud 3-channel tone, `reset()`, run the
boot, sample the PSG. Authentic came back **clean** (ch0-3 all muted); ours left
**channels 1-3 audible**. The authentic power-on beep list at GROM `>0484` opens
`06 BF DF FF …` — its first block **mutes generators 1-3** before the beep; our
`SND` opened straight at channel 0 with no mutes.

**Fix (`console.gpl` `SND`).** Extend `SND`'s first block with the three mute
bytes, matching the authentic list: `BYTE >06,>BF,>DF,>FF,>80,>05,>92,>28`. Every
reset now silences stale generators while the channel-0 beep still sounds.
Regenerated `grom/console-grom.bin`. Guard:
`tests/interrupts.rs::reset_mutes_stale_sound_channels`.

**Lesson.** Same tell as case study 6, one chip over: *reset persists chip state*
— PSG latches, like VRAM, survive `reset()`, so the boot must actively re-init
them. And when a report pins a bug on a specific change, **diff the change first**:
here the changed bytes were provably off the failing path, which redirected the
hunt from "what did I break" to "what does reset fail to clean" — a real,
pre-existing gap (QUALITY-ASSESSMENT §5 item 5) the change merely surfaced.

## Case study 8 — "runaway beep / can't launch a menu pick" → GROM-0 splice clobber (2026-07, fixed)

**Symptom.** While adding the L2 far-list scan code to `console.gpl`, the menu's
key-beep stopped terminating: pressing a valid program number left the console on
the menu, never launching. The far-list gate failed at *launch*, not listing.

**Trace (differential, authentic vs. ours — the house method).** `menu_farlist_probe`
showed the selection logic was fine: `KEY`=`>32` seen, in range, the right
`{KIND,ENTRY}` loaded. It hung one step later in `SBWAIT` — the menu arms `KBEEP`
and polls the sound-bytes-remaining cell `>83CE` until the ISR drains it, and
`>83CE` never reached 0. The ISR was **alive** (`>8379` advancing), but the sound
pointer `>83CC` had run **past `KBEEP`'s terminator** into `>20CA` — the ISR was
streaming random GROM bytes as sound blocks. A control cart (near-list
HuntTheWumpus) showed the *same* runaway, so it wasn't far-list-specific — and the
`interrupts` sound gates were now red too. My change had corrupted `KBEEP` for
*every* cart.

**Root cause — silent last-write-wins overlap.** `system_grom::console_gpl_source()`
lays the whole `console.gpl` (code, then the menu data incl. `SND`/`KBEEP`) from
`>0060`, **then** splices the beep (`>0484`) and fonts (`>04B4`, `>06B4`, `>1000`)
on top. GROM 0 is a packed 6 KiB chip; the data block sat at `>0435`/`>0440` with
only ~60 bytes of slack below `>0484`. The added scan code pushed the data past
`>0484`, and the splices **overwrote `SND`/`KBEEP` with font bytes** — the
assembler allows overlapping blocks (later write wins) with no error. Proof: a
byte-search for the `KBEEP` pattern found it at `>0440` in the committed image and
**absent** in the broken build.

**Fix.** Relocate the menu data block to the free GROM-0 gap above the thin font
(`GROM >0880`, below `FONT` at `>1000`), giving the code ~1 KB of headroom. The
`census` map-completeness gate then flagged the now-zero `>0406..>0438` (authentic
code our shorter layout no longer overlays) — classified `CODE-REPLACED` in
`SURFACE-MAP.md`'s addendum (the gate working as designed).

**Lesson.** In a packed GROM, **growing code silently eats into spliced data** —
the toolchain won't warn. Two cheap tripwires caught/located it: the byte-pattern
search (`SND`/`KBEEP` present at their expected address?) and running the existing
sound gates *before* assuming a new feature is done. When a beep "won't stop,"
suspect the *sound list bytes*, not the ISR — check `>83CC` hasn't run past its
terminator. See LIMITATIONS L2.

## Case study 9 — "F5, then the menu can't launch anything" → the `IO` list's hidden 4th field (2026-07, fixed)

**Symptom (Joel).** Play TI Invaders, press **F5**, pass the title with a key —
then pressing **2** on the menu does nothing (and there was no post-F5 power-on
beep). This is the second half of the report behind case study 7, which fixed
the beep (stale PSG latches) and recorded the press-2 half as **not
reproducing**. It reproduces — but only when the game's scratchpad state at
F5-time is unlucky; the earlier harness "played" too briefly and drew a lucky
layout. Class: boot state-contract drift (QUALITY-ASSESSMENT R3), the exact
cold-boot-zeros blindness RECON R1 warns about.

**Repro + first divergence (`f5_press2_probe`, libre99-core).** Runs cold and
F5-from-playing under the **committed** artifact and under authentic. Our warm
leg: `9901int=false` at menu-ready (VDP pending, R1=`>E0`), `>83CE` stuck at
`>01` (the title-exit click never drained), and the post-'2' fetch histogram
parked in one 5-byte loop at `>032E–0332` — **`SBWAIT`**, polling `>83CE`
forever. Cold and both authentic legs: fine. So the menu logic was innocent;
the ISR was dead — the question became *who disarmed the 9901*.

**Localize (`f5_mask_bisect`).** Frame-level: the mask flips true→false at
frame 10 after F5 (the boot is still in its slow title MOVEs until then).
Instruction-level: the flip lands at **PC `>0606` — the console ROM's `IO`
CRU-output handler — with R12 `>0004` (CRU bit 2) and GPL PC ≈ `>0113`: our
own arming `IO` at GROM `>010F`.** A cell-watch pass killed the obvious
theory: `>8300` (the data byte) was `>FF` when the `IO` executed — yet the
write cleared the bit.

**Root cause — the CRU-output list has FOUR fields.** The ROM reads
`{ address word, count byte, and a data-ADDRESS byte at list+3 (`>8305`) — a
`>83xx` offset it fetches the output bit *through* }` (Nouspikel gpl2 agrees;
execution confirms). RECON §11 had modeled three fields with the data at
`>8300` — true on every cold boot only because zeroed RAM left the pointer
`>00`. After F5 (a CPU-only reset; RAM survives) TI Invaders' leftover
`>8305 = >80` pointed the fetch at `>8380` — a stale even byte, bit 0 = 0 —
so the arming instruction executed `SBZ` and **disarmed the interrupt it was
enabling**. Boot order then explains every symptom: `SND` is armed *after* the
`IO` (hence no post-F5 beep), `TREL`'s click never drains, and the first valid
menu selection parks in `SBWAIT` forever.

**Fix (`console.gpl` `START`).** Write the whole list: `DST @>8304,>0100`
(count 1 **and** data-address `>00`) replaces `ST @>8304,>01`. One
instruction. Artifact rebuilt; RECON §11 corrected; probes kept. **Gate:**
`tests/f5_reset.rs` (cold launch → play → F5 → mask still armed, click drains,
press-2 relaunches; red before the fix, green after) — plus all four
`f5_press2_probe` legs OK.

**Lesson.** Two, both general (register §9): cold-boot zeros are camouflage —
every field of every structure handed to the ROM must be written *every*
boot, and **F5-from-real-gameplay is the adversarial state generator** that
exposes what the zeros hid. And **"did not reproduce" describes the harness,
not the bug**: a state-dependent report stays open until the failing state
itself is reproduced.

## Case study 10 — a "hardening" change that regressed Parsec → the coverage sweep caught it (2026-07, fixed)

**What happened.** The loud-stub grid (§5 item 7) filled the GPLLNK service tail
`>004A-005F` — which was zero bytes — with `B SVCBAD` stubs that **rebooted** to our
title, on the plan's premise that "a cart CALLing into the zero tail runs zeros = a
dangerous silent hole." All gates were green (the sweep's *listing* check and the
per-cart *launch* check both pass — they don't drive deep gameplay).

**What the coverage sweep found.** `tests/coverage_sweep.rs` (new, Chunk 2) drives
every cart through launch + attract + input + F5, recording GROM reads via the
coverage bitmap. It flagged that **16 carts reach `SVCBAD`** — and a differential
probe (launch each under the pre-grid GROM vs the new one) showed **Parsec *ran*
under the old zero tail but *rebooted to our title mid-game* under the new one.** A
regression, in a marquee cart, introduced by a "hardening" change and invisible to
the existing gates.

**Root cause.** GPL `RTN` is opcode `>00`, so the "dangerous" zero tail was already
a **graceful no-op RTN** — and 16 carts CALL an unimplemented service and *rely* on
that (they carry on). Rebooting broke exactly them. The premise was wrong; execution
corrected it. **Fix:** `SVCBAD` RTNs (leaves a breadcrumb, does not reboot).

**Two traps to register (§9).** (1) **A green suite is not a safe change** when the
gates stop where the behaviour starts — listing/launch checks never exercised the
deep path the stub changed; the coverage sweep (deep, all-cart) is what caught it.
(2) **Beware "make the silent case loud" when the silent case is load-bearing** —
`>00`=RTN was a feature carts depend on, not a bug. When tightening a stub, first
ask *who currently relies on the lax behaviour* (a corpus sweep answers it). (3)
Reboot-vs-title detection must key on a **unique** marker (`JOEL ODOM`), not
`TEXAS INSTRUMENTS`, which appears in carts' own screens (3 false positives until
tightened).
