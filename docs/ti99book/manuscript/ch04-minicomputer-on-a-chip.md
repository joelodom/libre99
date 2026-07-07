# Chapter 4 — The TMS9900: A Minicomputer on a Chip

*The processor as its designers thought of it — a process-control minicomputer that happens to fit in a socket.*

<!-- Part II · target 24 pp · prerequisites: Ch. 2–3 · Lab: paper-machine trace, verified on the lab bench -->

## Prologue: The Machine That Couldn't Stop to Think

Before it was a home computer's heart, this architecture ran factories. Picture a TI-990 minicomputer in the mid-1970s doing the job it was actually designed for: process control. A refinery line, say — sensors streaming readings, valves needing answers, and interrupts arriving not as exceptions but as *the workload itself*, hundreds per second, each one demanding that the machine drop everything, handle a physical-world event *now*, and resume as if nothing had happened.

On most processors of the era, "drop everything" is the expensive part. Every register the interrupted program was using must be saved somewhere, one store at a time, before the handler dares touch anything; every one must be restored afterward. The 990's designers looked at that ritual and asked an impertinent question: *why are the registers in the processor at all?* Put them in memory — ordinary RAM — and let one pointer say where. Then "drop everything" becomes: change the pointer. The interrupted program's sixteen registers lie untouched where they always were; the handler gets sixteen fresh ones somewhere else; three values moved, context switched, microseconds flat. The refinery never waits.

That is the idea inside the 40 acres of silicon TI compressed into the TMS9900 in 1976, and it is why this chapter keeps calling your console's CPU a *minicomputer*. It thinks like one. It treats registers as a view into memory rather than a treasure on the die; it treats context as cheap and interrupts as routine; it operates on memory directly, anywhere to anywhere, like a machine whose RAM was always fast enough to trust. Chapter 2 showed you the one cruel joke history played on it — a console where most RAM *isn't* fast, except for 256 bytes. This chapter shows you the elegant machine that joke was played on. Learn it on its own terms first; the coping strategies come after, and they will make no sense without today.

---

## What You Will Learn

After this chapter you can:

- Draw the 9900's complete programmer-visible state — PC, WP, ST, and nothing else — and explain what each register holds and how software reads or writes it.
- Define a workspace precisely (16 words at WP; register *n* lives at address WP + 2*n*), place one anywhere in RAM, and state both the superpower (pointer-swap context switches, registers with addresses) and the tax (every register touch is a memory cycle).
- Apply the memory model without bugs: 64 K bytes as 32 K aligned words, big-endian order, and the byte-instructions-use-the-high-byte rule — including recognizing the classic failure signatures on sight.
- Read and predict every status bit — L>, A>, EQ, C, OV, OP, X, and the interrupt mask — and pick the correct conditional jump for signed vs. unsigned comparisons using the semantics table.
- Explain subroutine linkage without a stack: `BL`/R11 for light calls, `BLWP`/`RTWP` for full context switches, and the register-role conventions (R0, R11, R12, R13–R15) that the hardware itself enforces.
- Describe the instruction-format families and the memory-to-memory philosophy — what "general addressing" buys and what it costs.
- Estimate instruction cost from the timing model (base cycles + memory accesses + where those accesses land) well enough to explain Chapter 2's lab numbers from first principles.
- Place the 9900 honestly among 8080, Z80, 6502, and 68000.

## The Bridge: A Register File Is Just a Struct You Point At

Modern intuition says registers are the fastest storage there is, a scarce treasure the compiler allocates, physically welded into the pipeline. The 9900 asks you to hold a different picture: imagine your CPU's entire register file as a C struct — `struct ws { u16 r[16]; }` — and the processor holding merely a *pointer* to the current instance. Every named register is syntax sugar for `wp->r[n]`. Want a completely different register file? Point somewhere else. Want two? Ten? They're structs; allocate them. Want to read your own R5 as data, or walk a pointer *through your registers*? They have addresses; help yourself.

If you've met green threads, `ucontext`, or coroutine libraries that heroically save and restore register sets to switch stacks, the 9900 will feel like meeting the machine those libraries wish existed: context switch as pointer assignment, provided by hardware, in one instruction. And if you've done performance work, you already sense the bill: `wp->r[n]` is a memory dereference, so this processor's "register speed" is exactly the speed of the RAM you parked the struct in. On the 990 minicomputer, with fast memory throughout, the bill was small and the elegance pure. On the 99/4A, Chapter 2's geography turns the parking decision into the single most consequential line of your program — `LWPI >8300` — which you have already written once without knowing why. By the end of this chapter, you'll know exactly why.

## 4.1 The TI-990 Inheritance

The 9900 is not "inspired by" the TI-990 minicomputer line; it is a 990 — instruction-compatible with the 990/4, shrunk to one die. That parentage explains nearly every design choice you're about to meet, so it's worth thirty seconds on what the parent was *for*.

The 990 family sold into the unglamorous middle of 1970s computing: factory floors, data collection, terminals, communications gear. Three requirements dominate that world. **Interrupt latency** — physical processes don't buffer politely, so the machine must switch contexts in microseconds; hence registers-in-memory and the three-pointer context switch, which turned the most expensive operation in computing into one of the cheapest. **Many small contexts** — a controller juggles dozens of little tasks, each wanting its own state; hence workspaces as cheap, plentiful structs rather than one precious register file to be fought over. **Direct data plumbing** — control work is mostly *moving and testing* values between device registers, tables, and counters; hence a fully general memory-to-memory instruction set (§4.7) where `A @SENSOR,@TOTAL` is one instruction, no load/store bucket brigade required, plus a bit-granular I/O fabric (the CRU, Ch. 10) for the thousand valves and switches.

