# Chapter 10 — The CRU: TI's Serial Nervous System

*The memory bus moves data by the word. The CRU moves it one bit at a time — and that single bit,
addressed through a register and set with a single instruction, is how this machine touches the
whole outside world: every key, every joystick, every relay, every card in the expansion box. It
is the strangest bus on the machine to modern eyes, and the most physical.*

<!-- Part II — The TMS9900 and Assembly Fundamentals · target ≈14 pp -->
<!-- STATUS: DRAFTED (session 5, 2026-07-06) — pending review passes. All three listings assemble via sh verify.sh; §10.1–10.6 + Lab machine-verified on BENCH99 at commit 84c585a; code in code/ch10/. Verdict files green (keyscan/crudemo/cruscan → VDP R7=>02): keyscan decodes 12 keys (A→>41 … Q→>51) via injected 9901 row bytes — BENCH99's `k` (boot mode) and single-step (bare mode) are mutually exclusive, gap noted; crudemo passes 7 SBO/SBZ/TB/LDCR/STCR tests on the disk-card latch at >1100 (a round trip needs latched bits — 9901 outputs don't read back); cruscan finds 0 DSR ROMs (none installable from the bench — gap) but confirms the >1100 disk-card CRU decode answers. R12 = 2× bit verified (>1100↔bit >0880, >0024↔bit 18). SURFACED to project (R-12): no keypress-into-user-code path on the bench; no DSR ROM installable on the bench; 9901 cassette-relay output unemulated (the vignette's click is real-hardware only). -->
<!-- SPEC: 00-master-outline.md, section "### Chapter 10 —" (lines 282–290 in outline v1.1). That bullet list is this chapter's contract. -->

## The Click

A converted sleeping porch on the back of a house in Austin, Texas, in the damp warmth of an early
March night in 1984 — a card table, a gooseneck lamp, the console wired to a small color set that
had been the kitchen's until it was replaced. The programmer here had a day job writing COBOL for
the State of Texas and a night habit that was slowly taking over the porch, and tonight he was
stuck on something that should not have been hard: he wanted to read the keyboard.

He had spent a week in the memory map by now. He knew, cold, that the VDP lived at >8800 and
>8C00, that the sound chip answered at >8400, that you talked to the whole machine by moving words
to and from magic addresses. So he had gone looking, reasonably, for the keyboard's address — the
port you read to find out which key was down. There wasn't one. He read the Editor/Assembler
manual's chapter on the keyboard twice and it kept talking about something called the CRU, and
about a chip called the 9901, and about *bits* — not bytes at an address, but individual bits, each
with its own number, set and cleared and tested one at a time by instructions he had skimmed past
as exotic and never used: SBO, SBZ, TB. There was no keyboard port because there was no keyboard
*byte*. The keys were bits on a bus he had not yet met.

What finally made it real was not the keyboard. It was the relay. Deep in the manual's tables, in
the list of what lived at which CRU bit, was the cassette motor control — a single bit that, set to
one, closed a physical relay inside the console to start a tape recorder's motor. He almost skipped
it. Then he thought: I can *hear* that. He set R12 to the 9901's base, and typed the smallest
program he had written all week — set the bit, wait, clear the bit, wait, loop — and ran it.

The console clicked. A real, mechanical, tactile *click*, a tiny relay closing and opening somewhere
under the beige, once a second, patient as a clock. He sat there in the lamplight and listened to a
computer keep time with a relay, and the CRU stopped being exotic. It was not a mysterious second
memory. It was a bank of switches and sense-lines, four thousand of them, each with a number, and
five instructions to work them — set one, clear one, test one, and move a handful at once. The
keyboard was just more switches. The joystick was switches. Every card in the expansion box out in
the garage was switches and sense-lines on this same bus. He had been looking for a port. The
machine had been offering him a nervous system.

He let it click for another minute before he stopped it, because it was the most satisfying thing
the machine had ever done — the first output he could feel. Then he set R12 back to the keyboard's
columns and got to work. This chapter is that porch, compressed: what the CRU is, how R12 aims it,
the five instructions that work it, and — by the end — the keyboard read that started the search,
plus a tour of the switches every loaded TI-99/4A is quietly holding.

---

## What You Will Learn

By the end of this chapter you will be able to:

- Describe the CRU as a 4,096-bit address space of single-bit I/O, entirely separate from the 64K
  memory space, and name the five instructions that work it.
- Load R12 correctly to address any CRU bit, applying the ×2 relationship between the value in R12
  and the hardware bit number — and explain, once and for all, where the factor of two comes from.
- Set, clear, and test individual bits with SBO, SBZ, and TB, and state which of the three touches
  the status register.
- Transfer up to sixteen bits at once with LDCR and STCR, apply the byte-operand rule for
  transfers of eight bits or fewer, and predict the bit order the hardware uses.
- Read the console keyboard directly through the TMS9901 — select a column, read the rows, decode a
  keypress — and verify the decode on the bench.
- Read the CRU map of a running system: which device answers at which base, and how a peripheral
  card's ROM pages itself into memory through a single CRU bit.

## The Bridge: I/O That Isn't Memory

Everything you have done to the outside world so far, you have done by moving a word to an address.
That is *memory-mapped I/O*, and it is the model nearly every processor you have met uses: a device
register looks like a memory location, and you read or write it with the same instructions you use
for RAM. The TI-99/4A does plenty of this — the VDP, the sound chip, and the GROMs are all
memory-mapped ports, as Chapter 5 mapped and later chapters will drive.

The CRU is the other model, and it is older and stranger and, for a certain kind of job, better.
CRU stands for Communications Register Unit, and it is a completely separate address space — not a
region of memory, but a parallel world of 4,096 individually addressable *bits*, reached by five
dedicated instructions and one register. Where memory-mapped I/O gives every device a byte or a
word, the CRU gives every device as many single bits as it needs: one bit for a relay, one for a
lamp, eight for a keyboard column, a handful for a mode setting. You do not read a keyboard byte
and mask it; you address the exact bit you care about and test it. If this sounds like the wiring
of an industrial control panel — hundreds of independent switches, sensors, and outputs, each its
own line — that is exactly its heritage, and the sidebar at the end of this chapter tells that
story. For now, hold the two models side by side: memory-mapped I/O is a set of mailboxes at
addresses; the CRU is a wall of four thousand labeled light switches. This chapter teaches you to
find a switch by its number and flip it.

## 10.1 A Wall of Four Thousand Switches

The CRU is defined by three facts, and the rest of the chapter is consequences. First: it is a
**bit-addressable** space. The unit of address is one bit, not one byte, and there are 4,096 of
them, numbered 0 through 4095. Second: it is **entirely separate from memory**. A CRU bit and a
memory address that happen to share a number have nothing to do with each other; the CRU is reached
only through its own instructions, and those instructions never touch RAM. Third: it is **worked by
exactly five instructions** — three that handle one bit at a time, and two that move a small group
of bits in a single operation. That is the whole architecture. There is no CRU "port" to read as a
word, no CRU "register" to load; there is a numbered bit, and there are instructions that set it,
clear it, test it, or move a run of its neighbors.

What lives on those 4,096 lines is up to the hardware. On a bare console the low end of the space
belongs to the TMS9901, the "programmable systems interface" chip that owns the machine's most
essential switches: the keyboard matrix, the joystick lines, the cassette relays and audio gate,
the interrupt mask, and an interval timer. Higher up the CRU space, each peripheral card in the
expansion box claims a block of bits at a fixed base — the disk controller, the RS-232 card, the
p-code card — and uses them both to control the card and, through one special bit, to switch the
card's ROM in and out of the memory map (§10.6). The CRU is where the console ends and the world
begins. Every device that is not memory-mapped is here, and to reach any of it you first have to
learn to aim.

## 10.2 Aiming the CRU: R12 and the Factor of Two

You aim the CRU with **R12**. Chapter 4 reserved it for exactly this purpose, and this is the
chapter that spends it: R12 holds the **CRU base address**, and the single-bit instructions address
a bit *relative* to that base. Set R12 once to point at a device, and a whole run of that device's
bits is a small signed displacement away — which is precisely how you walk a keyboard's columns or
a card's control bits without reloading R12 each time.

Now the one detail that has confused every newcomer to this machine for forty years, stated plainly
so it confuses you only once. The CRU bit number does not sit in the low bits of R12. It sits in
**bits 3 through 14** — the middle of the register — which means the *value you load into R12 is
twice the bit number you want*. To address CRU bit 0 you load R12 with >0000; to address CRU bit 1
you load >0002; to address the keyboard's column-select bits, which begin at CRU bit >0012, you
load R12 with >0024. Twice the bit address. Every time.

> **Where the two comes from.** The hardware reads the CRU bit number from R12 bits 3–14, ignoring
> bits 0–2 and bit 15. A 12-bit number parked in bits 3–14 is the same number shifted left by one
> compared to parking it in bits 4–15 — so the register value reads as double the bit address. You
> can derive it, or you can memorize the rule (*R12 = 2 × bit*) and move on; period programmers did
> the latter, and so will you after the third time you forget and the assembler-of-record — your own
> bench — shows you a keypress landing in the wrong column.

The single-bit instructions add a **signed displacement** to the base in R12, and that displacement
is in *bit* units, not doubled — the doubling lives only in R12 itself. `SBO 2` sets the bit two
positions above the base; `SBO -1` sets the bit just below it. So a device driver loads R12 with the
card's base once and then reads and writes the card's individual lines by small constant
displacements, the base doing the addressing and the displacement doing the indexing. Aim once,
work many.

## 10.3 One Bit at a Time: SBO, SBZ, TB

Three instructions handle the single bit, and they are as simple as the machine gets:

- **SBO** *disp* — **set bit to one**: drive the CRU bit at (base + disp) high.
- **SBZ** *disp* — **set bit to zero**: drive it low.
- **TB** *disp* — **test bit**: read the CRU bit at (base + disp) and reflect it in the **equal**
  status bit, so the very next JEQ / JNE branches on the line's state.

SBO and SBZ are outputs; they are also, as Chapter 8's flag survey noted, **flag-silent** — setting
or clearing a CRU bit changes no status bit, so you can drive a line in the middle of a delicate
sequence without disturbing a pending branch. TB is the input, and it is the exception that proves
the rule: a test has to report somewhere, and it reports through EQ. Read a switch with TB, then
JEQ or JNE on the result; that two-instruction pair — test a bit, branch on it — is the atom of
every CRU input routine in the book.

The relay of this chapter's opening is three of these instructions and a delay: `SBO` the cassette
motor bit to close the relay, wait, `SBZ` to open it, wait, loop. The keyboard read of §10.5 is TB
and its cousins run across a matrix. `code/ch10/crudemo.a99` exercises the trio directly and checks
the result on the bench.

There is one subtlety that will save you an afternoon, and `crudemo` exists to make it concrete: on
the 9901, and on CRU devices generally, **reading a bit and writing a bit are often different
functions of the same number.** Writing CRU bit N drives an output; reading CRU bit N senses an
input; and the two need not be the same physical signal. Set the console's alpha-lock output with
SBO and then TB the same bit, and you do not read back what you wrote — you read whatever input line
shares that number, because the output latch and the input sense are different pins the chip merely
addresses alike. This is not a bug; it is what an I/O bus *is*. It means a CRU "round trip" — write a
bit, read it back, confirm — only works on bits that are genuinely *latched and readable*, and you
must know which those are.

On this machine the cleanly round-trippable bits belong to a peripheral card. `code/ch10/crudemo.a99`
drives the TI Disk Controller's control latches at CRU base >1100 (§10.6), where bit 0 — the card's
DSR-ROM enable — is a real read/write latch. Load R12 with >1100 and the seven checks run: `SBO 0`
then `TB 0` reads back one; `SBZ 0` then `TB 0` reads back zero; `SBO 7` sets an independent neighbor
(the side-select latch) while bit 0 stays clear, proving the bits are individual; `LDCR >0081,8`
drives eight bits at once and `STCR R2,8` reads them back as >0081, confirming both the transfer and
the low-bit-first order of §10.4. All seven pass and the border comes up green. The single-bit
instructions cost, in this machine's timing, **12 cycles each** for SBO, SBZ, and TB — cheap, because
a single CRU bit is a single serial transaction with nothing to compute.

## 10.4 A Handful at Once: LDCR and STCR

Setting bits one at a time is right for a relay, tedious for a keyboard column and impossible at
speed for a byte-wide port. So the CRU offers two multi-bit instructions that move a run of
consecutive bits between a CPU operand and the CRU in a single operation:

- **LDCR** *src,count* — **load** `count` bits *from* the source operand *to* the CRU, starting at
  the base.
- **STCR** *dst,count* — **store** `count` bits *from* the CRU *into* the destination operand,
  starting at the base.

The `count` is 0 through 15, where **0 means sixteen** — the same count-field convention the shifts
used in Chapter 8. Two rules govern the operand, and both catch the unwary. First, the **byte rule**:
if `count` is eight or fewer, the operand is a **byte**, and the bits ride the *high* half of it, by
the high-byte law of Chapter 7; if `count` is nine or more, the operand is a full word. Eight is the
hinge — a transfer of exactly eight bits is a byte operation. Second, **bit order**: the CRU fills
from the least-significant end. The first CRU bit (at the base) corresponds to the *low* bit of the
transferred field, the next CRU bit to the next bit up, and so on. Read eight keyboard lines with a
single STCR and the line at the base lands in the low bit of your byte, the line at base+7 in the
high bit — the reverse of how you might draw the matrix on paper, and the source of a classic
off-by-a-mirror bug when you decode the result.

LDCR and STCR are how you read a whole keyboard column in one instruction, configure an RS-232 card's
mode word, or shift a byte out to a peripheral. They are also flag-touching in one respect worth
noting for later: like the byte compares of Chapter 8, a CRU transfer whose width makes it a byte
operation participates in the parity accounting the datasheet describes; the book's uses read cleanly
and the corner is catalogued in Appendix A. The mechanics — count, byte-versus-word, low-bit-first —
are the whole of it, and the keyboard of the next section puts all three to work at once.

> **Sidebar — Why a Bit Bus? The 990's Industrial Blood.** The CRU did not begin on a home computer.
> It began on the Texas Instruments 990 minicomputer and the TMS9900's industrial forebears, machines
> built for the factory floor, the process controller, the instrument rack — worlds where a computer's
> job is to watch a few hundred sensors and throw a few hundred switches, each an independent line.
> For that work, a byte-wide, memory-mapped port is the wrong shape: you do not want to read eight
> unrelated sensors as a byte and mask out seven, you want to test *this* limit switch and set *that*
> solenoid, by name, without disturbing their neighbors. A bit-addressable I/O bus is exactly right —
> it is a patch panel with four thousand labeled jacks. When TI built a home computer around the 9900,
> the CRU came along in the family DNA, and so the machine that played Parsec reads its keyboard the
> way a 990 read a chemical plant: one labeled bit at a time. It is over-engineered for a game console
> and precisely why the TI-99/4A was so easy to bolt hardware onto — every expansion card just claimed
> its block of jacks on a bus built to be extended.

## 10.5 The First Real Device: Reading the Keyboard Through the 9901

The keyboard is where the porch's search ended, and it is the CRU's masterpiece of economy. The keys
are not wired one-per-line; they are an electrical **matrix** — columns crossing rows, a key at each
intersection shorting its column to its row when pressed. To read it, you drive one column at a time
and sense which rows respond, and both halves of that are CRU operations. `code/ch10/keyscan.a99`
does it by hand.

Selecting a column is an LDCR. Load R12 with **>0024** — the base for CRU bit >0012, which is bit 18,
the first of the 9901's three column-select output lines — and `LDCR Rx,3` drives a three-bit column
number (0 through 7) onto CRU bits 18, 19, and 20, least-significant bit first. Reading the rows is an
STCR. Load R12 with **>0006** — the base for CRU bit 3, the first row-sense line — and `STCR Rx,8`
shifts eight row bits (CRU bits 3 through 10) into a byte. Two conventions govern the byte you get,
and both are the §10.4 rules made real: the bits arrive **least-significant first**, so row 0 lands in
bit 0, and each line is **active-low**, so a pressed key reads **0** and an untouched one reads 1. A
byte of all ones means nothing is down in that column. Walk the columns, and the one byte that is not
all-ones tells you the column and — by which bit is zero — the row; the (column, row) pair indexes a
small ASCII table, and you have your key.

On the bench, `keyscan` decodes correctly across a spread of the matrix — `A` → >41, `Z` → >5A, `ENTER`
→ >0D, `0` → >30, `SPACE` → >20, `/` → >2F, `Q` → >51, and an unmapped modifier key to >00 — twelve
vectors, no failures, green border. A word on *how* that decode is verified, because it is an honest
seam. BENCH99 can inject a keypress with its `k` command only while the console firmware is running
(boot mode), whereas single-stepping your own program (bare mode) is a separate, mutually exclusive
mode — so you cannot, on today's bench, press a key *and* drive your own keyscan over it in one run.
The decode is therefore proven by feeding the routine the exact active-low row byte the 9901 produces
for each key — the value `NOT (1 << row)` — and confirming the mapping, a fact cross-checked three
ways: the live column scan on the bare bench runs the genuine 9901 CRU path and reads the idle
keyboard as all-ones; the emulator core's own test drives a full physical press (an `A` becomes
column 5, row 5, exactly the bits this routine selects and senses); and in boot mode the firmware's
own keyboard scan reads injected keys end to end. The routine's logic is proven; a single bench mode
that both injects a key and single-steps user code is a gap noted for the project (R-12). The console
ROM, of course, has its own keyboard scanner — KSCAN, which Chapter 23 dissects and most programs
simply call. We did it by hand here for the same reason the porch programmer did: to watch the CRU
work, and to know, when KSCAN eventually does it for us, exactly what it is doing.

