# Chapter 43 — Capstone V: The Port — Bringing a Modern Game Back in Time

<!-- Part IX — Case Studies: Recreating the Classics · target ≈16 pp -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — pending review passes. DRIFT (an original one-button gravity-flip cavern flyer) MACHINE-VERIFIED on BENCH99 against toolchain commit 01e5b7b: drift.a99 -> 8,192-byte single-bank image (entry >6096), deterministic 4-part self-test GREEN (VR7=>02, FAILID=0) — dispatch routes ATTRACT/PLAY/OVER; no input -> crash the floor (gravity down); one flip -> crash the ceiling (gravity up); flip-every-frame -> hover and survive 20 frames (SHIPY drifts to 93.5 px, exactly as the 8.8 integration predicts). The whole game is Ch.8 fixed-point + Ch.16 sprite + Ch.17 scroll + Ch.36 SKELETON99. Closes Part IX. Code in code/ch43/. Applies R-19/R-16/R-12 + CQ-82; no new ruling. -->
<!-- SPEC: 00-master-outline.md, "### Chapter 43 —" (lines 667–673), Part IX preamble (621–623). -->

## The Long Way Home

Every other capstone in this book looked backward: we took a genre from the
console's own past and rebuilt it clean. This last one turns around. It takes a
*modern* sensibility — the kind of small, sharp, one-more-try design that a phone
or a game jam produces today, all rules and no fat — and carries it the other
way down the timeline, onto silicon that predates it by decades. That is a
different discipline from everything before it, and a fitting final exam, because
you do not get to invent the design as you go. Someone already decided what the
game *is*. Your whole job is translation: to look at a design that assumes free
pixels, infinite color, and a supercomputer in a pocket, and to find, decision by
decision, the honest 1979 equivalent of each thing it takes for granted.

Our patient is **DRIFT**: an original one-button game. A little ship holds a
fixed column of a scrolling cavern; gravity is always pulling it toward one wall;
and the single button *flips which wall gravity pulls toward*. That is the entire
design. There is no fire button, no menu of weapons, no inventory — one bit of
input, pressed or not, and a ship that falls one way or the other. Simple rules,
deep play: the depth is entirely in *timing*, in how late you dare to flip. It is
exactly the kind of design that ports beautifully to a small machine, and by the
end of this chapter it will be running on ours — and you will have the method,
written down as a table, to carry any such design back yourself.

---

## What You Will Learn

After this chapter you can:

- **Choose a design that ports well**, and say why simple-rules/deep-play games
  survive the trip to constrained hardware when spectacle-heavy ones do not.
- **Fill a constraint-translation table** — resolution, palette, sound channels,
  RAM, input — that turns a modern design's assumptions into 4A decisions, one
  row at a time. This is the chapter's reusable artifact.
- **Build a complete game fast** by standing on four parts of accumulated `lib99`
  and SKELETON99, so the new code is only the part that is genuinely new.
- **Compare a port to its green-field siblings** in cycles and bytes, and name
  what porting teaches that building-from-scratch cannot.
- **Sit the book's final exam**: port a design of your own choosing against a
  published rubric — CQ-82, the standard every capstone has met.

## The Bridge: Constraints Clarify

Porting sounds like a downgrade — take the rich thing, throw pixels and voices
and memory away until it fits. But every experienced porter knows the secret:
**constraints clarify.** A design that must survive on three sound channels and
fifteen colors and one button is forced down to its essential idea, and if the
essential idea is good, the stripped version is often *better* — sharper, more
legible, more honest — than the lush original. This is the same truth the whole
book has argued from the other direction: that holding an entire computer in your
head is a feature, not a hardship. Here it becomes a method. You do not port a
modern game by making the 4A pretend to be a modern machine. You port it by
asking, of each thing the design assumes, *what is this really for?* — and giving
the 4A's honest answer. The table that follows is that question, asked six times.

## 43.1 Choosing a Design That Ports

Not every modern game wants to come back in time. A game whose soul is
photorealism, or a hundred simultaneous enemies, or a symphonic score, leaves its
soul at the door of a 9918A. The designs that port are the ones whose soul is a
*rule* — a single clean mechanic that generates depth through timing and nerve
rather than through content. Those games were, in a sense, always vintage games;
they just hadn't been born yet.

