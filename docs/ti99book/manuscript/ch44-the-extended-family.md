# Chapter 44 — The Extended Family

<!-- Part X — Beyond the Console · target ≈14 pp -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — pending review passes. Survey chapter (Part X); shelf-heavy and honest per R-12: the project emulates the plain 9918A / TMS9900 4A only — the 99/8, Geneve 9640 (TMS9995/V9938/MDOS), 99000 family, F18A GPU, and GCC-for-TMS9900 are described from the hardware+community record and run on named shelf tools (MAME/js99er/Classic99, the tms9900-gcc repos), NOT the project emulator. External figures verified against the sources named in Further Reading and hedged per R-2. No code artifact of the project's own (the lab compiles/runs on shelf tools). Applies R-12/R-2. -->
<!-- SPEC: 00-master-outline.md, "### Chapter 44 —" (lines 679–685). -->

## The Machine on the Loading Dock

There is a photograph, in the collective memory of this community if not on any
wall, of a machine that never shipped: the TI-99/8, sitting finished — or nearly
so — in early 1983, while upstairs the decision was made to leave the home-computer
business entirely. It had a faster processor, far more memory, a built-in
peripheral bus, and a BASIC grown up from the one the 4A shipped with. It was the
4A's future, and it was cancelled on the loading dock. Reportedly only a handful
of prototypes survived.

That machine is the doorway to this chapter, because it tells you something the
four hundred pages behind us could not: the TI-99/4A was not an endpoint. It was
one member of a *family* — a lineage of chips and machines that ran before it,
beside it, above it, and, remarkably, long after it, in hardware its designers
never imagined. Knowing that family is not trivia. It is how you learn which of
the skills this book gave you are *portable* — true of the whole TMS9900 world —
and which are *parochial*, true only of the exact console on your desk. This
chapter is a tour of the relatives, and a running question at every stop: **what
that you already know still works here, and what doesn't?**

---

## What You Will Learn

After this chapter you can:

- **Place the 4A in its hardware family** — the cancelled 99/8 above it, the
  Geneve 9640 beside it, the 99000 chips that were meant to come next — and say
  what each changed.
- **Predict what ports "up" the family**: which 4A code runs unchanged on a
  TMS9995 or a Geneve, and which assumptions (timing, the plain 9918A, 16-bit
  memory) break.
- **Survey the other languages** that ran on this iron — UCSD Pascal, Forth, and
  C, historical and modern — and know when each was the right tool.
- **Name the modern C path**: cross-compiling with the community's GCC for the
  TMS9900 and calling hand-written assembly from it.
- **Program the F18A** — the modern FPGA video processor — and do one thing on it
  that the stock 9918A simply cannot.

## The Bridge: Every Platform Has a Family Tree

