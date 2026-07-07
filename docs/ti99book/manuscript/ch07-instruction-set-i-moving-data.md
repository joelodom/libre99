# Chapter 7 — Instruction Set I: Moving Data

*Most of a program is arranging for the right bytes to be in the right places — this chapter is
where you stop doing it by accident.*

<!-- Part II — The TMS9900 and Assembly Fundamentals · target ≈26 pp -->
<!-- STATUS: DRAFTED (session 4, 2026-07-06) — pending review passes. All listings assemble via sh verify.sh; §7.1–7.6 + Lab measurements machine-verified on BENCH99 at commit 2da67ae; code in code/ch07/. memlib self-tests green (VR7=>02) / sabotage red (>06). MOV dest-pre-read deviation (Ch. 5) inherited by expansion-destination pump/fill/copy figures; A-family cross-probe confirms the hardware numbers; libre99asm has no operator precedence (left-to-right) — logged. -->

## The Wrong Half of the Word

Ames, Iowa. A Tuesday night in February 1983: snow ticking at a third-floor dormitory window at
Iowa State, radiators knocking like they bill by the hour. On the desk, tethered to a hand-me-down
color portable through an RF switchbox, sits a silver-and-black TI-99/4A that came home over
Christmas break, after the price wars finally dropped the console within reach of a work-study
paycheck. The room is a composite — there were thousands exactly like it that winter — but the bug
in it is real, and so is the hour it is about to eat.

Its owner is an engineering sophomore, and TI BASIC lasted him about five weeks. Tonight the Mini
Memory cartridge is in the slot — the one with the battery inside, the one that lets a bare console
hold machine code — and its line-by-line assembler is waiting on screen, which means every
instruction he types assembles the instant he presses Enter. No editor, no passes, no ceremony.

His ambition tonight is deliberately small: one letter A, placed on the screen by his own machine
code. He knows his ASCII cold — A is 65. The plan writes itself. Put 65 in a register. Move the
byte to the screen, the way the manual's own example does. Two instructions.

`LI R1,65`. Enter. The byte move, aimed where the example aims. Enter. Run.

The screen does not show an A. It shows nothing he can use — a blank where his letter should be.

He tries `>41` instead, hex, in case decimal had offended something. Blank. He checks the target
address against the manual. Twice. He retypes both instructions character by character, the way you
re-dial a phone number that cannot possibly be busy. Blank. This goes on for the better part of an
hour, and the maddening part is that the machine never crashes. It runs, cheerfully, every single
time — it simply does not do what he asked. Which means — and this is the thought that finally
cools him down — it is doing exactly what he asked. He just does not yet know what he said.

So he goes back to the manual, not to the example this time but to the pages on byte instructions,
and reads them at half speed. There it is — one sentence, easy to skim at full speed: a byte
instruction that names a workspace register operates on the register's *most significant* byte. The
high half. His 65 — his A — is sitting in the *low* half of R1, where `LI` put it, minding its own
business at the far end of the word. `MOVB R1` has been shipping the high half all night: `>00`,
faithfully delivered, over and over. He has spent an hour delivering the wrong half of the word.

`LI R1,>4100`. Enter. Run. An A appears, mid-screen, small and completely unremarkable to anyone
who is not in this room.

He sits back until the chair creaks, then tears an index card in half, writes BYTES RIDE IN THE
HIGH HALF on it, and tapes it above the desk, where it will stay through finals week. What he met
tonight was not a bug, and not a quirk either. It was a law — one of a small set that govern how
data moves through this machine: five ways to name a place, two instructions that do the moving,
one rule about which half of the word your bytes ride in. All of it learnable in an evening, once
somebody lays it out in the right order. Most of it expensive to learn any other way. This chapter
is that evening.

---

## What You Will Learn

By the end of this chapter you will be able to:

- Write and read all five general addressing modes — `Rn`, `*Rn`, `*Rn+`, `@LABEL`, `@LABEL(Rn)` —
  and pick the idiomatic one for a given access pattern.
- Predict, byte for byte, what any `MOV` or `MOVB` does — including byte operations on registers
  (the high-byte law) and word operations at odd addresses — from two facts: big-endian layout and
  Rn = WP + 2n.
- Set up a program's registers with `LI` and `LWPI`, interrogate the machine with `STWP` and
  `STST`, and reach for `CLR`, `SETO`, and `SWPB` where they beat `LI`.
- Write the four classic data-movement loops — copy, fill, compare, scan — idiomatically, with
  autoincrement doing the pointer arithmetic for you.
- Lay out arrays and records by hand, access fields with indexed mode, and choose word tables
  versus byte tables on purpose rather than by accident.
- Read the instruction timing table, price a loop with T = C + 4 × (accesses in the 8-bit domain),
  and confirm the prediction on BENCH99.
- Build a program that tests itself and reports the verdict through the screen border — with no
  console I/O of our own anywhere in the book yet, deliberately.
- Leave with `memlib`: measured copy/fill/compare/scan primitives, the first deposit toward `lib99`
  (Ch. 11).

## The Bridge: You Are the Library Now

Modern code moves data constantly and almost never on purpose. Assignment operators, struct copies,
slice clones, serializers — and underneath every one of them, some descendant of `memcpy` that
somebody else wrote decades ago and that your compiler now inlines so aggressively you could work a
whole career without ever seeing it. You have almost certainly never chosen an addressing mode in
your life. Your compiler chooses thousands a day on your behalf, silently, and is graded on how
well it does it.

The 9900 hands the choice back. Five addressing modes, available to *both* operands of a move —
this is a memory-to-memory machine, minicomputer manners, no accumulator toll booth between source
and destination (the sidebar in §7.1 says where those manners came from). And because this
processor keeps its registers in RAM (Rn = WP + 2n — Ch. 4), the wall between "register" and
"memory" is thinner here than on any machine you have programmed: a register is just memory with a
short name. Choosing where data lives and which mode touches it is not a detail of 9900
programming; it mostly *is* 9900 programming. Ch. 5 taught that speed is a property of addresses,
not instructions — this is the chapter where that law starts billing you per instruction.

One more habit to invert: the library is empty. No `memcpy`, no `memset`, no `strlen` exists on
this machine until you write them — so today you write them. The Lab builds `memlib` — copy, fill,
compare, scan — tests it without the benefit of console I/O, and then puts it on Ch. 5's scale to
see what moving memory actually costs through the funnel. It is the first deposit in the toolkit
this book accumulates into `lib99` (Ch. 11), and the first code you write here that you will still
be shipping in Part IX.

## 7.1 Five Doors into Memory: The General Addressing Modes

