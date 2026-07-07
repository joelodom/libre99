# Chapter 2 — Grand Tour: The Architecture at 10,000 Feet

*Every major component introduced once, honestly, before we zoom in for the rest of the book.*

<!-- Part I · target 18 pp · prerequisites: Ch. 1 · Lab: watch the multiplexer on the lab bench -->

## Prologue: The Fold-Out

Picture a repair bench in 1983 — an authorized TI service center, a dead silver console open on the anti-static mat, and taped to the wall above it a large fold-out sheet from the service documentation: the system block diagram. A dozen rectangles, a web of lines. The tech doesn't reach for the oscilloscope first; she reads the wall. No video at all? Then the fault sits somewhere on the two-box island at the right — the video processor and its own private RAM — because *nothing else in the machine can even see the screen*. Title screen fine but every cartridge dead? Follow the lines to the pair of boxes the cartridge port shares with the console's GROMs. Machine boots but no sound? One box, one line, five minutes.

The fold-out worked as a diagnostic instrument because this machine, like every machine of its era, has an architecture a person can actually hold in their head: a small cast of chips, each with one job, connected by buses you can point at. That is a luxury modern hardware no longer offers and modern programmers rarely experience — and it is the working method of this entire book. Before we learn a single instruction, we are going to earn the fold-out: every box, every line, and the one line in particular — an 8-bit funnel between a 16-bit processor and nearly everything it needs — that explains this platform's whole personality.

By the end of the book you will have zoomed into every rectangle on the sheet at silicon-manual depth. Today we walk the wall.

---

## What You Will Learn

After this chapter you can:

- Name the machine's cast of chips — TMS9900, TMS9901, TMS9918A, TMS9919/SN94624, the console ROM, the three console GROMs — and state each one's job and its size in one sentence.
- Draw the console block diagram from memory, including the 16-bit fast domain (console ROM + 256-byte scratchpad) and the 8-bit multiplexed domain (everything else), and state the nominal cost of crossing between them.
- Explain where the memory actually is — why a "16 K home computer" gives its CPU only 256 bytes, and what living out of video RAM meant for every program on the platform.
- Describe the tower of interpreters (BASIC over GPL over 9900) and use it to explain, architecturally, why TI BASIC was slow — and which floors of the tower this book teaches you to occupy.
- List the modern conveniences this machine does not have (OS, scheduler, protection, heap, file abstraction, hardware stack) and articulate why their absence is a feature for learning.
- Apply the 1982 commercial-quality checklist (introduced here, used through Part IX) to any piece of software, vintage or new.
- Locate every remaining part of this book on the block diagram.

## The Bridge: Read the Die Shot Before the ISA Manual

A modern computer resists being understood top-down: the block diagram of a laptop SoC is a city map with a thousand districts, most of them undocumented, and no working programmer is expected to know it. So we learn modern systems from the *inside of an abstraction* outward — a language, an API, a framework — and trust the map to whoever drew it. You have probably never needed to know which bus your code's bytes traveled.

Here the situation inverts, and your habits must invert with it. This machine has no abstractions you didn't build; its map has about ten districts; and the performance and even the *correctness* of your code will depend on geography — which chip owns the byte you want, which bus it crosses, what the crossing costs. So we adopt the hardware engineer's reading order: floor plan first, instruction set second. If you have ever sketched a microservices diagram before diving into a codebase — boxes for services, arrows for the calls between them, a highlighter for the one slow RPC everything funnels through — you already know today's method. This chapter is that diagram for a 1981 computer, and yes, there is one slow RPC. We will highlight it in §2.2, measure it in the lab, and spend Chapter 37 exploiting everything about it.

## 2.1 The Cast of Chips

Six named parts, plus two memories, make the whole console. Meet them once, plainly; each gets its own deep chapter later.

**TMS9900 — the CPU (Ch. 4–11).** The 16-bit minicomputer-on-a-chip of §1.2, clocked at 3.0 MHz in the console. Its defining trait: almost no registers on the die. The sixteen general registers R0–R15 are a 32-byte window into ordinary RAM located by a hardware Workspace Pointer, so "register access" is memory access, and the speed of your registers is the speed of whatever RAM you parked them in. Hold that thought for two paragraphs.

**TMS9901 — the Programmable Systems Interface (Ch. 21–22).** One chip, three jobs: it funnels the machine's interrupt sources down to the CPU, it provides the I/O pins that scan the keyboard matrix and read the joysticks and drive the cassette lines, and it contains a programmable timer. The CPU talks to it not through the memory bus but through a separate serial fabric called the CRU (Ch. 10) — the first hint that this machine has *three* distinct ways for components to converse: memory bus, CRU bits, and interrupt lines.

