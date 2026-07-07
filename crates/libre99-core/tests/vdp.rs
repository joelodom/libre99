// Modified MIT License
//
// Copyright (c) 2026 Joel Odom
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, and sublicense copies of the
// Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// "Commons Clause" License Condition v1.0
//
// The Software is provided to you by the Licensor under the License, subject to
// the following condition.
//
// Without limiting other conditions in the License, the grant of rights under the
// License will not include, and the License does not grant to you, the right to
// Sell the Software.
//
// For purposes of the foregoing, "Sell" means practicing any or all of the rights
// granted to you under the License to provide to third parties, for a fee or other
// consideration (including without limitation fees for hosting or consulting/
// support services related to the Software), a product or service whose value
// derives, entirely or substantially, from the functionality of the Software. Any
// license notice or attribution required by the License must also include this
// Commons Clause License Condition notice.
//
// Software: Libre99
//
// License: Modified MIT
//
// Licensor: Joel Odom
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! TMS9918A VDP conformance tests.
//!
//! These cover the CPU-facing port protocol (the two-byte address/register setup,
//! the read prefetch, auto-increment, the status/interrupt handshake) and the
//! software rasterizer (Graphics I, Text, and sprites). RGB values are the
//! standard emulator palette (see `vdp.rs`).

use libre99_core::vdp::{Vdp, HEIGHT, WIDTH};

const WHITE: u32 = 0x00FF_FFFF;
const DARK_BLUE: u32 = 0x0054_55ED;
const BLACK: u32 = 0x0000_0000;
const MEDIUM_GREEN: u32 = 0x0021_C842; // palette index 2
const LIGHT_GREEN: u32 = 0x005E_DC78; // palette index 3
const CYAN: u32 = 0x0042_EBF5; // palette index 7

/// Write VDP register `reg` with `val` (low byte first, then the `0x80|reg`
/// command byte).
fn write_reg(v: &mut Vdp, reg: u8, val: u8) {
    v.write_control(val);
    v.write_control(0x80 | reg);
}
/// Point the VRAM address counter at `addr` for writing (top bits `01`).
fn set_write_addr(v: &mut Vdp, addr: u16) {
    v.write_control(addr as u8);
    v.write_control(0x40 | ((addr >> 8) as u8 & 0x3F));
}
/// Point the VRAM address counter at `addr` for reading (top bits `00`).
fn set_read_addr(v: &mut Vdp, addr: u16) {
    v.write_control(addr as u8);
    v.write_control((addr >> 8) as u8 & 0x3F);
}
/// Write a run of bytes to VRAM starting at `addr`.
fn write_vram(v: &mut Vdp, addr: u16, bytes: &[u8]) {
    set_write_addr(v, addr);
    for &b in bytes {
        v.write_data(b);
    }
}
fn blank_fb() -> Vec<u32> {
    vec![0; WIDTH * HEIGHT]
}

#[test]
fn register_write_via_control_port() {
    let mut v = Vdp::new();
    write_reg(&mut v, 1, 0xE0);
    assert_eq!(v.register(1), 0xE0);
    write_reg(&mut v, 7, 0x1F);
    assert_eq!(v.register(7), 0x1F);
}

#[test]
fn vram_write_then_read_uses_prefetch() {
    let mut v = Vdp::new();
    write_vram(&mut v, 0x0100, &[0xAA, 0xBB]);
    set_read_addr(&mut v, 0x0100);
    // The read-setup prefetched mem[0x0100]; the first read returns it.
    assert_eq!(v.read_data(), 0xAA);
    assert_eq!(v.read_data(), 0xBB);
}

#[test]
fn vram_address_autoincrements_and_wraps_at_16k() {
    let mut v = Vdp::new();
    // Write at the very top of VRAM; the counter must wrap to 0.
    set_write_addr(&mut v, 0x3FFF);
    v.write_data(0x5A);
    v.write_data(0x42); // lands at 0x0000 after wrap
    set_read_addr(&mut v, 0x3FFF);
    assert_eq!(v.read_data(), 0x5A);
    assert_eq!(v.read_data(), 0x42);
}

#[test]
fn reading_status_clears_frame_flag() {
    let mut v = Vdp::new();
    v.vblank();
    assert!(v.read_status() & 0x80 != 0, "F set after vblank");
    assert!(v.read_status() & 0x80 == 0, "F cleared by the read");
}

#[test]
fn interrupt_pending_needs_frame_flag_and_enable() {
    let mut v = Vdp::new();
    write_reg(&mut v, 1, 0x20); // IE on
    assert!(!v.interrupt_pending(), "no frame flag yet");
    v.vblank();
    assert!(v.interrupt_pending(), "F and IE -> interrupt");
    v.read_status();
    assert!(!v.interrupt_pending(), "status read acknowledges it");
    write_reg(&mut v, 1, 0x00); // IE off
    v.vblank();
    assert!(!v.interrupt_pending(), "disabled -> no interrupt");
}

