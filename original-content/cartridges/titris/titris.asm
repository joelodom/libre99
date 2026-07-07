* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.

* ============================================================================
* TITRIS — a falling-blocks puzzle game for the TI-99/4A, assembled by this
* project's own libre99asm.
* U-shaped well (open top, no ceiling) that starts 20 wide x 20 tall and narrows
* one cell on the right per level (every LEVTHR points); locked blocks past the
* new edge are trimmed. Next-piece preview, score, level, color, sound, SRS
* rotation with wall kicks, 60 Hz loop. Graphics I mode.
* Controls:  Left/Right arrow = move     Down arrow = soft drop
*            Up arrow or X = rotate CW    Z = rotate CCW
*            SPACE or Right-Alt (joystick fire) = hard drop
* (Arrows / Right-Alt drive TI joystick 1, read on CRU col 6; X/Z/SPACE on the keyboard.)
* Build: cargo run -p libre99-asm -- original-content/cartridges/titris/titris.asm \
*        -o original-content/cartridges/titris/titris.ctg
* ============================================================================
        IDT  'TITRIS'

VDPWD   EQU  >8C00            ; VDP VRAM write data
VDPWA   EQU  >8C02            ; VDP address/register write (byte writes only)
VDPST   EQU  >8802            ; VDP status read
SOUND   EQU  >8400            ; SN76489
MAXW    EQU  20               ; starting / maximum well width (cols)
MINW    EQU  8                ; minimum well width (it shrinks down to this)
HEIGHT  EQU  20               ; well height in cells
STRIDE  EQU  32               ; board row stride: cell = BOARD + (row<<5) + col
GRAVITY EQU  30               ; frames per gravity step (~0.5 s)
LEVTHR  EQU  1000             ; score points per level (each level: width -1)
NAMEB   EQU  >0041            ; name-table addr of well cell (0,0): row2,col1

BOARD   EQU  >A000            ; HEIGHT rows x stride 32; cell: 0=empty else char
CURTYP  EQU  >8320
CURROT  EQU  >8322
CURX    EQU  >8324
CURY    EQU  >8326
DROPTM  EQU  >8328
RNG     EQU  >832A
PREVK   EQU  >832C
TICK    EQU  >832E
GMOVER  EQU  >8330
SNDTMR  EQU  >8332
NEXT    EQU  >8334           ; next piece type
SCORE   EQU  >8336           ; 16-bit score
SCBUF   EQU  >8338           ; 5-byte decimal scratch (>8338..>833C)
LINES   EQU  >833E           ; total rows cleared (game-over stat)
NPIECE  EQU  >8340           ; total pieces placed (game-over stat)
ROTTMP  EQU  >8342           ; target rotation state during a kick test
CURW    EQU  >8344           ; current well width (MAXW..MINW)
LEVEL   EQU  >8346           ; current level = SCORE/LEVTHR (uncapped; display pegs at 99)
RET2    EQU  >8348           ; nested-call return save (UPDLVL)
RET3    EQU  >834A           ; nested-call return save (REWELL)
OLDCEL  EQU  >834C           ; 4 name-table addrs of the piece's last-drawn cells
NEXTAT  EQU  >8354           ; score at which the next level is reached
FANCNT  EQU  >8356           ; level-up fanfare frames remaining (0 = silent)

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
* so park the attribute table in empty VRAM (>0780, between the color and
* pattern tables) and end it at once, or the name table's bytes get misread as
* phantom sprites. (Harmless here only by layout luck; make it explicit.)
        LI   R1,>0780
        BL   @SETWR
        LI   R0,>D000
        MOVB R0,@VDPWD
* solid border glyph (char >80) at pattern >0C00
        LI   R1,>0C00
        BL   @SETWR
        LI   R2,8
        LI   R0,>FF00
BDL     MOVB R0,@VDPWD
        DEC  R2
        JNE  BDL
* 7 solid block glyphs (chars >88,>90..>B8) at pattern >0C40 step 64
        LI   R4,7
        LI   R5,>0C40
BPL     MOV  R5,R1
        BL   @SETWR
        LI   R2,8
        LI   R0,>FF00
BPI     MOVB R0,@VDPWD
        DEC  R2
        JNE  BPI
        AI   R5,64
        DEC  R4
        JNE  BPL
* faint column-guide glyph (char >C0) at pattern >0E00: a 1px DOTTED line down the
* cell's left edge (dark blue) -> the most subtle vertical alignment guide the
* TMS9918A allows. Dark blue is the dimmest color; sparse dots (one lit row every
* four) fade it further. Only 2 dots per 8-row cell.
        LI   R1,>0E00
        BL   @SETWR
        LI   R2,2             ; 2 dots: a lit row then 3 blanks, repeated (rows 0,4)
        LI   R0,>8000         ; >80 = leftmost pixel lit
        CLR  R3               ; >00 = blank row
GGL     MOVB R0,@VDPWD
        MOVB R3,@VDPWD
        MOVB R3,@VDPWD
        MOVB R3,@VDPWD
        DEC  R2
        JNE  GGL
* text font: (char, 8 pattern bytes) records, terminated by char 0
        LI   R4,FONT
FLP     MOVB *R4+,R0          ; char code (high byte); 0 ends the table
        JEQ  FLPD
        SRL  R0,8
        SLA  R0,3             ; code*8
        AI   R0,>0800         ; pattern address
        MOV  R0,R1
        BL   @SETWR
        LI   R2,8
FLPB    MOVB *R4+,@VDPWD
        DEC  R2
        JNE  FLPB
        JMP  FLP
FLPD
* color: groups 0..16 white-on-black (text + border), 17..23 piece colors
        LI   R1,>0300
        BL   @SETWR
        LI   R2,17
        LI   R0,>F100
CIL     MOVB R0,@VDPWD
        DEC  R2
        JNE  CIL
        LI   R1,>0311
        BL   @SETWR
        LI   R2,COLORS
        LI   R3,7
CCL     MOVB *R2+,@VDPWD
        DEC  R3
        JNE  CCL
* color group 24 (guide glyph >C0): a dim line on black
        LI   R1,>0318
        BL   @SETWR
        LI   R0,>4100         ; dark blue on black
        MOVB R0,@VDPWD
        LI   R0,>A55A
        MOV  R0,@RNG

* ============================= title screen =================================
TITLE   BL   @CLRNT           ; clear the name table
* big "TITRIS" in block glyphs (3x5 cells per letter), rows 4..8, cols 4..26
        LI   R4,BIGLET
        CLR  R5               ; letter 0..5
