> **ARCHIVED (2026-07-02).** This plan was executed to completion — M0–M6 all
> shipped. It is kept for provenance; do not work from it. Live docs:
> [`../README.md`](../README.md) (overview), [`../STATUS.md`](../STATUS.md)
> (what's built), [`../RECON.md`](../RECON.md) (interface facts). See
> [`README.md`](./README.md) in this folder for known stale statements.

# GROM Rewrite — exhaustive plan

A from-scratch, original-content reimplementation of the TI-99/4A **console GROM**
(`994AGROM.Bin`), assembled by a new GPL toolchain we build in this repo, running
on the genuine console ROM's GPL interpreter inside our own emulator.

This document is the engineering plan: the contract we must honor, the toolchain
we must build first, the reconnaissance to recover the real boot/menu flow, the
milestones and their validation gates, and the TI PYTHON v0 language. It is
written to the standard of `docs/PLAN.md` — references are cited as
`path:line` so an implementer can jump straight to the evidence.

---

## 0. Scope & goals

**In scope (this plan): rewrite the console GROM.**

1. Boot the console on an **original** GROM image — no TI copyrighted bytes in
   GROM — keeping the genuine console ROM (`994aROM.Bin`).
2. Draw an **original title screen** (`JOEL ODOM ROM REWRITE` /
   `(WWW.JOELODOM.COM)`), replacing TI's copyright and logo.
3. Reproduce the **master selection list**: scan headers, list programs, read the
   keyboard, dispatch — so **all bundled cartridges still list and launch**.
4. Replace TI BASIC with **TI PYTHON v0**, a tiny interactive integer calculator.
5. Build the **GPL toolchain** required for all of the above (Milestone 0).

**Out of scope (later phases):**

- Rewriting the console ROM / the GPL interpreter itself (**Phase 2** of the
  broader system-ROM project; tracked in the parent `README.md`).
- A full BASIC/Python with stored programs, control flow, functions, strings,
  floating point. TI PYTHON v0 is intentionally minimal; see §9.
- Cassette (CS1/CS2) and speech — stubbed or documented as gaps (§8, M5).

**Definition of done:** `cargo run -p libre99-app -- --system-grom
original-content/system-roms/grom/console-grom.bin` boots to our title screen;
pressing a key reaches a selection list; mounting any bundled cartridge adds it to
the list and launches it; selecting TI PYTHON gives a working REPL; and the
regression suite (boot, menu, cartridge-launch, REPL) is green.

---

## 1. Background: the GROM / GPL / console-OS split

The facts below are established from our core, from Classic99 (checked out on
both workstations — `C:\ClaudeShared\classic99` on the PC and
`/Users/Shared/classic99` on the Mac; consult, never copy), and from the real
ROM/GROM bytes. They are the foundation the whole plan rests on.

- **The GROM is GPL bytecode, not machine code.** The CPU runs the console ROM
  (`994aROM.Bin`); that ROM contains the **GPL interpreter**, which reads GPL
  bytes out of GROM through the `>9800` ports and executes them. Our emulator runs
  the real firmware and does **not** reimplement GPL
  (`crates/libre99-core/src/lib.rs:6-11`, `docs/ARCHITECTURE.md:19-24`,
  `docs/PLAN.md:4-6`). Classic99's only from-scratch GPL interpreter,
  `addons/gpl.cpp`, is disabled (`#if 0`, line 6) — it too runs the real ROM.
  **Consequence: a GROM rewrite must emit valid GPL.**

- **GROM hardware model** (`crates/libre99-core/src/grom.rs`): a flat 64 KiB space
  (eight 8 KiB slots), a 16-bit auto-incrementing address counter, a one-byte
  prefetch buffer, and a high/low address-byte flip-flop. Auto-increment **wraps
  inside the current 8 KiB slot** (`grom.rs:42-48,116-121`). The console GROM is
  loaded 1:1 at GROM `>0000` (GROMs 0/1/2 fill `>0000–5FFF`) by
  `Tms9900Bus::new` → `grom.load(0x0000, console_grom)` (`machine.rs:88-89`).

- **Ports** (decoded in `machine.rs:163-171,224-232`): read `>9800` = data, read
  `>9802` = address (post-increment counter high byte; **destructive**; resets the
  byte flip-flop — this reset was the historic boot bug, `docs/STATUS.md:136-157`);
  write `>9C00` = data, write `>9C02` = address.

- **Reset** (`crates/libre99-core/src/cpu.rs:149-155`): `WP=>83E0`, `PC=>0024` (the
  first bytes of `994aROM.Bin`). The ROM self-tests, sets up the 9901 + VDP and
  scratchpad at `>8300`, then enters the GPL interpreter.

- **The title screen is launched at a *fixed* GROM entry, not via the header.**
  The real GROM 0 header declares **no** powerup routine and **no** program (see
  §2). Yet the title screen appears — so the ROM kernel **hardcodes** the GPL
  entry address it jumps to after init. Recovering that exact address is
  reconnaissance task R1 (§6). Everything reachable from it — VDP setup, title
  draw, the menu scan/dispatch loop — is GPL code in GROM 0 that we must
  reimplement.

- **The menu / selection list is GPL in GROM 0.** Scanning each GROM/cartridge
  header for `>AA`, walking each program list, drawing `n FOR NAME`, reading the
  keyboard via the GPL `SCAN` opcode, and setting the GPL PC to the chosen entry
  is all done by GPL in the console GROM (the ROM provides the interpreter and
  low-level utilities). This is the heart of cartridge compatibility.

- **TI BASIC is GPL in GROMs 1–2** (~12 KiB), advertised by GROM 1's program list
  as `TI BASIC` → entry GROM `>216F`. **We do not reproduce it**; TI PYTHON takes
  its menu slot. GROM 2 has no `>AA` header — it is continuation code for BASIC,
  so once BASIC is gone, most of `>4000–5FFF` is simply free.

---

## 2. The compatibility contract (what MUST be reproduced)

This is the precise interface our rewritten GROM must present so the genuine ROM
and the real cartridges keep working.

### 2.1 The GROM/GPL header (at the base of each 8 KiB GROM)

Identical structure for console GROM, cartridge GROM, and DSR ROM
(Classic99 `addons/makecart.cpp:974-997`, `console/Tiemul.cpp:3292-3367`):

| Offset | Size | Field | Notes |
|---|---|---|---|
| `>00` | 1 | **Valid byte `>AA`** | header ignored unless `>AA` |
| `>01` | 1 | Version | `>02` on the 99/4A |
| `>02` | 1 | # programs | informational |
| `>03` | 1 | reserved | `>00` |
| `>04` | 2 | **Power-up list** ptr | GPL routines run at boot (`>0000` = none) |
| `>06` | 2 | **Program (menu) list** ptr | the selection-list entries (`>0000` = none) |
| `>08` | 2 | **DSR list** ptr | device service routines (`>0000` = none) |
| `>0A` | 2 | **Subprogram list** ptr | numbered GPL subprograms (`>0000` = none) |
| `>0C` | 2 | interrupt link / reserved | `>0000` in console GROMs |
| `>0E` | 2 | reserved | `>0000` |

**List-entry format** (every list is a singly-linked chain; `Tiemul.cpp:3312-3321`):

```
+0  2 bytes  pointer to NEXT entry   (>0000 = end of list)
+2  2 bytes  start address           (GROM address of the GPL routine)
+4  1 byte   name length  N          (powerup entries omit name; 1-byte name = numeric subprogram id)
+5  N bytes  name (ASCII, uppercase shown on the menu)
```

**The real console GROM headers, decoded (target behavior to match/replace):**

```
GROM 0 @>0000:  AA 02 00 00  0000 0000 1310 1320 0000 0000
                valid, v2, 0 programs; powerup none, program none,
                DSR list >1310 (CS1/CS2 cassette), subprog list >1320 (one: id >03 -> >1573)
GROM 1 @>2000:  AA 02 01 00  0000 214D 0000 4D1A
                program list >214D -> { next 0000, start >216F, "TI BASIC" }
GROM 2 @>4000:  (no >AA header — continuation code for TI BASIC)
```

Our rewrite keeps GROM 0's `>AA 02` header; supplies our own title/menu GPL at the
fixed entry the ROM jumps to; and puts a **program list** advertising
`TI PYTHON` (→ our REPL entry). Whether the BASIC-slot program list should live in
GROM 0 or GROM 1 is decided by R1/R2 (some ROM revisions expect the first menu
program from GROM 1); we replicate whatever the genuine boot trace shows.

### 2.2 The fixed GPL boot entry (reconnaissance R1)

The ROM jumps to a hardcoded GROM address to begin interpreting. Our GPL at that
address must (in order) set up the VDP (pattern/color/name tables, backdrop via
GPL `BACK`), draw the title, then fall into the menu loop. **We must place our
code at that exact address** — so the GPL assembler needs absolute GROM placement
(§5.3), and R1 must determine the address before M1.

### 2.3 The selection-list scan + dispatch (the compatibility core)

To list and launch cartridges, our GROM-0 GPL must reproduce the menu loop:

1. **Scan** GROM bases `>0000,>2000,…,>E000` (and the cartridge GROM the loader
   installs at `>6000`+, `machine.rs:372-379`) plus DSR ROM bases for a `>AA`
   valid byte.
2. For each, **walk the program list**, assigning the next selection number and
   drawing `n FOR NAME` into the name table.
3. **SCAN** the keyboard (GPL `>03`; key lands at scratchpad `>8375`), map the
   pressed digit to the matching program-list entry, set the GPL PC to its **start
   address**, and interpret onward.

A cartridge contributes its menu line through its own `>6000` GROM header with a
program list — `makecart.cpp:1509-1517` is a worked single-program example. If our
scan/dispatch matches the contract, **every** bundled cartridge lists and launches
unchanged.

### 2.4 Standard GPL subprograms cartridges may call (risk area)

Some cartridges/BASIC call **numbered GPL subprograms** in console GROM (GROM 0's
subprogram list at `>1320`, e.g. id `>03`). Reconnaissance R3 (§6) enumerates
which subprograms the bundled set actually invokes; we reimplement those that are
used and document any we deliberately drop. This is the least-understood part of
the contract and the biggest compatibility risk (§11).

### 2.5 Scratchpad / VDP conventions

GPL uses fixed scratchpad cells at `>8300–837F` (key `>8375`, keyboard mode
`>8374`, random `>8378`, joystick `>8376/77`, GPL status/flags, the GPL PC/stack
pointers the interpreter maintains). Our code must use these the way the
interpreter expects; R1's trace plus the GPL spec in `addons/gpl.cpp:336-446`
pin the exact cells.

---

## 3. What we replace (original content)

- **Title text:** replace the ASCII at GROM `>0490` (`1981 TEXAS INSTRUMENTS` /
  `HOME COMPUTER`) with `JOEL ODOM ROM REWRITE` (21 chars) and
  `(WWW.JOELODOM.COM)` (18 chars) — both fit the 32-column screen, on the two
  lines the copyright occupied. Replace the `READY-PRESS ANY KEY TO BEGIN` text
  (`>014B`) with our own wording if desired.
- **Logo:** replace the stylized "TI 99/4A" character-pattern bitmaps in GROM 0
  with our own banner patterns (a TI-style logo is also a trademark concern, so we
  make our own).
- **TI BASIC → TI PYTHON:** the `TI BASIC` program-list entry and its ~12 KiB of
  GPL are gone; the menu slot points at our TI PYTHON REPL (§9). This is a net
  *reduction* in GROM content.

---

## 4. Strategy & IP stance

**Keep the genuine console ROM; rewrite the GROM as original GPL.** Rationale:

- The console ROM is the GPL interpreter; reusing it guarantees our GPL runs
  identically to real GROMs — no interpreter bugs of our own, and automatic
  fidelity for the subtle bits (prefetch, the `>9802` flip-flop reset).
- It bounds the work: we write GPL *content*, not a CPU-code interpreter. Phase 2
  (a clean console ROM) can come later without blocking this.

**IP stance.** We respect the spirit of TI's IP by replacing the *copyrighted
creative content* (title text, logo, BASIC) with original work, and reproducing
only the *uncopyrightable interface* required for interoperability: the header
byte layout, the GPL entry/dispatch contract, and the chip port protocol — facts
about how to be compatible, not TI's expression of them. We consult the real
firmware's **behavior** (via the GROM tracer and Classic99) to learn that
interface; we copy none of TI's GPL source (there is none in either repo anyway —
only the 24 KiB binary). All on-screen content and all GPL we ship is new.

