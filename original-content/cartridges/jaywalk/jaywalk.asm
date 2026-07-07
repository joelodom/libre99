* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.

* ============================================================================
* JAYWALK — an endless-hopper arcade game for the TI-99/4A, assembled by this
* project's own libre99asm.
* A fledgling blue jay — too young to fly — hops north forever across a
* procedurally generated world of grass, roads, rivers, and rail lines.
* Cars, drifting logs, lily pads, level-crossing signals with real trains,
* coins, and a hawk that snatches birds who dawdle. Score 10 per lane
* northward, 25 per coin; the world speeds up as you go.
* A deliberate showcase for the machine: up to 24 simultaneous 16x16 sprites
* at independent sub-pixel speeds, the sprite early-clock bit, color-table
* animation (the flashing crossing signal), pattern-table animation (the
* shimmering river), and all four SN76489 voices (three tones + noise).
* Controls:  E/S/D/X or joystick 1 (host arrow keys) = hop N/W/E/S
*            H or AID at the title = help
* Build: cargo run -p libre99-asm -- original-content/cartridges/jaywalk/jaywalk.asm \
*        -o original-content/cartridges/jaywalk/jaywalk.ctg
* ============================================================================
        IDT  'JAYWALK'

VDPWD   EQU  >8C00            ; VDP VRAM write data
VDPWA   EQU  >8C02            ; VDP address/register write (byte writes only)
VDPST   EQU  >8802            ; VDP status read
SOUND   EQU  >8400            ; SN76489

* --- world tuning ---
HOPFRM  EQU  8                ; frames per hop (16 px at 2 px/frame)
HAWKWRN EQU  360              ; idle frames until the hawk's warning screech
HAWKATK EQU  480              ; idle frames until the hawk commits to the dive
TRKWRAP EQU  4480             ; road/river track length, 12.4 fixed (280 px)
TRKMIN  EQU  -384             ; track left edge, 12.4 fixed (-24 px)
TRKMAX  EQU  4096             ; track right edge, 12.4 fixed (256 px)
CARGAP  EQU  2432             ; spacing between a lane's two objects (152 px)
TRSPD   EQU  64               ; train speed, 12.4 fixed (4 px/frame)
TREND   EQU  5504             ; train sweep limit, 12.4 fixed (344 px)
TRBEG   EQU  -1408            ; train sweep limit, 12.4 fixed (-88 px)

* --- game state (scratchpad; workspace is >8300..>831F) ---
GMODE   EQU  >8320            ; 0 title, 1 playing, 2 dying, 3 game over
TICK    EQU  >8322            ; frame counter (all modes)
RNG     EQU  >8324            ; xorshift-16 state
PREVK   EQU  >8326            ; previous frame's key mask
KNEW    EQU  >8328            ; newly pressed this frame
VBOT    EQU  >832A            ; world lane index of the bottom visible row
PLANE   EQU  >832C            ; player's world lane
PX      EQU  >832E            ; player x, 12.4 fixed (pixel = PX/16, 0..240)
HOPT    EQU  >8330            ; frames left in the current hop (0 = grounded)
HOPDIR  EQU  >8332            ; 1 north, 2 south, 3 west, 4 east
RIDE    EQU  >8334            ; 0 afoot, 1/2 riding that log of lane PLANE
SCORE   EQU  >8336            ; 10/lane + 25/coin (16-bit)
COINS   EQU  >8338            ; coins collected this run
LCROSS  EQU  >833A            ; lanes crossed northward (furthest - start)
BEST    EQU  >833C            ; best score this power-on
DEAD    EQU  >833E            ; death cause: 1 car, 2 river, 3 train, 4 hawk
DTIMER  EQU  >8340            ; death-animation frames remaining
HAWKT   EQU  >8342            ; frames since the last hop (idle timer)
HAWKON  EQU  >8344            ; hawk state: 0 off, 1 diving
HAWKX   EQU  >8346            ; hawk x (pixels; title reuses it for the car)
HAWKY   EQU  >8348            ; hawk y (pixels, top edge; may be negative)
SCRLRQ  EQU  >834A            ; playfield repaint requested (scroll/coin)
SNDP0   EQU  >834C            ; sound-voice table pointers (0 = idle):
SNDP1   EQU  >834E            ;   voice 0 = tone ch 0, voice 1 = tone ch 1,
SNDP2   EQU  >8350            ;   voice 2 = noise
SCBUF   EQU  >8352            ; 5-byte decimal scratch (>8352..>8356)
RET2    EQU  >8358            ; nested-call return saves
RET3    EQU  >835A
FROMLN  EQU  >835C            ; lane a north/south hop departed from
TARGX   EQU  >835E            ; hop target x, 12.4 fixed
DIRTYF  EQU  >8360            ; HUD dirty flags: b0 score, b1 coins, b2 best
DIFF    EQU  >8362            ; difficulty tier 0..7 (= lanes crossed / 16)
GENL    EQU  >8364            ; next world lane index to generate
RUNTYP  EQU  >8366            ; lane-run generator: current type
RUNLEN  EQU  >8368            ;                     lanes left in the run
NEWBST  EQU  >836A            ; this run set a new best (game-over flourish)
JAYPAT  EQU  >836C            ; player sprite pattern (0 idle, 4 hop, 8 flat,
SIGB    EQU  >836E            ; 12 splash); crossing-signal color, this frame
RUMBT   EQU  >8370            ; train-rumble retrigger cooldown
DTHY    EQU  >8372            ; hawk-snatch animation y (pixels, signed)
HOLD0   EQU  >8374            ; sound driver: rest frames pending, voice 0
HOLD1   EQU  >8376            ;   (these three must stay contiguous, in
HOLD2   EQU  >8378            ;   step with SNDP0..SNDP2)
RET4    EQU  >837A            ; nested-call return save (HUDUPD, over DNUMn)

* --- expansion RAM ---
* Lane ring: 16 records x 16 bytes; record of world lane L is at
* LANES + (L AND 15)*16.
*   +0  type (byte): 0 grass, 1 road, 2 river, 3 rail
*   +1  dir  (byte): 0 rightward (+x), 1 leftward (-x)
*   +2  speed (word, 12.4 fixed, signed; matches dir)
*   +4  bitmask (word): grass = bushes, river = lily pads (bit c = cell col c)
*   +6  coin col (byte, >FF = none) ... +7 variant (byte: car style/color)
*   +8  object 0 x (word, 12.4)     ... +10 object 1 x (word, 12.4)
*   +12 rail timer (word: counts down to the next train; 0 = train sweeping)
*   +14 spare (word)
LANES   EQU  >A000
SATBUF  EQU  >A100            ; sprite-attribute shadow: 25 slots x 4 bytes
ROWA    EQU  >A180            ; 32-char stream for a lane band's upper row
ROWB    EQU  >A1A0            ; 32-char stream for a lane band's lower row

* Sprite slots: 0 player, 1 hawk, 2+2s / 3+2s = the two objects of the lane
* at screen position s (s = 0 at the bottom), slot 24 = the >D0 terminator.
* Bottom lanes get the low slots, so when a mid-hop player makes five sprites
* share a scanline, the TMS9918A drops the farthest lane's car, not the near
* action (authentic four-per-line hardware behavior, biased to be invisible).

* ======================= one-time hardware setup ============================
START   LIMI 0
        LWPI >8300
* program the VDP registers (Graphics I, 16K, display on, 16x16 sprites)
        LI   R1,REGTAB
        LI   R2,16
RGL     MOVB *R1+,@VDPWA
        DEC  R2
        JNE  RGL
* clear the pattern table >0800..>0FFF (char >88 stays all-zero = asphalt)
        LI   R1,>0800
        BL   @SETWR
        LI   R2,2048
        CLR  R0
PCL     MOVB R0,@VDPWD
        DEC  R2
        JNE  PCL
* park the whole sprite plane until a mode builds its own list
        LI   R1,>0780
        BL   @SETWR
        LI   R0,>D000
        MOVB R0,@VDPWD
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
* landscape glyphs: (char, 8 bytes) records too, same loader shape
        LI   R4,LANDPT
GLP     MOVB *R4+,R0
        JEQ  GLPD
        SRL  R0,8
        SLA  R0,3
        AI   R0,>0800
        MOV  R0,R1
        BL   @SETWR
        LI   R2,8
GLPB    MOVB *R4+,@VDPWD
        DEC  R2
        JNE  GLPB
        JMP  GLP
GLPD
* sprite patterns: 13 16x16 shapes = 416 bytes straight into >1800
        LI   R1,>1800
        BL   @SETWR
        LI   R2,SPRPAT
        LI   R3,416
        BL   @VMBW
* color table: 32 groups from COLTAB
        LI   R1,>0300
        BL   @SETWR
        LI   R2,COLTAB
        LI   R3,32
        BL   @VMBW
        LI   R0,>7E57
        MOV  R0,@RNG
        CLR  @BEST
        CLR  @TICK
        CLR  @PREVK           ; scratchpad is garbage on a real console
        CLR  @SNDP0
        CLR  @SNDP1
        CLR  @SNDP2
        CLR  @HOLD0
        CLR  @HOLD1
        CLR  @HOLD2

* ============================= title screen =================================
* Big JAYWALK block letters, a two-band diorama (the jay on grass, a car on a
* road) animated with real sprites, the session best, and the two prompts.
TITLE   CLR  @GMODE
        LI   R1,>0780         ; sprites off while the screen assembles
        BL   @SETWR
        LI   R0,>D000
        MOVB R0,@VDPWD
        BL   @CLRNT           ; clear the name table
* big "JAYWALK" in block glyphs: variable-width 5-row letters (the W needs
* five columns to read as a W), kerned one column apart, rows 3..7
        LI   R4,BIGLET
        LI   R5,1             ; current screen column
BTL     MOVB *R4+,R0          ; letter width; 0 ends the table
        SRL  R0,8
        JEQ  BTDONE
        MOV  R0,R13
        CLR  R6               ; row 0..4
BTR     MOVB *R4+,R7          ; this row's bits (low `width` bits)
        SRL  R7,8
        MOV  R13,R0           ; leftmost column's mask = 1 << (width-1)
        DEC  R0
        LI   R10,1
        MOV  R0,R0
        JEQ  BTM1
        SLA  R10,0
BTM1    CLR  R8               ; column within the letter
BTC     COC  R10,R7           ; is this cell filled?
        JNE  BTCN
        MOV  R6,R1            ; name = (3+row)*32 + col
        AI   R1,3
        SLA  R1,5
        A    R5,R1
        A    R8,R1
        BL   @SETWR
        LI   R0,>E000         ; solid block glyph (light blue, like the jay)
        MOVB R0,@VDPWD
BTCN    SRL  R10,1
        INC  R8
        C    R8,R13
        JNE  BTC
        INC  R6
        CI   R6,5
        JNE  BTR
        A    R13,R5           ; advance by width + 1 (the kerning gap)
        INC  R5
        JMP  BTL
