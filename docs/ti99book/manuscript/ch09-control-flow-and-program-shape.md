# Chapter 9 — Control Flow and Program Shape

*Chapter 8 taught this machine to decide: every arithmetic instruction files a report in the
status register. This chapter teaches it to act on the verdict — to jump, to call, to come back,
and to hold more than one strand of work at once. The hardware hands you exactly one breadcrumb
home. The stack, the conventions, the very shape of a program: you build those, and then you
agree to obey them.*

<!-- Part II — The TMS9900 and Assembly Fundamentals · target ≈26 pp -->
<!-- STATUS: DRAFTED (session 5, 2026-07-06) — pending review passes. All six listings assemble via sh verify.sh; §9.1–9.9 cycle/linkage/reach facts machine-verified on BENCH99 at commit b9291be; code in code/ch09/. Verdict files green (reach/link/stack99/task99 → VDP R7=>02); task99 interleave log AA BB AA BB AA BB AA BB; stack99 FACT(8)=>9D80 with R10 balanced to >8380; XOP transfer observed into console vector {>280A,>0C1C} with R11=&operand and ST X-bit set. New ruling R-16 (the book-wide calling convention) registered. -->
<!-- SPEC: 00-master-outline.md, section "### Chapter 9 —" (lines 269–280 in outline v1.1). That bullet list is this chapter's contract. -->

## The One Breadcrumb

A kitchen table in Dayton, Ohio, the second week of January 1984 — the kind of cold, bright
evening when the furnace runs all night and the good light is gone by five. A borrowed
black-and-white portable television stood in for a monitor, its rabbit ears folded down and
useless; the console's RF box fed it channel 3. Beside it sat the thing that had cost most of a
Christmas bonus: the Editor/Assembler package, its manual already soft at the corners, its
cartridge warm in the slot. Two weeks earlier the household had owned a game machine. Now, by an
act of will and forty dollars, it owned a development system.

The programmer at the table had gotten something to work, and the feeling was electric. A
subroutine — a real one, called with BL and returned from with RT, exactly as the manual
described — that cleared the screen. Twenty bytes of code, and it *worked*: call it from anywhere,
the screen went blank, control came back to the next instruction like a well-trained dog. So he
did the natural thing, the thing every programmer in history has done ten minutes after the first
subroutine works. He wrote a second one. This one drew a border of asterisks around the screen,
and because a clean border wants a clean screen, its very first act was to call the first: BL
@CLS, right there at the top of BOX.

The screen cleared. The border began to draw. And then the machine simply stopped being a
machine that made sense. Sometimes it hung. Sometimes it filled the screen with garbage and hung.
Once, memorably, it returned control not to the program but to some address deep in the console
ROM, and the little TV showed the title screen again as if nothing had happened. The screen-clear
worked. The border worked, when he tested it alone. Together, they came apart.

It took him past midnight and most of a legal pad to see it, single-stepping through the E/A
debugger one instruction at a time, and when he saw it he felt the particular embarrassment of a
thing that is obvious only afterward. BL does exactly one thing with the return address: it puts
it in R11. One register. When he called BOX, the way home went into R11. When BOX turned around
and called CLS, the way home *from CLS* went into R11 — on top of the first address, which was now
gone. CLS returned correctly, into the middle of BOX, because its breadcrumb was the freshest one.
But when BOX finished and did its own RT, R11 no longer remembered the caller. It remembered CLS.
BOX returned into itself.

The machine had handed him a single breadcrumb and he had dropped it in the woods. The fix, that
night, was crude: BOX stashed R11 in a spare register before calling anything, and put it back
before returning. It held. But he could already see the next cliff — what happens when BOX calls
something that calls something that calls something, and one spare register is not enough? The
9900 has no CALL that remembers, no RETURN that unwinds, no stack of its own at all. If he wanted
to go deep, he would have to keep his own trail of breadcrumbs, and the machine would not help him
do it.

This chapter is where a pile of instructions becomes a program with a shape. We will earn the
stack he was about to invent, adopt the conventions that make deep calls safe, and — by the last
page — run two independent programs at once on a processor that officially does one thing at a
time. It begins with the humblest instruction on the machine and the single register that started
all the trouble.

---

## What You Will Learn

By the end of this chapter you will be able to:

- Choose the correct jump from the J** family for any signed or unsigned condition, state the
  ±128-word reach of a jump from memory, and apply the two-jump idiom when the target is out of
  range — and explain the exact assembler error that forces it.
- Use B, BL, and RT correctly, and articulate the discipline of R11: why a leaf routine may keep
  its return there and a non-leaf may not, and what "the bug every beginner writes" looks like on
  the bench.
- Build and use the book's software stack on R10 — PUSH and POP as a fixed idiom — and follow the
  calling convention this chapter adopts for the entire rest of the book (ruling R-16).
- Explain BLWP and RTWP as a context-switch call: what a two-word vector holds, how the old
  workspace and program counter and status are preserved in the new R13/R14/R15, and how to write
  your own BLWP-style service — and judge when a workspace switch pays for itself and when it is
  ceremony.
- Express if/else, while, repeat, and switch as named jump patterns, and build a jump table
  dispatched with B *Rn.