A two-operand instruction on the 9900 reads like a short sentence: a verb — `MOV` today; `A`, `C`,
`S` and their relatives in Ch. 8 — and two noun phrases naming places. The grammar's whole power is
that each noun phrase independently picks one of five forms. Learn the five once and you can read
both halves of most of the instruction set; there is no separate vocabulary for sources and
destinations.

| Written | Read it as | Reach for it when |
|---|---|---|
| `R3` | "the word in R3" (or, for byte ops, its high byte — §7.2) | the value is already in hand |
| `*R3` | "the word R3 points at" | following a pointer |
| `*R3+` | "the word R3 points at — then step R3 past it" | walking a buffer (§7.4) |
| `@PLACE` | "the word at the address named PLACE" | globals, fixed buffers, MMIO ports (`>8C00` and friends) |
| `@PLACE(R3)` | "the word at PLACE plus the offset in R3" | arrays and records (§7.5) |

Any door may serve either side. `MOV @SRC,@DST` is one legal instruction that copies memory to
memory and never touches a register at all. There is no load–operate–store ritual here; you simply
say the move. What you will *not* find among the five is a constant: the 9900 has no `#65`, and the
only way a bare number enters a general operand is as an *address* — `@65` names the word sitting
at address 65, not the number 65 — a stumble every immigrant from other assembly languages makes
exactly once. Constants come from a separate, smaller family of instructions with their own format
(§7.3).

The doors differ in price, and in two currencies. First, instruction length: an instruction is one
word, and each `@` operand appends a word after it — the address has to ride somewhere — so a
two-`@` move is three words long, and every extra word is an extra fetch through whatever domain
the code lives in (Ch. 5). Second, address arithmetic: the dereferencing doors each spend
additional cycles and, for some, additional memory accesses doing their pointer work. The table
below prices all five in both currencies, because on this machine the bill always depends on where
things live; it is the chapter's first work product for App. A's master reference. One carve-out to
memorize now: R0 cannot serve as the index in `@PLACE(Rn)` — the encoding reserves that pattern to
mean plain `@PLACE`. The addressing field is two bits (call it T); indexed mode is `T = 10` *with a
nonzero index register*, but `T = 10` with register 0 is exactly the bit pattern that already means
symbolic `@PLACE`, so there is no room left to say "indexed by R0." The datasheet resolves the clash
in symbolic's favor; libre99asm goes one better and rejects `@PLACE(R0)` outright — `@EXPR(R0) is not
indexable; R0 means 'no index'` — turning a silent misconception into a caught mistake `[libre99asm]`.
R0 is a register with side jobs — Ch. 8 will introduce another — and this is the first.

First the encoding — where each door comes from in the instruction word. The addressing field is
two bits (`T`) plus the register number, and the two symbolic forms borrow the same `T` with the
register number doing double duty (which is the whole reason R0 cannot index):

| Mode | Syntax | `T` bits | Extra instruction word? |
|---|---|---|---|
| register | `Rn` | `00` | no |
| indirect | `*Rn` | `01` | no |
| autoincrement | `*Rn+` | `11` | no |
| symbolic | `@LABEL` | `10`, reg = 0 | yes — the address rides after the opcode |
| indexed | `@LABEL(Rn)` | `10`, reg ≠ 0 | yes — the address rides after the opcode |

Now the price. The table below is `code/ch07/modecost.a99` measured on BENCH99: one `MOV` per mode,
destination held in a register, operands on the fast island, **code in the cartridge** — so the
register baseline is 18 (the datasheet's 14, plus one +4 fetch toll for living across the funnel),
and each door adds its own cycles on top:

| Source mode | Measured `MOV` | Δ over `Rn` | Datasheet cycle add | Extra accesses |
|---|---|---|---|---|
| `Rn` | 18 | — | 0 | 0 |
| `*Rn` | 22 | +4 | +4 | +1 |
| `*Rn+` | 26 | +8 | +8 | +1 |
| `@LABEL` | 30 | +12 | +8 | +1 (+ the address word) |
| `@LABEL(Rn)` | 30 | +12 | +8 | +1 (+ the address word) |

Read it in the two currencies §7.1 named. Cycles: dereferencing costs +4, auto-stepping +8, and
symbolic or indexed +8 — but the two `@` forms carry a second instruction word, and *that word must
itself be fetched*, so their measured delta is +12 here (+8 for the mode, +4 to fetch the extra
word through the cartridge). Move the code onto the island and the extra word fetches for free,
dropping the `@` delta to +8; move it to expansion and every one of these numbers is a floor, not a
ceiling. Indexed costs exactly what symbolic costs — the index register is free — which is why §7.5
reaches for it without apology. These are the rows App. A's master timing table is built from.

> **Sidebar — The Minicomputer in the Cartridge Slot.** The TMS9900 (1976) is commonly and fairly
> described as TI's 990 minicomputer architecture reduced to a single chip — and this chapter is
> where its minicomputer manners show most plainly. Memory-to-memory instructions, general
> addressing on both sides, registers kept in main memory as a movable "workspace": these are
> choices from a world of refrigerator-sized machines whose memory was not desperately slower than
> their processors, where parking your registers in RAM cost little and bought you a context switch
> that was nearly free — an inheritance Ch. 9 spends when it meets `BLWP`. Drop the same manners
> into a 1981 home computer — one 256-byte island of fast RAM and everything else behind an 8-bit
> funnel (Ch. 5) — and every one of those elegant memory-touching habits acquires a toll. That
> tension, minicomputer architecture billed at home-computer prices, is this machine's engineering
> personality in a single sentence, and this book measures it every few chapters rather than
> repeating it as folklore.

## 7.2 MOV, MOVB, and the High-Byte Law

`MOV` moves a word; `MOVB` moves a byte; the `B` suffix recurs across the instruction set (Ch. 8),
and the two of them will appear in nearly every listing you write from now to the back cover. The
first thing to install is a surprise: on the 9900, the movers also *report on their cargo*. A move
sets status flags according to the value it moved — a move is also a measurement — which is why
`MOV R1,R1` is a sentence you will one day write on purpose, and Ch. 8 will use constantly. Exactly
which flags, and which movers stay silent instead, is the small table this section owes you below.

The second thing to install is the law from the vignette, and this time we will not skim. On most
processors "which byte of the register" is not even a grammatical question — registers are their
own private country. On the 9900, registers are words in ordinary RAM: R1 *is* the memory word at
WP + 2, its high byte living at WP + 2 and its low byte at WP + 3 (Ch. 4). A byte instruction that
names R1 therefore means "the byte at R1's own address" — and R1's own address is WP + 2, which is
even, which on a big-endian machine is the *high* byte. That is the entire law. It is not a quirk
of the instruction set, and you do not have to memorize it as one: **the high-byte law is
big-endianness seen through Rn = WP + 2n.** The big end comes first; a register's address is even
by construction (registers are words, and word addressing ignores A15 — Ch. 4); so the byte a
register offers to a byte instruction is its high one, always. When both operands are registers the
law simply applies twice: `MOVB R1,R2` carries R1's high byte into R2's high byte and leaves both
low halves exactly as they were.

```text
             R1, as LI R1,65 left it
        +-----------------+-----------------+
  R1 =  |    high byte    |    low byte     |
        |      >00        |   >41  ('A')    |
        +-----------------+-----------------+
          lives at WP+2      lives at WP+3
                 ^
                 MOVB R1,... ships THIS half — always
