# Chapter 15 — Bitmap Mode: Graphics II

*The mode where every pixel is finally yours — at the price of most of your memory, and with one last constraint you must master rather than mourn.*

<!-- Part III — The Video Display Processor · target ≈26 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. Register values, the linear name-table map, the (x,y)→byte&bit addressing, Bresenham lines (all directions), and the bring-up/clear cost machine-verified on BENCH99 at commit 0d3e5d5 (re-confirmed against the sibling's beam-accurate rasterizer bd1bbb6, behavior-preserving); code in code/ch15/ (bmplib, grapher). The 12 KiB two-table layout and the R4 cramp gotcha follow from libre99-core's verified render_graphics2 masking. Image-pipeline tooling (§15.6) is Ch. 38 and needs a PC-side converter (python), absent on the PC workstation — presented as method, not shipped here. -->

## Twelve Thousand Bytes for a Single Picture

Somewhere in 1983, a TI owner loads a program that draws a picture — a real picture, a digitized photograph or a hand-drawn title screen, not a mosaic of repeated characters. The screen fills, slowly, top to bottom, the way a fax machine reveals a page. And it is genuinely impressive: curves that are actually curved, shading, detail no arrangement of the 256 fixed characters of Graphics I could ever produce. This is **bitmap mode** — Graphics II — the 9918A's answer to the question every graphics programmer eventually asks: *can I just set the pixel I want?*

The answer is yes. In bitmap mode each of the 256×192 = 49,152 pixels has its own bit, and you can turn any one of them on. It is the closest the TI comes to the framebuffer that Chapter 12 told you it does not have. But look closer at that slowly-filling screen and you can read the price in the speed of the fill. Bitmap mode spends **twelve of the VDP's sixteen kilobytes** on a single image — six kilobytes of pixels and six kilobytes of colour — and writing all of it through the narrow VDP port takes, as we will measure, the better part of twenty video frames. There is no second picture in the remaining four kilobytes. There is barely room for the machinery to draw the first.

And there is one more constraint, the one that defines the bitmap-mode aesthetic as surely as the eight-character rule defined Graphics I. You get a bit per pixel, but you do **not** get a colour per pixel. Colour still comes in pairs — a foreground and a background — and a single pair must serve a horizontal run of eight pixels. Put a red pixel and a blue pixel in the same eight-pixel strip and they cannot both have their colour; the strip has room for two colours total, and the second one you set wins for all eight. This is the famous **attribute clash**, and this chapter's job is to make you master it rather than mourn it — to show you the addressing, the drawing primitives, and the honest performance envelope of the one mode where the TI lets you paint.

---

## What You Will Learn

- How bitmap mode really works: the screen divided into **thirds**, the name table as 768 sequential indices, and the 6 KiB pattern + 6 KiB colour tables that hold the picture.
- The **8×1 colour-pair rule** — the attribute clash — and how to design around it instead of fighting it.
- The register setup, including the **notorious mask bits** in R3 and R4 whose exact values (`>FF` and `>03`) decide whether you get a full bitmap or a cramped, repeating mess.
- The pixel-addressing math: turning an (x, y) coordinate into a VRAM byte address and a bit mask, and speeding it up with lookup tables.
- Drawing primitives — `PSET`, lines by Bresenham's algorithm, rectangles and fills — built into `bmplib`, our bitmap library.
- How text and full-screen images reach the bitmap, and where the modern asset pipeline (Ch. 38) fits.
- The **cost ledger**: why 12 of 16 KiB gone forces hybrid layouts, and the measured performance envelope that tells you exactly what full-screen bitmap animation can and cannot do at 3 MHz.

## The Bridge: A Framebuffer with Rules

To a modern programmer, "bitmap" and "framebuffer" are synonyms: a rectangle of memory, one entry per pixel, each entry the full colour of that pixel, and you write whichever you like whenever you like. The GPU has gigabytes of the stuff, and a 256×192 image in 32-bit colour — 192 KiB — is a rounding error.

Graphics II is a framebuffer with three rules bolted on, each a consequence of 1979 economics, and understanding the rules is understanding the mode:

1. **A bit, not a colour, per pixel.** The pixel memory stores one bit per pixel — on or off, foreground or background — because storing a full colour per pixel would need far more than 16 KiB. 256×192 bits is 6 KiB; that fits. 256×192 *bytes* would be 48 KiB; that does not.
2. **Colour shared per 8×1 strip.** The colour of "on" and "off" is stored separately, one foreground/background pair for every horizontal run of eight pixels — a second 6 KiB table. This is the compromise between "one colour for the whole screen" (too dull) and "a colour per pixel" (too expensive), and it is the attribute clash.
3. **No cheap update.** There is still no framebuffer the CPU can address (Ch. 12); the 12 KiB lives in VRAM, reachable only one byte at a time through the port. Changing a pixel is a read-modify-write across that port, and changing the whole screen is ~17 frames of them.

Hold those three rules in mind and every strange thing about bitmap mode — the thirds, the mask bits, the clash, the glacial full-screen fills — becomes not strange but inevitable. Let us take them in order.

## 15.1 How Bitmap Mode Really Works: The Screen in Thirds

Bitmap mode reuses the machinery of Graphics I — a name table, a pattern table, a colour table — and bends each into a new shape. The bend is worth following carefully, because it explains the thirds, and the thirds explain the addressing.

Recall Graphics I (Ch. 13): the 32×24 name table holds 768 bytes, each naming one of 256 patterns; the pattern table holds those 256 eight-byte glyphs; and 32 colour bytes tint them by group of eight. The picture is characters, and there are only 256 distinct ones.

Bitmap mode keeps the 768-entry name table but changes what a name *means*. The screen is split into three horizontal **thirds** — the top eight character-rows, the middle eight, the bottom eight — and each third gets its **own** 2 KiB region of the pattern table and its own 2 KiB region of the colour table. Within a third, the name-table entry for a cell still selects one of 256 pattern definitions — but because each third has its own 2 KiB (256 × 8 bytes) of definitions, and because the standard setup fills the name table so that the 768 cells select definitions 0, 1, 2, … in order, *every cell ends up pointing at its own unique eight bytes of pattern*. There is no sharing. 768 cells × 8 bytes = 6 KiB of pattern, one byte per 8×1 pixel strip, every strip independent. The "characters" are a fiction; what you really have is a linear canvas, and the name table is the indirection that unrolls it.

The colour table works identically and in parallel: 6 KiB, split into thirds, one colour byte for every pattern byte — that is, one foreground/background pair for every 8×1 strip. Pattern byte and colour byte at the same offset describe the same eight pixels: the pattern byte says which of the eight are foreground, the colour byte says what foreground and background *are*.

> **The 8×1 colour-pair rule (the attribute clash), stated once.** In bitmap mode, each horizontal run of eight pixels — one pattern byte — shares a single colour byte: high nibble the foreground colour, low nibble the background. The eight pixels may be any pattern of on and off, but "on" is one colour and "off" is one colour, the same two for all eight. You cannot have three colours in eight horizontal pixels. Two adjacent pixels that need different foreground colours, if they fall in the same 8×1 strip, will get whichever colour was written to that strip last — the other silently changes to match. This is the attribute clash, and it is not a bug to be fixed but a budget to be spent: eight pixels, two colours, choose them well. Artists on the TI (and the ZX Spectrum, which had the same disease one cell-shape worse) learned to align colour boundaries to the 8-pixel grid, to draw outlines in a colour that reads against any fill, and to let the clash define a chunky, confident style rather than apologize for it.

## 15.2 Register Setup and the Notorious Mask Bits

Bringing up bitmap mode is six register writes, and two of them carry values that TI programmers have cursed for forty years. Here is `bmplib`'s `BMODE`, verified on the bench:

```asm
       LI   R0,0
       LI   R1,>0200        R0 = >02: M3 = 1  (this is what makes it bitmap)
       BL   @VWTR
       LI   R0,1
       LI   R1,>E000        R1 = >E0: 16K on, display on
       BL   @VWTR
       LI   R0,2
       LI   R1,>0600        R2 = >06 -> name table at >1800
       BL   @VWTR
       LI   R0,3
       LI   R1,>FF00        R3 = >FF -> colour table at >2000, FULL span
       BL   @VWTR
       LI   R0,4
       LI   R1,>0300        R4 = >03 -> pattern table at >0000, FULL span
       BL   @VWTR
```

The mode bit is straightforward: bitmap mode is `M3 = 1`, and M3 is bit 1 of register 0, so `R0 = >02`. Registers 1 and 2 are familiar — 16 KiB and display on; name table at `>1800`, tucked into the 2 KiB gap between the 6 KiB pattern table below it and the 6 KiB colour table above.

Registers 3 and 4 are the notorious ones, because in bitmap mode they are not simple base pointers. Each is a **select bit plus an address mask**, and getting the mask wrong does not move your table — it *shreds* it.

**Register 4 (pattern), `>03`.** Bit 2 (`>04`) selects which half of VRAM the pattern table lives in: clear means the low half, so pattern base `>0000`. The **low two bits** are an AND-mask applied to the table-relative address. Set them (`>03`) and the mask is open — the full 8 KiB (6 KiB used) of pattern is addressable. Clear them (`R4 = >00`) and the mask forces the pattern address down to its low 11 bits: only the first 2 KiB is ever read, and it **repeats** in all three thirds. This is the single most common bitmap bug: the top third of your picture appears correctly, and the middle and bottom thirds are eerie copies of it. The fix is one hex digit: `R4 = >03`, not `>00`.

**Register 3 (colour), `>FF`.** Bit 7 (`>80`) selects the colour table's half: set means the high half, colour base `>2000`. The **low seven bits** (`>7F`) are the colour address mask, and as with R4 they must be open for the full table to be reachable. `>FF` is bit 7 set (table at `>2000`) plus a full `>7F` mask. Write `>7F` instead and you clear the select bit: the colour table drops to `>0000`, directly on top of the pattern table, and pattern and colour corrupt each other. `>FF` and `>03`: memorize them, or better, keep them named in a library and never type the bare hex again.

Why this baroque select-plus-mask scheme instead of a plain base register? Because the 9918A's designers exposed the raw address lines, and the "mode" is really just the chip fetching pattern and colour bytes with the third's offset added — the mask bits are address lines TI left under programmer control. It is hardware economy showing through the abstraction, and it is exactly the kind of leak this book would rather explain than paper over.

One more setup step lives inside `BMODE`: filling the name table so the canvas is linear. The 768 entries are written `0, 1, 2, …, 255, 0, 1, …, 255, 0, 1, …, 255` — three counts of 0 through 255, one per third — so that cell *n* within a third selects definition *n*, and combined with the per-third pattern regions, every cell maps to its own eight bytes:

```asm
BMNL   MOV  R3,R1
       ANDI R1,>00FF        i AND >FF   (0..255, repeating each third)
       SWPB R1
       MOVB R1,@VDPWD
       INC  R3
       CI   R3,768
       JNE  BMNL
```

Written once, in a library, and never thought about again.

## 15.3 From Coordinate to Bit: The Addressing Math

With the name table linear, a pixel's address is pure arithmetic — and deriving it is the intellectual core of the mode. Take pixel (x, y), x in 0–255, y in 0–191. Its character cell is column `x ÷ 8`, row `y ÷ 8`; its third is `row ÷ 8`. Walk the same indirection the chip walks — third's 2 KiB offset, plus the cell's definition, plus the line within the cell — and, with the linear name map, the whole thing collapses to a clean closed form:

```text
pattern byte address = (y AND >F8) * 32  +  (x AND >F8)  +  (y AND 7)
bit within the byte  = >80 >> (x AND 7)
```

Read the three terms. `(y AND >F8)` is y with its low three bits cleared — the pixel's character-row times eight; times 32 gives the row's base offset (each character-row of the canvas is 32 cells × 8 bytes = 256 bytes, and `(row×8)×32 = row×256`). `(x AND >F8)` is the cell's column times eight — the byte within the row. `(y AND 7)` is the line within the cell — which of the eight bytes of this cell. The bit is a single mask: pixel column 0 (leftmost) is `>80`, column 7 is `>01`.

The colour byte for the same pixel is at the *same* offset in the colour table: `>2000 + (that address)`. Pattern and colour move together, which is what makes the 8×1 rule so mechanical to honour — set the pattern bit, and if you care about colour, write the fg/bg pair to the twin byte at `+>2000`.

`bmplib`'s `BPSET` is that formula plus a read-modify-write, because turning on one pixel means OR-ing one bit into an existing byte without disturbing its seven neighbours:

```asm
BPSET  ...
       MOV  R1,R3
       ANDI R3,>00F8
       SLA  R3,5            (y AND >F8) * 32
       MOV  R0,R4
       ANDI R4,>00F8
       A    R4,R3          + (x AND >F8)
       MOV  R1,R4
       ANDI R4,7
       A    R4,R3          + (y AND 7)   -> R3 = pattern byte address
       MOV  R0,R5
       ANDI R5,7
       MOVB @BMASK(R5),R6   R6 high = the bit mask >80>>(x AND 7)
       MOV  R3,R0
       BL   @VSBR          read the current byte ...
       SOCB R6,R1          ... OR the bit in ...
       MOV  R3,R0
       BL   @VSBW          ... write it back
```

Two details earn their keep. The bit mask comes from an eight-entry **lookup table** `BMASK` — `>80, >40, >20, >10, >08, >04, >02, >01` — indexed by `x AND 7`, because the 9900's shift instructions take a variable count only through register 0, which we are already using for x; a table sidesteps the awkwardness and is faster besides. And the write is a genuine read-modify-write (`VSBR` then `SOCB` then `VSBW`), so that plotting a second pixel in a byte does not erase the first — the single most common way a hand-rolled `PSET` goes wrong is to write the bit as the whole byte, blanking its seven neighbours.

> **Sidebar — The lookup table you don't see: row addresses.** `BPSET` recomputes `(y AND >F8) * 32` on every call — three instructions of shifting and masking. A plotting-heavy program can precompute a table of character-row base addresses and replace the arithmetic with an indexed load. It trades a few hundred bytes of RAM for a handful of cycles per pixel — a classic space-for-time bargain, and exactly the sort of optimization Chapter 37 formalizes. We leave `bmplib` arithmetic-based for clarity; the exercises invite you to make it table-based and measure the win.

We can prove the whole apparatus. `bmplib`'s self-test draws a full-screen box outline and an X of diagonals through `BLINE`/`BPSET`, and the `pixels` oracle shows the top and left edges solid and the two diagonals marching corner to corner. The far edges (x = 255, y = 191) fall between the oracle's step-4 sample points, so we confirm them by reading the exact pixels back: pattern byte `>17FF` (the bottom-right corner, pixel 255,191) reads `>FF`, and `>0CF8` (the right edge at y = 96) reads `>01` — the rightmost bit, exactly one pixel. Every edge is where the math says it should be.

## 15.4 Lines, Rectangles, and Fills

A single-pixel `PSET` is the atom; real drawing is lines, boxes, and filled regions built on it.

### Lines: Bresenham on the 9900

The line-drawing algorithm every graphics library reaches for is **Bresenham's** — 1962, IBM, and still the right answer — because it draws a line from (x₀, y₀) to (x₁, y₁) using only integer addition and comparison, no division and no floating point, which is precisely the arithmetic a 3 MHz CPU with no FPU is good at. The idea is an *error accumulator*: step along the major axis one pixel at a time, and at each step add the minor-axis slope to a running error; when the error crosses a threshold, step the minor axis too and subtract the threshold back. The line stays as close to the ideal as integers allow, and every operation is an add or a compare.

`bmplib`'s `BLINE` is the standard integer form, its working variables — the two deltas, the two step directions, the error term, the current point — kept in a block of scratchpad so they survive the `BPSET` calls:

```asm
BLSTEP MOV  @BLERR,R6      e2 = 2 * err
       SLA  R6,1
       C    R6,@BLDY       if e2 >= dy: err += dy; x += sx
       JLT  BLSKX
       MOV  @BLERR,R7
       A    @BLDY,R7
       MOV  R7,@BLERR
       MOV  @BLX,R7
       A    @BLSX,R7
       MOV  R7,@BLX
BLSKX  C    R6,@BLDX       if e2 <= dx: err += dx; y += sy
       JGT  BLSKY
       ...
```

The signed comparisons (`JLT`, `JGT` on the arithmetic-greater flag, Ch. 8) are the subtlety: the error term goes negative, so this is one of the places where the difference between the 9900's signed and unsigned compares (Ch. 8's hard-won lesson) actually decides whether your lines are straight. `BLINE` is verified drawing all four box edges (horizontal and vertical) and both diagonals — the full range of slopes — with the corners landing on the exact pixels §15.3 predicted.

