# TITRIS — a falling-blocks puzzle for the TI-99/4A, in TMS9900 assembly

**Titris** is a small, playable falling-blocks puzzle game for the TI-99/4A — and
a **proof of concept for this
project's emulator and its from-scratch assembler**. Every byte of this cartridge
was written in TI Editor/Assembler-style source, assembled by the repo's own
`libre99asm` (`crates/libre99-asm`), and booted on the repo's own emulated console
(`crates/libre99-core`). Nothing external touched it: source → `libre99asm` → `.ctg` →
emulator, end to end.

It is deliberately simple (it *is* a POC), but it is a real game: a graphical
title screen, the seven tetrominoes with rotation, gravity, line clears, a
**U-shaped well that is open at the top and *narrows* as you level up** (locked
blocks past the new edge are trimmed), faint per-column alignment guides, a
next-piece preview, classic line-clear scoring, a game-over summary, color,
sound, SRS rotation with wall kicks, and both keyboard and joystick control.

```
   ===== title screen =====        ============ gameplay ============
                                     |                    |  NEXT
    ### ### ### ##  ### ###          |                    |   ##
     #   #   #  # #  #  #             |        ##          |   ##
     #   #   #  ##   #  ###           |        ##          |
     #   #   #  # #  #    #           |                    |  SCORE
     #  ###  #  # # ### ###           |                    |  00120
                                      |                    |  NEXT
       FOR THE TI-99                  |        ██          |  LEVEL AT
                                      |        ██          |  01000
    PRESS ANY KEY TO PLAY            U-shaped well, no top  |
    H OR AID FOR HELP                                       |
                                      +--------------------+  LEVEL  00

   The well starts twice as wide and one cell taller per the original, is open
   at the top, and SHRINKS one column from the right every level (every 1000
   points). Blocks past the new edge are trimmed away.

   controls:  ← / →  move    ↓  soft drop    ↑ or X  rotate CW
              Z  rotate CCW    SPACE / Right-Alt  hard drop
              (the arrow keys drive TI joystick 1)
```

## Files in this folder

| File | What it is |
|------|------------|
| `titris.asm` | The complete game source (TMS9900 assembly, ~1100 lines). |
| `titris.ctg` | The compiled cartridge image, ready to mount in the emulator. |
| `README.md` | This document. |

The `.ctg` is committed so you can play immediately without building anything.

## Play it

From the repository root:

```sh
# use the prebuilt cartridge…
cargo run --release -p libre99-app -- \
    --cartridge-file original-content/cartridges/titris/titris.ctg
```

On the console's selection screen press **2** (`2 FOR TITRIS`); the game's own
title screen appears, so **press any key to play** — or press **H** (or **AID**,
i.e. `FCTN`+`7`) for a help screen showing the scoring and controls. Then:

- **←** / **→** move left / right
- **↑** or **X** rotate clockwise; **Z** rotate counter-clockwise
- **↓** soft drop (fall faster while held)
- **SPACE** or **Right-Alt** hard drop (slam to the bottom)

Clearing rows scores the classic 40 / 100 / 300 / 1200 points (1–4 lines at
once). The right-hand panel shows (top to bottom) the **next** piece, the
**score**, the score at which the **next level** is reached (`NEXT LEVEL AT`), and
the current **level** (two digits, pegged at 99). Every 1000 points you gain a
level, and the well loses one column from the right wall — from 20 columns wide
down to a minimum of 8 — trimming any locked blocks that fall outside the new
edge; the narrower well makes lines easier to complete, so leveling tends to
accelerate. Levels keep climbing even after the well reaches its minimum width. A
short rising **fanfare** plays on each level-up. Full rows clear with a beep. When
the stack tops out, a **GAME OVER** panel summarizes the run — score, lines
cleared, pieces placed, time, and level reached — and any key returns you to the
title screen.

## Build it yourself

The assembler ships in this repo, so you can edit `titris.asm` and rebuild:

```sh
cargo run -p libre99-asm -- \
    original-content/cartridges/titris/titris.asm \
    -o original-content/cartridges/titris/titris.ctg
```

`libre99asm` speaks the TI Editor/Assembler dialect and emits a `ti99sim`-format
`.ctg` the emulator loads. (See `assembler/ASSEMBLER.md` for the language and
`docs/history/ASSEMBLER-POC-PLAN.md` for how the toolchain was bootstrapped.)

---

## How it works, and why it was built this way

A TI-99/4A cartridge is just 8 KiB of ROM mapped at CPU `>6000`, with a small
header the console scans on boot. The whole game lives there. A few design
choices keep it small and robust — and each one exercises a different part of the
emulator, which is the point of a POC.

### The cartridge header writes itself

The console lists a cartridge by scanning `>6000` for a magic `>AA` byte and a
little linked list of program entries. Writing that by hand is fiddly, so
`libre99asm` synthesizes it: you just declare a name and an entry label and the
assembler lays down the 16-byte header plus the menu entry for you.

