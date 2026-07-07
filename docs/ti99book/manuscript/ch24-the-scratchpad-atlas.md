# Chapter 24 — The Scratchpad Atlas

*Two hundred and fifty-six bytes, the only fast RAM the CPU has, coveted by everyone — a map of who owns what, and what you may take.*

<!-- Part V — Input, Interrupts, and Console Services · target ≈16 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The pad landmarks used here are machine-verified across the book: the fast island (Ch. 5), the frame counter >8379 and user hook >83C4 (Ch. 22, verified in boot mode), FAC >834A / ARG >835C (Ch. 23), and lib99's own layout (Ch. 9-23). PADWATCH (snapshot + diff, workspace outside the pad) machine-verified on BENCH99 at commit bb565e4 — reports a target's exact footprint (a >8340 write -> NCHG=2, FIRST=8340). Code in code/ch24/ (padwatch). The ISR's per-frame footprint was probed in boot mode. -->

## The Most Fought-Over Bytes in the Machine

There are exactly 256 bytes of RAM in the TI-99/4A that the CPU can reach at full speed — the scratchpad, `>8300`–`>83FF`, the "fast island" of Chapter 5. Everywhere else the CPU touches, save the console ROM, is 8-bit expansion RAM behind the multiplexer, costing extra cycles on every access; the scratchpad alone is 16-bit, zero-wait, as fast as the chip can go. It is where your workspace lives, where your stack lives, where every hot variable in an inner loop wants to be, because a byte here is worth several anywhere else.

And that is the problem, because *everyone* knows it. The GPL interpreter keeps its registers and status here. The keyboard scanner leaves its results here. The interrupt handler counts frames and services sound here. The floating-point package puts its accumulators here. The Editor/Assembler utilities carve out their vectors here. Your program wants a workspace, a stack, and its own variables here. Two hundred and fifty-six bytes, and half a dozen tenants, all wanting the fastest memory in the machine — so the scratchpad is not just the fastest RAM but the *most political*, a small territory where your code and the console's must coexist without treading on each other. A byte you assume is free might be the frame counter; a workspace you place carelessly might overlap KSCAN's state; and the bug that follows — the console's housekeeping corrupting your variable, or yours corrupting its — is among the nastiest on the platform, because the collision is invisible in the source and intermittent in the running.

This chapter is the map that prevents those collisions: an atlas of the 256 bytes — who owns what, in which environment, and what you may safely take. It is a reference chapter, and it earns its place because on this machine, more than most, *knowing where you may put a byte* is a load-bearing skill.

---

## What You Will Learn

- Why 256 bytes deserve a chapter: the fastest and most contested memory in the machine.
- A guided tour of `>8300`–`>83FF`: the free zones, GPL's variables, KSCAN's state, the interrupt handler's territory, and the two system workspaces.
- **Occupancy by environment**: what is safe on a bare console, under Editor/Assembler, and with the GPL interpreter alive — three different maps.
- The **standard layout** this book's projects use — where `lib99` puts its workspace, stack, and hot variables.
- Running *code* in the pad: carving a few dozen bytes for an inner loop.
- `PADWATCH`: a tool that shows you exactly what any routine touches.

## The Bridge: Registers by Another Name

A modern CPU has dozens of registers — the fastest storage there is, on-chip, single-cycle — and a compiler spends real effort deciding which variables live in them and which spill to cache and memory. Register *allocation* is a whole discipline, because registers are scarce and fast and putting the right values in them is most of what makes code quick.

The TI-99/4A's scratchpad is register allocation at a different scale. The 9900's sixteen "registers" are themselves in memory (Chapter 4) — a workspace is sixteen consecutive words *somewhere*, and if you put that somewhere in the scratchpad, your registers are as fast as the machine allows; put it in expansion RAM and every register access pays the multiplexer tax. So choosing where your workspace sits *is* register allocation, done by hand, and the scratchpad is the pool of fast slots you allocate from — not sixteen, but a hundred-odd words, enough for a workspace, a stack, and the variables an inner loop touches most. The skill a compiler applies to registers, you apply to the scratchpad: keep the hot things in the fast island, let the cold things live in the slow expanse, and know exactly how many fast bytes you have to spend. This atlas is the map of that pool — which slots are yours to allocate and which are already spoken for.

## 24.1 Why 256 Bytes

