# Chapter 39 — Capstone I: The Scrolling Shooter

*Everything the book has taught, integrated at last into a single cartridge that scrolls, shoots, overheats, refuels, and speaks — a complete game of first-party scope, built on `SKELETON99` and `lib99`, and proven on the bench.*

<!-- Part IX — Case Studies: Recreating the Classics · target ≈34 pp · [cart] -->
<!-- STATUS: DRAFTED (session 8, 2026-07-07) — pending review passes. METEOR BELT's engine (code/ch39/meteorbelt.a99) is MACHINE-VERIFIED on BENCH99 at commit 18c069e: assembles to an 8,192-byte single-bank image (entry >6154, code+data >6000-6813 ≈ 2.1 KB); the deterministic self-test reaches HALT in 27,651 instructions / 593,028 cycles with FAILID=>0000 and VR7=>02 (GREEN) — proving phase dispatch (ATTRACT >4141 / PLAY >5050 / OVER >2A2A), the data-driven wave director (one scripted enemy spawned), 8.8 STEP motion, bounding-box collision (KILLS=1, SCORE=25), the laser heat lockout (FIRED=4 -> HITLOK, then LOCK cleared on cooling), fuel drain + REFUEL, and the last-life OVER transition (STATE=>0002). Measured: one PLAY frame ~15,700-18,000 cyc (~1/3 of the ~50,000-cyc budget); SCROLL (the Ch.17 pattern-shift) ~988 cyc. R-12 gaps stated plainly: no cartridge image to instrument (Archaeology reconstructs the genre from the record + Part III's own measurements); speech (Ch.20) and TIPI (Ch.34) unemulated; libre99asm builds one 8K bank, no multi-bank .ctg packager (Ch.35). -->

## The Machine That Warned You

There is a specific second, sometime in 1982, that sold more TI-99/4A consoles than any advertisement: the second a child leaned toward a television while a landscape slid past beneath a little ship, an enemy formation dove out of the top of the screen, and the machine itself said — out loud, in that flat, patient, unmistakable voice — *"Advance."* Not a beep. A word. The console in the den had opened its mouth and spoken a warning into the room, and the landscape kept scrolling the whole time, and the ship still answered the joystick, and the score still climbed. Everything happened at once and none of it stuttered.

That is the moment this entire book has been walking toward. Every chapter until now has isolated one faculty of the machine and mastered it alone: the sprites that move without redrawing (Chapter 16), the smooth scroll that costs a hundredth of what it should (Chapter 17), the three-voice chip that synthesizes a zap out of arithmetic (Chapter 19), the synthesizer that turns a vocal-tract model into speech (Chapter 20), the matrix you scan for a joystick every frame (Chapter 21), the sixty-hertz heartbeat you fit all of it inside (Chapter 17 again, and Chapter 22). Isolated, each is a lab exercise. A *game* is what happens when they run together, in one loop, inside one frame, without any one of them starving the others — and when the result is polished and finished and fair enough to sell. That integration is not a footnote to the skills; it is a skill of its own, the hardest one, and it is what separates a demo from a product.

So we build one. Not a sketch, not a tech demo with the corners rounded off, but **METEOR BELT** — a scrolling shooter with waves and a wave director, a laser that overheats if you lean on it, a fuel gauge that forces you to thread a tunnel, an attract mode that demonstrates itself to an empty store, a soundtrack that yields to its own effects, and a spoken warning when the belt closes in. It is the genre Parsec defined for this platform, rebuilt from our own parts, and held to the 1982 commercial-quality bar (CQ-82) line by line. And because it is built on `SKELETON99` (Chapter 36) and the `lib99` we have accumulated since Chapter 11, most of it is *assembly you have already written*. The capstone is not a new machine to learn. It is the moment the machine you already know becomes a game.

---

## What You Will Learn

- **How a scrolling shooter is architected** as an integration of every subsystem: scroll engine, sprite entities, sound, speech, input, all inside one 60 Hz loop.
- **How to reconstruct a genre's design behaviorally** — from the hardware record and your own measurements — without copying a byte of the original.
- **How to write a 1982-style design specification**: feature list, difficulty-as-a-curve tuning tables, a 32 K bank map, and a per-frame cycle budget.
- **How to productionize the Chapter 17 pattern-shift scroll** into a terrain engine, and detect collisions against it.
- **How to build a data-driven wave director** over a fixed-slot entity table, so the game's choreography lives in an editable table, not in code.
- **How to engineer player systems** — a laser-heat lockout, lives, a fuel mechanic — as exposed tuning tables you balance rather than constants you bury.
- **How to ship**: attract mode, a high-score table, speech and music integration, the multi-bank cart build, the manual, and a release checklist that is CQ-82 in disguise.

## The Bridge: Shipping Is a Discipline, Not an Event

