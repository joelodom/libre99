# Appendix G — CRU Map

<!-- Appendices · target ≈4 pp · companion to Ch. 10, 21, 30 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The CRU model (G.1) is canon (Ch. 10). The TMS9901 bit map (G.2) and the keyboard/joystick matrix (G.3) are tier-1 for the project's machine: read directly from the emulator core — crates/libre99-core/src/cru.rs (mode/mask/column-select/alpha-lock bits, the /INT2 VDP line, the row reads) and src/keyboard.rs (the full 8×8 column×row matrix, joysticks on columns 6/7). The keyboard/joystick decode was additionally machine-verified in Ch. 21 via BENCH99's `press` (J1U -> >0010, J1U+J1F -> >0011). The cassette/audio P-port bits are HARDWARE facts the project core deliberately does NOT wire (R-12 gap, roadmapped — cru.rs). The peripheral-card CRU bases (G.4) are tier-2 (classic card ABI / manuals), hedged per R-2 and owned by Ch. 30/33/34. -->

The CRU (Communications Register Unit) is the TMS9900's other bus — not memory,
but a parallel space of 4,096 individually addressable I/O *bits*, where every
key, joystick, interrupt line, and peripheral card is wired one bit at a time.
This appendix is the map of who lives at which bit: the addressing model, the
TMS9901's assignments, the keyboard/joystick matrix behind them, and the card
bases out in the expansion box. The teaching — why bit-serial I/O, the factor-of-two
that trips everyone, the porch-light cassette relay — is Chapter 10; the keyboard
and joystick reads are Chapter 21; DSR paging is Chapter 30.

## G.1 The addressing model

- **4,096 bits**, numbered 0–4095, entirely separate from the 64 K memory space.
- **`R12` is the CRU base.** The hardware bit a CRU instruction touches is
  `(R12 >> 1) + displacement` — so the value in `R12` is **twice** the base bit
  number. This ×2 (the single most common CRU confusion) is because `R12` uses
  the bits of a byte address; only the top bits address CRU bits (Ch. 10).
- **Five instructions, and only five:**

| Mnemonic | Bits | Action |
|---|---|---|
| `SBO` | 1 | Set the addressed bit to 1 (output) |
| `SBZ` | 1 | Set the addressed bit to 0 (output) |
| `TB` | 1 | Test the addressed bit into `EQ` (input) |
| `LDCR` | 1–16 | Write a group of bits from a register/byte |
| `STCR` | 1–16 | Read a group of bits into a register/byte |

For `LDCR`/`STCR` of **eight bits or fewer**, the operand is a **byte** (the
high-byte law, Ch. 8); a count of 0 means 16.

## G.2 The TMS9901 (CRU base `>0000`)

The console's interface chip sits at CRU base 0 (`R12 = >0000`). Like the real
chip, a bit number means one thing **written** and another **read**. Bit numbers
below are hardware CRU bits (address = `2 × bit`).

**Written (`SBO`/`SBZ`/`LDCR`) — configuration:**

| Bit(s) | Function |
|---|---|
| 0 | Mode select: **0 = I/O / interrupt** mode, **1 = timer** mode |
| 1–15 | (I/O mode) Interrupt-enable **mask** — bit *n* enables `/INTn`; the console enables **bit 2** (the VDP). (Timer mode: these load the interval timer.) |
| 18–20 | Keyboard **column select** — a 3-bit value on pins P2–P4 (P2 = least significant) choosing which of the 8 columns to read (G.3) |
| 21 | Alpha-lock select (pin P5) |
| 22+ | Cassette motor / audio gate (pins P6+) — **not wired in the project core** (R-12; present on real hardware, Ch. 33) |

**Read (`TB`/`STCR`) — live inputs (idle high; asserted/pressed reads low):**

| Bit(s) | Function |
|---|---|
| 2 | `/INT2` — the VDP vertical-blank interrupt (active low) |
| 3–10 | The eight keyboard **rows** of the currently selected column (active low: pressed = 0) |

On the 99/4A the 9901's priority-encode pins are unconnected, so every enabled
interrupt reaches the CPU as **level 1** — the ISR answers a single "should I take
my level-1 interrupt?" (Ch. 22). The VDP interrupt is enabled by setting mask
bit 2 and is the heartbeat behind sound, sprite motion, and QUIT.

## G.3 The keyboard and joystick matrix

The keyboard is a passive **8×8 switch matrix** scanned over the 9901: write a
column (0–7) to bits 18–20, then read the eight rows on input bits 3–10. Columns
0–5 are the keyboard; columns 6 and 7 are the two joystick ports, sharing the same
row lines. Row *r* is read on CRU input bit `3 + r`.

```text
       Col0    Col1  Col2  Col3  Col4  Col5    Col6(Joy1)  Col7(Joy2)
Row0    =       .     ,     M     N     /       FIRE        FIRE
Row1   SPACE    L     K     J     H     ;       LEFT        LEFT
Row2   ENTER    O     I     U     Y     P       RIGHT       RIGHT
Row3    —       9     8     7     6     0       DOWN        DOWN
Row4   FCTN     2     3     4     5     1       UP          UP
Row5   SHIFT    S     D     F     G     A        —           —
Row6   CTRL     W     E     R     T     Q        —           —
Row7    —       X     C     V     B     Z        —           —
```

`FCTN`, `SHIFT`, and `CTRL` are ordinary cells in column 0 — software reads them
like any key and combines them itself. A joystick read is therefore just a column
6/7 scan: fire on row 0, then left/right/down/up on rows 1–4 (Ch. 21 decodes this
to a clean mask — `press J1U` reads `>0010`, plus fire `>0011`).

> **The Alpha-Lock trap (R-12).** On real hardware the Alpha-Lock key shares a
> matrix line with joystick 1's *up*, so a latched Alpha-Lock reads as
> joystick-up — the bug that made a thousand ships drift upward. The project core
> models each switch independently and does **not** reproduce the shared line, so
> the trap does not bite on the bench; code that must be robust on metal still
> needs the compensation (Ch. 21). Classic99 and MAME model it.

## G.4 System CRU allocation and card bases

The low CRU bits belong to the 9901 (base `>0000`); the peripheral cards in the
expansion box occupy the space up to `>1FFE`, each card decoded at a `>0100`
software-address boundary. A card's DSR ROM is paged into the `>4000`–`>5FFF`
window by **setting CRU bit 0 at the card's base** (`SBO 0` with `R12` = the base)
and paged out by clearing it — the mechanism Chapter 30 builds `DSRLNK` around.

The classic, widely-documented card bases (tier-2 — the community/card-manual ABI;
confirm against the specific card):

| CRU base (`R12`) | Card |
|---|---|
| `>0000` | TMS9901 — keyboard, joysticks, interrupts, timer, cassette |
| `>1100` | Floppy **disk controller** |
| `>1300` | **RS-232 / PIO** card |
| `>1F00` | UCSD **p-code** card |

The project emulator runs a genuine **disk** DSR (canon — Ch. 30–32), so the disk
base responds and `DSRLNK` works on the bench; the bundled cards **poll** rather
than interrupt. RS-232/PIO and cassette are **not** emulated at HEAD (R-12,
Ch. 33) — code for them on the shelf tools or real hardware. Modern cards (SAMS,
TIPI, and the like) claim their own bases; Chapter 34 surveys them.

*See also:* Chapter 10 (the CRU and the 9901), Chapter 21 (keyboard, joysticks,
the Alpha-Lock trap), Chapter 30 (DSRs and DSR paging), Appendix C (the
scratchpad bytes KSCAN leaves its results in), Appendix H (DSR headers and device
names).
