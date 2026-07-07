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

//! # TMS9918A — the Video Display Processor (VDP)
//!
//! The VDP draws the screen. It has its own private **16 KiB of VRAM** that the
//! CPU cannot address directly: instead the CPU pokes the VDP through four
//! memory-mapped ports, and the VDP autonomously scans VRAM 60 times a second to
//! produce a 256×192 picture in up to 15 colors plus transparent.
//!
//! ## CPU ports (in the console memory map)
//!
//! | Address | Operation                                                        |
//! |---------|------------------------------------------------------------------|
//! | `>8800` | read VRAM data (returns a prefetched byte, then auto-increments) |
//! | `>8802` | read the status register (clears the interrupt flag)             |
//! | `>8C00` | write VRAM data (then auto-increments)                           |
//! | `>8C02` | write the address counter or a control register (two-byte seq)   |
//!
//! ### The two-byte control sequence (`>8C02`)
//!
//! Software writes two bytes. The **first** byte is the low 8 address bits. The
//! **second** byte's top two bits select the operation, the rest carry the high
//! 6 address bits / register data:
//!
//! ```text
//! second byte  meaning
//! 00aaaaaa     set up VRAM address for READING  (and prefetch the first byte)
//! 01aaaaaa     set up VRAM address for WRITING
//! 100000rrr→   write VRAM register r with the first byte    (bit7 set)
//! ```
//! The 14-bit address counter auto-increments (mod 16 KiB) after every data
//! access. A flip-flop tracks "first vs second byte"; reading the status register
//! resets it, which is how software recovers if it loses sync.
//!
//! ## Registers R0–R7
//!
//! ```text
//! R0  ......M3 EXT     M3 = bit1
//! R1  16 BL IE M1 M2 . SZ MG   (BL=display enable, IE=interrupt enable,
//!                               M1/M2 = mode bits, SZ=sprite size, MG=magnify)
//! R2  name table base            = (R2 & 0x0F) << 10
//! R3  color table base           =  R3        << 6    (special in Graphics II)
//! R4  pattern generator base     = (R4 & 0x07) << 11  (special in Graphics II)
//! R5  sprite attribute base      = (R5 & 0x7F) << 7
//! R6  sprite pattern base        = (R6 & 0x07) << 11
//! R7  foreground (text) | backdrop color   (high nibble | low nibble)
//! ```
//!
//! Mode bits `M1 M2 M3` select Graphics I (`000`), Graphics II / bitmap (`001`),
//! Multicolor (`010`), or Text (`100`, no sprites).
//!
//! ## Status, interrupt, and frame timing
//!
//! The status register's top bits are **F** (frame/vblank flag, bit 7), **5S**
//! (a 5th sprite appeared on some line, bit 6), and **C** (two sprites
//! collided, bit 5), with the offending 5th-sprite number in the low bits. At the
//! end of each active frame F is set; if interrupts are enabled (`R1` bit 5) this
//! requests the CPU's level-1 interrupt. The console's interrupt handler reads the
//! status register, which **clears F** and releases the interrupt — so reading
//! status is the acknowledgement, and forgetting to clear F here would cause an
//! interrupt storm.
//!
//! The chip has no framebuffer of its own: it re-reads VRAM as the beam sweeps,
//! so software freely changes tables mid-frame and the change shows from the
//! next scanline down. We model this beam: the machine calls
//! [`render_line`](Vdp::render_line) (or, headless,
//! [`evaluate_line`](Vdp::evaluate_line) — same sprite flags, no pixels) for
//! each of the 192 active lines as CPU time advances, then [`vblank`](Vdp::vblank)
//! at the end of active display, with the 5S/C flags latching on the very line
//! the beam meets them. A mid-frame status read that clears C can therefore see
//! it re-latch on a later line of the same frame — exactly as on hardware.
//!
//! ## Sprites
//!
//! Up to 32 sprites, four bytes each (`Y, X, pattern, early-clock|color`). `Y` is
//! the line *above* the sprite, so a sprite is drawn at `Y+1`; `Y = >D0` ends the
//! active list. At most four sprites may appear on a scanline — a fifth sets the
//! 5S flag and is not drawn. Sprites can be 8×8 or 16×16 and optionally doubled.

