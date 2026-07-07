# Chapter 12 — Inside the TMS9918A

*The screen is not made of memory you can touch. It is made by a second processor — one with its
own RAM, its own clock, and its own idea of what a picture is — that the CPU can only reach through
a mail slot four bytes wide. This chapter introduces that processor on its own terms, because for
the next six chapters it is the machine you are really programming.*

<!-- Part III — The Video Display Processor · target ≈20 pp -->
<!-- STATUS: DRAFTED (session 6, 2026-07-06) — pending review passes. vdplib.a99 assembles via sh verify.sh (cross-dir COPY '../ch11/equates.inc' works — R-17 across the tree) and is machine-verified on BENCH99 at commit e97e8ce: self-test green (VWTR-painted R7=>02), pattern round-trips VRAM (vram 0100 = 01 23 45 67 89 AB CD EF, independently confirmed by the new `vram` oracle), VSBW/VSBR single byte >5A at >0200, VMBR read-back to pad matches. Port timings single-stepped: MOVB reg→@VDPWA = 34 cyc; MOVB *src+→@VDPWD = 40 cyc (fast source) / 44 (slow source); VMBW inner loop ≈ 68–72 cyc/byte after a one-time ~134-cyc address setup — all inherit the ledgered MOV dest-pre-read caveat at the VDP ports. -->
<!-- SPEC: 00-master-outline.md, section "### Chapter 12 —" (lines 307–316 in outline v1.1). That bullet list is this chapter's contract. -->

## The Missing Framebuffer

The programmer who sat down at the TI in the late winter of 1983 had drawn pixels before, and he
knew exactly how it worked, because on the machine he came from it worked the only way he had ever
seen. His Apple II kept its high-resolution screen in main memory — a stretch of bytes starting at
a known address, one region of the same RAM the processor used for everything else. To light a
pixel you computed which byte and which bit, and you poked it. The screen *was* memory. Drawing was
just writing to the right addresses, and he had a shoebox of routines that did it. He had come to
the TI expecting to port them in an afternoon.

He opened the memory map — the one you built in Chapter 5 — and went looking for the screen. It was
not there. Console ROM at the bottom, a little scratchpad at `>8300`, the cartridge, the expansion
RAM, and scattered through the high end a handful of addresses the manual called *ports*. But
nowhere in all 64 kilobytes was there a block of memory that was the picture. He checked twice. He
had a machine that was, at that moment, displaying a title screen in color on the television in
front of him, and there was no screen memory anywhere he could reach. The pixels were real. He
simply could not find where they lived.

They lived, he eventually learned, inside a different chip entirely — the TMS9918A, the Video
Display Processor — in sixteen kilobytes of RAM that belonged to *it*, wired to *its* pins, that the
CPU could not address at all. To put anything on the screen he would have to hand it to the VDP one
byte at a time through those ports, following a little handshake to say where each byte should go.
That was a nuisance, and he could see how to write around it. What stopped him cold was the next
thing he learned, when he finally got a byte across and it did not light a pixel. It lit a
*character*. There was no pixel `(0,0)` to set. The screen was not a grid of pixels at all in the
mode the machine booted into; it was a grid of little tiles, and a byte in the wrong table changed
not one pixel but every tile that shared a shape. His shoebox of routines was not going to port. It
was going to be irrelevant, because it answered a question — *which byte is this pixel?* — that this
machine refused to ask.

He was annoyed for about a week. Then he understood why the chip was built this way, and the
annoyance turned into something closer to respect, because the design was not primitive — it was a
tidy answer to two hard problems he had never had to think about on a machine with slow graphics
and fast memory. This chapter is that week, compressed: what the VDP is, how you talk to it, and
the single idea — tables, not pixels — that makes the whole video system make sense. Learn it here,
once, and the five chapters after this are variations on a theme. Fight it, and nothing about TI
graphics will ever quite fit in your head.

---

## What You Will Learn

By the end of this chapter you will be able to:

- Explain what the VDP is and why the CPU cannot address its VRAM — and why a separate processor
  with private memory was the right engineering choice in 1979, not a limitation.
