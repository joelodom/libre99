# TI PYTHON — user's guide, language specification, and growth plan

**TI PYTHON** is the small interactive language the clean-room Libre99 console
firmware ships in the menu slot where TI BASIC lived (`1 FOR TI PYTHON` on the
selection screen). It is an original work — the name is a nod to the language
that inspired its flavor, not a port — and it is **a language very loosely
based on Python 3**: today a glorified calculator with variables, intended to
grow, deliberately and slowly, toward something more like real Python.

This document is four things at once, and is **the spec of record** for the
TI PYTHON track (it supersedes the language section of
[`original-content/system-roms/grom/README.md`](../original-content/system-roms/grom/README.md)
and the v0 sketch in the archived GROM-rewrite plan §9):

1. a **user's guide** (§1–§2) — written against **v1**, the language this spec
   defines;
2. the **v1 language specification** (§3) — what "complete" means for the
   ROADMAP's TI PYTHON row;
3. the **v0 bug ledger** (§4) and the **implementation plan** (§5) — a
   milestone-by-milestone path from the shipped v0 to v1, written so a working
   session (Opus) can execute it without re-deriving anything;
4. a **feasibility study** (§6) — how TI PYTHON's own growth can build, brick
   by brick, the console primitives that would eventually let the clean-room
   firmware run **TI Extended BASIC** (the `LIMITATIONS.md` L9 / milestone-M6
   gap).

**Status (2026-07-07, end of day): v1 IS IMPLEMENTED AND GATED.** The §5 plan
was executed the same day the spec landed (commit `cbbcdb2` = P1, `7c2cae9` =
P2–P6; twelve gates in `crates/libre99-gpl/tests/ti_python.rs`; deep-tier
cartridge sweep green). Every §4 bug is fixed. The §6 feasibility study's F0
census also ran — and its result beat the estimate: **Extended BASIC now runs
end-to-end on the clean-room firmware** via a five-helper ROM substrate
(~200 bytes at pinned authentic addresses), not the feared interpreter's worth
of services. See `original-content/system-roms/XB-CENSUS.md` and §6.7 below.
Implementation decisions that refined this spec are recorded in §5.9.

**Versioning.** "v0/v1" are *language levels* used by this document. The
version on the banner is the workspace's one `CARGO_PKG_VERSION` (spliced at
build time — emulator = GROM = TI PYTHON, one number, see
`libre99_gpl::system_grom::sysinfo_block`), so it advances with the project,
not with this spec.

---

## 1. What TI PYTHON is (and is not)

TI PYTHON is an **immediate-mode interpreter**: you type a line, it answers on
the next line. There are no line numbers, no stored programs, no `RUN`. Think
of it as a pleasant desk calculator that speaks a small subset of Python:

- **expressions** with real operator precedence and parentheses,
- **variables with full-size names** (`RADIUS = 30`),
- **`print(…)`** with string literals for labeling output,
- **Python's integer-division and modulo semantics**,
- honest, Python-flavored **error messages** that never crash the machine.

What it is **not** (yet): there are **no floats, no strings as values, no
control flow, no functions, no lists** — values are 16-bit signed integers,
period. The growth map (§3.8) sketches where it could go; the feasibility
study (§6) explains why growing it carefully is also the road toward running
TI Extended BASIC on this firmware.

Everything runs *on the emulated TI-99/4A itself*: the interpreter is GPL
bytecode in console GROM 1 (`original-content/system-roms/grom/console.gpl`,
the `PYENTRY` section), executed by the clean-room console ROM. That is the
point of it — it is real 1981-class firmware, written today, in the open.

## 2. User's guide (v1)

### 2.1 Starting and leaving

Boot the emulator with no cartridge (`cargo run -p libre99-app --
--no-cartridge`), press any key at the title, then **1** for TI PYTHON. You
get the banner and a prompt:

```
TI PYTHON 0.0.1
A SUPER SIMPLE PYTHON-LIKE
INTERPRETER FOR THE TI-99/4A
EXIT() QUITS. 16-BIT INTEGERS.

>>>
```

Three ways out:

| You type / press | What happens |
|---|---|
| `EXIT()` or `QUIT()` | back to the **selection menu** |
| `FCTN`+`=` (QUIT — host: **Left Alt**+`=`) | reboot to the **title screen** (console-wide, handled by the ISR) |
| `F5` (emulator reset) | cold reset (emulator-level) |

A fresh entry into TI PYTHON always starts a fresh session — variables do not
survive leaving and re-entering. They *do* survive an emulator save-state
(`F6`/`F8`), because the whole machine does.

### 2.2 Typing and editing

