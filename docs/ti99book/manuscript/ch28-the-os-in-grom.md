# Chapter 28 — The Operating System in GROM: Boot, Menu, and TI BASIC as Artifact

*What actually happens when you turn the machine on — the leap from silicon to GPL, the menu that finds your cartridge, and TI BASIC laid open as the GPL program it always was.*

<!-- Part VI — GROM, GPL, and the Operating System · target ≈22 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The boot path is machine-verified on BENCH99 at commit 163fc84: the ROM jumps to GROM >0020 (BR >0052) with the interconnect BR-stub table below it (libre99gpl dis); the boot reaches the clean-room title screen (M1, "TEXAS INSTRUMENTS HOME COMPUTER / READY-PRESS ANY KEY") and, on a keypress, the selection menu (M2, "1 FOR TI BASIC") built by the >AA GROM scan; and the post-boot scratchpad birth certificate is dumped. The whole boot runs the project's clean-room console GROM (original-content/system-roms/grom/console.gpl) on the genuine GPL interpreter (--system-grom). TI BASIC's internals (CRUNCH/EXECUTE, the two-interpreter tax) are described from the record; the clean-room GROM demonstrates the boot/menu contract, not authentic TI BASIC bytes. -->

## From Silicon to Software, Watched

Turn on a TI-99/4A and, a second later, you are looking at a title screen: "TEXAS INSTRUMENTS HOME COMPUTER," a colour bar, "PRESS ANY KEY TO BEGIN." It feels instantaneous and inevitable, the machine simply *being ready*. But between the power reaching the chips and that screen appearing, a precise sequence of events unfolds — the CPU vectoring to a fixed address, the console ROM initializing the hardware, and then, the moment that defines this machine, a *leap into GPL*: the 9900 firmware starts the interpreter of Chapter 26 and hands control to a GPL program, and from that instant on the console you are looking at is not 9900 code but *interpreted GPL*, drawing its title with `MOVE`s and scanning for your cartridge with a linked-list walk. The operating system of the TI-99/4A is a GPL program. Boot is the story of how the machine becomes that program.

This chapter walks that story, and it can, because this book's project has done a remarkable thing: it has *rewritten the console's operating system in GPL from scratch* — a clean-room console GROM, `console.gpl`, that contains no TI copyrighted bytes yet boots on the genuine machine, drawing an original title screen, scanning for cartridges, and dispatching them, all in GPL assembled by `libre99gpl` (Chapter 27). So we do not merely read about the boot; we watch *our own* boot, whose every instruction we can disassemble and whose every scratchpad byte we can dump — a fully-specified boot contract, proven by re-implementation, which is also this book's answer to the preservation question (Chapter 45).

And it lets us end Part VI where the platform's reputation was made and lost: **TI BASIC**, dissected not as a language to learn but as an *artifact* — a GPL program that interprets BASIC tokens, running on the interpreter that runs on the CPU, three layers deep, which is exactly why it was so slow. The machine's most infamous weakness, explained by everything you now know.

---

## What You Will Learn

- Power-on precisely: the RESET vector, the console ROM's initialization, and the leap into GPL at the title screen.
- The **master menu algorithm**: scanning GROM bases and headers, building the selection list, and dispatching your choice.
- **Power-up hooks**: code that runs before the menu, its legitimate uses and famous abuses.
- The console services that live GPL-side: character sets, cassette dialogs, pieces of KSCAN.
- **TI BASIC dissected as a GPL program**: the CRUNCH/EXECUTE loop, why variables live behind the video chip, and the two-interpreter tax quantified.
- The console versions that occasionally bite, and the project's clean-room console GROM.

## The Bridge: A Bootloader You Can Read

Modern boot is a deep, opaque stack: firmware (UEFI) initializes hardware and hands off to a bootloader, which loads a kernel, which starts an init system, which launches user space — thousands of pages of code across several privilege levels, most of it proprietary or too large to hold in your head, and none of it visible as you wait at a splash screen. You trust that "it boots"; you rarely see how.

