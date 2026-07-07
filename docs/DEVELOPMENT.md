# Development Guide

How to build, test, and change this project without breaking its guarantees.
Architecture and module maps are in [ARCHITECTURE.md](ARCHITECTURE.md); the
current state of the world is in [STATUS.md](STATUS.md).

---

## Build and test

```bash
cargo test --workspace           # the whole suite (500+ tests, all four crates)
cargo clippy --workspace         # keep it clean — CI enforces -D warnings
cargo run --release -p libre99-app  # run the emulator
```

Useful narrower runs:

```bash
cargo test -p libre99-core          # the emulator core alone (pure std, no deps)
cargo test -p libre99-gpl           # GPL toolchain + console-GROM gates + 137-cart sweep
cargo test -p libre99-asm           # assembler + console-ROM differential gates
```

CI (`.github/workflows/ci.yml`) runs tests and clippy on Windows and macOS.
Repository scripts must be **`sh` + `cargo` only** — the development machines
have no `make` or `python`.

## The rules that keep the project sound

1. **`libre99-core` stays pure `std` with zero third-party dependencies** (and
   `#![forbid(unsafe_code)]`). The workspace split makes this structural: new
   emulated hardware goes in `libre99-core` with unit tests; anything needing a
   crate (windowing, audio, file dialogs, encoders) goes in `libre99-app`. The
   frontend's dependency budget is deliberately small (winit, softbuffer,
   cpal, log, simplelog, toml, rfd — the OS-native file chooser) — one line of
   justification each, recorded in the workspace `Cargo.toml` history and
   [history/PLAN.md](history/PLAN.md) §5.
2. **Classic99 is the reference of record.** A full checkout of Tursi's
   hardware-verified emulator lives at `C:\ClaudeShared\classic99` (PC) /
   `/Users/Shared/classic99` (Mac) — *consult, never copy*. When our behavior
   disagrees with Classic99, Classic99 is almost certainly right: reproduce
   its behavior and add a focused regression test. The most useful files are
   `console/cpu9900.cpp` (status-flag tables), `console/Tiemul.cpp` (bus,
   GROM/VDP port semantics), and `console/tivdp.cpp`.
3. **Every behavior fix ships with a regression test** that flips red→green,
   and hard-won root causes get written down (see
   [history/POSTMORTEMS.md](history/POSTMORTEMS.md) and
   [KNOWN-ISSUES.md](KNOWN-ISSUES.md) for the house style).
4. **Faithfulness beats convenience.** When users report authentic hardware
   behavior as a bug (the ~9-minute screen blank, small-caps lowercase), the
   disposition is never to break emulation — add an opt-in frontend
   preference, default off ([KNOWN-ISSUES.md](KNOWN-ISSUES.md)).
5. **The save-state format is versioned** (`state.rs`, magic + version); any
   change to what is serialized bumps the version. Loads are staged and must
   stay corruption-safe.
6. **Persisted choices go through `config.rs`** and the resilient TOML; new
   user files go under `~/.libre99/` via `config::data_dir()`, never
   elsewhere in `$HOME`.
7. **UI is drawn, not toolkited** — overlays use the `text::Canvas` bitmap
   framework; no GUI dependency.

## The firmware track

The clean-room console ROM/GROM sources live in
[`original-content/system-roms/`](../original-content/system-roms/README.md)
(`rom/console.asm`, `grom/console.gpl`) and are built by `libre99-asm` /
`libre99-gpl`. Rules specific to that work:

- **Rebuild the committed binaries when their source changes** —
  `console-rom.bin` / `console-grom.bin` are what the app embeds. Staleness
  gates (`committed_bin` tests in both toolchain crates) fail the suite if a
  committed binary doesn't match a fresh build.
- **Layout is gated.** Public entry addresses are frozen
  (`libre99-asm/src/system_rom.rs::FROZEN_ENTRIES`); the census and sweep gates
  in `libre99-gpl` bound the compatibility surface. Read
  [system-roms/README.md](../original-content/system-roms/README.md) →
  `STATUS.md` → `LIMITATIONS.md` before touching firmware, and the two
  debugging guides there before chasing a firmware bug.
- **Differential verification runs both firmwares inside this emulator**, so
  an emulator-model bug is invisible to it by construction. The standing
  mitigation plan (Classic99 golden-reference checks, a future MAME romset
  swap) is [CROSS-VALIDATION.md](CROSS-VALIDATION.md).

## Third-party media for tests (`third-party/`, git-ignored)

**The repository tracks zero TI or third-party bytes.** The differential
suites, probe examples, and the book's bench tool need the authentic images to
run — they load them **at run time** via `libre99_core::third_party` from a
**git-ignored** `third-party/` directory at the workspace root (override with
`$LIBRE99_THIRD_PARTY`):

```
third-party/
├─ roms/          994aROM.Bin, 994AGROM.Bin, Disk.Bin, …   (TI firmware)
├─ cartridges/    *.ctg                                    (commercial titles)
├─ disks/         *.Dsk                                    (commercial disks)
└─ Editor_Assembler_Manual.pdf                             (TI's E/A manual, for the book)
```

