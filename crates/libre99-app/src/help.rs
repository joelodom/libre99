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

//! The `Esc` / `F1` help overlay — a four-tab reference (Start, Keyboard,
//! Hotkeys, Settings) drawn at the window's **native** pixel resolution with
//! the embedded smooth fonts ([`crate::font`]).
//!
//! Unlike the other overlays (toast, CPU inspector, media browser), which paint
//! into the 256×192 framebuffer and are then nearest-neighbor upscaled, this one
//! renders straight into the window-sized surface so 10–30 px type stays crisp.
//! It fills the largest centered **4:3** rectangle that fits the window — the
//! same region the emulated image occupies — and letterboxes the rest.
//!
//! All layout is written in the design's 1024×768 coordinate space and scaled to
//! the live region, so it looks identical at every `window_scale` and fullscreen.
//! The layout, colors, and copy recreate the "quiet terminal" design of
//! `design_handoff_help_redesign`: solid black backdrop, hairline rules and
//! whitespace instead of cards, a single cyan chrome accent (amber and green
//! appear only as the keyboard map's SHIFT/FCTN semantics), Silkscreen only in
//! the wordmark and word-keycaps. All key/flag/pref values track
//! `docs/USER-GUIDE.md` (the source of truth — keep the two in sync when
//! controls or preferences change).
//!
//! This is a software renderer, so the drawing primitives take the usual
//! `x, y, w, h, …, color` argument lists; `too_many_arguments` is expected here.
#![allow(clippy::too_many_arguments)]

use crate::font::{FontId, Fonts};

/// Version string for the footer of the Start tab.
const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Compile-time platform switch. Both branches of every `if IS_MAC` are
/// compiled on every platform, so the macOS drawing paths can't rot silently
/// on a Windows/Linux checkout (and vice versa).
const IS_MAC: bool = cfg!(target_os = "macos");

/// An emulator-shortcut key chip: plain text, or the macOS chips whose ⌃ / ⌘
/// glyphs the embedded fonts lack and which are therefore drawn from strokes.
/// The per-platform constants below are kept in sync with `app.rs`'s hotkey
/// match so the labels never lie about the real bindings.
#[derive(Clone, Copy)]
enum KeyChip {
    Text(&'static str),
    /// `⌃ ⌘ <letter>` (macOS fullscreen).
    MacCtrlCmd(&'static str),
    /// `⌘ <letter>` (macOS quit / screenshot / inspector).
    MacCmd(&'static str),
}

const FULLSCREEN_CHIP: KeyChip = if IS_MAC {
    KeyChip::MacCtrlCmd("F")
} else {
    KeyChip::Text("F11")
};
const QUIT_CHIP: KeyChip = if IS_MAC {
    KeyChip::MacCmd("Q")
} else {
    KeyChip::Text("ALT F4")
};
const SCREENSHOT_CHIP: KeyChip = if IS_MAC {
    KeyChip::MacCmd("S")
} else {
    KeyChip::Text("CTRL S")
};
const INSPECTOR_CHIP: KeyChip = if IS_MAC {
    KeyChip::MacCmd("D")
} else {
    KeyChip::Text("CTRL D")
};

/// A styled text run for [`Screen::paragraph`]: the text, its face, its color.
type Run<'a> = (&'a str, FontId, u32);

// Short aliases for the embedded faces used here (see [`FontId`]).
const SR: FontId = FontId::SilkRegular; // Silkscreen 400 — wordmark, word-keycaps
const MR: FontId = FontId::MonoRegular; // IBM Plex Mono 400 — body
const MM: FontId = FontId::MonoMedium; // 500 — the two headline styles
const MS: FontId = FontId::MonoSemiBold; // 600 — chips, labels, emphasis
const MB: FontId = FontId::MonoBold; // 700 — eyebrows

// ---- design canvas ---------------------------------------------------------
const FRAME_W: f32 = 1024.0;
const FRAME_H: f32 = 768.0;
const PAD_X: f32 = 48.0;
const CONTENT_X: f32 = PAD_X;
const CONTENT_W: f32 = FRAME_W - 2.0 * PAD_X; // 928
const RIGHT_X: f32 = FRAME_W - PAD_X; // 976
const TOP_RULE_Y: f32 = 58.0; // bottom of the top bar
const FOOT_RULE_Y: f32 = FRAME_H - 44.0; // top of the footer (724)
const FOOT_CY: f32 = (FOOT_RULE_Y + FRAME_H) / 2.0; // footer text center (746)

// ---- color tokens (0xRRGGBB, per the design-handoff token table) -----------
const INK: u32 = 0xe9eefb; // headlines, key main legends
const CHIP_INK: u32 = 0xdbe4fa; // chip text, emphasized inline values
const BRIGHT: u32 = 0xaab6dd; // emphasized footer words, word-keycaps
const MUTED: u32 = 0x8a99c8; // body copy, labels
const FAINT: u32 = 0x5a6aa0; // footnotes, "HELP"
const TAB_NUM: u32 = 0x3f4f8c; // the small digits in the tab bar
const CYAN: u32 = 0x5cc8e8; // the one chrome accent
const AMBER: u32 = 0xf1c46b; // SHIFT legends + legend swatch only
const GREEN: u32 = 0x74d68a; // FCTN legends + legend swatch only
const RULE_STRONG: u32 = 0x26336a; // rules under eyebrows, table header rules
const RULE: u32 = 0x212d5c; // top-bar/footer rules, big section rules
const RULE_ROW: u32 = 0x1a2550; // table/list row separators
const CHIP_BG: u32 = 0x151f47; // key chip fill
const CHIP_BORDER: u32 = 0x2c3a74; // key chip border; START column top rules
const CAP_TOP: u32 = 0x1b2652; // keycap vertical gradient, top
const CAP_BOT: u32 = 0x141d42; // keycap vertical gradient, bottom
const CAP_BORDER: u32 = 0x2b3870; // keycap 1px border

/// Which help tab is showing. The order matches the on-screen 1–4 numbering.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HelpTab {
    Start,
    Keyboard,
    Hotkeys,
    Settings,
}

impl HelpTab {
    pub const ALL: [HelpTab; 4] = [
        HelpTab::Start,
        HelpTab::Keyboard,
        HelpTab::Hotkeys,
        HelpTab::Settings,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap_or(0)
    }

    /// Next/previous tab, wrapping (`delta` is +1 or -1).
    pub fn cycle(self, delta: i32) -> HelpTab {
        let n = Self::ALL.len() as i32;
        let i = (self.index() as i32 + delta).rem_euclid(n) as usize;
        Self::ALL[i]
    }

    fn label(self) -> &'static str {
        match self {
            HelpTab::Start => "START",
            HelpTab::Keyboard => "KEYBOARD",
            HelpTab::Hotkeys => "HOTKEYS",
            HelpTab::Settings => "SETTINGS",
        }
    }
}