Type normally; the emulator translates your keyboard (see `docs/USER-GUIDE.md`
— **Left Alt is the TI's `FCTN`**). The console keyboard folds letters to
**uppercase**; TI PYTHON is an uppercase world (keywords are recognized in
either case, so `print` and `PRINT` are the same word).

| Action | TI keys | Host keys |
|---|---|---|
| Enter the line | `ENTER` | `Enter` |
| **Backspace** (delete char left of cursor) | `FCTN`+`S` | `Backspace` (or `Delete`) |
| **Erase the whole line** | `FCTN`+`3` | `Left Alt`+`3` |
| Quit to title | `FCTN`+`=` | `Left Alt`+`=` |

Input is one screen line: the prompt `>>> ` plus up to **28 characters**.
Keys beyond the limit are ignored. Unrecognized control keys are ignored too
(they never print junk). A **block cursor** shows where you are. There is no
key auto-repeat in v1 — press per character (§5 P7 lists repeat as optional
polish).

The screen behaves like a terminal: when output reaches the bottom row, the
screen **scrolls up** one line (the banner eventually scrolls off, like any
terminal history).

### 2.3 A session

```
>>> 2 + 3 * 4
14
>>> RADIUS = 30
>>> AREA = 3 * RADIUS * RADIUS
>>> AREA
2700
>>> PRINT("AREA =", AREA)
AREA = 2700
>>> 7 // 2
3
>>> -7 // 2          # floor division, like Python
-4
>>> -7 % 2
1
>>> 2 * -3
-6
>>> 32767 + 1        # 16-bit wrap, unlike Python
-32768
>>> BOGUS
NAME ERROR: BOGUS
>>> 10 / 0
ZERO DIVISION ERROR
>>> EXIT()
```

Assignments print nothing (as in Python). A bare expression prints its value.
A blank line just re-prompts.

### 2.4 Errors

Errors report and re-prompt; they never crash or corrupt the session.

| Message | Meaning (Python cousin) |
|---|---|
| `SYNTAX ERROR` | the line does not parse (`SyntaxError`) |
| `NAME ERROR: <NAME>` | reading a variable that was never assigned (`NameError`) |
| `ZERO DIVISION ERROR` | `/`, `//` or `%` by zero (`ZeroDivisionError`) |
| `TOO COMPLEX` | expression nesting overflowed the evaluator's stacks (`RecursionError`) |
| `MEMORY ERROR` | all 32 variable slots are in use (`MemoryError`) |

### 2.5 Limits (v1, by design)

| Limit | Value |
|---|---|
| Values | 16-bit signed integers, −32768…32767, arithmetic wraps mod 2¹⁶ |
| Input line | 28 characters (one screen row after the `>>> ` prompt) |
| Variable names | 1–10 characters; letters, digits, `_`; must not start with a digit |
| Variables | 32 simultaneously |
| Statements per line | one |

---

## 3. Language specification (v1)

This section is normative: v1 is done when the firmware conforms to it and the
gates in `crates/libre99-gpl/tests/ti_python.rs` pin it.

### 3.1 Environment

Standard 32×24 text screen. The **banner** is exactly four rows at the top of
a fresh session, then one blank row, then the first prompt:

```
row 0  TI PYTHON <version>              (spliced PYBANR — carries CARGO_PKG_VERSION)
row 1  A SUPER SIMPLE PYTHON-LIKE
row 2  INTERPRETER FOR THE TI-99/4A
row 3  EXIT() QUITS. 16-BIT INTEGERS.
row 4  (blank)
row 5  >>> _
```

(The three tagline rows are plain `TEXT` data in `console.gpl`; wording may be
tuned but each row must fit 32 columns and row 0 must keep the spliced
version.) The **prompt is `>>> `** (4 characters, column 0); input occupies
columns 4–31. Echoed input, results, and errors each occupy whole rows; when
the next row would pass the bottom of the screen, the display scrolls up one
row instead of clearing.

A **block cursor** marks the input position, blinking at roughly 0.5 s
(toggled off/on every 16 VDP-interrupt ticks via the ISR tick cell `>8379`).

### 3.2 Lexical structure

- **Whitespace** (spaces) may appear between any tokens and is insignificant.
- **`NAME`** := `(letter | '_') (letter | digit | '_')*`, 1–10 characters.
  An 11th name character is a `SYNTAX ERROR`. Names are compared exactly as
  typed (the stock console types uppercase; that makes case a non-issue in
  practice).
- **`NUMBER`** := `digit+`, decimal. Literals wrap mod 2¹⁶ like all
  arithmetic (`70000` reads as `4464`).
- **`STRING`** := `"…"` or `'…'` — any characters except the opening quote,
  closed **on the same line** (else `SYNTAX ERROR`). No escape sequences.
  Strings exist only as `print` items in v1; they are not values.
- **Comment**: `#` (outside a string) — the rest of the line is ignored.
- **Keywords** (case-insensitive, reserved — not usable as variable names):
  `print`, `exit`, `quit`.

### 3.3 Grammar

```
line       := [ statement ] [ comment ]
statement  := assignment | printcall | exitcall | expression
assignment := NAME '=' expression
printcall  := PRINT '(' [ item ( ',' item )* ] ')'
item       := expression | STRING
exitcall   := ( EXIT | QUIT ) '(' ')'
expression := term  ( ('+' | '-') term )*
term       := unary ( ('*' | '/' | '//' | '%') unary )*
unary      := ( '+' | '-' ) unary | atom
atom       := NUMBER | NAME | '(' expression ')'
```

An empty line (or all-spaces, or comment-only) re-prompts silently.

### 3.4 Semantics

- **Values** are 16-bit two's-complement integers. All arithmetic (including
  literals) wraps mod 2¹⁶: `32767 + 1` is `-32768`. (Documented deviation
  from Python's unbounded ints — this is a 16-bit machine and says so on the
  banner.)
- **Operators**, precedence high → low:
  1. unary `+` / `-` (right-associative; `--5` is `5`, `-5 % 3` is `(-5) % 3`)
  2. `*`, `/`, `//`, `%` (left-associative)
  3. binary `+`, `-` (left-associative)
- **Division and modulo follow Python 3**: `/` and `//` are both **floor
  division** (`7 // 2 = 3`, `-7 // 2 = -4`); `%` is Python's modulo — the
  result has the **sign of the divisor** (`-7 % 2 = 1`, `7 % -2 = -1`), and
  the identity `a == (a // b) * b + a % b` holds. Divisor zero →
  `ZERO DIVISION ERROR`. (That `/` yields an int, not a float, is the one
  deliberate divergence — there are no floats to yield. **This changes v0's
  pinned truncate-toward-zero semantics**; the v0 behavior was itself
  documented as a deviation to revisit.)