BTL     CLR  R6               ; row 0..4
BTR     MOVB *R4+,R7          ; row bits (low 3)
        SRL  R7,8
        LI   R10,>0004        ; mask, left column first
        CLR  R8              ; col 0..2
BTC     COC  R10,R7           ; is this cell filled?
        JNE  BTCN
        MOV  R6,R1            ; name = (4+row)*32 + 4 + letter*4 + col
        AI   R1,4
        SLA  R1,5
        AI   R1,4
        MOV  R5,R9
        SLA  R9,2
        A    R9,R1
        A    R8,R1
        BL   @SETWR
        LI   R0,>8800         ; I-piece block glyph (>88)
        MOVB R0,@VDPWD
BTCN    SRL  R10,1
        INC  R8
        CI   R8,3
        JNE  BTC
        INC  R6
        CI   R6,5
        JNE  BTR
        INC  R5
        CI   R5,6
        JNE  BTL
* sub-title and prompt (centered)
        LI   R1,>0149         ; "FOR THE TI-99" at row 10, col 9
        BL   @SETWR
        LI   R2,TFOR
        LI   R3,13
        BL   @VMBW
        LI   R1,>01C5         ; "PRESS ANY KEY TO PLAY" at row 14, col 5
        BL   @SETWR
        LI   R2,TPRESS
        LI   R3,21
        BL   @VMBW
        LI   R1,>0207         ; "H OR AID FOR HELP" at row 16, col 7
        BL   @SETWR
        LI   R2,THELP
        LI   R3,17
        BL   @VMBW
* wait for all keys up, then a key down, then up again — so the starting
* keystroke (e.g. SPACE = hard drop) is fully consumed and not read as gameplay.
* H or AID (FCTN+7) opens the help screen instead of starting the game; bare
* modifier keys are ignored so holding FCTN for AID doesn't trigger a start.
TWU     BL   @ANYKEY
        CI   R0,0
        JNE  TWU
TWD     BL   @WAITVB
        BL   @HELPK          ; H or AID -> show the help screen
        CI   R0,0
        JNE  SHOWHLP
        BL   @ANYKY2         ; any non-modifier key -> start the game
        CI   R0,0
        JEQ  TWD
TWR     BL   @WAITVB
        BL   @ANYKEY
        CI   R0,0
        JNE  TWR
        JMP  GINIT
* H or AID at the title -> draw the help screen, then return to the title
SHOWHLP BL   @ANYKEY         ; wait for the help key to release
        CI   R0,0
        JNE  SHOWHLP
        BL   @HELP
SHWD    BL   @WAITVB         ; then wait for any key to dismiss it
        BL   @ANYKEY
        CI   R0,0
        JEQ  SHWD
SHWR    BL   @WAITVB
        BL   @ANYKEY
        CI   R0,0
        JNE  SHWR
        B    @TITLE

* ============================ start a new game ==============================
GINIT   BL   @CLRNT
        LI   R0,MAXW          ; start at full width, level 0
        MOV  R0,@CURW
        CLR  @LEVEL
        BL   @CLRBRD
        BL   @DBORD           ; U-shaped wall at the full width
        LI   R1,>0017         ; "NEXT" label (row 0, col 23) above the preview
        BL   @SETWR
        LI   R2,TNEXT
        LI   R3,4
        BL   @VMBW
        LI   R1,>00D7         ; "SCORE" label (row 6, col 23)
        BL   @SETWR
        LI   R2,TSCORE
        LI   R3,5
        BL   @VMBW
        LI   R1,>0177         ; "NEXT LEVEL AT", line 1 (row 11, col 23)
        BL   @SETWR
        LI   R2,TNEXT
        LI   R3,4
        BL   @VMBW
        LI   R1,>0197         ; "NEXT LEVEL AT", line 2 (row 12, col 23)
        BL   @SETWR
        LI   R2,TLEVAT
        LI   R3,8
        BL   @VMBW
        LI   R1,>0217         ; "LEVEL" label (row 16, col 23)
        BL   @SETWR
        LI   R2,TLEVEL
        LI   R3,5
        BL   @VMBW
        CLR  @SCORE
        CLR  @LINES
        CLR  @NPIECE
        CLR  @PREVK
        CLR  @TICK
        CLR  @GMOVER
        CLR  @SNDTMR
        CLR  @FANCNT
        LI   R0,LEVTHR        ; at level 0 the next level is reached at LEVTHR
        MOV  R0,@NEXTAT
        BL   @RNG7            ; seed the first "next" piece
        MOV  R0,@NEXT
        BL   @SPAWN
        LI   R0,GRAVITY
        MOV  R0,@DROPTM
        MOV  @SCORE,R5        ; draw the score (0) at row 7, col 23
        LI   R1,>00F7
        BL   @DNUM5
        MOV  @NEXTAT,R5       ; draw "next level at" (row 13, col 23)
        LI   R1,>01B7
        BL   @DNUM5
        MOV  @LEVEL,R5        ; draw the level (0) as 2 digits (row 17, col 23)
        LI   R1,>0237
        BL   @DNUM2
        BL   @DNEXT
        BL   @DRAWB           ; paint the (empty) board, then the first piece —
        BL   @PUTPC           ; this also primes OLDCEL for the incremental loop

* ================================ main loop =================================
MAIN    BL   @WAITVB
        INC  @TICK
* sound timer countdown -> silence
        MOV  @SNDTMR,R0
        JEQ  MSND
        DEC  R0
        MOV  R0,@SNDTMR
        JNE  MSND
        LI   R0,>9F00
        MOVB R0,@SOUND
MSND
* level-up fanfare: a short rising arpeggio on channel 1, one note set per frame,
* independent of the channel-0 lock/clear beeps. FANCNT counts the frames left.
        MOV  @FANCNT,R0
        JEQ  MFANX
        DEC  R0
        MOV  R0,@FANCNT
        JNE  MFANN
        LI   R0,>BF00         ; finished -> silence channel 1
        MOVB R0,@SOUND
        JMP  MFANX
MFANN   SRL  R0,2             ; note index = FANCNT/4 (steps 3..0 over time)
        SLA  R0,1             ; 2 bytes per note
        AI   R0,FANTAB
        MOV  R0,R1
        MOVB *R1+,@SOUND      ; channel-1 frequency, low nibble
        MOVB *R1,@SOUND       ; channel-1 frequency, high 6 bits
        LI   R0,>B000         ; channel-1 attenuation = loud
        MOVB R0,@SOUND
MFANX
* read keys; R13 = newly pressed, R15 = held (both survive COLLIDE)
        BL   @READK
        MOV  @PREVK,R5
        MOV  R1,@PREVK
        MOV  R1,R15
        MOV  R1,R13
        SZC  R5,R13           ; newpress = current AND NOT prev
