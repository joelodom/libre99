# Chapter 19 — The Sound Generator: Music and Effects Engineering

*Three square waves and a hiss of noise, poked one byte at a time — and out of that austerity, four decades of game music.*

<!-- Part IV — Sound and Speech · target ≈26 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The SN76489 latch/data protocol, the f=3,579,545/(32N) frequency math, the equal-tempered note table (all within +/-0.25%), attenuation, and white/periodic noise machine-verified on BENCH99 at commit 74c9b90 (which added the `sound` PSG-state oracle) — sndlib plays a C-major chord that reads back C4 261.4 / E4 330.0 / G4 392.5 Hz, all att 2, white noise att 8. Code in code/ch19/ (sndlib). The sound chip is fully emulated (psg.rs); frame-timed playback (the ISR auto-player) needs Ch. 22, and the tracker/VGM asset pipeline is Ch. 38 (no python on the PC workstation). TI Invaders register-log dissection is described, not run (no ROM). -->

## Four Voices, One Byte at a Time

The TI-99/4A makes sound with a chip barely more complicated than a handful of counters: the **SN76489**, three channels that each emit a square wave at a frequency you choose, plus a fourth that emits noise. That is the whole instrument. No samples, no waveforms you can draw, no filters — three tones and a hiss, each with a volume knob, and a single write-only port at `>8400` to command them all.

And yet. Out of those three square waves came the music of an era — the title themes, the laser zaps, the explosions, the descending menace of an invader fleet speeding up as it falls. The SN76489 (and its siblings across the ColecoVision, the Sega Master System, the BBC Micro) is one of the most-heard chips in computing history, and learning to program it is learning a discipline that sample-based audio has almost erased: making *music* out of arithmetic, making *character* out of constraint. A square wave is a harsh, buzzy thing; three of them and a noise channel is not an orchestra. The art is in what you do with the limits — the fast arpeggios that fake a chord the chip cannot hold, the pitch slides that turn a tone into a zap, the noise rates that become a snare or a surf — and this chapter is that art, built into `sndlib` and proven on the bench a frequency at a time.

Because the chip is *write-only* — you command it and it obeys, but it never answers — we need a way to see what we told it. BENCH99 gained one for this chapter: the `sound` command reads the chip's registers through the emulator's diagnostic window and prints each channel's frequency and volume, so every claim in this chapter — that this byte makes that note — is verified, not asserted.

---

## What You Will Learn

- The SN76489's structure — three tone channels, one noise channel — and the **latch/data** byte protocol that commands them through `>8400`.
- The **frequency math**: from the master clock to a 10-bit divider, and building an equal-tempered note table.
- **Attenuation** in 2 dB steps, and shaping notes over time with software **envelopes**.
- The **noise** channel: white versus periodic, the three rates, and the tone-tracked mode — drums, engines, explosions.
- An **effects cookbook**: pitch slides, vibrato, arpeggios, zaps, and alarms as reusable patches.
- The console **sound-list format** and its interrupt-driven auto-player — one format the whole platform shares.
- A real **music driver**, and the modern pipeline from a tracker to TI sound data.

## The Bridge: From Samples Back to Synthesis

Modern game audio is sample playback. A sound is a recording — a `.wav`, an `.ogg` — and the engine mixes dozens of them in floating point, applies reverb and spatialization, and hands the result to a DAC. Music is often just a long recording too. The synthesizer, where sound is *generated* from oscillators and math rather than played back from a recording, survives mainly inside instruments and the demoscene. To a modern programmer, "make a laser sound" means "find or record a laser sound and play the file."

The SN76489 cannot play a file. It has no memory for samples and no path to stream them; it has three oscillators and a noise source, and the only way to make a sound is to *synthesize* it — to compute, from the physics of what you want to hear, the register values that produce it, and to change those values over time to shape it. This is the older and more fundamental relationship with sound, and it is a genuine education: to make a laser zap you must understand that a zap is a tone whose pitch falls rapidly, and *write the falling pitch*; to make an explosion you must know it is noise that decays, and *write the decay*. The chip forces you to know what a sound *is*, because you are building it from arithmetic rather than replaying it from disk. That knowledge — sound as synthesis, not playback — is worth having even in 2026, and this is the chapter that teaches it.

