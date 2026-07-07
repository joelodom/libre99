# RECON — the GROM-rewrite interface dossier

Living record of what the **authentic** console ROM+GROM actually do, recovered
empirically with the emulator's GROM tracer. This is the **interface contract**
the rewrite must honour — the single reference for headers, dispatch mechanisms,
scratchpad cells, and verified GPL semantics. *How* facts like these get
discovered is [`DEBUGGING.md`](./DEBUGGING.md); the original plan/review that
seeded this dossier are archived in [`history/`](./history/). Reproduce the
boot/title/menu facts with `cargo run -p libre99-core --example recon_probe`
(from the repo root).

Confidence tags: ✅ reproduced on this emulator with the real images · 📖
literature · ❓ inference (names the step that confirms it).

---

## R1 — the fixed GPL boot entry: GROM `>0020` ✅

- After reset the ROM self-tests, then strobes the GROM data port **once**
  (`>1FFF`, a throwaway) before setting any address — so "first read = entry"
  is off by one. It then writes GROM address **`>0020`** and begins fetching.
- The bytes at `>0020` are `40 52`, executed as **`BR >0052`** (an
  unconditional branch — the condition bit is clear at power-up). Hence:
  - the ROM's hardcoded GPL entry is **`>0020`**;
  - `BR`/`BS` (`>40–7F`) are **13-bit slot-absolute**:
    `addr = (pc & >E000) | ((op & >1F) << 8) | operand_byte`
    (proved again by the sound-wait loop `43 3C` → `BR >033C`).
- `B`/`CALL` (`>05`/`>06`) take a **16-bit absolute** GROM address
  (`>0038: 05 4D 12` → `B >4D12`).

### Machine state the ROM hands to our GPL (the entry contract)

At the first GPL fetch:

- **Every VDP register `>00`** — display off, VBLANK interrupt off. Our GPL owns
  all VDP setup.
- Scratchpad is zero **except** the GPL interpreter workspace at `>83E0–>83FF`:
  `R0 (>83E0) = >0020` (the entry addr), `R13 (>83FA) = >9800` (GROM read
  port), `R14 (>83FC) = >0100` (flags), `R15 (>83FE) = >8C02` (VDP write port).
- ⚠ Those zeros are the **emulator's** RAM init, not hardware — real scratchpad
  powers up random. Our firmware must explicitly initialise every cell it (or
  the ISR) reads.

---

## R2 — title screen & menu

### Title state after boot (180 frames) ✅

VDP registers `R0=>00 R1=>E0 R2=>F0 R3=>0E R4=>F9 R5=>86 R6=>F8 R7=>F7`.
Effective layout (9918A masks unused bits): name `>0000`, pattern `>0800`,
color `>0380`, sprite-attr `>0300`, sprite-pattern `>0000`, backdrop **cyan**
(R7 low nibble 7). `R1=>E0` = 16K + display-on + interrupt-enable. During boot
GPL copies an 8-byte register table out of GROM `>0451–>0458`
(`00 20 F0 0E F9 86 F8 F7`; note R1 starts `>20` = display off, raised to `>E0`
after drawing).

Name table (character codes are **plain ASCII, no `>60` offset** ✅):

| Row | Content | Glyphs |
|---|---|---|
| 0–2, 18–20 | colour bars | custom `>60–>DF` |
| 5–7 | "TI" logo | custom `>01–>09` |
| 9 | `TEXAS INSTRUMENTS` | ASCII |
| 11 | `HOME COMPUTER` | ASCII |
| 16 | `READY-PRESS ANY KEY TO BEGIN` | ASCII |
| 22 | `©1981  TEXAS INSTRUMENTS` | `©` = custom glyph `>0A` |

Text rows and the "TI" logo are **col-positioned**: `TEXAS INSTRUMENTS` at row 9
col 8, `HOME COMPUTER` at row 11 col 10, the prompt at row 16 col 2, the logo as
a 3×3 glyph block at rows 5–7 cols 15–17. The **menu** reuses the same emblem +
banner in the top-left (logo rows 1–3 col 2; banner col 8; `PRESS` row 5 col 4;
entries from row 7, two rows apart). Reproduced in `grom/console.gpl`.

**Colour table (`>0380`, one byte / 8 chars = `fg<<4 | bg`)** ✅ — recovered with
`examples/title_recon.rs`. Groups 0–11 (chars `>00–>5F`, all text + the logo) =
`>17` (**black on cyan**). Groups 12–27 (chars `>60–>DF`, the colour bars) have
**fg = 0 (transparent)** and a bg colour, so a **blank** glyph shows a solid bar
of that bg colour; the sixteen bg values, in order, are `6 3 1 B C D F 4 2 D 8 E
5 9 A 6`. Backdrop is cyan (R7 low nibble 7). One bar-colour group spans two
name-table cells; a row is `>60 >60 >68 >68 … >D8 >D8` (step the code by 8 to
reach the next group). Our recreation reproduces this palette and mechanism (the
uncopyrightable colour-bar test pattern) while replacing the copyrighted logo and
copyright glyphs.

### The VBLANK ISR sound-list format ✅

The console ROM's VBLANK ISR plays a **GROM sound list** pointed to by `>83CC/D`
(a GROM address), with `>83CE` the frame countdown. Recovered byte-exactly by
tracing the ISR's GROM reads during the authentic **power-on beep**
(`examples/title_recon.rs`). Format — a chain of blocks:

```
block := [N][N bytes written to the sound chip >8400][D = duration in frames]
a block whose duration D = 0 ends the list (its N bytes are still written).
```

