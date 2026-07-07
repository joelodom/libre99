# Chapter 40 — Capstone II: The Fixed-Screen Arcade

<!-- Part IX — Case Studies: Recreating the Classics · target ≈20 pp · [console-only + cart] -->
<!-- STATUS: DRAFTED (session 9, 2026-07-07) — pending review passes. GRIDRUNNER 99's engine (state machine, algorithmic maze, target-tile AI, mode timers, three-channel sound, recorded-input attract) MACHINE-VERIFIED on BENCH99 at commit 408a451: gridrunner.a99 -> 8,192-byte single-bank image (entry >61FE, code+data >61FE-6F8E ≈ 3.4 KB, ~4 KB free), deterministic 11-part self-test GREEN (VR7=>02, FAILID=0). Code in code/ch40/. Builds on R-19 (Ch.39). -->
<!-- SPEC: 00-master-outline.md, "### Chapter 40 —" (lines 636–644), Part IX preamble (621–623). -->

## The Machine That Fit in a Thumbnail

In the autumn of 1981 a TI field engineer could carry the whole of *Munch Man*
into a sales meeting in his shirt pocket and, with nothing but a bare console
and a television, have a crowd playing it inside a minute. No disk drive. No
memory expansion. No thirty-two-kilobyte sidecar the size of a hardback book.
The cartridge went *click* into the slot, the title screen came up, and the
little yellow eater began its rounds through a blue maze while four colored
pursuers peeled off in four different directions — each, you slowly realized,
with a personality of its own.

Here is the part that should stop a modern programmer cold. That console had
**two hundred fifty-six bytes** of CPU RAM. Not kilobytes. Bytes — fewer than
the characters in this paragraph. Everything the game's logic could remember
from one sixtieth of a second to the next — where the player was, where four
enemies were, which way each was headed, how much of the maze remained, whose
turn it was to hunt and whose to scatter — all of it had to live in a space you
could dump on half a punch card. And it did. The maze itself, the score, the
very list of dots not yet eaten, lived somewhere else entirely, because there
was nowhere else in the pad to put them.

METEOR BELT, last chapter, had it easy. It ran on the standard system, so it
parked its entity tables up in expansion RAM at `>A000` and never gave the pad
a second thought. GRIDRUNNER 99 does not get that luxury. This chapter is about
what you build when the luxury is taken away — and about the strange, bracing
clarity that arrives when a machine tells you *there is no more room*, and means
it.

---

## What You Will Learn

After this chapter you can:

- **Architect a complete game inside 256 bytes of CPU RAM**, holding only the
  irreducible per-frame state in the pad and pushing everything else into VRAM
  or into pure computation.
- **Represent a maze as an algorithm instead of a table**, so the wall map costs
  *zero* bytes of RAM and one small routine.
- **Use the name table as a live database** — read a cell to ask "is there a dot
  here?", write a space to eat it — so the pellet state needs no bitmap.
- **Write a target-tile chase AI** with four distinct enemy personalities, a
  scatter/chase mode timer, and a frightened mode, all driven from small tables.
- **Give three sound channels a game's sonic identity** and drive an attract
  screen from a recorded-input reel.
- **Prove a whole arcade engine deterministically on the bench** and ship it as
  an 8K cartridge that runs on any unexpanded 99/4A ever built.

## The Bridge: Programming Against a Hard Wall

Every chapter of this book has spent memory it did not really have to. Even on a
"bare console" our labs leaned on the pad's roomy lower half and never counted
the cost too closely. That ends here, on purpose. GRIDRUNNER 99 is a
**constraint study**: it deliberately excludes the 32K expansion so that the
famine is real and the lessons are forced.

If you have written embedded firmware for a small microcontroller, you have met
this wall — the part with two kilobytes of RAM where every `struct` field is a
decision and the linker map is scripture. If you have not, the mental shift is
this: **you stop asking "where do I store this?" and start asking "do I need to
store this at all?"** A wall map can be a formula. A list of remaining pellets
can be the screen itself. The score can live in the pad because it is four bytes
and it earns its keep; the maze cannot, because it is hundreds of bytes and a
formula is free. Scarcity, handled well, is not a cage. It is an editor — a
strict one — and the program it hands back is smaller, faster, and often
cleaner than the one you would have written with room to be lazy.

