# KSCAN-SPEC — implementation spec for the keyboard scanner (`>02B2`) and the CLEAR/BREAK test (`>04B2`)

Clean-room behavioural specification (plan P5) for the two console-ROM routines
the Phase-2 rewrite must reproduce:

1. **KSCAN** — the keyboard/joystick scanner at authentic address **`>02B2`**,
   reached publicly via **`>000E`** (`B @>02B2`) and internally via the GPL
   `SCAN` opcode shim at **`>02AE`**.
2. **The CLEAR/BREAK (FCTN-4) test** at **`>04B2`**, reached via **`>0020`**
   (`B @>04B2`).

Everything here is an *interface fact* or a *behavioural rule* — addresses,
scratchpad cells, CRU operations, GROM-table indices, and the conditions under
which each happens. It is **not** a transcription of TI's instruction stream.
Short disassembly fragments appear only as *evidence* for a spec claim; the
implementation is authored from this spec. Reproduce any ✅ with
`cargo run -q -p libre99-asm --bin libre99asm -- dis roms/994aROM.Bin <hexaddr>`.

The oracle is `roms/994aROM.Bin`, sha256
`599da51e9e1968a806871d681f17b5acbb617accf07191891265aee44ebec2b6`.

Cross-references: RECON `rom/RECON.md` §5, `../RECON.md` §9 (GROM keytabs),
the archived `../history/ROM-REWRITE-PLAN.md` §2.1/§2.2; the emulator hardware model
`crates/libre99-core/src/{keyboard.rs,cru.rs,machine.rs}`; the reconstructed GROM
tables in `crates/libre99-gpl/src/keymap.rs`; the pinning probes
`crates/libre99-gpl/examples/{keymap_probe,fctn_probe,scan_check,joy_gromtrace,joystick_scan_check}.rs`.

---

## 0. The single most load-bearing fact — the raw scan-code formula ✅

A pressed key becomes a **raw scan code**:

```
raw = column * 8 + (7 - matrix_row)
```

where `column`/`matrix_row` are the cell coordinates in
`keyboard.rs::TiKey::position()`. This is *not* `column*8 + row` — the row bits
come back from `STCR` with **row 7 in the most-significant position and row 0 in
the least**, so the ROM's bit-scan (which finds the most-significant set row
first) counts `7 - row`.

Verified two independent ways:

- **keymap probe offsets** (`keymap_probe.rs`, cited in `keymap.rs` tests):
  `'2'` (col1,row4) → GROM offset `>170B` = base+11 = `1*8+(7-4)`; `'A'`
  (col5,row5) → `>172A` = base+42 = `5*8+(7-5)`; `'1'` (col5,row4) → `>172B`
  = 43; `Space` (col0,row1) → `>1706` = 6 = `0*8+(7-1)`; `Enter` (col0,row2)
  → `>1705` = 5.
- **the CLEAR/QUIT row masks**: mask `@>0036 = >1000` selects **row 4** and
  `@>004C = >1100` selects **rows 4+0** — only consistent if, in the 16-bit
  `STCR` result, word-bit3 (`>1000`) = row 4 and word-bit7 (`>0100`) = row 0
  (i.e. word-bit0 `>8000` = row 7). FCTN(row4)+`=`(row0) = QUIT ✅.

Get this formula and the row-bit order right first; almost everything else keys
off `raw`.

