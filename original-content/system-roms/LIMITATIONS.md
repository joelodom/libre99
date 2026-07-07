# ⚠️ KNOWN LIMITATIONS — the GROM rewrite

**Read this before shipping or demoing the rewrite.** These are the places where
the rewritten console GROM (`grom/console.gpl` → `grom/console-grom.bin`) does
**not** yet match the authentic firmware. Each entry says *what* is wrong, *why*
it happens, and a *suggested path forward*. They are collected here (rather than
scattered through the code) so a future pass can pick them up quickly.

These are **known, designed-in gaps**, not bugs. When investigating a reported
bug, check here first — if the symptom traces to one of these entries, the
answer is "known limitation, path forward documented," not a regression hunt
(see [`DEBUGGING.md`](./DEBUGGING.md) protocol Step 0).

Everything else works: the rewrite boots the genuine console ROM to an original
title, lists console GROM 1 (TI PYTHON) plus every cartridge program, and
launches GPL and ML cartridges (see `STATUS.md`, gates in
`crates/libre99-gpl/tests/`).

Numbering is stable — resolved items are kept below (not renumbered) so external
references stay valid.

> **Terminology.** Entries throughout this tree say "the bundled carts" — that
> is the **137-image differential test corpus**. Since the 2026-07-06 IP purge
> those images are *not* in the repository: they live in the git-ignored
> `third-party/` on development machines and load at run time (tests skip
> green when absent).

> **Open set (updated 2026-07-07):** the L1–L7 ledger is Resolved /
> deferred-by-decision (below), and the two entries the ship-review era added
> have both moved: **L8**'s launch symptom (Video Vegas to a dead console) is
> **resolved** — the XB substrate's ROM helpers cleared the wedge and the
> differential health panel passes **137/137 with an empty waiver list** (the
> GROM-2 library routine itself remains a stub with no known dependent; a
> gameplay eyeball is the final confirmation). **L9**'s Extended BASIC half is
> **resolved** — a user-supplied XB cartridge runs end-to-end on the clean-room
> pair via the census-bounded XB substrate ([`XB-CENSUS.md`](./XB-CENSUS.md)).
> What remains by design: **TI BASIC proper** doesn't execute under the rewrite
> (we ship TI PYTHON in BASIC's slot; the console GPL library and the ROM's
> PARSE/EXEC half stay deferred by policy), so TI BASIC needs user-supplied
> authentic ROMs, selected via `--system-rom` / `--system-grom`.

> **Ledger-closure complete** (the archived
> [`history/QUALITY-ASSESSMENT.md`](./history/QUALITY-ASSESSMENT.md) §7 plan —
> all six chunks landed 2026-07-02 → -07-04). Among its products: the byte
> census + [`grom/SURFACE-MAP.md`](./grom/SURFACE-MAP.md) enumerate and
> classify the full GROM-0 authentic surface (the finite contract behind L6),
> and the interface fonts ship at their authentic home addresses
> (`>04B4`/`>06B4`, B1). Execution log:
> [`history/QUALITY-ASSESSMENT-PROGRESS.md`](./history/QUALITY-ASSESSMENT-PROGRESS.md).

---

## ✅ Resolved

### L1 — QUIT + sound + all ISR-driven behaviour (was **MEDIUM**) · fixed 2026-07

Originally filed as "QUIT (`FCTN`+`=`) does not reboot to our title." The real
scope was larger and the root cause single: **our boot never enabled the 9901's
VDP interrupt (CRU bit 2), so the console ROM's VBLANK ISR never ran.** That one
gap killed QUIT *and* GPL sound lists (e.g. the Tunnels of Doom splash tune) *and*
sprite auto-motion *and* cursor/timers. The authentic GROM enables the interrupt
during boot with a GPL `IO` (CRU-output) instruction; the rewrite omitted it.

