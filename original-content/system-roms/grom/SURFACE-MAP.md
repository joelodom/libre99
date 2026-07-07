# GROM-0 Surface Map — the classified compatibility surface (Phase A1)

*Written 2026-07-02 for `QUALITY-ASSESSMENT.md` Phase A1 (chunk 1). Read
[`../history/QUALITY-ASSESSMENT.md`](../history/QUALITY-ASSESSMENT.md) (archived) §3/§5/§6 and
[`../RECON.md`](../RECON.md) first.*

## What this file is

The five post-milestone field bugs (TI-Invaders blank text, dead in-game
keyboard, dead joystick, silent console, reset artifacts) were **one defect
class**: a fixed-address behaviour or data table the authentic console GROM
provides but our rewrite shipped as zeros, which a cartridge (or the console
ROM's own code) reached directly. That class is **finite and mechanically
enumerable** — it is exactly the set of *authentic-only runs*: contiguous
stretches where the authentic image `roms/994AGROM.Bin` is non-zero and our
`grom/console-grom.bin` is zero.

This document **classifies every such run of ≥ 8 bytes in GROM 0**
(`>0000–17FF`, the console monitor GROM). The full documented surface is **98**
runs, totalling **3 229 bytes** (the census baseline this map classifies). Each
row below is one run; each is labelled with one of four dispositions and an
evidence-based identification. This bounds the remaining compatibility risk to
"a run we labelled `DEAD`/`CODE-REPLACED` was actually read by something" —
which the A2 read-coverage tripwire converts into a five-minute diagnosis.

> **Note on the live count (B1 interaction).** B1 (chunk 1) ships the two
> character sets byte-identical at their authentic homes (`>04B4`, `>06B4`) and
> an original menu-beep list at `>0484`. Doing so makes our image non-zero
> across those 13 runs, so they drop out of *authentic-only* and the **live
> census reports 85 runs / 2 522 bytes** once B1 has landed (the 707-byte
> difference is exactly the 697 font bytes + the 10-byte beep). **All 98 stay
> documented here on purpose:** the 12 `DATA-MUST-MATCH` font rows are the
> cart-facing contract and anchor the census gate's byte-identity assertion
> (which now holds *because* B1 shipped them), and `>0484` stays `CODE-REPLACED`
> (our original beep now lives at that address). The census gate is green either
> way — it requires every *live* run to be covered by a range here (85 ⊆ 98) and
> the fonts to be byte-identical.

## The census method (how the run list is generated)

The run list is **not** hand-maintained. It is emitted by the committed census
tool, which is the single source of truth:

```
cargo run -p libre99-gpl --example grom_census          # per-region stats + the GROM-0 runs
cargo run -p libre99-gpl --example grom_census -- all    # also GROM 1/2
```

The tool loads the authentic image and `system_grom::build_console_grom()`,
then walks `>0000–17FF` reporting every maximal run of `auth != 0 && ours == 0`
of length ≥ 8 (`crate::census::authentic_only_runs`). The census gate
[`crates/libre99-gpl/tests/census.rs`](../../../crates/libre99-gpl/tests/census.rs)
enforces this map: it asserts (a) every `DATA-MUST-MATCH` range here is
byte-identical between the two images, and (b) **no GROM-0 authentic-only run
≥ 8 bytes is absent from this file** — so a new run (e.g. after a splice
change) fails the build until it is classified here. The map cannot rot.

## How each run was classified (evidence, not assumption)

Every run was identified from primary evidence on the authentic image:

- **Our own disassembler** — `cargo run -p libre99-gpl --bin libre99gpl -- dis
  roms/994AGROM.Bin >XXXX` — run at each run start and at instruction-aligned
  entry points nearby. A run is **CODE** when a clean instruction-aligned flow
  through it tiles as coherent GPL (branches, `ST`/`MOVE`/`CALL`/`SCAN`/`XML`,
  scratchpad `>83xx` operands) with **zero (or a lone inline-data) unknown
  opcode**. Several anchors tile hundreds of bytes with **0 unknowns**
  (`>0D59`: 416 B, `>09B8`: 311 B, `>0EF9`: 129 B, `>0C82`: 128 B) — decisive.
- **Raw hex** (`od`) — to read data tables and embedded ASCII (fonts, the
  colour/register tables, the sound lists, glyph bitmaps, cassette prompt
  strings, the DSR list).
- **[`../RECON.md`](../RECON.md)** — the verified firmware dossier (the
  interconnect table decode, the scratchpad map, the DSR-list, the char-set
  loaders, the sound-list format, the VDP-register table at `>0451`).
- **Nouspikel / Classic99 (consulted, never copied)** — for the general shape
  of the TI monitor (KSCAN table location, the cassette DSR living in the
  console GROM, GPLLNK service conventions). No TI or Classic99 bytes or source
  are reproduced here; descriptions only.

## The four dispositions

| Label | Meaning | Action |
|---|---|---|
| **DATA-MUST-MATCH** | Interface data a cartridge may address directly (fonts; fixed-address tables). Must be byte-identical for interoperability. | Byte-identity gate + (usually) a B1 fix that ships it at its authentic home. |
| **CODE-REPLACED** | Authentic GPL *code*, or authentic *creative content* (title text/glyphs, sound lists, config tables), whose function our rewrite provides elsewhere (boot, title, menu, list-walk, dispatch, number-format, BASIC/cassette linkage). Carts do not jump into the middle of TI's monitor code. | None. |
| **SERVICE-ENTRY** | A fixed entry point / table on the console's interconnect / GPLLNK / DSR service surface (`>0010–005F` slot targets, the DSR list). | Cross-referenced to the B2 loud-stub grid and L6 closure (chunk 5). |
| **DEAD** | Nothing known addresses it; disassembles as neither recognizable code nor a known table. | None; revisit only on A2 evidence. |

## Summary

| Disposition | Runs | Bytes |
|---|---:|---:|
| DATA-MUST-MATCH | 12 | 697 |
| CODE-REPLACED | 82 | 2 489 |
| SERVICE-ENTRY | 4 | 43 |
| DEAD | 0 | 0 |
| **Total** | **98** | **3 229** |

**No run classified `DEAD`.** Every authentic-only run in GROM 0 was positively
identified — as the two character sets, the boot/title/menu creative content,
the master monitor code (list-walk, number-format, title/menu), the cassette
DSR, or the console service surface. This is the expected outcome:
QUALITY-ASSESSMENT §3 predicted GROM 0 is "a mix of authentic title/menu/BASIC-
entry *code* we replaced by design, and *data tables / library routines at
documented addresses*." The one genuinely readable-table hazard class
(DATA-MUST-MATCH) is confined to the two fonts, both already reproduced
byte-identically.

### SERVICE-ENTRY closure — chunk 5 / L6 (2026-07-04)

All **4 SERVICE-ENTRY runs (43 bytes)** are dispositioned; none needed a build-out
for the bundle at the time (QUALITY-ASSESSMENT §7.8 amendment 3). The coverage
sweep ([`COVERAGE-REPORT.md`](./COVERAGE-REPORT.md)) showed 137/137 run with zero
reboots, and ~16 carts CALL a stubbed service and carry on via the graceful `RTN`.

> ⚠ **Correction (2026-07-04, ship review).** The stronger *differential* health
> panel then found that "no bundled cart calls anything still stubbed *and needs
> it*" is false by one cart: **Video Vegas** CALLs stubbed slots `>002C`/`>0032`
> and *depends on their side effect*, so it launches to a dead console under ours
> (`../LIMITATIONS.md` **L8**, open). The graceful-stub disposition of the
> `>0010-005F` surface is still correct — it just is not sufficient for every
> bundled cart.

| Run | Slot / target | Disposition |
|---|---|---|
| `>004A..>0057` | GPLLNK grid tail | **graceful stub, except `>004A` = implemented** — `>0038-005F` is `B SVCBAD` (breadcrumb + `RTN`, GROM `>1201`) for every entry but `>004A`, the **lower-case character-set loader** (`LDLSET`, data at the `>0874` home; 26 carts CALL it — Parsec's in-game small-caps text). The remaining stubs: ~16 carts CALL an unimplemented service and rely on the no-op |
| `>043B..>0444` | slot `>0012` (GPL sub-stack unwind helper) | **out of contract** — internal monitor helper (cassette-only); our `>0012` → `ILRTN` |
| `>0446..>0450` | slots `>001A/>001C/>001E` (GROM-1 BASIC trampolines) | **out of contract** — we ship TI PYTHON in GROM 1; our slots → `ILRTN` |
| `>1310..>1317` | cassette DSR-list CS1 entry | **unreached / deferred** — header `>08 = >0000`; cassette has no emulator hardware (ROADMAP §6); `DSRLNK("CS1")` errors gracefully |

The in-contract services live where our image is non-zero (so they are not
authentic-only runs): `>0010` DSRLNK (`>1200`), `>0016`/`>0018` LDCSET/LDTSET,
`>004A` LDLSET (the lower-case set), `>0020` START, and the boot peripheral
power-up scan.

---

## Region narrative (the evidence, walked by address)

**`>0038–005F` — GPLLNK service tail.** Our image ships `BR` stubs at
`>0010–0049`; the authentic service grid continues to `>005F`. The one
authentic-only run here, `>004A..>0057`, is the tail past our stubs
(QUALITY-ASSESSMENT §5 item 7) → **SERVICE-ENTRY**; chunk 2's loud-stub grid
covers the full `>0038–005F`, and `>004A` has since been **implemented** — it
is the lower-case (small capitals) character-set loader (`B LDLSET`), the
service Parsec and 25 other carts CALL to stage the `>60..>7E` glyphs (data at
the `>0874` home below).

**`>043B–0450` — interconnect slot targets (BASIC/GPL service trampolines).**
RECON's decoded interconnect table gives `>0012 → >043C`, `>001A → >0446`,
`>001C → >0449`, `>001E → >044C`. Disassembly confirms: `>043C` is a **GPL
sub-stack unwind helper** (`DECT @>8373; ST @>83FA,*@>8373; DECT @>8373; RTN`),
called internally (the cassette DSR does `CALL >0012` at `>136E`); `>0446/49/4C`
are three `B` **trampolines into GROM-1 BASIC** (`B >284C/>284E/>2010`). These
are the console's own service entries, exactly parallel to the `>0396/>039E`
char-set loaders the plan cites as SERVICE-ENTRY examples → both runs
**SERVICE-ENTRY** (out of contract: we ship TI PYTHON in GROM 1; our slots
`>0012/>001A/>001C/>001E` stub to `ILRTN`). *(These deviate from the initial
"expect CODE-REPLACED" note — the disassembly shows them to be interconnect
slot targets, so the evidence-based label is SERVICE-ENTRY. Cross-ref chunk 2/5.)*

**`>0451–0483` — boot configuration + power-on beep (data).** The run
`>0452..>0482` is `MOVE`d/read by the console boot, not by carts. It is the
authentic **VDP register init table** (RECON R2), the **colour table** (twelve
black-on-cyan groups + the sixteen colour-bar values), and the **power-on beep
sound list**. Our boot ships original equivalents (`VREGS`, `COLORS`, `BARS`,
`SND`) at our own labels → **CODE-REPLACED**. *(Descriptions only — the
authentic byte values are not reproduced here; consult the image via the census
tool if you need them.)*

**`>0484..>048D` — the menu-beep (key-click) sound list.** The list `>83CC`
points at on a keypress (RECON menu §). Creative sound content; our original
beep list is shipped at `>0484` by B1 → **CODE-REPLACED**.

**`>048F..>04B3` — title banner text (data).** Hex is the ASCII string
`©-glyph "1981  TEXAS INSTRUMENTS" "HOME COMPUTER"` (37 bytes, ending exactly at
the font). Read by the console's own title code; our title ships its own
strings → **CODE-REPLACED** (authentic creative/copyright content, replaced).

**`>04B4–06B3` — the standard 8×8 character set (512 B).** DATA a cartridge may
`MOVE` from the documented address `>04B4` (the TI-Invaders class). Fragmented
by blank glyph rows (zeros) into six runs → all **DATA-MUST-MATCH**. B1 ships it
byte-identical at its authentic home `>04B4` (our `FONT` also lives at `>1000`).

**`>06B4–0873` — the thin/"small" 7-row character set (448 B).** Same story,
loaded by slot `>0018 → >039E`. Six runs → all **DATA-MUST-MATCH**; B1 ships it
byte-identical at `>06B4`.

**`>0874–094C` — the lower-case (small capitals) 7-row character set (217 B,
31 glyphs `>60..>7E`).** Immediately after the thin set; loaded by the fixed
service entry `>004A → >03C2`, which parameterizes the same 7-row engine
(`>03A7`) with source `>0874` and count `>1F`. Earlier revisions of this map
misread the tail of this block (`>092C..>0945`) as "title/menu glyph bitmaps" —
it is the `z { | } ~` glyph rows. Shipped byte-identical at `>0874` →
**DATA-MUST-MATCH** (2026-07-06, the Parsec small-caps-text fix); our menu-data
block moved from `>0880` to `>0950` to clear the home.

**`>094D–09B7` — title/menu glyph bitmaps + the "FOR" word (data).** Hex shows
symmetric glyph bitmaps (e.g. the authentic **© glyph** at `>0998`, TI's — our
`COPYR` is an original ringed-C, deliberately different) and the menu word
**"FOR"** at `>094D` (the console `MOVE`s "FOR" for `n FOR name`; our `SCANW`
writes it inline). Fragmented like a glyph table. Creative content our rewrite
replaces (emblem/logo/©/menu text) → **CODE-REPLACED**. *(Not cart-addressable;
the emblem is TI's copyrighted logo we deliberately do not reproduce.)*

**`>09B8–1007` — the master monitor: list-walk / number-format / title-menu
code.** This is the bulk. Instruction-aligned flows tile it with **0 unknown
opcodes** over hundreds of bytes (`>09B8` 311 B, `>09D8` 279 B, `>0B42` 170 B,
`>0C82` 128 B, `>0D59` 416 B, `>0E5B` 158 B, `>0EF9` 129 B, `>0F31` 73 B,
`>0F80` 78 B) — coherent GPL with `CALL`/`BR`/`SCAN`/`XML`/`MOVE` and `>83xx`
scratchpad operands (cartridge-header parsing, digit-to-ASCII number
formatting, screen painting). Our rewrite provides title/menu/list-walk/dispatch
(`START`, `MENU`, `SCANW`) → every run here **CODE-REPLACED**. One routine of
note: `>0C82` is the authentic target of interconnect slot `>0024` (a monitor
utility no bundled cart uses; our slot stubs to `ILRTN`) — recorded in its row
for the chunk-5 enumeration but classified CODE-REPLACED as internal code.

**`>1282..>12BE` — menu/module code + "REVIEW MODULE LIBRARY" text.** Beyond
where our `DSRLNK` fills `>1200+`, the authentic image has monitor code carrying
the ASCII string "REVIEW MODULE LIBRARY". Our menu ships its own text/layout →
**CODE-REPLACED**.

**`>1300–16DB` — the cassette (CS1/CS2) DSR.** RECON: "the cassette DSR is GPL
at `>1300–16FF`." Evidence: `>1310` is the **DSR-list CS1 entry**
(`next >1318, DSR >1326, "CS1"`; `>1318` declares `"CS2" → >132C`) — reached via
GROM header offset `>08`, which our header ships as `>0000`, so it is unreached.
The body disassembles as cassette code: a `CASE @>834A` op-dispatch jump table at
`>1347`, CRU I/O (`IO` opcodes = cassette motor/data), frame-timed delays
(`CGT @>8379,>1E`/`>3C`), a name-table scroll `MOVE >02C0,V@>0000,V@>0040`, key
input (`E`/`C`/`R`/`ENTER`), and prompt strings (`MOVE`-immediate fragments spell
"PRESS…ENTER", "…FOUND", "CHECK", "RECORD"). Cassette is **out of contract /
deferred** (no cassette hardware; QUALITY-ASSESSMENT §7.4, RECON) —
`DSRLNK("CS1")` fails gracefully per chunk 3. The DSR-list run `>1310..>1317` →
**SERVICE-ENTRY** (device-service declaration); all other cassette runs →
**CODE-REPLACED** (out-of-contract monitor code + creative prompt content; not
cart-addressable). *(Occasional lone unknown bytes in these flows are inline
data — prompt lengths, sound bytes — between instructions, normal for real GPL.)*

Above `>16DB` the authentic-only runs stop, because `>16EA–16FF` (joystick
deflection table) and `>1700–17EF` (KSCAN key tables) are already shipped
byte-identical in our image (gated by `tests/keyboard.rs`).

---

## The full run table (all 98, in census / address order)

Legend: **D** = DATA-MUST-MATCH, **C** = CODE-REPLACED, **S** = SERVICE-ENTRY,
**X** = DEAD. Each row's `>XXXX..>YYYY` range is the census run verbatim.

| Range | Bytes | Class | Identification / evidence |
|---|---:|:---:|---|
| `>004A..>0057` | 14 | S | GPLLNK service grid tail past our `>0038–0049` stubs (QA §5 item 7); chunk 2 grid covers `>0038–005F`. |
| `>043B..>0444` | 10 | S | Interconnect slot `>0012` target: GPL sub-stack unwind helper (`DECT @>8373; ST @>83FA,*@>8373; …`); called internally (cassette `CALL >0012`). Our slot → `ILRTN`. |
| `>0446..>0450` | 11 | S | Interconnect slots `>001A/>001C/>001E` targets: `B` trampolines into GROM-1 BASIC (`>284C/>284E/>2010`). Out of contract; our slots → `ILRTN`. |
| `>0452..>0482` | 49 | C | Boot data: VDP register table (`>0451`, RECON R2) + colour table + power-on beep list. Our boot ships `VREGS`/`COLORS`/`BARS`/`SND`. |
| `>0484..>048D` | 10 | C | Menu-beep/key-click sound list at `>0484`. Creative sound content; B1 ships an original beep list at `>0484`. |
| `>048F..>04B3` | 37 | C | Title banner text data: ©-glyph + `"1981  TEXAS INSTRUMENTS"` + `"HOME COMPUTER"`. Our title ships its own strings. |
| `>04DC..>04EE` | 19 | D | Standard 8×8 char set (`>04B4–06B3`), glyph rows. B1 ships it byte-identical at `>04B4` (also at `>1000`). |
| `>04F4..>0503` | 16 | D | Standard char set, glyph rows. Byte-identical at `>04B4`. |
| `>0534..>0583` | 80 | D | Standard char set, glyph rows. Byte-identical at `>04B4`. |
| `>05A5..>05B1` | 13 | D | Standard char set, glyph rows. Byte-identical at `>04B4`. |
| `>05B5..>0693` | 223 | D | Standard char set, glyph rows. Byte-identical at `>04B4`. |
| `>069C..>06A7` | 12 | D | Standard char set, glyph rows. Byte-identical at `>04B4`. |
| `>06C9..>06E7` | 31 | D | Thin/"small" char set (`>06B4–0873`, 7 rows/glyph). B1 ships it byte-identical at `>06B4`. |
| `>06EC..>06F9` | 14 | D | Thin char set, glyph rows. Byte-identical at `>06B4`. |
| `>0724..>0769` | 70 | D | Thin char set, glyph rows. Byte-identical at `>06B4`. |
| `>0775..>077E` | 10 | D | Thin char set, glyph rows. Byte-identical at `>06B4`. |
| `>0786..>0791` | 12 | D | Thin char set, glyph rows. Byte-identical at `>06B4`. |
| `>0793..>0857` | 197 | D | Thin char set, glyph rows. Byte-identical at `>06B4`. |
| `>092C..>093A` | 15 | D | Lower-case char set (`>0874–094C`, 7 rows/glyph), `y z { \|` rows — previously misread as title bitmaps. Byte-identical at `>0874` (2026-07-06). |
| `>093C..>0945` | 10 | D | Lower-case char set, `} ~` rows — previously misread as title bitmaps. Byte-identical at `>0874`. |
| `>094D..>095F` | 19 | C | Menu word `"FOR"` + glyph data. Our `SCANW` writes "FOR" inline. |
| `>0966..>0981` | 28 | C | Title/menu glyph bitmaps (creative). Replaced by ours. |
| `>0998..>09A0` | 9 | C | Authentic © glyph bitmap (TI's) + code; our `COPYR` is an original ringed-C, deliberately different. Replaced. |
| `>09AE..>09B6` | 9 | C | Title/menu glyph/data (creative), abutting the list-walk code at `>09B8`. |
| `>09B8..>09C9` | 18 | C | Master monitor code (list-walk); clean GPL flow, 0 unknowns. Ours: `SCANW`. |
| `>09D8..>0ABF` | 232 | C | Master monitor code (list-walk / entry render); 279-byte 0-unknown flow. |
| `>0AC1..>0AE6` | 38 | C | Monitor code (name copy / device-name scan); clean flow. |
| `>0AE8..>0B2D` | 70 | C | Monitor code; body of the list-walk block. |
| `>0B2F..>0B37` | 9 | C | Monitor code (`BR/ST/CGE/ADD`); clean flow. |
| `>0B39..>0B40` | 8 | C | Monitor code; continuation. |
| `>0B42..>0B5C` | 27 | C | Monitor code; 170-byte 0-unknown flow. |
| `>0B5E..>0BDD` | 128 | C | Monitor code (name/geometry math); 142-byte 0-unknown flow. |
| `>0BDF..>0BEA` | 12 | C | Monitor code; continuation. |
| `>0BEC..>0BF6` | 11 | C | Monitor number-format code; clean flow. |
| `>0BF8..>0C44` | 77 | C | Number-to-ASCII formatting (`DIV @>8310,>0A; ADD …,>3030`); 78-byte flow. |
| `>0C46..>0C4F` | 10 | C | Number-format helper (`.`/`0` fill); clean flow. |
| `>0C51..>0C6D` | 29 | C | Number-format helper; 38-byte 0-unknown flow. |
| `>0C79..>0C80` | 8 | C | Monitor helper (`>0C77` routine body); clean flow. |
| `>0C82..>0C8E` | 13 | C | Monitor utility; authentic target of interconnect slot `>0024` (unused by bundled carts; our slot → `ILRTN`). 128-byte 0-unknown flow. |
| `>0C90..>0CCA` | 59 | C | Monitor code; body continuing the `>0C82` routine. |
| `>0CD1..>0CEB` | 27 | C | Monitor code; clean flow. |
| `>0CF5..>0D00` | 12 | C | Monitor code; continuation. |
| `>0D06..>0D32` | 45 | C | Monitor code (`XML`-linked); 51-byte flow. |
| `>0D4F..>0D57` | 9 | C | Monitor code; clean flow. |
| `>0D59..>0D6D` | 21 | C | Master monitor code; 416-byte 0-unknown flow anchor. |
| `>0D73..>0D90` | 30 | C | Monitor code; body of the `>0D59` block. |
| `>0D92..>0D9D` | 12 | C | Monitor code (mid-`MOVE` start); within the `>0D59` flow. |
| `>0D9F..>0DB7` | 25 | C | Monitor code; within the `>0D59` flow. |
| `>0DB9..>0DC5` | 13 | C | Monitor code; within the `>0D59` flow. |
| `>0DC7..>0DF3` | 45 | C | Monitor code; within the `>0D59` flow. |
| `>0DFA..>0E21` | 40 | C | Monitor code; within the `>0D59` flow. |
| `>0E2B..>0E36` | 12 | C | Monitor code; within the `>0D59` flow. |
| `>0E38..>0E42` | 11 | C | Monitor code; within the `>0D59` flow. |
| `>0E4C..>0E59` | 14 | C | Monitor code; within the `>0D59` flow. |
| `>0E5B..>0E75` | 27 | C | Master monitor code; 158-byte 0-unknown flow anchor. |
| `>0E77..>0E8E` | 24 | C | Monitor code; within the `>0E5B` flow. |
| `>0E90..>0EB6` | 39 | C | Monitor code; within the `>0E5B` flow. |
| `>0EB8..>0EC3` | 12 | C | Monitor code; within the `>0E5B` flow. |
| `>0EC5..>0ED2` | 14 | C | Monitor code; within the `>0E5B` flow. |
| `>0EDC..>0EE8` | 13 | C | Monitor code; within the `>0E5B` flow. |
| `>0EEA..>0EF7` | 14 | C | Monitor code; end of the `>0E5B` flow. |
| `>0EFB..>0F04` | 10 | C | Master monitor code; 129-byte 0-unknown flow anchor (`>0EF9`). |
| `>0F06..>0F2F` | 42 | C | Monitor code; within the `>0EF9` flow. |
| `>0F31..>0F44` | 20 | C | Monitor code; 73-byte 0-unknown flow anchor. |
| `>0F46..>0F78` | 51 | C | Monitor code; within the `>0F31` flow. |
| `>0F80..>0F8B` | 12 | C | Monitor code; 78-byte 0-unknown flow anchor. |
| `>0F8D..>0F98` | 12 | C | Monitor code; within the `>0F80` flow. |
| `>0F9A..>0FA3` | 10 | C | Monitor code; within the `>0F80` flow. |
| `>0FA5..>0FC8` | 36 | C | Monitor code; within the `>0F80` flow. |
| `>0FD5..>0FDD` | 9 | C | Monitor code (`BS/NEG/BR`); clean flow. |
| `>0FEB..>1007` | 29 | C | Monitor code; spills into our blank space-glyph at `>1000–1007`. |
| `>1282..>128B` | 10 | C | Menu/module code (beyond our `DSRLNK`); clean flow region. |
| `>128D..>12BE` | 50 | C | Menu/module code + ASCII `"REVIEW MODULE LIBRARY"`. Our menu ships its own text. |
| `>1310..>1317` | 8 | S | Cassette DSR list, CS1 entry (`next >1318, DSR >1326, "CS1"`). Reached via header `>08`; our header ships `>08=>0000` (unreached). Cassette deferred (QA §7.4). |
| `>132F..>133C` | 14 | C | Cassette DSR code (out of contract/deferred). |
| `>133E..>136E` | 49 | C | Cassette DSR: op-dispatch `CASE @>834A` jump table + handlers; `CALL >0012`. |
| `>1370..>13FC` | 141 | C | Cassette DSR code (`CZ @>83CE`/beep-wait, CRU I/O). |
| `>13FE..>14AC` | 175 | C | Cassette DSR I/O (CRU bit r/w, name-table scroll `MOVE >02C0`). |
| `>14B5..>14CC` | 24 | C | Cassette DSR code; within the display/scroll block. |
| `>14CE..>14D5` | 8 | C | Cassette DSR code; continuation. |
| `>14D7..>14F6` | 32 | C | Cassette DSR code; continuation. |
| `>14F8..>1513` | 28 | C | Cassette DSR code; continuation. |
| `>1528..>1547` | 32 | C | Cassette DSR key input (`E`/`C`/`R`/`ENTER` prompts). |
| `>1549..>155C` | 20 | C | Cassette DSR motor/CRU + frame-timed delay (`CGT @>8379,>1E`). |
| `>155E..>1571` | 20 | C | Cassette DSR timing delays (`CGT @>8379,>3C`, repeat). |
| `>158D..>159D` | 17 | C | Cassette DSR code (buffer walk); clean flow. |
| `>159F..>15C0` | 34 | C | Cassette DSR code. |
| `>15C2..>15C9` | 8 | C | Cassette DSR branch/jump-table entries. |
| `>15CB..>15F6` | 44 | C | Cassette DSR jump table + prompt text. |
| `>15F8..>1601` | 10 | C | Cassette DSR branch/jump-table entries (our LOGO home >1600 may open with a blank glyph row). |
| `>1640..>164D` | 14 | C | Cassette DSR jump table + prompt text. |
| `>164F..>1663` | 21 | C | Cassette DSR prompt strings (57-byte flow; ASCII fragments). |
| `>1665..>1686` | 34 | C | Cassette DSR prompt text (`"PRESS…ENTER"` fragments). |
| `>1688..>1697` | 16 | C | Cassette DSR prompt text / branches. |
| `>1699..>16A0` | 8 | C | Cassette DSR prompt text / branches. |
| `>16A2..>16B7` | 22 | C | Cassette DSR prompt text (`"…FOUND"` fragments). |
| `>16B9..>16C9` | 17 | C | Cassette DSR prompt text (`"CHECK"` fragments). |
| `>16CB..>16DB` | 17 | C | Cassette DSR prompt text (`"RECORD"` fragments). |

---

## How to maintain this map

1. **Regenerate the run list** whenever `console.gpl`, the spliced data
   (`font.rs`, `keymap.rs`, `logo.rs`), or `system_grom.rs` changes:
   `cargo run -p libre99-gpl --example grom_census`. The 98 rows above are the
   full documented surface (3 229-byte pre-B1 baseline); after B1 the live
   census reports 85 runs / 2 522 bytes (see the B1 note near the top). Newly
   *filled* runs stay documented (as `DATA-MUST-MATCH`/`CODE-REPLACED`); newly
   *appearing* runs must be added.
2. **The census gate keeps it honest.** `cargo test -p libre99-gpl --test census`
   fails if any GROM-0 authentic-only run ≥ 8 bytes is missing from this file,
   or if any `DATA-MUST-MATCH` range stops being byte-identical. A splice that
   *fills* a run (e.g. B1 shipping the font at `>04B4`) removes it from the
   census; a splice that *shrinks* our coverage adds a new run and fails the
   gate until it is classified here.
3. **To add a row**: paste the census range verbatim (format `>XXXX..>YYYY`,
   four uppercase hex digits, two dots — a test parses this), give its byte
   length, disassemble the authentic bytes with `libre99gpl dis` from an
   instruction-aligned entry, and assign a disposition with the evidence.
4. **`DATA-MUST-MATCH` is the only load-bearing disposition** for the census
   identity gate — reserve it for data a cartridge can address at a fixed GROM
   address (today: only the two fonts). Do **not** invent DATA-MUST-MATCH
   tables; prefer `DEAD` (revisit on A2) when no concrete reader can be named.
5. **`SERVICE-ENTRY` rows feed chunk 2 (B2 loud-stub grid) and chunk 5 (L6
   closure)** — keep the authentic target + behaviour note current there.

## Addendum — runs exposed by our own layout, not authentic omissions

The 98-run table above is the authentic surface as it stood at the pre-B1
baseline. A few runs below are **not** authentic tables/routines we forgot to
ship — they are stretches of the authentic low-GROM boot/menu **code** that our
image happens to leave zero because *our* menu code and data are laid out
differently (shorter code here, data blocks relocated for headroom). The census
gate still flags them (authentic-non-zero, ours-zero), so they are classified
here to keep it honest; the disposition is `CODE-REPLACED` because the authentic
content is monitor boot/menu code whose function our rewrite provides in
`START`/`MENU`/`SCANW` (carts never jump into the middle of it).

| Range | Bytes | Disp | Why it appears |
|-------|------:|:----:|----------------|
| `>0406..>0438` | 51 | C | Authentic boot/menu code in the low-GROM code region. Exposed when the menu **data block** (`VREGS`…`KBEEP`) was relocated to the free gap above the thin font (`>0880`), so the menu code at `>0060` keeps room to grow without the `>0484` beep / `>04B4` font splices overwriting it (L2 / `SFAR` far-list window — see `console.gpl`). Padding between our code and the relocated data; authentic content is CODE-REPLACED. |