## 10.6 The CRU Map of a Loaded System

The 9901 owns the low end of the CRU, but the rest of the space belongs to the expansion box, and it
is organized by a convention worth knowing before you meet the peripherals in earnest (Part VII). Each
card claims a block of CRU bits at a fixed **base** — the disk controller near >1100, the RS-232 card
near >1300, others out toward >1E00 (the full map is Appendix G). And every card shares one convention
at **bit 0 of its base**: `SBO 0` pages that card's **DSR ROM** — its driver, its Device Service
Routine (Ch. 30) — into the >4000 window in the memory map; `SBZ 0` pages it back out. That single bit
is how a machine with a 64K address space hosts a dozen cards' worth of driver ROM: only one card's
ROM occupies the >4000 window at a time, switched in by the CRU exactly when the console wants to call
it. A ROM that has paged itself in announces its validity with a signature byte — >AA — at the base of
the window, so software can tell a real DSR from empty space.

`code/ch10/cruscan.a99` walks the bases from >1000 to >1F00 in >0100 steps, pages each candidate in
with `SBO 0`, tests >4000 for the >AA header, and pages it back out. On the bare bench it finds **zero**
signatures — and that result is honest and worth reading correctly. No DSR ROM image is installed on
the bench, so the >4000 window reads as open bus at every base; there is nothing to find. But the scan
is not toothless, and it proves so with a positive control: at exactly one base — >1100, the disk
controller — CRU bit 0 behaves as a genuine read/write latch (set it, read it back as one; clear it,
read zero) rather than the idle-high nothing every empty base returns. The card's *CRU decode* is
present and answering; only its *ROM* is absent. On real hardware, or on the desktop `libre99`
with a disk controller mounted, that same >1100 answers with its >AA and its driver; installing a card's
ROM image on the bench is a capability the project has not yet built, and naming that gap (R-12) is more
honest than a demo that quietly loads a ROM the reader can't see. The full CRU map is Appendix G; what a
DSR *is* and how the console calls it is Chapter 30. Here we have simply learned to knock on every door
and hear which frames are real.

