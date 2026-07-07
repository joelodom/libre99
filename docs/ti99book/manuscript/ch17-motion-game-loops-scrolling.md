# Chapter 17 — Motion: Game Loops, Scrolling, and the 60 Hz Contract

*The frame is the machine's heartbeat and your budget both — fifty thousand cycles, sixty times a second, and everything you want to move must fit inside one beat.*

<!-- Part III — The Video Display Processor · target ≈26 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The ~50,000-cycle frame (measured in boot mode), the full-screen coarse-scroll cost (~163,000 cyc ≈ 3.3 frames), and the pattern-shift smooth-scroll cost (~1,044 cyc, verified the line walks >80->40->20) machine-verified on BENCH99 at commit 0d3e5d5 (frame count re-confirmed vs bd1bbb6). Code in code/ch17/ (terrain = coarse scroll + budget, smooth = pattern-shift). Interrupt/poll-sync loop shapes and the VDP address-race pitfall are described with code; the F-flag wait and the ISR need the running beam (boot/desktop/Ch. 22), which the bare bench doesn't drive per-instruction. -->

## Sixty Times a Second, No Matter What

Every game you have ever played runs on a loop, and the loop runs to a clock. On the TI, that clock is the video beam. Sixty times a second the beam finishes painting the screen's 192 visible lines and drops into the vertical blanking interval — the brief darkness before it snaps back to the top and starts again — and at that instant the VDP raises a flag. That flag is the machine's heartbeat. It is the signal that one frame is over and the next may begin; it is the moment when the screen is not being drawn and so is safe to change; and the interval between two of its beats is the entire time your game has to think, move everything, and update the display before the beam comes around again.

We can measure that interval exactly. Boot the console and run the machine one frame — the bench does this and counts cycles — and the answer is **50,014 cycles**; run ten frames and it is 500,117, an average of 50,011.7 per frame. Call it fifty thousand cycles. That is the number this chapter is really about. Fifty thousand cycles of a 3 MHz processor is your whole budget for one frame of a game: read the controls, update the player and every enemy and every bullet, run the physics, detect the collisions, and push all the resulting changes into VRAM through that narrow port — all of it, in fifty thousand cycles, or you miss the beat and the game stutters.

Everything in Part III has been building to this moment, because motion is where the beam stops being a diagram and becomes a deadline. Chapter 15 measured that clearing the bitmap costs seventeen frames; this chapter will measure that scrolling the whole screen the obvious way costs *three*, and show you the technique the professionals used to do it in a *fraction* of one. The 60 Hz contract is strict and it does not negotiate. The art of real-time programming on this machine is learning to live inside it — and, occasionally, to cheat it so cleverly that the player never notices the frame was too small.

---

## What You Will Learn

- The frame as the unit of time: the VDP interrupt flag, the vertical-blank window, and the **~50,000-cycle** budget every design negotiates with.
- The two orthodox game-loop shapes — **interrupt-driven** and **poll-synchronized** — and how to choose between them.
- Separating **update** from **render**, batching VRAM writes into the safe window, and dirty-rectangle bookkeeping.
- **Coarse scrolling** by rewriting the name table — and the measured reason full-screen coarse scrolling does not fit a frame.
- **Smooth character scrolling**: the pattern-shift technique (the Parsec method), built and measured, and why it is ~150× cheaper.
- Vertical, split, and parallax scrolling; page flipping under motion.
- A **time system** for `lib99`: frame counters, timers, and scheduled events.
- The classic crash: the **VDP address-register race** between an interrupt handler and the main loop.

## The Bridge: From Delta-Time to a Fixed Beat

A modern game loop is written to be independent of the frame rate. It measures the real time elapsed since the last frame — the *delta time* — and advances the simulation by that amount, so the game runs the same whether the hardware delivers 30, 60, or 144 frames a second. Vsync, when enabled, hands the finished frame to a compositor that displays it at the monitor's refresh; the GPU does the drawing; and the CPU's job is mostly to decide *what* to draw, with cycles to spare.