* left (S) = newpress bit0
        MOV  R13,R0
        ANDI R0,>0001
        JEQ  MNL
        MOV  @CURTYP,R0
        MOV  @CURROT,R1
        MOV  @CURX,R2
        DEC  R2
        MOV  @CURY,R3
        BL   @COLLIDE
        CI   R0,0
        JNE  MNL
        DEC  @CURX
MNL
* right (D) = newpress bit1
        MOV  R13,R0
        ANDI R0,>0002
        JEQ  MNR
        MOV  @CURTYP,R0
        MOV  @CURROT,R1
        MOV  @CURX,R2
        INC  R2
        MOV  @CURY,R3
        BL   @COLLIDE
        CI   R0,0
        JNE  MNR
        INC  @CURX
MNR
* rotate clockwise = newpress bit2 (Up-arrow / X)
        MOV  R13,R0
        ANDI R0,>0004
        JEQ  MNT
        BL   @ROTCW
MNT
* rotate counter-clockwise = newpress bit5 (Z)
        MOV  R13,R0
        ANDI R0,>0020
        JEQ  MNC
        BL   @ROTCCW
MNC
* soft drop (Down-arrow held) = bit3 -> force a gravity step this frame
        MOV  R15,R0
        ANDI R0,>0008
        JEQ  MND
        LI   R0,1
        MOV  R0,@DROPTM
MND
* hard drop (SPACE) = newpress bit4
        MOV  R13,R0
        ANDI R0,>0010
        JEQ  MNH
MHL     MOV  @CURTYP,R0
        MOV  @CURROT,R1
        MOV  @CURX,R2
        MOV  @CURY,R3
        INC  R3
        BL   @COLLIDE
        CI   R0,0
        JNE  MNH2
        INC  @CURY
        JMP  MHL
MNH2    LI   R0,1
        MOV  R0,@DROPTM       ; lock on the gravity step below
MNH
* gravity
        MOV  @DROPTM,R0
        DEC  R0
        MOV  R0,@DROPTM
        JNE  MDRAW
        LI   R0,GRAVITY
        MOV  R0,@DROPTM
        MOV  @CURTYP,R0
        MOV  @CURROT,R1
        MOV  @CURX,R2
        MOV  @CURY,R3
        INC  R3
        BL   @COLLIDE
        CI   R0,0
        JNE  MLOCK
        INC  @CURY
        JMP  MDRAW
MLOCK   BL   @LOCK
        INC  @NPIECE          ; stat: one more piece placed
        BL   @BEEP
        BL   @CLEAR           ; R0 = number of rows cleared (0..4)
        A    R0,@LINES        ; stat: total rows cleared
        MOV  R0,R3
        SLA  R3,1             ; word index into the score table
        MOV  @SCORTB(R3),R1   ; classic points for that many lines
        A    R1,@SCORE
        BL   @UPDLVL          ; recompute level & width; shrink the well if needed
        MOV  @SCORE,R5        ; redraw the score (row 7, col 23)
        LI   R1,>00F7
        BL   @DNUM5
        MOV  @NEXTAT,R5       ; redraw "next level at" (row 13, col 23)
        LI   R1,>01B7
        BL   @DNUM5
        MOV  @LEVEL,R5        ; redraw the level as 2 digits (row 17, col 23)
        LI   R1,>0237
        BL   @DNUM2
        BL   @SPAWN
        BL   @DNEXT
        MOV  @GMOVER,R0
        JEQ  MLDRW
        B    @GAMEOV          ; topped out -> game-over overlay, then the title
MLDRW   BL   @DRAWB           ; board changed: repaint it...
        BL   @PUTPC           ; ...then draw the freshly spawned piece
        B    @MAIN
MDRAW   BL   @ERAPC           ; per-frame: erase the piece's old cells...
        BL   @PUTPC           ; ...and redraw it at its current position
        B    @MAIN            ; B (not JMP): the loop body exceeds JMP's +-256 range

* ============================ game-over overlay =============================
* Show the final board, then a summary panel (score + stats); any key -> title.
GAMEOV  LI   R0,>9F00         ; silence the lock/clear beep (no MAIN loop runs here)
        MOVB R0,@SOUND
        LI   R0,>BF00         ; and silence any level-up fanfare on channel 1
        MOVB R0,@SOUND
        BL   @DRAWB           ; render the final board behind the panel
* panel border (rows 6..17, cols 3..18), centered over the playfield
        LI   R1,>00C3         ; top edge (row 6, col 3)
        BL   @SETWR
        LI   R2,16
        LI   R0,>8000
GOBT    MOVB R0,@VDPWD
        DEC  R2
        JNE  GOBT
        LI   R1,>0223         ; bottom edge (row 17, col 3)
        BL   @SETWR
        LI   R2,16
        LI   R0,>8000
GOBB    MOVB R0,@VDPWD
        DEC  R2
        JNE  GOBB
        LI   R4,10            ; interior rows 7..16: wall, 14 spaces, wall
        LI   R5,>00E3
GOBR    MOV  R5,R1
        BL   @SETWR
        LI   R0,>8000
        MOVB R0,@VDPWD
        LI   R2,14
        LI   R0,>2000
GOBI    MOVB R0,@VDPWD
        DEC  R2
        JNE  GOBI
        LI   R0,>8000
        MOVB R0,@VDPWD
        AI   R5,32
        DEC  R4
        JNE  GOBR
* heading + stats
        LI   R1,>0106         ; "GAME OVER" (row 8)
        BL   @SETWR
        LI   R2,TGOVER
        LI   R3,9
        BL   @VMBW
        LI   R1,>0145         ; "SCORE" (row 10)
        BL   @SETWR
        LI   R2,TSCORE
        LI   R3,5
        BL   @VMBW
        MOV  @SCORE,R5
        LI   R1,>014C
        BL   @DNUM5
        LI   R1,>0165         ; "LINES" (row 11)
        BL   @SETWR
        LI   R2,TLINES
        LI   R3,5
        BL   @VMBW
        MOV  @LINES,R5
        LI   R1,>016C
        BL   @DNUM5
        LI   R1,>0185         ; "PIECES" (row 12)
        BL   @SETWR
        LI   R2,TPIECE
        LI   R3,6
        BL   @VMBW
        MOV  @NPIECE,R5
        LI   R1,>018C
        BL   @DNUM5
        LI   R1,>01A5         ; "TIME" in seconds = TICK/60 (row 13)
        BL   @SETWR
        LI   R2,TTIME
        LI   R3,4
        BL   @VMBW
        CLR  R2
        MOV  @TICK,R3
        LI   R0,60
        DIV  R0,R2
        MOV  R2,R5
        LI   R1,>01AC
        BL   @DNUM5
        LI   R1,>01C5         ; "LEVEL" (row 14)
        BL   @SETWR
        LI   R2,TLEVEL
        LI   R3,5
        BL   @VMBW
        MOV  @LEVEL,R5
        LI   R1,>01CC
        BL   @DNUM5
        LI   R1,>01E4         ; "PRESS ANY KEY" (row 15)
        BL   @SETWR
        LI   R2,TANYK
        LI   R3,13
        BL   @VMBW
