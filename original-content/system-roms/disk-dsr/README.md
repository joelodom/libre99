# Disk Controller DSR rewrite — Phase 3 of the system-ROM project

The **clean-room reimplementation of the TI Disk Controller's 8 KiB DSR
ROM** — formerly the last TI firmware that executed in the emulator's default
configuration. Original TMS9900 source assembled by our own `libre99-asm`,
honoring the console's DSR discovery/calling convention and the byte-exact TI
on-disk file-system format, verified differentially against the genuine DSR
as oracle. (Removing this dependency was the pre-public-release checklist's
item 3 — done.)

**Status: COMPLETE (M1–M6, 2026-07-06) — the emulator installs the clean-room
DSR by default.** The full stock-TI surface (PAB opcodes in every mode,
subprograms `>10`–`>16` incl. stock FORMAT, the byte-exact on-disk file
system) matches the authentic `Disk.Bin` across 24 differential gates,
including image-level byte-identity on the write flows, cross-oracle interop
in both directions, and Tunnels of Doom loading from disk under both console
firmwares. A user-supplied authentic TI DSR is selectable with
`--disk-dsr <path>` (the differential suites load one at run time from the
git-ignored `third-party/roms/`, skipping green when absent). Deep-tier
follow-ups (the random-PAB fuzz, the perf tripwire) are collected in
[`DSR-ASSURANCE-PLAN.md`](./DSR-ASSURANCE-PLAN.md) — post-0.1.0 by decision.

| Document | What it is |
|---|---|
| [`../history/DSR-REWRITE-PLAN.md`](../history/DSR-REWRITE-PLAN.md) | the executed plan, archived (the decision record): scope, contracts, method, milestones, risks — plus Appendix A, the planning-research seed dossier |
| [`PROGRESS.md`](./PROGRESS.md) | the execution ledger (complete) + pointers to the deep-tier follow-ups |
| [`DSR-ASSURANCE-PLAN.md`](./DSR-ASSURANCE-PLAN.md) | the deep-assurance execution plan (fuzz, parameterized estates, sweeps — future work) |
| `RECON.md` / `SURFACE-MAP.md` | the D1 dossier artifacts |
| `disk-dsr.asm` / `disk-dsr.bin` | the DSR source and the committed 8 KiB artifact (complete; a staleness gate ties them) |

Method and provenance follow the completed Phase 1 (GROM) and Phase 2
(console ROM) tracks — see [`../README.md`](../README.md) and
[`../STATUS.md`](../STATUS.md).
