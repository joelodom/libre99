# Chapter 26 — The GPL Language

*The TI has a second processor that does not exist — a virtual machine, running in ROM, whose bytecode is the language TI wrote its operating system and its flagship software in. Meet the machine's other mind.*

<!-- Part VI — GROM, GPL, and the Operating System · target ≈26 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. GPL toolchain and execution machine-verified on BENCH99 at commit 9aeb723: a small GPL program (demo.gpl: ALL/ST/MOVE/RTN) round-trips through libre99gpl asm->dis byte-exact; libre99gpl dis disassembles real console GPL (ST/XML/DECT/RTN at >018F, MOVE/ALL from console.gpl); and the new `gromlog` bench command traces the running interpreter fetching GPL bytecode from GROM (>018F-01A6 while the booted menu runs). Code in code/ch26/ (demo.gpl). GPL is fully executed by the emulator's genuine GPL interpreter over grom.rs; per-opcode cycle timing is a project non-goal (rom_perf.rs targets frame parity). -->

## The Processor That Isn't There

The TI-99/4A has one processor, the TMS9900. You know this; you have programmed it for two dozen chapters. And yet the machine also has a *second* processor — one with its own instruction set, its own registers, its own program counter — that does not physically exist. It is a **virtual machine**: a program, written in 9900 machine code and living in the console ROM, that fetches instructions from GROM, decodes them, and executes them. Its instruction set is **GPL** — Graphics Programming Language — and it is the language TI wrote the machine's operating system in, wrote TI BASIC in, wrote most of its cartridges in. Half the software this machine ever ran was not 9900 code at all. It was GPL, interpreted, byte by byte, out of the strange serial memory of Chapter 25.

This is a genuinely strange thing to find in a home computer of 1979, and it is the key that unlocks the rest of the platform. Why would TI build a fast 9900 and then run its software on a *slow interpreter* on top of it? For the same reasons it built the GROM: **density** (GPL bytecode is compact, and compact code fit in fewer of those expensive GROM bytes), **portability** (a bytecode could run on a planned family of machines with different CPUs — the 99/4A was meant to have siblings), and **economics** (writing high-level-ish code in GPL was faster than hand-assembling 9900, and the interpreter amortized across everything). TI bet that most of a console's software — menus, text, screen layout, game logic that runs a few times a second — did not need raw speed, and could be written smaller and faster in an interpreted language whose "machine" was tailored to exactly what a TI program does: paint the VDP, read the keyboard, move blocks of data between the console's several memories. For that workload, GPL was brilliant. For an inner loop at 60 Hz, it was fatal — and knowing which is which is the judgment this chapter builds.

We are not merely reading about GPL. The project's `libre99gpl` toolchain *assembles* it, its `dis` *disassembles* it, and the emulator's genuine interpreter *runs* it — so this chapter's GPL is written, round-tripped, and watched executing, byte by byte, on the bench. GPL stops being folklore and becomes a language you can hold.

---

## What You Will Learn

- What GPL is: a bytecode language and an interpreter (in ROM) whose "machine" is the whole console — GROM for code, VDP for screen and data, scratchpad for variables.
- The **execution model**: the interpreter's fetch loop (watched live), the GPL workspace, the status byte, and the honest cost per instruction.
- **Addressing** in GPL: how an operand names a place in one of the console's several memories — immediate, GROM, CPU RAM, VDP.
- The instruction set in three tours: **data and arithmetic** (`MOVE`, the block-mover; loads, stores, math), **flow** (branches, `CASE`, `CALL`/`RTN`), and the **console-flavored** ops (`FMT`, `ALL`, `BACK`, `SCAN`, `RAND`).
- How to **read real GPL** — TI's own conventions, and the same routine in assembly, GPL, and pseudo-code.
- GPL's costs and sweet spots, honestly: superb for menus and text, hopeless for inner loops.

## The Bridge: A Bytecode VM, Then and Now

Interpreted bytecode is the water a modern programmer swims in. Java compiles to JVM bytecode, run by a virtual machine; C# to CIL; Python to its own bytecode; JavaScript is JIT-compiled from an AST. In every case the idea is identical: define an abstract "machine" with an instruction set suited to your language, compile programs to that machine's bytecode, and run the bytecode with an interpreter (or JIT) written in native code. The reasons are the modern reasons — portability across hardware, compact code, safety, developer speed — and the cost is the modern cost: interpreted bytecode runs slower than native, which is why hot paths get JIT-compiled or written in native extensions.

