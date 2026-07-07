# Chapter 16 — Sprites

*Thirty-two objects that move without disturbing the world beneath them — the hardware that made the TI a games machine, and the four-per-line law that made programmers clever.*

<!-- Part III — The Video Display Processor · target ≈26 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The SAL layout, Y+1/>D0/early-clock semantics, size/magnify bits, the law of four (5 sprites on a line → 4 drawn, verified visually), and the DODGE engine (fall/bounding-box collide/respawn) machine-verified on BENCH99 at commit 0d3e5d5 (re-confirmed vs the sibling beam rasterizer bd1bbb6); code in code/ch16/ (spritelib, dodge). Coincidence/5th-sprite STATUS flags and the console auto-motion ISR are described from libre99-core/Classic99 behavior — the bare bench doesn't run the ISR and has no non-destructive status peek; the law of four is verified by the missing 5th sprite instead. Parsec asteroid-belt frame analysis is reserved for the author (see Field Notes). -->

## The Object That Floats

Everything in Part III so far has been *background* — tables the beam reads to paint a fixed picture, redrawn when you want it changed. Move a shape across a Graphics I screen by hand and you feel the cost: you must erase it from its old cell (restoring whatever was underneath) and draw it in the new one, every frame, and if two moving things cross you must sort out who covers whom. It works, but it is bookkeeping, and it is slow.

Sprites are the VDP's gift to anyone who wants things to *move*. A sprite is an object — up to 32 of them — that the chip draws *over* the background, at any pixel position, from its own small pattern, with its own colour, and that you move by writing two bytes: a new Y, a new X. The background underneath is never touched. Nothing smears, nothing needs erasing, nothing needs sorting by hand — the hardware composites the sprites on top of the picture as the beam sweeps, sixty times a second, for free. The first time you set up a sprite and change its X coordinate and watch it glide across a screen that does not so much as flicker, you understand why the TMS9918A made the TI-99/4A a games machine and not merely a computer that could draw.

But the gift comes wrapped in limits, and the limits are the interesting part. There are exactly 32 sprites, no more. Only **four** may appear on any one horizontal scanline; a fifth simply vanishes. Collision detection is a single bit that tells you *some* two sprites touched but not which. And a sprite is one small pattern in one colour, so anything richer — a multi-coloured ship, a large creature — is several sprites flown in tight formation. Forty years of TI games are, at bottom, forty years of programmers being clever *within* these limits: flickering sprites to fake a fifth, stacking them for colour, faking collision with arithmetic. This chapter is the hardware and those techniques, built into `spritelib` and spent on DODGE — our first actual game.

---

## What You Will Learn

- The sprite hardware model: the 32-entry **Sprite Attribute List**, its Y-first order, the `>D0` list terminator, the early-clock bit, and the size and magnification options.
- How sprite patterns are stored and how sprite and character pattern space relate.
- The **law of four** — only four sprites per scanline — the fifth-sprite status it sets, and **flicker multiplexing**, the technique that fakes more.
- Moving sprites smoothly with **8.8 fixed-point** sub-pixel velocity, and the console's built-in automatic-motion feature — how it works and why serious games often bypass it.
- Collision detection: the coincidence flag's honest limits, bounding-box tests, and the hybrid strategy real games use.
- Sprite priority, transparency, and layering against a background.
- Animation: frame tables, ping-pong cycles, and state-driven animation.
- `spritelib` and **DODGE**, a complete little game: player versus meteor field, with collision, respawn, and score.

## The Bridge: Hardware Sprites, Then and Now

A modern 2D engine has no sprite *limit* worth the name. It uploads textures to the GPU and draws them as textured quads — thousands per frame, any size, any rotation, per-pixel alpha blending, all composited by hardware that thinks 49,152 pixels is nothing. "Sprite" survives as a word for a moving image, but the constraints that gave the word meaning are gone.

On the 9918A the word means exactly what it meant in 1979: a small, hardware-composited object with a fixed budget. The chip has dedicated logic that, as it draws each scanline, checks the 32 sprites, finds the (up to four) that touch this line, and overlays their pixels on the background it just computed. That logic is *why* sprites are free to move — the compositing is the beam's job, not yours — and its fixed size is *why* there are only four per line and 32 total: the chip has time to check 32 and buffer 4 per line, and no more. Understanding sprites on this machine is understanding a piece of real-time hardware with a real-time budget, which is a more honest education in what a sprite *is* than any number of GPU quads. The limits are not obstacles to route around on the way to the "real" way; on hardware this old, the limits *are* the real way.