BTDONE
* subtitle and prompts
        LI   R1,>0124         ; "A TINY JAY VS THE WORLD" (row 9, col 4)
        BL   @SETWR
        LI   R2,TSUB
        LI   R3,23
        BL   @VMBW
        LI   R1,>0245         ; "PRESS ANY KEY TO PLAY" (row 18, col 5)
        BL   @SETWR
        LI   R2,TPRESS
        LI   R3,21
        BL   @VMBW
        LI   R1,>0287         ; "H OR AID FOR HELP" (row 20, col 7)
        BL   @SETWR
        LI   R2,THELP
        LI   R3,17
        BL   @VMBW
* the diorama: a grass band (rows 12-13) over a road band (rows 14-15)
        LI   R5,>0180         ; row 12, col 0
        BL   @TGRROW
        LI   R5,>01A0         ; row 13
        BL   @TGRROW
        LI   R1,>01C0         ; row 14: the road's dashed center line
        BL   @SETWR
        LI   R2,32
        LI   R0,>8900
TRD1    MOVB R0,@VDPWD
        DEC  R2
        JNE  TRD1
        LI   R1,>01E0         ; row 15: plain asphalt
        BL   @SETWR
        LI   R2,32
        LI   R0,>8800
TRD2    MOVB R0,@VDPWD
        DEC  R2
        JNE  TRD2
* the session best, once a run has been played
        MOV  @BEST,R0
        JEQ  TNOB
        LI   R1,>02CB         ; "BEST" (row 22, col 11)
        BL   @SETWR
        LI   R2,TBEST
        LI   R3,4
        BL   @VMBW
        MOV  @BEST,R5
        LI   R1,>02D0         ; its five digits (row 22, col 16)
        BL   @DNUM5
TNOB
        LI   R0,SFXTUNE       ; the title jingle (two-voice)
        MOV  R0,@SNDP0
        LI   R0,SFXTUN2
        MOV  R0,@SNDP1
        CLR  @HAWKX           ; title car x
* title loop: animate the diorama; H/AID = help, any other key starts.
* Wait for all keys released first so a held key can't relaunch instantly.
TWU     BL   @ANYKEY
        CI   R0,0
        JNE  TWU
TWD     BL   @WAITVB
        BL   @TANIM
        BL   @SNDTCK
        BL   @HELPK           ; H or AID -> the help screen
        CI   R0,0
        JNE  SHOWHLP
        BL   @ANYKY2          ; any non-modifier key -> a new game
        CI   R0,0
        JEQ  TWD
TWR     BL   @WAITVB
        BL   @TANIM
        BL   @SNDTCK
        BL   @ANYKEY
        CI   R0,0
        JNE  TWR
        B    @GINIT
* H or AID at the title -> draw the help screen, wait, back to the title
SHOWHLP BL   @ANYKEY          ; wait for the help key to release
        CI   R0,0
        JNE  SHOWHLP
        BL   @HELP
SHWD    BL   @WAITVB
        BL   @SNDTCK
        BL   @ANYKEY
        CI   R0,0
        JEQ  SHWD
SHWR    BL   @WAITVB
        BL   @SNDTCK
        BL   @ANYKEY
        CI   R0,0
        JNE  SHWR
        B    @TITLE

* TGRROW: one 32-char row of title grass (checkered tufts) at name addr R5.
* uses R14 save (calls SETWR).
TGRROW  MOV  R11,R14
        MOV  R5,R1
        BL   @SETWR
        LI   R2,16
TGRL    LI   R0,>8000         ; grass-a, grass-b, alternating
        MOVB R0,@VDPWD
        LI   R0,>8100
        MOVB R0,@VDPWD
        DEC  R2
        JNE  TGRL
        B    *R14

* TANIM: one frame of title life — tick the clock, hop the jay in place on
* the grass band, drive the car across the road band. Writes the 3-sprite
* attribute list directly (title has no SATBUF traffic). uses R14 save.
TANIM   MOV  R11,R14
        INC  @TICK
        LI   R1,>0780
        BL   @SETWR
* jay: sits on the grass band (rows 12-13 = y 96), bobbing with a hop cycle
        MOV  @TICK,R3
        SRL  R3,4
        ANDI R3,>0003         ; 4-phase cycle
        LI   R0,>5F00         ; y = 96-1
        LI   R2,>0000         ; idle pattern
        CI   R3,1
        JNE  TAN1
        LI   R0,>5D00         ; a 2 px bounce on the up phases
        LI   R2,>0400         ; hop pattern
TAN1    CI   R3,3
        JNE  TAN2
        LI   R0,>5D00
        LI   R2,>0400
TAN2    MOVB R0,@VDPWD        ; sprite 0: Y
        LI   R0,>3C00         ; X = 60
        MOVB R0,@VDPWD
        MOVB R2,@VDPWD        ; pattern (0 or 4)
        LI   R0,>0500         ; light blue
        MOVB R0,@VDPWD
* car: crosses the road band (rows 14-15 = y 112) at 2 px/frame
        INCT @HAWKX
        MOV  @HAWKX,R0
        ANDI R0,>00FF
        MOV  R0,R2
        LI   R0,>6F00         ; sprite 1: Y = 112-1
        MOVB R0,@VDPWD
        SWPB R2
        MOVB R2,@VDPWD        ; X = the wrapping counter
        LI   R0,>1000         ; pattern 16 = sedan, rightward
        MOVB R0,@VDPWD
        LI   R0,>0800         ; medium red
        MOVB R0,@VDPWD
        LI   R0,>D000         ; terminator
        MOVB R0,@VDPWD
        B    *R14

* ============================ start a new game ==============================
GINIT   MOV  @RNG,R0          ; fold the title dwell time into the RNG so
        XOR  @TICK,R0         ; every run gets a fresh world (tests hold the
        MOV  R0,@RNG          ; frame count fixed and stay deterministic)
        JNE  GIRK
        LI   R0,>7E57         ; xorshift must never be zero
        MOV  R0,@RNG
GIRK    BL   @CLRNT
        LI   R0,1
        MOV  R0,@GMODE
        CLR  @VBOT
        LI   R0,2
        MOV  R0,@PLANE        ; spawn on the third lane...
        MOV  R0,@FROMLN
        LI   R0,>0800         ; ...at cell col 8 (x = 128, 12.4 fixed)
        MOV  R0,@PX
        MOV  R0,@TARGX
        CLR  @HOPT
        CLR  @HOPDIR
        CLR  @RIDE
        CLR  @SCORE
        CLR  @COINS
        CLR  @LCROSS
        CLR  @DEAD
        CLR  @HAWKT
        CLR  @HAWKON
        CLR  @SCRLRQ
        CLR  @DIFF
        CLR  @NEWBST
        CLR  @JAYPAT
        CLR  @RUNLEN
        CLR  @RUMBT
        CLR  @PREVK
        CLR  @KNEW
        CLR  @SNDP0
        CLR  @SNDP1
        CLR  @SNDP2
        CLR  @HOLD0
        CLR  @HOLD1
        CLR  @HOLD2
* generate the opening world: lanes 0..3 are always safe grass, then random
        CLR  R0
        MOV  R0,@GENL
GIW     BL   @GENLANE         ; generates lane GENL, increments it
        MOV  @GENL,R0
        CI   R0,13            ; lanes 0..12 exist (visible 0..10 + lookahead)
        JLT  GIW
* paint everything: HUD labels, the full playfield, the sprite list
        LI   R1,>0000         ; "SCORE" (row 0, col 0)
        BL   @SETWR
        LI   R2,TSCORE
        LI   R3,5
        BL   @VMBW
        LI   R1,>000C         ; "COIN" (row 0, col 12)
        BL   @SETWR
        LI   R2,TCOIN
        LI   R3,4
        BL   @VMBW
        LI   R1,>0014         ; "BEST" (row 0, col 20)
        BL   @SETWR
        LI   R2,TBEST
        LI   R3,4
        BL   @VMBW
        LI   R0,7             ; draw all three numbers once
        MOV  R0,@DIRTYF
        BL   @HUDUPD
        BL   @DRAWALL
        BL   @SATBLD
        BL   @SATFLSH
        LI   R0,SFXGO         ; the three-note "go!"
        MOV  R0,@SNDP0
        B    @MAIN

* ================================ main loop =================================
* One frame: flush last frame's sprite shadow in the vblank window, repaint
* the world if a scroll asked for it (top-down, outrunning the beam), HUD
* deltas, water shimmer, sound; then input -> player -> lanes -> hawk ->
* collisions -> rebuild the sprite shadow for the next flush.
MAIN    BL   @WAITVB
        BL   @SATFLSH
        MOV  @SCRLRQ,R0
        JEQ  MNOSC
        CLR  @SCRLRQ
        BL   @DRAWALL
MNOSC   BL   @HUDUPD
        BL   @WATANM
        BL   @SNDTCK
        INC  @TICK
        MOV  @GMODE,R0
        CI   R0,2
        JEQ  MDIE
* --- playing ---
        BL   @READK
        MOV  @PREVK,R5
        MOV  R1,@PREVK
        MOV  R1,R2
        SZC  R5,R2            ; newly pressed = current AND NOT previous
        MOV  R2,@KNEW
        BL   @PLRUPD
        BL   @LNUPD
        BL   @HWKUPD
        BL   @COLCHK
        MOV  @DEAD,R0
        JEQ  MLIVE
        BL   @DIEGO
MLIVE   BL   @SATBLD
        B    @MAIN
* --- dying: the traffic keeps rolling while the jay plays its death scene ---
MDIE    BL   @LNUPD
        BL   @DIETCK
        BL   @SATBLD
        B    @MAIN

* ============================ player update =================================
* PLRUPD: advance a hop in flight (and resolve its landing), follow a ridden
* log, then read newly pressed directions and launch the next hop. uses R14.
PLRUPD  MOV  R11,R14
        MOV  @HOPT,R0
        JEQ  PLGRND
        DEC  R0               ; airborne: 2 px/frame toward TARGX
        MOV  R0,@HOPT
        MOV  @TARGX,R1
        C    R1,@PX
        JEQ  PLHY
        JLT  PLHW
        MOV  @PX,R0
        AI   R0,32
        MOV  R0,@PX
        JMP  PLHY
PLHW    MOV  @PX,R0
        AI   R0,-32
        MOV  R0,@PX
PLHY    MOV  @HOPT,R0
        JEQ  PLHY2
        B    @PLRET
PLHY2   BL   @LANDED          ; wheels down: resolve the destination cell
        B    @PLRET
* grounded: idle pattern, log drift, then input
PLGRND  CLR  @JAYPAT
        MOV  @RIDE,R0
        JEQ  PLINP
        MOV  @PLANE,R9        ; riding: x follows the log exactly
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        MOV  @RIDE,R0
        DEC  R0
        SLA  R0,1
        AI   R0,8
        A    R9,R0
        MOV  *R0,R1
        MOV  R1,@PX
        MOV  R1,@TARGX
        CI   R1,-128          ; carried past either bank -> in the drink
        JLT  PLSPL
        CI   R1,3968
        JLT  PLINP
PLSPL   LI   R0,2
        MOV  R0,@DEAD
        B    @PLRET
* input: one hop per fresh press; north wins ties, then south, west, east
PLINP   INC  @HAWKT
        MOV  @KNEW,R1
        ANDI R1,>000F
        JNE  PLINP2
        B    @PLRET
