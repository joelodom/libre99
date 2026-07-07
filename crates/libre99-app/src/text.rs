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

//! A tiny 5×7 bitmap font and helpers for drawing text overlays directly into
//! the core's `0x00RRGGBB` framebuffer.
//!
//! The app has no GUI toolkit — it presents a raw framebuffer — so on-screen UI
//! (the save/load toast, the keyboard reference card) is painted pixel by pixel
//! here. Text is drawn into the 256×192 image *before* it is upscaled to the
//! window, so it scales crisply with everything else and stays in the TI's
//! chunky aesthetic.
//!
//! Each glyph is 7 rows of 5 bits (bit 4 = leftmost pixel). Lowercase letters
//! reuse the uppercase glyphs (the TI's default font is uppercase anyway).

/// Glyph cell width in pixels (5 drawn + 1 spacing = 6-pixel advance).
pub const GLYPH_W: usize = 5;
/// Glyph cell height in pixels.
pub const GLYPH_H: usize = 7;
/// Horizontal advance between glyph origins (1 px inter-character gap).
pub const ADVANCE: usize = GLYPH_W + 1;
/// Vertical advance between text rows (1 px inter-line gap).
pub const LINE: usize = GLYPH_H + 1;

const BLANK: [u8; 7] = [0; 7];

