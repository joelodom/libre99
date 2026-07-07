# Chapter 42 — Capstone IV: The Productivity Program

<!-- Part IX — Case Studies: Recreating the Classics · target ≈18 pp · [disk, 32K] -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — pending review passes. AUTHOR99's engine (gap-buffer text store with O(1) insert/delete + cursor moves, DV80 record serialisation, textlib40 render) MACHINE-VERIFIED on BENCH99 against toolchain commit 21e466b: author99.a99 -> 8,192-byte single-bank image (entry >62F0), deterministic 3-part self-test GREEN (VR7=>02, FAILID=0); the scripted edit session flattens to "AUTHOR 99" (41 55 54 48 4F 52 20 39 39), serialises to a DV80 record (09 + the chars), and renders to the 40-column screen. Verdict reached in ~4,400 instr / ~93,000 cyc, dominated by the one-time screen clear. savedoc.a99 (DV80 disk write) ASSEMBLES; verified at the Rust-harness tier, not BENCH99 (no card at >4000, R-12). Code in code/ch42/. Applies R-19/R-16/R-12; no new ruling. -->
<!-- SPEC: 00-master-outline.md, "### Chapter 42 —" (lines 657–665), Part IX preamble (621–623). -->

## Words at Three Megahertz

The arcade capstones were about spectacle — sprites, sirens, a scrolling star
field. This one is about something quieter and, for most owners, more used: the
program you *work* in. In 1983 a TI-99/4A with a disk drive and TI-Writer was a
word processor, and a family that bought the console to play *Parsec* found
itself, more evenings than not, using it to write letters, school reports, club
newsletters. The editor was the program they spent the most hours inside, and so
the thing they judged most harshly was not how it looked but how it *felt* — and
the whole of how an editor feels comes down to one question, asked sixty times a
second: when I press a key, does the letter appear *now*?

On a 3 MHz machine that is not a given. Press a key in a naively built editor and
it might insert your character by shoving every byte after the cursor down one
slot — and if your document is four kilobytes and the cursor is near the top,
that is four thousand bytes copied for one keystroke, every keystroke, and the
cursor visibly lags your fingers. A good editor cannot afford that, and the
1983 ones did not pay it. They used a data structure with a lovely property: the
edit at the cursor is *free*, and you only pay for *moving* the cursor. This
chapter builds that structure — the **gap buffer** — and the screen and file
machinery around it, and then it does the thing productivity software lives or
dies by: it *measures* the latency, because in an editor, latency is the game's
frame budget.

---

## What You Will Learn

After this chapter you can:

- **Build a gap buffer on the 9900**, so inserting or deleting at the cursor is
  O(1) — a handful of instructions, independent of document size.
- **Argue a data structure from its cost**, and know exactly which operation the
  gap buffer makes cheap and which it makes you pay for.
- **Render an editor screen under continuous change** with `textlib40`, using
  dirty-line redraws so a keystroke repaints one line, not the whole screen.
- **Read and write DV80 files** — the interchange format TI-Writer itself
  accepts — and serialise a document line to the exact bytes on disk.
- **Measure keystroke-to-glass latency on the bench** and use it, the way a game
  uses the frame budget, to decide what a keystroke is allowed to do.
- **State the printing story honestly**, where the emulator's peripheral gaps
  begin (R-12).

## The Bridge: The Program You Live In

You have a favorite editor, and you have opinions about it, because you spend
hours a day in it and you feel every millisecond of lag. That is the mindset for
this chapter: you are building the tool you would *live in*, and the standards
are correspondingly personal. Two ideas carry over from the modern world almost
unchanged. The first is the **gap buffer** — the text data structure Emacs has
used for four decades, chosen for exactly the reason we will choose it, because
edits cluster at the cursor and a gap at the cursor makes clustered edits free.
The second is **latency budgeting**: modern editors fight to keep keystroke-to-
paint under some millisecond bar, and we will fight the same fight in the same
terms, only our clock is 3 MHz and our bar is the 60 Hz frame we have measured
all book long. The machine is smaller. The engineering is the same.