---

## 5. Milestone 0 — the GPL toolchain (the hard prerequisite)

Nothing else can start until we can turn GPL source into a GROM image. Today
`libre99-asm` is **TMS9900-only** (71-entry 9900 ISA, origin forced to `>6000`,
single 8 KiB ROM bank; `ASSEMBLER.md` is aspirational/un-implemented beyond that).
But its **foundations are reusable**: the lexer (`crates/libre99-asm/src/lex.rs`),
the left-to-right E/A expression evaluator (`src/expr.rs`), the two-pass
symbol/forward-reference driver pattern (`src/lib.rs:90-203`), and — already in
core — the container writer that **accepts GROM pages**:
`libre99_core::cartridge::write_v1(title, cru_base, rom, grom)`
(`crates/libre99-core/src/cartridge.rs:156-193`; the assembler currently passes
`&[]`).

### 5.1 The GPL instruction set to support

From the 256-entry table in `classic99/addons/gpl.cpp:46-314` (with TI's
per-opcode specs in the comments through `:446`; the bodies are stubbed because
the file is `#if 0`). The encoder must handle the **five operand addressing
forms** (`gpl.cpp:317-334`):

| Form | First operand byte | Meaning |
|---|---|---|
| 1 | `0aaaaaaa` | direct CPU scratchpad `>8300–837F` |
| 2 | `10 V I aaaaa` | V: 0=CPU/1=VDP RAM; I: 0=direct/1=indirect |
| 3 | `11 V I aaaaa` + index byte | form 2, indexed |
| 4 | `10 V I 1111` + 2 addr bytes | extended 16-bit addr (biased by `>8300`) |
| 5 | `11 V I 1111` + 2 addr bytes + index | extended, indexed |

