# Known issues & authentic quirks

Behaviors users may report as bugs that are actually faithful reproductions of
the real TI-99/4A, plus any genuine open issues. When something here is authentic
hardware behavior, the disposition is **never** to break faithful emulation — we
add an opt-in frontend preference (default off) instead.

---

## FORMAT under the *authentic* disk DSR (`--disk-dsr roms/Disk.Bin`) cannot lay sectors (emulator limitation)

The genuine TI DSR formats a disk with the FD1771 **Write Track** command,
which the emulated controller does not implement
(`crates/libre99-core/src/disk.rs` — `>E0`/`>F0` are no-ops), so formatting under
the authentic DSR silently does nothing. The **default clean-room DSR is not
affected**: its FORMAT re-initializes the mounted image via Write Sector (the
documented substitution, `original-content/system-roms/disk-dsr/`
DSR-REWRITE-PLAN §0), and the result is validated by the authentic DSR reading
and writing the formatted disk (`dsr_format_cross_oracle`). If Write Track is
ever added to the controller, the authentic path starts working too.

---

## ⚠ Extended BASIC / TI BASIC programs don't run on the clean-room ROMs — boot the authentic ROMs for BASIC (by design)

**Symptom.** Under the clean-room firmware (**the default** since 2026-07-06),
Extended BASIC starts and you can type at the `READY` prompt, but
**entered lines don't execute** — `PRINT "HELLO"` produces no output, and a
nonsense line produces no `SYNTAX ERROR`. The same applies to TI BASIC and other
TI-BASIC-based cartridges.

**Why — this is by design, not a regression.** Our clean-room console rewrite
**deliberately does not reimplement TI BASIC** or the shared **BASIC-era GPL
library** it uses (it ships TI PYTHON in that menu slot). Extended BASIC brings up
its own prompt but hands each entered line off to those console routines to
tokenize and execute — and under our firmware they aren't there, so nothing
happens (no output, and no error either, because the routine that would report
`SYNTAX ERROR` is part of the same missing library). Full detail:
[`original-content/system-roms/LIMITATIONS.md`](../original-content/system-roms/LIMITATIONS.md)
**L9**.

**Workaround — the authentic ROMs run BASIC perfectly.** Boot them explicitly:

```bash
cargo run --release -p libre99-app -- --system-rom roms/994aROM.Bin --system-grom roms/994AGROM.Bin
```

Making BASIC run under the clean-room firmware is a major future milestone
(M6, deferred indefinitely by policy — see [`docs/ROADMAP.md`](ROADMAP.md) and
[`LIMITATIONS.md`](../original-content/system-roms/LIMITATIONS.md) L9).

**Not caused by the keyboard fix.** This surfaced right after the 2026-07-06
Extended BASIC lowercase keytab fix — that fix only made typing work well enough
to reach the point where XB tries to execute; the execution gap is pre-existing.

---

## Idle screen goes blank after ~9 minutes (AUTHENTIC — not a bug)

**Report.** "Leave the emulator idle on the title screen for a few minutes and
the screen goes blank except for the background color; pressing a key brings it
back, and the keypress is honored."

**Verdict.** This is the genuine console ROM's anti-burn-in screen-timeout —
its level-1 VBLANK ISR (`>0900`, see
[`original-content/system-roms/rom/RECON.md`](../original-content/system-roms/rom/RECON.md)
§6) does the blanking itself; nothing on the emulator's side is wrong. It
happens under the authentic ROM and under the clean-room rewrite alike (the
rewrite reproduces the ISR's timeout contract, `>83D6`/`>83D4`).

**Mechanism (measured against the authentic ROM + GROM).**
- Each VBLANK tick the ISR `INCT`s a 16-bit counter at scratchpad **`>83D6`**
  (INTWS R11), i.e. **+2 per frame**. Observed climbing exactly +2/frame from a
  cold-boot title value near `>0194`.
- When that counter **wraps to `>0000`**, the ISR rebuilds VDP **register 1**
  from its copy at **`>83D4`** (= `>E000`) with the **display-enable bit (bit 6,
  `>40`) cleared** and writes it. R1 goes `>E0` → `>A0`; the picture drops to the
  backdrop color only. (Confirmed: R1 flips to `>A0` at the exact frame `>83D6`
  reaches `>0000`.)
- **KSCAN** (`>02B2`, RECON §5), on detecting a key, reloads R1 from `>83D4`
  (un-blank) **and resets `>83D6`** — which is exactly "press a key, screen
  returns, key honored." (Confirmed: after a blank, injecting a key restored
  R1 `>A0` → `>E0` and reset `>83D6` `>0000` → a small value.)

