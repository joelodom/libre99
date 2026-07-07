# Chapter 21 — Keyboard, Joysticks, and the TMS9901

*The machine has no keyboard controller and no gamepad chip — just a grid of switches and a clever way to read it, one column at a time, through the CRU.*

<!-- Part V — Input, Interrupts, and Console Services · target ≈20 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The keyboard matrix scan (ch10, verified) and the joystick read (inplib IREAD reads real presses: J1U->>0010, J1U+J1F->>0011, idle->>0000) and edge detection (IEDGE pressed/released) machine-verified on BENCH99 at commit bb565e4, which added `press`/`rel` (set a switch on the bare bench). Code in code/ch21/ (inplib). KSCAN is the firmware routine (described). The Alpha Lock trap is NOT modeled by the project keyboard (R-12) — described from the hardware record. -->

## No Controller At All

Open a modern game controller and you find a small computer: a microcontroller that scans the buttons, debounces them, tracks the analog sticks, and reports it all over USB or Bluetooth as clean, timestamped events. The keyboard on your desk is the same — a dedicated controller inside it does the scanning and hands your PC a tidy stream of key-up and key-down messages. Input, to a modern programmer, arrives pre-digested.

The TI-99/4A has none of that. There is no keyboard controller and no joystick chip. There is a *grid of switches* — the keys and the joystick directions, wired as an 8×8 matrix of simple on/off contacts — and there is the **TMS9901**, a general-purpose interface chip that can drive a few output pins and read a few input pins through the CRU (Chapter 10). That is the entire input hardware. To know whether a key is down, *your software* must drive the right pattern onto the 9901's output pins to select a column of the matrix, read the eight rows of that column back through the CRU, and decode which switch, if any, is closed. There is no event stream; there is a grid you interrogate. Every keypress this machine ever registered, every joystick waggle in every game, was software walking that matrix and reading switches.

This is more work than calling a TTS-style input API, and it is a better education, because it shows you what input *is* underneath the abstraction: switches and scanning. This chapter builds the scanning into `inplib` — the input layer every game in this book will use — and confronts the machine's two famous input gotchas: the debouncing and ghosting that raw switch grids suffer, and the Alpha Lock trap that has broken joystick-up in TI games for forty years.

---

## What You Will Learn

- The **TMS9901**: one chip doing three jobs — interrupt controller, I/O ports, and interval timer — and how the console wires it.
- The **keyboard matrix**: selecting a column through the CRU, reading its rows, and decoding a keypress (building on Chapter 10).
- The console's **KSCAN** routine: what the firmware's scanner gives you and what it costs.
- **Joysticks**: reading both sticks and their fire buttons through the same matrix, diagonals, and the notorious **Alpha Lock trap**.
- Rolling your own input layer — `inplib`: a per-frame snapshot, **edge detection** (pressed / held / released), and redefinable keys.
- The input conventions players expected: QUIT etiquette, pause, and two-player patterns.

## The Bridge: From Event Streams Back to Scanning

A modern program receives input as *events*: `keydown`, `keyup`, `gamepadbuttondown`, each delivered once, when it happens, with the key or button identified and the moment stamped. The program reacts to events or, for games, polls a pre-built snapshot of "what is currently held" that the runtime maintains for it. Either way, the messy physical reality — contacts bouncing, keys held across many frames, several keys down at once — has been cleaned up by layers below.

On the TI you are those layers. There are no events; there is the *current physical state* of a grid of switches, and you read it whenever you choose to look — typically once per frame (Chapter 17). From that raw snapshot you must build everything the modern runtime hands you for free: the *edges* (this key was up last frame and is down now — a press), the *held* state, the debouncing that keeps a single press from registering as several, and the mapping from physical switches to game actions. It sounds like a burden, and it is a little more code, but it is also complete control and complete comprehension: you know exactly when input is sampled (you sampled it), exactly what "pressed" means (you defined the edge), and exactly how a held key behaves (you decided). The chapter's `inplib` is that layer, built once and reused, and building it is understanding what an input system actually does.

## 21.1 The TMS9901: One Chip, Three Jobs

