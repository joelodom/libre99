> **ARCHIVED (2026-07-06).** Historical record — the plan was executed in full (all six §7 chunks landed by 2026-07-04). Live successors: ../STATUS.md, ../LIMITATIONS.md, ../grom/SURFACE-MAP.md.

# QUALITY ASSESSMENT & HARDENING PLAN — the GROM rewrite

*Written 2026-07-02, after the M0–M7 build-out and the first five field bugfixes.
Audience: Joel, and the Claude sessions that will execute the plan (each work
item is self-contained: files, steps, command, acceptance criterion). Read
[`README.md`](./README.md) → [`STATUS.md`](../STATUS.md) →
[`LIMITATIONS.md`](../LIMITATIONS.md) first if you haven't.*

> **Execution status lives in
> [`QUALITY-ASSESSMENT-PROGRESS.md`](./QUALITY-ASSESSMENT-PROGRESS.md)** — this
> document is the plan, pinned as written; dated outcome notes below mark what
> has since landed. **✅ §7 CLOSURE COMPLETE (2026-07-04): all six chunks landed
> and every §7.6 definition-of-done box is checked.** L1/L2/L4/L5/L6/L7 Resolved,
> L3 deferred by decision. The census/conformance/coverage/sweep/perf gates are
> green (two-tier: `cargo test -p libre99-gpl` + `-- --ignored`); new reports go
> through the §8 triage policy (DEBUGGING Step 0.5).
>
> ⚠ **The ledger is no longer zero-open (2026-07-04, ship review, after §7
> closed).** Strengthening the coverage sweep from a reboot check into a
> *differential health panel* found **L8** — Video Vegas launches to a dead console
> under our GROM (an unshipped GROM-2 library routine a *bundled* cart
> hard-depends on), correcting L6's "no bundled cart needs it" by one cart. §7's
> closure of L1–L7 stands; L8 is a new, separately-tracked open entry with a scoped
> path forward and a durable future-work note (enumerate all GROM-2 dependents via a
> static call-scan). See [`LIMITATIONS.md`](../LIMITATIONS.md) L8.

---

## 1. Verdict

**The rewrite is in good shape.** The five post-milestone bugs are not a sign
of poor construction — they are the *predictable tail* of the project's own
(sound) strategy: implement the console's de-facto interface **on demand**, as
bundled software is observed to need it (`LIMITATIONS.md` L6 says exactly
this). Everything about the process around those bugs is healthy:

- Every fix was root-caused by differential trace, landed with a regression
  test, a case study, and doc updates. None was patched blind.
- The test suite (68 tests, all green; the full 137-cart sweep passes in ~12 s)
  gates everything that has ever broken.
- The docs (RECON / DEBUGGING / GUIDE / LIMITATIONS) are unusually disciplined —
  a new session can be productive in minutes.
- The GPL source itself is clean, well-commented, and defensively written
  (window walk bounds, read-only cartridge scans, verified-encoding-only rule).

**The real finding** is that all five bugs are *one defect class*, and that
class is **finite and mechanically enumerable** — so the path to quality is a
bounded audit, not endless play-testing. That is what the plan below does.
Phase A (the surface map) is the highest-leverage item and comes first.

**Decision (2026-07-02, after Joel reviewed this):** we are not stopping at
insurance — the goal is **zero open entries in `LIMITATIONS.md`**. The
closure plan is §7; it is contract-driven (enumerate the documented surface →
implement each entry → verify each differentially against the authentic
oracle), not another round of play-and-see. §8's triage policy then governs
only *new* reports.

**Scope refinement (same day):** this project is about the **emulation**, not
TI PYTHON — everything TI-PYTHON-specific is **deferred by decision** to a
later track (§7.2); the REPL banner now reads `TI PYTHON 0.0.1` so users get
that it's early. Quality bar adopted: **"production ready by 1981 TI
standards"** — no crash, hang, or corruption reachable from the console's
user flows; cosmetic divergence documented; perfection not required (home
computing, not critical systems). The work is packaged as **five hand-off
chunks (§7.7)** sized for the executing session — one prompt per chunk, or
combine per §7.7's dependency notes.

---

## 2. What the five bugs have in common

| Commit | Symptom | Root cause | Found by |
|---|---|---|---|
| `e26b72b` | no sound / sprites / QUIT anywhere | boot never enabled 9901 VDP interrupt (CRU bit 2) — authentic boot does | playing Tunnels of Doom |
| `f87546c` | TI Invaders draws no text | interconnect slots `>0016`/`>0018` (char-set loaders) stubbed to bare `RTN` | playing TI Invaders |
| `b8ec02c` | in-game keyboard dead | keytab region `>1760–17EF` (FCTN/CTRL/joystick blocks) shipped as zeros | playing TI Invaders |
| `c966335` | joystick dead | deflection table `>16EA–16FF` shipped as zeros | playing TI Invaders |
| `ef0e245` | F5 reset leaves old tiles/sprites | `START` painted only its own cells; authentic repaints the whole screen | pressing F5 in a game |

Common shape: **a fixed-address behaviour or data table of the authentic
console that cartridges (or users) reach directly, which the rewrite did not
know it had to provide.** Not one of the five was a logic bug in code the
rewrite *did* write — the GPL that exists is essentially correct. The menu,
dispatch, DSRLNK, REPL, title, and toolchain have produced zero field bugs.

Two structural observations follow:

1. **The exposure is the authentic image itself.** A cartridge was written
   against TI's shipped 24 KiB; *any* byte of it is potentially load-bearing
   (a table MOVEd from a known address, a routine CALLed at a fixed entry, a
   scratchpad cell the boot is expected to have set). The compatibility
   surface is therefore *bounded by 24 KiB* — auditable in one pass.
2. **The bugs were found at the phase the gates don't cover.** The sweep
   asserts *menu listing* for all 137 carts but *launch-and-run* for only ~8
   class samples; four of the five bugs manifest only after launch, under
   input. The gate stops exactly where the bugs start.

---

## 3. The numbers — byte census, authentic vs ours

Method: classify every byte of `roms/994AGROM.Bin` (authentic) against
`grom/console-grom.bin` (ours). Reproducible with the one-liner in the
appendix; Phase A1 turns it into a committed tool.

