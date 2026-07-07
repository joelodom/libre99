# Disk DSR rewrite — execution progress & resume notes

Living status of [`DSR-REWRITE-PLAN.md`](./DSR-REWRITE-PLAN.md) execution, so
a fresh session resumes without re-deriving state. Sibling of the ROM track's
(now archived) execution ledger. **Read order:** `../README.md` →
`../STATUS.md` → the plan → this file → `RECON.md` (once it exists) → the
chunk you're picking up.

**House rules** (plan §13): dossier before code (Appendix A is the *seed* —
every 📖 fact re-pinned against `roms/Disk.Bin` before code relies on it; the
binary wins); a differential gate per element (the authentic DSR is the
oracle, driven through the PAB rig — no cartridge needed); completeness is
the bar, not usage (plan §0 ★); consult-never-copy; rebuild + commit
`disk-dsr.bin` on source changes once M1 lands; `cargo test -p libre99-asm -p
libre99-core -p libre99-gpl` + `cargo clippy --workspace` green; commit per
increment (check for sibling-session work first); update this file + RECON
as you go.

---

## Done

- **§10 decisions enshrined** (Joel, 2026-07-06): #2 (our DSR becomes the
  default *after* complete+tested; ship a `--disk-dsr` flag), #4 (stock TI
  FD1771 card only), #5 (no watermark) — **DECIDED**; #1 (write durability)
  and #3 (FORMAT scope) — **DEFERRED**, neither blocks the work (plan §10).
- **T1 — toolchain + tracer bullet** ✅ (2026-07-06, chunk DSR-1 half 1).
  `crates/libre99-asm/src/disk_dsr.rs`: `build_disk_dsr()` / `assemble_disk_dsr()`
  assemble the DSR at absolute base `>4000` into an 8 KiB image (constructing
  `Options { base: 0x4000, absolute: true, image_size: 0x2000, auto_header:
  false }` directly — `Options::absolute_image` hardcodes base 0), with the
  header pinned by `check_layout` and a guard that the FD1771 register shadow
  (`>5FF0..5FFF`) stays clear. A `libre99asm dsr <out.bin>` CLI subcommand mirrors
  `libre99asm rom`. The **tracer-bullet `disk-dsr.asm`** — a valid `>AA` header,
  an idempotent power-up reserving `>8370` → `>37D7`, and a DSK1 stub node — is
  discovered by the console's SROM and its power-up called, reserving the VRAM
  buffer **under both our console ROM and the authentic TI ROM**. Committed
  artifact `disk-dsr.bin` (8 KiB, 5 symbols). Gates: `libre99-asm` lib units
  (`disk_dsr_builds_to_8k_with_the_aa_header`, `the_chain_heads_point_into_the_image`)
  + `libre99-gpl/tests/disk_dsr.rs::our_dsr_power_up_reserves_the_vram_buffer`
  (both console ROMs). Clippy clean; no regressions in the touched crates.
  *(An unrelated sibling-session `tests/sokoban.rs` was red in the tree at
  commit time — not disk-DSR work; left untouched.)*

- **D1 — the dossier** ✅ (2026-07-06, chunk DSR-1 half 2; **probe-first**, a
  recorded method deviation from the console track — see `RECON.md`'s method
  note; SURFACE-MAP folded into RECON §§1b/5 since a DSR's contract is
  behavioral, not address-layout). Built the **PAB rig**
  (`crates/libre99-gpl/tests/dsr_common/mod.rs`: a generated system-GROM that
  runs the power-up scan, stages PABs/params, and drives the DSR through the
  real DSRLNK staging + `XML >19`, skip-return and all) + a **pure-Rust TI
  disk builder** (validated byte-for-byte: the authentic DSR LOADs and READs
  its files perfectly), + `Disk::drive_image` (image diffing). 17 probes
  green (`tests/disk_dsr.rs probe_*`) pinned: the header chains (DSK, DSK1-3;
  `>10`-`>16` + named FILES; unnamed len-0 power-up node → no strings — §10
  decision 5 = nothing to replace); power-up header bytes `AA 3F FF 11 nn` +
  the `FILES(n)` top formula `>3DEF−518n−6`; SECTOR's `>8350`-word input
  (ambiguity settled); the **FDIR 127-slot bisect** (empties compare high —
  decoded from probe orders `[5]`,`[5,3,2]`,`[5,6]`); error pins (missing
  OPEN=2, reclen/type mismatch=2, EOF=5, SCRATCH=**6**, bad volume=7,
  reclen-0 fill-in, sticky error bits); VAR eof-offset excludes the `>FF`;
  FIX eof=0; alloc = FDRs first-free-from-0 / data first-free-from `>22`;
  DELETE compacts the FDIR; catalog volume record =
  (trimmed name, 0, total−2, free) in radix-100; LOAD reads no VIB
  (hardcoded 9/40 stock geometry); the `>8354`→PAB / `>8356`→`>37E3`
  side-effects. Full fact table: [`RECON.md`](./RECON.md).