## Lab 10 — A CRU Explorer

A CRU explorer is a program that pokes, tests, and reports the state of individual CRU lines — the
software equivalent of the porch programmer's afternoon, made systematic. The bench-verifiable engine
of that explorer is the three programs this chapter built, and together they *are* the explorer's
core: `crudemo` is the bit-poker — set a bit, clear a bit, test a bit, move a run with LDCR/STCR;
`keyscan` is a live reader of the console's own switches; `cruscan` is the door-knocker that maps who
is home on the bus. Each proves itself on the bench with the green border you have trusted since
Chapter 7 — `crudemo` in 57 instructions, `keyscan` across twelve decoded keys, `cruscan` mapping the
sixteen peripheral bases — and each records its findings to the scratchpad for a bench script to read.

Two parts of the outline's explorer are honestly beyond the bench, and saying so is the lab. The
**interactive live display** — a screen that shows CRU bits updating as you poke them — is a job for
the desktop `libre99`, whose front end is where live interaction lives (see the project README
for running it); the bench is a scripted instrument, not an interactive one. And the **audible
cassette-relay click** — the very sound that opened this chapter — the project does not model at all:
the 9901's cassette-motor output bits are deliberately unemulated (a documented deferral on the
project's roadmap, R-12), so the bench can drive every bit of the CRU logic that *would* close the
relay and produce no sound, because there is no relay behind the bit. The click was real on the
Austin porch in 1984, on real hardware with a real relay; on the emulator it is a bit that goes high
and stays silent. That is not a disappointment to hide — it is the exact boundary of the model, and
knowing where your instrument stops measuring is itself a lab skill. Build the explorer's logic on the
bench where the logic lives; reach for the front end or real hardware for the light and the sound.

