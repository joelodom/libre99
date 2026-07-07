* TEXTLIB.INC — lib99's text engine: a println for a machine without one. (Ch. 13)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* Graphics I text output built on vdplib. A consumer COPYs, in order:
*     COPY '../ch11/equates.inc'      ports + colors
*     COPY '../ch12/vdplib.inc'       VWTR/VWA/VSBW/VMBW/...
*     COPY 'textlib.inc'              this file
* then supplies a START. textlib owns one pad word, CURPOS (>8342), the cursor.
*
* THE CONVENTION that makes text legible AND verifiable: a screen cell's
* name-table byte IS the character's ASCII code, and the font is loaded so that
* pattern[ascii] is that character's glyph. So `screen` (which prints the name
* table as ASCII) shows exactly what the program wrote, font or no font.
*
* ROUTINES (R-16; byte cargo in the HIGH half; R0-R9 are scratch — a value kept
* across an inner call goes on the R10 stack):
*   TXMODE                      set Graphics I, load the font, colors, clear
*   TXCLS                       clear the screen (spaces), home the cursor
*   TXLOC   R0 = row(0-23), R1 = col(0-31)      move the cursor
*   TXPUTC  R1(hi) = char       put a char (>0D = carriage return), advance
*   TXPUTS  R1 = ->string       put a null-terminated string
*   TXHEX   R1 = value          put the 16-bit value as four hex digits

CURPOS EQU  >8342            textlib's cursor: linear cell offset 0..767
NAME   EQU  >0000            Graphics I name table base (matches R2 below)
PATGEN EQU  >0800            Graphics I pattern table base (matches R4 below)
COLTAB EQU  >0380            Graphics I color table base (matches R3 below)

* TXMODE — bring up Graphics I: registers, font, colors, clear screen. Non-leaf.
TXMODE DECT R10
       MOV  R11,*R10
* --- registers (see the §12.7 worksheet): 16K + display on, Graphics I ---
       LI   R0,1             register 1 ...
       LI   R1,>E000        ... = >E0: 16K, BL(enable), Graphics I (M1=M2=M3=0)
       BL   @VWTR
       LI   R0,0             register 0 ...
       LI   R1,0            ... = >00 (M3=0, no external video)
       BL   @VWTR
       LI   R0,2
       LI   R1,0            R2 = >00 -> name table at >0000
       BL   @VWTR
       LI   R0,3
       LI   R1,>0E00        R3 = >0E -> color table at >0380
       BL   @VWTR
       LI   R0,4
       LI   R1,>0100        R4 = >01 -> pattern table at >0800
       BL   @VWTR
       LI   R0,7
       LI   R1,>0100        R7 = >01 -> backdrop (border) = black
       BL   @VWTR
* --- font: our hex-digit glyphs into pattern[>30..>39] and [>41..>46] ---
       LI   R0,>0980        pattern[>30 '0'] = >0800 + >30*8
       LI   R1,FDIG
       LI   R2,80           ten glyphs (0-9), 8 bytes each
       BL   @VMBW
       LI   R0,>0A08        pattern[>41 'A'] = >0800 + >41*8
       LI   R1,FHEX
       LI   R2,48           six glyphs (A-F)
       BL   @VMBW
* --- color table: all 32 groups white-on-black (fg >F, bg >1) ---
       LI   R0,COLTAB
       BL   @VWA            aim the write counter at the color table
       LI   R1,>F100        the pair >F1 in the high half
       LI   R2,32
TXMCOL MOVB R1,@VDPWD       32 color bytes = >F1
       DEC  R2
       JNE  TXMCOL
* --- clear + home ---
       BL   @TXCLS
       MOV  *R10+,R11
       RT

* TXCLS — fill the name table with spaces, home the cursor. Non-leaf.
TXCLS  DECT R10
       MOV  R11,*R10
       LI   R0,NAME
       BL   @VWA            aim the write counter at the name table
       LI   R1,>2000        space (>20 in the high half)
       LI   R2,768
TXCLSL MOVB R1,@VDPWD       768 spaces; the VDP auto-increments
       DEC  R2
       JNE  TXCLSL
       CLR  @CURPOS         cursor home (cell 0)
       MOV  *R10+,R11
       RT

