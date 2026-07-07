# Chapter 11 — Craftsmanship: Style, Macros, Debugging, and `lib99`

*You now know enough of the instruction set to write anything. This chapter is about writing it
so that a human — your future self, most of all — can read it, trust it, and build on it. The
machine does not care about craftsmanship. Everyone who ever has to touch your code, including you
in six months, cares about nothing else.*

<!-- Part II — The TMS9900 and Assembly Fundamentals · target ≈22 pp -->
<!-- STATUS: DRAFTED (session 5, 2026-07-06) — pending review passes. syschk.a99 + includes assemble via sh verify.sh; §11.1–11.7 + Lab machine-verified on BENCH99 at commit 84c585a; code in code/ch11/, library contract in code/lib99/README.md. SYSCHK green (>8320=>C0DE, both RAM banks answer, VDP present + VRAM 16K wrap >4000, 0 DSR; VDP R7=>02); sabotage drill red (8K demand → FVERD >0001, R7=>06). libre99asm COPY verified SOURCE-FILE-RELATIVE (bare filename for a sibling include). New ruling R-17 (include architecture + lib99 conventions). SURFACED (R-12): emulator models no unexpanded console (RAMPRB negative case unreachable); no DSR ROM installable from the bench. -->
<!-- The debugging-method §11.4 worked examples reuse the book's own ledgered bugs (R11 clobber Ch.9; libre99asm left-to-right precedence Ch.7; pc-not-at-START-after-load Ch.5). Millers Graphics sidebar hedged (R-2). -->
<!-- SPEC: 00-master-outline.md, section "### Chapter 11 —" (lines 292–301 in outline v1.1). That bullet list is this chapter's contract. -->

## The Stranger Who Was You

The disk was labeled in his own handwriting — SCREEN UTILS, and a date eight months gone — and the
programmer could not read what was on it. It was the summer of 1985 now; the disk was from the
previous autumn, and he had gone back to it for a good reason: it held a routine that drew a bordered
box on the screen, and he needed exactly that for the thing he was building tonight. Why write it
twice? He loaded the source, and the screen filled with a listing that might as well have been
someone else's.

There were no comments. Not few — none. The labels were L1, L2, L3, and a bleak stretch of X, Y, Z,
XX, and a thing called LOOP2 with no LOOP1 anywhere in sight. Numbers sat naked in the operands —
`LI R1,>0300`, and he no longer had any idea what >0300 meant, whether it was a screen address or a
count or a color, because nothing said. Two routines shared registers in a scheme he had clearly
understood completely on the night he wrote it and had taken with him when he closed the editor.
The box-drawing code was in there somewhere. Finding it meant reconstructing, by single-stepping his
own program like a stranger's, the reasoning he had not bothered to write down — because on that
autumn night, past-him had known exactly what he meant, and had felt no need to tell anyone, least
of all this idiot from the future squinting at the screen in July.

He found the routine eventually. It took most of an hour, and the hour taught him more than the
routine was worth. The code had worked perfectly the whole time; that was never the problem. The
problem was that working code which cannot be *read* is a dead end — you cannot fix it, extend it,
or safely reuse it, so in practice you rewrite it, which means the first writing was half wasted.
Every uncommented magic number, every clever register trick left unexplained, every label named for
the programmer's convenience in the moment rather than the reader's understanding later, was a small
loan taken out against the future at ruinous interest, and tonight the future had come to collect.

So he made two decisions that outlasted the machine. The first was a style — a set of habits about
names and comments and layout, applied the same way every time, whether or not he felt like it,
because discipline you only keep when convenient is not discipline. The second was a library: the
routines he had gotten *right* and *tested*, pulled out of the programs that birthed them, documented
once, and set aside where he could call them without ever rereading them. He would write the box
routine one final time, correctly, and never write it again. This chapter is those two decisions,
made properly, and it ends by founding the library — `lib99` — that the rest of this book builds on,
one trusted routine at a time.

---

## What You Will Learn

By the end of this chapter you will be able to:

- Apply a consistent style for 9900 assembly — naming, workspace discipline, comment density, file
  layout, and a per-routine register-use map — and say why each rule pays for itself.
- Judge the macro question honestly: what Editor/Assembler's lack of macros costs, what macro
  assemblers buy and hide, and how a disciplined `COPY`-include practice covers most of the gap.
