* QUEST.INC — the DUNGEONS OF FATE quest-file format, named ONCE.  (Ch. 41)
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
*
* This is the versioned binary SCHEMA that separates the ENGINE from its
* CONTENT — the whole point of a data-driven RPG (the Tunnels of Doom
* inheritance, Ch. 41). A quest file is a self-describing blob: a fixed header
* of counts and OFFSETS, followed by five sub-tables (map, monsters, items,
* text, tuning). The engine holds a POINTER to the blob and reads everything
* through these offsets, so it never contains a monster, a map, or a word of
* prose — change the file and you have a new game, no reassembly.
*
* This header is the SINGLE SOURCE OF TRUTH (the Ch. 38 asset-pipeline rule):
* the engine (dungeons.a99) COPYs it, and so does the PC quest-builder
* (§41.7) that emits the blob — one schema, two readers, no drift. Because it
* is only EQU definitions (no code, no entry label), verify.sh's *.a99 sweep
* skips it; a consumer pulls it in with  COPY 'quest.inc'.
*
* On the wire everything is big-endian; words sit on even offsets so the 9900
* can MOV them; every offset below is measured from the blob's own base.

QMAGIC EQU  >DF99            "Dungeons Of Fate", format id (header word 0)
QVER1  EQU  >0001            format version this engine reads

* ---- header (QHLEN bytes, at the blob base) --------------------------------
QMAG   EQU  0                >0  word  magic = QMAGIC
QVERS  EQU  2                >2  word  format version
QWID   EQU  4                >4  byte  map width  (tiles)
QHGT   EQU  5                >5  byte  map height (tiles)
QNMON  EQU  6                >6  byte  monster-type count
QNITM  EQU  7                >7  byte  item count
QNTXT  EQU  8                >8  byte  text-string count
QPSX   EQU  9                >9  byte  party start column (tiles)
QPSY   EQU  10               >A  byte  party start row
QRSV   EQU  11               >B  byte  reserved (0)
QOMAP  EQU  12               >C  word  offset -> RLE map stream
QOMON  EQU  14               >E  word  offset -> monster table
QOITM  EQU  16               >10 word  offset -> item table
QOTXT  EQU  18               >12 word  offset -> text index table
QOTUN  EQU  20               >14 word  offset -> tuning block
*                            >16 word  reserved -> header is QHLEN bytes
QHLEN  EQU  24               header length (even)

* ---- v1 reference map dimensions (buffers are sized for these) -------------
* The header still carries QWID/QHGT and the engine renders by them; a v1
* quest fixes them here so MBUF and the self-test index math are static.
MAPW   EQU  16               tiles across
MAPH   EQU  12               tiles down
MAPSZ  EQU  192              MAPW*MAPH  (decoded map buffer size)

* ---- monster record (MSIZE bytes; QNMON of them at QOMAP's sibling) ---------
MHP    EQU  0                byte  hit points (max; combat copies it, Ch.41.4)
MATK   EQU  1                byte  attack
MDEF   EQU  2                byte  defense
MSPD   EQU  3                byte  speed / initiative
MMOR   EQU  4                byte  morale: flees when HP falls to <= this
MGLY   EQU  5                byte  map glyph
MXP    EQU  6                byte  reward (xp / gold)
MSIZE  EQU  8                bytes per monster (even; +7 reserved)

* ---- item record (ISIZE bytes; QNITM of them) ------------------------------
ITYPE  EQU  0                byte  0 weapon / 1 armour / 2 heal
IPOW   EQU  1                byte  magnitude (weapon +ATK, armour +DEF, ...)
IGLY   EQU  2                byte  catalog glyph
ISIZE  EQU  4                bytes per item (+3 reserved)

* ---- tuning block: the party's starting stats + balance knobs, as data -----
TH0HP  EQU  0                byte  hero 0 HP
TH0AT  EQU  1                byte  hero 0 ATK
TH0DF  EQU  2                byte  hero 0 DEF
TH0SP  EQU  3                byte  hero 0 SPD
TH1HP  EQU  4                byte  hero 1 HP
TH1AT  EQU  5                byte  hero 1 ATK
TH1DF  EQU  6                byte  hero 1 DEF
TH1SP  EQU  7                byte  hero 1 SPD
THIT   EQU  8                byte  to-hit threshold: (roll & 7) < THIT hits
*                            >9 byte reserved

* ---- text pool: QNTXT words at QOTXT, each an offset to a packed string -----
* A packed string is a stream of 5-bit symbols, MSB first, ending in symbol 0
* (Ch. 38 packed prose, 37.5% off ASCII). Alphabet: 0=end, 1=space, 2..27=A-Z,
* 28='.', 29=',', 30='!', 31='?'. The unpacker (SAY/TXUNPK) walks the bitstream.
TSEND  EQU  0                symbol 0 terminates a string
TSSPC  EQU  1                symbol 1 = space
TSA    EQU  2                symbol 2 = 'A' (letters run 2..27)