```

Run the vignette again with that picture in view and it stops being mysterious. `LI R1,65` is a
*word* load; it fills all sixteen bits with `>0041`. The 65 you meant sits in the low half.
`MOVB R1,anywhere` ships the high half — `>00` — exactly as ordered. The repairs, in rising order
of style: load the constant pre-shifted (`LI R1,>4100`); or load it low and flip it into position
with `SWPB R1` (§7.3); or — the `memlib` way — adopt the convention that byte-sized cargo *always*
rides the high half, so there is never anything to repair. Say the law three ways and keep the one
that sticks:

- A register's byte is the byte at the register's own (even) address.
- Big end first: the high byte owns the lower address, in memory and therefore in registers.
- When a byte arrives in the wrong half, `SWPB` is the wrench.

One more reason to tattoo it now rather than later: `MOVB` is not a minor variant of `MOV` — it is
the instruction that talks to the world. The console's ports all live in the funnel's territory and
speak in bytes (Ch. 5), and when Part IV starts feeding the display — `MOVB *R1+,@>8C00`, the
pump aimed at the VDP's write port, will be the busiest instruction in the book — every byte that
reaches the hardware will be somebody's high half. Learn the law this chapter, while the worst
consequence is a blank dorm-room screen; from Ch. 12 onward it governs every pixel, pattern, and
sprite this book puts on a television.

Memory at large obeys the same picture. Every byte in the machine has its own address and `MOVB`
can reach any of them — bytes have no alignment rules. Words do: a word lives at an even address,
and a word instruction aimed at an odd one does not trap, does not warn, and does not do what you
hoped — the CPU ignores A15 and uses the aligned word (Ch. 4). The bug this manufactures detonates
far from its cause, which earns it a line in this chapter's Pitfalls box.

```text
  address:     >8300     >8301     >8302     >8303
  bytes:      [ high ]  [ low  ]  [ high ]  [ low  ]
  words:      [   word at >8300   ]  [   word at >8302   ]
```

Here is the law at work, from `code/ch07/hibyte.a99`. The result cells were pre-painted `>FFFF`, so
each byte write shows both what it changed and what it spared:

```asm
       LI   R1,>0041         'A' in the low half
       MOVB R1,@D1           D1 high <- >00, low stays >FF  => >00FF
       LI   R2,>4100         the same 'A', pre-shifted high
       MOVB R2,@D2           => >41FF
       LI   R3,>0041         low again ...
       SWPB R3               ... flip it: R3 => >4100
       MOVB R3,@D3           => >41FF
       LI   R4,>41AA
       LI   R5,>BBCC
       MOVB R4,R5            R5 high <- >41, low stays >CC => >41CC
       LI   R6,>1234
       MOV  R6,@ODDCEL       ODDCEL is >8351 — the aligned word is >8350
```

And the bench's verdict:

```text
>8340  00 FF 41 FF 41 FF     D1=>00FF  D2=>41FF  D3=>41FF
>8350  12 34                 the "odd" word MOV landed here, at >8350
   ... R4 =>41AA   R5 =>41CC
```

Every prediction holds. `MOVB @D1` shipped R1's high half — `>00` — and left D1's low byte `>FF`
untouched: proof both that the high half is what travels and that a byte write is surgical about
the other half. The pre-shifted `>4100` and the `SWPB` repair both land `>41`, two roads to the
same fix. `MOVB R4,R5` overwrote only R5's high byte, leaving `>CC` in place — the high-byte law
applied on the destination side. And the "odd" store proves the alignment rule from §4: `MOV R6`
aimed at `>8351` deposited `>1234` at `>8350`, no fault, no warning — the CPU simply ignored the low
address bit.

Which movers *report* on their cargo, and which stay silent? `code/ch07/flagpr.a99` steps each one
past a distinctive status word so a non-change is as visible as a change:

| Instruction | L> A> EQ set from result? | Parity (OP)? | Notes |
|---|---|---|---|
| `MOV` | **yes** | — | a move is also a measurement |
| `MOVB` | **yes** (from the byte) | **yes** | the only mover that touches OP |
| `LI` | **yes** | — | `LI Rn,0` sets EQ |
| `CLR` | no | no | leaves ST exactly as it found it |
| `SETO` | no | no | likewise — a surprise worth keeping |
| `SWPB` | no | no | the wrench sets nothing |
| `STWP` | no | no | introspection is flag-silent |
| `STST` | no | no | it reads ST; it must not disturb it |

The surprise is the bottom half of the table: `CLR` and `SETO` write a register but touch no flag,
so `CLR R4` will not tell you the result was zero — you knew that already, which is exactly why the
9900's designers spent no silicon reporting it. `MOV R1,R1` is the flip side, a move that changes
nothing but *reports* on R1 — a free test we will lean on constantly in Ch. 8.

## 7.3 Setting Up the Machine: LI, LWPI, STWP, STST, CLR, SETO, SWPB

The five doors all name places; constants have to come from somewhere else, and that somewhere is
the instruction stream itself. `LI Rn,value` carries a full sixteen-bit word in the word after the
opcode and drops it in the register — and it is word-sized only; every immediate instruction on the
9900 (`LI`, `AI`, `ANDI`, `ORI`, `CI`) takes a full word, and there is no byte-sized load among
them. That absence is why the vignette's trap exists at all: character constants arrive as words whether you like it or not, and
either you pre-shift them into the high half (`>4100`) or you budget a `SWPB`. `LWPI` is `LI`'s
sibling for the one register more important than registers: it aims WP itself, which is why the
programs in this book open by pointing it into the fast island (Ch. 5 is the reason). The third
immediate sibling, `LIMI`, loads the interrupt mask; it waits for Ch. 22, where interrupts stop
being something that merely happens to you.

`STWP` and `STST` run the other direction — they are the introspection pair, storing WP and ST into
a register of your choice. `STWP` is how running code learns *where its own registers physically
are*, which turns register-relative thinking into absolute addresses you can point a walking
pointer at; `STST` is how it photographs the machine's mood, flags and mask and all. They look like
curiosities this week. They are load-bearing the moment programs start examining themselves —
debuggers and monitors lean on them, and Ch. 9's context machinery makes WP-awareness urgent.

Then three small powers that repay their keep daily. `CLR` writes `>0000`; `SETO` — *set to ones* —
writes `>FFFF`, which Ch. 8 will teach you to read as −1. Each packs its constant into the opcode
itself: no immediate word, one instruction word total, and — better — each accepts *any* of the
five doors, because they are single-operand general instructions. `CLR *R2+` zeroes a word and
walks forward in a single breath, which is the seed of the zero-fill idiom the Lab will harvest.
And `SWPB` exchanges the two halves of a word — the high-byte law's official wrench, `MOVB`'s
constant companion, the one-instruction answer to "my byte is in the wrong half." Which of this
section's cast report to the flags and which stay silent is answered by §7.2's table — some of the
answers genuinely surprise, so treat that table as required reading, not reference.

The whole cast in one run — `code/ch07/setup.a99` — with the register file dumped at the end:

```asm
START  LWPI WS               aim WP at the fast island
       STWP R0               R0 <- where our registers live (>8300)
       STST R1               R1 <- a photograph of ST
       LI   R2,>1234         a plain immediate load
       CLR  R3               R3 <- >0000, packed in the opcode
       SETO R4               R4 <- >FFFF (Ch. 8 reads it as -1)
       LI   R5,>0041         'A' in the low half ...
       SWPB R5               ... flipped to the high half => >4100
