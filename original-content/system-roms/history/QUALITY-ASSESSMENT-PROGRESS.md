> **ARCHIVED (2026-07-06).** Historical record — the §7 closure it tracked is complete. Live successor: ../STATUS.md.

# QUALITY-ASSESSMENT execution log — resume here

*Living status of the [`QUALITY-ASSESSMENT.md`](./QUALITY-ASSESSMENT.md) §7.7
five-chunk hardening plan. Update this file as items land so a fresh session
(or a token reset mid-flight) can resume without re-deriving state. One section
per chunk; check the boxes as exit criteria are met.*

**How to resume:** read [`README.md`](./README.md) → [`STATUS.md`](../STATUS.md)
→ [`LIMITATIONS.md`](../LIMITATIONS.md) → this file → the chunk you're picking up
in QUALITY-ASSESSMENT.md §5–§7. House rules: probes before fixes; a regression
test per fix; byte-identity gates for interface data; rebuild + commit
`grom/console-grom.bin` whenever `console.gpl` or a spliced data block changes
(`cargo run -p libre99-gpl --bin libre99gpl -- console original-content/system-roms/grom/console-grom.bin`);
verify with `cargo test -p libre99-gpl`; commit per completed item, directly to
`main`. Do NOT touch TI PYTHON internals (§7.2, deferred).

Chunk dependency spine: **1 → 2 → 5**; chunks **3** and **4** are independent
(pair well in one prompt). Every chunk except 1's analysis edits `console.gpl`,
so run chunks **serially** (parallel edits to `console.gpl` clobber/merge-conflict).
The one safe parallelization is Chunk 1's run-classification (read-only analysis
→ writes only `grom/SURFACE-MAP.md`).

**Review checkpoint (2026-07-03).** Full re-verification: working tree clean,
committed `console-grom.bin` byte-identical to a fresh build, fast suite green
(16 test binaries), and the **deep gate re-run green post-L7** (the full
137-cart sweep hadn't run since the `SBAD` menu-key change — it passes,
137/137 listing intact). One correction from the review: the earlier **"L2
BLOCKED" note was wrong** — see the corrected box under Chunk 4 (option 3
needs no banned instruction; RECON §4 endorses the big-window strategy).
Remaining work, in suggested order: **L2 (now unblocked) + L5** finish Chunk 4;
then **Chunk 2** (differential harness); then **Chunk 5** (L6 closure).

**L2 landed (2026-07-03).** Far-list carts (`starpeg`, `xb25`) now list *and*
launch; deep sweep **137/137**; `FAR_LIST_CARTS` removed. Implemented via
`SFAR` + an 8 KiB-slot re-copy for far bases (`console.gpl`). En route it exposed
— and fixed — a GROM-0 headroom trap (menu data pushed into the beep/font splices
→ runaway sound; see DEBUGGING.md case study 8). **Only L5 (progress cue) is left
in Chunk 4;** next up per the order above: **L5**, then **Chunk 2**.

**Second review checkpoint (2026-07-03, Joel-requested; plan amended at
QUALITY-ASSESSMENT §7.8 — read it before resuming).** Joel's F5 report had a
**second, real half** after all: after F5-from-gameplay the menu couldn't
launch anything. Root cause (case study 9, fixed + gated the same day by the
Phase-2 session): the `IO` CRU-output list has a **fourth field** — a
data-address byte at `>8305` — that `START` never wrote; a game's leftover
value made the arming `IO` *disarm* the 9901 VDP interrupt. Fix
`DST @>8304,>0100` in `console.gpl`; gate `tests/f5_reset.rs`; RECON §11
corrected (four-field list); case study 7's "did not reproduce" corrected.
Consequences for the remaining work (all recorded in §7.8):
- **Chunk 2 amended**: + warm-reset (F5) conformance checkpoint, + an F5 leg
  in the coverage sweep (the `f5_reset.rs` pattern, all carts, deep tier),
  + a static structure-handoff audit (the `>8305` class) recorded in RECON;
  do the **loud-stub grid first** (`>004A–005F` still executes zeros).
- **Chunk 5 is small**: 4 SERVICE-ENTRY runs / 43 bytes remain.
- **New Chunk 6 (close-out)**: P1 perf-parity gate + STATUS numbers, C3
  triage policy → DEBUGGING protocol Step 0.5, the §7.6 box-by-box walk,
  docs sync, and the archive-to-`history/` question for Joel.
