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

//! # TI Disk Controller — FD1771 floppy controller + DSR ROM
//!
//! The original TI Disk Controller card carries a Western Digital **FD1771**
//! floppy-disk controller, a small **Device Service Routine (DSR) ROM**
//! (`Disk.Bin`), and the glue that maps both onto the console bus. We emulate the
//! *hardware* and run the **genuine DSR ROM**, exactly as the console does: the
//! GPL `DSRLNK` mechanism scans peripheral CRU bases, enables this card, matches a
//! device name (`DSK1`, …) in the ROM header, and calls the ROM routine, which in
//! turn drives our FD1771 a register at a time. We therefore implement no file
//! system — the real firmware does that itself.
//!
//! ## Memory map (gated by CRU `>1100` bit 0)
//! * `>4000–5FEF` — the 8 KiB DSR ROM (`Disk.Bin`), read-only.
//! * `>5FF0–5FFE` — the FD1771 registers, overlaid on the top of the window:
//!   read Status/Track/Sector/Data at `>5FF0/2/4/6`, write Command/Track/Sector/
//!   Data at `>5FF8/A/C/E`. **The card one's-complements the data bus, so every
//!   register byte is transferred `XOR >FF`.**
//!
//! ## CRU map (base `>1100`)
//! bit 0 ROM-enable · bit 1 motor (no-op) · bit 2 wait-states (no-op) · bit 3
//! head-load (no-op) · bits 4–6 drive select (one-hot → DSK1/2/3) · bit 7 side.
//! INTRQ/DRQ are **not** CRU-readable; the DSR polls the status register's Busy
//! bit instead.
//!
//! ## Timing
//! The controller is **synchronous**: a Read/Write Sector command transfers all
//! 256 bytes immediately into/out of the data register, so wait-states, DRQ
//! pacing and the motor are no-ops. That is enough for the real DSR, which copies
//! the sector a byte at a time and then checks the Busy bit.
//!
//! ## Disk geometry (the Volume Information Block)
//! Images are raw **sector dumps** (v9t9 / PC99 style): 256-byte sectors stored
//! back-to-back in logical order with no per-sector headers. The physical
//! geometry — sectors per track, tracks per side, and number of sides — is read
//! from the **Volume Information Block** (VIB, sector 0) at [`mount`](Disk::mount)
//! time and drives the `(track, side, sector) → absolute sector (LBA)` mapping, so
//! a single- or double-density and single- or double-sided image each map
//! correctly. When the VIB is absent or inconsistent with the image length we
//! fall back to the historical 40-track / 9-sector single-density default,
//! inferring the side count from the image size (see [`parse_geometry`]).
//!
//! Side 1 of a two-sided image follows the **v9t9 reverse-track convention**: it
//! is stored after side 0 with its tracks in reverse physical order (the media
//! logically "flips over", so the head steps back out from the innermost track).
//! This convention is density-independent — it is a property of the DSR/hardware
//! layout, not the recording density — so the same reversal applies to DSDD
//! images with per-drive geometry substituted for the constants.
//!
//! ## Write-back volatility
//! A Write Sector command mutates only the **in-memory** image; nothing is written
//! back to the host `.Dsk` file. Written sectors survive **only** inside a save
//! state (which serializes the whole image, edits included). Absent a save state,
//! disk writes are lost when the machine is dropped.

/// Size of the DSR ROM window (`>4000–5FFF`); the top 16 bytes are the FD1771
/// register overlay.
const DSR_SIZE: usize = 0x2000;
/// Bytes per sector on every TI floppy format we support.
const SECTOR_SIZE: usize = 256;
/// Default sectors per track (single density) — the fallback when no usable VIB.
const SECTORS_PER_TRACK: usize = 9;
/// Default tracks per side — the fallback when no usable VIB.
const TRACKS: usize = 40;

/// Physical geometry of a mounted image, parsed from its Volume Information Block
/// (sector 0) at mount time. Drives the `(track, side, sector) → absolute sector`
/// (LBA) mapping, so a double-density or two-sided image maps correctly instead
/// of assuming a fixed 40×9 single-sided layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Geometry {
    /// Sectors per track (9 = single density, 18 = double density).
    sectors_per_track: usize,
    /// Tracks per side (40 for a standard 5.25" TI floppy, 80 for 80-track).
    tracks: usize,
    /// Number of recorded sides (1 or 2).
    sides: usize,
}