- **Assignment** `NAME = expression` evaluates the right side, then binds.
  Nothing prints. Failed evaluation binds nothing. Assigning to a keyword is
  a `SYNTAX ERROR`. There is no chained (`A = B = 1`) or augmented (`A += 1`)
  assignment in v1.
- **Bare expression**: evaluates and prints the value on the next row.
- **`print(item, …)`**: prints the items on one row, separated by single
  spaces (Python's default `sep`); `print()` prints a blank row. Output past
  column 31 is truncated. String items print their contents verbatim.
- **`exit()` / `quit()`**: leave TI PYTHON for the selection menu (the GPL is
  a `B MENU` — same mechanism the system-information screen already uses).
- **Errors** (§2.4) abort the line, print on the next row, and re-prompt. The
  `NAME ERROR: <NAME>` form names the offending variable.
- **Variables**: up to 32 live bindings, names per §3.2, re-assignment
  overwrites. The table is cleared on entry to TI PYTHON.

### 3.5 Input protocol (normative — this is where the v0 bugs die)

The read loop must use the console KSCAN **new-key protocol**, not raw
key-state polling: issue GPL `SCAN` and proceed **only when the GPL condition
bit reports a new key** (`>837C` bit `>20` — set iff this scan latched a key
different from the last, per
[`rom/KSCAN-SPEC.md`](../original-content/system-roms/rom/KSCAN-SPEC.md) §5.2),
then take the key from `>8375`. **No wait-for-release loop anywhere.** This
single change is what fixes dropped characters under fast (overlapped) typing
— a second key pressed before the first is released produces its own new-key
event.

Key handling, in order:

| Key | Action |
|---|---|
| `>0D` (ENTER) | terminate input, evaluate |
| `>08` (`FCTN`+`S` — host Backspace) | if any input: step cursor left, blank the cell |
| `>07` (`FCTN`+`3`, ERASE) | clear the input back to column 4 |
| `>20`–`>5F` (printable) | if under the 28-char cap: echo and advance; else ignore |
| anything else | ignore (never echoed) |

### 3.6 Deliberate divergences from Python 3 (documented, not bugs)

| Python 3 | TI PYTHON v1 | Why |
|---|---|---|
| unbounded ints | 16-bit wrap | native machine word; banner says so |
| `/` yields float | `/` = `//` (floor) | no floats in v1 |
| lowercase world | uppercase world | stock console keyboard folds to uppercase |
| strings are values | string literals only, in `print` | no string heap in v1 |
| many statements/features | §3.3 grammar only | v1 is a calculator, on purpose |

### 3.7 v0 → v1 delta (summary for the implementer)

New: banner (4 rows), `>>> ` prompt, scrolling, cursor, backspace/ERASE,
input cap, new-key input protocol, multi-char names (VRAM table),
`print`/strings/comments, `exit()`/`quit()`, floor `/` `%`, `//`, fixed unary
precedence, `MEMORY ERROR`, `NAME ERROR: <NAME>`.
Unchanged: 16-bit wrap, `TOO COMPLEX` guard behavior, `-32768` print path,
assignment-prints-nothing, error-then-re-prompt, `FCTN`+`=` QUIT.
Changed gates: division/modulo rows of the reference session, prompt/row
layout, banner text.

### 3.8 Growth map (post-v1, not commitments)

- **v1.x polish**: key auto-repeat (time via `>8379`), reject-key click
  (`KBEEP` pattern), `**` (right-assoc, above unary), hex literals (`0x1F`),
  lowercase mode (KSCAN translation state 2 + the `>004A` small-caps loader).
- **v2**: comparisons (`== != < <= > >=` → 0/1), `abs()`/`min()`/`max()`,
  augmented assignment, maybe single-line `if expr: statement`.
- **v3**: **floats** — radix-100 reals via the console ROM's already-shipped
  FP package (§6.3); strings as values (VRAM string space). These are the
  milestones that pay directly into the Extended BASIC road (§6).

---

## 4. v0 bug ledger (what's broken today, and why)

All in the `PYENTRY` section of
`original-content/system-roms/grom/console.gpl`. B1/B2 are the two reported
by the owner (2026-07-07: *"I can't backspace and the characters sometimes
miss if I type fast"*); B3–B6 were found by inspection while writing this
spec. `LIMITATIONS.md` L3 already tracks B2/B3 as deferred work; this spec
re-opens the track.

- **B1 — fast typing drops characters.** The read loop (`RDK`/`RDKR`) waits
  for a key, then spins until **no key at all** is held before accepting the
  next. Overlapped typing (press `B` before releasing `A`) never presents an
  all-keys-up instant *between* the two presses, so `B` is eaten whole. Fix:
  the §3.5 new-key protocol (KSCAN's own debounce, `>837C` bit `>20`), which
  is how the BASIC-era firmware reads typing. Bonus: it is immune to the
  "menu keypress still held at entry" stray-echo hazard, and it drops the
  release-spin entirely (less CPU per key).
- **B2 — backspace ignored (worse: echoed as junk).** The loop special-cases
  only ENTER; every other code — including `>08` (`FCTN`+`S`, which the
  desktop frontend already sends for host Backspace, `libre99-app/src/input.rs`)
  — is **echoed to the screen as a glyph**. Codes below `>20` render garbage
  patterns. Fix: the §3.5 dispatch (backspace, ERASE, printable filter).
- **B3 — no input length cap.** Echo advances the VDP cursor without bound; a
  long line scribbles VRAM past the row (`LIMITATIONS.md` L3, QUALITY §5
  item 3). Fix: the 28-char cap.
- **B4 — screen wrap is jarring.** At the bottom row the whole screen clears
  and the session restarts at the top (`PYNEXT`), losing visible history.
  Fix: terminal-style scroll (§3.1).
- **B5 — single-letter variables.** `A`–`Z`, 16 scratchpad slots — the spec
  says full-size names (§3.2). Fix: §5 P4 (VRAM symbol table).
- **B6 — unary minus mis-associates after a binary operator.** The evaluator
  implements unary `-x` as "push 0, then a **precedence-1 binary** minus", so
  a pending higher-or-equal-precedence operator reduces first: `2 * -3`
  evaluates as `(2*0) - 3 = -3` (Python: `-6`), `2 - -3` as `(2-0) - 3 = -1`
  (Python: `5`), `-5 % 3` as `0 - (5%3) = -2` (Python: `1`). Parenthesized
  forms (`2 * (-3)`) are unaffected, which is why the reference session never
  caught it. Fix: a dedicated one-operand negate operator at precedence 3
  (§5 P5).

Root-caused non-bug: the banner says only `TI PYTHON 0.0.1` — v1 replaces it
with the four-row banner (§3.1); the owner asked that it plainly say the
interpreter is a super-simple Python-like one.

---

## 5. Implementation plan (for the working session)

Six milestones, **P1–P6, in order**; each is a self-contained commit that
builds, passes the fast tier, and leaves the REPL strictly better. P7 is
optional polish, explicitly not v1-gating. Total new GPL is a very
comfortable fit for GROM 1 (`>2000–>37FF`, 6 KiB; v0 uses well under 1 KiB —
and the `census` chip-gap-zero gate fails the build if code ever spills past
`>37FF`, so the budget is tripwired).

### 5.0 Ground rules (read first)

- **Files**: the interpreter is `original-content/system-roms/grom/console.gpl`
  (`PYENTRY` → the data block before the `SYSINF` section — **touch nothing
  outside that span**; the menu, boot, DSRLNK, SYSINF and splice blocks are
  shared with everything else). Banner version splice:
  `crates/libre99-gpl/src/system_grom.rs::sysinfo_block` (`PYBANR`). Gates:
  `crates/libre99-gpl/tests/ti_python.rs`. Probe for by-hand exploration:
  `crates/libre99-gpl/examples/tipython_probe.rs`.
- **GPL dialect**: the syntax and the banned-construct list are in
  `grom/README.md` ("GPL source syntax"). Remember `BR`/`BS` are same-slot
  only; `B`/`CALL` are 16-bit. `OP dst,src` operand order. No indexed GAS, no
  `MOVE` C=1, no `FMT`.
- **Committed artifact**: after every milestone, rebuild and re-commit the
  GROM image — `cargo run -p libre99-gpl --bin libre99gpl -- console
  original-content/system-roms/grom/console-grom.bin` — the staleness
  conventions expect it.
- **Tests**: `cargo test -p libre99-gpl` (fast tier) every milestone;
  `cargo test -p libre99-core` when touching anything the coverage instrument
  sees; the deep tier (`cargo test -p libre99-gpl -- --ignored`) once at the
  end of the track (the image changed; the menu paths didn't, but the sweep
  is the insurance). Keep `cargo clippy --workspace` clean. All verification
  is headless `Machine`-level — never launch the GUI or inject host
  keystrokes.
- **Scratchpad no-touch zones**: `>8370–837F` (VDP top-of-mem + GPL
  data/sub-stack pointers + status), `>83C0+` (ISR cells), `>83E0+` (GPLWS).
  The existing `TOO COMPLEX` guards and the
  `deep_nesting_overflows_cleanly_and_the_repl_survives` gate are the
  tripwire — keep both.
- **Scratchpad remap (v1)**: the REPL owns `>8300–>836F`. Retire the v0
  variable table (`>8320–834F`) — v1 variables live in **VRAM** (below).
  Reuse `>8320–832F` for the new cells: `>8320` name length, `>8321–832A`
  name buffer (10 chars), `>832B` keyword id / print-item state, `>832C–832D`
  VRAM walk pointer, `>832E–832F` spare. Operand/operator stacks stay at
  `>8350`/`>8360` with their guards. **Update the cell-map comment block in
  `console.gpl` in the same commit** — that comment is the documented
  authority for these cells.
- **VRAM symbol table**: `>1000–>11FF`, 32 slots × 16 bytes
  `{len:1, name:10, pad:1, value:2, pad:2}`, cleared at `PYENTRY`. Free by
  construction at REPL time: the menu's scan window uses `>1000+` only during
  the menu, the disk DSR's reservation lives at the top of VRAM (`>8370`
  floor), and the screen/pattern tables sit below `>1000`. Save-states
  serialize VRAM, so variables survive suspend/resume for free.
- **Docs in the same commit** (repo rule): each milestone updates
  `grom/README.md`'s TI PYTHON section (point it at this spec; keep the
  address map current), and the final milestone updates `docs/USER-GUIDE.md`'s
  one-line description, `LIMITATIONS.md` L3 (close the shipped items with
  commit hashes), and the ROADMAP status table. Coordinate with parallel
  sessions before touching shared docs (`git status` first — USER-GUIDE.md
  in particular has been hot).
- **Clean-room**: everything here is original code; the only external facts
  needed are already pinned in-repo (`KSCAN-SPEC.md` §5.2 for the input
  protocol; `grom/SURFACE-MAP.md` for layout). No TI bytes, ever.

### P1 — the input engine (kills B1, B2, B3; the owner's two complaints)

Replace `RDK`/`RDKR` with the new-key loop and dispatch of §3.5:

```gpl
RDK     SCAN
        BR   RDK                    ; loop until KSCAN reports a NEW key
        ST   @>8314,@>8375          ; CHAR := the key
        ; dispatch: >0D done · >08 backspace · >07 erase-line ·
        ; >20..>5F echo if cursor < INSTART+28 · else ignore
```

Backspace: if the cursor is past `INSTART` (`>831A`), `DDEC` it and store
`>20` at the cell. ERASE: blank back to `INSTART`. The cap compares the
cursor against `INSTART`+`>001C`. Blank input (ENTER at `INSTART`, or only
spaces) skips evaluation and re-prompts (also part of B-fix hygiene).

**Gates (extend `ti_python.rs`):** (a) *the marquee regression* — an
overlapped-typing helper (`press A, press B, release A, release B`, 2–3
frames apart) types a whole expression with every adjacent pair overlapped
and the echoed row must be exact (red under v0); (b) backspace: type
`12`+BS+`3`, expect `13`; (c) ERASE then retype; (d) 40 keys in → echo stops
at 28 chars and the next VRAM row is untouched; (e) control keys (`FCTN`+`E`
arrow etc.) echo nothing; (f) blank line re-prompts (next prompt row, no
error). Keep every existing gate green (same prompt/layout until P2).

### P2 — banner + prompt

Four-row banner per §3.1: row 0 stays the spliced `PYBANR` (20 bytes — keep
`MOVE >0014,G@PYBANR,V@>0000`); rows 1–3 are new `TEXT` lines in
`console.gpl` (pad to taste; each ≤ 32 chars). Prompt becomes `>>> ` (4
bytes), input column 4, first prompt on row 5. **Gates:** banner rows
asserted verbatim; the reference session updated for the new geometry.

### P3 — terminal scroll + cursor (kills B4)

At the point v0 wrapped (`PYNEXT`), scroll instead: copy rows 1–23 up one row
and blank row 23. Our dialect accepts VDP as a `MOVE` source (`asm.rs`
"MOVE source must be G@addr, G*@cell, @cpu, or V@vdp"), and a V→V name-table
scroll `MOVE` is an authentically-used GPL idiom (cited in
`grom/SURFACE-MAP.md`), so the expected shape is
`MOVE >02E0,V@>0020,V@>0000` + blank the last row — **but first write a tiny
probe/unit gate proving our ROM's interpreter executes a V→V MOVE with the
ascending-overlap semantics the scroll needs** (if not, fall back to a 32-byte
scratchpad row bounce, 23 iterations). Cursor: pick char `>1E` (BASIC's
cursor code, a nice nod), `MOVE` an 8-byte solid-block pattern from GROM into
the pattern table slot at `PYENTRY`, draw it at the input cell, blink by
toggling block/space each time bit 4 of the ISR tick cell `>8379` flips —
i.e. every 16 ticks (the authentic firmware times exactly this way —
`CGT @>8379,…` idiom). Erase it before echo/ENTER handling, redraw after.
**Gates:** fill past the bottom → top row scrolled off, banner gone, prompt on
row 23, rows in order; cursor cell shows the block at the input position
(deterministic frame counts make blink assertable — sample at two instants 16
ticks apart).

### P4 — full-size names + the VRAM symbol table (kills B5)

Lexer: accumulate `NAME` into the `>8320` buffer (len + 10 chars; letters,
digits, `_`; 11th char → `SYNTAX ERROR`; leading digit can't happen — digits
lex as numbers). Replace `VLOAD`/`VSTORE` with VRAM-table walkers
(`>832C–832D` pointer, 16-byte stride, length+bytes compare; empty slot =
len 0). Full table on a new name → `MEMORY ERROR`. `NAME ERROR: <NAME>`
appends the name from the buffer. Keyword screen: after lexing a name, match
(case-insensitively) against `PRINT`/`EXIT`/`QUIT`; as an assignment target
or bare atom they are `SYNTAX ERROR` in P4 (P6 gives them their real
meanings), other keyword-shaped uses fall through as names. Assignment
parsing generalizes from "single letter then `=`" to "NAME then `=`".
**Gates:** `RADIUS = 30` / `3 * RADIUS * RADIUS` → `2700`; `_A1` works;
`ALONGNAME12` (11 chars) → `SYNTAX ERROR`; 33rd distinct name →
`MEMORY ERROR`; 32 names all retrievable; `TOTAL` vs `TOTAL2` distinct;
`NAME ERROR: BOGUS` exact text.

### P5 — Python arithmetic semantics (kills B6; adopts floor `/` `%`, adds `//`)

Three changes to the evaluator:

1. **Unary minus becomes a real operator**: on a sign in operand position,
   push a dedicated negate op (e.g. byte `>6E`) at precedence 3 instead of
   the 0-inject; it is right-associative (pop-while `prec(top) > 3`, i.e.
   never pops another unary), and `APPLY` for it pops **one** operand and
   negates. Unary `+` in operand position is skipped outright.
2. **Floor division**: after the existing truncating `SDIV`, if the remainder
   is nonzero and its sign differs from the divisor's, decrement the
   quotient. `//` lexes as one token (peek the second `/`) and is the same
   operation as `/`.
3. **Python modulo**: after `SMOD`, if nonzero remainder differs in sign from
   the divisor, add the divisor.

**Gates (the pinned session changes here, deliberately):** `-10 / 3` → `-4`,
`-10 % 3` → `2`, `10 // 3` → `3`, `-7 // 2` → `-4`, `7 % -2` → `-1`,
`2 * -3` → `-6`, `2 - -3` → `5`, `--5` → `5`, `-5 % 3` → `1`; identity spot
check `A == (A//B)*B + A%B` for a sign matrix; wrap rows (`32767 + 1` →
`-32768`) and both zero-divisor errors unchanged; `-32768` still prints.

### P6 — `print(…)`, strings, comments, `exit()`/`quit()`

Statement dispatch after lexing the first token: keyword `PRINT` → parse
`(` item-list `)` — each item an expression (evaluate, `PRNUM` into the
output row) or a string literal (copy verbatim from the input row); single
space between items; truncate at column 31; `print()` → blank row. Keywords
`EXIT`/`QUIT` → require `()` then `B MENU` (the `SYSINF` screen's exit is the
precedent — same target label, cross-chip `B` is fine). `#` outside quotes
ends the line (lexer-level; works on comment-only lines). Trailing garbage
after a complete statement (`print(1) x`) → `SYNTAX ERROR`.
**Gates:** `PRINT("AREA =", AREA)` → `AREA = 2700`; `print(1, 2+3, "OK")` →
`1 5 OK`; `print()` → blank row then correct next prompt row; unterminated
string → `SYNTAX ERROR`; `5 # NOTE` → `5`; comment-only line re-prompts;
`EXIT()` lands on the selection menu (assert a menu-screen row); keywords
case-insensitive; `PRINT = 5` → `SYNTAX ERROR`.

### P7 — optional polish (post-v1, do not block on these)

Auto-repeat (initial-delay + rate timed via `>8379`), reject-key `KBEEP`
click, `**`, hex literals, lowercase mode (scan state 2 + `>004A` loader),
`USE EXIT() TO LEAVE` hint on bare `EXIT`. Each is small and independent;
pick by taste after v1 ships.

### Acceptance (v1 done)

The §2.3 session transcript runs verbatim (allowing for scroll positions);
all gates above green; fast tier + deep tier green; clippy clean;
`console-grom.bin` recommitted; `grom/README.md`, `USER-GUIDE.md`,
`LIMITATIONS.md` L3 and the ROADMAP status table updated in the landing
commits; the F1 in-app help checked for stale TI PYTHON wording (ROADMAP row
3 owns the full revamp). **Met 2026-07-07** (commits `cbbcdb2`, `7c2cae9`,
plus the docs-sweep commit).

### 5.9 Implementation decisions (2026-07-07 — refinements this spec adopts)

The plan was executed as written, with these judgment calls, now normative:

- **Commit grouping**: P1 landed alone (the cherry-pickable bugfixes);
  P2–P6 landed as one commit — a banner promising `EXIT()` should not ship
  before `exit()` works.
- **Blank lines advance one row** (the input row), not two — assignments
  likewise; only lines with output (results, print, errors) take two rows.
- **Errors always take a fresh row of their own.** A mid-`print` error
  leaves its partial output on the row above (Python-style); nothing is
  appended to a partially-printed row.
- **Unterminated strings are refused before printing** — `print("OOPS`
  emits nothing, then `SYNTAX ERROR` (the string is validated to its closing
  quote before any character is echoed).
- **ERR clears at line start**, not only inside EVAL — a stale `MEMORY
  ERROR` shadowed every following line until this was caught by the
  33rd-name gate.
- **The assignment target parks in ANAME (`>8335-833F`)** across the RHS
  evaluation, whose own name reads refill NAMEBUF (`AREA = 3 * RADIUS *
  RADIUS` mis-bound RADIUS until the names gate caught it).
- **A keyword read inside an expression** (`2 + PRINT`) reports
  `NAME ERROR: PRINT`, not `SYNTAX ERROR` — keywords are only special in
  statement position; as an assignment target they are `SYNTAX ERROR`
  (per §3.4, unchanged).
- **The scroll is a single VDP→VDP `MOVE`** (`>02E0` bytes, rows 1–23 up
  one) — the assembler accepts the V-source form and the rewritten ROM's
  MOVE handler executes it; no per-row bounce was needed.
- The banner's third tagline reads `EXIT() QUITS. 16-BIT INTEGERS.` (32-col
  fit chose the wording).