- **Remaining order: 4b (L5) → 2 (amended) → 5 → 6.**

---

## Chunk 1 — Surface map + authentic data homes · **DONE** (2026-07-02)

*(§6 A1 + B1 + B4; §9 doc fixes.)* Exit criteria all met: `cargo test -p
libre99-gpl` green including the census gate (4/4); SURFACE-MAP classifies every
run; artifact rebuilt + committed.

- [x] `crates/libre99-gpl/src/census.rs` — reusable byte-census module (pure fns:
  `stats`, `authentic_only_runs`, `all_zero`; region + chip-gap constants).
  Verified: GROM-0 tally matches QUALITY-ASSESSMENT §3 exactly
  (245/498/3900/149/1352).
- [x] `crates/libre99-gpl/examples/grom_census.rs` — the census tool (stats table +
  GROM-0 authentic-only run list; flags chip-gap leakage). `cargo run -p
  libre99-gpl --example grom_census`. Emits **98** GROM-0 runs >= 8 bytes.
- [x] `grom/SURFACE-MAP.md` — all 98 GROM-0 runs classified (via a background
  analysis agent + reviewed by me): **12 DATA-MUST-MATCH (fonts), 82
  CODE-REPLACED, 4 SERVICE-ENTRY, 0 DEAD**; byte sums match the census baseline
  (3229). Evidence-based (disassembler tiling counts, RECON cross-refs). I
  stripped the agent's verbatim TI byte sequences (© glyph, sound lists, table
  values) → descriptions only, per the project's "not copied" rule. Two
  interconnect-slot-target runs (`>043B`/`>0446`) landed SERVICE-ENTRY on
  RECON evidence (a reasoned deviation from the CODE-REPLACED prior).
- [x] **B1** — fonts ship at their authentic homes via `system_grom.rs`
  splicing: `FONT` also at `>04B4` (label `FONTA`), thin set 7-row stored form
  at `>06B4` (label `THINA`), an **original** beep list at `>0484`
  (`BEEP0484`). `FONT` stays at `>1000` too. New emitter
  `font::emit_gpl_bytes_thin_stored`. Confirmed by census: GROM-0 `identical`
  245→1067, runs 98→85.
- [x] **B4** — `FONT2` (8-row loader block) relocated from the `>1800` chip gap
  to `>4000` (GROM 2). console.gpl comments updated; `LDTSET`'s `G@FONT2`
  resolves by label (no code change). Census: chip gaps now all-zero in ours.
- [x] `crates/libre99-gpl/tests/census.rs` — written: (a) DATA-MUST-MATCH
  byte-identity (font `>04B4`, thin `>06B4`); (b) map-completeness (parses
  `>XXXX..>YYYY` ranges, asserts every GROM-0 authentic-only run covered);
  (c) chip-gap zero; (d) map well-formed. *Compiles once SURFACE-MAP.md exists.*
- [x] §9 docs: `grom/README.md` address map regenerated (interconnect = 20 BR
  stubs, KEYTAB `>1700-17EF` + deflections `>16EA`, DSRLNK `>1200`,
  LDCSET/LDTSET, FONT2 `>4000`, the `>0484/>04B4/>06B4` homes); interface-data
  policy note added to `grom/README.md` + parent `README.md` (final legal
  wording flagged for Joel, §9 bullet 3). `STATUS.md` links SURFACE-MAP +
  records Chunk 1 done. `LIMITATIONS.md` top note added.
- [x] Library + integration suite green (`cargo test -p libre99-gpl`, 37 lib +
  all integration incl. char_set/menu/sweep/ti_python) **before** census.rs was
  added; artifact `grom/console-grom.bin` rebuilt.
- [x] Full `cargo test -p libre99-gpl` green (37 lib + census 4/4 + all
  integration); clippy clean on new code; artifact rebuilt; **committed**.

**Note for the next session — a concurrent Phase 2 track exists.** A separate
planning doc `original-content/system-roms/rom/ROM-REWRITE-PLAN.md` (console-ROM
rewrite, Phase 2) appeared during this session, authored by another track that
explicitly coordinates with this GROM work (its §12). It is **left untracked /
uncommitted by this chunk** — it belongs to that track, not Chunk 1.