## Exercises

**✦ Warm-ups.**

1. To address CRU bit >0080 you load R12 with what value? To drive the keyboard's column-select lines,
   which begin at CRU bit 18, you load R12 with what value? State the rule you used in one sentence.
2. Write the three-instruction sequence that sets, then clears, then tests the CRU bit two positions
   above a base you have loaded into R12 — and name which of the three instructions changes the status
   register and which do not.
3. An `STCR` of exactly eight bits: is its operand a byte or a word, and which half of it carries the
   data? What about an `STCR` of twelve bits?

**✦✦ Consolidation.**

4. Using the disk card's latched bits at base >1100 (the method of `crudemo`), write a two-bit value
   with `LDCR` and read it back with `STCR`. Predict the byte `STCR` returns before you run it, then
   confirm on the bench, and explain the bit order.
5. A keyboard column read by `STCR Rx,8` returns the byte >EF. The row lines are active-low and arrive
   least-significant-first. Which row's key is down? Show your reasoning.
6. Explain, in three sentences, why writing and reading "the same" CRU bit number on the 9901 can
   touch different signals — and why `crudemo` demonstrates a round trip on the disk card at >1100
   rather than on one of the 9901's own output bits.

**✦✦✦ Extensions.**

7. Extend `keyscan` to recognize a two-key chord in one column (two rows down at once) and report it.
   Verify the decode against the active-low row byte the 9901 would produce for that pair — no live
   keypress required, per the chapter's method.