### Rectangles and fills

A rectangle outline is four `BLINE`s. A *filled* rectangle is a stack of horizontal lines — but here the 8×1 colour rule reasserts itself, and a smart fill respects it. Filling a horizontal span pixel-by-pixel with `BPSET` works but is wasteful: a run of eight aligned "on" pixels is just the byte `>FF`, and writing it as one byte through the port is eight times faster than eight read-modify-writes. A production span-fill therefore handles the ragged ends of the span pixel-by-pixel and the aligned middle byte-by-byte (whole `>FF` writes), and — because each of those bytes has a twin in the colour table — sets the colour of the spanned strips in the same pass. "Span fills that respect colour cells" is the phrase from the outline, and it means exactly this: fill in byte-aligned chunks, and colour those chunks as you go, so the fill is fast *and* clash-free within itself. The exercises build one.

## 15.5 Text Over the Bitmap

The bitmap has no character generator — it is raw pixels — so text on a bitmap means *drawing* the letters, pixel by pixel, from a font. The mechanism is pleasingly direct, and it reuses the fonts we already have (Ch. 13, Ch. 14). A character cell in the bitmap is eight consecutive pattern bytes — the eight lines of an 8×8 cell — at exactly the byte address §15.3 gives for the cell's top-left pixel, with the eight lines at consecutive `(y AND 7)` offsets. To stamp a glyph, you OR its eight font bytes into those eight pattern bytes:

```asm
* sketch: stamp the 8-byte glyph at *R1 into the bitmap cell (col R2, row R3)
*   addr = (R3*8 AND >F8)*32 + (R2*8 AND >F8)   ; the cell's top-left byte
*   for line = 0..7:  pattern[addr + line] |= glyph[line]
```

Because it is an OR, text lands *over* whatever pixels are already there — a label on a graph, a score on a play-field. And because the bitmap's colour is per 8×1 strip, a stamped character can be given its own colour independent of the graphics around it, as long as it sits on the 8-pixel grid — which is why bitmap text on the TI is almost always cell-aligned. The variable-width, any-position text a modern renderer takes for granted is possible here too, by shifting the glyph bits within the byte, but it means read-modify-writing two bytes per line and managing the colour clash at every boundary — a real cost, and the reason the productivity software of the era (Ch. 14's TI-Writer) stayed in the fixed-grid text mode instead. We return to proportional bitmap text as an optimization study in Chapter 37.

## 15.6 Full-Screen Images: The Modern Pipeline

The most dramatic use of bitmap mode is a full-screen image — a title screen, a digitized photograph, a piece of pixel art — and producing one in 2026 is a pipeline problem, most of it running on your PC before a single byte reaches the TI.

The stages are these. Start with a source image and reduce it to the TI's realities: 256×192 resolution, the fifteen-colour palette (App. D), and — the hard one — the 8×1 colour constraint. A general full-colour image cannot be shown directly; it must be *quantized* so that every horizontal run of eight pixels uses at most two colours. Good converters do this with **dithering** — scattering the quantization error into neighbouring pixels so that colours the palette cannot name are approximated by patterns of colours it can — and with clash-aware colour choice per 8×1 strip, picking the two colours that best represent each strip. The output is two byte-arrays: 6 KiB of pattern and 6 KiB of colour, in exactly the VRAM layout §15.1 describes. Those arrays are then either baked into the program (a cartridge carries them in ROM, Ch. 35) or loaded from disk at run time (Ch. 31), and pushed into VRAM through a single bulk transfer — `VMBW` (Ch. 12), 6 KiB to `>0000` and 6 KiB to `>2000`.

