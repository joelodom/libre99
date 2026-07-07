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

//! The **Libre99 chip logo** — an original title-screen emblem: an 8-bit-style
//! DIP chip (seven pins per long side) with a stylised **99** inside its body.
//! It takes the place of the authentic master title screen's "TI" logo, which is
//! TI's expression and cannot ship.
//!
//! The design is Joel's `libre99-chip` drawing (a Python rasteriser). Reproducing
//! it 1:1 would be ~14×12 characters — far too large to sit above the title
//! banner — so this is a faithful **compact** rendering at emblem size: a clean
//! re-rasterised chip body with seven hollow-bracket pins per side at a uniform
//! rhythm (the gap at each end of the pin row equals the gap between pins, as in
//! the drawing), and two **9**s at half the drawing's resolution (12×17) drawn in
//! **1-pixel strokes** throughout — the same line weight as the chip body — with
//! every structural element kept: the chamfered corners, square counter, keyline
//! stem, and bottom-left hook. It occupies a **5×4** block of 8×8 characters
//! (glyphs `>0B..>1E`).
//!
//! [`packed`] packs the emblem into the twenty 8×8 glyphs (row-major cell order);
//! [`emit_gpl_bytes`] renders them as a GPL `BYTE` block for splicing at GROM
//! `>1600`. `console.gpl` loads the block to the pattern table at char `>0B` and
//! places the cells (sequential codes `>0B..>1E`) on the title and menu screens.

/// The emblem's size in 8×8 characters (columns × rows).
pub const COLS: usize = 5;
pub const ROWS: usize = 4;
/// Pixel dimensions of the emblem canvas (COLS·8 × ROWS·8).
pub const WIDTH: usize = COLS * 8; // 40
pub const HEIGHT: usize = ROWS * 8; // 32
/// GROM address the emblem's glyphs are spliced at (free space below DSRLNK,
/// above the keyboard table at >1700). Twenty glyphs = 160 bytes (>1600..>169F).
pub const BASE: u16 = 0x1600;
/// The first character code the emblem uses in the pattern table. The twenty
/// glyphs occupy `>0B..>1E` (just above the copyright glyph `>0A`, below the font
/// at `>20`); `console.gpl` must load them here and reference them in that order.
pub const FIRST_CODE: u8 = 0x0B;
/// Number of glyphs (one per cell of the COLS×ROWS block).
pub const GLYPHS: usize = COLS * ROWS;

/// The **9**, 12×17, in 1-pixel strokes (`#` = ink). This is the drawing's 24×34
/// `nine()` at half resolution with every 2-unit stroke thinned to one pixel —
/// the same line weight as the chip body — keeping the full structure: the top
/// bar with stepped (chamfered) corners, the square counter, the bowl's
/// bottom-left curve, the bowl bottom bar, the keyline stem beside the detached
/// right wall, the bottom-left hook, and the closing bottom bar with mirrored
/// chamfers. Two hand adjustments (Joel, 2026-07-04): the hook's top bar gives
/// up the upper-right pixel of its 2×2 join with the inner bar, and the bowl's
/// bottom-left corner is the top-left curve **mirrored** (wall → `###` shoulder
/// → bar) so the upper loop reads consistently and clears the counter — that
/// sits the shoulder one half-step below the drawing's, whose step is a row
/// higher. The derivation is machine-checked: every ink pixel must lie within
/// the plain OR ÷2 of the drawing's glyph, except the mirrored shoulder's two
/// left pixels (`tests::thin_nine_is_a_subset_of_the_drawings_glyph`), so the
/// thin form can't silently drift from the drawing.
const NINE: [&str; 17] = [
    "..########..", // top bar
    "###......###", // chamfer steps / shoulders
    "#..........#", // walls (bowl face)
    "#..######..#", // counter top
    "#..#....#..#", // counter sides
    "#..#....#..#",
    "#..#....#..#",
    "#..######..#", // counter bottom
    "#..........#", // wall (bowl face)
    "###........#", // bowl bottom-left shoulder — the top-left curve, mirrored
    "..#######..#", // bowl bottom bar
    "........#..#", // keyline stem beside the right wall
    "####....#..#", // hook top bar (upper-right of the 2x2 join removed to stay 1px)
    "#..######..#", // hook inner bar
    "#..........#", // walls (hook face)
    "###......###", // bottom chamfer steps / shoulders
    "..########..", // bottom bar
];

