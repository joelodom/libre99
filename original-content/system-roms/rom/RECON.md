# RECON — the Console ROM interface dossier (D1)

Living record of what the **authentic** TI-99/4A console ROM (`994aROM.Bin`, 8
KiB at CPU `>0000–1FFF`) actually does — the **interface contract** the Phase-2
rewrite must honour, and the **complete element enumeration** the P9 completeness
mandate (the archived [`ROM-REWRITE-PLAN.md`](../history/ROM-REWRITE-PLAN.md) §0/§4) makes the source
of truth for every milestone gate. Sibling of the GROM dossier
[`../RECON.md`](../RECON.md); shared scratchpad/GPL-execution facts live there
and are cross-referenced, not duplicated.

**Clean-room discipline (plan P5).** Everything here is an *interface fact* or a
*behavioural specification* — addresses, table layouts, vector values, register
conventions, what each routine does and the cells it touches. It is **not** a
transcription of TI's instruction stream. We disassemble the authentic image
only to *recover the spec*; the implementation is written from this dossier, and
no TI code is copied. Addresses and table *contents* are uncopyrightable
functional facts (the same policy as the GROM keytab/header data).

**Confidence tags:** ✅ verified against `roms/994aROM.Bin` during this session
with our own `libre99asm dis` / word dumps · 📖 literature (TI Intern, Nouspikel,
E/A manual, Classic99 — consult never copy) · ❓ to be pinned by execution in
the implementing milestone. Reproduce any ✅ with
`cargo run -p libre99-asm --bin libre99asm -- dis roms/994aROM.Bin <hexaddr>` or an
`xxd -s 0x<off> -g2` word dump.

The oracle is pinned: `roms/994aROM.Bin`, 8192 bytes, sha256
`599da51e9e1968a806871d681f17b5acbb617accf07191891265aee44ebec2b6`.

---

## 1. The reset / power-up contract ✅

The CPU's reset loads **WP from `>0000`, PC from `>0002`** (`cpu.rs::reset`).
The authentic vector is **`WP=>83E0, PC=>0024`** ✅. `BLWP @>0000` is a soft
reset; the GPL `EXIT` opcode and the ISR's QUIT path both re-enter `>0024`, so
**warm reset ≡ cold reset** from the ROM's view.

The reset routine at `>0024` is deliberately tiny and touches **no** RAM, VDP,
or 9901 — all machine init is GROM-side GPL (📖 TI Intern; consistent with the
GROM dossier's R1 "scratchpad powers up random" warning). Its effect, as a
spec ✅ (disassembled `>0024`):

1. Establish the **GPL workspace** register images: R13 := `>9800` (GROM read
   port), R14 := `>0100` (high byte SPEED=1 at `>83FC`, low byte FLAGS=0 at
   `>83FD`), R15 := `>8C02` (VDP write-address port). These live in GPLWS
   `>83E0–83FF` (R13=`>83FA`, R14=`>83FC`, R15=`>83FE`).
2. Strobe the GROM data port once (a throwaway read — the GROM dossier's R1
   `>1FFF` observation), then write GROM address **`>0020`**.
3. Clear the GPL status byte `>837C` (via the `>006A` soft entry — see §3).
4. Fall into the interpreter main loop at `>0070`, whose first GPL fetch is
   GROM `>0020` — the console GROM's power-up routine.

⇒ **Our reset kernel must reproduce exactly this state and no more.** Anything
extra (a VDP write, a scratchpad clear) diverges from the authentic entry
contract and will fail the conformance diff / real-GROM boot.

---

## 2. Vectors and fixed public entry stubs (`>0000–0023`) ✅

Every word verified by dump. **Public addresses are frozen (P8).**

| Addr | Bytes | Meaning / contract |
|---|---|---|
| `>0000` | `83E0 0024` | **Reset vector**: WP=`>83E0`, PC=`>0024`. |
| `>0004` | `83C0 0900` | **Level-1 interrupt vector**: WP=`>83C0` (INTWS), PC=`>0900`. All 99/4A interrupts arrive here. |
| `>0008` | `83C0 0A92` | **Level-2 vector** → the ISR's screen-blank fragment; unreachable on this machine but present (P8 keeps the value). |
| `>000C` | `30AA` | Data word: high byte `>30` = a CPU-clock constant (some consoles `>28`); **low byte `>AA` at `>000D` is the standard-header marker the ISR compares card ROMs against** (`CB @>4000,@>000D`). |
| `>000E` | `0460 02B2` | **`B @>02B2` — the KSCAN entry.** ML callers use `BL @>000E` with WP=`>83E0` (never `BLWP`: R13/R14/R15 must hold the GPL port/flag values); KSCAN returns `B *R11`. 📖 caller idiom: `MOVB <mode>,@>8374 / LWPI >83E0 / BL @>000E / LWPI <own WS>`. |
| `>0012` | `0008` | Data (a KSCAN constant; also **harvested** — see §13). |
| `>0014` | `1E00 …` | **`SBZ 0`** prologue, falls into `>0016`. ✅ (resolves the §1-plan ❓). |
| `>0016` | `0460 007A` | **`B @>007A`** — enter the interpreter with the opcode already in R9. |
| `>001A` | `1E00 …` | **`SBZ 0`** prologue, falls into `>001C`. |
| `>001C` | `0460 0078` | **`B @>0078`** — enter the interpreter, fetch the next opcode (no interrupt window). |
| `>0020` | `0460 04B2` | **`B @>04B2` — the CLEAR/BREAK (FCTN-4) test.** E/A: `BL @>0020`, returns EQ set if pressed. |
| `>0024` | code | Reset / GPL `EXIT` target (§1). |
| `>0036` | `1000` word / code | Return stub from the never-released **extended-GPL card**: `SBZ 0 / LWPI >280A / RTWP` ✅. (The `>1000` word is also harvested as a CZC mask — §13.) |
| `>0040` | `280A 0C1C` | **XOP 0 vector** → the extended-GPL card at CRU `>1B00` (via `>0C1C`); crashes if the card is absent. Vestigial but present. |
| `>0044` | `FFD8 FFF8` | **XOP 1 vector** (top of 32K expansion RAM; "user defined"). |
| `>0048` | `83A0 8300` | **XOP 2 vector** (scratchpad; "user defined"; absent on some consoles 📖). |
| `>004C` | `1100` | Data: CRU **row mask for QUIT** detection (FCTN + `=`), read by the ISR (`CZC @>004C,R5`). |
| `>004E` | code | **`SWGR` (`>F8`) handler**, then the interpreter prologue falls toward `>0070`. |

---

## 3. The GPL interpreter (`>0070–08FF`) ✅

The ROM *is* the GPL interpreter. Structure verified by disassembly.

**GPLWS (`>83E0`) register roles** (📖 TI Intern + `disk.cpp` sanity checks,
✅ R13/14/15 by dump): source operand → R1 addr / R0 data; destination → R3
addr / R2 data; **R4** = VDP-RAM flags; **R5** = dispatch pointer / word-op
flag; **R8** = GROM search pointer, **cleared by the ISR every tick** (so
interpreter code must not keep state in R8); **R9 high byte = current opcode**;
R11 = BL return; R12 = CRU base; **R13=`>9800`** (all four GROM ports as
offsets: `*R13`=data `>9800`, `@>0002(R13)`=addr readback `>9802`,
`@>0400(R13)`=data write `>9C00`, `@>0402(R13)`=addr write `>9C02`);
**R14=`>0100`** (SPEED `>83FC` / FLAGS `>83FD`); **R15=`>8C02`** (VDP ports:
`*R15` write-addr `>8C02`, `@>FFFE(R15)` write-data `>8C00`, `@>FBFE(R15)`
read-data `>8800`, `@>FC00(R15)` status `>8802`).

**Main loop ✅** — `>0070`: `LIMI 2 / LIMI 0` (the *only* interrupt window, one
per GPL instruction); `>0078`: fetch the opcode byte from GROM into R9; branch
by sign: opcodes `≥>80` take one path, `<>80` index the **first-nibble table at
`>0C36`** by the top nibble. Warm entries `>0078` (fetch) and `>007A` (opcode
already in R9) are the `>001C`/`>0016` stub targets; `>006A` clears the GPL
status byte then enters the loop (the documented soft entry for
`GPLLNK`-from-assembly, 📖 gplcall).

**FLAGS byte `>83FD` (R14 low)** 📖: `>20` cassette-timer ISR active, `>10`
cassette verify, `>08` 16K VDP, `>02` multicolour, `>01` sound list is in VDP
(vs GROM).

**GPL status byte `>837C`** carries the condition bit (`>20`) plus H/GT/CARRY/OV
bits; `>00F4` is the shared "convert a status bit into the condition bit"
routine that H/GT/CARRY/OVF dispatch to. The exact **per-opcode set/clear rules
are the M1 correctness hazard** and are pinned by the M1 microsuite (§ element
enumeration); the shared ISA table (`../RECON.md` §8, `crates/libre99-gpl/src/isa.rs`)
gives the opcode families and MOVE bit field.

### Dispatch tables (`>0C36–0CF9`) ✅ — all contents verified by dump

The interpreter is table-driven from six tables. Their **contents are the
functional spec**; ours carry equivalent targets (our interior addresses, but
the same dispatch structure and the same table-F XML vector). Entries below are
authentic *targets* recorded as facts; "→" is the routine the entry selects.

| Table | Addr | Selects (by field) |
|---|---|---|
| First-nibble (`<>80`) | `>0C36` | nibble 0–1→specials `>0270`, 2–3→`MOVE >061E`, 4–5→`BR >011A`, 6–7→`BS >010E` |
| Special ops `>00–1F` | `>0C3E` | RTN`>0838` RTNC`>083E` RAND`>027A` SCAN`>02AE` BACK`>029E` B`>0104` CALL`>085A` ALL`>05A2` FMT`>04DE` H`>00F4` GT`>00F4` **EXIT`>0024`** CARRY`>00F4` OVF`>00F4` PARSE`>18C8` XML`>0608` CONT`>1920` EXEC`>1968` RTNB`>19F0` RTGR`>082C`; `>14–1E`→ext`>0C0C`, `>1F`→`>0C14` |
| Two-op `≥>80` | `>0C7E` | ABS`>0136` NEG`>013A` INV`>0140` CLR`>013E` FETCH`>0144` CASE`>0162` PUSH`>016E` CZ`>00EA` INC(→SUB`>0186`) DEC(→ADD`>0188`) INCT`>0184` DECT`>0182`; ADD`>0188` SUB`>0186` MUL`>01CE` DIV`>01EA` AND`>0190` OR`>0196` XOR`>019A` ST`>019E` EX`>01A2` CH`>00D6` CHE`>00DA` CGT`>00DE` CGE`>00CC` CEQ`>00EC` CLOG`>00E2` SRA`>01B0` SLL`>01B4` SRL`>01B8` SRC`>01C2` COINC`>06D2`; IO`>05C8` SWGR(→`>004E`); ext blocks → `>0C0C` |
| MOVE | `>0CCE` | src CPU`>0660` / GROM`>0672` / VDP`>0664`; dst CPU`>0682` / GROM`>0686` / VDP`>06BA` / VDP-reg`>0698` |
| FMT | `>0CDC` | 0/1→`>050A` 2/3→`>0508` 4/5→`>0504` 6/7→`>0502` 8/9→`>0534` A/B→`>0532` C/D→`>053A` E/F→`>056C` (see §7) |
| IO | `>0CEC` | 0/1 sound(GROM/VDP)→`>05D6`, 2 CRU-in→`>05E8`, 3 CRU-out→`>05EA`, 4 cassette-write→`>1346`, 5 read→`>142E`, 6 verify→`>1426` |

**`MOVE` variants** (a big correctness surface): source/dest ∈ {CPU RAM, VDP,
GROM/GRAM, VDP-register}, count immediate or from memory, plus the **per-byte
GROM re-addressing** on GROM sources (the slow path the GROM dossier §10
documents — reproduce it, P4). Every live combination gets an M1 microtest.
The authentic operand decoder also implements **indexed GAS** and **`MOVE` C=1
(computed GROM source)** — the two forms the GROM track *banned emitting* after
its probes (GROM `../RECON.md` §7). Under P9 the *interpreter must execute them
correctly* even though our GROM never emits them; D1's decoder analysis + M4
probes pin the true semantics ❓.

---

## 4. `>0070`-family + service entry contracts (ML clients) ✅📖

- `BLWP @>0000` soft reset; `BL @>000E` KSCAN; `BL @>0020` CLEAR test.
- `>0016`/`>001C` return-to-interpreter stubs (opcode-in-R9 / fetch) ✅.
- `XML >F0` → the RAM vector at **`>8300`** (XML master table entry F) — the
  ML-cartridge launch trampoline and general ML↔GPL bridge (GROM `../RECON.md`
  §2 ✅).
- **XMLLNK/GPLLNK conventions**: `>83FA`/`>8372` interpreter cells; the `>006A`
  soft entry for calling GPL from assembly (📖 gplcall).

---

## 5. KSCAN (`>02B2`) ✅ + the GROM table dependency

Entry `>02B2` (behind `>000E`), on GPLWS. Verified head by disassembly:
saves R11 → `>83D8`, `SBO 21` (alpha-lock select), reads the mode byte `>8374`,
dispatches on it.

**Modes (`>8374`)** ✅ mask logic + 📖 semantics: **0** = scan in the console's
current/default state (kept at `>83C6`); **1** = left split (mask `>0FFF`) +
joystick 1; **2** = right split (mask `>F0FF`) + joystick 2; **3/4/5** = full
scan in the 99/4 / Pascal / 99-4A-native **translation states** — the code
subtracts 3 and stores the result back into `>8374` *and* `>83C6`; values `>5`
return immediately.

**Cells**: result `>8375` (`>FF` = none), joystick Y `>8376` / X `>8377`,
condition bit (`>837C` `>20`) set only on a **new** key; debounce/last-scan-code
in **`>83C6–83CA`** (E/A: do not clobber). **Return-save `>83D8`.** On a
detected key KSCAN **reloads VDP R1 from the `>83D4` copy** (un-blank) and
resets the `>83D6` timeout.

**Hardware scan** 📖: column select = 3 bits `LDCR` at CRU base `>0024` (9901
P2–P4 → 74LS156); rows = 8 bits `STCR` at base `>0006`, active-low; columns 0–5
keys, 6/7 joysticks; alpha-lock select on P5 (`>002A`).

**GROM table dependency (critical).** KSCAN translates via tables in **GROM 0**,
read through the GROM ports — deflection `>16E0`, shift `>1730`, FCTN `>1760`,
split `>17C0` (📖 TI Intern). ⚠ These are *GROM-side* (both our GROM and the
authentic GROM ship them — GROM `../RECON.md` §9 records the observed
first-entry offsets `>16EA/>1705/>1735/>1765/>1795/>17C8`); the ROM's job is to
**index them identically**. Reconcile the two address sets in M2.

**CLEAR test (`>04B2`)** ✅ (target of `>0020`): two-column CRU probe (col 0 then
col 3, mask `>1000`), returns EQ set if FCTN-4 held; `B *R11`.

---

## 6. The VBLANK ISR (`>0900–0ABF`) ✅ — duty order verified by disassembly

Entry `>0900`, WP=`>83C0` (INTWS). Verified sequence ✅:

1. `LIMI 0`; `LWPI >83E0` (runs on GPLWS!).
2. `COC @>0032,R14` — test FLAGS bit `>20` (cassette-timer mode): if set →
   `B @>1404` (the cassette bit-timing ISR, §11).
3. `TB 2` — 9901: did the VDP cause the interrupt? If **not** → the
   **peripheral-card ISR scan**: for CRU bases `>1000..>1F00` (`LI R12,>0F00`,
   `AI >0100` loop to `>2000`), enable the card, compare its `>4000` header byte
   against `@>000D` (=`>AA`), follow the ISR-chain pointer at card `>400C`,
   `BL` each link; then exit.
4. If VDP (`>094A`): the `>83C2`-gated duties, **in this order** — unless
   `>83C2 & >80` (skip all): (a) **sprite auto-motion** unless `>40` — count
   `>837A`, motion table at VDP `>0780`; (b) **sound list** unless `>20` —
   countdown `>83CE`, next block via pointer `>83CC` from GROM/VDP per FLAGS
   `>01`, bytes to the sound chip `>8400`; (c) **QUIT** unless `>10` — read the
   key column, `CZC @>004C` (mask `>1100`), if FCTN+`=` down → `BLWP @>0000`.
5. **VDP status read → `>837B`** (`MOVB @>FC00(R15),@>837B`) — this is what
   clears the VDP interrupt line; it happens *after* the duties.
6. `LWPI >83C0`; **screen-timeout**: `INCT` the counter at INTWS R11 = `>83D6`
   (+2/tick); at zero, fall into `>0A92` — rebuild VDP R1 from the `>83D4` copy
   with the blank bit cleared and write it (screen blank).
7. `LWPI >83E0`; **`AB R14,@>8379`** — add SPEED (`>83FC`) to the VDP-interrupt
   timer `>8379` (the ISR-liveness signal).
8. **User hook**: `MOV @>83C4,R12`; if non-zero, `BL *R12` (runs on WP=`>83E0`,
   returns `B *R11`).
9. **`CLR R8`** (GPLWS R8 zeroed every interrupt); `LWPI >83C0`; `RTWP`.

**`>83C2` disable bits** 📖: `>80` skip all VDP duties, `>40` no sprite motion,
`>20` no sound, `>10` no QUIT. **INTWS `>83DA–83DF`** hold the interrupted
WP/PC/ST (the RTWP frame). The GROM-side arming of the 9901 VDP interrupt (CRU
bit 2) is the GROM dossier's §11 / the F5 case study 9 (the four-field `IO`
list) — the ROM's ISR is dead until the boot GPL arms it.