* wait: all up, key down, up again; then back to the title
GOWU    BL   @ANYKEY
        CI   R0,0
        JNE  GOWU
GOWD    BL   @WAITVB
        BL   @ANYKEY
        CI   R0,0
        JEQ  GOWD
GOWR    BL   @WAITVB
        BL   @ANYKEY
        CI   R0,0
        JNE  GOWR
        B    @TITLE

* ============================== subroutines =================================
* WAITVB: block until the next vertical blank (60 Hz). clobbers R2.
WAITVB  MOVB @VDPST,R2
        ANDI R2,>8000
        JEQ  WAITVB
        RT

* SETWR: set the VRAM write address from R1.
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

* CLRNT: clear the 768-byte name table to spaces. (Calls SETWR, so it must save
* the return in R14 — its own RT would otherwise re-enter after the BL.)
CLRNT   MOV  R11,R14
        LI   R1,>0000
        BL   @SETWR
        LI   R2,768
        LI   R0,>2000
CNL     MOVB R0,@VDPWD
        DEC  R2
        JNE  CNL
        B    *R14

* CLRBRD: zero the whole board (HEIGHT rows x stride 32).
CLRBRD  LI   R1,BOARD
        LI   R2,HEIGHT*STRIDE
        CLR  R0
CBL     MOVB R0,*R1+
        DEC  R2
        JNE  CBL
        RT

* DBORD: draw the U-shaped white wall for the current width — left wall (col 0),
* right wall (col 1+CURW), and a bottom across (row 22); no top, so the well is
* open at the top. Left-anchored, so the right wall moves in as CURW shrinks.
DBORD   MOV  R11,R14
        LI   R4,HEIGHT        ; side walls span all content rows (2..21)
        LI   R5,>0040         ; row 2, col 0 = top of the left wall
DBS     MOV  R5,R1
        BL   @SETWR
        LI   R0,>8000
        MOVB R0,@VDPWD        ; left wall (col 0)
        MOV  R5,R1
        A    @CURW,R1
        INC  R1               ; right wall column = 0 + CURW + 1
        BL   @SETWR
        LI   R0,>8000
        MOVB R0,@VDPWD        ; right wall (col 1+CURW)
        AI   R5,32
        DEC  R4
        JNE  DBS
        LI   R1,>02C0         ; bottom edge, row 22 col 0
        BL   @SETWR
        MOV  @CURW,R2
        AI   R2,2             ; CURW+2 cells: left wall .. right wall
        LI   R0,>8000
DBB     MOVB R0,@VDPWD
        DEC  R2
        JNE  DBB
        B    *R14

* ANYKEY -> R0 = 1 if any key is down on columns 0..5, else 0. clobbers R0,R1,R2.
ANYKEY  CLR  R1               ; column 0..5
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
        CI   R1,8             ; scan columns 0..7 (keyboard + both joysticks)
        JNE  AKL
        CLR  R0
        RT
AKHIT   LI   R0,1
        RT

* HELPK -> R0 = 1 if the help key (H, or AID = FCTN+7) is down, else 0. The CRU
* scan mirrors READK: column in R0's high byte, 8 row bits read into R2's high
* byte (a pressed key reads 0). clobbers R0-R3,R12.
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

* ANYKY2 -> R0 = 1 if any NON-MODIFIER key is down (FCTN/SHIFT/CTRL ignored), so
* holding FCTN for AID at the title doesn't count as a start. clobbers R0,R1,R2.
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

* RNG7 -> R0 in 0..6 (xorshift-16 LFSR in RNG).
RNG7    MOV  @RNG,R0
        JNE  RN1
        LI   R0,>1357
RN1     MOV  R0,R1
        SLA  R1,7
        XOR  R1,R0
        MOV  R0,R1
        SRL  R1,9
        XOR  R1,R0
        MOV  R0,R1
        SLA  R1,8
        XOR  R1,R0
        MOV  R0,@RNG
        ANDI R0,>0007
        CI   R0,7
        JNE  RN2
        CLR  R0
RN2     RT

* READK -> R1 control mask:
*   b0 Left  b1 Right  b2 RotateCW  b3 SoftDrop  b4 HardDrop  b5 RotateCCW
* Keyboard: X = CW, Z = CCW, SPACE = hard drop (conventional Z/X rotation).
* Movement / soft drop / a second CW come from joystick 1 (the host arrow keys).
READK   CLR  R1
        CLR  R0               ; column 0 -> SPACE (row1, >0200) = hard drop
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>0200
        JNE  RKA
        ORI  R1,>0010
RKA     LI   R0,>0100         ; column 1 -> X (row7, >8000) = rotate CW
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>8000
        JNE  RKB
        ORI  R1,>0004
RKB     LI   R0,>0500         ; column 5 -> Z (row7, >8000) = rotate CCW
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>8000
        JNE  RKE
        ORI  R1,>0020
RKE     LI   R0,>0600         ; column 6 -> joystick 1 (host arrow keys)
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>0200         ; Joy1 left
        JNE  JSR
        ORI  R1,>0001
JSR     MOV  R2,R3
        ANDI R3,>0400         ; Joy1 right
        JNE  JSU
        ORI  R1,>0002
JSU     MOV  R2,R3
        ANDI R3,>1000         ; Joy1 up -> rotate CW
        JNE  JSD
        ORI  R1,>0004
JSD     MOV  R2,R3
        ANDI R3,>0800         ; Joy1 down -> soft drop
        JNE  JSF
        ORI  R1,>0008
JSF     MOV  R2,R3
        ANDI R3,>0100         ; Joy1 fire -> hard drop
        JNE  JSE
        ORI  R1,>0010
JSE     RT

* ROTCW / ROTCCW: rotate the current piece with SRS wall kicks. Tries the 5
* offsets for this (from->to) transition; the first that fits is applied. uses
* R14 save (calls COLLIDE). COLLIDE preserves R10/R12, so the loop state lives
* there; the target state lives in ROTTMP.
ROTCW   LI   R5,1             ; +1 = clockwise
        JMP  ROTGO
