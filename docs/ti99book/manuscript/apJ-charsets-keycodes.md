# Appendix J — Character Sets and Key Codes

<!-- Appendices · target ≈4 pp · companion to Ch. 13, 21 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The character-set structure (J.1) — code ranges, GROM home addresses, pattern-table load addresses, the 8-byte glyph format — is tier-1 for the project's machine: read from crates/libre99-gpl/src/font.rs (the >20-5F standard set at GROM >04B4, the thin set at >06B4, the lower-case small-caps >60-7E at >0874, each gated byte-for-byte against the console GROM by the crate's tests). The key-code tables (J.2-J.4) are tier-1 from crates/libre99-gpl/src/keymap.rs (the four ASCII blocks at GROM >1705/>1735/>1765/>1795, the KEYS quads, the FCTN edit/arrow codes, the CTRL = >80+n rule, the state-0 uppercase fold, key-units 0/1/2, the joystick deflection table >16EA), execution-pinned against the authentic GROM in the crate's examples/. The 8x8 electrical key matrix is owned by Appendix G (from keyboard.rs) and cross-referenced, not duplicated. Individual glyph bitmaps are clean-room reproductions in the project's own licensed source (font.rs); a representative glyph is shown to teach the format, not the whole set. -->

Two lookup tables the console keeps in GROM turn the hardware into something you
can type on and print with: a **character-set** table (the 8×8 pixel pattern for
each code) and a **key-code** table (the byte each key produces). This appendix
catalogs both. The teaching — loading a font into the VDP pattern table, scanning
the keyboard — is Chapter 13 (`textlib`) and Chapter 21 (`inplib`); the electrical
8×8 key **matrix** (which switch is wired where) is Appendix G, and this appendix
picks up where that leaves off: what byte a scanned key *decodes to*.

## J.1 The character sets

The console GROM carries three contiguous character sets, each a run of 8×8
glyphs. Tier-1 (`font.rs`, gated byte-for-byte against the console GROM):

| Set | Codes | Glyphs | GROM home | Storage |
|---|---|---|---|---|
| **Standard** | `>20`–`>5F` | 64 | `>04B4` | 8 rows each |
| **Thin** ("small") | `>20`–`>5F` | 64 | `>06B4` | 7 rows (top row blank) |
| **Lower-case** (small caps) | `>60`–`>7E` | 31 | `>0874` | 7 rows (top row blank) |