- Drive the four VDP ports and perform the two-byte address-setup handshake by hand, then never do
  it by hand again because `vdplib` does it for you.
- Read and write any VDP register, name every bit in R0–R7, and say what turning each one on does.
- Read the VDP status byte, and design around the fact that reading it changes it.
- State the table-driven model of TI video — name table, pattern table, color table — in one breath,
  and recognize it as the shape every mode in Part III instantiates.
- Estimate how many bytes you can move to VRAM in one frame, and lay out a VRAM plan that every
  project in the book will reuse.

## The Bridge: You Already Know Two-Thirds of This

A modern graphics API and a 1979 video chip look nothing alike, but the VDP is built from two ideas
you already carry, plus one you have probably never had to face. Naming all three up front will save
you the week the programmer above lost.

The first idea you know is the **coprocessor with its own memory**. When you upload a texture to a
GPU, you do not write it into the array your `main()` sees; you hand it across a bus to memory that
belongs to the graphics hardware, and from then on the GPU reads it on its own schedule while your
CPU does other things. That is *exactly* the VDP's relationship to the 9900: a separate chip, its
own 16 KiB of VRAM, reached only by handing bytes across. The bus is narrower and the protocol is
older, but the shape is one you have met. The 9900 is a guest in the VDP's memory, the same way your
program is a guest in the GPU's.

The second idea you know, if you have touched a 2D game engine or any retro platform, is the
**tilemap**. Instead of storing a full picture, you store a small set of reusable tiles and a grid
that says which tile goes in each cell. A 40×30 tilemap of 8×8 tiles describes a whole screen with
1,200 cell indices and a few dozen tile definitions, not a quarter-million pixels. The VDP is a
tilemap engine in hardware: its "tiles" are called patterns, its "grid" is called the name table,
and it assembles them into a picture sixty times a second without the CPU's help.

The third idea is the one that stops everyone, because nothing in modern graphics forces it on you:
**there is no framebuffer.** A framebuffer is the array of actual pixels — the thing you write to
in a canvas, the thing a shader ultimately colors. On the VDP, in its normal modes, that array does
not exist anywhere you can address. There is a table of which tile goes where, a table of what each
tile looks like, and a table of colors, and the chip turns those into pixels *on the fly, per
scanline, as the television beam sweeps* — and then forgets them. Ask "what color is pixel
`(100,50)` right now?" and the honest answer is "nowhere, until the beam gets there." You do not
paint pixels and leave them. You arrange tables and let the chip paint, forever, from the tables.
Hold those three ideas — a coprocessor with private memory, a tilemap, and no framebuffer — and the
rest of this chapter is detail.

## 12.1 A Computer Behind the Screen

The TMS9918A is not a peripheral in the way a disk controller is a peripheral. It is a small
special-purpose computer that runs continuously and asynchronously to the CPU, and treating it as a
peer rather than a device is the right posture. It has its own **16 KiB of dynamic RAM** wired
directly to its pins — memory the 9900 has no wires to and cannot name. It has its own **clock**,
faster than the CPU's, timed to the television. And it has one job it does without being asked:
sixty times a second, it scans its VRAM and generates a **256×192** picture in up to fifteen colors
plus transparent, driving the composite-video signal to the television directly. While your program
runs, the VDP is *always* drawing, whether or not you have told it anything new. The picture is not
a thing you produce; it is a process the VDP performs, and you influence it only by changing what it
reads.

Why build it this way — why not do what the Apple II did, and let the screen be a slice of the
CPU's own memory? Two reasons, and both are worth internalizing because they explain the entire rest
of Part III.

The first is **capacity**. A 256×192 picture is 49,152 pixels. Even at a stingy four bits per pixel
that is 24 KiB — more than the VDP's entire 16 KiB of VRAM, and far more than the console could
have spared from its own tiny RAM in 1981. A literal framebuffer simply would not fit. The
table-driven scheme is, at heart, a **compression format the hardware decodes in real time**: a full
Graphics I screen — every one of the 768 cells filled, all 256 possible patterns defined, all
colors set — costs about 768 + 2,048 + 32, under **three kilobytes**, to describe a picture a
framebuffer would need twenty-four to hold. The indirection you are about to learn is not decoration.
It is how you get a screen at all out of a chip this small.