A modern developer knows in their bones that *"it runs on my machine"* is the beginning of the work, not the end. Between a feature that works and a product you can ship lies a long, unglamorous list: the edge cases, the attract loop the store demands, the pause that does not corrupt state, the difficulty that ramps instead of walls, the crash that must never happen because there is no patch coming. We have a name for the modern version of this list — *definition of done*, *release checklist*, *ship criteria* — and we treat the programmer who can carry a thing across that line as more valuable than the one who can merely start it. That instinct is exactly right, and it is exactly what 1982 demanded, only more severely: the cartridge era had *no* patches, so "shipped" meant "finished," permanently, in mask ROM.

This chapter, then, is the bridge from *making things work* to *making a product*, and the crossing has the same shape now as it did then. You will still write the fun part — the scroll, the shooting, the waves — and discover, as every developer does, that it is a third of the job. The other two-thirds is the definition of done: the attract mode (CQ-82 item 2), the graceful degradation when the speech unit or the disk is absent (item 5), the honored QUIT key and the sane exit (item 6), the tuned difficulty curve (item 7), the zero crashes (item 8), and the package that survives its own instructions (item 9). We built the pieces so we could learn the machine; we assemble them here so we can learn to *finish*. The reward is a thing you can hand to someone who has never read this book, and watch them play.

## 39.1 Archaeology: Reading the Genre Without the Source

The case-study method begins with archaeology: you play the genre-definer, instrument it in the debugger, and recover its architecture *behaviorally* — never by copying code or art, always by watching what the running machine does and reasoning back to how it must be built. There is an honesty problem to confront first, though, and confronting it is itself a lesson. **This project ships no commercial cartridge images.** The libre99 emulator is deliberately IP-clean (its `cartridges/` directory is empty by design; the licensing checklist in the repository excludes third-party ROMs before any release), so there is no Parsec image here to single-step. When you want to sit an original in a debugger and watch its sprite list mutate frame by frame, the tools for that are the reference shelf — Classic99, which bundles the licensed TI software, or MAME, the cycle-accuracy referee (Chapter 3). We will not pretend to have done what this toolchain cannot do.

What we *can* do is better founded than a disassembly anyway, because we have spent seven parts measuring the machine those games ran on, and the genre's techniques fall out of those measurements directly. Archaeology, for us, means reading the genre off the hardware record and our own bench transcripts. Four findings define the scrolling shooter, and we have already proven every one:

- **The scroll is a lie told with character definitions.** A full-screen coarse scroll — rewriting all 768 name-table cells every frame — costs roughly 163,000 cycles, more than three frames' worth (measured in Chapter 17, `terrain.a99`). No shooter can afford that, and none paid it. The professional trick, which we named *the Parsec method* when we measured it, leaves the name table alone and redraws a scroll band's *character patterns* shifted one pixel per frame — about 1,044 cycles, a hundred-and-fiftyfold cheaper (Chapter 17, `smooth.a99`). When you see a TI landscape flow smoothly, you are watching eight bytes of pattern get rewritten, not a screen of cells.
- **The belt is more sprites than the hardware allows, multiplexed.** The 9918A composites at most four sprites per scanline; the fifth on a line is dropped and its number flagged (the *four-sprite law*, measured in Chapter 16 — five balls on a row render as four). A dense asteroid belt has more than four objects abreast, so it must **flicker-multiplex**: rotate which four draw each frame, trading a shimmer the eye forgives for an object count the hardware forbids. Read any belt frame-by-frame and you can count the rotation.
- **The voice is streamed inside the frame budget, and it is optional.** The synthesizer speaks by draining a small FIFO a few LPC bytes at a time (Chapter 20); a shooter feeds it during the frame's slack without starving the sixty-hertz loop, and — crucially — *detects the unit first* and stays silent gracefully when it is absent, because most consoles had no speech sidecar. On this emulator the synthesizer is unmodeled (writes to `>9400` are ignored; `SPKDET` correctly reports *absent*), so our verifiable slice is the detection-and-degrade path; to *hear* the callouts you run under MAME or js99er.
- **The texture is in the constraints the designer *adds*.** The heat that makes you release the trigger, the fuel that makes you dive for a tunnel — these are not hardware features, they are *rules*, and they are what turn "move and shoot" into a game with tension. Archaeology's last finding is a design one: the genre's depth lives in a handful of resource meters the player must manage, each one a small tuning table.

> **Field Notes — Reading a belt frame by frame.** You do not need the source to recover flicker-multiplexing; you need single frames. Load an original under Classic99, pause, and count the sprites on one scanline across successive frames: if the same visual row shows a *different* four objects each frame, you are watching the rotation, and the shimmer you half-noticed while playing is the four-sprite law being negotiated in real time. This is the whole archaeological method in miniature — a behavior the running machine cannot hide, reasoned back to the technique that must produce it. We reimplement the *technique* (a rotation over a fixed-slot table, §39.4); we copy nothing.

