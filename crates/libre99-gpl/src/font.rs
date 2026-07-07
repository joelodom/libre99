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

//! The **TI-99/4A character sets**: the standard 8×8 set (codes `>20–>5F`), the
//! thin "small" set (same codes), and the lower-case small-capitals set (codes
//! `>60–>7E`), reproduced faithfully so on-screen text matches the real console.
//!
//! The 64 glyphs below are the exact patterns the console loads into the VDP
//! pattern table at boot — byte-identical to the console GROM's contiguous font
//! block at GROM `>04B4`, which [`tests::matches_authentic_character_set`] gates
//! and `examples/font_extract.rs` recovers. They are identity-mapped so
//! character code == ASCII; codes outside the range render blank.
//!
//! [`emit_gpl_bytes`] renders the range as GPL `BYTE` directives so the font can
//! be spliced into GPL source as a GROM data block at `>1000`.

/// The first and last ASCII codes the font defines (inclusive).
pub const FIRST: u8 = 0x20;
pub const LAST: u8 = 0x5F;
/// Number of glyphs, and total byte size of the packed font.
pub const COUNT: usize = (LAST - FIRST) as usize + 1;
pub const BYTES: usize = COUNT * 8;

/// The authentic 8×8 pattern for every code `>20..>5F` (one row per byte, MSB =
/// leftmost pixel), in code order. Verified byte-for-byte against the console
/// GROM by [`tests::matches_authentic_character_set`].
const GLYPHS: [[u8; 8]; COUNT] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // >20 space
    [0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x00, 0x20], // >21 !
    [0x48, 0x48, 0x48, 0x00, 0x00, 0x00, 0x00, 0x00], // >22 "
    [0x00, 0x48, 0xFC, 0x48, 0x48, 0xFC, 0x48, 0x00], // >23 #
    [0x10, 0x3C, 0x50, 0x38, 0x14, 0x78, 0x10, 0x00], // >24 $
    [0xC0, 0xC4, 0x08, 0x10, 0x20, 0x40, 0x8C, 0x0C], // >25 %
    [0x60, 0x90, 0x90, 0x60, 0x60, 0x94, 0x88, 0x74], // >26 &
    [0x08, 0x10, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00], // >27 '
    [0x08, 0x10, 0x20, 0x20, 0x20, 0x20, 0x10, 0x08], // >28 (
    [0x40, 0x20, 0x10, 0x10, 0x10, 0x10, 0x20, 0x40], // >29 )
    [0x00, 0x00, 0x48, 0x30, 0xCC, 0x30, 0x48, 0x00], // >2A *
    [0x00, 0x00, 0x10, 0x10, 0x7C, 0x10, 0x10, 0x00], // >2B +
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x10, 0x20], // >2C ,
    [0x00, 0x00, 0x00, 0x00, 0x7C, 0x00, 0x00, 0x00], // >2D -
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x30], // >2E .
    [0x00, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x00], // >2F /
    [0x38, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x38], // >30 0
    [0x10, 0x30, 0x50, 0x10, 0x10, 0x10, 0x10, 0x7C], // >31 1
    [0x78, 0x84, 0x04, 0x08, 0x10, 0x20, 0x40, 0xFC], // >32 2
    [0x78, 0x84, 0x04, 0x38, 0x04, 0x04, 0x84, 0x78], // >33 3
    [0x0C, 0x14, 0x24, 0x44, 0x84, 0xFC, 0x04, 0x04], // >34 4
    [0xF8, 0x80, 0x80, 0xF8, 0x04, 0x04, 0x84, 0x78], // >35 5
    [0x78, 0x80, 0x80, 0xF8, 0x84, 0x84, 0x84, 0x78], // >36 6
    [0xFC, 0x04, 0x04, 0x08, 0x10, 0x20, 0x40, 0x40], // >37 7
    [0x78, 0x84, 0x84, 0x78, 0x84, 0x84, 0x84, 0x78], // >38 8
    [0x78, 0x84, 0x84, 0x84, 0x7C, 0x04, 0x04, 0x78], // >39 9
    [0x00, 0x30, 0x30, 0x00, 0x00, 0x30, 0x30, 0x00], // >3A :
    [0x00, 0x30, 0x30, 0x00, 0x00, 0x30, 0x10, 0x20], // >3B ;
    [0x00, 0x08, 0x10, 0x20, 0x40, 0x20, 0x10, 0x08], // >3C <
    [0x00, 0x00, 0x00, 0x7C, 0x00, 0x7C, 0x00, 0x00], // >3D =
    [0x00, 0x40, 0x20, 0x10, 0x08, 0x10, 0x20, 0x40], // >3E >
    [0x38, 0x44, 0x04, 0x08, 0x10, 0x10, 0x00, 0x10], // >3F ?
    [0x00, 0x78, 0x84, 0x9C, 0xA4, 0x98, 0x80, 0x7C], // >40 @
    [0x78, 0x84, 0x84, 0x84, 0xFC, 0x84, 0x84, 0x84], // >41 A
    [0xF8, 0x44, 0x44, 0x78, 0x44, 0x44, 0x44, 0xF8], // >42 B
    [0x78, 0x84, 0x80, 0x80, 0x80, 0x80, 0x84, 0x78], // >43 C
    [0xF8, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0xF8], // >44 D
    [0xFC, 0x80, 0x80, 0xF0, 0x80, 0x80, 0x80, 0xFC], // >45 E
    [0xFC, 0x80, 0x80, 0xF0, 0x80, 0x80, 0x80, 0x80], // >46 F
    [0x78, 0x84, 0x80, 0x80, 0x9C, 0x84, 0x84, 0x78], // >47 G
    [0x84, 0x84, 0x84, 0xFC, 0x84, 0x84, 0x84, 0x84], // >48 H
    [0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x7C], // >49 I
    [0x04, 0x04, 0x04, 0x04, 0x04, 0x84, 0x84, 0x78], // >4A J
    [0x88, 0x90, 0xA0, 0xC0, 0xA0, 0x90, 0x88, 0x84], // >4B K
    [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7C], // >4C L
    [0x84, 0xCC, 0xB4, 0x84, 0x84, 0x84, 0x84, 0x84], // >4D M
    [0x84, 0xC4, 0xA4, 0x94, 0x8C, 0x84, 0x84, 0x84], // >4E N
    [0xFC, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0xFC], // >4F O
    [0xF8, 0x84, 0x84, 0x84, 0xF8, 0x80, 0x80, 0x80], // >50 P
    [0x78, 0x84, 0x84, 0x84, 0x84, 0x94, 0x88, 0x74], // >51 Q
    [0xF8, 0x84, 0x84, 0x84, 0xF8, 0x90, 0x88, 0x84], // >52 R
    [0x78, 0x84, 0x80, 0x78, 0x04, 0x04, 0x84, 0x78], // >53 S
    [0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10], // >54 T
    [0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x78], // >55 U
    [0x44, 0x44, 0x44, 0x44, 0x28, 0x28, 0x10, 0x10], // >56 V
    [0x84, 0x84, 0x84, 0x84, 0x84, 0xB4, 0xCC, 0x84], // >57 W
    [0x84, 0x84, 0x48, 0x30, 0x30, 0x48, 0x84, 0x84], // >58 X
    [0x44, 0x44, 0x44, 0x28, 0x10, 0x10, 0x10, 0x10], // >59 Y
    [0xFC, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0xFC], // >5A Z
    [0x38, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x38], // >5B [
    [0x00, 0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x00], // >5C \
    [0x70, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x70], // >5D ]
    [0x10, 0x28, 0x44, 0x82, 0x00, 0x00, 0x00, 0x00], // >5E ^
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFC], // >5F _
];