- **DSR-3..DSR-7 — M1–M6 COMPLETE** ✅ (2026-07-06, one extended session).
  `disk-dsr.asm` now implements the full stock-TI surface (plan §0 list) and
  **matches the authentic `Disk.Bin` across 24 differential gates**
  (`crates/libre99-gpl/tests/disk_dsr.rs`): power-up/FILES(n), SECTOR, PROTECT,
  RENAME, FILEIN/FILEOUT, stock FORMAT (cross-oracle-validated: the authentic
  DSR saves+loads on a disk ours formatted), OPEN in every mode (create /
  truncate / append), READ/WRITE (FIX rel+seq write-through with sparse
  extension; VAR append with cluster merge), CLOSE finalization, LOAD, SAVE,
  DELETE, STATUS, the radix-100 catalog, robustness (empty drive,
  unformatted disk, protected files) — **image-level byte-identity on every
  write flow**, the cross-oracle interop both directions, and ToD loading
  QUEST via our DSR under both console ROMs. **M6 integration landed:** the
  app installs the clean-room DSR **by default** (plan §10 decision 2's
  complete+tested bar met), `--disk-dsr` selects any image (e.g. the
  authentic `roms/Disk.Bin`, still embedded), the staleness gate ties
  `disk-dsr.bin` to its source. Behavioral pins added to RECON en route:
  unopened-PAB ops = error 7; protected files refuse write-mode OPEN and
  DELETE with error 1; UPDATE-on-missing creates a **name-only** FDR;
  DELETE-on-missing is silent; SAVE writes its final partial sector straight
  from VRAM (no zero-pad); unreadable-drive OPEN = error 6; the caller's
  FILEIN/FILEOUT block must avoid the DSR-owned scratch. House lessons (each
  cost a real bug): **CLR/SETO set no status flags** (`SZC R0,R0` = zero+EQ
  in one op); **a return-cell load (`MOV @RETF,R11`) sets flags — load the
  return first, assert the result flag last**; **the LV/RETV cells alias
  FNAME `>8360..`** — routines that still need the name park in **R6/R9**
  (the registers that survive the drivers); **SZCB already ANDs-NOT** (no
  pre-INV). Fit: the 8 KiB window forced the SLR/SLW/LDA/LDB/WRA/WRB/WR0/
  LD0/RDW/PUTNM helper layer (image 8192 B, 381 symbols, `>5FF0` shadow
  clear).

## Next — follow-ups (deep-tier, not blockers)

> **These follow-ups now have an execution plan:**
> [`DSR-ASSURANCE-PLAN.md`](./DSR-ASSURANCE-PLAN.md) (2026-07-06) — milestones
> A0–A6 covering the fuzz, the estate matrix, the bundled-disk sweep, the TI
> BASIC parity gate, the completeness sweep + entry census, the perf tripwire,
> the manual xdt99 round-trip, and hygiene/doc-sync from the post-M6 review.
> Tracked in `docs/ROADMAP.md` §8 as **[later]** — not required for 0.1.0.

- **The random-PAB differential fuzz** (plan §8) — seeded op-sequences over
  synthetic disks under both DSRs; the M5 robustness set landed, the fuzz
  did not. The manual third-party round-trip (xdt99, Mac) also remains.
- **A perf tripwire** (ToD-load frames ours ≤ authentic ×1.25) — expected
  faster; not yet measured.
- **Plan §10 decision 3** (FORMAT scope) stays DEFERRED — the shipped FORMAT
  is the stock single-density subset both options include; revisit before
  extending.
- Write durability (decision 1) remains emulator/app work, untouched.

## Concerns / notes (come back to these)

- **The skip-return discipline** (plan §2.2) is the known wedge hazard —
  make it M1's first gate, before any file-system work.
- **The FORMAT oracle gap** (plan §11.3): don't burn time trying to
  differentially gate FORMAT against the authentic DSR — our card no-ops
  Write Track; use the structural validator + cross-oracle read-back.
- **Scratchpad discipline**: the DSR owns `>834A–>836D` + `>83DA–>83DF`
  only; `>83E0–83FF` ARE the GPLWS registers (the ROM track's alias bugs —
  comment every reference with its register identity).
- **PC has no Python** — all automated gates cargo-only; xdt99/TIImageTool
  round-trips are a manual deep-tier step on the Mac (plan §4 P2).