**Measured timeout.** From a cold-boot title (counter ≈ `>0194`) the display
blanked after **32566 idle frames = 542.8 s ≈ 9.05 min**. From a fully reset
counter it is the full period `65536 / 2 = 32768 frames ≈ 546 s ≈ 9.10 min` at
~60 Hz. The timing is **faithful** — the counter runs at the ROM's own rate; our
~60 Hz frame pace (`app.rs FRAME = 16667 µs`) is within 0.13 % of the real NTSC
field rate. A user's "about five minutes" is a rough underestimate of the same
~9-minute event, not evidence of a fast counter. (Note: because `reset()` (F5)
preserves RAM, a game that left `>83D6` high can make the next title blank sooner
— see `crates/libre99-gpl/examples/blank_timeout_probe.rs`, HAZARD-1.)

**Remedy — implemented, opt-in, default off.** A preference
`defeat_screen_blank` (default `false`) in `~/.libre99/libre99.toml`
(`crates/libre99-app/src/config.rs`). When `true`, the frontend's frame loop
(`crates/libre99-app/src/app.rs`, `RedrawRequested`) resets `>83D6` to `0` after
each advanced frame — the same thing a keypress does — so the counter never wraps
and the picture never blanks. Default behavior is unchanged and remains faithful.
Verified: 34000 idle frames (past the natural blank point) with the reset never
blanked (R1 stayed `>E0`).

**Cross-check.** Classic99 runs the same ROM and exhibits the same ISR-driven
blank; this is inherent to the authentic firmware, not an emulator artifact.

---

## FIXED (2026-07-06, pending visual confirmation) — Parsec: in-game "PRESS FIRE TO BEGIN" cut off / random full-size characters

**Reports (Joel, from play, under the Libre99 rewrite GROM).** 2026-07-05: "The
flashing text while playing Parsec is cut off." 2026-07-06: in-game, "the small
caps message PRESS FIRE TO BEGIN is a garble of other characters." 2026-07-06,
after the beam-rendering fix below: "the problem persists… PRESS FIRE TO BEGIN
is small caps and the random characters are full sized" — the observation that
cracked it.

**Root cause — our system GROM stubbed the lower-case character-set loader
(fixed GPLLNK service `>004A`).** On a real 99/4A, `>004A` loads the console's
lower-case **small capitals** set (31 glyphs, `>60..>7E`; authentic loader
`>03C2`, data home `>0874`) to the VDP address in `>834A`. Parsec CALLs it at
game start to stage those letterforms, then copies glyphs into its own bitmap
character cells for the in-game prompt ("PRESS FIRE TO BEGIN", row 20,
Parsec-specific codes P=`>7F` R=`>8D` E=`>5C` …). Our rewrite's `>004A` was a
graceful no-op stub (`B SVCBAD`), so the staging area kept whatever was there —
the previously-loaded **full-size standard set** — and Parsec copied *that*:
some prompt letters rendered as random full-size character fragments (R showed
a "2", E a "%", S a "3") and others as **blanks** (B, G) — "cut off." The
committed coverage report had the lead all along: **26 bundled carts CALL
`>004A`** (Parsec, MISSION, Saguaro, Barrage, …).

