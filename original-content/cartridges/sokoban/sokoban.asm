* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.

* ============================================================================
* SOKOBAN — the classic warehouse-keeper puzzle for the TI-99/4A, assembled by
* this project's own libre99asm.
* Push every box onto a storage spot. 12 levels from David W. Skinner's
* "Microban" set (used with credit; the set "may be freely distributed
* provided they remain properly credited"). Undo (with hold-to-rewind),
* restart, level skip, move/push counters, flood-filled warehouse floors,
* color tile art, sound, a title screen, help, and a win screen. Graphics I
* mode, 60 Hz polled loop, no interrupts.
* Controls:  E/S/D/X or joystick 1 (host arrow keys) = move / push
*            U or joystick fire = undo (hold to rewind)   R = retry level
*            N / P = next / previous level                Q = quit to title
*            H or AID at the title = help
* Build: cargo run -p libre99-asm -- original-content/cartridges/sokoban/sokoban.asm \
*        -o original-content/cartridges/sokoban/sokoban.ctg
* ============================================================================
        IDT  'SOKOBAN'

VDPWD   EQU  >8C00            ; VDP VRAM write data
VDPWA   EQU  >8C02            ; VDP address/register write (byte writes only)
VDPST   EQU  >8802            ; VDP status read
SOUND   EQU  >8400            ; SN76489
NLEVELS EQU  12               ; number of levels in LEVTAB
BHEIGHT EQU  16               ; board buffer rows (max level height)
STRIDE  EQU  32               ; board row stride: cell = BOARD + (row<<5) + col
UNDOSZ  EQU  2048             ; undo ring entries (power of two)
UNDOMSK EQU  2047             ; ring index mask
REPDEL  EQU  16               ; auto-repeat: frames before the first repeat
REPRAT  EQU  5                ; auto-repeat: frames between repeats

* Tile-state bits (one byte per board cell):
*   bit0 = wall, bit1 = goal, bit2 = box, bit3 = player, bit4 = interior floor
* (interior = reachable from the player's start, marked by a flood fill; it
* only affects how empty cells render: dotted floor inside, black outside).
BOARD   EQU  >A000            ; BHEIGHT rows x stride 32, one state byte each
UNDOB   EQU  >B000            ; undo ring: 1 byte per move (dir 0-3, bit2=push)
* (the flood-fill BFS queue reuses UNDOB scratch during level load, before
* any undo entries exist)

LVLNUM  EQU  >8320            ; current level, 0-based
PPOS    EQU  >8322            ; player's board offset: (row<<5) | col
MOVES   EQU  >8324            ; moves this level (undo subtracts)
PUSHES  EQU  >8326            ; pushes this level
BOXLEFT EQU  >8328            ; boxes not yet on a goal; 0 = level solved
UHEAD   EQU  >832A            ; undo ring head (next write slot)
UCNT    EQU  >832C            ; undo entries available (saturates at UNDOSZ)
TICK    EQU  >832E            ; frame counter (game time)
SNDTMR  EQU  >8330            ; frames until channel 0 + noise fall silent
FANCNT  EQU  >8332            ; fanfare frames remaining on channel 1
PREVK   EQU  >8334            ; previous frame's key mask
NBASE   EQU  >8336            ; name-table addr of board cell (0,0), centered
SCBUF   EQU  >8338            ; 5-byte decimal scratch (>8338..>833C)
LVLW    EQU  >833E            ; current level width (cells)
LVLH    EQU  >8340            ; current level height (cells)
RET2    EQU  >8342            ; nested-call return save (TRYMOV/DOUNDO)
REPTIM  EQU  >8344            ; movement auto-repeat countdown
UREP    EQU  >8346            ; undo auto-repeat countdown
TOTMOV  EQU  >8348            ; total moves across completed levels
TOTPSH  EQU  >834A            ; total pushes across completed levels
WINFLG  EQU  >834C            ; set by TRYMOV when the last box is stored

* ======================= one-time hardware setup ============================
START   LIMI 0
        LWPI >8300
* program VDP registers
        LI   R1,REGTAB
        LI   R2,16
RGL     MOVB *R1+,@VDPWA
        DEC  R2
        JNE  RGL
* clear pattern table >0800..>0FFF
        LI   R1,>0800
        BL   @SETWR
        LI   R2,2048
        CLR  R0
PCL     MOVB R0,@VDPWD
        DEC  R2
        JNE  PCL
* terminate the (unused) sprite list: sprites are always live in Graphics I,
* so park the attribute table in empty VRAM and end it immediately, or the
* name table's bytes would be misread as phantom sprites
        LI   R1,>0780
        BL   @SETWR
        LI   R0,>D000
        MOVB R0,@VDPWD
* load every glyph: tiles, the title block, and the text font, all from one
* (char, 8 pattern bytes) record table terminated by char 0
        LI   R4,GLYPHS
GLP     MOVB *R4+,R0          ; char code (high byte); 0 ends the table
        JEQ  GLPD
        SRL  R0,8
        SLA  R0,3             ; code*8
        AI   R0,>0800         ; pattern address
        MOV  R0,R1
        BL   @SETWR
        LI   R2,8
GLPB    MOVB *R4+,@VDPWD
        DEC  R2
        JNE  GLPB
        JMP  GLP
GLPD
* color: groups 0..15 white-on-black (text), 16..23 tile colors, 24..31 white
        LI   R1,>0300
        BL   @SETWR
        LI   R2,16
        LI   R0,>F100
CIL     MOVB R0,@VDPWD
        DEC  R2
        JNE  CIL
        LI   R2,COLORS
        LI   R3,8
CCL     MOVB *R2+,@VDPWD
        DEC  R3
        JNE  CCL
        LI   R2,8
        LI   R0,>F100
CJL     MOVB R0,@VDPWD
        DEC  R2
        JNE  CJL

* ============================= title screen =================================
TITLE   BL   @CLRNT           ; clear the name table
* big "SOKOBAN" in title blocks (3x5 cells per letter), rows 3..7, cols 2..28
        LI   R4,BIGLET
        CLR  R5               ; letter 0..6
BTL     CLR  R6               ; row 0..4
BTR     MOVB *R4+,R7          ; row bits (low 3)
        SRL  R7,8
        LI   R10,>0004        ; mask, left column first
        CLR  R8               ; col 0..2
BTC     COC  R10,R7           ; is this cell filled?
        JNE  BTCN
        MOV  R6,R1            ; name = (3+row)*32 + 2 + letter*4 + col
        AI   R1,3
        SLA  R1,5
        AI   R1,2
        MOV  R5,R9
        SLA  R9,2
        A    R9,R1
        A    R8,R1
        BL   @SETWR
        LI   R0,>B800         ; solid title-block glyph
        MOVB R0,@VDPWD
BTCN    SRL  R10,1
        INC  R8
        CI   R8,3
        JNE  BTC
        INC  R6
        CI   R6,5
        JNE  BTR
        INC  R5
        CI   R5,7
        JNE  BTL
* sub-title, credit, and prompts
        LI   R4,TTLTAB
        BL   @TABWR
* a little tile vignette under the prompts: you, a box, a spot, a stored box
        LI   R4,VIGTAB
        BL   @GLYWR
* wait: all keys up, then a key down, then up again — so the starting
* keystroke is fully consumed and never leaks into gameplay. H or AID (FCTN+7)
* opens help instead; bare modifiers are ignored so FCTN alone can't start.
TWU     BL   @ANYKEY
        CI   R0,0
        JNE  TWU
TWD     BL   @WAITVB
        BL   @HELPK           ; H or AID -> show the help screen
        CI   R0,0
        JNE  SHOWHLP
        BL   @ANYKY2          ; any non-modifier key -> start the game
        CI   R0,0
        JEQ  TWD
TWR     BL   @WAITVB
        BL   @ANYKEY
        CI   R0,0
        JNE  TWR
        B    @NEWGAME
* H or AID at the title -> draw the help screen, then return to the title
SHOWHLP BL   @ANYKEY          ; wait for the help key to release
        CI   R0,0
        JNE  SHOWHLP
        BL   @CLRNT
        LI   R4,HLPTAB        ; help text
        BL   @TABWR
        LI   R4,HLGTAB        ; legend tiles and section bullets
        BL   @GLYWR
SHWD    BL   @WAITVB          ; any key returns to the title
        BL   @ANYKEY
        CI   R0,0
        JEQ  SHWD
SHWR    BL   @WAITVB
        BL   @ANYKEY
        CI   R0,0
        JNE  SHWR
        B    @TITLE

* ============================ start a new game ==============================
NEWGAME CLR  @LVLNUM
        CLR  @TOTMOV
        CLR  @TOTPSH
        CLR  @TICK

* ==================== load and present the current level ====================
LOADLV  BL   @CLRNT
        LI   R4,HUDTAB        ; static HUD labels and the key-hint line
        BL   @TABWR
        MOV  @LVLNUM,R5       ; "LEVEL nn" (1-based display)
        INC  R5
        LI   R1,>0018
        BL   @DNUM2
        LI   R5,NLEVELS       ; "OF nn" drawn from the real level count
        LI   R1,>001E
        BL   @DNUM2
        BL   @PARSE           ; ROM text -> board states, PPOS, BOXLEFT
        BL   @FLOOD           ; mark interior floor cells from the player
        BL   @DRAWBRD
        CLR  @MOVES
        CLR  @PUSHES
        MOV  @MOVES,R5        ; both counters read 00000
        LI   R1,>0026
        BL   @DNUM5
        MOV  @PUSHES,R5
        LI   R1,>0038
        BL   @DNUM5
        CLR  @UHEAD
        CLR  @UCNT
        CLR  @WINFLG
        CLR  @SNDTMR
        CLR  @FANCNT
        LI   R0,REPDEL
        MOV  R0,@REPTIM
        MOV  R0,@UREP
        BL   @READK           ; a key still held (N/P/R) must not re-fire
        MOV  R1,@PREVK

* ================================ main loop =================================
MAIN    BL   @WAITVB
        INC  @TICK
* sound timer countdown -> silence channel 0 and the noise channel
        MOV  @SNDTMR,R0
        JEQ  MSND
        DEC  R0
        MOV  R0,@SNDTMR
        JNE  MSND
        LI   R0,>9F00
        MOVB R0,@SOUND
        LI   R0,>FF00
        MOVB R0,@SOUND
MSND    BL   @FANSVC          ; fanfare / ding service on channel 1
* read keys; R13 = newly pressed, R15 = held (TRYMOV/DOUNDO preserve both)
        BL   @READK
        MOV  @PREVK,R5
        MOV  R1,@PREVK
        MOV  R1,R15
        MOV  R1,R13
        SZC  R5,R13           ; newpress = current AND NOT previous
* Q = quit to the title
        MOV  R13,R0
        ANDI R0,>0100
        JEQ  MNQ
        B    @TITLE
MNQ
* R = retry this level
        MOV  R13,R0
        ANDI R0,>0020
        JEQ  MNR
        B    @LOADLV
MNR
* N = next level (wraps)
        MOV  R13,R0
        ANDI R0,>0040
        JEQ  MNN
        MOV  @LVLNUM,R0
        INC  R0
        CI   R0,NLEVELS
        JNE  MNN2
        CLR  R0
MNN2    MOV  R0,@LVLNUM
        B    @LOADLV
MNN
* P = previous level (wraps)
        MOV  R13,R0
        ANDI R0,>0080
        JEQ  MNP
        MOV  @LVLNUM,R0
        DEC  R0
        JLT  MNP2
        JMP  MNP3
MNP2    LI   R0,NLEVELS-1
MNP3    MOV  R0,@LVLNUM
        B    @LOADLV
MNP
* undo: U or joystick fire; newpress acts at once, holding rewinds
        MOV  R13,R0
        ANDI R0,>0010
        JEQ  MUH
        BL   @DOUNDO
        LI   R0,REPDEL
        MOV  R0,@UREP
        JMP  MUDN
MUH     MOV  R15,R0
        ANDI R0,>0010
        JEQ  MURS
        DEC  @UREP
        JNE  MDIR
        BL   @DOUNDO
        LI   R0,REPRAT
        MOV  R0,@UREP
        JMP  MUDN
MURS    LI   R0,REPDEL
        MOV  R0,@UREP
        JMP  MDIR
MUDN    MOV  @MOVES,R5        ; refresh the counters after an undo
        LI   R1,>0026
        BL   @DNUM5
        MOV  @PUSHES,R5
        LI   R1,>0038
        BL   @DNUM5
* movement: newpress steps at once, holding walks (auto-repeat)
MDIR    MOV  R13,R0
        ANDI R0,>000F
        JEQ  MHELD
        BL   @TRYMSK
        LI   R0,REPDEL
        MOV  R0,@REPTIM
        JMP  MMOVD
MHELD   MOV  R15,R0
        ANDI R0,>000F
        JEQ  MDRS
        DEC  @REPTIM
        JNE  MLOOP
        MOV  R15,R0
        ANDI R0,>000F
        BL   @TRYMSK
        LI   R0,REPRAT
        MOV  R0,@REPTIM
        JMP  MMOVD
MDRS    LI   R0,REPDEL
        MOV  R0,@REPTIM
        JMP  MLOOP
MMOVD   MOV  @MOVES,R5        ; refresh the counters after a move
        LI   R1,>0026
        BL   @DNUM5
        MOV  @PUSHES,R5
        LI   R1,>0038
        BL   @DNUM5
        MOV  @WINFLG,R0       ; did that push store the last box?
        JEQ  MLOOP
        B    @LEVDONE
MLOOP   B    @MAIN

* ========================== level complete / win ============================
* Every box is on a spot: silence the move beep, flash the message, play the
* rising fanfare (top half of FANTAB), bank the stats, and move on.
LEVDONE LI   R0,>9F00
        MOVB R0,@SOUND
        LI   R0,>FF00
        MOVB R0,@SOUND
        LI   R1,>02A8         ; "LEVEL COMPLETE!" centered on row 21
        BL   @SETWR
        LI   R2,TDONE
        LI   R3,15
        BL   @VMBW
        LI   R0,16            ; 4-note ascending jingle
        MOV  R0,@FANCNT
        MOV  @MOVES,R0        ; bank this level's stats into the totals
        A    R0,@TOTMOV
        MOV  @PUSHES,R0
        A    R0,@TOTPSH
        LI   R9,100           ; let the jingle and message land (~1.7 s)
LDW     BL   @WAITVB
        BL   @FANSVC
        DEC  R9
        JNE  LDW
        MOV  @LVLNUM,R0
        INC  R0
        MOV  R0,@LVLNUM
        CI   R0,NLEVELS
        JEQ  WINSCR
        B    @LOADLV

* All levels solved: a stats panel over the final board, the full fanfare run,
* then any key returns to the title.
WINSCR  LI   R1,>00E6         ; panel top edge (row 7, cols 6..25)
        BL   @SETWR
        LI   R2,20
        LI   R0,>8000
WBT     MOVB R0,@VDPWD
        DEC  R2
        JNE  WBT
        LI   R1,>0226         ; bottom edge (row 17)
        BL   @SETWR
        LI   R2,20
        LI   R0,>8000
WBB     MOVB R0,@VDPWD
        DEC  R2
        JNE  WBB
        LI   R4,9             ; interior rows 8..16: wall, 18 spaces, wall
        LI   R5,>0106
WBR     MOV  R5,R1
        BL   @SETWR
        LI   R0,>8000
        MOVB R0,@VDPWD
        LI   R2,18
        LI   R0,>2000
WBI     MOVB R0,@VDPWD
        DEC  R2
        JNE  WBI
        LI   R0,>8000
        MOVB R0,@VDPWD
        AI   R5,32
        DEC  R4
        JNE  WBR
        LI   R4,WINTAB        ; heading and stat labels
        BL   @TABWR
        LI   R5,NLEVELS       ; LEVELS
        LI   R1,>0171
        BL   @DNUM2
        MOV  @TOTMOV,R5       ; MOVES
        LI   R1,>0191
        BL   @DNUM5
        MOV  @TOTPSH,R5       ; PUSHES
        LI   R1,>01B1
        BL   @DNUM5
        CLR  R2               ; TIME in seconds = TICK/60
        MOV  @TICK,R3
        LI   R0,60
        DIV  R0,R2
        MOV  R2,R5
        LI   R1,>01D1
        BL   @DNUM5
        LI   R0,32            ; the full eight-note fanfare run
        MOV  R0,@FANCNT
* wait: all up, key down, up again; then back to the title
WWU     BL   @WAITVB
        BL   @FANSVC
        BL   @ANYKEY
        CI   R0,0
        JNE  WWU
WWD     BL   @WAITVB
        BL   @FANSVC
        BL   @ANYKEY
        CI   R0,0
        JEQ  WWD
WWR     BL   @WAITVB
        BL   @ANYKEY
        CI   R0,0
        JNE  WWR
        B    @TITLE

* ============================== subroutines =================================
* WAITVB: block until the next vertical blank (60 Hz). clobbers R2.
WAITVB  MOVB @VDPST,R2
        ANDI R2,>8000
        JEQ  WAITVB
        RT

* SETWR: set the VRAM write address from R1. clobbers R1.
SETWR   SWPB R1
        MOVB R1,@VDPWA
        SWPB R1
        ORI  R1,>4000
        MOVB R1,@VDPWA
        ANDI R1,>3FFF
        RT

* VMBW: write R3 bytes from @R2++ to the (already-set) VRAM address.
VMBW    MOVB *R2+,@VDPWD
        DEC  R3
        JNE  VMBW
        RT

* CLRNT: clear the 768-byte name table to spaces. uses R14 save (calls SETWR).
CLRNT   MOV  R11,R14
        LI   R1,>0000
        BL   @SETWR
        LI   R2,768
        LI   R0,>2000
CNL     MOVB R0,@VDPWD
        DEC  R2
        JNE  CNL
        B    *R14

* TABWR: draw a table of (name-table addr, string addr, length) records,
* terminated by a -1 address (0 is a real address: the HUD's top-left corner).
* in: R4 = table. uses R14 save.
TABWR   MOV  R11,R14
TWL     MOV  *R4+,R1
        CI   R1,-1
        JEQ  TWD2
        BL   @SETWR
        MOV  *R4+,R2
        MOV  *R4+,R3
        BL   @VMBW
        JMP  TWL
TWD2    B    *R14

* GLYWR: draw a table of (name-table addr, char word) records, terminated by
* a -1 address — single glyphs for legends and accents. in: R4. uses R14 save.
GLYWR   MOV  R11,R14
GWL     MOV  *R4+,R1
        CI   R1,-1
        JEQ  GWD
        BL   @SETWR
        MOV  *R4+,R0
        MOVB R0,@VDPWD
        JMP  GWL
GWD     B    *R14

* ANYKEY -> R0 = 1 if any key/switch is down on columns 0..7, else 0.
* clobbers R0,R1,R2,R12.
ANYKEY  CLR  R1
AKL     MOV  R1,R0
        SWPB R0               ; column in the high byte
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00         ; the 8 row bits
        CI   R2,>FF00         ; all up?
        JNE  AKHIT
        INC  R1
        CI   R1,8
        JNE  AKL
        CLR  R0
        RT
AKHIT   LI   R0,1
        RT

* ANYKY2 -> R0 = 1 if any NON-MODIFIER key is down (FCTN/SHIFT/CTRL ignored),
* so holding FCTN for AID at the title doesn't start a game. clobbers R0-R2,R12.
ANYKY2  CLR  R1
AK2L    MOV  R1,R0
        SWPB R0
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        CI   R1,0             ; column 0 holds the modifier keys
        JNE  AK2C
        ORI  R2,>7000         ; force FCTN/SHIFT/CTRL (rows 4,5,6) to "up"
AK2C    CI   R2,>FF00
        JNE  AK2HIT
        INC  R1
        CI   R1,8
        JNE  AK2L
        CLR  R0
        RT
AK2HIT  LI   R0,1
        RT

* HELPK -> R0 = 1 if the help key (H, or AID = FCTN+7) is down, else 0.
* clobbers R0-R3,R12.
HELPK   LI   R0,>0400         ; column 4 (H is col 4, row 1)
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        MOV  R2,R3
        ANDI R3,>0200         ; row 1 = H
        JNE  HKAID            ; H up -> try AID
        LI   R0,1
        RT
HKAID   LI   R0,>0000         ; column 0 (FCTN is col 0, row 4)
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        MOV  R2,R3
        ANDI R3,>1000         ; row 4 = FCTN
        JNE  HKNO             ; FCTN up -> no AID
        LI   R0,>0300         ; column 3 (7 is col 3, row 3)
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        MOV  R2,R3
        ANDI R3,>0800         ; row 3 = 7
        JNE  HKNO             ; 7 up -> no AID
        LI   R0,1             ; FCTN+7 held = AID
        RT
HKNO    CLR  R0
        RT

* READK -> R1 control mask (keyboard E/S/D/X diamond OR-ed with joystick 1):
*   b0 up  b1 down  b2 left  b3 right  b4 undo (U / joy fire)
*   b5 retry (R)  b6 next (N)  b7 previous (P)  b8 quit (Q)
* clobbers R0,R2,R3,R12.
READK   CLR  R1
        LI   R0,>0100         ; column 1: S (row 5) = left, X (row 7) = down
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>2000
        JNE  RKS
        ORI  R1,>0004
RKS     MOV  R2,R3
        ANDI R3,>8000
        JNE  RKX
        ORI  R1,>0002
RKX     LI   R0,>0200         ; column 2: E (row 6) = up, D (row 5) = right
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>4000
        JNE  RKE
        ORI  R1,>0001
RKE     MOV  R2,R3
        ANDI R3,>2000
        JNE  RKD
        ORI  R1,>0008
RKD     LI   R0,>0300         ; column 3: U (row 2) = undo, R (row 6) = retry
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>0400
        JNE  RKU
        ORI  R1,>0010
RKU     MOV  R2,R3
        ANDI R3,>4000
        JNE  RKR
        ORI  R1,>0020
RKR     LI   R0,>0400         ; column 4: N (row 0) = next level
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>0100
        JNE  RKN
        ORI  R1,>0040
RKN     LI   R0,>0500         ; column 5: P (row 2) = previous, Q (row 6) = quit
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>0400
        JNE  RKP
        ORI  R1,>0080
RKP     MOV  R2,R3
        ANDI R3,>4000
        JNE  RKQ
        ORI  R1,>0100
RKQ     LI   R0,>0600         ; column 6: joystick 1 (the host arrow keys)
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>1000         ; Joy1 up
        JNE  JSD
        ORI  R1,>0001
JSD     MOV  R2,R3
        ANDI R3,>0800         ; Joy1 down
        JNE  JSL
        ORI  R1,>0002
JSL     MOV  R2,R3
        ANDI R3,>0200         ; Joy1 left
        JNE  JSR
        ORI  R1,>0004
JSR     MOV  R2,R3
        ANDI R3,>0400         ; Joy1 right
        JNE  JSF
        ORI  R1,>0008
JSF     MOV  R2,R3
        ANDI R3,>0100         ; Joy1 fire = undo
        JNE  JSE
        ORI  R1,>0010
JSE     RT

* FANSVC: play the fanfare countdown on channel 1. FANCNT frames remain; the
* note index is FANCNT/4 into FANTAB (index 0 = highest note), so a count of
* 16 plays the table's top four notes ascending and 32 plays the whole run
* (and a count of 4 is the single "box stored" ding). leaf; clobbers R0,R1.
FANSVC  MOV  @FANCNT,R0
        JEQ  FSX
        DEC  R0
        MOV  R0,@FANCNT
        JNE  FSN
        LI   R0,>BF00         ; finished -> silence channel 1
        MOVB R0,@SOUND
        RT
FSN     SRL  R0,2             ; note index = FANCNT/4
        SLA  R0,1             ; 2 bytes per note
        AI   R0,FANTAB
        MOV  R0,R1
        MOVB *R1+,@SOUND      ; channel-1 frequency, low nibble
        MOVB *R1,@SOUND       ; channel-1 frequency, high 6 bits
        LI   R0,>B200         ; channel-1 attenuation
        MOVB R0,@SOUND
FSX     RT

* TRYMSK/TRYMOV: attempt a move. in: R0 = direction mask (b0 up, b1 down,
* b2 left, b3 right; the lowest set bit wins). Walks, or pushes one box if
* the cell beyond is free; blocked moves buzz. Updates the board, PPOS,
* MOVES/PUSHES/BOXLEFT, the undo ring, and redraws the changed cells. Sets
* WINFLG when the last box lands on a goal. Saves its return in RET2 (it
* calls R14-using subroutines). preserves R13,R15; clobbers R0-R10,R12,R14.
TRYMSK  MOV  R11,@RET2
        CLR  R5               ; direction index 0..3
        MOV  R0,R1
        ANDI R1,>0001
        JNE  TMGO
        LI   R5,1
        MOV  R0,R1
        ANDI R1,>0002
        JNE  TMGO
        LI   R5,2
        MOV  R0,R1
        ANDI R1,>0004
        JNE  TMGO
        LI   R5,3
TMGO    MOV  R5,R4
        SLA  R4,1
        MOV  @DIRTAB(R4),R4   ; R4 = board/name offset for this direction
        MOV  @PPOS,R9         ; R9 = the player's cell offset
        MOV  R9,R6
        AI   R6,BOARD         ; R6 -> player cell
        MOV  R6,R7
        A    R4,R7            ; R7 -> target cell
        CLR  R0
        MOVB *R7,R0
        SRL  R0,8             ; target state
        MOV  R0,R1
        ANDI R1,>0001         ; wall?
        JNE  TMBLK
        CLR  R10              ; push flag (0 = plain step)
        MOV  R0,R1
        ANDI R1,>0004         ; box?
        JEQ  TMSTEP
        MOV  R7,R8
        A    R4,R8            ; R8 -> cell beyond the box
        CLR  R1
        MOVB *R8,R1
        SRL  R1,8
        ANDI R1,>0005         ; wall or another box -> can't push
        JNE  TMBLK
        LI   R2,>0400         ; move the box: clear bit2 here, set it there
        SZCB R2,*R7
        SOCB R2,*R8
        MOV  R0,R1            ; leaving a goal? one more box is loose
        ANDI R1,>0002
        JEQ  TMB1
        INC  @BOXLEFT
TMB1    CLR  R1
        MOVB *R8,R1
        SRL  R1,8
        ANDI R1,>0002         ; landing on a goal? one fewer box loose
        JEQ  TMB2
        DEC  @BOXLEFT
        LI   R0,4             ; bright "stored" ding on channel 1
        MOV  R0,@FANCNT
TMB2    INC  @PUSHES
        LI   R10,4            ; undo entry: bit2 = this move pushed
        BL   @BEEPPU
        JMP  TMMOVE
TMSTEP  BL   @BEEPST
TMMOVE  LI   R2,>0800         ; move the player: clear bit3 here, set it there
        SZCB R2,*R6
        SOCB R2,*R7
        A    R4,@PPOS
        INC  @MOVES
        MOV  R5,R1            ; record (direction | push flag) for undo
        SOC  R10,R1
        BL   @RECUND
        MOV  R9,R1            ; redraw the vacated cell...
        BL   @DRAWCEL
        MOV  R9,R1            ; ...the player's new cell...
        A    R4,R1
        BL   @DRAWCEL
        MOV  R10,R10          ; ...and, after a push, the box's new cell
        JEQ  TMWIN
        MOV  R9,R1
        A    R4,R1
        A    R4,R1
        BL   @DRAWCEL
TMWIN   MOV  @BOXLEFT,R0      ; all boxes stored -> the level is solved
        JNE  TMX
        SETO @WINFLG
TMX     MOV  @RET2,R11
        RT
TMBLK   BL   @BUZZ            ; bumped a wall or an immovable box
        MOV  @RET2,R11
        RT

* DOUNDO: pop one entry from the undo ring and reverse it — step the player
* back and, if it was a push, pull the box back with them. Underflow is a
* silent no-op. Saves its return in RET2. preserves R13,R15; clobbers
* R0-R9,R12,R14.
DOUNDO  MOV  R11,@RET2
        MOV  @UCNT,R0
        JEQ  UDX              ; nothing recorded -> no-op
        DEC  R0
        MOV  R0,@UCNT
        MOV  @UHEAD,R2
        DEC  R2
        ANDI R2,UNDOMSK
        MOV  R2,@UHEAD
        CLR  R1
        MOVB @UNDOB(R2),R1
        SRL  R1,8             ; entry: b0-1 direction, b2 push flag
        MOV  R1,R5
        ANDI R5,>0003
        MOV  R5,R4
        SLA  R4,1
        MOV  @DIRTAB(R4),R4   ; the offset the move added
        MOV  @PPOS,R9
        MOV  R9,R6
        AI   R6,BOARD         ; R6 -> player's current cell
        MOV  R6,R7
        S    R4,R7            ; R7 -> the cell stepped from
        ANDI R1,>0004         ; was it a push?
        JEQ  UDSTEP
        MOV  R6,R8
        A    R4,R8            ; R8 -> the box the push left ahead
        LI   R2,>0400         ; pull it back onto the player's cell
        SZCB R2,*R8
        CLR  R0               ; leaving a goal frees a box...
        MOVB *R8,R0
        SRL  R0,8
        ANDI R0,>0002
        JEQ  UDB1
        INC  @BOXLEFT
UDB1    CLR  R0               ; ...arriving on one stores it again
        MOVB *R6,R0
        SRL  R0,8
        ANDI R0,>0002
        JEQ  UDB2
        DEC  @BOXLEFT
UDB2    LI   R2,>0400
        SOCB R2,*R6
        DEC  @PUSHES
UDSTEP  LI   R2,>0800         ; step the player back
        SZCB R2,*R6
        SOCB R2,*R7
        S    R4,@PPOS
        DEC  @MOVES
        BL   @BEEPST
        MOV  R9,R1            ; redraw the three cells the undo touched
        BL   @DRAWCEL
        MOV  R9,R1
        S    R4,R1
        BL   @DRAWCEL
        MOV  R9,R1
        A    R4,R1
        BL   @DRAWCEL
UDX     MOV  @RET2,R11
        RT

* RECUND: push the byte in R1 (low) onto the undo ring, overwriting the
* oldest entry once the ring is full. leaf; clobbers R1,R2.
RECUND  SWPB R1
        MOV  @UHEAD,R2
        MOVB R1,@UNDOB(R2)
        INC  R2
        ANDI R2,UNDOMSK
        MOV  R2,@UHEAD
        MOV  @UCNT,R2
        CI   R2,UNDOSZ
        JEQ  RUX
        INC  R2
        MOV  R2,@UCNT
RUX     RT

* DRAWCEL: redraw one board cell. in: R1 = board offset ((row<<5)|col). The
* name-table address is NBASE plus the same offset (both use stride 32).
* uses R14 save; clobbers R0,R1,R2.
DRAWCEL MOV  R11,R14
        CLR  R0
        MOVB @BOARD(R1),R0
        SRL  R0,8
        ANDI R0,>001F
        MOV  R0,R2            ; the state, in an indexable register
        A    @NBASE,R1
        BL   @SETWR
        MOVB @TILECH(R2),R2
        MOVB R2,@VDPWD
        B    *R14

* DRAWBRD: paint every level cell through the tile-state -> glyph table.
* uses R14 save; clobbers R0-R7.
DRAWBRD MOV  R11,R14
        CLR  R4               ; row
DBROW   MOV  R4,R1
        SLA  R1,5
        A    @NBASE,R1
        BL   @SETWR
        MOV  R4,R5
        SLA  R5,5
        AI   R5,BOARD
        MOV  @LVLW,R6
DBCOL   CLR  R0
        MOVB *R5+,R0
        SRL  R0,8
        ANDI R0,>001F
        MOV  R0,R7
        MOVB @TILECH(R7),R7
        MOVB R7,@VDPWD
        DEC  R6
        JNE  DBCOL
        INC  R4
        C    R4,@LVLH
        JNE  DBROW
        B    *R14

* PARSE: expand the current level's XSB text into board state bytes.
* Levels are stored exactly as published (walls #, goals ., boxes $, boxes on
* goals *, player @ or +), each row padded to the level's width. Sets LVLW,
* LVLH, PPOS, BOXLEFT, and NBASE (the level is centered on screen).
* leaf; clobbers R0-R9.
PARSE   MOV  @LVLNUM,R9
        SLA  R9,1
        MOV  @LEVTAB(R9),R9   ; R9 -> level record: width, height, cells
        LI   R1,BOARD         ; clear the whole board buffer first
        LI   R2,BHEIGHT*STRIDE
        CLR  R0
PCLR    MOVB R0,*R1+
        DEC  R2
        JNE  PCLR
        CLR  R0
        MOVB *R9+,R0
        SRL  R0,8
        MOV  R0,@LVLW
        CLR  R0
        MOVB *R9+,R0
        SRL  R0,8
        MOV  R0,@LVLH
        CLR  @BOXLEFT
        CLR  R4               ; row
PROW    MOV  R4,R5
        SLA  R5,5
        AI   R5,BOARD         ; R5 -> this row's first cell
        CLR  R6               ; col
PCOL    CLR  R0
        MOVB *R9+,R0
        SRL  R0,8             ; the XSB character
        CLR  R1               ; default: empty
        CI   R0,'#'
        JNE  PC1
        LI   R1,>0001         ; wall
        JMP  PSTO
PC1     CI   R0,'.'
        JNE  PC2
        LI   R1,>0002         ; goal
        JMP  PSTO
PC2     CI   R0,'$'
        JNE  PC3
        LI   R1,>0004         ; box (off-goal: one more to store)
        INC  @BOXLEFT
        JMP  PSTO
PC3     CI   R0,'*'
        JNE  PC4
        LI   R1,>0006         ; box already on a goal
        JMP  PSTO
PC4     CI   R0,'@'
        JNE  PC5
        LI   R1,>0008         ; player
        JMP  PPLR
PC5     CI   R0,'+'
        JNE  PSTO
        LI   R1,>000A         ; player standing on a goal
PPLR    MOV  R4,R2            ; remember where the player starts
        SLA  R2,5
        A    R6,R2
        MOV  R2,@PPOS
PSTO    SWPB R1
        MOVB R1,*R5+
        INC  R6
        C    R6,@LVLW
        JNE  PCOL
        INC  R4
        C    R4,@LVLH
        JNE  PROW
        LI   R0,32            ; center the level: NBASE = (yoff<<5) + xoff
        S    @LVLW,R0
        SRL  R0,1             ; xoff = (32 - width) / 2
        LI   R1,18            ; board area spans rows 3..20
        S    @LVLH,R1
        SRL  R1,1
        AI   R1,3             ; yoff = 3 + (18 - height) / 2
        SLA  R1,5
        A    R1,R0
        MOV  R0,@NBASE
        RT

* FLOOD: breadth-first fill from the player's start, setting bit4 (interior)
* on every reachable non-wall cell, so the warehouse floor renders dotted
* while cells outside the walls stay black. The queue borrows the undo
* buffer, which is reset right after loading. leaf; clobbers R0-R8.
FLOOD   LI   R7,UNDOB         ; R7 = queue read, R8 = queue write
        LI   R8,UNDOB
        MOV  @PPOS,R0
        MOV  R0,*R8+
        MOV  R0,R1
        LI   R2,>1000
        SOCB R2,@BOARD(R1)    ; mark the start cell
FLLOOP  C    R7,R8
        JEQ  FLDONE
        MOV  *R7+,R0          ; pop a cell
        LI   R6,DIRTAB
        LI   R5,4
FLDIR   MOV  R0,R1
        A    *R6+,R1          ; a neighbor's offset
        CI   R1,BHEIGHT*STRIDE
        JHE  FLNEXT           ; off the buffer (or negative) -> skip
        CLR  R2
        MOVB @BOARD(R1),R2
        SRL  R2,8
        MOV  R2,R3
        ANDI R3,>0011         ; wall, or already marked?
        JNE  FLNEXT
        LI   R2,>1000
        SOCB R2,@BOARD(R1)
        MOV  R1,*R8+          ; enqueue it
FLNEXT  DEC  R5
        JNE  FLDIR
        JMP  FLLOOP
FLDONE  RT

* DNUM5: draw the 16-bit value in R5 as 5 decimal digits at name address R1.
* uses R14 save and DIV. (R1 survives the digit loop, so SETWR still has it.)
DNUM5   MOV  R11,R14
        LI   R4,SCBUF+4       ; fill least-significant digit first
        LI   R6,5
        LI   R0,10
DSL     CLR  R2
        MOV  R5,R3            ; dividend = R2:R3 = 0:value
        DIV  R0,R2            ; -> R2 quotient, R3 remainder (digit)
        AI   R3,>0030         ; '0' + digit
        SWPB R3
        MOVB R3,*R4
        DEC  R4
        MOV  R2,R5            ; value <- quotient
        DEC  R6
        JNE  DSL
        BL   @SETWR           ; R1 still holds the destination
        LI   R2,SCBUF
        LI   R3,5
        BL   @VMBW
        B    *R14

* DNUM2: draw the value in R5 as 2 decimal digits at name address R1, pegged
* at 99. uses R14 save and DIV.
DNUM2   MOV  R11,R14
        CI   R5,99
        JLE  D2OK
        LI   R5,99
D2OK    CLR  R2
        MOV  R5,R3            ; dividend R2:R3 = 0 : value
        LI   R0,10
        DIV  R0,R2            ; R2 = tens, R3 = ones
        BL   @SETWR           ; R1 = destination
        AI   R2,>0030         ; tens digit
        SWPB R2
        MOVB R2,@VDPWD
        AI   R3,>0030         ; ones digit
        SWPB R3
        MOVB R3,@VDPWD
        B    *R14

* BEEPST/BEEPPU: a soft tick for a step, a lower thud for a push, on channel
* 0; SNDTMR silences them (and the buzz) a few frames later. leaves.
BEEPST  LI   R0,>8000         ; period >060 -> a light ~1.2 kHz tick
        MOVB R0,@SOUND
        LI   R0,>0600
        MOVB R0,@SOUND
        LI   R0,>9B00         ; fairly quiet
        MOVB R0,@SOUND
        LI   R0,2
        MOV  R0,@SNDTMR
        RT
BEEPPU  LI   R0,>8000         ; period >120 -> a ~390 Hz shove
        MOVB R0,@SOUND
        LI   R0,>1200
        MOVB R0,@SOUND
        LI   R0,>9500
        MOVB R0,@SOUND
        LI   R0,3
        MOV  R0,@SNDTMR
        RT

* BUZZ: a brief burst of noise when a move is blocked. leaf.
BUZZ    LI   R0,>E600         ; white noise, slowest shift rate
        MOVB R0,@SOUND
        LI   R0,>F500
        MOVB R0,@SOUND
        LI   R0,2
        MOV  R0,@SNDTMR
        RT

* ================================= data =====================================
REGTAB  BYTE >00,>80          ; R0 Graphics I
        BYTE >C0,>81          ; R1 16K + display ON
        BYTE >00,>82          ; R2 name table >0000
        BYTE >0C,>83          ; R3 color table >0300
        BYTE >01,>84          ; R4 pattern table >0800
        BYTE >0F,>85          ; R5 sprite attributes >0780 (terminated, unused)
        BYTE >03,>86          ; R6 sprite patterns >1800 (unused)
        BYTE >11,>87          ; R7 backdrop black

* Tile color groups 16..23 (chars >80..>BF), each foreground/background:
* wall gray/black, floor dark blue, spot light yellow, box dark yellow,
* stored box light green, player white, player-on-spot cyan, title dark yellow
COLORS  BYTE >E1,>41,>B1,>A1,>31,>F1,>71,>A1

* Tile state (bits 0-4) -> screen glyph. Bit4 (interior) only matters for
* empty cells: inside shows the dotted floor, outside stays blank. Impossible
* wall combinations fall back to the wall glyph.
TILECH  BYTE >20,>80,>90,>80,>98,>80,>A0,>80   ; 0-7:  void, wall, spot, box...
        BYTE >A8,>80,>B0,>80,>98,>80,>A0,>80   ; 8-15: player, player-on-spot
        BYTE >88,>80,>90,>80,>98,>80,>A0,>80   ; 16-23: the same, interior
        BYTE >A8,>80,>B0,>80,>98,>80,>A0,>80   ; 24-31

* Direction index (0 up, 1 down, 2 left, 3 right) -> board/name offset.
* Board cells and name-table cells share the stride, so one table serves both.
DIRTAB  DATA -32,32,-1,1

* Fanfare notes, highest first, each (frequency low-latch byte, high byte):
* C6 B5 A5 G5 F5 E5 D5 C5. FANSVC plays FANCNT/4 as the index, so the jingle
* rises: 16 frames = G5 A5 B5 C6, 32 frames = the full C5..C6 run.
FANTAB  BYTE >AB,>06,>A1,>07,>AF,>07,>AF,>08
        BYTE >A0,>0A,>AA,>0A,>AE,>0B,>A6,>0D

* big-title letters "SOKOBAN": 7 letters x 5 rows, low 3 bits = a 3-wide row
BIGLET  BYTE >07,>04,>07,>01,>07   ; S
        BYTE >07,>05,>05,>05,>07   ; O
        BYTE >05,>06,>04,>06,>05   ; K
        BYTE >07,>05,>05,>05,>07   ; O
        BYTE >06,>05,>06,>05,>06   ; B
        BYTE >02,>05,>07,>05,>05   ; A
        BYTE >05,>07,>07,>05,>05   ; N

* ---- text ----
TSUB    TEXT 'THE WAREHOUSE KEEPER'
TCRED1  TEXT 'LEVELS FROM MICROBAN'
TCRED2  TEXT 'BY DAVID W SKINNER'
TPRESS  TEXT 'PRESS ANY KEY TO PLAY'
THELP   TEXT 'H OR AID FOR HELP'
TSOKO   TEXT 'SOKOBAN'
TLEVEL  TEXT 'LEVEL'
TLEVLS  TEXT 'LEVELS'
TOF     TEXT 'OF'
TMOVES  TEXT 'MOVES'
TPUSH   TEXT 'PUSHES'
THINT   TEXT 'ESDX MOVE U UNDO R RETRY N SKIP'
TDONE   TEXT 'LEVEL COMPLETE!'
TWIN    TEXT 'YOU WIN!'
TTIME   TEXT 'TIME'
TANYK   TEXT 'PRESS ANY KEY'
THTITLE TEXT 'SOKOBAN HELP'
THGOAL  TEXT 'GOAL'
THRULE  TEXT 'PUSH EVERY BOX ONTO A SPOT'
THBOX   TEXT 'BOX'
THSPOT  TEXT 'SPOT'
THSTOR  TEXT 'STORED BOX'
THYOU   TEXT 'YOU'
THCTRL  TEXT 'CONTROLS'
THK1    TEXT 'ESDX OR JOY'
THD1    TEXT 'MOVE'
THK2    TEXT 'U OR FIRE'
THD2    TEXT 'UNDO'
THK3    TEXT 'R'
THD3    TEXT 'RETRY LEVEL'
THK4    TEXT 'N OR P'
THD4    TEXT 'SKIP LEVEL'
THK5    TEXT 'Q'
THD5    TEXT 'QUIT TO TITLE'

* title-screen layout: (name-table addr, string, length), terminated by 0
TTLTAB  DATA >0126,TSUB,20
        DATA >0166,TCRED1,20
        DATA >0187,TCRED2,18
        DATA >01C5,TPRESS,21
        DATA >0207,THELP,17
        DATA -1

* title vignette (row 19): you, a box, a spot, a stored box, between walls
VIGTAB  DATA >026A,>8000
        DATA >026B,>8800
        DATA >026C,>A800
        DATA >026D,>8800
        DATA >026E,>9800
        DATA >026F,>8800
        DATA >0270,>9000
        DATA >0271,>8800
        DATA >0272,>A000
        DATA >0273,>8800
        DATA >0274,>8000
        DATA -1

* in-game HUD: title, level readout, counters, and the key-hint line
HUDTAB  DATA >0000,TSOKO,7
        DATA >0012,TLEVEL,5
        DATA >001B,TOF,2
        DATA >0020,TMOVES,5
        DATA >0031,TPUSH,6
        DATA >02E0,THINT,31
        DATA -1

* help-screen text layout
HLPTAB  DATA >004A,THTITLE,12
        DATA >0082,THGOAL,4
        DATA >00C2,THRULE,26
        DATA >0106,THBOX,3
        DATA >0126,THSPOT,4
        DATA >0146,THSTOR,10
        DATA >0166,THYOU,3
        DATA >0202,THCTRL,8
        DATA >0222,THK1,11
        DATA >0233,THD1,4
        DATA >0242,THK2,9
        DATA >0253,THD2,4
        DATA >0262,THK3,1
        DATA >0273,THD3,11
        DATA >0282,THK4,6
        DATA >0293,THD4,10
        DATA >02A2,THK5,1
        DATA >02B3,THD5,13
        DATA >02E9,TANYK,13
        DATA -1

* help-screen glyphs: section bullets and the tile legend
HLGTAB  DATA >0081,>9800
        DATA >0103,>9800
        DATA >0123,>9000
        DATA >0143,>A000
        DATA >0163,>A800
        DATA >0201,>9000
        DATA -1

* win-screen panel text
WINTAB  DATA >012C,TWIN,8
        DATA >0168,TLEVLS,6
        DATA >0188,TMOVES,5
        DATA >01A8,TPUSH,6
        DATA >01C8,TTIME,4
        DATA >0209,TANYK,13
        DATA -1

* ---- glyphs: game tiles, the title block, and the text font ----
* (char, 8 pattern bytes) records, terminated by char 0
GLYPHS  BYTE >80,>77,>77,>77,>00,>DD,>DD,>DD,>00   ; wall: running-bond bricks
        BYTE >88,>00,>00,>20,>00,>00,>00,>02,>00   ; floor: two faint dots
        BYTE >90,>00,>18,>24,>42,>42,>24,>18,>00   ; spot: a diamond outline
        BYTE >98,>FF,>C3,>A5,>99,>99,>A5,>C3,>FF   ; box: an X-braced crate
        BYTE >A0,>FF,>E7,>C3,>81,>81,>C3,>E7,>FF   ; stored box: diamond cutout
        BYTE >A8,>18,>18,>3C,>5A,>3C,>24,>24,>66   ; the keeper
        BYTE >B0,>18,>18,>3C,>5A,>3C,>24,>24,>66   ; the keeper, on a spot
        BYTE >B8,>FF,>FF,>FF,>FF,>FF,>FF,>FF,>FF   ; title block
        BYTE '!',>20,>20,>20,>20,>20,>00,>20,>00
        BYTE '-',>00,>00,>00,>F8,>00,>00,>00,>00
        BYTE '0',>70,>88,>98,>A8,>C8,>88,>70,>00
        BYTE '1',>20,>60,>20,>20,>20,>20,>70,>00
        BYTE '2',>70,>88,>08,>30,>40,>80,>F8,>00
        BYTE '3',>70,>88,>08,>30,>08,>88,>70,>00
        BYTE '4',>10,>30,>50,>90,>F8,>10,>10,>00
        BYTE '5',>F8,>80,>F0,>08,>08,>88,>70,>00
        BYTE '6',>30,>40,>80,>F0,>88,>88,>70,>00
        BYTE '7',>F8,>08,>10,>20,>40,>40,>40,>00
        BYTE '8',>70,>88,>88,>70,>88,>88,>70,>00
        BYTE '9',>70,>88,>88,>78,>08,>10,>60,>00
        BYTE 'A',>70,>88,>88,>F8,>88,>88,>88,>00
        BYTE 'B',>F0,>88,>88,>F0,>88,>88,>F0,>00
        BYTE 'C',>70,>88,>80,>80,>80,>88,>70,>00
        BYTE 'D',>F0,>88,>88,>88,>88,>88,>F0,>00
        BYTE 'E',>F8,>80,>80,>F0,>80,>80,>F8,>00
        BYTE 'F',>F8,>80,>80,>F0,>80,>80,>80,>00
        BYTE 'G',>70,>88,>80,>B8,>88,>88,>70,>00
        BYTE 'H',>88,>88,>88,>F8,>88,>88,>88,>00
        BYTE 'I',>70,>20,>20,>20,>20,>20,>70,>00
        BYTE 'J',>38,>10,>10,>10,>10,>90,>60,>00
        BYTE 'K',>88,>90,>A0,>C0,>A0,>90,>88,>00
        BYTE 'L',>80,>80,>80,>80,>80,>80,>F8,>00
        BYTE 'M',>88,>D8,>A8,>88,>88,>88,>88,>00
        BYTE 'N',>88,>C8,>A8,>98,>88,>88,>88,>00
        BYTE 'O',>70,>88,>88,>88,>88,>88,>70,>00
        BYTE 'P',>F0,>88,>88,>F0,>80,>80,>80,>00
        BYTE 'Q',>70,>88,>88,>88,>A8,>90,>68,>00
        BYTE 'R',>F0,>88,>88,>F0,>A0,>90,>88,>00
        BYTE 'S',>70,>88,>80,>70,>08,>88,>70,>00
        BYTE 'T',>F8,>20,>20,>20,>20,>20,>20,>00
        BYTE 'U',>88,>88,>88,>88,>88,>88,>70,>00
        BYTE 'V',>88,>88,>88,>88,>88,>50,>20,>00
        BYTE 'W',>88,>88,>88,>A8,>A8,>D8,>88,>00
        BYTE 'X',>88,>88,>50,>20,>50,>88,>88,>00
        BYTE 'Y',>88,>88,>50,>20,>20,>20,>20,>00
        BYTE 'Z',>F8,>08,>10,>20,>40,>80,>F8,>00
        BYTE 0

* ---- levels ----
* Twelve puzzles from "Microban" by David W. Skinner, transcribed exactly as
* published (microban.slc, Copyright David W Skinner; the sets "may be freely
* distributed provided they remain properly credited"). Each record is
* width, height, then height rows of XSB text padded to the width:
*   # wall   . spot   $ box   * box on spot   @ player   + player on spot
* The end-to-end test (crates/libre99-asm/tests/sokoban.rs) plays every level to
* completion, so a transcription typo cannot ship.
LEVTAB  DATA LVL01,LVL02,LVL03,LVL04,LVL05,LVL06
        DATA LVL07,LVL08,LVL09,LVL10,LVL11,LVL12

LVL01   BYTE 6,7              ; Microban 2
        TEXT '######'
        TEXT '#    #'
        TEXT '# #@ #'
        TEXT '# $* #'
        TEXT '# .* #'
        TEXT '#    #'
        TEXT '######'

LVL02   BYTE 6,7              ; Microban 1
        TEXT '####  '
        TEXT '# .#  '
        TEXT '#  ###'
        TEXT '#*@  #'
        TEXT '#  $ #'
        TEXT '#  ###'
        TEXT '####  '

LVL03   BYTE 8,6              ; Microban 4
        TEXT '########'
        TEXT '#      #'
        TEXT '# .**$@#'
        TEXT '#      #'
        TEXT '#####  #'
        TEXT '    ####'

LVL04   BYTE 8,7              ; Microban 5
        TEXT ' #######'
        TEXT ' #     #'
        TEXT ' # .$. #'
        TEXT '## $@$ #'
        TEXT '#  .$. #'
        TEXT '#      #'
        TEXT '########'

LVL05   BYTE 6,7              ; Microban 17
        TEXT '##### '
        TEXT '# @ # '
        TEXT '#...# '
        TEXT '#$$$##'
        TEXT '#    #'
        TEXT '#    #'
        TEXT '######'

LVL06   BYTE 7,8              ; Microban 7
        TEXT '#######'
        TEXT '#     #'
        TEXT '# .$. #'
        TEXT '# $.$ #'
        TEXT '# .$. #'
        TEXT '# $.$ #'
        TEXT '#  @  #'
        TEXT '#######'

LVL07   BYTE 6,7              ; Microban 9
        TEXT '##### '
        TEXT '#.  ##'
        TEXT '#@$$ #'
        TEXT '##   #'
        TEXT ' ##  #'
        TEXT '  ##.#'
        TEXT '   ###'

LVL08   BYTE 9,6              ; Microban 34
        TEXT '  ####   '
        TEXT '###  ####'
        TEXT '#       #'
        TEXT '#@$***. #'
        TEXT '#       #'
        TEXT '#########'

LVL09   BYTE 9,6              ; Microban 3
        TEXT '  ####   '
        TEXT '###  ####'
        TEXT '#     $ #'
        TEXT '# #  #$ #'
        TEXT '# . .#@ #'
        TEXT '#########'

LVL10   BYTE 7,7              ; Microban 33
        TEXT '#######'
        TEXT '#. #  #'
        TEXT '#  $  #'
        TEXT '#. $#@#'
        TEXT '#  $  #'
        TEXT '#. #  #'
        TEXT '#######'

LVL11   BYTE 7,10             ; Microban 35
        TEXT '  #### '
        TEXT ' ##  # '
        TEXT ' #. $# '
        TEXT ' #.$ # '
        TEXT ' #.$ # '
        TEXT ' #.$ # '
        TEXT ' #. $##'
        TEXT ' #   @#'
        TEXT ' ##   #'
        TEXT '  #####'

LVL12   BYTE 15,5             ; Microban 36
        TEXT '####           '
        TEXT '#  ############'
        TEXT '# $ $ $ $ $ @ #'
        TEXT '# .....       #'
        TEXT '###############'

        END  START