The conversion tool is a PC-side program — the kind of asset script Chapter 38 builds in earnest — and it is genuinely a *tool*, not something you write in 9900 assembly. This book's project toolchain does not yet ship an image converter (and the PC workstation these chapters are written on has no Python to run the usual community tools), so we describe the pipeline here as method and build the converter in Chapter 38, where the asset pipeline is the subject. What matters at this point in the book is that you understand the *shape* of the data — two 6 KiB tables, clash-quantized — and know that getting it onto the screen is one bulk `VMBW`, not fifty thousand `PSET`s.

## 15.7 The Cost Ledger: Twelve of Sixteen

Bitmap mode's defining number is its memory bill. Lay it out:

| Table | Size | Where |
|---|---|---|
| Pattern (the pixels) | 6 KiB | `>0000`–`>17FF` |
| Colour (per 8×1 strip) | 6 KiB | `>2000`–`>37FF` |
| Name table (the linear map) | 768 B | `>1800`–`>1AFF` |
| **Total committed** | **≈ 12.75 KiB** | of 16 KiB |

That leaves roughly 3 KiB of VRAM for everything else — sprite attributes and patterns (Ch. 16), a second workspace, anything. There is **no** room for a second full screen, which means bitmap mode cannot double-buffer the way Graphics I can (Ch. 13's two-name-table flip): the picture you are drawing is the picture being displayed, and the viewer sees your work in progress unless you are careful about *where* on the screen you draw relative to the beam (Ch. 17).

This scarcity drove real design. Professionals used **hybrid layouts** — "half-bitmap" tricks — that refuse to pay for what they do not use. A game with a bitmap play-field and a text status panel need not make the panel a bitmap: because each third has its own tables, you can run the top thirds as detailed bitmap and treat the bottom third as coarser or repetitive content whose pattern definitions repeat, reclaiming pattern and colour memory. A screen that only needs graphics in one band leaves the other thirds' tables nearly empty. The mode's rigidity — three fixed thirds, fixed table sizes — becomes, in skilled hands, a set of independently-budgetable regions. Knowing the ledger is what lets you spend it.

## 15.8 Performance Truth-Telling

Here is the number that governs everything you can do with a bitmap, measured on the bench. Bringing up bitmap mode and clearing it once — filling the 768-byte name map and writing all 12,288 bytes of a blank pattern-and-colour canvas through the port — costs:

**869,894 cycles** — about **17 video frames** at 50,000 cycles per frame.

Read that number and the consequences fall out. You cannot clear the bitmap in a frame; you cannot clear it in ten. A game that tried to erase and redraw the whole screen every frame — the naïve "clear, draw, repeat" loop of a modern immediate-mode renderer — would run at three or four frames per second, a slideshow. Full-screen bitmap *animation*, in the sense of repainting all 49,152 pixels sixty times a second, is simply not possible at 3 MHz through this port. The arithmetic forbids it.

So what *is* possible? Everything that touches only *part* of the screen. Plotting a function — our grapher touches 256 pixels, one per column, and finishes in a fraction of a frame. Drawing and moving a few lines. Updating a small dirty region — a gauge, a cursor, a scrolling band — while the rest of the picture holds still. The discipline this forces — *never redraw what did not change* — is not a limitation of bitmap mode so much as the central skill of real-time graphics on any constrained machine, and Chapter 17 makes it the organizing principle of the game loop. Bitmap mode is where you first feel the beam breathing down your neck: the picture is expensive, the frame is short, and the only way to win is to touch few pixels. The honest measurement is the gift. A programmer who knows the bitmap costs 17 frames to clear designs a game that never clears it; a programmer who guesses designs a slideshow and blames the machine.

> **Sidebar — The owners who couldn't run any of this.** Bitmap mode is a feature of the TMS9918**A**. The original TI-99/4 — the 1979 machine, before the 4A — shipped with the plain TMS9918, whose bitmap mode was not available to programmers the way the 4A's is. Those owners had Graphics I, multicolor, and text, but not the polished Graphics II: the most impressive mode of the chapter was, for them, effectively out of reach. This is a small, concrete instance of a large truth about the platform — that "the TI-99/4A" is not one fixed machine but a family with capability tiers (Ch. 44), and code that assumes bitmap mode assumes a chip revision. The professional habit, then and now, is to know exactly which capability you depend on and to fail gracefully — or refuse to run with a clear message — where it is absent, rather than draw garbage. We meet this again with the sound chip's variants, and most sharply with the F18A and 9938 successors (Ch. 18), where the *newer* chips add modes a 4A-first program must not assume either.

## Lab 15 — `bmplib` and a Function Grapher

The lab is the bitmap library and a demonstration of it, both in `code/ch15/`, both machine-verified.

**`bmplib` (`bmplib.inc` + `bmplib.a99`)** — the bitmap engine for `lib99`: `BMODE` (set Graphics II, lay the linear name map, clear the canvas), `BCLS` (clear and set the canvas colour), `BPSET` (turn on one pixel), and `BLINE` (Bresenham). Build and prove it:

```sh
libre99asm code/ch15/bmplib.a99 --format bin -o build/BMPC.bin --symbols build/bmp.map.json
```

On the bench: `load`, `pc` to the entry, `x 400000`, then `pixels 4` for the box-and-X picture and `vdp` for the green verdict (`R7=>02`, meaning pixel (0,0) read back set). Read the exact edge pixels with `vram 17FF 1` and `vram 0CF8 1` to see the far edges the sampled view misses.

**`grapher.a99`** — a function grapher: axes by `BLINE`, then a parabola `y = 40 + (|x−128|² ≫ 7)` plotted one pixel per column through `BPSET`, reusing the 9900's `MPY` for the square. `pixels 4` shows the crossed axes and the arc. It is the smallest complete argument for bitmap mode: per-pixel plotting of a computed function, something no character mode can do.

**A slideshow with fades** is the lab's stretch goal, and it is really a Chapter 38 exercise wearing a Chapter 15 costume: the *display* half is one bulk `VMBW` of a 12 KiB image (trivial, given §15.6's data), but producing the image — the clash-aware, dithered conversion — needs the PC-side converter we build there. Fades, on a chip with no palette registers to ramp, are done by swapping the colour table for progressively darker versions of itself, or by dithering toward the backdrop — a colour-table animation the exercises explore once you have an image to fade.