We build on **SKELETON99** (Ch. 36) and the `lib99` we have grown all book long,
and we hold the result to **CQ-82** (§2.6), the 1982 commercial-quality
checklist, exactly as METEOR BELT was held. The case-study arc is the one
established in Chapter 39 and fixed as ruling **R-19**: we dig up the genre, we
write its spec, we build it, and we hold a postmortem over the corpse of our own
mistakes.

## 40.1 Archaeology: Grid Logic and the Famine

The fixed-screen maze chase is the oldest arcade shape the 99/4A inherited, and
TI's own *Munch Man* is its canonical local specimen: a full-screen maze of
character cells, a player who clears the maze by eating everything in it, four
pursuers who are not a swarm but a *quartet*, and — the detail that makes the
genre — pursuers whose different behaviors turn a chase into a readable, learnable
dance.

As in Chapter 39, honesty comes first (**R-19**). This project is IP-clean by
construction; the `cartridges/` directory ships empty, and we own no maze-chase
cartridge to instrument. So we do not claim to have single-stepped *Munch Man*.
We reconstruct the genre's *mechanics* from three things we can actually stand
on: the published behavior of the arcade maze-chase form, which is broadly
documented; the hardware we have measured to the cycle across Parts III–V; and
the one constraint the outline hands this chapter — **it must run on a bare
console**. That last constraint is not a footnote. It is the archaeology,
because it dictates the architecture the way bedrock dictates a foundation.

Three findings survive the dig, and each is something we have already proven on
our own bench:

- **A maze is a character grid, not a bitmap.** The playfield is name-table
  cells (Ch. 13), one byte each, drawn once and left alone. The moving actors
  ride *over* the grid as sprites (Ch. 16), composited by the VDP for free. This
  is why the genre runs where bitmap games (Ch. 15, 12 KB of VRAM, ~17 frames to
  clear) cannot: a character maze is a few hundred bytes and redraws in a blink.
- **The four pursuers are a table, not a mob.** Each is a small state — a tile, a
  heading, a personality — and the "intelligence" is a short decision made at
  each maze intersection. Four distinct rules over one shared routine is the
  whole trick, and it is a *data* trick (Ch. 36.6), not a code trick.
- **The famine is the design.** On a bare console the pad holds the actors and
  nothing more. The maze and the pellet state must live elsewhere — and the only
  "elsewhere" a stock console offers is the 16 KB behind the VDP. The genre's
  console-only heritage is, at bottom, a story about where a program is *allowed*
  to keep its memory.

**Field Notes — counting the pad.** Take the state a maze chase must carry from
frame to frame and add it up in bytes: five actors at eight bytes each is forty;
a dozen scalars (score, lives, level, dots remaining, whose-turn timer) is
another two dozen; a workspace is thirty-two; a stack is a handful. You land near
**a hundred and forty bytes** — comfortably inside 256, but only because the maze
and the pellets are *not on the list*. Put a 19×19 wall map in the pad (361
bytes) and you are already over budget before the first sprite moves. The pad
does not have room for the maze. That single fact writes §40.3 for us.

## 40.2 Specification: GRIDRUNNER 99

Here is the design document a 1982 team would have written before a line of code
— a **labeled reconstruction** (**R-1**), invented for this book, not recovered
from an archive.

**GRIDRUNNER 99 — a maze-chase arcade game.**

*Premise.* The player runs a grid maze, eating every pellet. Four pursuers hunt
in a coordinated pattern. Four power pills, one in each quadrant, briefly turn
the hunters into the hunted. Clear the maze and the next level runs faster.

*Definition of done (the CQ-82 contract).* Instant response to the stick; a
title/attract screen that plays itself; visually and sonically distinct pursuers;
a scatter/chase rhythm the player can feel; a fair difficulty ramp; three lives
and a clean game-over; **and it must run on a console with no expansion of any
kind**. That last line is GRIDRUNNER's signature requirement, and every other
decision bends to it.