DRIFT qualifies on every count. Its input is one bit, which a joystick's fire
button supplies exactly. Its state is a handful of numbers — where the ship is,
how fast it's moving, which way gravity pulls — which fits in the scratchpad with
room to spare. Its motion is smooth, which an 8.8 fixed-point velocity (Ch. 8)
reproduces faithfully. Its world is a cavern that scrolls, which the pattern-shift
trick (Ch. 17) delivers for a kilobyte a frame. And its challenge is timing, which
runs identically at 3 MHz and at 3 GHz because a frame is a frame. The design maps
onto the 4A not by compromise but by *correspondence*: each modern assumption has
a real 1979 counterpart. Finding those counterparts is the work of §43.2.

## 43.2 The Constraint-Translation Table

Here is the chapter's deliverable, and it is not code — it is a *table*, the
artifact you fill in for any port before you write a line. Each row names
something the modern design takes for granted, what the 4A actually offers
instead, and the decision that reconciles them:

| The modern design assumes… | The 99/4A offers… | DRIFT's decision |
|:---------------------------|:------------------|:-----------------|
| free-pixel position, smooth motion | 256×192, 32 sprites, 8.8 fixed-point (Ch. 8) | the ship is one sprite; its momentum is a signed 8.8 velocity |
| a scrolling world of arbitrary art | a 32×24 tile grid; pattern-shift scroll (Ch. 17) | the cavern is tiled walls, scrolled ~1 KB/frame, never redrawn |
| 24-bit color, alpha, particles | 15 fixed colors + transparent, color-by-group | one ship color, a two-tone cavern; a particle trail is a second sprite, or nothing |
| mixed, many-voice audio | 3 square voices + 1 noise (the 9919) | engine hum on a tone channel, the crash on noise, a score blip on another |
| megabytes of RAM | 256 B fast pad + 32 KB expansion | *all* game state lives in the pad — about ten scalars; expansion goes unused |
| a gamepad, a mouse, keys | a joystick and one fire button | the one button is the whole interface: press = flip gravity |

Read the table top to bottom and the port is *specified* — every question answered
before a routine is written. The last row is the one that makes DRIFT DRIFT: the
modern design's single button becomes the joystick's single button, and because
there is only one, the game can be played one-handed, in the dark, by a child —
which is what "simple rules" was always trying to buy. The table did not
impoverish the design. It found its center.

## 43.3 Building Fast on SKELETON99

Here is the reward for four hundred pages of library-building: because the hard
parts already exist, tested, in `lib99`, the *new* code for DRIFT is almost
nothing. The state machine is SKELETON99's (Ch. 36). The fixed-point arithmetic is
Ch. 8's. The sprite is Ch. 16's, the scroll Ch. 17's, the sound Ch. 19's. What is
left — the actual game — is one routine, and it is short enough to read in a
breath:

```asm
UPDATE MOV  @INBTN,R0
       JEQ  UPNOF
       NEG  @GDIR           the one button: flip which way gravity pulls
UPNOF  MOV  @SHIPV,R0
       A    @GDIR,R0        velocity += gravity (a signed 8.8 accel)
       MOV  R0,@SHIPV
       MOV  @SHIPY,R1
       A    R0,R1           position += velocity
       MOV  R1,@SHIPY
       MOV  R1,R0
       SRL  R0,8            the integer part of Y is the pixel row
       CI   R0,TOPY
       JLE  UPCTOP          hit the ceiling
       CI   R0,BOTY
       JHE  UPCBOT          hit the floor
       INC  @DIST           survived another frame
```

That is the whole game. Read it again and notice what it *is*: gravity is a signed
number you add to a velocity; the button negates that number; the velocity adds to
a position; the position, shifted, is a pixel row you compare against two walls.
There is no physics engine, no vector math, no trigonometry — there is Chapter 8,
and a screen. Everything that makes DRIFT feel like smooth flight is the 8.8
fixed-point velocity carrying fractional pixels across frames, exactly the
technique that carried METEOR BELT's meteors and DODGE's dot. The port is small
because the book is large: four parts of accrued craft, spent in one afternoon.