```

```text
R0 =>8300   R1 =>0000   R2 =>1234   R3 =>0000
R4 =>FFFF   R5 =>4100   ...
```

`STWP` handed R0 the value `LWPI` had just installed — `>8300`, the workspace's own address, now a
number the code can do arithmetic on. `CLR` and `SETO` produced `>0000` and `>FFFF` from a single
instruction word apiece, no immediate to fetch; `SWPB` moved the `>41` where a later `MOVB` will
want it. And `STST`'s photograph came back `>0000` — no flags standing this early — which is only
meaningful because, per §7.2's table, `STWP` and `STST` do not disturb the mood they report on.

## 7.4 The Pump: Autoincrement and the memcpy Family

Strip any copy loop to its skeleton and three duties remain: move a datum, advance the pointers,
decide whether to go again. On most processors those are three separate stretches of code, and the
middle one — pointer arithmetic — is pure overhead, instructions that exist only to aim other
instructions. The 9900's third door folds that duty *into the operand itself*: `*R3+` fetches
through R3 and then steps it past what it just touched, by exactly the operand's size — one byte
under `MOVB`, two under `MOV`. It is the only mode that knows how big the cargo is. That folding is
why `MOV *R1+,*R2+` deserves the name this book will use for it from here on: **the pump**. One
instruction, and both pointers have already advanced; all that remains is the countdown, for which
we borrow `DEC` and `JNE` from Ch. 8 on their obvious readings — decrement, jump if not zero — with
their formal treatment deferred.

The pump has a small family, and the four members are the Lab's four routines. *Copy* is the pump
in a countdown. *Fill* is the pump with the source nailed down: `MOV R3,*R2+` stamps one word
everywhere — and for the special case of zero, `CLR *R2+` needs no source register at all. *Scan*
walks one pointer and interrogates instead of storing: `CB *R1+,R2` compares each byte against the
target riding R2's high half (the law again, now working *for* you), which is `strlen` and "find
the delimiter" and half of parsing, all in one shape. *Compare* walks two pointers and is your
`memcmp` — and in §7.7 it becomes something better: the referee that tests everything else. A fifth
shape waits offstage: nail the *destination* down instead — `MOVB *R1+,@PORT` — and the pump
becomes a feeder for memory-mapped hardware, which from Ch. 12 onward is its entire livelihood.
Notice, meanwhile, the uncomfortable ratio in the basic loop: of its three instructions, one moves
data and two administrate. That administrative tax is real, §7.6 prices it, and the Lab experiments
with the classic dodge — unrolling, paying the countdown once per several pumps.

The fine print is exactly where this book refuses to hand-wave, because autoincrement has corners.
What happens when the same register drives both sides — `MOV *R1+,*R1+`? In what order do the
reads, writes, and increments land? Does the byte/word step difference behave at the boundaries you
would guess? These get pinned to the bench in the edge-case notes below rather than asserted from
folklore, and exercise 7.8 sends you to check them yourself. One corner we can already name from
principle: an ascending copy whose destination overlaps its source will eat its own tail.
Characterizing exactly when — and building the descending escape — is exercise 7.10, and the full
`memmove` treatment joins the library when `lib99` formalizes in Ch. 11.

Here are the three forms, from `code/ch07/loops.a99`. *Copy* is the pump in a countdown — and this
exact loop is the one `memlib`'s `MEMCPY` adopts unchanged:

```asm
CPY    MOV  *R1+,*R2+        one word, both pointers step
       DEC  R0
       JNE  CPY
```

*Fill* nails the source down and lets only the destination walk; for the zero special case, `CLR`
needs no source register at all:

```asm
FIL    MOV  R1,*R2+          stamp the word in R1 everywhere
       DEC  R0
       JNE  FIL
*                            ... or, to zero a block: CLR *R2+ / DEC R0 / JNE
```

*Scan* walks one pointer and interrogates instead of storing — `strlen` and "find the delimiter"
in one shape, the target riding R2's high half so the high-byte law works *for* you:

```asm
SCN    CB   *R1+,R2          each byte against the target in R2's high half
       JEQ  SCNHIT
       DEC  R0
       JNE  SCN
```

On the bench all three behave: copy leaves `DST` byte-identical to `SRC`; fill stamps `>7777` into
every word including the last; scan for `'!'` in `"HI!"` returns the address of the third byte and,
asked for a byte that isn't there, returns zero. Look hard at the copy loop, because it is the whole
chapter in three instructions: one moves data and two administrate — the `DEC` and the `JNE` are
pure overhead, the tax you pay for the privilege of a countdown. That two-thirds administrative
ratio is what §7.6 prices and what the Lab's unrolling experiment attacks; the pump is efficient per
beat and wasteful per loop, and squaring that is most of what optimization means on this machine.

The fine print, demonstrated in `code/ch07/edges.a99`. When one register drives both operands —
`MOV *R1+,*R1+` with R1 at `>8340` — the order is observable and the bench pins it:

```text
   before:  >8340=AAAA  >8342=BBBB          R1 = >8340
   after:   >8340=AAAA  >8342=AAAA          R1 = >8344