Opcode groups to encode (single-byte opcode + operands):

- **Control/flow:** `>00 RTN`, `>01 RTNC`, `>05 B addr`, `>06 CALL addr`,
  `>0B EXIT`, `>0F XML` (call ROM machine-language), `>40–5F BR` (cond. relative),
  `>60–7F BS`.
- **Screen/setup:** `>04 BACK imm` (VDP R7 backdrop), `>07 ALL`, `>08 FMT` (the
  screen-layout sub-language), `>20–3F MOVE` (block move CPU/VDP/GROM — the
  drawing workhorse).
- **Input/util:** `>02 RAND`, `>03 SCAN` (keyboard/joystick), `>0E PARSE`.
- **ALU:** `>80–9F` unary (`FETCH CLR INV NEG INC DEC INCT DECT …`, byte and `D`
  word forms), `>A0–AF` `ADD/SUB/MUL/DIV`, `>B0–BF` `AND/OR/XOR/ST/DST`,
  `>C0–DF` compare/exchange (`CEQ CGT CGE CH CHE CLOG EX SRA …`, set the GPL
  condition bit), `>E0–EF` shifts.

Status model: compares set the condition bit; `BR`/`BS` branch on it; `RTN`
clears it, `RTNC` preserves it (`gpl.cpp:336-408`).