> **Field note — the render is on the machine.** The sprite ship and the scrolling
> cavern are the *interactive* layer (R-12): they belong to the running machine
> under the beam, drawn by `spritelib` and the Ch. 17 scroll each frame. The bench
> proves the ENGINE — the physics, the flip, the collision — by driving it with
> scripted inputs and reading the result, the same division of labor every Part IX
> capstone has used.

## 43.4 Postmortem and Ledger: What Porting Teaches

DRIFT is, by a wide margin, the smallest engine in Part IX. Its entire state is
ten scalars in the pad; its per-frame logic is the fifteen instructions above; it
needs no expansion RAM, no entity tables, no data files. Set beside its siblings
the contrast is the lesson: METEOR BELT integrated five subsystems, GRIDRUNNER
invented a maze from an algorithm, DUNGEONS built a file format, AUTHOR99 a data
structure — and DRIFT, the port, is *smaller than all of them*, because a good
one-button design is almost nothing but a rule, and a rule is cheap on any
machine.

That smallness is the first thing porting teaches, and it is a humbling one:
**depth is not the same as size.** DRIFT is as replayable as anything else in the
book and it fits in a paragraph. The second lesson is subtler. In a green-field
game you discover the design as you build, and the code and the idea grow
together. In a port you receive the design finished, and the creative work moves
*entirely* into the translation table — into the judgment calls about what a
button is *for*, what a color is *for*, what smooth motion is *really* asking. The
port teaches you to see a design as separable from its medium, which is the same
seeing that let DUNGEONS separate an engine from its content and AUTHOR99 separate
a document from a screen. It is, in the end, the book's one recurring idea: find
the essence, and give the machine its honest version.

The self-test proves the essence is intact. With no input the ship falls into the
floor; with one flip it rises into the ceiling; flipped every frame it hovers,
drifting to 93.5 pixels and surviving — the fixed-point integration correct to the
fractional pixel. No new ruling here: this is R-19's arc one last time, R-16 in
every routine, R-12 at the render, and CQ-82 as the bar.

## 43.5 The Graduation Exercise

Part IX ends not with a lecture but with an assignment, because you are ready for
it. The book has now shown you five complete games across five architectures —
cartridge shooter, console-only arcade, disk RPG engine, productivity tool, and
this port — each built the same way: reconstruct honestly, specify precisely,
build on `lib99` and SKELETON99, and prove it on the bench. The final exam is to
do it once more, unaided, on a design of your own choosing. Here is the rubric,
and it is CQ-82, the standard every capstone in this book has met:

- **Responds** to input within a frame; the control feels immediate.
- **Has an attract mode** — a title, a demo, a reason to walk up to it.
- **Keeps visual and sound discipline**: a coherent palette, an identity in three
  voices, no clashing, no silence where sound belongs.
- **Degrades gracefully** where hardware is absent (no speech unit? no crash).
- **Honors QUIT** — always a way back to the title, always a clean exit.
- **Has a difficulty curve** — it starts kind and turns cruel, and the curve is a
  table you can tune.
- **Never crashes** — not on bad input, not on a missing file, not at the edges.
- **Is packaged** — a name, a manual, a build that runs on a real console.

Choose a simple original design — one clean rule, all your own, no borrowed IP.
Fill the constraint-translation table for it before you write a line. Build it on
the library this book gave you. Hold it to the eight points above. When the border
goes green and the game runs on real hardware, you will have done, unaided, the
thing this entire book was written to teach: taken an idea and an empty machine,
and made them meet.

## Lab 43 — Fly It, Then Grow It

Working in `code/ch43/`:

1. **Tune the flight.** Change `GACC` — the gravity constant — and rebuild.
   Predict, before you run, whether a heavier gravity makes the no-input crash
   come *sooner* or *later*, then read the survived-frame count off the bench and
   check yourself. You are tuning feel with a single number, the way the whole
   design intends.
2. **Add the cavern.** Replace the straight test tunnel's fixed `TOPY`/`BOTY` with
   a per-column pair drawn from a small terrain function (a triangle wave, or a
   `WALLAT`-style rule from Ch. 40), and confirm the collision check is unchanged —
   only the walls it checks against moved.
3. **Score the run.** `DIST` already counts survived frames. Render it (Ch. 13
   text over the playfield) and add a high-score compare — the smallest possible
   difficulty loop closed.