## Exercises

**15.1** ✦ Give the exact `R3` and `R4` values for bitmap mode, and explain what goes visibly wrong on screen if you write `R4 = >00` instead of `>03`.

**15.2** ✦ Using §15.3's formula, compute the pattern-byte address and bit mask for pixel (100, 50). Check yourself on the bench: `BPSET` it and read the byte back with `vram`.

**15.3** ✦✦ Explain the attribute clash by constructing a case: plot a red pixel at (10, 10) and a blue pixel at (13, 10), setting each strip's colour as you go. What colour is each pixel afterward, and why? (Both are in the same 8×1 strip: pixels 8–15 of row 10.)

**15.4** ✦✦ Add `BBOX` (rectangle outline: x0,y0,x1,y1) to `bmplib` as four `BLINE`s, and `BFILL` (filled rectangle) as a stack of horizontal `BLINE`s. Then rewrite `BFILL`'s middle to write aligned `>FF` bytes directly instead of `BPSET`-ing each pixel, and measure the speedup with `cycles`.

**15.5** ✦✦ Make `BPSET` table-driven: precompute a 24-entry table of character-row base addresses and replace the `(y AND >F8) * 32` arithmetic with an indexed load. Measure the per-pixel cost before and after over a 1,000-pixel plot.

**15.6** ✦✦ Implement `BSTAMP` from §15.5's sketch — OR an 8-byte glyph into a bitmap cell — and use it with Chapter 14's font to label the grapher's axes. Confirm the text lands over the graphics with `pixels`.