**Fix:** `console.gpl` `START` now initialises the ISR scratchpad cells (F3
contract) and issues the CRU-output `IO` to set bit 2 (`ST @>8300,>FF ; DST
@>8302,>0002 ; DST @>8304,>0100 ; BYTE >F6,>02,>03` — the last word writes the
count **and** the IO list's data-address byte at `>8305`, whose omission caused
the 2026-07-03 F5 regression; DEBUGGING.md case study 9). See
[`DEBUGGING.md`](./DEBUGGING.md) "no sound at Tunnels of Doom" for the full
trace. **Gates:** `tests/interrupts.rs`, `tests/f5_reset.rs`, and
`tests/sweep.rs::quit_returns_to_our_title`.

### L4 — keyboard table covered unshifted + shifted only (was **LOW**) · fixed 2026-07

`keymap.rs` originally supplied only the unshifted (`>1705`) and shifted
(`>1735`) blocks. Commits `b8ec02c` + `c966335` now emit **all four** ASCII
blocks — unshifted, shifted, **FCTN `>1765`**, **CTRL `>1795`** — plus the
`>17C8` joystick/split-keyboard table and the `>16EA` deflection table; the
built image is byte-identical to the authentic GROM across `>16EA–16FF` and
`>1760–17EF`. This is what made `FCTN` combos (backspace `>08`, the arrows)
and in-game key-unit-1/2 input decode. **Gates:** `tests/keyboard.rs`
(FCTN arrows, joystick table + deflections) and the keymap unit tests.
Unblocks L3's backspace. Case studies: `DEBUGGING.md` 4 and 5.

---

## L2 — far-list cartridges (`starpeg`, `xb25`) · ✅ **RESOLVED** (2026-07-03)

