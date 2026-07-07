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

//! The `F1` / `Esc` help overlay — a five-tab reference (Start, Keyboard,
//! Hotkeys, Media & State, Settings) drawn at the window's **native** pixel
//! resolution with the embedded smooth fonts ([`crate::font`]).
//!
//! Unlike the other overlays (toast, CPU inspector, media browser), which paint
//! into the 256×192 framebuffer and are then nearest-neighbor upscaled, this one
//! renders straight into the window-sized surface so 9–26 px type stays crisp.
//! It fills the largest centered **4:3** rectangle that fits the window — the
//! same region the emulated image occupies — and letterboxes the rest.
//!
//! All layout is written in the design's 1024×768 coordinate space and scaled to
//! the live region, so it looks identical at every `window_scale` and fullscreen.
//! The recreated layout, colors, and copy follow `design_handoff_help_screen`;
//! all key/flag/pref values track `docs/USER-GUIDE.md` (the source of truth —
//! keep the two in sync when controls or preferences change).
//!
//! This is a software renderer, so the drawing primitives take the usual
//! `x, y, w, h, …, color` argument lists; `too_many_arguments` is expected here.
#![allow(clippy::too_many_arguments)]

use crate::font::{FontId, Fonts};

/// Platform-correct emulator-shortcut labels for the on-screen help: the command
/// modifier is `Cmd` on macOS and `Ctrl` elsewhere (see
/// [`crate::input::HostMods::command`]), and quit is the OS-standard `Cmd Q` on
/// macOS vs. `Alt F4` (window close, which auto-saves) on Windows. Kept in sync
/// with `app.rs`'s hotkey match so the labels never lie about the real bindings.
#[cfg(target_os = "macos")]
mod cmd_label {
    pub const INSPECTOR: &str = "Cmd D";
    pub const QUIT: &str = "Cmd Q";
    pub const SCREENSHOT: &str = "Cmd S";
    pub const SCREENSHOT_HYPHEN: &str = "Cmd-S";
    pub const SCREENSHOT_PNGS: &str = "Cmd-S PNGs";
}
#[cfg(not(target_os = "macos"))]
mod cmd_label {
    pub const INSPECTOR: &str = "Ctrl D";
    pub const QUIT: &str = "Alt F4";
    pub const SCREENSHOT: &str = "Ctrl S";
    pub const SCREENSHOT_HYPHEN: &str = "Ctrl-S";
    pub const SCREENSHOT_PNGS: &str = "Ctrl-S PNGs";
}

/// A styled text run for [`Screen::paragraph`]: the text, its face, its color.
type Run<'a> = (&'a str, FontId, u32);

// Short aliases for the six embedded faces (see [`FontId`]).
const SR: FontId = FontId::SilkRegular;
const SB: FontId = FontId::SilkBold;
const MR: FontId = FontId::MonoRegular;
const MM: FontId = FontId::MonoMedium;
const MS: FontId = FontId::MonoSemiBold;
const MB: FontId = FontId::MonoBold;

// ---- design canvas ---------------------------------------------------------
const FRAME_W: f32 = 1024.0;
const FRAME_H: f32 = 768.0;
const PAD_X: f32 = 30.0;
const CONTENT_X: f32 = PAD_X;
const CONTENT_W: f32 = FRAME_W - 2.0 * PAD_X; // 964
const CONTENT_TOP: f32 = 151.0;
const CONTENT_BOTTOM: f32 = FRAME_H - 18.0; // 750

// ---- color tokens (0xRRGGBB) ----------------------------------------------
const BG0: u32 = 0x16265a; // radial-gradient stops (center → edge)
const BG1: u32 = 0x0b1336;
const BG2: u32 = 0x070d24;
const PANEL: u32 = 0x0f1c46;
const PANEL_ALT: u32 = 0x0c1838;
const CODE_BG: u32 = 0x091327;
const CAP_TOP: u32 = 0x1d2f63; // letter/number keycap gradient
const CAP_BOT: u32 = 0x16244e;
const CAP_BIGBOT: u32 = 0x162149; // the larger "five keys" keycaps
const MOD_TOP: u32 = 0x16244e; // modifier keycap gradient (ENTER/SHIFT/…)
const MOD_BOT: u32 = 0x101b3c;
const CAP_BORDER: u32 = 0x34529c;
const CHIP_BG: u32 = 0x17274f;
const CHIP_BORDER: u32 = 0x3a5599;
const CARD_BORDER: u32 = 0x23386b;
const ACCENT_BORDER: u32 = 0x2a6f8a;
const DIV: u32 = 0x1a2a55;
const DIV2: u32 = 0x1c2c57;
const HAIRLINE: u32 = 0x233663;
const CYAN: u32 = 0x5cc8e8;
const GREEN: u32 = 0x74d68a;
const AMBER: u32 = 0xf1c46b;
const INK: u32 = 0xe9eefb;
const TITLE: u32 = 0xf3f7ff;
const INKDIM: u32 = 0xb9c4ea;
const INKDIM2: u32 = 0xc3cdee;
const MUTED: u32 = 0x8a99c8;
const MUTED2: u32 = 0x9aa8d4;
const MUTEDF: u32 = 0x7e8cba;
const CODE_GREEN: u32 = 0x7fdc8a;
const BADGE_BG: u32 = 0x16264f;
const BADGE_TXT: u32 = 0x9fb2e6;
const NAV_BORDER: u32 = 0x2f4a8a;
const NAV_BG: u32 = 0x0e1c44;
const NAV_DIV: u32 = 0x2a3a72;
const TAB_OFF: u32 = 0x8392c0;
const KEYCAP_INK: u32 = 0xeaf1ff;
const WORDCAP_INK: u32 = 0xcdd9f5;
const SHADOW_CHIP: u32 = 0x0a1430;
const SHADOW_CAP: u32 = 0x0d1838;
const HOST_SUB: u32 = 0x8a99c8;
const KEY_CHIP_INK: u32 = 0xe2ecff;

