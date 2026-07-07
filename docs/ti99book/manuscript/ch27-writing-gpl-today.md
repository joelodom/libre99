# Chapter 27 — Writing GPL Today: `libre99gpl` and Building GROM Cartridges

*The language TI kept to itself, now with a modern assembler — and the header choreography that gets your program onto the master menu, listed by name, launched by a keypress.*

<!-- Part VI — GROM, GPL, and the Operating System · target ≈22 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The GPL program/cartridge header choreography is machine-verified on BENCH99 at commit d81f2db: quizmaster.gpl assembles (libre99gpl asm) to a header AA 02 01 00 + program-list pointer >2010 + entry {next=0000, entry=>201B, namelen=06, "QUIZ99"} (raw bytes), whose entry code disassembles to ALL/MOVE/SCAN/BR; and the booted clean-room console menu discovers and lists a program ("1 FOR TI BASIC") by the same >AA scan. Code in code/ch27/ (quizmaster.gpl). TOOLING GAP (R-12, §27.5, exactly as the outline predicts): libre99gpl asm targets the 24 KiB CONSOLE-GROM image (0-0x5FFF), not a plug-in cartridge GROM at >6000; building a shipping .ctg cartridge GROM and booting it is a project roadmap item — shelf tools (xdt99 xga99, Classic99) build cartridges today. -->

## Writing in the House Language