ROTCCW  LI   R5,3             ; +3 = counter-clockwise (-1 mod 4)
ROTGO   MOV  R11,R14
        MOV  @CURROT,R6
        A    R5,R6
        ANDI R6,>0003         ; target state
        MOV  R6,@ROTTMP
        MOV  @CURROT,R7       ; kick-table row: CW -> state, CCW -> state+4
        CI   R5,1
        JEQ  ROTROW
        AI   R7,4
ROTROW  MOV  R7,R0            ; offset = row * 10 bytes
        SLA  R0,3
        A    R7,R0
        A    R7,R0
        MOV  @CURTYP,R1       ; I (type 0) has its own kick table
        CI   R1,0
        JNE  ROTJ
        LI   R12,KICKI
        JMP  ROTBAS
ROTJ    LI   R12,KJLSTZ
ROTBAS  A    R0,R12           ; R12 -> this transition's 5 (dc,dr) offsets
        LI   R10,5
ROTTRY  MOVB *R12+,R2         ; dc (signed byte)
        SRA  R2,8
        MOVB *R12+,R3         ; dr (signed byte)
        SRA  R3,8
        A    @CURX,R2         ; trial x
        A    @CURY,R3         ; trial y
        MOV  @CURTYP,R0
        MOV  @ROTTMP,R1
        BL   @COLLIDE
        CI   R0,0
        JEQ  ROTACC           ; fits -> apply
        DEC  R10
        JNE  ROTTRY
        B    *R14             ; no kick fit -> leave the piece as-is
ROTACC  MOV  @ROTTMP,R0
        MOV  R0,@CURROT
        MOV  R2,@CURX
        MOV  R3,@CURY
        B    *R14

* COLLIDE: in R0=type R1=rot R2=x R3=y. out R0=1 if collision else 0.
* preserves R1,R2,R3,R10-R15; clobbers R4-R9.
COLLIDE MOV  R0,R4
        SLA  R4,2
        A    R1,R4
        SLA  R4,2
        AI   R4,PIECES        ; R4 -> 4 cell bytes
        LI   R5,4
COLP    MOVB *R4+,R6
        SRL  R6,8             ; R6 = (dr<<4)|dc
        MOV  R6,R7
        SRL  R7,4             ; R7 = dr
        ANDI R6,>000F         ; R6 = dc
        A    R3,R7            ; br = y+dr
        A    R2,R6            ; bc = x+dc
        C    R6,@CURW
        JHE  COLHIT           ; bc out of range (>=CURW, or negative wrapped)
        CI   R7,HEIGHT
        JHE  COLHIT           ; br below the floor
        MOV  R7,R8
        SLA  R8,5             ; row * stride 32
        A    R6,R8
        AI   R8,BOARD
        MOVB *R8,R9
        JNE  COLHIT           ; cell occupied
        DEC  R5
        JNE  COLP
        CLR  R0
        RT
COLHIT  LI   R0,1
        RT

* LOCK: stamp the current piece into the board. clobbers R4-R8,R13.
LOCK    MOV  @CURTYP,R0
        SLA  R0,3
        AI   R0,>0088
        SWPB R0
        MOV  R0,R13           ; block char in the high byte
        MOV  @CURTYP,R4
        SLA  R4,2
        A    @CURROT,R4
        SLA  R4,2
        AI   R4,PIECES
        LI   R5,4
LKP     MOVB *R4+,R6
        SRL  R6,8
        MOV  R6,R7
        SRL  R7,4
        ANDI R6,>000F
        A    @CURY,R7
        A    @CURX,R6
        MOV  R7,R8
        SLA  R8,5             ; row * stride 32
        A    R6,R8
        AI   R8,BOARD
        MOVB R13,*R8
        DEC  R5
        JNE  LKP
        RT

* SPAWN: current piece <- NEXT; NEXT <- new random; set GMOVER if it can't fit.
SPAWN   MOV  R11,R14
        MOV  @NEXT,R0
        MOV  R0,@CURTYP
        BL   @RNG7
        MOV  R0,@NEXT
        CLR  @CURROT
        MOV  @CURW,R0         ; spawn centered: x = CURW/2 - 2
        SRL  R0,1
        AI   R0,-2
        MOV  R0,@CURX
        CLR  @CURY
        MOV  @CURTYP,R0
        MOV  @CURROT,R1
        MOV  @CURX,R2
        MOV  @CURY,R3
        BL   @COLLIDE
        CI   R0,0
        JEQ  SPK
        SETO @GMOVER
SPK     B    *R14

* CLEAR: remove full rows (shifting down), beep per clear. out R0 = #cleared.
CLEAR   MOV  R11,R14
        CLR  R9               ; cleared-row count
        LI   R4,HEIGHT-1
CLROW   MOV  R4,R5
        SLA  R5,5             ; row * stride 32
        AI   R5,BOARD
        MOV  @CURW,R6
CLCHK   MOVB *R5+,R7
        JEQ  CLNOTF
        DEC  R6
        JNE  CLCHK
        INC  R9
        BL   @BEEP2
        MOV  R4,R8
CLSH    CI   R8,0
        JEQ  CLTOP
        MOV  R8,R1
        SLA  R1,5             ; row * stride 32
        AI   R1,BOARD
        MOV  R1,R2
        AI   R2,-STRIDE
        MOV  @CURW,R3
CLCPY   MOVB *R2+,*R1+
        DEC  R3
        JNE  CLCPY
        DEC  R8
        JMP  CLSH
CLTOP   LI   R1,BOARD
        MOV  @CURW,R3
        CLR  R0
CLZ     MOVB R0,*R1+
        DEC  R3
        JNE  CLZ
        JMP  CLROW            ; recheck this row index after the shift
CLNOTF  DEC  R4
        JLT  CLEND
        JMP  CLROW
CLEND   MOV  R9,R0
        B    *R14

* The well is redrawn incrementally to stay inside the vertical-blank window:
* every frame only the falling piece is erased and redrawn (8 cells), and the
* whole board is repainted (DRAWB) only when it actually changes — on lock, line
* clear, level shrink, or a new game. A full per-frame repaint of the 20x20 well
* overran vblank and the piece flickered as the display caught it mid-draw.

* DRAWB: paint the locked board into the well (no current piece). uses R14 save.
DRAWB   MOV  R11,R14
        CLR  R4               ; row
DBROW   MOV  R4,R1            ; name addr = row*32 + NAMEB
        SLA  R1,5
        AI   R1,NAMEB
        BL   @SETWR
        MOV  R4,R5            ; board row base
        SLA  R5,5             ; row * stride 32
        AI   R5,BOARD
        MOV  @CURW,R6