/// Linearly interpolate two `0xRRGGBB` colors (`t` clamped to `0..=1`).
fn lerp_rgb(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |sh: u32| {
        let ca = ((a >> sh) & 0xFF) as f32;
        let cb = ((b >> sh) & 0xFF) as f32;
        (ca + (cb - ca) * t + 0.5) as u32
    };
    (mix(16) << 16) | (mix(8) << 8) | mix(0)
}

/// A native-resolution drawing surface: the window buffer plus the design→device
/// transform (offset + uniform scale) for the centered 4:3 region.
struct Screen<'a> {
    buf: &'a mut [u32],
    win_w: usize,
    win_h: usize,
    ox: f32,
    oy: f32,
    scale: f32,
    fonts: &'a mut Fonts,
}

impl<'a> Screen<'a> {
    fn new(buf: &'a mut [u32], win_w: usize, win_h: usize, fonts: &'a mut Fonts) -> Self {
        let aspect = FRAME_W / FRAME_H;
        let (rw, rh) = if win_w as f32 / win_h.max(1) as f32 > aspect {
            (win_h as f32 * aspect, win_h as f32) // window wider than 4:3 → fit height
        } else {
            (win_w as f32, win_w as f32 / aspect) // taller → fit width
        };
        Screen {
            buf,
            win_w,
            win_h,
            ox: (win_w as f32 - rw) / 2.0,
            oy: (win_h as f32 - rh) / 2.0,
            scale: rw / FRAME_W,
            fonts,
        }
    }

    // -- coordinate transform ------------------------------------------------
    fn mx(&self, x: f32) -> f32 {
        self.ox + x * self.scale
    }
    fn my(&self, y: f32) -> f32 {
        self.oy + y * self.scale
    }
    /// Device pixel size for a design point size.
    fn dpx(&self, px: f32) -> f32 {
        px * self.scale
    }

    // -- pixels --------------------------------------------------------------
    fn blend(&mut self, x: i32, y: i32, rgb: u32, a: f32) {
        if a <= 0.0 || x < 0 || y < 0 || x as usize >= self.win_w || y as usize >= self.win_h {
            return;
        }
        let i = y as usize * self.win_w + x as usize;
        let dst = self.buf[i];
        let a = a.min(1.0);
        let ia = 1.0 - a;
        let ch = |sh: u32| {
            let s = ((rgb >> sh) & 0xFF) as f32;
            let d = ((dst >> sh) & 0xFF) as f32;
            (s * a + d * ia + 0.5) as u32
        };
        self.buf[i] = (ch(16) << 16) | (ch(8) << 8) | ch(0);
    }

    // -- rectangles ----------------------------------------------------------
    /// Coverage of a point `(lx,ly)` inside a `w×h` rounded rect, radius `r`,
    /// with ~1px anti-aliasing on every edge.
    fn rr_cov(lx: f32, ly: f32, w: f32, h: f32, r: f32) -> f32 {
        let cx = lx.clamp(r, (w - r).max(r));
        let cy = ly.clamp(r, (h - r).max(r));
        let dx = lx - cx;
        let dy = ly - cy;
        let d = (dx * dx + dy * dy).sqrt();
        (r + 0.5 - d).clamp(0.0, 1.0)
    }

    /// Fill a rounded rect whose color is chosen per-pixel by `col(fx, fy)`,
    /// where `fx,fy ∈ 0..1` are the normalized position (for gradients).
    fn round_fill<F: Fn(f32, f32) -> u32>(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, col: F) {
        let x0 = self.mx(x);
        let y0 = self.my(y);
        let wd = w * self.scale;
        let hd = h * self.scale;
        let rd = r * self.scale;
        for yy in y0.floor() as i32..(y0 + hd).ceil() as i32 {
            for xx in x0.floor() as i32..(x0 + wd).ceil() as i32 {
                let lx = xx as f32 + 0.5 - x0;
                let ly = yy as f32 + 0.5 - y0;
                let cov = Self::rr_cov(lx, ly, wd, hd, rd);
                if cov > 0.0 {
                    let fx = (lx / wd).clamp(0.0, 1.0);
                    let fy = (ly / hd).clamp(0.0, 1.0);
                    self.blend(xx, yy, col(fx, fy), cov);
                }
            }
        }
    }