The second reason is **bandwidth**, and it is the one a systems programmer feels. The CPU reaches
VRAM only through the ports, one byte at a time, and as §12.6 will measure, each byte costs on the
order of forty CPU cycles to push across. Filling a hypothetical 49,152-byte framebuffer once would
cost roughly two million cycles — about **forty frames**, nearly a second of the CPU doing nothing
but shoveling pixels, to draw a single still image one time. No game could live like that. But the
table-driven screen changes the arithmetic completely: to redraw the whole visible layout you touch
the 768-byte name table, and — as we will find — that fits in about one frame. You animate by
changing a handful of table entries, not a field of pixels. The chip's model is not merely a way to
save memory; it is the only way the slow CPU-to-VDP link could ever have driven a moving picture.
Every technique in the chapters ahead — every clever way to make things move — is ultimately a way
to change as *few* table bytes as possible per frame.

## 12.2 Four Ports and a Two-Byte Handshake

The CPU's entire vocabulary for talking to the VDP is four memory addresses, which Chapter 5 mapped
and `equates.inc` (Chapter 11) already names:

| Port | Name | What it does |
|---|---|---|
| `>8800` | `VDPRD` | read one VRAM data byte (a *prefetched* byte), then auto-increment |
| `>8802` | `VDPST` | read the status byte (and, as a side effect, clear its top flags) |
| `>8C00` | `VDPWD` | write one VRAM data byte, then auto-increment |
| `>8C02` | `VDPWA` | write the address counter, or a register (a two-byte sequence) |

The two *data* ports are simple: the VDP keeps an internal **14-bit address counter**, and every
read from `>8800` or write to `>8C00` uses that counter and then advances it by one, wrapping at
16 KiB. That auto-increment is the hero of this chapter — it is what lets you set an address once and
then stream a whole block, byte after byte, without touching the address again. The whole trick of
efficient VRAM work is to aim once and pour.

Aiming is the job of the address port, `>8C02`, and it takes **two bytes** because a 14-bit address
does not fit through an 8-bit slot in one pass. You write the **low eight bits first**, then a second
byte whose top two bits choose the operation and whose low six bits carry the high part of the
address:

```text
second byte   meaning
00aaaaaa      set the counter to address aaaaaaaa_aaaaaa for READING (and prefetch)
01aaaaaa      set the counter to that address for WRITING
100000rrr…    (top bit set) write the FIRST byte into VDP register rrr
```

Two details make this protocol survivable. First, the VDP keeps a one-bit **flip-flop** tracking
whether it is expecting the first or the second byte, so the two writes must come as a pair — and if
your code ever loses track of which byte is next (an interrupt firing mid-sequence is the classic
cause, a hazard Chapter 17 returns to), **reading the status port resets the flip-flop**, so a status
read is how you recover sync. Second, notice that "write a register" and "set an address" go through
the *same* port, distinguished only by that second byte's top bits: a register write is just the
address protocol with the high bit set, carrying data instead of an address. One port, three jobs,
told apart by two bits.

Here is the handshake by hand, aiming the counter at VRAM address `>0100` for writing — the last
time in this book you will write it out longhand:

```
       LI   R0,>0040        low byte >00 (in R0's high half), write flag >40
       MOVB R0,@VDPWA       first control byte: the low 8 address bits (>00)
       SWPB R0
       MOVB R0,@VDPWA       second control byte: >41 → 01_000001 = write, hi bits 1
       LI   R1,>AA00
       MOVB R1,@VDPWD       >AA lands in VRAM[>0100]; the counter steps to >0101
```