PLINP2  MOV  R1,R2
        ANDI R2,>0004
        JNE  PLN
        MOV  R1,R2
        ANDI R2,>0008
        JNE  PLS
        MOV  R1,R2
        ANDI R2,>0001
        JEQ  JPLE
        B    @PLW
JPLE    B    @PLE
PLRETA  B    @PLRET
* --- north ---
PLN     MOV  @PLANE,R3
        INC  R3
        BL   @BUSHAT
        CI   R0,0
        JNE  PLRETA
        MOV  @PLANE,R0
        MOV  R0,@FROMLN
        MOV  R3,@PLANE
        MOV  @PX,R0
        MOV  R0,@TARGX
        LI   R0,1
        MOV  R0,@HOPDIR
        BL   @HOPGO
* fresh territory: score and difficulty
        MOV  @PLANE,R0
        AI   R0,-2
        C    R0,@LCROSS
        JLE  PLNS2
        MOV  R0,@LCROSS
        LI   R1,10
        A    R1,@SCORE
        MOV  @DIRTYF,R1
        ORI  R1,>0001
        MOV  R1,@DIRTYF
        MOV  R0,R1
        SRL  R1,4             ; a difficulty step every 16 lanes, capped
        CI   R1,7
        JLE  PLNDF
        LI   R1,7
PLNDF   MOV  R1,@DIFF
* camera: scroll at hop start so the glide absorbs the world's 16 px snap
PLNS2   MOV  @PLANE,R0
        S    @VBOT,R0
        CI   R0,6
        JLT  PLRETB
        INC  @VBOT
        SETO @SCRLRQ
PLGEN   MOV  @GENL,R0         ; keep two lanes of world beyond the top edge
        MOV  @VBOT,R1
        AI   R1,13
        C    R0,R1
        JHE  PLRETB
        BL   @GENLANE
        JMP  PLGEN
* --- south (never off the bottom edge) ---
PLS     MOV  @PLANE,R3
        DEC  R3
        C    R3,@VBOT
        JLT  PLRETB
        BL   @BUSHAT
        CI   R0,0
        JNE  PLRETB
        MOV  @PLANE,R0
        MOV  R0,@FROMLN
        MOV  R3,@PLANE
        MOV  @PX,R0
        MOV  R0,@TARGX
        LI   R0,2
        MOV  R0,@HOPDIR
        BL   @HOPGO
        B    @PLRET
* --- west / east (stay on the screen) ---
PLW     MOV  @PX,R1
        CI   R1,256
        JLT  PLRETB
        AI   R1,-256
        MOV  R1,R7
        MOV  @PLANE,R3
        BL   @BUSHATX
        CI   R0,0
        JNE  PLRETB
        MOV  @PLANE,R0
        MOV  R0,@FROMLN
        MOV  R7,@TARGX
        LI   R0,3
        MOV  R0,@HOPDIR
        BL   @HOPGO
        B    @PLRET
PLE     MOV  @PX,R1
        CI   R1,3584
        JGT  PLRETB
        AI   R1,256
        MOV  R1,R7
        MOV  @PLANE,R3
        BL   @BUSHATX
        CI   R0,0
        JNE  PLRETB
        MOV  @PLANE,R0
        MOV  R0,@FROMLN
        MOV  R7,@TARGX
        LI   R0,4
        MOV  R0,@HOPDIR
        BL   @HOPGO
        B    @PLRET
PLRETB  B    @PLRET
PLRET   B    *R14

* HOPGO: commit a hop the caller has validated (PLANE/FROMLN/TARGX/HOPDIR are
* already set): start the clock, dismount, feed the hawk's patience, chirp.
* leaf; clobbers R0.
HOPGO   LI   R0,HOPFRM
        MOV  R0,@HOPT
        CLR  @RIDE
        CLR  @HAWKT
        LI   R0,4             ; wings-out pattern while airborne
        MOV  R0,@JAYPAT
        LI   R0,SFXHOP
        MOV  R0,@SNDP0
        CLR  @HOLD0
        RT

* BUSHAT / BUSHATX: is lane R3 bush-blocked at x = @PX / x = R7 (12.4)?
* R0 = 1 blocked, 0 open; only grass lanes block. leaf; clobbers R0,R1,R2,R7,R9.
BUSHAT  MOV  @PX,R7
BUSHATX MOV  R3,R9
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        MOVB *R9,R0
        SRL  R0,8
        JNE  BAOPEN
        MOV  R7,R2
        AI   R2,128           ; nearest cell col = (x + 8 px) in cells
        SRL  R2,8
        MOV  R2,R0
        LI   R1,1
        MOV  R0,R0            ; col 0: a 0 shift count would mean 16
        JEQ  BAB0
        SLA  R1,0
BAB0    MOV  @4(R9),R2
        COC  R1,R2
        JNE  BAOPEN
        LI   R0,1
        RT
BAOPEN  CLR  R0
        RT