    fn round_rect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, rgb: u32) {
        self.round_fill(x, y, w, h, r, |_, _| rgb);
    }

    fn round_vgrad(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, top: u32, bot: u32) {
        self.round_fill(x, y, w, h, r, |_, fy| lerp_rgb(top, bot, fy));
    }

    /// Stroke a rounded-rect outline of design thickness `t` (AA ring).
    fn round_border(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, t: f32, rgb: u32) {
        let x0 = self.mx(x);
        let y0 = self.my(y);
        let wd = w * self.scale;
        let hd = h * self.scale;
        let rd = r * self.scale;
        let td = (t * self.scale).max(1.0);
        for yy in y0.floor() as i32..(y0 + hd).ceil() as i32 {
            for xx in x0.floor() as i32..(x0 + wd).ceil() as i32 {
                let lx = xx as f32 + 0.5 - x0;
                let ly = yy as f32 + 0.5 - y0;
                let outer = Self::rr_cov(lx, ly, wd, hd, rd);
                let inner = Self::rr_cov(lx - td, ly - td, wd - 2.0 * td, hd - 2.0 * td, (rd - td).max(0.0));
                let cov = (outer - inner).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.blend(xx, yy, rgb, cov);
                }
            }
        }
    }

    /// A 1-design-px horizontal rule from `x` to `x+w` at `y`.
    fn rule(&mut self, x: f32, y: f32, w: f32, rgb: u32) {
        let x0 = self.mx(x);
        let y0 = self.my(y);
        let th = self.scale.round().max(1.0) as i32;
        for dy in 0..th {
            for xx in x0.round() as i32..(x0 + w * self.scale).round() as i32 {
                self.blend(xx, y0 as i32 + dy, rgb, 1.0);
            }
        }
    }

    /// Fill a small solid design-space rect (swatches, underlines, stems).
    fn box_fill(&mut self, x: f32, y: f32, w: f32, h: f32, rgb: u32) {
        let x0 = self.mx(x);
        let y0 = self.my(y);
        for yy in y0.round() as i32..(y0 + h * self.scale).round() as i32 {
            for xx in x0.round() as i32..(x0 + w * self.scale).round() as i32 {
                self.blend(xx, yy, rgb, 1.0);
            }
        }
    }

    // -- text ----------------------------------------------------------------
    /// Blit one glyph at device pen position; returns its advance (device px).
    fn glyph(&mut self, penx: f32, baseline: f32, id: FontId, dpx: f32, c: char, rgb: u32) -> f32 {
        let r = self.fonts.raster(id, dpx, c).clone();
        let gx0 = penx.round() as i32 + r.left;
        let gy0 = baseline.round() as i32 + r.top;
        for gy in 0..r.h {
            for gx in 0..r.w {
                let a = r.cov[gy * r.w + gx] as f32 / 255.0;
                if a > 0.0 {
                    self.blend(gx0 + gx as i32, gy0 + gy as i32, rgb, a);
                }
            }
        }
        r.advance
    }

    /// Width of `s` in design units at `px`, including `track` between glyphs.
    fn text_w(&self, id: FontId, px: f32, s: &str, track: f32) -> f32 {
        let n = s.chars().count();
        let extra = if n > 1 { (n - 1) as f32 * track } else { 0.0 };
        self.fonts.measure(id, self.dpx(px), s) / self.scale + extra
    }

    /// Core left-aligned draw at a device baseline; returns the end pen x.
    fn draw_dev(&mut self, x: f32, baseline: f32, s: &str, id: FontId, dpx: f32, rgb: u32, track_dev: f32) -> f32 {
        let mut penx = x;
        for (i, c) in s.chars().enumerate() {
            if i > 0 {
                penx += track_dev;
            }
            penx += self.glyph(penx, baseline, id, dpx, c, rgb);
        }
        penx
    }

    /// Left-aligned text whose top sits at design `top`.
    fn text(&mut self, x: f32, top: f32, s: &str, id: FontId, px: f32, rgb: u32) {
        let dpx = self.dpx(px);
        let baseline = self.my(top) + self.fonts.ascent(id, dpx);
        self.draw_dev(self.mx(x), baseline, s, id, dpx, rgb, 0.0);
    }

    /// Left-aligned text with letter-spacing `track` (design px), top at `top`.
    fn text_tracked(&mut self, x: f32, top: f32, s: &str, id: FontId, px: f32, rgb: u32, track: f32) {
        let dpx = self.dpx(px);
        let baseline = self.my(top) + self.fonts.ascent(id, dpx);
        self.draw_dev(self.mx(x), baseline, s, id, dpx, rgb, track * self.scale);
    }

    /// Left-aligned text vertically centered on design `cy` (cap-height centered).
    fn text_mid(&mut self, x: f32, cy: f32, s: &str, id: FontId, px: f32, rgb: u32) -> f32 {
        let dpx = self.dpx(px);
        let baseline = self.my(cy) + 0.35 * dpx;
        self.draw_dev(self.mx(x), baseline, s, id, dpx, rgb, 0.0)
    }

    /// Text centered on design point `(cx, cy)`.
    fn text_center(&mut self, cx: f32, cy: f32, s: &str, id: FontId, px: f32, rgb: u32) {
        let w = self.text_w(id, px, s, 0.0);
        self.text_mid(cx - w / 2.0, cy, s, id, px, rgb);
    }

    /// Left-aligned text vertically centered on `cy`, with letter-spacing.
    fn text_mid_tracked(&mut self, x: f32, cy: f32, s: &str, id: FontId, px: f32, rgb: u32, track: f32) {
        let dpx = self.dpx(px);
        let baseline = self.my(cy) + 0.35 * dpx;
        self.draw_dev(self.mx(x), baseline, s, id, dpx, rgb, track * self.scale);
    }

    /// Tracked text centered on `(cx, cy)` (for Silkscreen word-keycaps).
    fn text_center_tracked(&mut self, cx: f32, cy: f32, s: &str, id: FontId, px: f32, rgb: u32, track: f32) {
        let w = self.text_w(id, px, s, track);
        self.text_mid_tracked(cx - w / 2.0, cy, s, id, px, rgb, track);
    }

    /// Right-aligned text ending at design `right`, vertically centered on `cy`.
    fn text_right_mid(&mut self, right: f32, cy: f32, s: &str, id: FontId, px: f32, rgb: u32) {
        let w = self.text_w(id, px, s, 0.0);
        self.text_mid(right - w, cy, s, id, px, rgb);
    }

    /// One line of mixed-style runs, first line top at design `top`.
    fn run_line(&mut self, x: f32, top: f32, runs: &[Run], px: f32) {
        let dpx = self.dpx(px);
        let baseline = self.my(top) + self.fonts.ascent(MR, dpx);
        let mut pen = self.mx(x);
        for &(t, id, rgb) in runs {
            pen = self.draw_dev(pen, baseline, t, id, dpx, rgb, 0.0);
        }
    }

    // -- vector glyphs (chars the embedded fonts lack) -------------------------
    /// A small filled arrow centered on `(cx,cy)` — cursor marks, ← → hints.
    fn arrow(&mut self, cx: f32, cy: f32, dir: Dir, size: f32, rgb: u32) {
        let h = size / 2.0; // half extent
        // Arrow head triangle + a short stem, in design space.
        let (tip, base_l, base_r, stem) = match dir {
            Dir::Up => (
                (cx, cy - h),
                (cx - h, cy),
                (cx + h, cy),
                (cx - 0.8, cy, 1.6, h),
            ),
            Dir::Down => (
                (cx, cy + h),
                (cx - h, cy),
                (cx + h, cy),
                (cx - 0.8, cy - h, 1.6, h),
            ),
            Dir::Left => (
                (cx - h, cy),
                (cx, cy - h),
                (cx, cy + h),
                (cx, cy - 0.8, h, 1.6),
            ),
            Dir::Right => (
                (cx + h, cy),
                (cx, cy - h),
                (cx, cy + h),
                (cx - h, cy - 0.8, h, 1.6),
            ),
        };
        self.box_fill(stem.0, stem.1, stem.2, stem.3, rgb);
        self.fill_tri(tip, base_l, base_r, rgb);
    }

    /// Filled triangle (design coords), 2×2-supersampled for light AA.
    fn fill_tri(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32), rgb: u32) {
        let pts = [self.to_dev(a), self.to_dev(b), self.to_dev(c)];
        let minx = pts.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor() as i32;
        let maxx = pts.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as i32;
        let miny = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32;
        let maxy = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32;
        let edge = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| {
            (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
        };
        for yy in miny..maxy {
            for xx in minx..maxx {
                let mut hits = 0;
                for sy in 0..2 {
                    for sx in 0..2 {
                        let p = (xx as f32 + 0.25 + 0.5 * sx as f32, yy as f32 + 0.25 + 0.5 * sy as f32);
                        let w0 = edge(pts[1], pts[2], p);
                        let w1 = edge(pts[2], pts[0], p);
                        let w2 = edge(pts[0], pts[1], p);
                        if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                            hits += 1;
                        }
                    }
                }
                if hits > 0 {
                    self.blend(xx, yy, rgb, hits as f32 / 4.0);
                }
            }
        }
    }

    fn to_dev(&self, p: (f32, f32)) -> (f32, f32) {
        (self.mx(p.0), self.my(p.1))
    }

    /// A thick line segment between two design points (drawn as a rotated quad).
    fn seg(&mut self, a: (f32, f32), b: (f32, f32), thick: f32, rgb: u32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
        let (nx, ny) = (-dy / len * thick / 2.0, dx / len * thick / 2.0);
        let p0 = (a.0 + nx, a.1 + ny);
        let p1 = (a.0 - nx, a.1 - ny);
        let p2 = (b.0 - nx, b.1 - ny);
        let p3 = (b.0 + nx, b.1 + ny);
        self.fill_tri(p0, p1, p2, rgb);
        self.fill_tri(p0, p2, p3, rgb);
    }

    /// The macOS control glyph `⌃` (an up chevron), centered on `(cx,cy)`. IBM
    /// Plex Mono has no glyph for it, so the mac shortcuts are drawn from strokes.
    fn icon_ctrl(&mut self, cx: f32, cy: f32, s: f32, rgb: u32) {
        let (w, h) = (s * 0.40, s * 0.26);
        let t = (s * 0.12).max(0.8);
        self.seg((cx - w, cy + h), (cx, cy - h), t, rgb);
        self.seg((cx, cy - h), (cx + w, cy + h), t, rgb);
    }

    /// The macOS command glyph `⌘` — four corner loops joined into a square.
    fn icon_cmd(&mut self, cx: f32, cy: f32, s: f32, rgb: u32) {
        let a = s * 0.28; // half-distance between loop centers
        let r = s * 0.17; // loop radius
        let t = (s * 0.11).max(0.8);
        self.seg((cx - a, cy - a), (cx + a, cy - a), t, rgb);
        self.seg((cx - a, cy + a), (cx + a, cy + a), t, rgb);
        self.seg((cx - a, cy - a), (cx - a, cy + a), t, rgb);
        self.seg((cx + a, cy - a), (cx + a, cy + a), t, rgb);
        for (lx, ly) in [(cx - a, cy - a), (cx + a, cy - a), (cx - a, cy + a), (cx + a, cy + a)] {
            self.round_border(lx - r, ly - r, 2.0 * r, 2.0 * r, r, t, rgb);
        }
    }

    /// The macOS option glyph `⌥` — a slash between two horizontal ticks.
    fn icon_opt(&mut self, cx: f32, cy: f32, s: f32, rgb: u32) {
        let (w, h) = (s * 0.42, s * 0.26);
        let t = (s * 0.11).max(0.8);
        self.seg((cx - w, cy - h), (cx - s * 0.14, cy - h), t, rgb);
        self.seg((cx - s * 0.14, cy - h), (cx + s * 0.14, cy + h), t, rgb);
        self.seg((cx + s * 0.14, cy + h), (cx + w, cy + h), t, rgb);
        self.seg((cx + s * 0.16, cy - h), (cx + w, cy - h), t, rgb);
    }
}