/// Display width in pixels.
pub const WIDTH: usize = 256;
/// Display height in pixels.
pub const HEIGHT: usize = 192;

/// The standard TMS9918A emulator palette as `0x00RRGGBB`. Index 0 is
/// transparent; we render it as black (it shows only where the backdrop is 0).
/// These exact values are shared by MAME and js99er.
pub const PALETTE: [u32; 16] = [
    0x0000_0000, // 0  transparent (shown as black)
    0x0000_0000, // 1  black
    0x0021_C842, // 2  medium green
    0x005E_DC78, // 3  light green
    0x0054_55ED, // 4  dark blue
    0x007D_76FC, // 5  light blue
    0x00D4_524D, // 6  dark red
    0x0042_EBF5, // 7  cyan
    0x00FC_5554, // 8  medium red
    0x00FF_7978, // 9  light red
    0x00D4_C154, // 10 dark yellow
    0x00E6_CE80, // 11 light yellow
    0x0021_B03B, // 12 dark green
    0x00C9_5BBA, // 13 magenta
    0x00CC_CCCC, // 14 gray
    0x00FF_FFFF, // 15 white
];

/// Status flag bits.
const ST_F: u8 = 0x80; // frame / vblank
const ST_5S: u8 = 0x40; // fifth sprite
const ST_C: u8 = 0x20; // sprite coincidence

pub struct Vdp {
    /// 16 KiB of video RAM, scanned to produce the picture.
    vram: Box<[u8; 0x4000]>,
    /// Write-only registers R0–R7.
    registers: [u8; 8],
    /// Status register (F / 5S / C / fifth-sprite number).
    status: u8,
    /// 14-bit VRAM address counter.
    address: u16,
    /// Low byte held between the two control-port writes.
    control_latch: u8,
    /// Flip-flop: are we expecting the second control byte?
    expecting_second: bool,
    /// Read-ahead buffer for `>8800` reads.
    read_buffer: u8,
    /// The picture accumulated scanline-by-scanline as the machine walks the
    /// beam through the frame ([`render_line`](Self::render_line));
    /// [`copy_frame`](Self::copy_frame) hands it to the frontend. Presentation
    /// state only — deliberately **not** serialized: saves always happen at
    /// frame boundaries, and after a load the machine's first render repaints
    /// the whole frame from the restored VRAM.
    frame: Box<[u32]>,
}

impl Default for Vdp {
    fn default() -> Self {
        Self::new()
    }
}

impl Vdp {
    pub fn new() -> Self {
        Vdp {
            vram: Box::new([0u8; 0x4000]),
            registers: [0; 8],
            status: 0,
            address: 0,
            control_latch: 0,
            expecting_second: false,
            read_buffer: 0,
            frame: vec![0u32; WIDTH * HEIGHT].into_boxed_slice(),
        }
    }

    // ---- inspection helpers ---------------------------------------------
    pub fn register(&self, n: usize) -> u8 {
        self.registers[n & 7]
    }
    pub fn vram(&self, addr: u16) -> u8 {
        self.vram[(addr & 0x3FFF) as usize]
    }
    /// Write a VRAM byte directly (diagnostics/tests), bypassing the address
    /// port — the mutable mirror of `vram`.
    pub fn set_vram(&mut self, addr: u16, value: u8) {
        self.vram[(addr & 0x3FFF) as usize] = value;
    }

    // =====================================================================
    // CPU port interface
    // =====================================================================

    /// Read a VRAM data byte (`>8800`): return the read-ahead buffer, then refill
    /// it from the (auto-incrementing) address counter.
    pub fn read_data(&mut self) -> u8 {
        let value = self.read_buffer;
        self.read_buffer = self.vram[self.address as usize];
        self.address = (self.address + 1) & 0x3FFF;
        self.expecting_second = false;
        value
    }

    /// Write a VRAM data byte (`>8C00`) and auto-increment.
    pub fn write_data(&mut self, byte: u8) {
        self.vram[self.address as usize] = byte;
        self.read_buffer = byte;
        self.address = (self.address + 1) & 0x3FFF;
        self.expecting_second = false;
    }

