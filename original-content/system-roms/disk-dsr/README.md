# Disk Controller DSR rewrite — Phase 3 of the system-ROM project

A planned **clean-room reimplementation of the TI Disk Controller's 8 KiB DSR
ROM** (`roms/Disk.Bin`) — the last TI firmware that executes in the
emulator's default configuration. Original TMS9900 source assembled by our
own `libre99-asm`, honoring the console's DSR discovery/calling convention and
the byte-exact TI on-disk file-system format, verified differentially
against the genuine DSR as oracle. Removing this dependency is named on the
pre-public-release checklist (`docs/DEVELOPMENT.md` §Pre-public-release,
item 3).

**Status: COMPLETE (M1–M6, 2026-07-06) — the emulator installs the clean-room
DSR by default.** The full stock-TI surface (PAB opcodes in every mode,
subprograms `>10`–`>16` incl. stock FORMAT, the byte-exact on-disk file
system) matches the authentic `Disk.Bin` across 24 differential gates,
including image-level byte-identity on the write flows, cross-oracle interop
in both directions, and Tunnels of Doom loading from disk under both console
firmwares. The authentic TI DSR stays embedded and is selectable with
`--disk-dsr roms/Disk.Bin`. Deep-tier follow-ups (the random-PAB fuzz, the
perf tripwire) are listed in [`PROGRESS.md`](./PROGRESS.md).

| Document | What it is |
|---|---|
| [`DSR-REWRITE-PLAN.md`](./DSR-REWRITE-PLAN.md) | the full plan: scope, contracts, method, milestones, decision record, risks — **plus Appendix A, the planning-research seed dossier** an executing session starts from |
| [`PROGRESS.md`](./PROGRESS.md) | execution ledger + the resume point (start here when picking up a chunk) |
| `RECON.md` / `SURFACE-MAP.md` | the D1 dossier artifacts (created by chunk DSR-1's D1 half) |
| `disk-dsr.asm` / `disk-dsr.bin` | the DSR source and committed 8 KiB artifact (T1 tracer bullet today; grows through M1–M4) |

Method and provenance follow the completed Phase 1 (GROM) and Phase 2
(console ROM) tracks — see [`../README.md`](../README.md) and
[`../STATUS.md`](../STATUS.md).