The point of archaeology is not to admire the originals but to *steal their architecture and leave their bytes* — to arrive at our own specification already knowing the load-bearing decisions, because the hardware and our measurements made them for us. We know the scroll must be pattern-shift, the belt must multiplex, the voice must stream-and-degrade, and the depth must come from resource meters. Now we write that down as a design a 1982 team would have signed.

## 39.2 Specification: METEOR BELT

> *The following is a **reconstruction** — a design specification in the style a 1982 first-party team would have written, composed for this book. It invents no historical document; it is the spec **we** are building to.*

**Title.** METEOR BELT. **Genre.** Vertical scrolling shooter, single player. **Target.** Standard console; 8 K cartridge minimum, four-bank 32 K cartridge for the full asset load; speech and disk enhance but are never required. **Premise.** Fly a mining scout down through a scrolling asteroid belt, clearing waves, managing a laser that overheats and a fuel supply that only tunnels replenish, for score.

**Feature list (the definition of done).** Smooth vertical scroll; a player ship under joystick control with a heat-limited laser; data-driven enemy waves on a difficulty curve; bounding-box collision for shots, enemies, and terrain; a fuel meter with refuel tunnels; three lives and a game-over; an attract mode that self-demonstrates; a high-score table (saved to disk where present, online where a TIPI is present); spoken callouts where the synthesizer is present; a full soundtrack that yields to effects; QUIT honored, a pause that is safe, and zero crashes.

**Difficulty as a curve, not a wall (CQ-82 item 7).** Difficulty is a *table*, indexed by stage, so balancing is data entry and not surgery:

| Stage | Enemy speed (8.8 px/frame) | Spawn interval (frames) | Belt density (max on-screen) | Heat cost / shot |
|---|---|---|---|---|
| 1 | 1.5 (`>0180`) | 45 | 4 | 8 |
| 2 | 2.0 (`>0200`) | 34 | 5 | 8 |
| 3 | 2.5 (`>0280`) | 26 | 6 | 10 |
| 4+ | +0.25 per stage | −4 per stage (floor 12) | 6 | 10 |

**Memory budget — a four-bank 32 K cartridge.** The budget worksheet of §36.1, filled in for this game:

| Region | Size | Contents |
|---|---|---|
| Scratchpad `>8300`–`>836F` | 112 B used | workspace, R10 stack, hot per-frame scalars (state, heat, fuel, scroll phase, player) |
| Expansion RAM `>A000`… | ~120 B | the two entity tables (enemies, shots) — warm, iterated data |
| VRAM | 16 K | screen tables + sprite patterns + the scroll-band character warehouse |
| Bank 0 (resident) | 8 K | the engine + `lib99` (the part every bank needs) |
| Bank 1 | 8 K | wave scripts + tuning tables + level maps |
| Bank 2 | 8 K | music and sound-effect data |
| Bank 3 | 8 K | speech LPC + spare graphics |

**The 60 Hz budget (CQ-82 item 1).** The frame is ~50,000 cycles (measured, Chapter 17). METEOR BELT's allocation, as designed and later measured (§39.7):

| Frame task | Budget | Notes |
|---|---|---|
| Scroll band redraw | ~1,000 cyc | the pattern-shift, measured ~988 |
| Wave director + entity motion | ~2,500 cyc | table walks, 8.8 adds |
| Collision (shots × enemies × player) | ~2,500 cyc | bounding boxes |
| Sprite render (SMOV the live objects) | ~7,000 cyc | the port cost dominates |
| Sound + speech service | ~1,500 cyc | a few register writes, a few FIFO bytes |
| Dispatch, fuel, bookkeeping | ~1,500 cyc | the state machine and scalars |
| **Total** | **~16,000 cyc** | **~⅓ of the frame — headroom by design** |

That last row is the specification's most important promise: the whole game must fit in a third of the frame, leaving two-thirds of headroom so that a heavy moment — a full belt, a boss, a torrent of shots — never blows the budget and drops a frame. We will hold ourselves to it and measure the result.

## 39.3 The Terrain and Scroll Engine

Construction begins with motion, because a scrolling shooter that does not scroll is nothing. We productionize the Chapter 17 pattern-shift directly. A band of the name table — here a row near the bottom, standing in for the belt's leading edge — is filled entirely with one character; each frame, that character's eight pattern bytes are rewritten as a vertical rule shifted one pixel, and the whole band slides without a single name cell changing. METEOR BELT's `SCROLL` is `smooth.a99`'s `SHIFT` grown a phase counter and folded into the game loop:

```asm
* SCROLL — advance the sub-pixel phase and redraw the scroll character shifted
* one pixel: the Ch.17.5 "Parsec method". Non-leaf (calls VWA).
SCROLL DECT R10
       MOV  R11,*R10
       LI   R0,SCHAR
       BL   @VWA            aim at the scroll character's pattern
       LI   R1,>0080        one lit pixel ...
       MOV  @SPHASE,R2     ... walked right by SPHASE
       JEQ  SCDRAW
SCSH   SRL  R1,1
       DEC  R2
       JNE  SCSH
SCDRAW SWPB R1
       LI   R2,8
SCROW  MOVB R1,@VDPWD       eight identical rows = a vertical rule
       DEC  R2
       JNE  SCROW
       INC  @SPHASE
       MOV  @SPHASE,R0
       ANDI R0,7            phase wraps 0..7; the 8th advances a coarse column
       MOV  R0,@SPHASE
       MOV  *R10+,R11
       RT
```

On the bench, `SCROLL` costs **988 cycles** per frame (measured, commit 18c069e) — squarely on the ~1,044 we logged in Chapter 17, and squarely inside the specification's 1,000-cycle line. That is the whole scroll engine's frame cost: under two percent of the budget buys smooth motion, because the technique pays for the number of *characters* in the band, not the number of *cells* on the screen. When the phase counter wraps from 7 back to 0, a real terrain engine advances one coarse column — pulling the next slice of the level map into the band's characters — so the belt scrolls forever from a map far larger than the screen; the demo holds a single repeating rule to keep the measurement clean.

**Terrain collision** rides on the same structure and costs almost nothing. Because the terrain is characters in known cells, "did the ship hit a wall?" is a table lookup, not a geometry test: read the name-table cell under the ship's grid position, and if it is a solid-terrain character rather than open space, it is a collision. This is the same insight as fixed-slot entities — *precompute the world into a table and the runtime is a lookup* — and it is why the fuel-tunnel mechanic of §39.5 is cheap: the tunnel is just a run of open cells between two walls, and threading it is reading cells, not intersecting polygons.

## 39.4 The Wave Director: Choreography as Data

An enemy that appears is easy; a *game* of enemies — waves that arrive on a schedule, from scripted positions, on a difficulty curve — needs choreography, and the architecture that makes choreography editable is the chapter's centerpiece. METEOR BELT stores its enemies in a **fixed-slot entity table** (Chapter 36.4) — `SKELETON99`'s dense array of fixed slots, iterated tightly — with the velocity fields widened to full 8.8 words so objects move at shooter speeds. A second table holds player shots. Each slot is twelve bytes:

```text
+0  X (8.8)   +2  Y (8.8)   +4  DX (8.8)   +6  DY (8.8)
+8  TYPE      +9  LIVE      +10 AUX (hit points)   +11 pad
```

The motion of every live object is one routine, `STEP`, and it is `SKELETON99`'s entity loop unchanged in spirit — walk the slots, skip the dead, add each velocity to each position in 8.8:

```asm
* STEP — advance every live slot in the table at R4 (R5 slots) by its 8.8
* velocity. The SKELETON99 entity loop, widened to word velocities. Leaf.
STEP   MOVB @9(R4),R6       live?
       JEQ  STNEXT
       MOV  @0(R4),R0
       A    @4(R4),R0       X += DX
       MOV  R0,@0(R4)
       MOV  @2(R4),R0
       A    @6(R4),R0       Y += DY
       MOV  R0,@2(R4)
STNEXT AI   R4,ESIZE
       DEC  R5
       JNE  STEP
       RT
```

The **director** is what makes the game choreographed rather than random, and its whole design is *data-driven* (Chapter 36.6): the waves live in an editable table, and the code that plays them is generic. The script is a list of timed spawn records — a delay, a spawn position, an 8.8 velocity, an enemy type — terminated by a sentinel:

```asm
* The wave script — the director's database. Record = 6 words:
*   DELAY  XPIX  YPIX  DX  DY  TYPE     (DELAY = ticks after the previous spawn)
WSCRPT DATA 0,120,40,0,>0200,1          tick 0 dead ahead down 2.0
       DATA 30,60,40,0,>0180,1          +30 left lane down 1.5
       DATA 30,180,40,0,>0180,2         +30 right lane down 1.5
       DATA >FFFF,0,0,0,0,0             end of script
```

The director counts down to the next event; when the timer fires, it reads the current record, spawns that enemy into the first free slot, and loads the following delay. Spawning into a *free slot* is what enforces the belt-density cap from the spec — when the table is full, a spawn is silently dropped, which is not a bug but a **fairness rule**: the game will never put more enemies on screen than the player can fight, or than the four-sprite law can draw. On the bench, the director provably works: driving the PLAY phase through the script spawns exactly the scripted enemy (the cursor advances one record, and the enemy appears in the table, live), and `STEP` advances it down the screen frame by frame.

