# Chapter 14 — Text Mode and Multicolor Mode

*Two roads out of Graphics I: one toward words, one toward a painted canvas — and the register dance that gets you there and back.*

<!-- Part III — The Video Display Processor · target ≈14 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. All register values, VRAM layouts, the multicolor addressing math, and every screen/pixels transcript machine-verified on BENCH99 at commit 0d3e5d5; code in code/ch14/ (textlib40, mcolib, carousel, monitor40). The `pixels` framebuffer view and the mode-aware `screen` were added to BENCH99 this session (commit 0d3e5d5) as the pixel-level oracle for Part III. -->

## The Widest Forty Columns in the World

Picture the machine doing something it was never advertised for: word processing. It is 1983, the cartridge in the slot is TI-Writer, and the family television is showing a page of text — someone's letter, or a school report, or the manual for a program they are writing. The screen holds forty characters across. Not eighty, the width a "real" word processor on a "real" computer would show; forty, because forty is what a television can resolve without the letters smearing into each other. To see a line longer than forty columns you scroll sideways, the text sliding under a window like a sheet of paper pulled past a slot.

It is easy, from here, to see only the limitation. But look at what had to change inside the VDP to make even those forty columns appear. Graphics I, the mode we spent Chapter 13 in, shows thirty-two columns — the same thirty-two every TI game uses — and it will not show a thirty-third. Forty columns is a *different display mode*, with a different cell shape, a different colour rule, a different name-table size, and no sprites at all. The chip that draws Parsec can also draw a word processor, but not at the same time and not with the same settings. Somewhere in TI-Writer's startup is the exact register dance this chapter teaches: blank the screen, rewrite the mode bits, re-lay-out video memory, unblank. Forty columns is not a smaller thirty-two. It is a whole other machine the VDP knows how to become.

This chapter is about the two modes on either side of Graphics I. **Text mode** leans toward words: it trades away colour-per-cell and the entire sprite plane to buy eight more columns and a narrower letter. **Multicolor mode** leans the other way, toward a painted canvas: it throws away the font entirely and lets you colour a coarse grid of fat pixels directly, no bitmaps to design. Neither is where the VDP's graphics story ends — that is bitmap mode, and it is the whole of Chapter 15 — but each teaches something the others cannot, and together they teach the most important operational skill in Part III: how to change the machine's mind about what kind of display it is, cleanly, at runtime.

---

## What You Will Learn

- How **text mode** is configured — the mode bits, the 40×24 name table, the two colours in register 7 — and why it has a 6-pixel cell and no sprites.
- Why a font drawn five pixels wide renders identically in Graphics I and text mode, and how to build one that does.
- `textlib40`: a 40-column text engine for `lib99`, the sibling of Chapter 13's `textlib` and the foundation the Chapter 42 editor will stand on.
- How **multicolor mode** turns the screen into a 64×48 grid of fat pixels, how its name table is laid out, and the addressing math that turns a pixel coordinate into a VRAM byte and nibble — the `PLOT64` API.
- Why almost no one shipped multicolor mode, and the handful of reasons to reach for it anyway.
- How to **switch modes cleanly at runtime** — the blank/reconfigure/relayout/unblank discipline — and how to keep your VRAM plans from fighting each other across modes.

## The Bridge: Every Mode Is a Contract with the Beam

A modern program never thinks about display *modes*. It asks the windowing system for a surface of some width and height in some pixel format, and a compositor — backed by a GPU with gigabytes of its own memory — makes it appear. Text is just pixels you drew from a font atlas; a bitmap is just pixels you drew some other way. There is one canvas, and everything is pixels on it.

The VDP has no canvas (Ch. 12). It has 16 KiB of VRAM and a beam that re-reads that VRAM sixty times a second, and the *only* thing that decides how those bytes become a picture is a handful of mode bits in registers 0 and 1. Flip them and the same 16 KiB is interpreted a completely different way: the bytes that were a name table are now a fat-pixel canvas; the bytes that were a colour table are now nothing at all. A **mode**, on this machine, is a contract between your software and the beam about how to read memory. There are four such contracts, selected by three bits:

| M1 | M2 | M3 | Mode | Cells | Colour | Sprites |
|----|----|----|------|-------|--------|---------|
| 0 | 0 | 0 | Graphics I | 32×24 | per group of 8 chars | yes |
| 0 | 0 | 1 | Graphics II (bitmap) | 32×24 | per 8×1 line | yes |
| 0 | 1 | 0 | Multicolor | 64×48 blocks | per block | yes |
| 1 | 0 | 0 | Text | 40×24 | one pair (R7) | **no** |

`M1` is bit 4 of register 1, `M2` is bit 3 of register 1, `M3` is bit 1 of register 0 — the layout we first met in §12.4. Only one bit is set in any legal mode; setting two is undefined (our emulator, like the hardware, falls back to a Graphics-I-like reading, but never rely on it). We spent Chapter 13 in the first row. Chapter 15 is the second. This chapter is the last two — and, just as importantly, the act of moving between rows.

## 14.1 Text Mode: The Productivity Contract

Text mode is what you choose when the screen is going to hold *words*. It makes three trades against Graphics I, and every one of them is in service of legible text on a television.

**It is forty columns wide, not thirty-two.** The name table grows from 32×24 = 768 entries to 40×24 = 960 entries, and each entry still names one character. Forty columns of text is a third more per line — the difference between a cramped display and one you can actually write a sentence on.

**Its cells are six pixels wide, not eight.** Forty cells across a 256-pixel-wide screen would need 320 pixels if each were eight wide; they do not fit. So text mode narrows the cell: it draws only the **top six bits** of each pattern byte and discards the low two. Forty cells × 6 pixels = 240 pixels, which sits inside the 256-pixel active line with a 16-pixel backdrop margin. The character *definitions* are still eight bytes of eight bits each, exactly as in Graphics I — the chip simply ignores bits 1 and 0 of every row.

**It has one colour pair for the entire screen, and no sprites.** There is no colour table in text mode. Instead, register 7 — which everywhere else holds only the backdrop colour in its low nibble — is pressed into double duty: its **high** nibble is the foreground colour for every character on the screen, its **low** nibble the background (and the border). White text on dark blue is one write to one register. And the sprite plane, present in every other mode, is simply gone: text mode is the one mode with no sprites at all. A word processor has no need of them, and their absence frees the beam's time and the programmer's attention.

### Setting it up

Bringing up text mode is five register writes. Here is the core of `textlib40`'s `TX40MD`, verified on the bench:

```asm
       LI   R0,1
       LI   R1,>D000        R1 = >D0: 16K on, display on, M1=1 (text)
       BL   @VWTR
       LI   R0,0
       LI   R1,0            R0 = >00: M3=0, no external video
       BL   @VWTR
       LI   R0,2
       LI   R1,0            R2 = >00 -> name table at >0000
       BL   @VWTR
       LI   R0,4
       LI   R1,>0100        R4 = >01 -> pattern table at >0800
       BL   @VWTR
       LI   R0,7
       LI   R1,>F400        R7 = >F4 -> white (F) on dark blue (4)
       BL   @VWTR
```

The one value worth staring at is **`R1 = >D0`**. Break it into bits: `1101 0000`. Bit 7 (`>80`) keeps the 16 KiB memory mode on. Bit 6 (`>40`) is the display-enable bit — the screen is *on*. Bit 4 (`>10`) is M1, and M1 alone among the mode bits is set: that is what makes this text mode. Bit 5 (interrupt enable) is off; we are not using the frame interrupt yet (Ch. 17). Registers 2 and 4 place the name and pattern tables exactly as in Graphics I; register 3 (the colour table) is not written because text mode has no colour table to point at. Register 7 carries the colours. That is the whole configuration.