**The fix (2026-07-06): implement the service.** `LDLSET` at `>004A` in
`console.gpl` mirrors the authentic loader's traced interface (blank top row +
seven stored rows per glyph, 31 glyphs to `[>834A]`, `>834A` advanced by
`>F8`); the glyph data ships byte-identical to the authentic GROM at its
documented home `>0874` (`LOWERA`; the expanded loader block `FONT3` at GROM 2
`>4200`), and our menu data moved `>0880`→`>0950` to clear the home. Proof of
causality, all headless: with the stub, the prompt-cell glyphs at the
visible-prompt moment are standard-font fragments/blanks; with `LDLSET` they
are **byte-identical to the authentic console's**, all eleven codes.

**Regression gates.** `libre99-gpl/tests/char_set.rs`
(`lower_case_loader_matches_the_authentic_console`: a synthetic cart performs
Parsec's exact staging call under both firmwares — 248 staged bytes and the
`>834A` side effect must match, and must equal the shipped font);
`libre99-gpl/src/font.rs` (`matches_authentic_lower_set`: byte-identity with
authentic `>0874`); `libre99-gpl/tests/census.rs` (the `>0874-094C` home is
DATA-MUST-MATCH, which also guards the splice layout); the sweep's font-home
safety invariant now covers `>0874`.

**Related but distinct: beam-accurate scanline rendering (also 2026-07-06,
`bd1bbb6`).** The first fix attempt rebuilt the render model — whole-frame VRAM
snapshots with a slice-start interrupt became a 262-line beam walk with the
interrupt at end-of-active-display (Classic99-verified timing), gated by
`libre99-core/tests/beam.rs`. That change is hardware-true and stands on its own
(flashing/screen-split/race-the-beam class), but it was **not** this garble's
cause: the garble was firmware, and reproduces identically under the beam
renderer with the `>004A` stub patched back in.

**Pending: Joel's GUI eyeball** — play Parsec **under the rewrite GROM** (now the
default, so no flag needed; the authentic GROM, which never had this bug, is the
`--system-grom` opt-out) and confirm the in-game prompt reads PRESS FIRE TO BEGIN
in small caps.

<details><summary>Diagnosis history (kept for the record; the two “rules out
fonts/character-set” conclusions below were wrong — they compared the title
screen and mid-animation states, never the prompt cells at the visible-prompt
moment, and used Graphics-I pattern addressing against what is a bitmap-mode
screen)</summary>

**Where to start (leads from `docs/history/QUALITY-EVALUATION-2026-07-05.md`):**
1. **Bisect against the 2026-07-05 P0 VDP fixes first** (`f8abe12` status-flag
   lifecycle, `72506f8` sprite-Y wraparound, `575c0ae` transparent-sprite
   coincidence, `fb8dc3f` evaluate-at-vblank). These deliberately changed
   sprite/status behavior that games react to — Parsec polls sprite
   coincidence during play — so establish whether the symptom reproduces at
   `cb5f005` (pre-fix) before assuming it is pre-existing.
2. **If pre-existing**, the prime suspects are the two known render-model
   gaps: the Graphics II (bitmap) table-masking path has **zero test
   coverage** (report §5 gap 6 / action item P1.6), and rendering is
   whole-frame — mid-frame VRAM/register updates are invisible (report §3.2
   A1), which is exactly how flashing/partial-text effects get mangled at
   frame granularity.
3. **Repro/compare:** `cargo run -p libre99-app -- --cartridge third-party/cartridges/Parsec.ctg`, play to
   the flashing in-game text, screenshot; run the same scene in Classic99
   (`C:\ClaudeShared\classic99`) side by side. Note which characters/rows are
   missing (horizontal truncation points at table masking; missing flash
   phases point at frame-granular rendering).

**Update (2026-07-06, differential probe — narrows it to lead #2, rules out
fonts).** A differential launch of Parsec under our GROM vs the authentic GROM
(diffing the live VDP name + pattern tables) shows:
- **The *title* is fine.** The attract title renders `PARSEC` / `PRESS ANY KEY
  TO BEGIN` / `©1982 TEXAS INSTRUMENTS` **byte-identically** under both GROMs —
  same name-table placement, same glyphs — except the one code `>0A` (the ©
  symbol, our deliberately-different ringed-C, `SURFACE-MAP.md`). So the title
  is **not** garbled by our firmware and is **not** a character-set/GROM issue.
  (Joel's 2026-07-06 "PRESS FIRE TO BEGIN garbled" is this screen — which reads
  "ANY KEY", not "FIRE", and renders correctly here — so the symptom he saw is
  either the *in-game* flashing text below or an older build.)
