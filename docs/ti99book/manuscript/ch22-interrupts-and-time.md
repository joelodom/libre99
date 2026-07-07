# Chapter 22 — Interrupts and Time

*Sixty times a second the beam knocks, and the machine answers — the one interrupt that runs the console, the hook where your code joins it, and the clock you build from the beat.*

<!-- Part V — Input, Interrupts, and Console Services · target ≈24 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The 60 Hz VDP interrupt (the firmware frame counter at >8379 reaches exactly 60 after `f 60`), the console ISR running each frame, and the USER ISR HOOK at >83C4 (a poked INC/RT routine fires per frame: 59 increments over 60 frames) machine-verified on BENCH99 (boot mode) at commit bb565e4. PROFILE99 measures lib99 costs via bench `cycles` (STONE ~560, SVOL ~256, SSILNC ~260, IREAD ~294, IEDGE ~156). Code in code/ch22/ (profile99). The 9901 interval timer is NOT emulated (R-12, ROADMAP §6) — described; on-machine profiling awaits it. -->

## The Beam Knocks

A program can find things out two ways. It can *ask* — poll, check, look again and again, spending cycles on the question even when the answer has not changed. Or it can be *told* — arrange to be interrupted the moment something happens, and otherwise get on with its work. Interrupts are the second way, and on the TI-99/4A there is essentially *one* of them that matters, and it knocks with metronomic regularity: sixty times a second, when the video beam finishes a frame and the VDP raises its interrupt line, the machine is interrupted. That knock is the heartbeat of Chapter 17 seen from the other side — not a flag you poll, but a summons you answer.

And the console *does* answer it, every frame, whether your program asked or not. The firmware installs an interrupt handler at power-on, and sixty times a second that handler runs: it scans the keyboard, advances the automatic sprite motion, services the sound list, checks for the QUIT key, counts down the screen-blank timeout, and ticks a clock. Your game, running its own loop, is quietly interrupted sixty times a second by this resident housekeeper doing the console's chores — which is convenient (you get keyboard scanning and sound playback for free) and treacherous (it runs behind your back, touches the VDP, and can land in the middle of anything you are doing).

This chapter is that one interrupt and everything around it: how the 9900's interrupt machinery works, what the console's handler does each frame, how you *join* it through a documented hook (verified this chapter — a routine of ours, called sixty times a second by the firmware), how you *avoid* it when you want the bare machine, and how you build a sense of *time* from the sixty-per-second beat. And it is where the notorious VDP address race of Chapter 17 finally gets its formal cure.

---

## What You Will Learn

- The 9900's 16-level interrupt model, why the console wires effectively **one** level, and `LIMI` as the everyday on/off switch.
- The sources that reach that level — the VDP vertical interrupt, the 9901 timer — and how they are identified.
- The **console ISR** walked behaviorally: what it does every frame, its scratchpad footprint, and its cost in your budget.
- Taming it: the control flags, and the **user ISR hook** at `>83C4` — its contract, and the verified fact that the firmware calls it every frame.
- Going bare: pure-poll architectures with interrupts masked off.
- The 9901 **interval timer** — and the honest note that the project does not yet emulate it.
- **Profiling**: measuring routines in cycles, and a table of `lib99` call costs.
- **Critical sections**: the VDP address race, formally, and the masking discipline that cures it.

## The Bridge: One Interrupt, Owned by the OS

A modern system has dozens of interrupt sources — timers, network cards, disk controllers, the GPU, every USB device — arbitrated by an interrupt controller and dispatched by the operating system to drivers, all beneath layers you never see. Interrupts are the OS's business; an application receives their consequences as events and callbacks, never the raw signal.

The TI has one interrupt that matters and no operating system in the modern sense — but it *does* have a resident firmware handler that owns that interrupt, which makes the situation more like a modern one than it first appears. The 60 Hz VDP interrupt is the console's single timer-tick, and the firmware's handler is a tiny always-running "OS" that does the periodic chores. The difference is that on the TI you can *see* all of it, *replace* parts of it, and *join* it: the handler, the hook, the scratchpad it uses are all right there, documented and pokeable, not hidden behind a driver model. Learning interrupts here is learning what an interrupt *is* — a forced subroutine call triggered by hardware — and what a periodic-timer OS service *is*, with nothing between you and the mechanism. It is the same concept a modern kernel scales to hundreds of sources, shown once, whole, at the scale of one.

## 22.1 The 9900's Interrupts and `LIMI`

