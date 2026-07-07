* VDPLIB.INC — lib99's VDP core, as a COPY-includable module.         (Ch. 12)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* The routines only — no START, no data — so any consumer can pull them in.
* A consumer must COPY 'equates.inc' (for VDPWA/VDPWD/VDPRD) BEFORE this file.
* The self-test lives in vdplib.a99 (which COPYs equates.inc then this). See
* code/lib99/README.md for the module = <name>.inc + <name>.a99 (harness) rule.
*
* ROUTINES (R-16; byte cargo rides a register's HIGH half; all clobber R0):
*   VWTR  R0 = register (0-7),  R1(hi) = value        write a VDP register
*   VWA   R0 = VRAM addr (14-bit)                      aim the counter, WRITE mode
*   VRA   R0 = VRAM addr (14-bit)                      aim the counter, READ mode
*   VSBW  R0 = addr,  R1(hi) = byte                    write one VRAM byte
*   VMBW  R0 = addr,  R1 = CPU src,  R2 = count        write a block to VRAM
*   VSBR  R0 = addr             -> R1(hi) = byte       read one VRAM byte
*   VMBR  R0 = addr,  R1 = CPU dst,  R2 = count        read a block from VRAM

* VWTR — write a VDP write-only register. R0 = register number 0-7, R1 high =
* the value. Two-byte control sequence: value first, then 100_00rrr. Leaf.
VWTR   MOVB R1,@VDPWA          the value (first control byte, R1 high half)
       SWPB R0                 register number -> high half
       ORI  R0,>8000           command byte = >80 | rrr (the register-write flag)
       MOVB R0,@VDPWA          second control byte
       RT                      ; back to caller

* VWA — aim the VRAM address counter at R0 for WRITING. Leaf; clobbers R0.
VWA    SWPB R0                 low address byte -> high half
       MOVB R0,@VDPWA          first control byte: low 8 address bits
       SWPB R0                 high address byte -> high half
       ORI  R0,>4000           second byte: 01aaaaaa (write-setup)
       MOVB R0,@VDPWA
       RT                      ; back to caller

* VRA — aim the VRAM address counter at R0 for READING (prefetches). Leaf.
VRA    SWPB R0
       MOVB R0,@VDPWA          first control byte: low 8 address bits
       SWPB R0
       ANDI R0,>3FFF           force the read-setup flag (top two bits 00)
       MOVB R0,@VDPWA          second control byte: 00aaaaaa
       RT                      ; back to caller

* VSBW — write one byte (R1 high) to VRAM address R0. Non-leaf. Clobbers R0.
VSBW   DECT R10               PUSH R11
       MOV  R11,*R10
       BL   @VWA              aim the write counter at R0
       MOVB R1,@VDPWD         the byte lands in VRAM; counter auto-increments
       MOV  *R10+,R11         POP R11
       RT                     ; back to caller

* VMBW — write R2 bytes from CPU source R1 to VRAM address R0. Non-leaf.
VMBW   DECT R10               PUSH R11
       MOV  R11,*R10
       BL   @VWA              aim the write counter once ...
VMBWL  MOVB *R1+,@VDPWD       ... then stream: one byte, step source; VDP steps dest
       DEC  R2
       JNE  VMBWL
       MOV  *R10+,R11         POP R11
       RT                     ; back to caller

* VSBR — read one byte from VRAM address R0 into R1 high. Non-leaf. Clobbers R0.
VSBR   DECT R10               PUSH R11
       MOV  R11,*R10
       BL   @VRA              aim the read counter (prefetches VRAM[R0])
       MOVB @VDPRD,R1         the prefetched byte -> R1 high half
       MOV  *R10+,R11         POP R11
       RT                     ; back to caller

* VMBR — read R2 bytes from VRAM address R0 into CPU dest R1. Non-leaf.
VMBR   DECT R10               PUSH R11
       MOV  R11,*R10
       BL   @VRA              aim the read counter (prefetches the first byte)
VMBRL  MOVB @VDPRD,*R1+       one byte -> CPU dest, step dest; VDP steps source
       DEC  R2
       JNE  VMBRL
       MOV  *R10+,R11         POP R11
       RT                     ; back to caller