* LANDED: a hop just ended. Grass/road/rail snap to the cell grid and may
* yield a coin; a river demands footing — a log to ride, a lily pad, or in
* you go. Saves its return in RET3 (called under PLRUPD's R14).
LANDED  MOV  R11,@RET3
        MOV  @PLANE,R9
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        MOVB *R9,R0
        SRL  R0,8
        CI   R0,2
        JEQ  LDRIV
        MOV  @PX,R2           ; snap x to the 16 px grid
        AI   R2,128
        SRL  R2,8
        MOV  R2,R1
        SLA  R1,8
        MOV  R1,@PX
        MOV  R1,@TARGX
        MOVB @6(R9),R0        ; standing on the coin cell?
        SRL  R0,8
        C    R0,R2
        JNE  LDONE
        LI   R0,>FF00         ; claim it
        MOVB R0,@6(R9)
        INC  @COINS
        LI   R0,25
        A    R0,@SCORE
        MOV  @DIRTYF,R0
        ORI  R0,>0003
        MOV  R0,@DIRTYF
        SETO @SCRLRQ          ; the repaint wipes it off the screen
        LI   R0,SFXCOIN
        MOV  R0,@SNDP0
        CLR  @HOLD0
LDONE   MOV  @RET3,R11
        RT
* river: within 10 px of a log -> ride it
LDRIV   MOV  @8(R9),R1
        MOV  R1,R2
        S    @PX,R2
        ABS  R2
        CI   R2,160
        JGT  LDR2
        LI   R0,1
        MOV  R0,@RIDE
        MOV  R1,@PX
        MOV  R1,@TARGX
        JMP  LDONE
LDR2    MOV  @10(R9),R1
        MOV  R1,R2
        S    @PX,R2
        ABS  R2
        CI   R2,160
        JGT  LDSTN
        LI   R0,2
        MOV  R0,@RIDE
        MOV  R1,@PX
        MOV  R1,@TARGX
        JMP  LDONE
* a lily pad underfoot?
LDSTN   MOV  @PX,R2
        AI   R2,128
        SRL  R2,8
        MOV  R2,R0
        LI   R1,1
        MOV  R0,R0
        JEQ  LDS0
        SLA  R1,0
LDS0    MOV  @4(R9),R2
        COC  R1,R2
        JNE  LDSPL
        MOV  @PX,R2           ; solid: snap onto the pad
        AI   R2,128
        SRL  R2,8
        SLA  R2,8
        MOV  R2,@PX
        MOV  R2,@TARGX
        JMP  LDONE
LDSPL   LI   R0,2             ; nothing under our feet
        MOV  R0,@DEAD
        JMP  LDONE

* ============================ lane update ===================================
* LNUPD: advance every visible lane: cars and logs drift and wrap the track;
* rail lanes count down, flash and ring, then send the express through.
* Also decides this frame's crossing-signal lamp color. uses R14 save.
LNUPD   MOV  R11,R14
        LI   R0,>6100         ; lamp idle: dim dark red
        MOV  R0,@SIGB
        MOV  @VBOT,R6
        LI   R7,11
LNL     MOV  R6,R9
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        MOVB *R9,R0
        SRL  R0,8
        CI   R0,1
        JEQ  LNMOV
        CI   R0,2
        JEQ  LNMOV
        CI   R0,3
        JEQ  LNRAIL
        B    @LNNXT
* road/river: both objects advance by the signed speed, wrapping the track
LNMOV   MOV  @2(R9),R1
        MOV  @8(R9),R2
        A    R1,R2
        CI   R2,TRKMAX
        JLT  LNMA1
        AI   R2,-TRKWRAP
LNMA1   CI   R2,TRKMIN
        JGT  LNMA2
        AI   R2,TRKWRAP
LNMA2   MOV  R2,@8(R9)
        MOV  @10(R9),R2
        A    R1,R2
        CI   R2,TRKMAX
        JLT  LNMB1
        AI   R2,-TRKWRAP
LNMB1   CI   R2,TRKMIN
        JGT  LNMB2
        AI   R2,TRKWRAP
LNMB2   MOV  R2,@10(R9)
        B    @LNNXT
* rail: counting down / warning / sweeping
LNRAIL  MOV  @12(R9),R1
        JEQ  LNSWP
        DEC  R1
        MOV  R1,@12(R9)
        JEQ  LNSPWN
        CI   R1,90            ; the last second and a half: flash + bell
        JHE  LNNXT
        MOV  @TICK,R0
        ANDI R0,>0008
        JEQ  LNRF1
        LI   R0,>8100         ; bright red
        MOV  R0,@SIGB
        JMP  LNRF2
LNRF1   LI   R0,>6100
        MOV  R0,@SIGB
LNRF2   MOV  R1,R0
        ANDI R0,>000F
        JNE  LNNXT
        LI   R0,SFXBELL
        MOV  R0,@SNDP1
        CLR  @HOLD1
        JMP  LNNXT
* the countdown just hit zero: the train enters from the far side
LNSPWN  MOVB @1(R9),R0
        SRL  R0,8
        JNE  LNSPL
        LI   R2,TRBEG
        JMP  LNSP2
LNSPL   LI   R2,TREND
LNSP2   MOV  R2,@8(R9)
        CLR  @12(R9)
        LI   R0,SFXHORNA      ; the two-chime horn
        MOV  R0,@SNDP0
        CLR  @HOLD0
        LI   R0,SFXHORNB
        MOV  R0,@SNDP1
        CLR  @HOLD1
        JMP  LNNXT
* sweeping: engine plus a boxcar in tow, rumble on the noise channel
LNSWP   LI   R0,>8100
        MOV  R0,@SIGB
        MOV  @2(R9),R1
        MOV  @8(R9),R2
        A    R1,R2
        MOV  R2,@8(R9)
        MOV  R2,R3
        CI   R1,0
        JLT  LNSWL
        AI   R3,-256          ; the wagon trails the engine by 16 px
        JMP  LNSW2
LNSWL   AI   R3,256
LNSW2   MOV  R3,@10(R9)
        MOV  @RUMBT,R0
        JNE  LNSWR
        LI   R0,SFXRUMB
        MOV  R0,@SNDP2
        CLR  @HOLD2
        LI   R0,6
        MOV  R0,@RUMBT
LNSWR   DEC  @RUMBT
        MOV  @8(R9),R2        ; both cars clear of the world? rearm
        CI   R2,TREND
        JGT  LNSDON
        CI   R2,TRBEG
        JLT  LNSDON
        JMP  LNNXT
LNSDON  BL   @RNGET
        ANDI R0,>01FF
        AI   R0,300
        MOV  R0,@12(R9)
        LI   R0,>2000         ; park the sprites until then
        MOV  R0,@8(R9)
        MOV  R0,@10(R9)
        LI   R0,SFXNOFF
        MOV  R0,@SNDP2
        CLR  @HOLD2
LNNXT   INC  R6
        DEC  R7
        JEQ  LNDONE
        B    @LNL
LNDONE  B    *R14

* ============================== the hawk ====================================
* HWKUPD: count idleness; screech a warning; then a committed dive at the
* player's position, drifting toward their column. uses R14 save.
HWKUPD  MOV  R11,R14
        MOV  @HAWKON,R0
        JNE  HWDIVE
        MOV  @HAWKT,R0
        CI   R0,HAWKWRN
        JNE  HWCHK2
        LI   R0,SFXHAWK
        MOV  R0,@SNDP1
        CLR  @HOLD1
HWCHK2  CI   R0,HAWKATK
        JLT  HWRET
        LI   R0,1             ; talons out
        MOV  R0,@HAWKON
        LI   R0,-16
        MOV  R0,@HAWKY
        MOV  @PX,R0
        SRL  R0,4
        MOV  R0,@HAWKX
        LI   R0,SFXHAWK
        MOV  R0,@SNDP1
        CLR  @HOLD1
        JMP  HWRET
HWDIVE  MOV  @HAWKY,R0
        AI   R0,3
        MOV  R0,@HAWKY
        MOV  @PX,R1
        SRL  R1,4
        C    R1,@HAWKX
        JEQ  HWCC
        JLT  HWXL
        INCT @HAWKX
        JMP  HWCC
HWXL    DECT @HAWKX
HWCC    BL   @PLY
        MOV  R0,R3
        S    @HAWKY,R3
        ABS  R3
        CI   R3,8
        JGT  HWRET
        MOV  @PX,R3
        SRL  R3,4
        S    @HAWKX,R3
        ABS  R3
        CI   R3,8
        JGT  HWRET
        LI   R0,4             ; snatched
        MOV  R0,@DEAD
HWRET   B    *R14

* ============================= collisions ===================================
* COLCHK: cars and trains vs the jay — the lane underfoot always, plus the
* departure lane while airborne (the sprite spans both). uses R14 save.
COLCHK  MOV  R11,R14
        MOV  @DEAD,R0
        JNE  CCRET
        MOV  @PLANE,R3
        BL   @CCLANE
        MOV  @DEAD,R0
        JNE  CCRET
        MOV  @HOPT,R0
        JEQ  CCRET
        MOV  @FROMLN,R3
        BL   @CCLANE
CCRET   B    *R14

* CCLANE: lane R3's two objects against PX: roads kill within 11 px, a
* sweeping train within 13. leaf; clobbers R0,R1,R6,R9.
CCLANE  MOV  R3,R9
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        MOVB *R9,R0
        SRL  R0,8
        CI   R0,1
        JEQ  CCROAD
        CI   R0,3
        JNE  CCLRT
        MOV  @12(R9),R0
        JNE  CCLRT
        LI   R6,208
        LI   R0,3
        JMP  CCTEST
CCROAD  LI   R6,176
        LI   R0,1
CCTEST  MOV  @8(R9),R1
        S    @PX,R1
        ABS  R1
        C    R1,R6
        JLE  CCHIT
        MOV  @10(R9),R1
        S    @PX,R1
        ABS  R1
        C    R1,R6
        JLE  CCHIT
CCLRT   RT
CCHIT   MOV  R0,@DEAD
        RT

* ========================== death and game over =============================
* DIEGO: the moment of death — freeze control and stage the scene. uses R14.
DIEGO   MOV  R11,R14
        LI   R0,2
        MOV  R0,@GMODE
        CLR  @HOPT
        CLR  @RIDE
        LI   R0,50
        MOV  R0,@DTIMER
        MOV  @DEAD,R0
        CI   R0,1
        JNE  DGWAT
        LI   R0,8             ; flattened; horn and thud
        MOV  R0,@JAYPAT
        LI   R0,SFXHONKA
        MOV  R0,@SNDP0
        CLR  @HOLD0
        LI   R0,SFXHONKB
        MOV  R0,@SNDP1
        CLR  @HOLD1
        LI   R0,SFXTHUD
        MOV  R0,@SNDP2
        CLR  @HOLD2
        JMP  DGRET
DGWAT   CI   R0,2
        JNE  DGTRN
        LI   R0,12            ; the splash ring
        MOV  R0,@JAYPAT
        LI   R0,SFXSPLSH
        MOV  R0,@SNDP2
        CLR  @HOLD2
        JMP  DGRET
DGTRN   CI   R0,3
        JNE  DGHWK
        LI   R0,8
        MOV  R0,@JAYPAT
        LI   R0,SFXTHUD
        MOV  R0,@SNDP2
        CLR  @HOLD2
        JMP  DGRET
DGHWK   BL   @PLY             ; carried off: remember where we were grabbed
        MOV  R0,@DTHY
        LI   R0,120
        MOV  R0,@DTIMER
        LI   R0,SFXHAWK
        MOV  R0,@SNDP1
        CLR  @HOLD1
DGRET   B    *R14

* DIETCK: one frame of the death scene; then the reckoning. uses R14 save
* (and abandons it for GAMEOV on the final frame).
DIETCK  MOV  R11,R14
        MOV  @DEAD,R0
        CI   R0,4
        JNE  DTNOTH
        MOV  @DTHY,R0         ; hawk and jay climb away together
        AI   R0,-4
        MOV  R0,@DTHY
DTNOTH  DEC  @DTIMER
        JNE  DTRET
        B    @GAMEOV
DTRET   B    *R14

* ========================= sprite shadow build ==============================
* SATBLD: rebuild SATBUF: the jay (slot 0), the hawk (1), then two slots per
* visible lane, bottom lanes in the low (kept-first) slots so any four-per-
* line drop hits the far lanes. Slot 24 carries the >D0 terminator. uses R14.
SATBLD  MOV  R11,R14
        LI   R8,SATBUF
        BL   @PLY
        MOV  R0,R5
        MOV  @DEAD,R1
        CI   R1,4
        JNE  SBJ1
        MOV  @DTHY,R5         ; being carried away
SBJ1    MOV  @GMODE,R1        ; a 1 px idle bob while grounded and alive
        CI   R1,1
        JNE  SBJ3
        MOV  @HOPT,R1
        JNE  SBJ3
        MOV  @TICK,R1
        ANDI R1,>0020
        JNE  SBJ3
        DEC  R5
SBJ3    MOV  @DEAD,R1         ; sunk after the splash: slip out of sight
        CI   R1,2
        JNE  SBJ4
        MOV  @DTIMER,R1
        CI   R1,20
        JGT  SBJ4
        LI   R5,>00C1
SBJ4    DEC  R5               ; attribute Y names the line above the sprite
        SWPB R5
        MOVB R5,*R8+
        MOV  @PX,R0
        SRL  R0,4
        SWPB R0
        MOVB R0,*R8+
        MOV  @JAYPAT,R0
        SWPB R0
        MOVB R0,*R8+
        LI   R0,>0500         ; jay blue; white foam while splashing
        MOV  @JAYPAT,R1
        CI   R1,12
        JNE  SBJ5
        LI   R0,>0F00
SBJ5    MOVB R0,*R8+
* the hawk
        MOV  @HAWKON,R0
        JNE  SBH1
        BL   @SBPARK
        JMP  SBLANE
SBH1    MOV  @HAWKY,R0
        MOV  @DEAD,R1
        CI   R1,4
        JNE  SBH2
        MOV  @DTHY,R0
        AI   R0,-12           ; talons riding just above the jay
SBH2    DEC  R0
        SWPB R0
        MOVB R0,*R8+
        MOV  @HAWKX,R0
        SWPB R0
        MOVB R0,*R8+
        LI   R0,>3000         ; pattern 48
        MOVB R0,*R8+
        LI   R0,>0600         ; dark red
        MOVB R0,*R8+
* the lanes, bottom (s = 0) upward
SBLANE  CLR  R4
SBLL    MOV  @VBOT,R6
        A    R4,R6
        MOV  R6,R9
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        MOV  R4,R5            ; this band's sprite Y attribute: 175 - 16s
        SLA  R5,4
        LI   R3,175
        S    R5,R3
        MOVB *R9,R0
        SRL  R0,8
        CI   R0,1
        JEQ  SBROAD
        CI   R0,2
        JEQ  SBRIVR
        CI   R0,3
        JEQ  SBRAIL
        BL   @SBPARK          ; grass: nothing moves here
        BL   @SBPARK
        JMP  SBNEXT
SBROAD  MOVB @7(R9),R1        ; variant picks body and paint
        SRL  R1,8
        MOVB @CARCOL(R1),R7
        MOV  R1,R2
        ANDI R2,>0001
        SLA  R2,3
        AI   R2,16            ; 16 sedan / 24 truck
        MOVB @1(R9),R0
        SRL  R0,8
        SLA  R0,2
        A    R0,R2            ; +4 for the leftward version
        MOV  @8(R9),R5
        BL   @SBOBJ
        MOV  @10(R9),R5
        BL   @SBOBJ
        JMP  SBNEXT
SBRIVR  LI   R2,32            ; the log
        LI   R7,>0A00         ; dark yellow
        MOV  @8(R9),R5
        BL   @SBOBJ
        MOV  @10(R9),R5
        BL   @SBOBJ
        JMP  SBNEXT
SBRAIL  MOV  @12(R9),R0
        JEQ  SBRL1
        BL   @SBPARK          ; no train due: empty rails
        BL   @SBPARK
        JMP  SBNEXT
SBRL1   MOVB @1(R9),R0
        SRL  R0,8
        SLA  R0,2
        AI   R0,36            ; 36 rightward / 40 leftward engine
        MOV  R0,R2
        LI   R7,>0600         ; dark red engine
        MOV  @8(R9),R5
        BL   @SBOBJ
        LI   R2,44            ; the boxcar
        LI   R7,>0E00         ; gray
        MOV  @10(R9),R5
        BL   @SBOBJ
SBNEXT  INC  R4
        CI   R4,11
        JNE  SBLL
        LI   R0,>D000         ; slot 24 ends the active list
        MOVB R0,*R8+
        B    *R14

* SBOBJ: append one lane sprite: x = R5 (12.4), Y attr = R3, pattern = R2,
* color = R7's high byte (the early-clock bit is added for x < 0). Fully
* offscreen parks instead. leaf; clobbers R0,R1.
SBOBJ   MOV  R5,R0
        SRA  R0,4
        CI   R0,-15           ; x <= -16 is fully off the left edge
        JLT  SBOPK            ; (JLT, not JLE: JLE compares unsigned)
        CI   R0,255
        JGT  SBOPK
        MOV  R3,R1
        SWPB R1
        MOVB R1,*R8+
        MOV  R7,R1
        CI   R0,0
        JLT  SBOEC
        SWPB R0
        MOVB R0,*R8+
        JMP  SBOP
SBOEC   AI   R0,32            ; early clock: the VDP shifts us 32 px left
        SWPB R0
        MOVB R0,*R8+
        ORI  R1,>8000
SBOP    MOV  R2,R0
        SWPB R0
        MOVB R0,*R8+
        MOVB R1,*R8+
        RT
SBOPK   LI   R0,>C000         ; parked below the visible field
        MOVB R0,*R8+
        CLR  R0
        MOVB R0,*R8+
        MOVB R0,*R8+
        MOVB R0,*R8+
        RT

* SBPARK: append one parked sprite. leaf; clobbers R0.
SBPARK  LI   R0,>C000
        MOVB R0,*R8+
        CLR  R0
        MOVB R0,*R8+
        MOVB R0,*R8+
        MOVB R0,*R8+
        RT

* PLY: R0 = the player sprite's top edge in pixels. The camera moves at hop
* start, so a north/south hop glides in from 2 px per remaining frame away.
* leaf; clobbers R0,R1,R2.
PLY     MOV  @PLANE,R0
        S    @VBOT,R0
        SLA  R0,4
        LI   R1,176
        S    R0,R1
        MOV  @HOPT,R0
        JEQ  PLYD
        SLA  R0,1
        MOV  @HOPDIR,R2
        CI   R2,1
        JNE  PLYS
        A    R0,R1            ; north: still below the new band
        JMP  PLYD
PLYS    CI   R2,2
        JNE  PLYD
        S    R0,R1            ; south: still above it
PLYD    MOV  R1,R0
        RT

* ============================ game over =====================================
GAMEOV  LI   R0,3
        MOV  R0,@GMODE
        LI   R0,>9F00         ; all four voices silent
        MOVB R0,@SOUND
        LI   R0,>BF00
        MOVB R0,@SOUND
        LI   R0,>DF00
        MOVB R0,@SOUND
        LI   R0,>FF00
        MOVB R0,@SOUND
        CLR  @SNDP0
        CLR  @SNDP1
        CLR  @SNDP2
        CLR  @HOLD0
        CLR  @HOLD1
        CLR  @HOLD2
        LI   R1,>0780         ; sprites off; the last frame stays painted
        BL   @SETWR
        LI   R0,>D000
        MOVB R0,@VDPWD
        CLR  @NEWBST
        MOV  @SCORE,R0
        C    R0,@BEST
        JLE  GONB
        MOV  R0,@BEST
        SETO @NEWBST
* the panel: rows 7..17, cols 6..25, block border over the final scene
GONB    LI   R1,>00E6
        BL   @SETWR
        LI   R2,20
        LI   R0,>E000
GOBT    MOVB R0,@VDPWD
        DEC  R2
        JNE  GOBT
        LI   R1,>0226
        BL   @SETWR
        LI   R2,20
        LI   R0,>E000
GOBB    MOVB R0,@VDPWD
        DEC  R2
        JNE  GOBB
        LI   R4,9
        LI   R5,>0106
GOBR    MOV  R5,R1
        BL   @SETWR
        LI   R0,>E000
        MOVB R0,@VDPWD
        LI   R2,18
        LI   R0,>2000
GOBI    MOVB R0,@VDPWD
        DEC  R2
        JNE  GOBI
        LI   R0,>E000
        MOVB R0,@VDPWD
        AI   R5,32
        DEC  R4
        JNE  GOBR
* heading, the cause, the numbers
        LI   R1,>012B
        BL   @SETWR
        LI   R2,TGOVER
        LI   R3,9
        BL   @VMBW
        MOV  @DEAD,R0
        DEC  R0
        MOV  R0,R1
        SLA  R0,2
        SLA  R1,1
        A    R1,R0
        AI   R0,CAUSTB
        MOV  R0,R4
        MOV  *R4+,R1
        BL   @SETWR
        MOV  *R4+,R2
        MOV  *R4,R3
        BL   @VMBW
        LI   R1,>0188
        BL   @SETWR
        LI   R2,TSCORE
        LI   R3,5
        BL   @VMBW
        MOV  @SCORE,R5
        LI   R1,>0192
        BL   @DNUM5
        LI   R1,>01A8
        BL   @SETWR
        LI   R2,TCOIN
        LI   R3,4
        BL   @VMBW
        MOV  @COINS,R5
        LI   R1,>01B2
        BL   @DNUM5
        LI   R1,>01C8
        BL   @SETWR
        LI   R2,TLANES
        LI   R3,5
        BL   @VMBW
        MOV  @LCROSS,R5
        LI   R1,>01D2
        BL   @DNUM5
        LI   R1,>01E8
        BL   @SETWR
        LI   R2,TBEST
        LI   R3,4
        BL   @VMBW
        MOV  @BEST,R5
        LI   R1,>01F2
        BL   @DNUM5
        MOV  @NEWBST,R0
        JEQ  GONN
        LI   R1,>020B         ; "NEW BEST!" earns the cheerful arpeggio
        BL   @SETWR
        LI   R2,TNEWB
        LI   R3,9
        BL   @VMBW
        LI   R0,SFXGO
        MOV  R0,@SNDP0
        JMP  GOPK
GONN    LI   R0,SFXOVER       ; otherwise the descending sigh
        MOV  R0,@SNDP1
GOPK    LI   R1,>0269
        BL   @SETWR
        LI   R2,TANYK
        LI   R3,13
        BL   @VMBW
* all keys up, a key down, up again: back to the title
GOWU    BL   @WAITVB
        BL   @SNDTCK
        BL   @ANYKEY
        CI   R0,0
        JNE  GOWU
GOWD    BL   @WAITVB
        BL   @SNDTCK
        BL   @ANYKEY
        CI   R0,0
        JEQ  GOWD
GOWR    BL   @WAITVB
        BL   @SNDTCK
        BL   @ANYKEY
        CI   R0,0
        JNE  GOWR
        B    @TITLE

* ========================== world generation ================================
* GENLANE: write world lane GENL's record and advance GENL. Lanes 0..3 are
* the safe spawn meadow. Types come in short runs (multi-lane highways, wide
* rivers); the mix and speeds harden with DIFF. Saves its return in RET2
* (PLRUPD may call this under its own R14).
GENLANE MOV  R11,@RET2
        MOV  @GENL,R0
        MOV  R0,R9
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        INC  @GENL
        CLR  R1               ; a bare grass record
        MOV  R1,*R9
        MOV  R1,@2(R9)
        MOV  R1,@4(R9)
        MOV  R1,@8(R9)
        MOV  R1,@10(R9)
        MOV  R1,@12(R9)
        MOV  R1,@14(R9)
        LI   R1,>FF00         ; no coin
        MOVB R1,@6(R9)
        CI   R0,4
        JHE  GLGO
        B    @GLDONE
GLGO
* continue the current run, or roll a new type against the difficulty mix
        MOV  @RUNLEN,R1
        JEQ  GLNEW
        DEC  R1
        MOV  R1,@RUNLEN
        MOV  @RUNTYP,R5
        JMP  GLFILL
GLNEW   BL   @RNGET
        MOV  R0,R2
        ANDI R2,>000F
        MOV  @DIFF,R1
        SLA  R1,2
        AI   R1,GENTAB
        CLR  R5               ; grass...
        MOVB *R1+,R0
        SRL  R0,8
        C    R2,R0
        JLT  GLRUN
        INC  R5               ; ...road...
        MOVB *R1+,R0
        SRL  R0,8
        C    R2,R0
        JLT  GLRUN
        INC  R5               ; ...river...
        MOVB *R1,R0
        SRL  R0,8
        C    R2,R0
        JLT  GLRUN
        INC  R5               ; ...rail
GLRUN   MOV  R5,@RUNTYP
        CLR  R3
        CI   R5,1
        JNE  GLR2
        BL   @RNGET           ; roads: 1-2 lanes, 1-3 once DIFF >= 3
        ANDI R0,>0001
        MOV  R0,R3
        MOV  @DIFF,R1
        CI   R1,3
        JLT  GLR4
        BL   @RNGET
        ANDI R0,>0001
        A    R0,R3
        JMP  GLR4
GLR2    CI   R5,2
        JNE  GLR3
        MOV  @DIFF,R1         ; rivers: 1 lane, 1-2 once DIFF >= 2
        CI   R1,2
        JLT  GLR4
        BL   @RNGET
        ANDI R0,>0001
        MOV  R0,R3
        JMP  GLR4
GLR3    CI   R5,0
        JNE  GLR4             ; rail: always a single lane
        BL   @RNGET           ; grass: 1-2
        ANDI R0,>0001
        MOV  R0,R3
GLR4    MOV  R3,@RUNLEN
GLFILL  MOV  R5,R0
        SWPB R0
        MOVB R0,*R9
        CI   R5,1
        JNE  GLF2
        B    @GLROAD
GLF2    CI   R5,2
        JNE  GLF3
        B    @GLRIVR
GLF3    CI   R5,3
        JNE  GLF4
        B    @GLRAIL
GLF4
* --- grass: 2..5 bushes, maybe a coin on an open cell ---
        BL   @RNGET
        ANDI R0,>0003
        AI   R0,2
        MOV  R0,R3
        CLR  R4
GLGB    BL   @RNGET
        ANDI R0,>000F
        LI   R1,1
        MOV  R0,R0
        JEQ  GLGB1
        SLA  R1,0
GLGB1   SOC  R1,R4
        DEC  R3
        JNE  GLGB
        MOV  R4,@4(R9)
        BL   @RNGET
        ANDI R0,>0003
        JEQ  GLGC0
        B    @GLDONE
GLGC0   BL   @RNGET
        ANDI R0,>000F
        MOV  R0,R2
        LI   R1,1
        MOV  R0,R0
        JEQ  GLGC1
        SLA  R1,0
GLGC1   COC  R1,R4            ; never under a bush
        JNE  GLGC2
        B    @GLDONE
GLGC2   SLA  R2,8
        MOVB R2,@6(R9)
        B    @GLDONE
* --- road: direction, speed, body/paint variant, two spaced cars ---
GLROAD  BL   @RNGET
        MOV  R0,R4
        ANDI R0,>0001
        MOV  R0,R3
        SWPB R0
        MOVB R0,@1(R9)
        MOV  R4,R0            ; speed 6 + (rnd AND 7) + 2*DIFF, sign = dir
        SRL  R0,4
        ANDI R0,>0007
        AI   R0,6
        MOV  @DIFF,R1
        SLA  R1,1
        A    R1,R0
        MOV  R3,R3
        JEQ  GLRSP
        NEG  R0
GLRSP   MOV  R0,@2(R9)
        MOV  R4,R0
        SRL  R0,8
        ANDI R0,>0003
        SWPB R0
        MOVB R0,@7(R9)
        BL   @RNGET
        ANDI R0,>0FFF
        AI   R0,TRKMIN
        MOV  R0,@8(R9)
        AI   R0,CARGAP
        CI   R0,TRKMAX
        JLT  GLRO1
        AI   R0,-TRKWRAP
GLRO1   MOV  R0,@10(R9)
        BL   @RNGET
        ANDI R0,>0003
        JEQ  GLRC0
        B    @GLDONE
GLRC0   BL   @RNGET           ; a coin on the asphalt: brave points
        ANDI R0,>000F
        SLA  R0,8
        MOVB R0,@6(R9)
        B    @GLDONE
* --- river: direction, drift, two logs, lily pads while the going is easy ---
GLRIVR  BL   @RNGET
        MOV  R0,R4
        ANDI R0,>0001
        MOV  R0,R3
        SWPB R0
        MOVB R0,@1(R9)
        MOV  R4,R0            ; drift 4 + (rnd AND 3) + DIFF
        SRL  R0,4
        ANDI R0,>0003
        AI   R0,4
        A    @DIFF,R0
        MOV  R3,R3
        JEQ  GLVSP
        NEG  R0
GLVSP   MOV  R0,@2(R9)
        BL   @RNGET
        ANDI R0,>0FFF
        AI   R0,TRKMIN
        MOV  R0,@8(R9)
        AI   R0,CARGAP
        CI   R0,TRKMAX
        JLT  GLRV1
        AI   R0,-TRKWRAP
GLRV1   MOV  R0,@10(R9)
        MOV  @DIFF,R0         ; pads: two below DIFF 2, one below 4, then none
        CI   R0,4
        JHE  GLDONE
        CI   R0,2
        JHE  GLV1P
        BL   @RNGET
        ANDI R0,>000F
        LI   R1,1
        MOV  R0,R0
        JEQ  GLVB1
        SLA  R1,0
GLVB1   MOV  R1,R4
        BL   @RNGET
        ANDI R0,>000F
        LI   R1,1
        MOV  R0,R0
        JEQ  GLVB2
        SLA  R1,0
GLVB2   SOC  R1,R4
        MOV  R4,@4(R9)
        JMP  GLDONE
GLV1P   BL   @RNGET
        ANDI R0,>000F
        LI   R1,1
        MOV  R0,R0
        JEQ  GLVB3
        SLA  R1,0
GLVB3   MOV  R1,@4(R9)
        JMP  GLDONE
* --- rail: a timer to the first train, parked sprites, maybe a daring coin ---
GLRAIL  BL   @RNGET
        MOV  R0,R3
        ANDI R3,>0001
        MOV  R3,R0
        SWPB R0
        MOVB R0,@1(R9)
        LI   R0,TRSPD
        MOV  R3,R3
        JEQ  GLTSP
        NEG  R0
GLTSP   MOV  R0,@2(R9)
        BL   @RNGET
        ANDI R0,>00FF
        AI   R0,240
        MOV  R0,@12(R9)
        LI   R0,>2000
        MOV  R0,@8(R9)
        MOV  R0,@10(R9)
        BL   @RNGET
        ANDI R0,>0003
        JNE  GLDONE
        BL   @RNGET
        ANDI R0,>000F
        SLA  R0,8
        MOVB R0,@6(R9)
GLDONE  MOV  @RET2,R11
        RT

* ========================= playfield painting ===============================
* DRAWALL: repaint all eleven bands from the lane ring, top band first — the
* repaint races the beam downward and outruns it (a band paints faster than
* the beam crosses one), so the post-vblank call never shows a torn frame.
* Saves its return in RET3 (DRAWLANE uses R14).
DRAWALL MOV  R11,@RET3
        LI   R13,10
DALL    MOV  R13,R0
        BL   @DRAWLANE
        DEC  R13
        JLT  DAEND
        JMP  DALL
DAEND   MOV  @RET3,R11
        RT

* DRAWLANE: paint screen band R0 (0 = bottom): build the two 32-char row
* streams in RAM from the lane record, then blast them to rows 22-2s and
* 23-2s. uses R14 save; preserves R13; clobbers R0-R9.
DRAWLANE MOV R11,R14
        MOV  R0,R4
        MOV  @VBOT,R6
        A    R4,R6
        MOV  R6,R9
        ANDI R9,>000F
        SLA  R9,4
        AI   R9,LANES
        MOVB *R9,R0
        SRL  R0,8
        CI   R0,1
        JEQ  DLROAD
        CI   R0,2
        JEQ  DLRIVR
        CI   R0,3
        JNE  DLGRS
        B    @DLRAIL
DLGRS
* --- grass: checkered tufts, bushes from the bitmask ---
        LI   R2,ROWA
        LI   R3,ROWB
        MOV  @4(R9),R5
        LI   R7,1
        CLR  R6
DLGL    COC  R7,R5
        JNE  DLGT
        LI   R0,>8283
        MOV  R0,*R2+
        LI   R0,>8485
        MOV  R0,*R3+
        JMP  DLGN
DLGT    MOV  R6,R0
        ANDI R0,>0001
        JNE  DLGB
        LI   R0,>8081
        MOV  R0,*R2+
        LI   R0,>8180
        MOV  R0,*R3+
        JMP  DLGN
DLGB    LI   R0,>8180
        MOV  R0,*R2+
        LI   R0,>8081
        MOV  R0,*R3+
DLGN    SLA  R7,1
        INC  R6
        CI   R6,16
        JNE  DLGL
        B    @DLCOIN
* --- road: plain asphalt; a dashed center line under a road neighbor ---
DLROAD  MOV  @VBOT,R0
        A    R4,R0
        INC  R0
        MOV  R0,R1
        ANDI R1,>000F
        SLA  R1,4
        AI   R1,LANES
        MOVB *R1,R0
        SRL  R0,8
        CI   R0,1
        JNE  DLRP
        LI   R0,>8989
        JMP  DLRF
DLRP    LI   R0,>8888
DLRF    LI   R2,ROWA
        LI   R3,16
DLRL    MOV  R0,*R2+
        DEC  R3
        JNE  DLRL
        LI   R0,>8888
        LI   R2,ROWB
        LI   R3,16
DLRL2   MOV  R0,*R2+
        DEC  R3
        JNE  DLRL2
        B    @DLCOIN
* --- river: rippling water, lily pads from the bitmask ---
DLRIVR  LI   R2,ROWA
        LI   R3,ROWB
        MOV  @4(R9),R5
        LI   R7,1
        CLR  R6
DLVL    COC  R7,R5
        JNE  DLVW
        LI   R0,>9899
        MOV  R0,*R2+
        LI   R0,>9A9B
        MOV  R0,*R3+
        JMP  DLVN
DLVW    MOV  R6,R0
        ANDI R0,>0001
        JNE  DLVB
        LI   R0,>9091
        MOV  R0,*R2+
        LI   R0,>9190
        MOV  R0,*R3+
        JMP  DLVN
DLVB    LI   R0,>9190
        MOV  R0,*R2+
        LI   R0,>9091
        MOV  R0,*R3+
DLVN    SLA  R7,1
        INC  R6
        CI   R6,16
        JNE  DLVL
        B    @DLBLIT
* --- rail: track, ties, and the crossing signal at cell col 1 ---
DLRAIL  LI   R0,>8A8A
        LI   R2,ROWA
        LI   R3,16
DLTL    MOV  R0,*R2+
        DEC  R3
        JNE  DLTL
        LI   R0,>8B8B
        LI   R2,ROWB
        LI   R3,16
DLTL2   MOV  R0,*R2+
        DEC  R3
        JNE  DLTL2
        LI   R0,>B08A
        MOV  R0,@ROWA+2
        LI   R0,>8C8B
        MOV  R0,@ROWB+2
        B    @DLCOIN
* --- a coin overlays its cell (grass tones or black-background tones) ---
DLCOIN  MOVB @6(R9),R0
        SRL  R0,8
        CI   R0,>00FF
        JEQ  DLBLIT
        SLA  R0,1
        MOV  R0,R5
        MOVB *R9,R0
        SRL  R0,8
        JNE  DLCR
        LI   R1,>A0A1
        MOV  R1,@ROWA(R5)
        LI   R1,>A2A3
        MOV  R1,@ROWB(R5)
        JMP  DLBLIT
DLCR    LI   R1,>A8A9
        MOV  R1,@ROWA(R5)
        LI   R1,>AAAB
        MOV  R1,@ROWB(R5)
* --- blast both streams to the name table ---
DLBLIT  MOV  R4,R1
        SLA  R1,6
        LI   R2,704
        S    R1,R2
        MOV  R2,R1
        BL   @SETWR
        LI   R2,ROWA
        LI   R3,32
        BL   @VMBW
        MOV  R4,R1
        SLA  R1,6
        LI   R2,736
        S    R1,R2
        MOV  R2,R1
        BL   @SETWR
        LI   R2,ROWB
        LI   R3,32
        BL   @VMBW
        B    *R14

* HUDUPD: redraw only the HUD numbers whose values changed, and refresh the
* crossing signal's color-table byte (group 22). Saves its return in RET4:
* DNUM5/DNUM2 use R14, so this must not (the classic no-stack trap).
HUDUPD  MOV  R11,@RET4
        MOV  @DIRTYF,R0
        ANDI R0,>0001
        JEQ  HUD2
        MOV  @SCORE,R5
        LI   R1,>0006
        BL   @DNUM5
HUD2    MOV  @DIRTYF,R0
        ANDI R0,>0002
        JEQ  HUD3
        MOV  @COINS,R5
        LI   R1,>0011
        BL   @DNUM2
HUD3    MOV  @DIRTYF,R0
        ANDI R0,>0004
        JEQ  HUD4
        MOV  @BEST,R5
        LI   R1,>0019
        BL   @DNUM5
HUD4    CLR  @DIRTYF
        LI   R1,>0316
        BL   @SETWR
        MOVB @SIGB,@VDPWD
        MOV  @RET4,R11
        RT

* WATANM: every 16 frames swap the two water glyphs' patterns — the whole
* river shimmers for the price of 16 pattern bytes. uses R14 save.
WATANM  MOV  R11,R14
        MOV  @TICK,R0
        ANDI R0,>000F
        JNE  WARET
        LI   R2,WATP0
        MOV  @TICK,R0
        ANDI R0,>0010
        JEQ  WAT1
        LI   R2,WATP1
WAT1    LI   R1,>0C80
        BL   @SETWR
        LI   R3,16
        BL   @VMBW
WARET   B    *R14

* SATFLSH: 100 bytes of sprite shadow into the attribute table, inside the
* vertical-blank window. uses R14 save.
SATFLSH MOV  R11,R14
        LI   R1,>0780
        BL   @SETWR
        LI   R2,SATBUF
        LI   R3,100
        BL   @VMBW
        B    *R14

* ============================ sound driver ==================================
* Three voices, one stream pointer each: voice 0 = tone channel 0, voice 1 =
* tone channel 1, voice 2 = the noise channel. A stream is a sequence of
* frame packets: a count 1..127 followed by that many raw SN76489 bytes; a
* count with the high bit set (>81..>FF) is a rest for (count AND 127)
* frames — never >80 alone; a zero count ends the stream (each effect
* silences its own channel in its final packet). Triggering an effect =
* storing its table address in SNDPn and clearing HOLDn. leaf.
SNDTCK  LI   R2,SNDP0
        LI   R4,HOLD0
        LI   R3,3
SNTV    MOV  *R2,R1
        JEQ  SNTN
        MOV  *R4,R0
        JEQ  SNTGO
        DEC  *R4
        JMP  SNTN
SNTGO   MOVB *R1+,R0
        SRL  R0,8
        JEQ  SNTEND
        MOV  R0,R5
        ANDI R5,>0080
        JEQ  SNTW
        ANDI R0,>007F
        DEC  R0
        MOV  R0,*R4
        MOV  R1,*R2
        JMP  SNTN
SNTW    MOVB *R1+,@SOUND
        DEC  R0
        JNE  SNTW
        MOV  R1,*R2
        JMP  SNTN
SNTEND  CLR  *R2
SNTN    INCT R2
        INCT R4
        DEC  R3
        JNE  SNTV
        RT

* ============================== the help screen =============================
* HELP: story, controls, field guide (with real glyph icons), scoring. The
* caller waits for the dismissing key. Saves its return in RET2.
HELP    MOV  R11,@RET2
        BL   @CLRNT
        LI   R1,>0780         ; park the title diorama's sprites
        BL   @SETWR
        LI   R0,>D000
        MOVB R0,@VDPWD
        LI   R4,HLPTAB
HLPL    MOV  *R4+,R1
        CI   R1,0
        JEQ  HLPB
        BL   @SETWR
        MOV  *R4+,R2
        MOV  *R4+,R3
        BL   @VMBW
        JMP  HLPL
* field-guide icons, full 16x16 (2x2 chars), from the (addr, char pair) table
HLPB    LI   R4,HICONS
HICL    MOV  *R4+,R1
        CI   R1,0
        JEQ  HICD
        BL   @SETWR
        MOVB *R4+,@VDPWD
        MOVB *R4+,@VDPWD
        JMP  HICL
HICD    MOV  @RET2,R11
        RT

* ============================== small helpers ===============================
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

* CLRNT: clear the 768-byte name table to spaces. uses R14 save.
CLRNT   MOV  R11,R14
        LI   R1,>0000
        BL   @SETWR
        LI   R2,768
        LI   R0,>2000
CNL     MOVB R0,@VDPWD
        DEC  R2
        JNE  CNL
        B    *R14

* READK -> R1 mask: b0 west, b1 east, b2 north, b3 south. The E/S/D/X
* diamond and TI joystick 1 (the host arrow keys) both work.
* clobbers R0,R2,R3,R12.
READK   CLR  R1
        LI   R0,>0100         ; column 1: S (row 5) = west, X (row 7) = south
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>2000
        JNE  RKX
        ORI  R1,>0001
RKX     MOV  R2,R3
        ANDI R3,>8000
        JNE  RKC2
        ORI  R1,>0008
RKC2    LI   R0,>0200         ; column 2: E (row 6) = north, D (row 5) = east
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>4000
        JNE  RKD
        ORI  R1,>0004
RKD     MOV  R2,R3
        ANDI R3,>2000
        JNE  RKJ
        ORI  R1,>0002
RKJ     LI   R0,>0600         ; column 6: joystick 1
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        MOV  R2,R3
        ANDI R3,>0200
        JNE  RJ1
        ORI  R1,>0001
RJ1     MOV  R2,R3
        ANDI R3,>0400
        JNE  RJ2
        ORI  R1,>0002
RJ2     MOV  R2,R3
        ANDI R3,>1000
        JNE  RJ3
        ORI  R1,>0004
RJ3     MOV  R2,R3
        ANDI R3,>0800
        JNE  RJ4
        ORI  R1,>0008
RJ4     RT

* ANYKEY -> R0 = 1 if any key/stick input is down (columns 0..7), else 0.
* clobbers R0,R1,R2,R12.
ANYKEY  CLR  R1
AKL     MOV  R1,R0
        SWPB R0
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        CI   R2,>FF00
        JNE  AKHIT
        INC  R1
        CI   R1,8
        JNE  AKL
        CLR  R0
        RT
AKHIT   LI   R0,1
        RT

* ANYKY2 -> R0 = 1 if any NON-MODIFIER key is down (FCTN/SHIFT/CTRL ignored),
* so holding FCTN for AID doesn't start a game. clobbers R0,R1,R2,R12.
ANYKY2  CLR  R1
AK2L    MOV  R1,R0
        SWPB R0
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        CI   R1,0
        JNE  AK2C
        ORI  R2,>7000
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
* clobbers R0,R2,R3,R12.
HELPK   LI   R0,>0400         ; column 4, row 1 = H
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        MOV  R2,R3
        ANDI R3,>0200
        JNE  HKAID
        LI   R0,1
        RT
HKAID   LI   R0,>0000         ; column 0, row 4 = FCTN
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        MOV  R2,R3
        ANDI R3,>1000
        JNE  HKNO
        LI   R0,>0300         ; column 3, row 3 = 7
        LI   R12,>0024
        LDCR R0,3
        LI   R12,>0006
        STCR R2,8
        ANDI R2,>FF00
        MOV  R2,R3
        ANDI R3,>0800
        JNE  HKNO
        LI   R0,1
        RT
HKNO    CLR  R0
        RT

* RNGET: advance the xorshift-16 stream; R0 = the new value. leaf (R0,R1).
RNGET   MOV  @RNG,R0
        MOV  R0,R1
        SLA  R1,7
        XOR  R1,R0
        MOV  R0,R1
        SRL  R1,9
        XOR  R1,R0
        MOV  R0,R1
        SLA  R1,8
        XOR  R1,R0
        MOV  R0,@RNG
        RT

* DNUM5: draw the 16-bit value in R5 as 5 decimal digits at name address R1.
* uses R14 save and DIV.
DNUM5   MOV  R11,R14
        LI   R4,SCBUF+4
        LI   R6,5
        LI   R0,10
DSL     CLR  R2
        MOV  R5,R3
        DIV  R0,R2
        AI   R3,>0030
        SWPB R3
        MOVB R3,*R4
        DEC  R4
        MOV  R2,R5
        DEC  R6
        JNE  DSL
        BL   @SETWR
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
        MOV  R5,R3
        LI   R0,10
        DIV  R0,R2
        BL   @SETWR
        AI   R2,>0030
        SWPB R2
        MOVB R2,@VDPWD
        AI   R3,>0030
        SWPB R3
        MOVB R3,@VDPWD
        B    *R14

* ================================= data =====================================
REGTAB  BYTE >00,>80          ; R0 Graphics I
        BYTE >C2,>81          ; R1 16K + display ON + 16x16 sprites
        BYTE >00,>82          ; R2 name table >0000
        BYTE >0C,>83          ; R3 color table >0300
        BYTE >01,>84          ; R4 pattern table >0800
        BYTE >0F,>85          ; R5 sprite attributes >0780
        BYTE >03,>86          ; R6 sprite patterns >1800
        BYTE >11,>87          ; R7 backdrop black

* color table: one byte per 8-char group (FG nibble, BG nibble)
COLTAB  BYTE >11,>11,>11,>11                       ; 0-3   unused
        BYTE >F1,>F1,>F1,>F1,>F1,>F1,>F1,>F1       ; 4-11  text, white on black
        BYTE >11,>11,>11,>11                       ; 12-15 unused
        BYTE >C3                                   ; 16 grass: dark grn/lt grn
        BYTE >E1                                   ; 17 road+rail: gray/black
        BYTE >54                                   ; 18 water: lt blue/dk blue
        BYTE >C4                                   ; 19 lily: dark grn/dk blue
        BYTE >B3                                   ; 20 coin on grass
        BYTE >B1                                   ; 21 coin on asphalt
        BYTE >61                                   ; 22 signal (animated live)
        BYTE >11                                   ; 23 unused
        BYTE >11,>11,>11,>11                       ; 24-27 unused
        BYTE >51                                   ; 28 blocks: lt blue/black
        BYTE >11,>11,>11                           ; 29-31 unused

* car paint by variant: white, medium red, light yellow, cyan
CARCOL  BYTE >0F,>08,>0B,>07

* lane-mix thresholds per difficulty tier: roll 0..15 -> grass under g,
* road under r, river under v, else rail. 4th byte pads the row to a word.
GENTAB  BYTE 7,12,15,0
        BYTE 6,11,14,0
        BYTE 5,10,14,0
        BYTE 4,10,14,0
        BYTE 4,9,13,0
        BYTE 3,9,13,0
        BYTE 3,8,13,0
        BYTE 2,8,13,0

* big-title letters "JAYWALK": width byte then 5 row-bit bytes per letter,
* 0 ends. Total 3+3+3+5+3+3+3 wide + 6 kerning gaps = 29 columns.
BIGLET  BYTE 3,>07,>01,>01,>05,>02   ; J
        BYTE 3,>02,>05,>07,>05,>05   ; A
        BYTE 3,>05,>05,>02,>02,>02   ; Y
        BYTE 5,>11,>11,>15,>15,>0A   ; W
        BYTE 3,>02,>05,>07,>05,>05   ; A
        BYTE 3,>04,>04,>04,>04,>07   ; L
        BYTE 3,>05,>05,>06,>05,>05   ; K
        BYTE 0

TSUB    TEXT 'A TINY JAY VS THE WORLD'
TPRESS  TEXT 'PRESS ANY KEY TO PLAY'
THELP   TEXT 'H OR AID FOR HELP'
TSCORE  TEXT 'SCORE'
TCOIN   TEXT 'COIN'
TBEST   TEXT 'BEST'
TLANES  TEXT 'LANES'
TGOVER  TEXT 'GAME OVER'
TNEWB   TEXT 'NEW BEST!'
TANYK   TEXT 'PRESS ANY KEY'
TCAUS1  TEXT 'SQUISHED BY A CAR'
TCAUS2  TEXT 'SWEPT DOWN THE RIVER'
TCAUS3  TEXT 'HIT BY THE EXPRESS'
TCAUS4  TEXT 'CARRIED OFF BY A HAWK'

* game-over cause lines: (name-table addr, string, length), picked by DEAD
CAUSTB  DATA >0167,TCAUS1,17
        DATA >0166,TCAUS2,20
        DATA >0167,TCAUS3,18
        DATA >0165,TCAUS4,21

* help-screen copy
H1      TEXT 'JAYWALK HELP'
H2      TEXT 'HOP NORTH. NEVER STOP.'
H3      TEXT 'CONTROLS'
H4      TEXT 'E S D X - HOP N W E S'
H5      TEXT 'OR THE JOYSTICK ARROWS'
H6      TEXT 'FIELD GUIDE'
H7      TEXT 'BLOCKS YOUR PATH'
H8      TEXT 'SAFE FOOTING'
H9      TEXT 'WORTH 25'
H10     TEXT 'TRAIN DUE - GET CLEAR'
H11     TEXT 'RIDE LOGS. 10 A LANE. 25 A COIN.'
H12     TEXT 'HAWKS TAKE IDLE BIRDS'

* HELP layout: (name-table addr, string, length) per line, terminated by 0
HLPTAB  DATA >002A,H1,12
        DATA >0065,H2,22
        DATA >00A2,H3,8
        DATA >00C4,H4,21
        DATA >00E4,H5,22
        DATA >0122,H6,11
        DATA >0145,H7,16
        DATA >0185,H8,12
        DATA >01C5,H9,8
        DATA >0205,H10,21
        DATA >0240,H11,32
        DATA >0285,H12,21
        DATA >02C9,TANYK,13
        DATA 0

* field-guide icons: (name addr, two chars) — the 2x2 art beside each line
HICONS  DATA >0142
        BYTE >82,>83               ; bush (rows 10-11, col 2)
        DATA >0162
        BYTE >84,>85
        DATA >0182
        BYTE >98,>99               ; lily pad (rows 12-13)
        DATA >01A2
        BYTE >9A,>9B
        DATA >01C2
        BYTE >A0,>A1               ; coin (rows 14-15)
        DATA >01E2
        BYTE >A2,>A3
        DATA >0202
        BYTE >B0,>8A               ; crossing signal (rows 16-17)
        DATA >0222
        BYTE >8C,>8B
        DATA 0

* ============================ sound effects =================================
* Stream format: see SNDTCK. Tone periods follow f = 111861 / P.
SFXHOP  BYTE 3,>88,>02,>92,3,>8C,>03,>96,1,>9F,0
SFXCOIN BYTE 3,>85,>05,>91,3,>89,>03,>91,>82,1,>9F,0
SFXGO   BYTE 3,>86,>0D,>91,3,>8A,>0A,>91,3,>8F,>08,>91,3,>8B,>06,>90
        BYTE >83,1,>9F,0
SFXHONKA BYTE 3,>80,>14,>90,>82,3,>80,>19,>91,>82,3,>80,>14,>90,>82
        BYTE 1,>9F,0
SFXHONKB BYTE 3,>A8,>1A,>B0,>89,1,>BF,0
SFXBELL BYTE 3,>A7,>04,>B1,>82,1,>B6,>82,1,>BB,1,>BF,0
SFXHORNA BYTE 3,>8E,>0F,>90,>8C,1,>9F,0
SFXHORNB BYTE 3,>AA,>0C,>B0,>8C,1,>BF,0
SFXTHUD BYTE 2,>E4,>F2,>81,1,>F5,>81,1,>F8,1,>FF,0
SFXSPLSH BYTE 2,>E5,>F1,>82,1,>F3,>82,1,>F6,>82,1,>F9,>82,1,>FC
        BYTE 1,>FF,0
SFXRUMB BYTE 2,>E6,>F4,>86,1,>FF,0
SFXNOFF BYTE 1,>FF,0
SFXHAWK BYTE 3,>A0,>05,>B2,>81,1,>06,>81,1,>07,>81,1,>08,>81,1,>09
        BYTE >81,1,>0A,>81,1,>BF,0
SFXTUNE BYTE 3,>8B,>06,>91,>84,3,>85,>05,>91,>84,3,>87,>04,>91,>84
        BYTE 3,>85,>05,>91,>84,3,>8B,>06,>91,>84,3,>85,>05,>91,>84
        BYTE 3,>87,>04,>91,>84,3,>8F,>03,>90,>88,1,>9F,0
SFXTUN2 BYTE 3,>A6,>0D,>B3,>89,3,>AD,>11,>B3,>89,3,>A6,>0D,>B3,>89
        BYTE 3,>AC,>1F,>B3,>88,1,>BF,0
SFXOVER BYTE 3,>AA,>0A,>B1,>84,3,>A6,>0D,>B1,>84,3,>AD,>11,>B0,>88
        BYTE 1,>BF,0

* ======================== landscape glyph patterns ==========================
* (char, 8 pattern bytes) records, terminated by char 0. Char >88 (plain
* asphalt) stays the cleared all-zero pattern and needs no record.
LANDPT  BYTE >80,>10,>10,>00,>02,>02,>40,>40,>00   ; grass tufts A
        BYTE >81,>04,>04,>00,>40,>40,>08,>08,>00   ; grass tufts B
        BYTE >82,>03,>0F,>1F,>3F,>3F,>7F,>7F,>7F   ; bush, four quadrants
        BYTE >83,>C0,>F0,>F8,>FC,>FC,>FE,>FE,>FE
        BYTE >84,>7F,>7F,>3F,>3F,>1F,>0F,>03,>00
        BYTE >85,>FE,>FE,>FC,>FC,>F8,>F0,>C0,>00
        BYTE >89,>EE,>EE,>00,>00,>00,>00,>00,>00   ; road center dashes
        BYTE >8A,>00,>00,>FF,>FF,>00,>24,>24,>24   ; rail, upper + tie tops
        BYTE >8B,>24,>24,>24,>00,>FF,>FF,>00,>00   ; rail, ties + lower
        BYTE >8C,>10,>10,>10,>10,>10,>10,>38,>7C   ; signal pole and base
        BYTE >90,>00,>00,>66,>00,>00,>99,>00,>00   ; water A (animated)
        BYTE >91,>00,>99,>00,>00,>66,>00,>00,>00   ; water B (animated)
        BYTE >98,>07,>1F,>3F,>7F,>7F,>FF,>FF,>FF   ; lily pad, four quadrants
        BYTE >99,>E0,>F8,>FC,>F8,>F0,>FE,>FE,>FE
        BYTE >9A,>FF,>FF,>7F,>7F,>3F,>1F,>07,>00
        BYTE >9B,>FE,>FE,>FC,>FC,>F8,>F0,>C0,>00
        BYTE >A0,>03,>0F,>1C,>38,>70,>60,>E0,>E0   ; coin ring (grass group)
        BYTE >A1,>C0,>F0,>38,>1C,>0E,>06,>07,>07
        BYTE >A2,>E0,>60,>70,>38,>1C,>0F,>03,>00
        BYTE >A3,>07,>06,>0E,>1C,>38,>F0,>C0,>00
        BYTE >A8,>03,>0F,>1C,>38,>70,>60,>E0,>E0   ; coin ring (asphalt group)
        BYTE >A9,>C0,>F0,>38,>1C,>0E,>06,>07,>07
        BYTE >AA,>E0,>60,>70,>38,>1C,>0F,>03,>00
        BYTE >AB,>07,>06,>0E,>1C,>38,>F0,>C0,>00
        BYTE >B0,>38,>7C,>7C,>7C,>38,>10,>10,>10   ; signal lamp head
        BYTE >E0,>FF,>FF,>FF,>FF,>FF,>FF,>FF,>FF   ; solid block
        BYTE 0

* water animation frames: patterns for chars >90 and >91, 8 bytes each
WATP0   BYTE >00,>00,>66,>00,>00,>99,>00,>00
        BYTE >00,>99,>00,>00,>66,>00,>00,>00
WATP1   BYTE >00,>00,>CC,>00,>00,>33,>00,>00
        BYTE >00,>33,>00,>00,>CC,>00,>00,>00

* ========================== sprite patterns =================================
* 13 16x16 shapes, 32 bytes each: the LEFT half's 16 rows, then the RIGHT
* half's 16 rows (the 9918A's quadrant order for 16x16 sprites). Pattern
* numbers count in 8-byte cells, so shape n is pattern number 4n.
* 0: the jay, grounded (plump, seen from above, tail south; the one-pixel
* gaps along rows 4-7 separate the folded wings from the body)
SPRPAT  BYTE >01,>03,>03,>01,>1B,>37,>6F,>6F
        BYTE >7F,>3F,>3F,>1F,>0F,>07,>03,>01
        BYTE >80,>C0,>C0,>80,>D8,>EC,>F6,>F6
        BYTE >FE,>FC,>FC,>F8,>F0,>E0,>C0,>80
