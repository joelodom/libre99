# Disk DSR assurance follow-through — plan (post-M6)

Closes the gap between what the DSR rewrite **shipped** (M1–M6, 24
differential gates, clean-room DSR as the app default — commits `2941e64`,
`3d2af2e`) and what [`DSR-REWRITE-PLAN.md`](./DSR-REWRITE-PLAN.md) §0 defined
as *done*, plus source-hygiene and doc-sync defects found in the 2026-07-06
production-readiness review. **Future work, explicitly NOT required for 0.1.0**
(Joel, 2026-07-06) — tracked in `docs/ROADMAP.md` §8 as **[later]**.

*Written 2026-07-06 from the post-M6 review. Status: **NOT STARTED.***

**Read order for an executing session:** `../README.md` → `../STATUS.md` →
[`DSR-REWRITE-PLAN.md`](./DSR-REWRITE-PLAN.md) (the parent plan; A0 archives it
to `../history/`) → [`PROGRESS.md`](./PROGRESS.md) → [`RECON.md`](./RECON.md)
→ this plan → your chunk (§5).

---

## 0. Scope & definition of done

**Goal.** The parent plan's §0 ★ bar — *functional completeness over usage
evidence, every element verified against the authentic oracle* — was met for
the hand-written gate set, but four of its testing instruments and one
integration promise never landed, and the review found two hygiene defects.
This plan lands exactly that remainder. **No new DSR functionality** — the
operation surface is complete; this is assurance, hygiene, and doc truth.

**The remainder, itemized** (each maps to a milestone below):

1. Source hygiene: `disk-dsr.asm` mojibake + an encoding tripwire (A0).
2. Doc truth: four stale docs + the parent plan's archival (A0).
3. The `[TI_DSR, OUR_DSR]` matrix over the *pre-existing* disk test estate —
   parent plan §8's first bullet, never executed (A1).
4. The bundled-disk catalog/read sweep — parent M2's gate said "all 15
   bundled disks byte-identical"; the shipped gate uses one synthetic disk (A2).
5. TI BASIC scripted file-I/O parity — parent M3's gate, absent (A3).
6. The **random-PAB differential fuzz** — parent M5's centerpiece, absent.
   This is the highest-value item: the strongest remaining protection against
   a disk-corruption bug the 24 hand-written gates don't reach (A4).
7. The **completeness sweep** (gate set ⊇ element enumeration, machine-checked)
   and the **entry-census tripwire** — parent §0 DoD checkboxes, absent (A5).
8. The **perf tripwire** (unmeasured) + the **manual xdt99 round-trip** (Mac,
   not done) + final ledger/checklist wrap-up (A6).

**Definition of done.**

- [ ] `grep -c 'Ã' disk-dsr.asm` = 0; license header intact; rebuilt
  `disk-dsr.bin` **byte-identical** (comment-only change); encoding tripwire
  test committed.
- [ ] `docs/STATUS.md`, `docs/USER-GUIDE.md`, `docs/DEVELOPMENT.md` (§Pre-
  public-release item 3) tell the truth about the clean-room DSR and
  `--disk-dsr`; `DSR-REWRITE-PLAN.md` archived to `../history/` with a banner.
- [ ] Every pre-existing disk-flow gate in `crates/libre99-core/tests/disk.rs`
  and `crates/libre99-gpl/tests/device_io.rs` runs under **both** DSRs.
- [ ] Every bundled `.Dsk` catalogs and sample-reads byte-identically under
  both DSRs.