#[test]
fn status_read_resets_the_control_byte_latch() {
    // Begin a two-byte address write, read the status (which resets the latch),
    // then a fresh full setup must still be interpreted as low-then-high.
    let mut v = Vdp::new();
    write_vram(&mut v, 0x0200, &[0x77]);
    v.write_control(0x00); // stray first byte
    v.read_status(); // resets the latch
    set_read_addr(&mut v, 0x0200);
    assert_eq!(v.read_data(), 0x77);
}

#[test]
fn graphics1_renders_foreground_and_background() {
    let mut v = Vdp::new();
    // Graphics I, screen on (BLANK=1). R0=0; R1 bit6 (0x40) = display enable.
    write_reg(&mut v, 0, 0x00);
    write_reg(&mut v, 1, 0x40);
    write_reg(&mut v, 2, 0x00); // name table @ >0000
    write_reg(&mut v, 3, 0x10); // color table @ >0400
    write_reg(&mut v, 4, 0x01); // pattern table @ >0800
    write_reg(&mut v, 7, 0x01); // backdrop = black
    // Cell (0,0) shows character 1.
    write_vram(&mut v, 0x0000, &[0x01]);
    // Character 1 pattern, row 0 = 0x80 (only the leftmost pixel set).
    write_vram(&mut v, 0x0800 + 8, &[0x80, 0, 0, 0, 0, 0, 0, 0]);
    // Color entry for character group 0: fg=white(15), bg=dark blue(4).
    write_vram(&mut v, 0x0400, &[0xF4]);

    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[0], WHITE, "pixel (0,0) is the set foreground pixel");
    assert_eq!(fb[1], DARK_BLUE, "pixel (1,0) is background");
}