/// Cursor/hint arrow direction.
#[derive(Clone, Copy)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// One piece of an inline legend/footer line: styled text, a drawn arrow, or a
/// drawn macOS modifier glyph.
enum Seg<'a> {
    T(&'a str, FontId, u32),
    A(Dir, u32),
    MacOpt(u32),
    MacCtl(u32),
}

// ===========================================================================
// Composite widgets
// ===========================================================================
impl Screen<'_> {
    /// A section eyebrow: Plex 700, 10px, tracking 2.6, cyan, uppercase copy.
    fn eyebrow(&mut self, x: f32, top: f32, s: &str) {
        self.text_tracked(x, top, s, MB, 10.0, CYAN, 2.6);
    }

    /// Chip frame: flat fill + 1px border (no gradients, no shadows).
    fn chip_frame(&mut self, x: f32, cy: f32, w: f32, h: f32, r: f32) {
        let y = cy - h / 2.0;
        self.round_rect(x, y, w, h, r, CHIP_BG);
        self.round_border(x, y, w, h, r, 1.0, CHIP_BORDER);
    }

    /// A flat text key chip centered vertically on `cy`. Returns its width.
    fn flat_chip(&mut self, x: f32, cy: f32, label: &str, h: f32, px: f32, r: f32, pad: f32) -> f32 {
        let w = self.text_w(MS, px, label, 0.0) + 2.0 * pad;
        self.chip_frame(x, cy, w, h, r);
        self.text_center(x + w / 2.0, cy, label, MS, px, CHIP_INK);
        w
    }

    /// Any [`KeyChip`] — text, `⌘ <letter>`, or `⌃ ⌘ <letter>`. Returns width.
    fn key_chip(&mut self, x: f32, cy: f32, chip: KeyChip, h: f32, px: f32, r: f32, pad: f32) -> f32 {
        match chip {
            KeyChip::Text(s) => self.flat_chip(x, cy, s, h, px, r, pad),
            KeyChip::MacCtrlCmd(s) => {
                let icon = px * 0.9;
                let gap = px * 0.36;
                let content = icon + gap + icon + gap + self.text_w(MS, px, s, 0.0);
                let w = content + 2.0 * pad;
                self.chip_frame(x, cy, w, h, r);
                let mut cx = x + pad;
                self.icon_ctrl(cx + icon / 2.0, cy, icon, CHIP_INK);
                cx += icon + gap;
                self.icon_cmd(cx + icon / 2.0, cy, icon, CHIP_INK);
                cx += icon + gap;
                self.text_mid(cx, cy, s, MS, px, CHIP_INK);
                w
            }
            KeyChip::MacCmd(s) => {
                let icon = px * 0.9;
                let gap = px * 0.36;
                let content = icon + gap + self.text_w(MS, px, s, 0.0);
                let w = content + 2.0 * pad;
                self.chip_frame(x, cy, w, h, r);
                self.icon_cmd(x + pad + icon / 2.0, cy, icon, CHIP_INK);
                self.text_mid(x + pad + icon + gap, cy, s, MS, px, CHIP_INK);
                w
            }
        }
    }

    /// A single line of mixed segments centered on `cy`; returns the end x.
    fn seg_line(&mut self, x: f32, cy: f32, px: f32, segs: &[Seg]) -> f32 {
        let mut pen = x;
        for s in segs {
            match *s {
                Seg::T(t, id, rgb) => {
                    self.text_mid(pen, cy, t, id, px, rgb);
                    pen += self.text_w(id, px, t, 0.0);
                }
                Seg::A(dir, rgb) => {
                    self.arrow(pen + 4.5, cy, dir, 9.0, rgb);
                    pen += 9.0;
                }
                Seg::MacOpt(rgb) => {
                    self.icon_opt(pen + px * 0.45, cy, px * 0.9, rgb);
                    pen += px * 0.9;
                }
                Seg::MacCtl(rgb) => {
                    self.icon_ctrl(pen + px * 0.45, cy, px * 0.9, rgb);
                    pen += px * 0.9;
                }
            }
        }
        pen
    }

    /// A wrapped rich paragraph of colored runs. Returns the bottom y (design).
    fn paragraph(&mut self, x: f32, top: f32, width: f32, runs: &[Run], px: f32, lh: f32) -> f32 {
        self.paragraph_indent(x, top, width, 0.0, runs, px, lh)
    }

    /// [`Self::paragraph`] whose first line starts `first_indent` in (for an
    /// inline chip preceding the copy).
    fn paragraph_indent(
        &mut self,
        x: f32,
        top: f32,
        width: f32,
        first_indent: f32,
        runs: &[Run],
        px: f32,
        lh: f32,
    ) -> f32 {
        let dpx = self.dpx(px);
        let ascent = self.fonts.ascent(MR, dpx);
        let space_w = self.fonts.measure(MR, dpx, " ");
        let line_h = px * lh;
        let start_dev = self.mx(x);
        let width_dev = width * self.scale;
        // Flatten into (char, font, color).
        let mut chars: Vec<(char, FontId, u32)> = Vec::new();
        for &(t, id, rgb) in runs {
            for c in t.chars() {
                chars.push((c, id, rgb));
            }
        }
        let mut line = top;
        let mut penx = first_indent * self.scale; // device, from line start
        let mut first_word = true;
        let mut pending_space = false;
        let mut i = 0;
        while i < chars.len() {
            if chars[i].0 == ' ' {
                pending_space = true;
                i += 1;
                continue;
            }
            // gather a word [i..j)
            let mut j = i;
            let mut word_w = 0.0f32;
            while j < chars.len() && chars[j].0 != ' ' {
                let (c, id, _) = chars[j];
                word_w += self.fonts.measure(id, dpx, &c.to_string());
                j += 1;
            }
            let space = if pending_space && !first_word { space_w } else { 0.0 };
            if !first_word && penx + space + word_w > width_dev {
                line += line_h;
                penx = 0.0;
            } else {
                penx += space;
            }
            pending_space = false;
            first_word = false;
            let baseline = self.my(line) + ascent;
            let mut gx = start_dev + penx;
            for &(c, id, rgb) in &chars[i..j] {
                gx += self.glyph(gx, baseline, id, dpx, c, rgb);
            }
            penx += word_w;
            i = j;
        }
        line + line_h
    }
}

