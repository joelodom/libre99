# Project Status

Where the project stands, at a glance. Kept current: update this file in the
same commit as any change that lands or retires a milestone-sized piece of
work (see the documentation policy in [DEVELOPMENT.md](DEVELOPMENT.md)).

_Last updated: 2026-07-07 — **0.1.0 is ready to tag**: the workspace is at
version **0.1.0** (one number for the emulator, the firmware markers, and TI
PYTHON's banner; [CHANGELOG](../CHANGELOG.md)), the `Esc`/`F1` help was
revamped with a first-run **`PRESS ESC FOR HELP`** banner and a `--version`
flag, and the docs swept for release. Earlier the same day: **Extended BASIC
runs on the clean-room firmware** (the XB substrate: five census-pinned ROM
helpers; [XB-CENSUS](../original-content/system-roms/XB-CENSUS.md)), **TI
PYTHON grew to v1** ([spec](TI-PYTHON.md)), the 137-cart health panel reached
**zero waivers**, save states finished (the automatic **resume state**
`resume.ti99` + user-named **snapshots**, `Shift`+`F5` fresh start, atomic
writes, portable format v3), and live disk mount/eject + the disk-persistence
model landed (host `.dsk` never written; in-memory images + `F4`
export/unload). 2026-07-06: **this repository is the IP-clean fork** — born
from a snapshot of the (discontinued) private predecessor's tree, history
clean from commit 1 — the project rebranded **Libre99** throughout, and the
clean-room firmware boots by default._

## The emulator

| Piece | Status |
|---|---|
| TMS9900 CPU (all instructions, flags, interrupts, cycle-aware timing) | ✅ complete, conformance-tested |
| TMC0430 GROM array (prefetch, destructive address read, slot wrap) | ✅ complete |
| TMS9918A VDP (all modes, sprites, beam-accurate scanline rendering) | ✅ complete |
| TMS9901 + CRU + keyboard matrix | ✅ complete |
| Cartridge loader (`.ctg`, byte-exact across a 137-image test corpus) + bank switching | ✅ complete |
| TI Disk Controller (FD1771, clean-room DSR by default, VIB-aware geometry) | ✅ complete |
| SN76489 PSG + host audio | ✅ complete |
| Desktop app (window, input, audio, overlays, file chooser, config, logging, save states) | ✅ complete, playable |
| Media model: **zero embedded media** — `.ctg`/`.dsk` load at run time (CLI paths / `F9` system file chooser) | ✅ complete (2026-07-06) |
| Disk persistence: host `.dsk` never written — live mount/eject, in-memory images survive eject/remount and save states, `F4` export/unload | ✅ complete (2026-07-07) |
| Save states: resume state (auto-save/resume, `F6`/`F8`, `Shift`+`F5` fresh start) + snapshot files (`Shift`+`F6`/`F8`, native dialogs), atomic writes, portable format v3 | ✅ complete (2026-07-07) |
| macOS `.app` bundle (packaging) | ⬜ open — run via `cargo run` for now |

The four historical validation gates all pass as integration tests: boot to
the master title screen, Tunnels of Doom listed on the selection menu, QUEST
loaded from disk by the genuine DSR, and the disk-boot title regression.

## The clean-room firmware (Libre99)

| Piece | Status |
|---|---|
| Console GROM (title, menu, TI PYTHON, system info, GPLLNK services) | ✅ complete (M0–M7), 137/137 carts list & launch |
| Console ROM (kernel, GPL interpreter, KSCAN, ISR, DSRLNK, FMT, floating point) | ✅ complete (M1–M5, M7, M8), differentially verified |
| **TI PYTHON v1** — the Python-like mini-language in TI BASIC's menu slot ([spec](TI-PYTHON.md)) | ✅ complete (2026-07-07): full-size names, Python floor `/`·`//`·`%`, `print(…)`, `#` comments, `exit()`, scrolling screen, cursor, and the new-key input engine — 12 gates |
| **Extended BASIC on the clean-room pair** — the XB substrate (five census-pinned ROM helpers, [XB-CENSUS](../original-content/system-roms/XB-CENSUS.md)) | ✅ complete (2026-07-07): user-supplied XB boots, `PRINT`s, computes floats, stores/`RUN`s/`LIST`s programs — differential gates |
| TI BASIC proper (ROM M6 interpreter half + the console GPL library) | ⬜ deferred indefinitely by policy — TI BASIC itself needs the authentic firmware |
| Boot default | ✅ clean-room ROM + GROM boot **by default** (2026-07-06); user-supplied authentic TI images selected via `--system-rom` / `--system-grom` |

The 137-cart differential health panel passes with **zero waivers** since
2026-07-07 — the former Video Vegas exception cleared incidentally with the
XB substrate ([LIMITATIONS L8](../original-content/system-roms/LIMITATIONS.md);
a gameplay eyeball remains the final confirmation). Detail and evidence:
[original-content/system-roms/STATUS.md](../original-content/system-roms/STATUS.md)
and [rom/README.md](../original-content/system-roms/rom/README.md).

## The toolchain and original content

| Piece | Status |
|---|---|
| `libre99asm` — TMS9900 assembler, `.ctg` packager, disassembler | ✅ complete (all 69 base opcodes; [guide](../assembler/ASSEMBLER.md)) |
| `libre99gpl` — GPL assembler/decoder/disassembler + GROM build | ✅ complete |
| Titris — original cartridge, source → own assembler → boots in own emulator | ✅ complete, playable, gameplay-tested |
| Sokoban — second original cartridge (12 credited Microban levels, undo, flood-filled floors) | ✅ complete, playable; the test suite plays every level to completion |
| Jaywalk — third original cartridge: endless hopper working 24 sprites + all four PSG voices | ✅ complete, playable, gameplay-tested (incl. a 5,000-frame input soak) |
| *Programming the TI-99/4A* (book manuscript) | 🔄 in progress ([docs/ti99book](ti99book/README.md)) |

## Health

- `cargo test --workspace` green — **500+ tests** across the four crates
  (CPU conformance, chip semantics, boot/cartridge/disk integration gates,
  firmware differential suites, the 137-cartridge sweep, save-state
  round-trips, frontend logic).
- `cargo clippy --workspace` clean; CI runs tests + clippy on Windows and
  macOS (`.github/workflows/ci.yml`).
- Committed firmware binaries are gate-checked against fresh builds from
  source, so a stale `console-rom.bin`/`console-grom.bin` fails the suite.

## What's next

Feature direction lives in [ROADMAP.md](ROADMAP.md). **Every engineering row
of the
[Road to 0.1.0](ROADMAP.md#road-to-010--the-first-public-release-early-testing)
has landed** — what remains is the owner's ship sequence: the hands-on
testing pass (plus the two pending gameplay eyeballs — Parsec's small-caps
prompt, a Video Vegas play-through), `git tag v0.1.0`, and prebuilt
Windows/macOS binaries on a GitHub Release. Post-0.1.0, the near-term
**[next]** items are the macOS `.app` bundle, key/joystick remapping, the
authentic lowercase keytab (then the alpha-lock host toggle). User-visible
quirks and open issues are tracked in [KNOWN-ISSUES.md](KNOWN-ISSUES.md);
firmware-rewrite limitations in
[LIMITATIONS.md](../original-content/system-roms/LIMITATIONS.md).

## The record

The engineering history — the original implementation plan with its hardware
citations, the deep root-cause write-ups of the boot-era bugs, the 2026-07-05
whole-project quality evaluation and its executed remediation plan, and the
assembler's bootstrap plan — is preserved in [docs/history/](history/).