For forty years, writing GPL was something only Texas Instruments could do. The language was internal, the assembler unpublished (Chapter 26's sidebar), and a hobbyist who wanted to write the machine's house language had to reverse-engineer the opcodes and hand-assemble bytes. That era is over. This book's project ships **`libre99gpl`**, a modern GPL assembler and disassembler — the very tool that built the clean-room console GROM this emulator boots — and with it you can write GPL the way TI's own engineers did: source in, GROM image out, running on the interpreter.

But writing GPL instructions (Chapter 26) is only half of shipping a GPL program. The other half is *getting the machine to run it* — and on the TI that means a specific piece of ceremony: the **cartridge header**, a small structure of magic bytes and pointers that the console's master menu scans for at boot, so that your program appears in the list ("`1 FOR QUIZ99`") and launches when the player presses its number. Get the header right and the console discovers you automatically, enumerates you into its menu, and hands you control; get it wrong and your program is invisible, a GROM full of code the machine never runs. This chapter is both halves: the `libre99gpl` toolchain, and the header choreography that turns a GPL program into a *cartridge* the console will boot — verified this chapter, header byte by byte, against the same menu that lists TI BASIC.

It is also a chapter that finds the edge of the tooling, honestly. The project's `libre99gpl` builds *console* GROMs today (it built the firmware); building a *plug-in cartridge* GROM and packaging it as a shippable file is a capability the project has not grown yet — and the outline predicted exactly this, that the book would drive the emulator's roadmap here. So we build and verify what the tooling supports, name what it does not, and mark the gap as the roadmap item it is.

---

## What You Will Learn

- The `libre99gpl` toolchain: the GPL source format, directives, symbols, and the assemble/disassemble workflow.
- The **GPL cartridge header** byte by byte — the `>AA` signature, the list pointers, the program-list entry — and how the master menu discovers and launches you.
- Program structure in practice: screens drawn with `MOVE`, scratchpad-variable planning, subroutine style, and keyboard loops with `SCAN`.
- Multi-GROM programs: crossing the slot boundaries, data GROMs, and base-address hygiene.
- Shipping formats — and the honest state of building a cartridge from a GPL program today.
- Debugging GPL: the bench's fetch log and `libre99gpl dis`, and the classic GPL bugs.

## The Bridge: From `main()` to a Header

A modern program has an *entry point* the runtime finds by convention: `main()`, or a manifest that names the entry class, or an ELF header field the loader reads. You do not think about it — the toolchain writes the header, the loader reads it, and your code runs. The mechanism is there, but it is hidden: a structured header that a discovering system parses to find and launch your code.

The TI cartridge header is that mechanism, unhidden. When you plug a cartridge in and the console lists your program on its menu, a very concrete thing happened: the console's boot code walked the cartridge's GROM looking for a **signature byte** (`>AA`), followed a **pointer** to a **linked list** of program entries, read each entry's **name** and **entry address**, and built the menu from what it found — then, on your keypress, jumped to the entry address you chose. It is `main()`-discovery with the lid off: a header format you write by hand, a discovery scan you can read (it is GPL, in `console.gpl`), and a launch you can watch. Learning it is learning what "the runtime finds your entry point" actually means — a parse of a header you control — on a machine where the whole handshake is visible. And it is the same skill Chapter 35 generalizes to ROM cartridges and Chapter 30 to peripheral DSRs: the TI's universal "introduce yourself to the system" pattern is a signature byte and a linked list, met here first.

## 27.1 The `libre99gpl` Toolchain

`libre99gpl` is a two-pass assembler with a disassembler, invoked simply:

```sh
libre99gpl asm quizmaster.gpl quiz.bin      # source -> GROM image
libre99gpl dis quiz.bin 2000                # GROM image -> listing (from >2000)
```

The **source format** is the one Chapter 26's `demo.gpl` used and `console.gpl` is written in: GPL mnemonics with space-tagged operands (`>` immediate, `V@` VDP, `@` CPU RAM, `G@` GROM, `*@` indirect), labels in the first column, and a small set of directives. **`GROM >addr`** (or `AORG >addr`) sets the location counter — GPL addressing is *absolute*, so labels resolve to GROM addresses and branches take absolute targets, unlike `libre99asm`'s forced `>6000` origin. **`BYTE`**, **`DATA`**, and **`TEXT`** emit bytes, words, and strings — the raw material of headers and data. Pass one sizes every statement and defines every label at its GROM address; pass two emits the bytes. It is a conventional assembler, and if you have written `libre99asm` (Chapters 3–11), `libre99gpl` will feel immediately familiar — same shape, different instruction set and a GROM-absolute address space. (The xdt99 suite's `xga99` is the community's other GPL assembler, noted for interchange; `libre99gpl` is the project's own, and the one that built the firmware.)

The **disassembler** is the other half and, as Chapter 26 showed, an essential debugging and archaeology tool: point it at any GROM — a cartridge dump, the console ROM, your own output — and it recovers the GPL. Assemble-then-disassemble is the round-trip that proves your program is what you meant: `quizmaster.gpl` assembled and disassembled back gives `ALL >20 / MOVE >0010, …, V@>00C7 / SCAN / BR` — the instructions you wrote.

## 27.2 The Cartridge Header, Byte by Byte

Here is the ceremony that gets you onto the menu. A GPL cartridge's GROM begins with a **header** the console scans, and `quizmaster.gpl` builds one whose bytes we can read straight out of the assembled image:

```text
>2000  AA 02 01 00      >AA signature (valid GROM), version 2, 1 program, reserved
>2004  00 00            power-up list   = none
>2006  20 10            program list    -> >2010   (our list!)
>2008  00 00            DSR list        = none
>200A  00 00            subprogram list = none
>200C  00 00            interrupt link  = none
>200E  00 00            reserved
```

The header is the `>AA` signature and six pointers, and the one that matters here is the **program list** at offset 6. It points to a **linked list of program entries**, each of which is:

```text
>2010  00 00            next entry      = >0000 (this is the last)
>2012  20 1B            entry address   -> >201B (where to start us)
>2014  06               name length     = 6
>2015  "QUIZ99"         the name the menu displays
```

So the whole handshake is: the console finds `>AA`, follows the program-list pointer to the first entry, reads its name (`QUIZ99`) and entry address (`>201B`), adds "`1 FOR QUIZ99`" to the menu, and — when the player presses `1` — jumps to `>201B` to run us. Multiple programs are just more entries in the linked list (the `next` pointer chains them), so one cartridge can offer several menu choices. We can watch the *scanning* end of this work: the booted clean-room console, whose menu code (`console.gpl`) does exactly this scan, discovers a program and lists it — `1 FOR TI BASIC` — by the identical `>AA`-and-list mechanism our header is built for. The header we wrote and the scan that reads it are two ends of one verified contract.

## 27.3 Program Structure in Practice

Past the header, a GPL program is the language of Chapter 26 arranged into a working shape, and `quizmaster.gpl`'s entry shows the skeleton:

```text
>201B  ALL  >20                       clear the screen to spaces
>201D  MOVE >0010, G@TITLE, V@>00C7   draw the title (row 6, col 7) with one MOVE
>2024  SCAN                           read the keyboard
>2025  BR   >2024                     loop until a key
```

Four idioms recur in every GPL program. **Screens are drawn with `MOVE`** (or the `FMT` layout sub-language of §26.6): text and data live in GROM as `TEXT`/`BYTE` and are `MOVE`d onto the VDP — `quizmaster` `MOVE`s its title string from GROM `TITLE` to a screen cell in one instruction, and a fuller UI is a cascade of such `MOVE`s (or a single `FMT` block laying out the whole screen). **Variables are planned in the scratchpad** (Chapter 24) — a GPL program reserves pad words for its state (a score, a question index, cursor positions) exactly as `console.gpl` documents its menu's `>8340`–`>835E` working set, naming each and keeping clear of the interpreter's `>83E0` and the ISR's `>83C0`. **Subroutines use `CALL`/`RTN`** — factoring the program into GPL subroutines the interpreter's data stack manages. And **input is a `SCAN` loop** — the program's outer loop scans the keyboard and branches on the key, the GPL equivalent of Chapter 21's per-frame input read. A complete GPL cartridge — QUIZMASTER's full form, with a title screen, question data in GROM, a `SCAN`-driven answer loop, and a score in the pad — is these four idioms composed: draw with `MOVE`, keep state in the pad, factor with `CALL`, drive with `SCAN`.

## 27.4 Multi-GROM Programs

A single GROM is 6 KiB, and a substantial program — code plus a lot of data (a quiz's questions, a game's levels) — outgrows it. GPL programs span **multiple GROMs**, and doing so cleanly is a matter of respecting the slot structure of Chapter 25. The auto-increment wraps within an 8 KiB slot, so code and data do not silently flow from one GROM into the next; crossing a boundary is an explicit re-aim (a branch to an address in the next slot, which the interpreter's program-counter load handles). The common arrangement is **code in one GROM, data in another** — a "data GROM" holding the questions, the level maps, the text, kept separate from the code GROM so each can be developed and sized independently, with the code `MOVE`ing from the data GROM as needed. The discipline is **base-address hygiene**: know which GROM slot each label lives in, keep cross-slot references explicit, and lay the program out so its hot code is in one slot (fewer boundary crossings) and its bulk data in others. A multi-GROM program is not harder to write than a single-GROM one — the assembler resolves absolute GROM addresses across slots — but it demands that you *know your layout*, because a stray reference that assumes the wrong slot lands in the wrong GROM, a bug the slot-wrapping containment (§25.3) turns from a crash into a subtler wrong-data read.

## 27.5 Shipping Formats — and an Honest Gap

A finished GPL program must become a *file* a machine can load, and the ecosystem has several: the **`.ctg`** format (Marc Rousseau's `ti99sim` format the project uses, which packages GROM pages — cartridge GROMs at pages 3+ — and CPU-ROM pages into one file), images for the **FinalGROM 99** (the modern GRAM cartridge that loads GROM images on real hardware, §25.5), **Classic99** configurations, and MAME **`.rpk`** cartridge files. The ideal is "one build, several targets" — assemble once, package for each.

Here is the honest state of it, and it is exactly the roadmap edge the outline anticipated. The project's `libre99gpl` today builds the **console GROM** — a 24 KiB image (GROM 0–2, addresses `0`–`>5FFF`) that replaces `994AGROM.Bin`, which is how the clean-room firmware is built and how this book runs its GPL. It does *not* yet target a **plug-in cartridge GROM** at `>6000`, nor emit the `.ctg`/`.rpk` packaging that would make a shippable cartridge: assembling `quizmaster.gpl` at `>6000` fails ("past the 24 KiB image"), so this chapter builds it in console-GROM space to verify the header and structure, and demonstrates *discovery* against the console's own menu rather than a plugged-in cartridge. Building a cartridge GROM and packaging it — the "one build, several targets" pipeline — is a **project roadmap item**, and a clear one: `libre99gpl` needs a cartridge-image mode and a `.ctg`/`.rpk` emitter. Until it grows them, a GPL program is shipped either by building it into a custom console GROM (as the firmware is) or through the shelf tools — **xdt99's `xga99`** assembler and the community packagers, or Classic99's cartridge support — which build cartridges today. This is the book driving the emulator: the chapter states precisely the capability the project should add, and why (R-12).

## 27.6 Debugging GPL

Debugging GPL is debugging *two* machines at once — the GPL program and the interpreter running it — and the bench gives you both, as Chapter 26 established. **`libre99gpl dis`** turns any GROM back into readable GPL, so you read your (or anyone's) program from its bytes. **BENCH99's `gromlog`** traces the interpreter's GROM fetches — the GPL instruction stream actually executing — while **`s`** traces the interpreter's 9900 steps, so you watch the program run at the GPL level and the interpreter grind at the 9900 level together. For richer GPL-aware stepping and a GROM viewer, **Classic99** on the shelf has a GPL-level single-stepper (R-12). And the classic GPL bugs are worth naming so you recognize them: **status-byte confusion** (a branch testing the wrong condition because an intervening instruction changed the status byte you meant to test — GPL's implicit status makes this easy to do), and **`MOVE` direction slips** (getting a `MOVE`'s source and destination spaces backwards, so you copy VDP-to-GROM garbage instead of GROM-to-VDP data — the space-tags are easy to transpose). Both are read straight out of a `gromlog`-plus-`dis` trace: you see the branch not taken, or the `MOVE` reading the wrong space, in the fetch stream.

## Lab 27 — QUIZMASTER

The lab is a GPL program with the full cartridge ceremony, in `code/ch27/`.

**`quizmaster.gpl`** — a GPL program with a valid program header and an entry that draws a title and waits on `SCAN`. Build and inspect it:

```sh
libre99gpl asm code/ch27/quizmaster.gpl build/quiz.bin
libre99gpl dis build/quiz.bin 201B
```

The raw bytes at `>2000` show the header — `AA 02 01 00`, the program-list pointer, and the entry `{>201B, "QUIZ99"}` — and the disassembly from `>201B` shows the entry code (`ALL`, `MOVE`, `SCAN`, `BR`). Boot the clean-room console and reach its menu, and you see the *discovery* side of the same contract: `1 FOR TI BASIC`, a program found by the identical `>AA` scan your header is built for.

The **full QUIZMASTER** — title screen, a menu entry, a `SCAN`-driven question loop over data in a second GROM, a score kept in the pad, running from the real console menu — is the chapter's aspiration, and it assembles as a console-GROM program today; running it as a *plugged-in cartridge* awaits the cartridge-image tooling of §27.5. Here you build the program and verify the header and structure that make it discoverable; the exercises extend it (more program-list entries, a data GROM, the `SCAN` answer loop) and specify the cartridge packaging as the roadmap task it is.

> **Field Notes — A 1981 cartridge header, annotated.** Dump the first bytes of any genuine TI-99/4A cartridge's GROM and you find the same structure `quizmaster.gpl` builds: the `>AA` signature, a version byte, and the list pointers, then a program-list entry with a name and an entry address. A 1981 cartridge — a game, an educational title — announces itself to the console in exactly the format of §27.2, because the format never changed: it is the platform's universal contract, and a cartridge from the machine's first year and a GPL program you write in 2026 introduce themselves to the master menu identically. Disassembling a real cartridge's header with `libre99gpl` (or reading its bytes with `gromdump` from Chapter 25) and annotating each field — signature, version, program-list pointer, the chain of entries with their names — is a small, satisfying act of reading a 1981 programmer's ceremony, and confirming that the ceremony you learned is the ceremony they performed. The names in the program list are the very strings the menu showed a child in 1982; they are still there, in the GROM, waiting to be read.

## Exercises

**27.1** ✦ Give the exact header bytes for a GPL cartridge with one program named "GAME" whose entry is at GROM `>6010`. (Signature, version, the program-list pointer, and the entry.)

**27.2** ✦ Round-trip a GPL program through `libre99gpl asm` and `dis`, and confirm the disassembly matches your source. Which directive sets the GROM origin, and why is GPL addressing absolute?

**27.3** ✦✦ Add a second program to `quizmaster.gpl`'s program list (chain a second entry with the `next` pointer) named "SCORES", and verify the two-entry linked list in the assembled bytes.

**27.4** ✦✦ Extend QUIZMASTER's entry: draw a question with `MOVE` from GROM, `SCAN` for a keypress, and `CASE`-dispatch on the answer. Verify the structure disassembles as intended.

**27.5** ✦✦ Lay out a two-GROM QUIZMASTER: code in one slot, question data in the next. Explain where the slot boundary falls and how a cross-slot `MOVE` reaches the data GROM.

**27.6** ✦✦✦ The cartridge tooling gap (§27.5): specify precisely what `libre99gpl` would need to build a shippable cartridge — a cartridge-GROM origin mode and a `.ctg` emitter — and sketch the `.ctg` bytes (banner, GROM page 3 region) your `quiz.bin` would become. (This is a real project roadmap design.)

**27.7** ✦✦✦ Debug a planted bug: introduce a `MOVE` direction slip into `quizmaster.gpl` (swap a source and destination space), assemble it, and find the bug from a `gromlog`-plus-`dis` trace — read the `MOVE` reading the wrong space in the fetch stream.

## Further Reading

- The `libre99gpl` toolchain sources (`crates/libre99-gpl`) — the assembler, the disassembler, and the instruction/operand encodings (App. B).
- `original-content/system-roms/grom/console.gpl` — a complete GPL cartridge/console header and a large body of real GPL structure (Chapter 28 tours it).
- The `.ctg` (`ti99sim`) and `.rpk` (MAME) cartridge format specifications — the shipping targets of §27.5.
- The xdt99 suite (`xga99`) — the community's GPL assembler and cartridge packagers, the shelf tools of §27.5.
- Chapter 25 (GROM) — the slots and bases §27.4's multi-GROM layout respects.
- Chapter 26 (The GPL Language) — the instructions QUIZMASTER is built from, and the `gromlog` debugging of §27.6.
- Chapters 28 and 35 — the menu scan that reads your header, and the ROM-cartridge version of the same "introduce yourself" contract.

## Summary

`libre99gpl` — the modern, open GPL assembler and disassembler that built this project's clean-room console GROM — ends forty years in which only TI could write the machine's house language: source in the format of `console.gpl` (space-tagged operands, `GROM`/`BYTE`/`DATA`/`TEXT` directives, absolute GROM addressing), assembled two-pass to a GROM image, disassembled back for verification and archaeology. Getting a GPL program *run* by the console is the **cartridge header** ceremony: a GROM begins with the **`>AA` signature**, a version, and six list pointers, of which the **program list** points to a linked list of entries — each a `next` pointer, an **entry address**, a name length, and a **name** — that the master menu scans at boot to list your program ("`1 FOR QUIZ99`") and launch it at its entry address on a keypress. This is verified: `quizmaster.gpl` assembles to exactly that header (`AA 02 01 00`, program-list pointer, entry `{>201B, "QUIZ99"}`) with entry code (`ALL`/`MOVE`/`SCAN`/`BR`), and the booted clean-room console discovers and lists a program (`1 FOR TI BASIC`) by the identical scan. A GPL program's structure is four idioms — screens drawn with `MOVE` (or `FMT`), variables planned in the scratchpad, subroutines with `CALL`/`RTN`, input via a `SCAN` loop — spanning **multiple GROMs** with base-address hygiene when it outgrows one slot. And the honest edge: `libre99gpl` builds **console** GROMs today, not **plug-in cartridge** GROMs at `>6000`, nor the `.ctg`/`.rpk` packaging a shippable cartridge needs — a stated **project roadmap item** (the book driving the emulator, R-12), with xdt99's `xga99` and Classic99 as the shelf tools that build cartridges now. Debugging is two machines at once — `libre99gpl dis` reads the program, `gromlog` traces the interpreter running it — and the classic bugs (status-byte confusion, `MOVE` direction slips) read straight out of the fetch trace. The header you write and the scan that reads it are one contract, and it is the same "introduce yourself to the system" pattern the DSRs (Chapter 30) and ROM cartridges (Chapter 35) obey — met here, on the master menu, verified end to end.