// ===========================================================================
// Chrome: top bar and footer (shared by every tab)
// ===========================================================================
impl Screen<'_> {
    fn top_bar(&mut self, tab: HelpTab) {
        // wordmark + HELP, baseline-aligned, vertically centered in the bar
        let bar_cy = TOP_RULE_Y / 2.0;
        let mark_dpx = self.dpx(15.0);
        let baseline = self.my(bar_cy) + 0.35 * mark_dpx;
        let end = self.draw_dev(self.mx(PAD_X), baseline, "LIBRE99", SR, mark_dpx, INK, 0.0);
        self.draw_dev(end + 11.0 * self.scale, baseline, "HELP", MS, self.dpx(10.0), FAINT, 2.5 * self.scale);

        // tab list, right-aligned: `number + label`, 24px between tabs
        let widths: Vec<(f32, f32)> = HelpTab::ALL
            .iter()
            .map(|t| {
                let nw = self.text_w(MR, 10.0, "0", 0.0);
                let lw = self.text_w(MS, 11.0, t.label(), 1.5);
                (nw, nw + 6.0 + lw)
            })
            .collect();
        let total: f32 = widths.iter().map(|(_, w)| *w).sum::<f32>() + 24.0 * (HelpTab::ALL.len() - 1) as f32;
        let mut x = RIGHT_X - total;
        for (i, t) in HelpTab::ALL.iter().enumerate() {
            let (nw, tw) = widths[i];
            let active = *t == tab;
            let num = (i + 1).to_string();
            self.text_mid(x, bar_cy, &num, MR, 10.0, TAB_NUM);
            let color = if active { CYAN } else { MUTED };
            self.text_mid_tracked(x + nw + 6.0, bar_cy, t.label(), MS, 11.0, color, 1.5);
            if active {
                // the 2px accent underline sits on the bar's bottom rule
                self.box_fill(x, TOP_RULE_Y - 2.0, tw, 2.0, CYAN);
            }
            x += tw + 24.0;
        }
        self.rule(0.0, TOP_RULE_Y, FRAME_W, RULE);
    }

    /// The footer: navigation hints left, a per-tab note right.
    fn footer(&mut self, right_note: &str) {
        self.rule(0.0, FOOT_RULE_Y, FRAME_W, RULE);
        self.seg_line(
            PAD_X,
            FOOT_CY,
            11.0,
            &[
                Seg::T("ESC", MS, BRIGHT),
                Seg::T(" close  ·  ", MR, FAINT),
                Seg::T("TAB", MS, BRIGHT),
                Seg::T(" or ", MR, FAINT),
                Seg::A(Dir::Left, BRIGHT),
                Seg::T(" ", MR, FAINT),
                Seg::A(Dir::Right, BRIGHT),
                Seg::T(" cycle  ·  ", MR, FAINT),
                Seg::T("1–4", MS, BRIGHT),
                Seg::T(" jump", MR, FAINT),
            ],
        );
        if !right_note.is_empty() {
            self.text_right_mid(RIGHT_X, FOOT_CY, right_note, MR, 11.0, FAINT);
        }
    }
}