You will *never* type that again, because the whole point of a library is to write the awkward thing
once. The Lab at the end of this chapter builds `vdplib`, whose `VWA` routine is precisely the
aim-the-counter dance above and whose `VMBW` is aim-once-then-pour. From Chapter 13 on, putting a
block into VRAM is one line — `BL @VMBW` — and the handshake is a solved problem you never look at
again. That is the deal the book keeps making: understand the machine at the bare-metal level once,
then wrap it so you can think at the level of the problem.

## 12.3 Eight Registers That Configure Everything

The VDP has eight **write-only** control registers, R0 through R7. They are not VRAM and not CPU
registers; they are eight bytes of configuration inside the chip, set through the register-write form
of the address port (`vdplib`'s `VWTR`). "Write-only" is a real constraint with a real consequence:
the chip will not tell you what is in them, so **your program must remember what it set** — a shadow
copy in RAM is standard practice, and Chapter 17's time system keeps one. Here is every bit that
matters, gathered in one place for reference; the modes that use each are the next chapters' work.

```text
R0   . . . . . . M3 EXT      M3 = mode bit 3;  EXT = external video (leave 0)
R1   16 BL IE M1 M2 . SZ MG   16 = 4K/16K VRAM,  BL = blank/enable display,
                              IE = interrupt enable,  M1 M2 = mode bits,
                              SZ = sprite size (8×8 / 16×16),  MG = magnify ×2
R2   . . . . n n n n         name table base       = (R2 & >0F) × >0400
R3   c c c c c c c c         color table base       =  R3 × >40    (special in bitmap)
R4   . . . . . p p p         pattern table base     = (R4 & >07) × >0800  (special in bitmap)
R5   . a a a a a a a         sprite attribute base  = (R5 & >7F) × >80
R6   . . . . . p p p         sprite pattern base    = (R6 & >07) × >0800
R7   f f f f b b b b         text/fg color (high) | backdrop color (low)
```

Three groups deserve a first word. The **mode bits** M1, M2, M3 (two of them in R1, one in R0)
select which of four screen modes the chip runs — Graphics I, bitmap, multicolor, or text — and each
of the next chapters is a mode. The **table-base registers** R2–R6 are pointers: they do not hold
tables, they hold *where in VRAM the tables are*, in coarse units (the name table on a 1 KiB
boundary, patterns on 2 KiB, and so on). Moving a table is a single register write, which is exactly
what makes the double-buffering trick of Chapter 13 cheap. And **R1's control bits** are the master
switches you will reach for constantly: `BL` blanks or enables the whole display (blank it while you
rearrange VRAM, so the user never sees the churn), and `IE` enables the frame interrupt that
Chapter 17's game loop is built on. R7 holds two colors in its two nibbles — the foreground/text
color and the backdrop, the latter being the border color the whole book has used as a pass/fail
light since Chapter 7. Setting the border green, it turns out, has been a one-register VDP write all
along; `vdplib`'s `VWTR` is what the border-verdict scaffold was secretly doing.

## 12.4 The Status Byte, and the Danger of Reading It

The registers are how you talk *to* the VDP. The single **status byte**, read from `>8802`, is how
the VDP talks back, and it is a byte you must handle with unusual care, because reading it is not a
passive act — it *changes the chip*.

Its three top bits are events the VDP latches as it draws each frame:

```text
bit 7  F    frame flag: set at the end of every frame (start of vertical blank)
bit 6  5S   fifth-sprite flag: more than four sprites landed on some scanline (Ch. 16)
bit 5  C    coincidence flag: two sprites overlapped a pixel this frame (Ch. 16)
low 5 bits  the number of that offending fifth sprite
```

`F` is the heartbeat of the machine. Sixty times a second, when the beam finishes the visible screen
and the VDP enters vertical blank — the brief window when it is *not* drawing and VRAM is safest to
touch — it sets `F`. If interrupts are enabled (R1's `IE` bit), setting `F` also requests the CPU's
level-1 interrupt, the one whose vector you read cold at `>0004` back in Chapter 9. This is the
clock every game loop in Chapter 17 runs on.

Now the danger. **Reading the status byte clears its top three flags** — this is hardware behavior,
tested on real machines and faithfully modeled by the project VDP (Classic99's `Tiemul.cpp` clears
them to the low five bits on read, and `libre99-core` does the same). Clearing `F` on read is not a bug;
it is the *acknowledgement*. The console's interrupt handler reads status precisely to clear `F` and
release the interrupt line — and if it forgot to, `F` would stay set and the interrupt would fire
again instantly, forever, an interrupt storm that locks the machine. But the same helpfulness is a
trap for you: if your main-loop code reads status to check `F`, and the interrupt handler *also*
reads status, whichever reads first clears the flag and the other sees nothing. Two readers of a
self-clearing flag will steal frames from each other. The whole discipline of Chapter 17's loop —
who reads status, when, and how the rest of the program learns a frame happened without reading it
themselves — exists because of this one side effect. For now, carry the rule: **status is read once
per frame, by one owner, and everyone else is told.** And remember the second, happier side effect
from §12.2: a status read also resets the address flip-flop, your escape hatch when a two-byte
sequence goes wrong.

## 12.5 The Idea That Runs the Next Five Chapters

Everything so far has been plumbing. Here is the idea the plumbing serves, and if you take one thing
from this chapter, take this. The VDP builds its picture from **three tables**, chained by
indirection:

1. The **name table** is the tilemap: one byte per screen cell, 768 of them for the 32×24 grid, each
   byte naming *which* pattern to show in that cell. It is small, and it is what you change to change
   the layout of the screen — move a byte and a different tile appears in that cell.
2. The **pattern table** is the shapes: for each of the 256 possible patterns, eight bytes that are
   the 8×8 bitmap of that tile, one byte per row, one bit per pixel. It is what you change to change
   what a tile *looks like* — and changing one pattern changes every cell that names it, all at once,
   which is either a bug or your best animation trick depending on whether you meant it.
3. The **color table** says which colors the pixels take — in Graphics I, one foreground/background
   pair per *group of eight* patterns, a famous constraint we will meet properly in Chapter 13.

To find the color of a screen cell, the chip does a tiny chain of lookups, every frame, for every
cell: read the name-table byte to get a pattern number; use that number to index the pattern table
for the 8×8 shape and the color table for the colors; emit the pixels. Name → pattern → color. It is
the same indirection a programmer reaches for by instinct — an array of indices into an array of
definitions — done in silicon, at the speed of the television scan.

This single structure is the whole of Part III. Every mode you are about to learn is a different
answer to "how are the three tables laid out and interpreted": Graphics I shares colors across groups
of eight patterns (Chapter 13); bitmap mode gives every cell its own pattern and per-row colors, at
the cost of nearly all your VRAM (Chapter 15); text mode narrows the tiles and drops the color table
to a single pair (Chapter 14); sprites add a *fourth* table of moving objects on top (Chapter 16).
Different layouts, same idea. When a later chapter says "point the name table here and the pattern
table there," you already know what those words mean and why the picture follows. Learn the tables,
and you have learned TI graphics; everything else is which knobs each mode turns.

## 12.6 How Hard You May Push

Because the VDP is drawing continuously and the CPU reaches VRAM only through the narrow ports, there
are two timing questions every graphics routine negotiates: *how expensive* is a VRAM access, and
*when* is it safe. Both have measured answers on the project bench.

**How expensive.** Single-stepping `vdplib` on BENCH99 gives the hard numbers. Writing the address
port — one `MOVB` from a register to `>8C02` — costs **34 cycles**; aiming the counter takes two of
those plus a couple of byte-swaps, so a full address setup is about **134 cycles**. Streaming one
data byte to VRAM — `MOVB *R1+,@VDPWD` — costs **40 cycles** when the CPU-side source is in the fast
island, **44** when the source is itself in slow memory (the "speed is a property of addresses" law
from Chapter 5, reaching all the way into video). With the loop's `DEC` and `JNE`, a bulk copy runs
about **68–72 cycles per byte**. (Each of these VDP-port figures inherits the one open emulator
deviation this book tracks — `libre99-core` omits the 9900's destination pre-read, so a real console may
charge about four cycles more per port write; the shape of the conclusions does not change, and the
ledger row records the caveat.)

Those numbers turn the abstract "there is no framebuffer" argument of §12.1 into arithmetic you can
hold. Clearing the whole 768-byte name table with `VMBW` costs roughly `134 + 768 × 68 ≈ 52,000`
cycles — just over **one frame** (a frame is 50,000 cycles). That is affordable: you can rewrite the
entire visible layout once per frame if you must, which is why full-screen text updates and coarse
scrolling (Chapter 17) are practical. But it is not *cheap* — spend the whole name table every frame
and you have spent the whole frame — which is why real games change a handful of bytes, not the whole
table. And the hypothetical framebuffer from §12.1, at 49,152 bytes, would cost about **forty frames**
to fill once: the measurement that proves the table model was never optional.

**When it is safe.** The honest answer for the TMS9918A on the 4A is gentler than the datasheets of
faster machines suggest, and the reason is a happy accident: the CPU is so slow to push a byte across
that it almost never outruns the VDP's ability to accept one. On systems with faster processors,
programmers must confine heavy VRAM writes to the **vertical-blank window** — the ~4,300-cycle gap
each frame when the beam is off the visible area and the VDP is not competing for VRAM — or risk
visible glitches. On the 4A you have more latitude, because the ~40-cycle round trip per byte is its
own throttle. The exception that bites, and the reason Chapter 17 devotes a pitfall to it, is not
speed but *concurrency*: when both your main loop and an interrupt handler touch the VDP address port,
the two-byte handshake of §12.2 can be torn in half, and the classic TI crash follows. Bandwidth the
4A mostly protects you from; the address-flip-flop race it does not. We will disarm it in Chapter 17.

## 12.7 Planning VRAM: The Layout Worksheet

Sixteen kilobytes is not much, and the table-base registers let you put each table almost anywhere,
so every project begins by deciding *where*. This is the video sibling of Chapter 5's scratchpad
budget, and it earns the same treatment: a written plan, made once, before a byte is placed. A
Graphics I plan — the mode of the next chapter, and the book's default — is short:

```text
VRAM budget — Graphics I (16 KiB)          base register       size
  name table        >0000 – >02FF          R2 = >00            768 B
  sprite attributes >0300 – >037F          R5 = >06            128 B  (Ch. 16)
  color table       >0380 – >039F          R3 = >0E             32 B
  pattern table     >0800 – >0FFF          R4 = >01           2 KiB
  sprite patterns   >1000 – >17FF          R6 = >02           2 KiB  (Ch. 16)
  --- free ---      >1800 – >3FFF                              10 KiB
```

Two habits the worksheet enforces, both learned the hard way by everyone who skipped it. First, the
table-base registers work in **coarse units** — the name table only lands on 1 KiB boundaries, the
pattern table on 2 KiB — so you cannot place tables wherever you like, only where the granularity
allows; the worksheet is where you reconcile what you want with what the registers can express.
Second, the tables **must not overlap** unless you mean them to, and the one that catches beginners is
that a sprite attribute table dropped carelessly can land on top of the color table, corrupting
colors whenever a sprite moves. Writing the map down turns both problems into arithmetic you do at a
desk instead of mysteries you debug on a screen. Every project chapter in this book opens with a
filled-in version of this worksheet, and `code/ch12/` carries a blank one to copy.

> **Field Notes — Reading a 1980 Datasheet as Literature.** The primary source for this entire part
> is the *TMS9918A / TMS9928A / TMS9929A Video Display Processors Data Manual*, and learning to read
> it is a skill worth naming, because a datasheet is not a tutorial and never pretends to be. It
> assumes you already know what you want and only need to be told the exact bits; it states the
> register layout as a table with no example, describes the two-byte address protocol in two dense
> sentences, and gives the sprite rules as a numbered list you must read four times. Read it the way
> you would read a language reference, not a textbook: skim for the *shape*, then return to a section
> only when you have a concrete question the code raised. The register bit-maps (our §12.3), the
> timing diagrams (our §12.6), and the mode descriptions (the next five chapters) are the parts you
> will return to for years. And read it against the hardware: where this book states a behavior, the
> project VDP models it and BENCH99 lets you check it, so the datasheet's flat assertions become
> things you can *watch* happen. The 9918A's manual is terse, exact, and — once you can read it — one
> of the most rereadable documents in the whole TI library, because unlike the folklore it never
> guesses.

## Lab 12 — `vdplib`: The VDP Core for `lib99`

The Lab builds the module every later chapter depends on: `vdplib`, the routines that turn the
awkward port protocol of §12.2 into named operations. They are our clean-room versions of the
Editor/Assembler VDP utilities — `VSBW`, `VMBW`, `VSBR`, `VMBR` — written to the book's calling
convention (R-16) rather than E/A's BLWP-and-list form, plus the register writer `VWTR` and the
address-aimers `VWA`/`VRA` they are built on. The full listing is `code/ch12/vdplib.a99`; here is the
heart of it, the aim-once-then-pour block writer:

```
* VMBW — write R2 bytes from CPU source R1 to VRAM address R0.
VMBW   DECT R10               PUSH R11
       MOV  R11,*R10
       BL   @VWA              aim the write counter once ...
VMBWL  MOVB *R1+,@VDPWD       ... then stream: one byte, step source; VDP steps dest
       DEC  R2
       JNE  VMBWL
       MOV  *R10+,R11         POP R11
       RT
```

Read that against §12.2 and the whole chapter is in six instructions: `VWA` performs the two-byte
handshake to aim the counter (once), and then the loop leans entirely on the auto-increment — the CPU
never touches the address again, it just pours bytes into `>8C00` and the VDP files each one and steps
its own counter. `VMBR` is the mirror image with `VRA` and `>8800`; `VSBW`/`VSBR` are the single-byte
cases; `VWTR` writes a register by the third form of the protocol. Every one is a leaf or a
short non-leaf obeying R-16, and every one comes from `equates.inc` for its port names — pulled across
the tree with `COPY '../ch11/equates.inc'`, which works because libre99asm resolves includes relative to
the source file (Chapter 11), so `lib99`'s shared vocabulary reaches every module.

The self-test dogfriends the library on the bench. It writes an eight-byte pattern to VRAM with
`VMBW`, reads it back to the scratchpad with `VMBR`, compares the two byte for byte, round-trips a
single byte through `VSBW`/`VSBR`, and then paints the verdict *using `VWTR` itself* — register 7 to
green on success — so the border light and the library that lit it are the same code. On BENCH99 at
commit `e97e8ce` it comes up green, and two independent oracles confirm it: the read-back buffer at
`>8340` holds the pattern, and the new `vram` bench command, reading the VDP's private memory directly
without going through the program at all, shows `01 23 45 67 89 AB CD EF` sitting at VRAM `>0100`
exactly as written. A library you have checked three ways — its own read path, a self-comparison, and
an outside witness — is a library the rest of Part III can stand on without looking down.

## Exercises

**✦ Warm-ups.**

1. Write the two control bytes (in order) that aim the VDP counter at VRAM address `>03C0` for
   *reading*. Then do it for *writing*. Which single bit differs, and what is it called?
2. The VDP registers are write-only. Give one concrete bug that this causes if a program forgets it,
   and name the standard defense.
3. Using `vdplib`, write the three-instruction sequence that sets the backdrop (border) color to
   cyan (color 7). Which register, and which nibble?

**✦✦ Consolidation.**

4. A full Graphics I screen is described by name + pattern + color tables. Add up their sizes for a
   screen that defines all 256 patterns, and compare the total to the 24 KiB a literal 4-bit
   framebuffer would need. State the compression ratio and explain, in one sentence, where the saving
   comes from.
5. Using the §12.6 numbers, estimate the cycles to copy a 2 KiB pattern table into VRAM with `VMBW`,
   and convert it to frames. Is defining a full character set once, at startup, affordable? Is doing it
   every frame?
6. Reading the status byte has two side effects. Name both, and for each, describe a situation where
   the side effect helps and one where it hurts.

**✦✦✦ Extensions.**

7. Extend `vdplib` with `VFILL` — fill R2 VRAM bytes at address R0 with the single byte in R1 (high) —
   and add it to the self-test. Confirm with the `vram` bench command that `VFILL` of `>20` across the
   768-byte name table leaves `>20` everywhere. What is `VFILL` good for that `VMBW` is not?
8. The self-test verifies `VMBW` by reading VRAM back with the program's own `VMBR`. Explain why the
   independent `vram` bench command is a *better* oracle than the program's own read routine, and
   construct a bug in `VRA` that a self-comparison would miss but the `vram` command would catch.

## Further Reading

- *TMS9918A / TMS9928A / TMS9929A Video Display Processors Data Manual*, Texas Instruments — the
  primary source for all of Part III: the register bit-maps, the port protocol, the mode
  descriptions, and the timing. Read as literature, per the Field Notes.
- *Editor/Assembler Manual*, Texas Instruments — the official `VSBW`/`VMBW`/`VSBR`/`VMBR` utilities
  and their BLWP-and-list calling convention, the ancestors of `vdplib`; treated fully in Chapters 6
  and 23.
- The project VDP source, `crates/libre99-core/src/vdp.rs` — a readable, commented, hardware-checked
  model of the chip: the port interface (§12.2), the register decode (§12.3), the status side effects
  (§12.4), and the per-mode rasterizers the next chapters explore. When the datasheet is ambiguous,
  this is the behavior the book's measurements pin down.
- Karl Guttag and the TMS9918 design team's contemporary write-ups — the chip's family (used in the
  ColecoVision, MSX, Sega SG-1000, and more) makes it one of the most widely deployed video processors
  of its generation, and its design rationale is history worth knowing.

## Summary

- The screen is produced by the **TMS9918A VDP**, a separate processor with its own **16 KiB of
  private VRAM** the CPU cannot address; the 9900 reaches it only through four ports. The picture is a
  continuous process the VDP performs, not memory you write — **there is no framebuffer**.
- The table-driven model is a necessity, not a quirk: a literal 256×192 framebuffer (24 KiB) exceeds
  VRAM and would take ~40 frames to fill through the slow ports; the tables describe a full screen in
  under 3 KiB and redraw it in ~1 frame.
- **Four ports**: `>8800` read data (prefetched, auto-increment), `>8802` read status
  (side-effect: clears flags), `>8C00` write data (auto-increment), `>8C02` write address/register.
  The **two-byte handshake** at `>8C02` sends the low address byte, then a second byte whose top bits
  pick read (`00`) / write (`01`) / register-write (`1…`). Aim once with the auto-increment, then
  stream. A status read resets the address flip-flop.
- **Registers R0–R7** are write-only chip configuration (keep a RAM shadow): mode bits (M1/M2/M3),
  the master switches in R1 (`BL` blank/enable, `IE` interrupt enable, sprite size/magnify), the
  table-base pointers R2–R6 (coarse-granularity, movable in one write), and R7's two color nibbles
  (foreground | backdrop — the border light). The **status byte** (F/5S/C) is the frame heartbeat;
  reading it clears the flags, so exactly one owner reads it per frame (Ch. 17).
- The **three tables** — name (which pattern per cell), pattern (the 8×8 shapes), color — chained by
  indirection (name → pattern → color) are the whole of Part III; each later mode is a different table
  layout. **Timing** (bench-measured): address-port write 34 cyc, VRAM byte ~40–44 cyc, bulk copy
  ~68–72 cyc/byte, name-table clear ~1 frame — the numbers that make the table model provably
  necessary.
- **Lab `vdplib`** (`lib99`): `VWTR`, `VWA`/`VRA`, `VSBW`/`VMBW`/`VSBR`/`VMBR` — the port protocol
  turned into named operations, verified green three ways (own read path, self-comparison, and the new
  independent `vram` oracle). Every chapter from here builds on it. Seeds: `textlib` (Ch. 13), bitmap
  (Ch. 15), sprites (Ch. 16), the game loop (Ch. 17).
