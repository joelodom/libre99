# Appendix M — Glossary

<!-- Appendices · target ≈6 pp · drawn from every chapter · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. One book-wide vocabulary: the named handles (R-8 canon — the funnel, the fast island, the tower of interpreters, CQ-82, the high-byte law, "speed is a property of addresses," R10 = software stack, lib99) defined exactly as the canon card (_stubs.md §5) coins them, plus the machine, toolchain, and technique terms the chapters use. Definitions state the book's usage and point to the origin chapter; they do not re-derive the concept (the chapter does). Standing order (per _stub-steering.md): harvest new terms of art as later revisions add them. -->

One vocabulary for the whole book — the terms of art, vintage and modern, with a
pointer to the chapter that earns each. Cross-references in *italics* are other
glossary entries. Where the book coins a **named handle** and reuses it (R-8), the
definition is the canonical one; use the handle, don't re-coin it.

---

**AORG** — "absolute origin," the assembler directive that fixes code at a literal
address. A cartridge assembles `AORG >6000`; in `libre99asm`, `AORG` is
absolute-only (Ch. 3, Ch. 35).

**ARG** — the floating-point *argument* register, `>835C`, the second operand of
the console's radix-100 math (Ch. 23; App C).

**attenuation** — volume expressed as a cut from full output on the *PSG*: 4 bits,
0 loudest, 15 silent, ≈2 dB per step (Ch. 19; App E).

**bank switching** — paging one of several 8 K ROM banks into `>6000`–`>7FFF` so a
cartridge exceeds 8 K; on the TI, triggered by a write to a magic address (Ch. 35).

**BENCH99** — the book's scriptable monitor over the emulator core (`code/bench/`):
load, step, poke, and read back machine state headlessly — the instrument every
code chapter verifies against (Ch. 2–3; App L).

**BLWP / RTWP** — "branch and load workspace pointer" / "return": the 9900's
context switch. `BLWP` loads a new *workspace* and entry from a two-word vector and
saves the old WP/PC/ST into the new R13/R14/R15; `RTWP` inverts it (Ch. 4, Ch. 9).

**bitmap mode** — the *VDP* mode giving per-pixel color control, at the cost of
~12 K of *VRAM* (Ch. 15).

**CQ-82** — the nine-item "commercial quality, 1982" checklist: the standard a
Part IX capstone must meet to count as a finished game (Ch. 36; Part IX).

**CRU** — Communications Register Unit: the 9900's bit-serial I/O bus, 4,096
addressable single bits, separate from memory, reached with five instructions
(Ch. 10; App G).

**CRU base** — the value in **R12** that aims a CRU instruction; the hardware bit is
`(R12 >> 1) + displacement`, so R12 is *twice* the base bit number — the factor of
two that trips everyone (Ch. 10).

**`.ctg`** — the project-native cartridge container `libre99asm` emits and the
emulator mounts (Ch. 3; App L).

**DSR** — Device Service Routine: the driver ROM on a peripheral card, paged into
the `>4000` window by a *CRU* bit and reached through `DSRLNK` (Ch. 30).

**DV80** — "display, variable, 80": the text file format of `TI-Writer` and most TI
text tools; `AUTHOR99` writes it (Ch. 31, Ch. 42).

**Editor/Assembler (E/A)** — TI's assembly-language development cartridge and its
manual; the *de facto* reference for the console's services and file formats, and
the dialect `libre99asm` follows (Ch. 6, Ch. 23).

**EA5** — the "Editor/Assembler option 5" memory-image program format: a loadable
binary run from disk without the assembler (Ch. 6; xas99, App L).

**`equates.inc`** — `lib99`'s shared file of named constants; the discipline of
naming every address and magic number once (Ch. 11).

**FAC** — the floating-point *accumulator*, `>834A`, the primary operand and result
of the console's radix-100 math (Ch. 23; App C).

**fast island** (also **fast domain**) — the console ROM plus the scratchpad: the
only 16-bit, zero-*wait-state* territory the CPU has, where code and hot data want
to live (Ch. 5).

**FMT** — the *GPL* sub-language for screen formatting: a compact byte-code for
laying out text and graphics (Ch. 26; App B).

**frame** — one 60 Hz video field; the unit of the game loop, worth about **50,000
CPU cycles** of work (Ch. 17).

**the funnel** (also **the mail slot**) — the 8-bit multiplexer between the 16-bit
CPU and most of the machine; the reason nearly everything off the *fast island*
costs extra cycles (Ch. 4–5).

**GPL** — Graphics Programming Language: TI's interpreted byte-code, run by an
interpreter in console ROM, in which most of the firmware and many cartridges are
written (Ch. 26; *tower of interpreters*).

**GPLLNK** — the linkage that runs a *GPL* routine from assembly (e.g. loading the
console font); slower than *XMLLNK* because it pays the interpreter's tax (Ch. 23).

**GROM** — Graphics Read-Only Memory: the serial, auto-incrementing ROM that holds
*GPL* code and data, reached only through ports at `>9800`/`>9C00`, not the memory
bus (Ch. 25).

**high-byte law** — byte operations on a register use the register's **high**
(most-significant) byte; the beginner's byte bug lives here (Ch. 4, Ch. 8).

**`inplib`** — `lib99`'s input layer: joystick/keyboard sampling and per-frame edge
detection (press / hold / release) (Ch. 21).

**ISR** — Interrupt Service Routine: the console's 60 Hz handler, which ticks the
frame timer, plays the *sound list*, moves auto-motion sprites, and calls the user
hook at `>83C4` (Ch. 22; App C).