The scratchpad's speed was established in Chapter 5: it is 16 bits wide and answers with no wait states, where expansion RAM is 8 bits wide behind the multiplexer and costs extra cycles per access. The practical multiplier is large — a workspace in the pad versus in expansion RAM changes the cost of every register-based instruction, which is *every* instruction — so code that runs hot (an inner loop, an interrupt handler, the game's per-frame work) wants its workspace and its most-touched variables in the pad, and the difference between a program that does and one that does not is measurable in the frame budget. The 256 bytes are precious because they are the only fast bytes, and the whole atlas exists to help you spend them without collision.

## 24.2 A Guided Tour, `>8300` to `>83FF`

Walk the 256 bytes. The territory divides, roughly, into your zone, the console's variable zone, and the two system workspaces at the top.

**The conventionally free lower pad (`>8300`–`>836F`, roughly).** The low scratchpad is the zone programs treat as theirs. This is where you put your workspace (this book's projects use `>8300`, so registers R0–R15 occupy `>8300`–`>831F`), your software stack (Chapter 9's R10 stack, growing down from a chosen top), and your hot variables. It is not *guaranteed* free in every environment (§24.3), but in a program that has taken over the machine it is the working space.

**The console's variable zone (`>8370`–`>83BF`, roughly).** The middle-to-upper pad is where the firmware and its services keep their state, and it is a minefield of specific bytes with specific meanings. Among the landmarks this book has already touched and verified: the **frame counter** the interrupt handler ticks every frame lives here at **`>8379`** (Chapter 22 watched it reach exactly 60 after sixty frames); the **user interrupt hook** the ISR checks is at **`>83C4`** (Chapter 22 verified a routine placed there runs every frame); the floating-point **FAC** and **ARG** accumulators are at **`>834A`** and **`>835C`** (Chapter 23); and KSCAN leaves its **key code and status** in known bytes here for a polling program to read. A boot-time probe confirms the shape of it: snapshot the pad after boot, run a few frames, and the bytes that change cluster in the `>8370`–`>837F` timer-and-counter region — the interrupt handler's per-frame footprint, exactly where the atlas says it should be.

**The GPL status byte (`>837C`).** One byte deserves singling out: the GPL status byte, which the interpreter and the console services use to pass condition information (Chapter 26). If the GPL side is alive, this byte is *not* yours.

**The two system workspaces (`>83C0` and `>83E0`).** The top 64 bytes are two 16-word workspaces the OS uses. `>83C0` is the **interrupt/utility workspace** — the registers the console's ISR and link routines run in — and `>83E0` is the **GPL workspace**, the sixteen registers of the GPL interpreter's virtual machine (Chapter 26). Each register in these has a meaning to the OS: the GPL workspace's high registers hold the interpreter's program counter, its data-stack pointer, its status — the state that *is* the running GPL machine. If GPL is executing, `>83E0`–`>83FF` is sacred; touch it and you derail the interpreter mid-instruction. Even a program that never calls GPL should leave these alone unless it has verified nothing GPL-side is alive.

## 24.3 Occupancy by Environment

The critical subtlety is that the map *changes* with what else is running. There are three environments, and what you may steal differs in each.