**15.7** ✦✦✦ Write a `BPRES` (pixel reset / erase) that clears a bit with `SZCB` (set zeros, byte) instead of setting it, and use `BPSET`/`BPRES` to animate a single pixel bouncing around the screen — clearing its old position, setting its new one, each frame. This is dirty-rectangle updating in miniature; note how many VRAM accesses per frame it costs versus clearing the whole screen (§15.8).

**15.8** ✦✦✦ The §15.8 measurement clears 12,288 bytes in ~870,000 cycles. Derive the per-byte cost, compare it to Chapter 12's measured VDP-port write timing, and explain the difference in terms of the aim-then-stream pattern (how many `VWA` address-setups the clear performs, and what each costs). Then estimate the largest screen region you *could* fully repaint every frame at 60 Hz.

## Further Reading

- *TMS9918A Video Display Processor Data Manual*, Texas Instruments — the bitmap (Graphics II) mode description and the register-3/4 mask semantics behind §15.2.
- J. E. Bresenham, "Algorithm for computer control of a digital plotter," *IBM Systems Journal* 4:1 (1965) — the line algorithm `BLINE` implements.
- Chapter 12 (Inside the TMS9918A) — the port protocol and `VMBW`, the bulk transfer §15.6 loads images through.
- Chapter 17 (Motion) — the beam, the frame budget, and the dirty-rectangle discipline §15.8's cost makes mandatory.
- Chapter 37 (Optimization) — table-driven addressing, proportional bitmap text, and the space-for-time trades the exercises preview.
- Chapter 38 (Data Compression and the Asset Pipeline) — the PC-side image converter §15.6 describes, built for real.