The last chapter taught you to port a design by separating its essence from its
medium. This chapter applies the same eye to *your own knowledge*. Everything you
learned is some mixture of two things: facts about the TMS9900 and the general art
of small-machine programming (portable — they travel to every relative in this
chapter and far beyond), and facts about the specific 4A console — its exact
memory map, its plain 9918A, its 3 MHz-with-wait-states timing (parochial — they
stop at the console's edge). A working programmer always knows which is which,
because that knowledge is exactly what makes a skill transferable. Meet the family
and the line between the two draws itself.

> **Where the project stops (R-12).** The Libre99 emulator models the **plain
> 9918A, TMS9900 4A** and nothing above it — no TMS9995, no Geneve, no V9938, no
> F18A. So this chapter is honest survey, not project demonstration: every
> relative here is described from the hardware and community record and *run*, if
> you want to run it, on the shelf tools named as we go (MAME emulates the
> Geneve; js99er and real hardware host the F18A; the community's GCC targets the
> chip). Verify a tool's current state before you lean on it; this community's
> software moves.

## 44.1 The 99/8 and the Hexbus World

The TI-99/8 was the console's intended successor, in prototype by early 1983 and
killed with the rest of TI's home line. Its heart was a **TMS9995** — the same
9900 instruction set you have spent this book mastering, but with an on-chip
improvement that matters enormously: a small block of on-chip RAM and a 16-bit
internal path, so that the "fast island" the 4A had to *find* in the scratchpad,
the 9995 partly *carries with it*. Code you wrote for the 4A's instruction set
runs on it; code you *tuned* for the 4A's specific wait-state penalties (Ch. 37)
must be re-measured, because the memory system underneath moved.

Around it, TI was betting on **Hexbus** — a serial peripheral bus meant to replace
the 4A's side-car expansion with something cheaper and smaller, the same intent
that later gave the world every thin serial link. The 99/8 is worth knowing not
because you will program one — you almost certainly never will — but because it is
the clearest statement of what TI thought the 4A's problems *were*: too little fast
RAM, too expensive to expand. Every relative that follows is an answer to those
same two complaints.

## 44.2 The Geneve 9640: Porting Knowledge Upward

If the 99/8 is the future that didn't ship, the **Geneve 9640** is the future that
did — built not by TI but by the third-party firm Myarc, released in 1987, and
still supported by its community today. It is the most important machine in this
chapter, because it is the one a 4A programmer actually *grows into*, and because
it shows exactly how 4A knowledge ports upward. Its specification, as the community
record documents it, reads like the 99/8's wish list granted:

| | TI-99/4A | Geneve 9640 |
|:--|:--|:--|
| CPU | TMS9900 @ ~3 MHz (16-bit bus over 8-bit) | TMS9995 @ 12 MHz |
| Memory | 256 B fast pad + up to 32 KB expansion | ~640 KB (main + video) |
| Video | TMS9918A (our whole Part III) | Yamaha **V9938** — 256 colors, 80 columns |
| Sound | SN76489 | SN76496 (the same voice, one family up) |
| Software | GPL + console ROM | **MDOS**, a real disk OS with mouse and GUI |

Read that table with a porter's eye (Ch. 43) and the transfer is precise. The CPU
is *compatible* — your instruction knowledge, your calling convention (R-16), your
`lib99` math and memory routines run unchanged, only faster. The sound chip is a
sibling — your `sndlib` idioms carry. But the video is a **different processor**:
the V9938 is a superset the 4A never had, with more colors, more resolution, and
an 80-column text mode, and every routine in `textlib`, `bmplib`, and `spritelib`
that touched the 9918A's exact registers and table layout must be **rewritten
against the 9938's**, not merely recompiled. And `MDOS` — designed to emulate the
4A while offering virtual memory, a mouse, and a GUI — means the whole console-ROM
world of Part VI is now a *compatibility layer*, not the ground floor.

The lesson generalizes to every "port up" in computing: **the CPU travels, the
peripherals do not.** Arithmetic is portable; the exact bits of a video chip are
parochial. A Part IX capstone would run on a Geneve with its logic intact and its
entire rendering layer replaced — which is, once again, the engine-versus-content
line this book keeps finding.

## 44.3 The 99000 Family and the Road Not Taken

Behind all of this sat a chip family TI meant to carry the architecture into the
1980s and beyond: the **TMS99000** line (the 99105, the 99110, and kin) — 9900
descendants with, most intriguingly, *macrostore*, a facility for defining new
instructions in on-chip microcode, so the instruction set itself could be
extended. It is the road not taken. TI's exit from home computers, and the
industry's stampede toward the 8086 and the 68000, left the 9900 lineage a
side branch — beloved, capable, and commercially over almost before it began.

Why learn a dead branch of a dead tree? For the same reason you learn any history:
to see that the architecture you now know intimately was not primitive or
inevitable but a *choice*, with descendants that pointed somewhere real. The
9900's memory-to-memory design, its workspace-in-RAM registers (Ch. 4) — the very
things that make it feel alien to a programmer raised on register-file machines —
were a considered bet about where computing would go. It went elsewhere. But the
bet was coherent, and holding it in your head is part of holding the whole machine
in your head.

## 44.4 Other Languages on the Iron

Assembly and GPL are this book's subject, but they were never the only languages
on the 4A, and the survey is worth taking — because each alternative is a
different answer to "how much of the machine do you want to hold at once?"

- **UCSD Pascal**, via the p-code card, ran a portable *virtual machine* on the
  4A: you wrote Pascal, it compiled to stack-based p-code, and an interpreter on
  the card ran it. It is the ancestor of every bytecode runtime you use today —
  Java's JVM, Python's, the browser's — and it made the same trade they do:
  portability and structure, bought with a layer of interpretation between you and
  the silicon.
- **Forth** — TI Forth in 1983, and the community's later **fbForth** and
  **TurboForth** — went the opposite way: a tiny, extensible, near-the-metal
  language that the small-machine world loved precisely because you could hold
  *all* of it in your head, dictionary and inner interpreter and all. Forth on the
  4A is the language equivalent of this book's ethic.
- **C** came in small-compiler form in the period (Small-C descendants for the
  9900), and comes today in a serious one: the community has ported **GCC to the
  TMS9900**, a real back end plus binutils, so you can cross-compile C on a modern
  PC and link it against hand-written assembly. Tursi's `libti99` gives that C
  access to the VDP, the sound chip, and the rest — the same jobs `lib99` does for
  our assembly. The modern workflow is: C for the breadth, assembly (this book)
  for the inner loops that C cannot make fast enough, linked together. Verify the
  toolchain's current state before printing a build line — these repositories
  evolve — but the capability is real and mature.

The through-line: every language here is a position on one axis, *how much
abstraction between you and the chip.* This book planted its flag at the far end —
none — so that you would understand the whole stack the others sit on. Knowing
assembly is what lets you use any of them wisely.

## 44.5 The F18A GPU in Earnest

Here is the twist the 4A's designers could not have foreseen: the most exciting
video hardware for the console arrived *decades* after it, from the community
itself. The **F18A**, created by Matthew Hagerty, is a field-programmable (FPGA)
drop-in replacement for the entire TMS9918A family — pin-compatible, so it goes
where the old chip was — that reproduces the original's behavior and then, having
reproduced it, keeps going. From the project's documentation, it offers a palette
of 64 registers drawn from **4096 colors** (against the 9918A's fixed fifteen),
*enhanced color modes* that give tiles and sprites up to eight colors each, **all
thirty-two sprites on a single scanline** with no flicker (retiring the
four-sprite law of Ch. 16 entirely), a 9938-style **80-column mode**, and crisp
VGA output — all while an unmodified Part III program still runs on it, because
the old chip is still in there.

And it exposes something genuinely new: a small **programmable GPU core** — a
9900-class processor living *inside the video chip*, able to manipulate VRAM
autonomously between the host's accesses. That is the thing to reach for in the
lab: a scanline effect the stock 9918A cannot do at all — say, a per-line palette
or scroll change that would demand impossible mid-frame CPU timing on a bare 4A —
becomes straightforward when a processor inside the VDP does the work in step with
the beam. The skills transfer directly: it is a 9900, so R-16 and everything you
know about the instruction set apply; only the peripheral — the VRAM it drives and
the beam it rides — is new. The F18A is where "hold the whole machine in your head"
gets a bigger machine, on your terms.

## Lab 44 — Meet a Relative

This lab runs on **shelf tools**, honestly, because the project emulator does not
model these machines (R-12). Scale each step to what you can actually run:

1. **Compile C for the chip.** Install the community's `tms9900-gcc` (verify its
   current build instructions first), compile a small C program that calls one
   `lib99`-style assembly routine you wrote by hand, and read the generated
   assembly. Where did the compiler spend cycles you would not have? That gap is
   why the inner loops in this book are hand-written.
2. **Run a capstone on a bigger machine.** Take one Part IX game and run it under
   an emulator that models a **Geneve** (MAME) or an **F18A** (js99er / real
   hardware). Write a one-page *port sheet*: what ran unchanged, what the different
   video chip broke, and what you would rewrite to make it native. That sheet is
   §44.2's lesson, done with your own hands.

If a tool is unavailable to you, do the paper version: from the specifications in
this chapter, predict the port sheet, and mark which predictions you could not
verify. Honest uncertainty, named, is worth more than a confident guess.

## Exercises

**✦ Warm-ups**

1. Name one thing about a Part IX capstone that would run **unchanged** on a
   Geneve, and one that would **break**, and say which chip each depends on.
2. The 99/8 and the Geneve are both answers to the same two complaints TI had
   about the 4A. What were the complaints, and how did each machine answer them?
3. UCSD Pascal and Forth sit at opposite ends of one axis. Name the axis, and say
   which end this book chose and why.

**✦✦ Building**

4. Write a C function and the hand-assembly routine it calls (the calling
   convention is the community toolchain's, not necessarily our R-16 — note the
   differences). What must each side agree on for the link to work?
5. From the 9938's published capabilities, sketch how you would re-implement one
   `spritelib` routine natively on a Geneve. Which parts of the interface stay and
   which change?
6. Design an F18A GPU-core routine (in prose or pseudo-9900) that does a per-scanline
   effect impossible on the stock chip, and explain what the inner core is doing
   *between* the host's VRAM writes.

**✦✦✦ Reach**

7. Take the constraint-translation table from Ch. 43 and run it **backward**: given
   a Part IX 4A game, produce the table you would fill to port it *up* to a Geneve —
   what each 4A decision becomes when the machine is faster and the video richer.
   Which rows get easier, and which get harder because there is now more to manage?

## Further Reading

- The **Geneve 9640** entries on Ninerpedia and Wikipedia, and Myarc's user manual
  in the community's WHTech archive, for the specifications summarized here.
- The **F18A** documentation and Matthew Hagerty's own project write-ups (and the
  AtariAge F18A programming threads) for the palette, enhanced color modes, and the
  GPU core.