* 4: the jay, mid-hop (wings out, feet tucked)
        BYTE >01,>03,>03,>01,>C3,>E7,>FF,>7F
        BYTE >3F,>0F,>07,>03,>03,>01,>02,>00
        BYTE >80,>C0,>C0,>80,>C3,>E7,>FF,>FE
        BYTE >FC,>F0,>E0,>C0,>C0,>80,>40,>00
* 8: the jay, flattened (a bad day on the asphalt)
        BYTE >00,>00,>00,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>08,>3F,>7F,>FF,>6D
        BYTE >00,>00,>00,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>10,>FC,>FE,>FF,>B6
* 12: the splash (spray ring over a water mound)
        BYTE >11,>00,>42,>00,>10,>00,>40,>00
        BYTE >21,>00,>00,>91,>49,>3F,>7F,>FF
        BYTE >10,>00,>08,>00,>20,>00,>04,>00
        BYTE >08,>00,>00,>89,>92,>FC,>FE,>FF
* 16: sedan, rightward
        BYTE >00,>00,>00,>00,>7F,>36,>7F,>FF
        BYTE >FF,>1C,>1C,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>00,>80,>C0,>FC,>FF
        BYTE >FF,>38,>38,>00,>00,>00,>00,>00
* 20: sedan, leftward
        BYTE >00,>00,>00,>00,>01,>03,>3F,>FF
        BYTE >FF,>1C,>1C,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>00,>FE,>6C,>FE,>FF
        BYTE >FF,>38,>38,>00,>00,>00,>00,>00