## Summary

Bitmap mode (Graphics II, `M3 = 1`) is the 9918A's real bitmap: a bit per pixel across all 256×192, the closest the chip comes to a framebuffer. The screen is divided into three horizontal **thirds**, each with its own 2 KiB of pattern and 2 KiB of colour; the 768-entry name table, filled `0..255` three times, unrolls the "characters" into a linear 6 KiB pixel canvas with a parallel 6 KiB colour table. Colour obeys the **8×1 rule**: one foreground/background pair per horizontal run of eight pixels — the attribute clash, a budget to spend rather than a bug to fix. Setup is six register writes, two of them the notorious mask bits: `R4 = >03` (not `>00`, which cramps the pattern to a repeating 2 KiB) and `R3 = >FF` (not `>7F`, which drops the colour table onto the pattern). A pixel's address is `(y AND >F8)*32 + (x AND >F8) + (y AND 7)`, bit `>80 >> (x AND 7)`, with the colour twin at `+>2000`; `bmplib` builds `BPSET` (a read-modify-write, bit-masked from a lookup table) and `BLINE` (Bresenham, integer-only, signed compares) on that math, all bench-verified. Text on the bitmap is font glyphs OR-ed in cell by cell; full-screen images are a PC-side clash-aware dithering pipeline (Ch. 38) that ends in one bulk `VMBW`. The cost ledger is stark — 12.75 of 16 KiB committed, no room to double-buffer — and the measured truth is starker: bringing up and clearing the bitmap costs ~870,000 cycles, about 17 frames, so full-screen animation is impossible and the only winning strategy is to touch few pixels. That discipline — never redraw what did not change — is the bridge into Chapter 17's game loop.