/// The 8×8 pattern for character `c`, or all-zero for codes outside `>20..>5F`.
pub fn glyph(c: u8) -> [u8; 8] {
    if (FIRST..=LAST).contains(&c) {
        GLYPHS[(c - FIRST) as usize]
    } else {
        [0u8; 8]
    }
}

/// The packed font for `>20–>5F`, 8 bytes per glyph, ready to MOVE into the VDP
/// pattern table at `>0800 + >20*8 = >0900`.
pub fn packed() -> Vec<u8> {
    let mut out = Vec::with_capacity(BYTES);
    for c in FIRST..=LAST {
        out.extend_from_slice(&glyph(c));
    }
    out
}

/// Render the font as GPL `BYTE` lines (8 bytes per glyph, one glyph per line,
/// annotated with the character) for splicing into GPL source.
pub fn emit_gpl_bytes(label: &str) -> String {
    let mut s = String::new();
    for (i, c) in (FIRST..=LAST).enumerate() {
        let g = glyph(c);
        let bytes: Vec<String> = g.iter().map(|b| format!(">{b:02X}")).collect();
        let lbl = if i == 0 { label } else { "" };
        let shown = if c == b' ' { "space".to_string() } else { (c as char).to_string() };
        s.push_str(&format!("{lbl:<7} BYTE {}   ; {shown}\n", bytes.join(",")));
    }
    s
}

