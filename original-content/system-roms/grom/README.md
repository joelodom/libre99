# `grom/` — the rewritten console GROM

Original-content TI-99/4A console firmware, assembled by this repo's
[`libre99-gpl`](../../../crates/libre99-gpl) toolchain and executed by the genuine
console ROM's GPL interpreter. See the [project overview](../README.md) for the
doc map, [`../RECON.md`](../RECON.md) for the interface facts this implements,
and [`../DEBUGGING.md`](../DEBUGGING.md) before debugging anything here.

### Provenance / interface-data policy

All **code** and all **creative on-screen content** here are **original** — the
GPL is our own; TI's expressive content (the "TI" logo, the `© 1981` line, TI
BASIC, the title artwork, the sound tunes) is replaced with original work (the
Texas + 99 emblem, `© 2026 JOEL ODOM`, TI PYTHON, original beep lists). The
image reproduces **only the functional, uncopyrightable interface data** a
compatible OS must present at fixed addresses for interoperability — the 8×8 and
thin **character-set bitmaps** and the **keyboard/joystick decode tables** —
byte-identical, each gated by an identity test and enumerated (with its authentic
address and disposition) in [`SURFACE-MAP.md`](./SURFACE-MAP.md)'s
`DATA-MUST-MATCH` set. *(The project is licensed under the Modified MIT License
with Commons Clause — [`LICENSE.md`](../../../LICENSE.md) at the repo root; the
interface-data policy above stands. The original discussion is archived in
[`history/QUALITY-ASSESSMENT.md`](../history/QUALITY-ASSESSMENT.md) §9.)*

## Files

| File | What it is |
|---|---|
| `console.gpl` | The GPL source: GROM 0 header, the fixed entry `>0020` → title screen, the selection-list menu, and (GROM 1, `>2000`) the TI PYTHON REPL. **Its comment blocks document the scratchpad cell layouts the menu and REPL use — they are the authority for those cells.** |
| `console-grom.bin` | The built 24 KiB system-GROM image (a committed build artifact, like `titris.ctg`). |

Several data blocks are **not** in `console.gpl` — they are generated in Rust and
spliced in at build time by `libre99_gpl::system_grom::console_gpl_source()`:

- the **8×8 character set** ([`src/font.rs`](../../../crates/libre99-gpl/src/font.rs))
  as label `FONT` at GROM `>1000` — the genuine console character patterns
  (`>20–>5F`); a test gates the byte-for-byte match. It is **also** spliced at
  its authentic home `>04B4` (label `FONTA`, B1) so a cartridge that reads the
  font from the documented address gets the real bytes;
- the **thin ("small") character set** (also `src/font.rs`) in two forms: the
  8-row loader block `FONT2` at GROM `>4000` (in empty GROM 2 — **not** the
  `>1800` chip gap that doesn't exist on hardware, B4), which the char-set
  utility at slot `>0018` copies to the VDP; and the 7-row stored form at its
  authentic home `>06B4` (label `THINA`, B1). Note the subtlety: the set is
  **stored** 7 rows/glyph at `>06B4` (448 B) while `FONT2` carries the 8-row
  **expanded** form (512 B) — generators must emit the stored form via the
  7-row path (`emit_gpl_bytes_thin_stored`), never the expanded one;
- an **original menu-beep** sound list at GROM `>0484` (label `BEEP0484`, B1) —
  the address the authentic firmware keeps its beep at; ours is an original tune,
  not TI's bytes;
- the **Texas + 99 emblem** ([`src/logo.rs`](../../../crates/libre99-gpl/src/logo.rs))
  as label `LOGO` at GROM `>1600` (glyphs `>01–>09`) — the title screen's
  original-content stand-in for TI's "TI" logo, built from an original per-row
  Texas outline (see the module); and
- the **keyboard scan-code → ASCII table** + joystick tables
  ([`src/keymap.rs`](../../../crates/libre99-gpl/src/keymap.rs)) as label `KEYTAB`
  at GROM `>1700` (through `>17EF`), preceded by the joystick deflection table at
  `>16EA`. The console ROM's `KSCAN` reads these at fixed addresses to decode
  keypresses and joystick directions; without them every key returns `>00`
  (RECON §9); and
- the **Libre99 emulator-identification block** (`L99I`) at GROM `>5700` plus
  the baked version strings after it (`VERSTR`/`PYVERS`/`PYBANR` from `>5740`),
  spliced by `sysinfo_block()` — the data behind the **system information
  screen** (`(S)` on the selection menu; its GPL is the `SYSINF` section of
  `console.gpl` at GROM `>4800`). The baked strings carry the workspace's one
  `CARGO_PKG_VERSION` (emulator = this GROM = TI PYTHON — the REPL banner is
  spliced too, so it cannot drift). The block's *stamped* fields (emulator
  version/build/commit, host, mounted console-ROM identity) ship blank with a
  `>00` flag; `libre99-app` recognizes the magic and fills them in its in-memory
  copy at launch. Any other emulator leaves them blank and the screen renders
  those rows `UNKNOWN` — the intended degraded mode. Layout authority:
  `libre99_core::sysinfo` (one set of constants shared by the GROM build, the
  stamper, and the gates in `tests/sysinfo_screen.rs`). The rewritten console
  ROM carries a matching `L99R` + version marker at `>0BF0`.