**TMS9918A — the Video Display Processor (Part III).** The most influential chip in the box, and the one with a private life: the 9918A owns its **own 16 K of DRAM**, on its own bus, refreshed and scanned by the VDP itself sixty times a second to generate the picture. The CPU cannot address that RAM at all. It can only pass notes through a mail slot — a pair of memory-mapped ports, one byte at a time (§2.3). The 9918A composes its display from tables the programmer writes into that private RAM — character patterns, a screen full of name bytes, color tables, and up to 32 hardware **sprites** — and raises an interrupt at every frame. In PAL territories the console carried the 9929A sibling; same brain, different television (frame-rate consequences are flagged where they matter, per this book's PAL policy).

**TMS9919 / SN94624 — the sound generator (Ch. 19).** Three square-wave tone channels plus one noise channel, programmed by writing command bytes to a single memory-mapped address. TI's part number for a design the wider world knows by its Texas Instruments commercial designation SN76489 — the same voice, near enough, as the ColecoVision, the Sega SG-1000, and the BBC Micro's rhythm section. Cheap, characterful, and capable of far more than its datasheet admits.

**Console ROM — 8 K bytes.** The machine's only true 9900-native firmware: the reset and interrupt vectors, the interrupt service routine, utility routines, a floating-point package (Ch. 23) — and, above all, the **GPL interpreter**, the program whose job is to run the *real* operating system. Critically, this ROM sits on the CPU's full 16-bit bus with zero wait states: it is fast memory, and TI's engineers wrote the interpreter's inner loop there for exactly that reason.

**Console GROMs — three chips, 6 K bytes each.** The operating system's actual home: the power-up code, the master menu, the character sets, KSCAN's personality, and all of TI BASIC live here as GPL bytecode — 18 K of it — readable only serially through the port protocol of §1.4 (mechanism in Ch. 25). The cartridge slot extends this space: plug in a Command Module and its GROMs join the family.

**Scratchpad RAM — 256 bytes, really.** At addresses `>8300`–`>83FF` sits the console's entire complement of CPU-addressable read/write memory: 256 bytes of static RAM on the full 16-bit bus, zero wait states — the fast island of §1.3. The operating system keeps its variables here, the GPL interpreter keeps its virtual machine here, interrupt handlers keep their workspaces here, and your programs will fight for the leftovers (Ch. 24 is the treaty).

**VDP RAM — 16 K bytes, behind the mail slot.** Counted on the box, owned by the video chip, addressable by the CPU only through ports. It is the machine's *data* warehouse whether you like it or not.

That is the whole console: a CPU, an I/O-and-interrupt chip, a video computer with private memory, a sound chip, 8 K of fast firmware, 18 K of serially-read bytecode, a quarter-kilobyte of fast RAM, and a television. Optional extras — the 32 K expansion, disk systems, speech — dock on from outside (§2.2). Everything this platform ever did was done with this cast.
## 2.2 The Block Diagram Walk: Who Talks to Whom

Here is the fold-out, redrawn for this book. Commit it to memory — every later chapter assumes you can see it with your eyes closed.

```text
                    THE TI-99/4A CONSOLE — SYSTEM BLOCK DIAGRAM
                    =========================================== 

  16-BIT DOMAIN (full bus, no wait states)
  +----------------------+        +----------------------------+
  |  Console ROM (8K)    |        |  Scratchpad RAM (256 B)    |
  |  vectors, ISR, GPL   |        |  >8300–>83FF               |
  |  interpreter, FP pkg |        |  OS state, workspaces      |
  +----------+-----------+        +-------------+--------------+
             |       16-bit data bus (D0–D15)   |
  ===========+=============+====================+==============
                           |
                    +------+-------+   TMS9900 CPU @ 3.0 MHz
                    |   TMS9900    |   WP/PC/ST on chip;
                    |              |   R0–R15 live in RAM
                    +--+-------+---+
      CRU serial bits  |       |  INTREQ (interrupts)
       +---------------+       +----------------+
       |                                        |
  +----+------+     8-BIT DOMAIN          +-----+-----+
  |  TMS9901  |  (via multiplexer:        | interrupt |
  |  PSI      |   ~4 extra cycles         | sources:  |
  |  ---------|   per 16-bit word)        | VDP frame,|
  |  keyboard |                           | 9901 timer|
  |  joysticks|   +------------------+    | ext. cards|
  |  cassette |   |   MULTIPLEXER    |    +-----------+
  |  timer    |   | 16 <—> 8 bits    |
  +-----------+   +---+----------+---+
                      |          |
        +-------------+--+    +--+------------------------------+
        | MEMORY-MAPPED  |    | 8-BIT MEMORY                    |
        | I/O PORTS      |    |  cartridge ROM  >6000–>7FFF     |
        |  sound  >8400  |    |  32K expansion  >2000–>3FFF     |
        |  VDP  >8800/.. |    |     (external)  >A000–>FFFF     |
        |  speech >9000/.|    |  DSR window     >4000–>5FFF     |
        |  GROM  >9800/..|    |     (external)                  |
        +---+------------+    +---------------------------------+
            |
   +--------+---------+      +---------------------+
   | TMS9918A VDP     +------+  VDP RAM (16K)      |   TMS9919/SN94624
   | own bus, own     |      |  private: CPU sees  |   sound: 3 tones
   | clock; frame IRQ |      |  it only via ports  |   + noise, via >8400
   +---------+--------+      +---------------------+
             |
        television

   Side port (right edge of console) —> speech synthesizer,
   Peripheral Expansion Box: 32K card, disk, RS232... (Part VII)
```

Walk it the way the bench tech did. The CPU sits at the center with three kinds of wiring leaving it. **The memory bus** carries addresses and data; notice that it forks. Up and left, at full 16-bit width and full speed: the console ROM and the scratchpad — the *fast domain*, exactly 8 K + 256 bytes, chosen by TI because the GPL interpreter's code lives in the first and its variables in the second. Down and right, the bus squeezes through the **multiplexer**, and every other memory-like thing in the machine — cartridge ROM, the expansion RAM, peripheral card windows, and all the memory-mapped I/O ports — lives on the far side, eight bits wide. The multiplexer does its job invisibly and honestly: a 16-bit word becomes two 8-bit transfers plus the bookkeeping between them, at a nominal cost of about **four extra clock cycles per word** touched. Invisible, honest — and *universal*: on the far side of that funnel, code fetches, data reads, and I/O all pay. (The lab makes you watch it happen; Chapter 5 measures it to the cycle and refines "about four" into exact figures per device.)

**The CRU** is the second wiring — a serial, bit-addressed I/O fabric completely separate from the memory map, over which the CPU sets, clears, and tests individual control bits (Ch. 10). In the console its main citizen is the TMS9901; out on the expansion bus, every peripheral card answers to CRU bits too — it is how cards are switched on and off (Ch. 30). **The interrupt lines** are the third: the 9918A raises one every video frame, the 9901's timer and external cards can raise others, and the 9901 funnels them to the CPU's single effective level (Ch. 22).

Two boxes deserve a second look because they are *computers of their own*. The 9918A runs continuously on its own clock against its own RAM; the CPU's relationship to the entire display is writing bytes through a port when the VDP isn't looking (Part III is largely the art of that sentence). And the GROM boxes, remember, are not on the address bus at all in the normal sense — they are port devices with internal counters, a memory you *ask* rather than *address* (Ch. 25).

Finally, the edges. The cartridge port (top) brings in ROM at `>6000` and/or GROMs onto the GROM ports — both flavors, which is why Chapter 35 and Chapter 27 describe two different native cartridge formats. The side port (right) is simply the 8-bit bus, CRU, and interrupt lines brought out on a connector: the speech synthesizer hangs there, and the Peripheral Expansion Box is, electrically, a very large side-port device (Part VII). Nothing that ever plugged into this machine — 1981 or 2026 — is anything but boxes and lines you have now seen.

## 2.3 Where the Memory *Is*

Now the fact that shapes every program ever written for this machine. The box said "16 K." The block diagram tells the truth: the CPU's writable world is **256 bytes**, and the 16 K belongs to the video chip, on the wrong side of a mail slot.

Look at the address space the CPU actually sees — the coarse map (Chapter 5 furnishes the surveyor's version, and Appendix C the poster):

| CPU address range | What lives there | Domain |
|---|---|---|
| `>0000`–`>1FFF` | Console ROM (8 K) | **16-bit, fast** |
| `>2000`–`>3FFF` | Expansion RAM, low 8 K *(empty on bare console)* | 8-bit |
| `>4000`–`>5FFF` | Peripheral DSR window *(cards take turns here — Ch. 30)* | 8-bit |
| `>6000`–`>7FFF` | Cartridge ROM *(empty without a ROM cart)* | 8-bit |
| `>8300`–`>83FF` | **Scratchpad RAM — the 256 bytes** | **16-bit, fast** |
| `>8400` | Sound chip (write) | 8-bit MMIO |
| `>8800` / `>8802` | VDP read data / read status | 8-bit MMIO |
| `>8C00` / `>8C02` | VDP write data / write address | 8-bit MMIO |
| `>9000` / `>9400` | Speech read / write *(if attached)* | 8-bit MMIO |
| `>9800`–`>9C02` | GROM read data / read addr / write addr ports | 8-bit MMIO |
| `>A000`–`>FFFF` | Expansion RAM, high 24 K *(empty on bare console)* | 8-bit |

Read the right-hand column and count the read/write memory a bare 1981 console offers its CPU: one line. A quarter of a kilobyte. The 32 K expansion — a peripheral, remember, that cost more than the late-war console (§1.6's price sheet) — fills the `>2000` and `>A000` gaps and is what this book's "standard system" assumes from Part II onward; but *the platform's installed base was mostly bare consoles*, TI's own cartridge software was written for bare consoles, and Chapter 40's capstone honors the constraint on purpose.

So where did a bare-console program keep its data? Behind the mail slot. The VDP's 16 K is real RAM — random-access, reliable, merely *inconvenient*: to read or write a byte, the CPU performs a little ritual at the ports (set the 14-bit VDP address by writing two bytes to `>8C02`, then move data bytes through `>8800` or `>8C00`, one at a time, the VDP's internal address auto-incrementing as you go — Ch. 12 makes this second nature). Everything followed from that arrangement. TI BASIC keeps your *program text and variables* in VDP RAM — behind the slot — which is one of the three floors of §2.4's slowness tower. Cartridge games keep level data, tables, even rarely-run code parked in VRAM and fetch it as needed; Chapter 36 elevates the trick to doctrine ("VRAM as data warehouse"). And the whole platform's software culture — small hot state in the pad, bulk data behind the ports, code in ROM — is nothing but this table, internalized.

One more honest note before we climb the tower. The two "fast" rows plus the workspace scheme from §2.1 combine into the platform's core performance law, stated here once and proven in Chapter 5: **on this machine, speed is a property of addresses, not of instructions.** The same `MOV` costs different amounts depending on where its opcode, its source, and its destination live. Registers in scratchpad: fast. The identical registers relocated to expansion RAM: every register touch pays the funnel. TI put the OS's workspaces in the pad; so will you; and when Chapter 37 makes 3 MHz feel fast, moving things will beat rewriting things ten times out of ten.
## 2.4 The Tower of Interpreters

Chapter 1 told you *why* TI built GPL — density, family portability, and the closed box. Now stand on the block diagram and watch what the design means at runtime, because it is the strangest and most instructive thing about this machine.

When the console sits at its color-bar title screen, the TMS9900 is not idle and it is not running the operating system — not directly. It is running one smallish 9900 program out of the fast console ROM, forever: the **GPL interpreter**. That program's loop is the machine's heartbeat: *fetch a byte from a GROM port; decode it as a GPL opcode; carry out its meaning — perhaps moving data between GROM, VDP RAM, and scratchpad; advance; repeat.* The operating system you experience — the title screen, the master menu that found your cartridge, the cassette dialogs, TI BASIC — is 18 K of GPL *bytecode* being fed through that loop from the console GROMs, one serial byte at a time. This is a virtual machine, in the full modern sense of the term, shipped in a 1979 appliance: the JVM analogy is not a teaching metaphor, it is a literal architectural description, right down to the bytecode-in-a-portable-format rationale.

Now type `10 PRINT "HELLO"` and run it, and count the floors your one statement passes through:

- **Floor 3 — BASIC.** Your program text lives *tokenized in VDP RAM* — behind the mail slot (§2.3). The TI BASIC interpreter must fetch each token through the VDP ports before it can even consider it.
- **Floor 2 — GPL.** That BASIC interpreter is not machine code; it is itself a large GPL program in the console GROMs. Every step it takes — fetch token, look up variable, execute PRINT — is some number of GPL opcodes, each of which must be *serially fetched from a GROM port* and decoded.
- **Floor 1 — the 9900.** Each GPL opcode is realized by a run of real instructions in the interpreter's ROM code — the only floor where silicon executes anything at all — with the VM's own state held in scratchpad (the GPL workspace at `>83E0` and status byte at `>837C`, addresses you will come to know personally in Ch. 26).

An interpreter, interpreting through an interpreter, with its subject matter behind an 8-bit port and its bytecode behind another. *That* is why TI BASIC was slow — not the 3 MHz, not the 16 bits, but the architecture of the tower; the same silicon running Floor-1-native code is dozens to hundreds of times faster at like-for-like work, a folk claim we will turn into measured numbers in Chapters 26 and 28. And TI knew the price and paid it deliberately: for menus, dialogs, and courseware, GPL's density and safety were worth more than speed, and the fast ROM + fast pad placement of the interpreter (§2.2–2.3) was precisely the engineering that kept the tower livable.

This book's promise, restated on the tower: you will live on **Floor 1** from Chapter 4 — native 9900, nothing between you and the metal — and then, unusually for any book about any machine, you will *also* master **Floor 2** in Part VI: reading GPL, writing GPL, and building the hybrid programs (GPL shell, assembly core) that were TI's own house style. Floor 3 we visit exactly once, in Chapter 28, as archaeologists — TI BASIC examined as the magnificent artifact it is, not taught as a language. You now know enough to see that this is not snobbery but architecture: the doorway to everything this machine can actually do is one floor off the ground.

## 2.5 A Modern Programmer's Disorientation Kit

Part II drops you onto Floor 1, and the fall disorients everyone the same way. Here is the full list of things that are *not there*, stated once, kindly, in advance — with the reason each absence is a gift.

**No operating system (while you run).** The moment your program starts, the OS of §2.4 is simply *suspended* — it is not scheduling you, protecting you, or watching you; unless you explicitly call back into console services (Ch. 23) or leave the console ISR hooked (Ch. 22), the machine is 100% yours: every cycle, every byte, every register of every chip. There is no "it" to crash. There is only what you wrote.

**No processes, no scheduler, no threads.** One CPU, one program counter, one flow of control — plus, at most, one interrupt level that *you* choose to enable (`LIMI 2`) or silence (`LIMI 0`, Ch. 22). Concurrency on this machine is a design you build (the ISR patterns of Ch. 22, the coroutines of Ch. 9), never an ambient service.

**No memory protection, no virtual memory.** Every address in §2.3's table is live to every instruction. Write to `>8400` and the speaker pops — from anywhere, no driver, no permission. Corrupt the GPL workspace and the title screen will greet you insane when you QUIT. The address space is a shared apartment with no locks; the treaty of who-touches-what (Ch. 24) is enforced only by discipline. Terrifying for a week; then you notice that *nothing is hidden from you either* — every bug is in principle observable, every behavior explainable, and a debugger showing 64 K shows the whole truth.

**No heap.** No `malloc`, no `new`, no allocator of any kind until you write one (and you mostly won't — Ch. 36 teaches the fixed-slot, table-driven layouts the era actually used, which turn out to be a masterclass in the data-oriented design modern performance culture rediscovered).

**No file abstraction.** `open()` is not a concept; it is a *peripheral's* concept, provided by the DSR code a disk controller carries in its own ROM, invoked by a calling convention (Part VII). A bare console has, quite literally, no notion that files exist.

**No hardware stack.** The one that stops people. The 9900 has no stack pointer, no PUSH, no POP, no CALL/RET in the sense you know. A subroutine call (`BL`) parks the return address in register R11 — *one* deep — and the deeper mechanism (`BLWP`) swaps entire workspaces instead of pushing anything. Recursion, nested calls, local variables: all are conventions you will build yourself in Chapter 9, and building them will teach you more about what a stack *is* than a decade of using one.

**Everything memory-mapped, everything timed.** The display is tables in RAM that a chip happens to scan (Part III). Sound is bytes at an address. Time is the 60 Hz frame interrupt and a 3 MHz cycle count you can reason about *exactly* — the same code takes the same cycles, every run, forever. No cache warmth, no branch-predictor moods, no OS jitter. Determinism this pure no modern machine offers you, and it is the foundation of every measurement this book makes.

The kit in one sentence: **you are not writing a program that runs on a computer; you are, for the duration, the computer.** Readers consistently report the same arc — a week of vertigo, then the addictive clarity of a machine held entirely in the head. Chapter 1 promised that clarity is why we're here. Starting in Chapter 4, it's yours.

## 2.6 What "Commercial Quality" Meant in 1982

This book's graduation bar — set in Chapter 1, enforced in Part IX — is software "equal in scope and polish to anything TI or its third parties released." Time to define that operationally, because 1982's professionals had a checklist, visible in every first-party cartridge, and it is measurable. We will call it **CQ-82** and return to it constantly:

1. **Instant, steady response.** Input is sampled every frame; the game *feels* wired to your hand (Ch. 21). No dropped frames in normal play; motion honors the 60 Hz contract (Ch. 17).
2. **An attract mode.** Left alone, the program demonstrates itself — title, gameplay demo, high scores, looping (store shelves demanded it; Ch. 39 builds one).
3. **Visual discipline.** No tearing, no unintended flicker beyond the hardware's honest limits (and *managed* flicker where the four-sprite law bites — §16.3); a coherent palette inside the 9918A's rules.
4. **Sound as design, not decoration.** Distinct voices for events; music that yields to effects gracefully (Ch. 19's priorities).
5. **Speech if the platform's signature is available** — and silence handled gracefully if not: which is the general law, **degrade gracefully**: detect the synthesizer, the 32 K, the disk, and behave sensibly in every configuration (detection recipes throughout; consolidated §34.6).
6. **House etiquette.** `FCTN`+`=` (QUIT) honored or deliberately, safely disabled; the console left sane on exit; works on any console revision.
7. **Difficulty as a curve, not a wall** — tuned, table-driven, fair (Ch. 39's wave director).
8. **Zero crashes.** Not "rarely." The cartridge era had no patches; shipped meant *finished*.
9. **The package.** A manual that teaches, box art that sells, a program that survives its own instructions. (We hold Part IX to this literally — Ch. 39 writes the manual.)

Run any 1982 first-party title from Lab 1 against this list and watch it score; run a modern quick homebrew and watch where it doesn't. CQ-82 is also, you'll notice, an *architecture* checklist in disguise — every line maps to specific chapters of machinery — which is exactly why it works as our rubric.

> **Sidebar — Karl Guttag and the Chip That Went Everywhere.** The 9918's story is a good corrective to Chapter 1's tale of corporate stumbles, because the same company, the same years, produced one of the quietly most successful chips of the era. TI handed the video problem to a small team including a young architect named Karl Guttag, and the resulting design — self-refreshing private DRAM, table-driven character graphics, and hardware sprites presented to the programmer as a clean abstraction — was so much machine for the money that it escaped its parent. The 9918/9928/9929 family became the video system of the ColecoVision, of Sega's SG-1000, and of the entire international MSX standard; its concepts flowed onward into the MSX2's V9938 and, by lineage, into Sega's later console video hardware — which means a design finished at TI around 1979 was still visibly ancestral in living rooms a decade later. Guttag went on to architect TI's pioneering programmable graphics processors (the TMS340 line) and, decades later, became one of our best primary sources, writing and speaking candidly about how the 99/4's chips were really designed — including the 9985 cancellation you met in §1.3. When Part III seems to describe a video chip with unusually good manners for 1979, this sidebar is the reason: it was designed by people who expected strangers to program it, inside a company that mostly didn't.

## 2.7 The Road Map, Drawn on the Block Diagram

You now hold the whole machine at 10,000 feet. Here is where the rest of the book lands on it — bookmark this table; it is the answer to "where am I?" for the next thousand pages.

| Block-diagram territory | Chapters | What you build there |
|---|---|---|
| TMS9900 + scratchpad (the fast domain) | Part II (4–11) | Fluency: the instruction set, workspaces, your own stacks, the start of `lib99` |
| The funnel itself (bus, wait states) | Ch. 5, then Ch. 37 | The measured map of what memory costs; later, the optimization doctrine built on it |
| TMS9918A + its 16 K | Part III (12–18) | Text engines, bitmap graphics, sprites, smooth scrolling, the 60 Hz game loop |
| TMS9919 + speech side-port | Part IV (19–20) | A music/SFX driver; the machine's voice, including new speech from your own |
| TMS9901, interrupts, console ROM services | Part V (21–24) | Input layers, ISR mastery, ROM floating point, the scratchpad treaty |
| Console GROMs + the interpreter tower | Part VI (25–29) | GPL itself: read it, write it, ship it; the OS understood; hybrid carts |
| Side port, DSR window `>4000`, CRU cards | Part VII (30–34) | Your own DSRLNK, file I/O, disk internals at sector level, modern peripherals |
| Cartridge port `>6000` + GROM ports | Ch. 27, Part VIII (35–38) | Real cartridges: headers, bank switching, asset pipelines, the project template |
| *All of it at once* | Part IX (39–43) | Five commercial-grade programs, CQ-82 enforced |
| Beyond the console's edge | Part X (44–45) | The family, other languages, the community you're joining |

One diagram, one table, one book.

> **Pitfalls.**
> - **"16-bit computer" intuitions.** The CPU is 16-bit; the *system* is an 8-bit machine with a 16-bit heart and a 264-byte fast island (8 K ROM + 256 B pad + nothing else). Plan like an 8-bit programmer, then exploit the heart.
> - **MMIO is not RAM.** The addresses in §2.3's port rows have *side effects*. Reading `>8800` consumes a byte from the VDP's address counter; reading the status port clears flags; a stray read while "just looking around" in a debugger can derail a running program. (This book's bench peeks are deliberately side-effect-free — tools that read through the *live* bus are not.) Full hazard catalog in Chs. 12, 18, 25 — for now: treat port addresses as verbs, not nouns.
> - **Assuming the 32 K exists.** `>2000`–`>3FFF` and `>A000`–`>FFFF` are *holes* on a bare console — writes vanish, reads float. Commercial software detected before touching (CQ-82 §5); so will ours.
> - **"The OS will clean that up."** There is no OS while you run (§2.5). Whatever you break stays broken until reset — including the pad state the *next* environment expects (the treaty, Ch. 24).
> - **Extrapolating from TI BASIC.** Its speed tells you about the tower (§2.4), not the silicon. Never benchmark the platform — or your expectations — on Floor 3.

## Lab 2 — Watching the Multiplexer

*Goal: see the 8-bit funnel with your own eyes, by making the emulator count cycles for the same instruction executed in both bus domains. This lab deliberately runs a chapter ahead of the toolchain — no assembler required, just the book's lab bench, **BENCH99**: a typed-command monitor over the emulator's own core. Bring-up is three lines (Chapter 3 explains what they mean; today, just run them from the book's folder in the emulator repository): `sh setup.sh`, then `code/bench/target/release/bench99`, and you're at the `bench>` prompt. Classic99's and MAME's GUI debuggers expose the same powers if you'd rather point and click — the numbers must come out the same.*

**Part A — the fast domain, live.** At the prompt, type `boot` — the real firmware boots to the master title screen — then `s 12`. Look at the addresses in the trace: PC is somewhere in `>0000`–`>1FFF` — console ROM, the 16-bit domain — because you have caught the GPL interpreter mid-heartbeat (§2.4), and each line prices one instruction in cycles. You are watching Floor 1 execute Floor 2's fetch loop; if the PC suddenly leaps to a low-ROM handler and back, congratulations — you've witnessed the 60 Hz frame interrupt photobombing your experiment (Ch. 22 explains everything you just saw).

**Part B — plant a flag in each domain.** Quit and restart the bench (it opens *bare*: the same machine, nothing running, PC and WP yours — a paper machine made of silicon). We'll execute the same one-word instruction — `JMP $`, opcode `>10FF`, an instruction that jumps to itself — from both sides of the funnel:

1. `pw A000 10FF` — plant the flag in high expansion RAM (8-bit domain; the machine's 32 K is present).
2. `pc A000`, then `s 5`. Each step is one full instruction: one word fetched *through the multiplexer*. Record the cycles per step.
3. Now `pw 8300 10FF` (scratchpad — 16-bit domain), `pc 8300`, `s 5`, and record again.
4. Tabulate: same opcode, same everything, different address. The pad execution costs the instruction's book price — **10 cycles** — and the expansion-RAM execution costs **14**: the funnel's toll, four cycles, on the single memory access involved (the instruction fetch). Run each several times; determinism (§2.5) predicts identical numbers every time, and you will get them.

**Part C — reflect (and note what you didn't have to clean up).** The bare bench never booted the OS, so there was nothing to trample — but notice that you just wrote to a running machine's RAM and kidnapped its PC, and had this been a *live* console (Part A's, or Classic99's debugger over a booted machine) you'd now owe it a power-cycle; §2.5's "no protection" cuts both ways, even for debuggers. Then write the lab's deliverable: your cycle table, plus two sentences answering — *if a one-access instruction pays ~4 cycles at the funnel, what will an instruction that fetches from expansion RAM, reads a workspace register also in expansion RAM, and writes it back pay?* Keep your prediction; Chapter 5's lab measures exactly that, and Chapter 4 explains why "workspace register" appeared in a sentence about memory.

## Exercises

**2.1 ✦** Close the book and redraw the block diagram — boxes, buses, the three wiring types, the two domains. Grade yourself against §2.2; repeat tomorrow. (This is the one diagram the whole book assumes.)

**2.2 ✦** Classify each address by domain and kind (fast RAM/ROM, 8-bit memory, MMIO port, hole-on-bare-console): `>0042`, `>8310`, `>6000`, `>8C00`, `>B000`, `>4000`, `>9800`, `>83E0`.

**2.3 ✦** From memory, one sentence per cast member (§2.1): the chip's name and its single job. Eight sentences, no peeking.

**2.4 ✦✦** Estimate the funnel's tax on a bulk copy. A loop copies 1,024 bytes as 512 words from `>A000`-land to `>2000`-land. Count, per iteration, the memory accesses that cross the multiplexer (a) when the loop code and its workspace also live in expansion RAM, and (b) when both live in scratchpad — then, using the nominal +4 cycles per crossing word, estimate the total surcharge each way. State every assumption you had to make; Chapter 5 will grade them.

**2.5 ✦✦** Trace `X=X+1` down the tower (§2.4): list, in order, every kind of fetch and port crossing involved, floor by floor, from token to silicon. Qualitative is fine — the point is the *count of indirections*, not cycles.

**2.6 ✦✦** Run a CQ-82 audit (§2.6) on any present-day indie or mobile game you know well. Which of the nine items does it satisfy, which are meaningless today, and which — honestly — would 1982 have done better?

**2.7 ✦✦✦** The counterfactual board meeting. It is 1980 and you may change exactly one thing: (a) full 16-bit bus to all memory, (b) 1 K of scratchpad instead of 256 bytes, or (c) VDP RAM mapped into the CPU's address space. Argue for one, against the other two, using only consequences traceable on the block diagram — including at least one *harm* your chosen change causes.

**2.8 ✦✦✦** Write the bench tech's flowchart (Prologue): a fault-isolation tree for the symptoms *no video*, *video but no cartridges found*, *no sound*, *no keyboard*, using only block-diagram reasoning about who talks to whom. Then check yourself: which single component's failure explains the most symptoms at once, and why is that the answer the diagram would predict?

## Further Reading

- **TMS9900 Microprocessor Data Manual** (TI) — skim the system-architecture chapters now; Part II reads it with you properly.
- **TMS9918A/9928A/9929A Video Display Processors Data Manual** (TI) — the text Part III treats as literature; today, just admire the block diagram on its early pages against ours.
- **TMS9901 Programmable Systems Interface Data Manual** (TI) — Ch. 21's companion.
- **TI-99/4A Console and Peripheral Expansion System Technical Data** (TI's service-level documentation) — the real fold-out; community archives host scans.
- **Thierry Nouspikel's TI-99/4A Tech Pages** — the community's encyclopedic internals reference, assembled over decades; the single most-cited modern source in this book's back matter (App. N maps it).
- **Karl Guttag's retrospectives** on the 9918 and the 99/4's chipset politics — primary-source texture for §2.1's cast and this chapter's sidebar.
- Forward: **Appendix C** (memory maps poster) and **Appendix D** (VDP reference) formalize today's tables.

## Summary

- Cast (§2.1): TMS9900 CPU @ 3.0 MHz (registers = RAM workspaces); TMS9901 (CRU-driven I/O + interrupts + timer); TMS9918A VDP with private 16 K VRAM (frame interrupt, sprites; 9929A in PAL); TMS9919/SN94624 sound (SN76489 family) at `>8400`; console ROM 8 K (vectors, ISR, FP, **GPL interpreter**) on the 16-bit bus; 3 console GROMs (18 K GPL: OS, menu, TI BASIC) behind serial ports; scratchpad = **256 B at `>8300`–`>83FF`**, 16-bit, zero-wait.
- Fast domain = console ROM + pad only; **everything else crosses the 8↔16 multiplexer at a nominal +4 cycles/word** (measured precisely in Ch. 5). Three fabrics: memory bus, CRU, interrupts.
- Memory geography (§2.3): bare console CPU RAM = 256 B; the "16 K" is VRAM behind ports `>8800/>8802/>8C00/>8C02`; 32 K expansion fills `>2000` (8 K) + `>A000` (24 K), 8-bit; DSR window `>4000`; cart ROM `>6000`; GROM ports `>9800/>9802/>9C00/>9C02`; speech `>9000/>9400`. **Law: speed is a property of addresses, not instructions.**
- The tower (§2.4): BASIC (tokens in VRAM) over GPL (bytecode in GROM, VM state at `>83E0`/status `>837C`) over 9900 (interpreter in fast ROM). Book occupies Floor 1 (Part II+) and Floor 2 (Part VI); Floor 3 visited once (Ch. 28).
- Disorientation kit (§2.5): no OS-while-running, no scheduler, no protection, no heap, no file abstraction, **no hardware stack** (BL→R11; BLWP swaps workspaces); fully deterministic timing.
- **CQ-82** checklist named (§2.6, 9 items: response, attract mode, visual/sound discipline, speech + graceful degradation, QUIT etiquette, difficulty curve, zero crashes, packaging) — Part IX's rubric.
- Lab 2: same `JMP $` stepped at `>8300` vs `>A000` on BENCH99 (bare bench: `pw`/`pc`/`s`), measured 10 vs 14 cycles — the funnel's +4, machine-verified; Part A traces the live GPL interpreter in console ROM (`boot`, `s 12`). Road-map table (§2.7) binds every book part to a block-diagram territory. New style ruling R-8 (named recurring artifacts get ledger handles).