- [ ] A TI BASIC battery (OPEN/PRINT#/INPUT#/CLOSE, SAVE/OLD, DELETE) under
  authentic console firmware leaves **byte-identical disk images** under both
  DSRs.
- [ ] The seeded random-PAB differential fuzz is green: fast tier pre-commit,
  deep tier (`#[ignore]`) over the committed corpus; every divergence found en
  route fixed with a named regression gate + RECON note.
- [ ] The op-surface completeness sweep runs every PAB opcode × file type ×
  mode × access cell (or names its skip with rationale — **zero silent
  gaps**) differentially green, and drives every pinned error code.
- [ ] The entry-census tripwire asserts the DSR is entered only via
  header-declared entries over a representative corpus.
- [ ] The perf tripwire (ToD QUEST load, ours ≤ authentic × 1.25) is green and
  the measured numbers are recorded in [`README.md`](./README.md).
- [ ] The manual xdt99/TIImageTool round-trip verdict is recorded in
  [`PROGRESS.md`](./PROGRESS.md).
- [ ] The parent plan's §0 checklist boxes are ticked where now true; this
  plan is archived to `../history/` at completion.
- [ ] `cargo test --workspace` + `cargo clippy --workspace` green throughout.

**Out of scope:** write durability to the host `.Dsk` (parent §10 decision 1,
TABLED — an emulator/app feature, not assurance); FORMAT scope extension
(decision 3, DEFERRED); any new operation surface; the 0.1.0 IP-severance work
(`docs/ROADMAP.md` — but see the A0 note about pull-forward).

---

## 1. Review evidence (why each item exists)

Findings of the 2026-07-06 review, verified against the tree at `e98807a`:

| # | Finding | Evidence |
|---|---|---|
| 1 | Random-PAB fuzz never ran; §10 decision 2 conditioned the default flip on "the M5 matrix green" | `PROGRESS.md` §Next ("the M5 robustness set landed, the fuzz did not") |
| 2 | No completeness assertion / entry census for the DSR track | census infra exists only for GROM/ROM (`libre99-gpl/src/census.rs`, `libre99-asm/tests/entry_census.rs`) |
| 3 | Catalog gate uses one synthetic 2-file disk, not the bundled corpus | `crates/libre99-gpl/tests/disk_dsr.rs::dsr_catalog_matches` |
| 4 | No TI BASIC file-I/O gate anywhere | `grep -ri 'PRINT#' crates/` — no hits |
| 5 | Existing estate not parameterized over both DSRs | `crates/libre99-core/tests/disk.rs:370,416` load only authentic `Disk.Bin`; `device_io.rs` likewise |
| 6 | `disk-dsr.asm`: 44 comment lines carry multi-round CP1252↔UTF-8 mojibake, **including the license header** | `grep -c 'Ã' disk-dsr.asm` = 44; compare the clean `../rom/console.asm` header. Comment-only — the staleness gate proves the binary is unaffected |
| 7 | Stale docs | `docs/STATUS.md:19,26,69` still say "genuine DSR"; `docs/USER-GUIDE.md` omits `--disk-dsr` (options table + flag list ~line 251); `docs/DEVELOPMENT.md` §Pre-public-release item 3 still names the disk DSR as the open dependency; the parent plan was never archived (its own M6 packaging step) |
| 8 | Perf tripwire unmeasured; manual xdt99 round-trip not done | `PROGRESS.md` §Next |

---

## 2. Method & house rules (inherited, with assurance-specific notes)

- **The differential oracle stands.** The authentic `Disk.Bin` under our
  emulator remains the spec for everything it can execute. Where the fuzz or
  sweep exposes *unpinned* authentic behavior, **the binary wins**: pin it in
  [`RECON.md`](./RECON.md), match it, gate it — never rationalize a divergence.
- **Skip-if-absent discipline.** All authentic media loads go through
  `libre99_core::third_party::load` (the `dsr_common` idiom): `None` ⇒ the test
  **skips**, never fails. This keeps the suite green on a tree without
  proprietary bytes — which is the post-0.1.0 reality (the libre99 fork ships
  no `roms/`; these differential gates become maintainer-local checks, exactly
  as `docs/ROADMAP.md` Road-to-0.1.0 row 2 prescribes).
- **Two-tier gating.** Fast tier pre-commit (seconds); `#[ignore]`d deep tier
  for corpus sweeps and the big fuzz. Every deliberate bound (seed count,
  pruned sweep cells) is **stated in the test's doc comment** — no silent caps.
- **Probes before fixes; a regression gate per fix; consult, never copy**
  (Classic99 and the authentic binary identify behavior; our code is written
  from the dossier). Every fuzz divergence fixed becomes a named fast-tier
  regression test plus a RECON/house-lessons note, the M1–M6 discipline.
- **Comment-only asm changes must not move bytes.** A0 touches only comments;
  the gate is a byte-identical rebuild (`committed_disk_dsr_artifact_is_fresh`
  stays green with an **unchanged** committed binary). If bytes move, stop —
  something other than comments changed.
- **Repo hygiene:** sh + cargo only (no python/make on the PC); commit per
  increment to `main` after checking for sibling-session work; update
  PROGRESS/RECON/docs **in the same commit** as the change they describe;
  never edit `../rom/**` / `../grom/**` beyond cross-references.
- **Workspace state note:** `libre99-gpl` builds our DSR **from source**
  (`dsr_common::our_dsr()` → `libre99_asm::disk_dsr::build_disk_dsr()`);
  `libre99-core` must NOT depend on `libre99-asm` (dependency direction), so
  libre99-core tests use the **committed artifact** via
  `include_bytes!("../../../original-content/system-roms/disk-dsr/disk-dsr.bin")`
  — freshness is already guaranteed by the libre99-gpl staleness gate.

---

## 3. Milestones

### A0 — source hygiene + doc truth (half a session; independent; pull-forward-able)

> **Note for the owner:** everything else here is post-0.1.0 by decision, but
> A0's doc items touch two files the 0.1.0 release gate reads
> (`docs/USER-GUIDE.md`, `docs/DEVELOPMENT.md`'s checklist). A0 is cheap and
> safe to pull forward any time.

1. **Mojibake repair.** `grep -n 'Ã' disk-dsr.asm` lists all 44 lines. The
   corruption is mixed-depth (some lines one mis-decode round, some three), so
   do **not** attempt a programmatic reverse-transform — visit each line and
   retype the intended character (almost all are em-dashes `—` and section
   signs `§`; the header lines 2 and 4 must end up matching
   `../rom/console.asm`'s wording exactly). Gates: `grep -c 'Ã'` = 0; the file
   decodes as valid UTF-8; **`disk-dsr.bin` rebuilds byte-identical** (see §2);
   `cargo test -p libre99-gpl --test disk_dsr` still 24 green.
2. **Encoding tripwire.** A unit test beside the source-embedding builders
   (`crates/libre99-asm/src/disk_dsr.rs` / `system_rom.rs` test modules, or one
   shared test) asserting every `include_str!`-ed `.asm`/`.gpl` source is
   valid UTF-8 and contains none of the mojibake marker characters
   (`'Ã'`, `'Â'`, `'â'` as *decoded chars* — legitimate house typography is
   `—`, `§`, `→`, `≤`, none of which decode to those). Keep the marker set a
   named const with a comment explaining the encoding-accident shape.
3. **Doc truth, one commit:**
   - `docs/STATUS.md` — the disk-controller row (line ~19) and prose (~26)
     say the **clean-room DSR is the default** (authentic selectable via
     `--disk-dsr`); drop "finish the clean-room disk DSR" from the outlook
     (~69).
   - `docs/USER-GUIDE.md` — add `--disk-dsr <path>` to the options table
     (beside `--system-rom`, ~line 72) and to the flag list (~line 251),
     mirroring `crates/libre99-app/src/cli.rs`'s help text.
   - `docs/DEVELOPMENT.md` §Pre-public-release item 3 — the disk-DSR
     dependency is **closed**; `roms/Disk.Bin` is comparison-only.
4. **Archive the parent plan.** Move `DSR-REWRITE-PLAN.md` →
   `../history/DSR-REWRITE-PLAN.md` with the house banner naming
   `PROGRESS.md` + `README.md` (and this plan) as successors; fix every
   inbound link (`grep -rn 'DSR-REWRITE-PLAN' --include='*.md' .` from the
   repo root — PROGRESS.md, README.md, RECON.md, `../STATUS.md`, this file).

**Gate A0:** greps clean · staleness gate green with an unchanged binary ·
`cargo test --workspace` + clippy green · no dead links among the touched docs.

### A1 — the `[TI_DSR, OUR_DSR]` matrix over the existing estate (with A2, one session)

Parent plan §8, first bullet, executed as written:

1. `crates/libre99-core/tests/disk.rs` — the register-level, ToD-end-to-end, and
   title-screen gates currently call `load_disk_controller` with authentic
   `Disk.Bin` only (lines ~370, ~416). Add a `for_each_dsr` helper: the
   authentic image (skip-if-absent) **and** the committed clean-room artifact
   (`include_bytes!` — §2's dependency note). Flows whose assertions are
   DSR-agnostic (title screen intact, VRAM reservation, QUEST loads) simply
   run twice; anything register-trace-specific that legitimately differs
   (FD1771 choreography is expressly *not* contract — parent §2.5) keeps its
   authentic-only scope **with a comment saying why**.
2. `crates/libre99-gpl/tests/device_io.rs` — same treatment across both console
   firmwares where the flow warrants: `bad_device_errors_gracefully…`,
   `disk_power_up_reserves_vram`, the QUEST-load gate. Our DSR comes from
   `dsr_common::our_dsr()` (built from source).

**Gate A1:** every pre-existing disk-flow gate runs under both DSRs (or
carries a written why-not); all green; the default configuration the app
ships (our ROM + our GROM + our DSR) is now covered by the estate, not only
by `disk_dsr.rs`.

### A2 — the bundled-disk catalog + read sweep

One new gate in `disk_dsr.rs` (or a sibling file sharing `dsr_common`):

- For **every** image in `disks/` (via `third_party::load`; skip absent —
  enumerate the *file list* from a committed const so a missing disk is a
  loud skip, not silence): drive the catalog under both DSRs — OPEN
  `"DSK1."` INT/FIX 38, READ record 0 (volume: name, 0, total−2, free in
  radix-100 — RECON §1) then every file record until the pinned EOF error —
  and **diff every record byte** plus the PAB status snapshots.
- For each disk, from its (authentic) catalog pick the first PROGRAM file →
  differential LOAD, and the first D/V file → differential OPEN(INPUT) +
  READ-to-EOF; diff VRAM + PAB + read-log order (the FDIR-bisection lockstep
  instrument, free of charge).
- Record counts vary per disk, so generate the `Rig` op list per disk in Rust
  after an authentic-side first pass (rig scripts are generated GROMs — this
  is the established pattern, not a new instrument).
- **Tier by measurement:** the 24 existing gates run in ~0.9 s; ~30 extra
  boots should stay fast-tier. If the read legs push past ~5 s, split: catalog
  sweep fast, read legs deep (`#[ignore]`), stated in the doc comment.

**Gate A2:** all bundled disks catalog + sample-read byte-identically under
both DSRs (the parent M2 gate, finally as written).

### A3 — TI BASIC scripted file-I/O parity (one session)

The parent M3 gate: **TI BASIC under authentic console firmware as a PAB
generator**, both DSRs, identical results. New test file (e.g.
`crates/libre99-gpl/tests/disk_basic.rs`):

- **Instrument (probe first).** Boot authentic console ROM+GROM
  (`third_party::load`, skip absent); adapt the `ti_python.rs`
  `key_for`/`type_line` keystroke idiom (that file drives our firmware — TI
  BASIC needs the master-menu `1` selection and its own prompt/READY
  detection; probe the name table for the READY signature and generous
  frame budgets before asserting anything).
- **Battery** (each script runs twice — once per DSR — on identical
  builder-authored formatted blank images):
  1. `OPEN #1:"DSK1.SEQ",DISPLAY,VARIABLE 80,OUTPUT` · `PRINT #1` several
     lines · `CLOSE #1` · reopen `INPUT` · `INPUT #1` · print to screen.
  2. The INTERNAL/FIXED 64 RELATIVE analogue (UPDATE: write, rewrite a middle
     record, read back).
  3. `SAVE DSK1.PROG` · `NEW` · `OLD DSK1.PROG` · `LIST`.
  4. `DELETE "DSK1.SEQ"`.
- **Diff:** the final disk **image bytes** (the heart of the gate), the final
  screen name table, and no-wedge (step budget per script).
- **Tier:** BASIC is slow under emulation — measure; expect deep tier
  (`#[ignore]`) with a one-script fast-tier smoke.

**Gate A3:** battery green; images byte-identical under both DSRs.

### A4 — the random-PAB differential fuzz (the M5 debt; one to two sessions)

New `crates/libre99-gpl/tests/disk_dsr_fuzz.rs`, sharing `dsr_common` (`mod`
path include, the existing pattern). House template:
`crates/libre99-asm/tests/gpl_fuzz.rs` — SplitMix64 PRNG (no wall clock, no OS
entropy), committed seeds, and on divergence print **seed + op trace +
differing cells** so any failure reproduces exactly.

- **Per seed:** author a randomized synthetic disk with the pure-Rust builder
  (0–8 files; mixed D/V, D/F, I/V, I/F, PROGRAM; sizes incl. 0-length and
  multi-cluster; fragmentation via build-then-delete ordering), then run a
  random sequence of 4–16 ops under both DSRs from identical state and diff
  after **every** op (PAB snapshot via `Snap`) and at the end (image bytes,
  owned scratchpad block, the *non-owned scratchpad unchanged* invariant,
  `>8370`, read-log order).
- **Op alphabet:** OPEN (random type/mode/reclen — including 0, mismatches,
  missing names, the protected set), CLOSE, READ/WRITE (sequential +
  relative, record numbers including past-EOF), RESTORE, LOAD/SAVE (random
  sizes incl. >1-sector tails), DELETE, STATUS, catalog OPEN/READ, and the
  subprograms `>10` SECTOR (r/w, random sector numbers incl. out-of-range),
  `>12` PROTECT, `>13` RENAME, `>16` FILES(1–5). Names drawn from
  {existing, missing, just-deleted}; occasional ops against an empty DSK2
  (the pinned error-6 path). Multiple PABs live at distinct VDP addresses.
- **Recorded exclusions** (state them in the test header, parent-plan style):
  FORMAT (the authentic oracle cannot run it on our Write-Track-less card —
  parent §11.3; instead, occasionally FORMAT under **ours only** and run the
  structural validator + an authentic read-back as the assert);
  FILEIN/FILEOUT param blocks constrained to legal shapes (the caller-contract
  pin, RECON §7b — wild pointers trample rig scratch by *contract*);
  `FILES(n)` only as a sequence-initial op (it re-lays the VRAM buffer region
  and would legally invalidate open files mid-sequence).
- **Tiers:** fast = ~8 committed seeds, short sequences (target < 3 s);
  deep `#[ignore]` = 256+ seeds × longer sequences.
- **Triage protocol:** every divergence is a real finding. Minimize by prefix
  replay, pin the authentic behavior in RECON, fix ours, land a named
  fast-tier regression gate + a house-lessons line in PROGRESS — the exact
  M1–M6 discipline. Budget for findings; that is the point of the milestone.

**Gate A4:** fast + deep tiers green over the committed corpus; divergences
found en route each closed with pin + fix + regression gate.

### A5 — the completeness sweep + the entry census (one session)

1. **The op-surface sweep** — the machine-checkable "gate set ⊇ element
   enumeration" the parent §0 DoD demanded, done programmatically (the
   `gpl_opcode_sweep.rs` precedent) instead of by bookkeeping: one test that
   **enumerates** opcodes 0–9 × file types {D/F, D/V, I/F, I/V, PROGRAM} ×
   modes {INPUT, OUTPUT, UPDATE, APPEND} × access {sequential, relative} and
   drives every cell differentially with a canonical fixture + stimulus —
   **including illegal combinations**, which must error identically. A cell
   may be skipped only via a named skip-table entry carrying its rationale
   (e.g. "APPEND × relative: rejected at OPEN, covered by cell X") — the test
   asserts every enumerated cell is either driven or named. Also assert the
   union of authentic error codes observed across the sweep equals RECON's
   pinned set — that closes the "every error code driven" checkbox.
   Subprograms `>10`–`>16` each get one canonical differential cell in the
   same sweep. Tier by measurement (~400 cells × 2 machines — likely deep
   tier with a pruned fast-tier smoke; state the bound).
2. **The entry-census tripwire** — the P8-DISK guard, deep tier: run a
   representative corpus (the ToD QUEST load + a rig batch) under our DSR
   with a PC-transition probe (idiom: `libre99-asm/tests/entry_census.rs` /
   `examples/rom_entry_census.rs`) and assert every external →
   `>4000..>5FEF` entry lands on a header-declared address — read the legal
   set (power-up, DSK1–3 device entries, `>10`–`>16` subprogram entries)
   from the image's own header chains, not from constants.

**Gate A5:** sweep green with zero silent gaps; census green; tick the
corresponding parent-plan §0 checkboxes (now in `../history/`).

### A6 — perf tripwire + manual round-trip + wrap (half a session + Mac time)

1. **Perf tripwire.** Count frames (deterministic machine ⇒ exact) from mount
   to the QUEST-load-complete signature under both DSRs — reuse
   `tod_loads_quest_with_our_dsr`'s completion detection. Assert **ours ≤
   authentic × 1.25** (expected: faster — we skip motor waits the card
   doesn't model). Record both numbers in [`README.md`](./README.md) (the
   parent M6 wanted perf numbers on the front door).
2. **Manual third-party round-trip** (Mac; documented, not gated — the PC has
   no python): a disk written and one formatted by **our** DSR → `xdm99`
   (xdt99) extracts every file, contents compared; a file imported by xdt99 →
   read back under both DSRs. Record the verdict + tool versions in
   [`PROGRESS.md`](./PROGRESS.md).
3. **Wrap:** final PROGRESS ledger entry; parent §0 checklist fully
   reconciled; `docs/ROADMAP.md` §8 item flipped to **[done]**; archive
   **this** plan to `../history/` with the house banner.

**Gate A6:** tripwire green with numbers committed; manual record committed;
docs current; both plans archived.

---

## 4. Risks & mitigations

1. **The fuzz finds real divergences.** Expected and desired — budget fix
   time in A4's estimate; the triage protocol (§3 A4) keeps findings from
   rotting. If a divergence reveals *authentic* behavior our pins
   contradict, the binary wins (§2).
2. **TI BASIC scripting is brittle** (keystroke pacing, prompt detection).
   Probe first; small deterministic battery; generous frame budgets; the
   image-bytes diff is the assertion that matters — screen diffs are
   secondary.
3. **Sweep/fuzz runtime.** Tier by measurement, never by guess; every bound
   stated in the test doc comment (§2 no-silent-caps).
4. **`FILES(n)` interactions** re-lay the VRAM buffer region and legally
   invalidate open state — constrain the generator (sequence-initial only)
   and document; a dedicated non-fuzz gate for mid-sequence `FILES` behavior
   can be added *if* RECON pins what the authentic DSR does there.
5. **Proprietary media absent** (public fork, CI): every authentic-oracle
   test already skips-if-absent via `third_party::load`; A1's libre99-core
   helper must follow suit. The clean-room-only arms (boot, reservation,
   validator-backed FORMAT) still run everywhere.
6. **Parallel sessions.** This plan touches `disk_dsr.rs` and the docs other
   tracks read — check `git status` for sibling work before every commit
   (house rule).

---

## 5. Work packaging — chunks for executing sessions

Ground rules per chunk: the read order in the header; probes before fixes;
consult-never-copy; fast gate `cargo test -p libre99-asm -p libre99-core -p
libre99-gpl` + `cargo clippy --workspace`; deep gate where the chunk says so;
update PROGRESS.md (a new "Assurance" section) + RECON as you go; commit per
increment.

| Chunk | Contents | Exit criterion |
|---|---|---|
| **ASSURE-1** | **A0** hygiene + doc truth | A0 gate; binary byte-identical; workspace green |
| **ASSURE-2** | **A1 + A2** the estate matrix + the bundled-disk sweep | every estate gate dual-DSR; all bundled disks differential-green |
| **ASSURE-3** | **A3** TI BASIC battery | image byte-identity under both DSRs |
| **ASSURE-4** | **A4** the fuzz | fast + deep corpus green; findings closed |
| **ASSURE-5** | **A5** completeness sweep + entry census | zero silent gaps; census green |
| **ASSURE-6** | **A6** perf + manual round-trip + wrap | numbers + verdict committed; plans archived |

Dependencies: **A0 is independent and pull-forward-able** (see its owner
note). A1 → A2 (they share the matrix helper and a session). A4 needs only
`dsr_common` but benefits from A1's helpers. A5 and A3 are independent. A6
last. Estimated effort at the established track pace: A4 is the fat chunk
(one to two sessions); everything else ≈ one session or less.

---

## 6. References

| Topic | Location |
|---|---|
| The parent plan (scope ★, decisions, milestone gates this plan completes) | [`DSR-REWRITE-PLAN.md`](./DSR-REWRITE-PLAN.md) → `../history/` after A0 |
| Execution ledger + the review's follow-up list | [`PROGRESS.md`](./PROGRESS.md) |
| The pinned interface facts (extend on every fuzz finding) | [`RECON.md`](./RECON.md) |
| The PAB rig + disk builder + `third_party::load` idiom | `crates/libre99-gpl/tests/dsr_common/mod.rs` |
| The 24 shipped gates + the staleness gate | `crates/libre99-gpl/tests/disk_dsr.rs` |
| House fuzz template (SplitMix64, committed seeds, divergence printout) | `crates/libre99-asm/tests/gpl_fuzz.rs` |
| House exhaustive-sweep precedent | `crates/libre99-asm/tests/gpl_opcode_sweep.rs` |
| House entry-census idiom | `crates/libre99-asm/tests/entry_census.rs`, `crates/libre99-asm/examples/rom_entry_census.rs` |
| Keystroke scripting idiom (adapt for TI BASIC) | `crates/libre99-gpl/tests/ti_python.rs` (`key_for`, `type_line`) |
| The estate to parameterize | `crates/libre99-core/tests/disk.rs`; `crates/libre99-gpl/tests/device_io.rs` |
| The clean-room DSR builder / committed artifact | `crates/libre99-asm/src/disk_dsr.rs`; `./disk-dsr.bin` |
| Roadmap home of this work | `docs/ROADMAP.md` §8 Assurance & hardening |
| Classic99 (consult, never copy) | `C:\ClaudeShared\classic99` (PC) / `/Users/Shared/classic99` (Mac) |
