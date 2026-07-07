# Chapter 23 — Console ROM Services: GPLLNK, XMLLNK, and Floating Point

*The console ROM is full of routines you can borrow — a font loader, a floating-point package, cassette I/O — reached through two narrow doorways, if you know the password and pay the toll.*

<!-- Part V — Input, Interrupts, and Console Services · target ≈22 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The §23.5 fixed-point anchor (FXMUL 8.8 multiply) is machine-verified on BENCH99 at commit bb565e4 (1.5*2.0=>0300, 2.5*2.5=>0640, 0.5*0.5=>0040; ~220 cyc/call); code in code/ch23/ (fxcalc). GPLLNK/XMLLNK and the radix-100 FP package are described from the E/A manual/record: the project's clean-room GROM DOES implement GPL services (e.g. the >004A char-set loader, see libre99-gpl/src/font.rs), but invoking them from a bare cartridge needs the E/A image or a shim (§23.6, Ch. 35) — so the doorways are taught as method, verified where the fixed-point alternative stands in for them. -->

## The Library You Didn't Write

By now you have written a great deal — a VDP library, a text engine, a sound driver, an input layer. But the console you are programming already contains, in its ROM and GROM, a substantial library of its own: routines to load character sets, to do floating-point arithmetic, to read the cassette, to scan the keyboard, to reverse bits, to format numbers. TI wrote them for its own firmware and for GPL (Chapter 26), and they sit in the machine whether you use them or not. The question this chapter answers is: can you, from your assembly program, *call* them — borrow TI's code instead of writing your own?

You can, through two doorways with peculiar names. **GPLLNK** is the door into the GPL side — the routines written in the console's bytecode language and living in GROM, including the character-set loaders and cassette services. **XMLLNK** is the door into the ROM side — the machine-code routines in the console's own ROM, the floating-point package chief among them. Both are narrow doors with strict etiquette: you must set up the scratchpad the routine expects, you must preserve the state it and the interrupt handler (Chapter 22) share, and — the sting in the tail — both doorways assume you are running in a particular environment (the one the Editor/Assembler cartridge sets up), so a program that ships as a bare cartridge finds the doors *locked* and must build its own.

This chapter opens both doors, examines the most useful thing behind them — the floating-point package — and confronts the honest complication that makes this a subtler chapter than "here is a function to call": the doorways are not always there, and knowing when they are, and what to do when they are not, is the real lesson. It also draws a line the rest of the book relies on: for the arithmetic a *game* needs, borrowing the ROM's floating point is usually the wrong call, and the fixed-point math of Chapters 8 and 16 — measured here against it — is the right one.

---

## What You Will Learn

- The landscape of console ROM/GROM services, and the two doorways — **GPLLNK** and **XMLLNK** — from assembly.
- **GPLLNK**: borrowing GPL-side routines (character sets, cassette), and the scratchpad/GPL-state contract behind every call.
- **XMLLNK**: the ROM routine table, calling conventions, and direct-address XML.
- The **floating-point package**: radix-100 reals, the 8-byte format, FAC and ARG, the operations, and number↔string conversion.
- **Judgment**: when floating point earns its cycles (rarely, in games) versus the fixed-point math of Chapters 8 and 16 — measured.
- **Environment honesty**: why these doorways assume the E/A image, what breaks in a bare cartridge, and the shims that fix it.

## The Bridge: Calling the Firmware

On a modern system the line between your code and the operating system's is a *system call* or a library call: a clean, documented interface (`read()`, `malloc()`, a math library's `sin()`), with a calling convention the compiler handles, error codes, and no obligation on your part to know the OS's internal state. You call `sqrt()`; you do not first arrange the FPU's scratch registers by hand or promise to preserve the scheduler's variables.