    /// Write the control port (`>8C02`): see the two-byte protocol above.
    pub fn write_control(&mut self, byte: u8) {
        if !self.expecting_second {
            self.control_latch = byte;
            self.expecting_second = true;
        } else {
            self.expecting_second = false;
            if byte & 0x80 != 0 {
                // Register write.
                self.registers[(byte & 0x07) as usize] = self.control_latch;
            } else {
                self.address = (((byte & 0x3F) as u16) << 8) | self.control_latch as u16;
                if byte & 0x40 == 0 {
                    // Read setup: prefetch the first byte so >8800 returns it.
                    self.read_buffer = self.vram[self.address as usize];
                    self.address = (self.address + 1) & 0x3FFF;
                }
            }
        }
    }

    /// Read the status register (`>8802`): returns the current status, then
    /// clears all three top flags — F (acknowledging the interrupt), 5S, and C —
    /// keeping only the low five bits, and resets the control latch. Classic99
    /// `Tiemul.cpp`: `VDPS &= 0x1f; // top flags are cleared on read (tested on
    /// hardware)`.
    pub fn read_status(&mut self) -> u8 {
        let value = self.status;
        self.status &= 0x1F;
        self.expecting_second = false;
        value
    }

    /// Called once per frame at the start of vertical blanking: set the frame
    /// flag (which may request an interrupt).
    pub fn vblank(&mut self) {
        self.status |= ST_F;
    }

    /// True when the VDP is requesting the CPU's interrupt: the frame flag is set
    /// **and** interrupts are enabled (`R1` bit 5).
    pub fn interrupt_pending(&self) -> bool {
        self.status & ST_F != 0 && self.registers[1] & 0x20 != 0
    }

    /// Serialize the VDP — its 16 KiB of VRAM, the registers, and the port
    /// latches — into a save state.
    pub(crate) fn save_state(&self, w: &mut crate::state::StateWriter) {
        w.raw(&self.vram[..]);
        w.raw(&self.registers);
        w.u8(self.status);
        w.u16(self.address);
        w.u8(self.control_latch);
        w.bool(self.expecting_second);
        w.u8(self.read_buffer);
    }