## 19.1 The Chip: Three Tones, One Noise, One Port

The SN76489 has four channels: **tones 0, 1, 2**, each a square-wave oscillator, and **channel 3, noise**. You command all four through the single write-only byte port at `>8400`, and every byte you send is one of two kinds, distinguished by the top bit.

A **latch** byte has bit 7 set: `1 cc t dddd`. Bits 6–5 (`cc`) select the channel (0–2 tone, 3 noise); bit 4 (`t`) selects the register type (0 = tone period or noise control, 1 = attenuation); and the low four bits (`dddd`) are data. A **data** byte has bit 7 clear: `0 · dddddd`, and its low six bits become the *high* six bits of whichever tone period was last latched. So setting a tone's 10-bit period takes two bytes — a latch carrying the low four bits, then a data byte carrying the high six — while setting an attenuation takes just one latch byte, because attenuation is only four bits.

`sndlib`'s `STONE` writes exactly that sequence — latch the period's low nibble, data the high six bits, then latch the attenuation:

```asm
STONE  MOV  R0,R3
       SLA  R3,5              ch << 5
       ORI  R3,>0080          | >80  -> the tone-latch base for this channel
       MOV  R1,R4
       ANDI R4,>000F          N AND >0F
       A    R4,R3             the latch byte
       SWPB R3
       MOVB R3,@SNDWR         write latch (period low 4 bits)
       MOV  R1,R4
       SRL  R4,4
       ANDI R4,>003F          (N >> 4) AND >3F
       SWPB R4
       MOVB R4,@SNDWR         write data (period high 6 bits)
       ... (attenuation latch) ...
```

One console-specific wrinkle worth knowing: the TI wires the sound chip so that a write only reaches it when the *address* is even — the firmware's `wsndbyte` routine, and our emulator, discard a write whose address has bit 0 set. Write to `>8400`, not `>8401`. It is a hardware-decoding detail that becomes a baffling "my sound driver does nothing" bug if you get the address parity wrong, and a reason `sndlib` names the port once (`SNDWR`, from `equates.inc`) and never types the bare address again.

## 19.2 Frequency Math: From a Clock to a Note

A tone channel is a divider. It counts down from a 10-bit value `N` — the *period* — at a rate derived from the chip's master clock, toggling its square-wave output each time it reaches zero, so a smaller `N` toggles faster and produces a higher pitch. On the TI the master clock is 3,579,545 Hz (the NTSC colourburst frequency, divided down), the divider runs at one-sixteenth of it, and the square wave is one full cycle per two toggles, so the output frequency is:

```text
f = 3,579,545 / (32 · N)
```

Ten bits of `N` (1 to 1023) gives a range from about 3,494 Hz at `N = 1` down to about 109 Hz at `N = 1023` — a bit under a decade, roughly the top five octaves of a piano. To play a *note*, you invert the formula: for a target frequency `f`, `N = 3,579,545 / (32 · f)`, rounded. Concert A (A4, 440 Hz) gives `N = 254`, and the bench confirms it to the tenth of a hertz — `sndlib` playing A4 reads back `f = 440.4 Hz`.

Music wants not one note but a scale, and the scale is **equal temperament**: twelve semitones per octave, each a frequency ratio of the twelfth root of two. Computing `N` for every note is a job for a small tool — the outline's "twenty-line Python script," and the first of the PC-side asset scripts Chapter 38 builds in earnest. `sndlib` ships the resulting table for one octave, C4 through B4:

```asm
NOTES  DATA 428,404,381,360,339,320    C4 C#4 D4 D#4 E4 F4
       DATA 302,285,269,254,240,226    F#4 G4 G#4 A4 A#4 B4
```

