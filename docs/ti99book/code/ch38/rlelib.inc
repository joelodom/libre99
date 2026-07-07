* RLELIB.INC — lib99's RLE decompressor (PackBits-style).             (Ch. 38)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* Run-length encoding earns its keep at 3 MHz: a tiny, fast decoder (this) and
* a compact stream, decompressed straight into VRAM or RAM at load time. The
* stream is a series of blocks, each led by a control byte C:
*     C = 0        end of stream
*     C >= >80     RUN: write (C & >7F) copies of the NEXT byte
*     C <  >80     LITERAL: copy the next C bytes verbatim
* Decompress from R0 (source) to R1 (dest). Clobbers R4,R5. Leaf.
RLEDEC MOVB *R0+,R4          control byte -> R4 high
       JEQ  RLEEND           0 -> done
       MOV  R4,R5
       ANDI R5,>8000         bit 7 set?  (run vs literal)
       JNE  RLERUN
       SRL  R4,8             literal count -> R4 low
RLELIT MOVB *R0+,*R1+        copy one literal byte
       DEC  R4
       JNE  RLELIT
       JMP  RLEDEC
RLERUN SRL  R4,8
       ANDI R4,>007F         run count = C & >7F
       MOVB *R0+,R5          the byte to repeat -> R5 high
RLERPT MOVB R5,*R1+          write it
       DEC  R4
       JNE  RLERPT
       JMP  RLEDEC
RLEEND RT