```

The source operand was resolved and read *completely* — R1 fetched `>8340`'s `AAAA` and stepped to
`>8342` — *before* the destination was even looked at; the store then wrote through the **new** R1,
landing `AAAA` at `>8342`, and stepped R1 again to `>8344`. Source first, entirely, including its
autoincrement: that is the 9900's rule, and it matches Classic99's post-increment notes. The same
file shows the step size obeying the operand, not habit: after `MOVB *R2+`, R2 advanced by one;
after `MOV *R3+`, R3 advanced by two. A byte loop that borrows a word instruction's `*Rn+` skips
half its data — Pitfall (6), now something you have watched happen.

> **Field Notes — A Data-Mover from 1983.** The period's magazines ran on type-in listings, and
> somewhere in nearly every issue that touched assembly sat a loop shaped like §7.4's — because
> everyone who published machine code for this machine had solved this chapter's problem in public.
> We will be honest about provenance: no single 1983 listing was in reach to verify against its own
> printed source, so what follows is not a citation but the *archetype* — the loop every screen-
> writing program on this machine actually ran, reduced to its bones (`code/ch07/fldnote.a99`, which
> assembles clean):
>
> ```asm
> FEED   MOVB *R1+,@VDPWD      push one byte to the VDP, step the pointer
>        DEC  R0
>        JNE  FEED
> ```
>
> Read it with this chapter's eyes. R1 walks a message in ROM via autoincrement — the source door
> that steps itself. The destination is not memory at all but `>8C00`, the VDP's write-data port
> (Ch. 5): a nailed-down destination, the pump's fifth shape, feeding hardware instead of a buffer.
> The high-byte law is doing quiet work — each byte ships from R1's *high* half, which is why the
> message bytes must arrive there, and they do, straight from ROM. And the cost, by §7.6's method,
> is where a modern reader flinches: every trip through `FEED` writes one byte *across the funnel* to
> a port, so the per-byte bill runs to dozens of cycles, and a screenful is thousands of them. The
> one choice this book would question is exactly that — one byte per crossing. The period's own
> answer was a block-move utility (VMBW, §6.7) that amortized the setup; ours will be `vdplib`
> (Ch. 12), and the deeper fix, unrolling and staging, waits for Ch. 37. The loop is not wrong. It
> is just paying retail, one byte at a time, for something the machine will sell you wholesale.

## 7.5 Arrays and Records Without a Compiler

No compiler is coming to lay out your data structures: you are the layout, the accessor, and the
discipline, and the fifth door is your whole toolkit. The pleasant surprise is that `@PLACE(Rn)` is
one mode wearing two costumes. In the *array* costume, the label is the base and the register is
the moving part: `@TABLE(R4)` walks a table as R4 sweeps the element offsets. In the *record*
costume the roles swap — the register holds the record's base address and the constant is the
moving part frozen at assembly time: with `STAT` equated to a field's offset, `@STAT(R9)` reads
that field of whichever record R9 points at. Same encoding, same cost, two mental models; fluency
is switching costumes without noticing.

The word-versus-byte decision deserves to be made with open eyes, because each choice taxes a
different resource. Word tables are computation-friendly — but element *n* lives at offset 2*n*, so
every index must be doubled on its way into the register (adding a register to itself does it — one
borrowed Ch. 8 instruction), and forgetting the doubling is the classic skew bug: you read a "word"
that is the low half of one entry glued to the high half of the next, plausible-looking garbage of
the worst kind. Byte tables need no scaling and cost half the RAM — 24 K of high expansion fills
faster than you think at two bytes an entry — but every payload arrives by `MOVB` under the
high-byte law's discipline, with a `SWPB` tax wherever you needed it low. The working rule this
book will apply from here to Part VIII: **words for what you compute with, bytes for what you store
and ship** — coordinates and counters in words; text, tiles, and maps in bytes.

Records are the same ideas compounded: pick the field offsets once, name them with `EQU`, and never
write a bare number again — the assembler is the only one of you two who never mistypes a 4 as a 6.
Keep record sizes addable — 2, 4, 6, 8 — so that stepping from one record to the next stays cheap
until Ch. 8 hands you shifts and `MPY` for fancier strides. This is not toy discipline: the entity
tables that drive Ch. 16's sprites are exactly the pattern below, grown up.

Here is both costumes in one file, `code/ch07/record.a99` — a 4-entry table of 6-byte game-entity
records, then two array lookups:

```asm
XOFF   EQU  0                record field offsets, named once
YOFF   EQU  2
HPOFF  EQU  4
FLOFF  EQU  5
RECSIZ EQU  6
ENT3   EQU  3*RECSIZ         entry-3 offset, PRECOMPUTED (see below)
TABLE  DATA >0064,>0032      entry 0: X=100  Y=50
       BYTE >0A,>01                  HP=10  FLAGS=1
*      ... entries 1, 2 ...
       DATA >0190,>012C      entry 3: X=400  Y=300
       BYTE >28,>02
WARR   DATA >1111,>2222,>3333,>4444
BARR   BYTE >AA,>BB,>CC,>DD

* record costume — register base, EQU'd constant offset:
       LI   R9,TABLE+ENT3       R9 -> entry 3's base
       MOV  @XOFF(R9),R1        R1 = entry3.X   => >0190
       MOVB @HPOFF(R9),R2       R2 high = entry3.HP => >28
* array costume — label base, register index:
       LI   R4,2*2              WORD table, element 2: index DOUBLED
       MOV  @WARR(R4),R3        R3 = WARR[2]    => >3333
       LI   R5,2                BYTE table, element 2: no scaling
       MOVB @BARR(R5),R6        R6 high = BARR[2] => >CC
