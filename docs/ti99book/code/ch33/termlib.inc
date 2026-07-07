* TERMLIB.INC — lib99's serial primitives (the TMS9902 as a CRU device). (Ch. 33)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* The RS232 card's UART, the TMS9902, is a CRU device (Ch. 10): you configure it,
* send a byte, and receive a byte entirely through CRU bits at the card's base
* (>1300 for the first serial port). No memory-mapped registers — LDCR loads the
* transmit byte, STCR reads the received byte, and single CRU bits carry the
* status flags (transmit-buffer-empty, receive-buffer-loaded).
*
* NOTE (R-12): the project does NOT emulate the TMS9902 (no serial card at >1300;
* roms/RS232.Bin is unwired). This code ASSEMBLES with libre99asm and shows the CRU-
* serial FORM from the 9902 datasheet; it is exercised on the shelf emulators
* (Classic99, MAME) or real hardware, not on this project's emulator or BENCH99.
* The exact 9902 CRU bit assignments (control/interval registers, the flag bits)
* are the datasheet's / App. G's; representative bits are used here to show shape.
*
* IN : R12 = the 9902's CRU base (e.g. >1300).

R9902B EQU  >1300            first RS232 serial port's CRU base (representative)
XBRE   EQU  22               transmit buffer empty (status bit; datasheet/App. G)
RBRL   EQU  21               receive buffer loaded  (status bit)

* TINIT — reset and configure the 9902 (baud, data bits) via its control register.
* The control/interval-register programming is the datasheet's; shown as a reset
* plus a control-load placeholder so the shape is clear.
TINIT  SBO  31               issue RESET to the 9902 (bit 31 = reset, per datasheet)
*      ... LDCR the control + interval registers for baud/format (App. G) ...
       RT

* TPUTC — send the byte in R1 (high half). Poll transmit-buffer-empty, then load.
TPUTC  TB   XBRE             transmit buffer empty?
       JEQ  TPUTC            no -> wait
       LDCR R1,8             load the 8-bit byte into the transmit register
       RT

* TGETC — receive a byte into R1 (high half). Poll receive-buffer-loaded, read.
TGETC  TB   RBRL             receive buffer loaded?
       JEQ  TGETC            no -> wait
       STCR R1,8             read the 8-bit byte from the receive register
       RT