## Exercises

**✦ Warm-ups**

1. DRIFT's entire input is one bit. Name two things a modern version of the game
   might do with a second button, and argue whether adding it would deepen the
   design or dilute it.
2. The port maps "smooth motion" to an 8.8 fixed-point velocity. In one sentence,
   say what would go visibly wrong if the ship's position were a plain integer
   instead.
3. Fill one more row of the constraint-translation table: the modern design wants
   a *pause* feature. What does the 4A offer, and what is your decision?

**✦✦ Building**

4. Give the ship a **particle trail** as a second sprite that lags one frame
   behind its position. What does it cost in sprites and cycles, and does the
   four-sprites-per-line law (Ch. 16) ever bite?
5. Implement **graceful QUIT and restart** (CQ-82): from PLAY, a key returns to
   ATTRACT; from OVER, a key starts a fresh flight. Assert the STATE transitions
   deterministically on the bench.
6. Turn the fixed tunnel into a **difficulty curve**: narrow the cavern as `DIST`
   grows, from a table of (distance → gap) rows. Show the curve is data you can
   tune without touching the engine.

**✦✦✦ Reach — the final exam**

7. Port a **simple original design of your own** — one rule, no borrowed IP —
   end to end: fill the constraint-translation table, build it on SKELETON99 +
   `lib99`, and drive a deterministic self-test to a green border. Then write the
   one-page manual and grade yourself against CQ-82's eight points. This is the
   book's graduation piece; there is no larger one.

## Further Reading

- **Chapter 8**, *Arithmetic, Logic, and Bits* — the 8.8 fixed-point that *is*
  DRIFT's physics; read `UPDATE` again with Ch. 8 open and there is nothing left
  unexplained.
- **Chapter 17**, *Motion: Game Loops, Scrolling, and the 60 Hz Contract* — the
  pattern-shift scroll the cavern rides on, and the frame budget the timing lives
  inside.
- **Chapter 36**, *Program Architecture in 16–48K* — SKELETON99 and the CQ-82
  rubric, the chassis and the standard for all five capstones.
- **Chapters 39–42** — the green-field siblings DRIFT is measured against; read
  their postmortems beside this one to feel the difference between inventing a
  design and translating one.

## Summary

DRIFT is the book's last game and its graduation piece: a modern one-button design
— a ship in a cavern, gravity pulling one way, a single button to flip it — carried
back onto 1979 hardware not by compromise but by correspondence. The method is a
table: for each thing the modern design assumes (free pixels, infinite color, mixed
audio, endless RAM, a gamepad), you name what the 4A actually offers and decide the
honest equivalent (a sprite and an 8.8 velocity, fifteen colors, three voices and a
noise channel, ten scalars in the pad, one joystick button). Fill that table and the
port is specified; the code that remains is almost nothing, because four parts of
`lib99` and SKELETON99 already hold the hard parts. DRIFT's whole game is fifteen
instructions of Chapter 8 arithmetic — a signed gravity you add to a velocity you
add to a position you compare to two walls — and the bench proves it exact: no input
falls to the floor, one flip rises to the ceiling, rapid flipping hovers to 93.5
pixels and flies on.

That smallness is the port's deepest lesson: depth is not size, and a good rule is
cheap on any machine. Where a green-field game discovers its design in the building,
a port receives the design whole and moves all the creativity into the translation —
into seeing a design as separable from its medium, which is the same seeing that ran
under every capstone in Part IX. Five games, five architectures, one method:
reconstruct honestly, specify precisely, stand on the library, prove it on the
bench, hold it to CQ-82. The exam is to do it once more on a design of your own. When
your border goes green and your game runs on a real console, the book has done its
work — you have taken an idea and an empty machine, and made them meet.

*Machine-verified against toolchain commit 01e5b7b: `drift.a99` assembles to an
8,192-byte single bank (entry `>6096`); the four-part deterministic self-test paints
VR7 = `>02` (GREEN) with FAILID 0 — the phase table dispatches, gravity-down crashes
the floor, one flip crashes the ceiling, and flip-every-frame hovers the ship to
`SHIPY` = 93.5 px (`>5D80`) and survives 20 frames. This chapter closes Part IX.*