**Key facts learned (for the next session):**
- The assembler zero-fills the 24 KiB image and `GROM >addr` sets an absolute
  LC, so data blocks can be spliced at any non-overlapping address regardless of
  order (`crates/libre99-gpl/src/asm.rs`).
- Splice happens in `system_grom.rs::console_gpl_source()`. `FONT`/`FONT2` emit
  raw BYTE lines (need an explicit `GROM` prefix); `LOGO` (`>1600`) and `KEYTAB`
  (`>16EA` deflections, then `>1700`) carry their own `GROM` directives.
- **Thin-set subtlety:** authentic `>06B4` stores the thin font as **7 rows/glyph
  (448 bytes)** = `font::packed_thin()`. `FONT2` (the loader block at `>1800`)
  is the **8-row expanded** form (512 bytes) = `emit_gpl_bytes_thin()`.
  Byte-identity at `>06B4` needs the **7-row stored** form — a new emitter
  (`emit_gpl_bytes_thin_stored`), NOT `emit_gpl_bytes_thin`.
- Sound-list format (`console.gpl` line ~438): `[N][N PSG bytes][duration frames]`,
  a duration-0 block ends the list. Beep at `>0484` must be original + <= ~10
  bytes so it stays clear of `>048F` and `FONT` at `>04B4`.
- `LDTSET` references `G@FONT2` by label, so relocating FONT2 only needs the
  splice `GROM >1800` → `GROM >4000` (immediate/label operands resolve forward).

---

## Chunk 2 — Differential harness: conformance, coverage, loud stubs · **DONE (2026-07-04)** · **AMENDED (§7.8)**

