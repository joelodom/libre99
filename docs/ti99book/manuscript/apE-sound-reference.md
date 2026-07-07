# Appendix E — Sound Generator Reference

<!-- Appendices · target ≈4 pp · companion to Ch. 19 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The command-byte encodings and the note table are tier-1: taken from `code/ch19/sndlib.inc` (the machine-verified sound engine) and confirmed on BENCH99's `sound` oracle in Ch. 19 (A4 = divider 254 reads back 440.4 Hz; the C-major self-test reads ch0 261.4 / ch1 330.0 / ch2 392.5 Hz). The frequency figures in E.3 are computed from the verified formula f = 3,579,545 / (32·N) and cross-checked against equal temperament (all within the ±0.25 % `sndlib` claims). The sound-list grammar (E.6) is from Ch. 19 §19.6 and the E/A manual it cites; the interrupt auto-player it feeds is installed in Ch. 22. -->

The TI-99/4A's sound comes from an **SN76489**-class programmable sound generator
(PSG): three square-wave tone channels and one noise channel, driven one byte at
a time through a **write-only** port at `>8400`. "Write-only" is the fact that
shapes everything — you cannot read the chip back, so in this book you prove what
it plays with BENCH99's `sound` oracle (Ch. 19). This appendix is the byte-level
card: the command format, the frequency and attenuation tables, the noise
control, and the console sound-list format the whole platform shares. The
narrative, the driver design, and the modern tracker pipeline are Chapter 19.

## E.1 The chip at a glance

| Property | Value |
|---|---|
| Chip | SN76489-class PSG |
| Port | `>8400`, **write-only**, one byte per transfer (use `MOVB`) |
| Channels | 3 tone (0, 1, 2) + 1 noise (3) |
| Tone generator | 10-bit divider `N`; square wave |
| Attenuation | 4 bits per channel, 0 (loudest) … 15 (silent), ≈2 dB/step |
| Reference clock | 3,579,545 Hz (NTSC color clock) |

Because the port is a single byte wide, every setting is sent as a stream of
command bytes, each either a **latch** (which register to write, plus data) or a
**data** continuation of the last-latched tone period.

## E.2 Command byte format

Bit 7 selects the byte's kind. A **latch** byte (bit 7 set) names a channel and a
register type and carries the low bits of the value; a **data** byte (bit 7
clear) carries the high 6 bits of the *last-latched* tone period.

```
Latch:   1 c c t d d d d      cc = channel (00,01,10 tone; 11 noise)
                              t  = type (0 = tone period / noise ctrl, 1 = attenuation)
                              dddd = low 4 data bits
Data:    0 - d d d d d d      the high 6 bits of the last-latched tone period
```

The resulting latch-byte bases (add the low data nibble):

| Channel | Tone-period latch | Attenuation latch |
|---|---|---|
| 0 | `>80` | `>90` |
| 1 | `>A0` | `>B0` |
| 2 | `>C0` | `>D0` |
| 3 (noise) | `>E0` (noise control) | `>F0` |

**Playing a tone** (channel `c`, divider `N`, attenuation `a`) is therefore three
bytes — latch the low nibble of `N`, send the high 6 bits as data, then latch the
attenuation:

```
byte 1:  >80 | (c<<5) | (N & >0F)      tone-period latch (low 4 bits)
byte 2:  (N >> 4) & >3F                 data (high 6 bits)
byte 3:  >90 | (c<<5) | a               attenuation latch
```

This is exactly what `sndlib`'s `STONE` emits (Ch. 19). To change only volume,
send the attenuation latch alone; to silence a channel, set its attenuation to 15.

## E.3 Frequency and the note table

The tone frequency is the reference clock divided by 32 and by the 10-bit divider:

$$f = \frac{3{,}579{,}545}{32 \cdot N}, \qquad N = 1 \dots 1023$$

Inverted, to hit a target frequency: `N = round(3,579,545 / (32·f))`. Anchor
points across the divider's range (computed from the formula; the `sndlib`
self-test confirms A4 on the bench):

