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

//! Present the core's 256×192 RGBA framebuffer into a window-sized pixel buffer.
//!
//! The VDP renders into `0x00RRGGBB` words, which is exactly softbuffer's pixel
//! format, so presentation is an integer nearest-neighbor upscale (the largest
//! whole multiple that fits), centered on a black background.

use libre99_core::vdp::{HEIGHT, WIDTH};

/// Blit `fb` (`WIDTH*HEIGHT`) into `dst` (`win_w*win_h`), scaled and centered.
pub fn blit(fb: &[u32], dst: &mut [u32], win_w: usize, win_h: usize) {
    for p in dst.iter_mut() {
        *p = 0x0000_0000;
    }
    if win_w == 0 || win_h == 0 {
        return;
    }
    let scale = (win_w / WIDTH).min(win_h / HEIGHT).max(1);
    let draw_w = (WIDTH * scale).min(win_w);
    let draw_h = (HEIGHT * scale).min(win_h);
    let ox = (win_w - draw_w) / 2;
    let oy = (win_h - draw_h) / 2;

    for dy in 0..draw_h {
        let sy = (dy / scale).min(HEIGHT - 1);
        let dst_row = (oy + dy) * win_w + ox;
        let src_row = sy * WIDTH;
        for dx in 0..draw_w {
            let sx = (dx / scale).min(WIDTH - 1);
            dst[dst_row + dx] = fb[src_row + sx];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source framebuffer whose every pixel encodes its own `(x, y)` so scaling
    /// and centering are verifiable by reading back where each source pixel went.
    fn coded_fb() -> Vec<u32> {
        (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| ((y as u32) << 16) | x as u32))
            .collect()
    }

    #[test]
    fn exact_fit_is_a_verbatim_copy() {
        let fb = coded_fb();
        let mut dst = vec![0xDEAD_BEEF; WIDTH * HEIGHT];
        blit(&fb, &mut dst, WIDTH, HEIGHT);
        // Scale 1, no offset: the output equals the source exactly.
        assert_eq!(dst, fb);
    }

    #[test]
    fn integer_scale_doubles_each_pixel_into_a_block() {
        let fb = coded_fb();
        let (w, h) = (WIDTH * 2, HEIGHT * 2);
        let mut dst = vec![0u32; w * h];
        blit(&fb, &mut dst, w, h);
        // Source (0,0) fills the top-left 2x2 block; (1,0) the next.
        let s00 = fb[0];
        let s10 = fb[1];
        assert_eq!(dst[0], s00);
        assert_eq!(dst[1], s00);
        assert_eq!(dst[w], s00);
        assert_eq!(dst[w + 1], s00);
        assert_eq!(dst[2], s10);
        assert_eq!(dst[3], s10);
    }

    #[test]
    fn scale_is_the_limiting_dimension_and_image_is_centered() {
        let fb = coded_fb();
        // Wide but short: height caps the scale at 1 even though width allows 2.
        let (w, h) = (WIDTH * 2, HEIGHT + 100);
        let mut dst = vec![0x00FF_00FFu32; w * h]; // non-black, to prove clearing
        blit(&fb, &mut dst, w, h);
        let scale = 1;
        let ox = (w - WIDTH * scale) / 2; // (512-256)/2 = 128
        let oy = (h - HEIGHT * scale) / 2; // (292-192)/2 = 50
        assert_eq!(ox, 128);
        assert_eq!(oy, 50);
        // Letterbox around the image is cleared to black.
        assert_eq!(dst[0], 0x0000_0000, "top-left corner is background");
        assert_eq!(dst[(oy - 1) * w + ox], 0, "row just above the image is black");
        // The image's top-left source pixel lands at the centered origin.
        assert_eq!(dst[oy * w + ox], fb[0]);
        // Its bottom-right source pixel lands at the far corner of the drawn area.
        assert_eq!(dst[(oy + HEIGHT - 1) * w + ox + WIDTH - 1], fb[(HEIGHT - 1) * WIDTH + WIDTH - 1]);
    }

    #[test]
    fn window_smaller_than_native_clamps_without_panic() {
        let fb = coded_fb();
        let (w, h) = (100, 80);
        let mut dst = vec![0u32; w * h];
        blit(&fb, &mut dst, w, h); // scale floors to 1; draw region clamps to the window
        // Top-left maps 1:1; nothing writes out of bounds.
        assert_eq!(dst[0], fb[0]);
        assert_eq!(dst[w + 1], fb[WIDTH + 1]);
        assert_eq!(dst[(h - 1) * w + (w - 1)], fb[(h - 1) * WIDTH + (w - 1)]);
    }

    #[test]
    fn zero_sized_window_is_a_no_op() {
        let fb = coded_fb();
        let mut dst: Vec<u32> = Vec::new();
        blit(&fb, &mut dst, 0, 10); // must not panic or index an empty buffer
        assert!(dst.is_empty());
    }
}