> **Sidebar — Why forty? Why not eighty?** Forty columns is a television's honest answer. The 9918A clocks out 256 pixels of active picture per line, and a character you can actually read on a 1982 TV — through the blur of composite video and the softness of a consumer CRT — wants about six pixels of width. 256 ÷ 6 ≈ 42, and forty is the round number below it. Eighty columns would demand ~three-pixel characters, which on a television dissolve into gray hash; you need a monochrome monitor and a sharper video path to make eighty legible, which is exactly what the TI's successors and add-on cards provided. The 9938 (Ch. 18) added a genuine 80-column text mode for precisely the productivity market TI-Writer was courting on borrowed time. On a stock 4A hooked to the family TV, forty is not a failure of ambition. It is the resolution of the display talking back.

### Fonts that read at six pixels

The 6-pixel cell sounds like it should force a special font, and it is the reason Chapter 13's `textlib` and this chapter's `textlib40` can share one. The trick is to draw every glyph inside the **top five bits** of each byte — bits 7 through 3 — and leave bits 2, 1, and 0 clear. A glyph drawn that way occupies five columns with a one-pixel gap, comfortably inside six; and because the pixels it uses are all in the top six bits, text mode's truncation of the low two bits changes *nothing*. The same eight bytes render as an 8-wide cell in Graphics I and a 6-wide cell in text mode, and the letter looks the same in both.

`textlib40` ships a full uppercase font on this principle — A through Z and 0 through 9, thirty-six glyphs, each eight bytes. The letter A is the same four words we met in Chapter 13:

```asm
F40LET DATA >7088,>88F8,>8888,>8800    'A'
```

Read the high bits of each byte down the column and the shape appears: `.XXX.` / `X...X` / `X...X` / `XXXXX` / `X...X` / `X...X` / `X...X`. Every set pixel lives in bits 7–3; the low two bits of every byte are zero. We can *prove* the font renders correctly rather than merely assert it, because BENCH99 gained a pixel-level oracle for this chapter: the `pixels` command renders the actual 256×192 picture the beam would draw and prints each pixel as its palette-index hex digit. Load the `textlib40` self-test, which prints the whole alphabet on the top row, and ask for the picture one pixel per character (`pixels 1`); the letters read straight back, white glyphs (`f`) on the blue field (`4`):

```text
2fff22ffff222fff22ffff22fffff2     A B C D E F, pixel rows 0-6
f222f2f222f2f222f2f222f2f22222     (each glyph six pixels wide;
f222f2f222f2f22222f222f2f22222      the '2' columns are the gaps
fffff2ffff22f22222f222f2ffff22      and the blue background)
f222f2f222f2f22222f222f2f22222
f222f2f222f2f222f2f222f2f22222
f222f2ffff222fff22ffff22fffff2
```

