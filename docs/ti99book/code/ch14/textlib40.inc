* TEXTLIB40.INC — lib99's 40-column text engine (TEXT mode).          (Ch. 14)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* The productivity-mode sibling of ch13's textlib. Where textlib drives
* Graphics I (32 wide, an 8x8 cell, colour by group of eight), this drives the
* VDP's TEXT mode: 40 columns of 6-pixel cells, ONE foreground/background pair
* for the whole screen (VDP register 7), and NO sprites. A consumer COPYs, in
* order:
*     COPY '../ch11/equates.inc'      ports + colours
*     COPY '../ch12/vdplib.inc'       VWTR/VWA/VSBW/VMBW/...
*     COPY 'textlib40.inc'            this file
* then supplies a START. textlib40 owns one pad word, CUR40 (>8346), the cursor.
*
* THE 6-PIXEL RULE. In text mode the chip shows only the TOP SIX bits of each
* pattern byte (bits 7..2); the low two are discarded. Our font is drawn 5 wide
* (bits 7..3), so every glyph renders identically in Graphics I (8-wide cells)
* and in text (6-wide cells) — one font, both modes. The same name-table-byte-
* IS-ASCII convention as textlib holds, so `screen` reads back exactly what was
* written, font or no font (and now `screen` knows text mode is 40 wide).
*
* ROUTINES (R-16; byte cargo in the HIGH half; R0-R9 scratch, keep-across-call
* on the R10 stack):
*   TX40MD                      set text mode, load the font, set colours, clear
*   TX40CL                      clear the screen (spaces), home the cursor
*   TX40LC  R0=row(0-23), R1=col(0-39)         move the cursor (CUR40=row*40+col)
*   TX40PC  R1(hi)=char         put a char (>0D = carriage return), advance
*   TX40PS  R1=->string         put a null-terminated string
*   TX40HX  R1=value            put the 16-bit value as four hex digits

CUR40  EQU  >8346            textlib40's cursor: linear cell offset 0..959
NAM40  EQU  >0000            text-mode name table base   (matches R2 = >00)
PAT40  EQU  >0800            text-mode pattern table base (matches R4 = >01)
TXCOLR EQU  >F400            R7 value: white (F) on dark blue (4), border = blue

* TX40MD — bring up TEXT mode: registers, font, colours, clear screen. Non-leaf.
TX40MD DECT R10
       MOV  R11,*R10
* --- registers: 16K + display-on + M1(text); name >0000, pattern >0800 ---
       LI   R0,1             register 1 ...
       LI   R1,>D000        ... = >D0: 16K, BL(enable), M1=1 (text), M2=0, IE=0
       BL   @VWTR
       LI   R0,0
       LI   R1,0            R0 = >00: M3=0, no external video
       BL   @VWTR
       LI   R0,2
       LI   R1,0            R2 = >00 -> name table at >0000
       BL   @VWTR
       LI   R0,4
       LI   R1,>0100        R4 = >01 -> pattern table at >0800
       BL   @VWTR
       LI   R0,7
       LI   R1,TXCOLR       R7 -> foreground/background (and the border)
       BL   @VWTR
* --- font: digits into pattern[>30..], letters into pattern[>41..] ---
       LI   R0,>0980        pattern[>30 '0'] = >0800 + >30*8
       LI   R1,F40DIG
       LI   R2,80           ten glyphs (0-9), 8 bytes each
       BL   @VMBW
       LI   R0,>0A08        pattern[>41 'A'] = >0800 + >41*8
       LI   R1,F40LET
       LI   R2,208          twenty-six glyphs (A-Z)
       BL   @VMBW
       BL   @TX40CL         clear + home
       MOV  *R10+,R11
       RT

* TX40CL — fill the 40x24 name table with spaces, home the cursor. Non-leaf.
TX40CL DECT R10
       MOV  R11,*R10
       LI   R0,NAM40
       BL   @VWA            aim the write counter at the name table
       LI   R1,>2000        space (>20 in the high half)
       LI   R2,960          40 * 24 cells
TX40CY MOVB R1,@VDPWD       960 spaces; the VDP auto-increments
       DEC  R2
       JNE  TX40CY
       CLR  @CUR40          cursor home (cell 0)
       MOV  *R10+,R11
       RT

* TX40LC — cursor to (row R0, col R1). CUR40 = row*40 + col. 40 = 32 + 8, so
* row*40 = (row<<5) + (row<<3): two shifts and an add, no multiply. Leaf.
TX40LC MOV  R0,R2           keep a copy of row
       SLA  R0,5            row * 32
       SLA  R2,3            row * 8
       A    R2,R0           row * 40
       A    R1,R0           + col
       MOV  R0,@CUR40
       RT

* TX40PC — put char R1(hi) at the cursor and advance. >0D = carriage return:
* jump to the start of the NEXT row. Row width 40 is not a power of two, so the
* row start is found with a DIV (CUR40 / 40 -> remainder = column). Wraps past
* the last cell (960) to home. Non-leaf. Reads R1(hi); clobbers R0, R2, R3.
TX40PC DECT R10
       MOV  R11,*R10
       CI   R1,>0D00        carriage return?
       JEQ  TX40NL
       MOV  @CUR40,R0       R0 = cell offset (= VRAM addr, name base >0000)
       BL   @VSBW           write the char (R1 hi) to VRAM[R0]
       MOV  @CUR40,R0       reload (VSBW clobbered R0)
       INC  R0             advance one cell
       JMP  TX40WR