The **TMS9901 Programmable Systems Interface** sits at CRU base 0 and does three unrelated jobs, which is why it recurs throughout Part V. It is an **interrupt controller** — it collects interrupt requests (the VDP's vertical-blank line chief among them) and presents them to the CPU, with a per-source enable mask (Chapter 22 lives here). It is a set of **I/O pins** — a handful of lines the CPU can drive as outputs or sample as inputs through the CRU, and it is these pins that select the keyboard column and read the joystick fire buttons. And it is an **interval timer** — a counter that can tick independently of everything else (Chapter 22 again). One chip, three hats; this chapter wears the I/O hat.

The console wires the 9901's pins to the input hardware in a specific way. Three output pins — the CRU bits the console calls the column-select lines — carry a 3-bit column number to the keyboard matrix; the matrix's eight row lines come back on eight CRU input bits. So reading the keyboard is: write a 3-bit column number to the select pins, read eight row bits back. Everything in this chapter is that pattern, driven through the `LDCR` and `STCR` instructions of Chapter 10.

## 21.2 The Keyboard Matrix

The keyboard is an **8×8 matrix**: eight columns crossed with eight rows, a switch at each crossing, wired so that pressing a key connects its column to its row. To find out what is pressed, you *scan*: for each column in turn, drive that column's select lines and read which of the eight rows are pulled active. A key is **active low** — pressing it pulls its row to 0 — so an idle row reads 1 and a pressed key reads 0. Chapter 10 built and verified exactly this scan; here is its core, the `READROW` primitive:

```asm
READROW MOV  R1,R6            the column number ...
       SWPB R6                ... into the HIGH half (LDCR byte cargo)
       LI   R12,COLSEL        R12 = >0024 -> the column-select CRU base
       LDCR R6,3              drive the 3 column-select pins, LSB first
       LI   R12,ROWRD         R12 = >0006 -> the row-read CRU base
       CLR  R2
       STCR R2,8              rows 0..7 -> a byte (active low)
```

Two hardware realities complicate raw matrix scanning, and a real input layer handles both. The first is **debouncing**. A mechanical switch does not close cleanly — its contacts *bounce* for a few milliseconds, opening and closing several times before settling — so a single physical press, sampled fast enough, can read as several presses. The cure is time: sample the keyboard once per frame (at 60 Hz, frames are ~16 ms apart, longer than the bounce), and treat a key as newly pressed only on the *edge* where it goes from up to down, ignoring the noise within a frame. Per-frame edge detection (§21.5) is debouncing almost for free. The second is **ghosting**: in a passive switch matrix, pressing three keys that form three corners of a rectangle can make the fourth corner read as pressed too, because current sneaks around through the closed switches — a phantom key. There is no software cure (it is physics), only mitigation: games avoid needing three-plus simultaneous keys that ghost, and the matrix layout is chosen so common combinations do not. For most games — a few directions and a fire button — ghosting never bites, but it is why you cannot rely on reading arbitrary many-key chords on this hardware.

## 21.3 KSCAN: The Firmware's Scanner

You do not *have* to scan the keyboard yourself — the console firmware includes **KSCAN**, a routine that scans the whole matrix, handles the shift and function modifiers, debounces, tracks auto-repeat, and returns a decoded key code plus status, leaving its results in known scratchpad locations (a key code and a status byte the GPL side also reads). You invoke it (through the console-link mechanisms of Chapter 23), select a *mode* by a scratchpad byte — the whole keyboard, or specific split configurations for two-player or calculator-style layouts — and read back the key it found.

KSCAN is convenient and it is what TI BASIC and most cartridges used, because it gives you shift-decoding and repeat for free. Its costs are that it scans the *entire* matrix every call (all columns, whether you care about them or not), it imposes the firmware's idea of debouncing and repeat timing (which a fast-action game may not want), and it returns *one* key, not the full set of what is held — awkward for a game where the player presses fire *and* a direction at once. So the rule of thumb splits by genre: a program that reads text or drives a menu uses KSCAN and enjoys its shift-and-repeat handling; a real-time game that needs simultaneous inputs and its own timing rolls its own scan of just the columns it cares about, which is `inplib`. Knowing KSCAN exists — and its scratchpad interface, which Chapter 24 maps — is knowing when *not* to reinvent it.

## 21.4 Joysticks, and the Alpha Lock Trap

The two joysticks hang off the *same* matrix — columns 6 and 7 — read exactly like keyboard columns. Select column 6 and the eight rows report joystick 1's switches: row 0 is fire, rows 1–4 are left, right, down, and up, each active low. Column 7 is joystick 2. So `inplib`'s `IREAD` is `READROW` on column 6 or 7, with the direction bits inverted to a clean active-high mask:

```asm
IREAD  MOV  R0,R6
       AI   R6,6             column = 6 + joystick number
       ...  (drive select, STCR the rows) ...
       INV  R1              active HIGH (1 = pressed)
       ANDI R1,>001F         keep the five direction/fire bits
```

We can now *verify* this against real presses, because BENCH99 gained a `press` command this chapter — it sets a switch on the bare bench so our own code reads it. Pressing joystick 1's up direction and running `IREAD` returns `>0010` (bit 4, up); pressing up *and* fire returns `>0011` (up + fire); nothing pressed returns `>0000`. The read decodes exactly. **Diagonals** fall out of this for free: up-left is simply bits for up *and* left both set, because they are independent switches, so a game reads all eight compass directions with no extra work — the mask just has one or two bits.

And then there is the trap. The **Alpha Lock** key — the caps-lock latch at the bottom-left of the keyboard — is wired into the same matrix as the joysticks, on a line it *shares* with joystick 1's up direction. When the Alpha Lock is latched *down* (as many users left it), it holds that shared line, and joystick-up reads as pressed — or fails to read — regardless of the stick. The result is a bug that has haunted TI games since 1981: with Alpha Lock engaged, your ship drifts upward on its own, or refuses to climb, and the player has no idea why. Every serious TI game had to cope: the professional fix is to read the Alpha Lock's own state and *subtract* its influence from the joystick-up reading, or, more bluntly, to display "RELEASE ALPHA LOCK" at startup and hope the player obeys. It is the single most infamous input gotcha on the platform, and it is pure hardware: the wiring shares a line, and software can only compensate.

> **Verification note (R-12).** The project's keyboard model treats every switch independently and does *not* reproduce the Alpha-Lock/joystick-up line sharing, so the trap does not bite on the bench — which is why this section describes it from the hardware record rather than demonstrating it. A program that must be robust on real hardware still needs the compensation; a program tested only against the project emulator would not discover the need. This is exactly the kind of gap where the emulator is kinder than the metal, and the honest move is to say so and code for the metal. (Classic99 and MAME model the Alpha Lock behavior for those who need to reproduce it.)

## 21.5 Rolling Your Own: `inplib`

A game does not want the raw switch grid; it wants to know *what just happened* — this direction was pressed this frame, that button is held, fire was released. The transformation from raw state to events is **edge detection**, and it is the heart of `inplib`. Each frame you sample the inputs into a current mask, and you compare that mask to last frame's: a bit that is set now and was clear then is a **press**; set now and set then is **held**; clear now and set then is a **release**. Three bitwise operations, and the 9900's `SZC` (set zeros corresponding — "AND with the complement") does two of them directly:

```asm
IEDGE  MOV  R1,R2
       SZC  R0,R2           R2 = cur AND NOT prev  = newly PRESSED
       MOV  R0,R3
       SZC  R1,R3           R3 = prev AND NOT cur  = newly RELEASED
```

Verified on the bench: with last frame holding *left* and this frame holding *left and right*, `IEDGE` reports *right* pressed and nothing released; the next frame, holding only *right*, it reports nothing pressed and *left* released. That is a complete event model — press, hold, release, per input, per frame — from two instructions and a remembered mask. Debouncing is inside it for free: a mechanical bounce within a single frame is invisible, because you only compare frame-to-frame, so a key registers exactly one press on the edge no matter how its contacts chattered.

The last piece is **redefinable keys**. Players expect to remap controls, and a game that hard-codes "the joystick, or the arrow-diamond keys" cannot honor that. The fix is a layer of indirection: define the game's *actions* (thrust, fire, left, right) and keep a **mapping table** from each action to the physical switch (a matrix cell, or a joystick bit) that triggers it. `inplib`'s sampler reads through the table, so the *action* mask it produces is the same whether the player is using the joystick or remapped keys, and remapping is editing the table, not the game. This indirection — actions, not switches, as the game's vocabulary — is the same lesson as `equates.inc` (Chapter 11): name the thing you mean, map the name to the hardware in one place, and the rest of the program speaks in meanings.

## 21.6 What Players Expected

Input is also *convention*, and TI players had expectations a well-behaved program honored. The most important is the **QUIT** discipline. The console wires `FCTN`-`=` (QUIT) to a hard reset — press it and the machine reboots to the title screen, losing everything, no confirmation. This is convenient for a hung program and catastrophic mid-game, so serious cartridges *disabled* it (there is a scratchpad flag the firmware checks, which a program clears to suppress the reset) and provided their own, safer quit — usually `FCTN`-`=` intercepted, or a menu option, that asks before throwing away the game. Disabling QUIT is a courtesy and a safety feature; a game that leaves it live loses a player's high score to a fat-fingered keypress. Beyond QUIT, players expected a **pause** (often `FCTN`-`4` or a dedicated key), sensible **two-player** conventions (joystick 1 and the left keyboard half for player one, joystick 2 and the right half for player two — the split KSCAN modes exist for exactly this), and controls that could be driven from *either* joystick *or* keyboard, because not everyone owned the joysticks. These are not hardware facts but cultural ones, learned by playing the era's games, and honoring them is the difference between a program that feels native to the machine and one that merely runs on it.

## Lab 21 — `inplib` and Arming DODGE

The lab is the input layer and its first use, in `code/ch21/`.

**`inplib` (`inplib.inc` + `inplib.a99`)** — the input layer for `lib99`: `IREAD` (read a joystick to an active-high mask) and `IEDGE` (edge-detect this frame against last). Build and prove it:

```sh
libre99asm code/ch21/inplib.a99 --format bin -o build/INPC.bin --symbols build/inp.map.json
```

On the bench, the self-test checks the edge cases and the idle read, painting green (`R7=>02`); then, using the new `press` command, you can prove the live read against real switches — `press J1U`, run a read, and watch `IREAD` return `>0010`; `press J1F` too and it becomes `>0011`. This is where you confirm, switch by switch, that your input decodes correctly — the input equivalent of the graphics chapters' `pixels`.

**Arming DODGE** is the lab's aim: retrofit Chapter 16's game with real controls — steer the player with the joystick *and* a redefinable keyboard fallback, fire mapped through the action table, QUIT disabled with a safe pause in its place. DODGE's meteors already fall and collide; now the player *moves*, which is the moment it becomes a game you can lose on purpose. The full playable DODGE comes together with Chapter 17's loop driving `inplib` each frame; here you build the input layer it steers by and verify it reads true. The exercises build the action-mapping table and the input visualizer.

> **Field Notes — How Munch Man reads the sticks.** The good TI action games *feel* tight — the player moves the instant you push the stick, with no lag and no drift — and that feel is an input-layer achievement. Munch Man, the TI's Pac-Man, is remembered for responsive control, and the recipe is the one this chapter teaches: sample the joystick once per frame, act on the *held* direction immediately (not on an edge — movement should continue while held), and keep the read cheap enough that it never delays the frame. A game that scanned sloppily, or leaned on KSCAN's repeat timing for movement, felt mushy; one that read its two joystick columns directly each frame and moved on the current state felt tight. The difference between tight and mushy controls is almost never the hardware — it is whether the input is sampled every frame and acted on immediately, which is precisely what `inplib` in Chapter 17's loop does.

> **Sidebar — The membrane wars.** The original TI-99/4 (1979) shipped with a *calculator-style* keyboard — small, hard, rubbery keys — that reviewers savaged, and the 1981 TI-99/4A's headline improvement was a proper **full-travel** keyboard, a real typewriter-style set of keys. It seems a small thing, but the keyboard was a genuine battleground of the early-1980s home-computer wars: the Sinclair ZX81's flat membrane, the Atari 400's flat panel, the flat "chiclet" keys of the PCjr — each was cost-cutting that users hated, and each cost sales to machines with real keys. The TI-99/4A's good keyboard was a competitive weapon, and the matrix behind it (the same 8×8 grid this chapter scans) was the same in cheap and good keyboards alike — only the key mechanism above each switch differed. The switches were always a grid; what the wars were about was what your fingers touched on top of them.

## Exercises

**21.1** ✦ Reading joystick 1 returns the mask `>0006`. Which directions/buttons are pressed? (Bits: 0 fire, 1 left, 2 right, 3 down, 4 up.)

**21.2** ✦ Explain, in terms of `SZC`, why `cur AND NOT prev` is exactly the set of inputs *newly pressed* this frame. What does `prev AND NOT cur` give?

**21.3** ✦✦ Write `IHELD` (inputs held both frames = `cur AND prev`) and use `IREAD`/`IEDGE`/`IHELD` on the bench with `press`/`rel` to produce a press, a hold across two reads, and a release, confirming each with `m`.

**21.4** ✦✦ Build the redefinable-key layer: an action table mapping four actions (up/down/left/right/fire) to switches, and a sampler that produces an action mask by reading through it. Show that swapping the joystick entries for keyboard cells changes the controls without touching the game logic.

**21.5** ✦✦ The Alpha Lock trap: describe precisely why joystick-up misreads when Alpha Lock is latched, and write the compensation (read the Alpha Lock state and correct the up bit). Note why the project emulator will not exercise it and where you would test it.

**21.6** ✦✦✦ Arm DODGE: drive Chapter 16's player from `inplib` (joystick + keyboard fallback), disable QUIT and add a pause, and confirm with `press` that the player moves left, right, and fires. (The full frame-timed game is Chapter 17's; here, step it and check the player position responds to injected presses.)