Evidence (the scan loop's bit-find, `>0366`):
`MOV R1,R3 / SLA R3,3 / DEC R3 / INC R3 / SLA R4,1 / JNC` — `R3 := column*8`,
then increment while shifting `R4` left until a set (pressed, post-`INV`) bit
falls out of the MSB; the MSB is row 7.

---

## 1. Entry / exit contract

### 1.1 Two callers, one body
- **Public ML entry `>000E`** = `B @>02B2`. Documented caller idiom (📖 E/A,
  Classic99 `makecart.cpp`): `MOVB <mode>,@>8374 / LWPI >83E0 / BL @>000E /
  LWPI <own WS>`. The caller **must** run KSCAN on **GPLWS `>83E0`** — KSCAN
  relies on R13/R14/R15 holding the GPL port images (R13=`>9800` GROM ports,
  R15=`>8C02` VDP ports). Never `BLWP` (that would install a different WP).
- **GPL `SCAN` opcode** → shim `>02AE`: `LI R11,>0070` then falls straight into
  `>02B2` ✅. So the `SCAN` opcode's "return" is the interpreter fetch loop at
  `>0070`.

### 1.2 Return-save and return
- `>02B2`: `MOV R11,@>83D8` — save the caller's return into **`>83D8`**.
- Exit (`>0492`): `MOV @>83D8,R11 / B *R11` — restore and `B *R11`. KSCAN
  therefore preserves R11 for its callers only via `>83D8`; the body freely
  reuses R11 (e.g. for the internal `BL` helpers).

### 1.3 GROM-position save/restore (why `>0864`/`>0842`)
KSCAN re-addresses the GROM to read its translation tables, which would corrupt
the GPL interpreter's instruction-fetch position. So:
- `>02B6 BL @>0864` — **push** the current GROM read address onto the GPL
  sub-stack (`>0864` reads the `>9802` address-readback, stores it at
  `>8300+[>8373]`, bumps the `>8373` byte pointer, and applies the `-1`
  read-ahead correction). Evidence: `INCT @>8373 / MOVB @>0002(R13),@>8300(R4)
  / DEC @>8300(R4)`.
- `>047C BL @>0842` — **pop** it back and re-write the GROM address port
  `>9C02` (`MOVB @>8300(R4),@>0402(R13)`), restoring interpreter fetch.

An implementation that keeps its own GROM address separate (our emulator can
save/restore the GROM address latch directly) may model this as "save the GROM
address on entry, restore before returning". The push/pop through `>8300+[>8373]`
is the authentic mechanism and touches the sub-stack scratch region.

### 1.4 Working registers (all in GPLWS `>83E0`)
| Reg | Role inside KSCAN | GPLWS byte aliases used as scratch |
|---|---|---|
| R0 | translated key byte / split-column mask | |
| R1 | column counter (5→0) then GROM table base+`raw` | `>83E3` = R1 low → GROM addr-low |
| R2 | "a key was already found this scan" flag | |
| R3 | raw scan code | `>83E7` = R3 low → new-scan-code cell |
| R4 | row/joystick read scratch | `>83E9` = R4 low → GROM addr-low (joystick) |
| R5 | mode / **unit** (0=full, 1=left/joy1, 2=right/joy2) | |
| R6 | **new-key flag** (`>2000` ⇒ cond bit); else 0 | |
| R7 | modifier byte (CTRL/FCTN/SHIFT) | `>83EF` = R7 low |
| R11 | internal `BL` return | |
| R12 | CRU base | |

`SBO 21` (`>02BC`) selects the alpha-lock output line (9901 P5) on entry; several
paths `SBZ 21`/`SBO 21` around the alpha-lock read (§6.1).

---

## 2. Mode dispatch (`>8374`) — `>02BE`..`>02F0` ✅

Read the mode byte, right-justify (`MOVB @>8374,R5 / SRL R5,8`), copy to R6.

| Mode | Path | Effect |
|---|---|---|
| **0** | `>02EC` | Full-keyboard scan in the current default translation state. `R0=0` (no split mask), `R5=0` (unit 0). |
| **1** | `>02F4` | **Left split + joystick 1.** Split mask `R0=>0FFF`, `R5=1`, joystick column **6**. |
| **2** | `>02F4` | **Right split + joystick 2.** Split mask `R0=>F0FF`, `R5=2`, joystick column **7**. |
| **3,4,5** | `>02E0` then `>02EC` | Full scan selecting **translation state** `mode-3` (0/1/2). The routine **subtracts 3, stores the result back into `>8374` *and* (byte-swapped) into `>83C6`**, sets `R5=0`, then does a normal full scan. |
| **≥6** | `>0382` | Invalid → short-circuit to the **no-key finalize** (`>8375=>FF`, cond clear). Gate: `C R6,@>0072` (`@>0072=>0002`) `/ JH >0382` — i.e. `(mode-3) > 2`. |

**The "subtract 3, store back" rule** (`>02E0`): after subtracting 3, `MOVB
R6,@>8374` and `SWPB R6 / MOVB R6,@>83C6` persist the translation state so a
subsequent **mode-0** scan continues in that state. On the 99/4A the practical
state is 0 (states 1/2 are the legacy 99/4 and UCSD-Pascal keyboards); our GROM
and cartridges only exercise state 0. See §11 open question.

**Split masks** apply to the **row** byte, not columns (§4.4): `>0FFF` keeps
rows 4-7; `>F0FF` keeps rows 0-3.

---

## 3. Joystick scan (modes 1/2 only, `>02F4`..`>032C`) ✅

Runs **before** the split-keyboard scan and falls into it.

1. Column select: `LI R12,>0024` (CRU base, bit 18), `LDCR @>0405(R5),3` — the
   selector byte is `@>0405+R5`: **`>0406=>06`** (mode 1 → column 6 = Joy1),
   **`>0407=>07`** (mode 2 → column 7 = Joy2) ✅. Matches `keyboard.rs`
   (Col6=Joy1, Col7=Joy2) and `cru.rs` column select on bits 18-20.
2. Row read: `LI R12,>0006` (bit 3), `SETO R4 / STCR R4,5` — read the **5**
   joystick lines (fire/left/right/down/up) active-low, then form an index from
   the pressed directions.
3. Deflection lookup: `SLA R4,1 / AI R4,>16E0` → GROM address `>16E0 + 2*index`;
   write it to `>9C02` and read the **`(Y, X)` pair** from GROM.
   - `MOVB *R13,@>8376` → **`>8376` = Y deflection**; `MOVB *R13,@>8377` →
     **`>8377` = X deflection**. Signed deflections **`+4`/`0`/`-4`** encoded
     `>04`/`>00`/`>FC` (see `keymap.rs::JOY_DEFLECT`, table lands at GROM
     `>16EA`; the ROM's base `>16E0` plus the direction index reaches it).
   - `@>02F1(R5)` supplies a "centered/no-direction" default scan code
     (`>0406`→`>29`, `>0407`→`>25`) copied to `>83E7` when no direction bit is
     set — used by the *split-keyboard* half, below.
4. Falls into the keyboard scan at `>032E` (`R3` was cleared, `R5`≠0 so the
   scan runs in split-unit mode).

**Joystick is a separate path from keyboard arrows.** `>8376/>8377` come only
from the deflection table (`>16EA`); the split-keyboard direction keys decode
through the `>17C8` table into `>8375` (§5). Omitting `>16EA` zeroes the
joystick but keeps keyboard play; omitting `>17C8` kills split-keyboard
movement (`joy_gromtrace.rs`, `joystick_scan_check.rs`).

---

## 4. The hardware scan loop (`>032E`..`>037C`) ✅ — CRU operations pinned

Setup: `LI R1,>0005` (start at column 5), `CLR R2` (no key yet), `CLR R7`
(no modifiers).

Per column (columns **5,4,3,2,1,0** in that order):

### 4.1 Column select
`LI R12,>0024` (CRU base ⇒ bit `>0024>>1 = 18`); `SWPB R1 / LDCR R1,3 / SWPB R1`
outputs the 3-bit column number to **CRU bits 18-20** (9901 P2-P4, P2=LSB).
Confirmed against `cru.rs::Tms9901::write_bit` (bits 18-20, `b = 1<<(bit-18)`).
Column value 0-7 maps 1:1 to `keyboard.rs` columns.

### 4.2 Row read
`LI R12,>0006` (CRU base ⇒ bit 3); `SETO R4 / STCR R4,8` reads **8 rows** from
**CRU bits 3-10** into R4's high byte, **active-low** (pressed = 0). `INV R4`
makes pressed = 1. Confirmed against `cru.rs::read_bit` (bits 3-10 ⇒ rows 0-7,
`!keyboard.is_pressed(column, bit-3)`).

**Row bit order (critical, §0):** after `STCR`+`INV`, in the 16-bit R4 the row
byte occupies word-bits 0-7 with **word-bit0 (`>8000`) = row 7 … word-bit7
(`>0100`) = row 0**. Equivalently row 0 (CRU bit 3, read first) lands at the
field LSB.

### 4.3 Column-0 modifier capture
When `column == 0`:
- `MOVB R4,R7` — R7 high byte := the full column-0 row byte. Modifier bits
  (byte-value form): **CTRL = `>40` (row6), SHIFT = `>20` (row5), FCTN = `>10`
  (row4)**.
- `ANDI R4,>0F00` — keep only rows 0-3 for **key** detection (`=`, Space,
  Enter, and unused row3), discarding the modifier rows so they aren't treated
  as keys.

Because column 0 is scanned **last**, modifiers are always captured; but the
`R2` "already found" guard (below) means a real key in columns 1-5 wins over the
column-0 keys.

### 4.4 Split mask
`SZC R0,R4` clears the masked rows (`R0` = `>0FFF` keeps rows 4-7; `>F0FF` keeps
rows 0-3; `0` for full scan). For column 0 this stacks on the `>0F00` mask.

### 4.5 First-key-wins + scan code
`JEQ next-column` if no rows set. Else, if `R2` already set (`MOV R2,R2 / JNE`),
skip (keep the first key found). Otherwise `SETO R2` (mark found) and compute
`raw = column*8 + (7-row)` via the MSB bit-scan (§0). Store the raw code into
**`>83E7`** for debounce (§5.2).

Loop: `DEC R1 / JOC >0336` continues while `R1 ≥ 0` (columns 5→0).

**No-key exit:** after all columns, `MOV R2,R2 / JNE >03AA` (found) else fall to
`>0382` (no-key finalize, §5.3).

---

## 5. Translation, debounce, and finalize

### 5.1 Modifier → GROM table base (`>03E6`..`>041C`) ✅
Table base is chosen by unit and modifier **priority CTRL > FCTN > SHIFT >
unshifted**. Bases are for scan code 0; the effective address is **base +
`raw`** (the 5-entry / 8-entry pads are absorbed because printable `raw` starts
at 5, joystick `raw` at 8):

| Selector | Base | First real entry | `keymap.rs` const |
|---|---|---|---|
| split unit (`R5≠0`) | `>17C0` | `>17C8` | `JOYSTICK` |
| CTRL held | `>1790` | `>1795` | `CTRL` |
| FCTN held | `>1760` | `>1765` | `FCTN` |
| SHIFT held | `>1730` | `>1735` | `SHIFTED` |
| none | `>1700` | `>1705` | `UNSHIFTED` |

Lookup: `A R3,R1` (base+raw), write high byte (`MOVB R1,@>0402(R13)`) then low
byte (`MOVB @>83E3,@>0402(R13)`) to GROM addr-write port `>9C02`, then
`MOVB *R13,R0` reads the **translated character** from GROM data port `>9800`.

`fctn_probe.rs` confirms the FCTN block carries the arrow keys (FCTN+S/D/E/X →
`>08`/`>09`/`>0B`/`>0A`, read at `>176A`/`>1772`/`>1771`/`>1768`).

### 5.2 Debounce — "new key vs held key" (`>83C6`-`>83CA`) ✅
Per-unit **last raw scan code**: `>83C8` (unit 0/full), `>83C9` (unit 1),
`>83CA` (unit 2) — addressed as `@>83C8(R5)`. `>83C7` is a modifier working
cell; `>83C6` is the persisted translation state (§2). `>83E7` holds this
scan's raw code.

- **Key found, code == last** (`CB @>83E7,@>83C8(R5) / JEQ`): a **held** key.
  Leave `R6 = 0` (no condition), keep `>8375`, do **not** un-blank.
- **Key found, code != last**: a **new** key. `LI R6,>2000` (⇒ cond bit),
  settle-delay (`BL @>0498`), and update `@>83C8(R5) := >83E7` (plus, on a full
  scan, propagate to the split cells `>83C9`/`>83CA` for coherence).
- **Cond bit rule:** `>837C & >20` is set **iff** a new key was detected this
  scan. Held keys and no-key leave it clear. (Auto-repeat is *not* KSCAN's job;
  the GPL caller times it.)

### 5.3 No-key path (`>0382`) ✅
`CLR R6`; `SETO R0` (⇒ `R0 = >FF`); if `@>83C8(R5)` wasn't already `>FF`,
settle-delay; set `@>83C8(R5) := >FF` (and `>83C9`/`>83CA := >FF` on a full
scan). Then finalize: **`>8375 = >FF`, `>837C` cond clear, no un-blank.** In
modes 1/2 the joystick cells `>8376/>8377` still hold the deflections from §3.

### 5.4 Finalize (`>0478`..`>0496`) ✅
1. `MOVB R0,@>8375` — store the result byte (`>FF` if none).
2. `BL @>0842` — restore the GROM address (§1.3).
3. `MOVB R6,@>837C` — store the condition byte (`>20` new / `>00` otherwise);
   the `MOVB` sets EQ when it wrote `>00`.
4. **Un-blank side effect (only on a new key, `R6≠0`):**
   - `MOVB @>83D4,*R15` — write the saved VDP-R1 value (`>83D4`) to the VDP
     address port, then `MOVB @>0B61,*R15` writes the register selector
     **`>81`** (= register 1, write) — reloading **VDP register 1** with the
     display-enable bit set (un-blank).
   - `CLR @>83D6` — reset the screen-blank **timeout** the ISR counts up.
5. `MOV @>83D8,R11 / B *R11` — return.

### 5.5 Alpha-lock & control-code normalization (full scan only, `>0428`..`>0476`) 📖❓
Skipped for split units (`>0420 JNE >0478`). For full-keyboard results the ROM:
- **Alpha-lock case fold:** range-checks `R0` (via the `>04A2` bound helper —
  inline low bound + dynamic high bound `@>83F9`), reads the alpha-lock switch
  through the CRU (`SBZ 21 / … / TB 7 / SBO 21`), and if unlocked folds case via
  `SB @>03B4,R0` (`@>03B4 = >20`). The GROM unshifted block holds **uppercase**
  (`keymap.rs` — flipped to authentic lowercase 2026-07-06), so this produces
  lowercase when alpha-lock is up.
- **Control-code range fixups:** compares `R0` against fixed bounds
  (`@>0025=>0D` Enter, `@>02CA=>0F`, CTRL range `@>0470=>809F`, `@>0586=>835F`)
  and applies `SOCB`/`SZCB` adjustments so the CTRL block yields `>80+n`.

> **⚠ Emulator gap.** `cru.rs` models alpha-lock only as a **write latch** (bit
> 21) — it is not exposed as a readable input, and `TB 7` in our machine reads a
> keyboard **row** of the selected column instead of the alpha-lock switch. The
> fold path therefore needs an emulator decision (model the alpha-lock switch on
> a readable line, or pin the observed default). The *result values* are already
> pinned by `keymap_probe.rs`, so a minimal KSCAN can return the raw GROM byte
> (uppercase) and defer the fold; see §10 and §11.

---

## 6. Un-blank & timeout summary (the ISR handshake)

KSCAN owns the *un-blank* half of the screen-saver; the VBLANK ISR (see
`RECON.md` §6) owns the *blank* half:
- ISR each tick: `INCT >83D6`; on wrap it rebuilds VDP R1 from the `>83D4` copy
  with the blank bit clear (blanks the screen).
- KSCAN on a **new key**: reloads VDP R1 from `>83D4` (un-blanks) and `CLR
  >83D6` (restarts the timeout).
- `>83D4` (VDP-R1 copy) and `>83D6` (timeout) are ISR-seeded cells; KSCAN only
  reads `>83D4` and clears `>83D6`. `>83D6` is intentionally *not* re-seeded at
  boot (`../RECON.md` blank-timeout note) — faithful behaviour.

---

## 7. The CLEAR/BREAK test — `>04B2` (via `>0020`) ✅

A tiny, self-contained two-column CRU probe. Uses only R11 (return) and R12
(CRU); needs **no** GPLWS, no scratchpad, touches neither `>8375` nor the mode
cells. E/A caller: `BL @>0020`; **returns with EQ set iff FCTN-4 (CLEAR/BREAK)
is held.**

1. `LI R12,>0024 / LDCR @>0012,3` — select **column 0** (`@>0012` byte = `>00`).
2. `SRC R12,7` — settle.
3. `LI R12,>0006 / STCR R12,8` — read 8 rows (active-low, no invert).
4. `CZC @>0036,R12` — test mask `>0036 = >1000` = **row 4** (FCTN in column 0).
   CZC sets EQ when the masked bit is 0 (= pressed, active-low). If **not** set
   (FCTN up) → `>04DC B *R11` with EQ **clear** (early out).
5. `LI R12,>0024 / LDCR @>0074,3` — select **column 3** (`@>0074` byte = `>03`).
6. `SRC R12,7`; `LI R12,>0006 / STCR R12,8`.
7. `CZC @>0036,R12` — test the same `>1000` = **row 4** (the `4` key is
   column 3, row 4).
8. `B *R11` — EQ is set **iff both** FCTN (col0/row4) **and** `4` (col3/row4)
   are down.

`@>0012` and `@>0074` are also NASTY harvested constants (RECON §13); place them
as explicit bytes at those addresses. The same `>1000`/`>1100` row-mask logic is
shared with the ISR QUIT test.

---

## 8. Cross-check with the emulator hardware model (must hold)

| KSCAN operation | CRU base (word) | CRU bit(s) | `cru.rs` handler |
|---|---|---|---|
| Column select (keyboard & joystick) | `>0024` | 18-20 (P2-P4, P2=LSB) | `write_bit` 18..=20 |
| Row read (8 rows) | `>0006` | 3-10 (row0=bit3), active-low | `read_bit` 3..=10 |
| Joystick lines (5) | `>0006` | 3-7 | `read_bit` 3..=7 |
| Alpha-lock select | (bit 21) | 21 (P5) | `write_bit` 21 → `alpha_lock` latch |

Column numbering needs **no** reconciliation: the ROM selects columns 0-5 for
keys and 6/7 for joysticks, matching `keyboard.rs` directly (Col6=Joy1,
Col7=Joy2). (The plan §2.1 warns some *external* references number joystick
columns differently — ours agree with the ROM.) `machine.rs` maps the LDCR/STCR
to these 9901 bits.

---

## 9. Scratchpad cell map (KSCAN's view)

| Cell | Meaning |
|---|---|
| `>8374` | mode in (0-5); modes 3-5 rewrite it to state 0-2 |
| `>8375` | **result key** (`>FF` = none) |
| `>8376` / `>8377` | joystick **Y** / **X** deflection (modes 1/2) |
| `>837C` | GPL status; **bit `>20` = condition, set only on a NEW key** |
| `>83C6` | persisted translation state (set by modes 3-5) |
| `>83C7` | modifier working cell |
| `>83C8` / `>83C9` / `>83CA` | last raw scan code, unit 0 / 1 / 2 (debounce) |
| `>83D4` | VDP-R1 copy (read to un-blank) |
| `>83D6` | screen-blank timeout (cleared on a new key) |
| `>83D8` | saved caller return (R11) |
| `>83E0`-`>83FF` | GPLWS (the workspace KSCAN runs on) |
| `>8300+[>8373]` | GPL sub-stack — GROM-address save/restore scratch |

---

## 10. Staged implementation plan

> *Historical: KSCAN is fully implemented and differentially gated
> (`rom_kscan.rs`); this staging plan is preserved as design rationale.*

**Increment M2-1 — "minimal KSCAN" (mode 0, enough to idle a key-wait loop and
register a keypress; unblocks the menu/`scan_check.rs`).** Deliver:
1. Entry/exit: save/restore R11 via `>83D8`; save/restore the GROM address
   (§1.3); `SBO 21` on entry.
2. Mode-0 dispatch only (`>8375=>FF`, cond clear on no key).
3. Full-keyboard scan loop: columns 5→0, `STCR` rows active-low, first-key-wins,
   `raw = col*8 + (7-row)`; column-0 modifier capture (CTRL/FCTN/SHIFT) with the
   `>0F00` key mask.
4. Modifier→base selection (unshift/shift/FCTN/CTRL) and the GROM `base+raw`
   lookup into `>8375`.
5. Debounce against `>83C8`: set `>837C & >20` **only on a code change**; update
   `>83C8`.
6. Un-blank side effect on a new key (reload VDP R1 from `>83D4`, `CLR >83D6`).
7. Defer the alpha-lock fold and control-code fixups — return the raw GROM byte
   (uppercase — keytab since flipped to authentic lowercase 2026-07-06);
   `keymap_probe`/menu still pass because the blocks already carry
   the right values.

Gate: a `SCAN ; BR LOOP` loop (`scan_check.rs`) idles with `>8375=>FF` and, on a
keypress, latches the ASCII in `>8375` with `>837C=>20` for exactly one scan.

**Increment M2-2 — full mode set:**
8. Modes 3/4/5 (subtract-3, store to `>8374`/`>83C6`, state 0); modes ≥6
   short-circuit to no-key.
9. Alpha-lock case fold + control-code range fixups (§5.5) — resolve the
   emulator alpha-lock-input gap first (§11).
10. Split modes 1/2 + joystick: joystick column select (6/7), 5-line read,
    deflection table `>16E0`→`>8376/>8377`; split masks `>0FFF`/`>F0FF`; unit
    debounce cells `>83C9`/`>83CA`; split-keyboard translate via `>17C8`.

**Independent (any time):**
11. The CLEAR/BREAK test `>04B2` (§7) — trivial, standalone, no scratchpad.

Every increment gets a differential microtest (mode × modifier × key → `>8375`,
plus the `>837C` cond and the un-blank writes), per the P9 completeness mandate.

---

## 11. Open questions / uncertainties (flagged)

1. **Alpha-lock readback — RESOLVED (2026-07-04, RECON §23).** The switch read
   is **state-gated**: translation state 0 (the 99/4 state, the zeroed-RAM boot
   default) folds a–z **without reading the switch at all**; only states 1/2
   execute `SBZ 21 / TB 7 / SBO 21`, and on our switchless 9901 the line idles
   high ("not locked") identically under both ROMs — differential integrity by
   construction. Both branches implemented and gated (`rom_kscan.rs`). Making
   the switch functional is emulator ROADMAP §6.
2. **Control-code fixups — RESOLVED (2026-07-04, RECON §23).** Decoded from the
   binary: the whole `>0444–0476` block is state-gated. State 0 rejects
   `>10..>1F` and `> >5F` (`@>0587 = >5F`) as "no key"; state 1 (Pascal) sets
   the `>80` bit for codes `<= >0F` (`@>02CA = >0F`) and clears it for
   `>80..>9F`, Enter (`@>0025 = >0D`) exempt; state 2 does nothing. Implemented
   in the rewrite.
3. **Translation states 1/2 (modes 4/5).** The 99/4 and UCSD-Pascal keyboard
   states persisted via `>83C6` are legacy and unexercised by our GROM/carts;
   their effect on `>83C6`-gated normalization is unverified. Reproduce the
   store-back mechanics; treat the state-dependent decode as ❓ pending a probe.
4. **`@>02F1(R5)` split defaults — RESOLVED (RECON §15).** The
   centered/no-direction scan codes (`>29`/`>25`) fed to the split-keyboard
   translate are the `JOYDEF` table, pinned by the joystick gates.
5. **Settle delay `>0498`** (`LI R12,>04E2` countdown) is pure timing (74LS156
   / matrix settle) — behaviourally inert in emulation but present in the
   authentic image; reproduce as a no-op-equivalent or a token delay.