### 5.2 Toolchain shape — decision

**Recommended: a new sibling crate `crates/libre99-gpl`** that shares the E/A
lexer and expression engine with `libre99-asm` (extract `lex.rs` + `expr.rs` into a
tiny shared crate, e.g. `libre99-asm-syntax`, or re-export them). It gets its own GPL
ISA table (analogous to `isa.rs`), GPL operand/addressing encoder, GROM placement,
GPL header synthesis, and **system-GROM image** output.

*Why a new crate, not a `--lang gpl` mode of `libre99-asm`:* GPL's instruction set,
addressing forms, location model (absolute GROM addresses), and output container
(a raw multi-slot GROM image, not a `>6000` cartridge) are all different. Keeping
`libre99-asm` a clean TMS9900 cartridge assembler — and `libre99-gpl` a clean GPL/GROM
assembler — matches this repo's preference for sharp module boundaries. The
*alternative* (one crate, two ISA tables, a mode flag) is viable and slightly less
code; choose it only if the shared-syntax extraction proves heavier than expected.

Public API mirrors `libre99-asm` (`assemble(src, opts) -> Result<Assembly, Vec<Diag>>`)
so tests can assemble in-memory and the same patterns apply.

### 5.3 Directives the GPL assembler needs

- **`GROM >addr` / `AORG`** — absolute GROM placement (unlike `libre99-asm`'s forced
  `>6000`). Required because GPL `B`/`CALL` take absolute GROM addresses and the
  ROM jumps to a fixed entry (§2.2). Labels resolve to GROM addresses.
