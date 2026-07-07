# Chapter 18 — Advanced and Modern VDP Topics

*The corners of the chip the datasheet whispers about, the frame's true bandwidth in one table, and the successors your 4A-first code should meet gracefully — the close of Part III.*

<!-- Part III — The Video Display Processor · target ≈18 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The §18.6 bandwidth cookbook is machine-verified on BENCH99 at commit 0d3e5d5 (cookbook.a99: VWTR ~188c, VSBW ~342c, VMBW ~73c/byte, raw stream ~62c/byte; combined with the ledgered frame 50k, bitmap-clear 66c/byte, name-rewrite 212c/cell, pattern-shift 1044c). Mid-frame split-screen is described from the project's beam-accurate per-scanline rasterizer (render_line reads live registers) — demonstrating it needs the running beam (desktop), not the whole-frame bench render. 9938/9958/F18A successors and 4K-mode/NTSC/PAL specifics are described per R-12: the project models the stock 9918A (60 Hz), and js99er/real hardware cover the F18A/9938; full F18A programming is Ch. 34/44. -->

## The Chip Beyond the Manual

We have spent six chapters treating the TMS9918A as a well-behaved machine with documented modes, and mostly it is. But every real chip has an underside — behaviors the datasheet mentions in a footnote or not at all, tricks that work because of *how* the silicon is built rather than what it promises, and edges where "it depends on your exact console and television" replaces the clean answer. This chapter is that underside, plus the two things a Part III ought to end with: an honest accounting of what the chip can actually move in a frame, and an introduction to the chips that came *after* the 9918A — the ones your code may someday run on, and should meet without falling over.

None of this is esoterica for its own sake. The mid-frame register trick is how a game puts a text scoreboard above a bitmap play-field on a chip that officially has one mode at a time. The status-read hazards are why a program that shares the machine with the console's interrupt handler — which is every program — must be careful about a register it might otherwise read casually. The bandwidth cookbook is the reference card working game programmers actually keep at their elbow. And the successors — the 9938, the F18A — are not museum trivia but live targets: a 4A program in 2026 might run on original hardware, on an F18A-upgraded console, or in an emulator configured as either, and the professional habit is to write code that degrades and enhances gracefully across that range rather than assuming one exact chip. Part III has taught you to command the 9918A; this chapter teaches you its limits, its bandwidth, and its heirs.

---

## What You Will Learn

- **Mid-frame register tricks**: split screens — a text panel over a bitmap field — by changing a register partway down the frame, and the timing that makes them fragile on a stock console.
- The **status-read hazards**: why reading the status register is a loaded action, and the interrupt-enable etiquette a program owes the console's own handler.
- **Mode-bit archaeology**: the 4K/16K memory bit, the external-video bit, and the undocumented corners worth knowing exist.
- **NTSC and PAL**: the artifact colours composite video invents, and the 50 Hz frame budget that changes your porting math.
- The **successors** — 9938/9958 and the F18A — as targets you may meet, and how to write 4A-first code that degrades and enhances gracefully.
- The **bandwidth cookbook**: measured VRAM throughput for every technique in Part III, in one table.

## The Bridge: Knowing the Edges

Modern graphics APIs work hard to hide the hardware's edges. A driver presents a clean, portable model; the messy specifics of a particular GPU — its exact timing, its undocumented quirks, its performance cliffs — are the driver-writer's problem, not yours, and code that pokes at them is code that breaks on the next card. The abstraction is the point.

The 9918A has no driver and no abstraction. You *are* the driver, and its edges are yours to know. This is not nostalgia for hardship — it is a different and, for a systems education, more revealing relationship with a machine. When you know exactly how many cycles a VRAM write costs, you can predict to the byte what a frame can do; when you know that reading the status register clears three flags, you can reason about who is allowed to read it and when; when you know the chip has no scanline interrupt, you understand why split-screens are cycle-counted and fragile. The edges are where the abstraction would have lied to you, and knowing them is the difference between a program that works on your machine and a program that works on *the* machine — every revision, every television, every successor. Part III's final chapter is about those edges, because commanding a chip means knowing where it ends.

## 18.1 Mid-Frame Register Tricks: Two Modes at Once