*The maze.* A 19×19 grid of tiles, centered on the screen at column offset 6 and
row offset 2, leaving the top rows for the score line. Walls form a regular
lattice —
border plus a post at every tile whose row and column are both odd — leaving
one-tile corridors that are, by construction, fully connected. Corridors carry
pellets; the four interior corners carry power pills. A small central room is
left open as the pursuers' den.

*The pursuers (the four-behavior pattern).* Each enemy computes a **target tile**
and, at every intersection, steps toward it — choosing, among the legal moves
that are not a reversal, the one whose neighbor tile is nearest the target. The
personalities differ only in how they pick the target:

| # | Name | Chase target |
|---|---|---|
| 0 | CHASER | the player's own tile — relentless pursuit |
| 1 | AMBUSH | four tiles ahead of the player — cuts you off |
| 2 | FLANK | the player's mirror image across the maze — attacks from behind |
| 3 | SHY | the player when far, its home corner when close — skittish |

A global **mode timer** flips all four between *scatter* (each heads for its own
corner and the pressure lifts) and *chase* (each hunts). Eating a power pill
opens a *frightened* window in which the pursuers flee and a touch sends them
home for points.

*Difficulty.* A per-level **speed tier** (pixels per frame, always a divisor of
the eight-pixel tile so actors land cleanly on intersections). Clearing a maze
advances the level and, at the thresholds, the speed.

*The memory budget — the whole point.* This is the worksheet from §36.1, filled
in for famine:

| Store | What lives there | Cost |
|---|---|---|
| Scratchpad `>8300`–`>831F` | the one workspace | 32 B |
| Scratchpad `>8320`–`>833F` | software stack | ≤32 B |
| Scratchpad `>8342`–`>8362` | ~16 game scalars | ~34 B |
| Scratchpad `>8364`–`>838B` | 5 actors × 8 bytes | 40 B |
| Scratchpad `>838C`–`>839D` | AI decision scratch | ~18 B |
| **VRAM name table** | **the maze + the pellet state** | **(in VRAM)** |
| **(none)** | **the wall map** | **0 B — it is a routine** |
| Cartridge ROM `>6000`–`>7FFF` | code, sprite shapes, tables | ≤8 KB |

Two rows in that table are the chapter. The wall map costs *nothing* because it
is computed. The pellet state costs nothing in the pad because it lives on the
screen. Everything the pad holds is state that genuinely changes sixty times a
second and has nowhere cheaper to be.

## 40.3 Construction I: The Famine Engine

GRIDRUNNER instantiates SKELETON99's chassis unchanged in spirit: a phase state
machine (ATTRACT → PLAY → OVER) dispatched through a table, each phase stamping a
signature so the dispatch is provable on the bench. What changes is *where the
data lives*, and two ideas carry the whole weight.

**Idea one: the walls are an algorithm.** A lattice maze does not need a map. A
tile is a wall if it is on the border or if both its coordinates are odd; every
other interior tile is open corridor. That is six instructions and no memory:

```asm
* WALLAT — is tile (R0=col,R1=row) a wall? Returns R2 (1=wall, 0=open). The
* whole maze IS this routine: the border, plus a post at every odd/odd cell.
* Zero bytes of RAM. Leaf; preserves R0/R1.
WALLAT CI   R0,1
       JLT  WAW
       CI   R0,MWM2          MW-2 = last interior column
       JGT  WAW
       CI   R1,1
       JLT  WAW
       CI   R1,MHM2
       JGT  WAW
       MOV  R0,R3
       ANDI R3,1
       JEQ  WAO              even column -> corridor
       MOV  R1,R3
       ANDI R3,1
       JEQ  WAO              even row -> corridor
WAW    LI   R2,1
       RT
WAO    CLR  R2
       RT
```

Because corridors are exactly the cells with at least one even coordinate, the
lattice is provably connected — every corridor cell touches another through a
shared even row or column — so no hand-drawn map can trap an actor in a sealed
pocket. A real product would layer a designed maze *table* over this skeleton
(the data-driven habit of §36.6), and the exercises ask you to; but the generated
lattice is a genuine, playable maze that costs one routine and not one byte, and
on a bare console that trade is the difference between shipping and not.