// ============================================================================
// The console's SECOND ("small") character set — codes `>20–>5F`, thin glyphs.
// ============================================================================
//
// The console GROM holds a *second*, thinner character set contiguously at GROM
// `>06B4` (immediately after the main set at `>04B4`). Each glyph is stored as
// **seven** rows (the top row is always blank); the console's load utility
// (interconnect slot `>0018`, authentic GROM `>039E`) copies the seven rows to
// VDP rows 1–7 and clears row 0. Games that want a compact font — TI Invaders is
// one — set the destination cell `>834A` and CALL `>0018` to load it. This
// module ships the seven-row patterns byte-identical to `>06B4`
// ([`tests::matches_authentic_thin_set`] gates it) and expands them to eight
// rows (leading blank) when emitting the GROM data block, so a plain 512-byte
// `MOVE` reproduces the authentic loader's effect.

/// Rows actually stored per thin glyph in the console GROM (top row is implied
/// blank), and the expanded (VDP) size once the blank top row is prepended.
pub const THIN_ROWS: usize = 7;
/// The seven stored rows for every code `>20..>5F`, in code order — byte-for-byte
/// the console GROM's thin set at `>06B4`, gated by
/// [`tests::matches_authentic_thin_set`].
const THIN_GLYPHS: [[u8; THIN_ROWS]; COUNT] = [
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // >20 space
    [0x10, 0x10, 0x10, 0x10, 0x10, 0x00, 0x10], // >21 !
    [0x28, 0x28, 0x28, 0x00, 0x00, 0x00, 0x00], // >22 "
    [0x28, 0x28, 0x7C, 0x28, 0x7C, 0x28, 0x28], // >23 #
    [0x38, 0x54, 0x50, 0x38, 0x14, 0x54, 0x38], // >24 $
    [0x60, 0x64, 0x08, 0x10, 0x20, 0x4C, 0x0C], // >25 %
    [0x20, 0x50, 0x50, 0x20, 0x54, 0x48, 0x34], // >26 &
    [0x08, 0x08, 0x10, 0x00, 0x00, 0x00, 0x00], // >27 '
    [0x08, 0x10, 0x20, 0x20, 0x20, 0x10, 0x08], // >28 (
    [0x20, 0x10, 0x08, 0x08, 0x08, 0x10, 0x20], // >29 )
    [0x00, 0x28, 0x10, 0x7C, 0x10, 0x28, 0x00], // >2A *
    [0x00, 0x10, 0x10, 0x7C, 0x10, 0x10, 0x00], // >2B +
    [0x00, 0x00, 0x00, 0x00, 0x30, 0x10, 0x20], // >2C ,
    [0x00, 0x00, 0x00, 0x7C, 0x00, 0x00, 0x00], // >2D -
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x30], // >2E .
    [0x00, 0x04, 0x08, 0x10, 0x20, 0x40, 0x00], // >2F /
    [0x38, 0x44, 0x44, 0x44, 0x44, 0x44, 0x38], // >30 0
    [0x10, 0x30, 0x10, 0x10, 0x10, 0x10, 0x38], // >31 1
    [0x38, 0x44, 0x04, 0x08, 0x10, 0x20, 0x7C], // >32 2
    [0x38, 0x44, 0x04, 0x18, 0x04, 0x44, 0x38], // >33 3
    [0x08, 0x18, 0x28, 0x48, 0x7C, 0x08, 0x08], // >34 4
    [0x7C, 0x40, 0x78, 0x04, 0x04, 0x44, 0x38], // >35 5
    [0x18, 0x20, 0x40, 0x78, 0x44, 0x44, 0x38], // >36 6
    [0x7C, 0x04, 0x08, 0x10, 0x20, 0x20, 0x20], // >37 7
    [0x38, 0x44, 0x44, 0x38, 0x44, 0x44, 0x38], // >38 8
    [0x38, 0x44, 0x44, 0x3C, 0x04, 0x08, 0x30], // >39 9
    [0x00, 0x30, 0x30, 0x00, 0x30, 0x30, 0x00], // >3A :
    [0x00, 0x30, 0x30, 0x00, 0x30, 0x10, 0x20], // >3B ;
    [0x08, 0x10, 0x20, 0x40, 0x20, 0x10, 0x08], // >3C <
    [0x00, 0x00, 0x7C, 0x00, 0x7C, 0x00, 0x00], // >3D =
    [0x20, 0x10, 0x08, 0x04, 0x08, 0x10, 0x20], // >3E >
    [0x38, 0x44, 0x04, 0x08, 0x10, 0x00, 0x10], // >3F ?
    [0x38, 0x44, 0x5C, 0x54, 0x5C, 0x40, 0x38], // >40 @
    [0x38, 0x44, 0x44, 0x7C, 0x44, 0x44, 0x44], // >41 A
    [0x78, 0x24, 0x24, 0x38, 0x24, 0x24, 0x78], // >42 B
    [0x38, 0x44, 0x40, 0x40, 0x40, 0x44, 0x38], // >43 C
    [0x78, 0x24, 0x24, 0x24, 0x24, 0x24, 0x78], // >44 D
    [0x7C, 0x40, 0x40, 0x78, 0x40, 0x40, 0x7C], // >45 E
    [0x7C, 0x40, 0x40, 0x78, 0x40, 0x40, 0x40], // >46 F
    [0x3C, 0x40, 0x40, 0x5C, 0x44, 0x44, 0x38], // >47 G
    [0x44, 0x44, 0x44, 0x7C, 0x44, 0x44, 0x44], // >48 H
    [0x38, 0x10, 0x10, 0x10, 0x10, 0x10, 0x38], // >49 I
    [0x04, 0x04, 0x04, 0x04, 0x04, 0x44, 0x38], // >4A J
    [0x44, 0x48, 0x50, 0x60, 0x50, 0x48, 0x44], // >4B K
    [0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7C], // >4C L
    [0x44, 0x6C, 0x54, 0x54, 0x44, 0x44, 0x44], // >4D M
    [0x44, 0x64, 0x64, 0x54, 0x4C, 0x4C, 0x44], // >4E N
    [0x7C, 0x44, 0x44, 0x44, 0x44, 0x44, 0x7C], // >4F O
    [0x78, 0x44, 0x44, 0x78, 0x40, 0x40, 0x40], // >50 P
    [0x38, 0x44, 0x44, 0x44, 0x54, 0x48, 0x34], // >51 Q
    [0x78, 0x44, 0x44, 0x78, 0x50, 0x48, 0x44], // >52 R
    [0x38, 0x44, 0x40, 0x38, 0x04, 0x44, 0x38], // >53 S
    [0x7C, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10], // >54 T
    [0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x38], // >55 U
    [0x44, 0x44, 0x44, 0x28, 0x28, 0x10, 0x10], // >56 V
    [0x44, 0x44, 0x44, 0x54, 0x54, 0x54, 0x28], // >57 W
    [0x44, 0x44, 0x28, 0x10, 0x28, 0x44, 0x44], // >58 X
    [0x44, 0x44, 0x28, 0x10, 0x10, 0x10, 0x10], // >59 Y
    [0x7C, 0x04, 0x08, 0x10, 0x20, 0x40, 0x7C], // >5A Z
    [0x38, 0x20, 0x20, 0x20, 0x20, 0x20, 0x38], // >5B [
    [0x00, 0x40, 0x20, 0x10, 0x08, 0x04, 0x00], // >5C \
    [0x38, 0x08, 0x08, 0x08, 0x08, 0x08, 0x38], // >5D ]
    [0x00, 0x10, 0x28, 0x44, 0x00, 0x00, 0x00], // >5E ^
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7C], // >5F _
];