DBCOL   MOVB *R5+,R7
        JNE  DBPUT
        LI   R7,>C000         ; empty -> faint column guide
DBPUT   MOVB R7,@VDPWD
        DEC  R6
        JNE  DBCOL
        INC  R4
        CI   R4,HEIGHT
        JNE  DBROW
        B    *R14

* PUTPC: draw the current piece's 4 cells (block char), saving each cell's
* name-table address in OLDCEL so ERAPC can blank exactly those next frame.
* uses R14 save.
PUTPC   MOV  R11,R14
        MOV  @CURTYP,R0
        SLA  R0,3
        AI   R0,>0088
        SWPB R0
        MOV  R0,R13           ; block char in the high byte
        MOV  @CURTYP,R4
        SLA  R4,2
        A    @CURROT,R4
        SLA  R4,2
        AI   R4,PIECES
        LI   R5,4
        LI   R8,OLDCEL        ; -> saved-cell array
PPVL    MOVB *R4+,R6
        SRL  R6,8
        MOV  R6,R7
        SRL  R7,4
        ANDI R6,>000F
        A    @CURY,R7
        A    @CURX,R6
        MOV  R7,R1
        SLA  R1,5
        A    R6,R1
        AI   R1,NAMEB         ; R1 = this cell's name-table address
        MOV  R1,*R8+          ; remember it for the next erase
        BL   @SETWR
        MOVB R13,@VDPWD
        DEC  R5
        JNE  PPVL
        B    *R14

* ERAPC: blank the 4 cells the piece occupied last frame (from OLDCEL). The
* floating piece never overlaps locked blocks, so these are always empty board
* cells -> writing a space is correct. uses R14 save.
ERAPC   MOV  R11,R14
        LI   R8,OLDCEL
        LI   R5,4
EPVL    MOV  *R8+,R1          ; a saved name-table address
        BL   @SETWR
        LI   R0,>C000         ; restore the empty-cell guide behind the piece
        MOVB R0,@VDPWD
        DEC  R5
        JNE  EPVL
        B    *R14

* DNUM5: draw the 16-bit value in R5 as 5 decimal digits at name address R1.
* uses R14 save and DIV. (R1 survives the digit loop, so SETWR still has it.)
DNUM5   MOV  R11,R14
        LI   R4,SCBUF+4       ; fill least-significant digit first
        LI   R6,5
        LI   R0,10
DSL     CLR  R2
        MOV  R5,R3           ; dividend = R2:R3 = 0:value
        DIV  R0,R2           ; -> R2 quotient, R3 remainder (digit)
        AI   R3,>0030        ; '0' + digit
        SWPB R3
        MOVB R3,*R4
        DEC  R4
        MOV  R2,R5           ; value <- quotient
        DEC  R6
        JNE  DSL
        BL   @SETWR          ; R1 still holds the destination
        LI   R2,SCBUF
        LI   R3,5
        BL   @VMBW
        B    *R14

* DNEXT: clear the preview box and draw the NEXT piece (rotation 0). uses R14.
DNEXT   MOV  R11,R14
        LI   R4,4             ; clear 4 rows x 4 cols at rows 1..4, cols 23..26
        LI   R5,>0037
DNC     MOV  R5,R1
        BL   @SETWR
        LI   R2,4
        LI   R0,>2000
DNCI    MOVB R0,@VDPWD
        DEC  R2
        JNE  DNCI
        AI   R5,32
        DEC  R4
        JNE  DNC
        MOV  @NEXT,R0
        SLA  R0,3
        AI   R0,>0088
        SWPB R0
        MOV  R0,R13
        MOV  @NEXT,R4
        SLA  R4,2
        SLA  R4,2            ; (type*4 + 0) * 4 = type*16
        AI   R4,PIECES
        LI   R5,4
DNP     MOVB *R4+,R6
        SRL  R6,8
        MOV  R6,R7
        SRL  R7,4
        ANDI R6,>000F
        MOV  R7,R1
        SLA  R1,5
        AI   R1,>0037         ; preview origin (row 1, col 23)
        A    R6,R1
        BL   @SETWR
        MOVB R13,@VDPWD
        DEC  R5
        JNE  DNP
        B    *R14

* DNUM2: draw the value in R5 as 2 decimal digits at name address R1, pegged at
* 99 (so the level field never overflows its two columns). uses R14 save and DIV.
DNUM2   MOV  R11,R14
        CI   R5,99
        JLE  D2OK
        LI   R5,99
D2OK    CLR  R2
        MOV  R5,R3           ; dividend R2:R3 = 0 : value
        LI   R0,10
        DIV  R0,R2           ; R2 = tens, R3 = ones
        BL   @SETWR          ; R1 = destination
        AI   R2,>0030        ; tens digit
        SWPB R2
        MOVB R2,@VDPWD
        AI   R3,>0030        ; ones digit
        SWPB R3
        MOVB R3,@VDPWD
        B    *R14

* HELP: draw the title-screen help (scoring + controls) with a colored accent
* bar and section bullets. The caller waits for the key to dismiss it. Saves its
* return in RET2 (it calls CLRNT, which uses R14).
HELP    MOV  R11,@RET2
        BL   @CLRNT
        LI   R4,HLPTAB        ; (name-addr, string-ptr, length) records, 0-term
HLPL    MOV  *R4+,R1
        CI   R1,0
        JEQ  HLPACC
        BL   @SETWR
        MOV  *R4+,R2
        MOV  *R4+,R3
        BL   @VMBW
        JMP  HLPL
HLPACC  LI   R1,>006C         ; a 7-block rainbow (the piece colors) under the title
        BL   @SETWR
        LI   R0,>8800         ; block glyph >88, then +8 per color group
        LI   R2,7
HLPRB   MOVB R0,@VDPWD
        AI   R0,>0800
        DEC  R2
        JNE  HLPRB
        LI   R1,>00A2         ; colored bullet before "SCORING"
        BL   @SETWR
        LI   R0,>9800
        MOVB R0,@VDPWD
        LI   R1,>0182         ; colored bullet before "CONTROLS"
        BL   @SETWR
        LI   R0,>B000
        MOVB R0,@VDPWD
        MOV  @RET2,R11
        RT

* BEEP/BEEP2: short tones on channel 0; SNDTMR silences them a few frames later.
BEEP    LI   R0,>8500
        MOVB R0,@SOUND
        LI   R0,>0800
        MOVB R0,@SOUND
        LI   R0,>9000
        MOVB R0,@SOUND
        LI   R0,4
        MOV  R0,@SNDTMR
        RT
