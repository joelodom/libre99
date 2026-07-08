# TI-99/4A Assembler & Cartridge Builder — Guide & Reference

Status: **shipped** (crate `crates/libre99-asm`, binary `libre99asm`). This document
is the user guide and language reference for the project's from-scratch
**TMS9900 assembler** and **cartridge packager**, which turns TI-99/4A assembly
source into a cartridge image this emulator boots (`libre99-app
--cartridge-file`). It began life as the implementation spec and keeps that
structure: the requirements and design rationale are preserved, the language
and directive references describe the shipped behavior, and anything specified
here but **not yet implemented** (ROM banking, the tagged-object format, and
the richer CLI/diagnostics surface) is explicitly marked *(future)* where it
appears. The shipped CLI is §8; the worked full-scale examples are the
[Titris](../original-content/cartridges/titris/README.md),
[Sokoban](../original-content/cartridges/sokoban/README.md), and
[Jaywalker 99](../original-content/cartridges/jaywalker99/README.md) cartridges; the
bootstrap record is archived at
[docs/history/ASSEMBLER-POC-PLAN.md](../docs/history/ASSEMBLER-POC-PLAN.md).

It has three parts, as requested:

1. **[Requirements](#1-requirements)** — what the tool must do and the
   constraints it works under.
2. **[Assembly-language reference](#5-assembly-language-reference)** — the source
   language, instruction set, addressing modes, directives, and output formats,
   specified precisely enough to implement and to write code against. Plus the
   **[target-machine / cartridge model](#4-target-machine-model)** the code runs
   in.
3. **[Implementation plan](#9-implementation-plan)** — crate layout, algorithms,
   data structures, milestones, and a test strategy.

A **[cookbook](#10-cookbook-for-humans-and-agents)** of complete, runnable
examples and a set of **[appendices](#11-appendices)** (opcode table, directive
table, object-format tags, memory equates) follow.

> **Grounding.** Every hardware/format claim here is cross-checked against the
> emulator core (file references are given inline: `cpu.rs`, `machine.rs`,
> `cartridge.rs`, `vdp.rs`, `cru.rs`, `grom.rs`, `keyboard.rs`) and against the
> 137-image cartridge test corpus (run-time media from the git-ignored
> `third-party/`). Where the original TI *Editor/Assembler* (E/A)
> defines the language, we match it; deviations and extensions are called out
> explicitly. The authority for *encoding* is `cpu.rs` (what the emulated CPU
> actually decodes); the authority for the *cartridge container* is
> `cartridge.rs` (what the emulator actually loads).

---

## Table of contents

1. [Requirements](#1-requirements)
2. [Goals and non-goals](#2-goals-and-non-goals)
3. [Background and rationale](#3-background-and-rationale)
4. [Target-machine model](#4-target-machine-model)
5. [Assembly-language reference](#5-assembly-language-reference)
6. [Directive reference](#6-directive-reference)
7. [Output formats](#7-output-formats)
8. [Command-line interface and diagnostics](#8-command-line-interface-and-diagnostics)
9. [Implementation plan](#9-implementation-plan)
10. [Cookbook (for humans and agents)](#10-cookbook-for-humans-and-agents)
11. [Appendices](#11-appendices)

---

## 1. Requirements

Identifiers `FR-*` (functional) and `NFR-*` (non-functional) are referenced by the
implementation plan and test strategy.

### 1.1 Functional requirements

- **FR-1 — Input language.** Accept TI-99/4A assembly source in the *Editor/
  Assembler* dialect (source-line format, expressions, directives, mnemonics, and
  addressing-mode syntax of §5–§6). Existing E/A source and the many published
  tutorials/snippets should assemble unchanged within the supported subset
  (§5.10).
- **FR-2 — Full TMS9900 instruction set.** Encode every instruction the emulator
  decodes (`cpu.rs`): all dual-operand, single-operand, immediate, jump,
  shift, CRU, and control instructions, plus the `MPY`/`DIV`/`XOP`/`COC`/`CZC`/
  `XOR`/`LDCR`/`STCR` family — see the complete table in §5.6 / Appendix A. The
  9900 has no instructions the emulator lacks, and the emulator has none beyond
  the 9900, so the target is exactly the documented TMS9900 ISA.
- **FR-3 — Primary output: a bootable cartridge.** Produce a `.ctg` image (the
  `ti99sim` V1 container the emulator loads — `cartridge.rs`) that boots in this
  emulator and appears on the console selection screen. This is the default
  output.
- **FR-4 — Standard ML cartridge header.** Emit the standard `>6000` ROM header
  (`>AA` valid flag + program list) so the console lists the program and
  transfers control to it as machine code (§4.3). The tool must support both
  **assisted** header generation (one directive/flag; §6.4) and **manual**
  headers written in source (for full control / E/A portability).
- **FR-5 — Secondary outputs.** Optionally emit: a **raw binary** of the
  `>6000–7FFF` image (for EPROM/other tools); a **TI tagged-object file** (the
  E/A-compatible interchange format, §7.3); a **listing** (address + object +
  source, with a symbol table); and a **machine-readable map** (JSON: symbols,
  sections, sizes, entry point) for tooling and agents.
- **FR-6 — Two-pass assembly with forward references.** Labels may be referenced
  before definition in instruction and data operands. Directives that determine
  layout (`AORG`, `RORG`, `BSS`, `EQU`, …) require well-defined operands at the
  point of use, matching E/A.
- **FR-7 — Expressions.** Evaluate constant expressions over hexadecimal (`>`),
  decimal, and character constants, the location-counter symbol (`$`), and
  symbols, with the operators and absolute/relocatable rules of §5.4.
- **FR-8 — Banking.** Support cartridge images larger than 8 KiB by splitting into
  8 KiB banks selectable by the emulator's scheme (`bank = (addr>>1) &
  (banks-1)`, `machine.rs`), which requires a **power-of-two** bank count (§4.4).
  Single-bank (≤8 KiB) is the common case and the default.
- **FR-9 — Includes.** Support textual source inclusion (`COPY`/`INCLUDE`) with a
  search path, for splitting source and sharing equate/header libraries.
- **FR-10 — Deterministic output.** Identical inputs (and tool version) produce
  byte-identical outputs — no timestamps, no map-iteration nondeterminism, no
  environment dependence. (Required for reproducibility and for agent
  diff-based workflows.)
- **FR-11 — Self-validation hook.** Provide a documented way to *prove a built
  cartridge works*: an integration-test harness that mounts the produced `.ctg`
  in a `libre99-core::Machine`, boots it, and inspects VRAM/registers (the same
  technique as `tests/cartridge.rs::tunnels_of_doom_appears_on_the_selection_screen`).

### 1.2 Non-functional requirements

- **NFR-1 — Pure Rust, minimal deps.** The assembler core is pure `std` with
  **zero third-party dependencies**, matching `libre99-core`'s discipline (see
  `crates/libre99-core/Cargo.toml`). It may depend on `libre99-core` itself (to share
  the `.ctg` container code and to run round-trip tests), nothing else.
- **NFR-2 — Agent-ergonomic.** Diagnostics are precise (`file:line:col`), stable,
  and optionally emitted as JSON (`--json-errors`); exit codes are meaningful
  (§8). Common foot-guns are guarded (odd-address instruction warning, jump
  out-of-range with the exact byte distance, register-name help). `R0`–`R15` are
  predefined (§5.2) so the most common E/A "undefined symbol R1" stumble is gone.
- **NFR-3 — Fast.** Assembling a full 8 KiB cartridge completes in well under a
  second; the tool is suitable for tight edit-build-run agent loops.
- **NFR-4 — Auditable.** The instruction encoder is **table-driven** from a
  single source of truth that can be diffed against `cpu.rs`'s decoder; the
  `.ctg` writer is the inverse of `cartridge.rs`'s reader and is covered by a
  parse-of-write round-trip test.
- **NFR-5 — Documented.** This guide, plus generated `--help`, plus the
  cookbook. (Done: the tool is linked from the `README`, `docs/STATUS.md`, and
  `docs/ROADMAP.md`, and documentation stays current with changes per
  `docs/DEVELOPMENT.md`.)

---

## 2. Goals and non-goals

**Goals**

- Let a human or an AI agent go from a `.asm` file to a cartridge that boots in
  this emulator with **one command** and no manual binary surgery.
- Be **source-compatible** with the TI Editor/Assembler so the large existing body
  of TI-99 assembly knowledge transfers directly.
- Be **correct against this emulator** — the encoder targets exactly what
  `cpu.rs` decodes, validated by running assembled code on `libre99-core::Machine`.

**Non-goals (v1)**

- **GPL / GROM-program cartridges.** 9900 machine code is not executable from
  GROM (GROM is not in CPU address space — `grom.rs`/`machine.rs`), so assembly
  cartridges are **ROM cartridges** in the `>6000–7FFF` window. Authoring GPL
  bytecode or emitting code into GROM pages is out of scope (data-in-GROM is a
  possible later extension, §9.10).
- **A macro assembler.** The TI E/A is not a macro assembler (macros were a
  separate TI product). v1 ships `COPY`/`INCLUDE` only; a small macro facility is
  an optional later extension (§9.10), clearly marked non-E/A if added.
- **The full relocating linker.** Cartridges are assembled **absolute** at
  `>6000` (`AORG`). Relocatable object output (`RORG`, tagged object) is provided
  for E/A interchange, but a multi-module linker is a later extension. The E/A
  relocatable-segment and custom-loader directives (`PSEG`/`PEND`/`CSEG`/`CEND`/
  `DSEG`/`DEND`, `LOAD`/`SREF`) belong to this path; v1 **recognizes** them and
  rejects them with a clear diagnostic rather than miscompiling (§5.10).
- **EA5 "program image" output** (the `>A000` loadable format used by disk-based
  ML). Noted as a future target (§9.10); not v1.

---

## 3. Background and rationale

The emulator already has a complete cartridge pipeline: `cartridge.rs` parses the
`ti99sim` `.ctg` container, `machine.rs::mount_cartridge` installs ROM banks into
`>6000–7FFF` and GROM pages into GROM space, and the console's own GROM menu code
then lists and launches the cartridge. What is missing is a way to *author* new
cartridges. This tool fills that gap on the input side.

Two facts shape the design:

- **Assembly cartridges are ROM cartridges.** Of the 137 corpus images, **33 are
  pure-ROM** (no GROM). Every one of those 33 carries the byte `>AA` at ROM
  `>6000` — the standard header the console scans (verified by decoding the images
  with a port of `cartridge.rs`'s RLE codec). So the recipe is settled: assemble
  9900 code into the `>6000` window behind a standard header.
- **"Compatible with the existing assembler" means the *source language*.** The
  TI deliverable an agent will be told to emulate is the E/A *language*. The
  binary an agent actually needs is a `.ctg` for this emulator. The tool therefore
  speaks E/A on the way in and `.ctg` on the way out, with the E/A tagged-object
  format available as an interchange option.

Worked decode of a real ROM cartridge header (`ant.ctg`, port of `cartridge.rs`):

```text
>6000:  AA 01 01 00            ; >AA valid flag, version 01, #programs 01, reserved 00
>6004:  00 00                  ; power-up list   = >0000 (none)
>6006:  60 10                  ; program list    = >6010
>6008:  00 00                  ; DSR list        = >0000
>600A:  00 00                  ; subprogram list = >0000
>600C:  00 00                  ; ISR/GPL link    = >0000
>600E:  00 00
>6010:  00 00                  ; program-list entry: next = >0000 (last)
>6012:  60 30                  ;                     entry address = >6030
>6014:  14                     ;                     name length = 20
>6015:  "ANT COLONY 0.5      " ;                     menu name (20 bytes)
>6030:  0300 0000  02E0 8340 …  ; entry: LIMI 0 / LWPI >8340 / … (machine code)
```

This is exactly what `libre99asm` will generate.

---

## 4. Target-machine model

The code an assembled cartridge runs in. All values verified against `machine.rs`.

### 4.1 CPU address space (the bus `Tms9900Bus`, `machine.rs`)

| Range | Contents | Bus | Wait states |
|-------|----------|-----|-------------|
| `>0000–1FFF` | Console system ROM (read-only) | 16-bit | 0 |
| `>2000–3FFF` | 32 KiB expansion RAM, low part | 8-bit | 4 |
| `>4000–5FFF` | Peripheral DSR ROM window + device regs (disk card when present) | 8-bit | 4 |
| `>6000–7FFF` | **Cartridge ROM window** (this is where your code lives) | 8-bit | 4 |
| `>8000–83FF` | 256-byte scratchpad RAM, mirrored every 256 bytes | 16-bit | 0 |
| `>8400–87FF` | SN76489 sound chip (write-only) | 8-bit | 4 |
| `>8800–8BFF` | VDP read ports (`>8800` data, `>8802` status) | 8-bit | 4 |
| `>8C00–8FFF` | VDP write ports (`>8C00` data, `>8C02` control) | 8-bit | 4 |
| `>9000–97FF` | Speech (not emulated; reads 0) | 8-bit | 4 |
| `>9800–9BFF` | GROM read ports (`>9800` data, `>9802` address) | 8-bit | 4 |
| `>9C00–9FFF` | GROM write ports (`>9C00` data, `>9C02` address) | 8-bit | 4 |
| `>A000–FFFF` | 32 KiB expansion RAM, high part | 8-bit | 4 |

Notes that matter for code generation:

- **The fast RAM is the 256-byte scratchpad at `>8300–83FF`** (and its mirrors
  down to `>8000`). Put your **workspace** and hot variables here. Everything else
  (including your own ROM at `>6000`) is on the 8-bit multiplexed bus and is
  slower (`wait_states` returns 4).
- **Device ports sit on the high byte of the multiplexed bus.** The VDP, GROM, and
  sound ports each latch only the **even**-address transfer; the odd half of a
  *word* access is discarded with no side effect (`machine.rs` ignores device-port
  accesses where `addr & 1`, matching Classic99). A word write therefore delivers
  just **one** byte — its high byte — to the chip, and *cannot* carry the two-byte
  control/address sequence these ports expect (and a word read would otherwise
  auto-increment the VDP/GROM counter twice). **Always drive the VDP/GROM/sound
  ports with byte instructions (`MOVB`)**, one transfer per byte. This is the
  single most common way a TI cartridge goes wrong; see §10.
- The expansion RAM at `>2000` and `>A000` exists in the emulator unconditionally.
  A cartridge may use it for large buffers, but the scratchpad is always the
  fastest and is enough for most programs.

### 4.2 Reset, workspace, and entry conditions

- **Reset** vectors through `>0000`: the console loads `WP=>83E0, PC=>0024`
  (`cpu.rs::reset`). Your cartridge does **not** get control via reset; it gets
  control from the console menu (§4.3).
- When the console launches your ML program, treat the machine state as
  *undefined for your purposes*: **establish your own environment immediately.**
  The universal prologue (seen verbatim in shipping carts, §3) is:

  ```asm
  START  LIMI 0           ; mask interrupts off while we set up
         LWPI >8300       ; install our own workspace in fast scratchpad
         …                ; set up VDP, then run
  ```

- **Workspace placement.** A workspace is 32 bytes (R0–R15). `>8300` is a good
  choice: fast, and clear of the console's interrupt/GPL area at the top of the
  pad. If you keep interrupts off (`LIMI 0`) you own essentially the whole pad;
  conservatively avoid the console/GPL scratch — the GPL status byte `>837C` and
  the `>83C0–83FF` interrupt/GPL workspace (which includes the user-ISR hook
  `>83C4` and the reset WP `>83E0`). See Appendix D.

### 4.3 The standard cartridge header (`>6000`)

Empirically confirmed across all 33 pure-ROM corpus cartridges. The console's
GROM menu code scans CPU `>6000` for `>AA`; if found, it walks the program list
and lists each entry. Selecting an entry transfers control to its **entry
address** as 9900 machine code.

Header at the base of the ROM (`>6000`):

| Offset | Size | Field | Typical value |
|-------|------|-------|---------------|
| `+0` | 1 | Valid flag — must be `>AA` | `>AA` |
| `+1` | 1 | Version | `>01` |
| `+2` | 1 | Number of programs (informational) | `>01` |
| `+3` | 1 | Reserved | `>00` |
| `+4` | 2 | Power-up routine list pointer | `>0000` |
| `+6` | 2 | **Program (menu) list pointer** | `>600C`+ |
| `+8` | 2 | DSR list pointer | `>0000` |
| `+A` | 2 | Subprogram list pointer | `>0000` |
| `+C` | 2 | Interrupt-link (GPL) pointer | `>0000` |
| `+E` | 2 | (unused; `>0000`) | `>0000` |

> Only `>AA` and the program-list pointer must be correct; the console tolerates a
> range of values for version/#programs (the bundled carts use `>01` and `>FF`
> for version and `>00`/`>01` for #programs). `libre99asm` emits `>AA, >01, >01, >00`.

Each **program-list entry** (a singly-linked list; list ends at a `>0000` link):

| Offset | Size | Field |
|-------|------|-------|
| `+0` | 2 | Pointer to next entry (`>0000` = last) |
| `+2` | 2 | Entry address (a 9900 address in the `>6000` window) |
| `+4` | 1 | Name length *n* |
| `+5` | *n* | Menu name (ASCII; uppercase shown on the menu) |

The console builds the selection list newest-first from the chain (multiple
cartridges/entries each contribute). The user presses the number next to your
entry; the console then runs the entry address.

### 4.4 Cartridge ROM and banking

- The `>6000–7FFF` window is **8 KiB**. A program that fits in 8 KiB is a
  **single bank** — the simplest, default case.
- Larger programs use **8 KiB banks**. The emulator selects a bank when the CPU
  *writes* anywhere in `>6000–7FFF`: `cart_bank = (addr >> 1) & (cart_banks - 1)`
  (`machine.rs::write_cartridge`). To select bank *k*, write any value to
  `>6000 + 2*k`. The mask requires the **bank count to be a power of two**
  (1, 2, 4, 8). The header lives at `>6000` of **bank 0**, which is the bank
  selected at power-up. Code that spans banks must keep the switching logic (and
  any shared trampoline) consistent across banks — typically the low part of
  every bank is identical.
- The emulator supports bank-switched cartridges as described. **`libre99asm`
  currently emits single-bank images only** — the `--banks` flag, the `BANK`
  directive (§6.4), and the §10.4 two-bank cookbook describe the designed
  *(future)* surface, not shipped behavior; a program must fit in 8 KiB today.

### 4.5 The `.ctg` container the emulator loads (`cartridge.rs`)

The packager writes a `ti99sim` **V1** image — the exact format `cartridge.rs`
parses:

```text
0x00  80 bytes  Banner "TI-99/4A Module - <title>\n\x1A", zero-padded to 80
0x50   1 byte   Version marker; high nibble = 1  (i.e. 0x10)
0x51   2 bytes  CRU base, big-endian (>0000 for a plain ROM cartridge)
0x53   …        Region records until EOF:
                  1 byte  index   (CPU 4 KiB page 0..15; here pages 6 and 7)
                  1 byte  #banks
                  per bank:
                    1 byte   bank type (2 = ROM, data follows)
                    …        RLE-compressed 4 KiB page (see below)
```

- An 8 KiB ROM bank is stored as **two 4 KiB CPU pages**: index `6` (the low half,
  `>6000–6FFF`) and index `7` (the high half, `>7000–7FFF`), each with `#banks`
  equal to the number of cartridge banks.
- **RLE codec** (`cartridge.rs::rle_decode`, from `ti99sim`'s `compress.cpp`):
  records of a **little-endian 16-bit tag**. If bit 15 is set it is a *run* — count
  = `tag & 0x7FFF`, followed by one byte repeated that many times. Otherwise it is
  a *literal* — `tag` bytes verbatim (a zero literal tag is illegal). Records
  repeat until the 4 KiB page is full.
- The packager may emit each 4 KiB page as a **single literal run** (tag
  `0x1000` = bytes `00 10`, then 4096 bytes) — valid and trivial — or apply real
  run-length compression as a size optimization. Either round-trips through
  `cartridge.rs` exactly.

CRU base is `>0000` for ordinary ROM cartridges.

**Metadata in the container.** The `.ctg` format is minimal: its only descriptive
field is the **title** in the banner (≤ ~60 chars). There is no slot for author,
version, date, or description. Two distinctions follow:

- The banner **title** is consumed only by the *emulator* (media browser, window
  title); the real console never reads it. The name shown on the **console's own
  selection screen** comes from the `>6000` cartridge header (§4.3) — bytes *inside
  the ROM image*, written by the assisted header (§6.4) or by hand. The assembler
  keeps the two in sync by default (both from `--name`/`IDT`).
- **Program data** (fonts, patterns, text, tables) is *not* container metadata — it
  is just bytes in the ROM image, emitted inline by `BYTE`/`DATA`/`TEXT`. The
  container draws no code/data distinction; the `>6000–7FFF` bank is one flat image.

Descriptive build metadata that has no home in the `.ctg` (tool version, entry
point, source provenance) belongs in the `.map.json` sidecar (§7.5), not the
cartridge. A richer metadata-bearing container (e.g. MAME's RPK, a ZIP with a
`layout.xml`) is a separate, out-of-scope decision.

---

## 5. Assembly-language reference

The input language. It follows the TI Editor/Assembler; §5.10 lists the precise
relationship (supported subset, extensions). Examples use `>` for hex throughout,
as TI does.

### 5.1 Source-line format

A source line has up to four fields, in order: **label**, **mnemonic**,
**operands**, **comment**. Fields are separated by **one or more spaces or tabs**.

```text
LABEL   MNEMONIC  OP1,OP2     comment text to end of line
```

Rules:

- **Label field** — if a line begins in **column 1** with a non-blank, non-`*`
  character, that token is a label. A label may also be written with a trailing
  colon (`LABEL:` — accepted; the colon is not part of the name) as an
  agent-friendly extension. A line whose first character is a space has no label.
- **Full-line comment** — a `*` in **column 1** makes the whole line a comment.
- **Trailing comment** — anything after the operand field (separated by
  whitespace) is a comment. A line with a label/mnemonic but no operands may carry
  a comment after the mnemonic. For clarity, a `;` may also begin a trailing
  comment (extension; E/A uses whitespace only).
- **Case** — mnemonics and directives are case-insensitive (`mov` = `MOV`).
  **Symbols are case-sensitive** by default (E/A uppercases everything; agents
  benefit from case sensitivity). `--fold-case` restores strict E/A
  case-insensitivity for symbols.
- **Operand separator** — operands are separated by commas; spaces inside the
  operand field end the field (so no spaces around the comma).
- **Line length / continuation** — lines may be of any length; there is no column
  cap (E/A capped at 80). No line-continuation character (E/A had none).

### 5.2 Symbols and labels

- A **symbol** is 1+ characters from `A–Z a–z 0–9 _`, not starting with a digit.
  `$` is **not** a symbol character — it is the location counter (§5.4) — so the
  two never collide. (E/A allowed up to 6 significant characters; `libre99asm` keeps
  the **full length significant** — a documented relaxation — and warns if
  `--ea-strict` and a name exceeds 6.)
- A **label** defines a symbol equal to the current location counter (its
  relocatability follows the active section — §5.4).
- **`R0`–`R15` are predefined** to `0`–`15` (extension/NFR-2). Source may redefine
  them with `EQU` without error provided the value is unchanged; defining `R3 EQU
  3` (as real E/A source often does) is therefore a no-op, and existing E/A source
  still assembles. `WR0`–`WR15` are *not* predefined.
- A symbol may be **defined once**. Re-definition is an error (except the
  register no-op above). Forward references are allowed in operands (§5.7).

### 5.3 Constants

- **Decimal** — a run of digits: `100`, `0`, `65535`. Range checked per context.
- **Hexadecimal** — `>` followed by hex digits: `>1F`, `>8C02`, `>FFFF`.
- **Binary** — `:` followed by binary digits: `:1010`. This is a `libre99asm`
  extension — **TI E/A has no binary constant** (§5.10) — and is rejected under
  `--ea-strict`. The leading-`:` binary prefix is distinct from the optional
  trailing-colon label form (`LABEL:`, §5.1): one opens an operand, the other
  closes a label, so they never collide.
- **Character constant** — one or two characters in single quotes: `'A'` = `>41`,
  `'AB'` = `>4142`. A doubled quote inside is a literal quote: `''''` = `>27`.
  Used anywhere a number is expected (a 2-char constant is a 16-bit value).
- **String** — only in `TEXT` (and the assisted-header name): characters in
  single quotes, e.g. `TEXT 'HELLO'`; emits one byte per character (ASCII).

### 5.4 Expressions

Operands are **expressions**. The evaluator supports:

- **Operands**: numeric/character constants, symbols, and the location counter
  **`$`** (the address of the *current* instruction/data being assembled).
- **Operators** (E/A set): unary `+` `-`; binary `+` `-` `*` `/`. Evaluation is
  **strictly left to right with no operator precedence** (this is E/A behavior:
  `2+3*4` = `20`, not `14`). `libre99asm` matches E/A and, under `--warn-precedence`,
  warns when left-to-right differs from conventional precedence so agents are not
  surprised. Parentheses are **not** part of E/A expressions and are reserved for
  the indexed addressing form (§5.5); they are *not* general grouping. (An
  optional `--ext-expr` mode adds parenthesized grouping and C-style precedence
  for new code — clearly non-E/A.)
- **Division** truncates toward zero; divide-by-zero is an error.
- **Width** — arithmetic is 16-bit, wrapping (`mod >10000`), matching the machine
  word. Range checks apply at *use* (a `BYTE` operand must fit 8 bits; a jump
  displacement must fit the field — §5.6).

**Absolute vs. relocatable.** Every value carries a *relocation type*:

- **Absolute** — a pure constant, or any symbol defined in an `AORG` (absolute)
  section. Independent of where the program loads.
- **Relocatable** — a label defined in a `RORG` (relocatable) section; its value
  is *program-base + offset*.

The legal algebra (E/A rules), where A = absolute, R = relocatable:

| Expression | Result |
|-----------|--------|
| A ± A | A |
| R + A, A + R | R |
| R − A | R |
| R − R | A (offset within the same section) |
| A × A, A / A | A |
| R × anything, R / anything, A + R + R … | **error** (ill-defined relocation) |

Contexts requiring a **well-defined** value (evaluable now — no forward reference,
the E/A sense of the term): `AORG`, `RORG`, `DORG`, `BSS`, `BES`, `BYTE` operands,
`EQU` (operand), shift counts, CRU/`XOP` counts, and bank numbers; an undefined
symbol in any of these is an error (matches E/A). Most of these *additionally*
require the value to be **absolute**, since a relocatable origin, byte count, byte
value, shift count, or bank number is meaningless — but **`EQU` does not**:
`FOO EQU LABEL` with a relocatable `LABEL` is legal and makes `FOO` relocatable
(§6.2). Instruction and `DATA` operands, by contrast, may be relocatable and may be
forward references (`BYTE` may not — §5.7).

For a **cartridge** (assembled `AORG >6000`) everything is absolute — relocation
matters only for the optional tagged-object output (§7.3).

### 5.5 Addressing modes (operand syntax)

The TMS9900 general-operand modes (the 2-bit *T* field + register, `cpu.rs::resolve`):

| Syntax | Mode (T) | Meaning | Extension word |
|--------|----------|---------|----------------|
| `Rn` / `n` | 0 — register | operand is workspace register *n* | — |
| `*Rn` | 1 — indirect | operand at the address in *Rn* | — |
| `@EXPR` | 2 — symbolic | operand at address `EXPR` | yes (the address word) |
| `@EXPR(Rn)` | 2 — indexed | operand at `EXPR + Rn` (Rn ≠ R0) | yes (the base word) |
| `*Rn+` | 3 — auto-increment | operand at `*Rn`, then `Rn += 1` (byte) or `2` (word) | — |

Notes:

- `@EXPR(R0)` is **not** indexable — index register R0 means "no index" in the
  encoding (`cpu.rs`: `reg == 0` ⇒ pure symbolic). `libre99asm` **errors** on an
  explicit `(R0)` index to catch the mistake.
- Symbolic/indexed modes consume one **extension word** that follows the
  instruction word (source operand's word precedes the destination operand's word
  — the order `cpu.rs` fetches them).
- Other operand kinds, by instruction format:
  - **Immediate** (`LI`, `AI`, `ANDI`, `ORI`, `CI`, `LIMI`, `LWPI`): an
    expression giving the 16-bit immediate, e.g. `LI R0,>1234`.
  - **Jump** (`JMP`, `JEQ`, …): a target **address expression**; the assembler
    computes the signed word displacement and range-checks it (§5.6).
  - **Shift count** (`SLA`, `SRA`, `SRL`, `SRC`): `op Wreg,count`, count `0–15`
    (`0` ⇒ take the count from R0's low 4 bits at run time, and if *that* is also 0
    the shift is 16; `cpu.rs::exec_shift`).
  - **CRU bit** (`SBO`, `SBZ`, `TB`): a signed displacement expression added to
    the CRU base in R12 (range −128…127).
  - **CRU multi-bit** (`LDCR`, `STCR`): `op src,count`, count `0–15` (0 ⇒ 16).
  - **XOP**: `XOP src,n`, n `0–15`.

### 5.6 Instruction set and encoding

The complete set the emulator decodes (`cpu.rs`). Bit layouts use TI numbering
(MSB = bit 0). `B`=byte bit, `Td`/`Ts`=2-bit type, `D`/`S`=4-bit register,
`C`=count/displacement field, `W`=workspace register, `Reg`=register.

A compact quick-reference table (encoding base, format, flags, base cycles) is in
**Appendix A**; the formats are:

**Format I — two general operands.** `oooo Td D Ts S` (top nibble is the opcode;
low bit of the nibble selects the byte variant for the arithmetic/move ops).

| Mnemonic | Base | Op | Flags set |
|----------|------|----|-----------|
| `SZC`  | `>4000` | dst ← dst AND NOT src | L A E |
| `SZCB` | `>5000` | byte | L A E P |
| `S`    | `>6000` | dst ← dst − src | L A E C O |
| `SB`   | `>7000` | byte | L A E C O P |
| `C`    | `>8000` | compare src to dst | L A E |
| `CB`   | `>9000` | byte compare | L A E P |
| `A`    | `>A000` | dst ← dst + src | L A E C O |
| `AB`   | `>B000` | byte | L A E C O P |
| `MOV`  | `>C000` | dst ← src | L A E |
| `MOVB` | `>D000` | byte | L A E P |
| `SOC`  | `>E000` | dst ← dst OR src | L A E |
| `SOCB` | `>F000` | byte | L A E P |

Encoded as `base | (Td<<10) | (D<<6) | (Ts<<4) | S`.

**Format II — jumps** (`oooooooo dddddddd`, signed 8-bit word displacement) and
**single-bit CRU** (same shape, displacement is a CRU offset):

`JMP >1000  JLT >1100  JLE >1200  JEQ >1300  JHE >1400  JGT >1500  JNE >1600
JNC >1700  JOC >1800  JNO >1900  JL >1A00  JH >1B00  JOP >1C00` ;
`SBO >1D00  SBZ >1E00  TB >1F00` (TB → EQ).

Jump target: `disp = (target − ($+2)) / 2`, must be an even byte distance with
`disp ∈ [−128, +127]` (i.e. target within −254…+256 bytes of the jump instruction
itself). Out of range is an error reporting the exact distance and the max.

**Format III — `COC`/`CZC`/`XOR`** (`oooooo D Ts S`): `COC >2000` (EQ),
`CZC >2400` (EQ), `XOR >2800` (L A E). Encoded `base | (D<<6) | (Ts<<4) | S`.

**Format IV — CRU multi-bit** (`oooooo C Ts S`): `LDCR >3000`, `STCR >3400`
(L A E, P if ≤8 bits). `base | (count<<6) | (Ts<<4) | S`, count 0⇒16.

**Format V — shifts** (`oooooooo C W`): `SRA >0800`, `SRL >0900`, `SLA >0A00`,
`SRC >0B00` (L A E C; `SLA` also O). `base | (count<<4) | W`, count 0⇒from R0.

**Format VI — single general operand** (`oooooooooo Ts S`):

`BLWP >0400  B >0440  X >0480  CLR >04C0  NEG >0500  INV >0540  INC >0580
INCT >05C0  DEC >0600  DECT >0640  BL >0680  SWPB >06C0  SETO >0700  ABS >0740`.

Flags: `NEG`/`INC`/`INCT`/`DEC`/`DECT`/`ABS` set L A E C O; `INV` sets L A E;
`CLR`/`SETO`/`SWPB`/`B`/`BL`/`BLWP`/`X` set none. `base | (Ts<<4) | S`.

**Format VII — control, no operand** (`oooooooooooooooo`): `IDLE >0340`,
`RSET >0360`, `RTWP >0380`, `CKON >03A0`, `CKOF >03C0`, `LREX >03E0`. (`CKON`/
`CKOF`/`LREX` are no-ops on this console — `cpu.rs`.)

**Format VIII — immediate / internal-register.**
- With register + immediate word: `LI >0200`, `AI >0220`, `ANDI >0240`,
  `ORI >0260`, `CI >0280`. Encoded `base | Reg`, then the immediate word.
  Flags: `LI`/`ANDI`/`ORI`/`CI` L A E; `AI` L A E C O.
- Immediate only (no register field): `LWPI >02E0`, `LIMI >0300`, then the
  immediate word. (`LIMI` uses only the low 4 bits.)
- Internal-register store, no immediate: `STWP >02A0`, `STST >02C0`. `base | Reg`.

**Format IX — `MPY`/`DIV` and `XOP`** (`oooooo D Ts S`): `MPY >3800` (unsigned
16×16→32 into `Rd:Rd+1`), `DIV >3C00` (sets O on overflow), `XOP >2C00`
(`XOP src,n`: `base | (n<<6) | (Ts<<4) | S`; sets ST's X bit).

**Pseudo-instructions** (assembler conveniences that emit a real opcode):

| Pseudo | Emits | Encoding |
|--------|-------|----------|
| `NOP` | `JMP $+2` | `>1000` |
| `RT`  | `B *R11` | `>045B` |

Machine instructions also **force a word boundary** (E/A behavior): if the LC is
odd, `libre99asm` advances it to even with a `>00` pad before the opcode (E/A manual
§14.1.6) — the CPU fetches words from even addresses (`machine.rs` masks
`addr & 0xFFFE`), so this both matches E/A and keeps the code executable. It also
emits an info diagnostic, since a misaligned instruction is almost always a missing
`EVEN` after an odd-length `BYTE`/`TEXT`.

### 5.7 Forward references and the location counter

- `$` is the address of the current statement (before it is emitted). `JMP $`
  is an infinite self-loop; `JMP $+2` is `NOP`.
- Forward references are permitted in instruction operands and **`DATA`**
  expressions; they are resolved in pass 2 (§9.4).
- **`BYTE` does *not* permit forward references.** Its operands must be
  well-defined (already-defined and absolute) at the point of use. This matches
  E/A, which specifies `BYTE <wd expr>` — a *well-defined* expression — while
  `DATA` takes an ordinary `<expr>` (E/A manual appendix 24.8). The rationale: a
  byte cannot hold a 16-bit relocatable address, so the value must be resolvable in
  pass 1 anyway, and keeping the same rule means a `BYTE` that assembles here
  assembles identically in E/A (and one E/A rejects is rejected here) — preserving
  round-trip portability. A forward-referenced `BYTE` is therefore an **error**,
  not a silent pass-2 fixup.
- `EQU` and the layout directives may not forward-reference either (their operands
  must be already-defined; `EQU` may be relocatable, the layout directives must be
  absolute — §5.4).

### 5.8 Constants and data emission

See the directive reference (§6) for `BYTE`, `DATA`, `TEXT`, `BSS`, `BES`, `EVEN`.

### 5.9 Reserved words

Mnemonics, directives, the predefined registers, and `$` are reserved and cannot
be redefined (except the register no-op of §5.2).

### 5.10 Relationship to the TI Editor/Assembler

**Supported, identical to E/A:** source-line format; `*`-comment; symbols;
`>`/decimal/character constants; left-to-right expressions with `+ - * /` and `$`;
all addressing modes; the full mnemonic set; the core directives `AORG RORG DORG
BSS BES BYTE DATA TEXT EQU DEF REF DXOP END EVEN IDT COPY TITL PAGE LIST UNL`;
absolute/relocatable algebra; the tagged-object output format.

> **`OPTION` is deliberately not provided.** It is *not* an Editor/Assembler
> directive — the E/A's only listing controls are `LIST`/`UNL`/`PAGE`/`TITL`/`IDT`
> plus the selection-time R/C/L/S options. `OPTION` (with operands like
> `XREF`/`SYMT`) comes from TI's mainframe/PC cross-assemblers and the Macro
> Assembler, so accepting it would be a *non-E/A extension*, not a compatibility
> feature, and risks implying E/A source portability it does not have. Listing
> extras such as a cross-reference are exposed as `libre99asm` CLI flags (`--xref`)
> instead, kept clearly outside the E/A directive set.

**`libre99asm` extensions (clearly non-E/A; gated off by `--ea-strict`):**
predefined `R0–R15`; full-length significant symbols; case-sensitive symbols and
`_` as a symbol character (E/A is uppercase `A–Z`/`0–9`, first char alphabetic);
`:`-binary and `LABEL:`/`;`-comment niceties; `INCLUDE` (alias of `COPY`); the
assisted cartridge-header directives (§6.4); the `.ctg`/JSON-map outputs; optional
`--ext-expr` precedence/grouping.

**Recognized E/A directives, diagnosed but not implemented in v1:** the
relocatable-segment directives `PSEG`/`PEND`/`CSEG`/`CEND`/`DSEG`/`DEND` and the
custom-loader linkage directives `LOAD`/`SREF`. These **are** genuine
Editor/Assembler directives (E/A manual §14.1.7–14.1.12, §14.4.4–14.4.5) — *not*
Macro Assembler features — but they only matter for multi-segment relocatable
modules consumed by a linking loader, which a single absolute cartridge image does
not use. `libre99asm` parses them and rejects each with a clear "recognized E/A
directive, not supported in v1" diagnostic (never silently ignored), so E/A source
that uses them fails loudly rather than miscompiling. They are candidates for the
future relocatable / disk-loaded path (§2).

**Not supported in v1 (out of scope, §2):** macros; a relocating linker beyond
single-module object output; GPL.

---

## 6. Directive reference

Syntax shown as `[label]` optional label, `expr` well-defined absolute unless
noted. All directives are case-insensitive.

### 6.1 Location and layout

- **`AORG expr`** — *absolute origin*. Set the location counter to the absolute
  address `expr` and switch to an absolute section. For a cartridge this is
  `AORG >6000`. If a cartridge target has **no** `AORG`, `libre99asm` inserts
  `AORG >6000` automatically (§6.4).
- **`RORG [expr]`** — *relocatable origin*. Switch to a relocatable section; set
  the relocatable location counter to `expr` (default: continue). Used for
  tagged-object output, not cartridges.
- **`DORG expr`** — *dummy origin*. Define subsequent labels relative to `expr`
  **without emitting bytes** (for overlaying a record/structure template on
  memory). Ends at the next `AORG`/`RORG`.
- **`[label] BSS expr`** — reserve `expr` bytes, uninitialized; `label` = the
  **start** (current LC); LC += expr.
- **`[label] BES expr`** — reserve `expr` bytes; LC += expr; `label` = the **end**
  (the new LC).
- **`EVEN`** — if the LC is odd, advance it by one (word-align). Emits a `>00`
  pad byte in an initialized section.

### 6.2 Data

- **`[label] BYTE e1[,e2,…]`** — emit one byte per expression (each 0–255 or
  −128…127; range-checked). Operands must be **well-defined** (absolute, no forward
  reference), matching E/A's `BYTE <wd expr>` (§5.7).
- **`[label] DATA e1[,e2,…]`** — emit one **word** (big-endian) per expression.
  Operands may be relocatable/forward. **Forces a word boundary** (E/A behavior):
  if the LC is odd, advance it to even and emit a `>00` pad byte *first*, then the
  word (E/A manual §14.3.3, appendix 24.8). `libre99asm` matches this so the bytes are
  identical to E/A, and additionally emits an info diagnostic so an odd-length
  `BYTE`/`TEXT` predecessor is noticed (App. G). (`BYTE` and `TEXT` do *not* pad.)
- **`[label] TEXT 'string'`** — emit the ASCII bytes of the string. A leading `-`
  (`TEXT -'string'`) negates the **last** byte (E/A idiom for length-prefixed/
  terminated strings); supported.
- **`[label] EQU expr`** — define `label` equal to `expr` (absolute or
  relocatable; must be well-defined at this point). The one directive whose label
  is mandatory.

### 6.3 Symbols, linkage, listing, and control

- **`DEF sym[,sym…]`** — mark symbols as **externally defined** (exported) in the
  object file.
- **`REF sym[,sym…]`** — declare symbols as **external references** (imported);
  unresolved at assembly time, fixed up by a loader/linker. (In a self-contained
  cartridge you will not use `REF`.)
- **`DXOP name,n`** — define `name` as a mnemonic for `XOP src,n` (so `name @X`
  assembles `XOP @X,n`).
- **`IDT 'name'`** — module identifier (≤8 chars). Goes in object tag `0`; also the
  default cartridge title if `--name` is not given.
- **`END [sym]`** — end of source. Optional `sym` is the **entry point**; for a
  cartridge this is the address the menu entry points at (default if the assisted
  header is used — §6.4).
- **`TITL 'string'`**, **`PAGE`**, **`LIST`**, **`UNL`** — listing control (title,
  page eject, enable/suppress listing). They affect the listing only, never the
  object. (`OPTION` is intentionally **not** a directive — see §5.10; a
  cross-reference is the `--xref` CLI flag.)
- **`COPY 'path'`** — textually include another source file. As shipped the
  filename is **single-quoted** (`COPY 'lib/ti99.inc'`), matching E/A's string
  syntax; paths are resolved relative to the including file, nesting is
  allowed, and cycles are detected and reported. *(The original spec called
  for double quotes plus `-I` search paths and an `INCLUDE` alias — not what
  shipped.)*

### 6.4 Cartridge header directives (extension)

To make the common case one line, `libre99asm` provides assisted header generation,
on by default for the `ctg`/`bin` cartridge targets:

- **Automatic mode (default).** If, in a cartridge target, the source does **not**
  itself place `>AA` at `>6000`, the assembler:
  1. ensures an `AORG >6000` (inserting it if absent),
  2. emits the standard header (§4.3) at `>6000` with one program-list entry,
  3. uses the menu **name** from `--name`, else `IDT`, else the input file stem,
  4. uses the **entry address** from `END`'s operand, else a symbol named
     `START`, else the first emitted instruction,
  5. lays your code immediately after the header.
- **`CART 'name'[,entry]`** *(explicit assisted header)* — emit the header at the
  current location (must be `>6000`) with a single entry `name`→`entry`. Use this
  when you want the header in a specific spot or a custom name without `--name`.
- **`MENU 'name',entry`** — add **another** program-list entry (chained), for a
  multi-program cartridge. Order on the console menu follows console rules.
- **`BANK expr`** *(future — not implemented; see §4.4)* — begin emitting into
  bank `expr` (0-based; selects which 8 KiB bank subsequent code/data land in).
  Implies the image is multi-bank (power-of-two rounded; §4.4).

Fully-manual headers (writing the `BYTE`/`DATA`/`TEXT` of §4.3 yourself, with the
automatic mode suppressed by the presence of `>AA` at `>6000`, or with
`--no-auto-header`) remain available and are E/A-portable.

---

## 7. Output formats

Selected by `--format` or inferred from the `-o` extension.

### 7.1 `.ctg` cartridge (default)

A byte-exact `ti99sim` V1 image as specified in §4.5 — the inverse of
`cartridge.rs::parse`. The implementation should live in `libre99-core` as a
`cartridge::write_v1(title, cru_base, &rom_banks, &grom_pages) -> Vec<u8>` so it
shares constants with the parser and is covered by a `parse(write(x)) == x`
round-trip test. Banner title = `--name`/`IDT`/file stem; CRU base `>0000`; ROM
banks split into page-6/page-7 region records; each 4 KiB page RLE-encoded
(literal-run or compressed). GROM region records are emitted only if data-in-GROM
is requested (future, §9.10).

### 7.2 Raw binary (`.bin`)

The flat `>6000` image: bank 0 low (4 KiB) + bank 0 high (4 KiB), then each
further bank, concatenated. Suitable for EPROM burning or other emulators/tools.
No header beyond what your source/auto-header produced. Size is `banks × 8 KiB`.
Libre99 mounts this raw form directly (`--cartridge out.bin`, or `F9`) — the
emulator's cartridge loader detects a raw ROM dump by its leading `>AA` header,
so a bootable `.bin` needs no repackaging into `.ctg`.

### 7.3 TI tagged-object file (`.obj`) — E/A interchange *(future — not implemented)*

The standard, human-readable TI object format (the E/A "Assemble" output), for
loading into the real Editor/Assembler or other TI tools. A stream of tagged
fields packed into **fixed-80** records — up to 71 characters of tagged data per
record, then the `7`-checksum and `F`, blank-filled to 80 columns with a trailing
sequence number. Tag characters:

| Tag | Field | Meaning |
|-----|-------|---------|
| `0` | 4 hex len + 8 ASCII | Program length (relocatable size) + module IDT; first field of the file |
| `1` | 4 hex | Entry point, absolute |
| `2` | 4 hex | Entry point, relocatable |
| `3` | 4 hex + 6 ASCII | `REF` (external ref), relocatable — chained link field + symbol |
| `4` | 4 hex + 6 ASCII | `REF`, absolute |
| `5` | 4 hex + 6 ASCII | `DEF` (external def), relocatable — value + symbol |
| `6` | 4 hex + 6 ASCII | `DEF`, absolute |
| `7` | 4 hex | Record checksum — **two's complement** of the summed 8-bit character values, from the record's first tag through the `7` |
| `8` | 4 hex | Checksum-ignore marker (value not verified) |
| `9` | 4 hex | Set load address, absolute |
| `A` | 4 hex | Set load address, relocatable |
| `B` | 4 hex | Data word, absolute (emit, advance 2) |
| `C` | 4 hex | Data word, relocatable (emit relocated, advance 2) |
| `F` | — | End of record (remainder of the 80-col line is padding/sequence) |
| `:` | text | End of file: `:` in column 1 of the final record, then the assembler-ID tail (E/A writes `9914 AS…`) |

Each record holds up to 71 characters of tagged data, then ends with
`7<checksum>F`, blank-filled to a fixed 80 columns with a trailing sequence number;
the file's first field is `0`. The checksum is the **two's complement** of the
summed 8-bit character values from the record's first tag through the `7` (a tag
`8` in its place tells the loader to skip verification). A
**compressed** variant stores the 4-hex fields as two raw bytes (halving size);
the loader distinguishes it by the file's leading bytes. v1 emits the
**uncompressed** form (simplest, most portable); the compressed form is an
optional later addition. For a cartridge built `AORG >6000`, everything is
absolute (tags `9`/`B`), so the object form is mainly useful for relocatable
modules and cross-tool exchange.

> Exact byte-for-byte conformance with a specific E/A version should be validated
> against reference `.obj` output during implementation (§9.10 lists this as an
> open validation item); the table above is the documented standard format.

### 7.4 Listing (`.lst`)

Per source line: location counter, the emitted object words/bytes, and the
original source text; plus a **symbol table** (name, value, relocation type,
defined/referenced) and, with `--xref`, a cross-reference. Deterministic and
diff-friendly.

### 7.5 Map (`.map.json`) — machine-readable

A JSON document for tooling and agents: tool version; input files; entry point;
sections (name, base, size); every symbol (name, value, type, source location);
banks; and the output artifact paths/sizes. Stable key order. This lets an agent
locate a routine's address, confirm a build's entry point, or diff two builds
without parsing the human listing.

---

## 8. Command-line interface and diagnostics

### 8.1 CLI (as shipped)

```text
libre99asm [OPTIONS] <input.asm>     assemble a cartridge (the default mode)
libre99asm rom <out.bin>             build the rewritten console ROM (8 KiB absolute image)
libre99asm dis <file.bin> [addr]     disassemble from an address (default >0000)

OPTIONS (cartridge mode):
  -o, --output <file>   Output path (default: input with .ctg/.bin extension)
      --format <fmt>    ctg | bin           (default: ctg)
      --bin             Shorthand for --format bin (raw >6000 image)
      --name <title>    Cartridge menu title (default: IDT, then "CART")
      --entry <symbol>  Entry-point symbol (default: END operand, then START)
      --listing <file>  Also write an address/object/source listing
      --symbols <file>  Also write the symbol table as JSON
  -h, --help            Print help and exit
```

The `rom` subcommand is the clean-room console-ROM build
(`original-content/system-roms/rom/`); `dis` is the table-driven disassembler
(same ISA table as the assembler, §11 Appendix A). *(Future, from the original
spec, not implemented: `--format obj` (§7.3), `--base`, `--banks` (§4.4),
`-I`/`--include` search paths, `--define`, `--no-auto-header`, `--ea-strict`,
`--warnings-as-errors`, `--json-errors`, `--quiet`/`--verbose`, `-V`.)*

### 8.2 Exit codes

`0` success; `1` assembly errors (diagnostics emitted); `2` usage/CLI error; `3`
I/O error (input/include/output). Warnings alone do not change a `0` exit unless
`--warnings-as-errors`.

### 8.3 Diagnostics (NFR-2)

As shipped, diagnostics are **1-based line-numbered, actionable messages**
(`file.asm:42: jump target out of range …`), the assembler reports as many
errors per run as it safely can, and exit codes follow §8.2. The richer format
below — column/caret rendering, stable `E###` codes, `--json-errors` — is the
original spec's target surface and remains *(future)*:

- Human form: `path:line:col: error[E###]: message`, followed by the source line
  and a caret under the offending column, and (where useful) a `note:`/`help:`
  line with a fix. Example:

  ```text
  hello.asm:42:14: error[E021]: jump target out of range
     42 |        JEQ  FARLABEL
        |             ^^^^^^^^ target is 612 bytes away; JEQ reaches -254..+256
     help: use `B @FARLABEL` for an unconditional far jump, or invert the
           condition and jump over a `B`.
  ```

- JSON form (`--json-errors`): one object per diagnostic with `severity`, `code`,
  `message`, `file`, `line`, `col`, `len`, and optional `help`. Stable, parseable
  by agents.
- Stable **error codes** (`E001`…): undefined symbol, redefined symbol, value out
  of range, bad addressing mode, odd-address instruction (warning), relocation
  error, include cycle, bank overflow, etc. Codes are documented and never
  renumbered.
- The assembler **continues after errors** where it safely can (so one run reports
  many problems), then exits non-zero.

---

## 9. Implementation plan *(historical — executed)*

> This section is the original build plan, preserved as design rationale. The
> shipped crate's actual module layout is smaller than proposed here
> (`lex`/`expr`/`front`/`isa`/`lib`/`disasm`/`system_rom`/`main` — see
> [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)); the milestones in §9.9 are
> long done; and §9.8's `--cartridge-file` flag shipped in `libre99-app`.

### 9.1 Crate layout

A new workspace member `crates/libre99-asm` (add to `members` in the root
`Cargo.toml`). Pure `std`; the only dependency is `libre99-core` (for the shared
`.ctg` writer and for round-trip/round-tested integration). It builds a library
(`libre99_asm`) and a binary (`libre99asm`):

```text
crates/libre99-asm/
  Cargo.toml            # [dependencies] libre99-core = { path = "../libre99-core" }
  src/
    lib.rs              # public API: assemble(source, options) -> Result<Artifacts, Diagnostics>
    diag.rs             # Diagnostic, Severity, Span, source map, JSON emit
    source.rs           # SourceSet: load file + COPY/INCLUDE, line/col mapping
    lex.rs              # line → fields → tokens (label/mnemonic/operands/comment)
    expr.rs             # expression parse + evaluate (value + relocation type)
    symbol.rs           # SymbolTable: name -> Symbol{ value, reloc, defined, refs }
    operand.rs          # addressing-mode parse -> (Ts, reg, Option<extension expr>)
    isa.rs              # the instruction table (single source of truth) + encoders
    directive.rs        # directive handlers
    assemble.rs         # two-pass driver, location counter, sections, fixups
    object.rs           # TI tagged-object writer (and a reader, for tests)
    cartridge.rs        # header synthesis + bank layout; calls libre99_core .ctg writer
    listing.rs          # .lst and .map.json emitters
    cli.rs              # argument parsing
    main.rs             # binary entry; wires cli -> lib -> outputs, sets exit code
  tests/
    encode.rs           # golden encodings for every mnemonic/mode (vs Appendix A)
    exec.rs             # assemble + run on libre99_core::Machine (behavioral)
    cartridge.rs        # assemble -> .ctg -> mount -> boot -> assert on screen
    directives.rs, expr.rs, errors.rs, object.rs
```

Add to `libre99-core`: `cartridge::write_v1(...)` (the inverse of `parse`) plus a
`parse(write_v1(x)) == x` test. This keeps the container format defined in one
place.

### 9.2 Core data structures

```rust
enum Reloc { Absolute, Relocatable }          // extend later for multi-section
struct Value { bits: u16, reloc: Reloc }

struct Symbol {
    value: Option<Value>,                     // None until defined (forward ref)
    defined_at: Option<Span>,
    exported: bool, imported: bool,           // DEF / REF
    uses: Vec<Span>,                          // for XREF and "unused" hints
}

enum Mode { Reg(u8), Indirect(u8), AutoInc(u8), Symbolic(Expr), Indexed(Expr,u8) }

enum Stmt {                                    // one assembled statement
    Insn { op: &'static InsnDef, operands: Vec<Operand>, span: Span },
    Directive { kind: DirKind, args: Vec<Expr>, span: Span },
    Label(SymbolId), Empty,
}

struct InsnDef { mnemonic: &'static str, base: u16, fmt: Fmt, flags: Flags }

struct Section { reloc: Reloc, lc: u16, image: Vec<u8>, emitted: Vec<bool> }
struct Fixup { at: u16, expr: Expr, kind: FixupKind, span: Span } // resolved pass 2
```

`isa.rs` holds a `&'static [InsnDef]` table — the single source of truth, ordered
and commented to mirror `cpu.rs`'s decode arms so the two can be diffed by a
human or a test.

### 9.3 Pipeline

```text
CLI/options ─▶ SourceSet (COPY/INCLUDE expansion, source map)
            ─▶ lex each line ─▶ parse to Stmt (operands kept as Expr until pass 2)
            ─▶ PASS 1: walk Stmts, advance LC, define labels, size each Stmt,
                       evaluate well-defined directive operands, pick sections
            ─▶ PASS 2: evaluate operand Exprs, encode, write bytes, record Fixups
                       and DEF/REF, build listing
            ─▶ locate/relocate (cartridge: absolute; object: emit reloc tags)
            ─▶ header synthesis + bank layout
            ─▶ emit artifacts (.ctg / .bin / .obj / .lst / .map.json)
```

### 9.4 The two passes

- **Pass 1 (layout & symbols).** Maintain the location counter per active section.
  For each statement: if it has a label, define that symbol = current LC (with the
  section's reloc type); compute the statement's byte size (instruction = 2 +
  2×(number of symbolic/indexed operands); `BYTE`/`DATA`/`TEXT`/`BSS`/`BES`/`EVEN`
  from their operands); advance the LC. Evaluate operands of **layout directives**
  now (`AORG`/`RORG`/`DORG`/`BSS`/`BES`/`EQU`/`EVEN` repeat) — they must be
  well-defined; an undefined symbol here is an error (matches E/A). Forward
  references in *instruction* operands are fine — only the size is needed in pass
  1, and instruction size does not depend on operand *values* (symbolic-mode
  operands always take one extension word regardless of value).
- **Pass 2 (encode).** Re-walk; every symbol is now known. Evaluate each operand
  expression to a `Value`; check ranges and relocation legality; encode the
  instruction word + extension words via `isa.rs`; write into the section image
  and mark bytes emitted. Record a `Fixup` for any value needing relocation in the
  object output. Emit listing rows. Report all errors with spans.
- After pass 2: for a **cartridge** the section base is fixed (`>6000`), so values
  are final; verify nothing was emitted outside `>6000–7FFF` (per bank). For
  **object** output, convert absolute/relocatable values into the appropriate
  tags + reloc records.

### 9.5 Expression evaluation & relocation

`expr.rs` parses the E/A grammar (left-to-right, `+ - * /`, unary `± `, constants,
symbols, `$`) into an `Expr` tree, and evaluates it against the symbol table to a
`Value`. The relocation type propagates per the table in §5.4; illegal
combinations (e.g. `R*A`, `R+R`) raise a relocation error with the operand span.
`$` resolves to the LC at the statement being assembled.

### 9.6 Instruction encoding

Driven entirely by `isa.rs`. For each `Fmt`, a small encoder places the operand
fields:

- Resolve each general operand to `(Ts, S)` and an optional extension word
  (symbolic/indexed). Source operand's extension precedes the destination's
  (the order `cpu.rs` consumes them — §5.5).
- Validate per-format constraints: shift/CRU/XOP counts `0–15`; jump displacement
  range and evenness; immediate width; `@expr(R0)` rejected; byte values fit.
- Assemble the base word `| field<<shift | …` and append extension/immediate
  words big-endian.

A **golden test** (`tests/encode.rs`) asserts the encoding of at least one example
per mnemonic and per addressing mode against Appendix A, and a **differential
test** assembles a word and feeds it back through a tiny decoder check (or runs it
on `Machine` and observes the architected effect) so the encoder cannot silently
disagree with `cpu.rs`.

### 9.7 Header synthesis & cartridge packaging

`cartridge.rs` (asm side): after assembly, if auto-header is active, lay out the
16-byte header + the program-list entries (from `CART`/`MENU`/auto) at `>6000`,
then the user image after it. Resolve the entry symbol(s). Split the
`>6000–7FFF`(×banks) image into page-6/page-7 region records and call
`libre99_core::cartridge::write_v1` to produce the `.ctg`. Validate: header present
and well-formed; entry addresses inside the window; bank count power-of-two; image
fits `banks × 8 KiB`.

### 9.8 Emulator integration (closing the build-run loop)

To let an agent *run* what it built without rebuilding the app's embedded assets,
add to `libre99-app` a CLI option **`--cartridge-file <path>`** that reads a `.ctg`
from disk (`std::fs::read`), `Cartridge::parse`s it, and mounts it (mirroring the
existing embedded path in `app.rs`). Today, a `.ctg` dropped into `cartridges/`
is embedded on the next build (`build.rs` scans the dir) and can be mounted with
`--cartridge <name>`; the flag removes the rebuild from the loop. (**Shipped** —
see the [User Guide](../docs/USER-GUIDE.md#command-line-options).)

### 9.9 Milestones

Each milestone is independently testable and lands with tests green and clippy
clean (repo policy).

| # | Milestone | Done when |
|---|-----------|-----------|
| **A1** | Lexer + source model + diagnostics scaffold | lines tokenize into fields; `COPY`/`INCLUDE` resolve; spans correct; unit tests |
| **A2** | Expressions + symbol table | `expr.rs` evaluates all forms incl. `$`, reloc algebra, forward refs; unit tests |
| **A3** | Instruction encoder (all formats) | every mnemonic/mode encodes per Appendix A; golden + differential tests vs `cpu.rs` |
| **A4** | Core directives + two-pass driver → `.bin` | `AORG EQU BYTE DATA TEXT BSS BES EVEN END` produce a correct flat image; tests |
| **A5** | `.ctg` writer in core + header synthesis + **first ROM-header boot** | `cartridge::write_v1` round-trips; assemble the §10.1 demo; **`tests/cartridge.rs` mounts it, boots, and asserts the screen turns the expected color — the first end-to-end proof that the console scans `>6000` for `>AA` and launches a *ROM* program** (the bundled boot tests use GROM carts; §9.10) |
| **A6** | Remaining directives + tagged-object output | `DEF REF DXOP IDT TITL PAGE LIST UNL` + `.obj`; object reader round-trip test |
| **A7** | CLI, listing, JSON map, JSON errors | `libre99asm` usable end-to-end; deterministic outputs; exit codes; agent-readable diagnostics |
| **A8** | `libre99-app --cartridge-file` + docs | run a built cart from a path; update `README`/`STATUS`/`ROADMAP`; link this spec |
| **A9** *(opt)* | RLE compression, banking polish, macros, EA5 image, data-in-GROM | as scoped in §9.10 |

### 9.10 Test strategy, risks, and open questions

**Tests** (mapped to requirements):

- *Unit* — lexer, expressions (incl. reloc algebra and error cases), symbol
  table, each directive (FR-1/6/7).
- *Golden encodings* — a table of `(source, expected bytes)` covering every
  mnemonic and addressing mode (FR-2; Appendix A is the oracle). **Appendix G**
  seeds this suite with E/A-cited conformance rows (expressions, constants, text,
  alignment, operand-word order) that must hold byte-for-byte against the
  Editor/Assembler — the concrete guard on the "builds in both" promise.
- *Differential vs the CPU* — assemble small routines and **run them on
  `libre99_core::Machine`**, asserting architected effects (register/memory/flags).
  This is the strongest guarantee that the encoder matches what the emulator
  executes (NFR-4).
- *Cartridge round-trip* — assemble → `.ctg` → `Machine::mount_cartridge` →
  boot → inspect VRAM/registers, reusing the `screen_text` technique from
  `tests/cartridge.rs` (FR-3/4/11). The §10.1 demo is the first such test — and
  the first to exercise the `>6000` `>AA` ROM-header path at all: the bundled
  `tunnels_of_doom_appears_on_the_selection_screen` test mounts a **GROM**
  cartridge, which lists via its GROM header, *not* the cartridge-ROM scan this
  tool relies on. So A5 is where that assumption is first proven, not inherited.
- *Container round-trip* — `cartridge::parse(write_v1(x)) == x` and
  `write_v1`-then-decode equals the source image (FR-3).
- *Object round-trip* — write `.obj`, read it back, compare (FR-5).
- *Determinism* — assemble twice, byte-compare every artifact (FR-10).
- *Property/fuzz (opt)* — random valid programs encode→decode consistently.

**Risks & open questions**

1. **Post-menu machine state.** When the console launches a ROM program, the
   exact VDP register / VRAM / scratchpad state is not contractually defined. The
   robust cookbook programs therefore **fully initialize the VDP** themselves
   (§10). The A5 round-trip test settles the actual state empirically for this
   emulator and is the guard against regressions.
2. **Exact E/A object-format conformance.** The tag table (§7.3) is the documented
   standard, but a specific E/A build's spacing/compressed form should be
   diffed against a reference `.obj` if byte-exact interchange is required. Tracked
   as an A6/A9 validation item; does not affect the cartridge path.
3. **Banking constraints.** The emulator's bank mask requires power-of-two banks
   and bank 0 holds the header; cross-bank code needs a shared trampoline. The
   tool enforces and warns (FR-8). Most agent-built carts are single-bank.
4. **Expression semantics surprise.** E/A's no-precedence, left-to-right rule
   surprises newcomers (and LLMs). Default to E/A semantics; `--warn-precedence`
   and the optional `--ext-expr` mode mitigate (§5.4).
5. **Scope creep.** Macros, linker, GPL, EA5 are explicitly deferred (§2/§9.10) to
   keep v1 shippable and correct.

---

## 10. Cookbook (for humans and agents)

Complete, self-contained programs. Each is small enough to paste, assemble with
`libre99asm demo.asm -o demo.ctg`, and run with
`cargo run -p libre99-app -- --cartridge-file demo.ctg` (after milestone A8) or by
dropping `demo.ctg` into `cartridges/` and rebuilding. `R0`–`R15` are predefined.

### 10.1 Smallest cartridge — a solid colored screen

Proves the whole pipeline: the menu lists the program, control reaches your code,
and you command the VDP. It blanks the display so the VDP shows the backdrop color
(`vdp.rs::render` fills the frame with the backdrop when display-enable R1 bit 6 is
0), which makes a trivial, unambiguous round-trip assertion.

```asm
        IDT  'COLOR'
        AORG >6000
* --- auto-header would do this for us; shown explicitly once for reference ---
        BYTE >AA,>01,>01,>00      ; valid, version, #programs, reserved
        DATA >0000                ; power-up list
        DATA MENU                 ; program list  -> entry below
        DATA >0000,>0000,>0000,>0000
MENU    DATA >0000                ; next = none
        DATA START                ; entry address
        BYTE 5
        TEXT 'COLOR'
        EVEN
* --- program ---
VDPWA   EQU  >8C02                ; VDP address/control port (byte writes only!)
START   LIMI 0                    ; interrupts off
        LWPI >8300                ; our workspace in fast scratchpad
* VDP R1 <- >80 : 16K on, DISPLAY OFF  (so the screen is pure backdrop)
        LI   R0,>8081             ; high byte = data >80 ; low byte = >81 (=>80|1)
        MOVB R0,@VDPWA            ; send data byte (high byte of R0)
        SWPB R0
        MOVB R0,@VDPWA            ; send (>80|reg) byte
* VDP R7 <- >04 : backdrop color = dark blue (palette index 4)
        LI   R0,>0487             ; data >04 ; ctrl >87 (=>80|7)
        MOVB R0,@VDPWA
        SWPB R0
        MOVB R0,@VDPWA
SPIN    JMP  SPIN                 ; done; loop forever
        END  START
```

With the assisted header (default), the same program is just:

```asm
        IDT  'COLOR'
VDPWA   EQU  >8C02
START   LIMI 0
        LWPI >8300
        LI   R0,>8081
        MOVB R0,@VDPWA
        SWPB R0
        MOVB R0,@VDPWA
        LI   R0,>0487
        MOVB R0,@VDPWA
        SWPB R0
        MOVB R0,@VDPWA
SPIN    JMP  SPIN
        END  START
```

`libre99asm` inserts `AORG >6000`, the `>AA` header, and a `START`→entry menu item
named from `IDT`/`--name`. **Round-trip test (A5):** mount, boot, press the menu
number, run a few frames, assert every framebuffer pixel equals `PALETTE[4]`.

### 10.2 Printing text (full VDP setup + a tiny font)

Self-contained Graphics-I text: set the VDP up from scratch, clear VRAM, load a
tiny font for just the glyphs used, and write a message. Robust regardless of the
machine state at entry. Complete and correct — copy, assemble, run.

The invariants an agent must preserve: **(1)** registers and VRAM addresses are
programmed through `>8C02` with **byte** writes, **data byte first, then the
control byte**; a register write's control byte is `>80 | reg`; a VRAM **write**
address's second control byte is `((addr>>8)&>3F) | >40` after the low byte.
**(2)** a `0` color-table entry renders as backdrop (`vdp.rs::render_graphics1`),
so set the color table (here `>1F` = black on white) or text is invisible.
**(3)** a name-table cell holds a *character code*; it displays pattern-table entry
`code*8`. We place glyphs at contiguous codes `>60..>63` and clear everything
else, so blanks are truly blank.

```asm
        IDT  'HELLO'
VDPWD   EQU  >8C00              ; VDP data write
VDPWA   EQU  >8C02              ; VDP address/control write  (byte writes only!)
START   LIMI 0
        LWPI >8300             ; our workspace in fast scratchpad
* ---- program VDP registers from (data,>80|reg) byte pairs ----
        LI   R1,REGTAB
        LI   R2,16             ; 8 registers x 2 bytes
RL      MOVB *R1+,@VDPWA
        DEC  R2
        JNE  RL
* ---- clear pattern table >0800..>0FFF (2048 bytes) so spaces are blank ----
        LI   R1,>0800
        BL   @SETWR
        LI   R2,2048
        CLR  R0
PCLR    MOVB R0,@VDPWD         ; MOVB writes R0's HIGH byte (>00)
        DEC  R2
        JNE  PCLR
* ---- fill color table >0300..>031F (32 bytes) with >1F (black on white) ----
        LI   R1,>0300
        BL   @SETWR
        LI   R2,32
        LI   R0,>1F00
CCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  CCLR
* ---- clear name table >0000..>02FF (768 bytes) to space code >20 ----
        LI   R1,>0000
        BL   @SETWR
        LI   R2,768
        LI   R0,>2000
NCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  NCLR
* ---- load 4 glyphs (32 bytes) into pattern slots >60..>63 ----
        LI   R1,>0B00          ; pattern slot >60  (>60*8 + >0800 = >0B00)
        BL   @SETWR
        LI   R2,FONT
        LI   R3,32
        BL   @VMBW
* ---- write "HELLO" at row 12, col 13  (name addr = 12*32+13 = >018D) ----
        LI   R1,>018D
        BL   @SETWR
        LI   R2,MSG
        LI   R3,5
        BL   @VMBW
SPIN    JMP  SPIN
* ============ subroutines ============
* SETWR: set the VRAM address in R1 for WRITING (clobbers nothing lasting)
SETWR   SWPB R1
        MOVB R1,@VDPWA         ; low address byte first
        SWPB R1
        ORI  R1,>4000         ; set bit 14 = "write" mode
        MOVB R1,@VDPWA         ; high address byte | >40
        ANDI R1,>3FFF         ; restore R1
        RT
* VMBW: copy R3 bytes from @R2++ to the (already-set) VRAM write address
VMBW    MOVB *R2+,@VDPWD
        DEC  R3
        JNE  VMBW
        RT
* ============ data ============
REGTAB  BYTE >00,>80           ; R0 = >00  Graphics I
        BYTE >C0,>81           ; R1 = >C0  16K + display ON, interrupts off
        BYTE >00,>82           ; R2 = >00  name table  >0000
        BYTE >0C,>83           ; R3 = >0C  color table >0300
        BYTE >01,>84           ; R4 = >01  pattern table >0800
        BYTE >00,>85           ; R5 = >00  (sprites unused)
        BYTE >00,>86           ; R6 = >00
        BYTE >17,>87           ; R7 = >17  border/backdrop = cyan
FONT    BYTE >88,>88,>88,>F8,>88,>88,>88,>00   ; >60  H
        BYTE >F8,>80,>80,>F0,>80,>80,>F8,>00   ; >61  E
        BYTE >80,>80,>80,>80,>80,>80,>F8,>00   ; >62  L
        BYTE >70,>88,>88,>88,>88,>88,>70,>00   ; >63  O
MSG     BYTE >60,>61,>62,>62,>63               ; H E L L O
        END  START
```

The assisted-header default applies here too: drop the explicit `AORG`/header and
`libre99asm` synthesizes them, naming the menu entry from `IDT`. This program is the
basis of the A5/A6 round-trip test (assemble → `.ctg` → boot → assert `"HELLO"`
appears in the name table via the `screen_text` helper).

### 10.3 Reading the keyboard (CRU)

The keyboard is the 8×8 CRU matrix (`cru.rs`, `keyboard.rs`): select a **column**
on CRU output bits 18–20 (with R12 = `>0024`), read the eight **rows** on input
bits 3–10 (with R12 = `>0006`); a pressed key reads **0** (active low). This
fragment sets the backdrop while the **`1`** key (column 5, row 4 — `keyboard.rs`)
is held:

```asm
POLL    LI   R12,>0024           ; CRU base for column select (bit 18)
        LI   R0,>0500            ; column 5 in the HIGH byte
        LDCR R0,3                ; drive 3-bit column select
        LI   R12,>0006           ; CRU base for row read (bit 3)
        STCR R1,8                ; read 8 rows into R1's high byte (1=up, 0=down)
        ANDI R1,>1000            ; isolate row 4 (high-byte bit 4 -> word bit 12)
        JEQ  KEYDN               ; ==0 -> '1' is pressed
        JMP  POLL
KEYDN   ...                      ; react (e.g. change R7), then loop
```

Row→bit mapping: `STCR Rx,8` stores rows LSB-first into the **high byte** of `Rx`,
so row *r* lands at word bit `8+r` (row 0 → `>0100` … row 7 → `>8000`). Columns
6–7 are the joystick ports on the same rows (`keyboard.rs`).

### 10.4 A two-bank cartridge (banking + a mirrored trampoline) *(future — needs §4.4 banking, not yet in `libre99asm`)*

Most carts are single-bank; reach for banking only past 8 KiB. The hazard that
makes it tricky: **switching banks changes the ROM you are currently executing**,
so the switch instruction *and the instruction fetched right after it* must live
at an **identical address in every bank**. The standard fix is a tiny
**trampoline** mirrored byte-for-byte in the low part of each bank; you cross into
another bank by writing its select address and letting the next fetch come from
the new bank's identical copy. This cart calls a subroutine that lives in bank 1
from `main` in bank 0, and returns.

```asm
        IDT  'BANKED'
* Two 8 KiB banks (16 KiB). Build:  libre99asm banked.asm --banks 2 -o banked.ctg
* Bank select (machine.rs): a CPU *write* in >6000-7FFF picks the bank via
*   cart_bank = (addr>>1) & (banks-1)   -> write >6000 = bank 0, >6002 = bank 1.
* Bank 0 is live at reset and holds the >AA header (auto-generated here).
VDPWA   EQU  >8C02
SEL0    EQU  >6000               ; write -> select bank 0
SEL1    EQU  >6002               ; write -> select bank 1
* ================= BANK 0 : header + main =================
        BANK 0
START   LIMI 0
        LWPI >8300
        BL   @FAR                ; call the routine that lives in bank 1
SPIN    JMP  SPIN                ; FAR returns here (we are back in bank 0)
* ---- trampoline: MUST be byte-identical in every bank (mirrored in BANK 1) ----
FAR     MOVB R0,@SEL1            ; bank 1 now visible; the NEXT fetch is from bank 1
        B    @SUB                ;   ...where this same `B @SUB` lives -> enter SUB
HOME    MOVB R0,@SEL0            ; bank 0 now visible; the NEXT fetch is from bank 0
        B    *R11                ;   ...where this same `B *R11` lives -> return
* ================= BANK 1 : the far routine =================
        BANK 1
        AORG FAR                 ; pin the mirror to the SAME address as bank 0
        MOVB R0,@SEL1            ; (identical bytes to bank 0's trampoline)
        B    @SUB
        MOVB R0,@SEL0
        B    *R11
SUB     LI   R0,>8081            ; R1 <- >80 : display OFF (screen = pure backdrop)
        MOVB R0,@VDPWA
        SWPB R0
        MOVB R0,@VDPWA
        LI   R0,>0687            ; R7 <- >06 : backdrop = dark red
        MOVB R0,@VDPWA
        SWPB R0
        MOVB R0,@VDPWA
        B    @HOME               ; switch back to bank 0 and RT to the caller
        END  START
```

Trace: `BL @FAR` sets R11 = `SPIN` and runs the trampoline *in bank 0*; `MOVB
R0,@SEL1` makes bank 1 visible, so the `B @SUB` at `FAR+4` is fetched from bank 1's
mirror and branches to `SUB`. `SUB` runs while bank 1 is selected, then `B @HOME`
re-enters the trampoline; `MOVB R0,@SEL0` restores bank 0 and the mirrored `B *R11`
returns to `SPIN`. Build with `--banks 2` (or let overflow past 8 KiB force it);
`libre99asm` warns if code crosses a bank boundary without a `BANK`/trampoline (§6.4,
FR-8). A natural later convenience is a *common section* the tool auto-replicates
into every bank; until then, `AORG`-pinning the stub (as above, or via a shared
`COPY` file) keeps the mirror in sync.

---

## 11. Appendices

### Appendix A — Opcode quick reference

Format key: I dual-operand · II jump/CRU-bit · III COC/CZC/XOR · IV LDCR/STCR ·
V shift · VI single-operand · VII control · VIII immediate · IX MPY/DIV/XOP.
Flags: L=L> A=A> E=EQ C=carry O=overflow P=parity X=XOP-bit. Base = the opcode
value before operand fields are OR-ed in. Cycles = base from `cpu.rs`.

> **The `Cyc` figures are a floor, not the real cost.** At run time the CPU adds
> the addressing-mode penalties (`*Rn` +4, `@A`/`@A(Rn)` +8, `*Rn+` +6 byte/+8
> word) *and* the bus wait-states for every access. Wait-states dominate cartridge
> code: each access to your ROM at `>6000–7FFF` — the opcode fetch, every extension
> word, and any operand in a slow region — costs **+4** (`machine.rs::wait_states`;
> only the console ROM and the `>8300` scratchpad are 0-wait). A `MOV @A,@B` run
> from ROM therefore costs far more than its base `14`. Treat the table as relative,
> not absolute, timing. (Note this also makes the `RT` pseudo cost **12**, not the
> `8` base of `B`: its `*R11` indirect mode is fixed and always adds 4.)

| Mnemonic | Base | Fmt | Operands | Flags | Cyc |
|----------|------|-----|----------|-------|-----|
| `A`    | >A000 | I | `gas,gad` | LAECO | 14 |
| `AB`   | >B000 | I | `gas,gad` | LAECOP | 14 |
| `ABS`  | >0740 | VI | `gad` | LAECO | 12/14 |
| `AI`   | >0220 | VIII | `Rn,iop` | LAECO | 14 |
| `ANDI` | >0240 | VIII | `Rn,iop` | LAE | 14 |
| `B`    | >0440 | VI | `gas` | — | 8 |
| `BL`   | >0680 | VI | `gas` | — | 12 |
| `BLWP` | >0400 | VI | `gas` | — | 26 |
| `C`    | >8000 | I | `gas,gad` | LAE | 14 |
| `CB`   | >9000 | I | `gas,gad` | LAEP | 14 |
| `CI`   | >0280 | VIII | `Rn,iop` | LAE | 14 |
| `CKOF` | >03C0 | VII | — | — | 12 |
| `CKON` | >03A0 | VII | — | — | 12 |
| `CLR`  | >04C0 | VI | `gad` | — | 10 |
| `COC`  | >2000 | III | `gas,Rn` | E | 14 |
| `CZC`  | >2400 | III | `gas,Rn` | E | 14 |
| `DEC`  | >0600 | VI | `gad` | LAECO | 10 |
| `DECT` | >0640 | VI | `gad` | LAECO | 10 |
| `DIV`  | >3C00 | IX | `gas,Rn` | O | 16/92 |
| `IDLE` | >0340 | VII | — | — | 12 |
| `INC`  | >0580 | VI | `gad` | LAECO | 10 |
| `INCT` | >05C0 | VI | `gad` | LAECO | 10 |
| `INV`  | >0540 | VI | `gad` | LAE | 10 |
| `JEQ`  | >1300 | II | `disp` | — | 8/10 |
| `JGT`  | >1500 | II | `disp` | — | 8/10 |
| `JH`   | >1B00 | II | `disp` | — | 8/10 |
| `JHE`  | >1400 | II | `disp` | — | 8/10 |
| `JL`   | >1A00 | II | `disp` | — | 8/10 |
| `JLE`  | >1200 | II | `disp` | — | 8/10 |
| `JLT`  | >1100 | II | `disp` | — | 8/10 |
| `JMP`  | >1000 | II | `disp` | — | 10 |
| `JNC`  | >1700 | II | `disp` | — | 8/10 |
| `JNE`  | >1600 | II | `disp` | — | 8/10 |
| `JNO`  | >1900 | II | `disp` | — | 8/10 |
| `JOC`  | >1800 | II | `disp` | — | 8/10 |
| `JOP`  | >1C00 | II | `disp` | — | 8/10 |
| `LDCR` | >3000 | IV | `gas,cnt` | LAEP | 20+ |
| `LI`   | >0200 | VIII | `Rn,iop` | LAE | 12 |
| `LIMI` | >0300 | VIII | `iop` | — | 16 |
| `LREX` | >03E0 | VII | — | — | 12 |
| `LWPI` | >02E0 | VIII | `iop` | — | 10 |
| `MOV`  | >C000 | I | `gas,gad` | LAE | 14 |
| `MOVB` | >D000 | I | `gas,gad` | LAEP | 14 |
| `MPY`  | >3800 | IX | `gas,Rn` | — | 52 |
| `NEG`  | >0500 | VI | `gad` | LAECO | 12 |
| `NOP`  | >1000 | (pseudo) | — | — | 10 |
| `ORI`  | >0260 | VIII | `Rn,iop` | LAE | 14 |
| `RSET` | >0360 | VII | — | — | 12 |
| `RT`   | >045B | (pseudo) | — | — | 12 |
| `RTWP` | >0380 | VII | — | restores ST | 14 |
| `S`    | >6000 | I | `gas,gad` | LAECO | 14 |
| `SB`   | >7000 | I | `gas,gad` | LAECOP | 14 |
| `SBO`  | >1D00 | II | `disp` | — | 12 |
| `SBZ`  | >1E00 | II | `disp` | — | 12 |
| `SETO` | >0700 | VI | `gad` | — | 10 |
| `SLA`  | >0A00 | V | `Rn,cnt` | LAECO | 12+2n |
| `SOC`  | >E000 | I | `gas,gad` | LAE | 14 |
| `SOCB` | >F000 | I | `gas,gad` | LAEP | 14 |
| `SRA`  | >0800 | V | `Rn,cnt` | LAEC | 12+2n |
| `SRC`  | >0B00 | V | `Rn,cnt` | LAEC | 12+2n |
| `SRL`  | >0900 | V | `Rn,cnt` | LAEC | 12+2n |
| `STCR` | >3400 | IV | `gad,cnt` | LAEP | 20+ |
| `STST` | >02C0 | VIII | `Rn` | — | 8 |
| `STWP` | >02A0 | VIII | `Rn` | — | 8 |
| `SWPB` | >06C0 | VI | `gad` | — | 10 |
| `SZC`  | >4000 | I | `gas,gad` | LAE | 14 |
| `SZCB` | >5000 | I | `gas,gad` | LAEP | 14 |
| `TB`   | >1F00 | II | `disp` | E | 12 |
| `X`    | >0480 | VI | `gas` | (of executed insn) | 8 |
| `XOP`  | >2C00 | IX | `gas,n` | X | 36 |
| `XOR`  | >2800 | III | `gas,Rn` | LAE | 14 |

`gas` = general source, `gad` = general destination (the modes of §5.5); `iop` =
immediate expression; `disp` = address (jump) or CRU offset; `cnt` = 0–15 count;
`Rn` = workspace register.

### Appendix B — Directive quick reference

| Directive | Form | Effect |
|-----------|------|--------|
| `AORG` | `AORG expr` | absolute origin |
| `RORG` | `RORG [expr]` | relocatable origin |
| `DORG` | `DORG expr` | dummy origin (no bytes) |
| `BSS`  | `[l] BSS expr` | reserve N bytes; label = start |
| `BES`  | `[l] BES expr` | reserve N bytes; label = end |
| `EVEN` | `EVEN` | word-align the LC |
| `BYTE` | `[l] BYTE e,…` | emit bytes |
| `DATA` | `[l] DATA e,…` | emit words |
| `TEXT` | `[l] TEXT 'str'` | emit ASCII (`-'str'` negates last byte) |
| `EQU`  | `l EQU expr` | define symbol |
| `DEF`  | `DEF s,…` | export symbols |
| `REF`  | `REF s,…` | import symbols |
| `DXOP` | `DXOP name,n` | define an XOP mnemonic |
| `IDT`  | `IDT 'name'` | module id / default title |
| `END`  | `END [sym]` | end; optional entry point |
| `TITL`/`PAGE`/`LIST`/`UNL` | — | listing control (`OPTION` is not E/A — §5.10) |
| `COPY` | `COPY 'path'` | textual include (single-quoted path) |
| `CART`/`MENU`/`BANK` | see §6.4 | cartridge header / banking (extension; `BANK` is *(future)*) |

### Appendix C — TI tagged-object tags *(future — see §7.3)*

See §7.3. Summary: `0` length+IDT · `1`/`2` entry abs/reloc · `3`/`4` REF
reloc/abs · `5`/`6` DEF reloc/abs · `7` checksum · `8` ignore-checksum · `9`/`A`
load-address abs/reloc · `B`/`C` data abs/reloc · `F` end-of-record · `:`
end-of-file.

### Appendix D — Memory & scratchpad equates (suggested `equ` library)

A `COPY`-able equate file the tool can ship (`ti99.inc`):

```asm
* Ports (drive VDP/GROM/sound with BYTE writes; see §4.1)
VDPRD   EQU  >8800     ; VDP VRAM read data
VDPSTA  EQU  >8802     ; VDP status read
VDPWD   EQU  >8C00     ; VDP VRAM write data
VDPWA   EQU  >8C02     ; VDP address / register write
GRMRD   EQU  >9800     ; GROM read data
GRMRA   EQU  >9802     ; GROM read address
GRMWD   EQU  >9C00     ; GROM write data
GRMWA   EQU  >9C02     ; GROM write address
SOUND   EQU  >8400     ; SN76489 (write only)
* CRU bases (load into R12) — see §10.3
KBDCOL  EQU  >0024     ; column-select base (CRU bits 18..20)
KBDROW  EQU  >0006     ; row-read base       (CRU bits 3..10)
* Scratchpad: use >8300..>83BF freely with interrupts off + own workspace.
* Avoid >83C0..>83FF (console/GPL interrupt area). Notables:
PAD     EQU  >8300     ; suggested workspace / variables
GPLSTA  EQU  >837C     ; GPL status byte (console)
INTWS   EQU  >83E0     ; console interrupt/GPL workspace (reset WP)
USRISR  EQU  >83C4     ; user ISR hook (console calls if interrupts enabled)
* Expansion RAM (always present in this emulator)
LORAM   EQU  >2000     ; >2000..>3FFF
HIRAM   EQU  >A000     ; >A000..>FFFF
* Cartridge ROM window
CART    EQU  >6000     ; >6000..>7FFF (8 KiB per bank)
```

### Appendix E — TMS9918A register cheat-sheet (`vdp.rs`)

| Reg | Meaning | Common value |
|-----|---------|--------------|
| R0 | mode bits (M3/EXT); bit1 = M3 | `>00` (Graphics I) |
| R1 | `16K BL IE M1 M2 · SZ MG` — bit7 16K, bit6 display-on, bit5 int-enable, bit4 M1, bit3 M2, bit1 sprite-size, bit0 magnify | `>C0` Graphics I display-on, ints off; `>80` display-off |
| R2 | name table base = `(R2&>0F)<<10` | `>00` → `>0000` |
| R3 | color table base = `R3<<6` | `>0C` → `>0300` |
| R4 | pattern base = `(R4&>07)<<11` | `>01` → `>0800` |
| R5 | sprite attr base = `(R5&>7F)<<7` | `>06` → `>0300` |
| R6 | sprite pattern base = `(R6&>07)<<11` | `>00` → `>0000` |
| R7 | text color (hi nibble) · backdrop (lo nibble) | `>17` white-on-cyan |

Palette indices (`vdp.rs::PALETTE`): 0 transparent, 1 black, 2 med-green,
3 lt-green, 4 dk-blue, 5 lt-blue, 6 dk-red, 7 cyan, 8 med-red, 9 lt-red,
10 dk-yellow, 11 lt-yellow, 12 dk-green, 13 magenta, 14 gray, 15 white. To set
register R to value V: `MOVB` V (data) then `>80|R` (control) to `>8C02`. To set a
VRAM **write** address A: `MOVB` `A&>FF` then `((A>>8)&>3F)|>40` to `>8C02`.

### Appendix F — Glossary

- **CRU** — Communication Register Unit, the 9900's bit-serial I/O bus
  (`SBO`/`SBZ`/`TB`/`LDCR`/`STCR`); base in R12 (`cru.rs`).
- **GROM/GPL** — TI's serial graphics ROM and the byte-code language in it
  (`grom.rs`); not a target for assembly cartridges (§2).
- **Scratchpad / pad** — the fast 256-byte RAM at `>8300–83FF` (`machine.rs`).
- **Workspace** — 16 consecutive words (R0–R15) at WP (`cpu.rs`).
- **`.ctg`** — the `ti99sim` cartridge container the emulator loads
  (`cartridge.rs`).
- **E/A** — the TI Editor/Assembler, the dialect this tool's input follows.
- **Relocatable** — a value that is *program-base + offset* (vs. absolute).

### Appendix G — E/A conformance cases (golden tests)

Ground-truth `source → emitted bytes` pairs that prove `libre99asm` matches the TI
Editor/Assembler, drawn from the E/A manual where it states a result. In the
implementation each row becomes an automated test in `libre99-asm` (the *golden
encodings* / *unit* suites of §9.10), so they run on **every `cargo test`** like
the existing `libre99-core` suites; a regression here means source would no longer
build identically in both assemblers. Bytes are big-endian, in assembly order.

**Expressions** — left-to-right, no precedence, unary-minus-first (manual §3.4.2):

| Source | Bytes | Note |
|--------|-------|------|
| `DATA 4+5*2` | `00 12` | = 18, not 14 — the manual's own example |
| `DATA 7+1/2` | `00 04` | = 4, not 7; division truncates (manual) |
| `DATA 2+3*4` | `00 14` | = 20 left-to-right |
| `DATA -5+2`  | `FF FD` | unary minus first → −3 |

**Constants** (manual §3.5):

| Source | Bytes | Note |
|--------|-------|------|
| `DATA >37AC` | `37 AC` | hex (manual: = 14252) |
| `DATA 1000`  | `03 E8` | decimal |
| `DATA -32768`| `80 00` | decimal low bound |
| `DATA 'AB'`  | `41 42` | 2-char constant (manual) |
| `BYTE 'C'`   | `43`    | 1-char constant (manual) |
| `BYTE ''''`  | `27`    | doubled quote = one `'` |
| `DATA ''`    | `00 00` | null char constant (manual) |

**Text** (manual §14.3.4):

| Source | Bytes | Note |
|--------|-------|------|
| `TEXT 'EXAMPLE'` | `45 58 41 4D 50 4C 45` | one ASCII byte per char |
| `TEXT -'AB'`     | `41 BE` | leading `-` negates the last byte (`42`→`BE`) |

**Alignment** — `DATA` and machine instructions force a word boundary; `BYTE`/`TEXT`
do not (manual §14.1.6, §14.3.3, appendix 24.8):

| Source | Bytes | Note |
|--------|-------|------|
| `BYTE 1` then `DATA >1234` | `01 00 12 34` | `DATA` pads `00` to even, then the word |
| `BYTE 1` then `BYTE 2`     | `01 02`       | `BYTE` does not pad |

**Instructions & operand-word order** (Appendix A; manual §15.1 for the `MOV`):

| Source | Bytes | Note |
|--------|-------|------|
| `MOV R1,R2`      | `C0 81`             | register → register |
| `MOV *R1+,R2`    | `C0 B1`             | auto-increment source |
| `MOV @INIT+3,@3` | `C8 20 01 28 00 03` | **source extension word precedes destination** (manual p.236; there `INIT+3` = `>0128`) |
| `LI R0,>1234`    | `02 00 12 34`       | immediate follows the opcode |
| `JMP $`          | `10 FF`             | self-loop, disp −1 |
| `JMP $+2`        | `10 00`             | = `NOP` |
| `NOP`            | `10 00`             | pseudo = `JMP $+2` (manual §13.1) |
| `RT`             | `04 5B`             | pseudo = `B *R11` (manual §13.2) |
| `SLA R1,4`       | `0A 41`             | shift count in the opcode |

> Illustrative, not exhaustive: the §9.10 *golden encodings* suite additionally
> covers **every** mnemonic and addressing mode against Appendix A, and the
> *differential* suite runs assembled code on `libre99_core::Machine` to prove the
> bytes do what `cpu.rs` executes.

---

*End of specification.*