GPL is a bytecode VM from 1979, and it is the *same idea*, thirty years before it was everywhere — with the same benefits (compact GROM-friendly code, a planned family's portability, faster development than raw assembly) and the same limitation (it is an interpreter, so it is slow, and the hot paths must drop to native 9900 code — Chapter 29's hybrid). Studying GPL is studying a bytecode VM stripped to its essentials, small enough to hold whole: an instruction set of a few dozen opcodes, an interpreter you can single-step, a program counter that is just a GROM address. Every intuition you have about the JVM or CPython applies here in miniature — the fetch-decode-execute loop, the "native extension for the hot loop," the "the VM's abstractions match the problem domain" — and here you can watch all of it happen on a machine you understand completely. GPL is the concept made small enough to see.

## 26.1 What GPL Is

GPL is a bytecode language whose interpreter is a 9900 program in the console ROM, and whose "machine" is *the whole console*. This last point is what makes GPL not a generic VM but a *console-specific* one, and it is the source of both its power and its oddity. GPL's instructions do not operate on a private memory the way a normal CPU's do; they operate directly on the console's real hardware and memories:

- **Code** lives in **GROM** — the interpreter's program counter is a GROM address (Chapter 25), and it fetches each instruction byte by streaming from GROM.
- **The screen and much data** live in **VDP RAM** — GPL instructions read and write VRAM directly, so painting the screen is a native operation, not a library call.
- **Variables** live in the **scratchpad** — the fast CPU RAM of Chapter 24, which GPL addresses as its working memory.

So a GPL program is code-in-GROM manipulating data-in-VDP-and-scratchpad, and its instruction set is shaped around exactly that: move blocks between those memories, format the screen, scan the keyboard, do a little arithmetic. TI wrote the operating system, the master menu, TI BASIC, and most first-party cartridges in this language, because for that kind of software — mostly screen and control flow, running at human speed — GPL is expressive, compact, and quick to write. The 9900 underneath is there for when GPL is not enough (Chapter 29), but the *default* software layer of the TI-99/4A is GPL, and the console you boot is a GPL program running on an interpreter written in 9900 code. Understanding the machine means understanding its other mind.

## 26.2 The Execution Model, Watched

The interpreter is a loop: fetch the next GPL instruction byte from GROM (advancing the GROM program counter), decode it, execute the corresponding 9900 code, repeat. We can *watch* this happen. Boot the console — whose menu is a GPL program — enable BENCH99's new `gromlog`, step the 9900, and the log shows the interpreter's GROM fetches: the actual GPL bytecode it is consuming.

```text
bench: boot; gromlog on; x 300; gromlog
  -> grom fetches: 19   >018F:BF >0190:72 >0191:00 >0192:80 >0193:BF ...
```

Those bytes *are* a GPL program, and `libre99gpl dis` decodes the same GROM region into the instructions the interpreter was running:

```text
>018F  ST     @>8372, >0080     store an immediate into a scratchpad variable
>0193  ST     *@>8372, >01A2    indirect store
>0198  XML    >1A               call a 9900 machine-code routine
>019A  ST     *@>8373, *@>8372
>019F  DECT   @>8372
>01A1  RTN                       return
```

This is the whole act, both sides visible: the `s` trace shows the *9900 instructions of the interpreter* grinding through its fetch-decode loop, and `gromlog` shows the *GPL bytes* it is grinding on. The GPL machine has state, too: a **GPL workspace** at `>83E0` (Chapter 24) holds the interpreter's virtual registers — including, in its high registers, the GPL program counter (a GROM address) and data-stack pointer — and a **status byte** at `>837C` carries condition flags that GPL's branches test. When you single-step and see the 9900 touching `>83E0` and reading GROM, you are watching the VM's registers and program counter being manipulated by its interpreter.