/// The emblem as a `HEIGHT × WIDTH` boolean grid (`true` = ink/black). Built once
/// from clean primitives: a 1-pixel body outline, seven hollow-bracket pins per
/// long side at a uniform rhythm, and the two thin-stroke 9s centred in the body.
pub fn bitmap() -> Vec<Vec<bool>> {
    let mut g = vec![vec![false; WIDTH]; HEIGHT];
    let fill = |g: &mut Vec<Vec<bool>>, x0: i32, y0: i32, x1: i32, y1: i32| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                if x >= 0 && (x as usize) < WIDTH && y >= 0 && (y as usize) < HEIGHT {
                    g[y as usize][x as usize] = true;
                }
            }
        }
    };

    // Chip body outline (1-pixel), 39×23. The width is chosen so the pin row sits
    // at a **uniform rhythm** like the drawing's: the interior is 37 px =
    // 2 (edge gap) + 7 pins × 3 + 6 inter-pin gaps × 2 + 2 (edge gap), i.e. the
    // space between the wall and the first/last pin equals the space between
    // pins — an even interior can never centre the odd 33-px pin span. The height
    // leaves a 21-row interior so the 17-row digits centre with 2-px margins.
    let (bx0, bx1, by0, by1) = (0i32, 38, 4, 26);
    fill(&mut g, bx0, by0, bx1, by0); // top edge
    fill(&mut g, bx0, by1, bx1, by1); // bottom edge
    fill(&mut g, bx0, by0, bx0, by1); // left edge
    fill(&mut g, bx1, by0, bx1, by1); // right edge

    // Seven hollow-bracket pins per long side, uniform pitch. Each pin is 3px wide
    // (two rails + a hollow centre) and protrudes 3px, closed by a cap at the tip.
    let (pin_w, pitch, n_pins, prot) = (3i32, 5, 7, 3);
    let span = (n_pins - 1) * pitch + pin_w;
    let px0 = bx0 + 1 + (bx1 - bx0 - 1 - span) / 2;
    for i in 0..n_pins {
        let x = px0 + i * pitch;
        // top pin: rails up to the body, capped at the tip.
        fill(&mut g, x, by0 - prot, x, by0 - 1);
        fill(&mut g, x + pin_w - 1, by0 - prot, x + pin_w - 1, by0 - 1);
        fill(&mut g, x, by0 - prot, x + pin_w - 1, by0 - prot);
        // bottom pin.
        fill(&mut g, x, by1 + 1, x, by1 + prot);
        fill(&mut g, x + pin_w - 1, by1 + 1, x + pin_w - 1, by1 + prot);
        fill(&mut g, x, by1 + prot, x + pin_w - 1, by1 + prot);
    }

    // The two thin-stroke 9s ([`NINE`]), centred in the body; the drawing's
    // 6-unit digit gap is 3 px at this scale.
    let (nh, nw) = (NINE.len() as i32, NINE[0].len() as i32);
    let gap = 3i32;
    let total = 2 * nw + gap;
    let ox = bx0 + 1 + (bx1 - bx0 - 1 - total) / 2;
    let oy = by0 + 1 + (by1 - by0 - 1 - nh) / 2;
    for &base in &[ox, ox + nw + gap] {
        for (yy, row) in NINE.iter().enumerate() {
            for (xx, &ink) in row.as_bytes().iter().enumerate() {
                let (x, y) = (base + xx as i32, oy + yy as i32);
                if ink == b'#' && x >= 0 && (x as usize) < WIDTH && y >= 0 && (y as usize) < HEIGHT {
                    g[y as usize][x as usize] = true;
                }
            }
        }
    }
    g
}

