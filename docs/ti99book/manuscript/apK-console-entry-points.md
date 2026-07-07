# Appendix K — Console Entry Points and Service Codes

<!-- Appendices · target ≈8 pp · companion to Ch. 22–24 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The CPU-ROM vectors and entry points (K.2), the GPL interconnect/GPLLNK table (K.3), the XMLLNK mechanism and XML table homes (K.4), and the floating-point package XML numbers (K.5) are tier-1 for the book's machine: read directly from the clean-room firmware original-content/system-roms/rom/console.asm (the >0000 vector block: reset WP >83E0 / PC START >0024, level-1 int WP >83C0 / PC ISR, KSCAN public entry >000E, interpreter entries >0016/>001C, XOP vectors >0040-0050; the FP routines FADD..CFI with their XML numbers; the XML master table >0CFA, FLTAB >0D1A, XTAB >12A0) and original-content/system-roms/grom/console.gpl (the twenty-slot GROM interconnect table >0010-0037 with DSRLNK, the char-set loaders >0016/>0018, boot entry >0020; the fixed service entries >0038-005F with the >004A lowercase loader). All of it is RECON-pinned / oracle-verified. The XML service numbers are the authentic TI FP-package numbers (byte-identical master table). The scratchpad interface (K.6) cross-references Appendix C (tier-1). K.7 states honestly (R-12) which classic entry points the project provides as firmware vs. which (the VSBW-family E/A utilities) the book reimplements as library code. -->

The console ROM and GROM are not just the boot code — they are a **service
library** the whole machine leans on: the interrupt handler, the keyboard scanner,
the GPL interpreter, the floating-point package, the linkage routines that let a
cartridge call all of it. This appendix catalogs the documented entry points and
service codes of the **book's machine** — the project's clean-room firmware, which
the emulator runs as real ROM/GROM. The teaching is Chapters 22–24 (the ISR, the
FP calculator, the scratchpad) and the boot trace of Chapter 28; this is the
address card behind them.

**Two address spaces, kept separate.** The console lives in two memories at once:
**CPU ROM** (`>0000`–`>1FFF`, TMS9900 code — K.2) and **GROM** (the GPL program
store — K.3). Both number their low entries `>0010`–`>0037`, so a stated address
is meaningless without its space. This appendix labels every entry **CPU** or
**GROM**.

## K.1 How a cartridge reaches a service

Three linkage routines carry almost all traffic (Ch. 29):

- **GPLLNK** — call a **GPL** service (a GROM interconnect slot, K.3) from either
  world. The console's char-set loaders and `DSRLNK` are reached this way.
- **XMLLNK** — call a **machine-language** service through the XML table (K.4).
  The floating-point package (K.5) is the great customer.
- **DSRLNK** — call a peripheral **device** service (a DSR, Appendix H). It is
  itself GPL interconnect slot 0.

The book's from-scratch ethos means the reader also *reimplements* several classic
utilities (VSBW, VMBW, …) as library code rather than calling TI's — K.7 draws
that line.

## K.2 CPU-ROM vectors and entry points

The bottom of CPU ROM is the hardware vector table and a small set of fixed public
entries. Tier-1 (`console.asm`, RECON-pinned):

| CPU addr | Entry | Contract |
|---|---|---|
| `>0000` | **reset** vector | WP = `>83E0` (GPLWS), PC = `START` (`>0024`) |
| `>0004` | **level-1 interrupt** vector | WP = `>83C0` (INTWS), PC = the console `ISR` |
| `>0008` | level-2 vector | WP = `>83C0`, PC = a stub (present, unreachable on the 4A) |
| `>000E` | **KSCAN** public entry | `B @KSCAN` — scan the keyboard/joysticks (Appendix J) |
| `>0016` | interpreter entry (opcode in `R9`) | resume the GPL loop with an opcode staged |
| `>001C` | interpreter entry (fetch) | resume the GPL loop, fetching the next opcode |
| `>0020` | CLEAR/BREAK test | the FCTN-4 break check |
| `>0024` | `START` | reset / power-up: set the GPLWS ports, strobe GROM, enter the loop |
| `>006A` | `SOFT` | the public soft-entry: clear the condition bit and run |
| `>0070` | GPL main loop | one interrupt window per instruction (`>0078` fetch) |