The TI has none of that machinery and needs none of it, because its frame rate is not variable — it is *sixty*, fixed by the video standard, forever. There is no delta time because every frame is the same length: fifty thousand cycles. There is no compositor and no GPU; the beam is the display and your code is the renderer. And there are no cycles to spare — fifty thousand is not much, and a busy frame spends every one. So the TI game loop is simpler in shape than a modern one and far stricter in budget: do a fixed amount of work, synchronized to a fixed beat, or fail visibly. This is, if anything, a *cleaner* way to learn what a game loop is, because the fixed beat makes the budget concrete. You are not optimizing against a vague "60 fps would be nice"; you are fitting inside 50,000 cycles, a number you can measure your code against on the bench, exactly as this chapter does.

## 17.1 The Frame as the Unit of Time

The heartbeat is a single bit. At the end of each frame's active display the VDP sets bit 7 of its status register — the **F** flag, for frame (Ch. 12). If VDP interrupts are enabled (register 1 bit 5), setting F also pulls the CPU's interrupt line, requesting service; either way, F stands set until someone reads the status register, which clears it. That read-clears-it behavior is not incidental — it is the acknowledgement, the thing that says "I have seen this frame's beat," and exactly one piece of code should do it per frame or the beats get miscounted (§17.2, and the pitfalls at the chapter's end).

The interval between beats is the frame, and we measured it: ~50,000 cycles. It is worth internalizing what fits in that. A register-to-register instruction is around 14 cycles, so a frame is roughly 3,500 of the fastest instructions — but most real work is slower, touching memory and especially the VDP port, and a single VRAM byte written through the port is ~40 cycles (Ch. 12). So a frame is more like *twelve hundred VDP writes*, total, for everything. That is the budget. A sprite moved is two VDP writes; thirty-two sprites moved is sixty-four, nothing. A screenful of tiles rewritten is 768, most of your frame gone. The whole discipline of this chapter follows from holding those two numbers — 50,000 cycles, ~40 per VDP write — in your head and measuring every technique against them.

## 17.2 Two Loop Shapes

There are two orthodox ways to synchronize your game to the beat, and TI programs use both.

The **poll-synchronized** loop keeps interrupts off and watches the flag itself. Each pass through the loop does all the game's work and then spins, reading the status register, until F comes up — then clears it and goes around again:

```asm
GLOOP  BL   @UPDATE          move everything (game logic)
       BL   @RENDER          push the changes into VRAM
WAITF  MOVB @VDPST,R0        read status ...
       COC  @FMASK,R0        ... is bit 7 (F) set?
       JNE  WAITF            no -> keep spinning
       JMP  GLOOP            yes (F now cleared by the read) -> next frame
```

Its virtue is control: nothing runs behind your back, the whole machine is yours, and the timing is dead simple to reason about — the loop runs exactly once per frame because it waits for exactly one flag. Its cost is that the CPU spins in `WAITF` doing nothing whenever the frame's work finishes early, and that *you* must guarantee the work always fits in a frame, because there is no safety net. Many cartridges chose this shape precisely because they wanted the whole machine and no surprises.

The **interrupt-driven** loop turns VDP interrupts on and lets the beat *call you*. The main code runs freely — or even halts — and the VDP interrupt fires sixty times a second, running a handler that does the per-frame work. This is how the console's own firmware is built (Ch. 22): its interrupt handler runs every frame to scan the keyboard, service sound, and move auto-motion sprites, whether or not your program asked. The virtue is that background work happens automatically and the main code can do other things between frames; the cost is that an interrupt can fire in the *middle* of anything your main code is doing, including a two-byte VDP address setup — which is the race that ends this chapter, and the reason interrupt-driven code needs careful critical sections.

Neither shape is drivable on the bare bench, because the bench steps CPU instructions without running the beam, so F never comes up and `WAITF` would spin forever. We verify the *work* a loop does — the updates, the scrolls, the budget — statically and by measurement, and exercise the F-synchronized loop on the desktop emulator and under the real ISR in Chapter 22. The shapes above are the skeletons; the flesh is the update and render they call.

## 17.3 Update and Render, and the Safe Window

The single most important structural decision in a game loop is to separate **update** from **render**. Update is pure computation: advance positions, run physics, decide collisions, change state — all in CPU RAM, touching no VDP. Render is the opposite: take the results update produced and push them into VRAM, touching no game logic. Keeping them apart buys two things. First, clarity — the game's rules live in one place and its drawing in another. Second, and on this machine crucially, it lets you put all the VDP traffic in one contiguous burst that you can aim at the **safe window**.

The safe window is the vertical blank — the interval after F sets and before the beam starts painting again. Writes to VRAM during active display are not forbidden, but they compete with the beam for VRAM access and, worse, a change made mid-frame shows *immediately*, from the next scanline down (Ch. 12's no-framebuffer truth), so an object updated while the beam is above it and again below it can tear. Batching all your VRAM writes to happen just after F, in the blank, means the beam is not drawing while you change things and the whole updated screen appears at once on the next pass. Update during the frame (in CPU RAM, safe anytime); render in the blank (into VRAM, all at once). That rhythm is why the loops in §17.2 do `UPDATE` then `RENDER` then wait for F, not the reverse.

The other half of fitting the budget is to **never render what did not change** — dirty-rectangle bookkeeping. A game does not rewrite the whole screen every frame (it cannot; §17.4 measures why). It tracks which cells or which small regions changed this frame — the player moved, a bullet advanced, a score digit ticked — and renders only those. Most of a frame's 768 cells are identical from one frame to the next; a game that renders only the handful that changed spends tens of VDP writes where a naïve redraw would spend hundreds, and stays inside the beat. Chapter 15's bitmap lesson — touch few pixels — is the same lesson in tiles: touch few cells.

## 17.4 Coarse Scrolling, and Why the Whole Screen Won't Fit

Scrolling is motion of the *background* — the world sliding past a fixed viewport — and it is where the 9918A's most consequential limitation surfaces: **the chip has no hardware scroll register.** Later video chips (the 9938 of Ch. 18, every console after) let you set an offset and the hardware shifts the picture for free. The 9918A does not. If you want the background to move, you must move the bytes yourself.

The coarsest way is to shift the name table. To scroll the world left by one whole cell, every cell's content moves one column left and a fresh column feeds in at the right. Our `terrain` demo does exactly this over a field of colour stripes, rewriting all 768 name-table cells from a moving offset each tick, and the `pixels` view confirms the stripes march left one column per tick. It works. And it is far too slow. Measured on the bench, one full-screen rewrite costs:

**~163,000 cycles — about 3.3 frames.**

Read that against the 50,000-cycle budget and the verdict is brutal: you cannot rewrite the whole name table in a frame. Not close. A game that tried to scroll the full screen this way would update the background once every three or four frames — a lurching, 15–20 Hz crawl beneath whatever sprites glide smoothly on top at 60. This is not a failure of the demo; it is the arithmetic of 768 cells at ~40 cycles of VDP traffic each plus the address math, and it is *why* full-screen coarse scrolling is simply not done on the 9918A.

The escapes are three. **Scroll a band, not the screen:** rewrite only the rows that need to move — a 32-cell strip is 32 writes, not 768 — and leave the rest still, which suits a game with a scrolling play-field and static panels. **Scroll less often:** move a whole cell every few frames rather than smoothly, accepting chunky motion for a strategy game or a slow vista. Or — the real answer, and the one that makes the TI's scrolling shooters possible — **do not scroll the name table at all.** Scroll the *patterns*.

## 17.5 Smooth Scrolling: The Pattern-Shift Method

Here is the trick that beats the beam. The name table is expensive to rewrite because it is 768 cells; but the *pattern* table is only 256 characters, and a scrolling band typically uses far fewer than that. So instead of moving which character sits in each cell, you leave the name table alone and **redraw the characters themselves, shifted by one pixel.**

Picture a horizontal band of cells that all name the same handful of "scroll" characters. Each frame, you take those characters' 8-byte definitions and shift every byte one bit sideways, feeding in the next column of the world's pixels at the edge. The name table never changes — the same characters sit in the same cells — but because their *shapes* have shifted one pixel, the whole band appears to slide one pixel. Do it eight times and the band has moved a full character width; at that point you do *one* cheap coarse step (a single name-table shift of just that band) and reset the pixel phase. The result is smooth, one-pixel-per-frame scrolling — the motion of Parsec's landscape, of every good TI side-scroller — built not by moving tiles but by animating them.

Our `smooth` demo shows the mechanism at its simplest: a band whose cells all name one character, whose pattern is redrawn each tick as a vertical line one pixel further along. The bench confirms the pattern byte walking `>80 → >40 → >20` as the phase advances — the line, and the whole band with it, moving one pixel per frame. And the cost, measured, is the entire point:

**~1,044 cycles** to redraw the character and shift the band one pixel — against ~163,000 for the full-screen rewrite, and comfortably inside the 50,000-cycle frame.

That is a **150-fold** difference, and it is the difference between a game that scrolls and a game that lurches. The pattern-shift method wins because it pays for the number of distinct *tiles* in the band (a handful of characters, a few dozen bytes) instead of the number of *cells* on the screen (768). A real scrolling background uses more than one scroll character — enough to draw the varied terrain, redrawn each frame with the shared one-pixel shift and the map's next column fed in — but the economics hold: even thirty-two scroll characters redrawn is 256 bytes of pattern, ~10,000 cycles, a fifth of a frame, with room left for the sprites and the game. This is the technique to reach for, and the measured reason to reach for it.

## 17.6 Vertical, Split, and Parallax

Everything in §17.5 has a vertical twin. **Vertical smooth scrolling** shifts the scroll characters' bytes *up or down* within their 8-byte definitions instead of sideways within each byte — the same redraw-the-pattern idea rotated ninety degrees — and feeds a new pixel row at the edge every frame, with a coarse row-shift of the band every eighth. The direction changes; the economics do not.

**Split scrolling** exploits the beam's one-line-at-a-time nature (Ch. 12): because a mid-frame change to the tables takes effect from the next scanline down, you can make the *top* of the screen scroll at one rate and the *bottom* at another by changing a base register partway down the frame — a technique that needs precise timing (you must make the change during a specific scanline's worth of CPU time) and that Chapter 18 develops as the mid-frame register trick. Even without touching registers mid-frame, you can fake depth with **parallax**: scroll a foreground band by pattern-shift every frame and a background band every *other* frame, and the slower-moving background reads as farther away. `terrain`'s two-rate stripe fields are the crudest version; a real parallax layers distant mountains drifting slowly behind near ground rushing past, each a pattern-shift band on its own clock. And the cheapest depth trick of all is **colour cycling** — not moving anything, just rotating the colours in the colour table so a band of "water" or "lava" appears to flow while every pixel stays put. Motion, on this machine, is often an illusion cheaper than the real thing.

## 17.7 Page Flipping Under Motion

Chapter 13 built a double buffer for Graphics I: two name tables in VRAM, draw into the hidden one while the other is displayed, then flip which is live by rewriting the name-table base register (register 2) — one register write, atomic, tear-free. Under motion, that flip is your friend. Render the whole next frame — scrolled band, moved score, everything — into the off-screen name table during the frame, then, in the safe window, flip register 2. The beam never sees a half-drawn frame because the frame it is drawing is the *finished* one and the one you were building was hidden. It is the closest the TI comes to the compositor's guarantee.

The catch is memory. Two name tables cost 768 bytes each — cheap in Graphics I, where VRAM is roomy. In bitmap mode (Ch. 15) there is no room for a second 6 KiB canvas, so page flipping is a Graphics-I-and-text luxury; bitmap games live with single-buffered drawing and careful timing instead. Which is one more reason the tile modes, not the bitmap, are where TI action games actually live: the tile modes can double-buffer and pattern-shift-scroll, and the bitmap can do neither within a frame.

## 17.8 A Time System for `lib99`

Games need to measure time, and the frame is the natural unit. The heartbeat gives `lib99` a small, durable time system, built from one idea: a **frame counter** incremented once per beat. From that everything follows.

The counter itself is a word bumped each frame — in the poll loop after the F wait, or in the interrupt handler. Sixty counts is a second; the counter is your clock. **Timers** are countdown words: to do something in two seconds, set a timer to 120 and decrement it each frame, acting when it hits zero. A table of such timers, decremented together each frame and firing a routine when each expires, is a **scheduled-event** system — spawn a wave of enemies in 90 frames, flash the score for 30, end the level at 3,600 — the whole choreography of a game expressed as counts of the beat. None of it needs a real-time clock or any hardware beyond the flag; the beat is the clock, and counting beats is timing. The sketch is a dozen instructions — a counter incremented, an array of countdowns walked and tested — and the exercises fold it into DODGE so the meteors spawn on a schedule and the game has a sense of time.

## Lab 17 — TERRAIN: Scrolling, Measured

The lab is the two scroll techniques and the budget that separates them, both in `code/ch17/`, both machine-verified.

**`terrain.a99`** — coarse scrolling by name-table rewrite: 32 colour stripes scrolled one column per tick from a moving offset. Build it, then watch it move and measure it:

```sh
libre99asm code/ch17/terrain.a99 --format bin -o build/TERRC.bin --symbols build/terr.map.json
```

On the bench, `u SNAP` to draw a frame and `pixels 8` to see the stripes; step past `SNAP` and `u SNAP` again to advance the offset and watch them march. Bracket one tick with `cycles` and read the cost: **~163,000 cycles**, over three frames — the measured proof that full-screen coarse scrolling does not fit.

**`smooth.a99`** — the pattern-shift method: a band of cells sharing one character, whose pattern is redrawn one pixel further each tick. `vram 0808 1` shows the pattern byte walking `>80 → >40 → >20`; the band slides one pixel per frame; and the same `cycles` bracket reads **~1,044 cycles** — 150× cheaper, inside the frame. Run them back to back and the lesson is undeniable in the numbers.

**The frame-budget HUD** the outline calls for — a live readout of cycles spent this frame — is the natural next step and a Chapter 22 collaboration: with the profiling timer of §22.7 you can measure a frame's real cost on the running machine and paint it as a bar, turning the static measurements here into a live gauge. TERRAIN's full flyover — parallax bands, sprites over the scroll, the HUD — assembles the whole of Part III and is the capstone the exercises build toward, now that every piece has been measured.

## Exercises

**17.1** ✦ A frame is ~50,000 cycles and a VDP write is ~40. Roughly how many VRAM bytes can you write in one frame if writing is *all* you do? How many whole 8-byte characters is that?

**17.2** ✦ Write the poll-synchronized wait loop from §17.2 and explain, precisely, why exactly one place in the program may read the status register per frame.

**17.3** ✦✦ `terrain` rewrites all 768 cells per tick (~163,000 cycles). Modify it to scroll only a 4-row band (rows 10–13), rewriting 128 cells, and measure the new cost with `cycles`. Does it fit a frame now? By how much?

**17.4** ✦✦ Extend `smooth` to a real two-character band: two scroll characters whose patterns you shift in lockstep, so the band shows a repeating two-tile texture sliding smoothly. Confirm with `pixels` and measure the per-frame cost.

**17.5** ✦✦ Implement vertical smooth scrolling: redraw a band's character shifting its 8 bytes *up* one row each tick (byte *i* takes byte *i*+1's value, a new row fed at the bottom). Verify the band drifts upward one pixel per tick.