And here is the honest cost. Every GPL instruction is *many* 9900 instructions — the fetch, the decode (a dispatch through a jump table), the execution — plus one or more **GROM reads**, and GROM is slow serial memory (Chapter 25). So a GPL instruction costs, very roughly, an order of magnitude more than the 9900 instruction it most resembles: an interpreter's tax, paid per opcode, dominated by the fetch-decode overhead and the serial GROM access. The project's emulator targets *frame-level* parity (the clean-room GROM boots no slower than authentic) rather than cycle-exact GPL timing, so a precise per-opcode cycle count is not a number this book pins to a decimal; but the shape is not in doubt and the trace shows it plainly — a handful of GPL instructions is hundreds of 9900 steps. GPL is slow, by construction, and §26.8 is the judgment that follows.

## 26.3 Addressing: Naming a Place in Several Memories

Because GPL operates across the console's several memories, an operand must say *which* memory it means, and GPL's addressing is a small set of space-tags. Our `demo.gpl` and the console's own code show them:

| Form | Means | Example |
|---|---|---|
| `>NN` | **immediate** — a literal value | `ST V@>0000, >48` (store the value `>48`) |
| `V@addr` | **VDP RAM** at `addr` | `V@>0000` (screen cell 0) |
| `@addr` | **CPU RAM** (scratchpad) at `addr` | `@>8372` (a variable) |
| `G@label` | **GROM** at `label` | `G@MSG` (data in GROM) |
| `*@addr` | **indirect** — the address is *in* `@addr` | `*@>8372` (follow the pointer) |

So `MOVE >0005, G@MSG, V@>0002` reads "move 5 bytes from GROM label `MSG` to VDP address `>0002`," and `ST *@>8372, >01A2` reads "store `>01A2` through the pointer in scratchpad `>8372`." The space-tag on each operand is the essence of GPL: an instruction can name a source in GROM, a destination in VDP, and a count as an immediate, all in one line, because the interpreter knows how to reach each memory. There are indexed and register-relative flavors too (the full operand-byte encodings are App. B), but the core idea is this handful of tags, and once you can read them, GPL listings stop being cryptic — every operand tells you which of the machine's memories it lives in.

## 26.4 Instruction Tour I: Data and the Mighty `MOVE`

GPL's data instructions are led by one that is genuinely its superpower: **`MOVE`**, the block-mover *between any two of the console's memories*. GROM to VDP, VDP to scratchpad, scratchpad to VDP, GROM to CPU RAM — one instruction, any source space to any destination space, with a byte count. The console's own startup is a cascade of `MOVE`s (from `console.gpl`):

```text
MOVE >0200,G@FONT,V@>0900     the character set from GROM into the VDP pattern table
MOVE >0011,G@TITLE1,V@>0107   "TEXAS INSTRUMENTS" from GROM onto the screen
```

Think about what that single instruction does: it reads 0x200 bytes streaming out of GROM (Chapter 25's auto-increment) and writes them streaming into VDP RAM (Chapter 12's auto-increment port), bridging two serial memories in one opcode. In 9900 assembly this is a loop with two port protocols (our `VMBW` plus a GROM read); in GPL it is *one instruction*, because the interpreter's whole reason for being is to make the console's cross-memory data shuffling trivial. `MOVE` alone justifies GPL for the kind of software TI wrote: a program that is mostly "put this GROM data on the screen" is mostly `MOVE`s, and each is a line.