## 16.1 The Sprite Hardware Model

A sprite is described by four bytes in the **Sprite Attribute List** (the SAL), a table of 32 such entries that you place in VRAM and point register 5 at. `spritelib` puts it at `>0300` (register 5 = `>06`). The four bytes, in order, are:

```text
byte 0   Y   vertical position — the line ABOVE the sprite (sprite shows at Y+1)
byte 1   X   horizontal position (0..255)
byte 2   pattern number   which sprite shape to draw
byte 3   early-clock | colour   bit 7 = early clock; low nibble = colour
```

Four features of that layout shape everything you do with sprites.

**Y comes first, and it is the line above.** The vertical coordinate is byte 0 — the chip sorts sprites by line, so it wants Y first — and its value is one *less* than the screen row the sprite appears on. Write `Y = 95` and the sprite's top pixel is at screen row 96. It is a hardware detail (the chip's line counter), and it is the kind of off-by-one that turns into a "why is my sprite one pixel high" bug, so `spritelib`'s `SPUT` takes the screen row you mean and writes `y − 1` for you:

```asm
       MOV  R1,R6
       AI   R6,-1          Y = screen y - 1 (Y is the line above)
       SWPB R6
       MOVB R6,@VDPWD      byte 0: Y
```

**`Y = >D0` ends the list.** The special value `>D0` (208) in a sprite's Y byte is not a position — it is a terminator. The chip scans the SAL from sprite 0 and stops the moment it sees `Y = >D0`; sprites at and beyond that entry do not exist as far as the display is concerned. This is how you tell the chip "I am using six sprites, ignore the other 26": put `>D0` in sprite 6's Y byte. `SINIT` writes it to sprite 0, so a freshly initialized list is empty, and every DODGE and demo caps its list with one. Forget the terminator and the chip scans all 32, drawing whatever garbage the unused entries contain — a classic "phantom sprites at the top-left" bug.

**Colour is per sprite, in the low nibble of byte 3.** One sprite is one colour — the low nibble of its fourth byte — plus transparent for its "off" pixels. There is no per-pixel colour within a sprite; a two-colour object is two sprites.

**The early-clock bit buys the left edge.** Bit 7 of byte 3, the *early-clock* bit, shifts the sprite 32 pixels to the *left* of where its X says. Why would you want that? Because X is an unsigned byte, 0–255, and a sprite at X = 0 is fully on-screen — there is no way to express "half off the left edge" with a position alone. The early-clock bit gives you 32 pixels of off-the-left-edge travel, so a sprite can slide *in* from the left smoothly instead of popping into existence at X = 0. It is a small feature with a specific purpose: smooth entry at the left margin.