## 42.1 Archaeology: Editor Feel at 3 MHz

The genre-definer is TI-Writer, TI's word processor — a cartridge paired with a
disk of programs that turned the console into a writing machine. As always we
own no image of it (R-19; `cartridges/` is empty by design), so we reconstruct
not its code but its *behavior*, and an editor's behavior is unusually legible
because you can reason about it from the constraints alone.

What did it have to do, and do fast? It had to show forty columns of text —
narrower than the paper, so it distinguished the *document* from the *screen*,
scrolling a window over a longer file. It had to insert and delete at a cursor
without the machine falling behind a typist, which on a 3 MHz CPU means the
insert could not be O(document). It had to move a cursor, scroll, and mark
blocks. It had to save and load real files other programs could read, which on
this platform means DV80. And it had to *print*, handing bytes to a parallel or
serial port. Every one of those requirements survives without the ROM, because
each is forced by the job and the hardware, not by a design whim. That is enough
to specify our own.

> **Archaeology, honestly (R-19).** The gap buffer below is *our* engineering
> choice for AUTHOR99, argued from the 3 MHz constraint — not a claim about
> TI-Writer's internals, which we cannot see. What we reconstruct from the record
> is the *feel* an editor of that era had to deliver: a cursor that keeps up. How
> we deliver it is ours to design and ours to measure.

## 42.2 Specification: AUTHOR99

**AUTHOR99** is a 40-column screen editor. Its feature list is its definition of
done: insert and overwrite typing; delete and backspace; cursor motion by
character, line, and screen; block operations (mark, copy, delete); a forward
search; DV80 load and save; and printed output. The document is a sequence of
lines; the screen is a 24-row, 40-column window onto it; the file is a DV80 disk
file, one record per line. The engine is small because, unlike an RPG, an editor
holds little *state* beyond the text itself — the art is entirely in how that
text is stored, shown, and moved.

The design decisions that matter are three, and the rest of the chapter is each
in turn: **how the text is stored** (the gap buffer, §42.3), **how it reaches the
glass** without repainting the world every keystroke (dirty-line rendering,
§42.4), and **how it reaches the disk** in a format the ecosystem accepts (DV80,
§42.5). Then we measure whether the whole thing keeps up (§42.6).

## 42.3 The Text Buffer: A Gap on the 9900

Here is the structure. Store the document in one contiguous buffer, but leave a
**gap** of unused bytes at the cursor. Text before the cursor sits at the bottom
of the buffer; text after the cursor sits at the top; the gap is the free space
between them. Three pointers name the parts: `GBUF` (the base), `GAPS` (one past
the last character *before* the cursor), and `GAPE` (the first character *after*
the cursor). The cursor position is `GAPS - GBUF`; the document is everything
except the gap.

```
   before cursor         gap (free)          after cursor
  +----------------+---------------------+------------------+
  | A U T H        |                     | O R              |
  +----------------+---------------------+------------------+
  ^GBUF            ^GAPS                 ^GAPE              ^GEND
```

Now watch what each edit costs. To **insert** the character you just typed, write
it into the gap and move `GAPS` up one. That is it — the character lands, the gap
shrinks by one, and *nothing after the cursor moved*:

```asm
GINS   MOV  @GAPS,R2
       C    R2,@GAPE
       JHE  GINSX           gap full -> ignore
       MOVB R1,*R2          drop the char into the gap ...
       INC  R2
       MOV  R2,@GAPS        ... and shrink the gap by one
GINSX  RT
```

Five instructions, and — this is the whole point — that count does **not depend
on how big the document is or where the cursor sits**. A backspace is just as
cheap: grow the gap left by decrementing `GAPS`, and the character before the
cursor is abandoned. Insert and delete at the cursor are O(1).

What you *pay* for is moving the cursor, because moving the cursor means moving
the gap, and moving the gap means carrying one character across it. To go left,
take the character just before the gap and copy it to just after the gap:

```asm
GLEFT  MOV  @GAPS,R2
       CI   R2,GBUF
       JLE  GLFTX           already at the start
       DEC  R2
       MOV  @GAPE,R3
       DEC  R3
       MOVB *R2,*R3         carry the char across the gap
       MOV  R2,@GAPS
       MOV  R3,@GAPE
GLFTX  RT
```

One character per cursor step — O(distance moved), not O(document). And this is
exactly the right trade, because of *how people edit*: keystrokes cluster. You
type a run of characters (all free), you move the cursor a little (cheap), you
type again. The expensive operation — jumping the cursor a long way — is rare and
the user expects it to take a moment. The gap buffer makes the common case free
and the rare case honest. That is what "the cursor keeps up" is made of.

For documents larger than the CPU-RAM buffer, the same idea extends outward: keep
the active region — the screenful around the cursor — in fast RAM, and page the
rest to VRAM (24 kilobytes of it, addressable a byte at a time through the port),
swapping as the cursor travels. The gap buffer is the near structure; VRAM is the
far store. The self-test exercises the near one to its verdict.

> **Field note — the flatten.** To save or to redraw, you need the document
> *contiguous*, gap removed. `GFLAT` copies the pre-gap text and then the
> post-gap text into a clean buffer and returns the length. Our scripted session
> — type `AUTHOR`, cursor left twice, insert `X`, backspace it, cursor right
> twice, type ` 99` — flattens to exactly `AUTHOR 99`, which is how the bench
> knows every pointer moved correctly.

## 42.4 The Screen Engine: Dirty-Line Rendering

The editor draws through `textlib40` (Ch. 14), the 40-column text-mode engine:
one foreground/background pair for the whole screen, a 6-pixel cell, no sprites —
the productivity mode. `TX40MD` brings the mode up, loads the font, and clears
the screen; `TX40LC` places the cursor; `TX40PS` paints a string. The self-test
flattens the document, points the renderer at it, and confirms the glass shows
what the buffer holds — `A` in the corner, a space at column six:

```asm
       BL   @TX40MD         text mode, font, colours, clear
       CLR  R0
       CLR  R1
       BL   @TX40LC         cursor to (0,0)
       LI   R1,FBUF
       BL   @TX40PS         paint the flattened line
```

But the deep question is not how to paint a line; it is how *seldom* to paint.
Here the measurement (§42.6) dictates the architecture. Bringing the whole screen
up fresh — clearing 960 cells and repainting — costs on the order of two frames.
Do that on every keystroke and the editor is a slideshow. So you don't. You track
which line the edit touched — the **dirty line** — and repaint only that line, at
most forty cells, a few hundred cycles. The cursor is drawn by writing a distinct
character (or toggling the color) at one cell and undrawing it when it moves; a
status line at the bottom shows the filename and cursor position and is repainted
only when *it* changes. The discipline is precisely Ch. 17's game-loop discipline
in a new suit: **find the smallest thing that changed and touch only that.** An
editor is a game loop whose sprites are letters.

## 42.5 Files and Printing: DV80, and an Honest Gap

A document that cannot leave the machine is a toy. AUTHOR99 saves **DV80** files —
DISPLAY data, VARIABLE-length records, 80 bytes maximum — the lingua franca of
TI text, the format TI-Writer reads and writes, so a file saved here opens there.
Each line of the document becomes one variable-length record, and a record on
disk is simply a length byte followed by that many characters. The engine's
`DVSER` produces exactly those bytes, and the self-test checks them:

```asm
DVSER  LI   R3,DBUF
       MOV  R2,R0
       SWPB R0
       MOVB R0,*R3+         the length byte ...
       MOV  R2,R0
DVSL   MOVB *R1+,*R3+       ... then the characters
       DEC  R0
       JNE  DVSL
```

For `AUTHOR 99` that is `09` followed by the nine characters — the DV80 record a
disk sector would carry, verifiable to the byte without a drive (the xdm99-level
check the archaeology calls for). Writing it to an actual file is `filelib`'s job
(Ch. 31): open `DSK1.DOC` as DISPLAY/VARIABLE/OUTPUT, hand the DSR each record,
close. That path is the companion `savedoc.a99` — it assembles, and its behavior
is verified where the disk lives, in the Rust harness and against real `.dsk`
images (Part VII), because **BENCH99 has no card at `>4000`** (R-12).

