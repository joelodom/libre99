* SPRITELIB.INC — lib99's sprite engine.                              (Ch. 16)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* Sprites are the VDP's moving objects — 32 of them, drawn OVER any non-text
* mode's background, positioned to the pixel, no background redraw required. A
* consumer COPYs, in order:
*     COPY '../ch11/equates.inc'      ports + colours
*     COPY '../ch12/vdplib.inc'       VWTR/VWA/VSBW/VMBW/...
*     COPY 'spritelib.inc'            this file
* then supplies a START (after bringing up a background mode).
*
* THE MODEL. A Sprite Attribute List (SAL) of 32 four-byte entries lives at
* SATTR; sprite n's entry is {Y, X, pattern#, early-clock|colour}. Y is the line
* ABOVE the sprite (so the sprite shows at Y+1) — SPUT hides that quirk, taking
* a screen y and writing y-1. Y = >D0 ENDS the active list. Sprite shapes live
* in a pattern table at SPATT (8 bytes = one 8x8 sprite). Size (8x8 vs 16x16)
* and 2x magnification are R1 bits SZ/MG, set by the background bring-up.
*
* ROUTINES (R-16; all clobber R0):
*   SINIT                        point R5/R6, empty the sprite list
*   SPAT   R0=pattern#, R1=->8 bytes         load one 8x8 sprite shape
*   SPUT   R0=n, R1=y, R2=x, R3=pattern#, R4=colour   place sprite n
*   SMOV   R0=n, R1=y, R2=x                   move sprite n (position only)
*   SHIDE  R0=n                               park sprite n off-screen

SATTR  EQU  >0300            sprite attribute list ; R5 = >06
SPATT  EQU  >1000            sprite pattern table  ; R6 = >02

* SINIT — R5 -> SAL at >0300, R6 -> sprite patterns at >1000, empty the list.
SINIT  DECT R10
       MOV  R11,*R10
       LI   R0,5
       LI   R1,>0600        R5 = >06 -> SAL at >0300
       BL   @VWTR
       LI   R0,6
       LI   R1,>0200        R6 = >02 -> sprite patterns at >1000
       BL   @VWTR
       LI   R0,SATTR        sprite 0 Y = >D0 -> the list is empty
       LI   R1,>D000
       BL   @VSBW
       MOV  *R10+,R11
       RT

* SPAT — load the 8 bytes at R1 as sprite shape #R0. Non-leaf.
SPAT   DECT R10
       MOV  R11,*R10
       SLA  R0,3           pattern# * 8
       AI   R0,SPATT       + base -> VRAM address
       LI   R2,8
       BL   @VMBW          R1 = src, R2 = 8
       MOV  *R10+,R11
       RT

* SPUT — place sprite R0=n at screen (R1=y, R2=x), shape R3, colour R4. Aims
* the counter once at the SAL entry and streams four bytes. Non-leaf. Clobbers
* R0-R6 (R2/R3/R4 are consumed).
SPUT   DECT R10
       MOV  R11,*R10
       MOV  R0,R5
       SLA  R5,2           n * 4
       AI   R5,SATTR       SAL entry address
       MOV  R5,R0
       BL   @VWA           aim at {Y,X,pat,colour}
       MOV  R1,R6
       AI   R6,-1          Y = screen y - 1 (Y is the line above)
       SWPB R6
       MOVB R6,@VDPWD      Y
       SWPB R2
       MOVB R2,@VDPWD      X
       SWPB R3
       MOVB R3,@VDPWD      pattern#
       SWPB R4
       MOVB R4,@VDPWD      early-clock(0) | colour
       MOV  *R10+,R11
       RT

* SMOV — move sprite R0=n to (R1=y, R2=x); rewrites only Y and X. Non-leaf.
SMOV   DECT R10
       MOV  R11,*R10
       MOV  R0,R5
       SLA  R5,2
       AI   R5,SATTR
       MOV  R5,R0
       BL   @VWA
       MOV  R1,R6
       AI   R6,-1
       SWPB R6
       MOVB R6,@VDPWD      Y
       SWPB R2
       MOVB R2,@VDPWD      X
       MOV  *R10+,R11
       RT

* SHIDE — park sprite R0=n off the bottom (Y=>C0), without ending the list. Non-leaf.
SHIDE  DECT R10
       MOV  R11,*R10
       MOV  R0,R5
       SLA  R5,2
       AI   R5,SATTR
       MOV  R5,R0
       LI   R1,>C000
       BL   @VSBW
       MOV  *R10+,R11
       RT