**The bare console** — your cartridge has taken over, interrupts masked, no GPL running (the poll-only architecture of Chapter 22). Here the pad is almost entirely yours: no ISR is ticking counters, no GPL is using its workspace, no KSCAN is running unless you call it. You may use nearly all 256 bytes, keeping clear only of anything the RESET/boot process left that you rely on. This is the freest environment and the one a self-contained game (Chapter 39's kind) creates deliberately, precisely to own the fast island whole.

**Under Editor/Assembler** — your program runs hosted by the E/A cartridge, which has installed its utility vectors (GPLLNK, XMLLNK — Chapter 23) and expects its own scratchpad usage to persist. Here a chunk of the pad is E/A's, and a program that trashes it loses the ability to call the very services it loaded E/A to use. You have the low pad and must respect E/A's territory.

**With GPL alive** — the interpreter is running (a GPL program, or the console using GPL services under you). Now `>837C` (status), `>83E0`–`>83FF` (the GPL workspace), and the GPL variable bytes are all in active use, and the interrupt handler is likely running too (its counters, its workspace at `>83C0`). This is the most constrained environment: you have the low pad and little else, and you must preserve everything the interpreter and ISR touch across any call that lets them run.

The discipline that falls out is: **know your environment, and preserve what it owns.** A routine that saves and restores the pad bytes it borrows can be safe in any environment; one that assumes the bare console's freedom will corrupt E/A or GPL when run there. This is why `lib99`'s routines are careful about the pad — they name their few pad words explicitly (Chapter 11's `equates.inc` discipline) and touch no others, so they compose into any environment without surprise.

## 24.4 The `lib99` Standard Layout

This book's projects share a scratchpad template, so that every case study inherits a known-good arrangement. The workspace sits at **`>8300`** (R0–R15 at `>8300`–`>831F`). The software stack (Chapter 9's R10 full-descending stack) grows *down* from a top the program chooses in the low-to-mid pad (`>8340` in most of this book's programs), leaving room between the workspace and the stack top. The library's own state — `textlib`'s cursor `CURPOS` (`>8342`), `textlib40`'s `CUR40` (`>8346`), and each module's few reserved words — occupies named locations just above the stack top, deliberately below the firmware's variable zone so they never collide with `>8379`, `>837C`, or the workspaces above. Every one of these is a named constant in `equates.inc` or the module's header, never a bare address, so the layout is legible and a conflict is a compile-time name clash rather than a runtime mystery.

The template's virtue is that it is *checked*: because every module names its pad words and touches no others, and because they all fit below the firmware zone, a program that composes several `lib99` modules gets a scratchpad with no internal collisions, and `PADWATCH` (the lab) can prove it — diff the pad around a sequence of `lib99` calls, and only the named words change. The layout is not arbitrary; it is the arrangement that keeps the fast island's tenants from fighting, encoded once and inherited everywhere.

## 24.5 Running Code in the Pad

The scratchpad holds *data* fast — but it holds *code* fast too, and the machine's tightest inner loops sometimes live there. Because instruction fetches from the pad are 16-bit and zero-wait, a loop that runs from the scratchpad executes faster than the same loop in expansion RAM or even paged cartridge ROM, and for a genuinely hot kernel — a pixel-plotting inner loop, a decompression core, a scanline routine — the few percent is worth carving out forty to eighty bytes of the precious 256 to hold it. The technique is a **loader**: at startup, copy the hot code from wherever it ships (cartridge ROM, a disk file) into a reserved patch of scratchpad, and call it there. It costs you those bytes for the program's life and the one-time copy, and it buys the fastest possible execution of the code that runs most.

This is a real optimization, and it is Chapter 37's territory to perfect — the measurement of whether a given loop earns the pad bytes, the loaders that stage code in, the trade of fast-island space against the data you displace. Here the point is only that the scratchpad is not just fast *storage* but fast *execution*, and that the same scarcity governs both: forty bytes of pad given to code are forty bytes not available for a stack or a variable, and the atlas is how you decide the split.

## Lab 24 — `PADWATCH`

The lab is the tool that makes the atlas empirical, in `code/ch24/`.

**`padwatch.a99`** — snapshot the whole scratchpad, run a target routine, then diff: report how many bytes changed and where. Its key trick is that its *own* workspace lives outside the pad (at `>A000`, expansion RAM), so the diff captures the *target's* footprint and nothing of PADWATCH's own register use. Build it:

```sh
libre99asm code/ch24/padwatch.a99 --format bin -o build/PWC.bin --symbols build/pw.map.json
```

With a target that writes one word to `>8340`, the bench reads back `NCHG = 2, FIRST = >8340` — two changed bytes, first at `>8340`, exactly the footprint of that write. Point PADWATCH at a `lib99` call and it shows the named pad words that routine touches; point it at a console service (in an environment where you can call one) and it reveals the service's true, often-undocumented pad usage. It is the empirical answer to "what did that touch?", and it turns the atlas from a chart you trust into one you can *check* — the same measure-first spirit as the graphics `pixels` and the sound `sound` oracles, applied to the fast island.

> **Sidebar — `>83C0`, the seed of chance.** One byte of the interrupt workspace has a second life as the console's source of randomness. The firmware's ISR, running every frame, updates a value used to seed random-number generation — a quantity that, because it advances on the unpredictable timing of when the program reads it relative to the 60 Hz beat, serves as a cheap entropy source. TI BASIC's `RND` draws on it; games seed their own generators from it. It is a small, characteristic piece of resourcefulness: with no hardware random generator, the machine mines its own interrupt timing for unpredictability, and one byte of the ISR's workspace becomes the reason no two games of a TI title play out quite the same. That a single scratchpad byte can be a frame counter's neighbor, a workspace register, and the wellspring of a game's luck is the atlas in miniature — 256 bytes doing the work of a much larger machine, every one of them spoken for.

## Exercises

**24.1** ✦ Why is the scratchpad faster than expansion RAM, and why does that make *where you put your workspace* a performance decision? (Reference Chapter 5.)

**24.2** ✦ Name three specific scratchpad addresses this book has verified in use and what each holds. (Hint: a frame counter, an interrupt hook, a floating-point accumulator.)

**24.3** ✦✦ Use `PADWATCH` to find the exact pad footprint of a `textlib` `TXPUTC` call: snapshot, call it, diff, and confirm the changed bytes are the ones `textlib` names (the cursor) and no others.

**24.4** ✦✦ For each of the three environments (bare, E/A-hosted, GPL-alive), list which pad regions you may freely use and which you must preserve, and explain the consequence of getting it wrong in each.

**24.5** ✦✦ Lay out a scratchpad plan for a program that needs a workspace, a 24-word stack, and eight hot variables, all below the firmware zone. Give the address of each and verify with `PADWATCH` that a run touches only those.

**24.6** ✦✦✦ Carve code into the pad: write a tiny hot loop, copy it into a reserved 40-byte patch of scratchpad at startup, run it there, and measure (with `cycles`) the speedup versus running it from cartridge ROM. Decide whether the pad bytes were worth it.

**24.7** ✦✦✦ Extend `PADWATCH` into a live visualizer: display the 256 bytes as a `textlib` grid, highlighting bytes that changed since the last frame, and run it under the interrupt handler to *watch* the ISR's per-frame footprint appear in real time.

## Further Reading

- *Editor/Assembler Manual* and the console ROM listings — the definitive (if scattered) map of the firmware's scratchpad usage, environment by environment.
- Chapter 5 (Console Memory Map) — why the scratchpad is the fast island, established and measured.
- Chapter 9 (Control Flow) — the R10 software stack that lives in the pad.
- Chapter 11 (Craftsmanship) — `equates.inc` and the discipline of naming every pad address once.
- Chapters 22 and 23 — the interrupt handler's and floating-point package's pad usage, the counterparts this atlas maps.
- Chapter 37 (Optimization) — running hot code in the pad, the technique §24.5 previews.

## Summary

The scratchpad, `>8300`–`>83FF`, is the only 256 bytes of 16-bit, zero-wait RAM the CPU has (Chapter 5's fast island) — the fastest and, because everyone wants it, the **most political** memory in the machine. It is register allocation by hand: your workspace, stack, and hot variables belong here for speed, but so do the GPL interpreter's registers, KSCAN's results, the floating-point accumulators, and the interrupt handler's counters. The atlas divides it: a conventionally **free lower pad** (`>8300`–`>836F`) for your workspace (`>8300`), stack, and variables; the firmware's **variable zone** in the middle-upper pad (the verified frame counter at `>8379`, the user hook at `>83C4`, FAC/ARG at `>834A`/`>835C`, KSCAN's bytes, the GPL status at `>837C`); and the two **system workspaces** at the top — `>83C0` (interrupt/utility) and `>83E0` (the GPL virtual machine, sacred while GPL runs). What you may take **depends on the environment**: the bare console gives you nearly all 256; Editor/Assembler reserves its utility area; a live GPL interpreter reserves its workspace, status, and variables — so the rule is *know your environment and preserve what it owns*, which `lib99` obeys by naming its few pad words (`>8300` workspace, stack from `>8340`, `CURPOS`/`CUR40` and friends below the firmware zone) and touching no others. The scratchpad holds fast **code** as well as data, so hot inner loops are sometimes staged into it (Chapter 37). And `PADWATCH` — snapshot, run, diff, with its own workspace safely outside the pad — makes the whole atlas checkable, reporting any routine's exact footprint (verified: a one-word write shows two changed bytes at `>8340`). Two hundred and fifty-six bytes, every one spoken for; the atlas is how you spend them without collision. Part V is complete — input, interrupts, and the console services and scratchpad that surround them.