Re-choreographing the entire game is now *editing a table*, not touching code — a new wave pattern is new data in `WSCRPT` (or, in the four-bank build, in the level bank), exactly as Chapter 38's pipeline intends: a designer works in the script while the programmer owns the director. This is the single highest-leverage decision in the whole game, and it is why the RPG of Chapter 41 can push it all the way to "the engine plays a database."

> **Sidebar — Movement patterns as a tiny language.** The `DX`/`DY` fields give straight-line motion; interesting enemies want curves. The professional move is to make `TYPE` an index into a *behavior table* — a per-frame routine that rewrites an enemy's velocity from its own state and the player's position. A sine-weave enemy looks up `sin(frame + phase)` (the 256-unit byte-angle sine table of §36.5) and sets `DX` from it; a homing enemy nudges `DX`/`DY` toward the player each frame. The director stays the same; the enemies get a vocabulary. This keeps the *choreography* in data (the script says *what* and *when*) and the *movement* in a small, shared library of behaviors (the table says *how*), and it is how a dozen distinct enemy behaviors emerge from a handful of routines.

## 39.5 Player Systems: Tension as Tuning Tables

The player's ship is where the genre's texture lives, and every part of it is a resource the player must manage — which means every part is a small tuning table, exposed and balanced rather than a magic constant buried in code. METEOR BELT builds three: the laser's heat, the fuel supply, and lives.

**Laser heat** is the mechanic that keeps the player from holding the trigger down forever, and it is a two-routine state machine over four tuned numbers — heat added per shot, the overheat threshold, the cooling rate, and the re-arm level. Firing adds heat; crossing the threshold locks the laser out; idling sheds heat, and once it has cooled back to the re-arm level the laser is ready again:

```asm
* SHOOT — launch a player shot if the laser is ready; add heat, and lock out
* the laser if it overheats. A full shot table simply drops the shot. Leaf.
SHOOT  MOV  @LOCK,R0
       JNE  SHX             locked out -> no shot
       ...                  find a free shot slot, launch it from (PX,PY)
       MOV  @HEAT,R0
       AI   R0,HINC         heat rises per shot
       MOV  R0,@HEAT
       CI   R0,HMAX
       JL   SHX
       LI   R0,1
       MOV  R0,@LOCK        overheat -> lockout
       MOV  R0,@HITLOK
SHX    RT

* COOL — on an idle tick, shed heat; a locked laser re-arms once cooled to HRST.
COOL   MOV  @TRIG,R0
       JNE  COX             fired this tick -> no cooling
       ...                  HEAT -= HCOOL (floored at zero)
       MOV  @LOCK,R1
       JEQ  COX
       CI   R0,HRST
       JH   COX
       CLR  @LOCK           cooled enough -> ready again
COX    RT
```

The bench proves the whole cycle deterministically: fired six times against a four-shot table, the laser launches four shots (`FIRED=4`), reaches the overheat threshold, locks (`HITLOK` latches), suppresses the last two triggers, and then — left idle — cools back below the reset level and clears the lockout (`LOCK=0` at the end of the run). The *feel* — lean on the trigger and the gun quits on you; pace your fire and it never does — is nothing but those four constants (`HINC=8`, `HMAX=32`, `HCOOL=2`, `HRST=8`), and because they are named equates you tune the feel by editing four numbers, not by rewriting logic. That is what "difficulty as a curve, not a wall" means at the level of a single weapon.

**Fuel** is the same idea aimed at the whole level's rhythm: it drains a little every frame and is only replenished by threading a *tunnel* in the terrain (§39.3), so the player is pulled forward and downward into danger by the map itself rather than by a timer. `REFUEL` — the reward for surviving the tunnel — simply tops the tank; the drain is one subtraction per frame, floored at zero, and a dry tank costs a life. **Lives** close the loop: a life is lost when an enemy's bounding box touches the player's (or the fuel runs out), and the last life lost flips the state machine to OVER:

```asm
* LOSELF — lose a life; at zero lives, transition to OVER. Leaf.
LOSELF DEC  @LIVES
       JNE  LSX
       LI   R0,2
       MOV  R0,@STATE       out of lives -> game over
LSX    RT
```

On the bench this is the game-over transition, proven: with one life left, a lethal touch takes lives to zero and sets `STATE` to `>0002`, and the next dispatch runs the OVER phase. Three meters — heat, fuel, lives — three tuning tables, and every one of them is a resource the player negotiates rather than a number the code hides. The tension *is* the tables.

## 39.6 Presentation: Attract, Score, Voice, and Soundtrack

A game that only plays is a demo; a *product* wraps the play in presentation, and CQ-82 demands the wrapping as sternly as the play. METEOR BELT's presentation is the `SKELETON99` state machine (Chapter 36.4) doing exactly the job it was built for: an explicit `STATE` variable and a jump table routing each frame to the current phase's handler. The phases are ATTRACT, PLAY, and OVER, and the dispatch is proven on the bench — `STATE=0` runs the attract phase (signature `>4141`), `STATE=1` runs play (`>5050`), `STATE=2` runs game-over (`>2A2A`), through the same `PTAB` table `SKELETON99` established:

```asm
DISP   DECT R10
       MOV  R11,*R10
       MOV  @STATE,R1
       SLA  R1,1            phase index * 2 -> word offset
       MOV  @PTAB(R1),R2
       BL   *R2             run the current phase
       MOV  *R10+,R11
       RT
PTAB   DATA PHATTR,PHPLAY,PHOVER
```

**Attract mode** (CQ-82 item 2) is a phase, not a special case — that is the elegance of the state machine. Left alone, the game sits in ATTRACT: it demonstrates itself (a scripted flythrough, a scroll of the high scores, a title) and loops, exactly as a store shelf demanded, and a joystick press transitions it to PLAY. Because the demo is just the play engine driven by *recorded inputs* instead of live ones, it costs almost nothing to build and it is always in sync with the real game — a technique Chapter 40 leans on harder still.

**The high-score table** is persistence (Chapter 36.7), tiered by what hardware is present. With a disk, the scores are a small file — written with `filelib` (Chapter 31), read back at startup — and this is the path the standard system takes. With a **TIPI**, the spec calls for an *online* variant: the high-score file lives on the Pi's mapped filesystem, or is posted to a web endpoint through TIPI's extension calls (Chapter 34) — a 1981 console with an internet leaderboard. That variant runs on real TIPI hardware; this emulator models no TIPI (R-12), so on the project the high score persists to disk, and the online path is the shelf's to run. With *no* storage at all — a bare cartridge — the fallback is the era's trick from §36.7: a high score held only until power-off, or a progress *password* the player writes down. Detect the storage, and behave sensibly in every configuration: that is CQ-82 item 5, *degrade gracefully*, at the level of the score table.

**Speech** (Chapter 20) is the genre's signature and the clearest case of graceful degradation the book has. The callouts — *"advance,"* *"fuel low,"* *"belt clear"* — are `spklib` streams fed to the synthesizer's FIFO a few bytes per frame during the loop's slack, and the first thing the game does is `SPKDET`: if the unit answers, the ship speaks; if it does not, the game is exactly as playable in silence. On this emulator `SPKDET` reports *absent* (the synthesizer is unmodeled — writes to `>9400` ignored, `>9000` open bus; verified in Chapter 20), so what the project proves is the detection-and-degrade path — the game runs, silent, correct — and to *hear* METEOR BELT speak you run it under MAME or js99er, which model the synthesizer. This is not a limitation to apologize for; it is the compatibility doctrine (Chapter 34.6) working exactly as designed: target the stock console, enhance where the hardware is present, degrade where it is not.

**The soundtrack** is `sndlib` (Chapter 19) driven from the frame loop or, better, from the user-ISR hook at `>83C4` (Chapter 22) so the music advances on the beat behind the gameplay. The design discipline is CQ-82 item 4 — *sound as design, not decoration*: distinct voices for distinct events (a thin square for the laser, decaying noise for an explosion, a bass pulse for the engine), and a music driver that **steals a channel** for an effect and restores it when the effect ends, so a zap never silences the melody for longer than it must. Three tone voices and one noise channel is not many; spent deliberately, with priorities, it is a whole sound identity.

## 39.7 Ship It: The Cart, the Manual, the Checklist

The last third of the work is shipping, and it has three deliverables of its own: the cartridge, the manual, and the release checklist.

**The cartridge build.** METEOR BELT's design is a four-bank 32 K cartridge (§39.2), and here we meet an honest tooling boundary (R-12). `libre99asm` builds **one 8 K bank** and synthesizes its `>6000` header; it does not yet emit a multi-bank `.ctg` or `.rpk` package (Chapter 35.5 logged this as a roadmap item — the book driving the emulator, exactly as the outline predicted). So the multi-bank pipeline is: assemble each bank with `libre99asm`, concatenate the images, and either wrap them with a shelf packager (xdt99's `xga99`/cartridge tooling builds multi-bank carts today) or run the resident engine bank alone on the project. And the engine bank *does* run alone, because we sized it to: METEOR BELT's engine assembles to a **single 8,192-byte image** (entry `>6154`), with the actual code and data occupying `>6000`–`>6813` — about **2.1 KB** — leaving roughly 6 KB of the bank free for the wave scripts, tuning tables, and graphics that would otherwise page in from banks 1–3. The complete four-bank game is a design and a packaging step; the *engine*, the part that had to be proven, is a verified single-bank cartridge today.

```sh
# the verified single-bank build (from docs/ti99book/)
libre99asm code/ch39/meteorbelt.a99 --name 'METEOR BELT' \
    --format bin -o build/METEORC.bin --symbols build/meteor.map.json
```