The TI's firmware services are older and more intimate. There is no clean ABI — there is a *convention*, documented in a manual, that you follow by hand: put this value in this exact scratchpad byte, load this workspace, execute this particular linkage instruction, and read your result from that scratchpad byte, all while not disturbing the state the firmware and the GPL interpreter are actively using. Calling a ROM service is less like calling a library and more like *cooperating with another program that shares your memory* — because that is exactly what it is. This intimacy is instructive: it shows you, with nothing hidden, what a "system call" actually is underneath — a controlled jump into shared code with a hand-managed contract about state — and it teaches a caution that the modern abstraction lets you forget: that borrowing someone else's code means inheriting their assumptions about the machine.

## 23.1 The Service Landscape and the Two Doorways

The console's built-in routines split by *which language* wrote them, and that split is the two doorways. The **GPL-side** routines are written in GPL bytecode and executed by the interpreter from GROM: the character-set loaders (the fonts the console draws with), the cassette device service, and various GPL utilities. You reach them with **GPLLNK** — a linkage that saves your context, switches the machine into the GPL interpreter to run a routine at a GROM address you name, and returns you when it finishes. The **ROM-side** routines are machine code in the console's own ROM: the floating-point package above all, plus utility routines TI's own code shares. You reach them with **XMLLNK** — a linkage that jumps to a ROM routine selected from a table, machine code calling machine code.