To **start** a sound, point `>83CC/D` at the list and set `>83CE = 1`; on the
next tick the ISR decrements `>83CE` to 0, processes block 0 (writes its N bytes,
loads `>83CE = D`, advances the pointer), and so on until a `D = 0` block. The
authentic power-on beep is `06 BF DF FF 80 05 92 0A` (mute ch1–3, set channel 0
to divider `>50` ≈ 1400 Hz at volume 2, hold 10 frames — held longer because it
is armed before the 9901 interrupt is unmasked, so the countdown is frozen until
the ISR goes live), then `01 9F 00` (mute channel 0, end). Our `START` arms an
**original** list of the same shape for the same beep character; the ISR needs
the 9901 VDP interrupt enabled first (§11).

### Menu scan + dispatch (Centipede = ROM-only, verified) ✅

- The menu lists **both** GPL and ML programs: `1 FOR TI BASIC` (GROM 1) then
  `2 FOR CENTIPEDE` (a **ROM-only** cart) — proving the scan reads **CPU
  `>6000`** headers too, not just GROM. (See the census below: 33/137 carts are
  ROM-only.)
- Scan state seen in scratchpad while the menu is up: `>8302/3 = >6010` (the ROM
  cart's program-list pointer, from CPU `>6006`), `>8306/7 = >214D` (console
  GROM 1's program list).
- Keypress → **key beep**: `>83CC/D` points at a **sound list in GROM** (`>0484`);
  the VBLANK ISR fetches the sound bytes out of GROM (saving/restoring the GROM
  address counter around them — the `>9802` flip-flop behaviour). The menu then
  **polls until the beep ends** in a tight loop at `>033C`
  (`8F 80 CE / 43 3C` = test `@>83CE` sound-bytes-remaining; `BR >033C`).
  ⇒ **any keyed test must keep calling `run_frame()`** or it hangs here forever.
- **ML dispatch (the trampoline our menu reuses):** the menu writes the
  program's entry address (`>6056`, from the ROM header's program list) into
  **CPU `>8300`** and executes **`XML >F0`** ("vector in `>8300`", per
  Nouspikel). One frame later the CPU is running the cartridge (observed PC
  `>7E04`). Sequence: `DST @>8300,entry ; XML >F0`. ✅ execution-verified —
  see Verified Mechanisms §2. (An early trace note claimed the cell was
  `>8380`; that is the GPL **sub-stack slot the menu copies from**, not the
  vector. Dispatching through `>8380` does nothing.)
- GROM-cart (GPL) dispatch: fake a frame on the GPL subroutine stack and
  `RTN` into the cartridge — see Verified Mechanisms §3. ✅

---

## Cartridge census — all 137 bundled `.ctg` (parsed via `cartridge.rs` logic) ✅

| Class | Count | Consequence |
|---|---|---|
| GROM-bearing (48 GROM-only + 56 GROM+ROM) | 104 | listed via GROM `>6000+` headers |
| **ROM-only (ML menu entries)** | **33** | **invisible unless CPU `>6000` scanned** |
| Bank-switched ROM (2 banks) | 21 | read header from power-on bank; scan must **not write** `>6000–>7FFF` (a write flips banks) |
| Multi-program menus | 10 (et=7, mine=7, Soccer=5, HuntTheWumpus=3, VideoGames1/2=3…) | walk the program list to the end |
| Declare a power-up list | **0** | boot power-up walk unneeded for the bundled gate |
| No `>AA` header at GROM or ROM `>6000` | 11 (connect, fantasy, frogger, germptl, hangman×2, popeye, qbert, rtpirat, sxba, zerozap) | triage in M3 vs the authentic GROM |

ROM-only list: ambulnc, ant, anteat, Barrage, centipe, chicoop, defend, digdug,
donkey, drivdem, henpeck, jungle, JungleHunt, king, moonpat, MoonPatrol, mspac,
mtennis, Munch2, pacman, picparn, polepos, prnfrog, protect, rabbitt, romox,
rotraid, schnoz, shamus, SuperStorm, topper, typoii, typoman.

---

## The three console GROMs are interconnected ✅

- GROM 0 `>0010–>0037` is a **table of twenty GROM addresses, all in
  `>4000–>5FFF` (GROM 2)**: `43DC 443C 49A9 4396 439E 4446 4449 444C 4052 51FE
  4C82 4D59 4DB4 4E64 4EF9 4F01 4F5F 4F80 43CE 43D6`.
- `>0038: B >4D12`, then short branch stubs (`>003B: BR >125E`, `>003D: BR
  >0417`, `>003F: B >2844`). These low fixed addresses are the 📖 **GPLLNK
  service entries** the E/A ecosystem calls.
- GROM 1's header points its **subprogram list at `>4D1A`** (inside GROM 2).

⇒ GROM 2 hosts the console's shared GPL library reached through fixed
low-address vectors in GROM 0. The **slots** (`>0010–>004F`) are the fixed
interface; the **targets** are ours to place. R3 (P3) measures whether any
bundled cart uses these.

---

## Console device I/O — the interconnect table, DSRLNK, and the DSR list ✅

How a cartridge loads a file from a device — traced with `tod_disk_probe` and
`tod_load_probe` driving Tunnels of Doom under the authentic GROM (the worked
user-session/differential repro; DEBUGGING.md "M7 — console device I/O"). This
is the **interface a compatible OS must honour**; implement it clean-room, never
copy the authentic GPL.