The standard set is what boots into the pattern table; it is **identity-mapped**,
so a character's code equals its ASCII value (`>41` is `A`). Codes below `>20` and
above the set's range render **blank**. The lower-case set is TI's small-capitals
`a`–`z` plus `` ` { | } ~ ``; it is not loaded at boot — a cartridge that wants
lowercase calls the console's loader (Appendix K, GPL interconnect slot `>004A`),
which is exactly why an unprepared cartridge shows garbled leftover patterns where
lowercase should be.

**The glyph format.** Each glyph is **8 bytes, one row top-to-bottom, MSB the
leftmost pixel.** So `A` (`>41`) is `>78 >84 >84 >84 >FC >84 >84 >84`:

```text
>78   . # # # # . . .
>84   # . . . . # . .
>84   # . . . . # . .
>84   # . . . . # . .
>FC   # # # # # # . .
>84   # . . . . # . .
>84   # . . . . # . .
>84   # . . . . # . .
```

The thin and lower-case sets store only **seven** rows (the top row is always
blank); the console's loader writes a blank row 0 and then the seven stored rows,
so a plain 8-byte-per-glyph `MOVE` of an expanded block reproduces the loader's
output. To place the standard set, `MOVE` the 512-byte block into the pattern
table at `pattern_base + >20 × 8` (Ch. 13). The complete bit patterns for all
three sets live in the project's `font.rs` (and assemble into the clean-room GROM);
this card teaches the format rather than reprinting all 159 glyphs.

## J.2 KSCAN, the mode cell, and the four code blocks

The GPL `SCAN` opcode (Appendix B) reads the keyboard into a code. Which decode it
performs is chosen by the **key-unit** in the mode cell **`>8374`** (tier-1,
`keymap.rs`):

| `>8374` | Key-unit | Scan |
|---|---|---|
| `>00` | **0** — full keyboard | the 48-key ASCII decode (J.3) |
| `>01` | **1** — left split / joystick 1 | the joystick / split-keyboard decode (J.4) |
| `>02` | **2** — right split / joystick 2 | the joystick / split-keyboard decode (J.4) |

In key-unit 0, `SCAN` walks the 8×8 matrix (Appendix G) to a scan code and indexes
one of **four 43-entry blocks** by the held modifier, each at a fixed GROM address:

| Modifier | GROM block |
|---|---|
| none (unshifted) | `>1705` |
| SHIFT | `>1735` |
| FCTN | `>1765` |
| CTRL | `>1795` |

The scanned code lands in **`KEY` (`>8375`)**; the console then normalizes it per
the translation state (J.3). Omit these tables from a GROM rewrite and every
keypress decodes to `>00` — the tables are a functional part of the machine, not
decoration.

## J.3 The key codes

Each key produces four possible bytes — unshifted, shifted, FCTN, CTRL. The full
43-key table is in `keymap.rs`; the reference values worth carrying are the
**special keys**, the **FCTN edit/arrow codes**, and the two **rules**.

**Special and representative keys** (tier-1):

| Key | Unshift | Shift | FCTN | CTRL |
|---|---|---|---|---|
| Enter | `>0D` | `>0D` | `>0D` | `>0D` |
| Space | `>20` | `>20` | `>20` | `>20` |
| `A` | `a` | `A` | `\|` `>7C` | `>81` |
| `1` | `1` | `!` | `>03` | `>B1` |
| `=` | `=` | `+` | `>05` (QUIT) | `>9D` |
| `,` | `,` | `<` | `>B8` | `>80` |

**The FCTN edit and arrow codes** — the canonical reference (tier-1, `keymap.rs`):

| Function | Key | Code | | Function | Key | Code |
|---|---|---|---|---|---|---|
| AID | FCTN+7 | `>01` | | ERASE | FCTN+3 | `>07` |
| CLEAR | FCTN+4 | `>02` | | **LEFT** | FCTN+S | `>08` |
| DEL | FCTN+1 | `>03` | | **RIGHT** | FCTN+D | `>09` |
| INS | FCTN+2 | `>04` | | **DOWN** | FCTN+X | `>0A` |
| QUIT | FCTN+= | `>05` | | **UP** | FCTN+E | `>0B` |
| REDO | FCTN+8 | `>06` | | PROC'D | FCTN+6 | `>0C` |
| BEGIN | FCTN+5 | `>0E` | | BACK | FCTN+9 | `>0F` |

**The two rules:**

- **CTRL + letter = `>80 + n`** (A = `>81`, B = `>82`, … Z = `>9A`) — the
  high-bit control codes.
- **The unshifted letters are lowercase (`a`–`z`)**, matching the authentic GROM.
  The console's KSCAN then folds by **translation state**: state 0 (the menu /
  TI-BASIC "99/4" screen) folds `a`–`z` **to uppercase** unconditionally, so the
  menu selects on `S`; the native state (mode 5, which Extended BASIC uses)
  **keeps lowercase**. This is why the menu is uppercase but Extended BASIC shows
  small caps — the same table, two fold states.

## J.4 Joystick and split-keyboard (key-units 1–2)

In key-units 1 and 2, `SCAN` does **not** use the ASCII blocks. It reads the
joystick column (Appendix G: columns 6/7, fire on row 0, directions on rows 1–4)
and turns the direction into signed X/Y **deflections** through a table at GROM
`>16EA`, storing them in **`JOYY` (`>8376`)** and **`JOYX` (`>8377`)**. The
deflections are the three values **`+4` (`>04`)**, **`0` (`>00`)**, and **`−4`
(`>FC`)** per axis (tier-1, `keymap.rs::JOY_DEFLECT`):

```text
        JOYX (>8377)   JOYY (>8376)
left      >FC (-4)        0
right     >04 (+4)        0
up         0            >04 (+4)
down       0            >FC (-4)
(centered) 0              0
```

A **split-keyboard** decode (the two halves of the keyboard used as two joysticks
for keyboard play) runs through a separate table at GROM `>17C8`, returning a
direction/fire code in `KEY` (`>8375`). The two paths are independent: omit
`>16EA` and the physical joystick reads zero deflection while keyboard play still
works; omit `>17C8` and in-game keyboard movement dies while the menu still scans.
Chapter 21 decodes all of this into `inplib`'s clean direction masks and measures
it on the bench (`press J1U`).

*See also:* Chapter 13 (`textlib` — loading a character set into the pattern
table), Chapter 21 (`inplib` — KSCAN, key-units, joystick decode), Appendix B
(the GPL `SCAN` opcode), Appendix C (the scratchpad cells `>8374`/`>8375`/`>8376`/
`>8377` KSCAN writes), Appendix D (the pattern table a font is loaded into),
Appendix G (the electrical 8×8 key/joystick matrix), Appendix K (the console's
character-set loader services).