**17.6** ✦✦ Add a `lib99` time system: a frame counter and an 8-entry timer table, `TSET n frames` and a `TTICK` that decrements all timers and returns which expired. Drive it from a poll loop (or, on the bench, tick it by hand) and confirm a timer set to 3 fires on the third tick.

**17.7** ✦✦✦ Give DODGE (Ch. 16) a real poll-synchronized loop and the time system: meteors spawn on a schedule (one every 45 frames), the game runs at a steady 60, and a frame counter drives a difficulty ramp (meteors fall faster every 600 frames). Measure that the whole frame's work fits in 50,000 cycles.

**17.8** ✦✦✦ Colour-cycle a "waterfall": a vertical band of cells whose colour table you rotate each frame so the band appears to flow, without moving a single pixel of pattern. Measure its per-frame cost and compare it to pattern-shift scrolling the same band — which is cheaper, and why?

## Further Reading

- *TMS9918A Video Display Processor Data Manual*, Texas Instruments — the status register, the F flag, and the vertical-blank timing that defines the frame.
- Chapter 12 (Inside the TMS9918A) — the beam, the no-framebuffer model, and the ~40-cycle VDP write these budgets are built from.
- Chapter 13 (Graphics I) — the double buffer §17.7 flips under motion.
- Chapter 16 (Sprites) — the objects that glide over the scroll, and the 8.8 motion that moves them.
- Chapter 18 (Advanced VDP) — mid-frame register tricks and split screens, §17.6 developed.
- Chapter 22 (Interrupts and Time) — the VDP interrupt installed for real, the profiling timer behind the frame-budget HUD, and the address-race critical sections this chapter warns of.