**The interconnect table is a jump table of executable `BR` stubs.** The twenty
words at GROM `>0010-0037` are not read as data — a cartridge **`CALL`s** a slot
and the console **executes** its two bytes as a `BR`. Authentic values, decoded
as instructions (slot 8 `>0020` doubles as the ROM's fixed boot entry):

```
>0010 BR >03DC   >0012 BR >043C   >0014 BR >01A9(*) >0016 BR >0396
>0018 BR >039E   >001A BR >0446   >001C BR >0449   >001E BR >044C
>0020 BR >0052   >0022 BR >11FE   >0024 BR >0C82   ... (>0026-0036)
```
(*) the raw word is a GROM-2 pointer; executed as `BR` it targets the low slot-0
address shown — the table entries resolve to GROM-0 routines, which in turn reach
the GROM-2 library. What matters: **`CALL >0010` runs the console's DSRLNK.**

**DSRLNK (the file-I/O linkage), authentic `>03DC-0445`.** A program builds a
PAB in VDP RAM naming `DEV.FILENAME` (e.g. `DSK1.QUEST`), sets **`>8356` =
VDP address of the PAB name-length byte**, and `CALL`s `>0010`. DSRLNK then:
1. parses the device name up to the `.` separator (max 8 chars), into DSRLNK
   scratch `>8352` (scan ptr) / `>8354`,`>8355` (length) / `>8358` (counter);
2. clears the DSR done flag **`>83D0`**;
3. `XML >19` — the **kept console ROM** locates the device's DSR (CRU scan +
   header match);
4. loops `XML >1A` (the ROM calls the DSR to transfer one record) until `>83D0`
   goes non-zero, managing GPL **sub-stack** (`>8373`) frames so the DSR can
   `RTN` back into GPL; `>83FA`/`>8372` are the interpreter cells the XMLLNK
   return convention uses.

The ROM's `XML >19`/`>1A` do the real work (CRU device search + `BLWP` into the
peripheral DSR); the GROM wrapper is only the name-parse + linkage. For disk,
the DSR is the disk card's `Disk.Bin` ROM at CPU `>4000` (kept, emulated) — it
reads sectors 1,3,85-135 for QUEST and the cart reaches `GAME SELECTION: NEW
DUNGEON`. **Our reimplementation lives at GROM `>1200` (`DSRLNK`), wired via
`>0010 BR DSRLNK`, byte-verified equivalent to `>03DC`** (DEBUGGING M7).

**The DSR list (header offset `>08`)** is how DSRLNK finds console-*resident*
devices (cassette). The authentic console GROM 0 declares it at `>08 = >1310`:
`>1310 "CS1" → >1326`, `>1318 "CS2" → >132C` — the cassette DSR is GPL at
`>1300-16FF`. Disk needs **no** console DSR-list entry: DSRLNK's CRU scan finds
`DSK1` in the disk card's own ROM. So for the disk path our GROM 0 header `>08`
can stay `>0000`.

**Peripheral power-up (implemented).** Before any load, the console **boot** must
run each peripheral card's DSR power-up. The authentic boot does this **inline**
at GROM `>0183` right before its key-wait: clear the done flag `>83D0`, set the
DSR opcode **`>836D = >04`** (power-up), then loop `XML >19` (find the next card)
/ `XML >1A` (run its power-up) via a data-stack (`>8372`) continuation until
`>83D0` clears. The disk card's power-up reserves a VRAM buffer at the top of
VRAM and **lowers `>8370` (top of free VDP RAM) from `>3FFF` to `>37D7`**
(`docs/STATUS.md`). Without it `>8370 = 0`, the DSR has no buffer, and a load
stalls at 0 sectors. **Our `START` now runs this scan** (after setting
`>8370 = >3FFF`), reproduced clean-room from the traced interface — so ToD loads
QUEST from disk and reaches `NEW DUNGEON` (gate `tests/device_io.rs`).

⚠ **Cassette (CS1) cannot work in-emulator regardless** — no cassette hardware
(`crates/libre99-core/src/cru.rs`: motor/level CRU outputs "not wired", no
read-data input). Disk is the supported load path.

---

## Console character-set loaders — interconnect slots >0016 / >0018 ✅