- Structure a program as an include architecture: an equates file that names every magic address
  once, library modules with stated conventions, and a clean separation of interface from code.
- Debug as a method rather than a brawl — hypothesis, breakpoint, evidence — using BENCH99's step
  and run-until as a lab notebook, and know where Classic99's watchpoints and MAME's second opinion
  fit on the shelf.
- Leave instrumentation in your code that earns its space: assertions, a debug-build screen flash,
  and a panic-dump routine.
- Write a scripted BENCH99 test that assembles, boots, drives, and asserts — real continuous
  integration for a 1981 target — and read a period ROM listing without drowning.
- Found `lib99` and its first consumer, `SYSCHK`, a system-info card you will extend all book long.

## The Bridge: The Program Is a Document

In a modern language the tools push you toward legibility whether you like it or not. Functions have
names and signatures; types document intent; a formatter fixes your indentation; a linter nags; the
compiler's errors teach. The language and its ecosystem carry much of the craft, so that even
careless code inherits a floor of readability.

Assembly gives you none of that floor and all of the responsibility. Nothing forces a comment,
nothing names a constant, nothing groups related code, nothing checks that a routine's callers honor
its register conventions — the assembler will happily turn an illegible thicket into a working
program and never once suggest you do better. Every quality a modern language provides by
construction, you must provide by *discipline*, and the discipline is the whole of the craft. This
is not a burden unique to old machines; it is the same craft that underlies good code everywhere,
laid bare because nothing hides it. The reward for practicing it here is that you learn it in the
open, on a machine small enough to hold entire in your head, where the difference between crafted and
hacked code is visible in a single screen of listing. Chapter 9 gave you conventions for how routines
*call* each other; this chapter is about how routines *read* — and the two together are what turn a
person who can write assembly into a person you would trust to maintain it.

## 11.1 A Style, Stated Once and Used Everywhere

Style in assembly is not decoration; it is the difference between code you can reason about and code
you can only run. The book adopts the following, states it here, and obeys it in every listing from
this point forward. None of it is novel — it is the distilled common sense of people who maintained
assembly for a living — and all of it is worth the keystrokes.

**Names carry meaning or they are noise.** A label names what the code *does* (`KSCAN`, `DIVSAF`,
`FEED`), not where it happens to sit (`L1`, `LOOP2`). An `EQU` names every constant that is not
self-evidently a small counter: `VDPWA EQU >8C02` appears once, and every use reads `@VDPWA` instead
of `>8C02`, so the address is documented at its definition and searchable everywhere. A naked magic
number in an operand is a comment you failed to write. The single most valuable habit in this whole
chapter is *name the address once, use the name always* — it is why §11.3's equates file exists.

**The workspace is a resource with a map.** Sixteen registers is enough to lose track of. Each
routine states, in its header, which registers it reads, which it writes, and which it destroys —
a **register-use map** — so a caller knows at a glance what survives the call and the book's calling
convention (R-16) is documented per routine rather than assumed. The reserved seats never move: R0
is a shift count and never an index, R10 is the stack pointer, R11 is the link, R12 is the CRU base,
R13–R15 are the BLWP linkage. Everything else is spoken for, per routine, in writing.

**Comments explain the why, never the how.** `INC R2 ; add one to R2` is worse than no comment,
because it costs a line and teaches nothing; the instruction already says it adds one. `INC R2 ;
advance to the next screen column` earns its place, because it says what adding one *means* here,
which the instruction cannot. Comment the intent, the invariant, the reason this and not the obvious
alternative — the things that live in your head on the night you write it and evaporate by morning.
The right comment density is not "every line"; it is "every decision."

**Layout is legibility.** A file reads top to bottom: a header block (what the file is, its
copyright, how to build and run it, the register-use map for its entry point), then equates, then
code, then data. Within a routine, the prologue, body, and epilogue are visually distinct.
Whitespace is free and silence is not — the listings in this book put an operand comment where it
helps and leave it off where the code speaks, and the ragged right margin you see is deliberate.

You have been reading this style for four chapters. `memlib` (Ch. 7), `mathlib` (Ch. 8), and the
`stack99` of Chapter 9 were all written to it, which is why you could read them at all. This section
just names what your eye already learned.

## 11.2 The Macro Question, Answered Honestly

