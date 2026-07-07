# Appendix D — TMS9918A Reference

<!-- Appendices · target ≈8 pp · companion to Part III (Ch. 12–18) · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The ports (D.1), registers and base-address formulas (D.2), status register (D.3), display modes and table layouts (D.4), colour palette (D.5), and sprite model (D.6) are tier-1 for the project's machine: read directly from the emulator core crates/libre99-core/src/vdp.rs (the port interface, the R0–R7 decoders name_base/color_base/pattern_base/sprite_*_base, the ST_F/ST_5S/ST_C flags, the four *_line rasterizers, the 16-entry PALETTE, and sprite_line). The palette RGB values are the exact 0x00RRGGBB constants MAME and js99er share (vdp.rs). The status-read side effect (clears the top three flags) is cross-checked tier-3 against Classic99 Tiemul.cpp ("VDPS &= 0x1f ... tested on hardware"). The VRAM access-timing cookbook (D.7) is behavioural/hardware truth (tier-2 datasheet convention, tier-3 Classic99): the project core does NOT model a per-access VRAM penalty (R-15 deviation, stated in place), so the spacing rule is asserted as hardware truth with the emulator's non-enforcement noted. -->

The TMS9918A is the Video Display Processor — the chip that draws the screen and
owns the machine's other 16 K. It is a co-processor with **its own private 16 KiB
of VRAM** that the CPU cannot address directly: the CPU reaches video memory only
by poking four memory-mapped ports, and the VDP autonomously scans VRAM 60 times
a second to produce a 256×192 picture in up to 15 colors plus transparent. This
appendix is the register-and-format card; the teaching — the funnel, the
address-port dance, why the display is a separate memory world — is Part III
(Chapters 12–18), and the project's `vdplib` is Chapter 12.

## D.1 The CPU ports and the address dance

The VDP occupies four addresses in the console memory map (Appendix C):

| Address | Operation |
|---|---|
| `>8800` | read VRAM data — returns a **prefetched** byte, then auto-increments |
| `>8802` | read the **status** register — clears the interrupt flag (D.3) |
| `>8C00` | write VRAM data — then auto-increments |
| `>8C02` | write the address counter / a control register (two-byte sequence) |

**Setting the address (`>8C02`) is a two-byte sequence.** Software writes the
**low** 8 address bits first, then a second byte whose top two bits select the
operation and whose low six bits are the high address / register data:

```text
second byte   meaning
00aaaaaa      set VRAM address for READING   (and prefetch the first byte)
01aaaaaa      set VRAM address for WRITING
100000rrr →   write VDP register r           (bit 7 of the second byte set)
```

Three rules follow, all tier-1 (`vdp.rs`) and hardware-confirmed:

- The **14-bit** address counter auto-increments (mod 16 K) after **every** data
  access, read or write — so a block transfer sets the address once and then
  streams.
- A **read** setup prefetches: `>8800` returns a byte held from the *previous*
  access and then refills from the counter, so the first `>8800` after a read
  setup returns the byte the setup prefetched. (A write setup does not prefetch.)
- A **flip-flop** tracks first-vs-second control byte. **Reading the status
  register resets it** — the standard recovery if software loses sync mid-sequence
  (Ch. 12; the boot-bug lesson in `docs/history/POSTMORTEMS.md` is the CPU-side
  analog).

## D.2 The registers R0–R7

The VDP has eight write-only registers, set through the register-write form of
`>8C02`. Bit layouts and the base-address formulas are tier-1 (`vdp.rs`):

```text
R0   . . . . . . M3 EXT            M3 = mode bit 3 (bit 1); EXT = external video
R1   16K BL IE M1 M2 . SZ MAG      BL = display enable, IE = interrupt enable,
                                   M1/M2 = mode bits, SZ = sprite size, MAG = magnify
R2   name table base               = (R2 & >0F) << 10
R3   color table base              =  R3 << 6            (masked in Graphics II)
R4   pattern generator base        = (R4 & >07) << 11    (masked in Graphics II)
R5   sprite attribute base         = (R5 & >7F) << 7
R6   sprite pattern base           = (R6 & >07) << 11
R7   text foreground | backdrop     high nibble | low nibble
```