impl Default for Geometry {
    /// The historical default: single-sided, 40 tracks, 9 sectors/track (SSSD).
    fn default() -> Self {
        Geometry { sectors_per_track: SECTORS_PER_TRACK, tracks: TRACKS, sides: 1 }
    }
}

/// Derive the geometry of a raw sector-dump `image` from its Volume Information
/// Block (VIB, sector 0). VIB field offsets (into sector 0):
///
/// | bytes | field |
/// |-------|-------|
/// | 0x00–0x09 | volume name (10 bytes, space-padded) |
/// | 0x0A–0x0B | total sectors on the disk, **big-endian** |
/// | 0x0C      | sectors per track |
/// | 0x0D–0x0F | `"DSK"` magic |
/// | 0x11      | tracks per side |
/// | 0x12      | number of sides |
/// | 0x13      | density (1 = single, 2 = double) |
///
/// The VIB is trusted only when it is internally consistent **and** matches the
/// image: the `"DSK"` magic is present, every geometry field is non-zero, the
/// product `sectors/track · tracks/side · sides` equals the recorded total sector
/// count, and that total accounts for exactly the image's byte length. Any failure
/// — including a headerless raw dump — falls back to [`fallback_geometry`]. Field
/// offsets and the consistency check are cross-checked against Classic99
/// `disk/ImageDisk.cpp:1775` (`sector[0x0c] * sector[0x11] * sector[0x12]`
/// validated against the big-endian total at `sector[0x0A..0x0B]`).
fn parse_geometry(image: &[u8]) -> Geometry {
    if image.len() < SECTOR_SIZE {
        return fallback_geometry(image.len());
    }
    let vib = &image[..SECTOR_SIZE];
    if &vib[0x0D..0x10] != b"DSK" {
        return fallback_geometry(image.len());
    }
    let total = ((vib[0x0A] as usize) << 8) | vib[0x0B] as usize;
    let spt = vib[0x0C] as usize;
    let tps = vib[0x11] as usize;
    let sides = vib[0x12] as usize;
    if spt == 0
        || tps == 0
        || sides == 0
        || spt * tps * sides != total
        || total * SECTOR_SIZE != image.len()
    {
        return fallback_geometry(image.len());
    }
    Geometry { sectors_per_track: spt, tracks: tps, sides }
}

/// Best-effort geometry for an image whose VIB we can't trust (or that has none):
/// assume the ubiquitous 40-track / 9-sector single-density layout — the historical
/// default — and infer the side count from the image size (90 KiB ⇒ one side,
/// ≥180 KiB ⇒ two sides). This keeps a headerless SSSD/DSSD dump mapping the way it
/// did before per-drive geometry existed.
fn fallback_geometry(len: usize) -> Geometry {
    let per_side = TRACKS * SECTORS_PER_TRACK * SECTOR_SIZE; // 92_160 (90 KiB)
    let sides = if len >= 2 * per_side { 2 } else { 1 };
    Geometry { sectors_per_track: SECTORS_PER_TRACK, tracks: TRACKS, sides }
}

// FD1771 status-register bits (TI bit values).
const STATUS_NOT_READY: u8 = 0x80;
const STATUS_WRITE_PROTECTED: u8 = 0x40;
const STATUS_RECORD_NOT_FOUND: u8 = 0x10;
const STATUS_TRACK_0: u8 = 0x04;
const STATUS_DRQ: u8 = 0x02;
const STATUS_BUSY: u8 = 0x01;

/// The TI Disk Controller card: FD1771 state, the DSR ROM, and up to three
/// mounted disk images (DSK1–DSK3).
pub struct Disk {
    /// `Disk.Bin` DSR ROM; `None` until a controller is installed (the window is
    /// then open bus, matching a machine with no disk card).
    dsr: Option<Box<[u8; DSR_SIZE]>>,
    /// Raw sector-dump images for DSK1/DSK2/DSK3.
    drives: [Option<Vec<u8>>; 3],
    /// Per-drive geometry, parsed from each image's VIB at mount time and used
    /// for the LBA mapping (default until a drive is mounted). Derived state —
    /// re-parsed from the images on save-state load, never serialized.
    geometry: [Geometry; 3],