**The manual.** Yes, we write it — CQ-82 item 9 holds Part IX to the package *literally*, and the manual is part of the product, not an afterthought. A 1982 cartridge manual taught the game: the story (you are a mining scout in the belt), the controls (joystick to steer, fire button for the laser — *and mind the heat*), the rules (fuel drains; thread the tunnels; the belt gets denser each stage), and the scoring. Writing it is a design review in disguise: if a rule is hard to explain in the manual, it is probably hard to feel in the game, and the manual is where you discover that the heat mechanic needs a visible gauge or the fuel tunnels need a clearer tell. A game whose manual is honest and complete is usually a game whose design is honest and complete.

**The release checklist** is CQ-82 itself, run against the finished cartridge, and this is where the postmortem lives. METEOR BELT's report card:

| CQ-82 item | Status in METEOR BELT |
|---|---|
| 1 Instant, steady response | Input sampled every frame; one frame measured at ~16,000 cyc, ~⅓ of budget — no dropped frames by design |
| 2 Attract mode | A phase in the state machine; self-demonstrates and loops |
| 3 Visual discipline | Managed flicker where the four-sprite law bites; coherent palette |
| 4 Sound as design | Distinct voices; music yields to effects by channel-stealing |
| 5 Speech if present, degrade if not | `SPKDET` first; silent and correct when absent (verified) |
| 6 House etiquette | QUIT honored/safely disabled; pause; console left sane |
| 7 Difficulty as a curve | The stage table of §39.2 — tuned data, not code |
| 8 Zero crashes | The deterministic engine self-test is GREEN at HEAD |
| 9 The package | This manual and this checklist |