TX40NL CLR  R2             newline: R2:R3 = CUR40 as a 32-bit dividend
       MOV  @CUR40,R3
       DIV  @F40W,R2       R2 = row (quotient), R3 = column (remainder)
       MOV  @CUR40,R0
       S    R3,R0          R0 = start of this row (CUR40 - column)
       AI   R0,40          start of the next row
TX40WR CI   R0,960          past the last cell?
       JL   TX40PX
       CLR  R0             yes -> wrap home
TX40PX MOV  R0,@CUR40
       MOV  *R10+,R11
       RT

* TX40PS — put the null-terminated string at R1 until >00. Non-leaf. Keeps the
* string pointer on the stack across each TX40PC (which clobbers R2/R3).
TX40PS DECT R10
       MOV  R11,*R10
       MOV  R1,R4           R4 = the string pointer (scratch, no inner use of R4)
TX40SL MOVB *R4+,R1         next char -> R1 high, step the pointer
       JEQ  TX40SX          null terminator -> done
       DECT R10             PUSH the pointer across TX40PC
       MOV  R4,*R10
       BL   @TX40PC
       MOV  *R10+,R4        POP the pointer
       JMP  TX40SL
TX40SX MOV  *R10+,R11
       RT

* TX40HX — put R1 as four hex digits at the cursor. Non-leaf. Keeps its
* value/counter on the stack across TX40PC (identical shape to ch13's TXHEX).
TX40HX DECT R10
       MOV  R11,*R10
       DECT R10
       MOV  R1,*R10         [sp+0] = the value, shifted left each digit
       DECT R10
       LI   R0,4
       MOV  R0,*R10         [sp+0] = digit counter; value now at [sp+2]
TX40HL MOV  @2(R10),R1      R1 = the running value
       SRL  R1,12           top nibble -> low 4 bits
       AI   R1,>0030        assume '0'..'9'
       CI   R1,>003A
       JL   TX40HD
       AI   R1,7            'A'..'F': >3A+7 = >41
TX40HD SWPB R1              char -> high half for TX40PC
       BL   @TX40PC
       MOV  @2(R10),R1     shift the value left one nibble for next time
       SLA  R1,4
       MOV  R1,@2(R10)
       MOV  *R10,R1        R1 = counter
       DEC  R1
       MOV  R1,*R10
       JNE  TX40HL
       AI   R10,4          drop the two locals
       MOV  *R10+,R11
       RT

F40W   DATA 40             row width, as a DIV divisor

* ===================== the 6-pixel font ========================
* Drawn 5 wide (bits 7..3) so the low-two-bit truncation of text mode changes
* nothing. Digits 0-9 then letters A-Z, ASCII order, 8 bytes (4 words) each.
* The digit and A-F glyphs are ch13's, verified; G-Z are new here.
F40DIG DATA >7088,>98A8,>C888,>7000    '0'
       DATA >2060,>2020,>2020,>7000    '1'
       DATA >7088,>0830,>4080,>F800    '2'
       DATA >F810,>2010,>0888,>7000    '3'
       DATA >1030,>5090,>F810,>1000    '4'
       DATA >F880,>F008,>0888,>7000    '5'
       DATA >3040,>80F0,>8888,>7000    '6'
       DATA >F808,>1020,>4040,>4000    '7'
       DATA >7088,>8870,>8888,>7000    '8'
       DATA >7088,>8878,>0810,>6000    '9'
F40LET DATA >7088,>88F8,>8888,>8800    'A'
       DATA >F088,>88F0,>8888,>F000    'B'
       DATA >7088,>8080,>8088,>7000    'C'
       DATA >F088,>8888,>8888,>F000    'D'
       DATA >F880,>80F0,>8080,>F800    'E'
       DATA >F880,>80F0,>8080,>8000    'F'
       DATA >7088,>80B8,>8888,>7000    'G'
       DATA >8888,>88F8,>8888,>8800    'H'
       DATA >7020,>2020,>2020,>7000    'I'
       DATA >3808,>0808,>8888,>7000    'J'
       DATA >8890,>A0C0,>A090,>8800    'K'
       DATA >8080,>8080,>8080,>F800    'L'
       DATA >88D8,>A888,>8888,>8800    'M'
       DATA >88C8,>A898,>8888,>8800    'N'
       DATA >7088,>8888,>8888,>7000    'O'
       DATA >F088,>88F0,>8080,>8000    'P'
       DATA >7088,>8888,>A890,>6800    'Q'
       DATA >F088,>88F0,>A090,>8800    'R'
       DATA >7880,>8070,>0808,>F000    'S'
       DATA >F820,>2020,>2020,>2000    'T'
       DATA >8888,>8888,>8888,>7000    'U'
       DATA >8888,>8888,>8850,>2000    'V'
       DATA >8888,>A8A8,>A8D8,>5000    'W'
       DATA >8888,>5020,>5088,>8800    'X'
       DATA >8888,>5020,>2020,>2000    'Y'
       DATA >F808,>1020,>4080,>F800    'Z'