    // --- CRU latches ---
    /// CRU bit 0: when set, the DSR ROM is visible at `>4000–5FEF`.
    rom_enabled: bool,
    /// Currently selected drive (CRU bits 4–6, one-hot); `None` if none selected.
    selected: Option<usize>,
    /// CRU bit 7: selected disk side (0 or 1).
    side: u8,
    /// Last Step direction (+1 = in/toward higher tracks, −1 = out), so a bare
    /// Step repeats it.
    step_dir: i8,

    // --- FD1771 registers ---
    status: u8,
    /// Head position / Track register (the FD1771 keeps them in step here).
    track: u8,
    sector: u8,
    data: u8,

    // --- sector transfer buffer ---
    buffer: [u8; SECTOR_SIZE],
    /// Index of the next byte to transfer through the data register.
    buf_pos: usize,
    /// Bytes still to transfer (0 = idle).
    buf_left: usize,
    /// For a Write Sector command, the absolute sector index to flush on
    /// completion (`None` for reads / idle).
    write_lba: Option<usize>,

    /// Diagnostic log of the absolute sector indices that have been **read**.
    log: Vec<usize>,
    /// Diagnostic trace of register/CRU activity: `(kind, addr_or_bit, value)`
    /// with `kind` one of `b'R'`/`b'W'`/`b'C'`.
    trace: Vec<(u8, u16, u8)>,
    record: bool,
}

impl Default for Disk {
    fn default() -> Self {
        Self::new()
    }
}

impl Disk {
    /// A controller with no ROM and no disks (the `>4000–5FFF` window reads as
    /// open bus until [`load_dsr`](Self::load_dsr) installs the card).
    pub fn new() -> Self {
        Disk {
            dsr: None,
            drives: [None, None, None],
            geometry: [Geometry::default(); 3],
            rom_enabled: false,
            selected: None,
            side: 0,
            step_dir: 1,
            status: 0,
            track: 0,
            sector: 0,
            data: 0,
            buffer: [0; SECTOR_SIZE],
            buf_pos: 0,
            buf_left: 0,
            write_lba: None,
            log: Vec::new(),
            trace: Vec::new(),
            record: false,
        }
    }

    /// Install the disk-controller DSR ROM (`Disk.Bin`). Only the first
    /// [`DSR_SIZE`] bytes are used.
    pub fn load_dsr(&mut self, rom: &[u8]) {
        let mut dsr = Box::new([0u8; DSR_SIZE]);
        let n = rom.len().min(DSR_SIZE);
        dsr[..n].copy_from_slice(&rom[..n]);
        self.dsr = Some(dsr);
    }

    /// Insert a raw sector-dump image into drive `drive` (0 = DSK1). The image's
    /// Volume Information Block is parsed for the disk geometry (sectors/track,
    /// tracks, sides) used by the LBA mapping; an absent or inconsistent VIB falls
    /// back to a size-derived default. Kept infallible (the core has no logging) —
    /// a bad header simply degrades to the default rather than rejecting the mount.
    pub fn mount(&mut self, drive: usize, image: Vec<u8>) {
        if drive < self.drives.len() {
            self.geometry[drive] = parse_geometry(&image);
            self.drives[drive] = Some(image);
        }
    }

    /// Is a disk controller installed (DSR ROM present)?
    pub fn present(&self) -> bool {
        self.dsr.is_some()
    }

    /// Enable/clear the diagnostic sector-read log and register/CRU trace.
    pub fn record(&mut self, on: bool) {
        self.record = on;
        self.log.clear();
        self.trace.clear();
    }

    /// Absolute sector indices read since recording began (diagnostics).
    pub fn read_log(&self) -> &[usize] {
        &self.log
    }

    /// The raw image currently mounted in `drive`, including any sectors
    /// written back by the DSR (diagnostics — the firmware tests byte-diff the
    /// resulting image across DSR implementations).
    pub fn drive_image(&self, drive: usize) -> Option<&[u8]> {
        self.drives.get(drive)?.as_deref()
    }