Two facts make this one octave enough. An octave *up* is exactly double the frequency, hence half the divider — shift `N` right one bit. An octave *down* doubles `N` — shift left. So the whole playable range comes from twelve numbers and a shift. And the tuning is good: every entry lands within a quarter of a percent of its ideal frequency, well inside what the ear accepts as in-tune. The bench proves it by playing a C-major chord — C4, E4, G4 on the three channels at once — and reading back **261.4, 330.0, and 392.5 Hz**, the triad the table promised.

> **Sidebar — One chip, many childhoods.** The SN76489 is not a TI curiosity; it is one of the most widely heard sound chips ever made. The same three-tones-and-noise design (sometimes under a different part number, sometimes integrated into a larger chip) sang in the ColecoVision, the Sega SG-1000, Master System, and Game Gear, the BBC Micro, the IBM PCjr and Tandy 1000, and more. A generation who never touched a TI-99/4A grew up on its exact timbre — the particular buzz of its square wave, the specific grit of its noise — because the chip travelled. Learning to program it here is learning an instrument that a remarkable slice of 1980s computing shared, and the arpeggio and pitch-slide techniques of §19.5 are, almost verbatim, the techniques of Master System and BBC composers too. The TI's voice was, in the most literal sense, a common one.

## 19.3 Attenuation and Envelopes

Each channel has a four-bit **attenuation** register: 0 is full volume, each step up removes 2 decibels, and 15 is silence. Attenuation, not amplitude — the number is how much you turn the channel *down*, so 0 is loudest and 15 is off. `sndlib`'s `SVOL` sets it, and `SSILNC` sets all four to 15 to hush the chip.

A constant attenuation is a flat, organ-like note — it starts abruptly, holds, and stops abruptly. Real instruments do not sound like that; they have a *shape* over time — a sharp attack, a decay, a sustained level, a release — and you make that shape on the SN76489 with a software **envelope**: a table of attenuation values that you step through over successive frames, writing each to the channel in turn. A plucked-string envelope starts at attenuation 0 (loud) and climbs quickly toward 15 (silent) — loud onset, quick fade. A pad starts at a middle attenuation and holds. An explosion starts loud and decays smoothly to nothing. The envelope is just a list of numbers and a per-frame index, and it is the difference between a beep and a *sound*: the same tone with a percussive envelope is a pluck, with a slow attack is a swell, with a fast decay is a blip. The exercises build an envelope engine; the idea is that volume, like pitch, is something you *animate* over the frames of Chapter 17's loop, not set once.

## 19.4 Noise: Drums, Engines, and Explosions

