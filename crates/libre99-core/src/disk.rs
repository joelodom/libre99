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
//! ## Disk persistence — the source file is never touched
//! A Write Sector command mutates only the **in-memory** image; the emulator
//! never writes back to the host `.dsk` file. Instead, every image mounted with
//! a host identity ([`mount_keyed`](Disk::mount_keyed)) stays in memory for the
//! life of the machine: ejecting moves it to an in-memory **shelf**, and
//! remounting the same file reattaches the shelved image — written sectors
//! intact — rather than re-reading the host bytes. A per-image **dirty** flag
//! records whether the DSR has written to it since it left its host file.
//! Save states serialize the drives *and* the shelf, so in-memory disks (and
//! their edits) survive quit-and-resume. Getting edits back onto the host
//! filesystem is an explicit frontend **export**:
//! [`image_for_key`](Disk::image_for_key) hands over the delta-applied bytes to
//! write to a file of the user's choosing, and [`forget`](Disk::forget) drops
//! an in-memory image so the next mount starts over from the host file.
//! (The bare [`mount`](Disk::mount) is the *anonymous* variant — no identity,
//! nothing remembered across an eject — used by tests and diagnostics.)

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

/// One in-memory disk image, as reported by [`Disk::in_memory_disks`] for the
/// frontend's disk-memory view — either mounted in a drive right now or
/// shelved after an eject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskInfo {
    /// The host identity the image is keyed by (the source file's path string,
    /// as given to [`Disk::mount_keyed`]).
    pub key: String,
    /// Image size in bytes.
    pub len: usize,
    /// Has the DSR written to this image since it was read from its host file
    /// (and not since been exported)?
    pub dirty: bool,
    /// The drive the image is mounted in (0 = DSK1), or `None` if shelved.
    pub drive: Option<usize>,
}

/// A keyed disk image parked in memory after an eject, waiting to be
/// reattached by a later mount of the same host file.
struct ShelvedDisk {
    key: String,
    image: Vec<u8>,
    dirty: bool,
}

/// The TI Disk Controller card: FD1771 state, the DSR ROM, and up to three
/// mounted disk images (DSK1–DSK3).
pub struct Disk {
    /// `Disk.Bin` DSR ROM; `None` until a controller is installed (the window is
    /// then open bus, matching a machine with no disk card).
    dsr: Option<Box<[u8; DSR_SIZE]>>,
    /// Raw sector-dump images for DSK1/DSK2/DSK3.
    drives: [Option<Vec<u8>>; 3],
    /// Host identity of each mounted image (`None` = an anonymous [`mount`]
    /// that is not remembered across an eject).
    ///
    /// [`mount`]: Disk::mount
    drive_keys: [Option<String>; 3],
    /// Whether the DSR has written to each mounted image since it was read
    /// from its host file (cleared by [`mark_clean`](Disk::mark_clean) after
    /// an export).
    drive_dirty: [bool; 3],
    /// The shelf: every keyed image ejected this session, kept in memory so a
    /// remount of the same file reattaches it, edits intact.
    shelf: Vec<ShelvedDisk>,
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
            drive_keys: [None, None, None],
            drive_dirty: [false; 3],
            shelf: Vec::new(),
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
    ///
    /// This is the **anonymous** mount (tests/diagnostics): the image carries no
    /// host identity and is dropped on eject. The frontend mounts through
    /// [`mount_keyed`](Self::mount_keyed) so edits survive eject/remount.
    pub fn mount(&mut self, drive: usize, image: Vec<u8>) {
        if drive < self.drives.len() {
            self.shelve(drive);
            self.geometry[drive] = parse_geometry(&image);
            self.drives[drive] = Some(image);
        }
    }

    /// Mount `image` — freshly read from the host file identified by `key` —
    /// into drive `drive`, preferring the in-memory copy: if a disk with the
    /// same key is on the shelf (ejected earlier, possibly carrying written
    /// sectors), the shelved image is reattached instead and `image` is
    /// dropped. Whatever the drive held before is shelved first (if keyed) so
    /// its edits are not lost either. Returns `true` when the shelved copy was
    /// used, so the frontend can tell the user their in-memory changes apply.
    pub fn mount_keyed(&mut self, drive: usize, key: &str, image: Vec<u8>) -> bool {
        if drive >= self.drives.len() {
            return false;
        }
        self.shelve(drive);
        let (image, dirty, resumed) = match self.shelf.iter().position(|s| s.key == key) {
            Some(i) => {
                let s = self.shelf.remove(i);
                (s.image, s.dirty, true)
            }
            None => (image, false, false),
        };
        self.geometry[drive] = parse_geometry(&image);
        self.drives[drive] = Some(image);
        self.drive_keys[drive] = Some(key.to_string());
        self.drive_dirty[drive] = dirty;
        resumed
    }