Which authentic addresses carry interface data we reproduce byte-identically
(the `DATA-MUST-MATCH` set) is enumerated and test-gated in
[`SURFACE-MAP.md`](./SURFACE-MAP.md).

## Build & run

```sh
# Rebuild console-grom.bin from console.gpl + the font (run from the repo root):
cargo run -p libre99-gpl --bin libre99gpl -- console original-content/system-roms/grom/console-grom.bin

# Boot the rewrite in the desktop emulator. With no cartridge you land on the
# title; press a key for the menu, then 1 for TI PYTHON:
cargo run -p libre99-app -- --system-grom original-content/system-roms/grom/console-grom.bin --no-cartridge

# Or mount a cartridge and launch it from our menu:
cargo run -p libre99-app -- --system-grom original-content/system-roms/grom/console-grom.bin --cartridge cartridges/Parsec.ctg
```

Re-run the `console` command and re-commit `console-grom.bin` whenever
`console.gpl` or any spliced data block (font, thin font, logo, keymap, the
`>0484/>04B4/>06B4` data homes) changes (the "committed artifact" convention).
Tests and probes don't need the artifact — they assemble in-memory via
`build_console_grom()`.

## Testing — the two-tier gate

```sh
# Fast tier — run on every change (a few seconds):
cargo test -p libre99-core      # emulator core + the GROM coverage instrument
cargo test -p libre99-gpl       # assembler + all fast GROM gates

# Deep tier — run before shipping and whenever the scan/menu/service code or the
# spliced data changes (~1–3 min): the #[ignore]d cartridge sweeps.
cargo test -p libre99-gpl -- --ignored
```

The **fast tier** gates everything that has ever broken as focused per-feature
tests — `census` (surface-map completeness + font byte-identity + chip-gap zero),
`conformance` (state contract, incl. the F5 reset-drift guard), `interrupts`
(ISR/sound), `keyboard`, `char_set`, `device_io` (DSRLNK), `menu*`,
`service_stubs` (unimplemented-service CALL RTNs gracefully, never reboots),
`f5_reset`, and the per-class `sweep_*` samples. The **deep tier** is the whole
bundle: `sweep_all_cartridges` (all 137 list exactly) and `coverage_sweep` — a
**differential health panel** that launches every cart under both our GROM and the
authentic one and asserts ours is never *less alive* after launch (display on +
ISR ticking), not merely that it did not reboot. It regenerates
`COVERAGE-REPORT.md` (which console-GROM surface each cart exercises) and holds one
named waiver, **Video Vegas** (`LIMITATIONS.md` L8). Keep the fast tier green
pre-commit;
run the deep tier before a release or after touching shared boot/menu/service code.

## Address map (GROM)

Regenerated from the current source; the authentic-image classification behind
it is [`SURFACE-MAP.md`](./SURFACE-MAP.md).

| Range | Contents |
|---|---|
| `>0000–000F` | GROM 0 header (`>AA >02 …`) |
| `>0010–0037` | interconnect table — twenty `BR` stubs: `>0010`→DSRLNK, `>0016`→LDCSET, `>0018`→LDTSET, `>0020`→START, others→`ILRTN` (clean return) |
| `>0020` | fixed GPL entry (`BR START`, the ROM jumps here after reset) |
| `>0038–005F` | GPLLNK service grid — graceful `B SVCBAD` RTN stubs, except `>004A` = `LDLSET`, the lower-case character-set loader (2026-07-06) |
| `>0060…` | entry code: ISR arming, title paint, peripheral power-up scan, key-wait, the menu, dispatch (`SCANW`) |
| data | `VREGS`, `DISPON`, `COLORS`, `BARS`, `EMBL1/2/3`, `COPYR`, `TITLE1/2`, `PRESS`, `PRESSW`, `COPYT`, `SND`, `KBEEP` |
| `>0484–048B` | `BEEP0484` — an **original** menu-beep list at its authentic home (B1) |
| `>04B4–06B3` | `FONTA` — the 8×8 character set at its authentic home (byte-identical, B1) |
| `>06B4–0873` | `THINA` — the thin character set at its authentic home (7 rows/glyph, byte-identical, B1) |
| `>0874–094C` | `LOWERA` — the lower-case small-caps glyph set at its authentic home (31 glyphs, byte-identical to authentic, B1; loaded by `LDLSET`) |
| `>1000–11FF` | `FONT` — the authentic 8×8 character set (`>20–>5F`, 512 bytes) |
| `>1200…` | `DSRLNK` (device service link) + `ILRTN` + `LDCSET`/`LDTSET` (char-set loaders) |
| `>1600–1647` | `LOGO` — the Texas + 99 emblem (glyphs `>01–>09`, 72 bytes) |
| `>16EA–16FF` | joystick deflection table (11 `(Y,X)` pairs) |
| `>1700–17EF` | `KEYTAB` — the keyboard scan-code → ASCII blocks + `>17C8` joystick table |
| `>2000…` | GROM 1: the TI PYTHON program header + REPL |
| `>4000–41FF` | `FONT2` — the thin-set loader block (8 rows/glyph, expanded; relocated out of the `>1800` chip gap, B4) |