- Read the X instruction and the XOP trap for what they are — execute-an-instruction-elsewhere and
  a supervisor call whose vectors live in ROM — and know why one is a working tool on the 4A and
  the other is mostly heritage.
- Assemble all of it into a cooperative multitasker: two coroutines sharing one CPU, switching by
  hand, in under two hundred bytes.

## The Bridge: Everything Your Compiler Was Hiding

If you learned to program on anything built after about 1975, control flow came to you fully
furnished. You wrote `if`, `while`, `for`, and a compiler turned them into branches you never saw.
You called a function and a return value came back; a call stack you never allocated grew and
shrank beneath you, holding return addresses, saved registers, and local variables in tidy frames
the calling convention guaranteed. When something went wrong you `throw`-ed, and a mechanism you
did not write unwound that stack until someone caught it. Threads — if you used them — were handed
to you by an operating system that saved and restored their registers on a timer you never set.

Stand at this table and every one of those conveniences is gone, and you are holding the parts. A
jump is a jump; you choose the condition and you count the distance yourself. A call is a BL and a
single link register; the stack that makes calls nest is one you build this chapter and maintain
by hand forever after. There are no exceptions — an error is a number you agree to return and a
caller agrees to check. And a thread is a *workspace*: sixteen registers somewhere in memory, plus
a program counter and a status word, that you switch to and from with two instructions and a table
you wrote. The astonishing thing is not that this is possible. It is that it is *small* — that the
whole apparatus of structured, re-entrant, concurrent code reduces, on this machine, to a handful
of instructions and a few disciplined conventions you can hold in your head. That reduction is the
subject of the chapter. Let us start where the trouble started.

## 9.1 Jumps: The J** Family and the Reach of a Branch

The unconditional jump is the simplest instruction on the machine, and it is worth a moment
precisely because it is so plain. JMP goes somewhere; it always goes; it costs, in cartridge ROM
with a scratchpad workspace, fourteen cycles. Everything interesting about the jump family is not
in JMP but in the *conditional* jumps, and in a number that will shape more of your code than you
expect: how far a jump can reach.

The conditional jumps read the status register that Chapter 8 taught every arithmetic instruction
to write, and branch or fall through accordingly. You met the grammar in Chapter 4 and drilled the
signed-versus-unsigned pairings in Chapter 8; the whole family is catalogued in Appendix A. The
short version, the one you will use without looking: after a compare, JH / JL are the *unsigned*
greater / less (they read the logical-greater and carry bits), JGT / JLT are the *signed* greater
/ less (they read the arithmetic-greater bit), JEQ / JNE test equality, and JHE / JLE / the rest
fill in the "or-equal" corners. Choose the wrong family and the machine will not complain — it will
simply give the wrong answer for operands that straddle >8000, exactly as the Chapter 8 lab proved
with >8000 against >0001, where JH branches and JGT does not.

Here is the fact that shapes code. A conditional jump encodes its destination as a *signed
displacement in words*, eight bits of it: −128 to +127 words from the instruction that follows the
jump. That is a reach of roughly a quarter-kilobyte in either direction — generous for a tight
loop, and a wall the moment your routine grows. When the target is farther than that, the
assembler does not paper over it. It stops:

```
jump target out of range (300 bytes away; JEQ reaches -254..+256)
```

That is libre99asm refusing an honest `JEQ` to a label 150 words down the listing. The reach it
quotes — −254 to +256 bytes, measured from the jump itself — is the ±128-word displacement made
concrete. You cannot wish the branch longer. What you do instead is the **two-jump idiom**, and it
is worth learning as a reflex because you will write it a thousand times: *invert the condition and
jump over an unconditional branch.* The conditional jump, now short, skips a B — and B, as the
next section shows, reaches the entire 64K address space.

```
* We want "if R1 = 0 then goto FAR", but FAR is >127 words away.
       MOV  R1,R1            set the flags from R1 (a move is a test)
       JNE  NEAR             R1 <> 0 ? skip the long branch
       B    @FAR             R1 = 0 : reach anywhere with B @LABEL
NEAR   ...                   the fall-through path continues here
```

The logic reads backwards the first few times: to jump *to* FAR when equal, you jump *past* the
long branch when not-equal. Say it aloud once — "if not the condition, skip the escape" — and it
sticks. The companion file `code/ch09/reach.a99` drives this idiom to a target deliberately padded
out of range with a 300-byte gap, and paints the border green when the far branch actually lands;
on the bench it reaches FAR in nine instructions and lights VDP register 7 to >02. The direct
`JEQ FAR` in the same file's comment is the error above, quoted from the assembler that produced
it. The idiom is not a workaround for a weak assembler — a longer displacement is not encodable in
the instruction at all — so it is simply how the 9900 branches long, and every period program is
full of it.

One habit to install now: since a jump's reach is counted from its own position, inserting code
*between* a jump and its target can push a previously legal jump out of range, and the assembler
will flag it at the next build. When that happens, it is not a regression in your logic — it is
geometry — and the two-jump idiom is the fix.

## 9.2 B, BL, RT — and the Discipline of R11