* TXLOC — cursor to (row R0, col R1). CURPOS = row*32 + col. Leaf.
TXLOC  SLA  R0,5            row * 32
       A    R1,R0           + col
       MOV  R0,@CURPOS
       RT

* TXPUTC — put char R1(hi) at the cursor and advance. >0D = carriage return
* (jump to the start of the next row). Wraps past the last cell to home.
* Non-leaf. Reads R1(hi); clobbers R0.
TXPUTC DECT R10
       MOV  R11,*R10
       CI   R1,>0D00        carriage return?
       JEQ  TXPNL
       MOV  @CURPOS,R0      R0 = cell offset (= VRAM addr, name base >0000)
       BL   @VSBW           write the char (R1 hi) to VRAM[R0]
       MOV  @CURPOS,R0      reload (VSBW clobbered R0)
       INC  R0             advance one cell
       JMP  TXPWRP
TXPNL  MOV  @CURPOS,R0      newline: round down to this row, add a row
       ANDI R0,>FFE0        CURPOS & ~31  (start of the current row)
       AI   R0,32           next row
TXPWRP CI   R0,768          past the last cell?
       JL   TXPCX
       CLR  R0             yes -> wrap home
TXPCX  MOV  R0,@CURPOS
       MOV  *R10+,R11
       RT

* TXPUTS — put the null-terminated string at R1 until >00. Non-leaf.
* Keeps the string pointer on the stack across each TXPUTC (R-16 correct).
TXPUTS DECT R10
       MOV  R11,*R10
       MOV  R1,R2           R2 = the string pointer
TXPSL  MOVB *R2+,R1         next char -> R1 high, step the pointer
       JEQ  TXPSX           null terminator -> done
       DECT R10             PUSH the pointer across TXPUTC (it may clobber R2)
       MOV  R2,*R10
       BL   @TXPUTC
       MOV  *R10+,R2        POP the pointer
       JMP  TXPSL
TXPSX  MOV  *R10+,R11
       RT

* TXHEX — put R1 as four hex digits at the cursor. Non-leaf. Keeps its
* value/counter on the stack across TXPUTC.
TXHEX  DECT R10
       MOV  R11,*R10
       DECT R10
       MOV  R1,*R10         [sp+0] = the value, shifted left each digit
       DECT R10
       LI   R0,4
       MOV  R0,*R10         [sp+0] = digit counter; value now at [sp+2]
TXHXL  MOV  @2(R10),R1      R1 = the running value
       SRL  R1,12           top nibble -> low 4 bits
       AI   R1,>0030        assume '0'..'9'
       CI   R1,>003A
       JL   TXHXD
       AI   R1,7            'A'..'F': >3A+7 = >41
TXHXD  SWPB R1              char -> high half for TXPUTC
       BL   @TXPUTC
       MOV  @2(R10),R1     shift the value left one nibble for next time
       SLA  R1,4
       MOV  R1,@2(R10)
       MOV  *R10,R1        R1 = counter
       DEC  R1
       MOV  R1,*R10
       JNE  TXHXL
       AI   R10,4          drop the two locals
       MOV  *R10+,R11
       RT

* ===================== the font (hex digits) ===================
* Our own 5×7 glyphs. A full ASCII font ships identically (Ch. 25 shows
* borrowing the console's from GROM). Space (>20) needs no glyph — cleared
* VRAM already draws it blank.
FDIG   DATA >7088,>98A8,>C888,>7000    '0'
       DATA >2060,>2020,>2020,>7000    '1'
       DATA >7088,>0830,>4080,>F800    '2'
       DATA >F810,>2010,>0888,>7000    '3'
       DATA >1030,>5090,>F810,>1000    '4'
       DATA >F880,>F008,>0888,>7000    '5'
       DATA >3040,>80F0,>8888,>7000    '6'
       DATA >F808,>1020,>4040,>4000    '7'
       DATA >7088,>8870,>8888,>7000    '8'
       DATA >7088,>8878,>0810,>6000    '9'
FHEX   DATA >7088,>88F8,>8888,>8800    'A'
       DATA >F088,>88F0,>8888,>F000    'B'
       DATA >7088,>8080,>8088,>7000    'C'
       DATA >F088,>8888,>8888,>F000    'D'
       DATA >F880,>80F0,>8080,>F800    'E'
       DATA >F880,>80F0,>8080,>8000    'F'