> **Pitfalls — the VDP address-register race (the classic crash).** The VDP address counter is a single piece of state shared by everyone who talks to the chip, and setting it takes *two* writes to the port (Ch. 12) — low byte, then high byte. Now suppose your main loop is halfway through that two-byte setup, aiming the counter at some VRAM address, when a VDP interrupt fires. The interrupt handler talks to the chip too — the console's own ISR reads the status register and may touch VRAM — and in doing so it moves the counter and resets the first-byte/second-byte flip-flop. When the interrupt returns and your main loop writes its second byte, the counter is now aimed somewhere else entirely, and your data lands in the wrong place: corrupted graphics, a crash, a bug that appears once a minute at random and cannot be reproduced under a debugger because single-stepping never lets the interrupt land in that exact two-instruction window. This is the most notorious bug in TI graphics programming, and its cause is a *critical section*: the two-byte address setup must not be interrupted. The fixes are the classic ones — mask interrupts around the VDP access (`LIMI 0` before, `LIMI 2` after, Ch. 22), or forbid the interrupt handler from touching the VDP at all, or route all VDP access through code that runs only in the safe window with interrupts known-off. Every `lib99` routine that aims the counter is a critical section in waiting; a program that mixes interrupts with VDP access and does not protect them will, eventually and unreproducibly, crash. Chapter 22 states the masking discipline formally and applies it back onto `vdplib`.

