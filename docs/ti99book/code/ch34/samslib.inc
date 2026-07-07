* SAMSLIB.INC — lib99's SAMS paged-memory mapper.                     (Ch. 34)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* SAMS (Super AMS) turns the console's fixed 32K expansion into 1 MB+ of RAM,
* seen 4 KiB at a time through a MAPPER. The CPU's memory map still has its eight
* expansion windows (>2000, >3000, >A000..>F000), but each window now shows a
* 4 KiB PAGE of the big RAM, and which page is chosen by a mapper register — so
* you page a huge world through the small window (Ch. 36's far data).
*
* THE MODEL (from the SAMS/AMS documentation):
*   - CRU base >1E00 bit 0 enables ACCESS to the mapper registers (they appear
*     over the >4000 DSR window while access is on). SBO 0 on, SBZ 0 off.
*   - While access is on, one mapper register per CPU window lives at >4000 +
*     2*window; writing a page number selects the physical 4 KiB page that window
*     shows. Window index: >2000 = 2, >3000 = 3, >A000 = 10, ... >F000 = 15.
*
* NOTE (R-12): the project does NOT emulate SAMS (memory is the fixed 32K; no
* mapper, no CRU >1E00). This code ASSEMBLES with libre99asm and shows the mapper
* FORM from the documentation; it runs on the shelf emulators (MAME, js99er) or
* real SAMS hardware, not this emulator or BENCH99. Exact register width/format
* is the SAMS reference (App. G).

SAMSCR EQU  >1E00            SAMS CRU base (bit 0 = mapper-register access)
SAMSRG EQU  >4000            mapper registers appear here while access is on

* SAMSEN / SAMSDI — turn mapper-register access on / off.
SAMSEN LI   R12,SAMSCR
       SBO  0                enable access to the mapper registers at >4000
       RT
SAMSDI LI   R12,SAMSCR
       SBZ  0                disable — restore the normal >4000 window
       RT

* SAMSMP — map physical page R1 into the CPU window whose index is R0.
* (R0 = window index 2..15; the register is at >4000 + 2*R0.)
SAMSMP MOV  R11,R13          save return
       BL   @SAMSEN          access on
       MOV  R0,R2
       SLA  R2,1             window index * 2 = register offset
       AI   R2,SAMSRG        R2 -> the window's mapper register
       MOV  R1,*R2           write the page number (selects the 4K page)
       BL   @SAMSDI          access off
       MOV  R13,R11
       RT