**Was.** Of the 137 bundled cartridges, two listed short — `starpeg` ("STARSHIP
PEGASUS") and `xb25` ("EXTENDED BASIC V2.5"): the menu showed TI PYTHON but not
the cartridge's own program. The menu scans each base by copying a **512-byte
window** of GROM/ROM into VRAM and walking the program list *in that window* (the
console ROM re-writes the GROM address per byte, so a full-slot copy of every
base would take ~70 frames — `RECON.md` §10). These two store their program list
far from the header (`starpeg` at slot `>7801`, `xb25` at `>6A01`), past the
window, so the walk-bound guard stopped before reaching them.

**Fix (the "bigger window for the outliers only" option).** `SCANW`'s walk bound
is now a scratch word `WBND` (`>835A`), and a helper `SFAR` reads each base's
program-list pointer (header `>1006`) up front: if it lands beyond the 512-byte
window the base re-copies its **whole 8 KiB slot** into VDP `>1000–2FFF` (all
free during the scan) and widens `WBND` to `>2FE0`, so the far list is reachable.
Immediate-source `MOVE`s only — no banned `MOVE` C=1 (RECON §7). Applied
uniformly to all cart bases (`>6000–E000` GROM and the `>6000` ROM window). Cost
≈ 12 frames, paid only by a base whose list is actually far. Both carts now
**list and launch**; deep sweep is **137/137**. Fast gates:
`tests/sweep.rs::sweep_farlist_starpeg` / `_xb25`; the `FAR_LIST_CARTS` exception
list is gone and `sweep_all_cartridges` asserts 137.

**Headroom note (a real trap, now fixed).** GROM 0 is a packed 6 KiB chip; the
menu code at `>0060` had only ~60 bytes of slack before the spliced beep (`>0484`)
and font (`>04B4`) blocks. The extra scan code pushed the menu **data** (incl. the
`SND`/`KBEEP` sound lists) past `>0484`, where the splices silently overwrote it —
so *every* beep ran away (the ISR read garbage as sound blocks and never hit the
list terminator; the menu's key-beep wait then hung, blocking launch). Fixed by
relocating the menu data block to the free gap above the thin font (`GROM >0880`,
`console.gpl`), giving the code ~1 KB of room. Lesson + trace: `DEBUGGING.md`
"runaway beep / GROM-0 splice clobber."

**Residual bound (documented, not a regression).** The widened window is **one
8 KiB slot**. A program list — or a `next` chain — that crosses into a *different*
slot than its base is still not followed (it would need per-pointer slot
re-copies). **No bundled cartridge does this** (the 137/137 sweep proves it); if
one ever appears it lists short exactly as before. The general follow-the-chain
form needs the banned computed-GROM `MOVE`, so it stays out until that (or an
equivalent) is execution-verified.

---

## L3 — TI PYTHON v0 gaps · ✅ **RESOLVED (2026-07-07) — the TI PYTHON v1 track, executed**

**Was.** Deferred by decision 2026-07-02 ("this project is about the emulation,
not TI PYTHON"): the v0 REPL had no backspace or line editing, dropped keys
under fast (rolled) typing, echoed control codes as junk glyphs, had no input
length cap, and knew only single-letter variables. The evaluator stack guards
landed early (2026-07-05, the `TOO COMPLEX` bound — QUALITY-EVALUATION §8.2
G1); everything else waited for the track to open.

**Joel opened the track 2026-07-07** (the spec of record:
[`docs/TI-PYTHON.md`](../../docs/TI-PYTHON.md)) and **v1 landed the same day**
(commits `cbbcdb2` P1, `7c2cae9` P2–P6):

- **Input engine (P1)** — the KSCAN new-key protocol (`>837C` bit `>20`, one
  event per changed key) replaces the wait-for-release loop, so rolled typing
  delivers every character; backspace (`FCTN`+`S` — the host Backspace) and
  ERASE (`FCTN`+`3`) edit the line; control codes are ignored, never echoed;
  input caps at the row edge; blank lines re-prompt.
- **The v1 language (P2–P6)** — four-row banner + `>>> ` prompt, a scrolling
  terminal screen with a blinking block cursor, **full-size names** (≤ 10
  chars, 32 slots in a VRAM table at `>1000–11FF`), **Python floor `/` `//`
  and divisor-signed `%`**, a real right-associative unary minus (v0 silently
  evaluated `2*-3` as `-3`), `print(…)` with string literals, `#` comments,
  and `exit()`/`quit()` back to the menu. Errors gained `NAME ERROR: <name>`
  and `MEMORY ERROR`.

Gates: the twelve-test suite in `crates/libre99-gpl/tests/ti_python.rs`
(reference session, overlapped-typing regression, editing, cap, scroll,
cursor, names, the floor/mod identity matrix, print/comments/exit, and the
kept stack-guard test). The banner still carries the workspace's one version
number by design (the `sysinfo_block` splice).

---

## L5 — menu appearance: no "SCANNING" cue, atomic reveal · ✅ **RESOLVED** (2026-07-07)

The console ROM re-writes the GROM address per byte (`RECON.md` §10), so the base
scan was assumed "visibly slow," and `MENU` once painted an original `SCANNING`
row (row 6) to mask the wait (2026-07-03). **Measurement corrected that premise.**
`tests/perf_parity.rs` shows the isolated menu-build segment is only ~7 frames
(~0.12 s), and our rewrite reaches the menu **sooner** than the authentic firmware
overall (reset → cart listed ~30 vs ~54 frames) — the banner was the only thing
that read as slow, and the authentic menu shows no such word. Two changes:

- **Cue removed.** `MENU` no longer draws it and `SGET` no longer erases it
  (`console.gpl`; the `SCANT`/`BLANK8` data are gone).
- **Atomic reveal.** So the per-byte scan does not paint program lines in one at a
  time, `MENU` now blanks the display (VDP R1 `>A0`) before the scan and reveals
  it whole (`SDONE`/`DISPON`, R1 `>E0`) only once every entry is drawn — the exact
  idiom the title screen already uses (`START` draws blanked, then `DISPON`). Safe
  to hold R1 off across the multi-frame scan: the console ISR rewrites R1 only when
  the screen-timeout wraps (~32k frames off, reset by the title keypress) or when
  KSCAN sees a new key (none during the scan), and `>83D4` (the display-on image)
  is left `>E0` for that later un-blank.

Narrowing the scanned bases was considered and rejected — the 2-byte peek already
skips empty bases cheaply, and a window-size change is not worth ~0.12 s against
the 137-cart enumeration gate. Guards:
`tests/menu_cue.rs::menu_builds_with_no_scanning_cue` and
`::menu_reveals_atomically_with_full_list`.

---

## L6 — GPLLNK / console GPL service surface · ✅ **RESOLVED** (2026-07-04)

**Was.** The authentic console GROM 0 holds an **interconnect table** at
`>0010-0037` (twenty GROM-2 vector slots) and a fixed **GPLLNK / XMLLNK service
grid** at `>0038-005F`, both reaching a shared GPL library in GROM 2
(`>4000-5FFF`). The rewrite shipped none of that library, left the interconnect
table zero, and pointed the service grid at a `B >0020` reboot stub — so a stray
service call rebooted to our title, and a call past `>0049` ran zeroed table
bytes as GPL. The first carts to *hard-depend* on the gap broke: Tunnels of
Doom's disk load (`CALL >0010`) ran off into garbage, and TI Invaders' opening
text rendered blank (`CALL >0016/>0018`) while its sprites still showed.

**The in-contract surface is now closed for the bundle.** Chunk 1 enumerated the
whole finite contract in [`grom/SURFACE-MAP.md`](./grom/SURFACE-MAP.md); every
service a bundled cartridge actually calls is either implemented or degrades
gracefully, and the coverage sweep (137 carts,
[`grom/COVERAGE-REPORT.md`](./grom/COVERAGE-REPORT.md)) proves it: **"Carts that
rebooted to our title after launch: none."**

**Implemented** (original, clean-room, byte-verified against the traced
interface — never from TI's GROM bytes):
- **`>0010` DSRLNK** — the device-I/O link; parses the PAB device name and
  delegates to the kept console ROM via `XML >19`/`>1A`. Lives at GROM `>1200`.
  **Disk (DSK1) loads**: Tunnels of Doom reads a QUEST scenario from
  `Tunnels.Dsk` and reaches `NEW DUNGEON`. Gate
  `crates/libre99-gpl/tests/device_io.rs`; interface `RECON.md` "Console device
  I/O"; `DEBUGGING.md` case study 2; reproduce with `cargo run -p libre99-gpl
  --example tod_disk_probe`.
- **Boot peripheral power-up scan** — `START` sets `>8370 := >3FFF`, then runs
  each card's DSR power-up (`XML >19`/`>1A` with `>836D := >04`) so the disk
  card reserves its VRAM buffer (lowering `>8370` to `>37D7`); without it the
  load stalled at 0 sectors. Gate `tests/device_io.rs`; `RECON.md` "Peripheral
  power-up."
- **`>0016` LDCSET / `>0018` LDTSET** — the two console character-set loaders. A
  cart points `>834A` at a VDP pattern-table address and CALLs the slot to have
  the console fill 64 glyphs there. `>0016` copies the standard set (`FONT`);
  `>0018` copies the thin "small" set (`FONT2`, shipped pre-expanded so one
  `MOVE` reproduces the authentic 7-rows-per-glyph loader). TI Invaders' text now
  draws. Gate `crates/libre99-gpl/tests/char_set.rs`; `RECON.md` "Console
  character-set loaders"; `DEBUGGING.md` case study 3.
- **`>0020` START** — the fixed boot GPL entry (slot 8), the reset target.

**Degrades gracefully (the rest of the surface), proven by the coverage sweep.**
- The `>0010-0037` interconnect table is now **twenty executable `BR` stubs** (no
  longer zero): the four implemented slots above; every other slot → `ILRTN` (a
  clean `RTN`).
- The `>0038-005F` GPLLNK/XMLLNK grid is **`B SVCBAD`** stubs — a breadcrumb
  (`>835E := >EE`) then a **graceful `RTN`** (`SVCBAD` at GROM `>1201`; a 1-byte
  `RTN` pads `>005F`) — **not** a reboot, with one entry now a real service:
  **`>004A` is the lower-case (small capitals) character-set loader** (`LDLSET`,
  2026-07-06), which 26 bundled carts CALL — Parsec stages its in-game
  small-caps text through it, and the no-op stub rendered that text as leftover
  full-size garbage. The grid closed two hazards at once: the head `>0038-0049`
  used to reboot (it regressed Parsec mid-game) and the tail `>004A-005F` used
  to run zeros (QUALITY-ASSESSMENT.md §5 item 7; `DEBUGGING.md` case study 10).
  Gates: `crates/libre99-gpl/tests/coverage_sweep.rs`, and
  `crates/libre99-gpl/tests/char_set.rs` for `>004A` byte-equality with the
  authentic console.

**SERVICE-ENTRY disposition.** SURFACE-MAP left **4 authentic-only SERVICE-ENTRY
runs (43 bytes)** beyond the implemented services; all are now closed:
- `>004A..>0057` (GPLLNK grid tail) → the `SVCBAD` grid (graceful RTN), safe by
  the sweep — and `>004A` itself has since been **implemented** (the lower-case
  character-set loader, see above).
- `>043B..>0444` (slot `>0012` target: an internal GPL sub-stack unwind helper
  the cassette DSR calls) → **out of contract**; our `>0012` → `ILRTN`.
- `>0446..>0450` (slots `>001A/>001C/>001E`: `B` trampolines into GROM-1 TI
  BASIC) → **out of contract** — we ship TI PYTHON in GROM 1, not BASIC; our
  slots → `ILRTN`.
- `>1310..>1317` (cassette DSR-list CS1 entry) → reached only via the GROM-0
  header offset `>08`, which our header ships `>0000`, so it is **unreached**;
  cassette is deferred (below).

**Deferred by decision (not open gaps).**
- ⚠ **Cassette (CS1) has no emulator hardware** (`crates/libre99-core/src/cru.rs`;
  emulator ROADMAP §6). The 1981 *no-tape* behaviour is correct and **verified
  (2026-07-03)**: `DSRLNK("CS1")` and a garbage device both **fail gracefully** —
  the kept ROM's `XML >19/>1A` return the DSR error, the cart shows `DEVICE
  ERROR`, and the console stays alive; never a hang — so Tunnels of Doom's "LOAD
  DATA FROM → CASSETTE" errors and recovers like the real machine. Closes
  QUALITY-ASSESSMENT.md §5 item 6 (probe `examples/dsrlnk_baddev_probe.rs`, gate
  `tests/device_io.rs::bad_device_errors_gracefully_without_hanging`).
- The **full shared GROM-2 GPL library** is not shipped. Only what the bundle
  exercises (DSRLNK, both char-set loaders, the boot power-up) is implemented; the
  other `>0010-005F` entry points are graceful stubs. ⚠ **The original claim here —
  "no bundled cart needs it, every cart runs" — was one cart too strong** (see the
  Correction below and **L8**): the reboot-only gate that "proved" it could not see
  Video Vegas, which *does* hard-depend on a GROM-2 routine and launches to a dead
  console. So this is a real bundle gap for at least one cart, and possibly more via
  un-exercised code paths — enumeration is L8 future work.

**Why Resolved.** The in-contract service surface is complete for the bundle:
every entry has an implementation or an evidence-backed graceful degradation,
with zero reboots across 137/137 carts. The two residuals are **deferred by
decision** — cassette (no hardware → ROADMAP §6) and the unshipped GROM-2 library
(no bundled cart needs it) — which, under the plan's finish-line rule of *zero
open-and-unworked entries* (QUALITY-ASSESSMENT.md §7.6/§7.8; L3 is the deferral
model), are not open. Closes QUALITY-ASSESSMENT.md §7.4 / chunk 5.

> **Correction (2026-07-04, ship review).** "No bundled cart needs the GROM-2
> library" turned out to be **one cart too strong**. L6's evidence was the
> coverage sweep's *reboot* check ("zero reboots across 137/137"); when that gate
> was strengthened into a **differential health panel** (does the console stay
> alive after launch, vs. authentic?), one bundled cart — **Video Vegas** — was
> found to launch to a *dead console* (not a reboot) because it hard-depends on an
> unshipped GROM-2 routine. That is a genuine open gap, now tracked as **L8**. The
> rest of L6 stands: the service *surface* is graceful for all 137, and only this
> one cart needs a library routine added on demand.

---

## L7 — menu key-beep parity · ✅ **RESOLVED** (2026-07-02)

TI's menu beeps on each keypress: it points the ISR's sound-list cells
(`>83CC/D`) at a short list in GROM (the authentic one lives at `>0484`, 3
bytes + terminator) and then polls `>83CE` until the beep finishes. Note that
GPL sound *itself* works — cartridges' sound lists play fine since the ISR
was armed (resolved L1); this is only about our menu's own chirp.

**Resolved (2026-07-02).** The menu beeps: `console.gpl` arms `KBEEP` (an
original short click list) when leaving the title (`TREL`) and on a valid
selection, polling `>83CE` until the click drains (`SBWAIT`). The last gap —
*rejected* keypresses — is now closed: a differential probe
(`crates/libre99-gpl/examples/menu_beep_probe.rs`) confirmed the authentic menu
**beeps on an out-of-range key**, so `SGET`'s two reject branches now route
through `SBAD`, which arms the same `KBEEP` click before re-prompting (no wait —
the click plays while polling for the next key). Gate:
`crates/libre99-gpl/tests/interrupts.rs::menu_beeps_on_rejected_key`. Plan:
[`history/QUALITY-ASSESSMENT.md`](./history/QUALITY-ASSESSMENT.md) §7.5.

---

## L8 — Video Vegas launches to a dead console (unshipped GROM-2 library) · ✅ **symptom cleared 2026-07-07** (incidentally, by the XB substrate; the library routine itself is still a stub)

> **Update (2026-07-07).** The dead-console regression **no longer
> reproduces**: the 137-cart differential health panel now passes with an
> **empty waiver list** (`coverage_sweep.rs` — Video Vegas ends display-on
> with the ISR ticking, like authentic). Nothing was done *for* this cart —
> the **XB substrate** (`XB-CENSUS.md`) populated console-ROM addresses that
> were zeros in our layout, and Video Vegas's data-driven launch divergence
> ("a console value it reads is non-zero under authentic, zero under ours" —
> below) evidently read one of them. The `>002C`/`>0032` interconnect slots
> are **still graceful `ILRTN` stubs**; a "console alive" verdict is not a
> gameplay verdict, so a GUI eyeball of actual play is the remaining
> confirmation, and the original analysis below stays for the record. If the
> symptom ever returns, the panel fails loudly by name.

**Symptom (as filed).** Selecting `2 FOR VIDEO VEGAS` from the menu launched to
a **dead console** under our GROM: the display turned off (VDP R1 = `>05`) and
the 9901 VDP interrupt was masked, so there was no ISR, no sound, and QUIT was
dead. Under the authentic GROM the same cart runs. Video Vegas
(`VideovegasC.ctg`) was the **only** one of the 137 bundled carts regressing.

**How it surfaced.** The 2026-07-04 ship review strengthened the coverage sweep's
post-launch check. The old gate asserted only *"did not reboot to our title,"* and
Video Vegas does not reboot — it wedges — so it passed. The new
[`tests/coverage_sweep.rs`](../../crates/libre99-gpl/tests/coverage_sweep.rs)
launches every cart under **both** our GROM and the authentic one and asserts,
*differentially*, that our console is never *less alive* (display on + ISR
ticking) than authentic. That caught it. (The check is differential because ~17
bundled arcade carts legitimately take the machine over and freeze the console
ISR themselves under *both* firmwares — only a cart the *console* leaves dead is a
bug.)

**Root cause.** Video Vegas hard-depends on a console **GROM-2 GPL library
routine** we do not ship. Its data-driven launch path diverges early (a console
value it reads is non-zero under authentic, zero under ours) and it CALLs
interconnect slots `>002C`/`>002D`/`>0032`/`>0033`, which vector into the GROM-2
library under authentic but are graceful `ILRTN` no-ops under ours. Without the
routine's side effect it runs on and disables the display. This is exactly the
on-demand gap L6 anticipated ("a future cart that hard-depends on some other
console GPLLNK routine would need it added on demand") — realised here by a
*bundled* cart, so it is tracked as its own open entry rather than folded into
L6's "Resolved."

**Why OPEN, not deferred-by-decision.** It clears neither bar: it is a bundled
cart's primary flow broken with the cause on our side (QUALITY-ASSESSMENT §8 Tier
2 = fix), and unlike cassette / the general GROM-2 library it is *reached by a cart
in the bundle*. So it is a real open gap, honestly counted.

**Path forward for *this* cart (one routine, not the whole library).** Disassemble
the authentic `>002C`/`>0032` slot vectors (`libre99gpl dis roms/994AGROM.Bin >002C`),
identify the GROM-2 routine and its side effect (cross-ref Nouspikel — consult,
never copy), implement it as original GPL, and add a per-entry differential
probe/gate (the char-set-loader pattern, `DEBUGGING.md` case study 3). This is the
first concrete instance of the deferred GROM-2 library work — sized at one routine.

### The broader gap this is an instance of

The **full shared GROM-2 GPL library is not reimplemented in our clean-room
firmware** (see L6). Our GROM 2 is ~94% zeros — the only content is `FONT2` at
`>4000`; the ~5.5 KiB of authentic console utility routines are absent, and the
`>0010-005F` entry points that vector into them are graceful stubs (`ILRTN` /
`SVCBAD`). A cart that merely *calls* a stub and carries on is fine (the coverage
sweep shows ~16 bundled carts do exactly this); a cart that *depends on the
routine's side effect* — like Video Vegas — wedges. **How many other carts depend
on it is not fully known**, because the coverage sweep only exercises code paths
reached by a scripted launch + brief attract; a dependency behind a menu option, a
later level, or a specific input would not have been triggered.

### Future work — systematically enumerate the dependents (a static call-scan)

Proposed but **not yet built** (assessed 2026-07-04). Statically scan every
cartridge's bytes for control-transfers into the stubbed `>0010-005F` entries, to
find dependents the dynamic sweep's un-exercised paths miss. Feasibility and
difficulty, so the trade-offs are not re-derived later:

- **GPL carts, cheap byte scan — EASY (~1 hr).** A `CALL` to a slot is the 3-byte
  GPL sequence `06 00 XX` (opcode `>06` = `CALL`, 16-bit absolute target — see
  `crates/libre99-gpl/src/isa.rs`; `B` = `05 00 XX`). Scan each cart's GROM for
  `06 00 XX` where `XX` is a stubbed entry (all of `>0010-005F` except the four we
  implement: `>0010`, `>0016`, `>0018`, `>0020`). Trivial, but **over-reports** —
  GPL interleaves code and data with no markers, so a data byte `06` before `00 2C`
  reads as a `CALL >002C` that is not code. A screening signal, not proof.
- **GPL carts, precise scan — MODERATE-TO-HARD.** Cut false positives by
  disassembling *along control flow* from each cart's program-list entry points
  (recursive descent) instead of raw bytes — real work (computed branches,
  embedded data), and it still cannot see **computed/indirect** calls, so it also
  under-reports.
- **ML carts — HARD and unreliable.** ML carts reach console routines via
  `BLWP @GPLLNK` + a `DATA >xxxx` word amid interleaved code/data, banking, and
  indirection; static scanning is lossy, and many arcade ML carts take the machine
  over and never use the mechanism.
- **Fundamental limit — this is triage, not a verdict.** "Calls a stub" is neither
  necessary nor sufficient for "breaks under ours": ~16 bundled carts call stubs
  and run fine (over-approximation), and Video Vegas's *root* dependency was a
  **data-driven divergence** (it read a console value zero in ours and branched) —
  a call-scan flags the symptom but would miss a cart that diverges on console data
  without ever calling a stub (under-approximation). The dynamic coverage sweep
  already gives the reliable **executed-path** version of this (its service-surface
  table records which slots each cart actually fetches); a static scan only adds
  **unexecuted-path** candidates.
- **Recommended shape when pursued:** the easy GPL byte scan as a *screen*,
  intersected with the dynamic coverage data — treat intersections as "confirmed
  reaches a stub, launch and watch," and static-only hits as "worth a look." Skip
  the precise disassembler and the ML scan unless a specific cart demands it. This
  narrows the manual test list from 137 to a handful; it does not, by itself,
  decide compatibility.

### Known impact

- **Confirmed:** 1 bundled cart (Video Vegas) launches to a dead console. Its
  primary flow is broken end-to-end under our clean-room GROM.
- **Possible, unquantified:** other carts (bundled or third-party) that depend on a
  GROM-2 routine's side effect via a code path the launch+attract sweep did not
  reach. Bounded above by "carts that call a stubbed `>0010-005F` slot" — the
  call-scan above would enumerate the candidates.
- **Not affected:** carts that only call the four implemented services (DSRLNK,
  the two char-set loaders, START) or that call a stub without depending on its
  side effect (~16 bundled carts, sweep-confirmed to run). The **authentic GROM
  runs all of them** (selected via `--system-grom roms/994AGROM.Bin` — the rewrite
  has been the emulator's default since 2026-07-06), so this is a limitation of the
  *clean-room rewrite firmware*, not of the emulator.

**Meanwhile it is gated, not silent.** `coverage_sweep.rs` waives Video Vegas by
name (`KNOWN_ISR_REGRESSIONS`) so the differential health panel still guards the
other 136 carts and would fail the moment a *second* cart regresses. Reproduce
with `cargo run -p libre99-gpl --example isr_regression_probe`.

---

## L9 — TI BASIC / Extended BASIC under the rewrite · ✅ **Extended BASIC RESOLVED (2026-07-07, the XB substrate)**; TI BASIC proper stays deferred by policy

**Was (Joel, 2026-07-06).** Under the clean-room pair, Extended BASIC reached
`READY` and echoed typing, but entered lines did **nothing** — no output, no
`SYNTAX ERROR`. The working theory blamed the unimplemented console GROM-2
"BASIC-era GPL library" (~5.5 KiB) plus the TI BASIC interpreter core — "a
whole interpreter's worth of console services."

**The theory was wrong, and the F0 census proved it**
([`XB-CENSUS.md`](./XB-CENSUS.md), 2026-07-07). Measured with the GROM
read-coverage and new CPU PC-coverage instruments across a scripted XB session
under the authentic pair: XB touches **no console-GROM BASIC code at all**
(header bytes only), tolerates the one stubbed interconnect slot it calls
(`>0032`), and needs exactly **five tiny console-ROM helpers (~200 bytes)** it
calls **directly by address** from its cartridge ROM — a symbol-chain search
and four VDP/stack primitives. Everything else it uses (GPL interpreter,
KSCAN, DSR search, the whole radix-100 FP + conversion package) the rewrite
already shipped.

**Resolved for XB: the five helpers are implemented** — original code at the
pinned authentic addresses (`SYMSRC >15E0`, `RDCELL >187C/>1880`, `RDVAL8
>1890`, `WRWORD >18AA/>18AE`, `STKON/STKOFF >1E7A/>1E8C`, `VPOPAG >1FA8`; the
**XB substrate** section of [`rom/console.asm`](./rom/console.asm), commit
`0e692eb`). **Extended BASIC now runs end-to-end on the default clean-room
boot**: `PRINT "HELLO"` prints, floats assign and compute (`X=1.5` /
`PRINT X*2` → `3`), programs store, `RUN` executes, `LIST` lists —
screen-identical to the authentic pair. Gates:
`crates/libre99-asm/tests/xb_substrate.rs` (per-helper differential
microtests) and `crates/libre99-gpl/tests/xb_smoke.rs` (the end-to-end
session, with an authentic differential leg).

**What remains deferred (by policy, unchanged):** **TI BASIC proper** — the
console GROM 1/2 interpreter (the `1 FOR TI BASIC` experience; TI PYTHON
stands in that menu slot) and the ROM's PARSE/CONT/EXEC/RTNB half with the
`>1C9C` tables and the `XML >13-18`/`>1B` symbol entries (still loud stubs).
A cartridge that leans on more of the BASIC half than xb25 does would need
its own census-first pass — `XB-CENSUS.md` §6 records the candidates
(`PGMCH`, `SMB`/`SYM`, `VPUSH`) and the method. The authentic TI images
remain selectable via `--system-rom` / `--system-grom` for TI BASIC itself.

**Verification scope note.** "Extended BASIC" here is `xb25.ctg` (Extended
BASIC V2.5), the XB in the local third-party media set — never committed to
this repository. The census instrument
(`cargo run -p libre99-gpl --example xb_census`) re-measures any other
XB-family cartridge in one command.

---

*Keep this file current: when a limitation is fixed, move its entry to a
"Resolved" note with the commit, and update `STATUS.md`.*