// ===========================================================================
// 1 · START
// ===========================================================================
impl Screen<'_> {
    fn tab_start(&mut self) {
        let x = CONTENT_X;
        // H1, two lines, 30px/1.3
        self.text_tracked(x, 104.0, "Libre99 emulates the TI-99/4A", MM, 30.0, INK, -0.5);
        self.text_tracked(x, 143.0, "and boots its own firmware.", MM, 30.0, INK, -0.5);
        self.paragraph(
            x,
            196.0,
            660.0,
            &[(
                "No ROMs to find, nothing to install. Press any key at the title screen to begin.",
                MR,
                MUTED,
            )],
            14.0,
            1.6,
        );

        // three ruled columns
        let col_w = (CONTENT_W - 2.0 * 38.0) / 3.0; // 284
        let cols_top = 285.0;
        let titles = ["Open source", "Load anything", "Keep your place"];
        for (i, title) in titles.iter().enumerate() {
            let cx = x + i as f32 * (col_w + 38.0);
            self.rule(cx, cols_top, col_w, CHIP_BORDER);
            self.text(cx, cols_top + 16.0, title, MS, 14.0, INK);
        }
        let body_top = cols_top + 37.0;
        self.paragraph(
            x,
            body_top,
            col_w,
            &[(
                "The emulator and its built-in firmware ROMs are all open source. Nothing proprietary is required.",
                MR,
                MUTED,
            )],
            12.5,
            1.6,
        );
        let c2 = x + col_w + 38.0;
        let chip_w = self.flat_chip(c2, body_top + 9.0, "F9", 19.0, 10.5, 4.0, 6.0);
        self.paragraph_indent(
            c2,
            body_top,
            col_w,
            chip_w + 5.0,
            &[(
                "opens the file chooser. Mount .ctg/.bin cartridges and .dsk floppies from disk.",
                MR,
                MUTED,
            )],
            12.5,
            1.6,
        );
        self.paragraph(
            c2 + col_w + 38.0,
            body_top,
            col_w,
            &[(
                "Quit any time — a resume state saves automatically and next launch picks up exactly where you left off.",
                MR,
                MUTED,
            )],
            12.5,
            1.6,
        );

        // five keys to start, anchored toward the bottom
        self.eyebrow(x, 554.0, "FIVE KEYS TO START");
        let keys: [(KeyChip, &str); 5] = [
            (KeyChip::Text("ESC"), "this help"),
            (KeyChip::Text("F9"), "mount media"),
            (KeyChip::Text("F4"), "export disk"),
            (FULLSCREEN_CHIP, "fullscreen"),
            (QUIT_CHIP, "quit — state saved"),
        ];
        let mut kx = x;
        for (chip, label) in keys {
            let cw = self.key_chip(kx, 596.0, chip, 32.0, 14.0, 6.0, 13.0);
            self.text(kx, 620.0, label, MR, 11.0, MUTED);
            kx += cw.max(self.text_w(MR, 11.0, label, 0.0)) + 30.0;
        }
        self.paragraph(
            x,
            668.0,
            740.0,
            &[(
                "The firmware is Libre99's own clean-room implementation. No original TI software is embedded.",
                MR,
                FAINT,
            )],
            12.0,
            1.65,
        );
    }
}

// ===========================================================================
// 2 · KEYBOARD
// ===========================================================================

/// A keycap's legends.
enum Cap {
    /// A character key: main legend top-left, optional amber SHIFT legend
    /// top-right, optional green FCTN legend bottom-left.
    Glyph {
        main: &'static str,
        shift: Option<&'static str>,
        fctn: Option<Mark>,
    },
    /// A word key (ENTER, SHIFT, SPACE, …) — centered Silkscreen.
    Word(&'static str),
}

enum Mark {
    Txt(&'static str),
    Arrow(Dir),
}

/// One key in a row: design width and legends (all keycaps are 56 tall).
struct Key {
    w: f32,
    cap: Cap,
}

fn g(main: &'static str, shift: Option<&'static str>, fctn: Option<Mark>) -> Key {
    Key { w: 72.0, cap: Cap::Glyph { main, shift, fctn } }
}

fn word(text: &'static str, w: f32) -> Key {
    Key { w, cap: Cap::Word(text) }
}

impl Screen<'_> {
    fn tab_keyboard(&mut self) {
        let x = CONTENT_X;
        // title row, right note baseline-aligned with the H2
        self.text_tracked(x, 92.0, "The TI-99/4A keyboard", MM, 22.0, INK, -0.3);
        let note = "your host keyboard already speaks TI";
        let h2_baseline = self.my(92.0) + self.fonts.ascent(MM, self.dpx(22.0));
        let note_w = self.text_w(MR, 12.0, note, 0.0);
        self.draw_dev(self.mx(RIGHT_X - note_w), h2_baseline, note, MR, self.dpx(12.0), MUTED, 0.0);

        // the map: keycaps 72×56, 8px gaps, 8px row gaps
        // Every SHIFT/FCTN legend below is the authoritative set from
        // docs/USER-GUIDE.md — verify against the guide when it changes.
        let up = || Some(Mark::Arrow(Dir::Up));
        let dn = || Some(Mark::Arrow(Dir::Down));
        let lf = || Some(Mark::Arrow(Dir::Left));
        let rt = || Some(Mark::Arrow(Dir::Right));
        let t = |s: &'static str| Some(Mark::Txt(s));
        let rows: [(f32, Vec<Key>); 5] = [
            (0.0, vec![
                g("1", Some("!"), t("DEL")), g("2", Some("@"), t("INS")), g("3", Some("#"), t("ERASE")),
                g("4", Some("$"), t("CLEAR")), g("5", Some("%"), t("BEGIN")), g("6", Some("^"), t("PROC'D")),
                g("7", Some("&"), t("AID")), g("8", Some("*"), t("REDO")), g("9", Some("("), t("BACK")),
                g("0", Some(")"), None), g("=", Some("+"), t("QUIT")),
            ]),
            (30.0, vec![
                g("Q", None, None), g("W", None, t("~")), g("E", None, up()),
                g("R", None, t("[")), g("T", None, t("]")), g("Y", None, None),
                g("U", None, t("_")), g("I", None, t("?")), g("O", None, t("'")),
                g("P", None, t("\"")), g("/", Some("-"), None),
            ]),
            (44.0, vec![
                g("A", None, t("|")), g("S", None, lf()), g("D", None, rt()),
                g("F", None, t("{")), g("G", None, t("}")), g("H", None, None),
                g("J", None, None), g("K", None, None), g("L", None, None),
                g(";", Some(":"), None), word("ENTER", 72.0),
            ]),
            (14.0, vec![
                word("SHIFT", 72.0), g("Z", None, t("\\")), g("X", None, dn()),
                g("C", None, t("`")), g("V", None, None), g("B", None, None),
                g("N", None, None), g("M", None, None), g(",", Some("<"), None),
                g(".", Some(">"), None), word("SHIFT", 72.0),
            ]),
            (75.0, vec![
                word("ALPHA LOCK", 126.0), word("CTRL", 94.0), word("SPACE", 428.0), word("FCTN", 94.0),
            ]),
        ];
        let mut y = 146.0;
        for (indent, keys) in &rows {
            let mut kx = x + indent;
            for key in keys {
                self.keycap(kx, y, key.w, &key.cap);
                kx += key.w + 8.0;
            }
            y += 64.0;
        }

        // legend strip anchored above the footer
        self.rule(x, 657.0, CONTENT_W, RULE);
        let cy = 683.0;
        let mut lx = x;
        self.round_rect(lx, cy - 4.5, 9.0, 9.0, 2.0, AMBER);
        lx = self.seg_line(lx + 17.0, cy, 12.0, &[Seg::T("SHIFT symbol", MR, MUTED)]) + 34.0;
        self.round_rect(lx, cy - 4.5, 9.0, 9.0, 2.0, GREEN);
        lx = self.seg_line(lx + 17.0, cy, 12.0, &[Seg::T("FCTN function", MR, MUTED)]) + 34.0;
        // host-modifier mapping (per docs/USER-GUIDE.md: Left Alt/Option, Left Ctrl)
        lx = if IS_MAC {
            self.seg_line(
                lx,
                cy,
                12.0,
                &[
                    Seg::T("FCTN", MS, BRIGHT),
                    Seg::T(" ", MR, MUTED),
                    Seg::A(Dir::Right, MUTED),
                    Seg::T(" ", MR, MUTED),
                    Seg::MacOpt(MUTED),
                    Seg::T(" Option · ", MR, MUTED),
                    Seg::T("CTRL", MS, BRIGHT),
                    Seg::T(" ", MR, MUTED),
                    Seg::A(Dir::Right, MUTED),
                    Seg::T(" ", MR, MUTED),
                    Seg::MacCtl(MUTED),
                    Seg::T(" Control", MR, MUTED),
                ],
            )
        } else {
            self.seg_line(
                lx,
                cy,
                12.0,
                &[
                    Seg::T("FCTN", MS, BRIGHT),
                    Seg::T(" ", MR, MUTED),
                    Seg::A(Dir::Right, MUTED),
                    Seg::T(" Left Alt · ", MR, MUTED),
                    Seg::T("CTRL", MS, BRIGHT),
                    Seg::T(" ", MR, MUTED),
                    Seg::A(Dir::Right, MUTED),
                    Seg::T(" Left Ctrl", MR, MUTED),
                ],
            )
        } + 34.0;
        self.seg_line(
            lx,
            cy,
            12.0,
            &[
                Seg::T("FCTN", MS, BRIGHT),
                Seg::T(" + E S D X ", MR, MUTED),
                Seg::A(Dir::Right, MUTED),
                Seg::T(" TI cursor · arrows ", MR, MUTED),
                Seg::A(Dir::Right, MUTED),
                Seg::T(" joystick 1", MR, MUTED),
            ],
        );
    }

