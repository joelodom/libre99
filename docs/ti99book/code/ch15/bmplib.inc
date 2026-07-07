* BMPLIB.INC — lib99's bitmap (Graphics II) drawing engine.           (Ch. 15)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* Graphics II gives every one of the 256x192 pixels its own bit: a real bitmap,
* the closest the 9918A comes to a framebuffer. A consumer COPYs, in order:
*     COPY '../ch11/equates.inc'      ports + colours
*     COPY '../ch12/vdplib.inc'       VWTR/VWA/VSBW/VSBR/...
*     COPY 'bmplib.inc'               this file
* then supplies a START.
*
* THE LAYOUT (the standard, uncramped bitmap). Pattern (the bit canvas) at
* >0000, colour table at >2000, name table at >1800. Registers R3=>FF and
* R4=>03 open the full 8 KiB span of each (the "notorious mask bits", S15.2).
* The name table is filled 0,1,2,...,255 three times so the 768 cells map to a
* linear canvas; pixel (x,y) then lives in pattern byte
*        (y AND >F8) * 32  +  (x AND >F8)  +  (y AND 7)
* at bit  >80 >> (x AND 7).  Deriving that is the heart of S15.3.
*
* Colour is per 8x1 line: each pattern byte shares one colour byte at the same
* offset in the >2000 table (high nibble = foreground, low = background). This
* engine draws in one global colour (BCOLR) set at clear time — monochrome
* vector graphics; the per-line "attribute clash" is S15.1's subject and the
* colour exercises' playground.
*
* ROUTINES (R-16; all clobber R0):
*   BMODE                       set Graphics II, map the name table, clear
*   BCLS    R2 = colour byte    clear all pixels; set the whole canvas' colour
*   BPSET   R0=x(0-255), R1=y(0-191)         turn one pixel ON
*   BLINE   R0=x0,R1=y0,R2=x1,R3=y1          draw a line (Bresenham)

BPATT  EQU  >0000            pattern (bit canvas) base ; R4 = >03
BCOLT  EQU  >2000            colour table base         ; R3 = >FF
BNAME  EQU  >1800            name table base           ; R2 = >06