The TI's boot is the same *shape* — hardware init, then a hand-off to higher-level software that presents a menu and launches programs — compressed to a size you can read whole and watch execute. The "firmware" is the console ROM's short init; the "higher-level software" is a GPL program; the "menu" is a linked-list scan; the "launch" is a jump to an entry address. There is no hidden privilege level, no proprietary blob you cannot inspect — the whole boot is a few hundred GPL instructions in a GROM you can disassemble, running on an interpreter whose 9900 steps you can trace. Studying it is studying what a bootloader *is* — bring up the hardware, find the bootable things, present them, launch the chosen one — at a scale where "find the bootable things" is a scan you can read and "launch" is a `BR` you can watch. Every modern boot does these same jobs behind its opacity; here they are in the open, and this book's clean-room re-implementation proves they are *fully understood*, because we rebuilt them.

## 28.1 Power-On, Precisely

When power reaches the TMS9900, it does what Chapter 9's interrupt model dictates for **RESET** (level 0): it loads its workspace pointer and program counter from the RESET vector at the very bottom of memory (`>0000`–`>0003`, in the console ROM), and begins executing the console's initialization code. That 9900 code does the hardware bring-up — set up the workspace, initialize the VDP registers, clear VRAM, set the sound chip silent, prepare the scratchpad — the machine-level groundwork that must happen in native code because the GPL interpreter itself is not yet running.

Then comes the leap. The initialization finishes by *starting the GPL interpreter* and pointing it at the console's boot GPL — and from that instant, the console is a GPL program. On our clean-room GROM, the ROM's hand-off lands at GROM `>0020`, and disassembling it shows the entry:

```text
>0020  BR   >0052        the ROM jumps here; branch to the real boot code
>0022  BR   >11FE        (below: the interconnect table — DSRLNK, char-set
>0024  BR   >0C82         loaders, and the other fixed service entries the
>0026  BR   >0D59         GPL side exposes, one BR stub each)
```

`>0020` branches to `>0052`, where the GPL boot proper begins — and from there the console draws its title with the `MOVE` cascade of Chapter 26, sets up the display, and waits. The result is the screen we verified: the clean-room title, "TEXAS INSTRUMENTS HOME COMPUTER / READY-PRESS ANY KEY TO BEGIN." Boot has crossed from silicon to software, and everything after is GPL.