The two you touch every frame are **R1** (bit 6 `BL` turns the display on; bit 5
`IE` enables the vblank interrupt; bits 4/3 `M1`/`M2` and bits 1/0 `SZ`/`MAG`)
and the **base registers** R2–R6, which place the tables in VRAM. Note the shift
counts: a name-table base is `(R2 & >0F) × >0400`, a pattern base
`(R4 & >07) × >0800`, a sprite-attribute base `(R5 & >7F) × >0080`. Write a base
register with a value that puts two tables on top of each other and you get the
classic "sprites in the text" corruption — the addresses are the whole game
(Ch. 12).

## D.3 The status register

One read-only byte, fetched at `>8802`:

| Bit | Mask | Name | Meaning |
|---|---|---|---|
| 7 | `>80` | **F** | frame / vertical-blank flag — set at end of active display |
| 6 | `>40` | **5S** | a **fifth** sprite appeared on some scanline |
| 5 | `>20` | **C** | two sprites **collided** (coincidence) |
| 4–0 | `>1F` | — | the number of the offending fifth sprite |

**Reading the status register has side effects** (tier-1 `vdp.rs`, tier-3
Classic99 `Tiemul.cpp`: "`VDPS &= 0x1f;` — top flags are cleared on read, tested
on hardware"): it returns the current value and then **clears F, 5S, and C**, and
resets the control flip-flop (D.1). Clearing **F** is how the console's interrupt
handler *acknowledges* the vblank interrupt — read the status once per frame and
the interrupt releases; forget to, and it re-fires in a storm (Ch. 22). The **C**
and **5S** flags latch as the beam meets those conditions, so a mid-frame status
read that clears C can see it re-latch later in the very same frame — exactly as
on hardware, because the chip re-evaluates each scanline (`vdp.rs` models the
beam line-by-line).

An interrupt is requested when **F is set and R1 bit 5 (IE) is enabled** — the
two-condition test in `vdp.rs::interrupt_pending`.

## D.4 The display modes

Mode bits `M1 M2 M3` (R1 bits 4, 3 and R0 bit 1) select one of four modes; an
illegal combination degrades toward Graphics I (`vdp.rs::mode`).

| M1 M2 M3 | Mode | Layout |
|---|---|---|
| `0 0 0` | **Graphics I** | 32×24 cells, one 8×8 pattern per character, one fg/bg color per **group of 8** characters |
| `0 0 1` | **Graphics II** (bitmap) | 32×24 cells over three independent 2 K thirds — every one of the 768 cells can have a unique 8×8 bitmap with **per-row** colors |
| `0 1 0` | **Multicolor** | each 8×8 cell is a 2×2 grid of 4×4 solid color blocks |
| `1 0 0` | **Text** | 40×24 columns of **6×8** characters, a single fg/bg pair from R7, **no sprites** |

Table roles per mode (tier-1, the four `*_line` rasterizers in `vdp.rs`):

- **Graphics I** reads the **name table** (R2) for each cell's character, indexes
  the **pattern table** (R4) for the 8×8 bitmap, and takes fg/bg from the
  **color table** (R3), one color byte per eight consecutive characters.
- **Graphics II** splits the screen into three vertical thirds; R3/R4 act as a
  half-select plus an AND-mask so the name table can address a unique pattern and
  color entry per cell — the mode that makes true bitmaps possible (Ch. 15).
- **Multicolor** takes two pattern bytes per cell (selected by vertical position)
  and paints four 4×4 color blocks — a chunky 64×48 color grid.
- **Text** is 40 columns of 6-pixel-wide glyphs (only the top 6 bits of each
  pattern byte show), one fg/bg pair for the whole screen, and **no sprite
  plane** — the mode `textlib40` uses (Ch. 14).

Color index **0 is transparent** everywhere; where a foreground or background
selects it, the backdrop (R7 low nibble) shows through (`vdp.rs`).

## D.5 The color palette

The TMS9918A produces 16 fixed colors (index 0 transparent). The RGB values below
are the exact `0x00RRGGBB` constants the project renders (tier-1 `vdp.rs::PALETTE`)
— the same values MAME and js99er use, so a screenshot from the bench matches a
screenshot from the shelf emulators pixel-for-pixel:

| # | Color | RGB | | # | Color | RGB |
|---|---|---|---|---|---|---|
| 0 | transparent | — (shown as black) | | 8 | medium red | `#FC5554` |
| 1 | black | `#000000` | | 9 | light red | `#FF7978` |
| 2 | medium green | `#21C842` | | 10 | dark yellow | `#D4C154` |
| 3 | light green | `#5EDC78` | | 11 | light yellow | `#E6CE80` |
| 4 | dark blue | `#5455ED` | | 12 | dark green | `#21B03B` |
| 5 | light blue | `#7D76FC` | | 13 | magenta | `#C95BBA` |
| 6 | dark red | `#D4524D` | | 14 | gray | `#CCCCCC` |
| 7 | cyan | `#42EBF5` | | 15 | white | `#FFFFFF` |

These are one emulator's well-chosen approximation of an analog NTSC signal, not
laboratory truth — real sets and later revisions (the 9928/9929, the F18A) shift
the hues (Ch. 34). For matching the book's machine, they are exact.

## D.6 Sprites

Up to **32 sprites**, each described by **four bytes** in the sprite-attribute
table (R5). Format and rules are tier-1 (`vdp.rs::sprite_line`):

```text
byte 0   Y     the line ABOVE the sprite (drawn at Y+1); Y = >D0 ENDS the list
byte 1   X     horizontal position
byte 2   pattern index (16×16 sprites use pattern & >FC — four 8×8 quadrants)
byte 3   early-clock (bit 7: shift left 32 px) | color (low nibble)
```

- **`Y` names the line above the sprite**, so a sprite is drawn at `Y+1`; a `Y`
  byte of `>D0` (208) **terminates** the active list — no later slot is scanned.
  (A `Y` near the bottom wraps so a sprite slides in from the top edge rather
  than vanishing — the project matches Classic99's `> 225` fade-in threshold.)
- **Size and magnify** come from R1: `SZ` (bit 1) picks 16×16 over 8×8; `MAG`
  (bit 0) doubles every sprite pixel. A 16×16 sprite is four 8×8 quadrants at
  `pattern & >FC`.
- **Four sprites per scanline.** A **fifth** on any line is not drawn and sets
  **5S** with its number in the status low bits (D.3). Lower-numbered sprites
  have priority and are drawn on top.
- **Coincidence (C)** is set when two sprites assert the same pixel, tested on
  **pattern** bits regardless of color — even a transparent (color 0) sprite
  contributes to the collision mask though it paints nothing (Classic99: "even
  transparent sprites get drawn into the collision buffer"). Read collisions
  through the C flag; the project does not implement the separate `COINC` GPL
  opcode (Appendix B).

## D.7 VRAM access timing — the cookbook

The one thing this card cannot give you from the project core is a cycle penalty
for accessing VRAM too fast, and it matters on real hardware, so state it as
hardware truth (R-15):

- **During active display the VDP is busy** fetching pattern and name bytes for
  the beam, and it grants the CPU only narrow windows to touch VRAM. Back-to-back
  `>8C00` writes issued faster than roughly one every ~8 µs during active display
  can be **dropped** on real hardware — the classic "snow" or missing bytes when
  a program blasts VRAM mid-frame.
- **The cookbook, therefore:** do bulk VRAM work either (a) with the **display
  blanked** (R1 bit 6 clear — the VDP stops fetching and grants full bandwidth),
  or (b) during **vertical blank**, from the interrupt handler, where the whole
  ~4,000-cycle blanking interval is yours (Ch. 22). A single write between frames
  is always safe; a `MOVE` of a screenful is not, unless blanked or vblank-timed.

**Project deviation (R-15, honest):** the emulator core accepts every VRAM access
immediately — it models the address counter, the prefetch, and the flip-flop, but
**not** a per-access timing penalty (`vdp.rs`). So on the bench a too-fast VRAM
blast **succeeds** where hardware would drop bytes. Code that must run on metal
still needs the spacing above; the bench will not warn you. Classic99 and MAME
model the contention; when a VRAM-heavy routine works on the bench, confirm it on
one of them (or hardware) before trusting the timing (CROSS-VALIDATION).

*See also:* Chapter 12 (`vdplib`, the ports and tables), Chapter 13 (`textlib`),
Chapter 14 (40-column text), Chapter 15 (bitmap mode), Chapter 16 (sprites and
DODGE), Chapter 18 (split-screen and the status flags), Chapter 22 (the vblank
ISR and vblank-timed VRAM work), Chapter 34 (the F18A and later VDPs), Appendix C
(the VDP ports in the memory map), Appendix E (the sound chip's neighboring port),
Appendix J (character patterns loaded into the pattern table).