    /// One keycap: vertical gradient face, 1px border, corner legends.
    fn keycap(&mut self, x: f32, y: f32, w: f32, cap: &Cap) {
        let h = 56.0;
        self.round_vgrad(x, y, w, h, 7.0, CAP_TOP, CAP_BOT);
        self.round_border(x, y, w, h, 7.0, 1.0, CAP_BORDER);
        match cap {
            Cap::Glyph { main, shift, fctn } => {
                self.text(x + 9.0, y + 6.0, main, MS, 16.0, INK);
                if let Some(s) = shift {
                    let sw = self.text_w(MS, 10.0, s, 0.0);
                    self.text(x + w - 8.0 - sw, y + 6.0, s, MS, 10.0, AMBER);
                }
                match fctn {
                    Some(Mark::Txt(m)) => self.text_tracked(x + 9.0, y + 40.0, m, MS, 11.0, GREEN, 0.2),
                    Some(Mark::Arrow(d)) => self.arrow(x + 13.5, y + 47.0, *d, 9.0, GREEN),
                    None => {}
                }
            }
            Cap::Word(text) => {
                self.text_center_tracked(x + w / 2.0, y + h / 2.0, text, SR, 8.0, BRIGHT, 0.5);
            }
        }
    }
}

// ===========================================================================
// 3 · HOTKEYS
// ===========================================================================

/// One hotkey row: its key chip(s) and what they do.
struct HotRow {
    chips: &'static [KeyChip],
    label: &'static str,
}

const fn hr(chips: &'static [KeyChip], label: &'static str) -> HotRow {
    HotRow { chips, label }
}

impl Screen<'_> {
    fn tab_hotkeys(&mut self) {
        // Six ruled lists in a 3×2 grid. Bindings per docs/USER-GUIDE.md.
        let groups: [(&str, &[HotRow]); 6] = [
            ("OVERLAYS", &[
                hr(&[KeyChip::Text("ESC"), KeyChip::Text("F1")], "this help"),
                hr(&[KeyChip::Text("F9")], "file chooser"),
            ]),
            ("MEDIA", &[
                hr(&[KeyChip::Text("F9")], "mount cart / disk"),
                hr(&[KeyChip::Text("F4")], "export disk writes"),
                hr(&[KeyChip::Text("F2"), KeyChip::Text("F3")], "eject cart / disk"),
            ]),
            ("PLAYBACK", &[
                hr(&[KeyChip::Text("F10")], "pause / resume"),
                hr(&[KeyChip::Text("TAB")], "fast-forward (hold)"),
                hr(&[KeyChip::Text("F12")], "frame advance"),
            ]),
            ("CONSOLE", &[
                hr(&[KeyChip::Text("F5")], "reset console"),
                hr(&[QUIT_CHIP], "quit — state saved"),
            ]),
            ("STATE", &[
                hr(&[KeyChip::Text("F6"), KeyChip::Text("F8")], "resume save / load"),
                hr(&[KeyChip::Text("SHIFT F6")], "save snapshot"),
                hr(&[KeyChip::Text("SHIFT F8")], "load snapshot"),
                hr(&[KeyChip::Text("SHIFT F5")], "fresh start"),
            ]),
            ("DISPLAY & TOOLS", &[
                hr(&[FULLSCREEN_CHIP], "fullscreen"),
                hr(&[SCREENSHOT_CHIP], "screenshot"),
                hr(&[INSPECTOR_CHIP], "CPU inspector"),
                hr(&[KeyChip::Text("F7")], "keyboard layout"),
            ]),
        ];
        let col_w = (CONTENT_W - 2.0 * 44.0) / 3.0; // 280
        let row1_top = 98.0;
        // each grid row is as tall as its longest list (rows are 45px each)
        let row1_h = 25.0 + 3.0 * 45.0; // MEDIA/PLAYBACK have 3 rows
        let row2_top = row1_top + row1_h + 40.0;
        for (i, (title, rows)) in groups.iter().enumerate() {
            let gx = CONTENT_X + (i % 3) as f32 * (col_w + 44.0);
            let gy = if i < 3 { row1_top } else { row2_top };
            self.eyebrow(gx, gy, title);
            self.rule(gx, gy + 24.0, col_w, RULE_STRONG);
            for (r, row) in rows.iter().enumerate() {
                let top = gy + 25.0 + r as f32 * 45.0;
                let mut cx = gx;
                for &chip in row.chips {
                    cx += self.key_chip(cx, top + 22.0, chip, 22.0, 11.5, 5.0, 8.0) + 5.0;
                }
                self.text_mid(gx + 110.0, top + 22.0, row.label, MR, 12.5, MUTED);
                self.rule(gx, top + 44.0, col_w, RULE_ROW);
            }
        }

        // loading & saving — the facts that were the Media & State tab
        self.rule(CONTENT_X, 547.0, CONTENT_W, RULE);
        self.eyebrow(CONTENT_X, 567.0, "LOADING & SAVING");
        let lines: [&[Run]; 4] = [
            &[
                ("F9", MS, CHIP_INK),
                (" mounts .ctg/.bin cartridge and .dsk floppy images. Nothing is embedded.", MR, MUTED),
            ],
            &[
                ("Disk writes stay in memory until ", MR, MUTED),
                ("F4", MS, CHIP_INK),
                (" exports them to the image file.", MR, MUTED),
            ],
            &[(
                "The resume state saves on quit and restores on launch. Snapshots are separate, on demand.",
                MR,
                MUTED,
            )],
            &[
                ("Everything Libre99 writes lives under ", MR, MUTED),
                ("~/.libre99/", MS, CHIP_INK),
                (".", MR, MUTED),
            ],
        ];
        for (i, runs) in lines.iter().enumerate() {
            self.run_line(CONTENT_X, 589.0 + i as f32 * 25.0, runs, 12.5);
        }
    }
}