```

The bench returns exactly `R1=>0190`, `R2=>2800`, `R3=>3333`, `R6=>CC00`, and the two costumes are
visible in the operands: in the record accessors the *register* (R9) is frozen at a base and the
*constant* (the field offset) does the choosing; in the array lookups the *label* (WARR, BARR) is
frozen and the *register* (R4, R5) does the walking. Watch the word table's index: element 2 lives
at offset 4, so R4 gets `2*2` — forget that doubling and you read half of `>2222` glued to half of
`>3333`, the classic skew bug. One more trap this file caught on the bench, and it is a `[libre99asm]`
fine point worth its own scar: **libre99asm has no operator precedence — it evaluates left to right.**
`TABLE+3*RECSIZ` would parse as `(TABLE+3)*RECSIZ`, a wild address, so entry 3's offset is
precomputed into `ENT3 EQU 3*RECSIZ` (a single multiply) and added on its own. This is the entity
table that drives Ch. 16's sprites, met early and in miniature.

> **Pitfalls.** (1) `LI Rn,65` then `MOVB Rn,...` ships `>00` — `LI` fills the word, `MOVB` reads
> the high half (§7.2); pre-shift the constant or `SWPB` first. (2) Word instructions at odd
> addresses do not trap: A15 is ignored, the aligned word is used, and the bug detonates somewhere
> far away. (3) Word-table lookups need the index doubled; forget the ×2 and you read half of one
> entry glued to half of the next. (4) `MEMCPY` counts words, `MEMCPB` counts bytes — keep the unit
> in the routine's name, as `memlib` does, or an off-by-×2 will follow you around. (5) There is no
> `@TBL(R0)` (§7.1). (6) `*Rn+` steps by the operand's size — 1 under `MOVB`, 2 under `MOV` — never
> "always 1"; a byte loop that borrows a word instruction skips half its data.

## 7.6 First Cycle-Counting: What a Move Costs

Ch. 4 proposed this book's working model and Ch. 5 measured it into the ledger: **T = C + 4 ×
(accesses in the 8-bit domain)** — an instruction's intrinsic cycle count, plus four for every
memory access that has to squeeze through the funnel. This section is where the model stops being a
slogan and becomes a reading skill. Warm up on the ledger's own numbers. `MOV R1,R2` with
everything in the fast domain: 14 cycles — that is pure C, zero surcharges. Move only the *code*
out to expansion RAM, workspace still in the pad: 18 — one surcharge, the instruction's own fetch.
Move everything out: 30 on real hardware — four slow accesses at +4 each: the fetch, the source
read, the destination's read-before-write, and the write. That read-before-write is genuine 9900
behavior, and it is also this book's one open honesty file: at this writing the project's core
skips it, so an all-expansion `MOV R,R` benches 26 until the fix lands — the ledger carries the
deviation, Ch. 5 tells the story, and every bench figure in this chapter that writes a slow
destination through `MOV` inherits a one-line caveat. Prose states the hardware truth. `JMP $`
taught the same lesson in miniature: 10 in the pad, 14 in expansion — one instruction, one fetch,
one toll.

General operands bend both terms at once: each of §7.1's doors adds its own cycles to C *and* its
own accesses to the count — which is why the datasheet prices an instruction as a base figure plus
address-modification surcharges, and why §7.1's cost table is really a timing table wearing a
different heading. The worked read below does the full arithmetic for the pump —
`MOV *R1+,*R2+`, then the whole three-instruction loop with `DEC` and `JNE` included — region by
region, prediction first, bench second, discrepancies confessed.

Here is why the exercise matters more than any single number. A frame is 50,000 cycles (Ch. 5's
ledger) — that is the wallet. Divide any per-word figure into it and you get the number a working
programmer actually feels: *words per frame*. The Lab measures the pump in the pad and in expansion
and turns both into that headline pair, and those two figures quietly govern everything Part IV
attempts, because moving screens' worth of bytes on a frame budget is most of what a video game
*is*. The pump earns its name here: it is the heart this machine's software beats with.

Price the pump first from the datasheet, then check the bill on the bench. `MOV *R1+,*R2+` is a
base `MOV` (C = 14) plus two autoincrement operands at +8 clocks each — an intrinsic **30 cycles**
in zero-wait memory — and it touches memory four times that a stopwatch can tax: the instruction
fetch, the source read, the destination read-before-write, and the destination write. `T = C + 4 ×
(slow accesses)` turns those four touches into the region table below, measured on BENCH99 with the
loop staged in the pad (the §5.5 trick, so code and workspace both sit on the island) and only the
copy buffers moved from region to region:

| Placement | `MOV *R1+,*R2+` | + `DEC` + `JNE` | Per word | Words / frame |
|---|---|---|---|---|
| all on the island | 30 | +20 | **50** | **≈ 1000** |
| code + WS island, buffers expansion | 42 † | +20 | **62** | **≈ 806** |
| all across the funnel | 46 † | +28 | **74** | **≈ 675** |

† These two carry the chapter's one honest asterisk. The 9900 reads a `MOV`'s destination before
overwriting it; our core skips that read (the ledger's open deviation, Ch. 5), so it benches the
buffers-expansion pump at 38 and the all-funnel pump at 42, each 4 light. The cross-probe settles
the hardware truth: `A *R1+,*R2+`, whose destination read is legitimate arithmetic the core *does*
model, measures exactly 4 more than `MOV` in the same spot — so the honest numbers are 42 and 46,
which is what the table prints.

Two readings close the section. First, the administrative tax is not a rounding error: of the
pump loop's 50 island cycles, 20 — two of every five — are the `DEC` and the `JNE`, moving no data
at all, which is precisely the fat the Lab's unrolling trims. Second, the headline. A frame is
50,000 cycles (Ch. 5); divide, and the pump moves roughly **1000 words per frame on the island,
about 806 with the data across the funnel, 675 with everything out there**. Those numbers are the
wallet every Part IV animation budget is drawn against, and you now know them cold — not from
folklore, but from a loop you can re-run.

## 7.7 Proof by Color: Self-Testing Before I/O

We still own no way to print, and that is a decision, not an oversight: text on this machine
belongs to the VDP, the VDP arrives in Part IV (Ch. 12–13), and a memory library "tested" by
eyeballing characters would be resting its weight on machinery far shakier than itself. Good tests
report through a channel *simpler* than the thing under test. The simplest output device on a
99/4A is the screen border: one VDP register paints it edge to edge, and setting that register
takes a two-write recipe aimed at `>8C02` that we borrow on faith — a lamp, not a lesson; Ch. 12
owns the anatomy. Green border, tests passed; the failure color, something broke. A one-word
verdict, visible from across a dorm room.

The scaffold pattern, which this book will reuse until you are sick of it and then grateful for it:
arrange known input; run the routine under test; `MEMCMP` the result against a hand-built expected
block; aim the border at the verdict. You should immediately object that we are testing `memlib`
with `memlib` — the referee plays for one of the teams. Two answers. First, `MEMCMP` is the
simplest routine in the library, a compare and a countdown, and it is the *only* one trusted with
refereeing. Second — and this is the habit that matters — the Lab's protocol includes the sabotage
drill: deliberately corrupt one expected byte and confirm the border turns *fail*. A test that
cannot fail is not a test; a scaffold that has demonstrably failed once is an instrument. This
pattern — self-checking program, verdict on the border, sabotage drill in the checklist — grows up
to become SYSCHK in Ch. 11 and is graded, eventually, by CQ-82's standards in Part IX.

The recipe is two byte-writes to the VDP's address port, `>8C02` — the color, then the byte that
says "register 7, please" (the `>80` bit is the write-select; register 7 is the backdrop). Folded
into `memlib.a99` as a subroutine, with the two verdict colors named as book-wide convention:

```asm
GREEN  EQU  >0200            medium green (palette 2), in the high half
RED    EQU  >0600            dark red     (palette 6), in the high half
* BORDER — R0's high half is the color; aim VDP register 7 (backdrop).
BORDER MOVB R0,@>8C02        the color byte ...
       LI   R0,>8700         ... register 7, write bit set
       MOVB R0,@>8C02
       RT