The 9918A displays one mode at a time — that is the rule of §12.4. But the beam draws one scanline at a time, from the *live* registers, at the moment it draws each line (Ch. 12). So if you change a register partway down the frame — after the beam has drawn the top of the screen but before it draws the bottom — the top and bottom are drawn under *different* register values. The chip still has "one mode," but it had one mode for the top and a different one for the bottom, and the seam is wherever you made the change.

This is the **mid-frame register trick**, and its signature use is the **split screen**: a bitmap play-field filling the top of the screen with a text status panel — score, lives, time — across the bottom, or the reverse. You set up bitmap mode, let the beam draw the play-field, then, at the right moment, rewrite the mode and base registers to text mode so the beam draws the panel from the text tables. Next frame, you switch back to bitmap before the top redraws, and repeat. The player sees two modes coexisting on a chip that has one.

The project's emulator models this faithfully, because its rasterizer is beam-accurate: it renders each scanline from the registers as they stand when that line is drawn (Chapter 12's `render_line`), so a register change between two lines shows exactly as it would on hardware — the change takes effect from the next line down, never retroactively. Demonstrating it, though, needs the *running* beam interleaved with your code — the register write must land between the beam's lines — which is the desktop emulator's domain (or real hardware), not the bench's whole-frame render; the bench proves the pieces (the modes, the registers) and the desktop shows them split.

The catch, and it is a serious one, is **timing**. The 9918A has no scanline interrupt — nothing tells you "the beam has reached line 128, change now." You must hit the moment yourself, by counting cycles from the frame interrupt (you know the frame is ~50,000 cycles and the beam descends at a known rate, so line 128 is a known number of cycles after F) or by using the 9901 timer (Ch. 22) to fire at the right instant. Either way it is fragile: a few cycles early or late and the seam jitters up or down, and any interrupt landing in the critical window throws the timing off. On a stock console the split is stable enough for a fixed panel if you count carefully and mask interrupts around the switch, but it is precision work, and it is why splits on the 9918A are usually a single, carefully-placed seam rather than the many-band extravaganzas later chips with scanline interrupts made routine.

## 18.2 The Status-Read Hazards Catalog

Reading the VDP status register looks innocent — one byte from `>8802` — but it is one of the most side-effect-laden actions on the machine, and Part III has met its hazards piecemeal. Here they are in one place.

**Reading status clears F, 5S, and C.** The read returns the flags and then clears the top three bits (Ch. 12). This is the acknowledgement that releases the frame interrupt — necessary and correct — but it means the flags are *consumed* by reading them. Read status and you have cleared the fifth-sprite and coincidence flags whether or not you looked at them.

**Two readers lose data.** Because the read consumes the flags, if *two* pieces of code read status in the same frame, the second sees them already cleared. This is the trap of sharing the machine with the console's interrupt handler: the console ISR reads status every frame to acknowledge the interrupt, so if your code *also* reads status hoping to check the coincidence flag, you and the ISR race, and usually the ISR wins — your read sees the flag already cleared by the handler that ran first. A program that wants the sprite flags either must read them before the ISR does (hard to guarantee) or must let the ISR capture them into a known location and read *that* — which is exactly what the console firmware does, mirroring the status byte into a scratchpad location the GPL/game side can read (Ch. 22). The lesson: with interrupts on, do not read the VDP status register directly for its sprite flags; read the copy the ISR leaves.

**Reading status resets the address flip-flop.** The read also resets the first-byte/second-byte flip-flop of the address port (Ch. 12) — which is a *feature* (it is how software recovers if it loses sync mid-address-write) but also a hazard, the flip side of §17.4's address race: an interrupt handler that reads status has just reset your address flip-flop, so a two-byte address setup straddling that interrupt is corrupted.

The etiquette that falls out of this is simple to state: **when the interrupt handler is not yours — and on the TI it usually is the console's — treat the status register as the ISR's to read, mirror what you need, and mask interrupts around any VDP access that must be atomic.**

## 18.3 Mode-Bit Archaeology

A few register bits are relics — present because the 9918A's family served machines other than the TI, and worth knowing exist even where the TI does not use them.