Sooner or later every assembly programmer wants a macro — a named pattern of instructions the
assembler expands in place, so that `PUSH R11` could stand in for the `DECT R10` / `MOV R11,*R10`
pair of Chapter 9 and read as one operation. It is a reasonable want, and the honest answer for this
book is: our baseline does not have them, and here is exactly what that costs and what we do instead.

The Editor/Assembler that defined the TI-99/4A's assembly world — and `libre99asm`, the project
assembler this book is built on, which tracks it — has **no macro facility** (R-12). That was not an
oversight; the 4A's E/A was a cartridge-and-disk product with tight memory, and macros are expensive
to implement. The macro *tradition* on this platform lived elsewhere: in TI's separate Macro
Assembler for larger development setups, and today in the community's **xas99** (xdt99 suite), whose
macro support is real and which Chapter 6 and Part VII lean on for period-format work. So macros are
available on this platform — just not in the baseline the book teaches in, and that is a deliberate
pedagogical choice as much as a tooling one.

What do macros buy? Two real things. They remove repetition — write the PUSH pattern once, use it a
hundred times, fix it in one place — and they raise the reading level, so a routine reads in
operations rather than instructions. Those are genuine goods. What do macros *hide*? Cost and
surprise. A macro that looks like one instruction may expand to twenty; a macro with a hidden branch
target or a clobbered register can bite in ways the call site does not show; a program that leans
hard on macros can become a language of its own that the next reader must learn before reading a
line. On a machine where you are counting cycles (Chapters 5, 7, 8) and every register is spoken for,
"looks like one thing, is secretly another" is precisely the wrong property.

The macro-free baseline demands something in exchange, and the exchange is the discipline this
chapter is about. Where a macro would remove repetition, we use a **`COPY`-include**: the PUSH/POP
idiom, the border-verdict scaffold, the equates — each written once in a file and pulled into every
consumer with `COPY 'file'`, so there is one definition to fix, without hiding what expands. Where a
macro would raise the reading level, we use **named routines and a stated convention**: `BL @PUSH`
is not quite as terse as a `PUSH` macro, but it is honest about being a call, and the register-use
map says what it costs. The trade is legibility-through-naming instead of legibility-through-
abstraction, and on this machine, at this altitude, it is the right trade — you never lose sight of
what the processor is actually doing, which is the entire reason to be writing assembly instead of
letting a compiler do it. When you reach xas99 in Chapter 6's interchange work, you will have macros
if you want them; by then you will know exactly what they are saving you from and hiding from you.

## 11.3 Include Architecture: Naming the Machine Once

The top of an include architecture is one file that names the whole machine. `code/ch11/equates.inc`
is the book's first, and it holds nothing but names: every hardware address the console forces on you
— the VDP ports at >8800 and >8C00, the sound port at >8400, the GROM ports at >9800, the scratchpad
landmarks, the peripheral CRU bases — is an `EQU`, in one place, so that no program ever hard-codes a
bare >8C02 again. A source file pulls the entire table in with a single line:

```
       COPY 'equates.inc'
```

and from then on reads `@VDPWA`, not `@>8C02` — the address documented at its one definition,
searchable everywhere it is used, and changeable in a single spot if it ever must be. This is the
"name the address once, use the name always" habit of §11.1, promoted from a discipline to a file.

One verified detail decides how you write that `COPY` line, and it is exactly the kind of thing §11.4
insists you check rather than assume. `libre99asm` resolves a `COPY` path **relative to the directory of
the source file being assembled**, not the directory you run the assembler from. `syschk.a99` sits in
`code/ch11/` beside `equates.inc`, so it writes `COPY 'equates.inc'` — a bare filename — even though
the build command names the source as `code/ch11/syschk.a99` from the repository's `docs/ti99book/`
root. Write `COPY 'code/ch11/equates.inc'` and the assembler, already anchored in `code/ch11/`, would
go looking for `code/ch11/code/ch11/equates.inc` and fail. Source-relative includes are the norm; you
confirm your assembler's rule once, on the bench, and then it is muscle memory.

