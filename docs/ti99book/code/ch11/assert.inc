* ASSERT.INC — the verdict / panic-dump idiom.                         (Ch. 11)
* Copyright (c) 2026 Joel Odom. All rights reserved.
*
* Section 11.5's failed-check idiom, factored for reuse: on a decision,
* RECORD a code to the pad AND FLASH the border in one call. A program
* asserts a condition like this (no macros exist — this is the pattern):
*
*        <test that should leave EQ set on success>
*        JEQ  OK                 the check held ...
*        LI   R0,PANBAD          ... it failed: a nonzero panic code
*        LI   R1,MYSLOT          the pad word to dump it into
*        BL   @VERDCT            record + flash RED, then park
*        ...
*   OK   ...                     (on success, end with R0 = >0000 -> GREEN)
*
* Depends on equates.inc (GREEN / RED / VDPWA / VR7SET). COPY equates.inc
* FIRST, then this file. Ships no data of its own and dictates no pad
* layout — the CALLER names the word to stamp — so any program can lean on
* it. Verified in Ch. 11 by SYSCHK (green path) and its sabotage drill (red).

* VERDCT — record a verdict/panic code and paint it on the border. Leaf.
*   R0 = the code: >0000 means all-clear (GREEN border), any nonzero value
*        is a panic/fail code (RED border).
*   R1 = address of the pad word to stamp the code into (the "dump").
VERDCT MOV  R0,*R1            dump the code to the caller's pad word
       MOV  R0,R0             all-clear?
       JNE  VRDRED
       LI   R0,GREEN
       JMP  VRDPNT
VRDRED LI   R0,RED
VRDPNT MOVB R0,@VDPWA         the colour byte ...
       LI   R0,VR7SET
       MOVB R0,@VDPWA         ... into VDP register 7 (the backdrop)
       RT                     ; back to caller