- **`BYTE` / `DATA` / `TEXT` / `BSS` / `EVEN` / `EQU`** — same data/symbol
  directives `libre99-asm` already implements (reuse the semantics).
- **Assisted GROM header (extension), mirroring `libre99-asm`'s auto cartridge
  header** (`lib.rs:293-313`): a directive set like `HEADER` / `POWERUP label` /
  `PROGRAM 'NAME', entry` / `SUBPROG id, entry` that synthesizes the `>AA` header
  and the linked program/powerup/subprogram lists from declarations — so source
  needn't hand-lay header bytes (the same ergonomics that make titris' source
  header-free).
- **Multi-slot layout:** emit GROM 0 at `>0000`, optionally GROM 1 at `>2000`,
  etc., honoring the 8 KiB slot wrap (don't let a routine straddle a slot
  boundary silently — warn, as `libre99-asm` warns on bank crossings).

### 5.4 Output formats

- **Raw system-GROM image** (default for this project): a `.bin` laid out at GROM
  addresses `>0000…`, byte-for-byte loadable where `994AGROM.Bin` is loaded
  (`Machine::new(CONSOLE_ROM, CONSOLE_GROM)`; `grom.load(0x0000, …)`). This is the
  artifact `--system-grom` consumes (§7) and the embed eventually replaces.
- **Cartridge GROM pages** (free, since the container already supports them): wire
  the assembler to pass real `(grom_addr, page)` tuples to
  `cartridge::write_v1(…, grom)` instead of `&[]`, so the same tool can also build
  **GPL cartridges** — a useful bonus and a good early test vehicle.

### 5.5 A GPL disassembler (for reconnaissance + differential testing)

Build a small GPL disassembler (the inverse of the encoder). Two payoffs:
(1) it turns R1's raw `grom_log` address trail into readable GPL so we can
understand the real title/menu code we're replacing; (2) it's the basis for
**golden round-trip tests** (assemble → disassemble → compare) and for
differential checks of our header/list bytes against the real GROM's.

### 5.6 M0 validation gate

- Golden **encoding** unit tests for every GPL opcode/addressing form (the
  `libre99-asm` ISA golden-test pattern, `lib.rs:654-739`).
- An **integration test**: assemble a trivial GROM (valid `>AA` header + a few
  GPL ops at the fixed entry that set the backdrop color via `BACK` and halt),
  load it as the system GROM with the real console ROM, run N frames, and assert
  the VDP backdrop register took our value — proving the real interpreter executed
  our GPL. This is the GPL analogue of `ASSEMBLER.md`'s "smallest cartridge —
  a solid colored screen."

---

## 6. Reconnaissance (recover the real boot/menu flow)

We have no GROM **source** — only the 24 KiB binary — so we recover the contract
empirically with the emulator's built-in tracer (`grom_record(on)` / `grom_log()`,
`machine.rs:508-517`, recording `(addr,byte)` of every GROM read).

