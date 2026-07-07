* DSRLIB.INC — lib99's own DSRLNK: link to a Device Service Routine.  (Ch. 30)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* A DSR is a peripheral's driver, in ROM on the card, paged into >4000-5FFF one
* card at a time by that card's CRU bit. To use a device you must FIND its card,
* PAGE IT IN, CALL its routine, and PAGE IT OUT — the job E/A's DSRLNK does. We
* write our own so it is cart-safe (no E/A vectors assumed) and so every step is
* ours to see.
*
* THE VERIFIED CONTRACT (project RECON, probe-pinned against the real disk DSR):
*   - Cards answer at CRU bases >1000, >1100, ... (steps of >0100). Bit 0
*     (SBO 0 at the base) pages that card's DSR ROM into >4000.
*   - A valid DSR ROM has >AA at >4000; the word at >4008 heads its device-name
*     list. Each node = [link(2)][entry(2)][namelen(1)][name...].
*   - Match the device name to a node; call the entry with R12 = the CRU base and
*     interrupts masked. The DSR SKIP-returns (R11+2) if it HANDLED the call, or
*     plain-returns to keep searching  <-- the skip-return is the pinned detail.
*   - Page the card out (SBZ 0) and restore interrupts.
*
* This routine shows that skeleton and ASSEMBLES with libre99asm. Two honesties
* (R-12): (1) BENCH99 cannot page in a DSR (its >4000 is open bus) — the disk
* DSR is exercised from the desktop app `--disk` and the Rust harness
* (device_io.rs: the >AA scan, the DSK1 link, the ToD QUEST load). (2) A DSRLNK
* that actually drives the disk DSR must also stage the PAB pointer in the GPL
* workspace and pad cells (>8355/>834A/>8356; PAB base = [>8356]-devlen-10) — the
* full parameter contract, specified in RECON and verified there. Here the device
* name is passed in CPU RAM to show the FIND/PAGE/CALL/PAGE-OUT bones cleanly.
*
* IN : R0 = addr of device name (CPU RAM), R1 = its length in bytes.
* OUT: R2 = CRU base that handled it (0 = device not found). Clobbers R3-R9,R12.

DSRROM EQU  >4000            a paged-in DSR ROM appears here
DSRNL  EQU  >4008            word at >4008 = head of the device-name list

DSRLNK MOV  R11,R9           save our caller's return
       LI   R2,>1000         first CRU base to try
DLCARD MOV  R2,R12           CRU base for SBO/SBZ and for the DSR
       SBO  0                page this card's DSR ROM in at >4000
       CB   @DSRROM,@DLAA     does >4000 begin with >AA?
       JNE  DLOFF            no card / not a DSR here
       MOV  @DSRNL,R4        R4 = first device node
DLNODE MOV  R4,R4            end of the name list?
       JEQ  DLOFF           yes -> no match on this card
       MOVB @4(R4),R5        node name length -> R5 high
       SRL  R5,8            -> R5 low
       C    R5,R1           same length as our name?
       JNE  DLLINK          no -> next node
       MOV  R0,R6           our name pointer
       LI   R7,5
       A    R4,R7           node name pointer (R4 + 5)
       MOV  R1,R8           byte count
DLCMP  CB   *R6+,*R7+        bytes equal?
       JNE  DLLINK          mismatch -> next node
       DEC  R8
       JNE  DLCMP
       MOV  @2(R4),R6        matched: R6 = the DSR entry address
       LIMI 0                DSRs run with interrupts masked
       BL   *R6             call the DSR (R12 = CRU base)
       JMP  DLLINK          PLAIN return = not handled -> keep looking
*      SKIP return (handled) lands here:
       SBZ  0               page the card out
       LIMI 2
       B    *R9             R2 = the handling CRU base
DLLINK MOV  *R4,R4           follow the node link
       JMP  DLNODE
DLOFF  SBZ  0               page this card out
       AI   R2,>0100         next CRU base
       CI   R2,>2000         tried >1000..>1F00?
       JNE  DLCARD
       CLR  R2              device not found
       LIMI 2
       B    *R9
DLAA   BYTE >AA
       EVEN