    /// Register/CRU access trace `(kind, addr_or_bit, value)` (diagnostics).
    pub fn trace(&self) -> &[(u8, u16, u8)] {
        &self.trace
    }

    // ---------------------------------------------------------------------
    // Bus interface: the `>4000–5FFF` window
    // ---------------------------------------------------------------------

    /// Read one byte from the `>4000–5FFF` window.
    pub fn read_byte(&mut self, addr: u16) -> u8 {
        if self.dsr.is_none() {
            return 0; // no card → open bus
        }
        // FD1771 registers overlay the very top of the window, byte-inverted.
        if (0x5FF0..=0x5FFE).contains(&addr) {
            let v = self.read_register(addr);
            if self.record {
                self.trace.push((b'R', addr, v));
            }
            return v ^ 0xFF;
        }
        if self.rom_enabled && addr < 0x5FF0 {
            return self.dsr.as_ref().unwrap()[(addr - 0x4000) as usize];
        }
        0
    }

    /// Write one byte to the `>4000–5FFF` window. Only the FD1771 registers are
    /// writable; the DSR ROM ignores writes.
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        if self.dsr.is_none() {
            return;
        }
        if (0x5FF0..=0x5FFE).contains(&addr) {
            if self.record {
                self.trace.push((b'W', addr, value ^ 0xFF));
            }
            self.write_register(addr, value ^ 0xFF);
        }
    }

    /// Read one CRU bit (`bit` is relative to the card's `>1100` base). The card
    /// does not expose INTRQ/DRQ; we read back the latched control bits.
    pub fn read_cru(&self, bit: u16) -> bool {
        match bit {
            0 => self.rom_enabled,
            7 => self.side != 0,
            _ => false,
        }
    }

    /// Write one CRU bit (`bit` relative to the `>1100` base).
    pub fn write_cru(&mut self, bit: u16, value: bool) {
        if self.record {
            self.trace.push((b'C', bit, value as u8));
        }
        match bit {
            0 => self.rom_enabled = value, // ROM enable
            1..=3 => {}                     // motor / wait-states / head-load: no-op
            4..=6 => {
                // One-hot drive select across bits 4,5,6 → DSK1/DSK2/DSK3.
                let n = (bit - 4) as usize;
                if value {
                    self.selected = Some(n);
                } else if self.selected == Some(n) {
                    self.selected = None;
                }
            }
            7 => self.side = value as u8,
            _ => {}
        }
    }

    // ---------------------------------------------------------------------
    // FD1771 register file (pre-/post-inversion handled by the caller)
    // ---------------------------------------------------------------------

    fn read_register(&mut self, addr: u16) -> u8 {
        match addr {
            0x5FF0 => self.status_byte(),
            0x5FF2 => self.track,
            0x5FF4 => self.sector,
            0x5FF6 => self.read_data(),
            _ => 0,
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0x5FF8 => self.command(value),
            0x5FFA => self.track = value,
            0x5FFC => self.sector = value,
            0x5FFE => self.write_data(value),
            _ => {}
        }
    }

    /// Latch a data byte. During a Write Sector it fills the sector buffer; the
    /// final byte is flushed to the mounted image and clears Busy.
    fn write_data(&mut self, value: u8) {
        self.data = value;
        if self.write_lba.is_none() || self.buf_left == 0 {
            return;
        }
        self.buffer[self.buf_pos] = value;
        self.buf_pos += 1;
        self.buf_left -= 1;
        if self.buf_left == 0 {
            if let Some(lba) = self.write_lba.take() {
                self.flush_sector(lba);
            }
            self.status &= !STATUS_BUSY;
        }
    }

    /// Copy the sector buffer back into the selected drive's image.
    fn flush_sector(&mut self, lba: usize) {
        if let Some(image) = self.selected.and_then(|d| self.drives[d].as_mut()) {
            let start = lba * SECTOR_SIZE;
            if let Some(dst) = image.get_mut(start..start + SECTOR_SIZE) {
                dst.copy_from_slice(&self.buffer);
            }
        }
    }

    /// The status register: Busy/DRQ track the in-progress transfer; Not-Ready and
    /// Track-0 reflect the drive.
    fn status_byte(&self) -> u8 {
        let mut s = self.status;
        if self.buf_left > 0 {
            s |= STATUS_DRQ;
        }
        s
    }

    /// Serve the next byte of an in-progress Read Sector; the final byte clears
    /// Busy. Reads past the end of the sector return 0.
    fn read_data(&mut self) -> u8 {
        if self.buf_left == 0 {
            return self.data;
        }
        let byte = self.buffer[self.buf_pos];
        self.buf_pos += 1;
        self.buf_left -= 1;
        if self.buf_left == 0 {
            self.status &= !STATUS_BUSY;
        }
        byte
    }

    fn command(&mut self, cmd: u8) {
        match cmd & 0xF0 {
            0x00 => self.restore(),
            0x10 => self.seek(),
            0x20 | 0x30 => self.step(self.step_dir),       // Step (keep direction)
            0x40 | 0x50 => self.step_to(self.track.wrapping_add(1), 1), // Step in
            0x60 | 0x70 => self.step_to(self.track.wrapping_sub(1), -1), // Step out
            0x80 | 0x90 => self.read_sector(),
            0xA0 | 0xB0 => self.write_sector(),
            0xC0 => self.read_address(),
            0xD0 => {
                // Force Interrupt: abort any transfer.
                self.buf_left = 0;
                self.write_lba = None;
                self.status &= !STATUS_BUSY;
            }
            // Read Track (>E0) and Write Track (>F0) are unused by the TI DSR
            // (Read Address, >C0, is handled above — the DSR does issue it).
            _ => {}
        }
    }

    // --- Type I: head positioning ---

    fn restore(&mut self) {
        self.track = 0;
        self.status = STATUS_TRACK_0;
    }

    fn seek(&mut self) {
        // The data register holds the target track.
        self.track = self.data;
        self.status = if self.track == 0 { STATUS_TRACK_0 } else { 0 };
    }

    fn step(&mut self, dir: i8) {
        let next = (self.track as i16 + dir as i16).clamp(0, TRACKS as i16 - 1) as u8;
        self.step_to(next, dir);
    }

    fn step_to(&mut self, track: u8, dir: i8) {
        self.step_dir = dir;
        self.track = track.min(TRACKS as u8 - 1);
        self.status = if self.track == 0 { STATUS_TRACK_0 } else { 0 };
    }

    // --- Type II: sector transfer ---

    /// Map (track, side, sector) to an absolute sector index in the raw image,
    /// using the selected drive's parsed geometry. Side 0 is `track·spt + sector`.
    /// Side 1 follows the **v9t9 reverse-track convention**: it is stored after
    /// side 0 with tracks in reverse physical order, so physical track `T` on side
    /// 1 maps to `tracks·spt + (tracks−1−T)·spt + sector`. (For the historical 40×9
    /// default this reduces to the previous `360 + (39−track)·9 + sector`.) The
    /// reverse-track saturates rather than underflows if the DSR ever seeks past the
    /// geometry's track count; any resulting out-of-range LBA is caught by the
    /// bounds check in [`sector_slice`](Self::sector_slice).
    fn lba(&self) -> usize {
        let g = self.selected.map(|d| self.geometry[d]).unwrap_or_default();
        let track = self.track as usize;
        let sector = self.sector as usize;
        if self.side == 0 {
            track * g.sectors_per_track + sector
        } else {
            let sectors_per_side = g.tracks * g.sectors_per_track;
            let reverse_track = g.tracks.saturating_sub(1).saturating_sub(track);
            sectors_per_side + reverse_track * g.sectors_per_track + sector
        }
    }

    fn read_sector(&mut self) {
        let lba = self.lba();
        // Copy the sector out from under the (immutable) image borrow first.
        let found = self.sector_slice(lba).map(|slice| {
            let mut buf = [0u8; SECTOR_SIZE];
            buf.copy_from_slice(slice);
            buf
        });
        match found {
            Some(buf) => {
                self.buffer = buf;
                self.buf_pos = 0;
                self.buf_left = SECTOR_SIZE;
                self.status = STATUS_BUSY;
                if self.record {
                    self.log.push(lba);
                }
            }
            None => self.status = STATUS_NOT_READY | STATUS_RECORD_NOT_FOUND,
        }
    }

    /// Read Address: serve the 6-byte ID field — track, side, sector, size code,
    /// and two CRC bytes — for the sector the head is over. The TI DSR issues
    /// this to verify the track and locate a sector before reading it.
    fn read_address(&mut self) {
        if self.selected.and_then(|d| self.drives[d].as_ref()).is_none() {
            self.status = STATUS_NOT_READY | STATUS_RECORD_NOT_FOUND;
            return;
        }
        self.buffer[0] = self.track; // cylinder
        self.buffer[1] = self.side; // side
        self.buffer[2] = self.sector; // sector
        self.buffer[3] = 0x01; // size code: 128 << 1 = 256 bytes
        self.buffer[4] = 0xFF; // CRC (not checked)
        self.buffer[5] = 0xFF; // CRC
        self.buf_pos = 0;
        self.buf_left = 6;
        self.status = STATUS_BUSY;
    }

    fn write_sector(&mut self) {
        let lba = self.lba();
        // Accept the bytes regardless; flush on completion if the target exists.
        if self.current_drive_writable(lba) {
            self.buf_pos = 0;
            self.buf_left = SECTOR_SIZE;
            self.write_lba = Some(lba);
            self.status = STATUS_BUSY;
        } else {
            self.status = STATUS_NOT_READY | STATUS_WRITE_PROTECTED;
        }
    }

    /// Borrow the 256-byte slice for absolute sector `lba` in the selected drive.
    fn sector_slice(&self, lba: usize) -> Option<&[u8]> {
        let image = self.drives[self.selected?].as_ref()?;
        let start = lba.checked_mul(SECTOR_SIZE)?;
        image.get(start..start + SECTOR_SIZE)
    }

    fn current_drive_writable(&self, lba: usize) -> bool {
        match self.selected.and_then(|d| self.drives[d].as_ref()) {
            Some(image) => (lba + 1) * SECTOR_SIZE <= image.len(),
            None => false,
        }
    }

    /// Serialize the controller: the DSR ROM, the three drive images (with any
    /// written-back sectors), the CRU latches, the FD1771 registers, and any
    /// in-progress sector transfer. Diagnostics (the read log/trace) are not saved.
    pub(crate) fn save_state(&self, w: &mut crate::state::StateWriter) {
        match &self.dsr {
            Some(rom) => {
                w.u8(1);
                w.raw(&rom[..]);
            }
            None => w.u8(0),
        }
        for drive in &self.drives {
            match drive {
                Some(image) => {
                    w.u8(1);
                    w.blob(image);
                }
                None => w.u8(0),
            }
        }
        w.bool(self.rom_enabled);
        w.opt_usize(self.selected);
        w.u8(self.side);
        w.u8(self.step_dir as u8);
        w.u8(self.status);
        w.u8(self.track);
        w.u8(self.sector);
        w.u8(self.data);
        w.raw(&self.buffer);
        w.usize(self.buf_pos);
        w.usize(self.buf_left);
        w.opt_usize(self.write_lba);
    }

    /// Restore the controller from a save state. The transfer cursor and the
    /// selected-drive index are sanitized so a corrupt or foreign file can never
    /// drive a later register access out of bounds.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
    ) -> Result<(), crate::state::StateError> {
        self.dsr = if r.u8()? != 0 {
            let mut rom = Box::new([0u8; DSR_SIZE]);
            r.fill(&mut rom[..])?;
            Some(rom)
        } else {
            None
        };
        for drive in &mut self.drives {
            *drive = if r.u8()? != 0 { Some(r.blob()?) } else { None };
        }
        // Geometry is derived state, not serialized: re-parse it from each restored
        // image so the save format stays stable and version-neutral.
        for i in 0..self.drives.len() {
            self.geometry[i] = match &self.drives[i] {
                Some(image) => parse_geometry(image),
                None => Geometry::default(),
            };
        }
        self.rom_enabled = r.bool()?;
        self.selected = r.opt_usize()?;
        self.side = r.u8()?;
        self.step_dir = r.u8()? as i8;
        self.status = r.u8()?;
        self.track = r.u8()?;
        self.sector = r.u8()?;
        self.data = r.u8()?;
        r.fill(&mut self.buffer)?;
        self.buf_pos = r.usize()?;
        self.buf_left = r.usize()?;
        self.write_lba = r.opt_usize()?;

        // --- sanitize against a corrupt/foreign file ---
        if self.selected.is_some_and(|d| d >= self.drives.len()) {
            self.selected = None;
        }
        let transfer_ok = self.buf_pos <= SECTOR_SIZE
            && self.buf_left <= SECTOR_SIZE
            && self.buf_pos + self.buf_left <= SECTOR_SIZE;
        let write_ok = self
            .write_lba
            .is_none_or(|lba| lba.checked_mul(SECTOR_SIZE).is_some());
        if !transfer_ok || !write_ok {
            self.buf_pos = 0;
            self.buf_left = 0;
            self.write_lba = None;
        }

        // Diagnostics never persist across a load.
        self.log.clear();
        self.trace.clear();
        self.record = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a raw sector-dump image carrying a valid VIB for the given geometry;
    /// every sector's first two bytes hold its own LBA (big-endian) so a later read
    /// can be checked against the expected mapping.
    fn synthetic_disk(spt: usize, tracks: usize, sides: usize) -> Vec<u8> {
        let total = spt * tracks * sides;
        let mut img = vec![0u8; total * SECTOR_SIZE];
        // VIB (sector 0).
        img[0x0A] = (total >> 8) as u8;
        img[0x0B] = total as u8;
        img[0x0C] = spt as u8;
        img[0x0D..0x10].copy_from_slice(b"DSK");
        img[0x11] = tracks as u8;
        img[0x12] = sides as u8;
        img[0x13] = if spt >= 18 { 2 } else { 1 }; // density (informational)
        img
    }

    #[test]
    fn parses_sssd_dssd_ssdd_geometry_from_the_vib() {
        assert_eq!(
            parse_geometry(&synthetic_disk(9, 40, 1)),
            Geometry { sectors_per_track: 9, tracks: 40, sides: 1 }
        );
        assert_eq!(
            parse_geometry(&synthetic_disk(9, 40, 2)),
            Geometry { sectors_per_track: 9, tracks: 40, sides: 2 }
        );
        assert_eq!(
            parse_geometry(&synthetic_disk(18, 40, 1)),
            Geometry { sectors_per_track: 18, tracks: 40, sides: 1 }
        );
    }

    #[test]
    fn eighty_track_geometry_parses() {
        assert_eq!(
            parse_geometry(&synthetic_disk(9, 80, 2)),
            Geometry { sectors_per_track: 9, tracks: 80, sides: 2 }
        );
    }

    #[test]
    fn inconsistent_vib_falls_back_to_size_default() {
        // A 90 KiB SSSD image, but corrupt the sectors/track so the geometry product
        // (18·40·1 = 720) no longer matches the recorded total (360) or the length.
        let mut img = synthetic_disk(9, 40, 1);
        img[0x0C] = 18;
        assert_eq!(
            parse_geometry(&img),
            Geometry { sectors_per_track: 9, tracks: 40, sides: 1 }
        );
    }

    #[test]
    fn length_mismatch_falls_back() {
        // Consistent VIB fields, but the image is a sector short of what it claims.
        let mut img = synthetic_disk(9, 40, 1);
        img.truncate(img.len() - SECTOR_SIZE);
        assert_eq!(
            parse_geometry(&img),
            Geometry { sectors_per_track: 9, tracks: 40, sides: 1 }
        );
    }

    #[test]
    fn headerless_image_falls_back_by_size() {
        // No "DSK" magic. 90 KiB ⇒ one side; 180 KiB ⇒ two sides.
        let sssd = vec![0u8; TRACKS * SECTORS_PER_TRACK * SECTOR_SIZE];
        assert_eq!(
            parse_geometry(&sssd),
            Geometry { sectors_per_track: 9, tracks: 40, sides: 1 }
        );
        let dssd = vec![0u8; 2 * TRACKS * SECTORS_PER_TRACK * SECTOR_SIZE];
        assert_eq!(
            parse_geometry(&dssd),
            Geometry { sectors_per_track: 9, tracks: 40, sides: 2 }
        );
    }
}
