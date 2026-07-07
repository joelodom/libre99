* SPKLIB.INC — lib99's speech-synthesizer interface.                  (Ch. 20)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* The TMS5200/5220 speech synthesizer lives in the sidecar Speech Synthesizer
* unit, memory-mapped at >9000 (read status/data, SPCHRD) and >9400 (write
* command/data, SPCHWR). A consumer COPYs, in order:
*     COPY '../ch11/equates.inc'      SPCHRD / SPCHWR
*     COPY 'spklib.inc'               this file
* then supplies a START.
*
* R-12 / VERIFICATION NOTE. The project emulator models a console WITHOUT the
* speech sidecar (writes to >9400 are ignored; >9000 reads open bus = >00). So
* the bench-verified fact here is that DETECTION correctly reports "absent":
* SPKDET returns >0000 on the project, exactly as a real console with no speech
* unit attached would resolve. The speak/stream primitives issue the datasheet's
* command bytes (App. F) — correct code that this console cannot voice; to HEAR
* speech, run under MAME or js99er, which emulate the synthesizer. The bytes are
* datasheet-derived, not bench-verified (there is nothing here to verify them
* against), and are flagged as such.
*
* ROUTINES (R-16; leaves unless noted; all clobber R1):
*   SPKWR  R1(hi) = byte        write one command/data byte to >9400
*   SPKRD               -> R1(hi) = byte    read the status/data port >9000
*   SPKDET              -> R0 = >FFFF present / >0000 absent   detect the unit

* SPKWR — write byte R1(hi) to the speech write port.
SPKWR  MOVB R1,@SPCHWR
       RT

* SPKRD — read the speech read/status port into R1(hi).
SPKRD  MOVB @SPCHRD,R1
       RT

* SPKDET — detect the synthesizer. A present unit, commanded to read speech-ROM
* byte 0, returns the ROM signature >AA (App. F); an absent unit reads open bus.
* Returns R0 = >FFFF present / >0000 absent. Non-leaf (calls SPKWR/SPKRD).
* [datasheet-derived command bytes; the >0000 result IS bench-verified.]
SPKDET DECT R10
       MOV  R11,*R10
       LI   R1,>2000            RESET the synthesizer (command >20, App. F)
       BL   @SPKWR
       LI   R1,>4000            LOAD ADDRESS command (>40) ...
       BL   @SPKWR
       CLR  R1                 ... to address 0 (a zero data byte)
       BL   @SPKWR
       LI   R1,>1000            READ BYTE command (>10)
       BL   @SPKWR
       BL   @SPKRD             R1(hi) = the returned byte
       CI   R1,>AA00            the speech-ROM signature?
       JEQ  SPKYES
       CLR  R0                absent (open bus, no signature) — the project's case
       JMP  SPKDX
SPKYES SETO R0                present (>FFFF)
SPKDX  MOV  *R10+,R11
       RT
