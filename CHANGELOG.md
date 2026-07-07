# Changelog

All notable changes to Libre99. The version number is the workspace
`version` in the root `Cargo.toml` — one number shared by the emulator, the
clean-room firmware it embeds, and TI PYTHON's banner.

## Unreleased

- The `Esc`/`F1` **help overlay was redesigned** to the approved four-tab
  "quiet terminal" design (per `design_handoff_help_redesign`): solid black
  backdrop, hairline rules and whitespace instead of cards, a single cyan
  chrome accent (amber/green reserved for the keyboard map's SHIFT/FCTN
  semantics), larger type throughout, and OS-correct shortcut labels. The
  Media & State tab folded into Hotkeys as a "loading & saving" note — four
  tabs (`1`–`4` jump; the footer now also hints `←`/`→`). The embedded fonts
  now interpret point sizes as em sizes (CSS-like), so design-handoff values
  render true; the unused Silkscreen Bold face was dropped.
- **Jaywalker 99**, a third original demonstration cartridge
  (`original-content/cartridges/jaywalker99/`): an endless-hopper arcade game —
  a fledgling blue jay crossing procedurally generated roads, rivers, and
  rail lines — built, like Titris and Sokoban, entirely with the project's
  own assembler and gameplay-tested end to end on the emulated console.
  Where the first two cartridges are character-graphics puzzles, Jaywalker 99
  works the arcade hardware: up to 24 simultaneous 16×16 sprites at
  independent sub-pixel speeds (early-clock edge slide-ins, priority-aware
  four-per-line budgeting), color-table and pattern-table animation (the
  flashing level-crossing signal, the shimmering river), and a table-driven
  driver for all four SN76489 voices. Its regression suite
  (`crates/libre99-asm/tests/jaywalker99.rs`) plays the game headlessly —
  movement, scoring, scrolling, every hazard and death, the hawk, sounds on
  the real PSG, and a 5,000-frame random-input soak.

## 0.1.0 — 2026-07-07 — the first public release (early testing)

The first source-available drop of **Libre99**: a from-scratch TI-99/4A
emulator in pure Rust that **contains and executes no Texas Instruments
bytes** — it boots this project's own clean-room firmware by default, in a
repository whose history has been IP-clean from commit 1.

**The machine** (`libre99-core`, pure `std`, zero dependencies)

- TMS9900 CPU — all instructions, status flags, interrupts, cycle-aware
  timing; conformance-tested.
- TMS9918A VDP — all modes, sprites, **beam-accurate scanline rendering**.
- TMC0430 GROM array, TMS9901 + CRU + keyboard matrix, SN76489 PSG.
- Cartridge loader (`.ctg`, bank switching) — byte-exact across a 137-image
  test corpus.
- TI Disk Controller (FD1771) with a **clean-room disk DSR** that reads *and
  writes* by default.

**The firmware** (clean-room, boots by default)

- Original console ROM + GROM: title screen, selection menu, GPL
  interpreter, KSCAN, ISR, DSRLNK, FMT, floating point — differentially
  verified against the authentic firmware; the 137-cart health panel passes
  with **zero waivers**.
- **TI PYTHON v1** — an original Python-like mini-language in TI BASIC's
  menu slot ([spec](docs/TI-PYTHON.md)).
- **Extended BASIC runs end-to-end** on the clean-room pair (the XB
  substrate). TI BASIC itself is the one thing that still needs
  user-supplied authentic ROMs (`--system-rom` / `--system-grom`).

**The desktop app** (`libre99`)

- Zero embedded media — the console boots bare; mount any `.ctg`/`.dsk` via
  the command line or the OS-native file chooser (`F9`); disks mount and
  eject **live**, no reboot.
- **Disk persistence that never touches your files**: writes stay in
  memory, survive eject/remount and save states, and export on demand
  (`F4`) through the native save dialog.
- **Save states**: the automatic resume state (auto-save on exit, resume at
  launch, `F6`/`F8`) plus user-named snapshot files (`Shift`+`F6`/`F8`) —
  atomic writes, portable format (v3) across Windows and macOS.
- Five-tab help overlay (`Esc`/`F1`) at native resolution, a first-run
  `PRESS ESC FOR HELP` banner, speed control (pause / frame advance /
  fast-forward), PNG screenshots, a live CPU inspector, character and
  positional keyboard mapping, `--version`.

**The toolchain and original content**

- `libre99asm` — a from-scratch, Editor/Assembler-compatible TMS9900
  assembler that emits bootable cartridges ([guide](assembler/ASSEMBLER.md)).
- `libre99gpl` — GPL assembler/decoder/disassembler; builds the console GROM.
- Two original, playable cartridges built by that toolchain: **Titris** and
  **Sokoban**.

**Assurance**

- 500+ tests across four crates; public CI (tests + clippy) green on
  Windows and macOS from a clean checkout, with zero proprietary bytes.
- Authentic-image comparisons run only on development machines, loading
  user-supplied images at run time from the git-ignored `third-party/`
  (tests skip green when absent).