Channel 3 is different: instead of a square wave it emits **noise**, generated by a 15-bit linear-feedback shift register (the same idea as Chapter 8's random numbers, now clocked into an audio hiss). Its control register (a latch of type 0 on channel 3) is three bits: bit 2 chooses **white** noise (a full hiss) versus **periodic** noise (a buzzier, more pitched rasp, because a shorter feedback makes it nearly repeat), and the low two bits choose the shift *rate* — three fixed rates (roughly high, medium, low), or a fourth mode that follows tone channel 2's period, so you can *tune* the noise by tuning channel 2.

That small palette is a surprising range of percussion and texture. White noise at a fast rate, with a quick decay envelope, is a **snare** or a hi-hat. White noise at a slow rate is **surf** or **wind** or a rocket's **rumble**. Periodic noise is a **buzz** — an engine, a laser's grit. And the tone-tracked mode lets the noise *pitch-bend* — a descending tuned-noise sweep is a classic explosion or a UFO. `sndlib`'s `SNOISE` sets the control and attenuation; the bench confirms white noise selected and attenuated to 8. A game's entire drum kit is a handful of noise-control-plus-envelope patches, and its explosions are noise with a decay — the same channel, shaped differently.

## 19.5 The Effects Cookbook

An effect is a *changing* sound — register values swept over frames — and a small vocabulary of sweeps covers most of what a game needs. Each is a reusable patch: a short routine, driven once per frame from the game loop, that walks a channel's registers through a shape.

A **pitch slide** writes a rising or falling sequence of `N` values to a tone channel — falling for a laser zap or a falling bomb, rising for a power-up. A **vibrato** wobbles `N` slightly around a centre with a small periodic offset, the warble that makes a held note sound alive. An **arpeggio** — the poor chip's chord — cycles a single channel rapidly through the notes of a chord, a new note every frame or two, so fast the ear fuses them into a shimmering harmony the chip could never hold as three separate sustained tones; it is *the* signature sound of constrained chip music, three voices' worth of harmony from one voice's worth of hardware. A **laser zap** is a fast downward pitch slide with a quick decay; an **alarm** is two notes alternated; a **siren** is a slow triangular pitch sweep. Collected into an `sfxlib` of named patches — `SFXZAP`, `SFXARP`, `SFXBOOM` — they become a game's sound-effects vocabulary, each a call and a per-frame update. The common thread is Chapter 17's lesson again: sound, like motion, is something you *update every frame*, and an effect is an animation of registers.

## 19.6 The Console Sound-List Format

The TI has a standard way to represent a piece of sound over time, and — beautifully — it is *one* format that the whole platform shares: the console firmware plays it, GPL plays it (Ch. 26), and your assembly can play it. A **sound list** is a stream of duration-tagged blocks. Each block is a count of how many bytes to write to the sound chip, then that many bytes (the latch/data commands — notes, attenuations, noise settings), then a **duration** in sixtieths of a second: "write these commands, then wait this many frames." Block follows block, and a duration byte of zero (or a count that signals the end) terminates the list. A whole tune, or a whole sound effect, is one such list — a compact, position-independent blob of data you can store in ROM and hand to a player.

The player can be the console's own. The firmware's frame interrupt handler (Ch. 22) includes a **sound-list auto-player**: you put a pointer to your list in a known scratchpad location and set a flag, and every frame the ISR advances the list — writing the current block's bytes when its duration expires, moving to the next — entirely behind your back, while your main code does other things. You start a tune with two pokes and the music plays itself. It is the same mechanism GPL uses, which is why TI BASIC's `CALL SOUND` and an assembly game's score and a cartridge's jingle are all, underneath, the same sound-list format fed to the same player. Writing lists by hand is tedious, so a real project defines assembler macros — `NOTE`, `REST`, `DUR` — that expand into the format musically, turning a score into readable source. (Driving the auto-player needs the running interrupt handler, so this chapter builds the *format* and the register control, and Chapter 22 installs the ISR that plays a list unattended.)

## 19.7 A Real Music Driver

A sound list is a *stream* — every command spelled out in order — which is simple but bulky, and rigid: to change a repeated phrase you must change every copy. A real music driver borrows the **tracker** model instead: the music is **patterns** (short blocks of notes) arranged by an **order** list (play pattern 3, then 1, then 1 again, then 4), so a repeated chorus is stored once and referenced many times, and the whole song is patterns plus a playlist. The driver keeps, per channel, a position in the current pattern and a countdown to the next note, advances them each frame, and writes the resulting notes to the chip. It is a small state machine per voice, driven from the frame loop.

The hard part of a music driver is **channel stealing**. A game has three tone channels, and its music wants all three — but then a laser fires, and the zap effect also wants a channel. Something must give. The driver assigns **priorities**: a sound effect temporarily *steals* a music channel (usually the least important melodic voice), plays its effect, and hands the channel back when done, the music resuming on it. Get the priorities right and the effects punch through without the music ever seeming to stop; get them wrong and either the effects are inaudible under the music or the music lurches every time a gun fires. This arbitration — three voices shared between an ongoing score and interrupting effects — is the central engineering problem of chip-tune game audio, and it is why the DODGE score (the lab) is not just "play some notes" but a driver that yields channels gracefully.

## 19.8 The Modern Pipeline

Composing SN76489 music by hand-writing note tables is possible but painful; in 2026 you compose in a **tracker** — a modern one, or a period-authentic one — that targets the chip, auditioning the actual timbres as you write. From the tracker you capture the register writes as a **VGM** file (Video Game Music, a logged stream of chip commands with timestamps — essentially a recording of every byte sent to the chip and when), which is the interchange format the chip-music community shares. Then a converter turns that VGM log into the TI's sound-list format (or into your music driver's pattern data), compressing the repetitive register streams — a tone held for a second is one command and a long duration, not sixty identical writes — to fit ROM. The pipeline is *tracker → VGM → TI data*, and every stage but the last runs on your PC; the last is the converter, another Chapter 38 asset script. The size/quality trade is real — a richly-arranged tune is more pattern data — but the SN76489's austerity helps here too: three voices and noise do not generate much data, and a whole game's music fits comfortably in a few kilobytes of ROM.