---

## 7. FMT — the screen-format sub-interpreter (`>04DE–05A1`) ✅ (M4 2026-07-05)

`FMT` (`>08`) switches the interpreter into an independent sub-language until
FEND (regular GPL opcodes are invalid inside it). Entry `>04DE` (pinned, P8);
dispatch table **`>0CDC`** (8 words at their authentic home, our handler
addresses). Cursor kept at `>837E` (row) / `>837F` (col); the target cell is the
linear name-table address `row*32 + col`, **base VRAM 0** (like ALL, §18 — reg2
does not move it), with a single 768-cell wrap. **Grammar pinned by
disassembling the authentic `>04DE-05B7` as a spec (P5)** and reproduced clean —
every sub-op differentially gated by `libre99-asm/tests/gpl_fmt.rs` (15 cases).

Each format byte's **top three bits** select a group via `>0CDC` (the 9900 word
access pairs odd/even nibbles: `0/1→>050A 2/3→>0508 4/5→>0504 6/7→>0502
8/9→>0534 A/B→>0532 C/D→>053A E/F→>056C`); the **low five bits** are a count
`n` (→ `n+1`) or, for the E/F group, a control selector:

| Byte | Sub-op | Operand + action |
|---|---|---|
| `00-1F` | HTEXT | `n+1` inline chars, written horizontally (+bias each), advance 1 cell |
| `20-3F` | VTEXT | `n+1` inline chars, written vertically (advance 1 row = 32) |
| `40-5F` | HCHAR | next byte = char, repeated `n+1×` horizontally (bias added once) |
| `60-7F` | VCHAR | next byte = char, repeated `n+1×` vertically |
| `80-9F` | HMOVE | advance the cursor `n+1` columns, no output (single `≥>0300` wrap) |
| `A0-BF` | VMOVE | advance the cursor `n+1` rows |
| `C0-DF` | RPTB | open a repeat block of `n+1` passes: push `255-n` on the GPL sub-stack (`>8373`); the block-end adds SPEED each pass until the byte wraps to 0 |
| `E0-FA` | (string) | a GAS operand → an address; emit `n+1` chars from it (+bias), horizontal |
| `FB` | FEND | outside a RPTB: store the cursor, return to the interpreter (`>0070`); inside one: read the 2-byte loop-back GROM address, tick the counter, re-point or pop |
| `FC` | BIAS | the next inline byte becomes the char bias (added to every emitted char, 8-bit wrap) |
| `FD` | BIAS | the byte at a GAS operand becomes the bias |
| `FE` | ROW | the next byte sets the cursor row (`>837E`) |
| `FF` | COL | the next byte sets the cursor column (`>837F`) |

The char-write wrap is `[>0300,>0320) → −>0300` (covers a horizontal +1 and a
vertical +32 overshoot); the HMOVE/VMOVE path takes the looser single `≥>0300`
subtract. **Every sub-op is an M4 element** (our GROM never emits FMT, but
cartridges/BASIC use it — P9). Verified: Parsec / TI-Invaders now launch under
our ROM (`sweep.rs`), and ToD's selection screens paint (its later LOAD path
next needs MOVE C=1 — `device_io.rs`).

---

## 8. XML system (`>0608` dispatch, master `>0CFA`, tables 0/1) ✅

`XML` (`>0F`) operand byte `>XY`: X selects a table pointer from the **master
table at `>0CFA`**, Y the entry. Master table ✅ (16 words):

```
>0  >0D1A (ROM FLTAB)     >8  >6030
>1  >12A0 (ROM XTAB)      >9  >7000 (cartridge)
>2  >2000                 >A  >8000 (→ scratchpad on the 4A)
>3  >3FC0                 >B  >A000
>4  >3FE0 (low-mem exp)   >C  >B000
>5  >4010                 >D  >C000
>6  >4030 (DSR card)      >E  >D000
>7  >6010                 >F  >8300 (scratchpad — the XML >F0 vector)
```

Tables 2–E point at RAM/card/cartridge addresses **with no ROM content behind
them** — E/A low-memory utilities and cartridge XML tables work unchanged; only
tables 0/1 (and the table-F vector) are ROM-resident work.