- **R1 — the fixed GPL entry + title draw.** Run `boots_to_master_title_screen`
  (`tests/boot.rs:38`) with `grom_record(true)`; the **first GROM read after
  reset** is the ROM's hardcoded GPL entry. Disassemble (§5.5) the trail to map
  the VDP setup, the title text/pattern placement, and the scratchpad cells used.
  *Output: the entry address and a readable listing of the title/powerup GPL.*
- **R2 — the menu scan/dispatch.** Continue the trace past the title into the
  selection loop; mount a cartridge (e.g. `parsec`/`blasto`) and trace how its
  `>6000` header is scanned, listed, and dispatched on keypress. *Output: the
  scan/list/SCAN/dispatch algorithm to reimplement.*
- **R3 — subprograms cartridges call.** With the tracer on, boot/launch a sample
  across the 137 cartridges and record which **console-GROM subprograms** (calls
  into `>0000–5FFF` from cartridge code) actually fire. *Output: the must-keep
  subprogram set, and a documented drop-list.*

R1 gates M1; R2 gates M2; R3 gates M5.

---

## 7. Emulator integration

- **`--system-grom <path>` (and `--system-rom <path>`) CLI overrides** in
  `libre99-app`, mirroring `--cartridge-file` (`README.md:180`): load the given image
  in place of the embedded `994AGROM.Bin` for one run. This is how we iterate
  without touching the embed. (The override skips the resume-from-savestate path,
  like the existing media flags.)
- **A rewrite boot test** in `libre99-core`, the analogue of
  `boots_to_master_title_screen`: assemble our GROM in-memory via `libre99-gpl`, build
  `Machine::new(CONSOLE_ROM, our_grom)`, run N frames, and assert our title text /
  colors are on screen. This keeps the GROM from silently breaking as the core or
  toolchain evolves (exactly how `tests/titris.rs` guards titris).