/// Printable ASCII `0x20..=0x7F`, in order. Lowercase slots are blank because
/// [`glyph`] uppercases before indexing.
#[rustfmt::skip]
const FONT: [[u8; 7]; 96] = [
    [0b00000,0b00000,0b00000,0b00000,0b00000,0b00000,0b00000], // (space)
    [0b00100,0b00100,0b00100,0b00100,0b00100,0b00000,0b00100], // !
    [0b01010,0b01010,0b01010,0b00000,0b00000,0b00000,0b00000], // "
    [0b01010,0b01010,0b11111,0b01010,0b11111,0b01010,0b01010], // #
    [0b00100,0b01111,0b10100,0b01110,0b00101,0b11110,0b00100], // $
    [0b11000,0b11001,0b00010,0b00100,0b01000,0b10011,0b00011], // %
    [0b01100,0b10010,0b10100,0b01000,0b10101,0b10010,0b01101], // &
    [0b00100,0b00100,0b00100,0b00000,0b00000,0b00000,0b00000], // '
    [0b00010,0b00100,0b01000,0b01000,0b01000,0b00100,0b00010], // (
    [0b01000,0b00100,0b00010,0b00010,0b00010,0b00100,0b01000], // )
    [0b00000,0b00100,0b10101,0b01110,0b10101,0b00100,0b00000], // *
    [0b00000,0b00100,0b00100,0b11111,0b00100,0b00100,0b00000], // +
    [0b00000,0b00000,0b00000,0b00000,0b00100,0b00100,0b01000], // ,
    [0b00000,0b00000,0b00000,0b11111,0b00000,0b00000,0b00000], // -
    [0b00000,0b00000,0b00000,0b00000,0b00000,0b00110,0b00110], // .
    [0b00001,0b00010,0b00100,0b00100,0b00100,0b01000,0b10000], // /
    [0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110], // 0
    [0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110], // 1
    [0b01110,0b10001,0b00001,0b00010,0b00100,0b01000,0b11111], // 2
    [0b11111,0b00010,0b00100,0b00010,0b00001,0b10001,0b01110], // 3
    [0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010], // 4
    [0b11111,0b10000,0b11110,0b00001,0b00001,0b10001,0b01110], // 5
    [0b00110,0b01000,0b10000,0b11110,0b10001,0b10001,0b01110], // 6
    [0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000], // 7
    [0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110], // 8
    [0b01110,0b10001,0b10001,0b01111,0b00001,0b00010,0b01100], // 9
    [0b00000,0b00110,0b00110,0b00000,0b00110,0b00110,0b00000], // :
    [0b00000,0b00110,0b00110,0b00000,0b00110,0b00100,0b01000], // ;
    [0b00010,0b00100,0b01000,0b10000,0b01000,0b00100,0b00010], // <
    [0b00000,0b00000,0b11111,0b00000,0b11111,0b00000,0b00000], // =
    [0b01000,0b00100,0b00010,0b00001,0b00010,0b00100,0b01000], // >
    [0b01110,0b10001,0b00001,0b00010,0b00100,0b00000,0b00100], // ?
    [0b01110,0b10001,0b00001,0b01101,0b10101,0b10101,0b01110], // @
    [0b01110,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001], // A
    [0b11110,0b10001,0b10001,0b11110,0b10001,0b10001,0b11110], // B
    [0b01110,0b10001,0b10000,0b10000,0b10000,0b10001,0b01110], // C
    [0b11100,0b10010,0b10001,0b10001,0b10001,0b10010,0b11100], // D
    [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b11111], // E
    [0b11111,0b10000,0b10000,0b11110,0b10000,0b10000,0b10000], // F
    [0b01110,0b10001,0b10000,0b10111,0b10001,0b10001,0b01111], // G
    [0b10001,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001], // H
    [0b01110,0b00100,0b00100,0b00100,0b00100,0b00100,0b01110], // I
    [0b00111,0b00010,0b00010,0b00010,0b00010,0b10010,0b01100], // J
    [0b10001,0b10010,0b10100,0b11000,0b10100,0b10010,0b10001], // K
    [0b10000,0b10000,0b10000,0b10000,0b10000,0b10000,0b11111], // L
    [0b10001,0b11011,0b10101,0b10101,0b10001,0b10001,0b10001], // M
    [0b10001,0b11001,0b10101,0b10011,0b10001,0b10001,0b10001], // N
    [0b01110,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110], // O
    [0b11110,0b10001,0b10001,0b11110,0b10000,0b10000,0b10000], // P
    [0b01110,0b10001,0b10001,0b10001,0b10101,0b10010,0b01101], // Q
    [0b11110,0b10001,0b10001,0b11110,0b10100,0b10010,0b10001], // R
    [0b01111,0b10000,0b10000,0b01110,0b00001,0b00001,0b11110], // S
    [0b11111,0b00100,0b00100,0b00100,0b00100,0b00100,0b00100], // T
    [0b10001,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110], // U
    [0b10001,0b10001,0b10001,0b10001,0b10001,0b01010,0b00100], // V
    [0b10001,0b10001,0b10001,0b10101,0b10101,0b10101,0b01010], // W
    [0b10001,0b10001,0b01010,0b00100,0b01010,0b10001,0b10001], // X
    [0b10001,0b10001,0b01010,0b00100,0b00100,0b00100,0b00100], // Y
    [0b11111,0b00001,0b00010,0b00100,0b01000,0b10000,0b11111], // Z
    [0b01110,0b01000,0b01000,0b01000,0b01000,0b01000,0b01110], // [
    [0b10000,0b01000,0b00100,0b00100,0b00100,0b00010,0b00001], // \
    [0b01110,0b00010,0b00010,0b00010,0b00010,0b00010,0b01110], // ]
    [0b00100,0b01010,0b10001,0b00000,0b00000,0b00000,0b00000], // ^
    [0b00000,0b00000,0b00000,0b00000,0b00000,0b00000,0b11111], // _
    [0b01000,0b00100,0b00010,0b00000,0b00000,0b00000,0b00000], // `
    BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, // a-j
    BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, BLANK, // k-t
    BLANK, BLANK, BLANK, BLANK, BLANK, BLANK,                             // u-z
    [0b00010,0b00100,0b00100,0b01000,0b00100,0b00100,0b00010], // {
    [0b00100,0b00100,0b00100,0b00100,0b00100,0b00100,0b00100], // |
    [0b01000,0b00100,0b00100,0b00010,0b00100,0b00100,0b01000], // }
    [0b00000,0b00000,0b01000,0b10101,0b00010,0b00000,0b00000], // ~
    BLANK,                                                     // 0x7F
];

/// The 7-row bitmap for `c` (lowercase folded to uppercase; unknown ⇒ blank).
pub fn glyph(c: char) -> [u8; 7] {
    let c = if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    };
    let idx = (c as usize).wrapping_sub(0x20);
    FONT.get(idx).copied().unwrap_or(BLANK)
}

/// Pixel width a string occupies at `scale` (no trailing inter-char gap).
pub fn text_width(s: &str, scale: usize) -> usize {
    let n = s.chars().count();
    if n == 0 {
        0
    } else {
        (n * ADVANCE - 1) * scale
    }
}