BEEP2   LI   R0,>8A00
        MOVB R0,@SOUND
        LI   R0,>0400
        MOVB R0,@SOUND
        LI   R0,>9000
        MOVB R0,@SOUND
        LI   R0,8
        MOV  R0,@SNDTMR
        RT

* ===================== levels: shrink the well over time =====================
* TRIM: discard every locked block at column >= CURW (the cells the shrinking
* right wall passed over). Leaf; clobbers R0-R4.
TRIM    CLR  R4               ; row
TRROW   MOV  R4,R1
        SLA  R1,5             ; row * stride 32
        A    @CURW,R1
        AI   R1,BOARD         ; -> &board[row][CURW]
        LI   R2,STRIDE
        S    @CURW,R2         ; R2 = stride - CURW = dead cells in this row
        CLR  R0
TRCOL   MOVB R0,*R1+
        DEC  R2
        JNE  TRCOL
        INC  R4
        CI   R4,HEIGHT
        JNE  TRROW
        RT

* REWELL: repaint the well after a width change — blank the maximal footprint
* (so the old, wider walls disappear), then redraw the U-wall at the new width.
* The board cells are repainted by the next DRAW. Saves its return in RET3
* because it calls DBORD (which itself uses R14).
REWELL  MOV  R11,@RET3
        LI   R4,HEIGHT+1      ; content rows + the bottom-wall row
        LI   R5,>0040         ; row 2, col 0
RWR     MOV  R5,R1
        BL   @SETWR
        LI   R2,MAXW+2        ; left wall .. widest possible right wall
        LI   R0,>2000
RWC     MOVB R0,@VDPWD
        DEC  R2
        JNE  RWC
        AI   R5,32
        DEC  R4
        JNE  RWR
        BL   @DBORD
        MOV  @RET3,R11
        RT

* UPDLVL: recompute LEVEL = SCORE/LEVTHR (uncapped) and NEXTAT = (LEVEL+1)*LEVTHR,
* the score at which the next level is reached. On any level-up, start the
* fanfare. The target width is MAXW-LEVEL floored at MINW; if it dropped, trim the
* lost columns, clear any rows the trim completed (not scored), and repaint.
* Saves its return in RET2 (it calls CLEAR/REWELL, which use R14).
UPDLVL  MOV  R11,@RET2
        MOV  @LEVEL,R4       ; old level, to detect a level-up
        CLR  R2
        MOV  @SCORE,R3       ; R2:R3 = 0 : score
        LI   R0,LEVTHR
        DIV  R0,R2           ; R2 = level = score/LEVTHR, R3 = remainder
        MOV  R2,@LEVEL
        MOV  @SCORE,R1       ; NEXTAT = score - rem + LEVTHR = (level+1)*LEVTHR
        S    R3,R1
        AI   R1,LEVTHR
        MOV  R1,@NEXTAT
        C    R2,R4           ; level went up?
        JLE  ULNF
        LI   R0,16           ; yes -> kick off the level-up fanfare
        MOV  R0,@FANCNT
ULNF    CI   R2,MAXW-MINW    ; target width = MAXW - level, floored at MINW
        JLT  ULWID
        LI   R0,MINW
        JMP  ULSET
ULWID   LI   R0,MAXW
        S    R2,R0
ULSET   C    R0,@CURW
        JHE  ULNS           ; target >= current -> nothing to shrink
        MOV  R0,@CURW
        BL   @TRIM
        BL   @CLEAR
        BL   @REWELL
ULNS    MOV  @RET2,R11
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

* Reskinned piece colors, deliberately different from the classic palette:
* I magenta, O light blue, T light green, S light red, Z light yellow,
* J cyan, L white (each foreground on black).
COLORS  BYTE >D1,>51,>31,>91,>B1,>71,>F1   ; I O T S Z J L

* classic line-clear scoring: 0,1,2,3,4 rows -> 0,40,100,300,1200 points
SCORTB  DATA 0,40,100,300,1200

* big-title letters "TITRIS": 6 letters x 5 rows, low 3 bits = a 3-wide row
BIGLET  BYTE >07,>02,>02,>02,>02   ; T
        BYTE >07,>02,>02,>02,>07   ; I
        BYTE >07,>02,>02,>02,>02   ; T
        BYTE >06,>05,>06,>05,>05   ; R
        BYTE >07,>02,>02,>02,>07   ; I
        BYTE >07,>04,>07,>01,>07   ; S

TFOR    TEXT 'FOR THE TI-99'
TPRESS  TEXT 'PRESS ANY KEY TO PLAY'
TSCORE  TEXT 'SCORE'
TNEXT   TEXT 'NEXT'
TLEVEL  TEXT 'LEVEL'
TLEVAT  TEXT 'LEVEL AT'

* level-up fanfare: 4 rising notes for channel 1, each (freq-low byte, freq-high
* byte). Played highest-index first, so pitch ascends over the ~16-frame jingle.
FANTAB  BYTE >A8,>07,>A0,>0A,>A8,>0C,>A0,>0F
TGOVER  TEXT 'GAME OVER'
TLINES  TEXT 'LINES'
TPIECE  TEXT 'PIECES'
TTIME   TEXT 'TIME'
TANYK   TEXT 'PRESS ANY KEY'

* help-screen text. THELP is the title-screen hint; the rest are drawn by HELP via
* HLPTAB, which pairs each string with a name-table address and a length so the
* key/value columns line up without hand-counted padding.
THELP   TEXT 'H OR AID FOR HELP'
HTITLE  TEXT 'TITRIS HELP'
HSCOR   TEXT 'SCORING'
HL1     TEXT '1 LINE'
HL2     TEXT '2 LINES'
HL3     TEXT '3 LINES'
HL4     TEXT '4 LINES'
HV40    TEXT '40'
HV100   TEXT '100'
HV300   TEXT '300'
HV1200  TEXT '1200'
HLVL    TEXT 'NEW LEVEL EVERY 1000 PTS'
HCTRL   TEXT 'CONTROLS'
HK1     TEXT 'LEFT RIGHT'
HK2     TEXT 'DOWN'
HK3     TEXT 'UP OR X'
HK4     TEXT 'Z'
HK5     TEXT 'SPACE'
HD1     TEXT 'MOVE'
HD2     TEXT 'SOFT DROP'
HD3     TEXT 'ROTATE'
HD4     TEXT 'ROTATE BACK'
HD5     TEXT 'HARD DROP'