/// The seven stored rows for thin-set character `c`, or all-zero outside range.
fn thin_rows(c: u8) -> [u8; THIN_ROWS] {
    if (FIRST..=LAST).contains(&c) {
        THIN_GLYPHS[(c - FIRST) as usize]
    } else {
        [0u8; THIN_ROWS]
    }
}

/// The thin set packed as it is stored in the console GROM: seven rows per
/// glyph, in code order. Gated byte-identical to `>06B4` by the test below.
pub fn packed_thin() -> Vec<u8> {
    let mut out = Vec::with_capacity(COUNT * THIN_ROWS);
    for c in FIRST..=LAST {
        out.extend_from_slice(&thin_rows(c));
    }
    out
}

/// Render the thin set **as stored in the console GROM** — seven rows per glyph,
/// no blank top row — for splicing at the authentic home address `>06B4`. Unlike
/// [`emit_gpl_bytes_thin`] (the eight-row *loader* block used by `FONT2`), this
/// is byte-identical to the authentic `>06B4` block, so a cartridge that reads
/// the thin font from its documented address gets the real bytes (B1). Gated by
/// [`tests::matches_authentic_thin_set`].
pub fn emit_gpl_bytes_thin_stored(label: &str) -> String {
    let mut s = String::new();
    for (i, c) in (FIRST..=LAST).enumerate() {
        let rows = thin_rows(c);
        let bytes: Vec<String> = rows.iter().map(|b| format!(">{b:02X}")).collect();
        let lbl = if i == 0 { label } else { "" };
        let shown = if c == b' ' { "space".to_string() } else { (c as char).to_string() };
        s.push_str(&format!("{lbl:<7} BYTE {}   ; {shown}\n", bytes.join(",")));
    }
    s
}