Below the equates sit the library modules. `lib99` — founded in this chapter's lab — is a set of
`COPY`-includable modules, each a file written to §11.1's style: a header block, a register-use map,
the R-16 convention stated. `memlib` from Chapter 7 and `mathlib` from Chapter 8 are its first two,
and the routines still to come join the same way. And here our macro-free baseline shows its shape: a
program is *composed*, not *linked*. There is no `DEF`/`REF` and no linker in our world — those belong
to Chapter 6's Editor/Assembler loader — so a finished program is its entry file plus the includes it
`COPY`s, assembled whole into one image in a single pass. That is a real constraint with a real
virtue: there is no link step to misconfigure, no symbol-resolution surprise, and the entire program
is visible to the assembler, and to you, at once. The cost is that composition is textual and global —
every name lives in one flat namespace — which is precisely why the naming discipline of §11.1 and the
convention of R-16 are not niceties but the load-bearing structure that keeps a `COPY`-composed
program from collapsing into a heap of colliding labels.

## 11.4 Debugging as a Method

The difference between an hour of debugging and a week of it is almost never talent. It is method.
The programmer who flails — changing things at random, adding and removing instructions to see what
happens, reasoning from hope — can burn days on a bug that yields in minutes to someone who treats it
as what it is: a discrepancy between what you believe the machine does and what it actually does,
locatable by evidence. The method is old and simple: form a **hypothesis** specific enough to be
wrong, find the **evidence** that would confirm or kill it, and let the machine — not your intuition —
render the verdict. BENCH99 is built to serve exactly this loop, and this book has already run it
several times in plain sight. Three of its own bugs make the method concrete.

**The return into the void (Chapter 9).** The symptom: a non-leaf subroutine that called another
subroutine returned to the wrong place — sometimes hanging, sometimes landing in ROM. The flailer's
response is to shuffle instructions until it stops. The method's response is a hypothesis precise
enough to test: *the inner call is overwriting the return link in R11.* The evidence that would settle
it is the value of R11 before and after the inner call. So `link.a99` recorded exactly that, and read
off the scratchpad after the run the two recordings sit side by side and settle the question:

```
>8342  60 36     BADB4 = >6036   the return address the caller handed the routine
>8344  60 7C     BADAF = >607C   R11 after the inner BL — an address inside the routine
```

>6036 was the way home; >607C is where the inner `BL` left R11 pointing, just past that call. They
differ, and that difference *is* the bug, as a number you can point at: a naive `RT` here would branch
to >607C and land back inside the routine — the return into the void, made concrete. Hypothesis
confirmed in one run; the fix (save R11 on the stack) follows necessarily. The bug was not found by
cleverness. It was found by asking the machine one well-posed question and reading the answer.

**The wild address (Chapter 7).** The symptom: a record-walking routine returned zero, reading from
an address that made no sense. Hypothesis: *the address arithmetic is wrong.* The evidence: the symbol
map and the listing, which showed `TABLE+3*RECSIZ` assembled not as "table plus three records" but,
because `libre99asm` has no operator precedence and evaluates strictly left to right, as "(table plus
three), times the record size" — a wild product. The evidence was in the assembler's own output the
whole time; the method was knowing to look there instead of at the code's logic, which was fine. The
fix (precompute the offset in an `EQU`) is now a documented pitfall, and it entered the book because
someone read the evidence instead of guessing.

**The program that "didn't run" (Chapter 5).** The symptom: a freshly loaded program produced nothing.
Hypothesis, tempting and wrong: *the program is broken.* A better hypothesis, cheap to test: *the
program never started at its entry point.* The evidence: the program counter after `load`, read
straight off the bench — and it was not at `START`, because loading a cartridge image does not set
the PC to your entry label, and the entry address had shifted with the program's title length. The
fix was one bench command (`pc` to the address from the symbol map), and the "bug" was a mistaken
assumption about the tool, exposed the instant someone checked a fact instead of trusting a belief.

The tools of the method are the bench commands you already know, used as a **lab notebook**. `s`
(step) walks one instruction at a time, showing the disassembly, the cycle cost, the status bits, and
the next PC — the finest grain of evidence there is. `u` (run-until) runs to a chosen address, a
software breakpoint you place on a hypothesis: *if control ever reaches here, my theory is right.*
`r`, `m`, `wp`, and `vdp` read the state — registers, memory, the workspace, the video chip — so you
can compare what you believe against what is. **Watchpoints** — "stop when this scratchpad word
changes" — are the natural next tool when a value is being corrupted by you-know-not-what; the bench's
approach is to diff pad state across steps, and where you want a hardware-grade watchpoint or a memory
*heat map*, **Classic99** carries those on the shelf (R-12), and **MAME** is the second opinion you
consult when you suspect the emulator itself rather than your code — the tier-3 cross-check the book's
verification ladder names. The method does not change with the tool. Hypothesis, evidence, verdict —
and the discipline to let the machine be right when it disagrees with you.

