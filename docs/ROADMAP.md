# Libre99 — Feature Roadmap

This document lays out where the emulator can go after the core machine is
faithful and playable (milestones 0–9 in the archived
[build plan](history/PLAN.md)). It is grounded in
two reference points the project already respects:

* **Classic99** (Tursi) — the gold-standard TI-99/4A emulator, whose *debugger*,
  *disk manager*, and *cartridge handling* are the features TI users reach for
  most. We already consult its source for hardware behavior (see
  [CLAUDE.md](../CLAUDE.md)); this roadmap borrows its *product* ideas too.
* **Modern multi-system emulators** (MAME, RetroArch, Mednafen, the bsnes
  family) — for conveniences that have become table stakes: save-state slots,
  rewind, fast-forward, run-ahead, shaders, input remapping, movie
  recording, and screenshots.

The goal of the roadmap is not only a list of features but a *shape* for adding
them so the codebase stays clean and each capability is independent.

---

## Road to 0.1.0 — the first public release (early testing)

**Status: the engineering is done — 0.1.0 awaits the owner's final hands-on
pass, the tag, and prebuilt binaries.** 0.1.0 is the first source-available
public drop, for early testers, and its governing constraint is met: **ship
only what the project owns** — the pure-Rust emulator, the clean-room
**Libre99** firmware (ROM + GROM + disk DSR), the `libre99asm`/`libre99gpl`
toolchain, and our original cartridges (Titris, Sokoban), with **zero TI or
third-party bytes** in the tree, the binary, or the published history (this
repository was born as the IP-clean fork; its history is clean from
commit 1).

Every release-gating row has landed: the IP severance and public/local test
split, the rename & fork, on-demand media (nothing embedded; CLI paths + the
`F9` system chooser), the clean-room disk DSR by default, live disk mounting
+ the never-touch-your-files persistence model (`F4` export), atomic and
portable save states with snapshots, TI PYTHON v1 + the XB substrate
(Extended BASIC end-to-end on the clean-room pair), the legal-notices
consolidation ([NOTICE.md](../NOTICE.md)), the `Esc`/`F1` help revamp + the
first-run **`PRESS ESC FOR HELP`** banner, the **0.1.0** version bump (with
`--version`), and the [CHANGELOG](../CHANGELOG.md). The full phase-by-phase
record — including every decision of record (save-slot shape, disk-delta
shape, data-dir migration, no-bundled-content, TI PYTHON's target, the
system-roms README wording) — is archived at
[history/ROAD-TO-0.1.0.md](history/ROAD-TO-0.1.0.md).

**What remains — the owner's ship steps:**

1. **Hands-on testing pass** (first run, fresh start, missing dir, bad
   input, no media — the crash/robustness eyeball), plus the two pending
   gameplay eyeballs noted in [KNOWN-ISSUES](KNOWN-ISSUES.md) /
   [LIMITATIONS](../original-content/system-roms/LIMITATIONS.md): Parsec's
   small-caps in-game prompt, and a Video Vegas play-through (L8).
2. **Tag `v0.1.0`** and publish the **GitHub Release** with prebuilt
   Windows + macOS binaries (`cargo build --release` artifacts; the macOS
   `.app` bundle stays a §7 **[next]** item — a plain binary is the 0.1.0
   vehicle).

### Definition of done for 0.1.0

Met, except the last clause — the tag ships it. The fresh **libre99**
repository — history clean back to commit 1 — builds from a clean checkout a
binary named **libre99** that contains **no TI or third-party bytes**, boots
the clean-room firmware, loads cartridges and disks from user-supplied files
(command line + the system file chooser; nothing bundled), reads *and writes*
disks on the clean-room DSR (mounting a disk **without rebooting** the running
console), saves and restores state portably across macOS and Windows, passes a
public CI with no proprietary inventory (green as of 2026-07-07), ships
refreshed docs and in-app help — **and is downloadable as a prebuilt binary
for both platforms (the remaining step)**.

---

## Design principles (so features stay modular and cherry-pickable)

1. **The core stays pure `std`, zero-dependency.** Anything that needs a crate
   (file dialogs, gamepads, image/audio encoders) lives in `libre99-app`. New
   *emulated hardware* (speech, cassette, more RAM) belongs in `libre99-core` and
   must come with unit tests, like every existing chip.