/// Pack the emblem into the [`GLYPHS`] 8×8 glyphs, row-major over the COLS×ROWS
/// block: cell `(gr, gc)` → glyph index `COLS*gr + gc`, covering pixel rows
/// `8*gr..` and cols `8*gc..`. Returns `GLYPHS*8` bytes (glyph 0 first, one row
/// per byte, MSB = leftmost pixel).
pub fn packed() -> Vec<u8> {
    let g = bitmap();
    let mut out = Vec::with_capacity(GLYPHS * 8);
    for gr in 0..ROWS {
        for gc in 0..COLS {
            for r in 0..8 {
                let mut b = 0u8;
                for c in 0..8 {
                    if g[gr * 8 + r][gc * 8 + c] {
                        b |= 0x80u8 >> c;
                    }
                }
                out.push(b);
            }
        }
    }
    out
}

/// The emblem's pixel rows as string-art (`#`/space), for eyeballing.
pub fn art_lines() -> Vec<String> {
    bitmap()
        .iter()
        .map(|row| row.iter().map(|&p| if p { '#' } else { ' ' }).collect())
        .collect()
}

/// Render the emblem as a GPL `BYTE` block (a `GROM >1600` origin, one glyph per
/// line) for splicing into GPL source. `label` names the first glyph's address.
pub fn emit_gpl_bytes(label: &str) -> String {
    let bytes = packed();
    let mut s = format!("        GROM >{BASE:04X}\n");
    for g in 0..GLYPHS {
        let glyph = &bytes[g * 8..g * 8 + 8];
        let hex: Vec<String> = glyph.iter().map(|b| format!(">{b:02X}")).collect();
        let lbl = if g == 0 { label } else { "" };
        s.push_str(&format!(
            "{lbl:<7} BYTE {}   ; chip glyph >{:02X}\n",
            hex.join(","),
            FIRST_CODE as usize + g
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_the_expected_number_of_glyphs() {
        assert_eq!(packed().len(), GLYPHS * 8);
        assert_eq!(GLYPHS, 20);
    }

    #[test]
    fn every_cell_carries_ink() {
        // The chip fills all 5×4 cells (body/pins/digits reach each one), so the
        // sequential-code name table never lands a blank glyph mid-emblem.
        let bytes = packed();
        for g in 0..GLYPHS {
            assert!(
                bytes[g * 8..g * 8 + 8].iter().any(|&b| b != 0),
                "glyph {g} (code >{:02X}) is unexpectedly blank",
                FIRST_CODE as usize + g
            );
        }
    }

    #[test]
    fn codes_stay_below_the_font() {
        // The glyphs must fit between the copyright glyph (>0A) and the font (>20).
        assert!(FIRST_CODE > 0x0A);
        assert!(FIRST_CODE as usize + GLYPHS - 1 < 0x20);
    }

    #[test]
    fn digits_close_with_a_bottom_bar() {
        // Each 9 must close with a horizontal bottom bar (a ÷3 reduction once
        // dropped this stroke). Look in the digits' lower band (below the
        // counter, above the body's bottom edge) for a horizontal run of >= 6
        // ink pixels.
        let g = bitmap();
        let has_bar = (18..26).any(|y| (0..WIDTH - 5).any(|x| (0..6).all(|d| g[y][x + d])));
        assert!(has_bar, "the 9s should close with a horizontal bottom bar");
    }

    #[test]
    fn digits_leave_a_visible_counter() {
        // The two 9s must keep an interior hole (their square counter) — i.e. some
        // body-interior pixel is background, proving the digits didn't blob solid.
        let g = bitmap();
        assert!(
            (10..22).any(|y| (11..30).any(|x| !g[y][x])),
            "the 9s' counters should stay open"
        );
    }

    #[test]
    fn pins_are_centered_in_the_body() {
        // The pin rows must be mirror-symmetric about the body's centreline —
        // the gap between the left wall and the first pin equals the gap between
        // the last pin and the right wall (an off-centre pin comb was a reported
        // visual defect). The body spans columns 0..=38, so mirror x -> 38-x.
        let g = bitmap();
        for y in [1, 2, 3, 27, 28, 29] {
            for x in 0..=38usize {
                assert_eq!(
                    g[y][x],
                    g[y][38 - x],
                    "pin row {y} is not symmetric about the body centre at col {x}"
                );
            }
        }
    }

    /// The drawing's exact **9** glyph (24×34), a faithful port of the Python
    /// `nine()`: filled `bar(y0,y1,x0,x1)` primitives plus the corner trim. Kept
    /// as the oracle for [`thin_nine_is_a_subset_of_the_drawings_glyph`].
    fn nine_full() -> Vec<Vec<bool>> {
        let mut set: Vec<(i32, i32)> = Vec::new();
        let bar = |y0: i32, y1: i32, x0: i32, x1: i32, s: &mut Vec<(i32, i32)>| {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    s.push((x, y));
                }
            }
        };
        bar(0, 1, 4, 19, &mut set); // top bar
        bar(2, 2, 4, 5, &mut set);
        bar(2, 2, 18, 19, &mut set); // corner steps
        bar(3, 4, 0, 5, &mut set);
        bar(3, 4, 18, 23, &mut set);
        bar(5, 17, 0, 1, &mut set); // left wall (upper bowl)
        bar(28, 28, 0, 1, &mut set); // wall stub linking hook to chamfer
        bar(3, 30, 22, 23, &mut set); // right wall (trimmed below)
        bar(6, 7, 7, 16, &mut set); // counter top
        bar(8, 12, 7, 8, &mut set);
        bar(8, 12, 15, 16, &mut set); // counter sides
        bar(13, 14, 7, 16, &mut set); // counter bottom
        bar(16, 17, 0, 5, &mut set); // bowl bottom-left step
        bar(18, 18, 4, 5, &mut set);
        bar(19, 20, 4, 16, &mut set); // bowl bottom bar
        bar(21, 25, 15, 16, &mut set); // stem inner edge
        bar(24, 25, 0, 8, &mut set); // hook top bar
        bar(26, 27, 7, 16, &mut set); // hook inner bar
        bar(26, 27, 0, 1, &mut set); // left wall alongside hook
        bar(29, 30, 0, 5, &mut set);
        bar(29, 30, 18, 23, &mut set); // bottom corner steps
        bar(31, 31, 4, 5, &mut set);
        bar(31, 31, 18, 19, &mut set);
        bar(32, 33, 4, 19, &mut set); // bottom bar

        let mut g = vec![vec![false; 24]; 34];
        for (x, y) in set {
            // trim the right wall to rows 3..30 (matches the Python set subtraction).
            if (x == 22 || x == 23) && !(3..=30).contains(&y) {
                continue;
            }
            g[y as usize][x as usize] = true;
        }
        g
    }

    #[test]
    fn thin_nine_is_a_subset_of_the_drawings_glyph() {
        // NINE is the drawing's 9 at ÷2 with strokes thinned to one pixel: every
        // ink pixel must lie within the plain OR ÷2 of the drawing's 24×34 glyph,
        // so the thin form can't silently drift from the drawing. Two deliberate
        // exceptions: the bowl's bottom-left shoulder pixels (9,0) and (9,1) sit
        // one half-step below the envelope, because that shoulder mirrors the
        // top-left curve while the drawing's own step is a row higher (see the
        // NINE doc comment).
        let full = nine_full();
        for (y, row) in NINE.iter().enumerate() {
            for (x, &b) in row.as_bytes().iter().enumerate() {
                if b != b'#' || (y, x) == (9, 0) || (y, x) == (9, 1) {
                    continue;
                }
                let covered = (0..2).any(|dy| {
                    (0..2).any(|dx| {
                        let (sy, sx) = (y * 2 + dy, x * 2 + dx);
                        sy < 34 && sx < 24 && full[sy][sx]
                    })
                });
                assert!(covered, "NINE ink at ({y},{x}) is outside the drawing's glyph");
            }
        }
    }
}