| Region | Bytes | identical | both zero | **authentic-only** | ours-only | differ |
|---|---|---|---|---|---|---|
| GROM 0 `>0000–17FF` (monitor) | 6144 | 245 | 498 | **3900** | 149 | 1352 |
| GROM 1 `>2000–37FF` (BASIC)   | 6144 | 9   | 251 | **4850** | 33  | 1001 |
| GROM 2 `>4000–57FF` (BASIC + GPL library) | 6144 | 0 | 275 | **5869** | 0 | 0 |
| chip gaps `>1800`, `>3800`, `>5800` (+2 KiB each) | 6144 | 5 | 21 | 5735 | 3 | 380 |

Reading it:

- **GROM 0 is the hazard zone.** 3,900 bytes of authentic monitor content have
  no counterpart in our image (99 runs ≥ 8 bytes — enumerated in the
  appendix). The keytab, deflection table, and thin font were all in this set
  until this week. What remains is a mix of: authentic title/menu/BASIC-entry
  *code* we replaced by design (harmless — carts don't jump into the middle of
  TI's menu), and *data tables / library routines at documented addresses*
  (each one a potential repeat of the TI Invaders class). Nobody has yet gone
  through the 99 runs and said which is which. **That classification is Phase
  A1 and is the single highest-value piece of work available.**
  - Concrete predicted example: the authentic standard font lives at
    **`>04B4–06B3`** (our `FONT` is byte-identical *content* but lives at
    `>1000`; `f87546c`'s own commit message notes the equivalence). A cart
    that draws text by `MOVE`ing from the *documented address* `>04B4` —
    rather than CALLing loader slot `>0016` — draws blank under ours. Same
    for the thin set at `>06B4+`, and the authentic menu-beep sound list at
    `>0484` (`>83CC` can be pointed at GROM data by address). These are
    zero-cost to fix (the addresses are empty in our image; the data is
    already in the repo) — see B1.
- **GROM 1/2 gaps are mostly by-design** (TI BASIC is deliberately not
  reimplemented; TI PYTHON stands in its menu slot). But GROM 2 also holds the
  authentic **shared GPL library** that the `>0038+` GPLLNK service entries
  vector into; ours ships 0 non-zero bytes there and points the service
  entries at a reboot stub. Cartridges that GPLLNK into console utilities
  (beyond DSRLNK/char-set loaders, already done) reboot to our title — loud,
  at least, but see B2 for making it *diagnosable*.
- **The `>1800–1FFF` note.** Real console GROMs are 6 KiB chips in 8 KiB
  slots; `>1800–1FFF` doesn't exist on hardware (the authentic dump carries
  ghost bytes there). Our emulator's flat 64 KiB model happily serves that
  range, and the rewrite *placed real content there* (`FONT2` at `>1800`).
  Works fine in-emulator; would fail on real hardware or a strict emulator
  (MAME). Cheap fidelity fix when convenient: relocate `FONT2` into empty
  GROM 2 space (`>4000+`) — one label + rebuild. Tracked as B4.

---

## 4. Residual risk, ranked

| # | Risk | Likelihood | Blast radius | Covered by |
|---|---|---|---|---|
| R1 | Cart reads a GROM 0 fixed-address **data table** we ship as zeros (font at `>04B4`, sound lists, other unclassified tables) | **High** — it already happened 3× | feature silently dead (text/sound/input) | A1 + A2, fix B1 |
| R2 | Cart CALLs an interconnect slot (15 of 20 are bare `RTN`) or GPLLNK `>0038+` entry (reboot stub) expecting the routine's side effect | Medium — ToD + Invaders already did | silent no-op *or* reboot mid-game | A2, stub-logging B2 |
| R3 | Boot/launch **state contract** drift: a scratchpad cell or VDP register the authentic boot sets and ours doesn't (the VBLANK-arming class) | Medium | anything ISR/timer/random-dependent | A3 |
| R4 | Our own GPL logic bugs (REPL stack overflows, menu >9 entries, unbounded input echo — §5) | Low (edge inputs) | REPL/menu crash or VRAM scribble | B3 |
| R5 | Post-launch *gameplay* regressions nothing gates (the phase all field bugs lived in) | Medium over time | re-breaking fixed behaviour | C2 |
| R6 | Hardware-fidelity landmines (`FONT2` at `>1800`) | Zero in-emulator | breaks only on real HW / other emulators | B4 |

---

## 5. Latent defects found by inspection (unconfirmed — probe before fixing)

Found by code review of `console.gpl` during this assessment. None is
observed in practice; each needs a 10-minute probe to confirm, then a
few-instruction fix + regression test, or an explicit waiver. House rule
applies: confirm by execution first (`GROM-DEBUGGING-GUIDE.md` §4.5).

*Scope update 2026-07-02: items 1–3 are TI-PYTHON-internal and move to the
deferred TI PYTHON track (recorded in `LIMITATIONS.md` L3) — do **not** fix
them in this plan. Items 4–7 are console-path and in scope (chunk 3; item 7
lands with chunk 2's stub grid).*

*Outcomes (chunk 3, 2026-07-02/03 — probes before fixes, as required): item 4
**confirmed and fixed** (synthetic 12-program cart reached CNT=13 and scribbled
the SAT; cap at `SWLOK`; gate `tests/menu_cap.rs`); item 5 **confirmed and
fixed** (this was Joel's field-reported F5 bug — `SND` now opens by muting
generators 1–3 like the authentic `>0484` list; gate
`tests/interrupts.rs::reset_mutes_stale_sound_channels`; DEBUGGING.md case
study 7); item 6 **refuted and waived** (the authentic DSRLNK at `>03DC` is
byte-identical to ours — the error handling lives in the kept ROM's
`XML >19/>1A`; CS1 and garbage devices already return `DEVICE ERROR` and stay
alive; gate `tests/device_io.rs::bad_device_errors_gracefully_without_hanging`);
item 7 still lands with chunk 2's stub grid.*

1. **TI PYTHON operator-stack overflow** — `EVAL` pushes `(` and operators at
   `>8360+` via `INC @>8311` with **no bound check**. 16 bytes are reserved;
   the 17th push writes `>8370` (top-of-free-VRAM cell), the 19th
   `>8372/>8373` (data/sub-stack pointers). Typing 19 `(` characters and
   ENTER should corrupt the GPL sub-stack → hang/crash. Fix: bound-check in
   `EV_NVAR`/`EV_PUSH` → `SYNTAX ERROR`. ~6 instructions.
2. **TI PYTHON operand-stack overflow** — same, `>8350–835F` (8 words), no
   bound: `1+(1+(1+(…` nine levels deep pushes the 9th operand into the
   operator stack at `>8360`. Fix: bound-check in the push paths → error.
3. **REPL input line unbounded** — `RDK` echoes every key with `DINC @>8312`
   and no length cap until ENTER; a long line walks the cursor across
   subsequent rows and eventually into the sprite attribute table (`>0300+`)
   and beyond. Cosmetic-to-weird. Fix: cap at row width (or 2 rows), ignore
   further keys.
4. **Menu with ≥ 10 programs corrupts the sprite table** — entry lines start
   at `>00E4` and advance `>40`/entry with no cap; entry 10 lands at `>0324`,
   inside the SAT. Also `DIGIT` becomes `:` `;` … past entry 9 and remains
   selectable. Bundled corpus maxes out well below 9 (the sweep passes), so
   this needs a synthetic multi-program cart to confirm. Fix: cap listed
   entries at 9 (authentic behaviour for >9: check via recon — likely also
   pages or truncates).
5. **F5 reset with sound playing may leave channels 1–3 droning** — `START`
   re-arms the ISR sound cells and plays our beep on channel 0, but never
   writes mute bytes (`>BF >DF >FF`) for the other generators; a cart mid-note
   on ch 1–3 at reset keeps sounding on the title screen. The `ef0e245` fix
   cleared VRAM but not the PSG. Differential probe: start a 3-channel sound
   under authentic, reset, sample PSG state; repeat under ours. Fix if
   confirmed: extend `SND`'s first block with the three mute bytes.
6. **DSRLNK's device-not-found path is unhandled** *(second inspection pass,
   2026-07-02 — the most user-reachable of these)*. `DSRLNK` never checks the
   `XML >19` search result — contrast `START`'s `PUSCAN`, which branches on
   it — and barrels into `XML >1A` regardless. A PAB naming a device no
   peripheral card serves (a garbage name, `DSK2` with one drive, or **`CS1`,
   whose DSR lives in the console ROM, not on a card** — and which Tunnels of
   Doom's "LOAD DATA FROM" screen offers by default) likely hangs or executes
   garbage instead of returning the authentic error. 1981 bar: a bad device
   must produce the DSR error return (condition set / PAB error bits) and the
   cart's own error prompt, never a hang. Differential probe first (extend
   `tod_disk_probe`: PABs for `CS1`, `DSK2`, `XYZ1` under authentic vs ours),
   then match the authentic convention.
7. **The GPLLNK service tail `>004A–0057` is zeros** — the census shows
   authentic content there (run `>004A..>0057`), but our reboot stubs cover
   only `>0038–0049`; a GPLLNK past `>0049` executes zero bytes. Fold into
   chunk 2's loud-stub grid: cover the full authentic service grid
   `>0038–005F` per the surface map.

**Where this inspection stopped, and why.** The pass covered the console path
end to end — boot, title, menu scan (window-bound arithmetic verified
correct), selection/dispatch, DSRLNK/char-set services, stubs, reset/QUIT —
plus the REPL. The classes that remain (state-contract drift, the
unclassified GROM-0 surface, post-launch behaviour across all 137 carts) are
precisely what chunks 1–2 enumerate *mechanically*; more eyeballing would
duplicate those gates, worse. Judged against the adopted bar — 1981 TI
production quality for a home computer — the in-scope findings above are the
ones that would have blocked a 1981 ship (hangs/corruption in user flows);
everything softer is documented divergence.

---

## 6. The hardening plan

Design principles, in order: **(1) enumerate, don't chase** — the surface is
24 KiB, finite; **(2) differential-first** — the authentic image under the
same console ROM is a perfect oracle, so equivalence never needs a written
spec; **(3) every check becomes a fast `cargo test` gate** — regressions are
caught at commit time, not in play; **(4) fix only on evidence** — everything
else becomes a documented LIMITATION or a loud stub. *(Scope note 2026-07-02:
for the already-enumerated ledger entries L1–L7, principle (4) is superseded
by the §7 closure track — those are being driven to zero deliberately.
It still applies to anything new.)*

> **For the executing session:** do items in order; one item per session is a
> good pace. Follow the house rules: probes before fixes, a regression test
> per fix, byte-identity gates for interface data, update
> `LIMITATIONS.md`/`STATUS.md`/`SURFACE-MAP.md` as you go, never copy TI bytes
> beyond the declared interface-data policy (§8). All commands run from the
> repo root; prepend `$env:USERPROFILE\.cargo\bin` to PATH on the Windows box.

### Phase A — map the surface (highest value; ~1–2 sessions; no GPL written)

**A1. The census tool + surface map.**
*Goal:* every byte of the authentic image classified; the classification
committed and test-enforced.
- Add `crates/libre99-gpl/examples/grom_census.rs`: load `roms/994AGROM.Bin` and
  `build_console_grom()`, emit per-region stats and every
  authentic-nonzero/ours-zero run ≥ 8 bytes (the §3 method; the appendix
  one-liner is the reference implementation).
- Create `grom/SURFACE-MAP.md`: one row per run — address range, what it is
  (identify with our own `libre99gpl` disassembler on the authentic bytes +
  Nouspikel/Classic99 *consulted, never copied*), and a classification:
  - `DATA-MUST-MATCH` — interface data carts may address directly → gets a
    byte-identity test and (usually) a B1 fix;
  - `CODE-REPLACED` — authentic code whose *function* ours provides elsewhere
    (title, menu, boot) → no action;
  - `SERVICE-ENTRY` — fixed entry points (interconnect/GPLLNK targets, e.g.
    the `>0396`/`>039E` loaders, DSRLNK internals) → cross-ref the B2 stub
    table;
  - `DEAD` — nothing known addresses it → no action, revisit only on A2
    evidence.
- Add `tests/census.rs`: asserts (a) every `DATA-MUST-MATCH` region is
  byte-identical (generalizing the existing font/keytab identity gates), and
  (b) no GROM 0 authentic-only run ≥ 8 bytes is *absent* from
  `SURFACE-MAP.md` (so the map can't rot).
*Acceptance:* `cargo test -p libre99-gpl --test census` green; SURFACE-MAP.md
covers all 99 runs; STATUS.md links it.

**A2. The read-coverage tripwire (turns unknown-unknowns into a list).**
*Goal:* observe every console-GROM address the corpus actually touches, and
flag any touch that lands where our bytes differ from authentic.
- Instrument `Grom` (`crates/libre99-core/src/grom.rs`) with a diagnostics-only
  read-bitmap over `>0000–5FFF` (24 K bits; record on data-read, method to
  take+clear). Same "instruments" pattern as the existing CRU accessors
  (`DEBUGGING.md` §Instruments).
- New `#[ignore]`d test `tests/coverage_sweep.rs`: for every bundled `.ctg`:
  boot ours → menu → launch first program → run ~600 frames while injecting a
  scripted input mash (reuse the key-injection from `keyboard.rs` /
  `joy_gameplay_probe.rs`; include arrows, FCTN combos, space, ENTER, joystick)
  → collect the post-launch read set → intersect with the not-byte-identical
  region set (from A1 data). Emit a per-cart report; write it to
  `grom/COVERAGE-REPORT.md`.
- Also record CALL/branch *fetches* in `>0010–005F` (which interconnect/GPLLNK
  slots real carts exercise) — this directly prioritizes B2.
*Acceptance:* report committed; every flagged (cart, range) pair is either
fixed (B1), stubbed-loud (B2), or waived with a LIMITATIONS entry. The test
then asserts "no unwaived hits" and joins the weekly gate (C2).
*Note:* ~137 carts × 600 frames is minutes, not hours — the 137-cart listing
sweep already runs in ~12 s.

**A3. Boot/launch state-contract conformance.**
*Goal:* mechanically diff the machine state the authentic vs our GROM hands to
(a) the title, (b) the menu, (c) a just-launched cartridge — the class of the
VBLANK bug and the `>8370` disk-buffer stall.
- New test `tests/conformance.rs`: boot both GROMs to three checkpoints
  (settled title ≈ 180 frames; menu built; +60 frames after launching one cart
  per class from `sweep.rs`'s class list). At each checkpoint diff:
  scratchpad `>8300–83FF`, all VDP registers, and VRAM
  name/pattern/color/sprite tables.
- Maintain an explicit whitelist (a `const` table with a comment per entry) of
  *intended* differences: our title/menu text cells, our copyright line, menu
  scratch cells `>8340–56`, TI PYTHON vs BASIC, frame-count-dependent timers.
  Everything not whitelisted must be equal; a new diff fails the test and
  demands either a fix or a reviewed whitelist entry.
*Acceptance:* `cargo test -p libre99-gpl --test conformance` green with a
documented whitelist. (Expect this to surface a handful of unset scratchpad
cells immediately — that's the point.)

### Phase B — close the gaps Phase A evidences (sized by A's output; ~1–2 sessions)

**B1. Ship interface data at its authentic homes.** Emit `FONT` also at
`>04B4`, the thin set at `>06B4` (pre-expanded content already in `font.rs`),
and an original menu-beep list at `>0484` — all currently zero regions, so
this is pure data placement via `system_grom.rs` splicing + identity tests.
Do this proactively even before A2 confirms a reader: cost ≈ zero, and it
retires the single most likely repeat of the TI Invaders class. (IP note:
same interface-data policy as the keytab/deflection tables — see §9.)

**B2. Make every unimplemented service loud.** The 15 bare-`RTN` interconnect
slots and the `>0038+` reboot stubs currently fail *silently* (that was the
char-set bug's month-long disguise potential). Point each unimplemented slot
at a tiny per-slot stub that leaves a breadcrumb — store the slot id to a
designated dead scratchpad cell (`GROM-DEBUGGING-GUIDE.md` §4.3 pattern) —
and add an emulator-side diagnostic (log line when a GROM fetch lands on a
stub entry address, gated like the other instruments). Implement an actual
routine **only when A2 or a field report shows a caller**. Update the
SURFACE-MAP row for each slot with: authentic behaviour (one line, from
consulted references) + current decision.
*Acceptance:* calling any unimplemented slot produces an identifiable log
line naming the slot, and `LIMITATIONS.md` L6 documents the mechanism.

**B3. Probe-confirm and fix the §5 inspection findings** *(in-scope items
only — items 1–3 are deferred with TI PYTHON, see §5's scope note)*. One probe each
(follow `DEBUGGING.md` §Probe inventory; `examples/tipython_probe.rs` and
`reset_artifacts_probe.rs` are the nearest templates). Fix what reproduces
(§5 sketches the fixes — all are a handful of GPL instructions), waive what
doesn't with a line in LIMITATIONS. Regression test per fix, house style.

**B4. Relocate `FONT2` out of the chip gap** (`>1800` → empty GROM 2, e.g.
`>4000`): one label + `LDTSET` reference + rebuild; add a census assertion
that our image is all-zero in all three chip gaps (`>1800/>3800/>5800` +2 KiB)
so nothing creeps back in. Do together with B1 since both touch
`system_grom.rs` splicing.

### Phase C — lock it in (~half session, then ongoing)

**C1. One umbrella gate.** Document (in `grom/README.md` build section) the
single pre-commit command for any GROM change:
`cargo test -p libre99-gpl` (now includes census + conformance + keyboard +
char_set + sweep samples + REPL) — and keep it under ~1 minute.

**C2. The weekly deep gate.** `cargo test -p libre99-gpl -- --ignored` runs the
full listing sweep + the A2 coverage sweep (launch + 600 frames + input mash,
with the DEBUGGING.md health-panel checks asserted mechanically per cart: ISR
counter advancing, no GROM-fetch wedge, name table non-blank). Run it before
any demo/release and after any multi-fix session; it is the automated
replacement for "Joel plays games until something looks wrong."

**C3. Adopt the triage policy (§8)** — add it to `DEBUGGING.md` as protocol
Step 0.5 so every future session applies it before investing.

### Performance (secondary — measure, budget, stop)

**P1. Measure once, assert forever:** add `tests/perf_parity.rs` measuring
frames-to-settled-title and frames-to-menu-complete for authentic vs ours on
the same machine config. Record both in STATUS.md. Assert ours ≤ authentic ×
1.25 (generous; today's title is already at parity by inspection — the boot
does strictly less work than authentic's GROM checksum + charset load).
**P2. Known costs are already at parity:** the menu scan (~1–2 s) is bounded
by the console ROM's per-byte GROM addressing, identical for authentic
(`LIMITATIONS.md` L5); the 512-byte-window + 2-byte-peek design is already the
right algorithmic shape. **Do no further GPL performance work unless P1's
assertion fails** — emulator-side speed is a different project
(`libre99-core`), out of this plan's scope.

### Definition of done (the whole plan)

1. Every authentic GROM 0 byte classified in `SURFACE-MAP.md`; census test
   enforces the map stays complete.
2. Every `DATA-MUST-MATCH` region byte-identical, test-gated.
3. Every unimplemented service entry fails *loud* (breadcrumb + log), not
   silent.
4. Conformance (state contract), coverage (read tripwire), and perf-parity
   gates green; the two-tier gate (fast pre-commit / weekly deep) documented.
5. §5 findings probed; fixed or waived.
6. Triage policy in DEBUGGING.md.

After that, **stop hardening**. New reports go through triage; the gates catch
regressions; the map bounds any future surprise to "we classified this DEAD
and were wrong" — which the A2 tripwire converts into a 5-minute diagnosis.

---

## 7. Closing the known-limitations ledger (decided 2026-07-02)

Joel's decision after reviewing this assessment: **drive `LIMITATIONS.md` to
zero open entries** — completeness over fix-on-evidence. This section is that
closure plan. It supersedes §6's principle (4) and the §8 triage policy *for
the enumerated ledger entries*; §8 still governs brand-new field reports. The
§6 machinery is not discarded — it is what makes closure *well-defined and
verifiable*: A1's `SURFACE-MAP.md` turns "all of L6" from a feeling into a
finite checklist, and the differential harness (A3 + per-entry probes)
verifies each closure against the authentic oracle without gameplay. This is
contract-first, not trial-and-error: enumerate the documented surface up
front, implement each entry, verify each differentially.

**Ledger truth check (2026-07-02).** The file was stale; the open set is
smaller than it reads (corrected in the same commit as this section):

- **L4 was already resolved** by `b8ec02c` — `keymap.rs` ships all four ASCII
  blocks (`>1705/>1735/>1765/>1795`) *and* the `>17C8` joystick table,
  byte-identical to authentic across `>1760–17EF`, gated by
  `tests/keyboard.rs`.
- **L7 is ~80% closed** — the menu arms `KBEEP` on leaving the title and on a
  valid selection, polling `>83CE` like TI does (`console.gpl`
  `TREL`/`SBWAIT`). Only reject-path beep parity remains unverified.
- The real open set is therefore: **L5 and the L6 remainder** (L2 and the L7
  reject-beep sliver are now Resolved — 2026-07-03 / 2026-07-02) — plus **L3,
  which is deferred by decision** to the TI PYTHON track (§7.2), not worked in
  this plan.

### 7.1 L2 — far-list cartridges (`starpeg`, `xb25`) · ✅ **DONE (2026-07-03)**

Shipped the "bigger window for the outliers only" option (follow-the-chain was
ruled out — it needs the banned `MOVE` C=1 computed-GROM-source form). `SCANW`'s
walk bound is now a cell (`WBND`, `>835A`); helper `SFAR` checks each base's
program-list pointer and, if it lands past the 512-byte window, the base
re-copies its whole 8 KiB slot with the immediate `MOVE >2000,G@base,V@>1000`
into free VDP `>1000–2FFF` and raises the bound to `>2FE0`. Applied to every
cart base. Both carts **list and launch**; `FAR_LIST_CARTS` removed from
`tests/sweep.rs`; deep sweep asserts **137/137**; fast gates
`sweep_farlist_starpeg`/`_xb25`. **L2 → Resolved.**

*Trap encountered + fixed:* the added scan code overflowed the tight GROM-0 code
region and pushed the menu **data** (incl. `SND`/`KBEEP`) into the `>0484` beep /
`>04B4` font splices, which silently overwrote it → runaway sound → menu launch
hung. Fixed by relocating the menu data block to the free gap above the thin font
(`GROM >0880`); `SURFACE-MAP.md` addendum classifies the vacated `>0406..>0438`;
DEBUGGING.md case study 8. *Residual (documented in LIMITATIONS L2):* the window
is one 8 KiB slot, so a cross-slot list/chain is still not followed — no bundled
cart does this (137/137).

### 7.2 L3 — TI PYTHON · **deferred by decision (Joel, 2026-07-02)**

This project is about the *emulation*; TI PYTHON is an early stand-in Joel
will improve later, so everything TI-PYTHON-specific is **out of this plan**.
Shipped now as the user-facing signal: the REPL banner reads **`TI PYTHON
0.0.1`** (source comment at `PYBANR` in `console.gpl`; gate updated in
`tests/ti_python.rs`). L3's ledger entry records the deferral — the "zero
open entries" goal treats L3 as *closed by recorded decision*, not by
silence.

The deferred track, for whenever Joel returns to it (kept in L3's entry):
backspace (now unblocked — FCTN block ships), input-length bound and the two
REPL stack guards (§5 items 1–3), multi-letter names, then bump the banner
version. **Executing sessions: do not work these items under this plan.**

### 7.3 L5 — menu scan speed · ✅ **DONE (2026-07-03)**

The per-byte cost is the console ROM re-writing the GROM address per byte
(RECON §10); authentic pays the same, so *faster than authentic* is not on the
table. Shipped the progress cue: `MENU` draws an original `SCANNING` row (row 6)
before the base scan and `SGET` erases it when the list is ready. Implementation
came out simpler than the "touch all 7 scan blocks" estimate — one draw + one
clear (at `SGET`, so every inbound path hits it) bracket the whole scan. Gate
`tests/menu_cue.rs`. **L5 → Resolved** (cosmetic parity documented in
LIMITATIONS L5).

### 7.4 L6 — the console service surface · the bulk (~2–4 sessions)

**Define "all" first — that is A1's job.** The public contract is finite and
enumerable: the twenty interconnect slots `>0010–0037` (their authentic
targets are data — read the words out of the authentic image and disassemble
each target with our own `libre99gpl` disassembler) plus the fixed GPLLNK
service entries `>0038+`, cross-identified against the Nouspikel/E-A
documentation (*consult, never copy*). `SURFACE-MAP.md` records, per entry:
authentic target, documented behaviour, and disposition. GROM-2 code
reachable **only** from TI BASIC internals is *out of contract* (we ship TI
PYTHON in BASIC's slot) — it gets classified as such with a rationale, not
reimplemented.

Then, per in-contract entry: implement the documented interface as original
GPL; verify **differentially** (a per-entry probe drives the same crafted
call under authentic and ours and diffs the observable effects — the
char-set-loader pattern, `DEBUGGING.md` case study 3); land a gate test.
Until an entry is implemented it keeps the loud-stub breadcrumb (B2), so
nothing fails silently while the plan is in flight.

**Cassette (CS1) disposition — recorded default (2026-07-02):** out of this
plan. The cassette DSR lives in the *kept console ROM*, but the emulator has
no cassette hardware (`crates/libre99-core/src/cru.rs`) — building it is a
libre99-core feature project, already tracked on the emulator ROADMAP §6, and it
would blow this plan's ≤5-prompt budget. What **is** in scope now is the 1981
behaviour without a tape: `DSRLNK("CS1…")` must fail gracefully with the
authentic error convention (§5 item 6, chunk 3), never hang. L6's GROM side
closes with this disposition recorded in the entry; Joel can commission the
cassette hardware separately by telling the executing session to include it.

*Acceptance:* every slot/entry has {authentic target, documented behaviour,
our implementation **or** out-of-contract rationale} in `SURFACE-MAP.md`;
per-entry differential gates green; L6 → Resolved.

### 7.5 L7 — key-beep parity · ~an hour

Differential probe: under the authentic GROM, does a *rejected* menu key (and
the title key-wait) beep? Match whatever it does; assert it in a
probe-derived test; L7 → Resolved.

*Done (2026-07-02): the probe (`examples/menu_beep_probe.rs`) showed the
authentic menu **does** beep on a rejected key; `SGET`'s reject branches now
route through `SBAD` (arms `KBEEP`); gate
`tests/interrupts.rs::menu_beeps_on_rejected_key`; L7 → Resolved.*

### 7.6 Definition of done (the whole plan)

- `LIMITATIONS.md`: **L2, L5, L6, L7 Resolved** (each with commit + gate);
  **L3 recorded as deferred** to the TI PYTHON track with the `0.0.1` banner
  shipped. Zero entries left open-and-unworked.
- `SURFACE-MAP.md` complete (every GROM-0 authentic-only run classified);
  census, conformance, coverage, and sweep gates green; every unimplemented
  service entry fails *loud*.
- §5 in-scope findings (4–7) probed, then fixed or explicitly waived.
- Both decision points carry recorded answers (they do, as of 2026-07-02:
  L3 deferred; cassette → emulator ROADMAP).
- The two-tier gate and the §8 triage policy are documented where future
  sessions will find them (`grom/README.md`, `DEBUGGING.md`).

**✅ Definition-of-done walk (2026-07-04) — every box checked:**
1. **Ledger.** `LIMITATIONS.md`: L1, L2, L4, L5, L6, L7 **Resolved** (each with a
   commit + gate); L3 **deferred** to the TI PYTHON track (`0.0.1` banner shipped).
   Zero open-and-unworked entries (cassette + the unshipped GROM-2 library are
   deferred-by-decision within L6, not open).
2. **Surface + gates.** `SURFACE-MAP.md` classifies every GROM-0 authentic-only
   run (incl. the SERVICE-ENTRY closure table); `census`, `conformance`,
   `coverage_sweep`, `sweep_all_cartridges` (137/137), and `perf_parity` gates are
   green. *One honest reinterpretation:* "every unimplemented service fails
   **loud**" became "**degrades gracefully** (RTN) and is **observable** via the
   GROM coverage instrument" — a visible reboot was tried and **regressed carts**
   (Parsec; `DEBUGGING.md` case study 10), so graceful-RTN + instrument
   observability is the correct form of the intent.
3. **§5 findings 4–7.** Probed, then fixed (item 4 menu 9-cap; item 5 reset sound
   mute; item 7 service tail) or **waived by execution** (item 6 DSRLNK bad-device,
   already graceful).
4. **Decision points recorded.** L3 deferred; cassette → emulator ROADMAP §6.
5. **Docs for future sessions.** Two-tier gate in `grom/README.md`; §8 triage
   policy as `DEBUGGING.md` protocol **Step 0.5**.

**§7 is complete.** New reports are governed by §8 from here.

### 7.7 Hand-off packaging — five chunks for the executing session

**Ground rules (apply to every chunk).** Read `README.md` → `STATUS.md` →
`LIMITATIONS.md`, then this document's §5–§7 for the items in your chunk;
read `GROM-DEBUGGING-GUIDE.md` + `DEBUGGING.md` before debugging anything.
Probes before fixes; a regression test per fix; byte-identity gates for
interface data; never copy TI bytes beyond the declared interface-data policy
(§9). Rebuild and commit `grom/console-grom.bin` whenever `console.gpl` or
spliced data change (`cargo run -p libre99-gpl --bin libre99gpl -- console
original-content/system-roms/grom/console-grom.bin`). Update
`LIMITATIONS.md` / `STATUS.md` / `SURFACE-MAP.md` as items land; commit per
completed item, directly to `main` (house convention). Verify with
`cargo test -p libre99-gpl`; run the deep gate (`cargo test -p libre99-gpl --
--ignored`) where the chunk says so. **Do not touch TI PYTHON internals
(§7.2).**

**Dependencies:** 1 → 2 → 5 is the spine (5 needs 1's enumeration; 2's
coverage evidence sharpens 5). Chunks 3 and 4 are independent of 2, of 5,
and of each other — they can run in any order after 1, and pair well in a
single prompt (3+4 is the natural combo). One chunk ≈ one prompt; chunk 5
may need two if the enumeration is fat.

**Status (2026-07-03):** chunk 1 ✅ done · chunk 2 ⬜ **(amended — see §7.8)** ·
chunk 3 ✅ done (3a waived by execution — §5 outcomes note) · chunk 4 ✅ done
(L7, L2 — 137/137, L5 cue) · chunk 5 ⬜ (smaller than budgeted — §7.8) ·
**chunk 6 (close-out) added — §7.8**. Details:
[`QUALITY-ASSESSMENT-PROGRESS.md`](./QUALITY-ASSESSMENT-PROGRESS.md).

---

**Chunk 1 — Surface map + authentic data homes.** *(§6 A1 + B1 + B4; §9 doc
fixes.)*
Build: `examples/grom_census.rs` (the census tool; reference method in the
appendix); `grom/SURFACE-MAP.md` classifying **all 99** GROM-0 authentic-only
runs (`DATA-MUST-MATCH` / `CODE-REPLACED` / `SERVICE-ENTRY` / `DEAD`);
`tests/census.rs` (byte-identity per DATA-MUST-MATCH region + map
completeness). Ship interface data at its authentic homes: `FONT` also at
`>04B4`, the thin set at `>06B4`, an **original** beep list at `>0484` (B1).
Relocate `FONT2` out of the `>1800` chip gap into empty GROM 2 and assert all
three chip gaps are zero in our image (B4). Regenerate `grom/README.md`'s
address map (§9 bullet 1).
**Exit:** `cargo test -p libre99-gpl` green including the new census tests;
SURFACE-MAP covers every run; artifact rebuilt and committed.

**Chunk 2 — Differential harness: conformance, coverage, loud stubs.**
*(§6 A3 + A2 + B2; gates C1/C2; §5 item 7.)*
Build: `tests/conformance.rs` (three-checkpoint scratchpad `>8300–83FF` +
VDP-register + VRAM-table diff vs authentic, with a commented whitelist of
intended differences); a diagnostics-only GROM read-bitmap instrument in
`crates/libre99-core/src/grom.rs`; `#[ignore]`d `tests/coverage_sweep.rs` (every
bundled cart: boot → launch → ~600 frames of scripted input mash → flag any
console-GROM read landing where our bytes ≠ authentic), report committed as
`grom/COVERAGE-REPORT.md`. Replace the service stubs with a **loud-stub grid
across the full authentic surface `>0010–005F`** (breadcrumb store §4.3 +
emulator log line naming the slot) — this also fixes §5 item 7. Document the
two-tier gate (fast pre-commit / deep weekly) in `grom/README.md`.
**Exit:** conformance green with documented whitelist; every coverage flag
fixed or waived in `LIMITATIONS.md`; deep gate documented and passing.

**Chunk 3 — Console robustness (§5 items 4–6).**
Probe-first, each with a regression test and (if a probe surprises) a
DEBUGGING.md case study:
(a) **DSRLNK bad-device path** (§5 item 6): differential probe with PABs for
`CS1`, `DSK2`, `XYZ1` under authentic vs ours; then check the `XML >19`
result and match the authentic error convention (condition/PAB error), so
"LOAD DATA FROM CS1" in Tunnels of Doom errors like 1981 instead of hanging.
(b) **PSG mute on boot/reset** (§5 item 5): differential probe (3-channel
sound → F5 → sample PSG) then, if confirmed, extend `SND`'s first block with
`>9F >BF >DF >FF`.
(c) **Menu entry cap** (§5 item 4): synthetic multi-program cart probe; cap
listed entries at 9 (one compare in `SWLOK`), guard test.
**Exit:** three probes committed; confirmed fixes green; full suite green.

**Chunk 4 — Ledger: L2 + L5 + L7.** *(§7.1, §7.3, §7.5.)*
(a) L2 ✅ **done (2026-07-03)**: 8 KiB-slot re-copy for far bases (`SFAR` +
cell-held walk bound) in `SCANW`; `FAR_LIST_CARTS` removed; deep sweep
**137/137**. (b) L5 ⬜: original progress cue during the scan; parity note.
(c) L7 ✅ **done (2026-07-02)**: reject-key beep differential probe; match
authentic. **Exit:** deep gate 137/137 (met); `LIMITATIONS.md` L2/L7 Resolved
(done), L5 pending.

**Chunk 5 — L6 service-surface closure.** *(§7.4; needs chunk 1.)*
From SURFACE-MAP's enumeration of `>0010–0037` + `>0038–005F`: implement
every **in-contract** entry as original GPL with a per-entry differential
probe + gate (the char-set-loader pattern); document out-of-contract entries
with rationale; record the cassette disposition (deferred → emulator ROADMAP
§6; chunk 3's CS1 error path is the shipping behaviour). Move L6 to
Resolved; mark §7 complete in this doc and `STATUS.md`.
**Exit:** `LIMITATIONS.md` zero open-and-unworked entries; fast + deep gates
green. *Sizing: may take a second prompt if the in-contract list is long —
split by entry list; it parallels cleanly.*

### 7.8 Review checkpoint (2026-07-03, Joel-requested) — amendments + the finishing line

**Verdict: healthy; keep going.** Six §7.7 work items landed in two days with
the discipline intact — differential probes, red-before/green-after gates,
case studies 7–9, and honest waivers (3a closed by *disproving* the
hypothesis). Ledger: L1/L2/L4/L7 resolved, L3 deferred by decision, L5
(cosmetic) + the L6 remainder open. Deep sweep 137/137. The amendments below
fold in what this week's field bugs taught; they are part of the plan.

**Amendment 1 — chunk 2 must cover warm-reset (F5) state.** Three field bugs
now share one root — state that survives `reset()` (CPU-only, like hardware):
VRAM (case study 6), PSG latches (case study 7), and scratchpad handed to ROM
services (**case study 9**: a game's leftover `>8305` made `START`'s own
arming `IO` *disarm* the ISR — found by Joel, not by a gate; fixed 2026-07-03,
`tests/f5_reset.rs`). The A3/A2 harness as originally specced boots cold only
and would have missed all three. Chunk 2 therefore also builds:
- a **fourth conformance checkpoint** — launch a class-sample cart → input
  mash → **F5 → settled title → menu**, diffing the same scratchpad/VDP/VRAM
  set against authentic driven through the identical flow (same whitelist
  machinery as the cold checkpoints);
- an **F5 leg in the coverage sweep** — after each cart's 600-frame mash:
  `reset()` → title key → menu → relaunch, asserting the health panel (ISR
  ticking, `>83CE` drains, the entry relaunches). This is
  `tests/f5_reset.rs`'s pattern generalized from TI Invaders to every bundled
  cart (deep tier);
- a **structure-handoff audit** (~an hour, static): enumerate every structure
  our GPL hands the kept ROM — `IO` lists, `XML >19`/`>1A` scan cells, sound
  lists, SCAN cells, the `XML >F0` vector, sub-stack frames — and verify every
  field is written on every path (the `>8305` class). Record the enumeration
  as a RECON table so it cannot rot.

**Amendment 2 — do the loud-stub grid first within chunk 2.** The GPLLNK tail
`>004A–005F` still executes zeros today (§5 item 7): a cart calling past
`>0049` runs zero bytes as GPL. It is both the most dangerous remaining hole
and the cheapest item in the chunk.

**Amendment 3 — chunk 5 is smaller than budgeted.** SURFACE-MAP returned only
**4 SERVICE-ENTRY runs (43 bytes)** beyond the services already implemented
(DSRLNK, both char-set loaders, the boot power-up scan). Expect
classification + loud stubs + out-of-contract rationale rows rather than a
build-out, with the A2 coverage report confirming no bundled cart calls
anything still stubbed. One prompt.

**Chunk 6 — close-out (new; runs last).** Two §6 items were never packaged
into a chunk, plus the finish line itself:
(a) **P1 perf parity**: `tests/perf_parity.rs` — frames-to-settled-title and
frames-to-menu-complete, authentic vs ours, assert ours ≤ authentic × 1.25;
record the numbers in `STATUS.md`. (b) **C3**: add the §8 triage policy to
`DEBUGGING.md` as protocol **Step 0.5**. (c) **The §7.6 walk**: check every
definition-of-done box item by item; sync `STATUS.md`/`README.md`/PROGRESS;
mark §7 complete in this document. (d) Ask Joel whether to archive this
document (+ PROGRESS) to `history/` per the GROM-REWRITE-PLAN precedent.
**Exit:** every §7.6 and §6 definition-of-done box checked; fast + deep gates
green; the ledger shows zero open-and-unworked entries; new reports go through
§8 triage from then on.

**Remaining order: 4b (L5 cue) → 2 (as amended; stubs first) → 5 → 6.**

---

## 8. Triage policy for new reports (the anti-rabbit-hole rule)

*Scope (2026-07-02): this policy governs **new** field reports and
discoveries. The seven enumerated ledger entries are exempt — they are being
driven to zero via §7.*

A reported or discovered bug gets an implementation session **only if** one of:

1. **Tier 1 — console behaviour**: title, menu, dispatch, TI PYTHON, ISR
   contract, reset. Always fix.
2. **Tier 2 — bundled-cart happy path**: a bundled cartridge's *primary flow*
   (boot → menu → launch → play with keyboard/joystick → QUIT) is broken.
   Fix if ≥ 1 cart is affected and the cause is ours (check LIMITATIONS
   first — DEBUGGING.md protocol Step 0).
3. **Evidence of breadth**: the A2 tripwire / stub log shows multiple carts
   touching the same gap.

Everything else — exotic carts outside the bundle, real-hardware-only
fidelity, deep BASIC-era library calls, cosmetic timing — becomes a
LIMITATIONS.md entry with a path forward, *by design and without guilt*. The
project's own L2 (two far-list carts, documented, deferred) is the model.

---

## 9. Housekeeping corrections (fold into the next docs commit)

- `grom/README.md` **address map is stale**: says the interconnect table is
  "left zero (L6)" (it's 20 `BR` stubs now), `KEYTAB` at `>1700–175F` (now
  `>1700–17EF`), and omits `DSRLNK` (`>1200`), `LDCSET`/`LDTSET`, and `FONT2`
  (`>1800`). Regenerate the table from the current source.
- `STATUS.md` header says "All planned milestones (M0–M6) are complete" but
  the table (correctly) includes M7 — update the header text. *(Fixed
  2026-07-02, same commit as §7.)*
- **Interface-data policy should be stated once, explicitly.** The README
  claims the OS "contains no Texas Instruments copyrighted bytes," while the
  image intentionally carries byte-identical *interface data* (the 512-byte
  font — commit `911695e` — the keytab blocks, the deflection table), gated
  by identity tests. Those two statements need reconciling in one paragraph
  (e.g., "original code + creative content; N bytes of uncopyrightable/
  functional interface data reproduced byte-identically for interoperability,
  enumerated in SURFACE-MAP.md"), so the claim is precise and B1 (more such
  data) has a stated home. Joel signs off on the wording — this document only
  flags the inconsistency, and B1 extends the *existing* practice, not the
  policy.

---

## Appendix — census method + the GROM 0 hazard list

Reference method (Phase A1 reimplements this in Rust; run in Git Bash from the
repo root):

```sh
od -An -v -tx1 roms/994AGROM.Bin                                | tr -s ' ' '\n' | grep -v '^$' > /tmp/auth.txt
od -An -v -tx1 original-content/system-roms/grom/console-grom.bin | tr -s ' ' '\n' | grep -v '^$' > /tmp/ours.txt
paste /tmp/auth.txt /tmp/ours.txt | awk '{a=$1;o=$2;addr=NR-1
  isao=(o=="00"&&a!="00")
  if(isao&&!r){r=1;s=addr}
  if(!isao&&r){if(addr-s>=8)printf ">%04X..>%04X (%d)\n",s,addr-1,addr-s;r=0}}'
```

GROM 0 authentic-only runs ≥ 16 bytes as of `ef0e245` (the Phase A1 work
list; runs ≥ 8 add ~45 more small fragments — the census tool owns the full
list). Known identifications marked; **all others need classifying**:

```
>0452..>0482 (49)    likely menu key-wait/beep code; authentic beep list at >0484 (L7)
>048F..>04B3 (37)    pre-font region
>0534..>0583 (80)  ┐
>05B5..>0693 (223) ┘ standard font >04B4–06B3 (fragmented by zero glyph rows) — B1
>06C9..>06E7 (31)  ┐
>0724..>0769 (70)  │ thin font >06B4–~0883 (7 rows/glyph) — B1
>0793..>0857 (197) ┘
>094D..>095F (19)   >0966..>0981 (28)   >09B8..>09C9 (18)
>09D8..>0ABF (232)  >0AC1..>0AE6 (38)   >0AE8..>0B2D (70)
>0B42..>0B5C (27)   >0B5E..>0BDD (128)  >0BF8..>0C44 (77)
>0C51..>0C6D (29)   >0C90..>0CCA (59)   >0CD1..>0CEB (27)
>0D06..>0D32 (45)   >0D59..>0D6D (21)   >0D73..>0D90 (30)
>0D9F..>0DB7 (25)   >0DC7..>0DF3 (45)   >0DFA..>0E21 (40)
>0E5B..>0E75 (27)   >0E77..>0E8E (24)   >0E90..>0EB6 (39)
>0F06..>0F2F (42)   >0F31..>0F44 (20)   >0F46..>0F78 (51)
>0FA5..>0FC8 (36)   >0FEB..>1007 (29)   >128D..>12BE (50)
>133E..>136E (49)   >1370..>13FC (141)  >13FE..>14AC (175)
>14B5..>14CC (24)   >14D7..>14F6 (32)   >14F8..>1513 (28)
>1528..>1547 (32)   >1549..>155C (20)   >155E..>1571 (20)
>158D..>159D (17)   >159F..>15C0 (34)   >15CB..>15F6 (44)
>164F..>1663 (21)   >1665..>1686 (34)   >1688..>1697 (16)
>16A2..>16B7 (22)   >16B9..>16C9 (17)   >16CB..>16DB (17)
```

(For orientation while classifying: authentic GROM 0 holds the boot/power-up,
master title, menu, KSCAN tables, the GPLLNK utility entries and small library
routines — the `>0Fxx–15xx` cluster is where the M2 recon saw menu/list code;
identify with `libre99gpl` disassembly + Nouspikel's GROM 0 documentation,
consulted never copied.)

*Census stats and run list regenerate with the appendix method; keep this
document's numbers pinned to commit `ef0e245` and let the census tool be the
living source of truth.*