When an image is absent the affected tests **skip and pass**, printing
`SKIPPED: third-party media not present (…)` — so a fresh public checkout (and
public CI) is green without a single proprietary byte, while a maintainer
machine with the directory populated runs the full differential suite. Probe
examples and the bench tool print what they need and exit instead. Both
project workstations keep this directory populated; never commit it, embed it,
or copy from it into source.

## Documentation policy — keep it fresh

Documentation is part of the change, not an afterthought. **When a change
alters behavior, status, or structure, update the affected documentation in
the same commit**:

- User-visible behavior (flags, hotkeys, config keys, file locations) →
  [USER-GUIDE.md](USER-GUIDE.md) (and the in-app `F1` help, `libre99-app/src/help.rs`).
- Module/crate structure, run-time flow → [ARCHITECTURE.md](ARCHITECTURE.md).
- Milestone-sized progress or retirement → [STATUS.md](STATUS.md); feature
  intent → [ROADMAP.md](ROADMAP.md) (its "implemented" table is updated in the
  same commit that lands a feature).
- New quirk reports / resolutions → [KNOWN-ISSUES.md](KNOWN-ISSUES.md);
  firmware gaps → `original-content/system-roms/LIMITATIONS.md`.
- Assembler language/CLI changes → [assembler/ASSEMBLER.md](../assembler/ASSEMBLER.md).

Completed plans and dated reports are not deleted — they move to
[docs/history/](history/) (project-wide) or
`original-content/system-roms/history/` (firmware track), each archive
carrying a banner naming what superseded it. Screenshots in the README are
regenerated with `cargo run -p libre99-gpl --example readme_gallery` whenever a
pictured surface changes (title screen, menu, TI PYTHON, …).

The book manuscript under `docs/ti99book/` is a separate work-in-progress with
its own conventions (`docs/ti99book/CLAUDE.md`) — don't sweep it into
project-doc refactors.

## Licensing and IP

- The project's original work is licensed **Modified MIT + Commons Clause**
  ([LICENSE.md](../LICENSE.md)). Every Rust source file carries the full
  license text as a `//` header; keep it verbatim on existing files and add it
  to new ones. The firmware sources (`.asm`/`.gpl`) carry a short license
  pointer in their headers. Never reintroduce "all rights reserved" wording —
  it contradicts the grant.
- The clean-room firmware reproduces **interface data** (fonts, key tables,
  headers, dispatch layouts) byte-identically for interoperability, under a
  documented policy — see the provenance sections of
  [system-roms/README.md](../original-content/system-roms/README.md).
- **Third-party material is not ours to license**: TI console/DSR/speech
  firmware and commercial cartridge/disk images are the property of their
  respective owners. They are **not tracked, distributed, or embedded** —
  maintainers keep local copies in the git-ignored `third-party/` directory
  (above) purely so the differential suites can run.

### Pre-public-release checklist (standing)

Before this repository (or any binary) is made public:

1. ✅ **Done 2026-07-06 — the working tree tracks no third-party bytes.**
   `roms/`, `cartridges/`, and `disks/` moved out of version control into the
   git-ignored `third-party/` directory (above); the app embeds only the
   clean-room firmware and loads media at run time; all tests/examples/bench
   load authentic images at run time and skip when absent; the gallery
   generator renders only our titles. The README screenshots of third-party
   titles (`docs/screenshots/parsec.png`, `tunnels-of-doom.png`) are **okay
   to keep** as static images (Joel, 2026-07-06). TI's Editor/Assembler
   manual PDF (which the predecessor repo tracked under `assembler/`) was
   excluded at the fork — it lives in `third-party/` now.
2. The clean-room firmware, TI PYTHON, Titris, Sokoban, and the toolchain are
   original and stay. (Sokoban's twelve levels are from David W. Skinner's
   **Microban** set, which "may be freely distributed provided they remain
   properly credited" — the cartridge's title screen and README carry the
   credit, so the levels stay too.)
3. ✅ **Done 2026-07-06 — the history is clean.** This repository *is* the
   fresh fork ([ROADMAP](ROADMAP.md) Phase 2): it was born from a snapshot of
   the (discontinued, never-published) predecessor's IP-free tree, so no
   commit here has ever contained a proprietary byte.
4. Re-check CI on this repo once pushed: green with zero proprietary bytes.

## Working conventions

- Commit directly to `main` (branch only when explicitly requested), small
  commits, message style `area: what` / `fix(core): what` matching the log.
- Multiple development sessions sometimes run in parallel on this repository.
  Before committing, check `git status` for another session's in-flight work;
  don't commit over a red test loop that isn't yours, and expect the firmware
  staleness gates to be red while a sibling session has source ahead of its
  committed binary.
- Temporary probes belong in `examples/` (the `libre99-gpl` probe inventory in
  [DEBUGGING.md](../original-content/system-roms/DEBUGGING.md)) or scratch
  space, not in `src/`.
- Occasionally worth running: `cargo mutants` against `libre99-core` (output
  directories are gitignored).