// ===========================================================================
// 4 · SETTINGS
// ===========================================================================
impl Screen<'_> {
    fn tab_settings(&mut self) {
        // All flags and preference keys per docs/USER-GUIDE.md, all rows shown.
        let cli: [(&str, &str); 10] = [
            ("--cartridge <path>", "mount a .ctg or raw .bin cartridge image"),
            ("--disk <path>", "insert a .dsk disk image into DSK1"),
            ("--system-rom <path>", "boot a console ROM in place of the clean-room default"),
            ("--system-grom <path>", "boot a console GROM in place of the clean-room default"),
            ("--disk-dsr <path>", "install a disk DSR ROM in place of the clean-room default"),
            ("--scale <n>", "integer window scale, 1–8"),
            ("--fullscreen", "start fullscreen"),
            ("--log-level <level>", "error / warn / info / debug / trace"),
            ("--version, -V", "print the version and exit"),
            ("--help, -h", "print usage and exit"),
        ];
        let prefs: [(&str, &str, &str); 10] = [
            ("log_level", "\"info\"", "logging verbosity: error / warn / info / debug / trace"),
            ("last_cartridge", "auto", "cartridge mounted at exit — managed by the app"),
            ("last_disk", "auto", "disk mounted at exit — managed by the app"),
            ("browser_dir", "auto", "where the F9 chooser opens — follows your last mount"),
            ("window_scale", "3", "integer upscale of the 256×192 image (1–8)"),
            ("fullscreen", "false", "start fullscreen"),
            ("audio_enabled", "true", "enable audio output"),
            ("audio_volume", "0.8", "output volume, 0.0–1.0"),
            ("key_layout", "\"character\"", "startup keyboard mapping: character or positional"),
            ("defeat_screen_blank", "false", "suppress the authentic ~9-minute idle screen blank"),
        ];
        // 20 rows + two headers must fit the 666px content area, so rows run a
        // 23px pitch (the handoff's ~28px rhythm doesn't fit with all rows real).
        const ROW: f32 = 23.0;
        let x = CONTENT_X;

        self.eyebrow(x, 94.0, "COMMAND LINE");
        let rows_top = self.table_header(117.0, &[(x, "FLAG"), (x + 230.0, "WHAT IT DOES")]);
        for (i, (flag, what)) in cli.iter().enumerate() {
            let cy = rows_top + i as f32 * ROW + 11.0;
            self.text_mid(x, cy, flag, MS, 13.0, CHIP_INK);
            self.text_mid(x + 230.0, cy, what, MR, 13.0, MUTED);
            self.rule(x, rows_top + (i + 1) as f32 * ROW - 1.0, CONTENT_W, RULE_ROW);
        }
        let t1_bottom = rows_top + 10.0 * ROW;

        self.eyebrow(x, t1_bottom + 38.0, "PREFERENCES — libre99.toml");
        let rows_top = self.table_header(
            t1_bottom + 61.0,
            &[(x, "KEY"), (x + 210.0, "DEFAULT"), (x + 360.0, "WHAT IT DOES")],
        );
        for (i, (key, default, what)) in prefs.iter().enumerate() {
            let cy = rows_top + i as f32 * ROW + 11.0;
            self.text_mid(x, cy, key, MS, 13.0, CHIP_INK);
            self.text_mid(x + 210.0, cy, default, MR, 13.0, MUTED);
            self.text_mid(x + 360.0, cy, what, MR, 13.0, MUTED);
            self.rule(x, rows_top + (i + 1) as f32 * ROW - 1.0, CONTENT_W, RULE_ROW);
        }
    }

    /// A table header row (uppercase column labels + strong rule); returns the
    /// y where the data rows start.
    fn table_header(&mut self, top: f32, cols: &[(f32, &str)]) -> f32 {
        for &(cx, label) in cols {
            self.text_tracked(cx, top + 9.0, label, MS, 10.0, FAINT, 2.0);
        }
        self.rule(CONTENT_X, top + 28.0, CONTENT_W, RULE_STRONG);
        top + 29.0
    }
}

/// Render the help overlay for `tab` into the window buffer (`win_w × win_h`),
/// at native resolution, filling the centered 4:3 region.
pub fn render(fonts: &mut Fonts, buf: &mut [u32], win_w: usize, win_h: usize, tab: HelpTab) {
    // solid black backdrop — the letterbox and the page are one surface
    for p in buf.iter_mut() {
        *p = 0x0000_0000;
    }
    if win_w == 0 || win_h == 0 {
        return;
    }
    let mut s = Screen::new(buf, win_w, win_h, fonts);
    if s.scale <= 0.0 {
        return;
    }
    s.top_bar(tab);
    s.footer(if tab == HelpTab::Start { VERSION } else { "" });
    match tab {
        HelpTab::Start => s.tab_start(),
        HelpTab::Keyboard => s.tab_keyboard(),
        HelpTab::Hotkeys => s.tab_hotkeys(),
        HelpTab::Settings => s.tab_settings(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_every_tab_without_panic() {
        let mut fonts = Fonts::new();
        let (w, h) = (1024usize, 768usize);
        let mut buf = vec![0u32; w * h];
        for tab in HelpTab::ALL {
            render(&mut fonts, &mut buf, w, h, tab);
            assert!(buf.iter().any(|&p| p != 0), "{tab:?} drew nothing");
        }
    }

    /// Dump each tab to a PNG under the system temp dir for visual inspection
    /// (`cargo test -p libre99-app dump_tabs_to_png -- --ignored`, then open them).
    #[test]
    #[ignore]
    fn dump_tabs_to_png() {
        let mut fonts = Fonts::new();
        let (w, h) = (1024usize, 768usize);
        let dir = std::env::temp_dir();
        for (i, tab) in HelpTab::ALL.iter().enumerate() {
            let mut buf = vec![0u32; w * h];
            render(&mut fonts, &mut buf, w, h, *tab);
            let png = crate::screenshot::encode_png(w, h, &buf);
            let path = dir.join(format!("libre99_help_{}_{:?}.png", i + 1, tab));
            std::fs::write(&path, &png).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }
}