    /// Empty drive `drive` — the live equivalent of pulling the floppy. A keyed
    /// image moves to the in-memory shelf (edits intact, ready to reattach on a
    /// later [`mount_keyed`](Self::mount_keyed) of the same file); an anonymous
    /// image is dropped.
    pub fn eject(&mut self, drive: usize) {
        if drive < self.drives.len() {
            self.shelve(drive);
            self.geometry[drive] = Geometry::default();
        }
    }

    /// Move `drive`'s keyed image (if any) onto the shelf, replacing any older
    /// shelved copy with the same key; an anonymous image is dropped. Leaves
    /// the drive empty either way.
    fn shelve(&mut self, drive: usize) {
        let image = self.drives[drive].take();
        let key = self.drive_keys[drive].take();
        let dirty = std::mem::replace(&mut self.drive_dirty[drive], false);
        if let (Some(image), Some(key)) = (image, key) {
            self.shelf.retain(|s| s.key != key);
            self.shelf.push(ShelvedDisk { key, image, dirty });
        }
    }

    /// Every disk image held in memory — mounted in a drive or shelved after an
    /// eject — for the frontend's disk-memory view. Mounted drives come first
    /// (in drive order), then the shelf (in eject order).
    pub fn in_memory_disks(&self) -> Vec<DiskInfo> {
        let mut out = Vec::new();
        for (d, (image, key)) in self.drives.iter().zip(&self.drive_keys).enumerate() {
            if let (Some(image), Some(key)) = (image, key) {
                out.push(DiskInfo {
                    key: key.clone(),
                    len: image.len(),
                    dirty: self.drive_dirty[d],
                    drive: Some(d),
                });
            }
        }
        for s in &self.shelf {
            out.push(DiskInfo { key: s.key.clone(), len: s.image.len(), dirty: s.dirty, drive: None });
        }
        out
    }

    /// Drop the in-memory image identified by `key` — off the shelf, or straight
    /// out of the drive holding it (which then reads empty). The next mount of
    /// the same file starts over from the host file's bytes. Returns whether
    /// anything was dropped.
    pub fn forget(&mut self, key: &str) -> bool {
        let before = self.shelf.len();
        self.shelf.retain(|s| s.key != key);
        let mut removed = self.shelf.len() != before;
        for d in 0..self.drives.len() {
            if self.drive_keys[d].as_deref() == Some(key) {
                self.drives[d] = None;
                self.drive_keys[d] = None;
                self.drive_dirty[d] = false;
                self.geometry[d] = Geometry::default();
                removed = true;
            }
        }
        removed
    }

    /// The in-memory image identified by `key` (mounted or shelved) — the
    /// delta-applied bytes a frontend export writes out.
    pub fn image_for_key(&self, key: &str) -> Option<&[u8]> {
        for (image, k) in self.drives.iter().zip(&self.drive_keys) {
            if k.as_deref() == Some(key) {
                return image.as_deref();
            }
        }
        self.shelf.iter().find(|s| s.key == key).map(|s| s.image.as_slice())
    }

    /// Mark `key`'s in-memory image clean — called after an export writes its
    /// bytes to a host file, so unsaved-changes warnings stand down.
    pub fn mark_clean(&mut self, key: &str) {
        for (k, dirty) in self.drive_keys.iter().zip(&mut self.drive_dirty) {
            if k.as_deref() == Some(key) {
                *dirty = false;
            }
        }
        if let Some(s) = self.shelf.iter_mut().find(|s| s.key == key) {
            s.dirty = false;
        }
    }

    /// Host identity of the image in `drive`, if it was mounted keyed.
    pub fn drive_key(&self, drive: usize) -> Option<&str> {
        self.drive_keys.get(drive)?.as_deref()
    }

    /// Has the DSR written to `drive`'s image since it was read from its host
    /// file (and not since been exported)?
    pub fn drive_dirty(&self, drive: usize) -> bool {
        self.drive_dirty.get(drive).copied().unwrap_or(false)
    }