```

On the bench, `vdp` confirms the register took the value — green (`R7=>02`) when the tests pass,
red (`R7=>06`) when one fails. The palette names come from the TMS9918A's own table (App. D owns
the full sixteen); the two writes' anatomy — why a register load is a color byte chased by a
register selector — is a lamp we light on faith here and wire up properly in Ch. 12. From here on,
`GREEN` means pass and `RED` means fail across the whole book.

## Lab 7 — memlib: Copy, Fill, Compare, Scan — Measured

This lab turns the chapter into property. You will write the four primitives every later chapter
leans on, prove them against a scaffold that can visibly fail, and then weigh them on Ch. 5's scale
so that every future "how fast can we move it?" question starts from a ledgered number instead of a
guess.

The contracts come first, because the code must be written *to* something. Linkage is `BL @NAME`
in, `RT` out (`RT` is `B *R11` — Ch. 4's ledger even knows its opcode, `>045B`). `BL` parks its
return address in R11, which makes these *leaf* routines: they must not call one another until
Ch. 9 builds the stack discipline that makes nesting safe. Arguments ride R0–R2 and may be
clobbered. Byte-sized cargo rides the high half of its register — the law, promoted to calling
convention.

| Routine | Job | In | Out |
|---|---|---|---|
| `MEMCPY` | copy R0 **words**, R1 → R2 | R0 count, R1 src, R2 dst | — |
| `MEMCPB` | copy R0 **bytes**, R1 → R2 (any count, any parity) | R0 count, R1 src, R2 dst | — |
| `MEMFIL` | stamp the word in R1 into R0 words at R2 | R0 count, R1 word, R2 dst | — |
| `MEMCMP` | compare R0 words at R1 vs R2 | R0 count, R1 a, R2 b | R0 = 0 if equal, else address of first mismatch (R1 side) |
| `MEMSCN` | find a byte in R0 bytes from R1 | target byte in R2's **high half** | R1 = match address, or 0 if absent |

The session runs in three movements. First, *build*: the library and its test harness, `memtest`,
which exercises every contract row against known vectors and renders the verdict on the border
(§7.7). Second, *distrust*: the sabotage drill — corrupt one expected byte, watch the border fail,
restore it, watch it pass; only then does the harness count as an instrument. Third, *measure*: the
placement matrix — code and workspace in the pad throughout, buffers in the pad versus in
expansion; byte copy versus word copy over the same bytes; and one taste of unrolling, the pump
doubled per countdown — all under the Ch. 5 rig's discipline, with the `MOV` destination-pre-read
caveat (§7.6) stated wherever it applies. What you keep is threefold: routines that seed `lib99`
(Ch. 11), numbers that go to the ledger, and the habit — contract, test, sabotage, measure — that
Part IX's CQ-82 will eventually grade as if it had been watching all along. It had.

**Build.** `code/ch07/memlib.a99` holds all five routines and, below them, the `START` harness that
proves them. Each routine is the matching §7.4 loop wearing a `BL`/`RT` collar. `MEMCPY` is the pump
in a countdown, verbatim; `MEMSCN` is the scan, its target riding R2's high half; `MEMCMP` — the one
trusted to referee — is the compare with a small tail that backs the pointer up to name the guilty
word:

```asm
MEMCPY MOV  R0,R0            zero words? nothing to do
       JEQ  MCPYX
MCPYL  MOV  *R1+,*R2+        the pump
       DEC  R0
       JNE  MCPYL
MCPYX  RT
* ...
MEMCMP MOV  R0,R0
       JEQ  MCMPEQ
MCMPL  C    *R1+,*R2+        compare a word, both pointers step
       JNE  MCMPNE
       DEC  R0
       JNE  MCMPL
MCMPEQ CLR  R0               all equal -> 0
       RT
MCMPNE DECT R1               R1 stepped past; back up to the mismatch
       MOV  R1,R0            report its address
       RT