/// Render the thin set as GPL `BYTE` lines, **expanded to eight rows** (a blank
/// top row prepended to the seven stored rows) so a plain 512-byte `MOVE` of
/// this block reproduces the authentic loader (`>039E`), which clears row 0 and
/// copies rows 1–7.
pub fn emit_gpl_bytes_thin(label: &str) -> String {
    let mut s = String::new();
    for (i, c) in (FIRST..=LAST).enumerate() {
        let rows = thin_rows(c);
        let bytes: Vec<String> = std::iter::once(0u8)
            .chain(rows)
            .map(|b| format!(">{b:02X}"))
            .collect();
        let lbl = if i == 0 { label } else { "" };
        let shown = if c == b' ' { "space".to_string() } else { (c as char).to_string() };
        s.push_str(&format!("{lbl:<7} BYTE {}   ; {shown}\n", bytes.join(",")));
    }
    s
}

// ============================================================================
// The console's LOWER-CASE ("small capitals") set — codes `>60–>7E`.
// ============================================================================
//
// The 99/4A console GROM stores the lower-case glyphs — TI's lowercase is
// small capitals — contiguously at GROM `>0874`, immediately after the thin
// set, in the same seven-rows-per-glyph form. The load utility is the fixed
// GPLLNK service entry `>004A` (authentic loader `>03C2`, which parameterizes
// the shared engine at `>03A7` with source `>0874` and count `>1F`): the
// caller points `>834A` at the VDP address for code `>60`'s pattern and CALLs
// `>004A`; the engine writes a blank top row plus the seven stored rows for
// each of the **31** glyphs (`>60..>7E` — `>7F` gets none), advancing `>834A`
// by the 248 bytes written. Parsec stages the set this way and then copies
// glyphs into its own character codes for the in-game small-caps text
// ("press fire to begin"); without the loader those slots keep leftover
// full-size patterns — the garble Joel reported. This module ships the
// seven-row patterns byte-identical to `>0874`
// ([`tests::matches_authentic_lower_set`] gates it) and expands them to eight
// rows for the loader block `FONT3`, exactly as the thin set does for `FONT2`.

/// The first and last codes the lower-case set defines (inclusive) and the
/// glyph count — 31, per the authentic loader's count parameter (`>7F` is not
/// part of the set).
pub const LOWER_FIRST: u8 = 0x60;
pub const LOWER_LAST: u8 = 0x7E;
pub const LOWER_COUNT: usize = (LOWER_LAST - LOWER_FIRST) as usize + 1;