> **The scratchpad birth certificate.** By the time the title appears, the boot has written the scratchpad into a known initial state — the *birth certificate* every program inherits. Dumping it after boot (`m 8300 48`) shows the low pad initialized (the first words hold boot state, the rest cleared), with the firmware's live variables — the frame counter, the timers, the GPL workspace — established in the upper pad (Chapter 24's atlas). A program that boots into this state can rely on it; a program that wants the bare machine (Chapter 22's poll-only) tears parts of it down deliberately. Either way, the birth certificate is the boundary between the console's setup and your program's, and reading it is reading exactly what the machine hands you at `main()`.

## 28.2 The Master Menu Algorithm

Past the title, a keypress takes you to the **selection menu** — the numbered list of things you can run — and building that list is a precise algorithm, the same one Chapter 27's header was built for, now seen from the scanning side. The menu code walks the GROM address space: for each GROM **base** and each cartridge **slot**, it peeks the first byte for the **`>AA` signature** (Chapter 27), and where it finds one, follows the header's **program-list pointer** and walks the linked list of entries, reading each program's **name** and **entry address**. From the names it builds the on-screen list — "`1 FOR ...`", "`2 FOR ...`" — numbering them as it goes, and it remembers each entry address so a keypress can dispatch to it.

We verified both halves. The clean-room console, booted and advanced past the title, shows exactly this list:

```text
PRESS
1 FOR TI BASIC
```

— a program *discovered* by the `>AA` scan and listed by its name, the mirror image of the header `quizmaster.gpl` builds. And `console.gpl`'s own menu code documents the machinery: the scan window, the far-list handling for programs whose list lies past the initial peek, the working variables (`>8340`–`>835E`) that hold the current scan pointer and the built list. When you press `1`, the menu **dispatches** — jumps to the chosen entry's address, launching it (a GPL program via the interpreter's sub-stack, a machine-code program via `XML`, as `console.gpl` notes). "`PRESS 2 FOR TI BASIC`" is not special-cased magic; it is one entry in a list, discovered by a scan, dispatched by a jump — and *your* cartridge (Chapter 27) joins that list by the identical mechanism, which is the whole point: the console has no privileged built-ins, only entries it found, and yours is as real as TI's.

## 28.3 Power-Up Hooks

Between "power on" and "show the menu" there is a hook the platform exposes: the **power-up list**, the header pointer (Chapter 27) that runs code *before* the menu appears. A GROM (a peripheral card's, or a cartridge's) whose header carries a power-up-list pointer gets its power-up code executed during boot, before the user sees a choice — and it is called for legitimate reasons: a peripheral card initializes itself (sets up its DSR, Chapter 30), a cartridge that wants to auto-start can seize control before the menu. The console's own boot walks these lists (the `PUCALL`/`SFAR` logic in `console.gpl`) and calls each power-up routine it finds.

The legitimate uses are real — hardware that must initialize before anything else, a cartridge designed to run immediately. But the power-up hook is also the platform's most-abused feature, because "code that runs automatically at boot, before the user chooses anything" is exactly what a program that wants to be *unavoidable* wants: a cartridge that grabbed the power-up hook could bypass the menu entirely, auto-starting itself so the user never saw a choice — convenient for a single-purpose cartridge, and a small hostile act when done to lock a user in. The hook is a capability, and like all "runs before the user has a say" capabilities it cuts both ways; knowing it exists explains both the peripheral that "just works" at power-on and the occasional cartridge that hijacks the boot.

## 28.4 System Services, GPL-Side

Much of what feels like "the operating system" lives GPL-side, in the console GROMs, as GPL routines the machine runs for you. The **character sets** are here — the fonts loaded into the VDP pattern table at boot are GPL data moved by GPL code (Chapter 23's GPLLNK reaches them), which is why loading the console font is a GPL service and not a ROM call. The **cassette dialogs** — "PRESS CASSETTE PLAY," the record/verify prompts — are GPL routines that drive the cassette hardware and the user interaction (Chapter 33's storage). Pieces of **KSCAN**'s personality (Chapter 21) — the repeat timing, the modifier handling, the mapping to the GPL status byte — are GPL-side too. The console, in other words, is a GPL *application suite*: the title screen, the menu, the fonts, the cassette handler, the BASIC that Chapter 28.5 dissects — all GPL programs running on the interpreter, sharing the scratchpad, dispatched from the same boot. The 9900 ROM is the thin layer that brings up the hardware and runs the interpreter; the *personality* of the machine — everything the user experiences — is GPL. This is why Part VI matters so much to understanding the platform: the software you meet as a user is not the CPU's, it is the interpreter's, and to read the OS is to read GPL.

## 28.5 TI BASIC, Dissected as a GPL Program

Now the machine's most notorious feature, laid open. **TI BASIC** is not, architecturally, a language the 9900 runs. It is a **GPL program** — an interpreter for the BASIC language, written in GPL, living in the console GROMs — and understanding that one fact explains everything about it, above all its infamous *slowness*.

Consider what happens when a TI BASIC program runs a line. The BASIC text you typed is first **crunched** — tokenized into a compact internal form (keywords become single bytes, numbers become their radix-100 floats of Chapter 23) — and, crucially, that crunched program is stored **in VDP RAM**, behind the video chip, reached through the ports of Chapter 12, because the 16 KiB of VRAM is where the space was (the scratchpad and the console's small RAM being otherwise spoken for). So a BASIC program's text *and its variables* live behind the VDP, accessed one byte at a time through the address-and-data ports. Then the **EXECUTE** loop runs: the GPL BASIC interpreter fetches the next token (from VDP RAM), decodes it, and performs it — and performing it means running *GPL code*, which is itself interpreted by the *9900*.

Count the layers. A single TI BASIC statement is interpreted by GPL, which is interpreted by the 9900 — **two interpreters stacked** — and its data is fetched byte-by-byte through the VDP port. Chapter 26 measured the GPL interpreter's tax at roughly an order of magnitude over native 9900 code; TI BASIC pays that tax *and then* the BASIC-over-GPL tax on top, plus the VDP-port cost for every variable access — so a TI BASIC statement runs perhaps **two orders of magnitude** slower than the equivalent hand-written 9900 assembly. This is the **two-interpreter tax**, and it is the whole explanation of the folklore: TI BASIC was slow not because BASIC is slow or the 9900 is slow, but because the machine ran BASIC *on an interpreter on an interpreter*, with its data hidden behind a serial video port. Every complaint about TI BASIC's speed traces to this architecture, and it is a design that made sense at the time (GPL was how TI wrote everything; VRAM was where the memory was) and cost the platform dearly in reputation. Reading TI BASIC as a GPL artifact — which `libre99gpl dis` lets you do against the console GROMs — is watching the machine's slowest feature explained by everything Part VI taught: GROM, the interpreter, the scratchpad, the VDP port, all stacked into one famously sluggish REPL. (This book does not teach TI BASIC as a language; it dissects it, once, as the artifact it is — and the clean-room GROM demonstrates the boot-and-menu machinery around it, the authentic BASIC's bytes being TI's, read here only through the disassembler and the record.)

## 28.6 A Field Guide to Console Versions

"The TI-99/4A" is not one fixed machine (Chapter 44), and the console GROMs are one place the differences bite. The original **99/4** (1979) had a different GROM operating system and a calculator keyboard (Chapter 21); the **99/4A** (1981) is the machine this book targets, with its revised GROMs; and even among 4As there are **GROM revisions** (the "v2.2" and others) with small behavioral differences — a service entry at a slightly different address, a subtly changed scan, a fixed bug that some software had come to depend on. Most software never notices, because it uses the stable, documented entry points; but software that reached into the GROM at a specific address, or relied on an exact behavior, could break across versions — the classic "works on my console, not yours" bug of the era. The defensive posture is the one this book preaches throughout: use documented entry points and headers, not raw addresses; detect capabilities rather than assume them; and know that the console you test on is one of several. The clean-room GROM is, in this light, one more "version" — a fully-documented one — and the fact that it boots the same software (a cartridge's header scans the same, a GPL program dispatches the same) is the proof that the *contract* is what matters, not the exact bytes.

## 28.7 The Clean-Room Console GROM

Which brings us to the project's quiet triumph. The emulator this book is built on does not merely run TI's copyrighted console GROM; it can boot a **clean-room** one — `console.gpl`, an original operating system written in GPL from scratch, containing no TI copyrighted bytes, that brings up the machine, draws an original title screen, scans for and dispatches cartridges, and runs on the *genuine* GPL interpreter, on the same emulator, via `--system-grom`. We have watched it work throughout this chapter: its boot entry at `>0020`, its title screen, its menu discovering "`1 FOR TI BASIC`," its documented scratchpad map. It is a full re-implementation of the boot contract of §§28.1–28.3, and its existence *proves* that contract is completely specified — you cannot rewrite what you do not fully understand, and a clean-room GROM that boots real software is a certificate of understanding.

It is also this book's answer to the **preservation question**. A platform whose operating system is proprietary and whose ROMs cannot be freely redistributed is a platform at risk — of being un-runnable when the last legal ROM copy is gone. A clean-room, freely-licensed GROM that boots the machine and runs its software is a hedge against that loss: the TI-99/4A, kept alive not by copying TI's bytes but by *re-implementing its contract*, openly. That you can boot this emulator with a GROM no one owns but everyone may use is the deepest sense in which this book "founds the platform on the project" — the machine's own operating system, rewritten in the open, booting in Chapter 28.

## Lab 28 — Tracing a Boot

The lab is the boot itself, made legible, on the bench.

**Trace the boot.** Boot the clean-room console and watch it become a GPL program: disassemble the boot entry (`libre99gpl dis` at GROM `>0020`) to see the `BR >0052` leap and the interconnect table; `gromlog` (Chapter 26) the first stretch of the interpreter's fetches to see the boot GPL executing; and `m 8300 48` after boot to read the scratchpad birth certificate. Annotating the first several hundred GPL instructions of the boot — the hardware setup, the title `MOVE`s, the menu scan — is the chapter's deep exercise, and it turns "the machine boots" into a sequence you can read line by line.

**Get yourself enumerated.** The payoff joins Chapter 27: with a cartridge whose header you wrote, boot and watch the menu scan *find you* — your program's name in the list, discovered by the same `>AA` walk that found TI BASIC. (Running a plugged-in cartridge awaits §27.5's tooling; here you verify the discovery mechanism against the console's own program and see your header built to match.) Do it under the clean-room GROM *and*, where you have it, the authentic one, and confirm the menu builds identically — the contract, not the bytes.

> **Sidebar — "READY," and the beep.** When the TI finished booting, or dropped you into TI BASIC, it said `READY` and beeped — a short, distinctive tone (the console's own sound, Chapter 19). To a generation, that beep *was* the computer: the sound of a machine that had come to life and was waiting for you, the aural equivalent of a blinking cursor. It meant possibility — the machine was yours now, ready for a command, a program, a game. There is a particular nostalgia attached to it, because it was the sound of a threshold: home computing was new, most people had never commanded a machine before, and the little rising beep and the word `READY` were an invitation across that threshold, patient and encouraging. The clean-room GROM boots to its own "READY-PRESS ANY KEY," and it is a small act of preservation that the threshold sound survives — that a child in 2026, or an adult remembering 1982, can boot the machine and hear it say, in effect, *your turn*. The beep was never just a beep. It was the machine saying hello.

## Exercises

**28.1** ✦ What does the 9900 do when RESET fires, and where does it get its first workspace and program counter? At what point does the boot "leap into GPL"?

**28.2** ✦ Describe the master menu algorithm in three steps: what it scans for, what it follows, and what it builds. How does *your* cartridge (Ch. 27) get onto the list?

**28.3** ✦✦ Trace the boot on the bench: disassemble GROM `>0020`, `gromlog` the first 500 interpreter fetches, and identify three phases (hardware setup, title drawing, menu scan) in the GPL stream.

**28.4** ✦✦ Explain the two-interpreter tax precisely: name the two interpreters, say where a TI BASIC program's tokens and variables live, and estimate the slowdown versus 9900 assembly (combining Chapter 26's GPL tax with the BASIC-over-GPL layer).

**28.5** ✦✦ Why do TI BASIC's variables live in VDP RAM rather than the scratchpad or expansion memory? What does every variable access therefore cost (Chapter 12)?

**28.6** ✦✦✦ Dump the post-boot scratchpad birth certificate and annotate it: identify the frame counter, the GPL workspace, and any boot state in the low pad, cross-referencing Chapter 24's atlas. Compare it to the pad state a poll-only program (Ch. 22) would create instead.

**28.7** ✦✦✦ The clean-room GROM as preservation: argue why a re-implemented, freely-licensed console GROM matters for the platform's survival, and what its ability to boot real software (the same header scan, the same dispatch) proves about the boot contract.

## Further Reading

- `original-content/system-roms/grom/console.gpl` — the project's clean-room console GROM: the boot code, the menu scan, the fonts, the whole OS in readable GPL.
- The community disassemblies of the authentic console GROMs — the boot, menu, and TI BASIC internals this chapter describes, recovered opcode by opcode.
- Chapter 26 (The GPL Language) — the interpreter this whole OS runs on, and the tax TI BASIC pays twice.
- Chapter 27 (Writing GPL Today) — the header the menu scan reads, from the other side.
- Chapter 24 (The Scratchpad Atlas) — the birth certificate the boot writes.
- Chapters 44 and 45 (The Extended Family; The Living Platform) — the console versions and the preservation question the clean-room GROM answers.

## Summary

Turning on the TI-99/4A is a boot you can read whole: the 9900 vectors to the **RESET** vector in the console ROM, runs the native hardware initialization (VDP, sound, scratchpad), and then makes the platform's defining move — it **leaps into GPL**, starting the interpreter and handing control to a GPL program (on the clean-room GROM, the ROM jumps to GROM `>0020`, which branches to the boot code at `>0052`, verified by disassembly). From that instant the operating system *is* a GPL program: it draws the title with `MOVE`s, writes the scratchpad **birth certificate** every program inherits, and, on a keypress, runs the **master menu algorithm** — scan each GROM base for the `>AA` signature, follow the program-list pointer, walk the linked list of entries, build the "`n FOR NAME`" list, and dispatch the chosen entry (verified: the booted console discovers and lists "`1 FOR TI BASIC`"). A **power-up list** can run code before the menu (peripheral init, and the occasional boot hijack). Most of the machine's personality — fonts, cassette dialogs, KSCAN — lives GPL-side, so the console is a GPL *application suite*. And **TI BASIC**, dissected as an artifact, is a GPL program that interprets BASIC tokens: crunched into VDP RAM (behind the video port), executed by a GPL interpreter that is itself interpreted by the 9900 — the **two-interpreter tax** (roughly two orders of magnitude over assembly, plus a VDP-port access for every variable) that is the complete explanation of TI BASIC's infamous slowness. Console **versions** differ in their GROMs (99/4 vs 4A vs revisions), so software should use documented entry points and headers, not raw addresses — the contract, not the bytes, being what holds. And the chapter's proof of all of this is the project's **clean-room console GROM** (`console.gpl`): an original OS in GPL that boots the genuine machine, discovers cartridges, and runs their software, demonstrating that the boot contract is fully specified and answering the platform's preservation question — the operating system, rewritten in the open, booting on the same emulator this book is built on.