## 11.5 Instrumentation You Leave In

Some debugging code is worth keeping — built to be switched on in a debug build and compiled out (or
simply left inert) in a release. Three kinds earn their space on this machine.

**Assertions.** An assertion is a check of an invariant you believe always holds, wired to fail loudly
if it ever doesn't. On a machine with no exceptions, "fail loudly" means the border-verdict scaffold
of Chapter 7, generalized: at a point where R3 must be nonzero, or the stack must be balanced, or a
pointer must lie in range, a few instructions test the invariant and, on violation, paint the border
red and halt — turning a silent corruption that would surface as garbage three routines later into a
stop *at the scene*. Because our baseline has no macros (§11.2), an assertion is a short `COPY`-include
idiom or a `BL` to a check routine, not a one-word `ASSERT` — but it does the same job, and the habit
of asserting the invariant you are *assuming* is one of the highest-yield in all of programming.

**Debug-build screen flashes.** The border is the cheapest instrument on the machine — one CRU-free
write to a VDP register and it changes color, visible instantly, costing nothing you care about. A
debug build that flashes the border a distinct color at each phase of a frame (input read, logic,
draw) turns timing into something you can *see*: a phase that runs long paints a visibly thicker band,
and a hang paints a frozen one. It is an oscilloscope you already own, and Part III's graphics chapters
and Chapter 22's interrupt work lean on it constantly.

**A panic dump.** When an assertion fires or the unthinkable happens, the last useful act is to
preserve the evidence. A panic routine writes the crucial state — the registers, the stack top, a
panic code identifying which check failed — to a fixed, known region of the scratchpad, then halts in
a tight, recognizable loop. On the bench you `m` that region and read the machine's dying words; on
real hardware you can often still inspect that RAM. A crash that leaves a labeled dump is a crash you
can debug once; a crash that leaves nothing is a crash you will meet again. The dump is the difference.

`code/ch11/assert.inc` marries the two into one routine, `VERDCT`: hand it a code and the address of a
pad word, and it stamps the code there and paints the border — green for an all-clear >0000, red for
any nonzero panic code — in a single call, no macro required. It is the assertion's alarm and the
panic dump's labeled evidence in a dozen instructions, and the lab below leans on it: `SYSCHK` reports
its verdict through `VERDCT`, and a deliberately sabotaged copy trips the red path on cue, which is how
you prove your instrumentation *works* before you trust it to tell you something does not.

## 11.6 Testing a 1981 Machine in 2026

The strongest idea in this book's approach to verification is that the emulator is a *library*, not
merely an application — and so a test is a *script*. You assemble the source, load the image, drive it
(set the program counter, step or run to a chosen address, inject a keystroke), and then **assert** on
the machine's resulting state: a word in RAM, a byte in VRAM, a register, the border color. Nothing
about the target being from 1981 prevents the discipline every modern codebase takes for granted; the
emulator-as-library supplies it, and the whole companion tree runs on it.

You have been writing these tests since Chapter 7 without calling them that. `memlib`'s self-test
painted the border green; `mathlib`'s harness wrote >C0DE to a verdict word with the counted LFSR
period beside it; `task99`'s log came out `AA BB AA BB…`. Each is a program that proves itself and a
bench script that reads the proof — the two halves of a test. `code/ch11/syschk.bench` is the shape,
stated plainly:

```
load build/SYSCHKC.bin
pc <START>
u <HALT>
m 8320 20
vdp
```

Load the image, run the harness to its halt, dump the findings block, read the border register — and
a human, or a script grepping the output, compares the result against the expected verdict (`>C0DE`
in the first word, `R7=>02` at the border). That is continuous integration for a 1981 target, and it
is not a metaphor: the `libre99` project tests its own core exactly this way, driving the
emulator as a library and asserting on the outcome, and the repository's own test suite is the worked
example (cite it rather than restating it here, R-12). `sh verify.sh` extends the same principle across
the whole book — it assembles every listing, and the self-checking labs prove their own behavior, so a
change that breaks a chapter's code breaks the build, loudly, which is the entire point of a test.