That is the font, drawn by the chip, not by us — the strongest evidence a graphics claim can have. (The background reads as `2`, not `4`, in the self-test's final frame because the test paints its pass/fail verdict into register 7 after printing; a green background means the read-back check passed. More on the verdict light in the lab.)

## 14.2 `textlib40`: A println for the Wide Screen

`textlib40` is to text mode what `textlib` (Ch. 13) is to Graphics I: the small, tested engine that turns "poke bytes into a name table" into "print a string at a cursor." It obeys the same calling convention (R-16), keeps its cursor in one reserved pad word (`CUR40` at `>8346`), and exposes the same six-verb interface, renamed so the two engines can coexist in one program:

| Routine | Does |
|---|---|
| `TX40MD` | set text mode, load the font, set colours, clear |
| `TX40CL` | clear the screen to spaces, home the cursor |
| `TX40LC` | move the cursor to (row, col) |
| `TX40PC` | put one character, advance (with carriage-return handling) |
| `TX40PS` | put a null-terminated string |
| `TX40HX` | put a 16-bit value as four hex digits |

Most of it is a straight transcription of `textlib` with 32 changed to 40. Two differences are worth the ink, because both come directly from the wider screen.

**Positioning cannot use a shift.** In Graphics I, the cell offset of (row, col) is `row × 32 + col`, and ×32 is a five-bit left shift — one instruction. In text mode the row is forty cells wide, and forty is not a power of two. `TX40LC` computes `row × 40` as `(row × 32) + (row × 8)` — two shifts and an add, still no multiply:

```asm
TX40LC MOV  R0,R2           keep a copy of row
       SLA  R0,5            row * 32
       SLA  R2,3            row * 8
       A    R2,R0           row * 40
       A    R1,R0           + col
       MOV  R0,@CUR40
```

**Carriage return needs a divide.** When `TX40PC` sees a carriage return (`>0D`), it must jump the cursor to the start of the *next* row. In Graphics I that is a bitmask — round the offset down to a multiple of 32 by clearing its low five bits — because 32 is a power of two. Forty is not, so there is no mask that finds the start of a row. The honest way to find "the largest multiple of 40 not exceeding the cursor" is to divide: `CUR40 ÷ 40` gives the row as the quotient and the column as the remainder, and the row start is the cursor minus that remainder. The 9900's `DIV` (Ch. 8) does it in one instruction:

```asm
TX40NL CLR  R2             R2:R3 = CUR40 as a 32-bit dividend
       MOV  @CUR40,R3
       DIV  @F40W,R2       R2 = row (quotient), R3 = column (remainder)
       MOV  @CUR40,R0
       S    R3,R0          R0 = start of this row (CUR40 - column)
       AI   R0,40          start of the next row
```

This is a small, honest illustration of a large truth: **powers of two are free and everything else costs an instruction or two.** Graphics I's thirty-two columns were chosen partly because thirty-two is `2⁵`; text mode's forty columns, chosen for the television, make you pay a shift here and a divide there. The costs are tiny — a divide happens once per newline, not once per character — but they are real, and noticing them is the difference between guessing at performance and knowing it.

The self-test (`textlib40.a99`) prints the alphabet, the digits, a pangram that exercises all twenty-six letters in one 39-character line — `PACK MY BOX WITH FIVE DOZEN LIQUOR JUGS` — and a hex value through `TX40HX`, then reads cell (0,0) back with `VSBR` and paints the verdict. On the bench the forty-column `screen` view (BENCH99 now sizes its dump to the mode) shows exactly what was written:

```text
|ABCDEFGHIJKLMNOPQRSTUVWXYZ              |
|0123456789                              |
|                                        |
|PACK MY BOX WITH FIVE DOZEN LIQUOR JUGS |
|                                        |
|HEX C0DE READS C0DE                     |
```

That last line is `TX40HX` proving itself: told the value `>C0DE`, it printed the four characters `C0DE`. `textlib40` joins `lib99` here as the second text engine, and the Chapter 42 productivity capstone — a real editor — is built on it.

> **Field Notes — TI-Writer's sideways screen.** The console's flagship word processor, TI-Writer, lived in text mode, and it faced the problem every 40-column editor faces: documents are wider than the screen. Its editor is remembered for handling this by *windowing* — the visible forty columns are a moving view onto a wider logical line (commonly cited as eighty columns of working width), and the view scrolls horizontally as the cursor travels past the right edge, the text sliding left under a fixed frame. The lesson for our own editor (Ch. 42) is that the screen width and the *document* width are two different numbers, and a text engine that conflates them can only ever edit what fits. `textlib40` deliberately separates the cursor (a screen position) from the text buffer (a memory structure we have not built yet), so the window can move independently of the words. The specific key bindings and formatter directives of TI-Writer are period detail we will not reproduce; the windowing idea is the part worth carrying forward.

## 14.3 Multicolor Mode: The Fat-Pixel Canvas

Multicolor mode is the VDP's other lean — away from characters entirely, toward a direct canvas of colour. It is the simplest graphics mode to *use* and one of the strangest to *understand*, because its simplicity for the programmer is bought with a genuinely odd arrangement in memory.

What you get is a grid of **64 × 48 fat pixels**, each a solid 4×4 block of screen pixels, each independently any of the fifteen colours or transparent. There is no font to design, no bitmap to lay out bit by bit: you name a block by its (x, y) coordinate and give it a colour. Sixty-four across, forty-eight down — 3,072 blocks — is a coarse canvas, but it is a *canvas*, and getting one at all out of a chip with no framebuffer is worth understanding.

### How 3,072 blocks fit in a name table

Here is the strangeness. Multicolor mode still has a name table, and it is the same 32×24 = 768 entries as Graphics I. But 768 is not 3,072. Where do the other blocks come from? From the pattern table, read in a way unique to this mode.

In multicolor mode the chip ignores the *bit* pattern of a character definition and reads its bytes as **colour pairs**: each byte is two nibbles, the high nibble the colour of a left 4×4 block, the low nibble the colour of a right 4×4 block. And it does not use all eight bytes of a definition for one cell. Instead, which two of the eight bytes it reads depends on the cell's vertical position: within each group of four character rows, row 0 of the group reads bytes 0–1 of its named definition, row 1 reads bytes 2–3, row 2 bytes 4–5, row 3 bytes 6–7. So four stacked character cells that name the *same* definition display its eight bytes as eight different fat-pixel rows.

The consequence is that to paint the whole screen you fill the name table so that each vertical group of four rows names a distinct run of definitions, and the 768 name entries end up pointing at 192 definitions × 8 bytes = 1,536 bytes of colour data — one flat canvas. `mcolib`'s `MCMODE` fills the name table with exactly the map that makes this work:

```asm
MCMNL  MOV  R3,R6
       SRL  R6,7           i >> 7   (which group of four character rows)
       SLA  R6,5           << 5     (* 32 name entries per group)
       MOV  R3,R7
       ANDI R7,31          i & 31   (the column)
       A    R7,R6          the name byte, 0..191
       SWPB R6
       MOVB R6,@VDPWD
```

for `i` from 0 to 767. It is worth doing once and never thinking about again — which is exactly why it lives in a library routine.

### From coordinate to byte: the `PLOT64` math

With the name table laid out, plotting a fat pixel is pure address arithmetic. Work it through, because deriving it is the point of this section. A fat pixel (x, y) with x in 0–63 and y in 0–47 lives, after the name-table map above, in the canvas byte at offset

```text
(y >> 3) * 256  +  (x >> 1) * 8  +  (y & 7)
```

and in the **high** nibble of that byte if x is even (the left block), the **low** nibble if x is odd. The three terms are the three questions "which group of eight fat-rows, which column of cells, which byte within the definition" — and the last one collapses, after the algebra, to simply `y & 7`. `mcolib`'s `MCPLOT` is that formula plus a read-modify-write, because changing one nibble means reading the byte, splicing in four bits, and writing it back:

```asm
       MOV  R1,R3
       SRL  R3,3
       SLA  R3,8           (y>>3) * 256
       MOV  R0,R4
       SRL  R4,1
       SLA  R4,3           (x>>1) * 8
       A    R4,R3
       MOV  R1,R4
       ANDI R4,7           y & 7
       A    R4,R3          R3 = canvas byte address
```

We can check the whole apparatus in one shot. The self-test paints sixteen vertical colour bars — for every x, colour = x ÷ 4 — by calling `MCPLOT` on all 3,072 blocks, and because multicolor is 64×48 and the bench's `pixels 4` samples the screen to exactly 64×48, one output character maps to one fat pixel. The bars read back precisely:

```text
|2222000022223333444455556666777788889999aaaabbbbccccddddeeeeffff|
```

Read it in groups of four. Bar 2 onward reads `2 3 4 5 … f` — colours 2 through 15, four blocks wide each, exactly as plotted. The first two groups are the mode teaching you two facts at once: bar 0 is colour 0, which is **transparent**, so it shows the backdrop (here green, `2`, because the test's pass-verdict has painted register 7); and bar 1 is colour 1, black, which the palette renders identically to transparent, so the pixel oracle prints it as `0`. Colour 0 is not a colour — it is a hole to the backdrop — and that single fact explains more multicolor (and sprite, Ch. 16) behaviour than any other.

### Who used it, and why so few

Multicolor mode is a museum piece, and it is worth being honest about why. It is *coarse*: 64×48 is a quarter of the linear resolution of the bitmap mode we reach in Chapter 15, and a fat pixel is a conspicuous 4×4 brick. It is *not* meaningfully cheaper than the alternatives for most art — 1,536 bytes of canvas is real, but Graphics I gives you crisper results for character-based scenes at similar cost, and when you truly need per-pixel freedom you want the bitmap. And it arrived on a machine whose sprite hardware (Ch. 16) already handled the moving, colourful objects a game most wanted, over a Graphics I background that looked better than fat pixels ever could.

So multicolor's niches are narrow and specific: a quick low-resolution plot or chart where legibility matters more than detail; a title screen or loading pattern where chunky colour is a deliberate aesthetic; a teaching example (like this one) where the absence of bitmaps makes the canvas idea legible. It is the mode you should *recognize* — so that when you meet it in someone's code or a magazine listing you know what those 4×4 bricks are — more than the mode you will reach for. Knowing it exists, and knowing exactly why you are not using it, is its own kind of competence.

## 14.4 Changing the Machine's Mind, Cleanly

Everything so far has assumed you set a mode once at startup. Real programs change modes while running — a game that shows a text high-score table between Graphics I rounds, a utility that flips to multicolor for a chart and back to text for its menu. Doing it *cleanly* — without the viewer seeing a flash of half-built garbage — is a discipline, and it is the most transferable skill in this chapter.

A mode change is not one write. It is four movements:

1. **Blank the display.** Clear the display-enable bit (bit 6 of register 1) so the beam paints pure backdrop. The screen goes to a solid colour; nothing you do to VRAM now is visible.
2. **Rewrite the mode and base registers.** Set the new mode bits (registers 0 and 1) and repoint the table-base registers (2 through 7) at the new mode's tables.
3. **Lay out the new mode's VRAM.** The tables mean different things now. Load the font and clear the name table for text; fill the multicolor name-table map and clear the canvas; whatever the incoming mode needs.
4. **Unblank.** Set the display-enable bit. The finished screen appears in one clean transition.

The reason to blank first is the beam. The VDP is reading VRAM sixty times a second whether or not you are ready; if you rewrite the mode bits and *then* spend a few milliseconds loading a font, the beam will happily draw those milliseconds — a smear of the old tables reinterpreted under the new mode, which is exactly the garbage flash blanking exists to prevent. Blank, build in the dark, reveal. It is the same instinct as double-buffering (Ch. 13, Ch. 17): never show work in progress.

The chapter's `carousel` demo does this three times a cycle, rotating Graphics I → text → multicolor forever, and its `BLANK` routine is the whole idea in five instructions:

```asm
BLANK  LI   R0,1
       LI   R1,>8000        R1 = >80: 16K on, enable OFF, all modes 0
       BL   @VWTR
```

Each phase then rebuilds its mode from that blanked state and unblanks as its final register write. On the bench you can stop at each phase and confirm the switch landed — `vdp` shows `R1=>E0` (Graphics I), then `>D0` (text), then `>C8` (multicolor), the mode bits marching through their three positions, and `screen`/`pixels` shows each mode's content built correctly.

> **Pitfalls — the three ways a mode switch goes wrong.**
> - **Forgetting to blank.** The switch "works" — the final screen is right — but every change flashes a frame of garbage. Intermittent, ugly, and invisible in single-step debugging where there is no beam; you only see it on real hardware or a beam-accurate emulator, running at speed.
> - **Forgetting to re-lay-out VRAM.** You set text mode over a name table still full of Graphics I character codes, and the screen fills with whatever those bytes happen to spell in the new font. The mode bits are only half the contract; the tables are the other half.
> - **Clobbering the 16K bit.** Register 1 holds the mode bits *and* the 16 KiB memory-enable bit (bit 7) *and* the display-enable bit. Write register 1 to change the mode and accidentally clear bit 7, and the VDP drops to its 4 KiB addressing — your tables above 4 KiB vanish. Every write to register 1 must preserve the bits you did not mean to change. This is why `BLANK` writes `>80`, not `>00`.

> **Sidebar — VRAM plans that survive a switch.** When a program switches modes often, it pays to lay its VRAM out so the modes do not fight. Text and Graphics I both want a name table and a pattern table; if you place them at the same addresses in both modes (name at `>0000`, pattern at `>0800`, as `textlib` and `textlib40` do), a switch between them need not move a single byte of font — only the mode bits and the colour arrangement change. Multicolor's canvas is a different shape and will overwrite whatever shares its addresses, so a program that flips between multicolor and text keeps their tables in disjoint regions and reloads only what changed. The general rule: **the tables a mode owns are a lease, not a deed.** Know which bytes each mode claims, and a switch becomes cheap.

## Lab 14 — The Mode Carousel, PLOT64, and a Wider Monitor

This chapter's lab is three artifacts, all in `code/ch14/`, all machine-verified on BENCH99.

**`textlib40` (`textlib40.inc` + `textlib40.a99`)** — the 40-column text engine and its 6-pixel font, added to `lib99`. Build and prove it:

```sh
libre99asm code/ch14/textlib40.a99 --format bin -o build/TL40C.bin --symbols build/tl40.map.json
```

Then on the bench: `load build/TL40C.bin`, `pc` to the entry, `x 30000`, and inspect with `screen` (the alphabet, digits, pangram, and hex read-out), `vdp` (`R7=>F2`, the pass verdict), and `pixels 1` (the font, drawn by the chip). A green verdict means cell (0,0) read back as `A`.

**`mcolib` — the PLOT64 mini-API (`mcolib.inc` + `mcolib.a99`)** — multicolor mode as `MCMODE` / `MCCLS` / `MCPLOT`. Its self-test paints sixteen colour bars; verify with `pixels 4`, where one character is one fat pixel and the bars read `…2233445566…ffff`. The exercise below asks you to plot something of your own.

**`carousel.a99`** — the three-mode rotation of §14.4, demonstrating the blank/reconfigure/relayout/unblank discipline. Stop at `PH1DON`, `PH2DON`, `PH3DON` (a breakpoint with `u`) to confirm each mode built cleanly. Its per-phase hold is a simple cycle-delay so the whole program is deterministic on the bare bench; a shipping carousel would hold each mode for a fixed number of *frames*, which needs the 60 Hz interrupt heartbeat we build a timer around in Chapter 17.

**`monitor40.a99`** — Chapter 13's `MONITOR99` hex viewer, ported to the 40-column screen. The port is nearly free: swap the `textlib` verbs for `textlib40`'s and widen the row from four words to six. Run it and cross-check its dump of console ROM against `m 0000 90` — they match to the byte, a full-stack proof that the wider engine is faithful.

## Exercises

**14.1** ✦ Text mode's register 7 holds foreground in the high nibble, background in the low. Give the `R1` value for `VWTR` that sets **black text on a light-yellow screen**. (Colour indices are in App. D; light yellow is 11, black is 1.)

**14.2** ✦ Why does `textlib40`'s `TX40LC` compute `row × 40` as `(row × 32) + (row × 8)` instead of a single multiply? Give both the correctness reason and the performance reason.

**14.3** ✦✦ Add a `TX40CH` routine to `textlib40` that changes the whole screen's colours by rewriting register 7, *without* clearing or redrawing. Confirm on the bench that the text stays and only its colours change. What does this tell you about where text-mode colour lives, compared to Graphics I?

**14.4** ✦✦ Extend `mcolib` with `MCHLIN` (draw a horizontal line of fat pixels: x from x0 to x1 at a fixed y, in one colour) and `MCVLIN` (vertical). Use them to draw a box outline. Verify with `pixels 4`.

**14.5** ✦✦ The multicolor self-test plots all 3,072 blocks through `MCPLOT`, one nibble-splicing read-modify-write each. `MCCLS` floods the whole canvas far faster. Explain why, in terms of VRAM accesses per block, and estimate the ratio.

**14.6** ✦✦✦ Write a `plasma`-style demo in multicolor: fill the 64×48 canvas from a function `colour(x, y) = (x + y) & 15` (or your own), timing how long the fill takes with `cycles`. Then reduce the per-pixel cost by writing whole bytes (two horizontally adjacent blocks) at once where their colours are known together, and measure the speedup. This is the §14.3 read-modify-write cost made concrete.

**14.7** ✦✦✦ Modify `carousel.a99` to add a fourth phase that stays in text mode but flips register 7 through several colour pairs on a timer, so the *same* text appears in changing colours. Then deliberately introduce the "forgot to blank" bug in one switch and describe — or, on real hardware or the desktop emulator, observe — what the viewer sees that the bench's single-step view hides.

## Further Reading

- *TMS9918A / TMS9928A / TMS9929A Video Display Processors Data Manual*, Texas Instruments — §2 on the mode-select bits and the multicolor and text table formats; the authority behind this chapter's tables.
- *Editor/Assembler Manual*, Texas Instruments — the VDP mode discussion and the standard register conventions text and multicolor programs follow.
- Chapter 13 (Graphics I and `textlib`) — the mode this chapter departs from, and the text engine `textlib40` mirrors.
- Chapter 15 (Bitmap Mode) — where the VDP's graphics ambitions actually lead; multicolor is the timid first step toward it.
- Chapter 18 (Advanced and Modern VDP) — the 9938's genuine 80-column text mode, the productivity market's answer to §14.1's forty.
- Chapter 42 (the productivity capstone) — the editor `textlib40` was built to support.

## Summary

The VDP is four display modes, selected by three mode bits, and this chapter covered the two that flank Graphics I. **Text mode** (M1) is the productivity contract: forty columns of 6-pixel cells (the chip shows only the top six bits of each pattern byte), one foreground/background pair in register 7 instead of a colour table, and no sprites — configured by `R1=>D0` and four more register writes. A font drawn five pixels wide (bits 7–3) renders identically in Graphics I and text mode, which is why one font serves both; `textlib40` is the 40-column text engine built on that fact, differing from Chapter 13's `textlib` only where forty's not being a power of two forces a shift-pair for positioning and a divide for carriage return. **Multicolor mode** (M2) turns the screen into a 64×48 grid of fat 4×4 pixels, each any colour or transparent; its 768-entry name table is filled with a fixed map so the 3,072 blocks land in a flat 1,536-byte canvas, and the `PLOT64` address of block (x, y) is `(y>>3)*256 + (x>>1)*8 + (y&7)`, high nibble for even x. Colour 0 is transparent everywhere — a hole to the backdrop, not a colour. Almost no one shipped multicolor, because it is coarse and the bitmap mode of Chapter 15 does real graphics better; recognizing it, and knowing why you are not using it, is the competence to take away. Finally, changing modes at runtime is a four-step discipline — blank, rewrite registers, re-lay-out VRAM, unblank — that keeps the beam from ever drawing a half-built screen, with the standing hazards of forgetting to blank, forgetting to rebuild the tables, and clobbering the 16 KiB bit in register 1. Every value, layout, and picture here was verified on BENCH99, whose new `pixels` view renders the chip's actual output as the pixel-level oracle for the rest of Part III.