Where a jump is a signed hop measured in words, **B** — branch — is a full-address instruction. Its
operand is a general memory address computed through any addressing mode, so `B @LABEL` reaches
anywhere in the 64K space, and `B *R4` goes to whatever address sits in R4. B is how you branch
long (the escape in the two-jump idiom), how you dispatch through a register (§9.5), and how you
leave a routine that computed its own destination. On the bench, `B @LABEL` costs 24 cycles and
`B *Rn` costs 16 — the register-indirect form is cheaper because it computes no operand address.

**BL** — branch and link — is B with a memory. Before it transfers control, it copies the address of
the *following* instruction into R11. That is the entire subroutine-call mechanism of the TMS9900:
one instruction, one link register, no stack. To return, you branch to the address BL left in R11,
and the machine gives you a dedicated encoding for exactly that:

> **RT** is not a distinct instruction. It is the assembler's name for `B *R11` — branch to the
> address in R11 — and it assembles to the single word **>045B**. When you write RT you are writing
> "go back to wherever the link register points," and the whole contract of a leaf subroutine is
> that R11 still points there when you say it.

`code/ch09/probe.a99` steps a bare call on the bench: BL @LEAF costs 28 cycles and leaves in R11
the address of the instruction after the BL; the leaf's RT (>045B) costs 16 and lands exactly one
instruction past the call. A companion file, `code/ch09/link.a99`, makes the leaf record the R11 it
was handed and compares it to the true return address; they match to the bit. So far, so tidy — and
so was the Dayton programmer's screen-clear, which was a *leaf*: it called nothing, so nothing
disturbed R11 between the call and the RT, and it worked every time.

The trouble is nesting, and it has a name in this book: the **one-breadcrumb problem**. R11 is a
single register. The instant a subroutine that was called with BL turns around and issues its own
BL, the second link overwrites the first, and the way home is gone. `link.a99` reproduces the
Dayton bug deliberately — a non-leaf that records R11 *before* and *after* an unguarded inner call:

```
* BADNL — the bug every beginner writes: a NON-leaf that does not save R11.
BADNL  MOV  R11,@BADB4       link as received (return to the caller)
       BL   @LEAF            <-- clobbers R11 (now points inside BADNL)
       MOV  R11,@BADAF       link now: no longer the caller's address
       ...
```

On the bench the two recorded values differ — BADB4 is the caller's address, BADAF is an address
inside BADNL — and that difference *is* the bug, made visible as data. A routine that executed its
RT here would return to itself, precisely the midnight symptom. The test file treats agreement
between the two as a failure, because agreement would mean the clobber never happened and the
demonstration proved nothing.

The discipline that prevents it is stated in one sentence and obeyed forever: **a routine that
calls anything must save R11 before the first call and restore it before its own return.** A *leaf*
— a routine that calls nothing — may keep its return in R11 and RT directly; that is the reward for
being a leaf, and it is why leaf routines are cheap. A *non-leaf* must find somewhere to keep the
caller's link while it makes its own calls. On the first night that somewhere was a spare register.
That does not scale: two levels of nesting need two saved links, and the registers run out while
the call depth does not. The general answer — the one that makes calls nest to any depth — is a
stack, and the 9900 does not have one. So we build it.

## 9.3 Software Stacks and the Conventions of the Book

The 9900 gives you no stack pointer, no PUSH, no POP, and — crucially — no opinion about which
register should serve. That silence is an invitation, and the book accepts it once, here, for
good. **We dedicate R10 as the stack pointer for the entire rest of the book.** From this page on,
R10 means "top of stack" in every listing, every library routine, every capstone; when you see it
used for anything else, that is a bug, not a style. Choosing a register and *keeping* the choice is
the whole art — a convention is only worth as much as your discipline in obeying it.

Our stack is **full-descending**: it grows toward lower addresses, and R10 points *at* the top item
(not the next free slot). Two idioms, fixed forever, do all the work:

```
*   PUSH x :  DECT R10  /  MOV x,*R10      grow down, then store
*   POP  x :  MOV *R10+,x                  read, then shrink up
```

PUSH decrements R10 by two (DECT — decrement by two, for a word) and stores through it; POP reads
through R10 and lets the autoincrement addressing mode shrink the stack in the same instruction.
That POP is a small monument to the machine's design: `MOV *R10+,x` is one instruction that both
retrieves the item and pops it, because the autoincrement you met in Chapter 7 was built for
exactly this. You initialize R10 once, to the top of a reserved region of the scratchpad, and from
then on the stack lives in the fast island (Ch. 5) where every push and pop is a zero-wait access —
speed is a property of addresses, and a stack is addressed constantly.

With a stack, the one-breadcrumb problem dissolves. A non-leaf's prologue pushes R11; its epilogue
pops it back and RTs. Nesting to any depth just means more items on the stack, and the stack is as
deep as the memory you reserve. The proof that this actually composes is recursion — a routine that
calls *itself*, where every level must keep its own return link and its own data alive across the
call — and `code/ch09/stack99.a99` proves it with a recursive factorial:

```
FACT   CI   R1,2              n < 2 ?
       JL   FACT1             n = 0 or 1 -> 1
       DECT R10               PUSH R11 (our return link)
       MOV  R11,*R10
       DECT R10               PUSH n (must survive the recursive call)
       MOV  R1,*R10
       DEC  R1                n-1
       BL   @FACT             R1 = (n-1)!
       MOV  *R10+,R3          POP n -> R3
       MPY  R3,R1             R1 * R3 -> product
       MOV  R2,R1             result -> R1
       MOV  *R10+,R11         POP R11 (restore the true return link)
       RT
```

Each level of FACT pushes two words — its return link and its own copy of *n* — makes the recursive
call, and pops them in reverse. On the bench, FACT(8) returns 40,320 (>9D80), FACT(7) returns 5,040
(>13B0), and — the invariant that proves nothing leaked — R10 finishes exactly where it started, at
the stack top >8380, every push matched by a pop. The self-test walks five vectors and paints the
border green; it lights VDP register 7 to >02 and stashes 40,320 to the pad as its proof. Recursion
on a machine with no stack, made undeniable by a machine you can read the source of.

Around that stack we adopt the rest of a calling convention. It is not the only workable one — it is
*ours*, stated once so every later chapter can lean on it without re-litigating:

> **The book's calling convention (ruling R-16).**
> - **Arguments and results ride R0–R2** (byte cargo in the high half, per the high-byte law of
>   Chapter 7). These are **caller-saved scratch**: a callee may clobber them freely, and a caller
>   that needs them preserved saves them itself.
> - **A leaf** — calls nothing — may keep its return in R11 and return with RT.
> - **A non-leaf** PUSHes R11 in its prologue and POPs it before RT; it also PUSHes any register it
>   must keep across an inner call, and POPs in reverse.
> - **R10 (the stack pointer) and R13–R15 (the BLWP linkage of §9.4) are never used as scratch.**
> - **Errors are reported in R0**: >0000 is success, any nonzero value is an error code. The caller
>   tests with `MOV R0,R0` (a move is a test) then `JNE`. This generalizes the DIVSAF guard of
>   Chapter 8, and §9.9 makes it policy.

Every routine in `lib99` from Chapter 11 onward obeys this, which is what lets you call a
graphics primitive from a sound routine from a game loop without any of them stepping on the
others. The convention is the interface. Write it down; then never break it.

## 9.4 BLWP and RTWP: The Context-Switch Call

BL is a light call: it borrows the caller's workspace, so the callee's R0–R15 *are* the caller's
R0–R15, and the two routines must negotiate over shared registers through the convention above.
Sometimes that sharing is exactly what you want. Sometimes it is a liability — you want a routine
to have its own sixteen registers, clean and private, and to hand them back untouched when it
returns. For that the 9900 offers a heavier call, and it is one of the architecture's genuine
signatures: **BLWP**, branch and load workspace pointer.

BLWP's operand is not code. It is the address of a two-word **vector**: the first word is a new
workspace pointer, the second is a new program counter. `BLWP @VEC`, where `VEC` holds `{WS, PC}`,
does four things in one instruction — it loads WP from the vector, loads PC from the vector, and
before jumping, saves the *old* WP, PC, and status word into the *new* workspace's R13, R14, and
R15. The callee wakes up with a fresh set of registers and, tucked into its last three, a complete
record of where it came from. It costs 50 cycles on the bench — the price of a whole context change
— and its partner **RTWP** (18 cycles) reverses it exactly: it restores WP from R13, PC from R14,
and the status register from R15, dropping the caller back into its own workspace as if nothing had
happened.

`code/ch09/probe.a99` builds the smallest honest BLWP service and reads the linkage straight off
the machine. The service runs in its own workspace and copies its inherited R13/R14/R15 out where
the bench can see them:

```
* the BLWP service: runs under NWS. Copy the linkage out where we can see it.
SVC    MOV  R13,R7           R7(new) = old WP  (should be >8300)
       MOV  R14,R8           R8(new) = old PC  (the BLWP's return addr)
       MOV  R15,R9           R9(new) = old ST
       RTWP
VEC    DATA NWS,SVC          the vector: {new WP, new PC}
```

On the bench the copied words read >8300, >6040, >C000 — the caller's workspace, the exact address
after the BLWP, and the caller's status with its logical- and arithmetic-greater bits set. RTWP
puts them all back and execution resumes one instruction past the call. That is the whole mechanism,
and it is the mechanism the entire console is built on: reset, every interrupt, and every XOP is a
BLWP through a vector in ROM, as the Field Notes at the end of this chapter show by reading them
cold.

The vector can live **anywhere in your RAM** — that is the freedom BLWP gives and XOP (§9.7) does
not. You write your own services by placing a two-word `DATA WS,ENTRY` somewhere and pointing BLWP
at it. So when does the heavier call earn its 50 cycles? **When the callee genuinely wants its own
sixteen registers** — an interrupt handler that must not disturb the interrupted program (Ch. 22), a
device driver invoked from unknown contexts (the DSRs of Ch. 30), a coroutine that keeps its live
state in registers across a switch (the Lab, below). The private workspace makes the save/restore
*atomic and total*: you do not enumerate which registers to preserve, because the switch preserves
all of them by definition. When does it not earn its price? **For an ordinary leaf or a small
helper** that a BL and the R11 discipline handle for a fraction of the cost. A workspace switch to
save one register is ceremony; a workspace switch to cross between two independent threads of
control is the cheapest miracle on the machine. Learn to feel the difference — the rest of the book
will lean on both calls, each where it belongs.