**The XOP vectors** (`>0040`–`>007F`) are present and behaviour-faithful even
though the 4A shipped no XOP hardware: XOP 0 (`>0040`) points at the extended-GPL
trampoline; XOP 1 (`>0048`) and XOP 2 (`>0050`) are user-definable (`console.asm`).
Chapter 9 uses an XOP as a software trap; this is where its vector lands.

**The reset contract** is worth stating because Chapter 28 traces it: the `>0000`
vector loads WP = `>83E0` and jumps to `START` at `>0024`, which sets `R13`/`R14`/
`R15` to the GROM/flag/VDP ports, strobes GROM once to settle the prefetch, points
the boot GROM address at `>0020`, clears the condition bit, and falls into the
interpreter — the machine is "running GPL" within a few dozen instructions of
power-on.

## K.3 The GPL interconnect table (GPLLNK)

In **GROM**, the console exposes a table of twenty branch slots at `>0010`–`>0037`,
each a 2-byte `BR` a `GPLLNK` call vectors through. Tier-1 (`console.gpl`):

| GROM slot | Addr | Service |
|---|---|---|
| 0 | `>0010` | **DSRLNK** — device service link (Appendix H) |
| 3 | `>0016` | load the **standard** character set (Appendix J) |
| 4 | `>0018` | load the **thin** ("small") character set |
| 8 | `>0020` | the fixed GPL **boot** entry (→ `START`) |
| 1,2,5–7,9–19 | — | return cleanly (a stray call no-ops and continues) |

Beyond the branch table, the fixed **service entries** at GROM `>0038`–`>005F`
are a stub grid with one live service:

- **`>004A` — load the lower-case (small-capitals) set** (Appendix J). This is the
  service a cartridge calls to get lowercase; it did not exist on the 99/4 (the
  lowercase hardware came with the 4A), and dozens of cartridges depend on it.
  Every other service entry returns gracefully, so a cartridge that calls an
  unimplemented service no-ops rather than hanging — the project logs a breadcrumb
  (`>83E` marker) when that happens, which is how the clean-room GROM was completed
  (Ch. 28).

## K.4 XMLLNK and the XML table

`XML` (GPL opcode `>0F`, Appendix B) and its assembly cousin **XMLLNK** dispatch a
**machine-language** routine through a two-level table. The immediate byte's
**high nibble selects a table** from the master **table-of-tables**, and its **low
nibble selects the entry** in that table; the console `BL`s the routine there.
Tier-1 homes (`console.asm`):

| Structure | Home | Role |
|---|---|---|
| XML master (table-of-tables) | GROM/ROM `>0CFA` | byte-identical to authentic; the `>F0` entry is the ML-launch vector |
| **FLTAB** (XML table 0) | `>0D1A` | the floating-point dispatch (K.5) |
| **XTAB** (XML table 1) | `>12A0` | the number-conversion package (K.5) |

So `XML >06` means "master table entry 0 (FLTAB), routine 6" — `FADD`. `XML >12`
means "master table entry 1 (XTAB), routine 2" — `CFI`. Because the master table
is byte-identical to the authentic console, **the XML service numbers below are the
standard TI numbers** — code that calls `XML >06` for a floating add works on the
book's machine exactly as on a real console.

## K.5 The floating-point package

The console's radix-100 floating-point package (Ch. 23) is a set of XML routines
operating on two scratchpad accumulators:

- **FAC** — the floating accumulator, `>834A` (8 bytes, radix-100; Appendix C).
- **ARG** — the argument, `>835C`.
- **FP error byte** — `>8354` (over/underflow and domain errors land here).

The service numbers (tier-1, `console.asm`; the authentic XML numbers):

