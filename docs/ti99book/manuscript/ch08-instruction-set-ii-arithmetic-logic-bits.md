# Chapter 8 — Instruction Set II: Arithmetic, Logic, and Bits

*Chapter 7 taught this machine to move data without changing it. Now we change it — and learn to
read the confession every arithmetic instruction files in the status register.*

<!-- Part II — The TMS9900 and Assembly Fundamentals · target ≈24 pp -->
<!-- STATUS: DRAFTED (session 4, 2026-07-06) — pending review passes. All listings assemble via sh verify.sh; §8.1–8.8 flag/cycle/layout facts machine-verified on BENCH99 at commit 1ae787c; code in code/ch08/. mathtest green (>83F8/>83FA/>83FC/>83FE = >FFFF/>C0DE/>0000/>0000) / sabotaged-tap red (period >03FF, FAILS 1, first-fail id 29). Surfaced: DIV success cost is flat on libre99-core (92/96) where the datasheet is data-dependent — a timing-fidelity simplification, not a functional error; results/flags exact. MOV dest-pre-read deviation (Ch. 5) untouched here — MPY/DIV/shift/arithmetic timings are unaffected by it. -->
<!-- The 990-lineage phrasing (§8.5 sidebar) and the "TMS9900 data manual" citation reuse the ledgered forms; Classic99 Field Notes names confirmed: console/cpu9900.cpp, WStatusLookup/BStatusLookup in buildcpu(). -->
<!-- SPEC: 00-master-outline.md, section "### Chapter 8 —" (lines 257–267 in outline v1.1). That bullet list is this chapter's contract. -->

## The Machine That Could Multiply

Saturday morning, November 13, 1982 — a borrowed classroom at a community college west of
Cleveland, the kind of room where the chalk tray still holds Tuesday night's algebra. Twenty-odd
folding chairs, a coffee urn on a cafeteria tray, and up front, on a wheeled AV cart, a users'
group's communal TI-99/4A: console, monitor, the Peripheral Expansion Box humming like a window
fan. Half the consoles in the room were new that fall, bought in the rebate wave the price war
was throwing off (Ch. 1) — and so half the owners were new too.

The newcomer in the second row had spent two years programming a 6502 — an Apple II at work, a
bare-board trainer at home — and he had a veteran's reflex of assuming every microprocessor was
poor in the same ways. So when the man running the meeting mentioned, in passing, that the 9900
could multiply, the newcomer laughed before he could stop himself. He had written 6502
multiplication by hand: a shift, a test, a conditional add, eight times around the loop, one
evening to get it working and a lifetime spent distrusting it. Every micro he had ever touched
treated multiplication the way a landlord treats hot water — available, in principle, provided
you build the boiler yourself.

The meeting runner did not argue. He opened the data manual to the instruction list, turned the
book around, and put his finger on a line. MPY. Multiply — sixteen bits by sixteen bits, the
full thirty-two-bit product delivered whole. A few lines down sat DIV: thirty-two bits divided
by sixteen, quotient and remainder both. These were not library routines someone had typed in
from a magazine. They were silicon, and they had been in the design since 1976.

So the room proved it to him. Through the Mini Memory cartridge's little line-by-line assembler,
the group's secretary keyed in a scrap of a program — two numbers into registers, MPY, a stop —
and the newcomer supplied the operands from his own past: 1,234 and 5,678, the pair he had once
used to torture his shift-and-add routine. The machine did in one instruction what his trainer
had needed a subroutine, a scratch page, and an evening for. Somewhere behind the beige bezel,
two registers together now held a number too big for either alone: 7,006,652.

Held it — and would not say it. That was the joke the room had been waiting to spring, the mild
hazing every new member evidently got. There is no PRINT in this machine's assembly language, no
output statement of any kind; as far as the newcomer could tell, paging the manual with rising
disbelief, there was nothing between him and the screen but raw video memory and his own wits.
The product of his two numbers existed — thirty-two bits of it, spread across two registers in a
notation no human reads at speed — and the machine had no opinion about ever showing it to him.
An old hand in the front row offered the consolations of experience. Learn the flags first, he
said, because everything you compute files a report and everything you decide reads one. And
when you get to the divide — he said this the way sailors mention weather — remember that it can
refuse the job.

The newcomer went home with a legal pad full of notes and one settled conviction: the machine
could multiply. Getting it to *admit the answer* — in decimal, on the screen, like a civilized
adding machine — was going to be his job. This chapter is where it becomes yours.

---

## What You Will Learn

By the end of this chapter you will be able to:

- Predict, from §8.1's table, the full LGT/AGT/EQ/C/OV (and, for byte ops, OP) outcome of any
  add, subtract, increment, negate, or compare — and name the instructions that touch no flag at
  all.
- Choose the correct conditional jump for a signed question versus an unsigned one, and produce
  an operand pair that makes the wrong choice give the wrong answer.
- Test bit masks with COC and CZC, and write them with SOC, SZC, XOR, and INV — including
  extracting and inserting multi-bit fields in packed data.
- Multiply and divide by powers of two with the four shifts, take a run-time shift count from
  R0, and use the carry bit as a one-bit data channel.
- Use MPY and DIV with their register-pair results, state DIV's overflow rule and guard against
  it, and wrap both instructions for signed operands with a defensible remainder convention.
- Add, subtract, compare, and negate 32-bit values on a CPU that has no add-with-carry — and
  keep the carry alive across the seam between instructions.
- Convert a 16-bit value to decimal ASCII by two different algorithms, choose between them on
  measured cost, and emit hexadecimal almost for free.
- Generate pseudo-random numbers with an LFSR whose period is proven, not presumed, and seed it
  from the console's own entropy.
- Prove every routine above with a self-checking test harness whose verdict BENCH99 reads out of
  memory — the book's first lab that grades itself.

## The Bridge: What the Compiler Never Told You

In the languages you grew up with, this chapter is about five characters long: `a * b`, `n / 10`,
`x & mask`, `x >> 4`, `rand()`. Each of those is a decision someone else made for you — which
machine instruction to emit, where a 32-bit product should live, what a division by zero should
do, what "random" means on a deterministic machine. You have been on intimate terms with an ALU
your entire programming life; you have simply never been introduced. This chapter performs the
introduction, one instruction family at a time, and by its end the five expressions above will
each have become a small, visible piece of engineering with a cost you can measure.

Two ideas carry the chapter. The first: on this machine every arithmetic instruction returns two
results — the value it computed, and a report about that value filed in the status register.
Modern processors still work exactly this way (x86 keeps its EFLAGS, ARM its NZCV bits); the
difference is that your compiler reads those reports so you never have to. Here you are the
compiler. The second idea: signedness is not a property of data. The same sixteen bits are
32,768 to one observer and −32,768 to another, and the 9900 — honest to a fault — keeps two
separate greater-than flags so that both observers get a correct answer. In C, the signed/
unsigned decision lives in the type system, made once at the declaration. In assembly it lives
in the jump instruction, made freshly at every comparison. Nothing in this book punishes
carelessness faster (§8.2).

And two familiar comforts simply do not exist. There is no `printf` — §8.7 builds decimal output
from raw division and subtraction, resolving the vignette's embarrassment — and there is no
`rand()` — §8.8 builds one from a shift register and a lucky handful of XORs. What *does*
transfer intact from your modern habits is the unit test. Lab 8 packages this chapter's routines
into `mathlib` and then does something no lab in this book has done yet: it proves them, with a
harness that runs every routine against known vectors and writes a verdict into memory where the
bench can read it. "Runs in the emulator" is evidence, not hope (Ch. 3) — this is the chapter
where that slogan grows teeth.

## 8.1 Add and Subtract: Arithmetic That Files a Report

Chapter 7's instructions were couriers: they moved bytes and words and left the cargo unchanged.
This chapter's instructions are the ones that open the crates. The founding family is addition
and subtraction, and it comes in three sizes. A and S are the general two-operand forms — add or
subtract source into destination, both operands drawn from any addressing mode you learned in
Ch. 7. AB and SB are their byte twins, and the high-byte law has not gone away: aim a byte
operation at a register and it is the *high* byte that participates. Then come the quick four —
INC, INCT, DEC, DECT — which add or subtract one or two without an immediate operand, and the
sign pair NEG and ABS, which flip and unflip. Why a dedicated "by two"? Because this is a
sixteen-bit machine that steps its words by twos: a pointer walking a word table advances with a
single INCT, and when Ch. 9 builds the book's stack on R10, DECT and INCT will be its heartbeat.

Every one of these instructions files a report. The status register's flags are not global
weather — they are the last arithmetic instruction's sworn statement about its own result:
logically greater than zero, arithmetically greater, equal to zero, carry out, overflow. The
discipline this chapter installs is to treat that statement as *perishable*. It is accurate only
until the next flag-writing instruction speaks, which is usually one instruction later, so the
jump that consumes a flag wants to stand immediately behind the arithmetic that produced it. How
perishable, exactly? That depends on knowing which instructions are silent. You met one in
Ch. 7: SWPB swaps its bytes and says nothing — not one status bit moves. It is not alone. There
is a whole quiet family, and knowing its membership by heart is what will make §8.6's multi-word
arithmetic survivable, because a carry you need can only be preserved by instructions that do
not talk over it.