---

## 6. Feasibility study — TI PYTHON's primitives as the road to TI Extended BASIC

**Question (owner, 2026-07-07):** can the primitives we build for TI PYTHON
be implemented so that they eventually let the clean-room firmware run **TI
Extended BASIC** — i.e. close the largest by-design gap in the rewrite
([`LIMITATIONS.md`](../original-content/system-roms/LIMITATIONS.md) L9:
authentic XB reaches `READY` under our firmware but executes nothing)?

**Short answer: yes, with eyes open.** The gap is well-bounded, roughly half
of the hard infrastructure already exists (most importantly the ROM's
floating-point package), and the project's proven differential method applies
directly. But the reusable half of the work is reusable **only if each
primitive lands at the authentic console addresses with the authentic
conventions** — TI PYTHON then becomes the interactive test harness that
exercises those primitives daily, not the thing XB literally calls. It is the
largest remaining track in the rewrite (bigger than the disk DSR), it is
**post-0.1.0 by construction**, and its first step is cheap and decisive: a
census, not code.

### 6.1 What Extended BASIC actually needs from the console

XB (a user-supplied cartridge; never redistributed by this project) brings
its own GROMs but is **not self-contained** — it hands each entered line to
the console's BASIC-era machinery (L9):

| Layer | What lives there | Status in our clean room |
|---|---|---|
| Console **GROM 1** (`>2000–37FF`) | the TI BASIC interpreter core (crunch/list/execute) the interconnect trampolines `>001A/>001C/>001E` vector into | **0 bytes** — TI PYTHON occupies this slot |
| Console **GROM 2** (`>4000–57FF`) | the shared BASIC-era GPL library, ~5.5 KiB (number/string services, the routines behind most of `>0010–005F`) | **0 bytes** (only our relocated `FONT2` + `SYSINF` live there); every entry is a graceful stub (`ILRTN`/`SVCBAD`) |
| Console **ROM** — FP | the radix-100 floating-point package `>0D3A–11A1` + `FLTAB` XML dispatch, FAC `>834A–8351` / ARG `>835C–8363` conventions | ✅ **shipped, bit-exact** (ROM rewrite M5; `rom/RECON.md` §9) |
| Console **ROM** — BASIC XMLs | `>15D6–18C7`: symbol-table ops, `PGMCH` program-text fetch, trampolines (`rom/SURFACE-MAP.md` §12) | ✗ — **ROM M6, deferred indefinitely by policy** (a tripwire test enforces the written-justification rule; re-opening it is a deliberate act) |
| Conventions | the scratchpad/VRAM contract BASIC-era code assumes (FAC/ARG, crunch buffer, value stack, string space, symbol/line tables in VRAM, `>8370` top-of-VRAM discipline) | partially pinned (`rom/RECON.md`); the XB-specific slice is **unpinned — a census deliverable (F0)** |