| XML | Routine | Operation |
|---|---|---|
| `>06` | `FADD` | FAC ⇐ ARG + FAC |
| `>07` | `FSUB` | FAC ⇐ ARG − FAC |
| `>08` | `FMUL` | FAC ⇐ ARG × FAC |
| `>09` | `FDIV` | FAC ⇐ ARG ÷ FAC |
| `>0A` | `FCOMP` | compare ARG against FAC (sets the status tail) |
| `>0B`–`>0F` | `SADD`/`SSUB`/`SMUL`/`SDIV`/`SCOMP` | the same, popping **ARG from the value stack** first |
| `>01` | `ROUND1` | round on the first guard digit |
| `>02` | `ROUND` | round at the digit position in `>8354` |
| `>03` | `STST` | set the 9900 flags from FAC's sign/zero |
| `>04` | `OVEXP` | the over/underflow filter (clean-zero underflow, saturating overflow) |
| `>05` | `OV` | overflow test |
| `>10` | `CSN` | convert a string to floating point (VDP text) |
| `>11` | `CSNGR` | convert a string to floating point (source-selectable) |
| `>12` | `CFI` | convert floating point to a signed 16-bit integer at `>834A` |
| `>1A` | `SGROM` | search GROM headers (the device/subprogram scan) |

The pattern (Ch. 23): stage an operand in FAC (and, for two-operand ops, ARG or
the value stack), `XMLLNK` the routine, read the result back from FAC. The
`F`-prefixed forms take ARG in place; the `S`-prefixed forms pop it from the GPL
value stack, which is how the BASIC interpreter chains expression evaluation.

## K.6 Scratchpad interface variables

The routines above communicate through fixed scratchpad cells — the console's
public variables. The full byte-by-byte atlas is **Appendix C**; the hand-off
cells this appendix's services use most:

| Cell | Name | Used by |
|---|---|---|
| `>834A` | FAC (8 bytes) | the FP package (K.5); also the char-set loader's destination |
| `>835C` | ARG | the FP package |
| `>8354` | FP error / round position | the FP package |
| `>8375` | KEY | KSCAN (K.2, Appendix J) |
| `>8374` | key-unit / mode | KSCAN |
| `>837C` | GPL status byte | the GPL interpreter (Appendix B) |
| `>83E0` | GPLWS | the interpreter workspace (K.2) |
| `>83C0` | INTWS + seed | the interrupt handler (Ch. 22) |
| `>83C4` | user ISR hook | the interrupt handler (Ch. 22) |

## K.7 What the project provides — and what it doesn't (R-12)

The book's machine supplies, **as real firmware**, the linkage mechanisms and the
services above: `GPLLNK`/`XMLLNK`/`DSRLNK`, the GPL interpreter, KSCAN, the ISR,
the character-set loaders, and the floating-point package at the authentic XML
numbers. Those are tier-1 and callable on the bench.

It does **not** expose the classic **E/A utility set** — `VSBW`, `VMBW`, `VSBR`,
`VMBR`, `VWTR`, `KSCAN` (the DEF/REF names) — as fixed console entry points,
because those are **Editor/Assembler-supplied** utilities the E/A loader resolves,
not console-ROM addresses. In keeping with the book's from-scratch method, the
reader **reimplements them as library code**: `vdplib`'s single-byte and
multi-byte VRAM writes are the reader's `VSBW`/`VMBW` (Ch. 12), built directly on
the VDP ports (Appendix D); `inplib` is the reader's KSCAN wrapper (Ch. 21). So
where a TI assembly listing writes `BLWP @VSBW`, the book writes `BL @VSBW` into
`vdplib`'s own routine — same idea, project-owned code, no dependence on a utility
table the project doesn't ship. On real hardware or under E/A, the classic names
resolve to TI's utilities; the semantics match, the addresses do not.

*See also:* Chapter 22 (the VBLANK ISR, the user hook, `PROFILE99`), Chapter 23
(the floating-point calculator via XMLLNK), Chapter 24 (`PADWATCH` and the
scratchpad), Chapter 28 (the boot trace and §28.7's clean-room GROM tour),
Chapter 29 (the XML hybrid — mixing GPL and machine language), Appendix B (the GPL
`XML` opcode and status byte), Appendix C (the full scratchpad atlas), Appendix H
(`DSRLNK` and device services), Appendix J (the character-set loaders and KSCAN).