The table below is the datasheet's flag definitions with the ambiguity boiled off, and every row
was read straight off the bench — the `s` trace prints the decoded status register after each
instruction, so nothing here is remembered; it is all observed (commit 1ae787c). A check mark
means the instruction sets that flag from its result; a dash means it does not touch that flag at
all. `AI` is the add-immediate Ch. 7 filed with the other full-word immediates (`LI`, `ANDI`,
`ORI`, `CI`); it reports exactly as `A` does. The byte forms obey the high-byte law — aimed at a
register they read and write its high byte (Ch. 7) — and file one extra report, the parity flag
OP, that the word forms never touch.

| Instruction | L> | A> | EQ | C | OV | OP |
|---|:--:|:--:|:--:|:--:|:--:|:--:|
| `A` add · `AB` add byte | ✓ | ✓ | ✓ | ✓ | ✓ | AB only |
| `S` subtract · `SB` subtract byte | ✓ | ✓ | ✓ | ✓ | ✓ | SB only |
| `AI` add immediate | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| `INC` +1 · `INCT` +2 | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| `DEC` −1 · `DECT` −2 | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| `NEG` negate | ✓ | ✓ | ✓ | ✓ | ✓ | — |
| `ABS` absolute value | ✓ | ✓ | ✓ | ✓ | ✓ | — |

Read the report by its columns. **L>** and **A>** are the two rulers §8.2 makes its whole subject
— logical (unsigned) and arithmetic (signed) greater-than-zero. **EQ** is result-is-zero. **C** is
the carry out of bit 15 on an add; on a subtract it is the same wire read the opposite way — C is
set when the subtraction did *not* borrow, and §8.6 leans its entire weight on that one sentence.
**OV** is signed overflow, the result the two's-complement range could not hold. Three rows earn a
footnote the datasheet states quietly and the bench states out loud.

First, and load-bearing for §8.6: **`INCT` and `DECT` are not flag-silent conveniences — they file
the full report, C included.** Step `INCT` and watch it happen. With the carry flag left standing
by a previous add, `INCT R4` on `>0010` — which does not itself carry — comes back with C
*cleared*; `INCT R5` on `>FFFE` comes back `>0000` with C *set* and EQ set. The pointer bump you
reach for by reflex overwrites exactly the bit a multi-word add is trying to keep. Hold that
thought until §8.6; it is the chapter's most expensive quiet fact.