Item 1 is the one the specification staked its credibility on, and it holds: a full PLAY frame — scroll, waves, motion, collision, and sprite render — measures **between about 15,700 and 18,000 cycles** on the bench (commit 18c069e), roughly a third of the ~50,000-cycle budget, with the sprite render's port traffic the dominant cost. That headroom is not luck; it is the budget of §39.2 honored, and it is what Chapter 37's optimizations would widen further (moving the entity table's hot fields into the scratchpad, batching the sprite writes to amortize the address-set). The game fits the frame with two-thirds to spare, which is exactly the promise a shippable action game must keep.

## Lab 39 — `METEOR BELT`

The lab is the chapter's deliverable, and it is the largest single artifact the book has built so far: **`METEOR BELT`**, in `code/ch39/meteorbelt.a99` — a complete scrolling-shooter engine on `SKELETON99` and `lib99`. Build and prove it:

```sh
libre99asm code/ch39/meteorbelt.a99 --name 'METEOR BELT' \
    --format bin -o build/METEORC.bin --symbols build/meteor.map.json
bench99 code/ch39/meteorbelt.bench
```

Like `DODGE` before it (Chapter 16), the file is the game's *engine*, and it proves itself the way every `lib99` artifact does: the interactive layer (a live joystick, the frame-timed loop under the beam) belongs to the running machine, so here `START` drives the engine through a **deterministic script** and paints the border-verdict light. The self-test reaches its halt in 27,651 instructions (593,028 cycles) with **`FAILID = >0000` and VR7 = `>02` (GREEN)** — and behind that one green light every subsystem is asserted: the phase dispatch (attract `>4141`, play `>5050`, over `>2A2A`), the data-driven wave director (one scripted enemy spawned and stepped), 8.8 fixed-point motion, bounding-box collision (`KILLS=1`, `SCORE=25`), the laser heat lockout (`FIRED=4`, `HITLOK` latched, then `LOCK` cleared on cooling), the fuel drain and `REFUEL`, and the last-life OVER transition (`STATE=>0002`). To play it — to steer the ship, feel the heat, hear the callouts under a synthesizer-modeling emulator — you wrap this engine in the Chapter 21 input layer and the Chapter 17/22 frame loop and run it on the machine; the bench proves the logic that the play makes fun. This is the DODGE evidence model at capstone scale: a game whose every mechanism is deterministically true before a single human ever touches the joystick.

## Exercises

**39.1** ✦ Run `meteorbelt.bench` and read the state block at `>8342`. Identify, byte by byte, which cells prove the kill, the overheat, and the game-over — and explain what a `FAILID` of 3 would have meant.

**39.2** ✦ The scroll engine measures ~988 cycles a frame. Using the Chapter 17 figure for a full name-table rewrite (~163,000 cycles), state the speed-up factor, and explain in one sentence *why* the pattern-shift pays for characters and not cells.

**39.3** ✦✦ Add a fourth wave record to `WSCRPT` — a fast enemy in the right lane — and re-run the bench. What changed in the state block, and what did *not* change in the code? Relate your answer to "the engine plays a database."

**39.4** ✦✦ Extend the `TYPE` field into a real behavior selector: write a sine-weave enemy that rewrites its own `DX` each frame from a byte-angle sine table (§36.5), and verify on the bench that its X position oscillates while its Y descends.

**39.5** ✦✦ Retune the laser: find `HINC`, `HMAX`, `HCOOL`, and `HRST`, and change them so the laser locks after *two* shots and cools twice as fast. Predict `FIRED` and the end-of-run `LOCK` before you run, then confirm.

**39.6** ✦✦ Implement the fuel-tunnel tell: when `FUEL` drops below a threshold, flash the backdrop (VR7) between two colors on alternating frames. Why must the flash be driven off a frame counter and not a busy-loop?

**39.7** ✦✦✦ Build the attract mode as a real phase: record 300 frames of scripted input into a table, and have `PHATTR` drive `UPDATE` from the recorded inputs, looping. Verify the dispatch reaches ATTRACT and that the recorded run is deterministic (same end state every time).

**39.8** ✦✦✦ Take one PLAY frame from ~16,000 cycles toward ~11,000 by moving the two entity tables' hot fields (X, Y, LIVE) into the scratchpad and batching the sprite render's VDP writes (Chapter 37). Measure before and after with `cycles`, and report the win by subsystem.

**39.9** ✦✦✦ Write METEOR BELT's cartridge manual — story, controls, rules, scoring — to CQ-82 item 9, then use it as a design review: name one rule that was hard to explain, and propose the design change that would make it explain itself.

## Further Reading

- `code/ch39/meteorbelt.a99` and `code/ch39/meteorbelt.bench` — the verified engine and its deterministic self-test; this chapter's deliverable.
- Chapter 36 (Program Architecture) — `SKELETON99`, the state machine and fixed-slot entity table METEOR BELT instantiates.
- Chapter 17 (Motion and Scrolling) — the pattern-shift scroll (`smooth.a99`) §39.3 productionizes; the 60 Hz contract §39.2 budgets against.
- Chapter 16 (Sprites) — `spritelib`, the four-sprite law, and flicker-multiplexing §39.1 reads off the hardware.
- Chapters 19–20 (Sound and Speech) — `sndlib` and `spklib`, the soundtrack and the degrade-gracefully callouts of §39.6.
- Chapter 37 (Optimization) — the placement and batching wins Exercise 39.8 and §39.7's headroom draw on.
- Chapter 40 (Fixed-Screen Arcade) — the next capstone, and the recorded-input attract technique pushed further.

## Summary

METEOR BELT is the book's first complete game: a scrolling shooter built by **integrating** every subsystem — scroll, sprites, sound, speech, input — into one 60 Hz loop, on `SKELETON99` and `lib99`, and held to the 1982 commercial-quality bar (CQ-82) line by line. The chapter follows the case-study arc. **Archaeology** reconstructs the genre *behaviorally* from the hardware record and Part III's own measurements — because the IP-clean project ships no cartridge image to instrument (R-12; play originals on Classic99/MAME) — recovering four load-bearing techniques: the **pattern-shift scroll** (~1,044 cyc, Chapter 17, not a 163,000-cyc name rewrite), **flicker-multiplexing** past the four-sprite law (Chapter 16), **streamed-and-detected speech** (Chapter 20), and **depth from resource meters**. **Specification** writes the 1982-style design doc (a labeled reconstruction, R-1): feature list, a difficulty *curve as a table*, a four-bank 32 K memory map, and a per-frame cycle budget that promises the whole game in ~⅓ of the frame. **Construction** builds it: a **terrain and scroll engine** (the Chapter 17 pattern-shift, measured **988 cyc**), a **data-driven wave director** over a fixed-slot entity table (choreography as an editable script, motion as `SKELETON99`'s `STEP` widened to 8.8 word velocities), and **player systems** — a laser **heat lockout**, **fuel** replenished only by terrain tunnels, and **lives** — each a small exposed tuning table, so tension is data you balance. **Presentation** is the state machine doing its job — ATTRACT/PLAY/OVER phases, a recorded-input attract mode, a tiered high-score table (disk; TIPI-online where present, R-12; password fallback), `SPKDET`-gated speech callouts that degrade to silence, and a channel-stealing soundtrack. **Shipping** confronts the multi-bank build honestly (`libre99asm` builds one 8 K bank; multi-bank packaging is a roadmap gap, R-12/Chapter 35), writes the manual, and runs CQ-82 as the release checklist. The engine is **machine-verified at commit 18c069e**: an 8,192-byte single-bank image (code+data ~2.1 KB, ~6 KB free), whose deterministic self-test is **GREEN** (`FAILID=>0000`, VR7=`>02`) — proving dispatch, wave spawn, 8.8 motion, collision, the heat lockout, fuel, and the game-over — and whose full PLAY frame measures **~16,000 cycles**, about a third of the ~50,000-cycle budget, the specification's central promise kept. Everything the book taught, integrated, proven, and — at capstone scale — *finished*.