```asm
        IDT  'TITRIS'          ; cartridge / menu name
START   LIMI 0                 ; the entry point
        LWPI >8300             ; give ourselves a workspace in fast scratchpad
        ...
        END  START             ; START becomes the menu's entry address
```

That is why the menu shows `2 FOR TITRIS` with no header bytes in the source.

### A 60 Hz game loop with no interrupts

Console programs usually take the VDP's vertical-blank *interrupt*. We don't need
that complexity: the VDP sets a "frame" flag in its status register every
1/60 s, and reading the status clears it. So the loop just **polls** for the next
frame with interrupts off — simple, deterministic 60 Hz pacing:

```asm
WAITVB  MOVB @VDPST,R2         ; read VDP status (>8802)
        ANDI R2,>8000          ; the frame (vblank) flag
        JEQ  WAITVB            ; not yet — spin
        RT
```

The main loop is then just: wait a frame, read input, apply gravity, redraw.

### Color from a 1970s video chip

The TMS9918A in Graphics I mode gives **one foreground/background color per group
of eight character codes**. To get seven differently-colored pieces we use seven
"solid block" glyphs, one in each color group (`>88, >90, >98, …`), and set seven
color-table entries. A board cell simply stores the block's character code, so
drawing is a plain copy of bytes into VRAM. The palette is deliberately
*reskinned* away from the classic tetromino colors to give the game its own feel
(I magenta, O light blue, T light green, S light red, Z light yellow, J cyan,
L white):

```asm
COLORS  BYTE >D1,>51,>31,>91,>B1,>71,>F1   ; I O T S Z J L (foreground on black)
```