/// A drawable view over a framebuffer: the pixel slice plus its dimensions.
/// Bundling them keeps the drawing methods to a sane argument count and lets
/// callers write `canvas.draw_text(...)` instead of threading `(fb, w, h)`
/// through every call.
pub struct Canvas<'a> {
    pixels: &'a mut [u32],
    width: usize,
    height: usize,
}

impl<'a> Canvas<'a> {
    /// Wrap a `width × height` framebuffer (`0x00RRGGBB` words).
    pub fn new(pixels: &'a mut [u32], width: usize, height: usize) -> Self {
        Canvas {
            pixels,
            width,
            height,
        }
    }

    /// Set one pixel, ignoring out-of-bounds coordinates.
    fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    /// Draw a single glyph at `(x, y)`, magnified by `scale`.
    pub fn draw_char(&mut self, x: usize, y: usize, c: char, color: u32, scale: usize) {
        self.draw_bits(x, y, &glyph(c), color, scale);
    }

    /// Draw an arbitrary 7-row, 5-bit-wide bitmap at `(x, y)`, magnified by
    /// `scale` (bit 4 = leftmost pixel, same packing as [`FONT`]). Used for the
    /// non-ASCII icons — cursor arrows, the return symbol, the shift glyph — on
    /// the keyboard reference card.
    pub fn draw_bits(&mut self, x: usize, y: usize, bits: &[u8; 7], color: u32, scale: usize) {
        for (row, b) in bits.iter().enumerate() {
            for col in 0..GLYPH_W {
                if b & (1 << (GLYPH_W - 1 - col)) != 0 {
                    for sy in 0..scale {
                        for sx in 0..scale {
                            self.put_pixel(x + col * scale + sx, y + row * scale + sy, color);
                        }
                    }
                }
            }
        }
    }

    /// Draw a left-aligned string at `(x, y)`, magnified by `scale`.
    pub fn draw_text(&mut self, x: usize, y: usize, s: &str, color: u32, scale: usize) {
        let mut cx = x;
        for c in s.chars() {
            self.draw_char(cx, y, c, color, scale);
            cx += ADVANCE * scale;
        }
    }

    /// Darken a rectangle toward black (a cheap translucent panel) so overlaid
    /// text stays legible over a busy screen. `shade` 0 = unchanged … 3 = nearly
    /// black.
    pub fn dim_rect(&mut self, x: usize, y: usize, w: usize, h: usize, shade: u32) {
        for yy in y..(y + h).min(self.height) {
            for xx in x..(x + w).min(self.width) {
                let p = self.pixels[yy * self.width + xx];
                let r = ((p >> 16) & 0xFF) >> shade;
                let g = ((p >> 8) & 0xFF) >> shade;
                let b = (p & 0xFF) >> shade;
                self.pixels[yy * self.width + xx] = (r << 16) | (g << 8) | b;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_glyphs_have_the_expected_shape() {
        assert_eq!(glyph(' '), [0; 7]);
        assert_ne!(glyph('A'), [0; 7]);
        // Lowercase folds to uppercase.
        assert_eq!(glyph('a'), glyph('A'));
        // 'T' is a top bar plus a centered stem.
        assert_eq!(glyph('T')[0], 0b11111);
        assert_eq!(glyph('T')[3], 0b00100);
        // Unknown characters are blank, never a panic.
        assert_eq!(glyph('€'), [0; 7]);
    }

    #[test]
    fn draw_text_sets_pixels_within_bounds_only() {
        let (w, h) = (40, 12);
        let mut fb = vec![0u32; w * h];
        Canvas::new(&mut fb, w, h).draw_text(1, 1, "HI!", 0x00FF_FFFF, 1);
        assert!(fb.iter().any(|&p| p == 0x00FF_FFFF), "nothing was drawn");
        // Drawing far off-screen must not panic or write anywhere.
        let mut fb2 = vec![0u32; w * h];
        Canvas::new(&mut fb2, w, h).draw_text(1000, 1000, "OFF", 0x00FF_FFFF, 2);
        assert!(fb2.iter().all(|&p| p == 0), "wrote out of bounds");
    }

    #[test]
    fn text_width_matches_advance() {
        assert_eq!(text_width("", 1), 0);
        assert_eq!(text_width("A", 1), GLYPH_W);
        assert_eq!(text_width("AB", 1), ADVANCE + GLYPH_W);
        assert_eq!(text_width("AB", 2), (ADVANCE + GLYPH_W) * 2);
    }
}