## Summary

The frame is the TI's unit of time and its budget both: the VDP raises the **F** flag at the end of each frame's active display (bit 7 of status, cleared by reading status — the once-per-frame acknowledgement), and the interval between beats is a measured **~50,000 cycles** at 3 MHz, roughly 1,200 VDP writes, for *everything* a frame does. Games synchronize to it two ways: a **poll-synchronized** loop that does its work then spins on F (full control, no safety net), or an **interrupt-driven** loop that lets the beat call a handler (background work for free, but interrupts land mid-code — the address race below). The structural key is to separate **update** (pure computation in CPU RAM, safe anytime) from **render** (VRAM writes, batched into the vertical-blank safe window so the beam never shows half-drawn work), and to render only what changed (dirty rectangles). Scrolling exposes the 9918A's lack of a hardware scroll register: rewriting the whole 768-cell name table to scroll coarsely costs a measured **~163,000 cycles — 3.3 frames — so full-screen coarse scrolling does not fit**. The professional answer is **pattern-shift smooth scrolling** (the Parsec method): leave the name table alone and redraw the band's *characters* shifted one pixel each frame, a measured **~1,044 cycles**, 150× cheaper and inside the beat, verified by the pattern byte walking `>80→>40→>20`. Vertical, split, and parallax scrolling are the same idea rotated, staggered, or faked with colour cycling; page flipping (Graphics I's two-name-table register flip) gives tear-free motion where VRAM allows; and a **time system** of a frame counter and countdown timers turns the beat into game time. The chapter's standing hazard is the **VDP address-register race** — an interrupt landing inside the two-byte counter setup, sending the main loop's write to the wrong address — the classic unreproducible crash, cured by masking interrupts around every VDP access (Ch. 22).