## 11.7 Reading the Ancients

You will spend real time, in this book and after it, reading assembly you did not write: the console
ROM disassembled, a period game picked apart, a *TI Intern*-style annotated listing of the machine's
own firmware. It is easy to drown in it — thousands of instructions, no comments but the annotator's,
conventions you have to infer — and the skill of reading it without drowning is worth naming.

Do not read it linearly. A ROM listing is not a story told front to back; it is a web of routines
reached through the vector and branch tables you learned to read cold in Chapter 9. Start from an
**entry point** you care about — a reset vector, an interrupt handler, a named service — and follow
*only* the thread that answers your question, treating every `BL` and `BLWP` as a door you may choose
not to open yet. Lean on the **landmarks** you already own: an `LWPI` names a workspace and tells you
the register frame that follows; a write to `>8C02` is aiming the VDP (Ch. 5); an `SBO`/`SBZ` is
working the CRU (Ch. 10); the scratchpad addresses in Appendix C tell you what a routine is touching
by *where* it touches. Read for **shape** before detail: find the loops, the tables, the calls, and
sketch the control flow before you trace a single value, because the shape is the argument and the
instructions are only its grammar. And keep a **notebook** — the same hypothesis-and-evidence habit of
§11.4, applied to comprehension: write down what you think a routine does, then step it on the bench
and let the machine correct you. The ancients wrote for machines and for the few who would read them
cold; the annotator of a *TI Intern* did the reading so you would not have to start from nothing.
Stand on that work, follow one thread, and the ROM stops being a wall and becomes what it is — a
program, written by people, in the language you now speak.

> **Sidebar — Single-Stepping the Whole Machine, 1985.** The debugging method of §11.4 had commercial
> tools in its own day. The best remembered came from **Millers Graphics**, a leading TI-99/4A
> developer-tools house of the mid-1980s, whose *Explorer* let a programmer stop the machine and walk
> it — single-step the CPU, inspect and change registers and memory, trace where control had been and
> was going — on real hardware, with no host computer and no source, only the running machine and a
> disciplined curiosity. It was, in spirit, BENCH99 in a cartridge: the same conviction that you debug
> a computer by *observing* it rather than guessing at it, four decades before the emulator-as-library
> made the same observation a scripted, repeatable test. The tools change. The method — stop, look,
> ask a precise question, believe the answer — is the same craft in 1985 and 2026. (Product and company
> details here are drawn from the community record and are hedged accordingly.)

## Lab 11 — Found `lib99`; Build `SYSCHK`

Two deliverables, and the first is a decision that outlasts the chapter.

*Founding `lib99`.* From here, the routines you have gotten right and tested stop living inside the
programs that birthed them and become a library. `lib99` is a set of `COPY`-includable modules under
`code/lib99/`, and its contract — formalized in `code/lib99/README.md` — is exactly the craft of this
chapter made mandatory: every module is a file written to §11.1's style, with a header, a register-use
map, and the R-16 convention stated; every module names its addresses through `equates.inc` (§11.3);
and every module ships as the `mathlib`/`stack99` single file — routines above the rule, a `START`
self-test below it — so it assembles stand-alone under `sh verify.sh` and *proves itself* on the bench
(§11.6). The library is versioned by the book: it grows one chapter at a time, and `lib99` "as of
chapter N" is what that chapter has added. `memlib` (Ch. 7) and `mathlib` (Ch. 8) are its founding
modules, retrofitted to the contract; every library the book builds from here — for video, for sound,
for disk — joins the same way and obeys the same rules. This is the accumulating trusted-parts
collection the vignette's programmer resolved to build, made a real directory with a real contract
[ruling R-17].