Two more options live in register 1, and they apply to *all* sprites at once. Bit 1 (`SZ`) chooses **8×8** sprites (clear) or **16×16** (set) — a 16×16 sprite is four 8×8 patterns arranged in a 2×2 block, consuming four consecutive pattern slots. Bit 0 (`MG`) turns on **magnification**, doubling each sprite pixel to 2×2 on screen, so an 8×8 sprite covers 16×16 screen pixels (blockier, but bigger for free). These are global: you cannot mix 8×8 and 16×16 sprites in one frame without changing the register mid-frame (Ch. 18's territory).

## 16.2 Sprite Patterns and the Descriptor Table

A sprite's *shape* is eight bytes — an 8×8 bitmap, one bit per pixel, exactly like a character glyph (Ch. 13) — living in the **sprite pattern table** that register 6 points at (`spritelib` uses `>1000`, register 6 = `>02`). Sprite byte 2 selects which of the 256 possible patterns to draw. `spritelib`'s `SPAT` loads one:

```asm
SPAT   SLA  R0,3           pattern# * 8
       AI   R0,SPATT       + base -> VRAM address
       LI   R2,8
       BL   @VMBW          R1 = src, R2 = 8
```

For 16×16 sprites the shape is 32 bytes — four 8×8 quadrants in the order top-left, bottom-left, top-right, bottom-right (column-major, a detail worth checking against the datasheet when you first draw one) — and the pattern number steps by four.

Because sprite patterns and character patterns are both just 8-byte bitmaps, a natural question is whether they can *share* a table. They can, if you place them at the same base — sprites and characters would then draw from one pool of 256 shapes — but in practice you usually keep them separate (as `spritelib` does, characters at `>0800`, sprites at `>1000`) so that the 256 character slots and the 256 sprite slots are independent budgets. In a bitmap-mode game (Ch. 15) where the 6 KiB pattern table has eaten the low half of VRAM, sprite patterns move up out of its way; in a Graphics I game with room to spare, the two tables coexist comfortably. The DODGE ball — a single 8×8 shape — is one such pattern:

```asm
BALL   DATA >3C7E,>FFFF,>FFFF,>7E3C     an 8x8 filled ball
```

## 16.3 The Law of Four, and How to Break It

Here is the constraint that defines sprite programming on this chip: **at most four sprites are drawn on any one scanline.** If a fifth sprite would appear on a line, the chip does not draw it — it draws the four lowest-numbered and drops the rest, and it sets a status flag to tell you it happened.

We can watch it directly. `spritelib`'s self-test places *five* ball sprites on the same row (Y = 96) at columns 24, 72, 120, 168, and 216, and asks the `pixels` oracle what the screen actually shows:

```text
2222222f22222222222d22222222222722222222222822222222222222222222
```

Four balls — white (`f`, sprite 0), magenta (`d`, sprite 1), cyan (`7`, sprite 2), red (`8`, sprite 3) — and *nothing* at column 216 where sprite 4 should be. The fifth sprite is gone, exactly as the law says. Lower-numbered sprites win; sprite 4 lost.

When this happens the chip sets the **fifth-sprite flag** (bit 6 of the status register) and records the offending sprite's number in the status register's low five bits, so a program that reads status each frame can *know* it is over budget on some line and which sprite first overflowed. (The bare bench does not run a beam continuously and has no non-destructive status peek, so we verify the law by the missing sprite rather than by reading the flag; a running program on the desktop reads it through the status port, and the console's own interrupt handler reads it every frame.)

Four sprites per line is not many — an asteroid field, a formation of invaders, a swarm of anything wants more. The technique that fakes it is **flicker multiplexing**, and it is beautifully simple: if you have, say, eight sprites that sometimes share a scanline, *rotate which four the hardware draws each frame*. Renumber the sprites — or, cheaper, rotate their order in the SAL — every frame, so that over several frames every sprite gets its turn in the drawn-four. Any given frame shows at most four per line, obeying the law; but because the eye integrates over several frames at 60 Hz, you perceive all eight, each flickering slightly. This is why the asteroid belt in a busy TI shooter *shimmers* — you are seeing the multiplexer share four hardware slots among more objects than fit. It is not a glitch; it is the four-per-line law being outvoted by persistence of vision. The exercises build a multiplexer into `spritelib`.

> **Field Notes — Reading the shimmer.** The way to *see* flicker multiplexing is to slow the machine down. Load a busy game in the project emulator, find a scene with more than four sprites crossing a line — a dense asteroid belt is the classic — and step it frame by frame, counting the sprites actually drawn on the crowded scanline. Some frames you count four of one subset, the next frame four of another; no single frame shows them all, yet at speed they all seem present. Parsec's asteroid belt is the canonical showcase of this trick on the 4A, and a frame-by-frame dissection of it — counting which sprites the multiplexer favours, measuring the flicker's period — is a study this book reserves for its author's hands rather than an agent's. What matters here is the method: when a vintage game's objects shimmer, do not assume a bug; count them per frame, and you will usually find a programmer spending four hardware slots on eight or twelve objects, and getting away with it.

## 16.4 Movement: Sub-Pixel Velocity and the Motion Machine

Moving a sprite is writing its Y and X — `spritelib`'s `SMOV` rewrites just those two bytes — and the naïve way is to add a whole-pixel velocity each frame: `x += 2`. That works, but it quantizes speed to whole pixels per frame, which at 60 Hz means the slowest non-zero speed is 60 pixels a second and the next is 120 — no gentle drift, no fine control. Real games want *sub-pixel* velocity.

The answer is **fixed-point arithmetic**, and the TI convention is **8.8**: represent a position as a 16-bit value whose high byte is the whole pixel and whose low byte is a fraction of a pixel (in 256ths). Velocity is also 8.8. Each frame you add the 8.8 velocity to the 8.8 position — ordinary 16-bit `A` (Ch. 8) — and when you hand the position to the sprite hardware you use only the *high* byte, the whole-pixel part. A velocity of `>0040` (0.25 of a pixel per frame) moves the sprite one pixel every four frames — a slow, smooth drift impossible with integer steps. This is the same fixed-point idea Chapter 8 introduced for fractions and Chapter 36 formalizes for game math; sprites are where it first pays for itself, because the difference between integer and 8.8 velocity is the difference between jerky and smooth motion, visible immediately on screen. DODGE's meteors fall at a whole-pixel speed for simplicity; the exercises convert them to 8.8 and you watch them glide.

There is also a feature you get *for free*, if you want it: the console's firmware includes an **automatic sprite-motion** facility. You fill a table in VRAM (at `>0780` in the standard console layout) with a velocity for each sprite — a Y-speed and an X-speed byte — and enable the feature, and the console's interrupt handler, which runs every frame (Ch. 22), updates every sprite's position from its velocity automatically. You set the velocities once; the sprites move themselves. It is genuinely convenient for simple, constant-velocity objects — a starfield, drifting debris — and it costs you nothing per frame in your own code.

So why do serious games often bypass it? Because it is *only* constant velocity: it cannot accelerate, cannot bounce, cannot steer, cannot respond to the game — it just adds a fixed delta each frame. The moment an object needs to do anything more interesting than drift in a straight line, you are updating its position yourself anyway, and mixing the firmware's automatic motion with your own manual motion on different sprites invites confusion about who owns which position. Many games therefore turn the auto-motion off and move every sprite by hand, in their own loop, where they control the physics completely. The rule of thumb: auto-motion for dumb, constant-velocity background objects; manual motion for anything the player or the game logic touches. (The bare bench does not run the console interrupt handler, so we describe auto-motion from the firmware's documented behavior rather than exercise it here; Chapter 22 installs and drives the ISR properly.)

## 16.5 Collisions: One Honest Bit, and the Boxes Around It

The chip offers collision detection, and it is exactly one bit. The **coincidence flag** (bit 5 of the status register) sets when *any* two sprites have an opaque pixel at the same screen location on the same frame. That is the whole of the hardware's help, and its limits are severe: it tells you *that* two sprites touched, but not *which* two, and not *where*. With 32 sprites potentially in play, "some pair collided somewhere" is rarely actionable — you cannot tell whether the player hit a wall or two enemies grazed each other across the screen.

So real games mostly ignore the coincidence flag and compute collisions themselves, with **bounding boxes**: treat each sprite as a rectangle and test whether two rectangles overlap. For 8×8 sprites, sprites A and B collide when `|Ax − Bx| < 8` and `|Ay − By| < 8` — two subtractions, two absolute values, two compares. DODGE's `TICK` does exactly this, meteor against player:

```asm
       LI   R2,PLYRY
       S    R1,R2          R2 = PLYRY - MY
       ABS  R2
       CI   R2,8
       JHE  TNOHIT          |dy| >= 8 -> miss
       MOV  @MXS(R7),R2
       S    @DPX,R2        R2 = MX - PX
       ABS  R2
       CI   R2,8
       JHE  TNOHIT          |dx| >= 8 -> miss
       INC  @DHITS          overlap on both axes -> a hit
```

We verified it fires: DODGE starts meteor 0 directly above the stationary player, runs the engine, and the collision counter (`DHITS`) reads 1 — the meteor fell onto the player and was caught, on exactly the tick their boxes overlapped. Bounding boxes know *which* objects collided and *where*, which is what a game needs to award a point, lose a life, or bounce a ball.

The **hybrid** strategy uses both: read the cheap coincidence flag as a fast "did *anything* collide this frame?" early-out, and only when it is set do you spend cycles on the bounding-box tests to find out *what*. On a frame where nothing touched — most frames — you skip the box tests entirely. It is a nice pattern: a cheap hardware hint gating expensive-but-precise software, and it is exactly how you would structure a collision system on any machine where a coarse test is cheaper than a fine one.

## 16.6 Priority, Transparency, and Layering

When sprites overlap each other, **lower-numbered sprites win** — sprite 0 draws over sprite 1, which draws over sprite 2, and so on down to sprite 31. This is a fixed priority, and it is a design tool: put the things that must stay visible (the player, the cursor) in low-numbered slots, and the things that may be occluded (background debris) in high ones. It is also the mechanism behind multi-sprite objects: a two-colour ship built from sprites 0 and 1 relies on their fixed ordering to layer correctly.

Against the *background*, sprites always draw on top — a sprite is foreground by definition — except where the sprite's pixel is transparent (its "off" bits, and any pixel of colour 0). Transparency is what makes a sprite an irregular shape rather than an 8×8 rectangle: the ball's corner pixels are "off," so the background shows through them and the ball reads as round. This is the same colour-0-is-a-hole rule we met in multicolor mode (Ch. 14), now doing the work of shaping every sprite. It also means you can layer sprites deliberately — a small bright highlight sprite over a larger body sprite — and the transparent regions of the top one let the bottom show through, building a richer object out of the flat, single-colour primitives the hardware gives you.

## 16.7 Animation: Making Sprites Come Alive

A moving sprite is not yet an *animated* one — motion changes where a sprite is, animation changes what it *looks like*. On this hardware, animation is changing a sprite's pattern number over time, and the techniques are a small, durable vocabulary.

The simplest is a **frame table**: a list of pattern numbers that make up an animation cycle — a walking figure's four poses, a flame's three flickers — and a per-sprite counter that advances through the list every few frames, writing the current pattern number into the sprite's SAL byte 2. Advance the counter, wrap it at the end of the list, and the sprite loops its animation. A **ping-pong** cycle walks the list forward then backward (1, 2, 3, 2, 1, 2, 3, …) for animations that should reverse rather than snap — a pulsing glow, a pendulum. And a **state-driven** controller picks *which* frame table to run from the object's game state: the player sprite runs its "walking" table while moving, its "standing" table while still, its "jumping" frame while airborne — the animation follows the logic. These are not TI-specific ideas; they are how sprite animation works everywhere. What is TI-specific is the cheapness: changing a sprite's appearance is writing one byte, so even elaborate animation costs almost nothing, and the budget you must watch is the pattern *table* (how many distinct shapes fit in its 2 KiB) rather than the CPU time to animate them.

## Lab 16 — `spritelib` and DODGE

The lab is the sprite engine and the first game, both in `code/ch16/`, both machine-verified.

**`spritelib` (`spritelib.inc` + `spritelib.a99`)** — the sprite engine for `lib99`: `SINIT` (point registers 5 and 6, empty the list), `SPAT` (load an 8×8 shape), `SPUT` (place a sprite — screen Y, X, pattern, colour), `SMOV` (move one, position only), `SHIDE` (park one off-screen). Build and prove it:

```sh
libre99asm code/ch16/spritelib.a99 --format bin -o build/SPRC.bin --symbols build/spr.map.json
```

Its self-test puts five sprites on one scanline; `pixels 4` shows four and the SAL dump (`vram 0300 18`) shows all five entries — the law of four, made visible. `vdp` shows the green verdict (sprite 0's Y byte read back as 95).

**`dodge.a99`** — our first game, or rather its engine: a player ball and four falling meteors, each meteor wrapping to a fresh pseudo-random column when it reaches the floor, and a bounding-box collision that costs a life. The interactive layer — steering the player with the keyboard or joystick — is Chapter 21's, and the frame-timed loop is Chapter 17's; here the player holds still and meteor 0 starts above it, so the collision logic is deterministic and provable. Run it (`x 200000`) and check `DHITS` (`m 834C 2`) reads 1 and `pixels 4` shows the player and the fallen meteors. In Chapter 17 DODGE grows a real loop; in Chapter 19 it gets sound; by Chapter 21 you can play it.

## Exercises

**16.1** ✦ A sprite's SAL entry is `5F 40 02 07`. Where does it appear on screen, what shape does it draw, and what colour is it? (Watch the Y+1 rule.)

**16.2** ✦ Why must a sprite list end with a `>D0` Y-byte? Describe precisely what appears on screen if you place four sprites and forget the terminator, given that unused VRAM is zero.

**16.3** ✦✦ Add `SMAG`/`SSIZE` helpers to `spritelib` that set the magnification and size bits in register 1 (preserving the other bits — recall Ch. 14's warning about clobbering register 1). Verify a 16×16 magnified sprite covers 32×32 screen pixels with `pixels`.

**16.4** ✦✦ Convert DODGE's meteors to **8.8 fixed-point** vertical velocity: store each meteor's Y as 8.8, its fall speed as 8.8, add per tick, and pass only the high byte to `SMOV`. Give one meteor a fractional speed (e.g. `>0180` = 1.5 px/frame) and confirm it moves smoothly between whole-pixel rows.

**16.5** ✦✦ Implement flicker multiplexing: place eight sprites on the same scanline and, each tick, rotate their SAL order so a different four are the lowest-numbered. Verify with `pixels` across successive ticks that the drawn four change.

**16.6** ✦✦ DODGE's collision uses `< 8` on both axes for 8×8 balls. Make the collision box *tighter* than the sprite (say `< 5`) so a near-miss grazes past, and argue why a forgiving box (tighter than the art) usually feels fairer to a player than an exact one.

**16.7** ✦✦✦ Give the DODGE player a two-frame animation (two ball shapes, alternating every 8 ticks) driven from a frame table, and a second, brighter highlight sprite layered over it (lower sprite number) whose transparent pixels let the body show through. This exercises §16.6 and §16.7 together.

**16.8** ✦✦✦ Build a hybrid collision system: on each tick, first assume the coincidence flag is set (since the bench can't read it live, simulate "flag set" every frame), and only then run the bounding-box tests — but structure the code so that a real program could skip the box tests on frames where the flag is clear. Measure, with `cycles`, the per-frame cost of the box tests you would skip.

## Further Reading

- *TMS9918A Video Display Processor Data Manual*, Texas Instruments — the sprite attribute and pattern formats, the size/magnify bits, and the four-sprites-per-line and coincidence behaviors behind this chapter.
- *Editor/Assembler Manual*, Texas Instruments — the console's automatic-motion table and the sprite utilities the firmware provides (§16.4).
- Chapter 14 (Multicolor) — colour 0 as transparency, the rule that shapes every sprite (§16.6).
- Chapter 17 (Motion) — the frame loop DODGE is waiting for, and where sub-pixel motion and the beam meet.
- Chapter 21 (Input) — the keyboard and joystick reading that makes DODGE playable.
- Chapter 22 (Interrupts) — the console ISR that drives automatic sprite motion, installed and controlled.

## Summary

Sprites are the VDP's 32 hardware-composited moving objects, drawn over the background at pixel positions with no background redraw. Each is four bytes in the **Sprite Attribute List** (register 5): Y (the line *above* the sprite — `SPUT` writes screen-y minus one), X, pattern number, and early-clock-plus-colour; a Y byte of `>D0` ends the active list, and the early-clock bit shifts a sprite 32 pixels left for smooth entry. Size (8×8 / 16×16) and 2× magnification are global bits in register 1. Shapes are 8-byte bitmaps in the sprite pattern table (register 6). The defining constraint is the **law of four**: at most four sprites draw per scanline, the lowest-numbered winning and the rest dropped (verified — five sprites on a line show four) while a status flag records the overflow; **flicker multiplexing** rotates which four are drawn each frame to fake more, at the cost of shimmer. Smooth motion uses **8.8 fixed-point** velocity — whole pixel in the high byte, 256ths in the low — added per frame, with only the high byte handed to the hardware; the console's firmware offers free automatic constant-velocity motion, which serious games bypass for anything that must accelerate, bounce, or respond. Collision hardware is one honest bit (the coincidence flag: *some* pair touched, but not which), so games compute **bounding-box** overlaps themselves — DODGE's `|dx| < 8 && |dy| < 8`, verified firing — optionally gated by the flag as a cheap early-out. Lower-numbered sprites have priority, transparency (colour 0) shapes each sprite and lets sprites layer, and animation is changing the pattern number over time via frame tables, ping-pong cycles, and state-driven controllers. `spritelib` packages the engine and DODGE spends it: a player, four falling meteors, respawns, and collision — our first game, waiting only for the loop of Chapter 17 and the input of Chapter 21.