    /// Adopt `key` as the host identity of the image already mounted in
    /// `drive`, if it has none — used when resuming a version-1 save state,
    /// whose drives carried no identities, with the identity the frontend
    /// recorded separately at exit.
    pub fn adopt_drive_key(&mut self, drive: usize, key: &str) {
        if drive < self.drives.len()
            && self.drives[drive].is_some()
            && self.drive_keys[drive].is_none()
        {
            self.drive_keys[drive] = Some(key.to_string());
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

    /// Copy the sector buffer back into the selected drive's image and mark the
    /// image dirty (it now differs from its host file until exported).
    fn flush_sector(&mut self, lba: usize) {
        let Some(d) = self.selected else { return };
        if let Some(image) = self.drives[d].as_mut() {
            let start = lba * SECTOR_SIZE;
            if let Some(dst) = image.get_mut(start..start + SECTOR_SIZE) {
                dst.copy_from_slice(&self.buffer);
                self.drive_dirty[d] = true;
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
    /// written-back sectors) plus their host identities and dirty flags, the
    /// in-memory shelf of ejected disks, the CRU latches, the FD1771 registers,
    /// and any in-progress sector transfer. Diagnostics (the read log/trace)
    /// are not saved. The identity/dirty/shelf fields are the save-format
    /// version-2 tail — appended after the version-1 layout so a v1 file loads
    /// by simply stopping short of them.
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
        // --- version-2 tail: host identities, dirty flags, the shelf ---
        for key in &self.drive_keys {
            w.opt_string(key.as_deref());
        }
        for dirty in self.drive_dirty {
            w.bool(dirty);
        }
        w.usize(self.shelf.len());
        for s in &self.shelf {
            w.string(&s.key);
            w.blob(&s.image);
            w.bool(s.dirty);
        }
    }

    /// Restore the controller from a save state of format `version`. The
    /// transfer cursor and the selected-drive index are sanitized so a corrupt
    /// or foreign file can never drive a later register access out of bounds.
    /// A version-1 file has no identity/dirty/shelf tail: those default to
    /// anonymous/clean/empty.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
        version: u32,
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
        self.drive_keys = [None, None, None];
        self.drive_dirty = [false; 3];
        self.shelf.clear();
        if version >= 2 {
            for key in &mut self.drive_keys {
                *key = r.opt_string()?;
            }
            for dirty in &mut self.drive_dirty {
                *dirty = r.bool()?;
            }
            let n = r.usize()?;
            for _ in 0..n {
                // No pre-allocation from the untrusted count: each entry reads at
                // least a byte or errors, so a lying count fails fast as Truncated.
                let key = r.string()?;
                let image = r.blob()?;
                let dirty = r.bool()?;
                self.shelf.push(ShelvedDisk { key, image, dirty });
            }
        }

        // --- sanitize against a corrupt/foreign file ---
        // An identity or dirty flag on an empty drive is meaningless — drop it.
        for (d, drive) in self.drives.iter().enumerate() {
            if drive.is_none() {
                self.drive_keys[d] = None;
                self.drive_dirty[d] = false;
            }
        }
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

    // ----------------------------------------------------------------------
    // Disk persistence: the keyed-mount shelf, dirty tracking, forget/export.
    // ----------------------------------------------------------------------

    /// A controller with a dummy DSR installed (register access requires a
    /// card) and DSK1 selected.
    fn card() -> Disk {
        let mut d = Disk::new();
        d.load_dsr(&[0u8; 4]);
        d.write_cru(4, true); // select DSK1
        d
    }

    /// Write a register byte through the bus window (which one's-complements).
    fn put(d: &mut Disk, addr: u16, value: u8) {
        d.write_byte(addr, value ^ 0xFF);
    }

    /// Write 256 bytes of `fill` into sector 0 of the selected drive via the
    /// FD1771, exactly as the DSR does.
    fn write_sector_zero(d: &mut Disk, fill: u8) {
        put(d, 0x5FFC, 0); // sector 0
        put(d, 0x5FF8, 0xA0); // Write Sector
        for _ in 0..SECTOR_SIZE {
            put(d, 0x5FFE, fill);
        }
    }

    #[test]
    fn keyed_eject_and_remount_reattaches_the_written_image() {
        let mut d = card();
        let host = vec![0u8; 4 * SECTOR_SIZE];
        assert!(!d.mount_keyed(0, "K", host.clone()), "first mount is from host bytes");
        assert!(!d.drive_dirty(0));
        write_sector_zero(&mut d, 0xEE);
        assert!(d.drive_dirty(0), "a flushed write marks the image dirty");

        d.eject(0);
        assert!(d.drive_image(0).is_none(), "the drive is empty after eject");
        let shelved = d.in_memory_disks();
        assert_eq!(shelved.len(), 1);
        assert_eq!(shelved[0].key, "K");
        assert!(shelved[0].dirty);
        assert_eq!(shelved[0].drive, None);

        // Remount the same file: the shelved copy wins over fresh host bytes.
        assert!(d.mount_keyed(0, "K", host), "remount reattaches the in-memory copy");
        assert_eq!(d.drive_image(0).unwrap()[0], 0xEE, "written sectors survive the eject");
        assert!(d.drive_dirty(0), "the dirty flag rides along");
        assert_eq!(d.in_memory_disks()[0].drive, Some(0));
    }

    #[test]
    fn mounting_a_different_disk_shelves_the_current_one() {
        let mut d = card();
        d.mount_keyed(0, "A", vec![0u8; 4 * SECTOR_SIZE]);
        write_sector_zero(&mut d, 0x11);
        d.mount_keyed(0, "B", vec![0u8; 4 * SECTOR_SIZE]);
        let disks = d.in_memory_disks();
        assert_eq!(disks.len(), 2);
        assert!(disks.iter().any(|i| i.key == "A" && i.dirty && i.drive.is_none()));
        assert!(disks.iter().any(|i| i.key == "B" && !i.dirty && i.drive == Some(0)));
        // And A's edits reattach later.
        assert!(d.mount_keyed(0, "A", vec![0u8; 4 * SECTOR_SIZE]));
        assert_eq!(d.drive_image(0).unwrap()[0], 0x11);
    }

    #[test]
    fn forget_reverts_to_the_host_bytes_on_the_next_mount() {
        let mut d = card();
        d.mount_keyed(0, "K", vec![0u8; 4 * SECTOR_SIZE]);
        write_sector_zero(&mut d, 0xEE);
        // Forgetting a *mounted* disk also empties the drive.
        assert!(d.forget("K"));
        assert!(d.drive_image(0).is_none());
        assert!(d.in_memory_disks().is_empty());
        // The next mount is from host bytes again.
        assert!(!d.mount_keyed(0, "K", vec![0u8; 4 * SECTOR_SIZE]));
        assert_eq!(d.drive_image(0).unwrap()[0], 0x00);
    }

    #[test]
    fn image_for_key_and_mark_clean_reach_both_drive_and_shelf() {
        let mut d = card();
        d.mount_keyed(0, "K", vec![0u8; 4 * SECTOR_SIZE]);
        write_sector_zero(&mut d, 0xEE);
        assert_eq!(d.image_for_key("K").unwrap()[0], 0xEE, "mounted image is exportable");
        d.mark_clean("K");
        assert!(!d.drive_dirty(0), "export marks the mounted image clean");

        write_sector_zero(&mut d, 0xDD);
        d.eject(0);
        assert_eq!(d.image_for_key("K").unwrap()[0], 0xDD, "shelved image is exportable");
        d.mark_clean("K");
        assert!(!d.in_memory_disks()[0].dirty, "export marks the shelved image clean");
    }

    #[test]
    fn anonymous_mounts_are_not_remembered() {
        let mut d = card();
        d.mount(0, vec![0u8; 4 * SECTOR_SIZE]);
        write_sector_zero(&mut d, 0xEE);
        d.eject(0);
        assert!(d.in_memory_disks().is_empty(), "no identity, nothing shelved");
    }

    #[test]
    fn version1_disk_state_loads_with_no_identities_and_an_empty_shelf() {
        // Hand-write the version-1 layout (no identity/dirty/shelf tail).
        let mut w = crate::state::StateWriter::new();
        w.u8(0); // no DSR
        w.u8(1); // DSK1 mounted
        w.blob(&vec![0xABu8; 4 * SECTOR_SIZE]);
        w.u8(0); // DSK2 empty
        w.u8(0); // DSK3 empty
        w.bool(false); // rom_enabled
        w.opt_usize(None); // selected
        w.u8(0); // side
        w.u8(1); // step_dir
        w.u8(0); // status
        w.u8(0); // track
        w.u8(0); // sector
        w.u8(0); // data
        w.raw(&[0u8; SECTOR_SIZE]); // buffer
        w.usize(0); // buf_pos
        w.usize(0); // buf_left
        w.opt_usize(None); // write_lba
        let bytes = w.into_bytes();

        let mut d = Disk::new();
        let mut r = crate::state::StateReader::new(&bytes);
        d.load_state(&mut r, 1).expect("a version-1 disk section loads");
        assert_eq!(d.drive_image(0).unwrap()[0], 0xAB);
        assert_eq!(d.drive_key(0), None);
        assert!(!d.drive_dirty(0));
        assert!(d.in_memory_disks().is_empty());
        // The frontend can then adopt the identity it recorded at exit.
        d.adopt_drive_key(0, "K");
        assert_eq!(d.drive_key(0), Some("K"));
    }
}