The TMS9900 has a proper interrupt architecture: **sixteen priority levels**, 0 (highest) through 15, each with a vector — a pair of words in low memory (a workspace pointer and a program counter) that a level-*n* interrupt loads to switch context, exactly like the `BLWP` of Chapter 9 but triggered by hardware instead of an instruction. When an interrupt is taken, the CPU does a context switch to its vector, sets the interrupt mask to *below* that level (so only higher-priority interrupts can preempt it), and runs the handler until it returns with `RTWP`. It is a clean, general design meant for a minicomputer with many devices.

The console uses almost none of it. The TI-99/4A wires its interrupt sources so that everything enabled arrives as **level 1** — the console does not use the priority encoder, so there is one effective maskable interrupt, not sixteen — plus the special **RESET** (level 0, the power-on and QUIT reset) and **LOAD** (a non-maskable input, the sidebar's back door). So in daily practice there is one interrupt to think about, its vector at the level-1 slot, and one control that turns it on and off: **`LIMI`** (Load Interrupt Mask Immediate), which sets the CPU's interrupt mask. `LIMI 2` (or any value ≥ 1) enables level-1 interrupts; `LIMI 0` masks them off. That single instruction — enable interrupts, do something delicate, disable them — is the everyday tool of interrupt programming here, and §22.8's critical sections are built on it.

## 22.2 The Sources, and Identifying Them

Two things can raise the console's level-1 interrupt, both through the 9901 (Chapter 21's chip in its interrupt-controller role). The first and constant one is the **VDP vertical interrupt**: at the end of each frame's active display the VDP asserts its interrupt line, which the 9901 presents to the CPU as level 1, provided the 9901's mask enables it (a CRU bit) *and* the CPU's mask enables it (`LIMI`). This is the 60 Hz metronome — the same F flag of Chapter 17, now delivering itself rather than waiting to be polled. The second is the **9901 interval timer**, which can be programmed to raise the same level-1 interrupt at a rate you choose (§22.6).

Because both arrive as level 1, a handler that enables both must *identify* which fired — by reading the 9901's interrupt-status bits and the VDP status — and dispatch accordingly. In practice most programs enable only the VDP interrupt, so the handler knows its cause without asking: the beam finished a frame. The acknowledgement matters and is Chapter 12's rule again — the handler must read the VDP status register to clear the F flag, or the interrupt re-asserts immediately and the machine drowns in a storm of its own interrupts. Reading status *is* the "I have handled this frame" signal.

## 22.3 The Console ISR, Walked

Boot the console and, sixty times a second, the firmware's handler runs. We can watch it work: after booting and running sixty frames, the console's frame counter in scratchpad — a byte at `>8379` — reads exactly `>3C`, sixty, one tick per frame. The interrupt is real, regular, and the firmware is counting it.

What the handler *does* each frame is the console's housekeeping, and knowing it is knowing what runs behind your program's back:

- **Scans the keyboard** (KSCAN, Chapter 21), updating the key-code and status bytes so a polling program finds fresh input.
- **Advances automatic sprite motion** (Chapter 16), adding each sprite's velocity from the motion table.
- **Services the sound list** (Chapter 19), advancing the auto-player and writing the next block of sound when a duration expires.
- **Checks the QUIT key** and honors or ignores it per the disable flag (Chapter 21).
- **Counts the screen-blank timeout** and other periodic bookkeeping, and **ticks the frame counter** we just watched.

All of this costs cycles — a slice of every frame's ~50,000-cycle budget spent before your code gets the frame — and it touches scratchpad (the counters, the KSCAN state, the sound pointers) and the VDP (reading status, moving sprites, writing sound). That last point is the danger of §22.8: the resident handler talks to the VDP every frame, on a schedule you do not control, which is precisely why an unprotected VDP access in your main code can be corrupted by it.

## 22.4 Taming It: Control Flags and the User Hook

You are not stuck with the firmware handler's full behavior. It reads **control flags** in scratchpad that switch its features off individually — you can tell it to stop scanning the keyboard, stop moving sprites, stop servicing sound — or you can mask the interrupt entirely (`LIMI 0`, §22.5) and lose all of it. Between "all of it" and "none of it" is the most useful option: **join** the handler with your own code, through a documented hook.

The console firmware, each frame, checks a scratchpad location — **`>83C4`** in the E/A convention — for a pointer to a **user interrupt routine**, and if it finds one, calls it as part of the frame's interrupt work. This is how your code runs sixty times a second without owning the interrupt: you write a routine, put its address in `>83C4`, and the firmware calls it every frame, after (or amid) its own chores. We verified this directly. Poking a tiny routine into RAM — increment a counter, return — and its address into `>83C4`, then running sixty frames, the counter reaches fifty-nine: our routine ran essentially every frame, called by the firmware's handler, exactly as the contract promises.

