* GROMLIB.INC — lib99's GROM reader.                                  (Ch. 25)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* A GROM is a ROM with an internal 16-bit address counter, read one byte at a
* time through memory-mapped ports (no parallel address bus). A consumer COPYs,
* in order:
*     COPY '../ch11/equates.inc'      GRMRD/GRMRA/GRMWD/GRMWA
*     COPY 'gromlib.inc'              this file
* then supplies a START.
*
* THE PROTOCOL. To read from GROM address A: write A's HIGH byte then its LOW
* byte to GRMWA (>9C02); the second write triggers the chip's automatic PREFETCH
* (it fetches mem[A] into a one-byte buffer and advances the counter to A+1).
* Then each read of GRMRD (>9800) returns the buffered byte and refills from the
* counter — so a whole table streams out with no further address writes. The
* prefetch is why reading the address port back gives A+1, not A.
*
* SAVE/RESTORE DISCIPLINE (S25.6). If the GPL interpreter or the OS is alive,
* they own the GROM counter; a polite reader saves it first (read GRMRA twice —
* high then low — which also resets the write phase) and restores it after
* (write those two bytes back), so the OS never notices you were there.
*
* ROUTINES (R-16; leaves):
*   GRDADR  R0 = GROM address        aim the counter (leaves it at A+1, mem[A] buffered)
*   GRDBYT           -> R1(hi) = byte    read one byte (auto-increments)
*   GRDBLK  R1 = CPU dest, R2 = count    read a block from GROM to CPU RAM

* GRDADR — aim the GROM counter at R0. Writes high byte then low byte. Leaf.
GRDADR MOVB R0,@GRMWA          address HIGH byte first
       SWPB R0
       MOVB R0,@GRMWA          address LOW byte -> completes + prefetches mem[A]
       SWPB R0                 restore R0 = the address
       RT

* GRDBYT — read one byte from GROM into R1's HIGH half. Leaf.
GRDBYT MOVB @GRMRD,R1          buffered byte -> R1 high; counter auto-increments
       RT

* GRDBLK — read R2 bytes from GROM (counter already aimed) into CPU dest R1. Leaf.
GRDBLK MOVB @GRMRD,*R1+        one byte -> CPU dest, step; GROM steps its counter
       DEC  R2
       JNE  GRDBLK
       RT