**Idea two: the name table is the pellet database.** When MAZINI paints the maze,
it writes a pellet glyph into every corridor cell and counts them into `DOTS`.
The pellet *state* is then simply the screen: a cell holds a pellet glyph until
it is eaten, and eating it writes a space. To ask "is there a dot on the player's
tile?" you read the cell; to eat it, you blank it. No pellet bitmap exists
anywhere in the pad, because the picture already *is* the bitmap:

```asm
* PLREAT — eat whatever pellet sits on the player's tile (@ATX,@ATY). The name
* table is the database: read the cell, and if it is food, blank it and score.
PLREAT ...
       BL   @CELLAD          R2 = the cell's VRAM address
       MOV  R2,R5            keep the address (survives VSBR/VSBW)
       MOV  R2,R0
       BL   @VSBR            R1 high = the glyph there
       SRL  R1,8
       CI   R1,PELCH
       JEQ  PREPEL           a pellet: +10
       CI   R1,PILLCH
       JEQ  PREPIL           a power pill: +50, go frightened
       ...                   (neither: nothing to eat)
```

The actors themselves are five sprites over the character maze — the "character
rendering with sprite garnish" the outline calls for. Each actor is eight bytes
in the pad: pixel position, heading, a buffered turn, a personality, and its home
corner. Movement is whole pixels per frame at the level's speed; because the
speed always divides the eight-pixel tile, an actor arrives exactly on tile
centers, and *only there* does it make a decision — the player consults its
buffered turn and stops at walls, the enemies think. Between decisions the sprite
simply glides, and the VDP composites it over the maze for free.

The result is a game whose logic footprint is the pad and whose world is the
screen. Dump `>8342` on the bench after a frame and you can read the entire state
of GRIDRUNNER 99 in forty bytes.

## 40.4 Construction II: The Enemy Mind

The four-behavior pattern is the chapter's centerpiece, and it is smaller than it
looks. One routine, `ENAI`, decides every enemy's next heading. It computes the
enemy's target tile (by mode and personality), then walks the four directions in
a fixed tie-break order — up, left, down, right — skipping the reversal and any
wall, and keeps the legal step whose neighbor is nearest the target:

```asm
ENLP   MOV  R8,R1
       SLA  R1,1
       MOV  @DIRORD(R1),R0
       MOV  R0,@ADC          d = the direction we are trying
       MOVB @2(R9),R1        the enemy's current heading ...
       SRL  R1,8
       LI   R2,1
       XOR  R2,R1            ... reversed
       C    @ADC,R1
       JEQ  ENNX            never reverse
       ...                  ntx,nty = this neighbour; skip if WALLAT says wall
       BL   @DIST           R2 = squared distance from the neighbour to target
       MOV  @AIMAX,R3
       JNE  ENMAX
       MOV  @ABDS,R0
       C    R2,R0
       JHE  ENNX            chase: keep the strictly nearer step
       JMP  ENUPD
ENMAX  MOV  @ABDS,R0
       C    R2,R0
       JLE  ENNX            frightened: keep the strictly farther step
ENUPD  MOV  R2,@ABDS
       MOV  @ADC,R0
       MOV  R0,@ABD
```

The distance is the true squared Euclidean metric, `dx² + dy²`, computed with a
single `MPY` per term. A pleasant accident of two's complement makes this cheap:
unsigned multiplication of a negative displacement returns the *correct* square
in its low word, because `(65536 − d)² ≡ d² (mod 65536)` for any small `d`, so we
never spend an instruction on `ABS`.

The personalities are the only thing that varies, and they vary in one place —
how `TARGET` fills the goal tile. CHASER aims at the player's tile; AMBUSH at four
tiles ahead of the player's heading; FLANK at the player's mirror across the
maze; SHY at the player when more than six tiles away and at its own corner when
closer. In *scatter* mode every personality is overridden: all four aim at their
home corners, the pressure visibly lifts, and the player gets a breath. The
**mode timer** counts down a fixed phase length and flips the mode, and that
single flipping byte is what turns four pursuers into a rhythm you can learn.

Frightened mode reuses the entire decision machinery with one bit flipped. Set
`AIMAX`, aim the target at the player, and the "keep the nearer step" test
becomes "keep the *farther* step" — the same routine now flees. Four behaviors,
a scatter/chase rhythm, and a flight response, all out of one decision loop and a
table of targets. That is the genre's intelligence, and it fits in a corner of an
8K ROM.