    /// Restore the VDP from a save state.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
    ) -> Result<(), crate::state::StateError> {
        r.fill(&mut self.vram[..])?;
        r.fill(&mut self.registers)?;
        self.status = r.u8()?;
        self.address = r.u16()? & 0x3FFF;
        self.control_latch = r.u8()?;
        self.expecting_second = r.bool()?;
        self.read_buffer = r.u8()?;
        Ok(())
    }

    // =====================================================================
    // Table base helpers
    // =====================================================================

    fn name_base(&self) -> usize {
        ((self.registers[2] as usize) & 0x0F) << 10
    }
    fn color_base(&self) -> usize {
        (self.registers[3] as usize) << 6
    }
    fn pattern_base(&self) -> usize {
        ((self.registers[4] as usize) & 0x07) << 11
    }
    fn sprite_attr_base(&self) -> usize {
        ((self.registers[5] as usize) & 0x7F) << 7
    }
    fn sprite_pattern_base(&self) -> usize {
        ((self.registers[6] as usize) & 0x07) << 11
    }
    fn backdrop(&self) -> u32 {
        PALETTE[(self.registers[7] & 0x0F) as usize]
    }

    /// Mode selector from M1/M2/M3.
    fn mode(&self) -> Mode {
        let m1 = self.registers[1] & 0x10 != 0;
        let m2 = self.registers[1] & 0x08 != 0;
        let m3 = self.registers[0] & 0x02 != 0;
        match (m1, m2, m3) {
            (false, false, false) => Mode::Graphics1,
            (false, false, true) => Mode::Graphics2,
            (false, true, false) => Mode::Multicolor,
            (true, false, false) => Mode::Text,
            // Illegal combinations fall back to Graphics I, as on real hardware
            // they degrade to a text-like display; Graphics I is the safest.
            _ => Mode::Graphics1,
        }
    }

    // =====================================================================
    // Rasterizer
    // =====================================================================
    //
    // The rasterizer is scanline-based, mirroring the chip: every producer
    // renders exactly one line from the **live** VRAM and registers, so a
    // mid-frame write (a new name-table byte, a base-register change, even a
    // mode switch) takes effect on the next line the beam draws — never
    // retroactively. The machine walks the beam ([`render_line`] /
    // [`evaluate_line`] per line, [`vblank`] at line [`HEIGHT`]); whole-frame
    // entry points ([`render`], [`evaluate_sprites`]) are just loops over the
    // same per-line code, so the two paths cannot drift apart.
    //
    // [`render_line`]: Self::render_line
    // [`evaluate_line`]: Self::evaluate_line
    // [`vblank`]: Self::vblank
    // [`render`]: Self::render
    // [`evaluate_sprites`]: Self::evaluate_sprites

    /// Latch a line's sprite findings into the status register: only ever
    /// **sets** the 5S/C bits — the sole clearer is a status read
    /// ([`read_status`](Self::read_status)); hardware re-raising C on a later
    /// line after a mid-frame read cleared it is exactly right. The
    /// fifth-sprite number is latched only when 5S is newly raised. (The
    /// fuller hardware lifecycle of the number field — e.g. the real-time
    /// last-scanned-sprite count when no fifth exists, which Miner 2049er
    /// reads — is out of scope; see the evaluation's P2.3 and ROADMAP.)
    fn latch_sprite_flags(&mut self, fifth: Option<u8>, coincidence: bool) {
        if coincidence {
            self.status |= ST_C;
        }
        if let Some(n) = fifth {
            if self.status & ST_5S == 0 {
                self.status = (self.status & !0x1F) | ST_5S | (n & 0x1F);
            }
        }
    }

    /// Evaluate scanline `y`'s sprite occupancy for status only (no pixels):
    /// the 4-per-line limit latches 5S, pattern overlap latches C. This is
    /// what the chip does whether or not anyone watches the picture — the
    /// machine calls it for every active line when no frontend has asked for
    /// pixels, so headless runs keep hardware-exact flags at zero pixel cost.
    /// When the display is blanked (BL=0) the chip isn't scanning sprites, so
    /// evaluation is skipped and previously latched flags persist until read;
    /// text mode has no sprite plane.
    pub fn evaluate_line(&mut self, y: usize) {
        if y >= HEIGHT {
            return;
        }
        if self.registers[1] & 0x40 == 0 {
            return; // display blanked: no sprite scan; flags persist
        }
        if matches!(self.mode(), Mode::Text) {
            return; // text mode has no sprite plane
        }
        let (fifth, coincidence) = self.sprite_line(y, None);
        self.latch_sprite_flags(fifth, coincidence);
    }

    /// Render scanline `y` (0..[`HEIGHT`]) into the internal frame from the
    /// live VRAM/registers, latching that line's 5S/C sprite flags exactly as
    /// [`evaluate_line`](Self::evaluate_line) would. The machine calls this
    /// once per active line as the beam crosses it; out-of-range lines are
    /// ignored (the core never panics on bad input).
    pub fn render_line(&mut self, y: usize) {
        if y >= HEIGHT {
            return;
        }
        let mut line = [0u32; WIDTH];

        // When the display is disabled (BL=0) the line is pure backdrop and
        // the chip does no sprite scan (flags persist until read).
        if self.registers[1] & 0x40 == 0 {
            line.fill(self.backdrop());
        } else {
            let mode = self.mode();
            match mode {
                Mode::Graphics1 => self.graphics1_line(y, &mut line),
                Mode::Graphics2 => self.graphics2_line(y, &mut line),
                Mode::Multicolor => self.multicolor_line(y, &mut line),
                Mode::Text => self.text_line(y, &mut line),
            }
            if !matches!(mode, Mode::Text) {
                // text mode has no sprite plane
                let (fifth, coincidence) = self.sprite_line(y, Some(&mut line));
                self.latch_sprite_flags(fifth, coincidence);
            }
        }

        self.frame[y * WIDTH..(y + 1) * WIDTH].copy_from_slice(&line);
    }

    /// The whole-frame sprite scan: [`evaluate_line`](Self::evaluate_line)
    /// over every scanline. Kept for direct-drive callers (tests, probes);
    /// the machine itself evaluates per line as the beam advances. Set-only
    /// latching makes a second evaluation in the same frame harmless.
    pub fn evaluate_sprites(&mut self) {
        for y in 0..HEIGHT {
            self.evaluate_line(y);
        }
    }

    /// Render a whole frame **now** — [`render_line`](Self::render_line) over
    /// every scanline from the current VRAM — into `fb` (length
    /// `WIDTH*HEIGHT`, `0x00RRGGBB`), latching sprite flags along the way.
    /// This is the on-demand path for direct callers (tests, screenshots of a
    /// machine that has never run beam-style); the beam-accurate picture a
    /// running machine accumulates is fetched with
    /// [`copy_frame`](Self::copy_frame) instead.
    pub fn render(&mut self, fb: &mut [u32]) {
        debug_assert!(fb.len() >= WIDTH * HEIGHT);
        for y in 0..HEIGHT {
            self.render_line(y);
        }
        fb[..WIDTH * HEIGHT].copy_from_slice(&self.frame);
    }

    /// Copy the accumulated frame out (length `WIDTH*HEIGHT`, `0x00RRGGBB`) —
    /// the picture as the beam last painted it, scanline by scanline.
    pub fn copy_frame(&self, fb: &mut [u32]) {
        debug_assert!(fb.len() >= WIDTH * HEIGHT);
        fb[..WIDTH * HEIGHT].copy_from_slice(&self.frame);
    }

    /// Graphics I, one scanline: 32 cells, an 8×8 pattern per character, one
    /// foreground/background color pair per group of 8 characters.
    fn graphics1_line(&self, y: usize, out: &mut [u32; WIDTH]) {
        let name = self.name_base();
        let pat = self.pattern_base();
        let col = self.color_base();
        let backdrop = self.backdrop();
        let (row, line) = (y >> 3, y & 7);
        for cell in 0..32 {
            let ch = self.vram[name + row * 32 + cell] as usize;
            let color = self.vram[col + (ch >> 3)];
            let fg = PALETTE[(color >> 4) as usize];
            let bg = PALETTE[(color & 0x0F) as usize];
            let fg = if color >> 4 == 0 { backdrop } else { fg };
            let bg = if color & 0x0F == 0 { backdrop } else { bg };
            let bits = self.vram[pat + ch * 8 + line];
            for px in 0..8 {
                let on = bits & (0x80 >> px) != 0;
                out[cell * 8 + px] = if on { fg } else { bg };
            }
        }
    }

    /// Graphics II (bitmap), one scanline: the name table indexes into three
    /// independent 2 KiB thirds of the pattern and color tables, so every one
    /// of the 768 cells can have a unique 8×8 bitmap with per-row colors.
    /// R3/R4 act as a 1-bit "which half" select plus an AND-mask over the
    /// table-relative address.
    fn graphics2_line(&self, y: usize, out: &mut [u32; WIDTH]) {
        let name = self.name_base();
        let backdrop = self.backdrop();
        // Pattern (R4): select bit 0x04 picks the half; the low bits mask.
        let pat_base = ((self.registers[4] as usize) & 0x04) << 11;
        let color_mask = (((self.registers[3] as usize) & 0x7F) << 6) | 0x3F;
        let pat_mask = (((self.registers[4] as usize) & 0x03) << 11) | (color_mask & 0x7FF);
        // Color (R3): select bit 0x80.
        let col_base = ((self.registers[3] as usize) & 0x80) << 6;
        let (row, line) = (y >> 3, y & 7);
        let third = (row / 8) * 0x800;
        for cell in 0..32 {
            let ch = self.vram[name + row * 32 + cell] as usize;
            let rel = third + ch * 8 + line;
            let bits = self.vram[pat_base + (rel & pat_mask)];
            let color = self.vram[col_base + (rel & color_mask)];
            let fg = PALETTE[(color >> 4) as usize];
            let bg = PALETTE[(color & 0x0F) as usize];
            let fg = if color >> 4 == 0 { backdrop } else { fg };
            let bg = if color & 0x0F == 0 { backdrop } else { bg };
            for px in 0..8 {
                let on = bits & (0x80 >> px) != 0;
                out[cell * 8 + px] = if on { fg } else { bg };
            }
        }
    }

    /// Multicolor, one scanline: each 8×8 cell is a 2×2 grid of 4×4 solid
    /// color blocks; the block colors come from two pattern bytes selected by
    /// the cell's vertical position within each group of four rows.
    fn multicolor_line(&self, y: usize, out: &mut [u32; WIDTH]) {
        let name = self.name_base();
        let pat = self.pattern_base();
        let backdrop = self.backdrop();
        let (row, line) = (y >> 3, y & 7);
        for cell in 0..32 {
            let ch = self.vram[name + row * 32 + cell] as usize;
            // Top half of the cell uses byte (row&3)*2, bottom half +1.
            let byte = self.vram[pat + ch * 8 + (row & 3) * 2 + (line >> 2)];
            for px in 0..8 {
                let nibble = if px < 4 { byte >> 4 } else { byte & 0x0F };
                let c = if nibble == 0 {
                    backdrop
                } else {
                    PALETTE[nibble as usize]
                };
                out[cell * 8 + px] = c;
            }
        }
    }

    /// Text mode, one scanline: 40 columns of 6×8 characters, a single
    /// foreground/background pair from R7, and no sprites. Only the top 6 bits
    /// of each pattern byte are shown. Columns 0–239 are the text area; the
    /// remaining 16 pixels are the backdrop border.
    fn text_line(&self, y: usize, out: &mut [u32; WIDTH]) {
        let name = self.name_base();
        let pat = self.pattern_base();
        // Color 0 is transparent on both halves of R7: a transparent text
        // foreground shows the backdrop, exactly like a transparent background.
        let fg_idx = self.registers[7] >> 4;
        let fg = if fg_idx == 0 {
            self.backdrop()
        } else {
            PALETTE[fg_idx as usize]
        };
        let bg_idx = self.registers[7] & 0x0F;
        let bg = if bg_idx == 0 {
            self.backdrop()
        } else {
            PALETTE[bg_idx as usize]
        };
        // Border (columns 240–255) first — fill the whole line, then overdraw
        // the 40 text cells.
        out.fill(self.backdrop());
        let (row, line) = (y >> 3, y & 7);
        for cell in 0..40 {
            let ch = self.vram[name + row * 40 + cell] as usize;
            let bits = self.vram[pat + ch * 8 + line];
            for px in 0..6 {
                let on = bits & (0x80 >> px) != 0;
                out[cell * 6 + px] = if on { fg } else { bg };
            }
        }
    }

    /// The sprite plane for a single scanline, shared by every producer
    /// ([`render_line`](Self::render_line) paints,
    /// [`evaluate_line`](Self::evaluate_line) only observes): honors the
    /// 4-per-line limit,
    /// detects the fifth-sprite and coincidence conditions, and — when `fb`
    /// is `Some` (one line of `WIDTH` pixels) — draws the sprite pixels.
    /// Lower-numbered sprites have priority, so a line's sprites are drawn
    /// from last to first. The attribute table (including the `>D0`
    /// terminator) is re-read every line, as the chip does. Does NOT touch
    /// `self.status`; callers latch via
    /// [`latch_sprite_flags`](Self::latch_sprite_flags).
    ///
    /// Returns `(fifth, coincidence)`: the number of the sprite found fifth
    /// on this scanline (it and later ones are not drawn), and whether two
    /// sprites asserted the same pixel. Coincidence is detected on pattern
    /// bits regardless of color — a transparent (color 0) sprite
    /// participates in the overlap mask but paints nothing (Classic99: "Even
    /// transparent sprites get drawn into the collision buffer").
    fn sprite_line(&self, y: usize, mut fb: Option<&mut [u32; WIDTH]>) -> (Option<u8>, bool) {
        let attr = self.sprite_attr_base();
        let pat = self.sprite_pattern_base();
        let big = self.registers[1] & 0x02 != 0; // 16x16 vs 8x8
        let mag = self.registers[1] & 0x01 != 0; // 2x magnify
        let sprite_px = if big { 16 } else { 8 };
        let screen_px = sprite_px * if mag { 2 } else { 1 };

        // Where the active sprite list ends (Y == >D0).
        let mut active = 32;
        for i in 0..32 {
            if self.vram[attr + i * 4] == 0xD0 {
                active = i;
                break;
            }
        }

        // Which sprites cover this scanline (in number order).
        let mut on_line: Vec<usize> = Vec::with_capacity(8);
        for i in 0..active {
            let top = sprite_top(self.vram[attr + i * 4]);
            if (y as i32) >= top && (y as i32) < top + screen_px as i32 {
                on_line.push(i);
            }
        }
        // The fifth (and beyond) is reported and not drawn.
        let mut fifth: Option<u8> = None;
        if on_line.len() > 4 {
            fifth = Some(on_line[4] as u8);
            on_line.truncate(4);
        }
        // Track sprite overlap with a per-scanline opacity mask.
        let mut coincidence = false;
        let mut opaque = [false; WIDTH];
        // Draw lowest-priority first so sprite 0 ends up on top.
        for &i in on_line.iter().rev() {
            let top = sprite_top(self.vram[attr + i * 4]);
            let sx = self.vram[attr + i * 4 + 1] as i32;
            let pattern = self.vram[attr + i * 4 + 2] as usize;
            let flags = self.vram[attr + i * 4 + 3];
            let color = flags & 0x0F;
            let early = flags & 0x80 != 0;
            let x0 = sx - if early { 32 } else { 0 };
            let line_in_sprite = ((y as i32) - top) / if mag { 2 } else { 1 };
            // 16x16 sprites use four 8x8 quadrants: pattern index low 2 bits
            // are ignored; columns are stored as the second pair.
            let base_pat = if big { pattern & 0xFC } else { pattern };
            for col in 0..sprite_px {
                let px = x0 + (col * if mag { 2 } else { 1 }) as i32;
                // Resolve the pattern bit for this (col,line_in_sprite).
                let (byte_index, bit) = if big {
                    let quad = (col / 8) * 2 + (line_in_sprite / 8) as usize;
                    let row = (line_in_sprite % 8) as usize;
                    ((base_pat + quad) * 8 + row, 0x80u8 >> (col % 8))
                } else {
                    (base_pat * 8 + line_in_sprite as usize, 0x80u8 >> col)
                };
                if self.vram[(pat + byte_index) & 0x3FFF] & bit == 0 {
                    continue;
                }
                for sub in 0..if mag { 2 } else { 1 } {
                    let fx = px + sub;
                    if fx < 0 || fx >= WIDTH as i32 {
                        continue;
                    }
                    let fx = fx as usize;
                    if opaque[fx] {
                        coincidence = true; // two sprites overlapped here
                    }
                    opaque[fx] = true;
                    if color != 0 {
                        if let Some(fb) = fb.as_deref_mut() {
                            fb[fx] = PALETTE[color as usize];
                        }
                    }
                }
            }
        }
        (fifth, coincidence)
    }
}

/// Top screen line of a sprite from its attribute-table Y byte. Y names the
/// line *above* the sprite (top = Y+1), wrapping at 8 bits so a sprite slides
/// in from the top edge (Y = `>FF` ⇒ first row on line 0) instead of vanishing.
/// The `> 225` threshold reproduces Classic99 (`tivdp.cpp`: `if (yy>225)
/// yy-=256; // fade in from top`), which is hardware-plausible but not
/// hardware-pinned. Do not "simplify" to `& 0xFF` modulo math — that is not
/// equivalent for 32-pixel-tall (magnified 16×16) sprites near Y ≈ 224.
fn sprite_top(y_attr: u8) -> i32 {
    let top = y_attr as i32 + 1;
    if top > 225 {
        top - 256
    } else {
        top
    }
}

#[derive(Clone, Copy)]
enum Mode {
    Graphics1,
    Graphics2,
    Multicolor,
    Text,
}