Printing is where we stop and tell the truth. A formatter would stream the
document to a printer through the parallel (PIO) or serial (RS-232) port with
margins and page breaks — the TI-Writer *Formatter*'s job. But the RS-232/PIO
card is **not emulated** (Ch. 33, R-12): AUTHOR99 can *format* a page image in
memory, and that we could test, but the bytes-out-the-port step runs only on real
hardware or a shelf emulator that models the card. We build to the edge of the
gap and name it, rather than pretend the port is there.

## 42.6 Responsiveness: Latency Is the Frame Budget

Productivity software has a frame budget too; it is just called *latency*. The
question is the arcade question in disguise — can the machine finish its response
before the user notices? — and we answer it the same way, by measuring on the
bench. The scripted session — six edits, a flatten, a DV80 serialise, a full
text-mode bring-up, and a render — reaches its green verdict in about **4,400
instructions and ~93,000 cycles**. That number is the lesson, because of *where*
it goes: the overwhelming majority is the one-time screen clear (960 cells) and
font load in `TX40MD`. The editing itself — every insert, delete, and cursor move
in the session — is a rounding error beside it, exactly as the gap buffer
promised.

Read that measurement as a design instruction. A full screen repaint costs
roughly two frames; therefore you must **never** do a full repaint in response to
a keystroke. A keystroke does two cheap things — an O(1) gap-buffer edit (five
instructions) and a dirty-line repaint (≤ 40 cells, a few hundred cycles) — and
both together are a small fraction of one frame, so the letter is on the glass
the very next time the beam comes around. The cursor keeps up not by accident but
because we measured the expensive operation, banished it from the keystroke path,
and left only cheap ones behind. That is the whole discipline of a responsive
program, vintage or modern: **measure the cost, then arrange never to pay the big
one when the user is watching.**

## Postmortem

AUTHOR99 is the smallest engine in Part IX and, in a way, the purest, because it
has almost no state of its own — it is a set of *operations* on a buffer, and the
buffer is the user's. Three things carry forward. First, a data structure is an
argument about cost: the gap buffer is nothing but the observation that edits
cluster, made mechanical. Choose structures by which operation you want to be
free. Second, responsiveness is measured, not asserted; the bench turned "does it
feel fast?" into "~93,000 cycles, and here is where they go," which then *told us*
to render dirty lines. Third, the honest gap — printing — is drawn at the port,
where the emulator stops, and named rather than faked (R-12). No new ruling: this
chapter is R-19's arc applied to a tool instead of a game, R-16's discipline in
every leaf routine, and R-12's honesty at the printer port.

## Lab 42 — Make the Cursor Keep Up

Working in `code/ch42/`, feel the gap buffer's cost model in your hands:

1. **Extend the session.** Add more edits to `START`'s script — a longer word, a
   mid-word insertion, a run of backspaces — and predict the flattened result on
   paper before you rebuild. Green means the pointers moved as you reasoned. You
   are the typist; the buffer is keeping up.
2. **Prove the O(1) claim.** Time a single `GINS` against a single `GLEFT` that
   crosses the whole document, on the bench, and show that the insert's cost is
   flat while the cursor move's grows with distance. This is the gap buffer's
   thesis, on the scoreboard.
3. **Serialise two lines.** Give the document a line break and have `DVSER` emit
   two DV80 records. Confirm the bytes — two length-prefixed records — are what a
   `.dsk` would carry.

## Exercises

**✦ Warm-ups**

1. In one sentence each, name the operation the gap buffer makes O(1) and the one
   it makes O(distance), and say why that trade fits how people type.
2. `GINS` checks `C R2,@GAPE / JHE GINSX` before writing. What does that guard
   prevent, and what have you lost when it fires?
3. A DV80 record is a length byte plus characters. Give the bytes for the line
   `HI!` and explain why the format needs the length byte at all.