Second, the sign edge. `NEG` and `ABS` share one input they cannot honor — `>8000`, the most
negative 16-bit number, whose positive twin `+32,768` will not fit in sixteen signed bits. Ask for
either and the machine hands back the bit pattern unchanged (`>8000` in, `>8000` out) and raises OV
to confess it: on the bench, `NEG` of `>8000` and `ABS` of `>8000` both return `>8000` with OV set.
Every signed routine in this chapter that takes a magnitude (§8.5's `MULS` and `DIVS`) must treat
that one value as the special case it is. A related `ABS` subtlety: its flags describe the
*original* operand, not the magnitude it stored — `ABS` of `>FFFB` (−5) leaves A> *clear*,
reporting the −5 it was handed even as it writes the `>0005` that is plainly positive.

### The quiet family

Perishability cuts the other way, too. A handful of instructions change a value and report
*nothing*, and a carry you need across §8.6's seam survives only if every instruction standing
between the add and its jump is one of these. Ch. 7 met the first — `SWPB` swaps bytes and sets no
flag. Here is the full roster the datasheet defines as altering no status bit:

| Group | Instructions that touch no status flag |
|---|---|
| The byte wrench and the free constants | `SWPB`, `CLR`, `SETO` |
| Introspection and the control-register loads | `STWP`, `STST`, `LWPI`, `LIMI` |
| Branches — they redirect, they do not report | `B`, `BL`, `BLWP` |
| The multiply | `MPY` |
| CRU and external | `SBO`, `SBZ`, `CKON`, `CKOF`, `RSET`, `LREX`, `IDLE` |

The bench confirms the ones we can reach in bare mode — `MPY`, `LWPI`, and Ch. 7's `SWPB`/`CLR`/
`SETO`/`STWP`/`STST` all step past a loaded status word without disturbing a bit — and the
datasheet (App. A premise, R-11) carries the branch and CRU rows. Now note who is *absent*. `MOV`
reports (Ch. 7). Every instruction in the flag table above reports. The four shifts report (§8.4).
`LDCR`, `STCR`, and `TB` report. So the only tools legal between a carry-producing add and the jump
that consumes its carry are setup and branch instructions — and almost nothing you would naturally
want mid-calculation is among them. That scarcity is not trivia; §8.6 is where it starts costing
money.

> **Field Notes — Executable datasheets.** The tables above have three parents. The first is the
> TMS9900 data manual — authoritative, but written in prose, and prose can mumble. The second is
> this book's own machine: `libre99-core`, the Rust CPU the bench runs on, which cannot mumble —
> code that computes a flag must commit to an answer. The third is Classic99, the community's
> long-serving reference emulator, whose CPU core (`console/cpu9900.cpp`) bakes decades of hardware
> cross-checking into the word- and byte-status lookup tables `WStatusLookup` and `BStatusLookup`,
> built once at start-up in its `buildcpu()` routine. When a datasheet sentence admits two
> readings, an emulator source is the community's ruling on which one real silicon chose. The
> method travels well beyond this chapter: when in doubt, read the emulator — and when emulators
> disagree with the paper, say so in the open. That is the ledger's deviation-row pattern, and you
> will see it used.

## 8.2 Comparison Done Right: Two Rulers and Two Bit Detectives

A comparison is a subtraction with the tact to throw away the difference and keep only the
report. C compares word to word, CB byte to byte, CI register to immediate — and because nothing
is stored, nothing is destroyed: comparing leaves both operands exactly as they were, which is
precisely why it exists. You could subtract and test, but then you would have to want the
difference. C lets you ask a question without paying for an answer you intend to discard.

The interesting part is not the subtraction — it is that the 9900 answers every comparison with
*two rulers at once*. Take the bit pattern >8000 and hold it against >0001. Read as unsigned
quantities, >8000 is 32,768 and towers over 1. Read as signed two's complement, the same pattern
is −32,768 and grovels beneath it. Neither reading is more correct; they are different questions
that happen to share a bit pattern. So the status register carries a logical greater-than and an
arithmetic greater-than, side by side, and the jump family splits to match: JH, JHE, JL, JLE
consult the unsigned ruler; JGT and JLT consult the signed one; JEQ and JNE serve both, because
equality is the one question the two observers agree on. Compare >8000 to >0001 and ask both:
JH says *higher*, JGT says *not greater* — and both are telling the truth. In C the signedness
decision was made once, in the type; here you make it at every jump, and §8.2's table is the
contract you sign each time.

Then there are the two instructions your laptop genuinely does not have. COC — compare ones
corresponding — answers "are all the bits of this mask set in that word?" in a single
instruction, reporting through EQ. CZC — compare zeros corresponding — answers "are all the bits
of this mask *clear* in that word?" the same way. They are bit detectives: give them a warrant
(the mask) and they search the suspect without disturbing anything. The spec's boast is fair —
no mainstream CPU you grew up with offers these as single instructions — and once sprites carry
attribute bits (Ch. 16) and devices report status lines (Ch. 10), you will use them weekly. One
more flag rounds out the byte story: byte operations also file a parity report, OP, which word
operations never touch; the usual account ties it to serial-line work, where parity earned its
keep, and JOP stands ready to read it.

`C`, `CB`, and `CI` all read the same direction: the first operand measured against the second.
`C X,Y` sets the status register as though it had computed `X − Y` and thrown the difference away,
so `C R1,R2` lights L> when R1 is the higher pattern unsigned and A> when R1 is the greater value
signed. `CB` compares two bytes and, being a byte operation, also files the parity flag OP; `CI`
measures a register against a full-word immediate. Nothing is stored and nothing is harmed — which
is the whole reason the compares sit beside the subtracts that could do the same arithmetic
destructively.

Here is the promised proof that the two rulers genuinely disagree. Hold `>8000` against `>0001` and
ask both questions with one compare (`code/ch08/scraps.a99`):

```asm
       LI   R1,>8000
       LI   R2,>0001
       C    R1,R2            compare the same bits two ways at once
       JH   UNSIGN           unsigned: >8000 IS higher (taken)
UNSIGN JGT  SIGNED           signed: >8000 is NOT greater (NOT taken)
```

The bench prints the decoded status register after the compare, and it ends the argument:

```text
>6028  8081  C R1,R2   18 cycles   ST=L> - - - - - -
```

L> is set and A> is clear from one subtraction of one bit pattern from another. `JH` reads
L>·¬EQ and jumps — `>8000` really is the higher *number*. `JGT` reads A> and stays — `>8000` really
is *not* the greater *value*. Both told the truth about the same sixteen bits, because they
consulted different rulers. Pick the wrong jump for your data's signedness and you have a bug the
assembler cannot see.

The full pairing is the contract you sign at every comparison. After `C X,Y`:

| The question you are asking | Jump | Status bits it reads |
|---|---|---|
| X equal to Y | `JEQ` | EQ |
| X not equal to Y | `JNE` | ¬EQ |
| X above Y — **unsigned** | `JH` | L> · ¬EQ |
| X above or equal — **unsigned** | `JHE` | L> + EQ |
| X below Y — **unsigned** | `JL` | ¬L> · ¬EQ |
| X below or equal — **unsigned** | `JLE` | ¬L> + EQ |
| X greater than Y — **signed** | `JGT` | A> |
| X less than Y — **signed** | `JLT` | ¬A> · ¬EQ |
| X greater or equal — **signed** | `JGT` then `JEQ` | A> + EQ |
| X less or equal — **signed** | `JLT` then `JEQ` | ¬A> |

The last two rows hold the asymmetry every newcomer trips on. The unsigned world gets
single-instruction `≥` and `≤` in `JHE` and `JLE`; the signed world does not — there is no signed
`JGE`, so signed `≥` is a `JGT`/`JEQ` pair (or you swap the operands and use the `JL` family) and
signed `≤` likewise. Three more jumps read what the compares leave alone: `JOC` and `JNC` test the
carry — an add's carry, a subtract's borrow, or the bit a shift dropped (§8.4) — `JNO` tests for the
absence of signed overflow, and `JOP` reads the byte-parity flag that only the byte operations set.

Then the two bit detectives, which report through EQ alone and leave every other flag exactly as
they found it. `COC` — compare ones corresponding — sets EQ when *every* one-bit of the mask is
also set in the word; `CZC` — compare zeros corresponding — sets EQ when every one-bit of the mask
is *clear* in the word. One mask, two opposite questions (`code/ch08/scraps.a99`):

```asm
       LI   R5,>8005         the word under test  (bits 15, 2, 0 set)
       LI   R4,>0005         a 3-bit mask (bits 2 and 0)
       COC  R4,R5            EQ <=> every mask bit is SET in R5   (here: yes)
       JEQ  ALLSET
ALLSET CZC  R4,R5            EQ <=> every mask bit is CLEAR in R5  (here: no)
```

On the bench `COC` returns EQ set (both mask bits are indeed set in `>8005`) and `CZC` returns EQ
clear — and, the part that makes them safe to chain, the L> and A> standing before each instruction
are still standing after it:

```text
>6038  2144  COC R4,R5   18 cycles   ST=L> A> EQ - - - -
>603C  2544  CZC R4,R5   18 cycles   ST=L> A> - - - - -
```

`COC` and `CZC` touch EQ and nothing else. That restraint is exactly what lets §8.3 build a packed
status word you can interrogate a bit at a time without ever disturbing its neighbors.

## 8.3 Boolean Surgery: ANDI, ORI, XOR, INV — and the Mask Writers

Section 8.2's instruments only *read*. This section's instruments cut. The Boolean family on the
9900 takes a little decoding, because the mnemonics you expect are half missing: there is ANDI
and ORI — AND-immediate and OR-immediate, a constant mask applied to a register — but no plain
AND or OR reaching into memory. That role is played by two instructions with stranger names. SOC
— set ones corresponding — copies the source's one-bits into the destination: an OR by any other
name. SZC — set zeros corresponding — *clears* the destination bits where the source has ones:
AND with the mask's complement, which turns out to be what you actually want when the mask in
your hand lists the bits to remove. Both come in byte flavors, SOCB and SZCB, funnel-friendly
for packed byte data. XOR toggles — with the quirk that it insists on a workspace register for
one operand — and INV is XOR-with-all-ones compressed into a single operand: every bit flipped,
no mask required.

Why does a chapter on arithmetic care so much about masks? Economy. Your fast world is an island
of 256 bytes (Ch. 5), and the natural response to that kind of scarcity is to stop spending a
whole word on a yes-or-no fact. Pack the facts: one word can carry a handful of one-bit flags
and a couple of small fields, and the Boolean family is how you get them in and out without
disturbing the neighbors. The worked example below builds exactly that — a packed status word
for a game actor, with COC/CZC asking the questions and SOC/SZC/XOR doing the surgery — and the
pattern it establishes (test, set, clear, toggle, extract, insert) will follow you through every
sprite table and device driver in this book.

Here is the packed word `code/ch08/fields.a99` builds — one 16-bit status word for a game actor,
three one-bit flags over two small fields:

```text
   bit  15    14      13     12 11 10  9   8  7  6  5  4   3 2 1 0
       ALIVE VISIBLE HOSTILE [  HP: 4  ] [  COLUMN: 5   ] [ spare ]
```

The actor starts at `>A6A0` — ALIVE and HOSTILE lit, HP = 3, COLUMN = 10. Each operation below is
the whole idiom, and the demo snapshots the working word into the pad after every step so the bench
can read the entire history at once. Testing comes first, and it is §8.2's detectives at work — one
instruction, one jump, nothing disturbed:

```asm
       LI   R1,ALIVE         >8000
       COC  R1,R0            EQ  <=>  ALIVE is set
       LI   R1,VISBL         >4000
       CZC  R1,R0            EQ  <=>  VISIBLE is clear
```

The three writers are the surgery. `SOC` (set ones corresponding) is the OR — it copies the mask's
one-bits into the word — and it reaches memory directly, no register required. `SZC` (set zeros
corresponding) is the AND-NOT — it clears the word wherever the mask has ones, which is what you
want when the mask names the bits to *remove*. `XOR` toggles, and it insists on a register for its
destination:

```asm
       LI   R1,VISBL
       SOC  R1,R0            set VISIBLE     => >E6A0
       LI   R1,ALIVE
       SZC  R1,R0            clear ALIVE     => >66A0
       LI   R1,HOSTL
       XOR  R1,R0            toggle HOSTILE  => >46A0
```

Multi-bit fields take two instructions each way. To *extract* the HP field, mask it off and shift
it down to the bottom; to *insert* a new value, clear the field, shift the value up into position,
and OR it back:

```asm
* extract HP -> R2
       MOV  R0,R2
       ANDI R2,HPMASK        keep the HP bits  (>1E00)
       SRL  R2,9             R2 = HP           => >0003
* insert HP = 5
       ANDI R0,HPCLR         clear the field   (>E1FF)
       LI   R1,5
       SLA  R1,9             align 5 into HP   => >0A00
       SOC  R1,R0            drop it in        => >4AA0
```

Notice the division of labor: `ANDI` and `ORI` take an immediate mask into a register (the mask is
baked into the instruction stream); `SOC`, `SZC`, and their byte twins `SOCB`/`SZCB` reach any
memory operand directly; `XOR` and `INV` want a register. On the bench, one `m` dump of the
snapshot area confirms every bit picture in sequence — set, clear, toggle, extract, insert:

```text
>8360  A6 A0 E6 A0 66 A0 46 A0 00 03 4A A0
        init  +VIS  -ALV  ^HOST  HP=3  HP:=5
```

`>A6A0` → set VISIBLE `>E6A0` → clear ALIVE `>66A0` → toggle HOSTILE `>46A0` → HP extracted as
`>0003` → HP reset to 5 giving `>4AA0`. Every value is the prediction. This exact pattern — test,
set, clear, toggle, extract, insert — is how every sprite attribute (Ch. 16) and device status
byte (Ch. 10) in this book gets handled: with instruments, not brute force.

## 8.4 Shifts: Four Ways to Slide a Word

The 9900 offers exactly four shifts, and the discipline is in knowing which of the four your
problem is actually asking for. SLA slides left, feeding zeros in from the right — arithmetic
multiplication by two per step, with the overflow flag standing guard over the sign bit. SRL
slides right logically, zeros entering at the top: unsigned division by two. SRA slides right
arithmetically, smearing the sign bit downward so that negative numbers stay negative: signed
division by two, with a rounding subtlety we will pin down in the table — a right shift rounds
toward negative infinity, which is *not* what a signed divide does to the same operands, and the
difference bites exactly when you stop expecting it. SRC rotates right, circularly, the bit
falling off one end reappearing at the other. There is no rotate-left mnemonic, but you do not
need one: rotating left by *n* is rotating right by 16 − *n*, and SRC wears both hats.

Two features make these shifts more interesting than the ones you know. First, the shift count
can be a run-time value: encode a count of zero and the instruction defers to R0 — a variable
shift, decided while the program runs, and the exact rule for what R0 contributes (including the
delicious edge case where R0 itself says zero) comes from the datasheet in the table below.
Second, every shift is also a conveyor belt with the carry flag standing at the end of the line:
the last bit shifted out lands in C, where JOC and JNC can inspect it. That turns a shift into a
one-bit data channel — feed a word through SRL one step at a time and you can make a decision
per bit, which is precisely the shape of things to come when Ch. 10 introduces a whole I/O
architecture built on serial, bit-at-a-time thinking. And of course a shift is the cheap
multiply: ×2, ×4, ×8 for the price of a slide, and sums of shifts for the constants in between —
whether the cheap multiply beats the real one is a question §8.5 settles with a stopwatch rather
than folklore.

The four shifts differ only in what enters the vacated end and what the sign bit does. In every
one, the last bit to fall off the moving end lands in the carry flag:

```text
  SLA  (arithmetic left)    C <- [b15 .............. b0] <- 0     zeros in at the right
  SRL  (logical right)      0 -> [b15 .............. b0] -> C     zeros in at the left
  SRA  (arithmetic right)  b15 -> [b15 ............. b0] -> C     the sign bit smears down
  SRC  (circular right)     +-> [b15 ............... b0] -> C     the exiting bit wraps around
                            +-------------------------------+
```

`SLA` is multiply-by-two per step, and it alone watches the sign: OV is raised if bit 15 ever
*changes* during the shift, which is precisely a signed overflow. `SRL` is unsigned divide-by-two.
`SRA` is signed divide-by-two, copying the sign bit downward so negatives stay negative. `SRC`
rotates, and since a left rotate by *n* is a right rotate by 16 − *n*, it is the only rotate you
need.

The count comes from one of two places. A count of 1–15 in the instruction is taken literally. A
count of **0** is the signal to take the count from R0 instead — specifically R0's low four bits —
and if *those* are also zero, the count is a full **16** (the datasheet's one genuinely surprising
corner, and the reason R0 keeps turning up with side jobs; Ch. 7 met the first). That makes the
shift amount a run-time value when you want it: `SLA R1,0` with R0 holding 4 shifts by four.

The idioms fall straight out (`code/ch08/scraps.a99`), and the SRA one carries a warning:

```asm
       LI   R1,>0005
       SLA  R1,3            x * 8   -> R1 = >0028 (40)
       LI   R2,>0040
       SRL  R2,2            unsigned x / 4 -> R2 = >0010 (16)
       LI   R3,>FFFB        -5
       SRA  R3,1            signed x / 2 -> R3 = >FFFD (-3): rounds toward -inf
```

The bench returns `R1 = >0028`, `R2 = >0010`, `R3 = >FFFD`. Look hard at that last one. `SRA` of
−5 by one gives −3, not −2 — an arithmetic right shift rounds toward negative infinity, while a
signed *divide* of −5 by 2 truncates toward zero and gives −2. For non-negative values the two
agree; for negative odd values they differ by one, and that discrepancy bites exactly when you have
stopped expecting it. When you need true signed division, §8.5's `DIVS` is the honest tool; a shift
is a floor, not a divide.

The dropped-into-carry behavior turns a shift into a one-bit data channel. Feed a word through
`SRL` one step at a time and each bit lands in C, where `JOC`/`JNC` can act on it — here, counting
the one-bits of `>00B4`:

```asm
       LI   R6,>00B4        1011 0100
       LI   R7,8
CHNL   SRL  R6,1            low bit -> C
       JNC  CHNL0           C = 0: nothing to tally
       INC  @BITCNT         C = 1: count it
CHNL0  DEC  R7
       JNE  CHNL
```

The loop reports four, which is the population count of `>B4`, and it is the exact shape of the
serial, bit-at-a-time thinking Ch. 10's CRU makes an entire I/O architecture out of.

Finally the price, because §8.5 is about to stage a race that turns on it. Unlike the flat
`T = C + 4 × A` model, a shift's cost grows with its count, and `libre99-core` models that growth
faithfully. Measured on the bench with the operand in a register and the code in the cartridge:

| Shift | Measured cycles | Cost model |
|---|---|---|
| `SLA R1,1` | 18 | base |
| `SLA R1,4` | 24 | +6 |
| `SLA R1,8` | 32 | +14 |
| `SLA R1,15` | 46 | +28 |

The slope is a clean **2 cycles per bit shifted** — every extra position costs two cycles,
matching the datasheet's `12 + 2 × count` clocks (plus the one +4 fetch toll for running from the
cartridge; on the fast island the base is 14, not 18). Two consequences worth banking. A run-time
count taken from R0 costs eight cycles more than the same fixed count — the machine charges for
consulting R0 — so `SLA R1,0` with R0 = 4 benches 32 where the literal `SLA R1,4` benches 24. And a
shift is cheap only while the count is small: sliding by one is a bargain, but a variable shift by
twelve is two dozen cycles, which is the number §8.5 holds up against a hardware multiply.