Difficulty rides on top as a table. A per-level speed tier (`SPDTAB`, values that
all divide eight) makes the actors quicker as the levels climb, and the scatter
phases could shorten with them; clearing the maze bumps the level and re-lays the
board. The whole balance of the game is a handful of `DATA` words you can tune
without touching a line of logic (§36.6) — the exercises invite you to.

## 40.5 Construction III: Identity — Sound and the Attract Screen

A 1982 arcade cabinet sold itself from across the room, and it did it with the
two things GRIDRUNNER finishes on: a *voice* and a *demo*.

**Three channels, one identity.** The SN76489 (Ch. 19) gives three tone voices
and a noise channel, and GRIDRUNNER spends them deliberately: channel 0 chirps a
short blip on every pellet; channel 1 answers with a higher note on a power pill;
the noise channel barks on a death; and channel 2 carries a two-tone **siren**
whose pitch tracks the mode, low in scatter and higher in chase, so the ear knows
the rhythm before the eye does. Four voices, one byte at a time, are enough to
make a maze feel like a place.

**The attract screen plays itself.** A commercial title never sits idle on a
menu; it demonstrates. GRIDRUNNER's ATTRACT phase replays a **recorded-input
reel** — a small table of joystick directions — feeding one canned move per frame
into exactly the same player-update path the live game uses:

```asm
DEMO   BYTE 3,3,3,1,1,3,3,0,0,2,2,2    a recorded reel of headings
DEMOLN EQU  12
```

Because the demo drives the real engine, the attract screen is not a special case
to maintain — it is the game, playing a script instead of a stick. The live game
substitutes the joystick for the reel: each frame it reads the stick through
`inplib` (Ch. 21) into the same "desired direction" the demo writes, and calls the
same dispatch. That substitution is the only difference between the machine
playing and you playing.

A word on where the beam lives (**R-12**). The bench cannot run the 60 Hz
interrupt loop — it has no beam to raise the frame flag (Ch. 17, Ch. 22) — so, as
with DODGE and METEOR BELT, GRIDRUNNER's *interactive* layer belongs to the
running machine, and the file we ship contains the engine and its proof, not a
live loop. Everything the engine does is exercised and verified below; the last
inch — the stick in your hand, the frame under the beam — is real hardware's to
close, and it closes exactly as Chapter 22 describes.

## 40.6 Postmortem: What Sixteen-K-of-Nothing Forces You to Invent

The famine paid off, and the ledger says how. GRIDRUNNER 99's engine assembles to
an **8,192-byte single bank** (entry `>61FE`, code and data occupying
`>61FE`–`>6F8E`, about **3.4 KB**, leaving some four kilobytes of ROM unused), and
its logic lives entirely in the scratchpad — no expansion RAM, no VRAM data
warehouse beyond the screen itself. Where METEOR BELT needed 120 bytes of `>A000`
for its entity tables, GRIDRUNNER needs zero bytes outside the pad, because the
constraint forced two inventions a roomier machine would never have provoked:

1. **The wall map became a formula.** Denied 361 bytes for a table, we found the
   maze was regular enough to *compute*, and got a connected maze for six
   instructions and nothing else. Constraint drove us to a better representation,
   not merely a smaller one.
2. **The screen became the database.** Denied a pellet bitmap, we noticed the
   information was already on the glass and read it back with `VSBR`. The most
   memory-constrained store on the machine — the pad — was relieved by the most
   visible one.

There is a third lesson, and it cost real debugging. On a famine machine your
routines nest deeply and your registers are few, so the **R-16 calling
convention** (Ch. 9) stops being bookkeeping and becomes load-bearing. Three bugs
in GRIDRUNNER were all one bug wearing three hats — a value left in a register
that the next call quietly clobbered:

- The sound helpers were written as leaves but ended `BL @STONE` / `RT`; the
  `BL` overwrote R11, so `RT` branched *back into the helper* — an infinite loop,
  the R11-clobber of Chapter 9 in period costume. The fix was a tail call: `B
  @STONE`, letting `STONE`'s own return carry through.