* 24: box truck, rightward
        BYTE >00,>00,>00,>FF,>FF,>FF,>FF,>FF
        BYTE >FF,>38,>38,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>C0,>C0,>F8,>FC,>FC
        BYTE >FC,>E0,>E0,>00,>00,>00,>00,>00
* 28: box truck, leftward
        BYTE >00,>00,>00,>03,>03,>1F,>3F,>3F
        BYTE >3F,>07,>07,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>FF,>FF,>FF,>FF,>FF
        BYTE >FF,>1C,>1C,>00,>00,>00,>00,>00
* 32: the log (round ends, a grain line)
        BYTE >00,>00,>00,>00,>3F,>7F,>FF,>DF
        BYTE >FF,>7F,>3F,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>00,>F8,>FC,>FF,>7D
        BYTE >FF,>FC,>F8,>00,>00,>00,>00,>00
* 36: locomotive, rightward (stack, boiler, cowcatcher, wheels)
        BYTE >00,>00,>0F,>0F,>FF,>FF,>FF,>FF
        BYTE >FF,>FF,>6D,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>00,>F0,>F0,>FC,>FC
        BYTE >FC,>FE,>B3,>00,>00,>00,>00,>00
* 40: locomotive, leftward
        BYTE >00,>00,>00,>00,>0F,>0F,>3F,>3F
        BYTE >3F,>7F,>CD,>00,>00,>00,>00,>00
        BYTE >00,>00,>F0,>F0,>FF,>FF,>FF,>FF
        BYTE >FF,>FF,>B6,>00,>00,>00,>00,>00
* 44: the boxcar (slatted, symmetric)
        BYTE >00,>00,>00,>00,>7F,>6D,>7F,>7F
        BYTE >7F,>7F,>36,>00,>00,>00,>00,>00
        BYTE >00,>00,>00,>00,>FE,>B6,>FE,>FE
        BYTE >FE,>FE,>36,>00,>00,>00,>00,>00
* 48: the hawk (a diving V, head down, talons out)
        BYTE >00,>C0,>E0,>78,>3E,>0F,>03,>03
        BYTE >01,>01,>02,>00,>00,>00,>00,>00
        BYTE >00,>03,>07,>1E,>7C,>F0,>C0,>C0
        BYTE >80,>80,>40,>00,>00,>00,>00,>00

* 8x8 text font: char code then 8 pattern bytes; A-Z, digits, punctuation.
FONT    BYTE '-',>00,>00,>00,>F8,>00,>00,>00,>00
        BYTE '.',>00,>00,>00,>00,>00,>60,>60,>00
        BYTE '!',>20,>20,>20,>20,>20,>00,>20,>00
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
        END  START