#[test]
fn text_mode_uses_r7_colors() {
    let mut v = Vdp::new();
    // Text mode: M1 = R1 bit4 (0x10); screen on (0x40).
    write_reg(&mut v, 0, 0x00);
    write_reg(&mut v, 1, 0x50);
    write_reg(&mut v, 2, 0x00); // name @ >0000
    write_reg(&mut v, 4, 0x01); // pattern @ >0800
    write_reg(&mut v, 7, 0xF4); // fg=white, bg=dark blue
    write_vram(&mut v, 0x0000, &[0x01]); // cell (0,0) = char 1
    write_vram(&mut v, 0x0800 + 8, &[0x80, 0, 0, 0, 0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[0], WHITE);
    assert_eq!(fb[1], DARK_BLUE);
}

#[test]
fn text_mode_transparent_foreground_shows_the_backdrop() {
    // Color 0 is transparent on BOTH halves of R7: with fg=0 the "on" pixels
    // must show the backdrop (here dark blue), not palette black.
    let mut v = Vdp::new();
    write_reg(&mut v, 0, 0x00);
    write_reg(&mut v, 1, 0x50);
    write_reg(&mut v, 2, 0x00);
    write_reg(&mut v, 4, 0x01);
    write_reg(&mut v, 7, 0x04); // fg=transparent, backdrop/bg=dark blue
    write_vram(&mut v, 0x0000, &[0x01]);
    write_vram(&mut v, 0x0800 + 8, &[0x80, 0, 0, 0, 0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[0], DARK_BLUE, "transparent fg pixel renders as backdrop");
    assert_eq!(fb[1], DARK_BLUE);
}

// ---------------------------------------------------------------------------
// Graphics II (bitmap) mode
// ---------------------------------------------------------------------------

/// A Graphics II stage: bitmap mode, display on, name table @ >1800, pattern
/// generator @ >0000 (R4=>03 ⇒ base 0, full mask), color table @ >2000
/// (R3=>FF ⇒ base >2000, full mask), backdrop black. All 768 name-table cells
/// default to character 0 (VRAM starts zero), so the three vertical thirds each
/// read character 0 from their OWN 2 KiB slice of the pattern/color tables — the
/// whole point of the mode.
fn graphics2_stage() -> Vdp {
    let mut v = Vdp::new();
    write_reg(&mut v, 0, 0x02); // M3 = 1  → Graphics II
    write_reg(&mut v, 1, 0x40); // display on
    write_reg(&mut v, 2, 0x06); // name table @ >1800
    write_reg(&mut v, 3, 0xFF); // color base >2000, color mask >1FFF
    write_reg(&mut v, 4, 0x03); // pattern base 0, pattern mask >1FFF
    write_reg(&mut v, 7, 0x01); // backdrop black
    v
}

// The name table indexes THREE independent thirds: rows 0-7 read patterns/colors
// from slice 0 (>0000/>2000), rows 8-15 from slice 1 (>0800/>2800), rows 16-23
// from slice 2 (>1000/>3000). Character 0 appears in every cell here, yet each
// third draws a DIFFERENT pattern because the third's 0x800/0x1000 base is added
// to the table-relative address. A naive "ch*8+line" implementation that ignored
// the thirds would draw slice 0's pattern in all three bands and fail.
#[test]
fn graphics2_name_table_indexes_three_thirds() {
    let mut v = graphics2_stage();
    // Slice 0 (rows 0-7): char 0 row 0 pattern = >80 (pixel 0 lit).
    write_vram(&mut v, 0x0000, &[0x80, 0, 0, 0, 0, 0, 0, 0]);
    // Slice 1 (rows 8-15): char 0 row 0 pattern = >40 (pixel 1 lit).
    write_vram(&mut v, 0x0800, &[0x40, 0, 0, 0, 0, 0, 0, 0]);
    // Slice 2 (rows 16-23): char 0 row 0 pattern = >20 (pixel 2 lit).
    write_vram(&mut v, 0x1000, &[0x20, 0, 0, 0, 0, 0, 0, 0]);
    // Each slice's row-0 color = white foreground / black background.
    write_vram(&mut v, 0x2000, &[0xF1]); // slice 0
    write_vram(&mut v, 0x2800, &[0xF1]); // slice 1
    write_vram(&mut v, 0x3000, &[0xF1]); // slice 2

    let mut fb = blank_fb();
    v.render(&mut fb);

    // Third 0 (y=0): pixel 0 lit.
    assert_eq!(fb[0], WHITE, "third 0 lights pixel 0");
    // Third 1 (row 8 → y=64): pixel 1 lit, pixel 0 dark — proves the >0800 base.
    assert_eq!(fb[64 * WIDTH + 1], WHITE, "third 1 lights pixel 1");
    assert_eq!(fb[64 * WIDTH], BLACK, "third 1 is NOT slice 0's pixel 0");
    // Third 2 (row 16 → y=128): pixel 2 lit, pixel 0 dark — proves the >1000 base.
    assert_eq!(fb[128 * WIDTH + 2], WHITE, "third 2 lights pixel 2");
    assert_eq!(fb[128 * WIDTH], BLACK, "third 2 is NOT slice 0's pixel 0");
}

// Graphics II carries an INDEPENDENT color byte for every one of the 8 rows of a
// cell (unlike Graphics I, one color per 8-character group). Light pixel 0 on
// all eight rows of cell (0,0) and give each row a different foreground.
#[test]
fn graphics2_per_row_colors() {
    let mut v = graphics2_stage();
    write_vram(&mut v, 0x0000, &[0x80; 8]); // char 0: pixel 0 lit on every row
    write_vram(&mut v, 0x2000, &[0xF1]); // row 0 fg = white
    write_vram(&mut v, 0x2001, &[0x31]); // row 1 fg = light green
    write_vram(&mut v, 0x2002, &[0x71]); // row 2 fg = cyan

    let mut fb = blank_fb();
    v.render(&mut fb);

    assert_eq!(fb[0], WHITE, "row 0 color");
    assert_eq!(fb[WIDTH], LIGHT_GREEN, "row 1 color (per-row, not per-cell)");
    assert_eq!(fb[2 * WIDTH], CYAN, "row 2 color");
}

// R3/R4 are AND-masks over the table-relative address, mirroring MAME:
//   colormask   = ((R3 & 0x7F) << 6) | 0x3F
//   patternmask = ((R4 & 0x03) << 11) | (colormask & 0x7FF)
// With R4 = >00 (low bits 00, not the usual 03) the pattern mask collapses to
// >07FF, folding the 6 KiB pattern generator onto its first 2 KiB — so third 2's
// nominal >1000 pattern address wraps back to slice 0. Pin that fold: a naive
// implementation that skipped the mask would read the un-folded >1000 pattern.
#[test]
fn graphics2_r3_r4_masking_folds_pattern_table() {
    let mut v = graphics2_stage();
    write_reg(&mut v, 4, 0x00); // pattern base 0, pattern mask collapses to >07FF
    // Slice-0 pattern (what the fold makes third 2 read): pixel 0 lit.
    write_vram(&mut v, 0x0000, &[0x80, 0, 0, 0, 0, 0, 0, 0]);
    // The UN-folded >1000 pattern a naive impl would read instead: pixel 7 lit.
    write_vram(&mut v, 0x1000, &[0x01, 0, 0, 0, 0, 0, 0, 0]);
    write_vram(&mut v, 0x2000, &[0xF1]); // slice 0 color (white/black)
    write_vram(&mut v, 0x3000, &[0xF1]); // slice 2 color (color table is NOT folded)

    let mut fb = blank_fb();
    v.render(&mut fb);

    assert_eq!(fb[0], WHITE, "third 0 pixel 0 lit");
    // Third 2 (y=128) reads the FOLDED slice-0 pattern (pixel 0), not >1000.
    assert_eq!(fb[128 * WIDTH], WHITE, "masked: third 2 folds to slice 0 (pixel 0)");
    assert_eq!(
        fb[128 * WIDTH + 7],
        BLACK,
        "masked: pixel 7 dark — a naive unmasked read of >1000 would light it"
    );
}

// ---------------------------------------------------------------------------
// Multicolor mode
// ---------------------------------------------------------------------------

/// A Multicolor stage: M2 = 1, display on, name table @ >0000, pattern
/// generator @ >0800, backdrop black.
fn multicolor_stage() -> Vdp {
    let mut v = Vdp::new();
    write_reg(&mut v, 0, 0x00);
    write_reg(&mut v, 1, 0x48); // display on | M2 (Multicolor)
    write_reg(&mut v, 2, 0x00); // name table @ >0000
    write_reg(&mut v, 4, 0x01); // pattern generator @ >0800
    write_reg(&mut v, 7, 0x01); // backdrop black
    v
}

// Each 8×8 Multicolor cell is a 2×2 grid of 4×4 solid blocks. Two pattern bytes
// per cell: the top byte's high nibble is the top-left block and its low nibble
// the top-right; the bottom byte does the bottom row. Color 0 is the backdrop.
#[test]
fn multicolor_block_layout_and_backdrop() {
    let mut v = multicolor_stage();
    // Cell (0,0) is character 0 (VRAM zero). Pattern bytes at >0800 / >0801.
    write_vram(&mut v, 0x0800, &[0x2F]); // top:    left=green(2)     right=white(15)
    write_vram(&mut v, 0x0801, &[0x40]); // bottom: left=dark blue(4) right=color0

    let mut fb = blank_fb();
    v.render(&mut fb);

    // Top-left 4×4 block (px 0-3, lines 0-3).
    assert_eq!(fb[0], MEDIUM_GREEN, "top-left block");
    assert_eq!(fb[3 * WIDTH + 3], MEDIUM_GREEN, "still inside top-left block");
    // Top-right block (px 4-7).
    assert_eq!(fb[4], WHITE, "top-right block = top byte's low nibble");
    // Bottom-left block (lines 4-7).
    assert_eq!(fb[4 * WIDTH], DARK_BLUE, "bottom-left block = bottom byte high nibble");
    // Bottom-right block: color 0 → backdrop.
    assert_eq!(fb[4 * WIDTH + 4], BLACK, "color 0 renders as the backdrop");
}

// The pattern byte pair is selected by (row & 3) * 2 + (line >> 2): cell rows
// 0,1,2,3 use byte pairs (0,1),(2,3),(4,5),(6,7) and row 4 wraps back to (0,1).
// Give row 0 and row 1 of the same character distinct top bytes and prove each
// row picks its own pair — a naive implementation reusing byte 0 would fail.
#[test]
fn multicolor_row_selects_pattern_byte_pair() {
    let mut v = multicolor_stage();
    write_vram(&mut v, 0x0800, &[0x2F]); // char 0, byte 0 (row 0 top): left = green
    write_vram(&mut v, 0x0802, &[0x70]); // char 0, byte 2 (row 1 top): left = cyan

    let mut fb = blank_fb();
    v.render(&mut fb);

    assert_eq!(fb[0], MEDIUM_GREEN, "cell row 0 uses pattern byte 0");
    // Cell row 1 spans y=8..15; its top block comes from byte 2, not byte 0.
    assert_eq!(fb[8 * WIDTH], CYAN, "cell row 1 uses pattern byte 2");
}

// The (row & 3) selector wraps: cell row 4 reuses byte pair (0,1), not a
// nonexistent pair (8,9). The existing pair test only exercises rows 0/1, so a
// per-line rasterizer that derived the pair from the raw row would slip past
// it — this pins the wrap itself.
#[test]
fn multicolor_row_four_wraps_back_to_the_first_byte_pair() {
    let mut v = multicolor_stage();
    write_vram(&mut v, 0x0800, &[0x2F]); // byte 0: rows 0 AND 4, top-left = green
    write_vram(&mut v, 0x0802, &[0x70]); // byte 2: rows 1 and 5 — must not bleed in

    let mut fb = blank_fb();
    v.render(&mut fb);

    // Cell row 4 spans y=32..39; its top-left block must come from byte 0.
    assert_eq!(fb[32 * WIDTH], MEDIUM_GREEN, "cell row 4 wraps to pattern byte 0");
}

#[test]
fn sprite_renders_at_its_position() {
    let mut v = Vdp::new();
    write_reg(&mut v, 0, 0x00);
    write_reg(&mut v, 1, 0x40); // screen on, 8x8 sprites, no magnify
    write_reg(&mut v, 5, 0x06); // sprite attribute table @ >0300
    write_reg(&mut v, 6, 0x00); // sprite pattern table @ >0000
    write_reg(&mut v, 7, 0x01); // backdrop black
    // Sprite 0: Y=10 (top row is line 11), X=20, pattern 0, color white(15).
    // Sprite 1: Y=0xD0 terminates the list.
    write_vram(&mut v, 0x0300, &[10, 20, 0, 0x0F, 0xD0, 0, 0, 0]);
    // Pattern 0, row 0 = 0x80 (top-left pixel of the sprite).
    write_vram(&mut v, 0x0000, &[0x80, 0, 0, 0, 0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[11 * WIDTH + 20], WHITE, "sprite pixel at (20,11)");
    assert_eq!(fb[0], BLACK, "elsewhere is backdrop");
}

#[test]
fn fifth_sprite_on_a_line_sets_status_flag() {
    let mut v = Vdp::new();
    write_reg(&mut v, 1, 0x40);
    write_reg(&mut v, 5, 0x06); // sprite attrs @ >0300
    write_reg(&mut v, 6, 0x00);
    // Five sprites all crossing line 11 (Y=10), then a terminator.
    let mut attrs = vec![];
    for i in 0..5 {
        attrs.extend_from_slice(&[10, (i * 16) as u8, 0, 0x0F]);
    }
    attrs.extend_from_slice(&[0xD0, 0, 0, 0]);
    write_vram(&mut v, 0x0300, &attrs);
    write_vram(&mut v, 0x0000, &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert!(v.read_status() & 0x40 != 0, "fifth-sprite flag set");
}

/// A minimal sprite stage shared by the sprite status/position tests:
/// Graphics I, display on, backdrop black, sprite attributes @ >0300, sprite
/// patterns @ >0800 (clear of the name/pattern/color tables at >0000, which
/// sit over all-zero VRAM so the whole background renders as the backdrop),
/// sprite pattern 0 = a solid 8x8 block.
fn sprite_stage() -> Vdp {
    let mut v = Vdp::new();
    write_reg(&mut v, 0, 0x00);
    write_reg(&mut v, 1, 0x40); // display on, 8x8 sprites, no magnify
    write_reg(&mut v, 5, 0x06); // sprite attribute table @ >0300
    write_reg(&mut v, 6, 0x01); // sprite pattern table @ >0800
    write_reg(&mut v, 7, 0x01); // backdrop black
    write_vram(&mut v, 0x0800, &[0xFF; 8]); // sprite pattern 0: solid block
    v
}

// Sprite Y wraps at 8 bits, so a sprite with Y in the >E1..>FF range enters
// partially from the TOP of the screen instead of vanishing. Classic99
// tivdp.cpp: `yy=VDP[curSAL]+1; if (yy>225) yy-=256; // fade in from top`.
// Anything that slides a sprite in from the top edge pops into view without it.
#[test]
fn sprite_enters_from_top_of_screen() {
    let mut v = sprite_stage();
    // Y = >F8: top = 249 wraps to -7, so of the solid 8x8 block only the last
    // row is on screen — on line 0.
    write_vram(&mut v, 0x0300, &[0xF8, 100, 0, 0x0F, 0xD0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[100], WHITE, "the sprite's last row shows on scanline 0");
    assert_eq!(fb[WIDTH + 100], BLACK, "the sprite ends above scanline 1");
}

#[test]
fn sprite_y_ff_puts_the_first_row_on_line_0() {
    let mut v = sprite_stage();
    // Y = >FF means "first sprite row on line 0" (top = 256 wraps to 0). Use a
    // pattern lit only in row 0 to prove it is the FIRST row that lands there.
    write_vram(&mut v, 0x0808, &[0x80, 0, 0, 0, 0, 0, 0, 0]); // pattern 1
    write_vram(&mut v, 0x0300, &[0xFF, 100, 1, 0x0F, 0xD0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[100], WHITE, "pattern row 0 shows on scanline 0");
    assert_eq!(fb[WIDTH + 100], BLACK, "row 1 (unlit) is on scanline 1");
}

// Pin the exact wrap threshold — top > 225 wraps, top = 225 does not — with the
// case where the alternative "(line - Y - 1) & 0xFF" formulation disagrees: a
// 32-pixel-tall (magnified 16x16) sprite near Y ≈ 224. The threshold value is
// Classic99's (its own comment calls it a best guess), reproduced per project
// policy.
#[test]
fn sprite_y_wrap_threshold_for_tall_sprites() {
    let mut v = sprite_stage();
    write_reg(&mut v, 1, 0x43); // display on | 16x16 sprites | magnify
    write_vram(&mut v, 0x0800, &[0xFF; 32]); // patterns 0-3: solid 16x16

    // Y = >E1: top = 226 wraps to -30; the last rows of the 32-pixel-tall
    // sprite are still crossing lines 0..1.
    write_vram(&mut v, 0x0300, &[0xE1, 100, 0, 0x0F, 0xD0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[100], WHITE, "Y=>E1 (top 226) wraps: visible on line 0");

    // Y = >E0: top = 225 does NOT wrap; the sprite is entirely below the
    // screen. (An &0xFF formulation would wrongly show it on lines 0..6.)
    write_vram(&mut v, 0x0300, &[0xE0, 100, 0, 0x0F, 0xD0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_eq!(fb[100], BLACK, "Y=>E0 (top 225) does not wrap: off screen");
}

// Reading the VDP status register clears ALL THREE top flags — F, 5S, and C —
// not just F. Classic99 Tiemul.cpp: `VDPS &= 0x1f; // top flags are cleared on
// read (tested on hardware)`. Games acknowledge a collision by reading status;
// if C survived the read, a poll loop would see one collision as many.
#[test]
fn status_read_clears_coincidence_flag() {
    let mut v = sprite_stage();
    // Two solid sprites at the same spot -> coincidence.
    write_vram(&mut v, 0x0300, &[50, 60, 0, 0x01, 50, 60, 0, 0x02, 0xD0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);

    let first = v.read_status();
    assert_ne!(first & 0x20, 0, "setup: coincidence must be detected");
    let second = v.read_status();
    assert_eq!(second & 0x20, 0, "the status read cleared C");
}

// A TRANSPARENT (color 0) sprite still participates in coincidence — the
// hardware detects collision on pattern bits, not visible color, and invisible
// hitbox sprites are a standard TI technique. Classic99 tivdp.cpp: "Even
// transparent sprites get drawn into the collision buffer."
#[test]
fn transparent_sprite_sets_coincidence() {
    let mut v = sprite_stage();
    // Sprite 0: transparent, solid pattern. Sprite 1: white, same spot.
    write_vram(&mut v, 0x0300, &[50, 60, 0, 0x00, 50, 60, 0, 0x0F, 0xD0, 0, 0, 0]);
    let mut fb = blank_fb();
    v.render(&mut fb);

    let status = v.read_status();
    assert_ne!(status & 0x20, 0, "coincidence detected on pattern bits");
    // Only the framebuffer write is skipped: the visible sprite's pixel must
    // not be painted over (or erased) by the transparent one on top.
    assert_eq!(fb[51 * WIDTH + 60], WHITE, "transparent sprite paints nothing");
}

#[test]
fn status_read_clears_fifth_sprite_flag() {
    let mut v = sprite_stage();
    // Five sprites all crossing one scanline -> 5S.
    let mut attrs = vec![];
    for i in 0..5 {
        attrs.extend_from_slice(&[10, (i * 16) as u8, 0, 0x0F]);
    }
    attrs.extend_from_slice(&[0xD0, 0, 0, 0]);
    write_vram(&mut v, 0x0300, &attrs);
    let mut fb = blank_fb();
    v.render(&mut fb);

    let first = v.read_status();
    assert_ne!(first & 0x40, 0, "setup: fifth sprite must be detected");
    let second = v.read_status();
    assert_eq!(second & 0x40, 0, "the status read cleared 5S");
}

// The 5S/C lifecycle is set-by-scan / cleared-by-read: evaluate_sprites() runs
// the per-scanline scan (the machine calls it once per frame at vblank, before
// the frame's CPU slice) and only ever SETS the flags; rendering produces
// pixels only. The four tests below pin each corner of that lifecycle.

#[test]
fn evaluate_sprites_sets_flags_without_rendering() {
    let mut v = sprite_stage();
    // Two solid sprites at the same spot -> C; no framebuffer involved.
    write_vram(&mut v, 0x0300, &[50, 60, 0, 0x01, 50, 60, 0, 0x02, 0xD0, 0, 0, 0]);
    v.evaluate_sprites();
    assert_ne!(
        v.read_status() & 0x20,
        0,
        "coincidence is frame-scan state, not a render side effect"
    );
}

#[test]
fn sprite_flags_persist_across_scans_until_status_read() {
    let mut v = sprite_stage();
    // Latch C from an overlap...
    write_vram(&mut v, 0x0300, &[50, 60, 0, 0x01, 50, 60, 0, 0x02, 0xD0, 0, 0, 0]);
    v.evaluate_sprites();
    // ...then separate the sprites; later scans and renders must NOT clear the
    // latched flag — only a status read does.
    write_vram(&mut v, 0x0300, &[50, 60, 0, 0x01, 100, 160, 0, 0x02, 0xD0, 0, 0, 0]);
    v.evaluate_sprites();
    let mut fb = blank_fb();
    v.render(&mut fb);
    assert_ne!(v.read_status() & 0x20, 0, "C persisted until the read");
    v.evaluate_sprites();
    assert_eq!(
        v.read_status() & 0x20,
        0,
        "after the read-clear, a scan with no overlap leaves C clear"
    );
}

#[test]
fn coincidence_reraises_after_read_while_sprites_still_overlap() {
    let mut v = sprite_stage();
    write_vram(&mut v, 0x0300, &[50, 60, 0, 0x01, 50, 60, 0, 0x02, 0xD0, 0, 0, 0]);
    v.evaluate_sprites();
    assert_ne!(v.read_status() & 0x20, 0, "first frame: C set (and cleared)");
    // The sprites still overlap, so the next frame's scan re-detects it —
    // exactly what hardware does after the ISR acknowledges a collision.
    v.evaluate_sprites();
    assert_ne!(v.read_status() & 0x20, 0, "next scan re-raises C");
}

#[test]
fn blanked_display_skips_sprite_evaluation_but_keeps_flags() {
    let mut v = sprite_stage();
    write_vram(&mut v, 0x0300, &[50, 60, 0, 0x01, 50, 60, 0, 0x02, 0xD0, 0, 0, 0]);
    v.evaluate_sprites(); // display on: C latched
    write_reg(&mut v, 1, 0x00); // blank the display (BL=0)
    v.evaluate_sprites(); // no scan while blanked...
    assert_ne!(v.read_status() & 0x20, 0, "...latched C persists until read");
    v.evaluate_sprites(); // still blanked: the standing overlap is not seen
    assert_eq!(
        v.read_status() & 0x20,
        0,
        "no sprite scan happens while the display is blanked"
    );
}

// ---------------------------------------------------------------------------
// Sprite features beyond one 8×8 sprite
// ---------------------------------------------------------------------------

// A 16×16 sprite is four 8×8 quadrants stored as bytes 0-7 = top-left,
// 8-15 = bottom-left, 16-23 = top-right, 24-31 = bottom-right — the COLUMNS are
// the second (outer) pair, so bottom-left precedes top-right. This matches
// Classic99 (tivdp.cpp: p_add for the same-column bottom half at +7, the far
// column top half at +15, far column bottom at +23). The pattern index's low
// two bits are ignored (a big sprite spans four consecutive patterns): pattern
// >06 resolves to base 4. Lighting a distinct pixel in each quadrant pins both
// the quadrant ORDER and the &>FC masking; a row-major (…,TR,BL,…) layout or an
// unmasked base 6 would land the pixels elsewhere.
#[test]
fn sprite_16x16_quadrant_layout() {
    let mut v = Vdp::new();
    write_reg(&mut v, 0, 0x00);
    write_reg(&mut v, 1, 0x42); // display on | 16×16 sprites, no magnify
    write_reg(&mut v, 5, 0x06); // sprite attribute table @ >0300
    write_reg(&mut v, 6, 0x01); // sprite pattern table @ >0800
    write_reg(&mut v, 7, 0x01); // backdrop black
    // Sprite 0: Y=20 (top row 21), X=50, pattern >06 (→ base 4), color white.
    write_vram(&mut v, 0x0300, &[20, 50, 0x06, 0x0F, 0xD0, 0, 0, 0]);
    // Pattern base 4 ⇒ 32 bytes at >0820..>083F. Each quadrant's row-0 byte gets
    // a distinct bit so its screen landing is unambiguous.
    write_vram(&mut v, 0x0820, &[0x80]); // top-left,     col 0  → x 50
    write_vram(&mut v, 0x0828, &[0x40]); // bottom-left,  col 1  → x 51, y +8
    write_vram(&mut v, 0x0830, &[0x20]); // top-right,    col 10 → x 60
    write_vram(&mut v, 0x0838, &[0x10]); // bottom-right, col 11 → x 61, y +8

    let mut fb = blank_fb();
    v.render(&mut fb);

    assert_eq!(fb[21 * WIDTH + 50], WHITE, "top-left quadrant (bytes 0-7)");
    assert_eq!(fb[29 * WIDTH + 51], WHITE, "bottom-left quadrant (bytes 8-15)");
    assert_eq!(fb[21 * WIDTH + 60], WHITE, "top-right quadrant (bytes 16-23)");
    assert_eq!(fb[29 * WIDTH + 61], WHITE, "bottom-right quadrant (bytes 24-31)");
}

// Magnification (MG=1) doubles every sprite pixel, so an 8×8 sprite covers a
// 16×16 screen area. Pin the extent: the corners are lit, one row/column past
// each edge is not.
#[test]
fn sprite_magnification_doubles_extent() {
    let mut v = sprite_stage();
    write_reg(&mut v, 1, 0x41); // display on | magnify (8×8 sprites)
    // Sprite 0: Y=20 (top 21), X=50, solid pattern 0, white.
    write_vram(&mut v, 0x0300, &[20, 50, 0, 0x0F, 0xD0, 0, 0, 0]);

    let mut fb = blank_fb();
    v.render(&mut fb);

    assert_eq!(fb[21 * WIDTH + 50], WHITE, "top-left corner");
    assert_eq!(fb[36 * WIDTH + 65], WHITE, "bottom-right corner (16×16 extent)");
    assert_eq!(fb[37 * WIDTH + 50], BLACK, "one row below the magnified sprite");
    assert_eq!(fb[21 * WIDTH + 66], BLACK, "one column right of the magnified sprite");
}

// The early-clock bit (attribute byte 3, bit >80) shifts X left by 32, so a
// sprite can be drawn partially off the left edge. With X=28 and early clock the
// origin is -4: columns 0-3 are clipped and columns 4-7 land at x 0-3, while the
// un-shifted position (x 28) stays empty.
#[test]
fn sprite_early_clock_shifts_left_by_32() {
    let mut v = sprite_stage();
    // Sprite 0: Y=20 (top 21), X=28, solid pattern 0, flags = early | white.
    write_vram(&mut v, 0x0300, &[20, 28, 0, 0x8F, 0xD0, 0, 0, 0]);

    let mut fb = blank_fb();
    v.render(&mut fb);

    assert_eq!(fb[21 * WIDTH], WHITE, "column 4 lands at x 0 (origin -4)");
    assert_eq!(fb[21 * WIDTH + 3], WHITE, "column 7 lands at x 3");
    assert_eq!(
        fb[21 * WIDTH + 28],
        BLACK,
        "the un-shifted X position is empty — early clock moved the sprite left"
    );
}

// Where two sprites overlap, the LOWER-numbered sprite is drawn on top. Sprite 0
// (white) and sprite 1 (dark blue) overlap; the shared pixels show sprite 0.
#[test]
fn sprite_priority_lower_number_on_top() {
    let mut v = sprite_stage();
    write_vram(
        &mut v,
        0x0300,
        &[20, 50, 0, 0x0F, 20, 54, 0, 0x04, 0xD0, 0, 0, 0],
    );

    let mut fb = blank_fb();
    v.render(&mut fb);

    assert_eq!(fb[21 * WIDTH + 50], WHITE, "sprite 0 only");
    assert_eq!(fb[21 * WIDTH + 54], WHITE, "overlap: lower-numbered sprite 0 wins");
    assert_eq!(fb[21 * WIDTH + 60], DARK_BLUE, "sprite 1 only");
}

// When a fifth sprite appears on a scanline the 5S flag sets AND the offending
// sprite's NUMBER lands in the status register's low 5 bits (Classic99
// tivdp.cpp:2644 stores b5OnLine there). Arrange four low-numbered sprites plus
// sprite 7 on one line (with sprites 4-6 parked elsewhere) so the fifth is
// number 7, not merely a count.
#[test]
fn fifth_sprite_number_lands_in_status_low_bits() {
    let mut v = sprite_stage();
    let attrs = [
        10, 0, 0, 0x0F, // sprite 0  on line 11
        10, 16, 0, 0x0F, // sprite 1
        10, 32, 0, 0x0F, // sprite 2
        10, 48, 0, 0x0F, // sprite 3
        100, 0, 0, 0x0F, // sprite 4  parked on line 101
        100, 16, 0, 0x0F, // sprite 5
        100, 32, 0, 0x0F, // sprite 6
        10, 64, 0, 0x0F, // sprite 7  — the FIFTH sprite on line 11
        0xD0, 0, 0, 0, // terminator
    ];
    write_vram(&mut v, 0x0300, &attrs);

    let mut fb = blank_fb();
    v.render(&mut fb);

    let status = v.read_status();
    assert_ne!(status & 0x40, 0, "5S flag set");
    assert_eq!(
        status & 0x1F,
        7,
        "the fifth sprite's NUMBER (7), not a count, is latched in the low bits"
    );
}
