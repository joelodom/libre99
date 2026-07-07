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

//! Embedded scalable fonts for the native-resolution help overlay.
//!
//! The chunky 5×7 bitmap font in [`crate::text`] is right for overlays painted
//! into the 256×192 framebuffer (toast, CPU inspector, media browser), but the
//! redesigned `F1` help screen is drawn at the window's *native* resolution and
//! wants smooth type at 8–30 px. This module embeds the two open-licensed faces
//! the design calls for — **Silkscreen** (the pixel display face) and **IBM Plex
//! Mono** (body/UI) — and rasterizes their glyphs on demand with `ab_glyph`.
//!
//! Glyphs are cached by `(face, size, char)`: the help screen is static and only
//! re-rendered when the tab or window size changes, so the cache fills once and
//! every later draw is a memcpy of coverage bytes.
//!
//! The `.ttf`s and their SIL OFL licenses live in `assets/fonts/`.

use std::collections::HashMap;

use ab_glyph::{Font, FontRef, ScaleFont};

/// One embedded face + weight. The discriminant doubles as an index into
/// [`Fonts::faces`], so keep the two in the same order.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FontId {
    /// Silkscreen 400 — the wordmark and the word-keycaps.
    SilkRegular = 0,
    /// IBM Plex Mono 400 — body copy, labels, footnotes.
    MonoRegular = 1,
    /// IBM Plex Mono 500 — the headline styles.
    MonoMedium = 2,
    /// IBM Plex Mono 600 — key chips, keycap legends, table keys, emphasis.
    MonoSemiBold = 3,
    /// IBM Plex Mono 700 — section eyebrows.
    MonoBold = 4,
}

const SILK_REGULAR: &[u8] = include_bytes!("../assets/fonts/Silkscreen-Regular.ttf");
const MONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf");
const MONO_MEDIUM: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Medium.ttf");
const MONO_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-SemiBold.ttf");
const MONO_BOLD: &[u8] = include_bytes!("../assets/fonts/IBMPlexMono-Bold.ttf");

/// A rasterized glyph: an 8-bit coverage mask plus the metrics needed to place
/// it relative to the pen origin (left edge, on the baseline).
#[derive(Clone)]
pub struct Raster {
    pub w: usize,
    pub h: usize,
    /// Pixel offset of the mask's top-left from the pen origin (`top` is
    /// relative to the baseline, so it is negative for the part above it).
    pub left: i32,
    pub top: i32,
    /// How far to advance the pen after this glyph, in device px.
    pub advance: f32,
    /// `w * h` coverage values, `0..=255`.
    pub cov: Vec<u8>,
}

/// Quantize a pixel size to quarter-px buckets so continuous window resizes
/// can't grow the cache without bound; quarter-px steps are visually lossless.
fn quant(px: f32) -> f32 {
    (px * 4.0).round() / 4.0
}

/// The five embedded faces plus a shared glyph-raster cache.
///
/// Every `px` argument below is an **em size** — the same unit as a CSS
/// `font-size`, so the design handoff's pixel values can be used verbatim.
/// `ab_glyph`'s `PxScale` measures the ascent-to-descent *height* instead, so
/// each face carries a precomputed em→height factor and the conversion happens
/// at this boundary.
pub struct Fonts {
    faces: [FontRef<'static>; 5],
    /// Per-face `height/em` ratio (≈1.3 for IBM Plex Mono).
    em_scale: [f32; 5],
    cache: HashMap<(FontId, u32, char), Raster>,
}

impl Fonts {
    /// Parse the embedded faces. Panics only if an embedded `.ttf` is corrupt,
    /// which would be a build-time mistake (the bytes ship in the binary).
    pub fn new() -> Self {
        let faces = [
            FontRef::try_from_slice(SILK_REGULAR).expect("Silkscreen-Regular.ttf"),
            FontRef::try_from_slice(MONO_REGULAR).expect("IBMPlexMono-Regular.ttf"),
            FontRef::try_from_slice(MONO_MEDIUM).expect("IBMPlexMono-Medium.ttf"),
            FontRef::try_from_slice(MONO_SEMIBOLD).expect("IBMPlexMono-SemiBold.ttf"),
            FontRef::try_from_slice(MONO_BOLD).expect("IBMPlexMono-Bold.ttf"),
        ];
        let em_scale = faces
            .each_ref()
            .map(|f| f.height_unscaled() / f.units_per_em().unwrap_or(f.height_unscaled()));
        Fonts {
            faces,
            em_scale,
            cache: HashMap::new(),
        }
    }