**The 4K/16K bit** (register 1, bit 7) selects whether the VDP addresses 4 KiB or 16 KiB of VRAM. The TI-99/4A wires 16 KiB and sets the bit; every layout in this book assumes 16 KiB. But the 9918A could be built with only 4 KiB (a cheaper machine), and in 4K mode the address counter wraps at 4 KiB and the table-base registers mean less — a corner the TI never occupies but that explains why the bit exists and why forgetting to set it (Ch. 14's warning) drops you into a cramped, wrong-addressing world. The project models the 16 KiB the 4A ships; 4K mode is archaeology, not a target.

**The external-video bit** (register 0, bit 0) enables the chip to overlay its output on an external video source fed to a pin — for genlock and video-titling hardware that the base console does not have. On a stock 4A it does nothing useful and stays clear. It is a window into the 9918A's other life as a video-overlay chip, and a reminder that the "TI VDP" is a general part the TI used a slice of.

There are other whispered corners — undefined mode-bit combinations (which our emulator, like hardware, degrades toward a text-like display), the exact behavior of sprites at the screen edges, the precise contents of the status low bits when no fifth sprite exists — and the honest posture toward all of them is the same: know they exist, do not depend on them, and when a program seems to rely on an undocumented behavior, verify it against real hardware or a reference emulator (Classic99, MAME) before trusting it. Undocumented is not the same as reliable.

## 18.4 NTSC, PAL, and the Colours Between the Colours

The 9918A does not emit red-green-blue; it emits a **composite** video signal, the single-wire standard of 1980s televisions, and composite has properties that pure digital colour does not. The most surprising is that the chip can display colours *between* its fifteen — **artifact colours** — because the composite encoding of certain high-frequency pixel patterns produces hues the palette does not name. A checkerboard of two palette colours, at the right spacing, reads on a real NTSC television as a third colour that exists nowhere in the chip. Some programs exploited this deliberately to widen the palette; the effect depends entirely on the composite signal and the television decoding it, so it appears on real NTSC hardware and a signal-accurate emulator but not in an emulator that renders the clean digital palette (as the project's does — our fifteen colours are the digital truth, not the composite artifact). It is a genuine capability of the *system* — chip plus signal plus TV — that the chip alone does not have.

The other axis is **NTSC versus PAL**. North American and Japanese TIs use NTSC: 60 fields a second, 262 lines, the ~50,000-cycle frame this book has measured. European TIs use PAL: **50** fields a second, 313 lines. The consequences for a programmer are two. First, the frame budget *changes*: at 50 Hz the frame is longer — about 60,000 cycles at the same CPU clock — so a PAL machine gives you more cycles per frame but fewer frames per second. Second, anything timed in frames runs *slower* on PAL: a game that spawns an enemy every 60 frames spawns one per second on NTSC and one every 1.2 seconds on PAL, and music or animation counted in frames plays 5/6 as fast. Porting a game between the standards means rescaling everything counted in frames, which is why serious games either detect the standard (the frame count between two real-time events differs) or are tuned for one and accept the drift on the other. The project emulator models NTSC timing; PAL is a porting consideration this book flags rather than a mode it switches.

## 18.5 The Successors: Targets You May Meet

The 9918A had heirs, and in 2026 they are not history — they are configurations your code may run on.

The **TMS9938 and 9958** (the "V9938"/"V9958," the MSX2 video chips) are the 9918A's direct descendants: backward-compatible with its modes, but adding 80-column text, more VRAM (up to 128 KiB), a genuine hardware **scroll register** (the thing §17.4 measured the lack of), more colours, and more sprites per line. A 9918A program runs on them unchanged, in its old modes; a program *written* for them can do things — smooth hardware scrolling, 80-column word processing — the 9918A must fake or forgo. The TI never shipped one, but upgrade cards and later machines did, and they are the "what the productivity market wanted" answer to Chapter 14's forty columns.

The **F18A** is the modern one: an FPGA re-implementation of the 9918A that drops into a real 4A in place of the original chip, is cycle-and-pixel compatible with everything Part III taught, and *adds* — VGA output, more colours, a hardware scroll register, **scanline interrupts** (making §18.1's fragile splits routine and stable), and an embedded GPU for offloading work. It is a live upgrade with a real community, and it poses the graceful-degradation question sharply: an F18A-aware program can light up its extra features, but must still run — as a plain 9918A program — on an unmodified console. The discipline is to **detect** the enhanced chip (the F18A answers a probe the 9918A does not) and branch: use the scroll register and scanline interrupts where present, fall back to pattern-shift scrolling and cycle-counted splits where not. This book is 4A-first: everything here runs on the stock chip, and the F18A's own programming — its GPU, its enhanced registers — is deferred to Chapters 34 and 44, where modern peripherals and the living platform are the subject. The project emulator models the **stock 9918A**; to run F18A-enhanced code you reach for js99er.net or real hardware (R-12), and a well-written program notices the difference and adapts.

## 18.6 The Bandwidth Cookbook

Here is the reference card — every VRAM-throughput number Part III measured, in one place, all on the bench. It answers the only question that finally matters for real-time graphics: *how much can I move in a frame?*

| Operation | Measured cost | Notes |
|---|---|---|
| **One frame (60 Hz)** | **~50,000 cycles** | the whole budget |
| Register write (`VWTR` + operands) | ~190 cycles | 8 registers ≈ 1,500 cycles |
| One VRAM byte (`VSBW`, aim + write) | ~340 cycles | the *aim* dominates — see below |
| Bulk copy (`VMBW`) | ~73 cycles/byte | aim once, stream N |
| Raw stream (aim once, `MOVB` loop) | ~62 cycles/byte | the floor for VRAM writes |
| Full name-table rewrite (768 cells + addr math) | ~163,000 cycles | ~212/cell; **3.3 frames** |
| Bitmap bring-up + clear (13,056 bytes) | ~870,000 cycles | ~66/byte; **17 frames** |
| Pattern-shift scroll (1 character, 8 bytes) | ~1,044 cycles | smooth, 1 px/frame |

Two derived numbers are the ones to memorize. Dividing the frame by the streaming cost: you can push **~700–800 VRAM bytes per frame** and nothing else — call it *one screenful of tiles, or nothing else that frame*. And the single-byte cost tells the deepest truth of the whole port: **~340 cycles for one byte, but ~62 for each byte in a stream.** The address *aim* is the tax — nearly 300 cycles of it — and it is paid once per stream or once per single byte. This is why every technique in Part III is built on *aim once, stream many*: `VMBW` over `VSBW`, the pattern-shift scroll that rewrites eight contiguous bytes, the bulk image load of §15.6. Scatter your writes and the aim tax bankrupts you; batch them and the frame affords real work. The cookbook is not trivia; it is the budget, and every design in Parts IX and X is negotiated against it.

## Lab 18 — The Split-Screen Scoreboard and a Tale of Two Chips

The lab has two halves, and it is the capstone of Part III.

**A split-screen scoreboard atop the Chapter 17 scroller.** Take TERRAIN's pattern-shift play-field and add a text status panel across the bottom by the §18.1 mid-frame trick: count cycles from the frame interrupt to the seam scanline, switch the mode and base registers to text there, and switch back before the top redraws. On the project emulator (beam-accurate) and on real hardware the two modes coexist; the timing is the craft, and Chapter 22's frame interrupt and timer are the tools that make the seam stable. The `cookbook.a99` measurements tell you, before you write a line, exactly how many cycles the switch and the panel redraw may spend without blowing the frame.

**A tale of two chips.** Run the *same* graphics code on the stock 9918A (the project emulator) and on an F18A configuration (js99er.net or real hardware), and document what differs: where the F18A's hardware scroll would replace your pattern-shift, where its scanline interrupt would replace your cycle-counted seam, where its extra colours would relax the attribute clash. The exercise is not to rewrite for the F18A — that is Chapter 34 — but to see your 4A-first code run unchanged on the newer chip, and to identify exactly the seams where a detection-and-branch would let one program serve both. This is graceful degradation made concrete: one binary, two chips, the stock behavior everywhere and the enhancement where it is offered.

## Exercises

**18.1** ✦ Using the cookbook, compute how many whole 8-byte characters you can stream into VRAM in one frame (`VMBW` rate), and how many you could write as isolated single bytes (`VSBW` rate). Explain the ratio in one sentence.

**18.2** ✦ Why must a program that runs with the console's interrupt handler active *not* read the VDP status register directly to check the coincidence flag? Where should it read the flag instead?

**18.3** ✦✦ The mid-frame split relies on the beam drawing each line from live registers. Explain why the switch must be timed to a specific scanline, why the 9918A makes this hard (what it lacks that later chips have), and two ways (§18.1) to hit the moment.

**18.4** ✦✦ A game spawns an enemy every 50 frames and plays music at one beat per 15 frames, tuned on an NTSC machine. Give the real-world spawn interval and beat tempo on a PAL machine, and describe the minimal change that would make the game run at the same real speed on both.

**18.5** ✦✦ Write a probe that would distinguish an F18A from a stock 9918A at run time (consult the F18A documentation for its detection protocol), and sketch the branch structure of a program that uses the hardware scroll register where present and pattern-shift scrolling where not.

**18.6** ✦✦✦ Build the split-screen scoreboard: bitmap (or Graphics I) play-field on top, text panel on the bottom, the seam placed by counting cycles from the frame interrupt. Measure, with `cycles`, the cost of the mid-frame switch, and show it plus the panel redraw fits the budget the seam leaves. (The stable seam needs Chapter 22's interrupt; here, place it by cycle count and observe it on the desktop emulator.)

**18.7** ✦✦✦ Extend `cookbook.a99` with two more measurements the labs will want: the cost of moving all 32 sprites (64 SAL bytes via one `VMBW`) and the cost of a Graphics I double-buffer flip (one register write). Add them to your own copy of the §18.6 table and confirm both are trivial against the frame.

## Further Reading

- *TMS9918A Video Display Processor Data Manual*, Texas Instruments — the register bits (4K/16K, external video), the composite output, and the timing this chapter's edges live in.
- *V9938 Technical Data Book* — the 9918A's direct successor: the modes, the scroll register, and the compatibility §18.5 relies on.
- The F18A project documentation (Matthew Hagerty) — the FPGA VDP's enhanced registers, scanline interrupt, and detection protocol; the target of §18.5 and Chapter 34.
- Chapter 12 (Inside the TMS9918A) — the beam and the status-read side effects this chapter catalogs.
- Chapter 17 (Motion) — the frame budget and the address race the bandwidth cookbook and §18.2 quantify and complete.
- Chapter 22 (Interrupts and Time) — the frame interrupt and 9901 timer that make §18.1's split-screen seam stable.
- Chapters 34 and 44 (Modern Peripherals; The Living Platform) — the F18A programmed for real, and the platform's continuing life.

> **Sidebar — "Impossible on a 9918A."** Every constraint in Part III has been broken by someone. The chip has four sprites per line — and the demo scene has put dozens on screen by multiplexing so fast the flicker becomes texture. It has no hardware scroll — and programmers scrolled whole worlds by pattern-shift. It has fifteen colours — and artifact tricks found more, mid-frame register changes stacked modes, and colour cycling animated the un-animate. The lesson of the demo scene, on this machine as on every constrained machine, is that "impossible" usually means "impossible the obvious way." The hardware's limits are real and this book has measured them honestly; but a limit measured is also a limit *understood*, and understanding is where the clever way begins. Part III has given you the honest limits. What you do at their edges is the art.

## Summary

This final Part III chapter is the chip's underside and its context. **Mid-frame register tricks** exploit the beam's per-scanline draw from live registers: change a register partway down the frame and the top and bottom draw under different modes — the **split screen**, a text panel over a bitmap field — which the project's beam-accurate rasterizer models but which is fragile on the 9918A because the chip has no scanline interrupt, so the seam must be hit by cycle-counting or the 9901 timer. The **status-read hazards** are that reading `>8802` clears F/5S/C and resets the address flip-flop, so two readers lose data and a program sharing the machine with the console ISR must let the handler read status and mirror the flags rather than read them directly. **Mode-bit archaeology** covers the 4K/16K bit (the TI wires 16K), the external-video overlay bit (unused on a stock 4A), and the posture toward undocumented corners: know they exist, do not depend on them. **NTSC/PAL**: composite video invents artifact colours the palette lacks (system, not chip), and PAL's 50 Hz gives a longer ~60,000-cycle frame but runs frame-counted timing 5/6 as fast — a porting rescale. The **successors** (9938/9958, F18A) are compatible upgrades adding hardware scroll, more colours, 80 columns, and scanline interrupts; 4A-first code should detect and branch, using enhancements where present and the Part III techniques where not (the project models the stock 9918A; js99er/hardware cover the F18A). And the **bandwidth cookbook** puts every measured throughput in one table — the frame is ~50,000 cycles, a VRAM byte streamed is ~62 cycles but ~340 aimed-and-written alone, so ~700–800 bytes move per frame and *aim once, stream many* is the law behind every technique in Part III. Part III is complete: from the beam with no framebuffer (Ch. 12) to the chip's honest bandwidth (Ch. 18), you can now make the TMS9918A show, move, and scroll anything it is capable of — and know exactly where that capability ends.