## Lab 19 — `sndlib` and Scoring DODGE

The lab is the sound engine and its first real use, in `code/ch19/`.

**`sndlib` (`sndlib.inc` + `sndlib.a99`)** — the sound engine for `lib99`: `SSILNC` (silence all channels), `STONE` (play a tone: channel, divider, attenuation), `SVOL` (set a channel's attenuation), `SNOISE` (set the noise channel), and the `NOTES` equal-tempered table. Build and prove it:

```sh
libre99asm code/ch19/sndlib.a99 --format bin -o build/SNDLC.bin --symbols build/sndl.map.json
```

On the bench, `load`, `pc` to the entry, `x 200`, then `sound` — the self-test plays a C-major chord, and the oracle reads it back: `ch0 261.4 Hz, ch1 330.0 Hz, ch2 392.5 Hz`, each at attenuation 2, with white noise underneath. Change the `NOTES` indices and hear (see) a different chord; this is where you confirm, byte by byte, that your driver plays what you meant.

**Scoring DODGE** is the lab's larger aim: give Chapter 16's game a title tune, an in-game loop, and six effects (a shoot, a hit, an explosion, a power-up, a menu blip, a game-over sting), with the priority discipline of §19.7 so the effects steal channels from the music and hand them back. The music and effects are `sndlib` calls and sound lists; making them *play in time* needs the frame loop of Chapter 17 and, for the unattended auto-player, the interrupt handler of Chapter 22 — so DODGE's full scoring assembles across those chapters, and here you build and verify the instrument the score is written for. The exercises take the first steps: an envelope engine, an arpeggio patch, and a two-channel jingle.

> **Field Notes — The invaders speed up.** The most famous piece of chip-audio engineering on any platform is the descending four-note loop of the *Space Invaders* lineage — the marching bass that plays faster as the invaders thin out, tightening the tension without a line of "AI." On the TI, TI Invaders does the same, and you can dissect it the way you dissect any chip music: capture the writes to `>8400` in a debugger, and the four repeating tone latches and their durations fall out as four notes and a tempo. Speed up the loop — shorten the durations — as the game progresses, and the music accelerates. The technique is a masterclass in doing much with little: one channel, four notes, a shrinking duration, and the whole game feels faster. (Reading a specific commercial ROM's register log to notate its loop is an exercise best done against the real cartridge in the debugger; the method — register log to notation — is the transferable part, and it is exactly what the `sound` command and a trace log let you do to your *own* music while developing it.)

## Exercises

**19.1** ✦ Give the two bytes that set tone channel 1 to `N = 300`, and the one byte that sets its attenuation to 4. (Latch/data, §19.1.)

**19.2** ✦ Using `f = 3,579,545 / (32·N)`, find the frequency for `N = 254` and for `N = 127`. What musical interval separates them, and why? (Look at the ratio of the `N` values.)

**19.3** ✦✦ Extend `NOTES` (or write `SNOTE ch, octave, semitone`) to play any octave by shifting the base divider. Play a two-octave C-major scale, verifying a few notes with `sound`.

**19.4** ✦✦ Write an envelope engine: a per-frame routine that walks a channel's attenuation through a table and stops at 15. Give a tone a percussive envelope (0 → 15 over eight frames) and a swell (12 → 0 over sixteen), and confirm the attenuation changes each frame with `sound`.

**19.5** ✦✦ Build `SFXARP`: an arpeggio patch that cycles one channel through a three-note chord, a new note per frame. Verify with `sound` across successive frames that the channel's frequency steps through the chord.

**19.6** ✦✦ Write a laser zap (`SFXZAP`): a fast downward pitch slide with a quick decay envelope. Log the channel's `N` each frame and confirm it falls; describe how the slope of the fall changes the character of the zap.

**19.7** ✦✦✦ Design a sound-list format and a *manual* player (not the ISR): a routine you call each frame that advances a pointer through duration-tagged blocks, writing commands when a block's duration expires. Play a short two-channel jingle and verify the notes with `sound` at the right frame counts.

**19.8** ✦✦✦ Give DODGE a minimal score: a two-channel background loop and a hit effect that steals the second channel, plays, and restores it. Using `sound`, confirm the background note returns after the effect finishes — the channel-stealing of §19.7 made audible (visible).

## Further Reading

- SN76489 datasheet and community documentation — the latch/data protocol, the noise generator, and the attenuation table this chapter's `sndlib` implements.
- *Editor/Assembler Manual*, Texas Instruments — the console sound-list format and the interrupt auto-player's scratchpad interface (§19.6).
- The VGM format specification — the chip-command log format §19.8's pipeline captures and converts.
- Chapter 8 (Arithmetic) — the LFSR behind the noise channel and the fixed-point math behind pitch slides.
- Chapter 17 (Motion) — the frame loop that drives every envelope, effect, and music update in this chapter.
- Chapter 22 (Interrupts) — the interrupt handler that plays a sound list unattended.
- Chapter 38 (Asset Pipeline) — the note-table generator and the VGM-to-TI converter built for real.

## Summary

The TI's sound is the **SN76489**: three square-wave tone channels and one noise channel, commanded through the write-only port `>8400` by **latch** bytes (bit 7 set: channel, type, low four data bits) and **data** bytes (bit 7 clear: the high six bits of the last-latched tone period), so a tone period is two bytes and an attenuation is one — and writes only reach the chip at even addresses. A tone's frequency is `f = 3,579,545 / (32·N)` for a 10-bit divider `N`, giving about five octaves; inverting it builds an **equal-tempered note table** (one octave of twelve dividers, shifted for other octaves, all within a quarter-percent of true), verified by a C-major chord reading back 261.4 / 330.0 / 392.5 Hz. **Attenuation** is four bits, 0 loud to 15 silent in 2 dB steps, and shaping it over frames with an **envelope** turns a flat beep into a pluck, swell, or decay. The **noise** channel is a 15-bit LFSR with white/periodic modes and three rates (plus a tone-2-tracked mode), the source of drums, engines, and explosions. Effects — pitch slides, vibrato, the chord-faking **arpeggio**, zaps, alarms — are register sweeps updated per frame, collected into `sfxlib` patches. The console **sound-list format** (duration-tagged register streams) is one representation the whole platform shares, played unattended by the firmware's interrupt **auto-player**; a real **music driver** adds tracker-style patterns and orders and the crucial **channel-stealing** that lets effects punch through a score. The modern pipeline is **tracker → VGM → TI data**, its converter a Chapter 38 asset script. `sndlib` packages the engine — `SSILNC`, `STONE`, `SVOL`, `SNOISE`, and the note table — all bench-verified through the new `sound` oracle, and it is the instrument DODGE's score (across Chapters 17 and 22) is written for.
