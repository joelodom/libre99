# SOKOBAN — the warehouse-keeper puzzle for the TI-99/4A, in TMS9900 assembly

**Sokoban** is a complete, playable port of the classic box-pushing puzzle for
the TI-99/4A — the project's **second original cartridge**, written like
[Titris](../titris/README.md) in TI Editor/Assembler-style source, assembled by
the repo's own `libre99asm` (`crates/libre99-asm`), and booted on the repo's own
emulated console (`crates/libre99-core`). Where Titris proved the *pipeline*,
Sokoban exercises the *architecture* of a data-driven game: levels stored as
readable text and parsed at run time, a bitfield board rendered through a
lookup table, a flood fill, and an undo ring buffer — all in about 1,100 lines
of 9900 assembly that still fit an 8 KiB cartridge window with ~3.4 KiB to
spare.

It is a real game, not a demo: a graphical title screen, a help screen, twelve
levels from a classic freely-distributable set, walking and pushing with
hold-to-repeat, **undo with hold-to-rewind**, retry and level skip, move and
push counters, distinct sounds for steps/pushes/bumps/stores, a rising jingle
per solved level, and a win screen with your totals.

```
   ===== title screen =====             ========== gameplay ==========
                                        SOKOBAN           LEVEL 01 OF 12
   ### ### # # ### ##   #  #  #        MOVES 00007      PUSHES 00002
   #   # # ##  # # # # # # ###
   ### # # #   # # ##  ### ###                    ######
     # # # ##  # # # # # # # #                    #    #
   ### ### # # ### ##  # # # #                    # #@ #
                                                  # $* #
       THE WAREHOUSE KEEPER                       # .* #
                                                  #    #
      LEVELS FROM MICROBAN                        ######
       BY DAVID W SKINNER
                                        # wall   $ box   . storage spot
     PRESS ANY KEY TO PLAY              * box on a spot  @ you
                                        (plus dotted floor inside the
       H OR AID FOR HELP                 warehouse, black void outside)

   controls:  E/S/D/X or joystick 1 (host arrow keys)  move / push
              U or joystick fire  undo (hold it to rewind the whole level)
              R retry level    N / P next / previous    Q quit to title
```

## Files in this folder

| File | What it is |
|------|------------|
| `sokoban.asm` | The complete game source (TMS9900 assembly, ~1,100 lines). |
| `sokoban.ctg` | The compiled cartridge image, ready to mount in the emulator. |
| `README.md` | This document. |

The `.ctg` is committed so you can play immediately without building anything.

## Play it

From the repository root:

```sh
cargo run --release -p libre99-app -- \
    --cartridge-file original-content/cartridges/sokoban/sokoban.ctg
```

On the console's selection screen press **2** (`2 FOR SOKOBAN`); the game's own
title screen appears, so **press any key to play** — or press **H** (or
**AID**, i.e. `FCTN`+`7`) for a help screen with the goal, a tile legend, and
the controls. Then:

- **E / S / D / X** (the TI's arrow diamond) or **joystick 1** — the host
  arrow keys — walk the keeper; walking into a box **pushes** it if the cell
  beyond is free. You can push exactly one box at a time, and never pull —
  that's the whole puzzle.
- **U** or the **joystick fire button** — undo one move (a pushed box comes
  back with you). *Hold* it and the game rewinds move by move; the ring
  remembers your last 2,048 moves, which is the whole level in practice.
- **R** — retry the level from the start.
- **N** / **P** — jump to the next / previous level (they wrap, so you can
  reach any level from the title without solving a thing — it's a POC, not a
  gauntlet).
- **Q** — back to the title screen.

The HUD shows the level number, **MOVES**, and **PUSHES** (undo takes both
back — solve it in fewer!). Push the last box onto its spot and a rising
jingle plays, `LEVEL COMPLETE!` flashes, and the next level loads. Solve all
twelve and the **YOU WIN** panel totals your moves, pushes, and time — then
any key returns to the title.

## The levels

The twelve puzzles are from **Microban** by **David W. Skinner** — the classic
beginner's set, chosen level by level for variety and a difficulty arc, and
transcribed exactly as published:

| Cartridge level | Microban # | Cartridge level | Microban # |
|---|---|---|---|
| 1 | 2 | 7 | 9 |
| 2 | 1 | 8 | 34 |
| 3 | 4 | 9 | 3 |
| 4 | 5 | 10 | 33 |
| 5 | 17 | 11 | 35 |
| 6 | 7 | 12 | 36 |

Skinner's sets are the Sokoban community's standard freely-usable corpus: they
"**may be freely distributed provided they remain properly credited**," and
this cartridge credits them on its own title screen (`LEVELS FROM MICROBAN BY
DAVID W SKINNER`) as well as here. The transcription source was the
`microban.slc` collection file (Copyright David W Skinner) as mirrored by
[sourcecode.se](https://www.sourcecode.se/sokoban/levels.php), cross-checked
against a second mirror.

*Sokoban* (倉庫番, "warehouse keeper") was created by Hiroyuki Imabayashi and
published by Thinking Rabbit in 1982; the name is used here descriptively, as
the puzzle genre's common name. None of Thinking Rabbit's original levels are
included — the level data is Skinner's set only, and everything else in this
folder is original work.

## Build it yourself

The assembler ships in this repo, so you can edit `sokoban.asm` and rebuild:

```sh
cargo run -p libre99-asm -- \
    original-content/cartridges/sokoban/sokoban.asm \
    -o original-content/cartridges/sokoban/sokoban.ctg
```

Adding a thirteenth level is a data edit: append a record (width, height, then
`TEXT` rows in standard XSB notation, each padded to the width), add its label
to `LEVTAB`, and bump `NLEVELS`. The parser, centering, flood fill, and win
logic all read the data — nothing else changes.

---

## How it works, and why it was built this way

A TI-99/4A cartridge is 8 KiB of ROM at `>6000` behind a small header the
console scans at boot (`libre99asm` synthesizes the header from `IDT` and `END`,
exactly as for Titris). Within that, Sokoban makes a few design moves that are
different in kind from Titris — that's what makes it a useful second cartridge.

### Levels are data, and the data is the published text

Each level is stored in ROM as its **XSB notation** — the Sokoban community's
standard text format — one `TEXT` row per line, so the source *is* the level:

```asm
LVL01   BYTE 6,7              ; Microban 2
        TEXT '######'
        TEXT '#    #'
        TEXT '# #@ #'
        TEXT '# $* #'
        TEXT '# .* #'
        TEXT '#    #'
        TEXT '######'
```

A load-time parser walks the record and expands each character into a **tile
state byte** on a 32-byte-stride board at `>A000`: bit 0 wall, bit 1 goal,
bit 2 box, bit 3 player. Storing *states* rather than glyphs means game logic
is pure bit tests (`is the target a wall? does it hold a box?`) and never
cares how anything looks. Because a published level is pasted in verbatim, a
transcription typo is possible — which is why the test suite plays every
shipped level to completion (below).

### One lookup table turns states into pixels

The board and the name table share the same 32-column stride, so a cell's
board offset **is** its screen offset: `name address = NBASE + offset`, where
`NBASE` just centers the level. Drawing any cell is three steps — read the
state byte, index a 32-entry table (`TILECH`) with it, write that character:

```asm
        MOVB @BOARD(R1),R0    ; tile state (bits 0-4)
        ...
        MOVB @TILECH(R2),R2   ; state -> glyph, one indexed load
        MOVB R2,@VDPWD
```

A move redraws exactly the two or three cells it touched; there is no
full-screen repaint after load, so the well-under-vblank budget that Titris
had to manage carefully never gets close to mattering here.

### A flood fill separates the warehouse from the void

XSB uses a space for both "floor inside the walls" and "nothing at all outside
them." Rendering both as black looks flat; classic Sokoban draws the interior
as floor. At load time a breadth-first **flood fill** from the keeper's start
cell marks every reachable non-wall cell with bit 4 (*interior*), and the
glyph table renders marked empty cells as a subtly dotted floor while
unreachable cells stay black. The BFS queue borrows the undo buffer's RAM —
it's free real estate until the first move is recorded a few frames later.

### Undo is a ring buffer of one-byte moves

Every move is two bits of direction plus one bit of *did-it-push* — so the
undo history is **one byte per move** in a 2,048-entry ring at `>B000`.
Undoing pops an entry, steps the player back, and (for a push) pulls the box
back with them, reversing the same `BOXLEFT` goal bookkeeping the forward move
did. The move/push counters decrement, so undo is honest: a solution's
displayed cost is what you actually ended up doing. Holding **U** (or fire)
rewinds at the key-repeat rate — the input layer treats undo exactly like a
movement key, so hold-to-walk and hold-to-rewind are the same dozen lines.

When the ring wraps, the oldest entry is silently overwritten; 2,048 moves is
several times the length of any sane solution to these levels (the longest
scripted solution is 156 moves).

### Input: new presses act, held keys repeat

The CRU keyboard scan reads the E/S/D/X diamond, U/R/N/P/Q, and joystick 1
(the emulator maps the host arrow keys onto it), OR-ing keyboard and joystick
into one control mask. Each frame, `newpress = current AND NOT previous`
triggers an immediate action; a *held* direction (or undo) fires again after
16 frames and then every 5 — the classic delayed-auto-repeat feel, implemented
once and shared by walking and rewinding. A blocked move costs nothing and
buzzes the noise channel, so holding a direction against a wall is tactile
feedback rather than an error.

### One fanfare table, three sounds

Channel 0 carries the step tick and the push thud; the noise channel carries
the bump. Channel 1 plays everything melodic from a **single eight-note
descending-pitch table** and one frame counter: the counter's quarter is the
note index, so `FANCNT = 4` plays only the top note (the "box stored" ding),
`16` plays the top four notes as the level-complete jingle, and `32` walks the
whole C5→C6 run for the win screen. Three effects, one mechanism, sixteen
bytes of data.

### The rest is the Titris playbook

The 60 Hz polled loop (`WAITVB` on the VDP status frame flag, interrupts off),
byte-wise VDP access, the `(char, 8-bytes)` glyph loader, per-color-group tile
art (gray brick walls, dark-yellow crates with X bracing, light-green stored
boxes with a diamond cutout, a white keeper who turns cyan on a spot), the 3×5
big-letter title, `DIV`-based decimal readouts, and the R11/R14/`RET2` return
discipline for nested calls are all the conventions Titris established, reused
deliberately so the two cartridges read as one codebase.

---

## What the test proves

`crates/libre99-asm/tests/sokoban.rs` assembles *this exact source*, boots it on
the emulated console, and plays it through the emulated keyboard and joystick:
title, help, credit text, parsing, centering, flood-fill interiors, walking,
pushing, blocked-move refusal, undo (single-step and hold-to-rewind), retry,
skip in both directions with wraparound, quit, and the level-complete flow.

The last test, `all_twelve_levels_are_winnable_as_shipped`, feeds each level a
**scripted solution generated by a breadth-first-search solver** run against
the same level text and asserts the game reaches the `YOU WIN` panel — so a
one-character typo in any level's data (or any regression in the push/undo
rules) fails CI. The solutions in the test file are move-optimal, if you want
something to race.

## Limitations & ideas

It's a proof of concept, so there's headroom: no level-select on the title
screen (N/P in-game covers it), no best-score memory across runs, counters cap
at 65,535, the keeper doesn't animate between cells, and twelve levels is a
taster — Microban has 155, and the record format makes adding more a data
edit. A deadlock detector (warning when a box is pushed into an unwinnable
corner) would be a nice exercise in exactly the kind of bit-tested board logic
this cartridge is built on.