*Building `SYSCHK`.* `lib99`'s first consumer is `SYSCHK`, a system-information card that interrogates
whatever machine it wakes up on and reports what it finds — the diagnostic the book extends every time
it adds a subsystem. It runs three probes, each a `lib99` routine obeying R-16. `RAMPRB` writes a
pattern and its complement to a cell and demands both survive — Chapter 5's `det32k` test, promoted to
a library routine. `VDPCHK` writes a byte through the VDP port and reads it back to prove the chip
answers, then walks the address counter to the point where it wraps, revealing the VRAM size. `DSRSCN`
scans the peripheral CRU bases, paging each card's ROM in with `SBO 0` and looking for the >AA
signature (§10.6). It records everything to a documented block of scratchpad and paints a verdict
through `VERDCT`. On the bench, `SYSCHK` runs 371 instructions and returns:

```
>8320  C0 DE FF FF FF FF FF FF FF FF 40 00 00 00 00 00
```

— DONE = >C0DE (every probe ran), both expansion banks answering (>FFFF each) for a full 32 K, the VDP
present (>FFFF) and measured at exactly 16 K (>4000, the 14-bit counter's wrap), the DSR scan finding
zero responders — and a green border, PASS.

The verdict gates on the single invariant that *must* hold here: the VDP present and 16 K. The RAM and
DSR findings are *reported, not gated*, and the reason is the chapter's whole ethic in one design
choice. Their correct values depend on the rig: a real, unexpanded console would report its expansion
RAM *absent* and still be a perfectly healthy machine, so gating "pass" on RAM-present would make
SYSCHK lie about a bare 4A. Two of those findings come with honesty notes that are also gifts to the
project's roadmap (R-12). The 32 K reads *present* here because the project emulator always backs
>2000 and >A000 with real RAM — it models no unexpanded console — so `RAMPRB`'s *negative* case cannot
be exercised on this bench (the same gap Chapter 5 logged; SYSCHK reconfirms it). And the DSR scan
finds *zero* responders because no card's DSR ROM can be installed from BENCH99's bare bus; a live
responder — the disk controller answering at CRU >1100 — needs the desktop `libre99` with a
mounted controller, or Classic99 on the shelf. Both are stated plainly rather than papered over. That
SYSCHK's measurement is *honest* and not merely green is itself provable: a sabotaged copy demanding
8 K instead of 16 turns the border red and writes the failing code, while the size probe still reads
its true >4000 — the check fails, the measurement does not lie.

`SYSCHK` gains a line every time the book adds a subsystem — the sound chip (Ch. 19), speech (Ch. 20),
the disk system (Ch. 31) — so that by the end it is a one-screen portrait of a running TI-99/4A,
assembled entirely from `lib99` parts you trusted once and never had to read again. That is the whole
argument of the chapter, compiled and running: craft is not what you do instead of shipping. It is what
lets you keep shipping.

## Exercises

**✦ Warm-ups.**

1. Take this snippet and rewrite it to the §11.1 style — meaningful labels, an `EQU` for each magic
   number, why-not-how comments, and a register-use map in a header:
   `L1 LI R1,>0300` / `L2 MOV *R2+,*R1+` / `DEC R3` / `JNE L2`.
2. Name five magic numbers you used in an earlier chapter's code as `EQU`s in a small include, and
   write a two-line consumer that `COPY`s it and uses one. Assemble it.
3. State the `COPY`-path rule libre99asm uses, then predict: from a source file in `code/ch11/`, does
   `COPY 'code/ch11/equates.inc'` work? Explain in one sentence.

**✦✦ Consolidation.**

4. Add an assertion to a `lib99` routine — that one of its arguments is nonzero, say — using `VERDCT`
   from `assert.inc`, and deliberately violate it. Confirm on the bench that the border goes red and
   the panic code lands in your chosen pad word.
5. Write a `.bench` CI script, in the shape of `syschk.bench`, that loads a routine, runs it to a halt,
   and asserts on a single output word in the scratchpad. Describe what a "green" and a "red" run each
   look like in the transcript.
6. SYSCHK reports the 32 K expansion *present* on the project emulator, but a real unexpanded console
   would report it *absent*. Explain why gating SYSCHK's PASS on RAM-present would be a bug, and state
   precisely what SYSCHK does instead and why that is the honest design.

**✦✦✦ Extensions.**

7. Add a module to `lib99`: a small routine of your own, written to the full contract (header,
   register-use map, R-16, a `START` self-test that paints the border), and wire one line of its result
   into SYSCHK's findings block so the system card reports it. Confirm both still assemble under
   `verify.sh` and run green.
8. Apply the §11.7 method to a real ROM: pick a console entry point from the Chapter 9 vector table
   (say, the reset vector's target at >0024), disassemble its first ten instructions on the bench, and
   annotate them — landmarks, workspace, what it is doing — in the commented style of §11.1. Note one
   thing you had to *check* on the bench rather than assume.

## Further Reading

- *Editor/Assembler Manual*, Texas Instruments — the assembler conventions the platform grew up on,
  and the confirmation that E/A itself has no macro facility (the premise of §11.2).
- Texas Instruments *Macro Assembler* documentation — the macro tradition on the 9900 that E/A lacked,
  worth reading once to know exactly what our macro-free baseline trades away.
- The **xdt99** (`xas99`) documentation — modern macros, conditional assembly, and structured directives
  on this platform; the interchange toolchain of Chapter 6, where you can reach for macros knowingly.
- *TI Intern* (Heiner Martin) — the annotated console ROM listing that is the model for §11.7's "reading
  the ancients," and the standard reference for what the firmware actually does.
- The `libre99` repository's own test suite — the worked, running example of §11.6's
  emulator-as-library CI: the same core the book runs on, tested the same way the book tests its labs.
- Millers Graphics' developer tools (the *Explorer* and others) in the community record — the period's
  own debugging instruments, discussed hedged per the sidebar.

## Summary

- Craftsmanship in assembly is the discipline a modern language provides for free and this machine
  provides not at all: the program is a document, and its legibility is entirely on you. Vignette = the
  eight-month-old disk of your own unreadable code — working, and useless because unreadable.
- **Style (stated once, obeyed book-wide):** names carry meaning (labels for what code does, an `EQU`
  for every magic number — "name the address once, use the name always"); the workspace has a
  per-routine register-use map; comments explain the *why*, never the *how*; files read top-to-bottom
  (header → equates → code → data).
- **The macro question, answered honestly:** E/A and `libre99asm` have no macros by design (R-12); the
  macro traditions are TI's Macro Assembler and today's `xas99` (Ch. 6). The macro-free exchange is
  `COPY`-includes (repetition without hiding) + named routines and R-16 (reading level without
  abstraction) — the right trade at this altitude, where you must never lose sight of the silicon.
- **Include architecture:** `equates.inc` names the whole machine once; `COPY` is resolved
  **source-file-relative** (verified — a consumer beside it writes `COPY 'equates.inc'`, bare); a
  program is *composed* by `COPY`, not linked (no DEF/REF on our baseline), so the flat namespace makes
  the naming discipline load-bearing.
- **Debugging is a method:** hypothesis → evidence → verdict, told through the book's own three bugs
  (the R11 clobber, the libre99asm left-to-right precedence trap, the "PC isn't at START after load"
  gotcha). The bench `s`/`u`/`r`/`m` is the lab notebook; Classic99's watchpoints/heat-map and MAME's
  second opinion sit on the shelf (R-12). Instrumentation you leave in: assertions and a panic dump via
  the border-verdict (`assert.inc`'s `VERDCT`), and debug-build border flashes as a free oscilloscope.
- **Testing a 1981 machine in 2026:** the emulator is a library, so a test is a script — assemble,
  load, drive, assert. The self-checking labs since Ch. 7 already are this; `syschk.bench` formalizes it
  as CI, the same way the `libre99` project tests its own core, the same way `verify.sh` tests
  the whole book. Reading the ancients (§11.7): entry-point-first, lean on landmarks, shape before
  detail, keep a notebook.
- **Lab founds `lib99`** (`code/lib99/`, its contract in the module README; `memlib`+`mathlib` as
  founding modules; every module = header + register-map + R-16 + a single-file bench self-test —
  **ruling R-17**) and builds **`SYSCHK`**, its first consumer: RAM/VDP/DSR probes, verdict gated only
  on the VDP-present-and-16 K invariant (RAM/DSR *reported*, not gated, because their honest values are
  rig-dependent). Verified green (>C0DE / R7=>02, 32 K + VDP 16 K + 0 DSR); sabotage drill red. Two
  R-12 gaps reconfirmed: the emulator models no unexpanded console, and no DSR ROM is installable from
  the bench. Seeds: `lib99` grows every remaining chapter; `SYSCHK` extended at Ch. 19/20/31…; the CI
  practice IS the whole `code/` tree.