How a cartridge fills the VDP pattern table with the console's built-in fonts —
traced with `examples/invaders_probe.rs` driving TI Invaders under the authentic
GROM (the worked user-session/differential repro; DEBUGGING.md "TI Invaders text
doesn't draw"). These are two entries of the interconnect jump table (above);
implement clean-room, never copy the authentic GPL or font bytes.

**The console ships two 8×8 character sets, contiguous in GROM 0:**
- the **standard (heavy) set** at GROM `>04B4`, 512 bytes = 64 glyphs (codes
  `>20..>5F`), **8 rows/glyph** — the same block `font.rs` already ships and the
  title/menu load (`tests::matches_authentic_character_set`);
- a **thin ("small") set** at GROM `>06B4` (immediately after), **7 rows/glyph**
  (the top row is implied blank), 64 glyphs (`>20..>5F`) — now also shipped as
  `font.rs::THIN_GLYPHS` (`tests::matches_authentic_thin_set`).

**The two loaders (interconnect slots, decoded as `BR` targets):**
- `>0016 → >0396`: **`MOVE >0200, G@>04B4, V@*>834A`** then `RTN`. A single
  512-byte copy of the standard set to the VDP address held (word, indirect) in
  cell **`>834A`**. Leaves `>834A` unchanged.
- `>0018 → >039E`: loads the thin set. Sets a source pointer (`>83D0 := >06B4`)
  and a counter (`>83D2 := 64`), then loops **64×**: `CLR V@*>834A` (blank the
  glyph's top row); copy **7** source rows to the next 7 VDP rows; `ADD
  @>834A,>0008` (dest += 8); `ADD @>83D0,>0007` (src += 7); `DEC @>83D2`. Net:
  writes 64×8 = 512 bytes (each glyph = blank row + 7 stored rows) and **advances
  `>834A` by `>0200`**.

**The caller's contract:** put the target VDP pattern-table address in **`>834A`**,
then `CALL >0016`/`>0018`. Games that draw text with the console font (TI
Invaders, and per the R3 histogram amazing/Parsec/HuntTheWumpus/MoonPatrol read
these slots) depend on it; with the slots stubbed to `RTN` the glyphs stay blank
and the text is invisible while cart-defined sprites still show.

**Our reimplementation** (`grom/console.gpl`, byte-verified effect vs authentic):
`>0016 → LDCSET` is the same single `MOVE` from `FONT`; `>0018 → LDTSET` is a
single `MOVE` from **`FONT2`** — the thin set shipped **pre-expanded** to 8
rows/glyph (leading blank row) at GROM `>1800`, so one 512-byte `MOVE` produces
byte-identical VRAM to the authentic per-glyph loop — followed by `DADD
@>834A,>0200` to match the authentic loader's destination-advance side effect
(the cart may chain a second load). Reproduced clean-room from this interface.

---

## Raw boot trace — first 120 GPL fetches (addr=byte) ✅

Reproduce: `recon_probe` probe 1. Entry `>0020`; note the `>1FFF` dummy at
index 0 and the branch `40 52 → >0052`.

```
  0: >1FFF=00   1: >0020=40   2: >0021=52   3: >0052=87   4: >0053=80   5: >0054=CE
  6: >0055=BE   7: >0056=8F   8: >0057=11   9: >0058=00  10: >0059=70  11: >005A=BE
 12: >005B=81  13: >005C=00  14: >005D=9F  15: >005E=BE  16: >005F=81  17: >0060=00
 18: >0061=BF  19: >0062=BE  20: >0063=81  21: >0064=00  22: >0065=DF  23: >0066=BE
 24: >0067=81  25: >0068=00  26: >0069=FF  27: >006A=BF  28: >006B=72  29: >006C=FF
 30: >006D=7E  31: >006E=39  32: >006F=00  33: >0070=08  34: >0071=00  35: >0072=04
 36: >0073=51  37: >0451=00  38: >0452=20  39: >0453=F0  40: >0454=0E  41: >0455=F9
 42: >0456=86  43: >0457=F8  44: >0458=F7  45: >0074=86  46: >0075=00  47: >0076=35
```

Hand-tiled (bootstrapping the ISA, confirmed by the P2 oracle):

```
>0020  40 52            BR   >0052            ; unconditional (cond clear)
>0052  87 80 CE         <op87> @>83CE         ; 1 GAS operand
>0055  BE 8F 11 00 70   ST    @>9400, >70      ; GAS + imm8  (BE = store imm byte)
>005A  BE 81 00 9F      ST    @>8400, >9F      ; sound chip mute ch0
>005E  BE 81 00 BF      ST    @>8400, >BF      ; mute ch1
>0062  BE 81 00 DF      ST    @>8400, >DF      ; mute ch2
>0066  BE 81 00 FF      ST    @>8400, >FF      ; mute ch3
>006A  BF 72 FF 7E      <opBF> ...             ; DST-style (GAS + imm/GAS)
>006E  39 00 08 00 04 51  MOVE 8, >0451→VDPregs ; count imm16, GROM src >0451
```

The `>0451–>0458` reads are the 8 source bytes (`00 20 F0 0E F9 86 F8 F7`) —
the VDP register init table (R2 above).

---

## The GPL execution model ✅

The GPL "program counter" *is* the GROM chip's auto-incrementing address
counter: the console ROM's interpreter fetches opcode/operand bytes through
`>9800`, executes, and only rewrites the counter (via `>9C02`) on branches.
Consequently the emulator's `grom_log` **is an instruction-fetch trace** — the
recon technique everything here rests on. `CALL` pushes the return address on a
scratchpad sub-stack (byte pointer at `>8373`, stack at `>8380+`; CALL
pre-increments by 2 then stores, RTN reads then post-decrements). The VBLANK ISR
may move the GROM address mid-instruction (e.g. to fetch sound-list bytes) and
restores it afterwards — trace analysis must tolerate those excursions.
Interrupts are effectively always enabled between GPL instructions (once the
9901 mask is set — §11).

---

## The GROM/GPL header contract ✅

The `>AA` header sits at the base of each 8 KiB GROM (console GROM, cartridge
GROM at `>6000`, DSR ROM) **and** at CPU `>6000` for ROM-only cartridges:

| Offset | Size | Field | Notes |
|---|---|---|---|
| `>00` | 1 | **Valid byte `>AA`** | header ignored unless `>AA` |
| `>01` | 1 | Version | `>02` on 99/4A GROMs; ROM carts vary (`>FF` seen) |
| `>02` | 1 | # programs | informational |
| `>03` | 1 | reserved | `>00` |
| `>04` | 2 | **Power-up list** ptr | GPL routines run at boot (`>0000` = none; **zero bundled carts use it** — census) |
| `>06` | 2 | **Program (menu) list** ptr | the selection-list entries (`>0000` = none) |
| `>08` | 2 | **DSR list** ptr | device service routines |
| `>0A` | 2 | **Subprogram list** ptr | numbered GPL subprograms |
| `>0C` | 2 | interrupt link / reserved | `>0000` in console GROMs |
| `>0E` | 2 | reserved | `>0000` |

Every list is a singly-linked chain of entries:

```
+0  2 bytes  pointer to NEXT entry   (>0000 = end of list)
+2  2 bytes  start address           (GROM addr for GPL, CPU addr for ML)
+4  1 byte   name length N
+5  N bytes  name (ASCII, shown on the menu)
```

Pointers in GROM headers are GROM addresses; pointers in a CPU `>6000` ROM
header are CPU addresses in `>6000–>7FFF`. Decoded examples ✅: the authentic
console GROMs —

```
GROM 0 @>0000:  AA 02 00 00  0000 0000 1310 1320   (no programs; DSR = cassette,
                                                    subprog list -> one entry)
GROM 1 @>2000:  AA 02 01 00  0000 214D 0000 4D1A   (program list >214D ->
                                                    { 0000, >216F, "TI BASIC" })
GROM 2 @>4000:  no >AA header — continuation code for BASIC
```

and ROM-only carts — `centipe`: `AA FF 00 00  0000 6010 …`, program list
`>6010` → entry `>6056`, name `"CENTIPEDE"`; `moonpat`: entry `>60FE`,
`"MOON PATROL"`. Our rewrite's own headers (GROM 0 menu shell, GROM 1
TI PYTHON) follow the same layout — see `grom/console.gpl`.

---

## Scratchpad map (`>8300–>83FF`) ✅

Ownership: **SYS** = the ROM interpreter / ISR / SCAN — initialise as noted,
never repurpose; **OURS** = free for our GPL. SYS values verified from the
boot/title snapshots.

| Cells | Owner | Contents / required init |
|---|---|---|
| `>8300–>836F` | OURS | free workspace. The menu and the TI PYTHON REPL each document their exact cell layout in comment blocks in [`grom/console.gpl`](./grom/console.gpl) — that source is the authority. (TI literature reserves `>834A–>836F` as the ROM's floating-point workspace, but only BASIC's ROM routines use it; our GPL never calls them, so we use the cells freely.) ⚠ `>8300–>8304` double as the `XML >F0` dispatch vector and the `IO` CRU list — set those immediately before use. |
| `>8370/1` | SYS | top of free VDP RAM; ROM/GPL sets `>3FFF` |
| `>8372` / `>8373` | SYS | GPL data / subroutine stack pointers (`>FE` / `>7E` at entry) |
| `>8374` / `>8375` | SYS | SCAN: keyboard mode / key code (init `0` / `>FF`) |
| `>8376/7` | SYS | joystick Y/X (SCAN modes 1–2) |
| `>8378` | SYS | random byte (ISR-stirred) |
| `>8379` | SYS | VDP-interrupt timer (free-running counter — the ISR-liveness signal) |
| `>837A` | SYS | auto-sprite count — **init 0** |
| `>837B` | SYS | VDP status copy |
| `>837C` | SYS | GPL status byte (condition/carry/…) |
| `>837D–>837F` | SYS | interpreter internals — leave alone |
| `>8380–>83BF` | SYS | GPL subroutine sub-stack (grows up from `>8380`); `>8380/1` is also the slot the authentic menu stages entries in |
| `>83C0–>83DF` | SYS | ISR workspace: `>83C2` disable flags (**init 0**), `>83C4/5` user hook (**init 0**), `>83C8–>83CA` scan debounce, `>83CC/D` sound-list ptr + `>83CE` bytes-remaining (**init 0**), `>83D4` VDP-R1 image (**init = our R1**), `>83D6/7` screen-blank timeout |
| `>83E0–>83FF` | SYS | GPL interpreter workspace R0–R15; R13=`>9800`, R14=flags, R15=`>8C02` |

---

## VERIFIED MECHANISMS — the 2026-07-02 research session ✅

Every mechanism M2–M6 needs is now **execution-verified on this emulator**
against the real console ROM, with the full GPL ISA extracted from Classic99
(`../classic99/addons/gpl.cpp:43-334` — the 256-entry opcode table and TI's
per-opcode format specs) and cross-checked against
[Nouspikel's console-ROM docs](https://www.unige.ch/medecine/nouspikel/ti99/roms.htm)
and [headers page](https://www.unige.ch/medecine/nouspikel/ti99/headers.htm).
The complete ISA now lives in `crates/libre99-gpl/src/isa.rs` (families, format
bits, MOVE bit field); the probes are `crates/libre99-gpl/examples/{m2_probe,
move_c_probe,m4_probe,menu_scan_trace}.rs`.

### 1. Key-wait (M2 menu / M4 REPL input) ✅

```
        ST   @>8374,>00      ; keyboard mode 0
        ST   @>8375,>FF      ; 'no key'
KEYLP   SCAN
        CEQ  @>8375,>FF      ; >8375 holds the key while pressed, >FF when none
        BS   KEYLP           ; loop while no key
        ; key (ASCII, e.g. '5' = >35) is in >8375 — COPY IT OUT IMMEDIATELY
        ; (`ST @mycell,@>8375`): the VBLANK ISR can overwrite >8375 later.
```

Proven by backdrop-flip on keypress (`m2_probe` step 1). Do **not** rely on the
condition bit SCAN sets (its 'new key' pulse is easy to miss); the `CEQ` test
against `>FF` is level-triggered and robust. For "wait for key release" loop
with `BR KEYLP` instead. (KSCAN debounce nuance: `>8375` holds the key *while
pressed* but SCAN's new-key pulse fires only once — so SCAN exactly once per
loop iteration and test `>8375` immediately; don't re-scan before reading.)

### 2. ROM-cartridge dispatch (M2) ✅

```
        DST  @>8300,>xxxx    ; the ML entry address from the CPU >6000 header
        XML  >F0             ; branch through the vector at >8300
```

`m2_probe` step 3: PC observed landing **exactly at `>6056`** (Centipede's
entry). The earlier failure used `>8380` — wrong cell; Nouspikel confirms
"XML >F0 (vector in >8300)". Do the pre-launch cleanup (§5) first.

### 3. GROM-cartridge dispatch (M2) ✅

```
        DST  @>8380,>xxxx    ; fake one frame on the GPL subroutine stack
        ST   @>8373,>80      ; sub-stack pointer -> that frame
        RTN                  ; 'return' into the cartridge GPL
```

`m2_probe` step 4: first cartridge-GROM fetch observed **exactly at `>602A`**
(A-MAZE-ING's entry), 3.7k sustained fetches after. (The sub-stack grows up
from `>8380`; pointer byte at `>8373`; CALL pre-increments by 2 then stores,
RTN reads then post-decrements — Nouspikel gpl.htm.)

### 4. Reading cartridge headers (M2) ✅ — the VDP-window strategy

The **authentic menu itself** (traced in `menu_scan_trace`) does NOT use
computed GROM addressing: it `MOVE`s a fixed window of the cartridge GROM into
a VDP buffer and chases pointers in the VDP copy:

```
>01C4:  31 001E A400 6000     MOVE >001E, G@>6000, V@>0400
```

Copy that strategy with a bigger window (the C=1 "computed GROM source" MOVE
variant failed every probe layout — banned; see §7):

- **GROM headers**: per base `>6000,>8000,…,>E000` (unrolled immediates),
  `MOVE >1000, G@base, V@>2000` (4 KiB window into free VRAM ≥ `>1000`), then
  parse the copy: `>AA` check via `CEQ V@>2000,>AA`, program-list pointer at
  `V@>2006-7`, entries/names chased with `*V@cell` word pointers (subtract
  `base->2000` from GROM pointers to get window offsets).
- **ROM headers**: read CPU `>6000` directly — `MOVE >0020, @>6000, @>83xx`
  works from GPL (verified: `AA FF 00 00` read from mounted Centipede;
  the 16-bit CPU GAS form encodes `>6000` as `>DD00`, bias `>8300`).
- Cart names → screen: VDP→VDP MOVE (the menu's own VRAM-fill uses it) or
  VDP→CPU→VDP; VDP→CPU is verified.

### 5. Pre-launch cleanup contract (M2) ✅ 📖

Traced from the authentic menu (fetches `>0341-037A`) and confirmed by
Nouspikel's headers page — do this before either dispatch:

```
        DCGT @>8370,>1000    ; (guard: top of free VRAM > >1000)
        BR   SKIPCLR
        DST  @>8300,@>8370   ; count = top-of-VRAM - >0FFF
        DSUB @>8300,>0FFF
        MOVE @>8300,V@>0FFF,V@>1000   ; smear-clear VRAM >1000..top
SKIPCLR CLR  @>8300
        MOVE >006F,@>8300,@>8301      ; zero >8300-836F (cascade fill)
        CLR  @>8374                   ; keyboard mode 0
        DCLR @>8382
        MOVE >003C,@>8300,@>8384      ; zero >8384-83BF
        ; color table: fill 32 bytes with >17 (black on cyan) — TI's value;
        ; MOVE >001F,V@>0380,V@>0381 after ST V@>0380,>17
```

(TI's exact stream: `CF 70 1000 / 43xx / BD 00 70 / A7 00 0FFF / 34 00 AF1000
AF0FFF / 86 00 / 35 006F 01 00 / 35 003C 8084 00 / 86 74 / 35 001F A381 A380 /
87 8082 / … / BD 00 8080 / 0F F0` — kept here as the behavioral reference.)

### 6. Arithmetic + control semantics (M4) ✅

All pinned by `m4_probe` on the real interpreter:

| Op | Verified semantics |
|---|---|
| `DMUL dst,src` | 32-bit product → **dst (high word), dst+2 (low word)**; 7×6 left `>8340=0000`,`>8342=002A`. dst+2 is overwritten! |
| `DDIV dst,src` | dividend = 32-bit at dst (dst:dst+2); **quotient → dst, remainder → dst+2**; 47/5 → `0009`,`0002`. Unsigned — do sign fix-up manually. |
| `DADD/DSUB dst,imm` | plain 16-bit two's-complement (`1234+111=1345`; `5-8=FFFD`) |
| `CASE @cell` | PC += 2×value — a table of 2-byte `BR` entries follows; value 2 picked the 3rd ✅ |
| `CZ @cell` | condition := (byte == 0); `DCZ` word form |
| `CEQ/CH/CHE/CGT/CGE dst,src` | condition := compare(dst,src); `CH`>logical, `CGT`>arithmetic; imm and mem source forms both live (menu trace uses `DCGT`, `CEQ` verified on CPU **and VDP** operands) |
| `*@cell` (CPU indirect) | final = `>8300 +` **byte** at cell — scratchpad-only pointers; `ST *@>8356,>55` verified |
| `*V@cell` (VDP indirect) | final VDP addr = **word** at cell; the encoded field is the *cell's* `>8300` offset (`B0 56`) — verified read of a planted VDP byte |

### 7. Banned constructs (verified-failed or unverified — do NOT emit)

- **Indexed GAS (X bit)** — an indexed `ST` did not land per the documented
  semantics; the assembler now rejects `(…)` syntax. Use indirect instead.
- **MOVE C=1 (computed GROM source)** — no operand layout worked across the
  swept encodings (`move_c_probe`); the real menu doesn't use it either. Use
  the VDP-window strategy (§4).
- **FMT** — decode-only, assembler rejects (unchanged).

### 8. The full ISA is now in the toolchain ✅

`isa.rs` carries the complete opcode map from Classic99's table: format-1
two-operand families (`ADD SUB MUL DIV AND OR XOR ST EX CH CHE CGT CGE CEQ
CLOG SRA SLL SRL SRC`, byte/word `D`-prefix, imm/mem auto-selected), format-5
single-operand (`ABS NEG INV CLR FETCH CASE PUSH CZ INC DEC INCT DECT`),
`MOVE` with its exact `001GRVCN` bit field (all needed variants verified:
GROM→CPU, GROM→VDP, CPU→CPU, VDP→CPU, VDP→VDP, CPU-cart-window→CPU, →VDP-regs,
count-from-memory), and the named ops. The decoder tiles the authentic menu's
pre-launch stream byte-exactly (`decode.rs` tests).

---

## VERIFIED MECHANISMS — the 2026-07-02 M2 build-out ✅

Two facts pinned while making the selection list (M2) actually list and launch
carts on the real ROM. Both cost real debugging; record them here.

### 9. `SCAN` needs a **keyboard scan-code table in GROM** — CRITICAL ✅

The console ROM's `KSCAN` (which the GPL `SCAN` opcode calls) converts the key
matrix to ASCII by indexing a table **in the console GROM at a fixed address**.
A GROM rewrite that omits it makes *every* keypress decode to `>00`, so the
menu can't read a selection and TI PYTHON can't read input. Verified by
executing `SCAN` against the authentic GROM (`>8375` = `>32` for `'2'`) vs. our
early rewrite (`>8375` = `>00`); `examples/keymap_probe.rs` presses each key and
records the offset the ROM reads:

- **unshifted table at GROM `>1705`**, **shifted table at GROM `>1735`**, each
  43 bytes (scan codes 5..=47), preceded by 5 unused entries — so the block is
  `>1700..=>175F`.
- The *layout* (which scan-code offset → which key) is the ROM's hardware
  interface, recovered by probe. The *values* are plain ASCII (`'2'`→`>32`,
  `'A'`→`>41`, shift-`'2'`→`'@'`…), reconstructed from the ASCII standard and the
  key legends — **no TI bytes copied** (a key→ASCII table is a functional
  interface, like the `>AA` header format).

Our rewrite generates and splices this table from `crates/libre99-gpl/src/keymap.rs`
(mirrors the font splice). The earlier note that "`>8375` holds the ASCII key"
(§1) is true **only with this table present**.

### 10. GROM→VDP `MOVE` is slow — the console ROM re-writes the GROM address per byte ✅

Tracing a big `MOVE >0800,G@base,V@dst` shows the ROM writes the *full* GROM
address (`>9C02` high+low) for **every byte** copied, not once with
auto-increment. A 2 KiB window therefore costs ~2048 address writes (~250
cycles/byte), so copying all six 8 KiB cartridge slots would take ~70 frames and
looks like a hang. The menu instead (a) peeks 2 header bytes per base and skips
absent carts, and (b) copies only a **512-byte window** (`V@>1000..>11FF`) of
present carts, walking the program list in that copy with a bound guard. This is
not an emulator bug (a single `MOVE` and back-to-back `MOVE`s work; the earlier
"second MOVE fails" symptom was just insufficient frames). Tests give the menu
build a generous frame budget for the same reason.

### 11. The boot must **enable the VDP interrupt** or the ISR never runs — CRITICAL ✅

The console ROM's VBLANK **interrupt service routine** (which drives GPL sound
lists, sprite auto-motion, cursor/timers, and QUIT — finding F3) only fires once
the **9901's VDP interrupt is unmasked (CRU bit 2)**. The console ROM does **not**
do this on its own: the **console GROM's boot GPL** does, and a rewrite that
omits it boots to a console whose ISR is dead — silent, no QUIT, no sprite motion
(the interrupt counter `>8379` never advances; `tms9901.vdp_interrupt_enabled()`
is false; `interrupt_line()` stays `None` even though `vdp.interrupt_pending()`
is true). Verified by differential trace: the authentic boot writes CRU bit 2 at
ROM `>0604` (`SBO`, `R12=>0004`) via a GPL `IO`; our early rewrite issued zero
CRU writes.

The mechanism is the GPL **`IO`** opcode (`>F4–F7`, CRU I/O). To output to the
CRU, point its **destination at a list** and use **source function 3 = CRU
output**. The list is **FOUR fields** (first pinned by
`examples/cru_experiment.rs`; the fourth field re-pinned 2026-07-03 by
`libre99-core/examples/f5_mask_bisect.rs` after the F5 field bug — the earlier
three-field model was an artifact of zeroed RAM):

```
        ST   @>8300,>FF        ; the data byte (bit 0 drives a 1-bit write; non-zero = SBO)
        DST  @>8302,>0002      ; word: the CRU bit address (2 = VDP interrupt)
        DST  @>8304,>0100      ; count byte (1 bit) + data-ADDRESS byte (>00 -> data at >8300)
        BYTE >F6,>02,>03       ; IO @>8302,#3  -> SBO CRU bit 2
```

i.e. list `{ address-word @dst, count @dst+2, data-address @dst+3 }` — the byte
at dst+3 is a **`>83xx` offset through which the ROM fetches the data byte**;
the data lives at `>8300` only while that pointer byte is `>00` (which zeroed
emulator RAM made true on every cold boot, masking the field ✅ case study 9).
**Initialise every field, every boot**: after F5 (a CPU-only reset — RAM
survives) a game's leftover `>8305` pointed the fetch at a stale even byte and
the arming `IO` did `SBZ`, disarming the very interrupt it was enabling (gate
`crates/libre99-gpl/tests/f5_reset.rs`). The second operand byte `>03` is the
function code (CRU output), **not** a memory operand — our disassembler models
`>F6` as GasGas, which is why it prints `IO @>8302,@>8303`, but the ROM reads
that byte by value. The data byte's low bit must be set to `SBO` — a zero bit
does `SBZ` and silently *clears* (the authentic firmware sets its data with
`INV @>8300`); counts > 1 are unprobed ❓. `console.gpl` `START` now writes all
four fields; see [`DEBUGGING.md`](./DEBUGGING.md) case study 9 for the trace.

---

## Structure-handoff audit — every field our GPL hands the kept ROM ✅ (2026-07-03)

Three field bugs (case studies 6/7/9) shared one root: **the kept ROM reads a
scratchpad/VDP field our GPL did not write on some path, so after F5 (CPU-only
reset — RAM survives) the service consumes the previous program's leftover
value.** This table enumerates every structure our `console.gpl` hands a kept-ROM
service, so that class **cannot rot** (QUALITY-ASSESSMENT §7.8 amendment 1 /
Chunk 2). Verdicts marked ✅ are execution-verified by the cited gate/probe; ⚙
are static reasoning (a reaching-path grep, not yet a gate).

| Structure | Kept ROM reads | Written on every reaching path? | Verdict |
|---|---|---|---|
| **`IO` arming list** `>8300`/`>8302-3`/`>8304`/`>8305` | all four fields (§11) | `START` writes all four immediately before the `BYTE >F6` (straight-line, so cold **and** F5) | ✅ OK — `tests/f5_reset.rs` (case study 9) |
| **ISR workspace seed** `>837A`/`>83C2`/`>83C4`/`>83CC`/`>83CE`/`>83D4`/**`>83D6`** | ISR reads all every frame | six seeded in `START`'s ISR-init; **`>83D6` never written** (see note below) | ✅ OK — but `>83D6` is faithful-un-seeded, `blank_timeout_probe` |
| **Sound-list handoff** `>83CC/D` ptr, `>83CE` count, `SND`/`KBEEP` data | ISR walks the list | seeded then armed; list data is static GROM (F5-immune) | ✅ OK — `tests/interrupts.rs` (case study 7) |
| **Keyboard `SCAN` cells** `>8374` mode, `>8375` result | `SCAN` reads mode; `>FF` preset for the no-key test | `>8374=0`+`>8375=>FF` set before every SCAN loop | ✅ OK — `tests/keyboard.rs`, `f5_reset.rs` |
| **`XML >F0` ML vector** `>8300/1` | ROM reads the entry word | `DST @>8300,@>8344` the instruction before `XML >F0` | ✅ OK — `tests/sweep.rs` |
| **GPL sub-stack launch frame** `>8380/1`, `>8373` | `RTN` pops `[>8373]`→`>8380` | both written the two instructions before `RTN`; `>8373` also seeded at boot | ✅ OK — `tests/sweep.rs` |
| **Peripheral power-up `XML >19/1A`** `>8370`/`>83D0`/`>836D`/`>8355`/`>8372` | ROM device scan + DSR power-up | all set before the loop; `>8372` set inside `PUCALL` before `XML >1A` | ✅ OK — `tests/device_io.rs` |
| **DSRLNK `XML >19/1A`** name-parse cells, `>83D0`, `>836D`, caller PAB `>8356` | ROM device search + DSR call | DSRLNK re-inits its scratch on entry; `>8356` is the cart's to fill | ⚙ OK (see confidence note) |

**Note — `>83D6/7` (ISR screen-blank timeout) is un-seeded, and that is faithful,
not a bug.** `START`'s ISR-init seeds every neighbouring ISR cell but not
`>83D6`, which the ROM's ISR advances +2/frame and, on wrap-to-0, uses to blank
the display (a keypress resets it and un-blanks). The audit flagged this as a
case-study-9-class hazard on the strength of an *older* doc that marked `>83D6`
"init 0". **A differential probe (`examples/blank_timeout_probe.rs`) refuted the
divergence:** the authentic GROM does **not** seed `>83D6` either — after F5 with
`>83D6` pre-dirtied, both authentic and ours tick it up from the leftover value
identically (neither re-seeds), and both blank the title within a few frames when
`>83D6` is near wrap. So a rare post-F5 title-blank (needs an idle-before-F5 game
that left `>83D6` near `>FFFE`) is **shared with authentic and self-heals on the
keypress the title is already requesting** — faithful reproduction, below the
"1981 quality" fix bar (cosmetic, self-healing, matches authentic). Not fixed
(seeding it would *diverge* from authentic). Execution beats the literature here.

**Confidence.** Most-confident-OK: the `IO` list, `XML >F0`, sub-stack, `SCAN`,
and sound handoffs — each field written just before the handoff (or on the linear
boot path) and gated. Least-confident: the `XML >19/1A` device linkage — the
ROM's *complete* read-set for those two XMLs lives inside the kept ROM and isn't
enumerated field-by-field here, so "we write everything it reads" rests on
`tests/device_io.rs` (a cold disk load) rather than a field proof; a hidden
`>8305`-class field would most likely hide there. Closing it wants a wholesale
scratchpad diff (authentic vs ours) across a **warm** (F5) disk load — a Chunk 2
coverage-sweep candidate. Also examined and cleared (reasoning in the 2026-07-03
audit): `>83C6` mode-0 state (cleared empirically by `f5_reset.rs`), `>83C7–CA`
debounce (our key-waits are level-triggered), `>8378/9`/`>837B` (any value valid
/ ISR-overwritten), the sound-source FLAGS bit (ROM reset re-establishes it),
`>8372` (unused before it's set).

**Adjacent (out of the field-write class).** Menu dispatch does not run the
authentic pre-launch cleanup (§5: smear-clear VRAM, zero `>8300-836F`, reset the
colour table). That is "clean state handed to the *cartridge*", not "a field the
ROM reads", so it is outside this audit — but it is a real latent-compatibility
item (all 137 launchable carts tolerate it today — 137/137 list-and-launch since
L2's resolution 2026-07-03; M3 sweep) worth a look in the
Chunk 5 / close-out pass.

---

## Closed recon questions (kept as one-liners; details above)

- **GROM-cart GPL dispatch** — closed: the sub-stack trampoline, §3.
- **ROM-cart ML dispatch** — closed: vector at `>8300` + `XML >F0`, §2. (An
  early hypothesis dispatched through `>8380` and failed — that cell is the
  sub-stack slot the menu copies *from*, not the vector.)
- **R3 vector/subprogram histogram** — measured during the M3 sweep: several
  GPL carts *read* interconnect-table words (`>0016/17/18`) after launch. All
  135 launch and run (as of that sweep; 137/137 list-and-launch since L2's
  resolution 2026-07-03 — the one open post-launch exception is L8, Video
  Vegas), but two slots are not tolerate-the-zeros after all: they
  are the console **character-set loaders**, and a cart that draws text with the
  console font (TI Invaders) needs them — its text is blank without them (fixed;
  see "Console character-set loaders" above and [`LIMITATIONS.md`](./LIMITATIONS.md)
  L6). DSRLNK (`>0010`) is the other hard dependency (device I/O).
- **GAS address bias** — closed: the 12/16-bit CPU forms are biased `+>8300`
  (CPU `>6000` encodes as `>DD00`, confirmed by `ST @>83CE` → `80 CE`); VDP
  addresses are **not** biased (proved by the verified `MOVE`/`ST V@` forms).
- **SCAN debounce** — closed: see the key-wait idiom + nuance in §1 (the old
  "needs ISR setup" guess was wrong; so was distrusting the emulator).
