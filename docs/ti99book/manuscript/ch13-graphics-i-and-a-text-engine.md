# Chapter 13 — Graphics I Mode and a Real Text Engine

*This machine has no `print`. It has a grid of tiles and a chip that draws them. To put a word on
the screen — a score, a prompt, a line of a memory dump — you must build the output routine that
every other language handed you for free. We build it here, once, call it `textlib`, and use it for
the rest of the book. Along the way we meet the constraint that gives TI graphics their unmistakable
look: color comes eight characters at a time.*

<!-- Part III — The Video Display Processor · target ≈24 pp -->
<!-- STATUS: DRAFTED (session 6, 2026-07-06) — pending review passes. textlib.a99 + monitor99.a99 assemble via sh verify.sh (34 sources) and are machine-verified on BENCH99 at commit 31417ef: textlib self-test green (R1=>E0 display enabled, R7=>02), `screen` shows "C0DE"/"DEAD", font glyphs loaded (vram 0980/0A08). MONITOR99 green, hex dump matches memory exactly (row 0 `0000 83E0 0024 83C0 0900` = m 0000, the Ch. 9 vectors); ~282,858 cyc (~5.7 frames) full setup+dump. vdplib refactored into vdplib.inc (routines) + vdplib.a99 (harness) — the lib99 composition pattern (README updated); textlib.inc composes on it. Two bugs found+fixed in verification: TXMODE wrote reg 0 twice (display left blanked — vdp oracle caught it); VSBR returns the byte in R1 high (compare with CB not CI). -->
<!-- SPEC: 00-master-outline.md, section "### Chapter 13 —" (lines 318–327 in outline v1.1). That bullet list is this chapter's contract. -->

## The Word You Cannot Print

The first program anyone writes prints a word. `print("hello")`, `System.out.println`,
`printf`, `cout <<` — every language on earth gives you a line of text on the screen before it gives
you anything else, and it does so on the very first page, because output is how a program becomes
real to the person writing it. So the newcomer to TI assembly, having wrestled a byte across to the
VDP in Chapter 12, reasonably expected that somewhere there was a way to just *say a word* — and went
looking for it with the confidence of someone who has never once had to implement `print`.

There was nothing. No print instruction, no output routine in a library he could call, no operating
system service that took a string and a position. The console ROM had such routines, buried, meant
for its own use and reachable only through conventions he had not learned yet (Chapter 23). What he
had, at the bare-metal level this book starts from, was Chapter 12: a name table of 768 cells, a
pattern table of shapes, and the knowledge that to show the letter H he had to (a) make sure a
pattern somewhere *was* the shape of an H, and (b) write that pattern's number into the cell where he
wanted it. To print "HELLO" he would place five bytes in five cells, having first ensured that five
patterns held the five letters' shapes. There was no line, no cursor, no wrap, no newline — those are
not things the hardware has. They are things you *build*, out of tile placement, and then they feel
like they were always there.

He built them, crudely, and got "HELLO" onto the screen, and felt the small triumph the whole
chapter is organized around. Then he tried to make it red, and met the wall that every TI programmer
meets and that gives the machine's graphics their particular period look. He could not color one
word. In this mode the color of a character is not a property of its cell or even of the character
itself — it is a property of the character's *pattern number*, shared across a **group of eight
consecutive patterns**. His H, E, L, and O had pattern numbers scattered across several groups, and
changing the color of one group to red recolored not just his letters but every other character that
happened to share a group — punctuation, digits, whatever. To make one word red he would have had to
plan which patterns his letters used so they fell in a group he could own. The constraint was not a
bug. It was the chip trading per-character color for a color table thirty-two bytes long instead of
seven hundred and sixty-eight, and once he understood *that* trade he could see it in every TI game
he had ever played — the reason sprites carried the bright independent colors and the background
came in broad same-colored swaths.

