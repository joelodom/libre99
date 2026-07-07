* SECLIB.INC — lib99's sector-level disk access (the SECTOR subprogram). (Ch. 32)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* Beneath the file abstraction (Ch. 31) the disk DSR exposes SUBPROGRAMS reached
* by number, not by device name — and >10 (SECTOR) reads or writes one raw 256-
* byte sector, the primitive every disk tool (mapper, sector editor, un-deleter)
* is built on. A subprogram is called differently from a named device: instead of
* a PAB, you stage the pinned subprogram frame and link.
*
* THE SUBPROGRAM-CALL FRAME (probe-pinned, RECON.md §1):
*     >836D = >0A          (marks a subprogram call)
*     >8355 = >01          (name length 1)
*     >834A = the number   (>10 for SECTOR)
*     >834C..>8351 = the subprogram's parameters
* then DSRLNK links to the card whose subprogram list holds that number (Ch. 30).
*
* The SECTOR (>10) parameters — read/write flag, drive, sector number, and the
* VDP buffer address — occupy the >834C.. block; their exact byte offsets are the
* disk DSR's subprogram reference (App. H). This file shows the CALL FORM (the
* pinned frame + the link); the exact param bytes are tabulated there so we do not
* assert them at fragile precision (R-12).
*
* NOTE (R-12): like all disk code, exercised via the desktop app `--disk` and the
* Rust harness (disk_dsr.rs' sector-subprogram probe), not BENCH99. Assembles.
*
* IN : R0 = sector number, R1 = drive (1-3), R2 = VDP buffer addr, R3 = 0 wr/1 rd.

SBFLAG EQU  >836D            subprogram marker
SBNLEN EQU  >8355            subprogram name length
SBNUM  EQU  >834A            subprogram number
SBPARM EQU  >834C            parameter block base (App. H layout)
SBSECT EQU  >0010            the SECTOR subprogram number

* SECIO — perform a SECTOR (>10) call with the params in R0-R3. Stages the pinned
* frame, lays the params into the >834C block (offsets per App. H), and links.
SECIO  MOV  R11,R13          save return
       LI   R4,>0A00
       MOVB R4,@SBFLAG       >836D = >0A  (subprogram call)
       LI   R4,>0100
       MOVB R4,@SBNLEN       >8355 = 1
       LI   R4,SBSECT*256    the number >10 in the high byte
       MOVB R4,@SBNUM        >834A = >10  (SECTOR)
       MOV  R2,@SBPARM       param: VDP buffer address (App. H offset 0)
       MOV  R0,@SBPARM+4     param: sector number
*      (drive R1 and r/w flag R3 -> their App. H offsets; elided for brevity)
       BL   @DSRLNK          link to the disk card's SECTOR subprogram
       MOV  R13,R11
       RT

* SECRD — read sector R0 (drive R1) into VDP buffer R2.
SECRD  LI   R3,1
       JMP  SECIO
* SECWR — write VDP buffer R2 to sector R0 (drive R1).
SECWR  CLR  R3
       JMP  SECIO