Also already in hand and load-bearing: the complete non-BASIC GPL
interpreter, KSCAN (all modes), DSRLNK + the disk DSR, the GROM
read-coverage instrument + differential coverage sweep
(`tests/coverage_sweep.rs` → `grom/COVERAGE-REPORT.md`), and local authentic
XB images under the git-ignored `third-party/` split for differential gates
(skip-green when absent — the established pattern).

### 6.2 The reuse principle (honest version)

The char-set loaders are the precedent: `LDCSET`/`LDTSET`/`LDLSET` are
original code **at the authentic entry points** (`>0016`/`>0018`/`>004A`)
honoring the authentic calling convention (`>834A` destination), which is why
foreign cartridges just work. A TI PYTHON-internal routine helps XB **only**
when built the same way. So the rule for every shared primitive:

> Implement it once, at its authentic console address with its authentic
> register/scratchpad/VRAM convention, differentially gated against the
> authentic firmware — then have TI PYTHON *call* it, rather than embedding a
> private twin.

Two corollaries. (1) TI PYTHON's private evaluator cells overlap FAC/ARG
today (operand stack `>8350–835F` vs FAC `>834A–8351`/ARG `>835C–8363`) —
harmless while self-contained, but the **floats milestone must remap the
REPL's stacks off the FP cells before its first `XML` into the FP package**
(the §5.0 remap already frees `>8320–834F`). (2) Primitives that can't land
at authentic addresses yet (because the surrounding library doesn't exist)
should still adopt the authentic *data formats* (radix-100 reals, BASIC
string descriptors) so relocation later is mechanical.

### 6.3 How TI PYTHON's growth maps onto the gap

| TI PYTHON milestone | Primitive it forces into existence | The XB consumer of the same primitive |
|---|---|---|
| v1 P4 — full-size names | symbol-table discipline (VRAM-resident name/value walkers) | the ROM `>15D6+` symbol XMLs + GROM library walkers |
| v1 P6 — `print` | number→text and screen/scroll services | GROM-2 display/format services |
| v3 — **floats** | calls into the shipped ROM FP package; radix-100 number↔string conversion routines in GROM 2 at their authentic entries | XB's every numeric statement (the FP package is already bit-exact; the *conversions* are the missing half) |
| v3 — strings as values | VRAM string space + garbage discipline | XB string handling |
| later — stored programs | tokenizer (crunch), program storage, `PGMCH`-style text fetch | XB's line entry/`RUN` pipeline |

Reading the table bottom-up is the effort estimate; reading it top-down is
the payoff schedule — every row improves TI PYTHON for its own sake even if
the XB road is never finished, and each landed row shrinks L9.

### 6.4 The address-space conflict (must be decided eventually, not now)

TI PYTHON squats at `>2000` — exactly where the BASIC interpreter core must
eventually live (both need a valid program header there; the menu finds
TI PYTHON through it, and XB-era code reaches GROM 1 through the interconnect
trampolines). Options when F4 (below) actually needs the space:

1. **Relocate TI PYTHON into GROM 2's free tail** above the library, if it
   fits (it is small; the library is ~5.5 KiB of the 6 KiB chip — tight).
2. **Rebuild TI PYTHON as a repo cartridge** (like Titris/Sokoban: source +
   built `.ctg`, mounted by hand — consistent with the "nothing bundled"
   decision), freeing GROM 1 entirely.
3. A build-time **firmware variant** (calculator GROM vs BASIC-compat GROM) —
   workable but two images to test; least preferred.

Recommendation: decide at F4 kickoff; until then TI PYTHON stays where it is
(F0–F3 don't touch `>2000`), and option 2 is the default assumption — TI
PYTHON's REPL loses nothing by living in a cartridge, and the console menu
slot then advertises the real thing (BASIC) once it exists.

### 6.5 Phased path (each phase pays for itself)

| Phase | Work | Size | Gate / deliverable |
|---|---|---|---|
| **F0 — XB console-call census** | Run authentic XB under the **authentic** GROM in the existing harness with scripted sessions (`PRINT`, variables, strings, `FOR`, `GOSUB`, `DIM`, disk I/O); record every console-GROM fetch (`>2000–57FF`), `>0010–005F` entry, ROM XML dispatch, and scratchpad/VRAM cell the hand-off touches — the GROM read-coverage instrument already does the hard part. Cross-check with the L8 static `06 00 XX` call-scan (screen, not proof). | **S** (days) | `XB-CENSUS.md` (RECON-style): the exact surface, GROM 1 vs GROM 2 vs ROM split — turns ~11.5 KiB of unknown into an enumerated contract |
| **F1 — first GROM-2 brick** | The Video Vegas routine (L8: slots `>002C`/`>0032`), exactly as LIMITATIONS scopes it. | **S** | Video Vegas launches; the L8 waiver is deleted; proves the brick pattern |
| **F2 — numeric services** | Radix-100 number↔string conversions + FP-adjacent GROM services at authentic entries, driven by F0's list; remap TI PYTHON's evaluator off FAC/ARG; TI PYTHON v3-floats rides on the result. | **M** (weeks) | differential microtests vs authentic per routine; TI PYTHON floats as the living demo |
| **F3 — string + symbol services** | VRAM string space/GC + symbol services; the ROM `>15D6–18C7` XMLs — **requires deliberately re-opening ROM M6** (write the justification the tripwire demands). | **L** | differential gates; TI PYTHON strings-as-values |
| **F4 — crunch/list/execute core** | The GROM-1 interpreter core per the census (XB's own GROMs supply the XB extensions). Resolve §6.4 first. | **XL** — the largest single piece of the whole rewrite | staged: tokenize-only → `PRINT`-only → statement classes |
| **F5 — the XB differential harness** | Scripted XB sessions under authentic-vs-ours must converge (the coverage-sweep/DSR model, public/local split, skip-green without images). Grows with F2–F4, not after them. | **M**, amortized | the L9 closure gate |

### 6.6 Risks, and the verdict

- **Sparse documentation of the internal contract** — mitigated by F0 (trace,
  don't guess), Nouspikel/Classic99 *consult-never-copy*, and the fact that
  TI BASIC's user-visible semantics (the behavior oracle for F4) are
  exhaustively documented in public manuals.
- **Scope creep** — mitigated by the census: implement only what XB is
  *observed* to reach, graceful stubs for the rest (the L6 discipline).
- **Policy** — ROM M6 is deferred-indefinitely *by policy*; F3 must clear
  that gate consciously, with the written justification the tripwire test
  demands. This study is the raw material for it.
- **IP** — unchanged posture: original code from traced interfaces;
  authentic XB stays a user-supplied third-party image, used locally for
  differential gates only.

**Verdict:** feasible and well-shaped; F0 should precede any commitment
(days, and it converts the remaining unknowns into a checklist); the interim
answer for users stays the authentic-ROM boot (`--system-rom`/`--system-grom`,
`KNOWN-ISSUES.md`); and TI PYTHON is the right vehicle — every primitive gets
a user the day it lands, which is exactly how the rest of this firmware got
good.

### 6.7 Outcome (2026-07-07 — F0 ran, and the substrate landed the same day)

The census (`crates/libre99-gpl/examples/xb_census.rs`, built on a new CPU
PC-coverage instrument) **overturned this study's own cost model, in the good
direction**: for the XB in the local media set (`xb25.ctg`), the entire
console gap was **five ROM helpers, ~200 bytes, called directly by address**
from the cartridge ROM — no console-GROM BASIC library is touched at all
(§6.1's GROM 1/2 rows were the L9 theory, not the measured reality), and the
one stubbed interconnect slot XB calls (`>0032`) is tolerated. F1's Video
Vegas routine therefore remains open but is **no longer on XB's critical
path**. The helpers are implemented at their pinned authentic homes (the *XB
substrate*, `original-content/system-roms/rom/console.asm`; census, interface
dossier and the M6-policy justification:
`original-content/system-roms/XB-CENSUS.md`), and **Extended BASIC runs
end-to-end on the default clean-room boot** — gated by
`libre99-asm/tests/xb_substrate.rs` and `libre99-gpl/tests/xb_smoke.rs`.
§6.4's address-space conflict never materialized for XB (TI PYTHON keeps
GROM 1); it returns only if TI BASIC proper (F4) is ever pursued. The F2/F3
rows (floats, strings — the TI PYTHON growth path) stand as written.

---

## 7. Document status & cross-references

- **This is the TI PYTHON spec of record** (v1 + the road beyond). The
  ROADMAP's TI PYTHON row cites it; `grom/README.md`'s language section
  defers to it as of the first v1 landing.
- Bug tracking: §4 here; `LIMITATIONS.md` L3 items close against §5
  milestones (update L3 with commit hashes as they land).
- The Extended BASIC gap: `LIMITATIONS.md` L9 (and L8's single-routine
  instance); `docs/KNOWN-ISSUES.md` for the user-facing note; §6 here for the
  path.
- Interface authorities used: `rom/KSCAN-SPEC.md` §5.2 (input protocol),
  `grom/SURFACE-MAP.md` + `rom/SURFACE-MAP.md` (layout/classification),
  `rom/RECON.md` §9 (FP package, FAC/ARG), `grom/README.md` (GPL dialect,
  build/test).
- History: the v0 design is §9 of the archived
  `original-content/system-roms/history/GROM-REWRITE-PLAN.md`; the v0
  deferral decision is `LIMITATIONS.md` L3 (2026-07-02); the decision to
  grow TI PYTHON per this spec is the owner's, 2026-07-07 (ROADMAP row 2).
