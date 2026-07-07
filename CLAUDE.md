# CLAUDE.md

Guidance for working in this repo — **Libre99**, a from-scratch TI-99/4A
emulator in pure Rust, plus its assembler/GPL toolchain, clean-room console
firmware, and an in-progress book. (This repository was born 2026-07-06 as the
IP-clean fork of the private `ti-99-emulator` repository — its history has
never contained a proprietary byte, and the predecessor is discontinued.)

## Documentation map — and keep it fresh

The [README](README.md) is the overview; `docs/` holds the project docs:
[USER-GUIDE](docs/USER-GUIDE.md) (emulator manual),
[ARCHITECTURE](docs/ARCHITECTURE.md) (machine + crate/module layout),
[DEVELOPMENT](docs/DEVELOPMENT.md) (build/test, conventions, IP checklist),
[STATUS](docs/STATUS.md), [ROADMAP](docs/ROADMAP.md),
[KNOWN-ISSUES](docs/KNOWN-ISSUES.md), [CROSS-VALIDATION](docs/CROSS-VALIDATION.md),
and `docs/history/` (archived plans/reports). The assembler guide is
[assembler/ASSEMBLER.md](assembler/ASSEMBLER.md); the firmware-rewrite docs
start at [original-content/system-roms/README.md](original-content/system-roms/README.md).

**Documentation is part of every change — keep it fresh.** When a change
alters behavior, status, or structure, update the affected docs (and the
in-app `F1` help for user-visible behavior) **in the same commit**; when a
plan finishes, archive it to the matching `history/` directory with a banner
naming its successor instead of leaving it to rot. The full policy, including
which doc owns what and how to regenerate the README screenshots, is in
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md). The book under `docs/ti99book/` is
a separate work-in-progress with its own `CLAUDE.md` — don't sweep it into
project-doc refactors.

## Reference: Classic99 source

A full checkout of **Classic99** (Tursi's well-established, hardware-verified
TI-99/4A emulator, C++) is available on **both** workstations used for this
project: **`C:\ClaudeShared\classic99`** on the PC and
**`/Users/Shared/classic99`** on the Mac (same tree, sibling of this repo). It
is *not* part of this project — it is an external reference to **consult, never
copy**. Consult it when emulating subtle hardware behavior; it has been
cross-checked against real hardware and resolves ambiguities the datasheets
leave open. Most useful files:

- `console/cpu9900.cpp` — TMS9900 core. The `WStatusLookup` / `BStatusLookup`
  tables (built in `buildcpu()`) are the authoritative word/byte status-flag
  definitions (LGT/AGT/EQ/C/OV/OP). The post-increment ordering notes near the
  top of the file are subtle and worth reading.
- `console/Tiemul.cpp` — the machine/bus. `ReadValidGrom` / `WriteValidGrom`
  (and the VDP read/write helpers) document the GROM and VDP prefetch and
  address-port semantics precisely. Note: an address-port **access resets the
  address-write byte selector** — this exact detail was a boot bug (see
  `docs/history/POSTMORTEMS.md`).
- `console/tivdp.cpp` — TMS9918A details (prefetch-inhibit, status read side
  effects, etc.).

When our behavior disagrees with Classic99, Classic99 is almost certainly right;
reproduce its behavior and add a focused regression test on our side.

## User data

Everything user-specific lives in **`~/.libre99/`** (the frontend creates
it at startup via `config::ensure_data_dir`): the preferences
(`libre99.toml`), the run log (`libre99.log`, appended across
runs), the resume state (`resume.ti99`, the automatic save state), and
screenshots. Any new
user-specific file (TOML, logs, caches, exports, …) must go under this directory
— derive its path from `config::data_dir()`, never write elsewhere in `$HOME`.
The desktop binary is named `libre99`.

## Licensing

The project is licensed under the **Modified MIT License with Commons Clause**
(source-available; the sell right is removed) — see `LICENSE.md`. Every Rust
source file we author (all of `crates/**/*.rs` plus `docs/ti99book/code/bench`)
carries the **full text of `LICENSE.md` as a commented (`//`) header** at the top
of the file; the firmware/cartridge sources (`.asm`/`.gpl`) carry a short
two-line license pointer instead. **Preserve those headers on every existing
file, and add one to every new source file**, keeping them in sync with
`LICENSE.md`. Do not reintroduce "all rights reserved" notices — they
contradict the license grant.

The license covers this project's own source and the clean-room ROM/GROM rewrite
under `original-content/system-roms/` only. It does **not** cover Texas
Instruments console/DSR/speech firmware (`roms/*.Bin`) or third-party cartridge
images (`cartridges/*.ctg`) — those are not the project's to license or
redistribute, and must be excluded before any public release (the standing
checklist is in `docs/DEVELOPMENT.md`).

## Git workflow

Normally commit and push directly to `main`. Only create a branch when one is
explicitly requested. Multiple Claude sessions sometimes work this repo in
parallel — check `git status` for a sibling's in-flight work before committing.

## Build / test

- `cargo test --workspace` — all four crates (core, app, asm, gpl); the
  emulator core (`-p libre99-core`) is pure `std` with zero third-party deps.
- `cargo clippy --workspace` — keep this clean.
- Repo scripts must be `sh` + `cargo` only (no `make`, no `python` on the PC).
- Orientation: `docs/STATUS.md` first, then `docs/ARCHITECTURE.md` and
  `docs/DEVELOPMENT.md`.