/// The seven stored rows for every code `>60..>7E`, in code order — byte-for-byte
/// the console GROM's lower-case set at `>0874`, gated by
/// [`tests::matches_authentic_lower_set`]. The letters are small capitals with
/// two blank leading stored rows; `` ` ``/`{`/`|`/`}`/`~` use taller forms.
const LOWER_GLYPHS: [[u8; THIN_ROWS]; LOWER_COUNT] = [
    [0x00, 0x20, 0x10, 0x08, 0x00, 0x00, 0x00], // >60 `
    [0x00, 0x00, 0x38, 0x44, 0x7C, 0x44, 0x44], // >61 a
    [0x00, 0x00, 0x78, 0x24, 0x38, 0x24, 0x78], // >62 b
    [0x00, 0x00, 0x3C, 0x40, 0x40, 0x40, 0x3C], // >63 c
    [0x00, 0x00, 0x78, 0x24, 0x24, 0x24, 0x78], // >64 d
    [0x00, 0x00, 0x7C, 0x40, 0x78, 0x40, 0x7C], // >65 e
    [0x00, 0x00, 0x7C, 0x40, 0x78, 0x40, 0x40], // >66 f
    [0x00, 0x00, 0x3C, 0x40, 0x5C, 0x44, 0x38], // >67 g
    [0x00, 0x00, 0x44, 0x44, 0x7C, 0x44, 0x44], // >68 h
    [0x00, 0x00, 0x38, 0x10, 0x10, 0x10, 0x38], // >69 i
    [0x00, 0x00, 0x08, 0x08, 0x08, 0x48, 0x30], // >6A j
    [0x00, 0x00, 0x24, 0x28, 0x30, 0x28, 0x24], // >6B k
    [0x00, 0x00, 0x40, 0x40, 0x40, 0x40, 0x7C], // >6C l
    [0x00, 0x00, 0x44, 0x6C, 0x54, 0x44, 0x44], // >6D m
    [0x00, 0x00, 0x44, 0x64, 0x54, 0x4C, 0x44], // >6E n
    [0x00, 0x00, 0x7C, 0x44, 0x44, 0x44, 0x7C], // >6F o
    [0x00, 0x00, 0x78, 0x44, 0x78, 0x40, 0x40], // >70 p
    [0x00, 0x00, 0x38, 0x44, 0x54, 0x48, 0x34], // >71 q
    [0x00, 0x00, 0x78, 0x44, 0x78, 0x48, 0x44], // >72 r
    [0x00, 0x00, 0x3C, 0x40, 0x38, 0x04, 0x78], // >73 s
    [0x00, 0x00, 0x7C, 0x10, 0x10, 0x10, 0x10], // >74 t
    [0x00, 0x00, 0x44, 0x44, 0x44, 0x44, 0x38], // >75 u
    [0x00, 0x00, 0x44, 0x44, 0x28, 0x28, 0x10], // >76 v
    [0x00, 0x00, 0x44, 0x44, 0x54, 0x54, 0x28], // >77 w
    [0x00, 0x00, 0x44, 0x28, 0x10, 0x28, 0x44], // >78 x
    [0x00, 0x00, 0x44, 0x28, 0x10, 0x10, 0x10], // >79 y
    [0x00, 0x00, 0x7C, 0x08, 0x10, 0x20, 0x7C], // >7A z
    [0x18, 0x20, 0x20, 0x40, 0x20, 0x20, 0x18], // >7B {
    [0x10, 0x10, 0x10, 0x00, 0x10, 0x10, 0x10], // >7C |
    [0x30, 0x08, 0x08, 0x04, 0x08, 0x08, 0x30], // >7D }
    [0x00, 0x20, 0x54, 0x08, 0x00, 0x00, 0x00], // >7E ~
];

/// The seven stored rows for lower-case character `c`, or all-zero outside range.
fn lower_rows(c: u8) -> [u8; THIN_ROWS] {
    if (LOWER_FIRST..=LOWER_LAST).contains(&c) {
        LOWER_GLYPHS[(c - LOWER_FIRST) as usize]
    } else {
        [0u8; THIN_ROWS]
    }
}