2. **Each feature is a self-contained module.** A feature adds a module
   (`input_layout`, `menu`, `speed`, …) and wires into `app.rs` in one small,
   additive place. That keeps feature commits independent enough to **cherry-pick
   onto `main` individually**.
3. **UI is drawn, not toolkited.** On-screen overlays use the `text::Canvas`
   framework (a bitmap font + rectangle/dim helpers) introduced with the
   keyboard reference. No GUI dependency is required for menus, the debugger
   view, or HUD indicators.
4. **The save-state format is versioned.** `state.rs` carries a magic + version;
   any change to what is serialized bumps the version and stays
   backward-detectable (a foreign/old file is rejected cleanly, never
   mis-read).
5. **Persisted choices go through `config.rs`.** New preferences extend the
   resiliently-parsed TOML (missing/invalid keys fall back to defaults).

---

## Themes & features

Each item is tagged: **[done]** implemented and merged to `main` ·
**[next]** designed and high-priority · **[later]** valuable, larger ·
**[stretch]** ambitious.

> **Clean-room firmware — where BASIC stands (milestone M6).** Since 2026-07-07
> the clean-room rewrite (the default firmware) **runs Extended BASIC
> end-to-end**: the census-first plan in [TI-PYTHON.md §6](TI-PYTHON.md)
> measured XB's real console demand at **five small ROM helpers** (the old
> "whole interpreter's worth of services" model was wrong — no console-GROM
> BASIC library is touched at all), and the **XB substrate** implements them
> at their pinned authentic addresses, differentially gated
> ([XB-CENSUS.md](../original-content/system-roms/XB-CENSUS.md);
> [LIMITATIONS](../original-content/system-roms/LIMITATIONS.md) L9). What
> remains deferred **by policy** is **TI BASIC proper** — the console-resident
> interpreter (TI PYTHON deliberately ships in its menu slot) and the ROM's
> PARSE/EXEC half — plus the L8 Video Vegas GROM-2 routine (no longer on
> XB's critical path). TI BASIC itself still needs the authentic ROMs
> (`--system-rom` / `--system-grom`); an unusual BASIC-family cartridge can
> be measured in one command (`cargo run -p libre99-gpl --example xb_census`).
> **[TI BASIC proper: later — large, policy-gated]**

### 1. Input & control
- **Host keyboard layout translation (QWERTY / Dvorak / …).** Toggle between
  *positional* mapping (host physical key → same TI position, best for games and
  joystick-style control) and *character* mapping (host **logical** key → TI key,
  so a Dvorak or AZERTY typist gets the letters they actually typed). **[done]**
- Full key/joystick **remapping** from the config file. **[next]**
- **Gamepad** support (via a frontend crate such as `gilrs`). **[later]**
- **Paste-to-type**: inject host clipboard text as TI keystrokes. **[later]**

### 2. Media management (Classic99-grade)
- **Media from arbitrary file paths** — `--cartridge <path>` / `--disk <path>`
  on the command line, and the **OS-native file chooser** on `F9` (`rfd`;
  starts in the last-mounted-from directory, remembered in the preferences);
  `F2`/`F3` eject. This **replaced** the earlier embedded-media pickers (the
  on-screen browser and F2/F3/F4 cycling) when the media embeds were removed
  with the third-party IP (2026-07-06; owner call: system chooser over a
  custom browser). Nothing is bundled — the console boots bare. **[done]**
- **Disks mount and eject live — no reboot** *(was: bug, Joel 2026-07-07)*. `F9`
  slots a disk into the **running** machine and `F3` pulls it just as live,
  like a real floppy; only a *cartridge* change cold-boots (the console scans
  cartridge ROM at power-up), and the rebuild carries the disk subsystem across
  intact. **[done]** (2026-07-07)
- **Disk persistence — the source `.dsk` is never written; in-memory images +
  on-demand export.** Writes mutate the machine's in-memory copy, keyed by the
  file's canonical path; ejected images shelve in memory and reattach on
  remount; save states carry all of it (format v2+). The **`F4` disk-memory
  overlay** exports an image via the OS-native save dialog (its replace-prompt
  means no host `.dsk` is ever overwritten unprompted) or unloads one (with a
  native save-first prompt when changed). **[done]** (2026-07-07)
- **Create / format blank disks**; **import/export** files to and from TI disk
  images (TIFILES / FIAD). **[later]**
- **Recently-used** media list; per-title default disk. **[later]**
- **Fast console-menu cartridge scan (fidelity, firmware).** *(bug/roadmap note,
  Joel 2026-07-06; resolved 2026-07-07.)* Our rewritten console GROM menu painted
  a **`SCANNING`** row while it built a cartridge's program list; the **authentic**
  menu is fast and shows **no such word**. Measurement (`tests/perf_parity.rs`)
  **corrected the "~1–2 s" premise**: the isolated menu-build segment is only
  ~7 frames (~0.12 s), and our rewrite already reaches the menu *sooner* than the
  authentic firmware overall (reset → cart listed ~30 vs ~54 frames). The banner
  was the only artifact that read as slow, so it was **removed**, and the build is
  now hidden: `MENU` blanks the display (VDP R1 `>A0`) while it scans and reveals
  the whole list at once (`SDONE`/`DISPON`, the title screen's own idiom) instead
  of painting entries in one at a time (`original-content/system-roms/LIMITATIONS.md`
  **L5**). Speeding the `SCANW` walk further was declined: ~0.12 s is imperceptible
  once hidden, and a window-size change is not worth risking the 137-cart
  enumeration gate. **[done]** (2026-07-07)

### 3. Emulation control
- **Variable speed**: fast-forward / turbo, slow motion, **pause**, and
  **single-frame advance**, with an on-screen state indicator. **[done]**
- **Save states**: the automatic **resume state** (auto-saved on exit, resumed
  at launch, `F6`/`F8` live, `Shift`+`F5` fresh-start delete) plus user-named
  **snapshot** files via the OS-native dialogs (`Shift`+`F6`/`F8`); atomic
  writes, portable format (v3), and **screenshots** (PNG). **[done]** (2026-07-07)
- **Rewind**: a ring buffer of recent save states scrubbed with a key. **[later]**
- **Run-ahead** for lower input latency. **[stretch]**

### 4. Debugging & development (Classic99's debugger is the standout)
- On-screen **debugger overlay**: CPU registers (WP/PC/ST), the workspace, and a
  memory peek window — a live, non-modal panel. **[done]** (live disassembly at
  PC and breakpoints are the natural next step.)
- **Breakpoints**, single-step, and watchpoints driven from the overlay. **[later]**
- **VDP inspector**: pattern/sprite/name-table and palette viewers. **[later]**
- **GROM/GPL trace** and a memory editor. **[later]**
- **Assembler & cartridge builder** (`libre99asm`, crate `libre99-asm`): a from-scratch,
  Editor/Assembler-compatible TMS9900 assembler that emits bootable `.ctg`
  cartridges, so new software — and AI agents — can author cartridges for the
  emulator end to end. User guide and language reference:
  **[ASSEMBLER.md](../assembler/ASSEMBLER.md)**; pairs with the `libre99-app
  --cartridge-file <path>` flag to close the build-run loop (relates to §2's
  "arbitrary file paths"). **[done]** (full TMS9900 ISA; the playable
  [Titris](../original-content/cartridges/titris/README.md) and
  [Sokoban](../original-content/cartridges/sokoban/README.md) cartridges prove
  the pipeline; the bootstrap record is archived at
  [docs/history/ASSEMBLER-POC-PLAN.md](history/ASSEMBLER-POC-PLAN.md).)

### 5. Video & audio
- **Beam-accurate (scanline) VDP rendering**: each of the 192 active lines is
  rasterized from live VRAM at the moment the beam crosses it, interleaved with
  that line's ~190.84 CPU cycles; the frame interrupt rises at end of active
  display (line 192 of 262) and 5S/coincidence latch per line — so mid-frame
  VRAM writes, flashing text, and screen splits render as on hardware (gates in
  `libre99-core/tests/beam.rs`; the Parsec in-game garble first pinned on this
  turned out to be firmware — the `>004A` lower-case loader, see
  KNOWN-ISSUES — but the beam model is hardware-true and stands). **[done]**
  (2026-07-06 — was QUALITY-EVALUATION §3.2 A1 / the "scanline-stepped VDP
  behind the existing seam" deferral.) Two Classic99 refinements deliberately
  remain:
  - **Sub-instruction status-read catch-up** (Classic99 `Tiemul.cpp:5155`):
    advance the VDP to the exact mid-instruction cycle on status reads, for
    software that polls the F bit with interrupts enabled (fbForth's RNG) —
    our F/5S/C changes are quantized to instruction boundaries. **[later]**
  - **Real-time fifth-sprite-number counting** (Classic99 `tivdp.cpp:2641`):
    when no fifth sprite exists the status low bits count the last sprite
    scanned, in real time; Miner 2049er reads it mid-frame. We latch the
    number only with 5S (P2.3). **[later]**
- **CRT presentation**: aspect correction, selectable smooth scaling, and
  optional scanline/shader *filters* (a look, unrelated to the beam-accurate
  rasterizer above). **[later]**
- Selectable **palettes** (TI, the perceptual Classic99 set, greyscale). **[later]**
- **Screenshot** (PNG) — see §3 — and **GIF/video** capture. **[later]**
- **Audio recording** to WAV. **[later]**
- **F18A** enhanced VDP (extra modes, palette, GPU). **[stretch]**

### 6. Hardware & peripherals
- **TMS5220 speech synthesizer** (the Speech Synthesizer module). **[later]**
- **Cassette (CS1/CS2)** in/out (load/save programs to a `.wav`/sound file).
  **[later]** — *not yet emulated:* `crates/libre99-core/src/cru.rs` leaves the
  cassette motor/level CRU outputs "not wired" and provides no read-data input,
  so the console's cassette DSR can prompt but never transfers a byte. This
  blocks loading a file from **CS1** (e.g. a Tunnels of Doom scenario from tape);
  the **disk (DSK1)** path is the supported alternative. Building this means
  modeling the CRU cassette bits (22/23 motor, 24 audio gate, 25 out, 27 in),
  the 9901 interval timer the tape loops rely on, + a tape image/`.wav` source
  feeding the read-data bit the DSR polls. **Decision (Joel, 2026-07-02):**
  the console-ROM rewrite (decision record in the archived
  `original-content/system-roms/history/ROM-REWRITE-PLAN.md`
  §10.2) ships **interface-correct CS1/CS2 error behavior only** — its ROM-side
  tape transport (bit engines + timer ISR) is deferred until this hardware
  exists; when someone builds this item, commission the ROM transport with it.
- **Alpha-lock switch input.** `cru.rs` models alpha-lock only as the P5 output
  latch; the console ROM reads the physical switch by driving P5 low and testing
  CRU bit 7 (`SBZ 21 / TB 7 / SBO 21` — pinned in
  `original-content/system-roms/rom/RECON.md` §23). With no switch input the
  line idles high ("not locked"), so lowercase-capable keytabs never fold — under
  the authentic ROM and the rewrite alike. Building this: latch a host toggle
  (Caps Lock) as the switch state, return it on CRU bit 7 while the P5 latch is
  low (keyboard row 4 otherwise), and reproduce the real-hardware quirk that an
  engaged alpha lock interferes with joystick-up (Classic99 models it). The ROM
  side is already written and differentially gated (`libre99-gpl/tests/rom_kscan.rs`);
  this item makes the fold functional under both ROMs. **[later]**
  - **Decoupled prerequisite win (do first): ship the authentic lowercase
    keytab.** Extended BASIC currently types **uppercase** because our GROM's
    unshifted keytab (`crates/libre99-gpl/src/keymap.rs`) stores uppercase where the
    real machine stores lowercase (see `docs/KNOWN-ISSUES.md` "Extended BASIC …
    types UPPERCASE"). Because the switchless line already idles "not locked,"
    flipping the table to lowercase makes native-mode typing lowercase (real
    behavior) **with zero new hardware** — the menu still folds to uppercase in
    state 0. The alpha-lock *host toggle* above is only needed to let a user
    *lock* back to uppercase; it is **not** required for lowercase-by-default.
    **[next]**
- Memory options: **SAMS/AMS** expansion beyond the 32K. **[later]**
- Multiple **console ROM revisions**, and the TI-99/4 (vs 4A) keyboard. **[later]**
- More **PEB cards**: RS232/serial, p-code. **[stretch]**

### 7. Quality of life & packaging
- macOS **`.app` bundle** (milestone 10) and an optional **native menu bar** that
  mirrors the in-app overlays. **[next]**
- A **settings overlay** to edit preferences without leaving the app. **[later]**
- **Cheats / poke editor** (RAM patches with a small database). **[later]**
- **Netplay** over the `Bus`/input seam. **[stretch]**

### 8. Assurance & hardening
- **Disk-DSR assurance follow-through.** The clean-room disk DSR shipped as the
  default (2026-07-06) with 24 differential gates green, but several
  instruments from its plan's own definition-of-done remain: the random-PAB
  differential **fuzz** (the highest-value item — corruption protection beyond
  the hand-written gates), parameterizing the pre-existing disk test estate
  over `[TI_DSR, OUR_DSR]`, the all-bundled-disks catalog/read sweep, a TI
  BASIC scripted file-I/O parity gate, the op-surface completeness sweep +
  entry-census tripwire, the perf tripwire, a manual xdt99 round-trip, plus
  source hygiene (mojibake repair in `disk-dsr.asm` + an encoding tripwire)
  and doc sync. **Explicitly not required for 0.1.0** (Joel, 2026-07-06) —
  the DSR's differential evidence already supports daily use; this deepens
  it. Full execution plan for a working session:
  [`original-content/system-roms/disk-dsr/DSR-ASSURANCE-PLAN.md`](../original-content/system-roms/disk-dsr/DSR-ASSURANCE-PLAN.md)
  (its A0 hygiene/doc chunk is cheap and pull-forward-able any time). **[later]**

---

## Status — implemented (merged to `main`)

Each feature below landed as a single, self-contained commit that **builds and
runs on its own** — designed so any one could be cherry-picked independently — and
the slice has since been merged to `main`. This table is updated in the same commit
that lands a feature.

| Feature | Module(s) | Commit |
|---|---|---|
| Roadmap (this document) | `docs/ROADMAP.md` | `a81148c` |
| Host keyboard layout translation (Dvorak/QWERTY) | `input` (`KeyLayout`), `config` | `e698ff8` |
| ~~In-app media browser + metadata~~ (removed with the media embeds — superseded by the `F9` system chooser below) | `menu` (deleted) | `22a644d` |
| Variable speed (turbo / pause / frame-advance) | `speed` | `4bfba1d` |
| Screenshot (built-in PNG encoder) | `screenshot` | `980550b` |
| Save state (single; auto-save on exit + resume at launch) | `app`, `config` | `2ad3d05` |
| Live CPU inspector overlay (registers + memory) | `debug` | `e681a7a` |
| Assembler & cartridge builder — full TMS9900 ISA (`libre99asm`) | `libre99-asm` | `93055a6` |
| Mount a `.ctg` from disk (`--cartridge-file`) | `cli`, `app` (main) | `a034949` |
| Public/local test split — runtime `third-party/` media loading, skip-when-absent | `libre99-core` (`third_party`), all test suites | `e5fef69` |
| Zero embedded media — CLI paths + the system file chooser (`F9`), `F2`/`F3` eject | `media`, `cli`, `app`, `config` | `b41a03e` |
| Live disk mount/eject + in-memory disk persistence, `F4` export/unload (save format v2) | `disk`, `machine`, `state` (core); `disks`, `media`, `app` | `c153aa8` |
| Save states: resume state + snapshots + fresh start, atomic writes, portable format v3 (cartridge identity) | `machine` (core); `app`, `config`, `media`, `help` | `09f8fd8` |
| Extended BASIC on the clean-room firmware — the XB substrate (census instrument + five pinned ROM helpers + differential gates) | `cpu` (PC coverage, core); `rom/console.asm`; `xb_census`, `xb_substrate`, `xb_smoke` | `0e692eb` |
| TI PYTHON P1 — the new-key input engine (fixes dropped keys, backspace, junk echo, unbounded input) | `grom/console.gpl`; `ti_python.rs` | `cbbcdb2` |
| TI PYTHON v1 — the spec'd Python-like mini-language (names, floor math, `print`, comments, `exit()`, scroll, cursor) | `grom/console.gpl`; `ti_python.rs`; spec `docs/TI-PYTHON.md` | `7c2cae9` |
| First-run `PRESS ESC FOR HELP` banner + Esc-first help revamp + `--version` | `app`, `help`, `cli`, `main` | `a1f16b4` |
| Version 0.1.0 across the workspace (firmware markers + TI PYTHON banner track it) + `CHANGELOG.md` | workspace `Cargo.toml`; firmware artifacts | `1dd5c0d` |

> The list above is the *committed* slice of the roadmap; everything tagged
> **[next]/[later]/[stretch]** is future work, captured here so the design intent
> is recorded even where the code isn't written yet.