/// Which help tab is showing. The order matches the on-screen 1–5 numbering.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HelpTab {
    Start,
    Keyboard,
    Hotkeys,
    Media,
    Settings,
}

impl HelpTab {
    pub const ALL: [HelpTab; 5] = [
        HelpTab::Start,
        HelpTab::Keyboard,
        HelpTab::Hotkeys,
        HelpTab::Media,
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
            HelpTab::Media => "MEDIA & STATE",
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

    // -- background ----------------------------------------------------------
    /// Radial gradient backdrop, lightest at top-center, filling the 4:3 region.
    fn fill_bg(&mut self) {
        // radial-gradient(130% 115% at 50% -12%, BG0, BG1 @55%, BG2)
        let cx = 0.5 * FRAME_W;
        let cy = -0.12 * FRAME_H;
        let rx = 1.30 * FRAME_W;
        let ry = 1.15 * FRAME_H;
        let x0 = self.mx(0.0).floor() as i32;
        let y0 = self.my(0.0).floor() as i32;
        let x1 = self.mx(FRAME_W).ceil() as i32;
        let y1 = self.my(FRAME_H).ceil() as i32;
        for yy in y0..y1 {
            for xx in x0..x1 {
                // back to design space for the gradient math
                let dx = (xx as f32 - self.ox) / self.scale - cx;
                let dy = (yy as f32 - self.oy) / self.scale - cy;
                let d = ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt();
                let c = if d < 0.55 {
                    lerp_rgb(BG0, BG1, d / 0.55)
                } else {
                    lerp_rgb(BG1, BG2, (d - 0.55) / 0.45)
                };
                if xx >= 0 && yy >= 0 && (xx as usize) < self.win_w && (yy as usize) < self.win_h {
                    self.buf[yy as usize * self.win_w + xx as usize] = c;
                }
            }
        }
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

    /// A card: filled panel + 1px border, radius `r`.
    fn card(&mut self, x: f32, y: f32, w: f32, h: f32, bg: u32, border: u32, r: f32) {
        self.round_rect(x, y, w, h, r, bg);
        self.round_border(x, y, w, h, r, 1.0, border);
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

    /// Fill a small solid design-space rect (swatches, dividers, stems).
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

    /// Tracked text centered on `(cx, cy)` (for Silkscreen labels).
    fn text_center_tracked(&mut self, cx: f32, cy: f32, s: &str, id: FontId, px: f32, rgb: u32, track: f32) {
        let w = self.text_w(id, px, s, track);
        self.text_mid_tracked(cx - w / 2.0, cy, s, id, px, rgb, track);
    }

    /// Right-aligned text ending at design `right`, vertically centered on `cy`.
    fn text_right_mid(&mut self, right: f32, cy: f32, s: &str, id: FontId, px: f32, rgb: u32) {
        let w = self.text_w(id, px, s, 0.0);
        self.text_mid(right - w, cy, s, id, px, rgb);
    }

    // -- composite widgets ---------------------------------------------------
    /// An inline key chip centered vertically on `cy`. Returns its width.
    fn chip(&mut self, x: f32, cy: f32, label: &str, min_w: f32, h: f32, px: f32, shadow: bool) -> f32 {
        let tw = self.text_w(MS, px, label, 0.0);
        let w = (tw + 16.0).max(min_w);
        let y = cy - h / 2.0;
        if shadow {
            self.round_rect(x, y + 2.0, w, h, 7.0, SHADOW_CHIP);
        }
        self.round_rect(x, y, w, h, 7.0, CHIP_BG);
        self.round_border(x, y, w, h, 7.0, 1.0, CHIP_BORDER);
        self.text_center(x + w / 2.0, cy, label, MS, px, KEY_CHIP_INK);
        w
    }

    /// The macOS fullscreen chip `⌃⌘F`, drawn with vector mac glyphs (the font
    /// has none) followed by a real `F`. Returns its width.
    fn mac_chip(&mut self, x: f32, cy: f32) -> f32 {
        let px = 11.0;
        let icon = 11.0;
        let gap = 3.0;
        let f_w = self.text_w(MS, px, "F", 0.0);
        let content = icon + gap + icon + gap + f_w;
        let h = 26.0;
        let w = (content + 16.0).max(36.0);
        let y = cy - h / 2.0;
        self.round_rect(x, y + 2.0, w, h, 7.0, SHADOW_CHIP);
        self.round_rect(x, y, w, h, 7.0, CHIP_BG);
        self.round_border(x, y, w, h, 7.0, 1.0, CHIP_BORDER);
        let mut cx = x + (w - content) / 2.0;
        self.icon_ctrl(cx + icon / 2.0, cy, icon, KEY_CHIP_INK);
        cx += icon + gap;
        self.icon_cmd(cx + icon / 2.0, cy, icon, KEY_CHIP_INK);
        cx += icon + gap;
        self.text_mid(cx, cy, "F", MS, px, KEY_CHIP_INK);
        w
    }

    /// A small filled arrow (cursor-diamond glyph) centered on `(cx,cy)`.
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
    /// Plex Mono has no glyph for it, so the mac shortcut is drawn from strokes.
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
}

/// Cursor-diamond arrow direction.
#[derive(Clone, Copy)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

// ===========================================================================
// Chrome: header, tab bar
// ===========================================================================
impl Screen<'_> {
    fn header(&mut self) {
        self.text_tracked(PAD_X, 20.0, "TI-99/4A EMULATOR · HELP", SB, 11.0, CYAN, 1.8);
        self.text_tracked(PAD_X, 36.0, "LIBRE99", SR, 26.0, TITLE, 1.0);
        // right: CLOSE  [F1] / [ESC]
        let cy = 50.0;
        let right = FRAME_W - PAD_X;
        let esc_w = 44.0;
        let f1_w = 38.0;
        let esc_x = right - esc_w;
        self.chip(esc_x, cy, "ESC", esc_w, 28.0, 12.0, true);
        self.text_right_mid(esc_x - 9.0, cy, "/", MR, 12.0, 0x6c7cab);
        let f1_x = esc_x - 9.0 - self.text_w(MR, 12.0, "/", 0.0) - 9.0 - f1_w;
        self.chip(f1_x, cy, "F1", f1_w, 28.0, 12.0, true);
        self.text_right_mid(f1_x - 9.0, cy, "CLOSE", MR, 11.0, MUTED);
    }

    fn tab_bar(&mut self, tab: HelpTab) {
        let cy = 112.0;
        let mut x = PAD_X;
        for &t in &HelpTab::ALL {
            let active = t == tab;
            let badge = (t.index() + 1).to_string();
            // number badge
            let by = cy - 9.0;
            self.round_rect(x, by, 18.0, 18.0, 5.0, BADGE_BG);
            self.round_border(x, by, 18.0, 18.0, 5.0, 1.0, CAP_BORDER);
            self.text_center(x + 9.0, cy, &badge, MB, 10.0, BADGE_TXT);
            let label_x = x + 18.0 + 7.0;
            let color = if active { CYAN } else { TAB_OFF };
            let label_w = self.text_w(SB, 12.0, t.label(), 1.0);
            self.text_mid_tracked(label_x, cy, t.label(), SB, 12.0, color, 1.0);
            let tab_w = 18.0 + 7.0 + label_w;
            if active {
                self.box_fill(x - 2.0, 132.0, tab_w + 16.0, 2.0, CYAN);
            }
            x += tab_w + 28.0;
        }
        // bottom hairline under the whole bar
        self.rule(0.0, 133.0, FRAME_W, HAIRLINE);
        self.nav_pill();
    }

    /// The pinned "TAB cycles · 1–5 jump" navigation hint on the right.
    fn nav_pill(&mut self) {
        let cy = 112.0;
        let h = 34.0;
        let right = FRAME_W - PAD_X;
        // measure contents to size the pill
        let pad = 11.0;
        let gap = 8.0;
        let tab_w = (self.text_w(MS, 11.0, "TAB", 0.0) + 18.0).max(0.0);
        let jump_w = (self.text_w(MS, 11.0, "1–5", 0.0) + 18.0).max(0.0);
        let cycles_w = self.text_w(MM, 11.0, "cycles", 0.0);
        let jumpl_w = self.text_w(MM, 11.0, "jump", 0.0);
        let inner = tab_w + gap + cycles_w + gap + 1.0 + gap + jump_w + gap + jumpl_w;
        let w = inner + 2.0 * pad;
        let x = right - w;
        let y = cy - h / 2.0;
        self.round_rect(x, y, w, h, 9.0, NAV_BG);
        self.round_border(x, y, w, h, 9.0, 1.0, NAV_BORDER);
        let mut cx = x + pad;
        cx += self.chip(cx, cy, "TAB", tab_w, 22.0, 11.0, false) + gap;
        self.text_mid(cx, cy, "cycles", MM, 11.0, BADGE_TXT);
        cx += cycles_w + gap;
        self.box_fill(cx, cy - 7.5, 1.0, 15.0, NAV_DIV);
        cx += 1.0 + gap;
        cx += self.chip(cx, cy, "1–5", jump_w, 22.0, 11.0, false) + gap;
        self.text_mid(cx, cy, "jump", MM, 11.0, BADGE_TXT);
    }

    // -- shared content helpers ---------------------------------------------
    /// A Silkscreen section eyebrow/header.
    fn eyebrow(&mut self, x: f32, top: f32, s: &str, rgb: u32) {
        self.text_tracked(x, top, s, SB, 11.0, rgb, 1.2);
    }

    /// A wrapped rich paragraph of colored runs. Returns the bottom y (design).
    fn paragraph(&mut self, x: f32, top: f32, width: f32, runs: &[Run], px: f32, lh: f32) -> f32 {
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
        let mut penx = 0.0f32; // device, from line start
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
            let space = if pending_space && penx > 0.0 { space_w } else { 0.0 };
            if penx > 0.0 && penx + space + word_w > width_dev {
                line += line_h;
                penx = 0.0;
            } else {
                penx += space;
            }
            pending_space = false;
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
// Tab content
// ===========================================================================
impl Screen<'_> {
    fn tab_start(&mut self) {
        let x = CONTENT_X;
        // lead paragraph
        let bottom = self.paragraph(
            x,
            CONTENT_TOP,
            720.0,
            &[(
                "Launch and you land on the TI title screen. Press any key, choose a cartridge, and play — your host keyboard already speaks TI.",
                MR,
                INKDIM,
            )],
            17.0,
            1.5,
        );
        // three concept cards
        let cards_top = bottom + 18.0;
        let gap = 14.0;
        let cw = (CONTENT_W - 2.0 * gap) / 3.0;
        let ch = 152.0;
        let bodies: [(&str, u32, &[Run]); 3] = [
            (
                "01  JUST TYPE",
                GREEN,
                &[("Character mode maps every keystroke to the TI key that makes the same character — QWERTY, Dvorak or AZERTY alike. SHIFT and FCTN combos are pressed for you.", MR, INKDIM)],
            ),
            (
                "02  LOAD ANYTHING",
                CYAN,
                &[
                    ("Nothing is built in — the console boots bare. Press ", MR, INKDIM),
                    ("F9", MB, INK),
                    (" and pick any ", MR, INKDIM),
                    (".ctg", MB, INK),
                    (" cartridge or ", MR, INKDIM),
                    (".dsk", MB, INK),
                    (" disk image with your system's file chooser.", MR, INKDIM),
                ],
            ),
            (
                "03  KEEP YOUR PLACE",
                AMBER,
                &[
                    ("Your session auto-saves on quit and resumes on launch. ", MR, INKDIM),
                    ("F6", MB, INK),
                    (" saves, ", MR, INKDIM),
                    ("F8", MB, INK),
                    (" loads — one slot, always there.", MR, INKDIM),
                ],
            ),
        ];
        for (i, (eyebrow, color, body)) in bodies.iter().enumerate() {
            let cx = x + i as f32 * (cw + gap);
            self.card(cx, cards_top, cw, ch, PANEL, CARD_BORDER, 13.0);
            self.eyebrow(cx + 20.0, cards_top + 20.0, eyebrow, *color);
            self.paragraph(cx + 20.0, cards_top + 44.0, cw - 40.0, body, 13.0, 1.55);
        }
        // "five keys to start" strip
        let strip_top = cards_top + ch + 18.0;
        let strip_h = 150.0;
        self.card(x, strip_top, CONTENT_W, strip_h, PANEL_ALT, CARD_BORDER, 13.0);
        self.text_tracked(x + 22.0, strip_top + 20.0, "FIVE KEYS TO START", SB, 11.0, CYAN, 1.4);
        let keys = [
            ("F1", "Help"),
            ("F9", "Files"),
            ("F10", "Pause"),
            ("F5", "Reset"),
            ("F11", "Fullscreen"),
        ];
        let key_top = strip_top + 56.0;
        let col_w = (CONTENT_W - 44.0) / keys.len() as f32;
        for (i, (k, a)) in keys.iter().enumerate() {
            let col_cx = x + 22.0 + col_w * (i as f32 + 0.5);
            self.big_keycap(col_cx, key_top, k);
            self.text_center(col_cx, key_top + 56.0, a, MM, 11.0, MUTED2);
        }
    }

    /// A large gradient keycap centered horizontally on `cx`, top at `top`.
    fn big_keycap(&mut self, cx: f32, top: f32, label: &str) {
        let w = 52.0_f32.max(self.text_w(MB, 16.0, label, 0.0) + 26.0);
        let h = 40.0;
        let x = cx - w / 2.0;
        self.round_rect(x, top + 3.0, w, h, 9.0, SHADOW_CHIP);
        self.round_vgrad(x, top, w, h, 9.0, CAP_TOP, CAP_BIGBOT);
        self.round_border(x, top, w, h, 9.0, 1.0, CHIP_BORDER);
        self.text_center(cx, top + h / 2.0, label, MB, 16.0, KEYCAP_INK);
    }

    fn tab_hotkeys(&mut self) {
        let x = CONTENT_X;
        let bottom = self.paragraph(
            x,
            CONTENT_TOP,
            720.0,
            &[("These drive the emulator itself, not the TI. They are ignored while an overlay is open — except the keys that close it.", MR, 0xa9b5de)],
            14.0,
            1.5,
        );
        let groups: [(&str, &[(&str, &str)]); 6] = [
            ("OVERLAYS", &[("F1 / Esc", "Keyboard reference"), (cmd_label::INSPECTOR, "CPU inspector")]),
            ("MEDIA", &[("F9", "Mount media (file dialog)"), ("F2 / F3", "Eject cart / empty DSK1")]),
            ("PLAYBACK", &[("F10", "Pause / resume"), ("F12", "Frame advance"), ("Tab", "Fast-forward (hold)")]),
            ("CONSOLE", &[("F5", "Reset console"), ("F7", "Toggle key layout")]),
            ("STATE", &[("F6", "Save state"), ("F8", "Load state"), (cmd_label::QUIT, "Quit (auto-saves)")]),
            ("DISPLAY & TOOLS", &[("F11", "Fullscreen"), ("⌃⌘F", "Fullscreen (macOS)"), (cmd_label::SCREENSHOT, "Screenshot PNG")]),
        ];
        let top = bottom + 16.0;
        let gap = 13.0;
        let cw = (CONTENT_W - 2.0 * gap) / 3.0;
        let ch = 156.0; // header + up to three rows, sized to content
        for (i, (title, items)) in groups.iter().enumerate() {
            let col = i % 3;
            let row = i / 3;
            let cx = x + col as f32 * (cw + gap);
            let cy = top + row as f32 * (ch + gap);
            self.card(cx, cy, cw, ch, PANEL, CARD_BORDER, 13.0);
            self.eyebrow(cx + 18.0, cy + 16.0, title, CYAN);
            let mut ry = cy + 44.0;
            for (k, a) in items.iter() {
                // The macOS fullscreen combo needs vector ⌃⌘ glyphs (no font glyph).
                let chip_w = if *k == "⌃⌘F" {
                    self.mac_chip(cx + 18.0, ry + 13.0)
                } else {
                    self.chip(cx + 18.0, ry + 13.0, k, 36.0, 26.0, 11.0, true)
                };
                self.text_mid(cx + 18.0 + chip_w + 11.0, ry + 13.0, a, MR, 12.0, INKDIM2);
                self.rule(cx + 18.0, ry + 30.0, cw - 36.0, DIV);
                ry += 38.0;
            }
        }
    }

    fn tab_media(&mut self) {
        let x = CONTENT_X;
        let gap = 14.0;
        let cw = (CONTENT_W - gap) / 2.0;
        let top = CONTENT_TOP;
        // Upper two-column area sized to content, then the full-width save-state
        // bar; both top-aligned (the design doesn't stretch cards to fill).
        let upper_h = 290.0;
        let bar_top = top + upper_h + 16.0;
        let bar_h = 118.0;

        // left: mounting media
        self.card(x, top, cw, upper_h, PANEL, CARD_BORDER, 13.0);
        self.eyebrow(x + 18.0, top + 16.0, "MOUNTING MEDIA — F9", CYAN);
        let pb = self.paragraph(
            x + 18.0,
            top + 38.0,
            cw - 36.0,
            &[("F9 opens your system's file chooser. Pick any .ctg cartridge or .dsk disk image — the extension decides the port (cartridge slot vs. DSK1). The chooser opens where you last mounted from, and the machine pauses while it is up.", MR, 0xa9b5de)],
            12.0,
            1.5,
        );
        let browser = [
            ("F9", "Mount a media file (warm reset)"),
            ("F2", "Eject the cartridge"),
            ("F3", "Empty DSK1"),
        ];
        let mut ry = pb + 6.0;
        for (k, a) in browser.iter() {
            self.chip(x + 18.0, ry + 13.0, k, 88.0, 26.0, 11.0, false);
            self.text_mid(x + 18.0 + 88.0 + 11.0, ry + 13.0, a, MR, 12.0, INKDIM2);
            self.rule(x + 18.0, ry + 30.0, cw - 36.0, DIV);
            ry += 38.0;
        }

        // right column: bare console + screenshots
        let rx = x + cw + gap;
        let qh = upper_h * 0.5 - 7.0;
        self.card(rx, top, cw, qh, PANEL, CARD_BORDER, 13.0);
        self.eyebrow(rx + 18.0, top + 16.0, "NOTHING EMBEDDED", CYAN);
        self.paragraph(
            rx + 18.0,
            top + 38.0,
            cw - 36.0,
            &[("The binary carries no cartridge or disk images — the console boots bare until you mount something. F2/F3 take you back to the bare console / empty drive. Every media change warm-boots, and the window title shows what is mounted.", MR, 0xa9b5de)],
            12.0,
            1.5,
        );
        let sh_top = top + qh + 14.0;
        let sh_h = upper_h - qh - 14.0;
        self.card(rx, sh_top, cw, sh_h, PANEL_ALT, ACCENT_BORDER, 13.0);
        self.eyebrow(rx + 18.0, sh_top + 16.0, "SCREENSHOTS", GREEN);
        self.paragraph(
            rx + 18.0,
            sh_top + 38.0,
            cw - 36.0,
            &[(cmd_label::SCREENSHOT_HYPHEN, MB, INK), (" saves a clean 256×192 PNG (no HUD) to your data folder.", MR, INKDIM)],
            12.0,
            1.5,
        );
        self.code_line(rx + 18.0, sh_top + sh_h - 34.0, cw - 36.0, "~/.libre99/screenshots/");

        // bottom: save state & auto-resume (accent, full width)
        self.card(x, bar_top, CONTENT_W, bar_h, PANEL_ALT, ACCENT_BORDER, 13.0);
        self.eyebrow(x + 20.0, bar_top + 16.0, "SAVE STATE & AUTO-RESUME", GREEN);
        self.paragraph(
            x + 20.0,
            bar_top + 38.0,
            CONTENT_W - 40.0,
            &[
                ("F6", MB, INK),
                (" snapshots the whole machine — RAM, VRAM, GROM, cartridge ROM and mounted disks (with written sectors) — to one portable file. ", MR, INKDIM),
                ("F8", MB, INK),
                (" restores it. The session also auto-saves on quit and resumes on launch. One slot, shared by all four.", MR, INKDIM),
            ],
            12.0,
            1.55,
        );
        self.code_line(x + 20.0, bar_top + bar_h - 32.0, 320.0, "~/.libre99/savestate.ti99");
    }

    /// A monospaced code/path chip on a dark background.
    fn code_line(&mut self, x: f32, y: f32, w: f32, s: &str) {
        let h = 24.0;
        self.round_rect(x, y, w, h, 7.0, CODE_BG);
        self.round_border(x, y, w, h, 7.0, 1.0, DIV2);
        self.text_mid(x + 10.0, y + h / 2.0, s, MM, 11.0, CODE_GREEN);
    }

    fn tab_settings(&mut self) {
        let x = CONTENT_X;
        let top = CONTENT_TOP;
        // command line card (full width)
        let cli = [
            ("--cartridge <path>", "Mount a .ctg cartridge image (e.g. libre99asm output)"),
            ("--disk <path>", "Insert a .dsk disk image into DSK1"),
            ("--system-rom <path>", "Boot a console ROM in place of the clean-room default"),
            ("--system-grom <path>", "Boot a system GROM in place of the clean-room default"),
            ("--disk-dsr <path>", "Install a disk DSR ROM in place of the clean-room default"),
            ("--scale <n>", "Integer window scale, 1–8"),
            ("--fullscreen", "Start fullscreen"),
            ("--log-level <lvl>", "error / warn / info / debug / trace"),
            ("--help, -h", "Print usage and exit"),
        ];
        let cli_h = 250.0;
        self.card(x, top, CONTENT_W, cli_h, PANEL, CARD_BORDER, 13.0);
        self.eyebrow(x + 18.0, top + 16.0, "COMMAND LINE", CYAN);
        let mut ry = top + 42.0;
        let row_h = (cli_h - 56.0) / cli.len() as f32;
        for (f, e) in cli.iter() {
            self.text_mid(x + 18.0, ry + row_h / 2.0, f, MS, 12.0, AMBER);
            self.text_mid(x + 218.0, ry + row_h / 2.0, e, MR, 12.0, INKDIM2);
            self.rule(x + 18.0, ry + row_h, CONTENT_W - 36.0, DIV);
            ry += row_h;
        }
        // preferences (1.5fr) + where files live (1fr)
        let lower_top = top + cli_h + 14.0;
        let lower_h = CONTENT_BOTTOM - lower_top;
        let gap = 14.0;
        let pw = (CONTENT_W - gap) / 2.5 * 1.5;
        let fw = CONTENT_W - gap - pw;
        self.card(x, lower_top, pw, lower_h, PANEL, CARD_BORDER, 13.0);
        self.eyebrow(x + 18.0, lower_top + 16.0, "PREFERENCES — TOML", CYAN);
        self.text(x + 18.0, lower_top + 36.0, "~/.libre99/libre99.toml", MR, 11.0, MUTEDF);
        let prefs = [
            ("last_cartridge", "\"…\"", "auto-written: resumed cartridge path"),
            ("last_disk", "\"…\"", "auto-written: resumed DSK1 path"),
            ("browser_dir", "\"…\"", "auto-written: where the F9 chooser opens"),
            ("window_scale", "3", "upscale of 256×192 (1–8)"),
            ("fullscreen", "false", "start fullscreen"),
            ("audio_enabled", "true", "enable audio output"),
            ("audio_volume", "0.8", "output volume 0.0–1.0"),
            ("key_layout", "\"character\"", "character or positional"),
            ("log_level", "\"info\"", "verbosity error…trace"),
        ];
        let mut py = lower_top + 56.0;
        let prow = (lower_h - 70.0) / prefs.len() as f32;
        for (k, v, d) in prefs.iter() {
            self.text_mid(x + 18.0, py + prow / 2.0, k, MS, 12.0, INK);
            self.text_mid(x + 156.0, py + prow / 2.0, v, MS, 12.0, CODE_GREEN);
            self.text_mid(x + 240.0, py + prow / 2.0, d, MR, 11.0, MUTED2);
            self.rule(x + 18.0, py + prow, pw - 36.0, DIV);
            py += prow;
        }
        // where files live
        let fx = x + pw + gap;
        self.card(fx, lower_top, fw, lower_h, PANEL_ALT, CARD_BORDER, 13.0);
        self.eyebrow(fx + 18.0, lower_top + 16.0, "WHERE FILES LIVE", CYAN);
        self.text(fx + 18.0, lower_top + 38.0, "~/.libre99/", MS, 12.0, CODE_GREEN);
        let files = [
            ("libre99.toml", "preferences (commented)"),
            ("libre99.log", "run log (appended)"),
            ("savestate.ti99", "the single save state"),
            ("screenshots/", cmd_label::SCREENSHOT_PNGS),
        ];
        let mut fy = lower_top + 60.0;
        let frow = (lower_h - 74.0) / files.len() as f32;
        for (p, d) in files.iter() {
            self.box_fill(fx + 22.0, fy + 2.0, 1.0, frow - 4.0, NAV_DIV);
            self.text(fx + 36.0, fy + 4.0, p, MS, 12.0, 0xdfeaff);
            self.text(fx + 36.0, fy + 20.0, d, MR, 11.0, MUTED);
            fy += frow;
        }
    }
}

// ---- keyboard tab ----------------------------------------------------------
/// A keycap's central legend / corner marks.
enum Cap {
    /// A character key: big glyph, optional amber SHIFT mark above, optional
    /// green FCTN mark below.
    Glyph {
        main: &'static str,
        shift: Option<&'static str>,
        fctn: Option<Mark>,
    },
    /// A wide modifier/word key (ENTER, SHIFT, SPACE BAR, …) drawn in Silkscreen,
    /// with an optional small sub-label (e.g. `=L-CTRL`).
    Word {
        text: &'static str,
        sub: Option<&'static str>,
        px: f32,
    },
}

enum Mark {
    Txt(&'static str),
    Arrow(Dir),
}

fn g(main: &'static str, shift: Option<&'static str>, fctn: Option<Mark>) -> (f32, Cap) {
    (1.0, Cap::Glyph { main, shift, fctn })
}

impl Screen<'_> {
    fn tab_keyboard(&mut self) {
        let x = CONTENT_X;
        let card_top = CONTENT_TOP;
        let card_h = 332.0;
        self.card(x, card_top, CONTENT_W, card_h, PANEL_ALT, CARD_BORDER, 14.0);

        let inner_x = x + 16.0;
        let inner_w = CONTENT_W - 32.0;
        // legend row
        self.text_tracked(inner_x, card_top + 14.0, "TI-99/4A KEYBOARD MAP", SB, 11.0, CYAN, 1.3);
        // right legend swatches
        let lg_cy = card_top + 20.0;
        let amber_lbl = "SHIFT symbol";
        let aw = self.text_w(MM, 11.0, amber_lbl, 0.0);
        let ax = x + CONTENT_W - 16.0 - aw;
        self.round_rect(ax - 16.0, lg_cy - 5.0, 10.0, 10.0, 3.0, AMBER);
        self.text_mid(ax, lg_cy, amber_lbl, MM, 11.0, MUTED2);
        let green_lbl = "FCTN function";
        let gw = self.text_w(MM, 11.0, green_lbl, 0.0);
        let gx2 = ax - 16.0 - 15.0 - gw;
        self.round_rect(gx2 - 16.0, lg_cy - 5.0, 10.0, 10.0, 3.0, GREEN);
        self.text_mid(gx2, lg_cy, green_lbl, MM, 11.0, MUTED2);

        // FCTN edit-function strip aligned to the 11 number-key columns
        let keys_top = card_top + 51.0;
        let strip = ["DEL", "INS", "ERASE", "CLEAR", "BEGIN", "PROC'D", "AID", "REDO", "BACK", "", "QUIT"];
        let gap = 6.0;
        let unit = (inner_w - 10.0 * gap) / 11.0;
        for (i, label) in strip.iter().enumerate() {
            if label.is_empty() {
                continue;
            }
            let cxx = inner_x + i as f32 * (unit + gap) + unit / 2.0;
            self.text_center(cxx, card_top + 42.0, label, MB, 9.0, GREEN);
        }

        // rows
        let up = || Some(Mark::Arrow(Dir::Up));
        let dn = || Some(Mark::Arrow(Dir::Down));
        let lf = || Some(Mark::Arrow(Dir::Left));
        let rt = || Some(Mark::Arrow(Dir::Right));
        let num: Vec<(f32, Cap)> = vec![
            g("1", Some("!"), None), g("2", Some("@"), None), g("3", Some("#"), None),
            g("4", Some("$"), None), g("5", Some("%"), None), g("6", Some("^"), None),
            g("7", Some("&"), None), g("8", Some("*"), None), g("9", Some("("), None),
            g("0", Some(")"), None), g("=", Some("+"), None),
        ];
        let qwer: Vec<(f32, Cap)> = vec![
            g("Q", None, None), g("W", None, Some(Mark::Txt("~"))), g("E", None, up()),
            g("R", None, Some(Mark::Txt("["))), g("T", None, Some(Mark::Txt("]"))), g("Y", None, None),
            g("U", None, Some(Mark::Txt("_"))), g("I", None, Some(Mark::Txt("?"))), g("O", None, Some(Mark::Txt("'"))),
            g("P", None, Some(Mark::Txt("\""))), g("/", Some("-"), None),
        ];
        let asdf: Vec<(f32, Cap)> = vec![
            g("A", None, Some(Mark::Txt("|"))), g("S", None, lf()), g("D", None, rt()),
            g("F", None, Some(Mark::Txt("{"))), g("G", None, Some(Mark::Txt("}"))), g("H", None, None),
            g("J", None, None), g("K", None, None), g("L", None, None), g(";", Some(":"), None),
            (1.9, Cap::Word { text: "ENTER", sub: None, px: 12.0 }),
        ];
        let zxcv: Vec<(f32, Cap)> = vec![
            (1.9, Cap::Word { text: "SHIFT", sub: None, px: 12.0 }),
            g("Z", None, Some(Mark::Txt("\\"))), g("X", None, dn()), g("C", None, Some(Mark::Txt("`"))),
            g("V", None, None), g("B", None, None), g("N", None, None), g("M", None, None),
            g(",", Some("<"), None), g(".", Some(">"), None),
            (1.9, Cap::Word { text: "SHIFT", sub: None, px: 12.0 }),
        ];
        let modr: Vec<(f32, Cap)> = vec![
            (1.7, Cap::Word { text: "ALPHA LOCK", sub: None, px: 11.0 }),
            (1.3, Cap::Word { text: "CTRL", sub: Some("=L-CTRL"), px: 13.0 }),
            (1.3, Cap::Word { text: "FCTN", sub: Some("=L-ALT"), px: 13.0 }),
            (6.0, Cap::Word { text: "SPACE BAR", sub: None, px: 12.0 }),
        ];

        let row_h = 50.0;
        self.key_row(inner_x, inner_w, keys_top, row_h, 0.0, &num);
        self.key_row(inner_x, inner_w, keys_top + 56.0, row_h, 0.5, &qwer);
        self.key_row(inner_x, inner_w, keys_top + 112.0, row_h, 0.9, &asdf);
        self.key_row(inner_x, inner_w, keys_top + 168.0, row_h, 0.0, &zxcv);
        self.key_row(inner_x, inner_w, keys_top + 224.0, 42.0, 0.0, &modr);

        // three support cards
        let sup_top = card_top + card_h + 14.0;
        let sup_h = CONTENT_BOTTOM - sup_top;
        let sgap = 14.0;
        let scw = (CONTENT_W - 2.0 * sgap) / 3.0;

        // typing modes
        self.card(x, sup_top, scw, sup_h, PANEL, CARD_BORDER, 13.0);
        self.eyebrow(x + 16.0, sup_top + 16.0, "TYPING MODES", CYAN);
        self.text(x + 16.0, sup_top + 40.0, "Character", MS, 12.0, INK);
        let chw = self.text_w(MS, 12.0, "Character ", 0.0);
        self.text(x + 16.0 + chw, sup_top + 40.0, "(default)", MS, 12.0, GREEN);
        self.text(x + 16.0, sup_top + 56.0, "Type normally; combos synthesized for you.", MR, 12.0, 0xa9b5de);
        self.text(x + 16.0, sup_top + 80.0, "Positional", MS, 12.0, INK);
        self.text(x + 16.0, sup_top + 96.0, "Maps by physical key position. Best for games.", MR, 12.0, 0xa9b5de);
        self.rule(x + 16.0, sup_top + sup_h - 36.0, scw - 32.0, DIV2);
        let chip_w = self.chip(x + 16.0, sup_top + sup_h - 18.0, "F7", 32.0, 26.0, 12.0, false);
        self.text_mid(x + 16.0 + chip_w + 9.0, sup_top + sup_h - 18.0, "toggle modes", MR, 12.0, 0xa9b5de);

        // modifier keys
        let mx2 = x + scw + sgap;
        self.card(mx2, sup_top, scw, sup_h, PANEL, CARD_BORDER, 13.0);
        self.eyebrow(mx2 + 16.0, sup_top + 16.0, "MODIFIER KEYS", CYAN);
        let mods = [("SHIFT", "Left Shift"), ("CTRL", "Left Ctrl"), ("FCTN", "Left Alt / Option")];
        let mut my2 = sup_top + 40.0;
        for (k, a) in mods {
            self.text_mid(mx2 + 16.0, my2 + 11.0, k, MS, 12.0, INK);
            self.text_right_mid(mx2 + scw - 16.0, my2 + 11.0, a, MR, 12.0, 0xa9b5de);
            self.rule(mx2 + 16.0, my2 + 26.0, scw - 32.0, DIV2);
            my2 += 33.0;
        }

        // cursor & joystick
        let cx3 = x + 2.0 * (scw + sgap);
        self.card(cx3, sup_top, scw, sup_h, PANEL, CARD_BORDER, 13.0);
        self.eyebrow(cx3 + 16.0, sup_top + 16.0, "CURSOR & JOYSTICK", GREEN);
        let cj = [
            ("FCTN E/S/D/X", "TI cursor diamond"),
            ("Arrows", "Joystick 1 move"),
            ("R-Alt", "Joystick 1 fire"),
        ];
        let mut cy3 = sup_top + 40.0;
        for (k, a) in cj {
            self.text_mid(cx3 + 16.0, cy3 + 11.0, k, MS, 12.0, INK);
            self.text_right_mid(cx3 + scw - 16.0, cy3 + 11.0, a, MR, 12.0, 0xa9b5de);
            self.rule(cx3 + 16.0, cy3 + 26.0, scw - 32.0, DIV2);
            cy3 += 31.0;
        }
        self.text(cx3 + 16.0, cy3 + 6.0, "Arrows drive the joystick — not", MR, 11.0, MUTEDF);
        self.text(cx3 + 16.0, cy3 + 20.0, "the TI cursor.", MR, 11.0, MUTEDF);
    }

    /// Lay out one keyboard row with an optional leading spacer (in key units).
    fn key_row(&mut self, x: f32, w: f32, y: f32, h: f32, lead: f32, caps: &[(f32, Cap)]) {
        let total_units: f32 = lead + caps.iter().map(|(u, _)| *u).sum::<f32>();
        let gaps = caps.len() as f32 - 1.0 + if lead > 0.0 { 1.0 } else { 0.0 };
        let gap = 6.0;
        let unit = (w - gaps * gap) / total_units;
        let mut cx = x;
        if lead > 0.0 {
            cx += lead * unit + gap;
        }
        for (units, cap) in caps {
            let cw = units * unit;
            self.keycap(cx, y, cw, h, cap);
            cx += cw + gap;
        }
    }

    fn keycap(&mut self, x: f32, y: f32, w: f32, h: f32, cap: &Cap) {
        match cap {
            Cap::Glyph { main, shift, fctn } => {
                self.round_rect(x, y + 2.0, w, h, 7.0, SHADOW_CAP);
                self.round_vgrad(x, y, w, h, 7.0, CAP_TOP, CAP_BOT);
                self.round_border(x, y, w, h, 7.0, 1.0, CAP_BORDER);
                self.text_center(x + w / 2.0, y + h / 2.0, main, MB, 18.0, KEYCAP_INK);
                if let Some(s) = shift {
                    self.text_center(x + w / 2.0, y + 11.0, s, MB, 12.0, AMBER);
                }
                match fctn {
                    Some(Mark::Txt(t)) => self.text_center(x + w / 2.0, y + h - 11.0, t, MB, 12.0, GREEN),
                    Some(Mark::Arrow(d)) => self.arrow(x + w / 2.0, y + h - 11.0, *d, 9.0, GREEN),
                    None => {}
                }
            }
            Cap::Word { text, sub, px } => {
                self.round_rect(x, y + 2.0, w, h, 7.0, SHADOW_CAP);
                self.round_vgrad(x, y, w, h, 7.0, MOD_TOP, MOD_BOT);
                self.round_border(x, y, w, h, 7.0, 1.0, CAP_BORDER);
                let id = if (*text == "CTRL" || *text == "FCTN") && sub.is_some() { MB } else { SB };
                let cy = if sub.is_some() { y + h / 2.0 - 4.0 } else { y + h / 2.0 };
                if id == SB {
                    self.text_center_tracked(x + w / 2.0, cy, text, SB, *px, WORDCAP_INK, 1.0);
                } else {
                    self.text_center(x + w / 2.0, cy, text, id, *px, WORDCAP_INK);
                }
                if let Some(s) = sub {
                    self.text_center(x + w / 2.0, y + h - 8.0, s, SB, 7.0, HOST_SUB);
                }
            }
        }
    }
}

/// Render the help overlay for `tab` into the window buffer (`win_w × win_h`),
/// at native resolution, filling the centered 4:3 region.
pub fn render(fonts: &mut Fonts, buf: &mut [u32], win_w: usize, win_h: usize, tab: HelpTab) {
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
    s.fill_bg();
    s.header();
    s.tab_bar(tab);
    match tab {
        HelpTab::Start => s.tab_start(),
        HelpTab::Keyboard => s.tab_keyboard(),
        HelpTab::Hotkeys => s.tab_hotkeys(),
        HelpTab::Media => s.tab_media(),
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