Notice what is *not* on that list: raw compute throughput, large address spaces, glamour. The 990 was a plumber-philosopher, and the 9900 inherited both the philosophy and the plumbing — including, fatefully, the 16-bit/64 K address model that looked ample for a controller in 1974 (§4.9 counts the cost). When TI's consumer division reached for a CPU (Ch. 1), they got a superb factory brain with reflexes tuned for interrupts the console would barely use, and memory assumptions the console would violate. Both facts become *your* material: the reflexes power Chapter 22's ISR work and Chapter 9's coroutines, and the assumptions define Chapter 37's entire optimization doctrine.

## 4.2 The Programmer's Model: Three Registers and a Pointer's Worth of Radicalism

Here is the entire programmer-visible state of the TMS9900. Not the highlights — the *entirety*:

```text
        THE COMPLETE TMS9900 PROGRAMMER'S MODEL
        ---------------------------------------
        PC   16 bits   Program Counter — address of next instruction
        WP   16 bits   Workspace Pointer — address of R0
        ST   16 bits   Status — flags (§4.5) + interrupt mask

        ...that's all. R0–R15 are not on this chip.
```

Everything else you will ever call a "register" lives in RAM. A **workspace** is any 32-byte, word-aligned span of memory; the WP register holds its starting address; and the sixteen general registers are defined by nothing more than arithmetic:

```text
        address of Rn  =  WP + 2n

   RAM  >8300  R0     <- WP points here        (our HELLO's choice)
        >8302  R1
        >8304  R2
        ...
        >831E  R15
```

When an instruction names R5, the processor computes WP+10 and performs a memory cycle at that address. That's the whole mechanism — no register file, no rename engine, just a base pointer and an offset, executed with total sincerity on every register access.

Software touches the three real registers like so. **WP**: set it wholesale with `LWPI addr` (Load Workspace Pointer Immediate — HELLO's second instruction), read it with `STWP Rn` (STore WP into a register — yes, into RAM, where you can then do arithmetic on the location of your own registers; §4.3 plays with this). **ST**: read with `STST Rn`; its interrupt-mask field is set with `LIMI n` (HELLO's first instruction); its flag bits are set implicitly by nearly everything (§4.5). **PC**: no direct read — the idiom is `BL @NEXT` / `NEXT ...`, which deposits the address of `NEXT` in R11 as a side effect of a do-nothing call; you'll meet it in real code by Chapter 9. All three change *together*, atomically, during the context-switch operations (`BLWP`, `RTWP`, interrupts, `XOP`) that §4.3 unpacks — that trio-swap is the 990 reflex the whole architecture is built around.

Two registers in the workspace aren't quite ordinary, and two more have hardware-assigned jobs; learn the reserved seating now, hardware's reasons in the sections noted:

| Register | Hardware-assigned role | Where it bites |
|---|---|---|
| R0 | Shift count source when a shift instruction's count field is 0 (low 4 bits; 0 ⇒ 16). **Cannot serve as an index register** — the encoding that would mean "indexed by R0" means *symbolic* addressing instead. | §4.7, Ch. 8 |
| R11 | Return-address link for `BL`; also receives the source effective address during `XOP`. | §4.6, Ch. 9 |
| R12 | CRU base address for all CRU instructions. | Ch. 10 |
| R13–R15 | Context linkage: `BLWP`/interrupt/`XOP` deposit the *old* WP, PC, ST here (in the **new** workspace); `RTWP` reloads from them. Treat as spoken-for in any workspace that receives calls or interrupts. | §4.3, Ch. 22 |

Everything else — R1–R10 — is genuinely general, and this book's own conventions for them (R10 as our software stack pointer, chief among them) get declared when we build the machinery that needs them (Ch. 9).
## 4.3 Consequences: What Registers-in-RAM Buys and Costs

Three consequences follow from `Rn = WP + 2n`, and together they *are* the 9900's personality.

### Consequence one: context switches by pointer swap