- **In-game, the only diffs are animated graphics, off by one animation phase.**
  Once play starts the sole differing code is an animated tile (e.g. `>60`)
  whose pattern is rewritten every frame; our copy vs authentic differ only in
  the last row, i.e. sampled one animation step apart — a **timing/frame-
  granularity** artifact, not a font or GROM defect. This **reinforces lead #2**
  (whole-frame rendering mangles flashing/partial effects) as the prime suspect
  and **eliminates the character-set angle**.
- **Not related to the Extended BASIC lowercase issue below** — Parsec's text is
  stored/emitted in uppercase (`>50 >52 >45…`) and never touches the lowercase
  small-caps path, so the two reports do **not** share a root cause.

**Deeper diagnosis (2026-07-06, of the in-game "PRESS FIRE TO BEGIN" garble).**
Reproduced the in-game screen headlessly and diffed our GROM vs the authentic one
frame-by-frame into the game:
- **It's Parsec's own custom-font text, not ASCII and not a console font.** "PRESS
  FIRE TO BEGIN" is drawn on **row 20** of Parsec's name table with Parsec-specific
  codes (P=`>7F`, R=`>8D`, E=`>5C`, S=`>8E`, F=`>5D`, I=`>6C`, T=`>8F`, O=`>7E`,
  B=`>4D`, G=`>5E`, N=`>7D`) into a font Parsec loads at pattern base `>1800`. (The
  earlier "stored uppercase" note was about the *title* "PRESS ANY KEY TO BEGIN" —
  separate ASCII, renders fine.)
- **Our clean-room GROM is NOT feeding Parsec corrupt data.** In every stable
  captured state the message renders **identically** under our GROM and authentic —
  same name-table codes *and* byte-for-byte the same glyphs. The font Parsec uses is
  faithful under our firmware; this is **not** a GROM-data/font-reproduction bug.
- **The only diffs are timing/phase artifacts**, not data: a font copy caught
  mid-flight in the load window (`>5E`/`>5F` differ for a few frames, then match)
  and the animated `>60` tile off by one step — both because the two launches aren't
  cycle-aligned, so they sample the same animation at different phases.
- **Conclusion — a rendering-timing issue, not firmware/data.** This is the
  whole-frame rendering gap (report §3.2 A1 / lead #2 above): Parsec draws/flashes
  the message with **mid-frame** VRAM writes, and our once-per-frame renderer can
  latch it **mid-update**, showing a garble of intermediate character codes. It is
  frame-phase-sensitive — clean at some moments, broken at others.
- **Not stably reproduced headlessly** — the scripted launch lands in a semi-attract
  state and the captured frames render clean. To pin the exact trigger: a **GUI
  screenshot** of the garble (to compare the shown codes/glyphs against the expected
  P‑R‑E‑S‑S‑…), or implement **finer-than-whole-frame rendering** (scanline, or a
  mid-frame VDP latch) — the same fix the "flashing text cut off" symptom needs.

---

## RESOLVED (2026-07-06) — Extended BASIC typed UPPERCASE where the real machine types lowercase

**Report (Joel, 2026-07-06).** "On the authentic TI ROM when I start Extended
BASIC, my typing appears in lowercase. On our ROMs, the typing appears in
uppercase." (The real TI-99/4A renders lowercase as **small capitals** — the
distinctive look Joel is describing.)