This chapter is that education, made systematic and turned into a tool. We will lay out Graphics I
precisely, build `textlib` — the print statement the machine never had — join it to the number
formatting of Chapter 8 to make a debugging dashboard, master the eight-character color rule instead
of fighting it, and end by building `MONITOR99`, a live hex memory viewer you will actually use for
the rest of the course. By the last page, "print a word" will be one line of your own code, and you
will know exactly what every one of those other languages was doing all along.

---

## What You Will Learn

By the end of this chapter you will be able to:

- Lay out and enable Graphics I mode from scratch: the 32×24 name table, the 256-entry pattern table,
  and the 32-entry color table — and explain the eight-character color rule and why it exists.
- Load a character set into the pattern table, and describe the two ways to get a font (ship your own
  or borrow the console's) and the famous "lowercase" story.
- Build and use a text engine — `textlib` — with a cursor, character and string output, newline and
  wrap, clear, and positioning: a `println` for a machine that never had one.
- Compose lib99 modules: build `textlib` on top of `vdplib` using the include architecture, and
  explain why a shared module must be an `.inc`.
- Format numbers to the screen — hex now, decimal by joining Chapter 8's math — to make a debugging
  dashboard.
- Use the color table deliberately: palettes, inverse video, and highlight-by-group tricks within
  the eight-character rule; compose borders, panels, and a status line.
- Flip between two screens by changing one register (name-table double buffering) for tear-free
  full-screen updates.

## The Bridge: You Have Never Had to Build `print` Before

To a programmer in 2026, `print` is bedrock — so far below the waterline of what you think about that
it feels like a property of computers rather than a program someone wrote. It is a program someone
wrote. Underneath your language's print statement is a runtime that maintains a cursor position,
tracks the current line, interprets `\n` as "move to the start of the next line," scrolls the buffer
when the cursor falls off the bottom, translates each character code into a glyph from a font, and
copies that glyph's pixels into the console's framebuffer — and underneath *that* is an operating
system doing terminal emulation, and underneath that a windowing system, and so on down. You have
stood on this tower your whole career without seeing a single floor of it.

This chapter is the ground floor, and it is small enough to build in an afternoon. The cursor is a
number you keep in a pad word. "Move to the next line" is arithmetic on that number. A font is a
table you load once. "Put a character" is: write the character's code into the name-table cell the
cursor points at, then advance the cursor. That is the whole of `print`, minus the operating system —
and building it yourself, on a machine with no floors beneath you, is the fastest way to understand
what all those floors were doing. When you finish `textlib` and call it to put a word on the screen,
you will be using a print statement you can read end to end, and the mystery will be gone for good.

## 13.1 The Shape of Graphics I

Graphics I is the mode the console boots into and the mode this book defaults to. It is the
table-driven model of Chapter 12 in its most straightforward form, and its three tables are laid out
like this:

The **name table** is 768 bytes — 32 columns by 24 rows — one byte per screen cell, holding the
*pattern number* (0–255) to display in that cell. Writing a byte here places a tile. It is the table
you change constantly; everything the text engine does is, at bottom, writing bytes into the name
table.

The **pattern table** is 2,048 bytes — 256 patterns of 8 bytes each — where each pattern is an 8×8
bitmap, one byte per row, one bit per pixel (a 1-bit is foreground, a 0-bit background). This is the
font, or the tileset; you load it once and it defines what every pattern number *looks like*. Because
there are exactly 256 patterns and a byte names one, every possible cell value has a defined shape.

The **color table** is where Graphics I makes its defining trade. It is only **32 bytes**, and each
byte holds a foreground/background color pair (high nibble foreground, low nibble background) for a
*group of eight consecutive pattern numbers*: color-table byte 0 colors patterns 0–7, byte 1 colors
patterns 8–15, and so on up to byte 31 for patterns 248–255. **Color is a property of the pattern
number, eight at a time.** This is why the newcomer could not make one word red: two characters share
a color if and only if their pattern numbers fall in the same group of eight — regardless of where
they sit on screen, regardless of anything else.

Make that concrete with the very word he tried. Because `textlib` uses each character's ASCII code as
its pattern number, the letters of "HELLO" carry the pattern numbers of their ASCII codes: H is `>48`
(72), E is `>45` (69), L is `>4C` (76), and O is `>4F` (79). Divide each by eight to find its color
group: E lands in group 8 (patterns 64–71), while H, L, L, and O all land in group 9 (patterns
72–79). So "HELLO" straddles two color groups, 8 and 9. To turn it red you must set both color-table
bytes 8 and 9 to red — and that recolors *every* pattern in the range 64–79, which is ASCII `>40`
through `>4F`: the characters `@ A B C D E F G H I J K L M N O`. Your red "HELLO" drags the whole
first half of the alphabet red with it, wherever those letters appear on the screen. There is no
color you can give H, E, L, and O that you do not also give to A, B, C, and the rest of their groups.
*That* is the eight-character rule in the concrete, and it is why the escape — planning your font so
the things you want one color share a group — is the only way through.

Why eight at a time? Do the arithmetic the chip's designers did. Per-character color — a fg/bg pair
for each of the 768 cells — would cost 768 bytes of a fresh color table *and* a per-cell color write
every time you moved a tile. The group-of-eight scheme costs 32 bytes and never changes as tiles
move, because color follows the pattern, not the position. It is the same kind of compression that
justified the whole table model in Chapter 12, applied to color: give up independence you rarely need
(most of a screen is one or two colors) to save memory and bandwidth you always need. Master it and
it stops being a limit and becomes a design input — you *assign pattern numbers so that things you
want the same color share a group*, which is exactly the technique of §13.5. The constraint that
defines the TI look is, once you flip it around, a layout decision you get to make.

## 13.2 Character Sets: Where a Font Comes From

A pattern table is a font when its patterns are letter shapes. There are two ways to get one, and the
chapter's `textlib` demonstrates the first while pointing at the second.

**Ship your own.** Define the glyphs as data in your program and copy them into the pattern table at
startup with `VMBW` (Chapter 12). This is what `textlib` does: `code/ch13/textlib.inc` carries a
compact set of 5×7 glyphs for the sixteen hexadecimal digits, and `TXMODE` loads the digit glyphs to
`pattern[>30]` (ASCII `'0'`) and the letter glyphs to `pattern[>41]` (ASCII `'A'`), so that a
character's ASCII code doubles as its pattern number and the console's `screen` view — which reads the
name table as ASCII — shows exactly what you wrote. A full printable-ASCII font ships the identical
way; it is only more glyphs, and hand-drawing ninety-six of them is tedious but not deep. The book
ships the hex digits because the Lab, `MONITOR99`, needs exactly those, and because sixteen glyphs are
enough to prove the machinery without burying the chapter in bitmaps.

**Borrow the console's.** The TI-99/4A already has a font — the small uppercase set TI BASIC and the
console menus use — sitting in GROM, the graphics-language ROM we do not open until Chapter 25. The
standard professional move is to copy that font from GROM into your pattern table at startup, getting
a complete, legible character set for the cost of a copy loop and none of the glyph-drawing. We defer
it only because reading GROM is its own protocol (Chapter 25's subject); when you have it, borrowing
the console font is usually the right choice, and Chapter 25's Lab does exactly that.

A word on the **"lowercase" story**, because it is a piece of TI folklore worth knowing. The
console's built-in font has no true lowercase — the "lowercase" letters are small capitals, uppercase
shapes shrunk into the lower half of the cell, because a full lowercase set with descenders would not
fit the memory and the 8×8 cell the designers budgeted. For years, TI text had that unmistakable
small-caps look, and adding real lowercase — by shipping your own font — was one of the first things
serious software (TI-Writer among them) did. It is a small detail that instantly dates a TI screen,
and now you know why: lowercase was a font you had to bring yourself.

## 13.3 Building `textlib`, and How lib99 Modules Compose

`textlib` is the print statement, and building it surfaces the first real question of library
architecture: a module that is *used by another module* cannot look like the single-file `mathlib`
of Chapter 8. `mathlib` was one file with its own `START` self-test, which is perfect for a module
nothing else builds on — but `textlib` builds on `vdplib`, and two files each with a `START` cannot be
assembled together (two entry points). So Chapter 13 refines the lib99 pattern (the `code/lib99/`
README records it): a module is **two files** — a routines-only **`vdplib.inc`** that any consumer
`COPY`s, and a **`vdplib.a99`** self-test harness that includes the `.inc` and adds the `START`. The
routines live once, in the `.inc`; the harness proves them standalone; consumers pull in the `.inc`.
We refactor `vdplib` into this shape here, and `textlib` is the first module to consume another:

```
       COPY '../ch11/equates.inc'      ports + colors        (dependencies first,
       COPY '../ch12/vdplib.inc'       the VDP core           in order — one flat
       COPY 'textlib.inc'              the text engine        COPY-namespace)
START  ...                             the consumer's code
```

The engine itself is small, because Chapter 12 did the hard part. Its state is one pad word,
`CURPOS`, the cursor as a linear cell offset 0–767. Its core is `TXPUTC`: write the character to the
name-table cell at `CURPOS` (via `vdplib`'s `VSBW`), then advance `CURPOS` by one, wrapping at 768,
with a carriage return (`>0D`) special-cased to jump to the start of the next row by rounding the
cursor down to a multiple of 32 and adding 32. `TXPUTS` walks a null-terminated string calling
`TXPUTC` — keeping its string pointer on the R10 stack across each call, because per the calling
convention (R-16) a called routine may clobber the scratch registers, so a value that must survive a
call belongs on the stack, not in a register you hope stays untouched. `TXLOC` sets `CURPOS` from a
row and column; `TXCLS` fills the name table with spaces and homes the cursor; `TXMODE` brings the
whole mode up — registers, font, colors, clear. On the bench the self-test comes up green: it prints
`C0DE` with the hex formatter and `DEAD` with the string writer, then reads the name table back to
confirm the characters landed, and `screen` shows the two words exactly where they were placed.

Two bugs found while building it are worth keeping, because they are the two bugs *you* will write.
The first: `TXMODE`'s first register write named register 0 where it meant register 1, so the mode
byte that enables the display went to the wrong register and the screen stayed blanked — invisible in
the `screen` view (which reads the name table regardless of whether the display is on) but caught
instantly by the `vdp` command showing `R1 = >00` instead of `>E0`. Cross-checking two oracles found
what one would have hidden. The second: `VSBR` returns its byte in the *high* half of the register
with the low half unspecified, so a word compare (`CI`) against the expected character failed until it
became a byte compare (`CB`). Both are exactly the kind of off-by-a-detail that the measure-first
discipline turns from an afternoon of confusion into a two-minute fix.

## 13.4 Numbers on the Screen: The Debugging Dashboard

A print statement that can only print letters is half a tool. The other half is numbers, and it is
where `textlib` joins hands with the arithmetic of Chapter 8. `textlib`'s `TXHEX` takes a 16-bit value
and prints it as four hex digits: it walks the value a nibble at a time from the top, converts each
nibble to its ASCII digit (`0`–`9` by adding `>30`, `A`–`F` by adding seven more), and calls `TXPUTC`.
Decimal output is the same idea wearing Chapter 8's clothes — the divide-by-ten chain of `mathlib`'s
`U16DEC` produces the digits, and `textlib` puts them on the screen — so a `TXDEC` is a short bridge
between two libraries you already have (and one of this chapter's exercises).

This is the **debugging dashboard** pattern, and it changes how you work on this machine. Until now,
watching a value meant reading it off BENCH99 with the `m` command — indispensable, but outside the
program. With `TXHEX` you can make the *program itself* show you its state: put a register on the
screen every frame, watch a pointer walk, display a score. The Lab, `MONITOR99`, is this pattern taken
to its natural end — a whole memory viewer — but the smallest version, a single `TXHEX` of a variable
you are unsure about, is a debugging technique you will reach for constantly, and it is the direct
descendant of the border-flash instrumentation of Chapter 11: make the machine tell you what it is
doing, in a form you can read at a glance.

## 13.5 Color Strategy: Making the Eight-Character Rule Work for You

The eight-character color rule (§13.1) is not something to endure; it is something to *plan around*,
and a handful of standard moves turn it from a constraint into a palette.

The base move is **uniform text**: set all 32 color-table bytes to the same fg/bg pair, and every
character is that color, whatever its pattern number — which is what `TXMODE` does (white on black,
`>F1`, in all 32 entries) so that ordinary text just works. From there you buy variety by making
color-table entries *differ*, remembering always that you are coloring pattern-number groups, not
positions. **Highlight by group**: if you lay out your font so that a set of characters you want to
recolor together shares a group of eight pattern numbers, one color-table write recolors exactly them
— a status line in a different color, a set of special symbols that glow. **Inverse video** — dark
text on a light bar, the classic way to show a selection or a header — is a color-table entry with the
nibbles swapped (`>1F` instead of `>F1`), applied to a group whose characters you have arranged to use
for the highlighted text. And a full **palette** for a game's tiles is simply a deliberate assignment
of tile pattern-numbers to color groups: put the sky tiles in one group, the ground tiles in another,
and you can recolor sky and ground independently while the name table — the expensive, constantly
changing table — never carries color at all. The discipline is always the same: **decide which things
share a color, and give them pattern numbers in the same group.** Do that at design time and the
eight-character rule costs you nothing; ignore it and you will fight it forever.

## 13.6 Composing a Screen: Borders, Panels, and a Status Line

With text output in hand, a screen becomes a composition rather than a field of characters, and a few
conventions carry through the whole book. A **border** is a frame of a chosen pattern drawn around the
edge cells — the outermost row, column, last row, last column — with the interior left for content; it
costs a few short loops and instantly makes a screen look designed rather than dumped. A **panel** is
a rectangular sub-region you treat as its own little screen — clear it, position within it, write to it
— which is how you build the side-by-side layouts of a game (play field beside a score panel) or a
tool (a hex column beside an ASCII column, as `MONITOR99` hints at). And a **status line** — the book
adopts the bottom row, row 23, as a reserved status line from here on — is a panel one row tall where
running programs report state: a mode, a filename, an error, a prompt. Reserving it once, as a
convention, means every tool and game in the book has somewhere to talk to the user without redesigning
the screen each time. None of these are hardware features; they are agreements you make with yourself,
built on `textlib`, and they are what separate a screen that works from a screen someone wants to look
at.

## 13.7 Two Screens, One Register: Name-Table Double Buffering

There is a visible problem lurking in everything above. When you rewrite a screen in place — clear it
and redraw — the VDP is drawing *continuously* (Chapter 12), so for the fraction of a second while your
`textlib` calls are updating the name table, the chip is showing the screen *half-updated*: old content
and new content at once, a flicker or a tear. For a static tool this is invisible; for anything that
redraws often it is ugly.

The cure is one of the prettiest tricks the table model affords, and it is nearly free. Recall from
Chapter 12 that register R2 holds the *name-table base* — where in VRAM the chip looks for the screen —
and that moving a table is a single register write. So keep **two** name tables in VRAM, at two
different 1 KiB-aligned addresses, and let R2 choose which one the VDP is displaying. Draw the next
frame entirely into the *other* one — the one that is not on screen, where the user cannot see the
work in progress — and when it is complete, write R2 to flip to it. The switch is atomic and instant:
one moment the old screen is showing, the next the new one is, with no half-drawn state ever visible,
because you only ever *display* a finished buffer. This is **page flipping**, the same technique a
modern engine calls double buffering, and on the VDP it costs one 768-byte spare buffer and one
register write per frame. It is how full-screen updates stay tear-free, and Chapter 17 builds it into
the game loop; for now, know that "flip the screen with one register" is a tool you have, and that it
falls directly out of R2 being a movable pointer.

> **Sidebar — Why Thirty-Two Columns?** A modern terminal is 80 columns; even the humblest 1980s
> business machine managed 40. The TI shows 32 columns of text in Graphics I, and the reason is the
> television. The VDP was designed to drive a **home TV** through the RF connector, not a monitor, and
> a TV's picture is soft and its edges are lost to *overscan* — the outer border of the signal that
> the tube physically cannot show, hidden behind the bezel. Thirty-two columns of 8-pixel cells is 256
> pixels across, a width chosen so that every column lands safely inside the overscan on a typical
> set, legibly, without characters crawling off the edge. Text mode (Chapter 14) squeezes to 40 columns
> by narrowing the cells to 6 pixels — the "40-column temptation" — and it works, but its thinner
> characters are the first to blur on a marginal TV, which is exactly the trade: more text, less
> margin. The 4A chose legibility on the machine most owners actually had — a set in the den — over
> density on a monitor most did not.

## Lab 13 — `MONITOR99`: A Live Memory Viewer

The Lab turns `textlib` into a tool you will use for the rest of the course: `MONITOR99`, a hex memory
viewer. It is barely any code — the whole of `code/ch13/monitor99.a99` fits on a page — because it is
`textlib` doing the work. It brings up the screen with `TXMODE`, then loops eight rows, and for each
row positions the cursor with `TXLOC`, prints the address with `TXHEX`, and prints four consecutive
words of memory with four more `TXHEX` calls, separated by spaces:

```
MROW   MOV  R7,R0            cursor to (row R7, col 0)
       CLR  R1
       BL   @TXLOC
       MOV  R6,R1            print the running address ...
       BL   @TXHEX
       ...
MWORD  MOV  *R6+,R1          ... then four words from memory
       BL   @TXHEX
       ...
```

Pointed at console ROM `>0000`, it produces, on the bench, exactly this:

```text
0000 83E0 0024 83C0 0900
0008 83C0 0A92 30AA 0460
0010 02B2 0008 1E00 0460
...
```

— and every value is right: row zero reads `83E0 0024 83C0 0900`, which is the reset vector and the
level-1 interrupt vector you read cold with the `m` command back in Chapter 9, now displayed *by a
program on the TI's own screen*. The verification is a cross-check between two independent views:
`MONITOR99` computes and displays the dump, and BENCH99's `m 0000` reads the same memory directly, and
they agree byte for byte. The whole run — bring up the mode, load the font, clear 768 cells, format
and place forty hex numbers — costs about 283,000 cycles, roughly six frames, almost all of it the
one-time screen setup; the dump itself is cheap. Point `MONITOR99`'s address register at any region and
it shows you that region, which is precisely the "genuinely useful tool" the outline promised: from
here on, when you want to see what is in memory, you have a way that runs on the machine itself.

## Exercises

**✦ Warm-ups.**

1. In Graphics I, character `'A'` (ASCII `>41`) and character `'I'` (ASCII `>49`) — do they share a
   color? What about `'A'` and `'H'` (`>48`)? Show the group arithmetic.
2. `textlib` keeps the cursor as a single number 0–767. Write the two lines that convert a
   `(row, col)` pair to that number, and the arithmetic that turns a carriage return into "start of the
   next row."
3. Why must `TXPUTS` save its string pointer on the stack across each `TXPUTC` call, rather than just
   leaving it in a register? Cite the calling convention (R-16).

**✦✦ Consolidation.**

4. Write `TXDEC`, decimal number output, by joining `mathlib`'s `U16DEC` (Chapter 8) to `textlib`'s
   `TXPUTC`. Print the value 12345 and confirm it on the bench with `screen`.
5. Extend `MONITOR99` to show an ASCII column beside the hex (like BENCH99's own `m`): for each byte,
   print it as a character if it is printable (`>20`–`>7E`) and a `.` otherwise. Which color-table
   consideration, if any, applies?
6. `TXMODE`'s first-draft bug wrote the enable byte to register 0 instead of register 1, and the
   `screen` command did not catch it. Explain precisely why `screen` was blind to the bug and the `vdp`
   command was not, and state the general lesson about verification oracles.

**✦✦✦ Extensions.**

7. Implement name-table double buffering (§13.7): reserve a second name table at `>0400`, and write a
   demo that fills buffer A with one pattern, buffer B with another, and flips between them by writing
   R2. Verify with `screen` that after a flip it shows the other buffer (the `screen` command reads R2
   to find the name base).
8. Design a font layout for a game with a blue sky, green ground, and white text, using the
   eight-character color rule so that each can be recolored by a single color-table write. Give the
   pattern-number groups you would assign to each, and explain why the name table never has to change
   when you recolor.

## Further Reading

- *TMS9918A Data Manual*, Texas Instruments — the Graphics I mode description, the color-table
  organization, and the exact meaning of the mode bits, per Chapter 12's Field Notes.
- *Editor/Assembler Manual*, Texas Instruments — the console's own screen and character conventions,
  and the built-in character-set codes, the ancestors of `textlib`'s ASCII-as-pattern-number scheme.
- The project VDP source, `crates/libre99-core/src/vdp.rs`, function `render_graphics1` — the exact
  name→pattern→color lookup this chapter's tables feed, in forty lines of readable Rust.
- TI-Writer and other period software that shipped its own lowercase font — the practical answer to
  the small-caps console font, and a preview of the text-mode editor work in Chapter 42.

## Summary

- **Graphics I** is three tables: the **name table** (768 bytes, one pattern number per 32×24 cell),
  the **pattern table** (256 × 8-byte 8×8 glyphs — the font), and the **color table** (32 bytes, one
  fg/bg pair per *group of eight* pattern numbers). Color follows the pattern number, eight at a time —
  the constraint that defines the TI look, and a 32-byte-vs-768-byte compression trade you plan around
  by giving same-colored things pattern numbers in the same group.
- A **font** is a loaded pattern table. Ship your own (define glyphs, `VMBW` them in — `textlib`'s hex
  digits) or borrow the console's from GROM (Ch. 25). The console font is small-caps — no true
  lowercase — which serious software replaced by shipping its own.
- **`textlib`** is the print statement the machine lacks: `CURPOS` (cursor as a 0–767 cell offset),
  `TXPUTC`/`TXPUTS` (character/string, with CR and wrap), `TXLOC`/`TXCLS`/`TXMODE`, and `TXHEX` for
  numbers. Verified green; `screen` shows the output. Building it establishes the **lib99 composition
  pattern**: a shared module is a routines-only `.inc` plus a `.a99` self-test harness, `COPY`d by
  consumers in dependency order (`vdplib` refactored into this shape here).
- **Numbers on screen** (`TXHEX`; `TXDEC` via `mathlib`) make a **debugging dashboard** — the program
  shows you its own state, the descendant of Chapter 11's instrumentation. **Color strategy**:
  uniform text, highlight-by-group, inverse video (swap the nibbles), palettes by pattern-group
  assignment. **Screen composition**: borders, panels, and a reserved status line (row 23, book-wide).
- **Double buffering** (§13.7): keep two name tables, draw into the hidden one, flip R2 to display it —
  atomic, tear-free, one register write; the page-flip the game loop (Ch. 17) will use. Text is 32
  columns because the VDP drives a **TV** and 256 pixels fits inside overscan (the sidebar).
- **Lab `MONITOR99`**: a live hex memory viewer in a page of `textlib` calls; its dump of `>0000`
  matches `m 0000` byte for byte (the Ch. 9 vectors, shown on the TI's own screen). Machine-verified
  at commit 31417ef. Seeds: 40-column `textlib40` (Ch. 14), the bitmap canvas (Ch. 15), sprites over
  a Graphics I background (Ch. 16), the game loop and real double buffering (Ch. 17).