    fn face(&self, id: FontId) -> &FontRef<'static> {
        &self.faces[id as usize]
    }

    /// The em size `px` as an `ab_glyph` height scale, quantized for caching.
    fn scale(&self, id: FontId, px: f32) -> f32 {
        quant(px * self.em_scale[id as usize])
    }

    /// Distance from the baseline up to the top of the face's ascenders, at
    /// em size `px`. Used to convert a top-left text position into a baseline.
    pub fn ascent(&self, id: FontId, px: f32) -> f32 {
        let s = self.scale(id, px);
        self.face(id).as_scaled(s).ascent()
    }

    /// Total advance width of `s` at em size `px`, in device px (no tracking).
    pub fn measure(&self, id: FontId, px: f32, s: &str) -> f32 {
        let sf = self.face(id).as_scaled(self.scale(id, px));
        s.chars().map(|c| sf.h_advance(sf.glyph_id(c))).sum()
    }

    /// Rasterize (or fetch from cache) the glyph for `c` at em size `px`.
    pub fn raster(&mut self, id: FontId, px: f32, c: char) -> &Raster {
        let px = self.scale(id, px);
        let key = (id, px.to_bits(), c);
        // Two-step (contains/insert) keeps the borrow checker happy without an
        // entry() closure capturing `self.faces`.
        if !self.cache.contains_key(&key) {
            let raster = self.rasterize(id, px, c);
            self.cache.insert(key, raster);
        }
        &self.cache[&key]
    }

    fn rasterize(&self, id: FontId, px: f32, c: char) -> Raster {
        let face = self.face(id);
        let sf = face.as_scaled(px);
        let gid = sf.glyph_id(c);
        let advance = sf.h_advance(gid);
        match face.outline_glyph(gid.with_scale(px)) {
            Some(outline) => {
                let b = outline.px_bounds();
                let w = b.width().ceil().max(0.0) as usize;
                let h = b.height().ceil().max(0.0) as usize;
                let mut cov = vec![0u8; w * h];
                outline.draw(|x, y, c| {
                    let (x, y) = (x as usize, y as usize);
                    if x < w && y < h {
                        cov[y * w + x] = (c * 255.0 + 0.5).min(255.0) as u8;
                    }
                });
                Raster {
                    w,
                    h,
                    left: b.min.x.round() as i32,
                    top: b.min.y.round() as i32,
                    advance,
                    cov,
                }
            }
            // Whitespace and any glyph with no outline still advance the pen.
            None => Raster {
                w: 0,
                h: 0,
                left: 0,
                top: 0,
                advance,
                cov: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_parse_and_rasterize() {
        let mut fonts = Fonts::new();
        // A visible glyph produces a non-empty, partially-covered mask.
        let a = fonts.raster(FontId::MonoBold, 32.0, 'A').clone();
        assert!(a.w > 0 && a.h > 0, "no bitmap for 'A'");
        assert!(a.cov.iter().any(|&v| v > 0), "blank 'A'");
        assert!(a.advance > 0.0);
        // Space has an advance but no bitmap.
        let sp = fonts.raster(FontId::MonoRegular, 32.0, ' ');
        assert!(sp.advance > 0.0 && sp.w == 0);
    }

    #[test]
    fn measure_is_sum_of_advances() {
        let fonts = Fonts::new();
        let w1 = fonts.measure(FontId::MonoRegular, 20.0, "M");
        let w3 = fonts.measure(FontId::MonoRegular, 20.0, "MMM");
        // Plex Mono is monospaced, so three Ms are exactly three advances.
        assert!((w3 - w1 * 3.0).abs() < 0.5, "w1={w1} w3={w3}");
    }
}