## 9.5 Structured Shapes in an Unstructured Language

The 9900 has no `if`, no `while`, no `switch`. It has jumps and it has B, and every structured
shape you know is a *named pattern* of those primitives — a pattern worth naming because naming it
is what keeps assembly readable. You are not inventing control flow here; you are hand-compiling
the control flow you already think in, and the value is in doing it the same recognizable way every
time.

An **if/else** is a conditional jump over the "then" arm to the "else" arm, with an unconditional
jump past the "else" at the end of the "then":

```
       CI   R1,10
       JLT  ELSE            if not (R1 >= 10), take the else arm
       ...  then-arm ...
       JMP  ENDIF
ELSE   ...  else-arm ...
ENDIF  ...
```

A **while** tests at the top and jumps out; the bottom jumps back to the test. A **repeat/until**
tests at the *bottom* and jumps back to the top while the condition holds — one fewer jump, which is
why period code favors it for inner loops. A **for** counting down to zero is the tightest of all,
because DEC sets the flags for free and `JNE` reads them: the loop counter and the loop condition
are the same instruction, the idiom you have used since Chapter 7. Write these the same way every
time and a reader — including you, six months on — parses them at a glance instead of tracing every
branch.

The shape that most rewards a systematic treatment is the **switch**, and its systematic form is the
**jump table**: an array of addresses, indexed by a selector, branched through with one instruction.
`code/ch09/probe.a99` dispatches a selector this way:

```
       LI   R3,2             selector 1 -> word offset 1*2 = 2
       MOV  @DTAB(R3),R4     R4 = target address for case 1
       B    *R4              dispatch
DTAB   DATA D0,D1            table of case addresses
```

The selector is doubled — `1 * 2 = 2` — because each table entry is a word, and word addresses step
by two; a selector of 1 must index the *second* word. The indexed load `MOV @DTAB(R3),R4` fetches
the case address into R4 (34 cycles on the bench), and `B *R4` branches to it (16 cycles). On the
machine, selector 1 lands in case D1, which loads >00D1 to prove it — the dispatch is exact. A jump
table turns an *n*-way decision into two instructions and a constant-time branch, no matter how many
cases; it is how a command interpreter, a state machine, or a bytecode dispatcher earns its speed,
and you will build all three before the book is done.