**XML table 0 / FLTAB (`>0D1A`)** ✅ — 16 words; the floating-point dispatch.
Entry `>00`=`>0000` (an `XML >00` branches to the reset vector — a reset/crash;
reproduce faithfully). `>01` ROUND1`>0F54` · `>02` ROUND`>0FB2` · `>03`
**STST`>0FA4`** (binary; Nouspikel's `>0F4A` is a typo — the dump settles it) ·
`>04` OVEXP`>0FC2` · `>05` OV`>0FCC` · `>06` FADD`>0D80` · `>07` FSUB`>0D7C` ·
`>08` FMUL`>0E88` · `>09` FDIV`>0FF4` · `>0A` FCOMP`>0D3A` · `>0B` SADD`>0D84` ·
`>0C` SSUB`>0D74` · `>0D` SMUL`>0E8C` · `>0E` SDIV`>0FF8` · `>0F` SCOMP`>0D46`.

**XML table 1 / XTAB (`>12A0`)** ✅ — 12 meaningful entries `>10–1B` then 4
**vestigial** `>1C–1F`: `>10` CSN`>11AE` · `>11` CSNGR`>11A2` · `>12` CFI`>12B8`
· `>13`→`>1648` · `>14`→`>164E` · `>15`→`>1642` · `>16`→`>15D6` · `>17`→`>163C`
(symbol-table package trampolines: SMB`>1670`, SYM`>176A`, ASSGNV`>1788`,
VPUSH`>1EAA`) · `>18` VPOP`>1F2E` · **`>19` SROM`>0AC0`** · **`>1A` SGROM`>0B24`**
· `>1B` PGMCH`>1868`. **`>1C–1F` = `C120 834A 1342 04C0`** ✅ — these index
*past* the table into CFI's first instruction words (RAM/mid-code), i.e.
undefined targets. Under P9 they are reproduced **behavior-faithfully** (decode
+ dispatch exactly as authentic; a garbage XML there does what the authentic
ROM does).

---

## 9. Floating point (`>0D3A–~11A1`, + conversions `>11A2–1345`) 📖✅tables

**Format** 📖: radix-100, 8 bytes — exponent byte biased `>40`, 7 mantissa
bytes of 0–99 (~14 digits); negatives negate the first word; zero = first word
`>0000`. **Scratchpad interface** 📖: **FAC `>834A–8351`**, **ARG `>835C–8363`**,
**error byte `>8354`** (also the DSR name-length cell), sign `>8375`, exponent
copy `>8376`, **VDP value-stack pointer `>836E`** (S-ops auto-adjust by 8),
condition/error signalled in `>837C`, FP-error GROM addr `>836C`. **Error codes
(`>8354`)** 📖: `>01` overflow/÷0 · `>02` syntax · `>03` integer-overflow (CFI) ·
`>04` √neg · `>05` neg^non-integer · `>06` log(0)/log(neg) · `>07` trig.

**In this ROM** (the M5 surface): FADD/FSUB/FMUL/FDIV/FCOMP + the S-variants
(VDP value-stack forms), ROUND/ROUND1/STST/OVEXP/OV (table 0), CSN/CSNGR/CFI
(table 1). **Transcendentals (SIN/COS/TAN/ATN/LOG/EXP/SQR/PWR) are GROM-0 GPL,
NOT in this ROM** (§0 exception 2) — out of scope, not a gap. Every ROM FP
routine + every error path is an M5 element, bit-exact vs authentic ❓ (pinned
by planted-operand microtests in M5).

---

## 10. Device linkage — SROM / SGROM (`>0AC0` / `>0B24`) 📖✅home

`XML >19` = **SROM `>0AC0`**: search peripheral-card ROM headers (CRU
`>1000–1F00`) for DSR / subprogram / power-up chains — the console-internal
DSRLNK. Inputs 📖: DSR name length at `>8354`/`>836D` (`>08` for a DSR call),
name pointer `>8356`; cursor cells `>83D0` (card CRU base) / `>83D2` (DSR entry).
`XML >1A` = **SGROM `>0B24`**: the same over GROM standard headers (power-up /
program / DSR / subprogram). **DSR call invariants** 📖 (Classic99 `disk.cpp`):
run on GPLWS `>83E0`, R12 = card CRU base, R13/R15 = GROM/VDP ports intact,
`>83D2` = DSR entry, **interrupts masked** (`LIMI 0`) for the call. The standard
`>AA` header format (offsets +4 power-up / +6 program / +8 DSR / +A subprogram /
+C card-ISR list) is the GROM dossier's header contract. Our GROM's DSRLNK (M7)
already delegates to `XML >19/>1A` in the kept ROM — the ROM side is these two
routines. M3 element.

---

## 11. Cassette bit engines (`>1346–15D3`) 📖 — hardware-gated (§0 exception 1)

The console-ROM cassette **modem layer**: write `>1346` (GPL `I/O 4`), read
`>142E` (`I/O 5`), verify `>1426` (`I/O 6`), plus the **cassette timer ISR
`>1404–1422`** the level-1 ISR forwards to when FLAGS `>20` is set. Reachable
**only via GPL `I/O`** — the user-visible CS1/CS2 **DSRs are GPL in GROM 0**
(`>1320–16DC`), which issue these `I/O` ops. Encoding 📖: FSK, ~725 µs cells
(689 Hz `0` / 1379 Hz `1`), 9901 decrementer loaded `>0011` per half-cell;
records written twice. CRU bits 📖: 22 CS1 motor, 23 CS2 motor, 24 audio gate,
25 mag-out, 27 tape-in.

**Disposition (P9 exception 1):** the emulator has no tape hardware
(`crates/libre99-core/src/cru.rs`), so the transport can't move bytes. The ROM code
is **written and behavior-correct** (a `DSRLNK("CS1…")` errors the authentic
way, never hangs — GROM chunk-3a already showed the kept ROM does this); only the
byte transfer is inert until tape hardware exists (ROADMAP §6). The engines are
**present and complete**, gated only on the missing device.

---

## 12. The TI BASIC ROM half (`>15D6–1FFB`) 📖 — the M6 surface

The ROM and TI BASIC's GROMs 1–2 are **co-designed**; this half is what makes
the authentic GROM run TI BASIC on our ROM (the M6 acid test). Contents 📖:
- **BASIC-support XMLs** `>15D6–18C7`: symbol-table search/assignment
  (SMB`>1670`, SYM`>176A`, ASSGNV`>1788`), PGMCH`>1868`, trampolines
  `>163C–164E` for `XML >13–17`.
- **The interpreter core** `>18C8–1FFB`: PARSE`>18C8`, CONT`>1920`, EXEC`>1968`,
  RTNB`>19F0`; statement entries `>19E6–1A2C` (DEF/DIM/DATA/REM/OPTION/END/STOP/
  GO/ON/GOSUB/GOTO/RETURN/IF/LET/NEXT); **jump tables `>1C9C–1DE2`** where an
  MSB-set entry means "the handler is GPL in BASIC's GROMs" (📖 TI Intern);
  support subroutines; VPUSH`>1EAA` (`XML >17`), VPOP`>1F2E` (`XML >18`).
- Trailing words `>1FFC` = `2A61 A38A` (📖 "checksum"; ❓ never read — D2's
  data-read census confirms; our tail ships all-zero, §10.5).

`PARSE/CONT/EXEC/RTNB` are reached via the special-op dispatch (§3). Every
BASIC-ROM entry the GROM exercises is an M6 element, verified by TI-BASIC
differential smoke scripts.

---

## 13. NASTY — code words harvested as data constants ✅📖 (the P8 hazard)

The authentic ROM reads several of its own **instruction words as data** — move
any of them and an unrelated routine silently breaks. Our source must place each
as an **explicit named constant at the identical address** (P8), never as an
accidental encoding. Confirmed/derived set (📖 Lee Stewart's OS project; ✅ where
dumped):

| Cell | Value | Harvested as | Also is |
|---|---|---|---|
| `@>0012` | `>00` (byte) | CLEAR-test column selector | KSCAN constant |
| `@>0032` | `>0020` | ISR cassette-flag `COC` mask | the `LI R0,>0020` immediate in reset |
| `@>0036` | `>1000` | CLEAR-test `CZC` row mask | a `JMP` opcode in the ext-GPL stub |
| `@>0072` | `>0002` | KSCAN mode compare constant | the `LIMI 2` word |
| `@>0074` | `>0300` (→`>03`) | CLEAR-test second column value | the `LIMI 0` word |
| `@>004C` | `>1100` | ISR QUIT row mask | (dedicated data) |
| `@>000D` | `>AA` | card-header marker (ISR/SROM) | (dedicated data) |
| `@>011B` | mask byte | reset status-clear `SZCB` mask | inside the `BR` handler |

D1's full enumeration (M-milestone deliverable) confirms the complete list from
the disassembly; **each becomes a `DATA`/`EQU` at its exact address.**

---

## 14. Scratchpad conventions (the ROM's view) 📖

The ROM reads/writes only `>83xx`; the authoritative cell map is the GROM
dossier `../RECON.md` scratchpad map + Nouspikel padram. ROM-owned cells named
above: FAC `>834A`, error/name-len `>8354`, name-ptr `>8356`, ARG `>835C`,
value-stack `>836E`, KSCAN mode/key/joy `>8374–7`, RND `>8378`, ISR timer
`>8379`, sprite count `>837A`, VDP-status copy `>837B`, GPL status `>837C`,
cursor `>837E/F`, INTWS/seed `>83C0`, ISR gates `>83C2`, user hook `>83C4`,
KSCAN state `>83C6–CA`, sound `>83CC/CE`, DSR-search `>83D0/D2`, VDP-R1 copy
`>83D4`, timeout `>83D6`, KSCAN return `>83D8`, RTWP frame `>83DA–DF`, GPLWS
`>83E0–FF`. The reset kernel seeds only the GPLWS register images (§1); the rest
is GROM-side.

---

## 15. The complete element enumeration (the P9 spine) — the source of truth

This is the authoritative checklist the milestone gates mirror to a Rust
`const` and the **coverage-completeness test cross-checks against** (§0
definition-of-done; a gate fails if any element lacks a differential test). It
enumerates *every functional element* of the authentic ROM, regardless of usage.
Status legend: ⬜ not yet implemented, 🔨 in progress, ✅ implemented + gated.

**GPL interpreter (M1/M4)** — *status 2026-07-04:* the format-1 + format-5
families, BR/BS/B/CALL/RTN/RTNC, RAND, BACK, H/GT/CARRY/OVF, EXIT, ALL, and
the per-opcode condition-bit rules ✅ (§§16–18); MOVE ✅ (all combos incl.
**C=1 computed-GROM source**, **GRAM dest**, and **indexed GAS** — M4, gated in
`gpl_core.rs`; the index adds the >8300-indexed word to the base, authentic
`>0758`/`>07D2`); **M4 COMPLETE 2026-07-05** — see §§25-26 (the GRAM-dest
gates pin the control flow); FMT ✅ (§7); **COINC / SWGR / RTGR ✅** (M4 slice
3, §25 — with the driver's uniform imm/mem source discipline realigned);
IO ✅ (M4; §25, cassette §26) (sound 0/1 ✅ M2; **CRU-in/out ✅** §25 — the
synthesized LDCR/STCR engine, counts pinned; cassette 4/5/6 ✅ done (§26));
XML ✅ (M3+M5; dispatch + master/FLTAB/XTAB tables ✅ §21; SROM/SGROM ✅ §24;
the XTAB `>1C-1F` vestige ✅ §25; FP/conversion entries ✅ M5 §27; the M6
symbol entries stay loud stubs by policy); SCAN ✅ (§§22–23);
**PARSE / CONT / EXEC / RTNB ⬜ — the M6 BASIC surface, deferred indefinitely
by policy** (the plan's M6 deferral note):
- The 256 opcodes, by family (full map: `crates/libre99-gpl/src/isa.rs`,
  `../RECON.md` §8): format-1 two-operand `ADD SUB MUL DIV AND OR XOR ST EX CH
  CHE CGT CGE CEQ CLOG SRA SLL SRL SRC` (byte + `D` word, imm + mem source);
  format-5 single-operand `ABS NEG INV CLR FETCH CASE PUSH CZ INC DEC INCT
  DECT`; named ops `RTN RTNC RAND SCAN BACK B CALL ALL FMT H GT EXIT CARRY OVF
  PARSE XML CONT EXEC RTNB RTGR SWGR COINC IO`; branches `BR BS`.
- Every **addressing form**: register-direct, `*Rn`, `*Rn+`, `@sym`, `@sym(Rn)`
  for the CPU/VDP/GROM spaces; **indexed GAS** and **MOVE C=1 computed-GROM**
  (the two the GROM banned emitting — interpreter must execute them).
- Every **MOVE** src×dst combination + count-imm/count-mem, incl. per-byte GROM
  re-addressing.
- The **condition-bit set/clear rule for each opcode**.
- `RAND` PRNG algorithm + `>83C0` seed (bit-exact, §10 decision 4).

**FMT sub-language (M4)** — every sub-op of the `>0CDC` table. *Status
2026-07-05:* ✅ — the full sub-interpreter (HTEXT/VTEXT/HCHAR/VCHAR, HMOVE/VMOVE,
RPTB, and the E/F control group: string-from-operand, FEND, BIAS ×2, ROW, COL),
grammar pinned in §7, gated by `gpl_fmt.rs` (15 cases). Entry `>04DE` pinned;
body free-placed (Zone I, layout ledger).

**KSCAN (M2)** — modes 0–5, each translation state, split-keyboard masks,
joystick columns, alpha-lock, debounce cells, VDP-R1 reload + timeout reset,
the CLEAR test. *Status 2026-07-05:* ✅ (all modes) — mode 0, modes 3–5
store-back + the state-gated result normalization/alpha-lock read (§§22–23),
and **split modes 1/2 + joysticks** (mode dispatch, split mask `SZC R0,R4`, the
`>17C0` split base, unit-indexed debounce, joystick deflections `>16E0`→
`>8376/77`, centered defaults), plus un-blank and the CLEAR test. One deferral:
the full-scan split-cell coherence write to `>83C9/CA` (authentic `>03CA`) — no
flow uses it; the bounded fuzz would surface it. Gates: `rom_kscan.rs` (11).

**ISR (M2)** — every duty in the §6 order; every `>83C2` gate bit; the card-ISR
scan; the sound-list, sprite-motion, QUIT, status-read, timeout, timer, and
user-hook steps; the cassette-timer-mode fork. *Status 2026-07-05:* ✅
(bare-console duties) — the full gated duty structure, QUIT, status-read,
timeout-blank, SPEED timer, user hook, `CLR R8` (§20), **sound-list processing**
(authentic `>09EC`, all block/edge forms) and **sprite auto-motion** (authentic
`>095C`, fixed-point + edge-wrap) ✅; the card-ISR scan ⬜ (M3, no cards on a
bare console); the cassette fork acknowledge-only (hardware-gated). Also live:
**IO functions 0/1** (arm a GROM/VDP sound list, authentic `>05D6`).

**XML (M3/M5)** — the `>0608` dispatch + `>0CFA` master table; **every** table-0
entry (§9) and **every** table-1 entry incl. the `>1C–1F` vestigial four; the
table-F `>8300` vector. *Status:* ✅ complete (M3+M5; §21, §27) — dispatch + the
master table + FLTAB/XTAB homes ✅ (§21); SROM found+call ✅, SGROM full walk ✅
(§24); the FP/conversion entries ✅ (M5, §27 — incl. the `XML >00` = `>0000`
authentic reset accident in FLTAB); the M6 symbol entries stay loud stubs
(deferred by policy).

**Floating point (M5)** — FADD FSUB FMUL FDIV FCOMP + S-variants, ROUND ROUND1
STST OVEXP OV, CSN CSNGR CFI; every `>8354` error path; the radix-100 format.
*Status:* ✅ complete (M5; §27).

**Device linkage (M3)** — SROM (`XML >19`), SGROM (`XML >1A`), the DSR call
invariants, the `>AA` header walk, the power-up scan. *Status 2026-07-05:* ✅ —
SROM found+call + `>83D0` resume (the disk power-up lowers `>8370` identically),
the full SGROM 16-base GROM walk (PUSCAN parity), the DSR invariants, all pinned
in **§24**. The ISR **card-chain** scan is hardware-gated (no card interrupts in
the emulator — §24, an M2-scope duty deferred like the cassette transport).

**Service entries (M1/M3)** — reset/vectors, `>000E`/`>0014`/`>0016`/`>001A`/
`>001C`/`>0020` stubs, `XML >F0`, XMLLNK/`>006A`. *Status 2026-07-05:* ✅ —
reset/vectors/stubs/`>006A`/`>000E`/`>0020` ✅; `XML >F0` ML-cart launch verified
under our ROM (the sweep ML samples, §24); XMLLNK conventions are the GPUSH/GPOP
sub-stack discipline (exercised by `device_io`; the ML-entry form vestigial per
the R-3 census).

**Cassette (M4)** — the `I/O 4/5/6` bit engines + the `>1404` timer ISR:
present + behavior-correct, transport hardware-gated (§11). *Status:* ✅
interface-correct error behavior shipped (M4; §26); the tape transport itself
stays deferred with the cassette hardware (plan §10.2).

**BASIC ROM half (M6)** — PARSE CONT EXEC RTNB, the statement entries, the
`>1C9C` jump tables, the symbol/value-stack package (SMB SYM ASSGNV VPUSH VPOP
PGMCH). *Status:* ⬜.

**Vestigial-but-present (faithful reproduction)** — the extended-GPL trampolines
(`>0C0C/0C14/0C1C`, XOP-0 → CRU `>1B00`), the XTAB `>1C–1F` entries, the level-2
vector, the harvested NASTY constants (§13). *Status 2026-07-05:* ✅ — the
`>0036–004D` stub/vector/data block, the level-2 vector, the NASTY constants
placed to date (`>000C/>0012/>0032/>0036/>004C/>011B/>0025`), and (M4 slice 3,
§25) the ext-GPL trampolines at their pinned homes (byte-identical; the XOP-0
vector live) + the XTAB `>1C–1F` harvested dispatch constants — all gated
(`vestigial_surfaces_match_authentic_bytes` + the bounded departure pair).

*Open ❓ (pinned by execution in the owning milestone):* FP bit-exact edge
cases; IO CRU-output with count > 1. (**Pinned at M4 2026-07-05:** the FMT
sub-op grammar — §7; the indexed-GAS + MOVE-C=1 + GRAM-dest semantics — §15
GPL-interpreter line, the >8300-indexed-word add of authentic `>0758`/`>07D2`.) **M4 per-opcode-form details surfaced by the M2
fuzz** (`gpl_fuzz.rs`): MUL/DIV byte-form result semantics (MUL byte `>09*>96` →
authentic `>FC` vs ours `>05`); EX's undefined immediate forms; and the
SRA/SLL/SRL/SRC handlers mirroring their result flags into the `>837C` status
byte. (RESOLVED at M2 close-out: the `@>02F1(R5)` split-scan centered defaults —
the JOYDEF table, pinned by the joystick gates; and the SHCNT shift-count mask,
`>001F`→`>000F`, caught by the fuzz.)
(Pinned since R-2: the condition-bit rules + RAND §16; MOVE §17; ALL §18; IO
count=1 §19; the SPEC/ISR mechanics §20; XML dispatch + the no-card scan §21;
KSCAN §22; result normalization + alpha-lock §23.) Every one is an *element to
implement*, never to skip.

---

## 16. M1 execution-pinned semantics (the oracle round, 2026-07-04) ✅

Measured on the authentic ROM with `crates/libre99-asm/examples/gpl_oracle.rs`
(GPL programs at `>0020`, zero RAM, no ISR — deterministic), enforced by the
79-case differential microsuite `crates/libre99-asm/tests/gpl_core.rs`.

**The status byte `>837C` is the 9900 status high byte**: H=`>80`(L>),
GT=`>40`(A>), **cond=`>20`(EQ)**, CARRY=`>10`, OV=`>08`; bits `>07` are
interpreter-internal. Per-class rules (all oracle-pinned):

| Class | `>837C` effect |
|---|---|
| ADD/SUB/INC/DEC/INCT/DECT (byte/word) | full replace: L>/A>/EQ→cond/C/OV from the real CPU op; word forms leave `>04` **set**, byte forms leave the CPU's natural odd-parity `>04` |
| AND/OR/XOR | replace L>/A>/EQ→cond from the result; **word: C reads set, `>04` clear; byte: C clear, natural parity**; OV clears |
| Compares CEQ/CH/CHE/CGT/CGE/CLOG | full replace with **only** the cond bit (a pending C/OV is wiped); predicate = dest OP src (CH/CHE logical, CGT/CGE arithmetic, CLOG: (dest AND src)==0) |
| CZ/DCZ | compare-to-zero copy: H/GT from the value, cond=(value==0); word sets `>04`, byte natural parity |
| ST/EX/MUL/shifts/ABS/NEG/INV/CLR/FETCH/PUSH/BACK/RAND | untouched |
| DIV | untouched on success; **divide-by-zero: operands unchanged, status ∣= `>09`; word quotient-overflow: status ∣= `>01`** (M4 refines the hi≥divisor operand residue) |
| BR/BS (taken or not), B, CALL, CASE, RTN | cond **reset** (BR/BS consume the bit — ⚠ `BR $` is *not* a self-loop while cond is set; test terminators need the double-BR idiom) |
| RTNC | cond preserved |
| H/GT/CARRY/OVF | cond := the named bit |

**Other pinned mechanics:**
- **RAND** (`>02`): seed word at `>83C0`: `seed' = seed×>6FE5 + >7AB9`;
  `>8378 := byteswap(seed') mod (limit+1)`. (Constants read straight out of
  the authentic handler — functional facts.)
- **CALL**: reads the 16-bit target, then pushes the **resume address**
  (`counter−1` after the operands) via the `>0864` helper: `INCT` on the
  `>8373` byte pointer is a **word op on the (`>8372`,`>8373`) pair** (carry
  into `>8372` on wrap — the authentic quirk, reproduced); the stored word is
  then decremented by 1 (`counter` reads one ahead). RTN/RTNC pop then
  post-decrement the same way. Verified: return address `>0026` stored at
  `>8380/81` for a CALL at `>0023`.
- **FETCH**: reads the byte AT the caller's stored return address (inline
  data after CALL) into the destination and increments the stored word — the
  caller resumes past the data. Runs **through the sub-stack** (pushes its own
  resume frame; the frame bytes remain above the pointer as residue).
- **PUSH/DPUSH**: pre-increment the `>8372` byte pointer by **1**, store one
  byte at `>8300+ptr` (DPUSH stores the word's low byte).
- **Shift counts**: count 0 means 16; byte SRC rotates within 8 bits.
- **The `>011B` mask byte is `>20`** — RTN and the `>006A` soft entry clear
  exactly the condition bit.
- **MUL**: byte → 16-bit product at D,D+1; word → 32-bit at D(hi),D+2(lo).
  **DIV**: byte → (16-bit at D,D+1)/src, q→D r→D+1 low bytes, no overflow
  check; word → per RECON §6.

**Recorded deviations (ours vs authentic, documented + suite-masked):**
- `>837C` bits `>07`: unspecified interpreter-internals (we reproduce the
  oracle-observed `>04` patterns; `>03` may differ).
- `>8300–>8307` and the `>8372/>8373` bytes **between stack uses**: the
  authentic operand engine leaves scratch residue there (padram documents
  them as temporaries); ours stays clean. Stack *behaviour* (pointer values
  around PUSH/CALL/RTN/FETCH, stored cells) is verified via explicit copies
  into compared cells.
- DDIV with high-word ≥ divisor: we leave operands unchanged + status `>01`;
  the authentic also scribbles `>8000` into the destination — M4 pins.

---

## 17. M1 increment-3 MOVE semantics (oracle-pinned, 2026-07-04) ✅

Measured against the authentic ROM by the differential microtests in
`crates/libre99-asm/tests/gpl_core.rs` (`move_*`). MOVE (`>20–3F`) decodes
`001 G R V C N` (bit order high→low: G non-GRAM-dest, R VDP-register-dest,
V RAM-source, C computed-GROM-source, N immediate-count).

**Operand stream order = opcode, count, destination, source** ✅ (also pinned
in `libre99-gpl`'s `m2_probe`/`move_probe`). Each field:

- **count** — `N=1`: a 16-bit **immediate word** from the stream. `N=0`: a GAS
  operand; the value read is a **full word** (not a byte — pinned: a from-memory
  count cell holding `>0003` moves exactly 3 bytes; a byte read of that cell's
  high byte would move 0). The shared `isa.rs` doc's "byte count" describes the
  *values* the menu happens to store (small, high byte 0), not the ROM's read
  width.
- **destination** — `R=1`: one raw byte = the **starting VDP register** (the
  copy writes consecutive registers). `G=0`: a 16-bit GRAM address (GRAM write —
  deferred, §below). Else: a GAS operand (CPU or VDP per its own V bit).
- **source** — `V=1`: a GAS operand (CPU or VDP). Else a GROM source: `C=0` a
  16-bit **immediate GROM address**; `C=1` a GAS operand naming the CPU cell that
  *holds* the GROM address (computed GROM — deferred).

**Copy mechanics** ✅: byte-at-a-time, **ascending**, reading each byte *after*
the previous store — so the CPU→CPU and VDP→VDP overlap-propagation idioms work
(`MOVE >0003,@>8349,@>8348` fills `>8349..>834B` from `>8348`; the title's
`MOVE >03FF,V@>0B00,V@>0B01` fill). GROM sources are **re-addressed per byte**
(P4); the interpreter's GROM position is saved (via the `>9802` readback) before
the copy and restored after, so instruction fetch resumes at the next opcode.
MOVE leaves `>837C` untouched. Live combinations verified: source
GROM-immediate / CPU / VDP × dest CPU / VDP / VDP-register × count immediate /
from-memory (7 `move_*` microtests).

**Deferred to M4 (loud stub, breadcrumb `>837D`)** — GRAM destination (`G=0`)
and computed-GROM source (`C=1`); the interpreter still decodes their leading
bits and stubs faithfully.

## 18. M1 increment-3 ALL semantics (oracle-pinned, 2026-07-04) ✅

`ALL` (`>07`, home `>05A2`) fills the **768-cell name table** with the
immediate operand character. Oracle-pinned (`gpl_oracle.rs`): the fill is
exactly VRAM **`>0000..>02FF`** (768 bytes), and the **base is hardcoded** —
setting VDP register 2 (which would relocate a reg2-based name table) does
*not* move it (the chip's registers are write-only, so there is no reg2 to
read back). Status byte unchanged. Gate: `all_fills_screen` / `all_clears_screen`
in `tests/gpl_core.rs`.

## 19. M1 increment-3 IO semantics (2026-07-04) ✅

`IO` (`>F4–F7`, used as `>F6`) is a CRU/sound/cassette I/O op. It does **not**
fit the format-1 two-operand model: the byte after the destination GAS is a
**function code read by value**, not a source operand (the shared `isa.rs`
models it as GasGas only for the disassembler). Our interpreter routes every
opcode `>=>EC` (COINC/IO/SWGR/ext) past the format-1 source-parse so each handler
reads its own operands; the destination-list pointer is already resolved in R7.
The function code indexes the `>0CEC` sub-table (functions 0–6).

**Function 3 (CRU output)** ✅ — the destination points at a four-field list
(shared `../RECON.md` §11): `{ CRU-address word, count byte, data-address byte }`.
The handler loads `R12 := address << 1` (so `SBO/SBZ 0` hits CRU bit `address`)
and drives `count` consecutive bits from the data byte at `>8300 + data-address`,
**LSB first**: a 1 bit does `SBO`, a 0 does `SBZ`. The console boot's
`IO @>8302,#3` (address 2, count 1, data `>FF`) thereby `SBO`s CRU bit 2 — the
9901 VDP-interrupt mask. Gate: `io_cru_output_arms_vdp_interrupt` (verified via
`tms9901.int_mask()`, not scratchpad, so the still-stubbed ISR is not in play).

**Deferred:** functions 0/1 (sound-list arming) ship with the M2 ISR; function 2
(CRU input) is a loud stub pending its own pin; functions 4/5/6 (cassette) are
hardware-gated (plan §10.2). All are `STUB` entries in the `>0CEC` table.

## 20. M1 increment-3d — the specials-dispatch mechanic + the minimal ISR (2026-07-04) ✅

**`SPEC` index isolation (a correctness invariant).** The specials sub-dispatch
(`>0270`) must compute `(opcode & >1F) * 2` from **only R9's high byte** — R9's
low byte is interpreter scratch that opcode handlers freely overwrite (the MOVE
handler parks a dest-storer address there). The robust form is `ANDI R4,>1F00 /
SRL R4,7` (via a scratch copy, preserving R9 for the stub breadcrumb). A shift of
the whole R9 (`SLA R9,3 / SRL R9,10`) is **wrong**: a set low-byte bit 7 makes the
index odd (a misaligned table read → garbage dispatch). This bit the title path —
the boot does MOVEs then `XML >19`, and the MOVE's leftover low byte derailed the
`XML` dispatch — but no format-1/format-5 opcode is affected (they read R9 only
through `SRL …,8` or high-bit `COC` masks).

## 21. M1 increment-3e — XML dispatch + the no-card device scan (2026-07-04) ✅

**XML dispatch** (`>0F`, handler at `>1200`). Operand `>XY`: `X` selects a table
pointer from the master table (`>0CFA`, placed byte-identical to authentic), `Y`
the entry word; XML calls the routine at `*(master[X] + Y*2)` via `BL` (XMLLNK).
Table F = `>8300` is the `XML >F0` ML-launch vector. The operand is read into a
**cleaned** register (`CLR R4 / MOVB / SRL 8`) before the index math — R9's low
byte is opcode scratch (the same hazard as the SPEC fix, §20). Tables live at
their authentic homes: **FLTAB `>0D1A`** (FP — M5 stubs), **XTAB `>12A0`** (device/
BASIC — `>19` SROM / `>1A` SGROM are ours, the rest M5/M6 stubs).

**The no-card PUSCAN contract** (traced on the authentic ROM,
`examples/puscan_trace.rs`): the boot's power-up scan calls **SROM once** (finds
no peripheral card, returns not-found) then **SGROM repeatedly** (16× — it walks
every GROM base) until `>83D0` returns to 0, then falls through to the key-wait
with cond = 0. M1 reproduces the **net** result:
- **SROM** (`>19`, pinned home `>0AC0`): the real scan — CRU bases `>1000..>1F00`,
  enable each card ROM (`SBO 0`), test `*>4000` against the `>AA` marker
  (`@>000D`); none match, so `>83D0 = 0` and `B @>006A` (soft entry: clears cond,
  re-enters the loop). Differentially identical to authentic on a bare console
  (`xml_srom_no_card` microtest). The found+call and `>83D0`-resume paths are M3.
- **SGROM** (`>1A`, pinned home `>0B24`): M1 minimal — `>83D0 = 0`, `B @>006A`.
  This makes PUSCAN's `PUDONE` (`DCZ >83D0`) see "done" and fall through after one
  iteration instead of the authentic 16; the final visible + `>83D0`/cond state
  matches, but the intermediate PUSCAN fetch stream differs (the authentic
  multi-base GROM walk is M3). With both, our ROM boots past PUSCAN to the
  title's key-wait `SCAN` (breadcrumb `>03` — KSCAN is next).

**VBLANK ISR (M1 minimal → M2 core).** The `>0900` ISR started M1 as
acknowledge-only (status read + `CLR R8` + `RTWP`). **M2 upgrades it to the full
control structure** (RECON §6): the cassette-timer fork (hardware-gated,
acknowledge-only), the VDP-vs-card source test, the four `>83C2`-gated duties in
order, the VDP status read, the **screen-timeout blank** (`>83D6` `INCT` +2, at
wrap rebuild VDP R1 from the `>83D4` copy with the display bit cleared), the
**SPEED timer** (`>8379 +=` SPEED), and the **`>83C4` user hook**. **QUIT** (the
`>004C` `CZC` on column 0 → `BLWP @>0000`) is live. **Sprite auto-motion** (the
`>0780` velocity math) and **sound-list processing** (the boot beep) are gated-off
scoped follow-ups — they touch only the SAT / PSG / sound cells, never the
interrupt-ack / timer / timeout the boot and idle depend on. Verified: with the
interrupt armed and nothing else, our ISR advances `>8379` / `>83D6` / `>837B`
byte-identically to authentic's idle ISR (`isr_advances_timer_and_timeout`).
Because sprite/sound aren't yet processed, the M1 title gate still compares
**VRAM + VDP registers only**, not scratchpad.

## 22. M2 — KSCAN + the CLEAR test (2026-07-04) ✅

Full spec in [`KSCAN-SPEC.md`](./KSCAN-SPEC.md) (clean-room, from the authentic
disassembly + the emulator's `keyboard.rs`/`cru.rs`). Implemented (M2-1): the
mode-0 full-keyboard scan.

**KSCAN** (`>02B2`, pinned; reached via `>000E` and, for the `SCAN` opcode, the
`>02AE` shim `LI R11,>0070`). Because Zone C occupies the authentic `>0300+`
interior, the pinned entry trampolines to the body in free space (P8 escape
hatch). Entry saves R11→`>83D8`, pushes the interpreter's GROM position
(`GPUSH`/`GPOP` — the new `>0842` pop), `SBO 21` (alpha-lock select). The scan:
columns **5→0**, select via `LDCR` at CRU base `>0024` (bits 18-20), read 8 rows
via `STCR` at base `>0006` (bits 3-10, active-low → `INV`); **first-key-wins**;
column 0 captures the modifier rows (CTRL `>40` / SHIFT `>20` / FCTN `>10`) then
masks to rows 0-3. **Raw = `column*8 + (7-row)`** (the row byte comes back with
row 7 in the MSB). The raw lives in **R3** (whose low byte *is* `>83E7` — a
GPLWS register alias, **not** a separate cell: `CLR R3` wipes it, the bug found
in test). Modifier priority CTRL>FCTN>SHIFT>none picks a GROM table base
(`>1790`/`>1760`/`>1730`/`>1700`); `base+raw` indexes GROM → the character in
`>8375`. Debounce vs `>83C8`: the condition bit `>837C&>20` is set **only on a
code change**; a new key also un-blanks (reload VDP R1 from `>83D4`, `CLR >83D6`).
No-key → `>8375=>FF`, cond clear. Gate: `tests/rom_kscan.rs` (digit / letter /
Enter / Space / no-key, differential vs authentic).

**Deferred (M2-2):** split modes 1/2 + joystick, the alpha-lock case fold
(⚠ `cru.rs` gap — alpha-lock is write-only), the control-code fixups, and
legacy translation states (KSCAN-SPEC §10/§11).

**CLEAR/BREAK test** (`>04B2`, pinned; behind `>0020`): a two-column CRU probe
(column 0 then column 3, via the NASTY selectors `@>0012=>00` / `@>0074=>03` and
the row mask `@>0036=>1000`), returning EQ iff FCTN-4 is held. Same CRU-probe
form as the ISR QUIT test.

## 23. M2 — KSCAN result normalization + the alpha-lock switch (2026-07-04) ✅

Pinned by disassembly (`>0422–0476`, range helper `>04A2`) and differential
tests. After the table lookup, full-keyboard results are normalized per the
**`>83C6` translation state** (0 = 99/4, 1 = Pascal, 2 = 99/4A native; split
units skip the whole block):

- **a–z (`>61..>7A`)**: state 0 folds to uppercase **unconditionally** (the
  99/4 had no lowercase and no alpha-lock key) — the switch is *not read* in
  state 0. States 1/2 read the alpha-lock switch: `CLR R12 / SBZ 21 / (settle)
  / TB 7 / SBO 21`; **line low = locked → fold**, line high = keep lowercase.
  Fold delta `>20` (authentic harvests it from `@>03B4`).
- **not a–z**: state 0 rejects the 4A-only codes — `>10..>1F` and anything
  above `>5F` (`@>0587`) become "no key" (the `>0382` finalize). State 1
  (Pascal): Enter (`>0D` — the NASTY `@>0025`, our own `LI R13` low byte) is
  exempt; codes `<= >0F` (`@>02CA`) get the `>80` bit set; `>80..>9F` get it
  cleared. State 2 (4A native): nothing.
- The authentic loads the state through R3's low byte (**`>83E7` — a GPLWS
  register alias**), the same alias mechanics as the raw scan code.

**The switch (the former "blocker", dissolved).** Our 9901 models alpha-lock
only as the P5 write latch (`cru.rs`), so `TB 7` reads keyboard row 4 of the
selected column — idling **high** = "not locked". Both ROMs read the same
emulated line, so they remain differentially identical by construction, and
both normalization branches are exercised and gated by `tests/rom_kscan.rs`
(`kscan_state0_folds_authentic_lowercase_table_to_uppercase`,
`kscan_native_state_reads_the_switch_and_keeps_lowercase`) using the authentic
GROM's keytabs. Making the switch *functional* (host Caps Lock → a readable
line while P5 is low, plus the real joystick-up interference quirk) is an
emulator feature — **`docs/ROADMAP.md` §6**.

**Keytab finding (for the GROM track's ledger).** The authentic unshifted
keytab holds **lowercase** letters (`>1708` = `x w s …`); **our GROM's holds
uppercase**. State 0's fold masks the difference (lowercase+fold ≡ ours
unfolded), so every current gate passes; in the 4A-native state (TI BASIC's
mode 5) the authentic GROM yields lowercase where ours cannot. A GROM-track
fidelity note, not a ROM-rewrite defect. (Resolved 2026-07-06: our keytab now
stores authentic lowercase; gate
`rom_kscan.rs::our_keytab_types_lowercase_in_native_state_and_folds_in_state0`.)

## 24. M3 — device linkage: SROM / SGROM / XML >F0 (2026-07-05) ✅

Pinned by disassembly (`>0AC0–0C0A`, the SROM/SGROM/SNAME block) and differential
tests. The console-internal DSRLNK: **SROM** (`XML >19`, pinned `>0AC0`) searches
the peripheral cards, **SGROM** (`XML >1A`, pinned `>0B24`, trampolines to a
free-space body) searches the GROM headers; both call every match. Inputs from
the GPL caller: the chain offset **`>836D`** (`>04` power-up / `>06` program /
`>08` DSR / `>0A` subprogram), the search-name length **`>8355`** (0 = match
every node) + text **`>834A`**, and the resume cursor **`>83D0`** (0 = fresh,
non-0 = resume the card/base saved in **`>83D2`**).

**SROM (`>0AC0`) — found+call+resume.** `>83D0=0` → scan CRU bases
`>1000..>1F00`, enable each card ROM (`SBO 0`), test `*>4000 == >AA` (`@>000D`).
On a card, walk the chain at `>4000 + >836D` node by node — each node is
`[link][routine][namelen][name…]` — SNAME-matching, and on a match `BL *R9` with
**R12 = the card CRU base, the card ROM enabled, LIMI 0** (already in force from
the interpreter's per-instruction window) — the DSR-call invariants (§10,
Classic99 `disk.cpp`). Continues the chain (calls every match); not-found →
`B @>006A` (`>83D0=0`, cond clear). The M3 body fits the authentic 100-byte home
directly. **Verified under our ROM end-to-end, FMT-free:** with a disk controller
the boot's peripheral power-up finds the disk card (routine `>4070`, CRU `>1100`)
and calls its power-up routine, lowering **`>8370`** (top of free VRAM) from
`>3FFF` to `>37D7` byte-identically to authentic (`device_io::disk_power_up_reserves_vram`).

**SGROM (`>0B24`) — the multi-base GROM walk.** Scans the eight GROM bases
(R1 = `>E000` down to `>0000`, step `AI R1,>E000` ≡ `->2000`), and at each base
whose `>AA` header is present follows the chain field (at `>836D`) node by node
through the **GROM ports** (`MOVB …,@>0402(R2)` writes the address, `MOVB *R2,…`
reads), SNAME-matching each. A match pushes the found routine onto the GPL **data
stack** (`>8372`/`>8300+`), sets the condition bit, and re-enters the interpreter
(`SOCB @…,@>837C / B @>0070`, the authentic `>00CE`), so **PUSCAN's GPL loop runs
the routine**; a full base-pass with no match steps the port cursor `>83D0` by 4
and, for the un-named power-up scan, returns with `>83D0` still set so PUSCAN
re-enters. The authentic **16 iterations** are the `>9800..>9840` cursor sweep.
All GROM addressing rides the **R1/R3/R9 low-byte GPLWS aliases** `>83E3`/`>83E7`/
`>83F3` exactly as authentic (the §14 house rule). **Two harvested constants are
pinned as explicit named DATA (P8)** — the authentic ROM reads its own instruction
words `@>0128 = >1FFF` (the clean-base mask) and `@>0C04` byte `= >06` (the
program key); our packed bodies differ there, so `GR13M`/`PGMKEY` name the values.
Verified: `firmware_matrix::matrix_puscan_walk_matches_authentic` — SROM ×1,
SGROM ×16, the `>83D0` cursor sweeping `>9804..>9840` back to 0, byte-identical to
authentic. The full walk realigns the DSR-entry cell `>83D2` (out of the matrix
whitelist).

**SNAME (free space) — the shared name comparator.** Length `>8355`, text
`>834A`; length 0 matches every node. Shared by SROM (card pointers, `R2 < >9800`,
the `INC R2` always fires) and SGROM (GROM port, `R2 = >9800`, the port
auto-advances so the `CI R2,>9800 / JHE` skips the INC). Match → `INCT R11` so the
caller falls through its keep-walking JMP into the call.

**XML >F0 (the ML-cart launch) + XMLLNK.** XML operand `>F0`: master[`F`] = `>8300`
(§8), entry `0` → the RAM vector at `>8300`; XMLH calls `*(>8300)` — the ML↔GPL
launch trampoline. Verified under our ROM by the ML-cart samples in `sweep.rs`
(centipe, MoonPatrol): selecting the cart runs the console launch GPL, `XML >F0`
jumps to the cart's `>6000` ROM, and the CPU PC enters the ROM window. **XMLLNK
return conventions** (`>83FA`/`>8372` — R13 save + the data-stack pointer) are the
GPUSH/GPOP sub-stack discipline SROM/SGROM use and DSRLNK exercises through
`device_io`; the fixed *ML-entry* XMLLNK form is vestigial in our corpus (the R-3
census: ML enters the ROM only at `>000E`).

**Cross-milestone dependency — FMT (M4).** The *Tunnels of Doom* end-to-end load
and the two GROM+ROM sweep samples Parsec / TI-Invaders paint their startup
screens with the GPL **`FMT`** opcode (`>08`), which is M4 (§7). Under our ROM
that trips the FMT loud-stub (`>837D=>08`) before any device I/O, so those flows
run under the authentic oracle until M4; the device linkage itself is proven
FMT-free (the power-up gate + the ML XML >F0 samples). This is test ordering, not
a device-linkage gap.

**The ISR card-chain — hardware-gated (not modelled).** The authentic level-1 ISR,
on a **non-VDP** interrupt (`TB 2` clear), scans the peripheral cards' `>400C`
ISR-chain pointers (§6 step 3). Our emulator raises **only the VDP interrupt line**
(`crates/libre99-core/src/cru.rs` `pending_interrupt`: "the bundled cards poll rather
than interrupt"), so no non-VDP level-1 interrupt ever occurs and the scan has no
stimulus — the same disposition as the cassette **transport** (§0 exception 1,
hardware the emulator lacks). Our ISR's non-VDP path acknowledges and returns; the
full card-ISR walk is deferred to whenever the emulator grows card-interrupt
modelling, commissioned together with it. (This is an M2-scope duty, not M3.)

**Residual conformance note.** With the full SGROM in place the only remaining
firmware_matrix scratchpad differences are the ISR-driven counters `>8379` (SPEED
timer), `>83D6/D7` (screen timeout), `>83CC-CE` (sound progress): our interpreter
spends more CPU cycles per GPL instruction, so PUSCAN spans ~10 more frames before
the boot arms the VDP interrupt, and every ISR counter then lags by that fixed
offset. This is frame-level-not-cycle-level parity (§2.4 of the plan), a deliberate
non-goal, plus the phantom power-up routines the walk runs off the absent GROM at
`>E000` (undefined GPL). Title and menu stay pixel-identical (VRAM + all VDP regs).

---

## 25. M4 — the interpreter long tail: driver source discipline, COINC, SWGR/RTGR, IO CRU-in/out, ext-GPL vestige (2026-07-05) ✅

Disassembly-as-spec of everything M4 still owed (P5: interface facts only).
Reproduce any ✅ with `libre99asm dis roms/994aROM.Bin <addr>`.

**The two-op driver (`>0086–00CA`) — the authoritative operand discipline.**
- Dest first: `BL @>077A` (OPGET) parses **and loads** the destination →
  value in R2, address in R3.
- Opcodes `>80–9F` (format-5): dispatch `@>0BFE(opcode)` — **two-opcode
  granularity** (word access eats bit 0); R0 := `SETO` (no source).
- Opcodes `≥>A0`: the source is **uniform**: `COC @>0030,R9` tests **opcode bit
  `>02`** (`@>0030` = `>0200`, the `LI R0` opcode word of reset — a NASTY
  harvest). Set → an **inline immediate** (one byte, or two if the word bit
  `>01` is set); clear → a second OPGET (a memory GAS). **There is no special
  routing for `>EC+`** — COINC/IO/SWGR take the same path (so their
  memory-source forms are live). Dispatch `@>0C4E(opcode/2)` — **four-opcode
  granularity** (byte-offset indexing + word access).
- **Byte values are right-justified with sign extension** (`>07AA`:
  `SRA R0,8`); word immediates fill R0 via the `>83E1` low-byte alias. The
  compare family (`C R2,R0` at `>00C8` before dispatch) relies on it. This is
  why `IO @>8302,#3` is the *byte*-imm form `F6 02 03`: the function value
  `>0003` arrives right-justified and indexes `>0CEC` sanely.
- Our ROM keeps its internal high-justified convention (M1, gate-pinned); the
  three value-consuming `≥EC` handlers normalize at entry (`SRA 8` byte forms —
  sign included). Divergence kept: function bytes `≥>80` / functions `≥7` hit
  our loud stub where the authentic garbage-dispatches (`@>0CEC` past-the-end →
  the `>0CFA` master table executed as code). Documented, diagnosable.

**IO functions 2 (CRU-in `>05E8`) / 3 (CRU-out `>05EA`) — a synthesized
`X`.** The authentic builds the exact transfer instruction in a register and
executes it: `LDCR *Rx,count` (out) or `STCR *Rx,count` (in; fn 2 is `INC R9`
falling into fn 3's builder). List = `{CRU-address word, count byte,
data-address byte}` at the **dest GAS address**; R12 := CRU-address ×2;
data cell = `>8300+data-address`. Consequences (all the 9900's own semantics —
this settles the §15 "count > 1" ❓): count is the **4-bit field** (`&15`,
`0 → 16`); count ≤ 8 accesses the **byte** at the data cell, count > 8 the
**word** (odd address → the even pair, the 9900 word rule); bits transfer
LSB-first to/from consecutive CRU addresses; **STCR zero-fills** the rest of
the byte/word. Our rewrite synthesizes the same instruction from `LI`
skeletons (`>3012`/`>3412`) — interface-identical by construction, expression
our own.

**COINC (`>EC–EF` → `>06D2`) — a bitmap coincidence test.** Stream:
`[opcode][dest GAS][source imm/mem][scale byte][table16]`. Dest and source are
Y,X **byte-pair words** (point coordinates). Semantics: ΔY := srcY−destY,
ΔX := srcX−destX (byte subtracts); if scale ≠ 0, both deltas `SRA` by scale
(low 4 bits, 0 → 16 — the 9900 count rule); the 4-byte header at `table16`
(GROM) is `[Ylimit, Xlimit, Yoffset, Xoffset]`: Δ += offset (byte add,
**negative → no**, X first then Y), Δ > limit → no (Y then X). Inside: bit
index = (ΔY_byte × (Xlimit+1)) + ΔX_byte into the **bitmap following the
header** (base+4), MSB-first within each byte; the addressed bit decides.
**`>837C` is overwritten wholesale** — `>20` (hit) or `>00` (miss) — clobbering
H/GT/CARRY/OV. The interpreter position is GPUSHed before re-addressing to the
table and restored by exiting through **RTNC** (position pop; the fresh status
survives because RTNC does not clear).

**SWGR (`>F8–FB` → `>004E`) — switch GROM base.** Dest value → the new **R13**
(GROM port base, `>9800+4n`); source value → the new GROM address. Sequence:
GPUSH ×2 (both capture the return position), the **top slot overwritten with
the old R13** (stack: `[retPC, oldR13]`); R13 := new base; one **settle read**
of the new base's data port (`MOVB *R13` — advances that GROM, authentic
strobe); write the new address (MSB, LSB); fall into the `>006A` soft entry
(condition cleared) → the loop. Interpretation continues at the new base.

**RTGR (`>13` → `>082C`) — return across a base switch.** GPOP (writes the
popped word — the saved R13 — to the *current* base's address port, a spurious
but faithful side effect); `R13 := the popped word` (restore); one byte
written to the restored base's **GRAM data port** (`>0400(R13)`, value = the
pop index high byte — inert on mask ROMs, present-and-faithful); falls into
**RTN**: condition cleared, the return position popped, loop.

**Ext-GPL trampolines (`>0C0C/>0C14/>0C1C`, pinned; helper `>0C28`).**
`>0C0C`: `BL @>0C28 / B @>4020` (special ops `>14–1E` + the two-op `>98–9F`,
`>F0–F3`, `>FC–FF` blocks); `>0C14`: `BL @>0C28 / B @>401C` (special `>1F`);
`>0C1C`: `LWPI >2800 / BL @>0C28 / B @>4028` (the XOP-0 vector target, `>0040`);
`>0C28`: `LI R12,>1B00 / SBO 0 / B *R11` (card ROM on). With no card the
branches land on empty bus — the authentic accident, reproduced exactly (the
encodings are pinned-address-forced; conformance-checked byte-identical like
the vectors).

**MOVE's position restore is a sub-stack pop.** The authentic GRAM storer path
(`>0686`) ends `B @>083E` — MOVE saves the interpreter position by **GPUSH and
exits through RTNC**, leaving the pushed bytes above the stack pointer (ghost
bytes our explicit save/restore does not write). Slice-6 lead for the ToD
GRAM residual; the SWGR/RTGR differential gates pin ghost parity for the new
ops. (GROM pacing `JMP $+2` delays between authentic port reads carry no
port-visible effect under the emulator and are not reproduced.)

### §25 addendum — slice 4: the per-opcode-form semantics the fuzz owed (2026-07-05) ✅

Pinned by disassembly (`>0186–0270`, `>077A–0834`, `>0880–08FE`) plus targeted
oracle probes; verified by the widened differential fuzz (MUL/DIV/EX in the
pool, `>837C & F8` compared, 20 000-program soak green) and 27 new microtests.

- **SUB is NEG-then-ADD** (`>0186` = `NEG R0` falling into ADD `>0188`): the
  status is an *addition's* — C differs from a subtract's no-borrow exactly at
  source 0, OV at source `>8000`.
- **The arithmetic tail mirrors the raw 9900 STST byte** into `>837C` — the
  GPL status bits ARE the 9900 layout (L>`>80` A>`>40` EQ`>20` C`>10` OV`>08`
  OP`>04`).
- **The jump-family compares (CH/CHE/CGT/CGE/CLOG) preserve** everything but
  the condition bit (`SOCB`/`SZCB >20`) — the old wholesale-replace reading
  was wrong. **CEQ and CZ are raw-STST wholesale** instead: H/GT/EQ from the
  compare; CEQ's visible C = the opcode's word bit (the `SRL R8,9` dispatch
  carry = opcode bit 0); OV deterministically 0 (the last OV-setter on the
  path is the operand engine's `AI >8300`/`DEC`, never overflowing); OP =
  stale byte-parity (masked, interpreter-internal). CZ's C is register
  residue (masked).
- **MUL byte** keeps the source's right-justified **sign extension**
  (`>07AA`'s `SRA 8`) while the dest byte is cleaned unsigned — the 16×16
  product's LOW word stores at D, D+1 (`>09*>96` → `FC 46`). Word: high at D,
  low at D+2.
- **DIV presets `>837C` wholesale** (`MOVB R5` → `>01` word / `>00` byte),
  then dividend `sext(D-byte)::(D:(D+1))` (byte) or `D::(D+2)` (word) ÷ the
  sign-extended source; overflow (the 9900's own `JNO`, ÷0 included) ORs
  `>08` — the harvested `@>0013` byte (NASTY) — and the *unchanged* halves
  still store back. q → D, r → D+1/D+2.
- **EX's immediate forms**: the immediate stores to the dest normally; the
  old dest value goes to the imm path's leftover pointer — the speech-write
  region `>97FF`(/`>9800`) — inert on this bus (machine.rs no-ops both), so
  the observable is `dest := imm`.
- **`>837D` is the character buffer**: OPGET fetches the screen byte at the
  cursor (`>837E` row / `>837F` col) into the cell before ANY CPU-space load
  that resolves to `>837D` (short, long, and indirect forms — the long/
  indirect paths rejoin the short path's `>0780` check); the `>0232` store
  tail paints the last-stored byte back at the cursor when a CPU store ends
  at `>837D` (word stores to `>837C/D` included; MOVE's storers exempt).
  Standard cell = `>4000|row*32+col`; **multicolour** (FLAGS `>02`, the
  `@>0072` NASTY word) uses the pattern nibble at
  `>0800 + (row/8)*256 + row%8 + (col/2)*8`, the column's low bit picking
  the half — reads extract the nibble, writes read-modify-write it (the
  first address setup carries no mode bit, authentic `>08AA+`).
- **`*@>837C` is the data-stack pop quirk**: a CPU-indirect operand naming
  `>837C` reads the stack byte `>8372` as the offset and post-decrements it
  (`SB R14`) — the operand is the popped stack cell.
- Kept divergences (documented): the multicolour echo's register clobbers
  differ (the authentic tramples R8; ours R0) — visible only to a MUL/DIV
  whose two-cell store straddles `>837D` in multicolour mode, garbage-range;
  CZ's C bit is residue on both sides.

---

## 26. M4 — the cassette modem layer + the MOVE VDP-register mirror (2026-07-05) ✅

Disassembly-as-spec of `>1346–15D3` (the FSK engines), `>1404–1422` (the
cassette timer ISR), and — surfaced by the gates — the `>0698` VDP-register
storer's `>83D4` mirror.

**The JMP-$ timer idiom.** The engines wait out each FSK half-cell by spinning
on `JMP $` (`>10FF`); every 9901 interval-timer interrupt enters the cassette
timer ISR, which — still on GPLWS — tests **GPL R1's sign** (negative = the
edge-hunt phase), then on INTWS compares the interrupted instruction with the
`JMP $` encoding (the authentic harvests its own `@>13F0`): a match `INCT`s the
saved PC past the wait (a hardware-timed single step); anything else **warps
the PC to the resume address parked in GPL R6's cell (`>83EC`)** — the
timeout/abort escape the engines re-park phase by phase (`LI R6,...`).

**The shared setup (`>13BA`).** The IO operand list is `{byte-count word,
VDP-address word}`; records = ceil(count/64). Sets the VDP address (read mode
for write/verify, `+>4000` for read), **FLAGS |= `>20`** (the ISR fork on),
`SBZ 2` (VDP interrupt OFF), `SBZ 12`, `LDCR R3,15` (timer mode + the
half-cell interval — `>0023` write / `>002B` read), `SBZ 0`, `SBZ 1`, `SBO 3`
(the timer interrupt armed), R12 = 0. No motor bits — the CS1/CS2 DSRs (GROM
GPL) drive those via `IO #3`.

**Write (`>1346`, IO 4):** a 768-zero-byte leader, sync `>FF`, the record
count twice, then per 64-byte record ×2 (R9 toggles): an 8-zero leader, sync,
the bytes read from VDP with an additive checksum, the checksum byte. Bytes go
out INVERTED, MSB first; the mag-out line toggles via an X-executed
`SBZ 25`/`SBO 25` pair (the `>0300` XOR mask — the authentic harvests its
`LIMI 1` word). Ends on a stepped `JMP $` into the teardown.

**Read/verify (`>142E`/`>1426`, IO 5/6):** verify ORs FLAGS `>10` (the
`@>1344` harvest), read clears it and arms the VDP for writing. `>837C` is
preset `>20` then `>21` (the `@>1443`/`@>1424` byte harvests) while a record
is in flight, `>00` on success. The leader hunt requires 48 (read) / 96 good
cells, calibrates the half-cell interval by riding 8 edges against a
`>7FFF`-loaded timer (`STCR`-read, ×5/64, minimum `>001F`), rides out the
sync, then: read mode streams 64-byte records into VDP; verify compares each
byte against VDP (`SB @>FBFE(R15)`) — a first-copy mismatch retries on the
record's second pass (R5 negated as the marker); a good first copy times out
the second (73 byte-times, re-parking the warp per byte so a timeout skips
one byte). The checksum must sum to zero. Teardown (`>155E`): FLAGS `>10`+`>20`
off, `SBZ 3`, `SBO 12`, `SBO 1`, **`SBO 2` (the VDP interrupt back on)**, into
the loop.

**Placement + the gates.** No external caller enters the authentic homes (the
`>0CEC` table + the ISR fork are ours), so the whole layer is free-placed
(Zone K, `>1820+`) — FMT keeps its Zone-I squat. The emulator models no
interval timer and no tape line: the engines park on their first half-cell
wait identically under both ROMs. Gated: the setup surface (`cassette_write_
arms_and_parks`, `cassette_read_and_verify_flags`) and the fork + warp
(`cassette_timer_isr_fork_warps`: the fork's race-free `SBO 3` signature under
both ROMs; the JMP-$ ladder ride asserted on the authentic — the poked warp
target races with GPL-R6 rewrites, which are interpreter-internal by design).

**The `>0698` VDP-register mirror (a gate discovery).** A `MOVE` to VDP
registers **starting at register 1** mirrors its FIRST value byte into
`>83D4` (the ISR's R1 copy, read by the screen-timeout blank/unblank); with
FLAGS `>08` (16K) the `>80` mode bit is forced into the value first — the
copy AND the register get it. Only the pre-ORI selector can match, so a
multi-register MOVE passing through R1 mid-copy does NOT mirror. Ours
reproduces all three facets (`MDREG`; the `move_grom_to_vdp_registers` +
fork gates pin both directions).

### §26 addendum — the SROM DSR skip-return exit (the ToD residual, 2026-07-05) ✅

The "ToD diverges at a GRAM MOVE" hypothesis was **wrong** — the `>837D=>20`
"breadcrumb" was ToD legitimately writing a SPACE to the character buffer
(§25 made that cell live; the breadcrumb cell is dual-use). The real residual,
found by PC-histogram + stale-cell probing: **the DSR call's skip-return
convention.** A DSR that HANDLED the request returns to `BL *R9`+2. The
authentic SROM's skip-exit (`>0B16`): `SBZ 0` (the card ROM off), `BL @>0842`
(GPOP — **pops the GPL DSRLNK's CALL frame**, so the interpreter resumes at
DSRLNK's *caller*, short-circuiting its not-found/error tail), `B @>006A`
(condition cleared). `>83D0` keeps the found card base — a later
`XML >1A`/`>19` with that stale cursor "resumes" reading **ROM-as-GROM**, a
garbage accident whose termination depends on the ROM's own interior bytes
(the authentic's happen to terminate; unreproducible byte-for-byte and
irrelevant once the skip-exit works, since the GPL flow it feeds is the
error path the skip-exit bypasses). Our SROM's missing skip-exit landed the
skip-return on `SRNONE` — card left ON, the GPL stuck in DSRLNK's error tail,
ToD hung at WORKING. With the exit in place **ToD's full disk load completes
under our ROM** (`device_io.rs`, both flows, both ROMs).

### §26 addendum 2 — the 256-opcode sweep's catches (2026-07-05) ✅

**MOVE's destination decode peels G before R** (`>063E`): G=0 is a GRAM
destination regardless of R — `>28-2F` never touch the VDP registers. (Ours
had tested R first; the sweep's `>29/>2C-2F` cases caught the five affected
forms writing VDP register 7.)

**FETCH's word form (`>89`) stores the inline byte sign-extended as a WORD**
(`>0152` `SRA R2,8` + the shared R5-honoring store): dest gets
`[sext:byte]`; only the byte form (`>88`) stores the bare byte. (Ours had
stored one byte regardless.)

With these, `gpl_opcode_sweep.rs` holds: **all 252 non-M6 opcode bytes
dispatch identically under both ROMs** from canonical well-formed programs;
the M6-deferred four (PARSE/CONT/EXEC/RTNB) breadcrumb loudly — the
deferral tripwire. **M4 is complete**; §15's GPL-interpreter line is fully
✅ outside the deferred M6 surface.

### §26 addendum 3 — M5 slice 1 notes (2026-07-05)

**The great relocation.** The FP/conversion ranges (`>0D3A-1345`) are clear:
Zone H → `>1CF0`, MOVEH → `>1E90`, XMLH → `>1FB8` (all in the deferred-M6
region), SNAME → `>17D4` (the Zone J-K gap), IOH → `>0E94` (the FMUL-interior
gap — an M5 body may displace it again), SGROMB → `>1346` (+6, clearing the
conversion tail `>1340-1345`). Pure relocation; every suite green.

**An unpinned garbage corner (M7).** MOVE's `G=0,R=1` count-from-memory
forms (`>2C`/`>2E`): the authentic's parse leaves an unbalanced sub-stack
frame — its dest decode differs structurally when R rides a GRAM dest — and
the divergence is interpreter-geometry-sensitive (our pre-relocation pass was
masked-zone luck). No real emitter produces the combination. Ours runs them
as coherent GRAM moves, parks cleanly, no breadcrumb
(`the_garbage_corner_parks_cleanly_under_ours`); pinning the authentic's
exact garbage parse is an M7 robustness probe. Documented exclusions:
`gpl_opcode_sweep.rs::M7_GARBAGE_CORNER`.

**The byte budget (measured).** Non-BASIC content now: interpreter+kernel+
tables `>0000-0CF9` + FP-to-come `>0D3A-1345` + squatters. Free after this
slice: `>141C-143F` (36B), `>1609-161F` (22B), `>17FC-181F` (36B),
`>1AC0-1AFF` (64B), `>1FE8-1FFB` (20B) — ~178B of slack plus whatever M5's
bodies leave inside `>0D3A-1345`. Fit pressure is real: M5's FP bodies must
be written lean, and IOH's `>0E94` squat may need one more hop.

---

## 27. M5 — the radix-100 floating-point + conversion package (2026-07-05) ✅

Pinned by three parallel disassembly dossiers (session scratchpad:
`fp-recon-fmul.md`, `fp-recon-round-fdiv.md`, `fp-recon-conv.md` — annotated
instruction-for-instruction with raw-byte cross-checks and hand-traced
vectors) plus the in-session FADD/FCOMP/cluster decode. The headline facts:

- **The geometry is self-packing.** The pinned entries force the authentic
  packing, and our reimplementation lands every internal label at the
  authentic address by natural flow: `FPNR18 >0F18`, `FPNRM >0F1C`,
  `FPZERO >0F2A`, `RND1B >0F56`, `FPBIG >0F86`, `FPTA6/FPTAA >0FA6/>0FAA`,
  `FPESYN >0FBC`, `FDSTOR >1148`, `CNV50/CNVE3 >1158/>1159`,
  `CNVDEC >115A`, `CNV100 >1320`, the CFI zero exit `>1342`. **CFI's first
  four words are byte-identical (`C120 834A 1342 04C0`)**, so XML `>1C-1F`'s
  index-past-the-table accident reproduces exactly with the DATA tail
  retired.
- **Everything funnels through ROUND1's body**: FMUL/FDIV enter the `>0F18`
  error-word clear, FADD the `>0F1C` normalize, CSN the `>0F56` round; the
  tail rounds half-up on the first guard digit (`C @>8352,>3200`),
  range-checks the `>8376` exponent word (unsigned ≥ `>0080` = over/under),
  re-installs the exponent byte via the R3-low alias, re-applies the `>8375`
  sign, and publishes the raw STST byte (`FPTA6` from FAC's first word;
  `FPTAA` with the caller's live flags — FCOMP's exit).
- **Errors**: overflow saturates FAC to ±`7F63 6363 6363 6363` with `>8354`
  := `>01`; **divide-by-zero := `>02`** (the `>0FBC` stub, the dividend's
  sign); CFI integer overflow := `>03` (the `>1159` byte) with no result
  stored and the first word left ABS'd; **underflow and every insignificant
  CSN input are SILENT zeros** — the ROM never reports a syntax error
  (callers compare `>8356`), and `>836C` is never touched (no error warp).
- **The S-forms** pop ARG unconditionally via the `>1FA8`-protocol helper
  (address from the `>836E` word, LSB/MSB to `>8C02`, pointer -= 8, eight
  `>8800` reads; ours free-placed at `FPPOP >1420`).
- **FMUL** = in-place schoolbook: FAC rows consumed-and-zeroed (the slot
  catches the row carry), `digit*digit + acc` split by `DIV 100`, lazy byte
  carries (bounded 198); W = eF + eA - `>3F`. **FDIV** = Knuth D in radix
  100: the divisor relocated to `>8354-835B`, a zero spill digit + 8-byte
  extension, the m = 100/(d1+1) two-pass pre-scale, two-digit qhat estimates
  with the D3 while-correction (qhat = 100 special-cased), byte multiply-
  subtract with +100 borrow, the rare add-back, NINE digits (two become the
  guard); W = (Ea - Ef) + 64. **CSN/CSNGR** = a two-pass parse over VDP (or
  GROM when `>8389` ≠ 0) text at `*>8356`: grammar `[+|-] 0* d* [. f*]
  [E[+|-]d+]` (uppercase E, no blanks), DADJ digit-count alignment, the
  biased exponent = (E10+128)>>1 with the parity picking the radix-100
  pairing, EIGHT output bytes (the 8th = the guard), the exit pointer = the
  terminator (except the documented zero/abort paths); `1E` with no digits
  is a full abort writing nothing but the sign. **CFI** = floor(x+0.5), ties
  toward +∞ (a negative tie scans the residue digits), −32768 accepted.
- **Gates**: `gpl_fp.rs` — 58 tests: 45+ planted-operand cases over every
  XML table-0 entry + the S-forms + the conversions (23 CSN grammar cases,
  17 CFI edges, both CSNGR source rows), a 160-case FP operand fuzz and an
  80-case CSN text fuzz — all bit-exact against the authentic. The
  `XML >00` = `>0000` reset accident ships in FLTAB.
- **Kept divergences**: none new at M5 close. (The gate for CSNGR's GROM row
  plants its halt AS the grammar terminator — the conversion leaves the GPL
  PC in the text, and a `,` there would execute as opcode `>2C`, the §26
  garbage corner.)
- **Kept divergence added 2026-07-05 (deep-fuzz find, post-M8)**: ROUND
  (XML `>02`) with a garbage `>8354` position — positions ≥ `>96` walk the
  round ripple through the LIVE GPLWS, so the outcome depends on the
  interpreter's own transient register file, which a reimplementation
  cannot share. Exactly three position bytes diverge (`>AA`/`>AB`/`>B1` —
  walks starting on the R10/R13 cells); every other position byte 0-255 is
  differentially bit-exact, and no real emitter passes garbage positions.
  Pinned both ways by `gpl_fp.rs round_position_sweep_pins_the_contract`:
  the 253 clean positions must match, the three ledgered ones must still
  diverge. The same deep-fuzz session (250k operand pairs: arithmetic core,
  S-forms, conversions, RAND all bit-exact) also root-caused Tunnels of
  Doom's "blocky hallway floor" to the `>837D` echo/read tails spilling
  interpreter temporaries into `>8302-8306` — the program's own space
  (ToD's corridor renderer keeps live state there). Fixed: the window paths
  now preserve the whole pad like the authentic (§25; `gpl_core.rs`
  `chb_*` gates).