8. `cruscan` finds zero DSR ROMs on the bench yet proves the disk card's CRU decode answers at >1100.
   Describe the experiment that would show a *live* >AA responder: which tool you would run it on (the
   desktop emulator with a mounted controller, or Classic99), what you would expect at >4000 after
   `SBO 0`, and precisely why the bare bench cannot show it.
9. The interval timer lives on the 9901 too (Ch. 22 will use it). Read its datasheet entry in
   Appendix A as a premise, and sketch — in CRU instructions and R12 loads, without running it — how
   you would start it and read it back. Name the one thing about the 9901's mode bit you would need to
   verify on the bench before trusting your sketch.

## Further Reading

- *TMS9901 Programmable Systems Interface Data Manual*, Texas Instruments — the chip that owns the
  low CRU space: the keyboard-scan lines, the interrupt mask, the interval timer, and the cassette and
  audio control bits, each at its documented CRU number. The keyboard matrix wiring this chapter drove
  by hand is tabulated there.
- *TMS9900 Family Systems Design and Data Book* — the precise semantics of SBO, SBZ, TB, LDCR, and
  STCR, including the byte-versus-word operand rule and the cycle costs; catalogued for this book in
  Appendix A.
- *Editor/Assembler Manual*, Texas Instruments — the console's own KSCAN keyboard service and the
  utility conventions that most programs use instead of scanning the matrix themselves (Chapter 23).
