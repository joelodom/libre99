* MCOLIB.INC — lib99's Multicolor-mode canvas (the PLOT64 API).        (Ch. 14)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* Multicolor mode turns the 256x192 screen into a 64x48 grid of fat 4x4 pixels,
* each independently one of 15 colours (or transparent). There is no font and no
* pattern of bits to design — you just plot coloured blocks. A consumer COPYs,
* in order:
*     COPY '../ch11/equates.inc'      ports + colours
*     COPY '../ch12/vdplib.inc'       VWTR/VWA/VSBW/VSBR/...
*     COPY 'mcolib.inc'               this file
* then supplies a START.
*
* THE LAYOUT. Canvas (the VDP calls it the pattern table) at >0000; name table
* at >0800. The name table is filled ONCE so that the 64x48 fat-pixel grid maps
* to a flat 1536-byte canvas: fat pixel (x,y) lives in the byte
*        CANVAS + (y>>3)*256 + (x>>1)*8 + (y&7)
* with x even -> high nibble (the left block), x odd -> low nibble. Deriving
* that mapping from the chip's addressing is the heart of S14.3.
*
* ROUTINES (R-16; all clobber R0):
*   MCMODE                      set Multicolor mode, fill the name table, clear
*   MCCLS   R2 = colour (0-15)  flood the whole canvas with one colour
*   MCPLOT  R0=x(0-63), R1=y(0-47), R2=colour(0-15)   set one fat pixel

CANVAS EQU  >0000            fat-pixel canvas (VDP pattern table); matches R4=>00
MCNAME EQU  >0800            name table; matches R2=>02

* MCMODE — bring up Multicolor mode: registers, the name-table map, clear. Non-leaf.
MCMODE DECT R10
       MOV  R11,*R10
       LI   R0,1
       LI   R1,>C800        R1 = >C8: 16K, BL(enable), M2=1 (multicolor)
       BL   @VWTR
       LI   R0,0
       LI   R1,0            R0 = >00
       BL   @VWTR
       LI   R0,2
       LI   R1,>0200        R2 = >02 -> name table at >0800
       BL   @VWTR
       LI   R0,4
       LI   R1,0            R4 = >00 -> canvas (pattern table) at >0000
       BL   @VWTR
       LI   R0,7
       LI   R1,>0100        R7 = >01 -> black backdrop (shows through colour 0)
       BL   @VWTR
* --- the name-table map: name[i] = ((i>>7)<<5) | (i&31), i = 0..767 ---
       LI   R0,MCNAME
       BL   @VWA
       CLR  R3             R3 = i
MCMNL  MOV  R3,R6
       SRL  R6,7           i >> 7   (which group of four character rows)
       SLA  R6,5           << 5     (* 32 name entries per group)
       MOV  R3,R7
       ANDI R7,31          i & 31   (the column)
       A    R7,R6          the name byte, 0..191
       SWPB R6             into the high half for MOVB
       MOVB R6,@VDPWD
       INC  R3
       CI   R3,768
       JNE  MCMNL
* --- start with a clear (transparent) canvas ---
       CLR  R2
       BL   @MCCLS
       MOV  *R10+,R11
       RT

* MCCLS — flood the 1536-byte canvas with colour R2 (both nibbles). Non-leaf.
MCCLS  DECT R10
       MOV  R11,*R10
       MOV  R2,R1          build the fill byte: (colour<<4) | colour
       SLA  R1,4
       SOC  R2,R1
       SWPB R1             fill byte -> high half
       LI   R0,CANVAS
       BL   @VWA
       LI   R3,1536        48 fat-rows * 32 bytes
MCCLSL MOVB R1,@VDPWD
       DEC  R3
       JNE  MCCLSL
       MOV  *R10+,R11
       RT

* MCPLOT — set fat pixel (R0=x 0-63, R1=y 0-47) to colour R2 (0-15). Reads the
* canvas byte, replaces the one nibble, writes it back. Non-leaf. Clobbers
* R0, R3-R6 (and R1 during the read).
MCPLOT DECT R10
       MOV  R11,*R10
       MOV  R0,R5          R5 = x parity: 0 = left block (high nibble), 1 = right
       ANDI R5,1
       MOV  R2,R6          R6 = colour (survives the VSBR read)
* --- address = (y>>3)*256 + (x>>1)*8 + (y&7) ---
       MOV  R1,R3
       SRL  R3,3
       SLA  R3,8           (y>>3) * 256
       MOV  R0,R4
       SRL  R4,1
       SLA  R4,3           (x>>1) * 8
       A    R4,R3
       MOV  R1,R4
       ANDI R4,7           y & 7
       A    R4,R3          R3 = canvas byte address (CANVAS = >0000)
* --- read, splice one nibble, write back ---
       MOV  R3,R0
       BL   @VSBR          R1 high = the current byte
       SWPB R1             bring the byte down to R1's low half to mask it
       CI   R5,0
       JNE  MCPLO1
       ANDI R1,>000F       left block: keep the low nibble ...
       SLA  R6,4           ... put the colour in the high nibble
       SOC  R6,R1
       JMP  MCPLO2
MCPLO1 ANDI R1,>00F0       right block: keep the high nibble, colour -> low
       SOC  R6,R1
MCPLO2 SWPB R1             merged byte back to the high half
       MOV  R3,R0
       BL   @VSBW          write it to VRAM[R3]
       MOV  *R10+,R11
       RT