The hardware's crown move. `BLWP @VEC` (Branch and Load Workspace Pointer) reads a two-word **vector** at `VEC` — a new WP, then a new PC — and performs the full switch in one instruction: new WP and PC loaded, and the *old* WP, PC, and ST deposited into the **new** workspace's R13, R14, and R15. `RTWP` is the exact inverse, reloading all three from R13–R15. Sixteen fresh registers, complete linkage, one instruction each way — and precisely the same mechanism, vectors and all, is what a hardware **interrupt** performs (vectors for the sixteen levels live at `>0000`–`>003F`, which is why Chapter 2's map starts with ROM: the machine *must* find vectors at address zero), and what the `XOP` instruction performs (its sixteen vectors at `>0040`–`>007F` — a "supervisor call" facility the console mostly leaves for us to play with; Ch. 9). Subroutine call, OS trap, and interrupt are, on this machine, *one idea* at three trigger points. The console ROM's ISR runs in its own workspace at `>83C0`; the GPL interpreter lives in its own at `>83E0` (Ch. 2's tower, now explicable); and your programs will deploy workspaces-per-subsystem the same way — a main loop's registers here, a sound driver's there, an ISR's in the pad, no saving ceremonies anywhere (Ch. 22 institutionalizes it).

### Consequence two: registers have addresses

Your registers are bytes in RAM, and *nothing stops you from treating them as data*. Read that twice, then read this four-line demonstration — legal, idiomatic, and slightly vertiginous:

```asm
       STWP R4              R4 = address of R0 (our own workspace!)
       MOV  @4(R4),R5       load the word at WP+4  — that is R2 — into R5
       LI   R6,>1234
       MOV  R6,@12(R4)      store >1234 into the word at WP+12 — R6? no: R6 is +12... 
```

— and there the comment trails off on purpose, because the arithmetic *is* the lesson: WP+12 is R6 itself, so that last instruction stores R6 into R6, a no-op with extra steps, and if you caught that before reading this sentence, §4.4's byte-order material will hold no terrors for you. The serious uses are real: iterate a pointer across your own registers to clear or save them in a loop; alias two workspaces a few words apart so callee's R13–R15 *are* caller's R0–R2 (a professional's overlap trick, revisited in Ch. 9); build a debugger like Chapter 13's `MONITOR99` that displays any workspace by simply *reading memory*. On most CPUs "dump the registers" requires privileged magic. Here it's a `MOV` loop.

### Consequence three: every register access is a memory access

The tax, stated without anesthesia. `MOV R1,R2` — the simplest register-to-register operation — performs **four** memory cycles: instruction fetch, read R1 (a RAM address), a read of R2, and the write of R2. (Yes, a read of the destination it's about to overwrite: the 9900 reads destinations before writing them, a hardware habit with consequences we'll respect when destinations are I/O ports — Ch. 12.) Four cycles at memory speed, times whatever your memory's speed is: that is the whole performance story of this platform in one sentence. Park the workspace in the 16-bit pad and those four accesses run free; park it in expansion RAM and each one pays Chapter 2's funnel toll; the *same instruction* varies by more than 2× (§4.8 puts numbers on it, Ch. 5 measures them, Ch. 37 builds the doctrine). The 990's designers assumed uniform fast memory and bought elegance with it. The console broke the assumption but kept 256 bytes of it alive — so on this machine, the first act of every serious program is claiming its patch of the fast island, which is why `LWPI >8300` came before anything else HELLO did, and why Chapter 24 exists to adjudicate the island's real estate.

## 4.4 The Memory Model: Words, Bytes, and the High-Byte Rule

The 9900 sees 64 K bytes organized as 32 K **words** of 16 bits. Words live at even addresses, and the hardware is serenely unbending about it: word operations ignore the address's bottom bit, so a word access "at" `>A001` is silently an access at `>A000` — no fault, no warning, just not what you meant. (`EVEN` in your source, after any odd-length `TEXT`, exists to keep labels off odd addresses; the assembler aligns instructions itself.)

Byte order is **big-endian**: the high-order byte of a word lives at the lower address. The word `>1234` stored at `>A000` puts `>12` at `>A000` and `>34` at `>A001` — the order hex dumps read naturally, the order network protocols chose, and (fair warning) the *opposite* of the x86/ARM world most readers grew up debugging.

```text
                 word at >A000 = >1234
        address:   >A000    >A001
        byte:       >12      >34
                  (high)    (low)
```

So far, tidy. Now the rule that generates more newcomer bugs than everything else in this book combined, stated in bold and then drilled:

**Byte instructions applied to a register operate on the register's HIGH byte.** 

It is perfectly consistent — a register is a word, a word's byte 0 is its high byte, and `MOVB R1,...` means "the byte of R1," which is byte 0, which is the *top* half — but consistency is cold comfort at the debugger at midnight, so here is the failure parade in advance. You want to output the character `A` (`>41`):

```asm
* THE CLASSIC BUG                        * THE FIX
       LI   R1,>0041                            LI   R1,>4100
       MOVB R1,@VDPWD    sends >00 !            MOVB R1,@VDPWD    sends >41
```

The buggy version loads `>41` where every modern instinct puts it — the low byte — and `MOVB` faithfully ships the high byte, `>00`. Screen shows nothing; nothing crashes; you stare. The idiom, then: **byte constants ride high** (`LI R1,>4100`, or the assembler-arithmetic spelling `LI R1,'A'*256` `[libre99asm]` — character literals in expressions), and when a byte you need is stranded in the low half, **`SWPB`** — swap bytes — is the one-instruction rescue, which is why HELLO's address dance went `MOVB` / `SWPB` / `MOVB`. Three more corollaries complete the survival kit. *Byte writes to a register leave the low byte untouched* — `MOVB` into R3 changes R3's top half only, so registers can carry a byte and a flag or two bytes deliberately (a Ch. 37 packing trick) or accidentally (a bug where stale low bytes haunt later word compares). *Byte reads from memory* take whichever byte the address names — `MOVB @>A001,R1` fetches `>34` from our diagram into R1-high — so even addresses fetch high bytes, odd fetch low, and `*R2+` in a byte instruction advances R2 by **one**, not two (auto-increment steps by operand size; word instructions step by two — one of the instruction set's genuinely lovely symmetries, Ch. 7). And *the status flags on byte operations reflect the byte*, including a parity bit no word operation touches (§4.5).

Drill it now, sixty seconds, honor system — cover the right column:

| You wrote | Register/effect afterward |
|---|---|
| `LI R1,>00FF` then `MOVB R1,R2` | R2 high byte = `>00` (EQ set!); R2 low byte unchanged |
| `LI R1,>FF00` then `MOVB R1,R2` | R2 high byte = `>FF`; the one you probably wanted |
| `MOVB @>A000,R3` (word `>1234` there) | R3 = `>12xx` (low byte preserved) |
| `MOVB @>A001,R3` | R3 = `>34xx` |
| `SWPB R1` (R1=`>00FF`) | R1 = `>FF00` — no flags changed (Ch. 8's list of flag-silent ops) |
| word `MOV` "at" `>A001` | operates at `>A000`; A15 ignored, no error |

If the parade felt obvious: excellent, you have the model. If not: bookmark the table — Pitfalls below repeats the signatures, and the lab makes you predict one of these under oath.
## 4.5 The Status Register, Bit by Bit

ST is where every computation leaves its residue and every conditional jump finds its evidence. TI numbers status bits from the *most* significant end — ST0 is the top bit — and the layout is:

| Bit | Name | Meaning | Set by |
|---|---|---|---|
| ST0 | **L>** | Logical (unsigned) greater than | compares, arithmetic, moves |
| ST1 | **A>** | Arithmetic (signed) greater than | compares, arithmetic, moves |
| ST2 | **EQ** | Equal / result zero | compares, arithmetic, moves, CRU `TB` |
| ST3 | **C** | Carry out (no-borrow, for subtracts) | add/subtract/shift family |
| ST4 | **OV** | Signed overflow | add/subtract family, `DIV` |
| ST5 | **OP** | Odd parity of a **byte** result | byte operations only |
| ST6 | **X** | `XOP` in progress | `XOP` |
| ST7–11 | — | unused on the 9900 | — |
| ST12–15 | **mask** | Interrupt mask level (0–15) | `LIMI`, context switches |

Two reading protocols cover ninety percent of real code, and the distinction between them is the distinction between `JH` and `JGT` — burn it in now.

**After a compare, `C S,D` (or `CB` for bytes):** the flags describe *S relative to D*, twice over — once as unsigned quantities (L>), once as signed (A>), plus EQ. Nothing else changes; compare is a pure question.

**After nearly everything else — `MOV`, `A`, `S`, `INC`, logic ops:** the flags describe *the result relative to zero*: EQ means the result was zero, A> means signed-positive, L> means simply nonzero. The gift inside this rule: **`MOV` is also a test.** Moving a value sets EQ/A>/L> on it for free, so the idiom `MOV @COUNT,R1` / `JEQ EMPTY` tests-and-loads in one breath, and the explicit `C R1,0`-style compare you keep wanting to write is almost always redundant. (Chapter 8 lists the handful of flag-silent exceptions — `SWPB`, notably, changed nothing in §4.4's drill table on purpose.)

Now the jump menu, which is nothing but ST bits with mnemonics — the letters **H/L** always mean the *logical* (unsigned) reading, **GT/LT** the *arithmetic* (signed) one:

| Jump | Taken when | After `C S,D`, means | Family |
|---|---|---|---|
| `JEQ` / `JNE` | EQ / ~EQ | S = D / S ≠ D | either |
| `JH` | L> and ~EQ | S > D | unsigned |
| `JHE` | L> or EQ | S ≥ D | unsigned |
| `JL` | ~L> and ~EQ | S < D | unsigned |
| `JLE` | ~L> or EQ | S ≤ D | unsigned |
| `JGT` | A> | S > D | signed |
| `JLT` | ~A> and ~EQ | S < D | signed |
| `JOC` / `JNC` | C / ~C | carry (no-borrow) / not | — |
| `JNO` | ~OV | no signed overflow | — |
| `JOP` | OP | odd byte parity | bytes |
| `JMP` | always | — | — |

Study the signed column's *gaps*: there is no signed ≥ and no signed ≤ in one instruction. The idioms are inversion — want `JGE`? Write `JLT` around the fall-through — or the two-jump pair (`JGT` then `JEQ` to the same target). Every 9900 programmer writes these pairs within their first week; write yours in Exercise 4.5 so the first time isn't in anger. Two more residues worth naming: **C after subtraction means *no borrow*** (the add-the-complement convention — so `DEC R2` leaves C=1 every pass until R2 wraps past zero, and HELLO's `DEC`/`JNE` loop was also, silently, a `DEC`/`JOC`-compatible one); and the interrupt-mask nibble at the bottom is what `LIMI 0`/`LIMI 2` actually writes — the console recognizes, effectively, "off" and "on" (Ch. 2's single wired level; ceremony and exceptions in Ch. 22).

## 4.6 No Stack: The R11 Convention

Search the model in §4.2 for a stack pointer. There isn't one — no SP, no `PUSH`, no `POP`, no `CALL` that stores a return address in memory. The 9900 offers exactly two calling mechanisms, both stackless, both already half-familiar:

**`BL @SUB` — Branch and Link**, the light call: the address of the *next* instruction is placed in **R11**, and PC jumps to `SUB`. Return is `RT` — an E/A-standard pseudo-instruction that assembles to `B *R11`, "branch to wherever R11 points." Cost: nearly nothing. Depth: **one.** A second `BL` before the first returns silently overwrites R11, and the outer caller is unreachable forever — the classic crash of every newcomer's second week (Pitfalls). The disciplines, previewed here and drilled in Ch. 9: *leaf* routines (which call nothing) may use R11 freely and `RT`; *non-leaf* routines must first save R11 — to another register, to a memory slot, or to the software stack we'll build; and any routine's register usage gets documented in its header comment (a `lib99` law from Ch. 11).

**`BLWP @VEC` / `RTWP`** — the heavy call, §4.3's context switch wearing a subroutine's clothes: fresh workspace, automatic linkage in R13–R15, caller's registers untouched and *readable through R13* (the callee can reach the caller's Rn at `@2n(R13)` — parameter passing without moving anything, an idiom Ch. 9 formalizes). Cost: several times a `BL`. Depth: as many workspaces as you allocate. It is the right tool for module boundaries, interrupt-like services, and anything reentrant — and overkill for the small hot routines that dominate real programs, which is why the *software stack* exists.

That software stack — a register we dedicate as SP, decrementing through a RAM region with two-instruction push/pop sequences, wrapped in macros — is Chapter 9's opening construction, and this book's convention for it is declared now so every intervening example can respect it: **R10 is reserved as the stack pointer** in all `lib99`-conforming code. What should strike you today is the *inversion of the modern default*: on this machine, calls are cheap and stacks are optional equipment. A 1982 cartridge with three subsystems and no stack anywhere — just `BL` leaves, a few `BLWP` modules, and static variables — isn't primitive; it's the architecture speaking its native dialect. You'll write both dialects fluently.

## 4.7 Instruction Formats and the General-Address Philosophy

The 9900's instruction set is organized less like a menu and more like a grammar: a small set of **formats** (the datasheet counts nine) crossed with a powerful idea called **general addressing**. Master the idea and the formats become predictable; here is the idea.

In a **Format I** instruction — the two-operand workhorses `MOV`, `A` (add), `S` (subtract), `C` (compare), `SOC`/`SZC` (bit set/clear), and their byte twins — *each* operand independently carries a 2-bit mode and a 4-bit register, and the five resulting modes are the entire addressing repertoire of the machine (Ch. 7 gives each its own workout):

| Mode | Syntax | Meaning |
|---|---|---|
| Register | `R5` | the workspace word itself |
| Indirect | `*R5` | memory at the address in R5 |
| Auto-increment | `*R5+` | as indirect, then R5 += operand size (1 or 2) |
| Symbolic | `@LABEL` | memory at an address in the instruction stream |
| Indexed | `@LABEL(R5)` | memory at LABEL + R5 — any register **except R0** (§4.2) |

Both operands. Any combination. `A @SENSOR,@TOTAL` adds memory to memory — fetch, add, store, flags, one instruction — with no accumulator anointed and no load/store bucket brigade. `MOV *R1+,*R2+` is a copy engine in four bytes. This is the 990 plumber's philosophy delivered whole: the machine assumes your data is *out there* and comes to it, symmetrically, in bytes or words. CISC before the acronym, and unusually orthogonal even by that standard.

The price is printed on the same tag. Every general operand the instruction must resolve adds fetch and access cycles — the exact menu is Chapter 7's cost table — so that same glorious `A @X,@Y` runs several times longer than `A R1,R2`, and *where* those extra accesses land (§4.8) multiplies the difference. General addressing is power billed by the touch; 9900 style, which you'll absorb through Part II, is knowing when one rich instruction beats three spare ones and when it's the other way around.

The remaining formats are the grammar's shorter sentences, met as their chapters arrive: single-operand general (Format VI: `B`, `BL`, `BLWP`, `CLR`, `INC`, `DEC`, `INV`, `NEG`, `SWPB`, `SETO`, `ABS` — one operand, same five modes); jumps (Format II: PC-relative, signed 8-bit displacement *in words*, reach about ±127 words — the reason long-distance control flow uses `B @` instead, Ch. 9); shifts (Format V: count in the instruction or, when zero, from R0 — §4.2's reserved seat); immediates and internal-register ops (Format VIII: `LI`, `AI`, `CI`, `ANDI`, `ORI`, `LWPI`, `LIMI`, `STWP`, `STST` — note how short the list is: immediates are the *exception* here, not the idiom); multiply/divide and `XOP` (Format IX, Ch. 8); CRU transfers and bit tests (Formats IV and II's other half, Ch. 10); and the no-operand control set (Format VII: `RTWP`, `IDLE`, `RSET`, and friends — including three pin-wiggling oddities the console never wired to anything, a nice reminder that this chip expected to run factories). Full encodings, cycle counts, and the opcode matrix live in Appendix A, drafted alongside Chapters 7–9; nobody memorizes formats — you internalize the *addressing grammar*, and the rest is reference.
## 4.8 The Timing Model: Where Cycles Actually Go

The 9900 in the console runs at 3.0 MHz — one clock cycle every 333 nanoseconds — and its datasheet prices every instruction with two honest numbers: **C**, the base cycle count, and **M**, the number of memory accesses performed. On the 990's uniform fast memory, C was the whole story. On the console, M is where the story gets interesting, because each access lands *somewhere* on Chapter 2's map, and accesses that land in the 8-bit domain each pay the multiplexer's toll of about four extra cycles. The working model, good enough until Chapter 5's laboratory refines it:

```text
   time  =  C  +  4 × (memory accesses that land in the 8-bit domain)
```

Now run the model on the humblest instruction in the set. `MOV R1,R2`: the datasheet says C=14, M=4 — instruction fetch, read R1, read R2, write R2 (that destination read is §4.3's read-before-write habit, now visible in the accounting). Three placements, three prices:

| Where things live | Accesses crossing the funnel | Time |
|---|---|---|
| Code in pad/ROM, workspace in pad | 0 | **14 cycles** (4.7 µs) |
| Code in cartridge ROM, workspace in pad | 1 (the fetch) | **18 cycles** |
| Code *and* workspace in expansion RAM | 4 | **30 cycles** (10 µs) |

Same instruction, same bytes, 2.1× spread — Chapter 2's law of addresses, now derived rather than asserted. And your Lab 2 measurement slots straight in: `JMP` is C=10, M=1, so pad gave you 10 and expansion RAM 14, which is precisely the row you recorded. (One honesty note for bench users: the table above is the *hardware's* arithmetic, from the datasheet. Our emulator currently executes `MOV`'s write without performing the read-before-write, so it prices the all-expansion row at 26, not 30 — a logged fidelity gap the project tracks, and the first entry in Chapter 5's discrepancy hunt. Instruments are subject to calibration too; that is a lesson, not an apology.) Two budget reflexes to install alongside the model. First, the *fetch always counts*: even an instruction touching no data pays M≥1 for its own opcode, which is why hot **code** placement matters as much as hot data (Ch. 37's pad-loader trick exists for this). Second, scale intuition to the frame: at 60 Hz the machine gives you about 50,000 cycles per frame (Ch. 17's contract), so the all-expansion `MOV` costs 0.06% of a frame and a 768-iteration screen-clear loop costs — well, that's Exercise 4.7's opening act. Wait states from the *ports* (VDP and GROM have pacing rules of their own) and the exact per-device figures are Chapter 5's measured business; the model above, plus Appendix A's C/M tables, will carry you to it.

## 4.9 The 9900 Among Its Peers

The honest scorecard, with the era's usual suspects. ("Fast op" = the CPU's cheapest register-to-register move/add, at each chip's typical early clock — a crude yardstick, calibrated in the notes.)

| CPU | Year | Data width | Registers | Address space | Fast op, approx. |
|---|---|---|---|---|---|
| Intel 8080 | 1974 | 8 | 7×8-bit | 64 K | ~2.5 µs |
| MOS 6502 | 1975 | 8 | A,X,Y (+zero page) | 64 K | ~2–3 µs |
| Zilog Z80 | 1976 | 8 | 8×8 + alternates | 64 K | ~1 µs |
| **TMS9900** | **1976** | **16** | **16×16, in RAM** | **64 K** | **~4.7 µs (pad)** |
| Intel 8086 | 1978 | 16 | 8×16 | 1 M (segmented) | ~0.4 µs |
| Motorola 68000 | 1979 | 16/32 | 16×32 | 16 M | ~0.5 µs |

Read it in three passes. **As a 1976 machine**, the 9900 stands alone: the only true 16-bit micro on the table for two more years, moving 16-bit data in one operation that costs its 8-bit rivals two or three, comparing signed *and* unsigned in one instruction, multiplying in hardware while the others call subroutines. Per raw cycle it looks slow — 14 cycles for a register move! — but each of those instructions is doing minicomputer-sized work; on byte-oriented tasks it runs roughly with a good Z80, and on 16-bit data it pulls ahead. (Spare a nod to the 6502's zero page while you're on that row: 256 bytes of specially-cheap low memory used as a pseudo-register file — the workspace idea's scrappy cousin, arrived at from the opposite direction, for the opposite reason: the 6502 made memory fast because registers were scarce; the 9900 made registers *of* memory because memory was assumed fast. The console, cruel as ever, gave the 9900 exactly a zero page's worth of its assumption.)

**As a 1979 machine**, the ground shifts: the 8086 and 68000 arrive with the 9900's word size, several times its effective speed, and — the column that decided everything — *room*. Which brings the third pass, **the tragedy**: 64 K, flat, full stop. The 990 heritage included the controller's address space, and when software's appetite exploded past it, the 9900 family had no story — no segmentation kludge like Intel's, no clean vastness like Motorola's; the improved TMS9995 (Ch. 44's Geneve heart) stayed at 64 K, and the ambitious 99000 line arrived after the war was decided. Add §4.1's insularity and the sidebar's packaging costs, and you have the answer to §1.2's question in engineering terms: the 9900 lost the decade not to any flaw in its beautiful central idea, but to the two numbers on its birth certificate — 64 pins and 64 K — and a parent company facing inward. The idea itself never really lost; you'll spend Chapter 22 using context-switch machinery that mainstream CPUs wouldn't make this cheap again for years.

> **Sidebar — Sixty-Four Pins and Three Voltages.** Hold a 9900 and a 6502 side by side and you can *see* why one conquered the home and the other didn't, before reading a single spec. The 6502: 40-pin plastic DIP, one +5 V supply, clock generation on the chip, famously sold in the mid-two-figures. The 9900: a monster 64-pin ceramic package — among the largest DIPs ever in volume production, priced like it — demanding **three** supply rails (+5, −5, +12) and an external four-phase clock generator (the TIM9904, another chip on your bill of materials, another patch of board). None of this was vanity: 64 pins bought the full non-multiplexed 16-bit bus that made memory-to-memory addressing sing, and the power/clock demands were 1976 NMOS reality for a die this ambitious. But every design-in started three chips and two voltage regulators behind the competition, and inside TI it made the hunt for cheap 8-bit-bus derivatives (the doomed 9985, the hobbled 9980 — §1.2–1.3) a matter of survival rather than preference. The console you own is the fossil of that hunt: a 64-pin aristocrat, full bus intact, wired into an economy car — with 256 bytes of the manor grounds preserved behind the house.

> **Pitfalls.**
> - **The `>0041` bug.** Byte constants belong in the *high* half: `LI R1,>4100` (or `LI R1,'A'*256` `[libre99asm]`). Signature of the miss: zeros where characters should be, and `MOVB` piously shipping `>00`.
> - **Clobbered R11.** A `BL` inside a `BL` overwrites the only return address there is. Signature: the inner routine returns fine, the outer "returns" into hyperspace. Non-leaf routines save R11 *first* — no exceptions, even in ten-line experiments.
> - **Word ops at odd addresses.** No fault is raised; A15 is silently ignored, and you operate one byte early. Signature: "impossible" values that look byte-shifted; a `TEXT` of odd length just before a `DATA`. Keep `EVEN` handy and labels aligned.
> - **`MOV` between compare and jump.** `MOV` (and almost everything) rewrites L>/A>/EQ, so any instruction slipped between the test and its jump destroys the evidence. Signature: a branch that "ignores" its compare. (`SWPB` and the flag-silent shortlist of Ch. 8 are the only safe intruders.)
> - **Indexing by R0.** `@TABLE(R0)` isn't slow — it isn't *that instruction at all*; the encoding means symbolic mode, and assemblers (libre99asm and xas99 alike) reject the syntax. Reflex: index registers start at R1.
> - **Running "just for a second" with interrupts on.** Until Chapter 22 hands you the rules, an enabled console ISR will happily run against whatever pad state you've been squatting on. `LIMI 0` early — HELLO's very first instruction — is the training-wheels posture for all of Part II.

## Lab 4 — The Paper Machine

*Goal: execute the 9900 in your head — PC, WP, ST, and memory, every step — then let the bench grade you. This is the one lab in the book where slower is better; the students who rush it repay the time in Chapter 8 with interest.*

**The subject.** Six traced instructions inside minimal scaffolding — `src/trace.a99` in the Ch. 3 skeleton (build the cartridge: `libre99asm src/trace.a99 --name TRACE --format bin -o build/TRACEC.bin --listing build/TRACE.lst --symbols build/TRACE.map.json`):

```asm
* TRACE — Lab 4: six instructions under oath          (Ch. 4)
* Runs as a cartridge; the workspace lives in the scratchpad so we
* can watch the registers as plain RAM (BENCH99: `r`, then `m 8300 32`).

WS     EQU  >8300            the workspace: sixteen words of honest, fast RAM

START  LIMI 0               scaffolding: quiet machine
       LWPI WS              scaffolding: registers where we can SEE them
       LI   R3,>AAAA        scaffolding: a stage prop in R3
* --------- the six, under oath ---------
T1     LI   R1,>0005
T2     LI   R2,>FFFB        (that's -5, wearing its unsigned coat)
T3     A    R1,R2
T4     MOVB R1,R3
T5     SWPB R1
T6     BL   @HALT
* ---------------------------------------
HALT   JMP  HALT
       END
```

(Chapter 3's version of this file put the workspace behind an `EQU` rather than a `BSS` for a reason worth one sentence: a cartridge's bytes live in *ROM*, and a workspace must be *writable* — so the registers go to the scratchpad, and the lab gets the fast island's timing into the bargain. The E/A-era build of this program reserved workspace RAM inside the program with `BSS` because it *loaded into* RAM — a distinction Chapter 6 makes physical.)

**Part A — predict (paper only).** Rule a worksheet: one row per instruction T1–T6; columns for R1, R2, R3, R11, and the five flag bits L>, A>, EQ, C, OV *after* the instruction (write `–` for "unchanged"). Fill every cell from this chapter alone. Then answer three essay-lets underneath: (1) What exact value lands in R11 at T6, and why is it equal to something else visible on your sheet? (2) Which bits set at T3 are still set after T5, and what does that persistence imply about writing tests far from their jumps? (3) R3 after T4 is neither `>AAAA` nor `>0005` — derive its exact value from two different rules in §4.4.

**Part B — verify.** Assemble with the listing and symbol map (the build line above; per R-14 you always generate them — you'll need `START`'s and `T1`'s real addresses, and the map file hands them to you). On the bench: `load build/TRACEC.bin`, `pc` to `START`, then `u` to `T1`'s address and `s 6` — six trace lines, each priced in cycles with the flags decoded after it. Grade your worksheet cell by cell against `r`. Then the demonstration this whole chapter has been building to: `m 8300 32` — the *same sixteen words, two costumes*. R1's `>0500` is sitting at `>8302`; R11's copy of `HALT` is at `>8316`. Watch a register change *in the memory dump* as you step, and you will never again confuse "register" with "on the chip." Where the bench disagrees with your sheet, don't just correct the cell — write the one-line rule you'd violated (there are only about five candidates in this chapter).

**Part C — one workspace becomes two.** With the machine spinning at `HALT`, type `wp 8308` and then `r`: your old R4 is now "R0," your old R11-that-equals-HALT is now "R7," and sixteen new registers exist that mostly overlap sixteen old ones. Step the `JMP` a few times (`s 3`); nothing objects. Write three sentences on what, exactly, a "register" turned out to be — then `wp 8300` to put it back like a good citizen.

*Deliverables: the graded worksheet with rule-corrections, the three essay-lets, Part C's three sentences. Into the lab journal; Chapter 9's coroutine lab opens by asking for your Part C.*

## Exercises

**4.1 ✦** WP = `>8320`. Give the addresses of R7, R11, and R13 — then the register number that lives at `>833E`. (Do these until they're instant; Part V assumes it.)

**4.2 ✦** Memory holds `>C0` at `>A014` and `>DE` at `>A015`. Predict R5 after each, separately, given R5 = `>1111` before: (a) `MOV @>A014,R5` (b) `MOVB @>A014,R5` (c) `MOVB @>A015,R5` (d) `MOV @>A015,R5` — and explain (d)'s address arithmetic in one clause.

**4.3 ✦** Paper-trace flags only: `LI R1,>8000` / `A R1,R1` — predict L>, A>, EQ, C, OV and say which two bits *disagree about the same event*, and why that disagreement is the entire reason ST has both.

**4.4 ✦✦** Write the missing jumps as two-line idioms: signed ≥ (branch to `YES` if R1 ≥ R2, signed) and unsigned ≤ strictly-below-else pattern of your choosing. Assemble both inside the skeleton to prove syntax; comment each with its truth condition in flag terms.

**4.5 ✦✦** Design the workspace overlap of §4.6's `BLWP` note: caller's workspace at `>8300`; place the callee's workspace so that its R13–R15 land exactly on the caller's R0–R2. Show the arithmetic, state the callee's WP value, and name one thing that makes this trick dangerous enough to be a Ch. 9 topic rather than a habit.

**4.6 ✦✦** Using `STWP` and one loop, write and assemble a routine that copies all sixteen current registers into a 32-byte buffer `SAVE` — while running from those registers. (Place the buffer with an `EQU` at a writable address, as Lab 4's workspace was; saying *why* a cartridge's `BSS` couldn't serve is part of the exercise.) (Hint: the copying loop's own counters are among the things being copied; decide whether that's a bug or a feature, and comment your verdict.) `[console-only]` friendly.

**4.7 ✦✦✦** Frame-budget accounting, using premises you may take on faith today (audited in App. A): `MOVB Rs,@addr` C=14/M=4; `DEC` C=10/M=3; `JNE` C=10 taken, 8 not, M=1. Cost HELLO's 768-iteration clear loop per iteration and in total, twice: everything in pad versus code in expansion RAM with workspace in pad — as cycles, milliseconds, and *frames* (50,000 cycles each). Conclude with one sentence on why Chapter 13's screen routines will not look like HELLO's.

**4.8 ✦✦✦** The die-budget essay (one page): you may delete exactly one 990 inheritance from the 9900 — workspaces, full general addressing, or the 64-pin full bus — and spend the savings on any one improvement, holding 1976 feasibility constant. Argue your trade against the two you rejected, citing at least three specific consequences from this chapter's sections.

## Further Reading

- **TMS9900 Microprocessor Data Manual** — now it's assigned: the architecture section and the instruction descriptions behind §4.5–4.7; the cycle tables you'll live in from Chapter 7. Read it beside this chapter once; you'll be surprised how much is now legible.
- **A 990 family reference** (the 990/4 or /10 processor manuals circulate in archives) — an afternoon's skim that turns §4.1 from assertion into recognition; the console's CPU documented as what it was born as.
- **Editor/Assembler manual**, processor-overview chapter — the period's own telling of today's material, terser and colder; calibrate against it.
- **Thierry Nouspikel's Tech Pages**, CPU section — the community's annotated model, including corner-case behaviors the datasheet mumbles about.
- Forward: **Appendix A** (drafting alongside Ch. 7–9) is this chapter's reference-card afterlife; **Chapter 5** takes today's timing model into the laboratory.

## Summary

- Programmer-visible state = **PC, WP, ST — nothing else**; R0–R15 are RAM at **WP + 2n** (`LWPI`/`STWP`, `LIMI`/`STST`; PC read only via `BL` idiom). Interrupt vectors `>0000`–`>003F`, XOP vectors `>0040`–`>007F` (WP,PC pairs).
- Reserved seats: R0 shift-count / **never an index**; R11 `BL` link (+ XOP source EA); R12 CRU base; R13–R15 context linkage (old WP/PC/ST land in the **new** workspace; `RTWP` inverts; callee reads caller's Rn at `@2n(R13)`). Book convention declared: **R10 = software stack pointer** (built Ch. 9).
- Consequences (§4.3): context switch = pointer swap (`BLWP`/interrupt/`XOP` are one mechanism); **registers have addresses** (self-referential idioms live); **every register touch is a memory access** — incl. the dest-read-before-write habit (MMIO implications owed to Ch. 12).
- Memory model (§4.4): 32 K aligned words, A15 ignored on word ops (silent!), **big-endian**; **byte ops on registers use the HIGH byte** — constants ride high (`>4100`, `'A'*256` `[libre99asm]`), `SWPB` rescues (and sets no flags), byte writes preserve low bytes, `*Rn+` steps by operand size.
- ST (§4.5): L>/A>/EQ/C/OV/OP/X + mask (ST12–15). Compares grade **S vs D**; everything else grades **result vs 0** (⇒ `MOV` is a free test). Jump grammar: H/L unsigned, GT/LT signed; **no single-op signed ≥/≤** (inversion or pair idioms); C = no-borrow after subtract; `DEC`/`JNE` loops as in HELLO.
- No stack (§4.6): `BL`→R11 depth **one** (`RT` = `B *R11`); non-leaf saves R11 first; `BLWP` = heavy modular call; stacks are Ch. 9 equipment, not ambient.
- Formats (§4.7): five addressing modes (R, `*R`, `*R+`, `@sym`, `@sym(Rn≠0)`) on **both** operands of Format I = memory-to-memory philosophy; jumps reach ±~127 words; immediates are the exception, not the idiom; full encodings → App. A.
- Timing (§4.8): **T = C + 4 × (accesses in 8-bit domain)**; `MOV R,R` = 14c/4a → 14/18/30 by placement (datasheet truth; our emulator's missing MOV dest-pre-read makes it measure 26 in the all-slow row — deviation ledgered for Ch. 5); `JMP` = 10c/1a (matches Lab 2's bench numbers); fetches always count; 50,000-cycle frames scale intuition.
- Peers (§4.9): alone at 16 bits in 1976; per-op slow but per-op big; undone by 64 K + 64 pins + insularity; 6502 zero page = the workspace idea inverted. Lab 4: paper-trace T1–T6 (incl. R11=HALT, flag persistence, `MOVB` high-byte verdict), graded on BENCH99 (`u`/`s 6`/`r`) with registers watched *in memory* (`m 8300 32` — WS is an `EQU >8300` now that the program is a cartridge); Part C reseats WP mid-flight (`wp 8308`). New ruling R-11 (exercises may cite App-A premises).