- The pellet-vs-pill flag was parked in R3 and then a call to `CELLAD` — which
  clobbers R3 — ran between setting it and reading it, so every pellet scored as a
  pill and the game went permanently frightened. The fix was to keep the flag in a
  register the callee preserves.
- The self-test kept the player pointer in R4 across `MOVEP`, forgetting that
  `MOVEP` eats a pellet, which plays a sound, which calls `STONE`, which clobbers
  R4 (`sndlib` says so in its header). The pointer survived on paper and died in
  practice, three calls deep.

None of these is exotic. All three are the same discipline — *know what every
call you make is allowed to destroy* — and all three are easier to violate when
the machine is too poor to let you keep a spare copy of everything. The pitfalls
box below is the whole lesson: **on a small machine, the calling convention is
not etiquette, it is the contract that keeps your data alive.**

> **Pitfalls — register discipline under famine.**
> - A "leaf" that calls anything is not a leaf. If it `BL`s, it must save R11 or
>   *tail-call* the last thing it does. `BL @X` / `RT` is an infinite loop.
> - A value you need after a call must live in a register the callee preserves,
>   or on the R10 stack, or in the pad. "It was still there a moment ago" is not
>   a storage strategy.
> - Consult the header of every `lib99` module you call: it names what it
>   clobbers. `sndlib`'s `STONE` eats R3 and R4; `spritelib`'s `SMOV` walks R0–R6;
>   `ENAI` uses R8 as its loop index. Cross a call with a live register at your
>   peril.

## Lab 40 — GRIDRUNNER 99

The lab is the game, and its proof is the file. As with METEOR BELT, the
interactive game belongs to the running machine, so `code/ch40/gridrunner.a99`
drives its own engine through a **deterministic script** and paints the border
verdict — GREEN if every subsystem holds, RED with a failure id otherwise. Eleven
scenarios are checked in one run: the phase dispatch (ATTRACT and OVER
signatures); an integrated PLAY frame; a scripted player run that moves and eats;
a wall that stops the player cold; a CHASER that steers toward the player; the
mode timer flipping scatter to chase; a pursuer catching the player and costing a
life; a frightened pursuer eaten for points; the last pellet clearing the level;
the last life ending the game; and the attract demo replaying its recorded input.

**Machine-verified (BENCH99, commit 408a451).** `gridrunner.a99` assembles to an
8,192-byte single-bank image (entry `>61FE`); the self-test reports **VR7 = `>02`
(GREEN)** with **`FAILID` = 0**, and `verify.sh` assembles all sixty-five
companion sources and builds the bench. Run it yourself:

```text
libre99asm code/ch40/gridrunner.a99 --format bin -o build/GRIDC.bin
bench99 code/ch40/gridrunner.bench      # -> VR7=>02, FAILID=0
```

To *play* it, wrap the same engine in the frame loop of Chapter 22 — read the
stick into the desired-direction byte, dispatch one PLAY frame, wait for the
vertical interrupt — and run it on the project emulator, a flash cart, or, since
it needs nothing but a console, any unexpanded 99/4A you can find.

## Exercises

**✦ Warm-up**

1. **Recolor the quartet.** Give each pursuer its own sprite color so the four
   personalities are told apart at a glance, then explain why the *pellets* and
   *walls*, sharing a color-table group (Ch. 13), cannot be recolored
   independently without moving their character codes.
2. **Retune the rhythm.** Change `MODLEN` and watch the scatter/chase cadence
   shift. Find a value that feels fair at level 1 and argue for it.
3. **Read the pad.** After a PLAY frame on the bench, dump `>8342` and annotate
   every byte. Confirm the whole live state fits where the spec says it does.

**✦✦ Real work**

4. **A designed maze.** Replace the generated lattice with a hand-drawn maze
   *table* in ROM (the §36.6 habit), keeping `WALLAT`'s signature so nothing else
   changes. Prove your maze is fully connected. How many ROM bytes did the design
   cost, and was it worth it?
5. **Bonus fruit.** Add a bonus item that appears mid-maze for a limited time and
   scores when eaten, using a single pad word for its timer. Extend the self-test
   to prove it scores and expires.
