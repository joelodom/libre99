# Appendix B — GPL Instruction Reference

<!-- Appendices · target ≈14 pp · companion to Ch. 25–27 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The opcode catalog (B.4), the six operand formats (B.2), and the GAS operand encoding (B.3) are tier-1 for the project's toolchain: read directly from libre99gpl — crates/libre99-gpl/src/isa.rs (the NAMED/ONE_OPS/TWO_OPS/MoveBits tables and the 0x00-0xFF opcode-range map) and src/operand.rs (the short/12-bit/16-bit GAS forms, the >8300 bias), both execution-pinned against the authentic console ROM in the crate's examples/ and tests/. The GPL status-byte layout (B.5) is tier-1 for the book's machine: read from the clean-room firmware original-content/system-roms/rom/console.asm (H=>80, GT=>40, COND=>20, C=>10, OV=>08, RECON §16, oracle-pinned). Per-opcode status effects are cross-checked tier-3 against Classic99 addons/gpl.cpp (which embeds TI's per-opcode format specs). FMT (B.6) is presented structurally: libre99gpl is DECODE-ONLY for it (an R-12 gap — isa.rs; xas99/xga99 assemble it), so the byte-level sub-opcode table is left to the E/A-manual tier rather than fabricated (R-15/R-21). xga99 deltas (B.7) are tier-2/4, hedged (R-2). -->

GPL — Graphics Programming Language — is the byte-code the console itself is
written in: the second floor of the tower of interpreters (Ch. 25), a compact
instruction set executed by a 9900-language engine in console ROM, with its
program and data living in **GROM**. This appendix is the catalog: every opcode,
how its operands are laid out in the GROM byte stream, the general address format
that makes those operands self-describing, and the status byte the comparisons
feed. The teaching — why GPL exists, how the interpreter fetches and dispatches,
how you hand-assemble it — is Chapters 25–27; the `libre99gpl` tool that
assembles and disassembles everything here is Appendix L.

Two orientation facts first. GPL is **not** TMS9900 assembly: an opcode is one
byte (not a 16-bit word), addresses are 13-bit GROM slots, and the "registers"
are scratchpad cells, not `R0`–`R15`. And GPL is **big-endian and byte-stream**:
an instruction is an opcode byte followed by a variable number of operand bytes,
read in order, so the disassembler (and the interpreter) must know each opcode's
*shape* to know where the next instruction begins. B.2 is that shape table.

## B.1 The machine model

- **Program store:** GROM/GRAM, addressed 0–`>FFFF` but delivered through the
  GROM ports (`>9800`/`>9802`) one auto-incrementing byte at a time (Ch. 25).
  A GPL "address" in a branch is a **13-bit** slot value (`>0000`–`>1FFF`
  within a GROM), because a single GROM is 8 K (Ch. 25).
- **Data store:** CPU scratchpad (`>8300`–`>83FF`) and VDP RAM. GPL has no
  general-purpose registers; its operands name memory cells directly through
  GAS (B.3). The GPL workspace is `>83E0`; the data (value) stack is anchored
  in scratchpad (Appendix C).
- **Status:** a single byte at **`>837C`** (B.5), holding the comparison result
  and the condition bit that the conditional branches test.