Both are *linkages* — small standard sequences, `BLWP`-based (Chapter 9's context-switching call), that a program executes to cross into firmware and back. The pattern is: execute the linkage, follow it with a `DATA` word naming *which* routine you want (a GROM address for GPLLNK, a table code for XMLLNK), set up whatever scratchpad the routine reads, and collect the result. The names are archaeology — GPLLNK is "GPL link," XMLLNK is "XML link," XML being TI's term for the ROM's routine-dispatch mechanism — but the idea is simple: two standard ways to call the two kinds of code the console already contains.

## 23.2 GPLLNK: Borrowing the GPL Side

`GPLLNK` runs a GPL routine for you. The canonical use is loading a character set: the console's fonts live in GROM as GPL-drawn data, and rather than ship your own font (as `textlib` did in Chapter 13), you can ask the console to load *its* font into your pattern table. You point a scratchpad pointer at the destination VDP address, execute the GPLLNK linkage with the `DATA` word naming the load routine, and the GPL side draws the console's characters into your VRAM.

This is real in the project, not merely documented: the clean-room console GROM this book's emulator runs implements exactly these services — the fixed service entry that loads the lowercase character set, the loader that writes the console's glyphs to a VDP address the caller supplies (the mechanism lives in the emulator's font code, and it is the same one that fixed a real cartridge's garbled lowercase). So the GPL routines are there to borrow. The catch is the **contract**. GPLLNK does not run in a vacuum — it invokes the GPL interpreter, which has its own workspace (`>83E0`, the GPL registers) and its own status byte (`>837C`) and its own scratchpad variables, and it expects the machine to be in the state the firmware left it. Your program must set up the specific scratchpad the routine reads (the destination pointer, any parameters), must not have trampled the GPL workspace, and must tolerate that the routine may touch VDP state and scratchpad you also use. Calling GPLLNK is borrowing the GPL interpreter for a moment, and you must return the machine in the state it lent it to you — which is why §23.6's environment question is not academic.

## 23.3 XMLLNK: Borrowing the ROM Side

`XMLLNK` runs a ROM machine-code routine. Where GPLLNK crosses into the interpreted world, XMLLNK stays in machine code: the linkage looks up a routine in the ROM's **routine table** by a code you supply, and jumps to it — faster than GPLLNK (no interpreter), and the doorway to the floating-point package. The table is a fixed list of the ROM routines TI chose to expose, each reachable by its index; the `DATA` word after the linkage selects it. For routines not in the table, or when you know the exact ROM address you want, there is **direct-address XML** — a variant that jumps to a ROM address you name outright, an escape hatch that trades the table's stability (addresses can differ across console versions) for reach.

The trade between the two doorways is the interpreter's overhead. XMLLNK's ROM routines are machine code and run at machine speed; GPLLNK's GPL routines run under the interpreter and pay its per-instruction tax (Chapter 26 measures it). So for anything performance-sensitive you prefer the ROM side (XMLLNK) when the routine exists there, and accept GPLLNK only for the genuinely GPL-only services (the character sets, some cassette handling). Both, again, assume the environment of §23.6.

## 23.4 The Floating-Point Package

The most substantial thing behind the XMLLNK door is a full **floating-point package**, and its format is a delightful piece of period engineering: **radix-100**. Instead of binary floating point (a base-2 mantissa and exponent, as an IEEE float uses), TI stored reals in base *one hundred* — each mantissa "digit" is a value 0–99, packed one per byte. A number is eight bytes: a **exponent** byte (a power of 100, biased) and **seven mantissa bytes**, each a base-100 digit, with a sign folded in. Why base 100? Because the machine's users thought in decimal and its BASIC printed decimal, and base 100 is decimal-friendly — converting to and from the decimal digits a person reads is trivial (each byte is two decimal digits), with no binary-to-decimal rounding drama — and because two decimal digits per byte is a tidy fit. It is decimal floating point, chosen so that what you type and what you see match what is stored, at the cost of some efficiency binary would have given.

The package works on two scratchpad accumulators: **FAC**, the floating-point accumulator (at `>834A`), and **ARG**, the argument register (at `>835C`). You load an operand into FAC, another into ARG, and call the operation — **add**, **subtract**, **multiply**, **divide**, **compare** — each an XMLLNK to its ROM routine, each leaving its result in FAC. Around those are the conversions that make it usable: **CIF** (convert integer to float) and **CFI** (float to integer) bridge to the integer world, and the **number↔string** routines convert between FAC and the ASCII text a person reads or types — the routine that turns `3.14159` typed at a keyboard into a radix-100 FAC, and the one that turns a FAC back into digits for the screen. With those, you have a complete calculator: parse a number, do arithmetic, format the result. TI BASIC *is* this package with a parser on top; every `PRINT 2+2` runs through FAC and ARG and these very routines.

## 23.5 The Judgment: Floating Point Versus Fixed

Here is the counsel the rest of the book depends on: **for a game, you almost never want the floating-point package.** It is general and accurate — it handles any magnitude, any fraction, with seven digits of precision — but that generality is expensive. Each operation is an XMLLNK into a ROM routine that manipulates eight-byte radix-100 values digit by digit, and it costs *hundreds to over a thousand cycles* per operation, per the manual's timings. In a 50,000-cycle frame (Chapter 17) that must move sprites, scroll, read input, and play sound, a few floating-point multiplies can eat a meaningful slice, and a physics loop doing dozens of them per frame simply will not fit.

The alternative is **fixed point** (Chapters 8 and 16): represent fractions as scaled integers — 8.8, a whole byte and a 256ths byte — and do arithmetic with the CPU's own integer instructions. A fixed-point multiply is one `MPY` and a shift. We can measure the gap. `FXCALC`'s `FXMUL` multiplies two 8.8 values and is verified correct — 1.5 × 2.0 = 3.0 (`>0300`), 2.5 × 2.5 = 6.25 (`>0640`), 0.5 × 0.5 = 0.25 (`>0040`) — at about **220 cycles** per call, most of that the call overhead around a single `MPY`. Against the floating-point package's per-operation cost, fixed point is several times cheaper and, for a game's bounded ranges (positions 0–255, velocities in pixels-per-frame), just as accurate where it matters.

| Multiply | Approx. cost | Range / precision |
|---|---|---|
| 8.8 fixed point (`FXMUL`) | ~220 cycles (verified) | bounded (game scale), 1/256 |
| radix-100 floating point (ROM) | hundreds–1000+ cycles (manual) | any magnitude, 7 decimal digits |

So the rule: reach for the floating-point package when you genuinely need its *range* or *decimal precision* — a scientific calculator, a financial figure, a value that spans many orders of magnitude — and reach for fixed point for everything a game does. This is why every moving thing in Part III used 8.8 and no floating point appeared: at 3 MHz, in a frame, floating point is a luxury you spend only where its generality earns the cycles.

## 23.6 Environment Honesty: The Locked Door

Now the complication that makes this chapter subtle. GPLLNK and XMLLNK, as most documentation presents them, are **Editor/Assembler utilities** — the linkage vectors and the workspace they use are set up in low RAM *by the E/A cartridge* when it loads your program. Write `BLWP @GPLLNK` in a program assembled and run under E/A, and it works because E/A installed the `GPLLNK` vector for you. But a program that ships as its *own* cartridge (Chapters 27 and 35) — booting from the console menu with no E/A in sight — has **no E/A image**, and those vectors are not there. The door is locked; the `BLWP` jumps into uninitialized memory and crashes.

This is not a corner case; it is the normal situation for a shipped game. The professional response is one of two things. Either you **re-implement minimal shims** — small routines of your own that do what GPLLNK/XMLLNK do (set up the GPL workspace or find the ROM routine and jump to it), installed by your cartridge at startup so `BLWP @GPLLNK` works because *you* provided the vector — a technique Chapter 35 builds for cart-only programs. Or you **avoid the services entirely**: ship your own font (as `textlib` does), do your own fixed-point math (§23.5), and never call the firmware, which is the cleanest path and the reason this book's libraries are self-contained. The honesty here is the whole point of the section: the tempting one-liner `BLWP @GPLLNK` carries a hidden dependency on an environment your shipped program will not have, and a chapter that showed the call without the caveat would be teaching a bug. Know the doorways; know that they are E/A's doorways; and build your own or avoid them when you leave E/A's house.

## Lab 23 — A Calculator, Two Ways

The lab is the judgment of §23.5 made into a program, in `code/ch23/`.

**`fxcalc.a99`** — the fixed-point half, verified: `FXMUL` (8.8 × 8.8 → 8.8) with three checked cases and a measured cost. Build it:

```sh
libre99asm code/ch23/fxcalc.a99 --format bin -o build/FXC.bin --symbols build/fxc.map.json
```

On the bench, `m 8320 6` shows the three products (`>0300`, `>0640`, `>0040`), and bracketing `FX0`/`FX1` with `cycles` reads the ~220-cycle cost. This is the arithmetic a game does, at game speed.

**The floating-point half** — a scientific-notation calculator over `textlib` (Chapter 13) that parses a typed number into FAC, does arithmetic with the ROM package, and formats the result — is the doorway lab, and it comes with §23.6's asterisk: it needs the E/A image or a GPLLNK/XMLLNK shim, so it is built for an E/A-hosted context (or with the shims of Chapter 35) rather than the bare bench, and the project's GPL services stand behind it. The pedagogical payoff is the *benchmark*: run the same computation both ways — the ROM floating-point package and `FXMUL` — and measure them side by side, and the several-fold difference (§23.5's table) turns the "use fixed point in games" advice from assertion into a number you produced. The exercises build the fixed-point calculator fully and specify the floating-point one for an E/A host.

> **Field Notes — Radix-100 in the wild.** You can *see* the floating-point format if you go looking. A TI BASIC program's numeric variables live in VDP RAM (Chapter 28), stored as radix-100 floats, so a running BASIC program has its numbers sitting in the VDP where `vram` can dump them. Find a variable holding, say, `100`, and you will see the radix-100 encoding — an exponent byte and mantissa bytes in base 100 — rather than the binary you might expect: `100` is `1 × 100¹`, a clean single-digit mantissa. Reading a BASIC program's numbers out of the VDP with the bench, and decoding the radix-100 bytes back to the decimal values, is a small, satisfying act of format archaeology, and it makes the abstract "eight bytes, base 100" concrete: there is the exponent, there are the digits, there is the number you typed, stored exactly as decimal because that is what the users thought in.

## Exercises

**23.1** ✦ Name the two doorways into the console's built-in routines, and say which kind of code each reaches (GPL bytecode vs ROM machine code) and therefore which is faster.

**23.2** ✦ Why is TI's floating point stored in radix-100 rather than binary? Give the reason in terms of what the machine's users typed and read.

**23.3** ✦✦ Extend `fxcalc` with `FXDIV` (8.8 ÷ 8.8) using the CPU's `DIV`, verify a case or two on the bench, and add its cost to §23.5's table.

**23.4** ✦✦ Explain precisely why `BLWP @GPLLNK` works in an E/A-hosted program but crashes in a bare cartridge. What two responses does §23.6 offer, and which does this book's `lib99` take?

**23.5** ✦✦ Build the fixed-point calculator: parse two 8.8 numbers typed via `textlib`/`inplib`, `FXMUL` them, and display the result. (The floating-point twin is 23.7.)

**23.6** ✦✦✦ Read radix-100 in the wild: run a small TI BASIC program that sets a numeric variable, find the value in VDP RAM with `vram`, and decode the radix-100 bytes back to the decimal number. Document the exponent and mantissa you found.

**23.7** ✦✦✦ For an E/A host (or with Chapter 35's shims), write the floating-point calculator using the ROM package: number-string → FAC, an operation, FAC → number-string, over `textlib`. Benchmark one operation against `FXMUL` and confirm the several-fold gap of §23.5.

## Further Reading

- *Editor/Assembler Manual*, Texas Instruments — the definitive reference for GPLLNK, XMLLNK, the ROM routine table, the floating-point package, and the scratchpad (FAC/ARG) conventions this chapter surveys.
- Chapter 8 (Arithmetic) — the fixed-point and integer math that §23.5 measures against floating point.
- Chapter 22 (Interrupts) — the GPL state and scratchpad the ISR and GPLLNK both touch (the shared-state contract).
- Chapter 26 (The GPL Language) — what the GPL routines behind GPLLNK actually are, and the interpreter's per-instruction cost.
- Chapter 28 (The OS in GROM) — TI BASIC as the floating-point package with a parser, and where its numbers live.
- Chapter 35 (Cartridge Engineering) — building the GPLLNK/XMLLNK shims a bare cartridge needs (§23.6).

## Summary

The console ROM and GROM contain a library of routines — character-set loaders, a floating-point package, cassette I/O, utilities — reachable from assembly through two doorways: **GPLLNK**, a `BLWP`-based linkage that runs a **GPL-side** routine (from GROM, via the interpreter — the character sets, cassette; the project's clean-room GROM really implements these), and **XMLLNK**, which runs a **ROM-side** machine-code routine selected from a table (or by direct address), faster and the door to floating point. Both carry a **contract**: set up the exact scratchpad the routine reads, preserve the GPL workspace (`>83E0`) and state the routine and the interrupt handler share, and return the machine as you found it. The **floating-point package** stores reals as **radix-100** — an exponent byte and seven base-100 mantissa bytes, decimal-friendly so typed and stored numbers match — and operates on two accumulators, **FAC** (`>834A`) and **ARG** (`>835C`), with add/subtract/multiply/divide/compare and integer/string conversions (CIF, CFI, number↔string); TI BASIC is this package with a parser. The **judgment** the book relies on: floating point is general but costs hundreds-to-thousands of cycles per operation, so it earns its cycles only where range or decimal precision is genuinely needed, while a game's math uses **fixed point** — `FXMUL`, an 8.8 multiply verified correct and measured at ~220 cycles, several times cheaper. And the **honesty**: GPLLNK/XMLLNK are Editor/Assembler utilities whose vectors E/A installs, so a bare cartridge finds the doors locked and must either re-implement minimal shims (Chapter 35) or, as `lib99` does, avoid the services and be self-contained. Know the doorways, know they are E/A's, and prefer your own fixed-point, own font, own code when you ship without E/A.