6. **Honest speed tiers.** Make the *enemies* faster than the player at high
   levels while keeping every speed a divisor of eight. Show that actors still
   land exactly on intersections.
7. **A real den.** Give the pursuers a staggered release from the central room so
   they emerge one at a time, using one byte of per-enemy release delay. Verify
   the release order on the bench.

**✦✦✦ Challenge**

8. **The fifth-sprite question.** The genre lives comfortably under the four-per-
   scanline law (Ch. 16), but only just — five actors on one row is possible.
   Demonstrate the case, then decide whether GRIDRUNNER should multiplex, reorder,
   or design the maze so it never happens.
9. **Inky, faithfully.** The FLANK personality here uses a simple mirror. Replace
   it with the classic two-enemy vector construction (target = player-ahead
   reflected through the CHASER's position) and defend, with a bench trace,
   whether the harder rule actually plays better.

## Further Reading

- **Chapter 36** — SKELETON99, the chassis GRIDRUNNER instantiates; the memory-
  budget worksheet of §36.1 is the spec's backbone.
- **Chapter 16** — sprites, the four-per-scanline law, and bounding-box collision,
  all of which GRIDRUNNER leans on directly.
- **Chapter 13** — the character-cell name table and the eight-character color
  rule that shapes GRIDRUNNER's palette.
- **Chapter 9** — the R11 link and the R-16 calling convention; §40.6's three bugs
  are all this chapter's discipline, learned the hard way.
- **Chapter 39** — METEOR BELT, the standard-system capstone whose `>A000` entity
  tables are exactly the luxury this chapter does without.
- *The arcade maze-chase form* — its four-personality pursuer design is widely
  documented in the enthusiast literature; read it for the *why* behind the
  target-tile rules, then measure your own on the bench for the *how*.

## Summary

- **GRIDRUNNER 99** is the book's second complete game and a deliberate constraint
  study: a fixed-screen maze chase that runs on a **bare console** — 256 bytes of
  CPU RAM, no 32K, a single 8K cartridge — where METEOR BELT (Ch. 39) had the
  standard system and parked its tables at `>A000`. The case-study arc (R-19)
  structures the chapter; the famine is the design.
- **Two inventions carry the famine.** The wall map is an *algorithm* (`WALLAT`:
  border plus odd/odd posts, a provably connected lattice) costing **zero bytes**
  of RAM; the pellet state *is the name table* (read a cell with `VSBR` to test,
  write a space to eat), costing **zero bytes** of pad. Only five actors (8 bytes
  each) and ~16 scalars live in the scratchpad — about 140 bytes, all of it state
  that truly changes each frame.
- **The enemy mind is one routine and a table.** `ENAI` picks the legal, non-
  reversing step nearest a **target tile** (squared-Euclidean via `MPY`, no `ABS`
  needed); four personalities (CHASER/AMBUSH/FLANK/SHY) differ only in how
  `TARGET` is filled; a **mode timer** flips scatter↔chase for rhythm; frightened
  mode reuses the same loop with the comparison flipped to *flee*. Difficulty is a
  speed-tier table.
- **Identity is three channels and a reel.** Chomp (ch0), pill (ch1), death noise,
  and a mode-tracking siren (ch2) give the maze a voice; the ATTRACT phase replays
  a **recorded-input** table through the real player path, so the demo *is* the
  game.
- **Verified (BENCH99, commit 408a451):** `gridrunner.a99` → 8,192-byte single
  bank (entry `>61FE`, code+data `>61FE`–`>6F8E` ≈ 3.4 KB, ~4 KB free); an 11-part
  deterministic self-test is **GREEN** (VR7=`>02`, `FAILID`=0). The interactive
  loop (joystick → dispatch → vblank) belongs to the running machine (R-12), as in
  DODGE and METEOR BELT.
- **The lasting lesson (§40.6):** scarcity is an editor. Denied tables, we found
  formulas and used the screen as storage; nesting deep with few registers, we
  relearned that the **R-16 calling convention is a load-bearing contract**, not
  etiquette — three bugs, one cause: a live value a nested call was allowed to
  destroy. Seeds: DUNGEONS OF FATE (Ch. 41, the data-driven RPG that plays a
  database), and Part IX's remaining capstones.