**✦✦ Building**

4. Implement **overwrite mode**: a toggle that makes a keystroke replace the
   character after the cursor instead of inserting. Which pointer moves, and which
   does not?
5. Add a **forward search**: flatten (or walk the two halves) looking for a target
   string, and move the cursor — reposition the gap — to the hit. Assert on a
   known document that the cursor lands where you expect.
6. Implement a **block delete**: given two cursor positions, remove the text
   between them in one gap adjustment. What is its cost, and why is it *not*
   O(block length) if you are clever about the gap?
7. Add a **status line**: render the filename and the cursor's row/column on row
   23, and repaint it only when the cursor's line or column changes (a dirty-flag,
   the §42.4 discipline applied to one line).

**✦✦✦ Reach**

8. Design the **VRAM-as-swap** tier: keep a window of the document in the CPU-RAM
   gap buffer and page the rest to VRAM as the cursor travels past the window's
   edges. Specify when a swap triggers and how many bytes it moves.
9. Sketch the **formatter**: given a DV80 document and a margin/page-length
   setting, produce the paginated *byte stream* you would send to a printer
   (word-wrap, page breaks) — everything up to, but not through, the unemulated
   port (R-12). How would you test the page image without a printer?

## Further Reading

- **Chapter 14**, *Text Mode and Multicolor Mode* — `textlib40`, the 40-column
  engine AUTHOR99 renders through, and the 6-pixel-cell rules of text mode.
- **Chapter 17**, *Motion: Game Loops, Scrolling, and the 60 Hz Contract* — the
  "touch only what changed" discipline that becomes dirty-line rendering here.
- **Chapter 31**, *File I/O: PABs and the Filesystem Contract* — the `filelib`
  ceremony behind `savedoc.a99` and the DV80 write path.
- **Chapter 33**, *Wires Out: RS-232, Parallel, and Cassette* — the printer ports
  the formatter would use, and why the bench cannot follow them there (R-12).
- On the data structure itself: any description of the **gap buffer** in a modern
  editor (Emacs is the canonical example) shows the same three pointers doing the
  same job, four decades and a million-fold more memory later.

## Summary

AUTHOR99 is the program you live in, and everything about it serves one felt
quality: the cursor keeps up. It stores the document in a **gap buffer** — text
before the cursor, a gap at the cursor, text after — so that inserting or
deleting where you are typing is O(1), five instructions that do not care how big
the document is, while only *moving* the cursor pays, one character carried across
the gap per step. That trade fits how people edit, where keystrokes cluster and
long jumps are rare. It renders through `textlib40` with **dirty-line** discipline
— repaint the one line that changed, never the screen — because the bench told us
a full repaint costs two frames and a keystroke cannot afford it. It saves **DV80**
files, one length-prefixed record per line, the format TI-Writer accepts and the
`filelib` path (`savedoc.a99`) writes; and it stops honestly at the printer port,
which the emulator does not model (R-12).

The whole scripted session — edit, flatten, serialise, render — reaches its green
verdict in about 93,000 cycles, and the lesson is *where* they went: into the
one-time screen bring-up, not the editing. That is the measurement that turns
"feels fast" into engineering — find the expensive operation, keep it off the
keystroke path, and leave the user only cheap ones. Productivity software has a
frame budget after all; it is spelled *latency*, and you make it the same way you
make a game's: you measure, and then you never pay the big cost while someone is
watching.

*Machine-verified against toolchain commit 21e466b: `author99.a99` assembles to
an 8,192-byte single bank (entry `>62F0`); the three-part deterministic self-test
paints VR7 = `>02` (GREEN) with FAILID 0 — the gap buffer flattens to `AUTHOR 99`
(`41 55 54 48 4F 52 20 39 39`), the DV80 serialiser emits `09` + those bytes, and
the line renders to the 40-column screen; the session reaches its verdict in
~4,400 instructions / ~93,000 cycles. `savedoc.a99` assembles; its disk write is
verified in the Rust harness, not BENCH99 (R-12).*