## GPL source syntax (what `console.gpl` is written in)

Our own assembler dialect (`crates/libre99-gpl/src/asm.rs`); there is no TI GPL
source anywhere in this project, so the syntax is ours by fiat:

- **Two-operand ops are `OP dst,src`** — `ST @>837A,>00` stores `>00` *into*
  `>837A`. `D` prefix = word form (`DST`, `DADD`, `DCEQ` …). An immediate vs.
  memory source is auto-selected from the operand's shape.
- **`MOVE count,src,dst`** — e.g. `MOVE >0200,G@FONT,V@>0900`. `#n` as a MOVE
  destination means VDP register *n* (`MOVE >0008,G@VREGS,#0`).
- **Operands:** `@>83xx` CPU address (far CPU addresses like `@>6000` work —
  the assembler applies the `+>8300` encoding bias); `V@>0380` VDP RAM;
  `G@LABEL` GROM (in MOVE); `*@cell` CPU byte-pointer indirect; `*V@cell` VDP
  word-pointer indirect; bare `>17` immediates.
- **Directives:** `GROM >addr` (absolute origin), `BYTE`, `DATA`, `TEXT`,
  `EQU`. `BYTE` can hand-lay an encoding the assembler has no mnemonic for
  (e.g. the `IO` in `START`) — only do that with semantics pinned by a probe.
- **Banned constructs** (assembler rejects; verified-failed on real hardware
  semantics — RECON §7): indexed GAS `(…)`, `MOVE`'s C=1 computed-GROM-source
  form, `FMT`.
- `BR`/`BS` are 13-bit **slot-absolute** (same 8 KiB slot only); `B`/`CALL`
  take 16-bit absolute GROM addresses (RECON R1).

## TI PYTHON v0 — the language

An interactive, immediate-mode **integer calculator with variables**, living in
GROM 1 where TI BASIC lived. Deliberately tiny; no stored programs, control
flow, functions, strings, or floats. Reached as `1 FOR TI PYTHON` from the
menu; QUIT (`FCTN`+`=`) returns to the title.

```
line       := assignment | expression
assignment := NAME '=' expression
expression := term   (('+' | '-') term)*
term       := factor (('*' | '/' | '%') factor)*
factor     := NUMBER | NAME | '(' expression ')' | ('+'|'-') factor
NAME       := a single uppercase letter A–Z   (16 variable slots)
NUMBER     := digit+                          (decimal)
```

Semantics (pinned by `crates/libre99-gpl/tests/ti_python.rs`):

- Values are **16-bit two's-complement**; wraparound is defined
  (`32767+1` → `-32768`; `-32768` has a special print path).
- `/` and `%` **truncate toward zero** (C semantics, *not* CPython floor):
  `10/3=3`, `-10/3=-3`, `-10%3=-1`. A deliberate v0 deviation from Python.
- Errors re-prompt without crashing: `SYNTAX ERROR`, `NAME ERROR`
  (unset variable read), `ZERO DIVISION ERROR`, and `TOO COMPLEX` (expression
  nested past the evaluator's operand/operator stacks — see `LIMITATIONS.md` L3).
- Implementation: an iterative **shunting-yard** evaluator with two scratchpad
  stacks (cell map in `console.gpl`'s REPL comment block); the screen row is
  the line buffer (input is tokenized by reading the row back from VDP RAM).

```
>>> 2 + 3 * 4
14
>>> X = 7
>>> X * (X - 1)
42
>>> Y
NAME ERROR
>>> 10 / 0
ZERO DIVISION ERROR
```

Known v0 gaps (paths forward in [`../LIMITATIONS.md`](../LIMITATIONS.md) L3/L4):
no backspace/line editing (needs the `FCTN` keymap block), single-letter
variable names.

## What works today

Everything in [`../STATUS.md`](../STATUS.md): title, menu (137/137 carts list
and launch — L2 resolved; the one post-launch exception is LIMITATIONS L8, Video
Vegas), TI PYTHON, and the ISR-driven
behaviours (sound, sprite motion, QUIT) since the boot arms the 9901 VDP
interrupt. The rewrite is the emulator's default boot since 2026-07-06; pass
`--system-grom roms/994AGROM.Bin` to boot the authentic GROM instead.