```text
bench: poke a routine [INC @counter; RT] at >8340, set >83C4 = >8340, run 60 frames
       -> counter = 59      (our code, called ~once per frame by the console ISR)
```

The hook is powerful and it has a **contract**. Your routine runs inside the interrupt, so it must be fast (it is spending the frame's budget), it must preserve any register and scratchpad state the firmware and the interrupted code rely on (the handler has its own workspace at `>83C0`, but your routine must not trample GPL's state or the interrupted program's), and it must return cleanly so the handler finishes its frame. Done right, the hook is the ideal place for periodic work that must happen on time regardless of what the main loop is doing — a music driver (the lab), a timer system, a background animation — because the beat calls it, not your loop. If you need several such tasks, you **chain**: your routine does its work and then calls whatever was in the hook before you, so multiple periodic jobs share the one interrupt.

## 22.5 Going Bare: The Poll-Only Machine

The opposite of joining the interrupt is refusing it. A program can `LIMI 0` at startup and never enable interrupts, and then *nothing* runs behind its back — no keyboard scan, no sprite motion, no sound service, no QUIT check, no frame counter. The machine is entirely the program's, every cycle of every frame, with no resident handler stealing a slice. Many action cartridges did exactly this: they wanted deterministic control and the whole budget, so they masked the interrupt, polled the VDP F flag themselves for frame timing (Chapter 17's poll loop), scanned only the input columns they cared about (Chapter 21's own scan), and drove their own sound — reimplementing the handful of ISR services they actually needed and skipping the rest.

The trade is total control for total responsibility. Poll-only means *you* scan the keyboard (or lose input), *you* time the frame, *you* move sprites and service sound, and *you* had better not want the firmware's conveniences, because they are off. It is the right choice when the frame is tight and the firmware's chores are overhead you cannot afford, and the wrong choice when you would only reimplement what the ISR already does well. The decision — join the ISR or go bare — is one of the first architectural choices a TI program makes, and Chapters 39–43's case studies make it explicitly, each for its own reasons.

## 22.6 The 9901 Interval Timer

The 9901 contains an **interval timer**: a 14-bit counter, clocked from the system clock through a fixed divider, that counts down and raises the level-1 interrupt (or sets a flag you can poll) when it reaches zero, then reloads. You program it by writing the 9901 into timer mode (a CRU bit) and loading the count; you read the current value back the same way. Its uses are the ones the 60 Hz frame interrupt cannot serve: a **music tempo** independent of the frame rate (a drum machine that does not care about 60 Hz), **mid-frame timing** for the split-screen tricks of Chapter 18 (fire an interrupt partway down the frame), and **input sampling** or any periodic task at a rate other than sixty per second.

Here honesty is required. **The project emulator does not yet emulate the 9901 interval timer** — it is a documented gap (the ROADMAP notes it alongside the cassette transport that depends on it), so a program that programs the timer will find it inert on the project, and none of this section's behavior can be exercised on the bench. This is the same posture as the speech synthesizer (Chapter 20): the hardware and its use are described from the datasheet and record, the gap is stated plainly, and the shelf tools (Classic99, MAME) that model the timer are named for those who need it. It is also, squarely, a **project roadmap item** — emulating the 9901 timer would close this gap and enable both timer-driven code and the cassette work that waits on it — and exactly the kind of book-drives-the-emulator feedback this project is built to produce.

## 22.7 Profiling: Measuring in Cycles

You cannot optimize what you have not measured (Chapter 37's creed), and measurement means *timing*. The natural on-machine profiler uses the 9901 timer — start it before a routine, read it after, and the difference is the routine's duration — but with the timer unemulated (§22.6), on-machine profiling awaits that roadmap work. So this book profiles the way it has all along: **BENCH99's `cycles`**, bracketing a routine between two checkpoints and reading the exact cycle count the emulator charged. `PROFILE99` is that harness — a program with labeled checkpoints after each `lib99` call — and the bench reads off the costs:

| `lib99` call | Cost (cycles) | What it does |
|---|---|---|
| `IEDGE` | ~156 | edge-detect one frame's input |
| `VWTR` | ~190 | write one VDP register |
| `SVOL` | ~256 | set one sound channel's attenuation |
| `SSILNC` | ~260 | silence all four sound channels |
| `IREAD` | ~294 | read a joystick to a mask |
| `VSBW` | ~340 | write one VRAM byte (the address aim dominates) |
| `STONE` | ~560 | play a tone (latch period + data + attenuation) |
| `VMBW` | ~73 / byte | stream a block to VRAM |

(The graphics rows are the Chapter 18 cookbook's; the input and sound rows are `PROFILE99`'s, measured here.) The table is the point: with these numbers and the ~50,000-cycle frame, you can *budget* — a frame that moves thirty-two sprites (`SMOV` each), plays a tone, reads two joysticks, and edge-detects them spends a few thousand cycles, a small fraction of the frame, so it fits with room to spare; a frame that also rewrites the whole name table (§17.4's 163,000 cycles) does not. Profiling turns "will it fit?" from a worry into arithmetic, and publishing a library's per-call costs (as this table does) lets every program that uses it budget in advance.

## 22.8 Critical Sections: The Address Race, Cured

Chapter 17 warned of the VDP address-register race and promised the cure here. State it formally. A **critical section** is a sequence that must run without interruption because an interrupt landing inside it would corrupt shared state. On the TI the canonical one is the VDP address setup: aiming the VDP counter takes two writes to the port (Chapter 12), and the console's resident ISR (§22.3) touches the VDP every frame — reads status, moves sprites. If the 60 Hz interrupt fires *between* your two address bytes, the ISR runs, moves the VDP's address counter and resets its byte flip-flop, and returns; your second byte then lands against a counter aimed somewhere else, and your data corrupts wherever the counter now points. It is unreproducible under single-stepping (the interrupt never lands in that two-instruction window when you step by hand) and it strikes at speed, once in a while, at random — the worst kind of bug.

The cure is to make the critical section *actually* critical: mask the interrupt around it.

```asm
       LIMI 0              interrupts off — nothing may preempt the aim
       ...  aim the VDP counter (two port writes)  ...
       LIMI 2              interrupts on again
```

`LIMI 0` before the two-byte aim, `LIMI 2` after, and the ISR cannot land in the window. Every `lib99` routine that aims the counter (`VWA`, and everything built on it) is a critical section, and a program running with interrupts enabled must protect them — mask around each VDP access, or route all VDP traffic through code that runs only with interrupts known-off (the poll-only architecture of §22.5 sidesteps the race entirely, because there is no ISR to collide with). The discipline generalizes: any state shared between the main code and the ISR — a queue the ISR drains, a flag it reads, the VDP it touches — is a critical section, and the checklist for an ISR-safe library is to identify every such shared access and mask it. Applied back onto `vdplib`, this is the difference between a graphics library that works in a poll-only game and crashes in an interrupt-driven one, and one that works in both.

## Lab 22 — A Music Driver on the Beat, and PROFILE99

The lab is two artifacts, in `code/ch22/`.

**A user ISR that plays music.** Install a routine at `>83C4` that advances Chapter 19's music driver one step each frame, then start a tune and `HALT` the main code — and the music plays, rock-steady at 60 Hz, while the main program does *nothing*, because the beat, not the loop, drives the sound. This is the auto-player pattern the console uses for its own sound lists (Chapter 19), built with your own driver: the verified `>83C4` hook (a routine of ours called every frame) is exactly the mechanism, and here it carries a music step instead of a counter increment. On the desktop you hear steady music under an idle main loop; on the bench you verify the hook fires and the driver's state advances each frame.

**`PROFILE99`.** The profiling harness of §22.7 — labeled checkpoints around `lib99` calls, measured with `cycles`. Build it and read the costs:

```sh
libre99asm code/ch22/profile99.a99 --format bin -o build/PRF.bin --symbols build/prf.map.json
```

Bracket consecutive `PN` labels with `cycles` and the deltas are the per-call costs (this chapter's table). Extend it with your own routines — every `lib99` call you rely on in a tight frame deserves a measured number, and `PROFILE99` is where you get it.

> **Sidebar — LOAD, the nonmaskable back door.** Below the maskable level-1 interrupt sits **LOAD**, a non-maskable interrupt input the 9900 honors regardless of the interrupt mask — `LIMI 0` cannot stop it. On the stock console LOAD is not wired to anything a program encounters normally, but it is the hardware basis of single-step and debugging add-ons: pulse the LOAD line and the CPU vectors to the LOAD vector no matter what it was doing, even mid-critical-section, even with interrupts masked — a back door into a running, uncooperative machine. The Mini Memory and various hardware debuggers used it to break into a program that had otherwise seized the machine, and it is why "mask all interrupts" is not quite "nothing can interrupt me": LOAD always can. It is a small reminder that the maskable interrupt this chapter is mostly about is not the *only* way the hardware can seize the CPU — just the only one you meet in daily programming.

## Exercises

**22.1** ✦ What does `LIMI 0` do, and what does `LIMI 2` do? Why is the exact value above 0 usually irrelevant on this console?

**22.2** ✦ The firmware ISR reads the VDP status register every frame. What specific thing does that read accomplish (beyond looking), and what happens if a handler forgets it?

**22.3** ✦✦ Install a `>83C4` user hook that increments a scratchpad counter, and verify with the bench (`boot`, poke the routine and the hook, run frames) that it fires once per frame. Then make it *chain* — call the previous hook value after your work — and confirm two hooked routines both run.

**22.4** ✦✦ Using `PROFILE99`, measure the cost of moving a sprite (`SMOV`) and printing a character (`TXPUTC`). Add them to §22.7's table and compute how many of each fit in one frame.

**22.5** ✦✦ Write the VDP-address critical section with `LIMI` guards (§22.8), and explain precisely which two-instruction window the guard protects and why the bug it prevents cannot be reproduced by single-stepping.

**22.6** ✦✦ Contrast the join-the-ISR and poll-only architectures: list what each gives you and costs you, and name a game genre that would choose each. (Reference §22.4 and §22.5.)

**22.7** ✦✦✦ Build the music-on-the-beat lab: a `>83C4` hook that steps a simple two-note music driver each frame while the main code halts. Verify on the bench that the driver's frame counter and note pointer advance without the main loop running.

**22.8** ✦✦✦ The 9901 interval timer is unemulated (§22.6). Design (do not run) a timer-driven music tempo independent of the frame rate: how you would program the timer, hook its interrupt, and advance the music — and state exactly what you would test once the project emulates the timer.

## Further Reading

- TMS9900 data manual — the 16-level interrupt architecture, the context switch, and `LIMI`/`RTWP`.
- TMS9901 datasheet — the interrupt mask and the interval timer (§22.6), the input chip in its interrupt and timer roles.
- *Editor/Assembler Manual*, Texas Instruments — the console ISR's behavior, the control flags, and the `>83C4` user-hook convention.
- Chapter 12 (Inside the TMS9918A) — the VDP F flag and the status-read acknowledgement the ISR performs.
- Chapter 17 (Motion) — the frame as budget, the poll loop, and the address race this chapter cures.
- Chapter 37 (Optimization) — profiling as the foundation of optimization, extending §22.7.
- Chapter 24 (The Scratchpad Atlas) — the ISR's scratchpad footprint mapped byte by byte.

## Summary

The TI-99/4A has one interrupt that matters: the **VDP vertical interrupt**, level 1, sixty times a second — the 9900's sixteen-level architecture wired down to one maskable line (plus RESET and the non-maskable LOAD), controlled by **`LIMI`** (`LIMI 0` off, `LIMI 2` on). The console's firmware installs a resident handler that runs every frame — verified by the scratchpad frame counter reaching exactly 60 after sixty frames — doing the console's housekeeping: keyboard scan, automatic sprite motion, sound-list service, QUIT check, and the frame tick, all spending a slice of the ~50,000-cycle budget and touching the VDP behind your program's back. You can **tame** it through control flags, **avoid** it entirely (`LIMI 0`, the poll-only architecture many action games chose for total control), or **join** it through the documented **user hook at `>83C4`** — verified this chapter, a routine of ours called by the firmware essentially every frame (59 calls in 60 frames) — the ideal home for periodic work like a music driver, with a contract to be fast and preserve state, and a chaining pattern for several tasks. The **9901 interval timer** would give a clock independent of the frame rate, but the project does not yet emulate it (R-12, a roadmap item), so timer-driven code and on-machine profiling await that work; meanwhile profiling is done with BENCH99's `cycles`, and `PROFILE99` publishes a table of `lib99` call costs (`IEDGE` ~156, `VWTR` ~190, `IREAD` ~294, `STONE` ~560, `VMBW` ~73/byte, …) that turns "will it fit the frame?" into arithmetic. Finally, the VDP **address race** of Chapter 17 is cured formally: any state shared with the resident ISR — the VDP counter above all — is a **critical section**, protected by masking interrupts (`LIMI 0` … `LIMI 2`) around it, the discipline that makes a library safe under interrupts.