```

The harness runs four checks — a `MEMCPY` round-trip, a `MEMFIL` verified against an expected block,
a `MEMSCN` hit at a known offset, and a `MEMSCN` miss — and any failure branches to `FAIL`. Reach
the end and it paints the border green.

**Distrust.** A test that cannot fail is decoration. So the sabotage drill: run the harness once
clean, then have the bench corrupt one word of the copy buffer just before `MEMCMP` inspects it, and
run on. The two BENCH99 runs, read off the `vdp` register dump, are the whole point:

```text
clean run:     PC -> PSPIN   VDP R7=>02   (green: all four checks passed)
sabotaged:     PC -> FSPIN   VDP R7=>06   (red:  MEMCMP caught the bad word)
```

Only now is the scaffold an instrument: it has demonstrably failed on purpose, so its green means
something. This pattern — self-checking program, verdict on the border, sabotage drill in the
checklist — is exactly what grows into SYSCHK (Ch. 11) and gets graded by CQ-82 in Part IX.

**Measure.** With correctness banked, the routines go on Ch. 5's scale — code and workspace staged on
the island, buffers moved region to region, the per-instruction costs read straight off `s` traces:

| Routine | Inner op | Island, /unit | Buffers in expansion, /unit |
|---|---|---|---|
| `MEMCPY` (word) | `MOV *R1+,*R2+` | **50** | **62** † |
| `MEMFIL` (word) | `MOV R1,*R2+` | **42** | **50** † |
| `MEMCPB` (byte) | `MOVB *R1+,*R2+` | **46** | **58** † |
| `MEMSCN` (byte) | `CB *R1+,R2` | **40** | **44** |
| `MEMCPY` unrolled ×2 | two pumps / countdown | **40** | **52** † |

† Writes a `MOV`/`MOVB` destination, so the expansion figure is the hardware value; our core benches
it 4/unit lighter until the dest-pre-read fix lands (§7.6). `MEMSCN` only reads, so it carries no
caveat.

Read three things off this table. First, `MEMSCN` is the cheapest and the only one immune to the
deviation, because a compare writes nothing — reading really is cheaper than moving here. Second,
unrolling `MEMCPY` ×2 drops the per-word cost from 50 to 40 — a clean 20% — by making one `DEC`/`JNE`
serve two words instead of one; that is the administrative tax of §7.4 being refunded, and Exercise
7.9 chases where the refund stops paying. Third, reconcile with §7.6's headline: `MEMCPY` at 50
cycles a word is **1000 words per frame** on the island, 806 with the data across the funnel — the
same numbers, now earned by a routine you wrote, tested, tried to break, and clocked. Every one of
them is a restatement of the one law: speed is a property of addresses, not instructions.

## Exercises

7.1 ✦ For each of the five modes, write one instruction that uses it as the *destination*, and say
in one sentence what your instruction does.

7.2 ✦ After `LI R4,>1234`, what byte does `MOVB R4,@FLAG` store at `FLAG`, and why — answered in
terms of WP + 2n and even addresses, in one sentence?

7.3 ✦ `LI R5,>FF00` followed by `MOVB R5,@X` stores `>FF`, but `LI R5,>00FF` stores `>00`. Explain
each in a sentence, then give two different one-instruction repairs for the second case.

7.4 ✦✦ `MEMFIL` fills words. Write `BYTFIL`, a byte-at-a-time fill that accepts any count, odd or
even, and prove it with the §7.7 scaffold on an odd-length buffer. (✦✦✦ extension: do the body in
words and the tail in bytes — you will need one shift from Ch. 8, borrowed on faith — and measure
what the fuss bought using the rig.)

7.5 ✦✦ Using the §7.6 table, predict the per-word cost of the copy loop with buffers in the pad
versus in expansion RAM; then measure both with the Ch. 5 rig and reconcile — including the
deviation caveat if it still applies at your HEAD.

7.6 ✦✦ Rewrite `MEMCPY`'s inner move in indexed mode — `@SRC(R3),@DST(R3)` with R3 doing the
walking — plus whatever bookkeeping that entails. Predict the cost delta from the §7.1 and §7.6
tables, then measure it.

7.7 ✦✦ Design a 6-byte record for a game entity — X word, Y word, HP byte, FLAGS byte — write the
`EQU`'d offsets and accessors in the record costume, and initialize a 4-entry table with
`DATA`/`BYTE`. Which fields did you make words, and why?

7.8 ✦✦✦ Predict what `MOV *R1+,*R1+` does — every read, every write, every increment, and R1's
final value — then check yourself on BENCH99 against §7.4's edge-case notes.

7.9 ✦✦✦ Unroll `MEMCPY`'s body ×2 and ×4; measure each with the rig. Where exactly does the savings
come from, and where does the knee of diminishing returns show up in your numbers?

7.10 ✦✦✦ An ascending copy corrupts itself when source and destination overlap in one particular
way. Characterize exactly when, write `MEMMOV` — choosing the descending path at runtime takes a
compare from Ch. 8, borrowed on faith — and prove both paths with the scaffold.

7.11 ✦✦ Place a 200-byte string in the pad and again in expansion; predict the `MEMSCN` cost ratio
from the model, then measure it. Explain why the ratio is smaller than the naive "everything +4"
slogan suggests.

## Further Reading

- The *Editor/Assembler* manual — the addressing-mode and instruction-description sections. The
  period bible for this chapter; its examples are the ones a 1983 reader actually typed.
- The TMS9900 data manual (the datasheet) — instruction formats and the execution-time tables §7.6
  taught you to read; App. A rebuilds them for this book with measured cross-checks.
- *TI Intern* (Heiner Martin) — the console ROM disassembled; watch TI's own engineers move bytes
  for a living, and the Field Notes' fallback source.
- Classic99's CPU core source — a modern, hardware-cross-checked second opinion on instruction and
  flag behavior when a datasheet sentence turns ambiguous.
- This project's `libre99-core` CPU source and the BENCH99 README — the exact instruments behind this
  chapter's numbers; the README documents every bench command the Lab used.

## Summary

- Five general addressing modes serve both operands: `Rn`, `*Rn`, `*Rn+` (steps by operand size —
  the loop mode), `@LABEL`, `@LABEL(Rn)`; R0 cannot index; immediates are a separate family —
  `LI`/`LWPI` load, `STWP`/`STST` introspect, `CLR`/`SETO` are the free constants, `SWPB` the byte
  wrench. Mode encoding/cost table established here feeds App. A.
- The high-byte law is big-endianness seen through Rn = WP + 2n: a register's byte is the byte at
  its own even address — the high half. Byte cargo rides the high half book-wide (now a calling
  convention); `SWPB` repairs; word ops silently align (A15 ignored); `MOVB` is the instruction
  that talks to the ports, so the law governs all future device I/O.
- `MOV`/`MOVB` also test their cargo (flag table §7.2): a move is a measurement; `MOV R1,R1`
  foreshadowed for Ch. 8.
- The pump is `MOV *R1+,*R2+`; the family is copy/fill/compare/scan (plus the nailed-destination
  port feeder, promised to Ch. 12); in the basic loop one instruction moves and two administrate,
  which unrolling amortizes. Same-register autoincrement ordering pinned on the bench (§7.4 edge
  notes).
- `@X(Rn)` wears two costumes — array (label base, register index, ×2 for word tables) and record
  (`EQU` offset, register base). Words for computing, bytes for storing and shipping.
- Timing: T = C + 4 × (8-bit accesses) applied for real; `MOV R,R` = 14/18/30 (hardware truth; the
  core's dest-pre-read deviation caveated per the ledger). Addressing-mode add-ons measured
  (`*Rn` +4, `*Rn+` +8, `@` +8 plus the fetched address word). The pump `MOV *R1+,*R2+` = 30 island
  / 42 buffers-expansion (hw) / 46 all-expansion; the copy loop is 50 / 62 / 74 cycles per word →
  **≈ 1000 / 806 / 675 words per frame**; unrolling ×2 cuts the island copy to 40/word (1250 wpf).
- `memlib` born — `MEMCPY`/`MEMCPB`/`MEMFIL`/`MEMCMP`/`MEMSCN`, `BL`/`RT` leaf linkage, args
  R0–R2 — tested by border-color verdict with the sabotage drill (pattern seeds SYSCHK, Ch. 11),
  measured pad-versus-expansion with Ch. 5's rig; seeds `lib99` (Ch. 11).
- Border verdict recipe (two writes to `>8C02`) borrowed on faith; anatomy owed to Ch. 12; the two
  verdict colors become a book-wide convention.
