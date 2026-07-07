# history/ — archived planning documents

These documents planned, reviewed, and guided the GROM and console-ROM rewrites
**before they were completed** (GROM M0–M7 shipped, 2026-07; console ROM
M1–M5/M7/M8 shipped, M6 deferred by policy). They are kept for provenance — the
reasoning behind design decisions, the IP stance as originally argued, and the
evidence trail — but they are **not maintained** and contain statements that
were later corrected or superseded.

**Do not work from these.** The live documents are one directory up:

| Live document | Replaces (from here) |
|---|---|
| [`../README.md`](../README.md) | the project overview + doc map |
| [`../STATUS.md`](../STATUS.md) | milestone tracking in the plan/playbook |
| [`../RECON.md`](../RECON.md) | all interface facts (headers, mechanisms, scratchpad map) |
| [`../DEBUGGING.md`](../DEBUGGING.md) | the review's traps/testing mechanics |
| [`../LIMITATIONS.md`](../LIMITATIONS.md) | open gaps and their paths forward |

| Archived file | What it was |
|---|---|
| `GROM-REWRITE-PLAN.md` | the original engineering plan (milestones M0–M6, TI PYTHON spec, toolchain design) |
| `GROM-REWRITE-REVIEW.md` | the senior review of the plan + the phase-by-phase implementation playbook (P0–P10); its §2 recon results seeded `RECON.md` |
| `FINISHING-PLAYBOOK.md` | the condensed M2→M6 completion guide written after the research session that verified every mechanism |
| `QUALITY-ASSESSMENT.md` | the 2026-07-02 GROM hardening plan (executed) — live: `../STATUS.md` + `../LIMITATIONS.md` + `../grom/SURFACE-MAP.md` |
| `QUALITY-ASSESSMENT-PROGRESS.md` | the hardening plan's execution log — live: `../STATUS.md` |
| `ROM-REWRITE-PLAN.md` | the console-ROM rewrite plan (executed; archived per its own §13 instruction) — live: `../rom/README.md` + `../rom/RECON.md` |
| `ROM-PROGRESS.md` | the console-ROM execution ledger — live: `../rom/README.md` (maintenance notes) + `../rom/RECON.md` |
| `ROM-ENTRY-CENSUS.md` | the R-3 dynamic entry census snapshot — live: `../rom/SURFACE-MAP.md` ("Validated empirically") + the `entry_census` gate |

The console-ROM (`rom/`) track's archives carry a `ROM-` prefix here to avoid
name collisions in this flat directory (e.g. `rom/PROGRESS.md` →
`ROM-PROGRESS.md`, `rom/ENTRY-CENSUS.md` → `ROM-ENTRY-CENSUS.md`).

Known corrections to watch for if you do read them: the ML-dispatch vector cell
is **`>8300`** (the review's §2.3 first said `>8380`); the assembler's operand
order is **`OP dst,src`** (the review's Phase 4 says `src, dst`); the review §8
scratchpad map's "OURS" rows describe a layout that was **not** what got
implemented (the implemented cell maps live as comments in
[`../grom/console.gpl`](../grom/console.gpl)).
