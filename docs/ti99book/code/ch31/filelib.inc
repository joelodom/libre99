* FILELIB.INC — lib99's file I/O: the PAB and the opcode wrappers.   (Ch. 31)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* File I/O on the TI is a CONTRACT written into a block of bytes — the Peripheral
* Access Block (PAB) — that lives in VRAM. You fill in its fields (what to do, to
* which file, through which buffer), then hand it to the device's DSR via DSRLNK
* (Ch. 30). filelib lays out the PAB, names the opcodes, and wraps the common
* operations. Requires (COPY in order):
*     COPY '../ch11/equates.inc'   VSBW/VMBW/... and the pad cells
*     COPY '../ch12/vdplib.inc'    VDP writers (the PAB lives in VRAM)
*     COPY '../ch30/dsrlib.inc'    DSRLNK (find/page/call the device)
*     COPY 'filelib.inc'           this file
*
* NOTE (R-12): the disk DSR is exercised from the desktop app `--disk` and the
* Rust harness (device_io.rs: a real PAB drives Tunnels of Doom's QUEST load,
* GREEN at HEAD), not BENCH99 (no card at >4000). This code ASSEMBLES with
* libre99asm and follows the probe-pinned PAB contract (RECON.md §3).

* ---- PAB field offsets (from the PAB's base in VRAM) -----------------------
POPCOD EQU  0                >0  I/O opcode (below)
PFLAG  EQU  1                >1  file-type/mode flags; error -> bits 5-7 (sticky)
PBUF   EQU  2                >2  VDP data-buffer address (word)
PRECLN EQU  4                >4  logical record length
PCOUNT EQU  5                >5  character count (bytes actually read/to write)
PRECNO EQU  6                >6  record number (word; relative files)
PSCROF EQU  8                >8  screen offset (unused for disk)
PNAMLN EQU  9                >9  device+file name length
PNAME  EQU  10               >A  the name: "DSK1.MYFILE"

* ---- I/O opcodes (PAB+0) ---------------------------------------------------
OPOPEN EQU  >00              open a file
OPCLOS EQU  >01              close it
OPREAD EQU  >02              read one record  -> PAB+5 = count, data in the buffer
OPWRIT EQU  >03              write one record (PAB+5 bytes from the buffer)
OPREST EQU  >04              restore/rewind
OPLOAD EQU  >05              LOAD a PROGRAM image to VRAM (PAB+2), no file open
OPSAVE EQU  >06              SAVE a PROGRAM image from VRAM
OPDELE EQU  >07              delete the file
OPSCRA EQU  >08              scratch a record (disk DSR rejects -> error 6)
OPSTAT EQU  >09              status -> PAB+8 (>08 exists / >80 missing)

* ---- file-type flags (PAB+1) ----------------------------------------------
FVARIA EQU  >10              VARIABLE records (else FIXED)
FINTRN EQU  >08              INTERNAL (else DISPLAY)
FOUTPT EQU  >02              OUTPUT mode
FUPDAT EQU  >00              UPDATE/INPUT mode bits (see App. H matrix)

* FDOP — perform the opcode in R1 on the PAB whose VRAM base is in R0.
* Sets PAB+0, then links to the device. On return the DSR has written PAB+1's
* error bits (5-7) and, for READ, PAB+5 (count). The name must already be in the
* PAB (PNAME) and its length in PNAMLN; DSRLNK stages it (Ch. 30 §30.4).
* IN : R0 = PAB VDP base, R1 = opcode.   OUT: R2 = handling CRU base (0 = none).
FDOP   MOV  R11,R13          save return (DSRLNK/VSBW clobber R11)
       BL   @VSBW            write the opcode byte (R0 = addr, R1 = byte)
       BL   @DSRLNK          find the device and call it (Ch. 30)
       MOV  R13,R11
       RT

* Convenience wrappers — each loads the opcode and falls into FDOP.
FOPEN  LI   R1,OPOPEN
       JMP  FDOP
FREAD  LI   R1,OPREAD
       JMP  FDOP
FWRITE LI   R1,OPWRIT
       JMP  FDOP
FCLOSE LI   R1,OPCLOS
       JMP  FDOP