**Resolved.** Flipped the unshifted keytab in `crates/libre99-gpl/src/keymap.rs`
from uppercase to the authentic **lowercase**, rebuilt + recommitted
`console-grom.bin`, and added the regression gate
`rom_kscan.rs::our_keytab_types_lowercase_in_native_state_and_folds_in_state0`
(native mode now types `>61`; state 0 still folds to `>41`). Full workspace suite
green, 137-cart sweep clean, clippy clean. **Display side: not visually confirmed
headlessly** — Extended BASIC (and even authentic TI BASIC) doesn't reach its
READY prompt in the bare test harness, and the console never auto-loads lowercase
glyphs (its `>0016`/`>0018` loaders only cover `>20–5F`), so XB must self-provide
its small-caps; recommend eyeballing Extended BASIC in the GUI to confirm the
typed lowercase renders as small caps (if it shows blanks instead, the follow-up
is to ship the console's lowercase small-caps glyphs — the authentic GROM's
`>0874` block our rewrite zeros).

<details><summary>Root-cause analysis (kept for the record)</summary>

**Root cause — confirmed, and it is a one-table fidelity gap, NOT the big
"alpha-lock feature" the docs imply.** Our KSCAN case-fold machinery already
works; the shipped **keytab data is wrong**:
- Our GROM's *unshifted* keytab (`crates/libre99-gpl/src/keymap.rs`, `KEYS`) stores
  **uppercase** letters (`('A','A',…)`). The **authentic** unshifted keytab
  holds **lowercase** (`>1708` = `x w s …`; `RECON.md` §23,
  `KSCAN-SPEC.md` §5.5).
- The console-ROM KSCAN fold is state-gated and **already correct in our ROM**
  (proven today by `crates/libre99-gpl/tests/rom_kscan.rs`, 11/11 passing): given
  the authentic *lowercase* table it folds `a→A` in **state 0** (the menu /
  TI-BASIC state — `kscan_state0_folds_authentic_lowercase_table_to_uppercase`)
  and **keeps lowercase** in the **native mode 5** Extended BASIC uses, because
  our switchless 9901 alpha-lock line idles "not locked"
  (`kscan_native_state_reads_the_switch_and_keeps_lowercase`).
- So with our *uppercase* table, every mode returns uppercase: state 0 has
  nothing in `>61..>7A` to fold (stays uppercase — masks the bug for the menu),
  and native mode has no lowercase to keep → Extended BASIC shows uppercase.

**The fix (small, safe, and decoupled from ROADMAP §6).** Change the unshifted
column of `keymap.rs::KEYS` from uppercase to **lowercase** (`'a'..'z'`), leaving
the shifted column uppercase. Then, *unchanged and already-tested* KSCAN gives:
menu/TI-BASIC (state 0) → folds to uppercase (menu still works); Extended BASIC
(native) → keeps lowercase (the real behavior). This does **not** require the
ROADMAP §6 alpha-lock host-toggle — that feature only adds the ability to *lock*
to uppercase via Caps Lock; lowercase-by-default needs just the table.

**Scope / gates to update when applying:**
- `crates/libre99-gpl/src/keymap.rs` — flip the 26 unshifted letters to lowercase;
  update the `table_layout_and_values` unit test (`t[>172A]` becomes `b'a'`).
- Rebuild + re-commit the console GROM binary (`console-grom.bin`); the
  staleness gate `libre99-gpl/tests/committed_bin.rs` enforces this.
- Add a positive gate: with **our** (flipped) table, native mode yields `>61`.
- Verify no cartridge regresses (the 137-cart sweep; carts that scan in mode 0
  are unaffected — they still see folded uppercase).

**Display side (verify on apply).** Small-caps lowercase glyphs must exist in the
VDP pattern table for the lowercase codes. In TI BASIC (uppercase-only) this
never matters; in Extended BASIC the *cartridge* sets up its own screen — confirm
XB loads lowercase patterns (or provide them) so the kept lowercase codes render
as small caps rather than blanks once the keytab is flipped.

> **Note — related to, but distinct from, the Parsec issue.** This entry is
> keyboard *input* (the keytab produced uppercase codes); Parsec (above) turned
> out to be small-caps *display* — the `>004A` lower-case character-set loader
> our GROM stubbed. Two mechanisms, one family: the rewrite's lower-case
> support. With `>004A` implemented (2026-07-06), a cartridge that loads the
> console's small caps the documented way now gets them.

</details>