* bmplib state (scratchpad, above the caller's stack)
BCOLR  EQU  >8348            the canvas colour byte (fg<<4 | bg), high half
BLX    EQU  >834A            Bresenham: current x
BLY    EQU  >834C                       current y
BLX1   EQU  >834E                       end x
BLY1   EQU  >8350                       end y
BLDX   EQU  >8352                       dx  = abs(x1-x0)
BLDY   EQU  >8354                       dy  = -abs(y1-y0)
BLSX   EQU  >8356                       sx  = +/-1
BLSY   EQU  >8358                       sy  = +/-1
BLERR  EQU  >835A                       err = dx + dy

* BMODE — bring up Graphics II: registers, the linear name map, a cleared canvas.
BMODE  DECT R10
       MOV  R11,*R10
       LI   R0,0
       LI   R1,>0200        R0 = >02: M3=1 (Graphics II / bitmap)
       BL   @VWTR
       LI   R0,1
       LI   R1,>E000        R1 = >E0: 16K, display enable
       BL   @VWTR
       LI   R0,2
       LI   R1,>0600        R2 = >06 -> name table at >1800
       BL   @VWTR
       LI   R0,3
       LI   R1,>FF00        R3 = >FF -> colour table >2000, full 8K span
       BL   @VWTR
       LI   R0,4
       LI   R1,>0300        R4 = >03 -> pattern table >0000, full 8K span
       BL   @VWTR
       LI   R0,7
       LI   R1,>0100        R7 = >01 black backdrop
       BL   @VWTR
* name table: 0,1,...,255, three times (768 bytes) -> a linear canvas
       LI   R0,BNAME
       BL   @VWA
       CLR  R3
BMNL   MOV  R3,R1
       ANDI R1,>00FF        i AND >FF
       SWPB R1
       MOVB R1,@VDPWD
       INC  R3
       CI   R3,768
       JNE  BMNL
* clear to white-on-blue and blank all pixels
       LI   R2,>F400        white (F) on dark blue (4)
       BL   @BCLS
       MOV  *R10+,R11
       RT

* BCLS — clear the canvas: all pattern bits 0, all colour bytes = R2 (fg/bg,
* high half). Sets BCOLR so later BPSETs paint in this colour. Non-leaf.
BCLS   DECT R10
       MOV  R11,*R10
       MOV  R2,@BCOLR        remember the canvas colour
* pattern: 6144 zero bytes
       LI   R0,BPATT
       BL   @VWA
       CLR  R1
       LI   R3,6144
BCLSP  MOVB R1,@VDPWD
       DEC  R3
       JNE  BCLSP
* colour: 6144 copies of the fg/bg byte
       LI   R0,BCOLT
       BL   @VWA
       MOV  R2,R1           the colour byte, high half
       LI   R3,6144
BCLSC  MOVB R1,@VDPWD
       DEC  R3
       JNE  BCLSC
       MOV  *R10+,R11
       RT

* BPSET — turn pixel (R0=x, R1=y) ON. addr = (y&>F8)*32 + (x&>F8) + (y&7);
* the bit is >80 >> (x&7). Read-modify-write ORs the bit in, so pixels in the
* same byte accumulate. Non-leaf. Clobbers R0,R1,R3-R6.
BPSET  DECT R10
       MOV  R11,*R10
       MOV  R1,R3
       ANDI R3,>00F8
       SLA  R3,5            (y & >F8) * 32
       MOV  R0,R4
       ANDI R4,>00F8
       A    R4,R3          + (x & >F8)
       MOV  R1,R4
       ANDI R4,7
       A    R4,R3          + (y & 7)   -> R3 = pattern byte address
       MOV  R0,R5
       ANDI R5,7           x & 7
       MOVB @BMASK(R5),R6   R6 high = the bit mask >80>>(x&7)
       MOV  R3,R0
       BL   @VSBR          R1 high = current pattern byte
       SOCB R6,R1          set the bit
       MOV  R3,R0
       BL   @VSBW          write it back
       MOV  *R10+,R11
       RT

* BLINE — Bresenham line (R0=x0,R1=y0,R2=x1,R3=y1), plotting through BPSET.
* State rides scratchpad (BLX..BLERR) so it survives the BPSET calls. Non-leaf.
BLINE  DECT R10
       MOV  R11,*R10
       MOV  R0,@BLX         current point = (x0,y0)
       MOV  R1,@BLY
       MOV  R2,@BLX1        end point
       MOV  R3,@BLY1
* dx = abs(x1-x0), sx = sign(x1-x0)
       MOV  R2,R4
       S    R0,R4          R4 = x1 - x0
       LI   R5,1
       MOV  R4,R4
       JGT  BLDXS
       JEQ  BLDXS
       NEG  R4
       LI   R5,-1
BLDXS  MOV  R4,@BLDX
       MOV  R5,@BLSX
* dy = -abs(y1-y0), sy = sign(y1-y0)
       MOV  R3,R4
       S    R1,R4          R4 = y1 - y0
       LI   R5,1
       MOV  R4,R4
       JGT  BLDYS
       JEQ  BLDYS
       NEG  R4
       LI   R5,-1
BLDYS  NEG  R4             dy = -abs(y1-y0)
       MOV  R4,@BLDY
       MOV  R5,@BLSY
* err = dx + dy
       MOV  @BLDX,R6
       A    @BLDY,R6
       MOV  R6,@BLERR
BLLOOP MOV  @BLX,R0        plot the current point
       MOV  @BLY,R1
       BL   @BPSET
       MOV  @BLX,R0        reached the end?
       C    R0,@BLX1
       JNE  BLSTEP
       MOV  @BLY,R0
       C    R0,@BLY1
       JEQ  BLDONE
BLSTEP MOV  @BLERR,R6      e2 = 2*err
       SLA  R6,1
       C    R6,@BLDY       if e2 >= dy: err += dy; x += sx
       JLT  BLSKX
       MOV  @BLERR,R7
       A    @BLDY,R7
       MOV  R7,@BLERR
       MOV  @BLX,R7
       A    @BLSX,R7
       MOV  R7,@BLX
BLSKX  C    R6,@BLDX       if e2 <= dx: err += dx; y += sy
       JGT  BLSKY
       MOV  @BLERR,R7
       A    @BLDX,R7
       MOV  R7,@BLERR
       MOV  @BLY,R7
       A    @BLSY,R7
       MOV  R7,@BLY
BLSKY  JMP  BLLOOP
BLDONE MOV  *R10+,R11
       RT

BMASK  BYTE >80,>40,>20,>10,>08,>04,>02,>01   bit for pixel column x&7
       EVEN