- **Interpreter:** a 9900-language loop in console ROM (`>0070` onward in the
  project's clean-room firmware) that fetches an opcode, dispatches through a
  jump table, and runs a small 9900 routine per opcode. The project runs this
  interpreter as real firmware, so every encoding below is executed, not
  merely tabulated.

## B.2 The six operand formats

`libre99gpl` classifies every opcode into one of six formats — the shape that
tells the reader (and the disassembler) how many operand bytes follow and how to
read them. The names and byte-stream orders are the project assembler's, pinned
against the authentic ROM (`isa.rs`; the classic references number these
formats 1–6).

| Format | Used by | Byte stream after the opcode |
|---|---|---|
| **1** (two-operand) | `>A0`–`>EB` families (B.4.1) | **destination GAS, then source** — a GAS operand, or 1–2 immediate bytes when the opcode's `U` bit is set |
| **2** (immediate) | `RAND`, `BACK`, `ALL`, `PARSE`, `XML` (1 byte); `B`, `CALL` (a 16-bit GROM address) | the immediate operand |
| **3** (no operand) | `RTN`, `SCAN`, `H`, `GT`, … (B.4.3) | nothing |
| **4** (branch) | `BR`, `BS` | a 13-bit slot-absolute target packed into the opcode's low bits + one byte |
| **5** (single-operand) | `>80`–`>97` families (B.4.2) | one GAS operand |
| **6** (`MOVE`) | `>20`–`>3F` | count, destination, source — selected by the opcode's `G R V C N` bits (B.4.4) |

Within format 1, two opcode bits carry the shape: **`W`** (bit 0) picks the
**word** form over the byte form, and **`U`** (bit 1) makes the **source
immediate** instead of a GAS operand. So each family occupies four consecutive
opcodes — byte/GAS, word/GAS, byte/immediate, word/immediate — and the assembler
picks among them from the mnemonic's `D` prefix (word) and the source operand's
shape (B.4.1).

## B.3 GAS — the general address format

Every memory-referencing GPL operand is a **GAS** operand: a variable-length,
self-describing field of one to three bytes (plus an optional index byte). The
format byte's top bit chooses the shape (`operand.rs`, recovered by execution and
gated by the crate's round-trip tests):

```text
  0aaaaaaa                     direct CPU cell  >8300 + a           (1 byte)
  1 X V I nnnn , lo            12-bit address   (nnnn:lo)           (2 bytes)
  1 X V I 1111 , hi , lo       16-bit address                      (3 bytes)
      X = indexed   (one index byte — a CPU cell — appended LAST)
      V = VDP RAM   (else CPU RAM)
      I = indirect  (through the addressed word)
```

The address arithmetic is the single easiest thing to get wrong, so it is worth
stating flatly (all tier-1, execution-pinned in the crate's `m1`/`m4` probes):

- **CPU operands are biased by `>8300`.** The short form reaches `>8300`–`>837F`
  (7 bits); the 12-bit form `>8300`–`>92FF`; the 16-bit form is `>8300 + value`,
  wrapping. A scratchpad cell like `>83CE` therefore encodes as the *offset*
  `>0CE`, not `>00CE`.
- **VDP operands are used verbatim** (0–`>3FFF`) when **direct**. When
  **indirect** (`V` and `I` both set), the encoded field is the `>8300`-offset
  of the **scratchpad cell that holds** the VDP address — the operand names a
  pointer, not the target.
- The low-nibble value `1111` is the **escape** to the 16-bit form, so any
  12-bit address whose high nibble would be `>F` is promoted to three bytes.
- An **index** byte (a CPU cell number) is appended last when `X` is set; the
  effective address adds that cell's contents.

`libre99gpl` emits the shortest legal form (1-byte when a direct un-indexed CPU
cell fits 7 bits, else 12-bit, else 16-bit) and decodes all three; the assembler
rejects a non-memory operand where GAS is required (immediates and GROM addresses
are emitted directly, not as GAS).

## B.4 The opcode catalog

Opcodes are one byte. The whole 0–`>FF` space divides into the ranges below
(`isa.rs::decode_sig`, cross-checked against Classic99 `gpl.cpp`'s dispatch
table); the families follow.

| Range | Group | Format | Notes |
|---|---|---|---|
| `>00`–`>13` | named / control ops | 2/3/4 | B.4.3 (`>08` is FMT — B.6) |
| `>14`–`>1F` | XGPL (GPL extensions) | — | not emitted by libre99gpl (B.4.5) |
| `>20`–`>3F` | `MOVE` | 6 | B.4.4 |
| `>40`–`>5F` | `BR` (branch if reset) | 4 | B.4.3 |
| `>60`–`>7F` | `BS` (branch if set) | 4 | B.4.3 |
| `>80`–`>97` | single-operand families | 5 | B.4.2 |
| `>98`–`>9F` | XGPL | — | B.4.5 |
| `>A0`–`>EB` | two-operand families | 1 | B.4.1 |
| `>EC`–`>EF` | `COINC` (sprite coincidence) | — | not emitted (B.4.5) |
| `>F4`–`>F7` | `I/O` (CRU) | 1-like | not emitted (B.4.5) |
| `>F8`–`>FB` | `SWGR` (swap with GROM) | — | not emitted (B.4.5) |

### B.4.1 Two-operand families (format 1, `>A0`–`>EB`)

Each family is four consecutive opcodes: `base` = byte / GAS source; `base|1` =
word; `base|2` = byte / immediate source; `base|3` = word / immediate. In the
byte stream the **destination GAS comes first**, then the source. In assembler
source the **`D` prefix** selects the word form (`ST`/`DST`, `CEQ`/`DCEQ`, …);
the assembler chooses the immediate variant automatically when the source is an
immediate value.

| Mnemonic | `base` | Operation (dest ⇐ dest ∘ source) |
|---|---|---|
| `ADD` | `>A0` | add |
| `SUB` | `>A4` | subtract |
| `MUL` | `>A8` | multiply |
| `DIV` | `>AC` | divide |
| `AND` | `>B0` | logical AND |
| `OR` | `>B4` | logical OR |
| `XOR` | `>B8` | logical exclusive-OR |
| `ST` | `>BC` | store (dest ⇐ source) |
| `EX` | `>C0` | exchange dest ↔ source |
| `CH` | `>C4` | compare, sets **H** (logical) |
| `CHE` | `>C8` | compare high-or-equal |
| `CGT` | `>CC` | compare, sets **GT** (arithmetic) |
| `CGE` | `>D0` | compare greater-or-equal |
| `CEQ` | `>D4` | compare equal, sets **COND** |
| `CLOG` | `>D8` | compare logical (AND, set COND on any common bit) |
| `SRA` | `>DC` | shift right arithmetic |
| `SLL` | `>E0` | shift left logical |
| `SRL` | `>E4` | shift right logical |
| `SRC` | `>E8` | shift right circular |

So `DST` = `>BD`, `ST` immediate-byte = `>BE`, `DCEQ` = `>D5`, `CGT` with a
16-bit immediate = `>CF`. (`DIV`/`DEC`/`DECT` begin with `D` but are their own
stems — `DDIV` is the word form of `DIV`; the assembler resolves this exactly.)

### B.4.2 Single-operand families (format 5, `>80`–`>97`)

`base` = byte form, `base|1` = word form; one GAS operand follows.

| Mnemonic | `base` | Operation |
|---|---|---|
| `ABS` | `>80` | absolute value |
| `NEG` | `>82` | negate |
| `INV` | `>84` | ones-complement |
| `CLR` | `>86` | clear to zero |
| `FETCH` | `>88` | fetch a byte from GROM into the operand |
| `CASE` | `>8A` | computed branch: GROM PC += 2 × value |
| `PUSH` | `>8C` | push the operand onto the value stack |
| `CZ` | `>8E` | compare to zero, set **COND** on equal |
| `INC` | `>90` | increment (by 1) |
| `DEC` | `>92` | decrement (by 1) |
| `INCT` | `>94` | increment by two |
| `DECT` | `>96` | decrement by two |

`DCLR` = `>87`, `DEC` = `>92`, `DDEC` = `>93`, `DECT` = `>96`.

### B.4.3 Named and control ops (formats 2, 3, 4)

| Mnemonic | Opcode | Format | Operand / effect |
|---|---|---|---|
| `RTN` | `>00` | 3 | return from subroutine; **resets COND** |
| `RTNC` | `>01` | 3 | return, condition bit **not** touched |
| `RAND` | `>02` | 2 | random number → `>8378` (immediate = modulus) |
| `SCAN` | `>03` | 3 | keyboard/joystick scan (Appendix J); COND set on a new key |
| `BACK` | `>04` | 2 | fill/scroll helper (immediate) |
| `B` | `>05` | 2 | branch to a 16-bit absolute GROM address; resets COND |
| `CALL` | `>06` | 2 | call a 16-bit absolute GROM address (pushes return) |
| `ALL` | `>07` | 2 | fill the screen with the immediate character |
| `FMT` | `>08` | — | enter the format sub-interpreter (B.6) |
| `H` | `>09` | 3 | COND ⇐ **H** bit |
| `GT` | `>0A` | 3 | COND ⇐ **GT** bit |
| `EXIT` | `>0B` | 3 | software reset → master title screen |
| `CARRY` | `>0C` | 3 | COND ⇐ **C** bit |
| `OVF` | `>0D` | 3 | COND ⇐ **OV** bit |
| `PARSE` | `>0E` | 2 | BASIC number-parse helper (immediate) |
| `XML` | `>0F` | 2 | execute a machine-language routine via the XML table (immediate; Appendix K) |
| `CONT` | `>10` | 3 | return to the BASIC interpreter |
| `EXEC` | `>11` | 3 | begin executing a BASIC program |
| `RTNB` | `>12` | 3 | return to BASIC, address on the sub-stack |
| `RTGR` | `>13` | 3 | restore GROM PC from the sub-stack; resets COND |
| `BR` | `>40`–`>5F` | 4 | **branch if reset** (COND = 0); consumes COND |
| `BS` | `>60`–`>7F` | 4 | **branch if set** (COND = 1); consumes COND |

`BR`/`BS` pack the 13-bit target across the opcode byte's low five bits and one
following byte. Both **consume** (reset) the condition bit whether or not the
branch is taken (`console.asm`), so a compare-then-branch leaves COND clear for
the next comparison.

### B.4.4 `MOVE` (format 6, `>20`–`>3F`)

`MOVE` is GPL's block-transfer engine — the one instruction that copies a run of
bytes between GROM, CPU RAM, and VDP RAM, and the workhorse of every screen load
(Ch. 25). The opcode is `001 G R V C N`; the operand stream is **count,
destination, source** (`MoveBits`, `isa.rs`, execution-verified):

| Bit | Name | Meaning |
|---|---|---|
| `G` (`>10`) | not-GROM dest | set for every form the project emits; clear = write to GRAM |
| `R` (`>08`) | register dest | destination is a **VDP register number** (one raw byte) |
| `V` (`>04`) | RAM source | source is a GAS operand (CPU/VDP RAM); clear = GROM source |
| `C` (`>02`) | computed GROM | (when source is GROM) the GROM address comes from a CPU cell, not an immediate |
| `N` (`>01`) | immediate count | count is an immediate word; clear = count from a GAS operand |

The common forms: `>31` copies a GROM immediate block to RAM (the VRAM font/data
load); `>39` writes a VDP register; `>35` copies RAM→RAM with an immediate count;
`>34` copies RAM with the count read from memory. `MOVE` does not touch the
status byte.

### B.4.5 The higher opcodes (`>14`–`>1F`, `>98`–`>9F`, `>EC`–`>FF`)

The project assembler recognizes these ranges in the disassembler but **does not
emit** them (R-12 — `isa.rs`): `>14`–`>1F` and `>98`–`>9F` are **XGPL** (GPL
extension opcodes, used for card/DSR launch and never by ordinary carts);
`>EC`–`>EF` is **`COINC`** (test sprite coincidence — the project reads sprite
collision through the VDP status flag instead, Appendix D); `>F4`–`>F7` is **CRU
`I/O`**; `>F8`–`>FB` is **`SWGR`** (swap-with-GROM). Code that needs these is
shelf-tool territory (xga99); Chapters 25–27 do not use them.

## B.5 The GPL status byte (`>837C`)

GPL keeps its condition state in one scratchpad byte. The bit assignments are
tier-1 for the book's machine — read from the clean-room firmware, RECON-pinned
against the authentic ROM as oracle (`console.asm` §16):

| Bit | Mask | Name | Set by |
|---|---|---|---|
| 0 | `>80` | **H** — high (logical greater, unsigned) | a compare where dest > source unsigned (mirrors the 9900 `L>`) |
| 1 | `>40` | **GT** — greater (arithmetic, signed) | a compare where dest > source signed (`A>`) |
| 2 | `>20` | **COND** — the condition bit | equality (`CEQ`, `CZ`), `SCAN` on a new key, and the `H`/`GT`/`CARRY`/`OVF` copies; **this is the bit `BR`/`BS` branch on** |
| 3 | `>10` | **C** — carry | an add/shift that carried (`C`) |
| 4 | `>08` | **OV** — overflow | a signed overflow (`OV`) |
| 5 | `>04` | word-op marker | always set by a **word** operation (oracle-pinned) |

The pattern to internalize: a **comparison** (`CH`/`CGT`/`CEQ`/…) records its
full result in H/GT/COND at once; a **test op** (`H`, `GT`, `CARRY`, `OVF`)
copies one of those bits *into* COND so a following branch can act on it; and
`BR`/`BS` read COND and then clear it. So the idiom "compare, then branch on the
kind of inequality you care about" is: `CH` (or `CGT`) to set the bits, then `H`
(or `GT`) to fold the one you want into COND, then `BR`/`BS`. Equality skips the
fold — `CEQ` sets COND directly, so `CEQ`/`BS` branches when equal.

The console's floating-point and comparison routines copy the 9900's own status
into this byte verbatim (`>837C := L>|A>|EQ|C|OV`, plus `>04` for word ops), which
is why the GPL bits line up one-for-one with the 9900 flags they mirror (Ch. 8,
Appendix A).

## B.6 FMT — the format sub-language

`FMT` (opcode `>08`) switches the interpreter into a small, **self-contained
screen-formatting sub-language** and runs until its end marker. It is how the
title screens and menus paint text and repeated characters compactly: a stream
of sub-commands — emit a text literal, repeat a character horizontally or
vertically, step the cursor row/column, set a bias — rather than a `MOVE` per
field. Classic99's notes put it plainly: "the FMT interpreter is independent of
the GPL interpreter" (`gpl.cpp`; the authentic engine lives at ROM `>04DE`–`>05A1`).

Two honest limits apply here (R-12, R-21):

- **`libre99gpl` is decode-only for FMT.** The disassembler flags the `>08`
  opcode, but the assembler **rejects** it (`isa.rs`) — the project toolchain
  does not emit an FMT stream. Source that needs FMT is assembled on the shelf
  with **xga99** (Appendix L), which supports the full sub-language.
- The **byte-level sub-opcode table** (the exact encodings of the text / repeat /
  positioning commands and the end marker) is period-manual matter — it belongs
  to the tier-2 E/A and GPL documentation (Appendix N), and is deliberately not
  reproduced from memory here. When the project gains an FMT assembler this
  section gains its table, machine-verified; until then the reader who needs the
  encodings has xga99 and the manuals.

Chapters 25–27 build their screens with `ALL`, `MOVE`, and the character-set
loaders (Appendix J) rather than FMT, precisely so the companion code stays
inside the project toolchain.

## B.7 `libre99gpl` syntax and xga99 deltas

`libre99gpl` (Appendix L) is the project's GPL assembler/disassembler. Its source
syntax follows the classic TI GPL conventions this appendix uses: TI hex `>XXXX`,
the `D` prefix for word forms, `BYTE`/`DATA`/`TEXT`/`GROM` directives, and GAS
operands written with the `V@` (VDP) and `*` (indirect) sigils. Assemble and
disassemble with:

```
libre99gpl asm  source.gpl  out.bin      # assemble a GROM image
libre99gpl dis  grom.bin  [addr]         # disassemble
libre99gpl console out.bin               # assemble the clean-room console GROM
```

Deltas against **xga99** (the xdt99 GPL assembler — the widely-used shelf tool,
tier-2/4, hedged), for a reader moving code between them:

- **FMT:** xga99 assembles the full FMT sub-language; `libre99gpl` does not
  (B.6). This is the one difference that blocks source outright.
- **XGPL / `COINC` / `I/O` / `SWGR`:** xga99 emits the high-opcode ranges
  (B.4.5); `libre99gpl` decodes but does not emit them.
- **Directive spelling and macro facilities** differ in the usual small ways
  (label columns, `EQU`, conditional assembly); port directives, not just
  opcodes, when moving a file. The opcode mnemonics and GAS semantics in
  B.2–B.5 are common to both — those transfer unchanged.

For the reverse direction (reading someone else's GROM), `libre99gpl dis` and
xga99's disassembler agree on the mnemonics catalogued above; where a byte falls
in an un-emitted range, `libre99gpl` labels it (`XGPL`, `COINC`, …) rather than
inventing an operand.

*See also:* Chapter 25 (GROM and the GPL interpreter), Chapter 26 (hand-assembling
GPL), Chapter 27 (QUIZMASTER, the GPL capstone), Chapter 28 (§28.7, the clean-room
console GROM tour), Appendix A (the TMS9900 flags the GPL status byte mirrors),
Appendix C (the GPL workspace and value stack in scratchpad), Appendix J (SCAN and
the character sets), Appendix K (the XML table GPL's `XML` opcode dispatches
through), Appendix L (`libre99gpl` and xga99).