Around `MOVE` are the ordinary data and arithmetic ops: **`ST`** (store a value or a copy — our `ST V@>0000, >48` pokes a character to the screen), loads and stores in the several spaces, `ADD`/`SUB`/`MUL`/`DIV`/`INC`/`DEC` (our disassembly's `DECT @>8372` decrements a variable by two), the logic operations, and the compares that set the status byte for the branches of §26.5. They are unremarkable individually — a small integer instruction set — but each operates across the space-tagged memories, so `INC V@>0300` increments a byte *in video RAM* directly, no port dance in your code. The arithmetic is not GPL's strength (it is an interpreter; §23.5's fixed-point lives in 9900 code), but for the counter-and-pointer bookkeeping of screen software it is exactly enough.

## 26.5 Instruction Tour II: Flow

GPL's control flow is built on the **status byte** (`>837C`): compares and operations set its condition bits, and branches test them. The core branches are **`B`** (unconditional branch to a GROM address), **`BR`** (a short relative branch), and **`BS`**/**`BR`** in their condition-testing forms (**branch on status set / reset** — the `if` of GPL, jumping when the last operation set or cleared a condition). Our very first console disassembly (Chapter 25's) was a table of them — `BR >0F5F`, `B >4D12`, `BS >000D` — the branch vectors of the interpreter's entry.

For structured dispatch GPL has **`CASE`** (and `DCASE`): a jump-table branch on a value, the `switch` of the language, which the menu uses to dispatch the selected program and TI BASIC uses to dispatch tokens. And for subroutines it has **`CALL`** and **`RTN`** (with **`FETCH`** to pull parameters) — `CALL` pushes the GPL program counter onto the GPL data stack and branches; `RTN` (which ended our `>018F` routine) pops it and returns; `EXIT` leaves a program entirely. So GPL has the full control-flow vocabulary — conditionals, switches, subroutines with a call stack — all operating on GROM addresses (a GPL "pointer" is a GROM address) and the status byte, and all interpreted. Reading GPL flow is reading branches-on-status and `CALL`/`RTN`, exactly as you would read assembly flow, but one level up: the interpreter maintains the call stack, so a GPL `CALL` is a subroutine call without the workspace-and-`BLWP` machinery of Chapter 9 — the VM handles it.

## 26.6 Instruction Tour III: The Console-Flavored Ops

Here GPL reveals that its "machine" really is the whole console, with instructions no general CPU would have. The crown jewel is **`FMT`** — a screen-layout *sub-language embedded in the bytecode*. `FMT` enters a formatting mode with its own little instruction set: place text at a row and column, repeat a character N times, draw runs, move the cursor — a domain-specific language for painting a screen, *inside* a GPL instruction. A menu or a game's title screen is often one `FMT` block, and it is astonishingly compact: the layout of a whole screen in a few dozen bytes of `FMT` sub-instructions, interpreted into VDP writes. It is the clearest evidence that GPL was designed for *this* machine's job — a whole DSL for screen layout, promoted to a first-class opcode.

Around `FMT` are the other console verbs. **`ALL`** fills the entire name table with one character (our `demo.gpl`'s `ALL >20` clears the screen to spaces — one instruction, a whole screen cleared). **`BACK`** sets the border colour. **`SCAN`** scans the keyboard (the KSCAN of Chapter 21, as a single GPL opcode, leaving the key in a known place). **`RAND`** produces a random number (from the entropy of Chapter 24's `>83C0`). And there are opcodes to dispatch a sound list (Chapter 19's format, played by the interpreter) and to reach the I/O and link mechanisms. Each is a console operation TI did so often it made it a single bytecode — and together they are why a GPL program that would be pages of 9900 assembly (set VDP mode, load font, clear screen, lay out text, scan input) is a handful of GPL lines. The instruction set *is* the console's job description.

## 26.7 Reading Real GPL: A Rosetta Stone

To read GPL fluently, see the same routine three ways. Take "clear the screen and print a word at the top" — our `demo.gpl`, which round-trips through `libre99gpl` byte-for-byte:

```text
GPL (what we wrote, and what dis gives back):
        ALL  >20                    ; clear the name table to spaces
        ST   V@>0000, >48           ; put 'H' at cell 0
        MOVE >0005, G@MSG, V@>0002  ; copy "HELLO" from GROM to cells 2..6
        RTN

9900 assembly (the same effect, from Part III):
        BL   @TXCLS                 ; clear (a loop over 768 cells)
        LI   R0,0 / LI R1,>4800 / BL @VSBW      ; 'H' to VRAM 0
        LI   R0,2 / LI R1,MSG / LI R2,5 / BL @VMBW   ; "HELLO" to VRAM 2
        RT

pseudo-code:
        clear_screen(' ')
        screen[0] = 'H'
        copy(screen[2..7], MSG, 5)
```

The GPL is the *shortest* of the three — four instructions — because each GPL opcode (`ALL`, `ST` with a VDP operand, `MOVE` across spaces) does what a whole assembly sequence does, and the interpreter supplies the loops and the port protocols. That compactness is GPL's case, and reading it is a matter of the space-tags (§26.3) and the console verbs (§26.6): once `ALL` means "fill the screen," `V@` means "in video RAM," and `MOVE` means "block-copy between spaces," a GPL listing reads as directly as the pseudo-code. TI's own code (the `console.gpl` this book's clean-room GROM is built from) reads exactly this way — cascades of `MOVE`s to lay out a screen, `FMT` blocks for structured layout, `CASE` to dispatch, `SCAN` to read keys — and the disassembler turns any cartridge's GROM back into it.

## 26.8 Costs and Sweet Spots, Measured

The judgment, now, with the cost of §26.2 in hand. GPL is an interpreter, so every instruction pays the fetch-decode-and-GROM-read tax — roughly an order of magnitude over equivalent 9900 code. That fact sorts every use of GPL into a sweet spot or a trap.

**GPL is superb for:** menus, user interface, text layout, attract screens, data-driven control flow, and any logic that runs at *human* speed (a few times a second, or once per keypress). Here the interpreter tax is invisible — a menu that redraws in "instant" human terms can afford to be ten times slower than assembly and no one notices — and GPL's compactness and console verbs (`FMT`, `MOVE`, `SCAN`) make the code dramatically shorter and faster to write. The whole master menu, the whole title screen, TI BASIC's editor: all GPL, all fine, because none of it is hot.

**GPL is hopeless for:** inner game loops, per-frame motion, collision detection, sound service, anything that must run *dozens or hundreds of times per frame* inside the 50,000-cycle budget (Chapter 17). Here the order-of-magnitude tax is fatal — a physics loop that fits in a frame as 9900 code blows the frame ten times over as GPL — which is exactly why TI's own action games are **hybrids** (Chapter 29): GPL for the shell and the menu, 9900 machine code (reached via `XML`, which we saw the console GPL itself call at `>0198`) for the parts that run hot. The rule is clean and it is the folklore made precise: **GPL for what runs at human speed, 9900 for what runs at machine speed.** The numbers behind the folklore are the order-of-magnitude interpreter tax, and the design that follows is the hybrid architecture the next chapters build.

## Lab 26 — Watching the Interpreter Interpret

The lab is GPL made concrete, in `code/ch26/`.

**`demo.gpl`** — a tiny GPL program (clear, poke a character, `MOVE` a string, return) that round-trips through the toolchain:

```sh
libre99gpl asm code/ch26/demo.gpl build/demo.bin
libre99gpl dis build/demo.bin 2000
```

The disassembly gives back `ALL >20 / ST V@>0000, >48 / MOVE >0005, V@>0002, … / RTN` — the instructions you wrote, proving `libre99gpl` assembles and disassembles GPL faithfully. This is where GPL becomes a language you *use*, not just read.

**Watching the interpreter.** The chapter's headline act: boot the console (a GPL program), `gromlog on`, `s` a few instructions, and watch — the `s` trace shows the interpreter's 9900 fetch-decode loop, and `gromlog` shows the GPL bytecode it is consuming from GROM (`>018F:BF >0190:72 …`), which `libre99gpl dis` decodes into the very instructions (`ST`, `XML`, `DECT`, `RTN`) being run. Both sides of the interpretation, traced on the bench — the moment GPL stops being magic and becomes a fetch loop over bytes you can read. Chapter 27 puts *your* GPL into writable GROM (GRAM) and runs it the same way; here you watch TI's.

> **Sidebar — The language TI never sold.** TI never published a GPL assembler or a GPL programmer's manual for the public. GPL was internal — the language TI's own engineers wrote the console in — and outsiders were meant to use TI BASIC, Extended BASIC, or the Editor/Assembler's 9900, never GPL itself. But the language was *right there*, interpreted out of GROMs anyone could read, and the community reverse-engineered it: leaked internal manuals surfaced, dedicated hobbyists disassembled the console ROMs and reconstructed the instruction set opcode by opcode, and eventually fan-made GPL assemblers appeared — decades after TI, letting enthusiasts write in the language TI had kept to itself. This book's `libre99gpl` stands in that tradition: a modern, open GPL assembler and disassembler, and a clean-room console GROM written in GPL from scratch (Chapter 28) — the language TI never sold, finally documented, tooled, and free to use. That you can `libre99gpl asm` a GPL program in 2026 is the end of a forty-year act of collective reverse-engineering, and a small triumph of preservation over secrecy.

## Exercises

**26.1** ✦ In one sentence, what is GPL's "machine" — where does its code, its screen data, and its variables each live?

**26.2** ✦ Read this GPL: `MOVE >0020, G@BARS, V@>0000`. What does it do, and what would the equivalent take in 9900 assembly?

**26.3** ✦✦ Extend `demo.gpl`: clear the screen, then use `MOVE` to place two strings on different rows, and `ALL` to set a background fill. Round-trip it through `libre99gpl asm`/`dis` and confirm your instructions come back.

**26.4** ✦✦ Boot the console, `gromlog on`, and trace a stretch of the menu's GPL. Disassemble the fetched region with `libre99gpl dis` and identify three different instruction types the interpreter ran.

**26.5** ✦✦ Write the Rosetta stone for "increment a counter and branch if it reaches 10": the GPL, the 9900 assembly, and the pseudo-code, and explain which GPL status bit the branch tests.

**26.6** ✦✦✦ Estimate GPL's interpreter tax empirically: find a short pure-GPL loop in the console (no `XML`), count the 9900 instructions the interpreter executes per GPL instruction (with `s` and `gromlog`), and compare to the equivalent assembly. Report the ratio and where it comes from (fetch, decode, GROM read).

**26.7** ✦✦✦ Read a real cartridge's GPL: disassemble a region of a cartridge GROM with `libre99gpl dis`, identify a `FMT` block or a `CASE` dispatch, and explain what the original programmer was doing. (You are reading someone's 1982 source, recovered from the bytecode.)

## Further Reading

- The community-reconstructed GPL documentation and the `libre99gpl` toolchain sources — the instruction set, operand encodings (App. B), and the disassembler this chapter drives.
- `original-content/system-roms/grom/console.gpl` — the project's clean-room console GROM in GPL, a large, readable body of real GPL (Chapter 28 tours it).
- Chapter 25 (GROM) — the serial memory GPL's program counter walks, and the bytes the interpreter fetches.
- Chapter 23 (Console ROM Services) — GPLLNK, the 9900-to-GPL doorway, from the other side.
- Chapter 27 (Writing GPL Today) — putting your own GPL into GRAM and running it.
- Chapter 29 (Hybrid Architecture) — `XML`, the GPL-to-9900 call this chapter saw the console make, and the GPL-shell/9900-core pattern.

## Summary

The TI-99/4A runs on a second, virtual processor: **GPL**, a bytecode language interpreted by a 9900 program in the console ROM, in which TI wrote the operating system, the menu, TI BASIC, and most cartridges. GPL's "machine" is the whole console — **code in GROM** (the interpreter's program counter is a GROM address), **screen and data in VDP RAM**, **variables in the scratchpad** — chosen for GROM-friendly density, a planned family's portability, and development speed, at the cost of interpreter slowness. The execution model is a fetch-decode-execute loop, watchable on the bench: `gromlog` shows the interpreter fetching GPL bytecode from GROM (`>018F:BF …`), which `libre99gpl dis` decodes into the running instructions (`ST`, `XML`, `DECT`, `RTN`), with the GPL registers and program counter living in the `>83E0` workspace and the status byte at `>837C` — and every GPL instruction costs roughly an order of magnitude over its 9900 equivalent (fetch-decode plus serial GROM reads; the project targets frame parity, not cycle-exact GPL timing). Operands are **space-tagged** — immediate `>NN`, VDP `V@`, CPU RAM `@`, GROM `G@`, indirect `*@` — so one instruction spans several memories, and the instruction set is built for the console's job: **`MOVE`** (the block-mover between any two spaces, GPL's superpower), stores and arithmetic across the spaces, branches on the status byte with `CASE` and `CALL`/`RTN` for structured flow, and console verbs no general CPU has — **`FMT`** (a screen-layout DSL inside an opcode), **`ALL`**, **`BACK`**, **`SCAN`**, **`RAND`**. Real GPL reads as compactly as pseudo-code once you know the tags and verbs (our `demo.gpl` round-trips through `libre99gpl` byte-for-byte). And the judgment is precise: GPL is **superb for human-speed software** (menus, text, UI — the tax is invisible) and **hopeless for machine-speed inner loops** (the order-of-magnitude tax is fatal), which is why TI's action games are hybrids — GPL shell, 9900 core via `XML` — the architecture Chapters 27–29 build. This book's `libre99gpl` is a modern open tooling for a language TI never sold: GPL, at last, written, run, and read.