| `N` | Frequency | Note |
|---|---|---|
| 1 | ≈111,861 Hz | smallest divider (ultrasonic) |
| 32 | ≈3,496 Hz | top of the practical musical range |
| 254 | 440.4 Hz | A4 — concert A (machine-verified) |
| 1023 | ≈109 Hz | largest divider (lowest tone) |

The equal-tempered octave `lib99` ships (`sndlib`'s `NOTES` table), C4 through B4,
all within ±0.25 % of true equal temperament. Halve `N` (`SRL R,1`) to go up an
octave, double it (`SLA R,1`) to go down:

| Note | `N` | Hz | | Note | `N` | Hz |
|---|---|---|---|---|---|---|
| C4 | 428 | 261.4 | | F#4 | 302 | 370.4 |
| C#4 | 404 | 276.9 | | G4 | 285 | 392.5 |
| D4 | 381 | 293.6 | | G#4 | 269 | 415.8 |
| D#4 | 360 | 310.7 | | A4 | 254 | 440.4 |
| E4 | 339 | 330.0 | | A#4 | 240 | 466.1 |
| F4 | 320 | 349.6 | | B4 | 226 | 495.0 |

## E.4 Attenuation

Volume is *attenuation* — a 4-bit cut from full output, so **larger is quieter**.
Each step is about 2 dB; 15 is silence. The attenuation latch for channel `c` is
`>90 | (c<<5) | a` (the noise channel's is `>F0 | a`).

| `a` | Effect | | `a` | Effect |
|---|---|---|---|---|
| 0 | loudest (0 dB cut) | | 8 | ≈−16 dB |
| 2 | ≈−4 dB (the book's default) | | 12 | ≈−24 dB |
| 4 | ≈−8 dB | | 15 | silent |

## E.5 Noise control

The noise channel (channel 3) takes a **control** latch (`>E0 | ctrl`) and its own
attenuation latch (`>F0 | a`). The 3-bit control byte:

```
1 1 1 0 - f b b        f  (bit 2) = feedback: 0 = periodic ("buzz"), 1 = white noise
                       bb (bits 1-0) = shift rate:
                            00 = clock / 512    (highest pitch)
                            01 = clock / 1024
                            10 = clock / 2048
                            11 = use tone channel 2's frequency
```

Setting `bb = 11` slaves the noise pitch to tone 2's divider — the trick that lets
noise sweep (engines, surf, explosions) by writing channel 2's period. `sndlib`'s
`SNOISE` emits exactly this pair of latches.

## E.6 The console sound-list format

The platform shares **one** representation of sound-over-time — the console
firmware plays it, GPL plays it (Ch. 26), and your assembly can play it. A
**sound list** is a stream of duration-tagged blocks:

```
[ count ] [ command byte × count ] [ duration ]   ← one block
[ count ] [ command byte × count ] [ duration ]   ← next block
...
[ 0 ]                                              ← terminator
```

Each block reads: *write these `count` bytes to the sound chip (latch/data
commands from E.2–E.5), then wait `duration` frames* (sixtieths of a second).
Block follows block; a terminating count/duration ends the list. A whole tune or
a whole sound effect is one such blob — compact, position-independent, storable
in ROM.

**The auto-player.** The console's frame-interrupt handler (Ch. 22) includes a
sound-list player: put a pointer to your list in the agreed scratchpad location,
set the flag, and every frame the ISR advances the list on its own — writing each
block's bytes when its duration expires — while your main code does other work.
Two pokes start a tune and it plays itself. It is the same mechanism GPL uses,
which is why a cartridge's jingle, an assembly game's score, and the firmware's
own sounds are all this one format fed to this one player. Driving it requires the
running interrupt handler, so Chapter 19 builds the format and the register
control and Chapter 22 installs the ISR that plays a list unattended.

*See also:* Chapter 19 (the sound generator, `sndlib`, the driver and channel
stealing), Chapter 22 (the interrupt auto-player), Appendix K (the scratchpad
interface variables the player reads).
