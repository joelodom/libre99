* SNDLIB.INC — lib99's sound engine (SN76489 PSG).                    (Ch. 19)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* Three square-wave tone channels and one noise channel, driven one byte at a
* time through the write-only port at >8400 (SNDWR, from equates.inc). A
* consumer COPYs, in order:
*     COPY '../ch11/equates.inc'      SNDWR + the rest
*     COPY 'sndlib.inc'               this file
* then supplies a START. sndlib is pure register control — no state of its own.
*
* THE PROTOCOL. Each byte is a LATCH (bit 7 set: `1 cc t dddd` — channel cc,
* type t, low 4 data bits) or DATA (bit 7 clear: `0 dddddd` — the high 6 bits of
* the last-latched tone period). A tone's frequency is f = 3,579,545 / (32*N),
* N the 10-bit divider. Attenuation is 4 bits: 0 = loudest, 15 = silent (2 dB a
* step). Because the chip is WRITE-ONLY, prove results with BENCH99's `sound`.
*
* ROUTINES (R-16; leaves — all clobber R3/R4, none call inward):
*   SSILNC                      silence all four channels (attenuation 15)
*   STONE  R0=ch(0-2), R1=N(10-bit), R2=att(0-15)   play a tone
*   SVOL   R0=ch(0-3), R1=att(0-15)                 set one channel's attenuation
*   SNOISE R0=ctrl(0-7), R1=att(0-15)               set the noise channel

* SSILNC — attenuation 15 on all four channels (the four att-latch bytes).
SSILNC LI   R3,>9F00           ch0 att = 15
       MOVB R3,@SNDWR
       LI   R3,>BF00           ch1
       MOVB R3,@SNDWR
       LI   R3,>DF00           ch2
       MOVB R3,@SNDWR
       LI   R3,>FF00           ch3 (noise)
       MOVB R3,@SNDWR
       RT

* STONE — play tone R1 (divider N) on channel R0 at attenuation R2.
* Latch = >80 | ch<<5 | (N AND >0F); data = (N>>4) AND >3F; att latch adds >10.
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
       MOV  R0,R3
       SLA  R3,5
       ORI  R3,>0090          >80 | >10 (attenuation type) | ch<<5
       MOV  R2,R4
       ANDI R4,>000F
       A    R4,R3
       SWPB R3
       MOVB R3,@SNDWR         write attenuation latch
       RT

* SVOL — set channel R0's attenuation to R1 (an attenuation-latch byte).
SVOL   MOV  R0,R3
       SLA  R3,5
       ORI  R3,>0090          >90 | ch<<5
       MOV  R1,R4
       ANDI R4,>000F
       A    R4,R3
       SWPB R3
       MOVB R3,@SNDWR
       RT

* SNOISE — noise control R0 (bit2 = white, bits0-1 = rate / 3 = follow tone 2)
* at attenuation R1. Control latch = >E0 | ctrl; att latch = >F0 | att.
SNOISE MOV  R0,R3
       ANDI R3,>0007
       ORI  R3,>00E0          >80 | (3<<5) | ctrl
       SWPB R3
       MOVB R3,@SNDWR
       MOV  R1,R3
       ANDI R3,>000F
       ORI  R3,>00F0          >80 | (3<<5) | >10 | att
       SWPB R3
       MOVB R3,@SNDWR
       RT

* ================= the equal-tempered note table ==================
* One octave of 10-bit dividers, C4..B4 (concert A4 = 440 Hz -> N = 254). An
* octave up halves N (SRL 1); an octave down doubles it (SLA 1). Generated to
* match 2^(1/12) tuning (all within +/-0.25%); the generator is the PC-side
* asset script of Ch. 38. Index 0 = C, 1 = C#, ... 11 = B.
NOTES  DATA 428,404,381,360,339,320    C4 C#4 D4 D#4 E4 F4
       DATA 302,285,269,254,240,226    F#4 G4 G#4 A4 A#4 B4