- The community **GCC for the TMS9900** repositories (the `tms9900-gcc` ports) and
  Tursi's **`libti99`**, for the modern C path; verify current build state before
  relying on any invocation (R-12).
- **Appendix N** collects these with provenance notes; **Chapter 43**'s
  constraint-translation table is the tool this chapter's lab turns upward.

## Summary

The TI-99/4A was a member of a family, and knowing the family is how you learn what
your skills are really worth. Above it stood the **TI-99/8**, its cancelled
successor — a TMS9995 with on-chip fast RAM and a Hexbus, the future left on the
loading dock in 1983. Beside it lives the **Geneve 9640**, the future that shipped:
a 12 MHz TMS9995 with 640 KB, a Yamaha V9938 video chip, and the MDOS operating
system — the machine a 4A programmer grows into, and the clearest lesson in what
ports upward (the CPU, the arithmetic, R-16, `lib99`'s math) and what does not (the
exact video chip, and with it all of Part III's register-level code). Behind them
was the **99000** family and its macrostore, the road the architecture didn't get
to take. The 4A ran **other languages** too — UCSD Pascal's portable p-code, Forth's
hold-it-all-in-your-head minimalism, and C, in period Small-C and in today's
community **GCC for the TMS9900** linked against hand assembly — each a different
answer to how much abstraction you want between yourself and the chip. And ahead,
impossibly, sits the **F18A**: a modern FPGA video processor that is the old 9918A
and then far more — 4096 colors, no sprite flicker, 80 columns, and a programmable
9900-class GPU core inside the video chip, on which you can do things the stock
console never could.

None of these run on this book's own emulator, which models the plain 4A and says
so (R-12); they run on the shelf, and the chapter is honest survey. But the point
is not to run them. It is to see the console you now know completely as one node in
a lineage — to hold not just the machine in your head, but its whole family — and
to carry the portable half of everything you have learned outward, to every
relative and every 9900 that ever was.
