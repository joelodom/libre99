* INPLIB.INC — lib99's per-frame input layer (keyboard/joystick, 9901). (Ch. 21)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* Reads the console's input hardware through the TMS9901 CRU (the mechanism
* built and verified in ch10) and turns raw switch states into the pressed /
* held / released events a game loop wants. A consumer COPYs, in order:
*     COPY '../ch11/equates.inc'      (nothing 9901-specific yet, but by habit)
*     COPY 'inplib.inc'               this file
* then supplies a START.
*
* THE MATRIX (ch10). Software drives a 3-bit COLUMN onto 9901 pins P2-P4 (CRU
* base COLSEL, LDCR 3 bits) then reads that column's eight ROWS back (CRU base
* ROWRD, STCR 8). Keys are ACTIVE LOW (0 = down). Columns 6 and 7 are the two
* joystick ports: rows 0-4 = fire, left, right, down, up.
*
* ROUTINES (R-16; leaves):
*   IREAD  R0 = joystick(0/1)  -> R1 = active-HIGH input mask
*                                  (bit0 fire, 1 left, 2 right, 3 down, 4 up)
*   IEDGE  R0 = prev mask, R1 = cur mask
*                              -> R2 = newly pressed, R3 = newly released

COLSEL EQU  >0024            column-select CRU base (bit >12 * 2)  [ch10]
ROWRD  EQU  >0006            row-read CRU base      (bit >03 * 2)  [ch10]
INFIRE EQU  >0001            input mask bits ...
INLEFT EQU  >0002
INRGHT EQU  >0004
INDOWN EQU  >0008
INUP   EQU  >0010

* IREAD — read joystick R0 (0 = joy 1 / column 6, 1 = joy 2 / column 7) and
* return R1 = active-HIGH direction/fire mask. Leaf. Clobbers R1, R6, R12.
IREAD  MOV  R0,R6
       AI   R6,6             column = 6 + joystick number
       SWPB R6               column -> high half (LDCR byte cargo)
       LI   R12,COLSEL
       LDCR R6,3             drive P2,P3,P4 = column, LSB first
       LI   R12,ROWRD
       CLR  R1
       STCR R1,8             rows 0..7 -> R1 high half (active low)
       SWPB R1              right-justify: R1 low = the 8 rows
       INV  R1              active HIGH (1 = pressed)
       ANDI R1,>001F         keep the five direction/fire bits (rows 0-4)
       RT

* IEDGE — edge-detect this frame's input against last frame's.
* R0 = previous mask, R1 = current mask. Returns:
*   R2 = cur AND NOT prev  = newly PRESSED this frame
*   R3 = prev AND NOT cur  = newly RELEASED this frame
* (HELD = cur AND prev, one instruction for the caller.) Leaf.
IEDGE  MOV  R1,R2
       SZC  R0,R2           R2 = cur AND NOT prev  (pressed edges)
       MOV  R0,R3
       SZC  R1,R3           R3 = prev AND NOT cur  (released edges)
       RT