* HELP layout: (name-table addr, string, length) per line, terminated by 0.
HLPTAB  DATA >004A,HTITLE,11
        DATA >00A4,HSCOR,7
        DATA >00C5,HL1,6
        DATA >00D4,HV40,2
        DATA >00E5,HL2,7
        DATA >00F3,HV100,3
        DATA >0105,HL3,7
        DATA >0113,HV300,3
        DATA >0125,HL4,7
        DATA >0132,HV1200,4
        DATA >0144,HLVL,24
        DATA >0184,HCTRL,8
        DATA >01A5,HK1,10
        DATA >01B2,HD1,4
        DATA >01C5,HK2,4
        DATA >01D2,HD2,9
        DATA >01E5,HK3,7
        DATA >01F2,HD3,6
        DATA >0205,HK4,1
        DATA >0212,HD4,11
        DATA >0225,HK5,5
        DATA >0232,HD5,9
        DATA >0289,TANYK,13
        DATA 0

* 8x8 text font: char code then 8 pattern bytes; uppercase, digits, dash.
FONT    BYTE '-',>00,>00,>00,>F8,>00,>00,>00,>00
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
        BYTE 'C',>70,>88,>80,>80,>80,>88,>70,>00
        BYTE 'E',>F8,>80,>80,>F0,>80,>80,>F8,>00
        BYTE 'F',>F8,>80,>80,>F0,>80,>80,>80,>00
        BYTE 'G',>70,>88,>80,>B8,>88,>88,>70,>00
        BYTE 'H',>88,>88,>88,>F8,>88,>88,>88,>00
        BYTE 'I',>70,>20,>20,>20,>20,>20,>70,>00
        BYTE 'K',>88,>90,>A0,>C0,>A0,>90,>88,>00
        BYTE 'L',>80,>80,>80,>80,>80,>80,>F8,>00
        BYTE 'M',>88,>D8,>A8,>88,>88,>88,>88,>00
        BYTE 'N',>88,>C8,>A8,>98,>88,>88,>88,>00
        BYTE 'O',>70,>88,>88,>88,>88,>88,>70,>00
        BYTE 'P',>F0,>88,>88,>F0,>80,>80,>80,>00
        BYTE 'R',>F0,>88,>88,>F0,>A0,>90,>88,>00
        BYTE 'S',>70,>88,>80,>70,>08,>88,>70,>00
        BYTE 'T',>F8,>20,>20,>20,>20,>20,>20,>00
        BYTE 'V',>88,>88,>88,>88,>88,>50,>20,>00
        BYTE 'X',>88,>88,>50,>20,>50,>88,>88,>00
        BYTE 'Y',>88,>88,>50,>20,>20,>20,>20,>00
        BYTE 'B',>F0,>88,>88,>F0,>88,>88,>F0,>00
        BYTE 'D',>F0,>88,>88,>88,>88,>88,>F0,>00
        BYTE 'J',>38,>10,>10,>10,>10,>90,>60,>00
        BYTE 'Q',>70,>88,>88,>88,>A8,>90,>68,>00
        BYTE 'U',>88,>88,>88,>88,>88,>88,>70,>00
        BYTE 'W',>88,>88,>88,>A8,>A8,>D8,>88,>00
        BYTE 'Z',>F8,>08,>10,>20,>40,>80,>F8,>00
        BYTE 0

* SRS true-rotation cell offsets: 7 types x 4 states (0,R,2,L) x 4 cells;
* byte = (dr<<4)|dc within the spawn bounding box (4x4 for I, top-left 3x3 else).
PIECES  BYTE >10,>11,>12,>13           ; I  0
        BYTE >02,>12,>22,>32           ;    R
        BYTE >20,>21,>22,>23           ;    2
        BYTE >01,>11,>21,>31           ;    L
        BYTE >01,>02,>11,>12           ; O  (does not rotate)
        BYTE >01,>02,>11,>12
        BYTE >01,>02,>11,>12
        BYTE >01,>02,>11,>12
        BYTE >01,>10,>11,>12           ; T  0
        BYTE >01,>11,>12,>21           ;    R
        BYTE >10,>11,>12,>21           ;    2
        BYTE >01,>10,>11,>21           ;    L
        BYTE >01,>02,>10,>11           ; S  0
        BYTE >01,>11,>12,>22           ;    R
        BYTE >11,>12,>20,>21           ;    2
        BYTE >00,>10,>11,>21           ;    L
        BYTE >00,>01,>11,>12           ; Z  0
        BYTE >02,>11,>12,>21           ;    R
        BYTE >10,>11,>21,>22           ;    2
        BYTE >01,>10,>11,>20           ;    L
        BYTE >00,>10,>11,>12           ; J  0
        BYTE >01,>02,>11,>21           ;    R
        BYTE >10,>11,>12,>22           ;    2
        BYTE >01,>11,>20,>21           ;    L
        BYTE >02,>10,>11,>12           ; L  0
        BYTE >01,>11,>21,>22           ;    R
        BYTE >10,>11,>12,>20           ;    2
        BYTE >00,>01,>11,>21           ;    L

* SRS wall-kick offsets, as (dc,dr) signed bytes. 8 rows (5 tests each), ordered
* by (from-state): rows 0-3 = clockwise from 0/1/2/3, rows 4-7 = CCW from 0/1/2/3.
KJLSTZ  BYTE 0,0,-1,0,-1,-1,0,2,-1,2        ; 0->1
        BYTE 0,0,1,0,1,1,0,-2,1,-2          ; 1->2
        BYTE 0,0,1,0,1,-1,0,2,1,2           ; 2->3
        BYTE 0,0,-1,0,-1,1,0,-2,-1,-2       ; 3->0
        BYTE 0,0,1,0,1,-1,0,2,1,2           ; 0->3
        BYTE 0,0,1,0,1,1,0,-2,1,-2          ; 1->0
        BYTE 0,0,-1,0,-1,-1,0,2,-1,2        ; 2->1
        BYTE 0,0,-1,0,-1,1,0,-2,-1,-2       ; 3->2
KICKI   BYTE 0,0,-2,0,1,0,-2,1,1,-2         ; 0->1
        BYTE 0,0,-1,0,2,0,-1,-2,2,1         ; 1->2
        BYTE 0,0,2,0,-1,0,2,-1,-1,2         ; 2->3
        BYTE 0,0,1,0,-2,0,1,2,-2,-1         ; 3->0
        BYTE 0,0,-1,0,2,0,-1,-2,2,1         ; 0->3
        BYTE 0,0,2,0,-1,0,2,-1,-1,2         ; 1->0
        BYTE 0,0,1,0,-2,0,1,2,-2,-1         ; 2->1
        BYTE 0,0,-2,0,1,0,-2,1,1,-2         ; 3->2
        END  START