- **Eventual embed + selection.** Once stable, embed `console-grom.bin` and add a
  preference/flag to boot the **rewrite** vs the **authentic TI** GROM — we can
  even default to the rewrite (it's original content) and keep the TI image as the
  "authentic firmware" option. Update `crates/libre99-app/build.rs` and the
  README/help per the repo's "keep docs and help in sync" rule.

---

## 8. Milestones & validation gates

TDD throughout, mirroring `docs/STATUS.md`'s gate style. Each milestone is
shippable and tested before the next starts.

| # | Milestone | Validation gate |
|---|---|---|
| **M0** | **GPL toolchain** — `libre99-gpl` crate: GPL ISA + 5 addressing forms, GROM placement, assisted header synthesis, raw system-GROM + cartridge-GROM output, GPL disassembler. | Golden encoding tests for all opcodes/forms; integration test boots a trivial GPL GROM on the real ROM and observes its effect (§5.6). |
| **R1** | **Boot-flow recon** — trace the fixed GPL entry + title/powerup code. | A documented entry address + disassembled title routine in `grom/README.md`. |
| **M1** | **Title screen** — header + GPL at the fixed entry: VDP setup, draw `JOEL ODOM ROM REWRITE` / `(WWW.JOELODOM.COM)` + our logo, wait for a key. | `boots_to_rewrite_title_screen` asserts our text/colors on the framebuffer; manual screenshot matches. |
| **R2** | **Menu recon** — trace scan/list/dispatch with a cartridge mounted. | Documented menu algorithm. |
| **M2** | **Selection list** — scan all GROM/cartridge headers, render `n FOR NAME`, SCAN the keyboard, dispatch to the chosen entry. | Test: mount a real cartridge (e.g. `blasto`), assert it appears as a numbered entry and that selecting it transfers control to its entry. |
| **M3** | **Cartridge compatibility sweep** — verify the menu lists/launches across the bundled set; `QUIT` (`FCTN-=`) returns to the title. | Regression test booting a representative sample of the 137 cartridges on our GROM and asserting each lists + dispatches; any failures triaged. |
| **M4** | **TI PYTHON v0** — the REPL program in the BASIC menu slot (§9). | Test types `2+3`→`5`, `x = 7`, `x*6`→`42`, an error case→`SYNTAX ERROR`; `QUIT`→title. |
| **R3 / M5** | **Other-ROM compatibility** — reimplement the console-GROM subprograms cartridges actually call (R3); stub/document cassette + speech gaps; confirm the disk DSR handshake still works. | Tunnels-of-Doom-style disk title still boots on our GROM; documented subprogram coverage. |
| **M6** | **Embed & polish** — embed `console-grom.bin`, add the rewrite/authentic toggle, write `grom/README.md`, update repo README + help overlay. | Default-rewrite boot works in the desktop app; docs/help updated; full suite green. |

---

## 9. TI PYTHON v0 — language specification

Deliberately **super lightweight**: an interactive, immediate-mode **integer
calculator with variables**. No stored programs, no control flow, no functions, no
strings, no floats — those are explicit non-goals for v0 (candidates for a later
TI PYTHON v1). It exists to be a tasteful, original replacement for BASIC's menu
slot, and a second real exercise of the GPL toolchain.

**Entry/exit.** Reached as `1 FOR TI PYTHON` from the selection list. Shows a
banner (`TI PYTHON 0.1`) and a `>>> ` prompt. `FCTN-=` (`QUIT`) returns to the
master title screen (BASIC's `BYE`).

**Per line, the REPL:** reads a line into a VDP/CPU buffer, tokenizes, parses,
evaluates, and prints the integer result (or an error) on the next line, then
re-prompts.

**Grammar (v0):**

```
line       := assignment | expression
assignment := NAME '=' expression
expression := term   (('+' | '-') term)*
term       := factor (('*' | '/' | '%') factor)*
factor     := NUMBER | NAME | '(' expression ')' | ('+'|'-') factor
NAME       := letter (letter|digit)*        ; first 1–2 chars significant, capped table
NUMBER     := digit+                          ; decimal; values are 16-bit signed
```

**Semantics.**

- Integers are **16-bit** (the GPL/9900 native word); document wraparound. `/` is
  integer division; `%` is remainder; divide-by-zero → `ZERO DIVISION ERROR`.
- Variables live in a small fixed table in RAM (e.g. up to 16 names). Assignment
  stores; a bare expression prints its value. Referencing an unset name →
  `NAME ERROR`.
- Operator precedence is real (`*` `/` `%` above `+` `-`), parentheses group —
  implemented as a small **recursive-descent** evaluator using GPL `CALL`/`RTN`
  (the GPL substack makes recursion natural). Arithmetic uses GPL
  `ADD/SUB/MUL/DIV` on RAM words.
- Errors print a short message (`SYNTAX ERROR`, `NAME ERROR`,
  `ZERO DIVISION ERROR`) and re-prompt — no crash.

**Session example:**

```
TI PYTHON 0.1
>>> 2 + 3 * 4
14
>>> x = 7
>>> x * (x - 1)
42
>>> y
NAME ERROR
>>> 10 / 0
ZERO DIVISION ERROR
>>>
```

**Implementation note.** This is the most code-heavy GPL we write (a tokenizer +
recursive-descent evaluator + decimal I/O, all in bytecode). Build it
incrementally behind tests: (a) echo a typed line; (b) parse/print a single
integer literal; (c) `+`/`-`; (d) `*`/`/`/`%` + parentheses; (e) variables; (f)
errors. Decimal printing reuses the repeated-divide idiom (the GPL analogue of
titris' `DIV`-based score display).

**v1 ideas (not now):** string literals + `print()`, comparisons/booleans, a few
built-ins (`abs`, `min`, `max`), and possibly a single retained expression
history. Stored multi-line programs remain out of scope by design.

---

## 10. Testing strategy

- **Unit (toolchain):** golden GPL encodings per opcode/addressing form;
  assemble→disassemble round-trips; header/list-byte synthesis matches the §2.1
  layout.
- **Integration (firmware):** the M0 trivial-GROM boot; `boots_to_rewrite_title_screen`
  (M1); the menu list/dispatch test (M2); the TI PYTHON REPL test (M4) — all
  in-memory (assemble the GROM in the test, no committed binary needed to pass),
  the way `tests/titris.rs` works.
- **Regression (compatibility):** the cartridge sweep (M3) and the disk-title
  boot (M5), on our GROM, guarding the contract.
- **Differential:** compare our header/list bytes and (where we intend to match)
  menu behavior against the real GROM via the tracer — our deviations should be
  *only* the intended content changes (text, logo, BASIC→PYTHON).

Per `CLAUDE.md`: the toolchain lives where `cargo test -p libre99-core` and the new
`-p libre99-gpl` run with zero third-party deps; keep `cargo clippy` clean. The
committed `console-grom.bin` is a build artifact like `titris.ctg` — rebuild and
re-commit it when the source changes (and keep its `grom/README.md` address
assertions current), per the repo's "committed artifact" convention.

---

## 11. Risks & open questions

- **R1 risk — the fixed GPL entry & scratchpad conventions.** If the title code
  depends on ROM-set scratchpad state we don't reproduce, the screen won't draw.
  *Mitigation:* trace first; reuse the ROM's exact setup sequence; keep the M0
  trivial-GROM test as the minimal reproducer.
- **Menu-program placement.** Some ROM revisions expect the first menu program
  from **GROM 1**, not GROM 0. *Mitigation:* R2 shows what the genuine boot does;
  replicate it.
- **Subprograms cartridges call (the big one).** If a cartridge calls a
  console-GROM subprogram we dropped, it may misbehave after launch (listing/launch
  still works; in-game features might not). *Mitigation:* R3 enumerates the
  actually-used set; reimplement those; document the rest as known gaps (mirroring
  the README's honest "known limitations" culture).
- **FMT sub-language.** GPL `FMT` (`>08`) is a mini screen-layout language; if the
  title/menu lean on it heavily, the encoder must support its sub-opcodes.
  *Mitigation:* R1 reveals usage; implement only what's used.
- **Open — embed default.** Ship the rewrite as the *default* GROM (it's original
  content) with TI as an option, or keep TI default and the rewrite opt-in?
  Proposed: rewrite opt-in via `--system-grom` until M6, then offer a preference.
- **Open — TI PYTHON surface.** v0 is integers + variables. Confirm that's the
  desired bar before M4, or pull a v1 feature (e.g. `print()` + strings) forward.

---

## 12. References (code & data)

| Topic | Location |
|---|---|
| "Run real firmware; don't reimplement GPL" | `crates/libre99-core/src/lib.rs:6-11`; `docs/ARCHITECTURE.md:19-24`; `docs/PLAN.md:4-6` |
| GROM chip model / prefetch / slot-wrap | `crates/libre99-core/src/grom.rs:42-184` |
| GROM port decode `>9800/02`, `>9C00/02` | `crates/libre99-core/src/machine.rs:163-171,224-232` |
| Console GROM load (1:1 @ `>0000`) | `crates/libre99-core/src/machine.rs:88-89` |
| Cartridge GROM mount (`>6000`+) | `crates/libre99-core/src/machine.rs:372-379` |
| Reset vector `WP=>83E0 PC=>0024` | `crates/libre99-core/src/cpu.rs:149-155` |
| Boot test + GROM tracer | `crates/libre99-core/tests/boot.rs:38`; `machine.rs:508-517`; `grom.rs:93-137` |
| `.ctg`/GROM container writer (`write_v1`) | `crates/libre99-core/src/cartridge.rs:156-193` |
| TMS9900 assembler (to reuse lex/expr/driver) | `crates/libre99-asm/src/{lex,expr,lib}.rs`; auto-header `lib.rs:293-313`; golden tests `lib.rs:654-739` |
| GROM/GPL header + list layout | `classic99/addons/makecart.cpp:974-997,1509-1517`; `classic99/console/Tiemul.cpp:3292-3367`; real bytes in `roms/994AGROM.Bin` |
| GPL opcode table + addressing forms | `classic99/addons/gpl.cpp:46-446` (table 46-314, addressing 317-334) |
| Title/menu strings in GROM 0 | `roms/994AGROM.Bin`: `>014B`, `>0490` (title); `>2152`/`>216F` (TI BASIC entry) |
| Cartridge auto-header (model for GPL header synthesis) | `assembler/ASSEMBLER.md:267-371` (cartridge header + `.ctg` container) |