**Structure-handoff audit (amendment 1) — DONE (2026-07-03).** Ran as a
read-only background agent in parallel with L5, then verified. Enumerated all 8
structures our GPL hands the kept ROM → a **RECON.md table** ("Structure-handoff
audit"). 7 OK (each field written before its handoff, gate-cited); the one flagged
hazard — **`>83D6/7` (ISR screen-blank timeout) un-seeded** — was **refuted as a
divergence** by `examples/blank_timeout_probe.rs`: the authentic GROM doesn't seed
`>83D6` either (both tick it up from a pre-dirtied value identically, both blank
the title only if it's left near-wrap, both self-heal on the keypress the title
requests). Faithful reproduction, below the fix bar — not fixed (seeding would
diverge from authentic). Least-confident structure recorded: the `XML >19/1A`
device linkage (rests on `device_io.rs`, not a field proof) → a warm-disk-load
scratchpad-diff candidate for the coverage sweep.

**Service-stub grid (amendment 2 — the "do it first" item) — DONE (2026-07-03),
with an important correction.** The GPLLNK service tail `>004A-005F` was zero; the
whole grid `>0038-005F` is now `B SVCBAD` stubs. **Correction (caught by the
coverage sweep, below):** the first form of `SVCBAD` **rebooted** to our title,
which *regressed 16 bundled carts* — they CALL an unimplemented service and rely on
the graceful no-op the old zero tail (`>00` = RTN) already gave; e.g. **Parsec
rebooted to our title mid-game**. The `SVCBAD` stub now RTNs gracefully (it leaves
a breadcrumb first, but does **not** reboot). So §5 item 7's premise ("the zero
tail is a dangerous silent hole") was **corrected by execution**: `>00`=RTN is a
*safe* no-op carts depend on; the win is making it explicit/uniform + observable,
not louder. **§5 item 7 resolved** (graceful RTN + the emulator coverage instrument
for diagnosability). Gate `tests/service_stubs.rs`: **Parsec launches and does NOT
reboot** (real-cart regression guard) + static (no zero tail). Interconnect table
`>0010-0037` untouched (already a defined `BR` grid).

**GROM read-bitmap instrument (A2 foundation) — DONE (2026-07-03).** Built by a
worktree-isolated background agent (in parallel with the loud-stub grid), then
integrated + re-verified in the main tree. `libre99-core/src/grom.rs` gains a
coverage bitmap (`record_coverage`, `was_read`, `coverage_count`,
`read_addresses`) — one bit per GROM address, gated so normal runs pay nothing,
marking the same prefetch-corrected address as the read log. 3 unit tests; clippy
clean. This is what the coverage sweep queries to see which console-GROM
addresses (incl. the service entries) each cart exercises.

**Coverage sweep (A2) — DONE (2026-07-03).** `tests/coverage_sweep.rs`
(`#[ignore]`d, ~1 min): every cart → boot/menu/launch/attract + F5 relaunch, with
`grom_record_coverage` on (new bus wrappers in `machine.rs`), aggregated into
`grom/COVERAGE-REPORT.md`. It **caught the reboot regression above** (that is how we
found Parsec breaks), and now guards it: asserts **no cart reboots to our title
after launch** (unique `JOEL ODOM` marker — `TEXAS INSTRUMENTS` alone is a cart's
own screen). **Chunk 5 signal recorded:** ~16 carts CALL an unimplemented GPLLNK
service and carry on via the graceful RTN — Chunk 5 can decide implement-vs-waive
from the report (they run fine today).

**Coverage sweep — strengthened to a differential health panel (ship review,
2026-07-04).** The original coverage sweep asserted only "no cart reboots to our
title after launch" — weaker than the C2 intent ("play each game until it looks
wrong"). It now runs each cart under **both** our GROM and the authentic one and
asserts, *differentially*, that our console is never *less alive* after launch:
display on (VDP R1 bit 6) and, where authentic drives the console ISR as the
primary timekeeper, ours does too. Differential is essential — ~17 bundled arcade
carts legitimately take the machine over and freeze the console ISR under *both*
firmwares (faithful, not a bug); a fixed ISR-tick threshold misclassified them
(MISSION, burgerbeta, subcom, … straddle any cutoff), which is why the check keys
on the end-state, not a tick rate. **This caught the one real regression the
reboot-only gate missed: Video Vegas launches to a dead console under ours
(display off, 9901 masked) — LIMITATIONS L8**, waived by name so a *second* such
cart still fails the gate. Also added: **per-cart tripwire attribution** (2518
ours-zero addresses read, by all 137 carts; 2360 of them read under authentic too
— proof it is console execution over CODE-REPLACED regions, not cart data) and a
**font-home safety assertion** (no cart may read cart-facing interface data we
ship as zeros). Probe: `examples/isr_regression_probe.rs`. Runtime ~4-5 min (both
firmwares) — deep tier only.

**Conformance harness (A3, amendment 1) — DONE (2026-07-04).**
`tests/conformance.rs`, three checkpoints: (1) title, (2) menu — both assert the
observable **contract** under BOTH the authentic GROM and ours (display on, VBLANK
ISR live via `>8379`, each firmware's title / the menu lists the cart); and (3)
**F5-from-gameplay → title → menu**, the reset-drift guard — our post-F5 read of the
`START`-init cells (`>8300-8305` IO list, `>8373`) must equal cold boot, or a
program's leftover survived into a ROM service (the case-study-9 `>8305` class).
Our scratchpad *values* differ from authentic by design, so vs-authentic is on the
contract, not a byte diff; the reset guard is our-cold vs our-warm. **Chunk 2
complete.** (The optional log-line-on-service-fetch is folded into the coverage
instrument — not separately built.)

*(§6 A3 + A2 + B2; gates C1/C2; §5 item 7; §7.8 amendments 1–2.)* Needs Chunk
1's enumeration. Order inside the chunk: **loud-stub grid first**
(`>0010-005F`; the `>004A-005F` tail executes zeros today), then
`tests/conformance.rs` (**four** checkpoints — title, menu, post-launch, and
**F5-from-gameplay → title → menu**), the GROM read-bitmap instrument in
`crates/libre99-core/src/grom.rs`, `#[ignore]`d `tests/coverage_sweep.rs` with
the **F5 relaunch leg** per cart → `grom/COVERAGE-REPORT.md`, and the
**structure-handoff audit** (every ROM-consumed structure's fields written on
every path — the `>8305` class; table into RECON.md). Useful rigs already in
the tree: `tests/f5_reset.rs` (the warm-leg pattern), `libre99-core/examples/
{f5_press2_probe,f5_mask_bisect}.rs` (warm-state diagnosis), `examples/
f5_repro.rs` (broad F5 health harness).

## Chunk 3 — Console robustness (§5 items 4–6) · **DONE** (2026-07-03)
Probe-first. Independent of 2/5. All three items closed: 3b fixed (reset sound
mute), 3c fixed (menu 9-cap), 3a waived by execution (bad device already errors
gracefully, not a hang).
- [x] **(b) PSG mute on reset (§5 item 5)** — *this was Joel's reported F5 bug.*
  `reset()` is CPU-only so the SN76489 keeps its latches; a game's tune drones on
  channels 1-3 over our title after F5 ("no fun beep"). Differential probe
  `examples/reset_psg_probe.rs`: authentic mutes ch1-3 on reset, ours didn't.
  Fix: `SND`'s first block now opens `>06,>BF,>DF,>FF,…` (mute gen 1-3), matching
  authentic `>0484`. Test `tests/interrupts.rs::reset_mutes_stale_sound_channels`.
  Also `examples/f5_repro.rs` (broad F5 health harness from the investigation).
  Case study 7 in DEBUGGING.md. **Committed.**
- [x] **(c) menu entry cap at 9 (§5 item 4)** — confirmed with a synthetic
  12-program cart (`Cartridge.grom` is public): CNT reached **13**, entries
  10-13 scribbled the sprite table (`>0300+`). Fix: `CHE @>8350,>09 / BS SWRET`
  at `SWLOK` in `console.gpl` (also stops the digit running past `'9'`). Gate
  `tests/menu_cap.rs`; full 137-cart sweep still green (no bundled cart nears 9).
  **Committed.**
- [x] **(a) DSRLNK bad-device path (§5 item 6)** — **WAIVED by execution: not a
  bug.** The §5 hypothesis ("DSRLNK skips the `XML >19` result and barrels into
  `XML >1A` → hang") is **refuted**: the authentic DSRLNK (`>03DC`) is
  byte-identical to ours and *also* has no check — the not-found handling lives
  in the kept ROM's `XML >19/>1A`, which set the PAB error. Probe
  `examples/dsrlnk_baddev_probe.rs` drove ToD's "LOAD DATA FROM → CASSETTE (CS1)"
  and OTHER→garbage-device under authentic vs ours: **both stay alive (ISR keeps
  ticking); ours shows `DEVICE ERROR` and recovers to the menu.** This already
  meets the 1981 bar and L6/§7.4's shipping requirement. Guard
  `tests/device_io.rs::bad_device_errors_gracefully_without_hanging`. **Chunk 3
  now complete (3a/3b/3c). Committed.**

## Chunk 4 — Ledger: L2 + L5 + L7 · **DONE** (L7 2026-07-02; L2 + L5 2026-07-03)
(a) L2 far-list window re-anchor in `SCANW` → 137/137; (b) L5 progress cue;
(c) L7 reject-key beep parity. Independent; pairs with Chunk 3.
- [x] **(c) L7 reject-key beep** — differential probe
  `examples/menu_beep_probe.rs`: authentic beeps on an out-of-range key, ours
  didn't. Fix: `SGET`'s two reject branches route through new `SBAD` (arms
  `KBEEP`, `B SGET`). Gate `tests/interrupts.rs::menu_beeps_on_rejected_key`.
  **L7 → Resolved. Committed.**
- [x] **(b) L5 progress cue** — **DONE (2026-07-03).** `MENU` draws a `SCANNING`
  row (row 6) before the base scan; `SGET` erases it once the list is ready. No
  per-base edits were needed after all — a single draw (after the menu-header
  setup) + a single clear (as `SGET`'s first instruction, so every inbound path
  hits it, incl. the no-CPU-ROM `BR SGET` and reject-key `B SGET`) covers the
  whole scan. Data `SCANT`/`BLANK8` in the (relocated) data block. Gate
  `tests/menu_cue.rs`. **L5 → Resolved. Chunk 4 complete (L2/L5/L7 all done).**
- [x] **(a) L2 far-list** — **DONE (2026-07-03).** Implemented the reviewed
  "bigger window for the outliers only" plan: `SCANW`'s walk bound is now the
  scratch word `WBND` (`>835A`); helper `SFAR` reads each base's program-list
  pointer and, if it lands past the 512-byte window, the base re-copies its whole
  8 KiB slot into VDP `>1000–2FFF` and widens `WBND` to `>2FE0`. Immediate-source
  `MOVE`s only (no banned C=1). Applied to all cart bases (`>6000–E000` GROM +
  `>6000` ROM). `starpeg` and `xb25` now **list and launch**; deep sweep
  **137/137**; `FAR_LIST_CARTS` removed from `tests/sweep.rs`; fast gates
  `sweep_farlist_starpeg`/`_xb25`. **L2 → Resolved.**
  - *Headroom trap hit + fixed.* GROM 0 is packed; the added scan code pushed the
    menu **data** (incl. `SND`/`KBEEP`) past the `>0484` beep / `>04B4` font
    splices, which silently overwrote it (assembler = last-write-wins) → every
    beep ran away → the menu key-beep wait hung → no launch. Fixed by relocating
    the menu data block to the free gap above the thin font (`GROM >0880`). The
    `census` gate then flagged the vacated `>0406..>0438`; classified
    `CODE-REPLACED` in `SURFACE-MAP.md`'s addendum. Case study 8 in DEBUGGING.md.
  - *Residual bound (documented).* The widened window is one 8 KiB slot; a list
    that chains across a slot boundary is still not followed (no bundled cart
    does — 137/137). The general follow-the-chain form needs the banned computed
    `MOVE`. See LIMITATIONS L2.

## Chunk 5 — L6 service-surface closure · **DONE (2026-07-04)**
As §7.8 amendment 3 predicted, this was classification + rationale, not a
build-out. The coverage sweep confirmed **no bundled cart calls anything still
stubbed** (137/137 run, zero reboots). All 4 SERVICE-ENTRY runs (43 bytes)
dispositioned: `>004A-0057` = the graceful `SVCBAD` grid; `>043B` (slot `>0012`
sub-stack helper) + `>0446` (slots `>001A/1C/1E` BASIC trampolines) = out of
contract → `ILRTN`; `>1310` (cassette CS1 entry) = unreached (header `>08=0`).
Deferred by decision: cassette (no hardware → ROADMAP) + the unshipped GROM-2
library (unneeded). **L6 → Resolved.** LIMITATIONS L6 + SURFACE-MAP SERVICE-ENTRY
closure table + the stale-`console.gpl`-header-comment fix committed. (Analysis
by a read-only background agent, reviewed + integrated.)

## Chunk 6 — close-out · **IN PROGRESS (2026-07-04)**
(a) P1 `tests/perf_parity.rs` — **done** (built by a worktree agent; numbers in
STATUS.md; ours ≤ authentic ×1.25). (b) C3 §8 triage policy → DEBUGGING.md
protocol **Step 0.5** — **done**. (c) The §7.6 definition-of-done walk + docs sync
(STATUS/README/this file) + the two-tier gate in `grom/README.md` — **done**;
§7 marked complete. (d) Archive QUALITY-ASSESSMENT(+this file) to `history/`? —
**a question for Joel** (see the close-out report; not done unilaterally).

---

## Ledger snapshot (mirror of LIMITATIONS.md open set)
- **L1** Resolved (VBLANK ISR). **L4** Resolved (keytab). **L7** Resolved
  (reject-key beep, 2026-07-02). **L2** Resolved (far-list 8 KiB-slot window,
  2026-07-03 — both carts list+launch, deep sweep 137/137). **L5** Resolved
  (menu `SCANNING` cue, 2026-07-03 — **Chunk 4 now complete**).
- **L6** Resolved (service surface, 2026-07-04 — in-contract surface complete for
  the bundle; cassette + unshipped GROM-2 library deferred-by-decision, not open).
  **L3** deferred (TI PYTHON track). L1–L7 are Resolved or deferred-by-decision.
- **L8** ⚠ **OPEN (2026-07-04, ship review):** Video Vegas launches to a dead
  console under ours (unshipped GROM-2 library routine a bundled cart hard-depends
  on) — found by the strengthened differential coverage health panel, which the
  old reboot-only gate missed. Corrects L6's "no bundled cart needs it" by one
  cart. Gated (named waiver) with a scoped path forward. So the ledger is **not**
  zero-open anymore: L8 is a genuine, honestly-counted open gap.
- **Also addressed (not formal L entries):** §5 item 5 (reset sound mute /
  the beep half of Joel's F5 bug, Chunk 3b fix), §5 item 4 (menu 9-cap, Chunk
  3c fix), §5 item 6 (DSRLNK bad device, Chunk 3a — **waived by execution**:
  already errors gracefully, not a hang), and the **F5 launch half** (case
  study 9: the `IO` list's `>8305` data-address byte; fixed 2026-07-03, gate
  `tests/f5_reset.rs`).