One caution the jump table inherits from the assembler: because libre99asm has no operator precedence
(Chapter 7's hard-won lesson — `2+3*4` is 20, not 14), any index arithmetic mixing addition and
scaling must be precomputed into a single EQU rather than written inline. The selector-times-two
here is done in a register at run time and is safe; a *table+offset* computed in the operand is
exactly the trap Chapter 7's record example sprang, and the fix is the same — name the scaled
constant with an EQU and let the assembler carry it whole.

## 9.6 The X Instruction: Execute Elsewhere

Every processor has one instruction that reads like a magic trick, and on the 9900 it is **X** —
execute. X takes an operand, fetches the *word at that operand as if it were an instruction*, and
executes it, in place, as though it had appeared in the instruction stream. Then control continues
after the X (unless the executed instruction was itself a branch). You are not calling code; you are
reaching out, grabbing one instruction from somewhere else, and running it right here.

`code/ch09/probe.a99` executes an INC held off to the side:

```
       LI   R5,>0000
       X    @XTGT            executes "INC R5" from the stream at XTGT
       ...
XTGT   INC  R5               the instruction X reaches out and executes
```

On the bench, R5 comes back holding 1 — the INC ran — and the whole X cost 38 cycles, which folds
in fetching and executing the target instruction. Read for its own sake, X is a curiosity. Its
legitimate uses are specific and real. The first is **dispatch by instruction**: where a jump table
holds addresses to branch to, an X can run one of a *table of instructions*, choosing an operation
rather than a destination — the difference between "go here" and "do this." The second is
**parameterizing an operation you cannot know at assembly time**: a routine handed a shift count, or
a register number, or a whole instruction to apply, can build or select that instruction and X it,
which is how a general shift-by-R0 or a register-file walker gets written without a jump ladder. The
third, historically, is **self-modifying dispatch** in ROM-tight code, where an instruction is
patched in memory and then X-ed. X is not something you reach for daily. But when the shape of a
problem is "apply a run-time-chosen operation," X is the instruction that was waiting for it, and
recognizing that shape is the skill.

## 9.7 XOP: The Trap You Do Not Own

BLWP lets you point a context-switch call at a vector *anywhere in your RAM*. **XOP** — extended
operation — is BLWP's institutional cousin: a software trap that switches context through a vector
at a *fixed* location the CPU computes from the opcode. `XOP src,n`, for n from 0 to 15, traps
through the two-word vector at **>0040 + n×4**. Like BLWP it saves the caller's WP, PC, and status
into the new R13/R14/R15. Unlike BLWP it does two extra things, and `code/ch09/xop.a99` shows both
by single-stepping one XOP on the bench:

```
       XOP  @OPND,0          trap through the console's XOP-0 vector at >0040
```

Stepping that single instruction, the machine transfers to PC >0C1C in workspace >280A — and
reading the vector at >0040 confirms those are exactly the two words sitting there. Two details make
XOP distinct from BLWP. First, **the new R11 holds the operand's effective address** — not its
value: after `XOP @OPND,0`, R11 is the *address* of OPND, so an XOP handler is handed its argument
by reference for free. Second, **the CPU sets the status register's X bit** (bit 6), the
"XOP-in-progress" flag, which the bench shows lit the moment the trap fires. The old context lands
in R13/R14/R15 just as with BLWP, and the handler would return with RTWP.

So XOP is a clean mechanism. Why is it, on the TI-99/4A, mostly a museum piece? Because of where
those sixteen vectors live. **>0040–>007F is console ROM** — read-only on a stock console — so you
cannot install your own XOP the way you point a BLWP vector into your RAM. The console owns all
sixteen; a user program takes what the ROM defined and no more. That single fact is why this book's
calling conventions (§9.3) are built on BLWP and BL, not XOP: BLWP's vector is yours to place, and
XOP's is not. The instruction shines on its heritage machines — the 990 minicomputers it descends
from, and later the Geneve 9640, where system software owned the vector table and used XOP as a
genuine supervisor call. On the 4A it is a door the console keeps the keys to. Know it, read it when
you meet it in ROM, and reach for BLWP when you want a trap of your own.

## 9.8 Coroutines and State Machines by Workspace Switching

A subroutine has a subordinate relationship to its caller: it runs, it returns, it is done. A
**coroutine** is an equal. Two coroutines take turns — each runs for a while, then *yields* to the
other, and later resumes exactly where it yielded, with all its state intact. It is the pattern
behind smooth game logic (the player-input strand and the enemy-AI strand advancing in lockstep),
behind generators and producers-and-consumers, behind any code that is naturally two stories told
one paragraph at a time.

On most processors coroutines take real machinery to build, because "resume exactly where it
yielded, with all its state intact" means saving and restoring a full register set at every switch.
On the 9900 that machinery is already in the silicon, because a workspace *is* a saved register set.
Give each coroutine its own workspace and its state — all sixteen registers — survives a switch for
free, with no save loop at all. The switch itself is a context change, and §9.4 already named the
instruction for a context change: BLWP, to a service that swaps one saved context for another.

A **state machine** is the coroutine's simpler sibling and uses the same parts: a "current state"
that is an address you B to, or an index into a jump table (§9.5), advanced by events. Where a
coroutine keeps a whole workspace of live state, a state machine often keeps just one word — which
state it is in — and that word plus a jump table is an entire input parser, animation sequencer, or
protocol handler. You will build state machines explicitly for the terminal handler (Ch. 33) and
the game loops of Part IX; the coroutine, being the harder and more beautiful of the two, is this
chapter's lab.

## 9.9 Error Handling Without Exceptions

There is no `try`, no `throw`, no `catch`, and no stack unwinding on this machine, and pretending
otherwise is how period code earned its reputation for fragility. What there *is* is disciplined
convention, and the book adopts a two-tier one that has served real systems for decades.

The first tier is **the flags**, for the immediate caller. An instruction that already sets a
meaningful status — a compare, a subtract, a DIV that may overflow — lets the very next instruction
branch on the outcome. This is the fast path: no register spent, no code beyond the jump you were
going to write anyway. Chapter 8's DIVSAF, which reads the overflow bit the instant DIV sets it, is
the archetype. Use the flags when the check is right here and right now.

The second tier is **a return code in R0**, for errors that must travel. Per the convention of §9.3,
a routine reports >0000 for success and a nonzero code for failure, and the caller tests with a move
and a JNE. This is the tier that composes: a code in R0 survives the return, so a caller can check
it, act on it, or pass it up to *its* caller, building the manual equivalent of a propagated
exception one honest test at a time. The stack (§9.3) is what makes that propagation safe — each
level's return link and saved registers are preserved across the call that might fail, so unwinding
by hand is just popping what you pushed.

The rule that ties it together is cultural, not architectural, and it is the one worth internalizing:
**every routine documents which tier it uses, and every caller honors it.** A function that returns
an error code whose callers ignore it is not error handling; it is a latent crash with a
conscientious author. The machine will never force the check. The convention is the only thing
standing between a deep call chain and a return into the woods — which is where this chapter began.

> **Field Notes — The Console's Vectors, Read Cold.** Everything this chapter built by hand, the
> console does in ROM, and you can read its whole reflexive personality as a table of {WP, PC}
> pairs. Load any program on BENCH99 and dump the bottom of memory. At **>0000** sits the reset
> vector — the two words the CPU loads when it powers on or is reset — reading `{WP=>83E0, PC=>0024}`:
> the console starts life in a workspace at >83E0 and begins executing at >0024. At **>0004** is the
> level-1 interrupt vector, the one the VDP pulls sixty times a second, reading `{WP=>83C0, PC=>0900}`
> — and that workspace address, >83C0, is the same scratchpad corner where Chapter 8 watched the
> console's random-seed byte twitch frame to frame, because the interrupt service routine lives
> there and stirs it. The interrupt vectors for levels 1–15 fill >0004–>003F; the sixteen XOP
> vectors fill >0040–>007F, where §9.7's XOP 0 reads its `{>280A, >0C1C}`. There is no magic in the
> console's control flow. It is BLWP and a table, the same two things you now command — the console
> just keeps its table in ROM. Reading it cold, on a machine whose every byte you can inspect, is
> the closest thing this book offers to opening the hood while the engine runs.

## Lab 9 — `task99`: A Cooperative Multitasker in Under 200 Bytes

Everything in this chapter converges on one demonstration: a processor that officially does one
thing at a time, running two independent programs at once. Not by interrupts, not by an operating
system — by two coroutines that hand the CPU back and forth on purpose, each picking up exactly
where it left off. `code/ch09/task99.a99` is that multitasker, and the idea fits in a sentence:
BLWP/RTWP *is* a context switch, so a scheduler is BLWP/RTWP aimed at a table of saved contexts.

Each task owns a workspace, so its registers survive a yield for free — that is what "its own
workspace" buys. A task gives up the CPU with `BLWP @YVEC` and wakes on the next instruction, its
world untouched. A task's saved state is a three-word **control block** — `{WP, PC, ST}`, exactly
what RTWP restores — and the scheduler keeps two pointers, CURPTR (the running task's block) and
NXTPTR (the one to run next). The yield service is short enough to read whole:

```
* YSVC runs under its own workspace. BLWP handed us the yielding task's
* context in R13/R14/R15 (old WP / resume PC / old ST). Save it, rotate, RTWP.
YSVC   MOV  @CURPTR,R3       R3 -> running task's control block
       MOV  R13,*R3          save WP  (CB+0)
       MOV  R14,@2(R3)       save PC  (CB+2) — the resume point
       MOV  R15,@4(R3)       save ST  (CB+4)
       MOV  @CURPTR,R1       rotate: swap CURPTR <-> NXTPTR
       MOV  @NXTPTR,R2
       MOV  R2,@CURPTR
       MOV  R1,@NXTPTR
       MOV  @CURPTR,R3       load the newly-current task's context ...
       MOV  *R3,R13
       MOV  @2(R3),R14
       MOV  @4(R3),R15
       RTWP                  ; ... and become the other task, mid-stride
```

The service saves the caller's context (handed to it in R13/R14/R15 by the BLWP), swaps the two
pointers so the other task becomes current, loads that task's saved context back into the linkage
registers, and RTWPs — which resumes the *other* task, because its WP/PC/ST are now the ones in
R13/R14/R15. Two tasks make the rotation a pointer swap; a longer ring would thread a link through
the blocks instead. Booting is the one asymmetry: main sets up the pointers, seeds the second task's
block, and launches the first not through a yield but by loading its own R13/R14/R15 and executing
RTWP directly — stepping into task A as if returning to it.

The self-test makes the switch undeniable. Task A writes >AA into a shared log and yields; task B
writes >BB and yields; a real, fair switch must interleave them. On the bench the log comes out

```
>8386  AA BB AA BB AA BB AA BB
```

— strict alternation, eight hand-offs, proof that control crossed between two workspaces and back
four times. After task A's fourth turn it stops the world and a checker confirms the pattern
byte-for-byte and paints the border green (VDP register 7 = >02); the whole run is 218 instructions
and 5,556 cycles. The multitasker — scheduler, boot, and both tasks — is 160 bytes; the reusable
scheduler core is about 50; the whole file including its self-checking harness is 216. Two programs,
one CPU, switched by hand, in less code than this paragraph's worth of prose. The chapter's ideas,
made undeniable.

*Extending the lab (▶ suggested):* add a third task and replace the pointer swap with a ring link
in each control block; the yield service barely changes and the interleave becomes `AA BB CC AA BB
CC`. Then add a `yield-with-value` that passes a word between coroutines through the shared log —
the seed of the producer/consumer you will want for the game loops of Part IX.

## Exercises

**✦ Warm-ups.**

1. State, without running it, the reach of `JNE` in words and in bytes, and rewrite `JEQ FARLBL`
   (with FARLBL 200 words ahead) using the two-jump idiom. Then assemble both and confirm the
   direct form produces the out-of-range error and the idiom does not.
2. A leaf routine and a non-leaf routine differ by exactly one discipline. Write the four-line
   prologue/epilogue a non-leaf needs, using the book's PUSH/POP idiom on R10, and explain in one
   sentence why a leaf may omit it.
3. Give the two words a `BLWP` vector contains, and name the three registers of the *new* workspace
   that hold the caller's saved context after the call.

**✦✦ Consolidation.**

4. `code/ch09/link.a99` treats agreement between the before-and-after R11 recordings as a *failure*.
   Explain why, and describe the one-line change to `BADNL` that would make the test wrongly pass —
   and what real bug that change would be hiding.
5. Take the recursive `FACT` from `stack99.a99` and hand-trace the stack contents (as a column of
   words, R10 marked) at the deepest point of `FACT(3)`. Confirm against the bench that R10 returns
   to >8380.
6. Write a four-case jump table that dispatches on a selector in R1 (0–3) to four routines that load
   R2 with >00, >11, >22, >33 respectively. Note where you double the selector and why, and confirm
   on the bench that selector 2 yields >22.
7. Using `X`, write a two-instruction sequence that applies *one of two* operations to R3 — `INC R3`
   or `DEC R3` — chosen by a flag you set, without a conditional jump between the choice and the
   operation. (Hint: select which instruction word to execute.)

**✦✦✦ Extensions.**

8. Extend `task99` to three cooperative tasks using a ring link in each control block instead of the
   two-pointer swap. Prove the interleave is `AA BB CC AA BB CC` on the bench, and report the new
   code size — is the scheduler still under the 50-byte core?
9. The console's level-1 interrupt vector at >0004 is a `BLWP`-style entry. Without enabling
   interrupts (that is Chapter 22), read the vector cold on the bench, then write a *voluntary*
   `BLWP @>0004`-style call to your *own* look-alike vector that mimics what an interrupt would do —
   save context, do a trivial task, `RTWP` — and confirm the linkage registers hold what you expect.
   Explain in two sentences why the real interrupt cannot simply be a `BL`.

## Further Reading

- *TMS9900 Family Systems Design and Data Book*, Texas Instruments — the instruction descriptions
  for BL, B, BLWP, RTWP, X, and XOP, including the status-bit effects catalogued in Appendix A of
  this book. The context-switch semantics of BLWP/RTWP are stated there in the language of the
  990 architecture it inherits.
- *Editor/Assembler Manual*, Texas Instruments — the calling conventions the console's own utilities
  use (the `>2000`-region utility linkage, XML, and the DSR call), which differ from ours and are
  worth contrasting once you have internalized this chapter's; treated in full in Chapter 6 and
  Chapter 30.
- The Classic99 source (`console/cpu9900.cpp`), for the exact cycle accounting of BLWP and XOP and
  the order in which the linkage registers are written — the hardware-verified reference this book's
  measurements are checked against when the datasheet leaves a corner ambiguous.
- Any text on *coroutines* in the Knuth or Marlin lineage, read after this chapter, will show you
  that the pattern you built in 160 bytes is the same one the literature treats at length — the
  9900 simply had the register-set-swap in silicon.

## Summary

- Control flow on the 9900 is jumps, `B`, and `BL`, with everything structured built on top. A
  conditional jump reaches ±128 words (the assembler quotes −254..+256 bytes when you exceed it);
  the **two-jump idiom** — invert the condition, jump over a `B @far` — branches long, since `B`
  reaches all 64K. Measured (cart code, pad workspace): `JMP` 14, `Jcc` 12/14, `B @L` 24, `B *R` 16.
- `BL` links the return address in **R11** (RT = `B *R11` = >045B, 16 cyc; BL 28 cyc). R11 is one
  register: a **non-leaf must save R11 before calling and restore it before RT**, or it returns into
  itself — the one-breadcrumb bug, made visible in `link.a99` as before/after R11 that differ.
- **R10 is the book's software stack pointer** (full-descending): PUSH = `DECT R10`/`MOV x,*R10`,
  POP = `MOV *R10+,x`. Recursion proves it composes — `stack99.a99` computes FACT(8)=40,320 (>9D80)
  and returns R10 balanced to >8380. **Ruling R-16** fixes the book-wide calling convention: args in
  R0–R2 (caller-saved), leaf keeps R11, non-leaf saves it, R10/R13–R15 sacred, errors in R0
  (0=success).
- **`BLWP @vec`** (50 cyc) is the context-switch call: the two-word vector `{WP, PC}` (placeable
  anywhere in RAM) becomes the new context, and the old WP/PC/ST are saved into the new R13/R14/R15;
  **`RTWP`** (18 cyc) restores them. Verified linkage in `probe.a99`: >8300/>6040/>C000. Use it when
  a routine wants its own 16 registers; a BL suffices otherwise.
- Structured shapes are named jump patterns (if/else, while, repeat, for-down-to-zero). A **jump
  table** dispatched with `MOV @TAB(Rn),Rx` (selector×2) + `B *Rx` makes an n-way switch two
  instructions; `X` executes an instruction fetched elsewhere (dispatch-by-instruction, run-time
  operations). **XOP** traps through ROM vectors at >0040+n×4 (you cannot install your own on a
  stock 4A; R11 = operand address, ST X-bit set) — heritage, not daily tool.
- **Coroutines** are workspace switches: each owns a workspace, so its state survives a yield for
  free. `task99` (Lab) is a cooperative multitasker — two tasks, `BLWP @YVEC` to a scheduler that
  saves/rotates/restores 3-word control blocks; the shared log interleaves `AA BB AA BB AA BB AA BB`,
  green, in 160 bytes (50-byte scheduler core). Error handling is convention, not machinery: flags
  for the immediate caller, an R0 code for errors that must travel, and the stack that makes manual
  unwinding safe. The Field Notes read the console's own reset/interrupt/XOP vectors cold — proof
  that the machine's reflexes are the very BLWP-and-a-table you now command.