- The Classic99 source (`console/tms9901.cpp` and the keyboard handling in `console/Tiemul.cpp`) — the
  hardware-verified cross-check for the 9901's CRU map and the keyboard matrix, consulted here to
  anchor the decode.
- Appendix G, *The CRU Map of a Loaded System* — the full census of which device answers at which base,
  the reference `cruscan` only samples.

## Summary

- The **CRU** (Communications Register Unit) is a 4,096-bit address space of single-bit I/O, entirely
  separate from memory, worked by five instructions. It is where every non-memory-mapped device lives:
  the keyboard, joysticks, cassette, interrupt mask, and every expansion card. Heritage: the 990
  minicomputer's industrial-control bit bus, inherited wholesale.
- **R12 holds the CRU base**, and the value in R12 is **twice** the hardware bit number (the address
  sits in R12 bits 3–14): to address bit >0012 you load R12 with >0024; to address the disk card's bit
  >0880 you load >1100. Verified on the bench. Single-bit ops take a signed *bit* displacement from the
  base.
- **SBO** / **SBZ** set and clear a bit (outputs, flag-silent); **TB** tests a bit into EQ (the input,
  the one that touches the status register). Costs: 12 cycles each. Crucial gotcha, proven by
  `crudemo`: reading and writing one CRU bit number can touch **different signals** — round trips work
  only on genuinely latched bits (the disk card at >1100, not the 9901's own outputs).
- **LDCR** / **STCR** move up to sixteen bits at once (count 0 = 16). The operand is a **byte** for
  eight bits or fewer (data in the high half, per the high-byte law), a **word** for nine or more; bits
  transfer **least-significant-first**. Verified: `STCR R2,8` → >0081, `STCR R4,12` → >0FFF.
- **Keyboard** (`keyscan`): the keys are a matrix; drive a column with `LDCR Rx,3` at R12=>0024 (CRU
  bits 18–20), read the rows with `STCR Rx,8` at R12=>0006 (CRU bits 3–10), active-low and LSB-first
  (a pressed key reads 0). Decode (column,row) through an ASCII table. Verified across 12 keys (A→>41…);
  the decode is proven by injecting the 9901's exact row byte, since BENCH99's `k` (boot mode) and
  single-step (bare mode) are mutually exclusive — a noted project gap. The ROM's own KSCAN does this
  for you (Ch. 23).
- **CRU map** (`cruscan`): each card claims a base (disk >1100, RS-232 >1300, … App. G); `SBO 0` pages
  its DSR ROM into the >4000 window (Ch. 30), `SBZ 0` pages it out; a valid ROM shows >AA at >4000. On
  the bare bench, zero ROMs are installed (honest none-found), but the disk card's CRU decode answers
  at >1100 (positive control). Installing a card ROM on the bench is a roadmap gap; the desktop emulator
  and Classic99 show live cards. Seeds: `inplib` (Ch. 21), the 9901 timer and interrupts (Ch. 22), DSRs
  (Ch. 30).