**KSCAN** — the console firmware's keyboard scanner: scans the matrix, decodes
modifiers, and leaves a key code at `>8375` (Ch. 21; App C, App G).

**`lib99`** — the reader's accumulating standard library, built module by module
across the book (`vdplib`, `textlib`, `sndlib`, `inplib`, …), born in Chapter 11.

**`libre99asm` / `libre99gpl`** — the project's TMS9900 assembler and GPL tool; the
book's daily compilers (App L).

**LDCR / STCR** — the *CRU* group-transfer instructions: load / store up to 16 bits
at once (a byte operand for ≤8 bits) (Ch. 10; App G).

**loader** — startup code that copies a routine into faster memory (e.g. a hot loop
staged into the *scratchpad*) before running it (Ch. 24, Ch. 37).

**multicolor mode** — the obscure low-resolution *VDP* mode of 4×4-pixel color
blocks; rarely used (Ch. 14).

**the multiplexer** — see *the funnel*.

**name table** — the *VDP* table of one byte per screen cell, naming which pattern
each cell shows; the "screen image" `BENCH99`'s `screen` prints (Ch. 12–13).

**PAB** — Peripheral Access Block: the block in *VRAM* describing a file operation
(opcode, flags, buffer, name) that a *DSR* reads (Ch. 31; App H).

**pattern table** — the *VDP* table of 8×8 character/sprite bitmaps the *name table*
and sprites refer to (Ch. 12–13, Ch. 16).

**PSG** — Programmable Sound Generator (the SN76489): three square-wave tone
channels plus noise, driven through the write-only port `>8400` (Ch. 19; App E).

**prefetch** — the *VDP*/*GROM* habit of reading the byte at the current address
into a latch *before* incrementing, so the first read after setting an address is
stale — a classic source of off-by-one bugs (Ch. 12, Ch. 25).

**QUIT** — `FCTN`-`=`, wired to a hard reset; serious programs disable it (an ISR
duty bit, `>83C2`) and offer a safe quit (Ch. 21; App C).

**radix-100** — the console's floating-point base: eight bytes holding a sign, a
biased exponent, and base-100 mantissa digits (Ch. 23).

**R10** — by book-wide convention, the **software stack pointer**: a full-descending
stack the calling convention pushes return addresses and saves onto, since the 9900
has no hardware stack (Ch. 9).

**R12** — see *CRU base*.

**RLE** — run-length encoding, the book's first compression scheme (`rlelib`);
control byte 0 ends, ≥`>80` a run, else a literal count (Ch. 38).

**scratchpad** (the **pad**) — the 256 bytes of 16-bit, zero-*wait* RAM at
`>8300`–`>83FF`; the *fast island*'s data half, and the most contested memory in
the machine (Ch. 5, Ch. 24; App C).

**SKELETON99** — the project template every Part IX capstone instantiates: phase
structure, 8.8 fixed point, 256-degree angles, entity tables (Ch. 36).

**sound list** — the platform's shared representation of sound over time:
duration-tagged blocks of *PSG* commands the *ISR* auto-player advances each frame
(Ch. 19; App E).

**sprite** — a *VDP* hardware object positioned independently of the background;
32 of them, with automatic motion via a *VRAM* table (Ch. 16).

**status register (ST)** — the 9900's flags word: logical-greater, arithmetic-greater,
equal, carry, overflow, odd-parity, and the interrupt mask (Ch. 4, Ch. 8).

**"speed is a property of addresses, not instructions"** — the book's law of TI
performance: the same instruction costs different cycles depending on *where* its
operands live, because of *the funnel* (Ch. 5).

**textlib / textlib40** — `lib99`'s text engines for the 32-column graphics screen
and the 40-column text mode (Ch. 13–14).

**tower of interpreters** — the machine's three execution layers: Floor 1 the 9900
(machine code), Floor 2 *GPL*, Floor 3 TI BASIC — each interpreting the one below's
guest (Ch. 2, Ch. 26).

**TMS9900** — the console's 16-bit CPU at 3.0 MHz, with its registers *in memory*
(the *workspace*) (Ch. 4).

**TMS9901** — the interface chip owning the keyboard, joysticks, interrupt mask,
timer, and cassette control over the *CRU* (Ch. 10, Ch. 21; App G).

**TMS9918A** — the *VDP*: the video processor with its own private 16 K of *VRAM*
(Ch. 12; App D).

**TMS5220** — the speech synthesizer chip; not emulated by the project (Ch. 20;
App F).

**VDP** — Video Display Processor (the TMS9918A): draws the screen from tables in
its own *VRAM*, reached only through the `>8800`/`>8C00` ports (Ch. 12; App D).

**VRAM** — the 16 K of video RAM private to the *VDP*; the CPU touches it only a
byte at a time through the ports, so it is a warehouse, not workspace (Ch. 12).

**wait state** — an extra CPU cycle imposed when accessing 8-bit memory through
*the funnel*; the tax the *fast island* escapes (Ch. 5).

**workspace** — the sixteen consecutive words of RAM that *are* the 9900's registers
R0–R15; `WP` points at R0, and `Rn` is at `WP + 2n`. Placing the workspace in the
*scratchpad* makes registers fast (Ch. 4).

**XMLLNK** — the linkage that calls a console **ROM** routine (machine code, full
speed) from assembly; preferred over *GPLLNK* where the routine exists (Ch. 23).

**xdt99** — Ralph Benzinger's cross-development suite (`xas99`/`xga99`/`xdm99`), the
book's bridge to the E/A object formats and disk images `libre99asm` does not emit
(Ch. 6; App L).