## 8.5 MPY and DIV: The Luxury Instructions

Now the instructions that made the vignette's newcomer laugh, then stop laughing. MPY multiplies
two unsigned 16-bit values and delivers the entire 32-bit product — no truncation, no "high half
lost", the whole thing, landed across a pair of adjacent workspace registers. DIV runs the film
backward: a 32-bit dividend across a register pair, divided by a 16-bit divisor, yielding a
16-bit quotient and a 16-bit remainder, both kept. The precise choreography — which register of
the pair holds which half, and what that implies for how you stage operands — is the table's
job below, and it is worth memorizing cold, because every use of these instructions for the rest
of the book stands on it. Remember, too, what Ch. 4 taught about where registers live: the
workspace is memory, so a two-word product is two memory writes, and the geography of your
workspace (Ch. 5) prices them.

Luxury has a price list, and this book does not quote prices from folklore. The received wisdom
about hardware multiply on this generation of processors is that it is *slow* — slower than
clever shifting, some period sources imply. Maybe. The table below prints the datasheet's own
figures beside numbers measured on the bench, and then stages the showdown directly: multiply
by ten, once with MPY and once with the classic shift-and-add, both timed. One sentence will
crown the winner, and it will be a measured sentence. Division carries a darker footnote:
unlike any instruction you have met so far, DIV can *refuse the job*. Hand it a dividend too
large for the quotient to fit sixteen bits and it declines — the overflow flag raised, the work
left in a state the manuals describe more quietly than they should. The Pitfalls box takes that
apart with the datasheet in one hand and the bench in the other, and leaves behind a guard
idiom, DIVSAF, that checks the one condition that matters before committing.

> **Sidebar — A Multiplier in the House.** In 1976, hardware multiplication was not something a
> microprocessor owner expected. The mainstream eight-bit chips of the era — the 8080, the 6502,
> the Z80 — shipped without it, and a generation of programmers learned the shift-and-add waltz
> as a rite of passage, tuning loops by hand and swapping tricks in newsletters. The 9900
> arrived with MPY and DIV on board because it was not, at heart, a microcomputer part at all:
> it carried the instruction set of TI's 990 minicomputer line, and minicomputers multiplied. The rest of
> the industry caught up quickly — Intel's 8086 (1978) and Motorola's 68000 (1979) both brought
> hardware multiply to the mainstream — so the 9900's luxury was early rather than unique. But
> pause on what it meant in 1982: a home computer the price war would soon push below fifty
> dollars (Ch. 1) offered, in one instruction, an operation whose absence elsewhere defined an
> entire folk literature of workarounds. The newcomer laughed because on his machine multiply
> was an achievement. Here it is a *given* — and this chapter's only real job is teaching you
> what the given costs.

Both instructions work through an adjacent register *pair*, and the choreography is the same one
the whole rest of the book will use: the high word lives in the lower-numbered register.