**21.7** ✦✦✦ Write an input visualizer: a `textlib` (Ch. 13) screen that shows the live state of both joysticks and a set of keys as a grid of on/off indicators, updated each frame. Drive it with `press`/`rel` and watch the indicators track — a debugging tool you will reuse whenever input misbehaves.

## Further Reading

- TMS9901 datasheet — the interface chip's I/O pins, interrupt mask, and timer (the interrupt and timer roles return in Chapter 22).
- *Editor/Assembler Manual*, Texas Instruments — the KSCAN routine, its modes, and its scratchpad interface.
- Chapter 10 (The CRU) — the `LDCR`/`STCR` column-select and row-read this chapter's scan is built on, verified there.
- Chapter 17 (Motion) — the frame loop that samples `inplib` once per frame, where edge detection and debouncing live.
- Chapter 23 (Console ROM Services) — how a program invokes KSCAN and other firmware routines.
- Chapter 24 (The Scratchpad Atlas) — KSCAN's scratchpad state and the input-related pad bytes.

## Summary

The TI-99/4A has no keyboard controller or joystick chip — only an **8×8 matrix of switches** read through the **TMS9901**, a one-chip interface serving three roles (interrupt controller, I/O pins, interval timer; this chapter uses the I/O). Reading input is *scanning*: drive a 3-bit column number onto the 9901's select pins, read the eight rows back through the CRU (`LDCR`/`STCR`, Chapter 10), and decode — keys and joystick directions are **active low** (0 = pressed). The console's **KSCAN** firmware routine scans the whole matrix with shift-decoding, debouncing, and repeat, ideal for text and menus but returning one key and its own timing — so real-time games roll their own scan of just the columns they need. The two **joysticks** are columns 6 and 7 (row 0 fire, rows 1–4 left/right/down/up); `inplib`'s `IREAD` reads them to a clean active-high mask, verified against real presses via the new `press` command (up → `>0010`, up+fire → `>0011`), with diagonals free as independent bits. The infamous **Alpha Lock trap** — the caps-lock latch sharing a matrix line with joystick-up, so an engaged lock breaks upward movement — is pure hardware wiring the project does not model (R-12); real code compensates. `inplib` turns raw state into events by per-frame **edge detection** — `cur AND NOT prev` is pressed, `prev AND NOT cur` is released (the `SZC` instruction), verified on the bench — which also debounces for free, and a **redefinable-key** action table names controls as *actions* mapped to switches in one place. Players expected conventions too: disabling the destructive `FCTN`-`=` QUIT, a pause, and joystick-or-keyboard control. `inplib` (`IREAD`, `IEDGE`) is the input layer every later game uses, and it is what arms DODGE (with Chapter 17's loop) into a game you can play.