/// The lower-case set packed as it is stored in the console GROM: seven rows
/// per glyph, in code order. Gated byte-identical to `>0874` by the test below.
pub fn packed_lower() -> Vec<u8> {
    let mut out = Vec::with_capacity(LOWER_COUNT * THIN_ROWS);
    for c in LOWER_FIRST..=LOWER_LAST {
        out.extend_from_slice(&lower_rows(c));
    }
    out
}

/// Render the lower-case set **as stored in the console GROM** — seven rows per
/// glyph — for splicing at the authentic home address `>0874` (B1), so a
/// cartridge that reads the set from its documented address gets the real bytes.
pub fn emit_gpl_bytes_lower_stored(label: &str) -> String {
    let mut s = String::new();
    for (i, c) in (LOWER_FIRST..=LOWER_LAST).enumerate() {
        let rows = lower_rows(c);
        let bytes: Vec<String> = rows.iter().map(|b| format!(">{b:02X}")).collect();
        let lbl = if i == 0 { label } else { "" };
        s.push_str(&format!("{lbl:<7} BYTE {}   ; {}\n", bytes.join(","), c as char));
    }
    s
}

/// Render the lower-case set as GPL `BYTE` lines, **expanded to eight rows** (a
/// blank top row prepended to the seven stored rows) so a plain 248-byte `MOVE`
/// of this block reproduces the authentic loader (`>004A` -> `>03C2`), which
/// clears row 0 and copies rows 1–7 for each of the 31 glyphs.
pub fn emit_gpl_bytes_lower(label: &str) -> String {
    let mut s = String::new();
    for (i, c) in (LOWER_FIRST..=LOWER_LAST).enumerate() {
        let rows = lower_rows(c);
        let bytes: Vec<String> = std::iter::once(0u8)
            .chain(rows)
            .map(|b| format!(">{b:02X}"))
            .collect();
        let lbl = if i == 0 { label } else { "" };
        s.push_str(&format!("{lbl:<7} BYTE {}   ; {}\n", bytes.join(","), c as char));
    }
    s
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    /// The authentic console GROM, loaded at run time from the git-ignored
    /// `third-party/` directory (`None` when the media are absent — the
    /// differential tests below then skip with a notice).
    static GROM: LazyLock<Option<Vec<u8>>> =
        LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

    /// The authentic GROM, or `None` (announced on stderr) when unavailable.
    fn authentic_grom() -> Option<&'static [u8]> {
        let grom = GROM.as_deref();
        if grom.is_none() {
            eprintln!("SKIPPED: third-party media not present (third-party/roms/994AGROM.Bin)");
        }
        grom
    }

    /// The console character set sits contiguously in the console GROM at
    /// `>04B4` (located by `examples/font_extract.rs`). Our packed font must be
    /// byte-identical to it — the "reproduce the character set exactly" gate.
    #[test]
    fn matches_authentic_character_set() {
        let Some(grom) = authentic_grom() else { return };
        const OFF: usize = 0x04B4;
        assert_eq!(
            packed().as_slice(),
            &grom[OFF..OFF + BYTES],
            "font must match the console character set byte-for-byte"
        );
    }

    /// The console's second (thin) character set sits contiguously at GROM
    /// `>06B4`, immediately after the main set — seven rows per glyph, 64 glyphs.
    /// Our packed thin set must be byte-identical to it.
    #[test]
    fn matches_authentic_thin_set() {
        let Some(grom) = authentic_grom() else { return };
        const OFF: usize = 0x06B4;
        let bytes = COUNT * THIN_ROWS; // 64 * 7 = 448
        assert_eq!(
            packed_thin().as_slice(),
            &grom[OFF..OFF + bytes],
            "thin set must match the console's second character set byte-for-byte"
        );
    }

    /// The console's lower-case (small capitals) set sits contiguously at GROM
    /// `>0874`, immediately after the thin set — seven rows per glyph, 31 glyphs
    /// (`>60..>7E`; the authentic block ends at `>094C`). Our packed lower set
    /// must be byte-identical to it.
    #[test]
    fn matches_authentic_lower_set() {
        let Some(grom) = authentic_grom() else { return };
        const OFF: usize = 0x0874;
        let bytes = LOWER_COUNT * THIN_ROWS; // 31 * 7 = 217
        assert_eq!(
            packed_lower().as_slice(),
            &grom[OFF..OFF + bytes],
            "lower-case set must match the console's small-capitals set byte-for-byte"
        );
    }

    /// The expanded lower-case GPL block is 248 bytes (8 rows/glyph, 31 glyphs)
    /// with each glyph's top row blank and rows 1–7 equal to the stored rows —
    /// exactly what the authentic `>004A` loader writes to VRAM.
    #[test]
    fn lower_expanded_block_matches_loader_output() {
        let src = emit_gpl_bytes_lower("FONT3");
        let vals: Vec<u8> = src
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|t| t.strip_prefix('>'))
            .filter_map(|h| u8::from_str_radix(h, 16).ok())
            .collect();
        assert_eq!(vals.len(), LOWER_COUNT * 8, "expanded lower block is 248 bytes");
        for (i, c) in (LOWER_FIRST..=LOWER_LAST).enumerate() {
            assert_eq!(vals[i * 8], 0, "glyph top row blank");
            assert_eq!(&vals[i * 8 + 1..i * 8 + 8], &lower_rows(c), "rows 1-7 = stored rows");
        }
    }

    /// The stored (seven-row) lower-case emitter re-parses to exactly
    /// `packed_lower()` — the bytes that go at the authentic home `>0874` (B1).
    #[test]
    fn lower_stored_block_matches_packed_lower() {
        let src = emit_gpl_bytes_lower_stored("LOWERA");
        let vals: Vec<u8> = src
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|t| t.strip_prefix('>'))
            .filter_map(|h| u8::from_str_radix(h, 16).ok())
            .collect();
        assert_eq!(vals, packed_lower(), "stored lower block must equal packed_lower()");
        assert_eq!(vals.len(), LOWER_COUNT * THIN_ROWS);
    }

    /// The expanded GPL block is 512 bytes (8 rows/glyph) with each glyph's top
    /// row blank and rows 1–7 equal to the seven stored rows — exactly what the
    /// authentic loader (`>039E`) writes to VRAM.
    #[test]
    fn thin_expanded_block_matches_loader_output() {
        let src = emit_gpl_bytes_thin("FONT2");
        // Re-parse the emitted bytes back to a flat Vec<u8>.
        let vals: Vec<u8> = src
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|t| t.strip_prefix('>'))
            .filter_map(|h| u8::from_str_radix(h, 16).ok())
            .collect();
        assert_eq!(vals.len(), COUNT * 8, "expanded thin block is 512 bytes");
        for (i, c) in (FIRST..=LAST).enumerate() {
            assert_eq!(vals[i * 8], 0, "glyph top row blank");
            assert_eq!(&vals[i * 8 + 1..i * 8 + 8], &thin_rows(c), "rows 1-7 = stored rows");
        }
    }

    /// The stored (seven-row) emitter re-parses to exactly `packed_thin()` — the
    /// bytes that go at the authentic home `>06B4` (B1). Distinct from the
    /// expanded loader block above.
    #[test]
    fn thin_stored_block_matches_packed_thin() {
        let src = emit_gpl_bytes_thin_stored("THIN");
        let vals: Vec<u8> = src
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter_map(|t| t.strip_prefix('>'))
            .filter_map(|h| u8::from_str_radix(h, 16).ok())
            .collect();
        assert_eq!(vals, packed_thin(), "stored thin block must equal packed_thin()");
        assert_eq!(vals.len(), COUNT * THIN_ROWS);
    }

    #[test]
    fn packed_size_and_identity_mapping() {
        assert_eq!(packed().len(), BYTES);
        assert_eq!(BYTES, 512);
        // 'A' (>41) is non-blank; space is blank; out-of-range codes are blank.
        assert!(glyph(b'A').iter().any(|&b| b != 0), "A should not be blank");
        assert_eq!(glyph(b' '), [0u8; 8]);
        assert_eq!(glyph(0x7F), [0u8; 8], "codes outside >20..>5F render blank");
    }
}