| Instruction | Reads | Writes | Flags |
|---|---|---|---|
| `MPY Rs,Rd` | Rd × Rs, both unsigned 16-bit | Rd:Rd+1 = the 32-bit product (Rd high, Rd+1 low) | none — `MPY` is on §8.1's silent list |
| `DIV Rs,Rd` | Rd:Rd+1 ÷ Rs, unsigned 32 ÷ 16 | Rd = quotient, Rd+1 = remainder | OV only, and only on overflow (§8.5's drama) |

The pair is the thing to memorize. `MPY R5,R4` multiplies R4 by R5 and lays the product across R4
(high) and R5 (low) — so the multiplicand register is also the top half of the answer, and the
register *after* it is quietly consumed. Point it at the vignette's operands and the machine keeps
its promise:

```text
>8340  3905  MPY R5,R4   52 cycles   ST=- - - - - - -
       R4 =>006A   R5 =>E9BC
```

`>006A E9BC` is 7,006,652 — 1,234 × 5,678, the number that would not fit in one register, delivered
whole across two (and the harness's vector 12 checks exactly this pair, so the figure is
machine-verified, not retyped from the vignette). `DIV` runs the film backward: `DIV R10,R2` takes
the 32-bit dividend in R2:R3, divides by R10, and leaves the quotient in R2 and the remainder in R3.

```text
>8342  3C8A  DIV R10,R2  92 cycles   ST=- - - - - - -
       R2 =>000A   R3 =>0000
```

100 ÷ 10 = 10 remainder 0, quotient in the low register of the pair, remainder in the next. Because
registers are memory (Ch. 4), that product and that quotient/remainder pair are two memory writes
apiece, priced by wherever your workspace lives (Ch. 5) — on the island, as here, they are free of
the funnel toll.

Now the price list, because folklore says hardware multiply on this generation is *slow* and this
book quotes from the bench, not from folklore. Measured with operands on the island:

| Operation | Island | Cartridge | Data-dependent? |
|---|---|---|---|
| `MPY` | 52 | 56 | no — flat, any operands |
| `DIV` (success) | 92 | 96 | flat on `libre99-core` (see note) |
| `DIV` (overflow, refused) | 16 | 20 | it bails before dividing |

`MPY` is a flat 52 cycles no matter what you multiply — `>0002 × >0003` and `>FFFF × >FFFF` cost
exactly the same. A note on `DIV`'s cost, in the open per R-15: our core prices every successful
divide at a flat 92 cycles, and the bench confirms it is flat across a wide spread of operands
(6 ÷ 3, 0 ÷ 1, `>00FFFFFF` ÷ `>0100` all measure 96 from the cartridge). Real 9900 silicon varies
the divide with the shape of the quotient — the datasheet quotes a range rather than a single
number — so `libre99-core` models a timing simplification here, not a functional error; the results
and flags are exact, only the worst-case cycle wobble is absent. The session report flags it as a
possible fidelity refinement for the core.

That price list sets up the showdown. Multiply by ten two ways (`code/ch08/mul10.a99`), both timed
from the same region, cartridge code and pad operands. The hardware way loads the constant and
multiplies; the classic way is `x × 10 = (x << 3) + (x << 1)`:

| ×10, measured body (cartridge) | Cycles |
|---|---|
| `MPY` — `LI R2,10` · `MPY R2,R1` · `MOV R2,R1` | 94 |
| shift-and-add — `MOV` · `SLA R1,3` · `SLA R2,1` · `A R2,R1` | 76 |

The shift-and-add wins, 76 to 94 — and the one-sentence verdict is more interesting than the
number: the hardware multiply *loses* here because ten is a small constant with only two one-bits,
so the shift version needs just two slides and an add, while `MPY` pays its flat toll regardless. Flip
the multiplier to an arbitrary 16-bit value and the verdict flips with it — a general shift-and-add
would need up to sixteen conditional steps and `MPY`'s flat cost becomes the bargain. The luxury
instruction earns its keep when the multiplier is *arbitrary*, not when it is ten.

> **Pitfalls — the divide that refuses.** `DIV` is the one instruction so far that can decline the
> job. The rule is exact: it refuses when the divisor is **less than or equal to the high-order
> word of the dividend** — equivalently, when the quotient would not fit in sixteen bits. When it
> refuses it raises OV, and — the part the manuals state too quietly — it leaves the dividend pair
> *exactly as it found it*, having bailed before doing any arithmetic. The bench shows all three
> facts in one line: dividing `>0002:>0000` by `>0001` (a quotient of 131,072, far past 16 bits),
>
> ```text
> >8340  3C84  DIV R4,R2   16 cycles   ST=- - - - OV - -
> >8304  00 02 00 00        the dividend pair, untouched
> ```
>
> OV is up, the pair still reads `>0002:>0000`, and the whole thing cost 16 cycles instead of 92 —
> the refusal is *cheaper* than the divide, because the machine checks the one condition and quits.
> The boundary is genuinely `≤`, not `<`: a high word *equal* to the divisor also overflows (a
> quotient of exactly 65,536 does not fit either), which the bench confirms — `>000A:>0000 ÷ >000A`
> raises OV, while `÷ >000B` divides cleanly. All of this holds on `libre99-core` exactly as the
> datasheet describes, so there is no deviation to log here — only a rule to guard.

The guard writes itself: check the one condition before committing. `DIVSAF` (which ships in
`mathlib`, §8's lab) compares the high word to the divisor and refuses politely — returning a
documented flag in R0 rather than a raised OV the caller might forget to test — and, because the
condition is "high word ≥ divisor," it catches a zero divisor for free:

```asm
* DIVSAF — guarded unsigned divide. Dividend R1:R2, divisor R3.
DIVSAF C    R1,R3            high word vs divisor (unsigned)
       JHE  DSAFOV           high >= divisor -> would overflow
       DIV  R3,R1            safe: quotient -> R1, remainder -> R2
       CLR  R0               R0 = 0: success
       RT
DSAFOV SETO R0               R0 = >FFFF: refused, dividend left intact
       RT
```

`JHE` is the exact translation of the overflow rule — high word *higher than or equal to* the
divisor — so `DIVSAF` refuses in precisely the cases raw `DIV` would, and the lab's vectors 16–18
prove both branches, including the divide-by-zero the guard swallows.

The section closes by paying an honest debt: MPY and DIV are unsigned, and the world is not.
Signed multiply and divide are built, not bought — record the sign the result must carry (the
operands' signs disagree exactly when the answer is negative), take magnitudes, run the unsigned
instruction, and restore the sign afterward, with one policy decision you must make consciously:
when a signed division has a remainder, whose sign does the remainder take? The library commits
to an answer and documents it, because a convention you can state is a bug you cannot have.

The recipe is honest bookkeeping, not a new instruction. The sign of a product or quotient is
negative exactly when the operands' signs disagree, and `XOR` computes that in one stroke: XOR the
two operands and bit 15 of the result *is* the answer's sign. Record it, take both magnitudes with
`ABS`, run the unsigned `MPY` or `DIV`, and negate the result if the recorded sign says so. `MULS`
does it for a signed product:

```asm
* MULS — signed R1 * R2 -> R1:R2 (R1 high, R2 low).
MULS   MOV  R2,R0            ...
       XOR  R1,R0            R0 bit15 = sign(R1) ^ sign(R2) = product sign
       ABS  R1              |R1|  (>8000 stays >8000 — the right magnitude)
       ABS  R2              |R2|
       MPY  R2,R1            unsigned product -> R1:R2
       MOV  R0,R0            product sign?
       JGT  MULSX            positive -> done
       JEQ  MULSX
       ...                   negative -> negate the 32-bit product
MULSX  RT
```

The `>8000` edge from §8.1 is doing quiet, correct work here. `ABS` of `>8000` returns `>8000` and
raises OV — but `>8000` read as an *unsigned* magnitude is 32,768, which is exactly `|−32,768|`, so
`MPY` gets the right magnitude and the final negate restores the sign. The lab's vector 9 checks
`−32,768 × 2 = −65,536` for exactly this reason; the edge that looks like a trap is load-bearing.

`DIVS` is the same idea over `DIV`, plus one policy decision the book must make out loud: when a
signed division leaves a remainder, whose sign does it take? This library implements **truncated
division — the remainder takes the dividend's sign** (the rule C and most modern languages use), and
it is book-wide from here on. One pair of vectors makes the choice visible: `−7 ÷ 2` gives quotient
−3, remainder −1; `7 ÷ −2` gives the same quotient −3 but remainder **+1**. The quotients agree; the
remainders differ, and only because the remainder follows the *dividend*, not the divisor. The lab's
vectors 13–15 pin all three sign combinations, so `DIVS` needs no separate transcript — the harness
is its proof, and a convention you can state (and test) is a bug you cannot have.

## 8.6 Wider Than a Word: 32-Bit Arithmetic by Hand

Sixteen bits run out faster than intuition suggests. A score with bonuses breaks 65,535 in an
afternoon of play; money kept honestly in cents overflows at $655.36; and this machine at
3.0 MHz executes so many cycles that a 16-bit counter fed one count per cycle would wrap roughly
46 times every second. MPY has already forced the issue anyway — its product is born 32 bits
wide — so the question is not whether you will do multi-word arithmetic, but how.

Here is the uncomfortable truth the spec hands us: the 9900 has no add-with-carry instruction.
On most processors you chain wide additions with a special add that folds the previous carry in;
here, the carry from the low-word addition exists only as a flag — one perishable bit in ST —
and the chain is built from an ordinary A followed immediately by a conditional jump that
decides whether the high word gets an extra INC. *Immediately* is the operative word. Between
the addition that produces the carry and the jump that consumes it, you may place only
instructions from §8.1's silent list — and almost nothing you would naturally want there is on
it. The classic self-inflicted wound is the pointer bump: an innocent INCT slipped between the
low-word add and the carry check, which files its own report and buries the one you needed. The
Pitfalls box below commits that crime on the bench, in public, and then shows the reordering
that makes it impossible. Subtraction chains work the same way with the borrow — which on this
machine travels through the same carry bit, under a sign convention we will pin to the datasheet
before trusting it with anything.

Out of the pattern comes a module. ADD32, SUB32, CMP32, NEG32 — the chapter's first real library
code, written once, proven in Lab 8, and adopted for the rest of the book, with a register-pair
convention (which register carries the high word) declared once and defended in the table. This
is deliberately how the book will build everything from here on: a pattern earned in prose, then
a routine with a name, a contract, and a test — the muscle Ch. 11 will formalize into `lib99`.

The pattern has two legal shapes, and they are mirror images. Add the low words; the carry now
stands in C. Either jump *into* an `INC` of the high word when the carry is set (`JOC`), or jump
*around* that `INC` when it is clear (`JNC`):

```asm
* JOC-into                        * JNC-around  (the library's choice)
       A    R4,R2                        A    R4,R2
       JNC  $+4                          JNC  SKIP
       INC  R1                           INC  R1
       A    R3,R1                  SKIP   A    R3,R1
```

They compute the same thing; the module uses the `JNC`-around form because it names its
label instead of counting bytes with `$+4`, and a named jump is a jump you can read six months
later. Before trusting it, though, one fact must be nailed rather than assumed: the *borrow sense*
of `S`. The datasheet says the carry flag means "no borrow," and the bench agrees without
ambiguity — subtracting `>0001` from `>0000` (which must borrow) and `>0003` from `>0005` (which
must not):

```text
>8340  6001  S R1,R0   14 cycles   ST=L> - - - - - -    0 - 1: borrow, C CLEAR
>8342  6001  S R1,R0   14 cycles   ST=L> A> - C - - -   5 - 3: no borrow, C SET
```

So a subtract chain reads the carry *inverted* from an add chain: after the low-word `S`, a
**clear** carry means borrow, and the high word must be decremented. The module is four routines,
and it declares one convention the rest of the book obeys: a 32-bit value lives in an adjacent
register pair with the **high word in the lower-numbered register** — the same layout `MPY` and
`DIV` chose for us (§8.5), so the hardware and the library agree on where the top half lives.

```asm
* ADD32 — R1:R2 += R3:R4.  low words, carry folded into the high word.
ADD32  A    R4,R2            low words; C = carry
       JNC  ADD32H
       INC  R1               fold the carry
ADD32H A    R3,R1            high words
       RT
* SUB32 — R1:R2 -= R3:R4.  a CLEAR carry (borrow) decrements the high word.
SUB32  S    R4,R2            low words; C set = no borrow
       JOC  SUB32H
       DEC  R1               borrow: pull one from the high word
SUB32H S    R3,R1
       RT
```

`CMP32` compares the high words first and consults the low words only when the high words are equal
— the unsigned form shown in the lab; the signed variant differs by exactly one instruction, using
`JGT`/`JLT` on the high word instead of `JH`/`JL`. `NEG32` is invert-both-and-add-one, the carry
crossing the same seam. All four are proven by the lab (vectors 1–8).

> **Pitfalls — the pointer bump that eats the carry.** Everything above rests on one fragile fact
> from §8.1: the carry lives exactly one instruction. Between the low-word `A` and the jump that
> reads its carry, you may place only §8.1's flag-silent instructions — and the one you reach for
> most, a pointer bump, is not among them. `INCT` files the full report, C included (§8.1). Watch
> the crime (`code/ch08/carrybug.a99`): both routines add `>0000:>FFFF + >0000:>0001`, whose true
> sum is `>0001:>0000`; `BUGADD` slips an `INCT R5` between the `A` and the `JNC`, and `FIXADD`
> moves that same bump *after* the carry is spent. The bench reads the two sums straight out of the
> pad:
>
> ```text
> >8340  00 00 00 00     BUGADD:  0000:0000  -- INCT cleared the carry; the high word never got it
> >8344  00 01 00 00     FIXADD:  0001:0000  -- carry consumed first, then the pointer moved
> ```
>
> The `INCT` did exactly its job — it advanced the pointer and, as §8.1's table promises, filed a
> fresh carry report of its own (no carry out of `>8380 + 2`, so C came back clear) — and in doing
> so it overwrote the one carry the addition needed. The fix is not a new instruction; it is
> *order*. Consume the perishable carry, then do the bookkeeping. This is the whole reason §8.1
> made you memorize which instructions stay silent.

## 8.7 Printing a Number: Decimal Without Hardware Help

Return, at last, to the newcomer's humiliation: 7,006,652 sitting in two registers with no way
to say so. The machine's arithmetic is binary and unrepentant about it — there is no decimal
mode, no BCD instruction set, no conversion service in the CPU. Decimal is not the machine's
language; it is a *diplomatic translation* performed entirely by your code, one digit at a time,
and there are exactly two classical ways to perform it.

The first is the divide-by-ten chain, and DIV makes it almost embarrassingly direct: divide the
value by ten and the remainder is a digit — the *last* digit. Divide the quotient again and the
next-to-last digit falls out, and so on until the quotient dies. The digits arrive in reverse,
which every implementation must confront: buffer them and play them back, or fill a field from
its right-hand end. The second method inverts the deal. Subtract ten thousand from the value as
many times as it will go, and the count of subtractions is the *first* digit; proceed down the
powers — >2710, >03E8, >0064, >000A, and the units — and the digits arrive in reading order,
streaming left to right with no buffer at all, at the price of a small table of powers and a
subtraction loop per digit. Chain or table: one needs DIV and delivers digits backward, the
other needs only S and delivers them forward. Which is *faster* is not a matter of taste, and
the measured comparison below ends the discussion the way this book prefers — with numbers.
Both must also face the small vanities of formatting: suppressing the leading zeros a five-digit
field would otherwise flaunt, or deliberately keeping them when a scoreboard wants `00042`
(Ch. 18 will want exactly that).

Two footnotes complete the toolkit. The digits themselves are a lesson in ASCII arithmetic — a
computed digit and its printable character differ by exactly >30, one ORI or AI apart. And
hexadecimal output, which sounds harder, is actually the free gift of §8.4: a hex digit is four
bits, four bits are one shift-and-mask away, and sixteen glyphs sit in a little table. Hex is
the honest radix for addresses, and once HEX16 exists you will wonder how you debugged without
it. Where does all this output *go*, this far from Ch. 12's screen? Into a memory buffer — which
is no consolation prize, because a buffer in memory is exactly what the bench can read, and
exactly what Lab 8's harness will byte-compare against expectation. The screen can wait; the
proof cannot.

`U16DEC` takes the divide-by-ten road. Each `DIV` by ten peels off one remainder — a digit — and
the digits fall out *last* first, so the routine fills a five-character field from its right-hand
end backward. The remainder arrives in a register's low byte; adding `>30` turns it into an ASCII
digit (`0` is `>30`), and `SWPB` lifts it into the high byte the store wants:

```asm
* U16DEC — R1 -> five ASCII digits at R2, right-filled with leading zeros.
       AI   R2,4             R2 -> the rightmost slot
       LI   R7,10
       MOV  R1,R6            R6 = value
       LI   R3,5
UDECL  CLR  R5              high word of the dividend = 0
       DIV  R7,R5           R5 = value/10, R6 = value mod 10
       AI   R6,>0030        digit -> ASCII
       SWPB R6              into the high byte (the high-byte law)
       MOVB R6,*R2          store it
       DEC  R2             step left
       MOV  R5,R6          the quotient feeds the next position
       DEC  R3
       JNE  UDECL
       RT
```

`U16DEB` inverts the deal with a table of powers — `>2710`, `>03E8`, `>0064`, `>000A`, `>0001`
(10000 down to 1). For each power it subtracts as many times as it can; the count is that digit, and
because it works from the largest power down, the digits stream out *forward*, in reading order,
with no buffer to reverse. That also makes leading-zero handling a local decision: a flag in R3
chooses between zero-padding (`00042`, which Ch. 18's scoreboard wants) and blank-padding
(`   42`), with the units digit always printed so zero renders as `00000` or `    0`:

```asm
UDEBS  C    R1,R10           value >= this power?
       JL   UDEBD            no -> the digit is done
       S    R10,R1           subtract the power
       INC  R0              digit++
       JMP  UDEBS
```

Hexadecimal, which sounds harder, is the free gift of §8.4: a hex digit is four bits, one shift and
a mask away, and sixteen glyphs sit in a `TEXT '0123456789ABCDEF'` table. `HEX16` rotates each
nibble to the top, indexes the table, and copies the glyph — four nibbles, four characters, almost
no arithmetic. The harness byte-compares every one of these outputs against a known string (ASCII
`>30`–`>39` for digits, `>41`–`>46` for hex), so "it converted" means "the bytes matched," not "it
looked right."

Which decimal converter is faster is a measured question, and the two answers are different in kind
(cartridge code, pad operands):

| Input | `U16DEC` (divide chain) | `U16DEB` (subtract table) |
|---|---|---|
| `>0000` | 1234 | 1524 |
| `>0009` | 1234 | 2158 |
| `>FFFF` | 1234 | 2978 |

`U16DEC` is *flat* — always five divides, so always 1234 cycles regardless of the value — while
`U16DEB` is *data-dependent*, its subtract loop turning once per unit in each digit, so a field of
nines costs nearly twice a field of zeros. On cost alone the divide chain wins outright and
predictably. (One honest asterisk, inherited from §8.5: `U16DEC`'s flat number rests on `DIV`'s flat
cost on our core; real silicon varies the divide slightly, so hardware would show a small wobble the
bench does not.) But `U16DEB` earns its place by *shape*: it streams forward with no reversal buffer
and hosts leading-zero policy naturally, which is why the scoreboard in Ch. 18 will reach for it
even though it costs more. Use the divide chain when you want speed and a fixed width; use the
subtract table when you want forward streaming and formatting control. Either way, the newcomer's
7,006,652 can finally be *said* — turned from two registers of binary into a string of digits a
human reads — which was the whole point.

## 8.8 Dice for a Deterministic Machine

Every game this book builds — the dodging (Ch. 16), the meteors (Ch. 39), the dungeons (Ch. 41)
— needs the machine to surprise you, and the machine is constitutionally incapable of it. Given
the same state, the 9900 will do the same thing until the power fails; determinism is not a
limitation of the chip but its entire job description. So we do what every game programmer since
the beginning has done: we manufacture surprise from arithmetic that merely *looks* lawless, and
the classic machinery for it costs almost nothing on this instruction set. A linear-feedback
shift register is a word, a shift, and a conditional XOR against a constant — the "taps." Shift
the word; if the bit that fell off was a one, XOR the tap constant in. That is the whole
generator. Choose the taps well and the register marches through every one of the 65,535
nonzero 16-bit states before repeating — a period of 2¹⁶ − 1, the best a 16-bit register can do
— while visiting them in an order that passes a casual eye for chaos. Choose the taps badly and
the orbit collapses to something short and embarrassing. This book does not ask you to trust its
tap constant: the Lab 8 harness *counts the period on the machine* — sixty-five thousand
iterations are an idle moment for the bench — and asserts the full 65,535. One state is
genuinely cursed, and it is zero: an LFSR at zero shifts zeros, XORs nothing, and stays zero
forever. The seeder's first duty is refusing that seed.

Which raises the real question — where does a seed come from on a machine that cannot surprise
itself? The honest answer is: from outside the machine. The player is the entropy source; the
cycle count at which a human finally presses a key is noise of a very serviceable grade, an idea
Ch. 21 will build into the input library properly. But the spec of this chapter promises
something better still: *the console's own seed byte*. TI's firmware keeps one — a byte the
console disturbs as the machine runs, sitting there for the taking. We could print its address
from folklore right now, and we will not. *TI Intern* maps it; the bench can watch it move; the
passage below states it with the evidence attached, which is the only way this book states an
address it did not inherit from its own ledger.

So we keep the honest posture. Boot the stock console on the bench, let it run, and watch the word
at `>83C0` — which Ch. 5's live-console survey already placed in the scratchpad's ISR-and-GPL churn
region — move on its own:

```text
booted bare console
>83C0  00 00        at power-up
>83C0  A9 BE        after 20 frames
>83C0  08 C2        after 20 more
```

The word changes frame to frame, with no program of ours touching it — an entropy source the
firmware stirs as it runs. *TI Intern* maps the console's random-number seed to this corner of the
scratchpad, and this is the byte to seed `RND` from. Two honesties, though, in the book's usual key.
First, the exact identity of the cell is *TI Intern* territory (tier 2), and this session read the
mover on the bench rather than the map — so we assert what we watched (`>83C0` moves) and attribute
the seed role to the reference we could not open here, hedged. Second, and more practical: it moves
*because the console's interrupt handler runs every frame and stirs it*. A bare-cartridge program
that never enables interrupts — every program in this chapter — will find that byte frozen at
whatever it held on entry. So for our programs the real entropy is the player: the cycle count at
the instant a human finally presses a key, an idea Ch. 21 builds into the input library and Ch. 22
returns to when interrupts stop being optional.

That is where the generator comes in, because once you have *a* seed, one word and three
instructions manufacture all the surprise you need. `RND` is a 16-bit linear-feedback shift register
in Galois form: shift right, and if the bit that fell off was a one, XOR in the tap constant.

```asm
* RND — advance a 16-bit maximal LFSR. R1 = state in, R1 = next out.
RND    SRL  R1,1             C = the bit shifted out
       JNC  RNDX             0 -> no feedback
       XOR  @TAP,R1          1 -> fold in the taps
RNDX   RT
TAP    DATA >B400            x^16 + x^14 + x^13 + x^11 + 1
```

The tap constant is `>B400`, and the period is not cited — it is *counted*. The lab's harness seeds
the register with `>0001` and steps `RND` until the state returns to `>0001`, tallying the steps in
a 16-bit counter, and asserts the tally is `>FFFF`. It is: the run stamps `>FFFF` into the pad, so
`>B400` walks all **65,535** nonzero states before repeating — a maximal LFSR, proven on the machine
(and the same harness, handed the wrong tap, measures a collapsed period of 1,023 and fails loudly,
which is how you know the test has teeth). One state is fatal and it is zero — `SRL` of zero is zero,
XOR of nothing is nothing, so an LFSR at zero stays there forever — which is why `RNDS` exists to
refuse a zero seed, and why the harness checks that `RND(0)` really does stay `0`.

Ranging without division is the last trick. To fold a raw LFSR word into `0..N−1`, multiply by N and
keep the *high* word of the 32-bit product — the mod-free scaling `RNDB` does in one `MPY`:

```asm
* RNDB — R1 in 0..R2-1.  R1 = raw word, R2 = N.
RNDB   MPY  R2,R1            R1 = high word of R1 * N
       RT
```

A word near `>FFFF` scaled into six lands on 5, a word near zero lands on 0, and everything between
distributes across the range — no `DIV`, no remainder, just the top half of a product the hardware
already computes whole.

## Lab 8 — mathlib, Proven: the Library and the Harness That Vouches for It

Every lab so far has ended the same way: you ran the program and *watched*. The screen looked
right, the trace looked right, and you moved on — which was fine while the programs were small
enough to inspect with your eyes. They no longer are. This chapter produced a dozen routines
with edge cases deliberately chosen to be invisible to watching: a remainder's sign, a digit
string's leading zero, a divide that refuses, a generator's orbit. So this lab does something
new, and the book never goes back: it ships the library *with its own proof*. Alongside
`mathlib` you will build a second program, `mathtest`, whose only job is to call every routine
in the library against a table of known inputs and expected outputs, count the disagreements,
and write a verdict into memory at fixed, published addresses. Then BENCH99 — which can read
memory without asking anyone's permission — loads the pair, runs to the halt, and reads the
verdict. Three words tell the whole story: a done-marker proving the run completed, a failure
count, and the id of the first failing test so a red run tells you where to look. Pass is pass,
fail is fail, and the bench does not grade on effort.

If that sounds like a unit-test harness four decades before your CI pipeline — it is, minus the
YAML. The pattern is the lab's real deliverable, worth more than any single routine in the
library: a contract in memory, a machine that checks it, and a transcript you can paste into a
ledger. We will even break it on purpose once, to see what red looks like, because a test you
have never seen fail is a test you do not actually understand. When Ch. 11 formalizes the
reader's accumulating library into `lib99`, this harness pattern rides along, and by the time
the book starts holding your programs to CQ-82's standard (Part IX), "it works" will have meant
"the bench says so" for thirty chapters.

The library and its harness ship in one file, `code/ch08/mathlib.a99`, exactly as `memlib` did in
Ch. 7 — and for a concrete reason worth stating, because it is a fact about this toolchain you will
meet again. `libre99asm` has no linker; the only way to combine source is textual inclusion, and
`verify.sh` assembles every `.a99` on its own, so a routines-only file with no `START` fails the
build outright. The clean answer is the one Ch. 7 already proved: put the routines and their harness
in a single translation unit that assembles standalone and tests itself. So `mathlib.a99` *is*
`mathtest` — the library above the `START` label, the harness below it — and `code/ch08/mathtest.bench`
is the four-line bench script that runs it and reads the verdict.

That verdict is three words at the top of the pad (ours to use in bare-bench mode, where no firmware
is competing for them), plus one bonus word carrying the proof from §8.8:

| Pad word | Holds | PASS value |
|---|---|---|
| `>83FA` | done-marker, stamped only if the run completes | `>C0DE` |
| `>83FC` | FAILS — the count of vectors that disagreed | `>0000` |
| `>83FE` | first-fail — the id of the first failing vector | `>0000` |
| `>83F8` | the LFSR period the harness counted (§8.8) | `>FFFF` |

The harness is a straight-line runner: set up known inputs, call the routine, compare the result to
a hand-computed expectation, and on any mismatch bump `FAILS` and record the first offender's id.
The comparison helpers are the whole mechanism, and they are deliberately tiny — a routine trusted to
referee must be simpler than the routines it grades (the lesson Ch. 7's `MEMCMP` taught):

```asm
* one 32-bit vector: ADD32 of 0000:FFFF + 0000:0001 must be 0001:0000
       LI   R1,>0000
       LI   R2,>FFFF
       LI   R3,>0000
       LI   R4,>0001
       BL   @ADD32
       LI   R6,>0001         expected high
       LI   R7,>0000         expected low
       LI   R0,1             this vector's id
       BL   @CK32
* ... and the referee it calls:
CK32   C    R1,R6            result high vs expected
       JNE  CK32F
       C    R2,R7            result low vs expected
       JNE  CK32F
       RT
CK32F  INC  @FAILS           a disagreement: count it
       MOV  @FFID,R8         first failure already recorded?
       JNE  CK32X
       MOV  R0,@FFID         no -> this id is the first
CK32X  RT
```

Twenty-nine vectors run in all, and they were chosen to be the cases §8.1–§8.8 flagged as
invisible to a casual glance: the 32-bit carry and borrow crossing the word seam; `MULS` of the
`>8000` edge; the three sign combinations of `DIVS` and its dividend-signed remainder; `DIVSAF`
refusing both an overflow and a zero divisor; the digit strings byte-compared against `TEXT`
constants; `RND(0)` staying zero; and the LFSR period counted to a full `>FFFF`. One subtlety the
harness had to respect is Ch. 4's leaf-call rule: the string-test drivers call *other* routines, so
they are non-leaf and must save their own `R11` return link (and anything the inner calls would
clobber) before reaching for `BL` — the exact discipline Ch. 9 will formalize into a stack.

**The green run.** `bench99 code/ch08/mathtest.bench` loads the image, runs to the halt, and reads
the verdict out of the pad:

```text
break at >648A after 558139 instructions (10114082 cycles)
>83F8  FF FF C0 DE 00 00 00 00
```

Read it straight: period `>FFFF`, done-marker `>C0DE`, `FAILS = >0000`, first-fail `>0000`. Pass is
`>C0DE / >0000 / >0000`, and this is it — every routine in the chapter, proven in one run the bench
can repeat on demand.

**The red run.** A test you have never seen fail is a test you do not understand, so we break one
thing on purpose — change the LFSR tap from `>B400` to a non-maximal `>A400` — and run again:

```text
break at >648A after 9749 instructions
>83F8  03 FF C0 DE 00 01 00 1D
```

The done-marker is still `>C0DE` — the run *completed* — but the period collapsed to `>03FF` (1,023,
not 65,535), `FAILS` reads `>0001`, and first-fail reads `>001D`, which is 29: the period test. This
is why the verdict is three words and not one. A green done-marker means the harness finished; only
`FAILS = >0000` means it finished *clean*. The bench does not grade on effort, and now neither do
you. This contract-in-memory pattern — a verdict the machine writes and the bench reads — is what
Ch. 11 lifts into `SYSCHK`, and what Part IX's CQ-82 will hold every program in this book to.

## Exercises

8.1 ✦ Using the §8.1 table, give the full flag picture — LGT, AGT, EQ, C, OV — after each of:
`A R1,R1` with R1 = >8000; `A R1,R2` with R1 = >0001, R2 = >7FFF; `DEC R3` with R3 = >0000.
Which of the three results would mislead a signed comparison, and why?

8.2 ✦ A routine must take a jump when an index in R4 is at or beyond a buffer limit in R5. Write
the compare-and-jump twice — once treating the values as unsigned, once as signed — and name one
operand pair for which the two versions disagree.

8.3 ✦ Answer each in one instruction plus one jump, using COC or CZC: (a) every bit of mask
>9100 is set in R7; (b) no bit of mask >2040 is set in R7. State which flag your jump reads.

8.4 ✦✦ Extend §8.3's packed-status layout with a two-bit "team" field without disturbing the
existing fields. Write the insert sequence (team ← low bits of R2) and the extract sequence
(R2 ← team) — masks and shifts only, no MPY, no loops.

8.5 ✦✦ Multiply R1 by 12 without MPY, using shifts and adds. Predict the cycle cost from the
§8.4 and §8.5 tables, then measure both your version and the MPY version on BENCH99. Which wins,
by how much, and does the answer change if the code runs from scratchpad instead? (Say where
yours ran.)

8.6 ✦✦ DIVSAF (§8.5) refuses an overflowing divide. Write DIVSAT, which saturates instead:
on would-be overflow it returns quotient >FFFF, remainder >0000. Use §8.5's overflow rule for
the guard, and prove both behaviors with two vectors that would abort a raw DIV.

8.7 ✦✦ Extend the §8.6 module to a 48-bit accumulator (three words). Between the low-word add
and the final carry fix-up, which instructions are legal to interleave? Answer with §8.1's
flag-silent list in hand, and state the general rule in one sentence.

8.8 ✦✦ The scoreboard field of §8.7 wants exactly five characters: first zero-padded (`00042`),
then blank-padded (`   42`). Modify U16DEB for each policy. Which single decision point in the
routine changes — and why is the subtract-table method the friendlier host for this than the
divide chain?

8.9 ✦✦✦ Build U32DEC: 32-bit unsigned to decimal ASCII, using the §8.6 module and either §8.7
strategy. DIV alone cannot divide a full 32-bit value by ten once the quotient outgrows 16 bits
— schoolbook long division does it with two DIVs per digit. Ten digits, no leading-zero
regressions, and your vectors added to mathtest.

8.10 ✦✦✦ Swap §8.8's tap constant for one of your own choosing and rerun the harness period
test. Report the period you actually measured, and explain why a non-maximal tap set still can
never walk into (or out of) the zero state.

8.11 ✦✦✦ Add a routine of your own to mathlib — suggestion: SAT16, a saturating 16-bit signed
add — with at least four vectors in mathtest, covering both saturation directions. The bench
verdict must stay green: >C0DE, >0000, >0000.

## Further Reading

- **The TMS9900 data manual** — the instruction descriptions and status-bit definitions behind
  every premise this chapter audits in App. A. Read the arithmetic pages with §8.1's table
  beside you; the table is the manual with the ambiguity boiled off.
- **The Editor/Assembler manual** — the period reference our vignette's newcomer paged in vain
  for a PRINT statement. Its instruction-set chapter is the reader's period second opinion on
  every semantics claim made here.
- ***TI Intern*** (Heiner Martin) — the console-ROM disassembly. §8.8's seed byte comes from its
  scratchpad map, and Part VI will lean on it far harder.
- **The Classic99 source** — the reference emulator's CPU core encodes hardware-verified flag
  semantics as lookup tables (see the Field Notes in §8.1): executable arbitration for datasheet
  ambiguities.
- **This repository** — `libre99-core`'s ALU and its tests are the semantics your bench actually
  runs; the project README documents the BENCH99 workflow Lab 8 leans on (R-12).

## Summary

- Ch. 8 completes the computational instruction set. Flag semantics: every arithmetic op files a
  perishable report in ST; A/AB/S/SB/AI/INC/INCT/DEC/DECT/NEG/ABS all set the full L>/A>/EQ/C/OV
  (byte forms add OP; word forms never touch OP). INCT/DECT write C — the §8.6 pitfall; NEG/ABS of
  >8000 return >8000 with OV set; ABS's flags read the original operand. Flag-silent family (extends
  Ch. 7's SWPB): SWPB, CLR, SETO, STWP, STST, LWPI, LIMI, B, BL, BLWP, MPY, + CRU/external — nothing
  you want mid-calculation.
- Comparison C/CB/CI (first vs second operand) non-destructive; two rulers — JH/JHE/JL/JLE unsigned,
  JGT/JLT signed, no single-op signed ≥/≤ (pair idiom); >8000 vs >0001 the canonical disagreement
  (JH takes, JGT does not — bench-shown). COC/CZC report through EQ alone.
- Booleans: ANDI/ORI immediate-to-register; SOC = OR, SZC = AND-NOT reaching memory (+B forms);
  XOR/INV want a register. Packed-field idioms (test/set/clear/toggle/extract/insert) in
  code/ch08/fields.a99.
- Shifts SLA/SRA/SRL/SRC: count 0 → R0's low nibble, 0-there → 16; carry = last bit out (a data
  channel); SRA rounds toward −∞ (−5→−3, not −2). Cost slope measured 2 cyc/bit (12+2n island;
  R0-count path +8). MPY/DIV register-PAIR: high word in the lower register; MPY Rd:Rd+1 = product,
  DIV Rd = quotient / Rd+1 = remainder. Costs (island/cart): MPY 52/56, DIV 92/96 (flat on core;
  datasheet data-dependent — noted), DIV-overflow abort 16/20. ×10 showdown: shift-and-add 76 beats
  MPY 94 for a small constant. DIV overflow iff high word ≥ divisor; on refusal OV set, dividend
  pair intact, fast bail — DIVSAF guards it. MULS/DIVS signed on MPY/DIV; remainder takes the
  dividend's sign (book-wide).
- 32-bit module ADD32/SUB32/CMP32/NEG32 — pair convention: high word in the lower register
  (book-wide, matches MPY/DIV). No ADC: carry chains need flag-silent interleaving only. S borrow
  sense: no-borrow SETS C, borrow CLEARS C (bench-confirmed). INCT-in-carry-chain pitfall
  (carrybug.a99): buggy sum >00000000 vs fixed >00010000. U16DEC (divide chain, flat 1234 cyc) /
  U16DEB (subtract table, 1524–2978 cyc, forward + leading-zero policy) / HEX16; digit = value + >30.
- RND: 16-bit maximal LFSR (Galois), tap >B400 (x^16+x^14+x^13+x^11+1), period 65,535 PROVEN by
  bench count (>83F8=>FFFF); zero stays zero, RNDS refuses it; RNDB ranges via MPY high word. Console
  seed byte observed at >83C0 (moves 0000→A9BE→08C2 across frames; TI Intern maps the seed here,
  hedged; ticks only while the ISR runs — bare carts seed from the player, Ch. 21).
- Lab 8 = the book's first self-checking lab: routines + harness ship in one file (mathlib.a99, the
  memlib precedent) with code/ch08/mathtest.bench; verdict at >83FA/>83FC/>83FE = done/FAILS/
  first-fail, PASS = >C0DE/>0000/>0000 (29 vectors, machine-verified); a sabotaged tap turns it red
  (>03FF/FAILS 1/id 29). Harness pattern feeds lib99 (Ch. 11) and the CQ-82 discipline (Part IX).