One more color group drives the **alignment guides**: an extra glyph (a one-pixel
*dotted* line down a cell's left edge) sits in its own color group set to dark
blue — the dimmest color the TMS9918A offers (its only "gray" is a near-white
light gray) — and every *empty* well cell is drawn with it instead of a blank
space, so faint vertical lines mark the columns and help you line up a drop. A
falling piece draws over them, and they are restored as it moves on.

### Pieces and SRS rotation

Each piece is stored as its **four SRS rotation states** (spawn, then one/two/
three turns clockwise), each just four cells in the spawn bounding box. Every cell
is one byte — high nibble = row, low nibble = column — so collision, locking, and
drawing all share the same trivial "loop over four bytes" routine. There's no
rotation math at run time; the states are precomputed to the official SRS layouts
(and the pieces spawn in the SRS orientations).

```asm
* SRS true-rotation cell offsets: 7 types x 4 states x 4 cells; byte = (dr<<4)|dc
PIECES  BYTE >10,>11,>12,>13           ; I  state 0 (horizontal)
        BYTE >02,>12,>22,>32           ; I  state R (vertical)
        ...
        BYTE >01,>10,>11,>12           ; T  state 0
```

Rotation implements **SRS wall kicks**. When a turn would overlap a wall, the
floor, or the stack, the game tries the five candidate offsets for that exact
(from-state → to-state) transition and applies the first that fits — so a piece
jammed against the wall slides over instead of refusing to turn, and the familiar
T-spin / I-piece kicks behave the way players expect. The offsets live in two
small tables (one for I, one for the J/L/S/T/Z family); each offset byte is signed
and sign-extended with `SRA` before being added to the piece's position.

The board itself lives in expansion RAM with a **stride of 32** (up to 20 columns
are used), so a cell address is `base + (row<<5) + col` — a shift and an add,
never a multiply. The *current* width is a runtime value (`CURW`), so collision,
clearing, and drawing all read it instead of a fixed constant; lowering it is all
it takes to narrow the well.

### Levels: a well that closes in

The one genuinely new mechanic. After each lock the game divides the score by a
threshold to get a **level**, and the target width is `MAXW - level` (clamped to
`MINW`). If that's narrower than the current well, three things happen, in order:

1. **Trim** — every locked cell at a column `>= CURW` is zeroed. Those blocks sat
   under the old, wider right wall; now they're past the edge, so they're
   discarded.
2. **Settle** — the normal line-clear pass runs once more, because trimming a
   column can complete a row that was only missing cells on the right.
3. **Repaint** — the maximal well footprint is blanked and the U-wall is redrawn
   one column to the left. The board is repainted by the very next frame's draw,
   so the player only ever sees the finished, narrower well.

The well is **left-anchored**: the left wall never moves, the right wall marches
inward, and pieces spawn re-centered in whatever width is current. Because score
only rises, the width only ever shrinks — there's no "grow" path to get wrong.

### Input over the CRU — keyboard *and* joystick

The TI keyboard and joysticks are read the same way: select a column on the
9900's bit-serial CRU bus, then read eight rows back (a pressed switch reads `0`).
The game reads the letter-key columns and **also** joystick 1 (column 6), OR-ing
both into one control mask — which is why the host arrow keys (mapped to TI
joystick 1 by the emulator) work alongside the keys:

```asm
        LI   R0,>0600          ; select column 6 = joystick 1
        LI   R12,>0024
        LDCR R0,3              ; drive the 3-bit column select
        LI   R12,>0006
        STCR R2,8             ; read 8 rows into R2's high byte (pressed = 0)
        ...
        ANDI R3,>1000         ; Joy1 up -> rotate
```

### Sound from the SN76489

A beep is three byte-writes to the sound chip at `>8400` — latch a tone period,
then its high bits, then an attenuation. The game beeps on lock and on a line
clear, and a small frame counter silences the channel a few frames later. The
level-up **fanfare** is a four-note rising arpeggio on a *second* channel, played
one note per frame from a frame counter so it overlaps the lock/clear beeps
without fighting them for a channel.

### Working without a stack

The TMS9900 has no hardware call stack; `BL` just stashes the return address in
R11. The game keeps the convention that **leaf** routines use R11 directly, while
routines that call others first save R11 (in R14) and return through it — enough
structure for clean subroutines without building a software stack. (A routine
that forgets this — calling `BL` without saving R11 — returns into the middle of
itself; that exact bug, in the screen-clear routine, was the one real snag while
adding the title screen.)

### A title, a font, and a scoreboard

The console hands the cartridge a blank slate — no usable character font — so the
game loads its own. A compact 8×8 uppercase font (letters, digits, a dash) is
stored as `(char, 8 pattern bytes)` records and blitted into the pattern table at
each glyph's ASCII code; after that, drawing text is just writing ASCII into the
name table. The big **TITRIS** on the title screen is separate: a tiny 3×5
bitmap per letter, "inflated" into whole character cells of the cyan block glyph,
so it reads as graphics rather than text.

The **score** is a 16-bit counter; line clears add the classic table
(`0, 40, 100, 300, 1200` for 0–4 rows) looked up with indexed addressing
(`MOV @SCORTB(R3),R1`). Displaying it turns the value into five decimal digits by
repeated division — a use of the 9900's `DIV` instruction:

```asm
DSL     CLR  R2
        MOV  R5,R3             ; dividend = value
        DIV  R0,R2             ; R0 = 10 -> R2 quotient, R3 remainder (a digit)
        AI   R3,>0030          ; remainder -> ASCII '0'..'9'
        ...
```

The **next-piece preview** just draws the upcoming tetromino's rotation-0 cells
into a 4×4 box beside the well; the spawn routine pulls the current piece from
`NEXT` and rolls a fresh `NEXT` each time, so the box always shows what's coming.

The **game-over overlay** is a bordered panel drawn *on top of* the final board
(the board is rendered first, then the box). It reuses that same five-digit
number routine to print the run's stats — score, lines, pieces placed, and time
(the frame counter divided by 60). Topping out, the panel, and "press any key →
title" all reuse pieces already built for the title screen, so it was mostly
composition rather than new mechanism.

### A help screen on the title

Pressing **H** — or **AID** (`FCTN`+`7`), the TI's conventional help key — at the
title opens a help screen covering the line-clear scoring and the controls. It is
drawn from a small table of `(name-address, string, length)` records so the
key/value columns align without hand-counted padding, with a row of the seven
piece-color block glyphs as a rainbow accent and colored block "bullets" beside
the section headers. Reading the keys is the same CRU scan the game uses; bare
modifier keys are filtered out of the "any key starts" check so holding `FCTN`
for AID doesn't launch a game by accident. (Adding the screen also completed the
font to the full A–Z, since words like `DROP` needed the missing letters.)

---

## What this proves (the POC part)

Building a real-ish game shook out the whole pipeline:

- **The assembler** had to encode the full TMS9900 instruction set — not just the
  handful the "hello world" demo used. Titris leans on `LDCR`/`STCR` (CRU), the
  shift instructions, `XOR` (its xorshift piece bag), `DIV` (decimal score),
  indexed addressing (the score and kick tables), and `SRA` (sign-extending the
  signed wall-kick offsets) — all added and golden-tested for this.
- **The emulator** had to faithfully run multi-bit CRU I/O, the keyboard/joystick
  matrix, the SN76489, and VDP timing — together, in a tight 60 Hz loop.
- **The loop was closed** by the `--cartridge-file` flag, so an assembled `.ctg`
  mounts straight from disk with no rebuild.

An automated end-to-end test (`crates/libre99-asm/tests/titris.rs`) assembles *this
exact `titris.asm`*, boots it on the emulated console, drives the keyboard and
joystick, and asserts that pieces spawn, move, rotate, fall, lock, and render —
so the game can't silently break as the assembler or core evolves.

## Limitations & ideas

It's a POC, so there's plenty of room: levels narrow the well but don't speed up
gravity, there's no hold piece, no soft-drop scoring, and each stat caps at 65535
(16-bit). The random bag is a plain xorshift (no 7-bag). All of these are small,
self-contained additions on top of the structure above — good next exercises for
the toolchain.
