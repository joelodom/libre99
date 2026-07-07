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

//! The **DSR test rig** shared by the disk-DSR gates (Phase 3,
//! `original-content/system-roms/disk-dsr/`).
//!
//! The rig is a generated **system GROM**: it boots on the (authentic) console
//! ROM, runs the peripheral power-up scan, stages test PABs / buffers into VDP
//! RAM, then drives the disk DSR through the console's real DSRLNK calling
//! sequence — the same staging our console GROM's DSRLNK performs (parse the
//! device name from the PAB, stage `>834A`/`>8355`/`>8356`, `XML >19`), wrapped
//! in a GPL `CALL` so the DSR's skip-return pops the frame exactly as it does
//! for real software. Subprograms (`>10`-`>16`) are driven the same way with
//! the one-byte name + `>836D = >0A`.
//!
//! Tests are **differential**: run an identical script under the authentic
//! `Disk.Bin` and under our clean-room DSR, then diff the observables (PAB
//! bytes, VDP buffers, scratchpad cells, the resulting disk image, the sector
//! read log). The module also carries a pure-Rust **TI-disk builder** used to
//! author fixture disks (validated by the authentic DSR reading them).

#![allow(dead_code)]

use std::sync::LazyLock;

use libre99_core::machine::Machine;

// The authentic images the differential gates run against — loaded at run time
// from `third-party/` (see `libre99_core::third_party`), `None` when the media is
// absent (every test then skips instead of running).
pub static TI_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
pub static TI_DSR: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/Disk.Bin"));
pub static TUNNELS: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("disks/Tunnels.Dsk"));

/// Our clean-room DSR, built from source (never the committed artifact).
pub fn our_dsr() -> Vec<u8> {
    libre99_asm::disk_dsr::build_disk_dsr().expect("disk DSR assembles")
}

/// Completion marker cell (rig GPL writes `>55` when the script finishes) and
/// the "no card DSR handled the call" breadcrumb (`>EE`).
pub const DONE_CELL: u16 = 0x8320;
pub const NOTFOUND_CELL: u16 = 0x8321;

/// One scripted DSR interaction.
pub enum Op {
    /// A device-level call: run the DSRLNK sequence on the PAB staged at this
    /// VDP address (the PAB's name field must already hold `DEVICE.NAME`).
    Dev { pab: u16 },
    /// A subprogram call (`>10`..`>16`): stages the one-byte name and the given
    /// CPU scratchpad parameter bytes, then `XML >19` with key `>0A`.
    Sub { n: u8, cpu: Vec<(u16, u8)> },
    /// Overwrite one VDP byte between calls (e.g. patch a PAB's opcode).
    Poke { vdp: u16, byte: u8 },
    /// Copy `len` VDP bytes `from` → `to` between calls (snapshot a PAB's
    /// status bytes before the next op overwrites them).
    Snap { from: u16, to: u16, len: u16 },
}

/// A rig script: VDP data blocks staged at boot + the op sequence.
pub struct Rig {
    pub blocks: Vec<(u16, Vec<u8>)>,
    pub ops: Vec<Op>,
}

impl Rig {
    pub fn new() -> Self {
        Rig { blocks: Vec::new(), ops: Vec::new() }
    }
    pub fn stage(mut self, vdp: u16, bytes: &[u8]) -> Self {
        self.blocks.push((vdp, bytes.to_vec()));
        self
    }
    pub fn dev(mut self, pab: u16) -> Self {
        self.ops.push(Op::Dev { pab });
        self
    }
    pub fn sub(mut self, n: u8, cpu: &[(u16, u8)]) -> Self {
        self.ops.push(Op::Sub { n, cpu: cpu.to_vec() });
        self
    }
    pub fn poke(mut self, vdp: u16, byte: u8) -> Self {
        self.ops.push(Op::Poke { vdp, byte });
        self
    }
    pub fn snap(mut self, from: u16, to: u16, len: u16) -> Self {
        self.ops.push(Op::Snap { from, to, len });
        self
    }

    /// Generate the rig GROM's GPL source.
    fn gpl(&self) -> String {
        let mut s = String::from(
            "
        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000
        DATA >0000
        DATA >0000
        DATA >0000
        DATA >0000
        DATA >0000

        GROM >0020
START   MOVE >0008,G@VREGS,#0        ; VDP regs: 16K, display off
        DST  @>8370,>3FFF            ; VRAM ceiling before the power-up scan
        ST   @>836D,>04              ; power-up chain key
        DST  @>8354,>0000            ; name length 0 = match every node
        DCLR @>83D0
        XML  >19                     ; run every card's power-up routine
",
        );
        // Stage the data blocks.
        for (i, (addr, bytes)) in self.blocks.iter().enumerate() {
            s += &format!(
                "        MOVE >{len:04X},G@BLK{i},V@>{addr:04X}\n",
                len = bytes.len()
            );
        }
        // The op sequence.
        for op in &self.ops {
            match op {
                Op::Dev { pab } => {
                    s += &format!("        DST  @>8356,>{:04X}\n", pab + 9);
                    s += "        CALL DOLNK\n";
                }
                Op::Sub { n, cpu } => {
                    for (addr, byte) in cpu {
                        s += &format!("        ST   @>{addr:04X},>{byte:02X}\n");
                    }
                    s += &format!("        ST   @>834A,>{n:02X}\n");
                    s += "        DST  @>8354,>0001\n"; // name length 1 (>8355)
                    s += "        CALL DOSUB\n";
                }
                Op::Poke { vdp, byte } => {
                    s += &format!("        ST   V@>{vdp:04X},>{byte:02X}\n");
                }
                Op::Snap { from, to, len } => {
                    s += &format!("        MOVE >{len:04X},V@>{from:04X},V@>{to:04X}\n");
                }
            }
        }
        s += "        ST   @>8320,>55              ; script complete\n";
        s += "HANG    B    HANG\n";
        // DOLNK: the DSRLNK staging sequence (same shape as our console GROM's
        // DSRLNK — our own code): parse the device name out of the PAB whose
        // name-length byte >8356 points at, stage >834A/>8355/>8356, XML >19.
        s += "
DOLNK   ST   @>836D,>08              ; DSR (device) chain key
        ST   @>8355,*V@>8356         ; total PAB name length
        CLR  @>8354
        CLR  @>8358
        DST  @>8352,@>8356
DLNXT   DINC @>8352
        CEQ  @>8358,@>8355
        BS   DLDON
        CEQ  *V@>8352,>2E            ; '.' separator?
        BS   DLDON
        INC  @>8358
        BR   DLNXT
DLDON   ST   @>8355,@>8358           ; device-name length
        CLR  @>8354
        DCLR @>83D0
        DINC @>8356                  ; -> first name char
        MOVE @>8354,*V@>8356,@>834A  ; stage the device name
        DADD @>8356,@>8354           ; -> past the device name
        XML  >19                     ; search + call the card DSR
        ST   @>8321,>EE              ; no card DSR handled it
        RTN
DOSUB   ST   @>836D,>0A              ; subprogram chain key
        DCLR @>83D0
        XML  >19
        ST   @>8321,>EE
        RTN
VREGS   BYTE >00,>80,>00,>10,>01,>06,>01,>17
";
        for (i, (_, bytes)) in self.blocks.iter().enumerate() {
            s += &format!("BLK{i}");
            for (j, b) in bytes.iter().enumerate() {
                if j % 12 == 0 {
                    if j > 0 {
                        s.push('\n');
                    }
                    s += "        BYTE ";
                } else {
                    s.push(',');
                }
                s += &format!(">{b:02X}");
            }
            s.push('\n');
        }
        s
    }

    /// Assemble the rig GROM.
    pub fn grom(&self) -> Vec<u8> {
        let src = self.gpl();
        libre99_gpl::assemble(&src)
            .unwrap_or_else(|d| panic!("rig GPL failed to assemble: {d:?}\n{src}"))
            .image
    }
}

impl Default for Rig {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a rig script against `dsr` with the given disks mounted (drive, image).
/// Returns the machine after the script's completion marker appears, or `None`
/// when the authentic console ROM is absent (the caller then skips).
pub fn run_rig(dsr: &[u8], disks: &[(usize, Vec<u8>)], rig: &Rig) -> Option<Machine> {
    let ti_rom = TI_ROM.as_deref()?;
    let grom = rig.grom();
    let mut m = Machine::new(ti_rom, &grom);
    m.load_disk_controller(dsr);
    for (drive, image) in disks {
        m.mount_disk(*drive, image.clone());
    }
    m.reset();
    m.bus_mut().disk.record(true);
    for _ in 0..900 {
        m.run_frame();
        if m.bus().peek(DONE_CELL) == 0x55 {
            return Some(m);
        }
    }
    panic!(
        "rig script did not complete (>8320={:02X}, >8321={:02X}, >8370={:04X})",
        m.bus().peek(DONE_CELL),
        m.bus().peek(NOTFOUND_CELL),
        m.bus().peek_word(0x8370),
    );
}

/// Run the same script under the authentic DSR and ours; return both machines,
/// or `None` when the authentic media is absent (the caller then skips).
pub fn differential(disks: &[(usize, Vec<u8>)], rig: &Rig) -> Option<(Machine, Machine)> {
    Some((run_rig(TI_DSR.as_deref()?, disks, rig)?, run_rig(&our_dsr(), disks, rig)?))
}

/// Read `len` bytes of VDP RAM.
pub fn vram(m: &Machine, addr: u16, len: usize) -> Vec<u8> {
    (0..len).map(|i| m.vdp().vram(addr + i as u16)).collect()
}

/// Hex-dump helper for probe output.
pub fn hex(bytes: &[u8]) -> String {
    bytes
        .chunks(16)
        .map(|c| c.iter().map(|b| format!("{b:02X} ")).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

// ---------------------------------------------------------------------------
// A standard PAB, staged at a VDP address.
// ---------------------------------------------------------------------------

/// Build the bytes of a PAB: opcode, flags, buffer, reclen, charcount, record
/// number, screen offset, then the `DEVICE.NAME` string.
pub fn pab(opcode: u8, flags: u8, buffer: u16, reclen: u8, count: u8, rec: u16, name: &str) -> Vec<u8> {
    let mut p = vec![
        opcode,
        flags,
        (buffer >> 8) as u8,
        buffer as u8,
        reclen,
        count,
        (rec >> 8) as u8,
        rec as u8,
        0,
        name.len() as u8,
    ];
    p.extend_from_slice(name.as_bytes());
    p
}

// PAB flag bits.
pub const F_VAR: u8 = 0x10;
pub const F_INT: u8 = 0x08;
pub const F_REL: u8 = 0x01;
pub const M_UPDATE: u8 = 0x00;
pub const M_OUTPUT: u8 = 0x02;
pub const M_INPUT: u8 = 0x04;
pub const M_APPEND: u8 = 0x06;

// ---------------------------------------------------------------------------
// The pure-Rust TI-disk builder (fixture disks; format per the plan's
// Appendix A, validated by the authentic DSR reading its output).
// ---------------------------------------------------------------------------

pub const SECTOR: usize = 256;

#[derive(Clone, Copy, PartialEq)]
pub enum Ftype {
    DisFix,
    DisVar,
    IntFix,
    IntVar,
    Program,
}

pub struct FileSpec {
    pub name: &'static str,
    pub ftype: Ftype,
    pub reclen: u8,
    /// Records for FIX/VAR files; ignored for Program.
    pub records: Vec<Vec<u8>>,
    /// Raw image for Program files.
    pub program: Vec<u8>,
    pub protected: bool,
}

impl FileSpec {
    pub fn fixed(name: &'static str, internal: bool, reclen: u8, records: &[&[u8]]) -> Self {
        FileSpec {
            name,
            ftype: if internal { Ftype::IntFix } else { Ftype::DisFix },
            reclen,
            records: records.iter().map(|r| r.to_vec()).collect(),
            program: Vec::new(),
            protected: false,
        }
    }
    pub fn var(name: &'static str, internal: bool, reclen: u8, records: &[&[u8]]) -> Self {
        FileSpec {
            name,
            ftype: if internal { Ftype::IntVar } else { Ftype::DisVar },
            reclen,
            records: records.iter().map(|r| r.to_vec()).collect(),
            program: Vec::new(),
            protected: false,
        }
    }
    pub fn program(name: &'static str, image: &[u8]) -> Self {
        FileSpec {
            name,
            ftype: Ftype::Program,
            reclen: 0,
            records: Vec::new(),
            program: image.to_vec(),
            protected: false,
        }
    }
}

/// Author a formatted SSSD (360-sector) disk image carrying `files`.
///
/// Layout choices (any valid layout is fine — the authentic DSR reading it is
/// the validation): FDRs at sectors 2, 3, …; file data packed upward from
/// sector >22 (34); FDIR pointers sorted by name; bitmap bits for every used
/// sector plus the permanently-reserved 0/1; bitmap bytes beyond the disk's
/// total sectors set to >FF (the authentic formatter's convention — verified
/// against `Tunnels.Dsk`).
pub fn build_disk(volname: &str, files: &[FileSpec]) -> Vec<u8> {
    let total = 360usize;
    let mut img = vec![0u8; total * SECTOR];

    // --- VIB skeleton ---
    let name10 = format!("{volname: <10}");
    img[..10].copy_from_slice(&name10.as_bytes()[..10]);
    img[0x0A] = (total >> 8) as u8;
    img[0x0B] = total as u8;
    img[0x0C] = 9;
    img[0x0D..0x10].copy_from_slice(b"DSK");
    img[0x10] = b' ';
    img[0x11] = 40;
    img[0x12] = 1;
    img[0x13] = 1;

    let mark = |img: &mut Vec<u8>, s: usize| {
        img[0x38 + s / 8] |= 1 << (s % 8);
    };
    mark(&mut img, 0);
    mark(&mut img, 1);
    // Bitmap tail beyond the disk's sectors: >FF (allocated), per the TI
    // formatter (Tunnels.Dsk ground truth: bytes >38+45.. are >FF).
    for b in &mut img[0x38 + total.div_ceil(8)..0x38 + 200] {
        *b = 0xFF;
    }

    // --- files ---
    let mut fdr_secs: Vec<(String, usize)> = Vec::new();
    let mut next_data = 0x22usize;
    for (i, f) in files.iter().enumerate() {
        let fdr_sec = 2 + i;
        mark(&mut img, fdr_sec);

        // Serialize the file's data sectors.
        let (data, eof, recs_per_sec, level3): (Vec<u8>, u8, u8, u16) = match f.ftype {
            Ftype::Program => {
                let d = f.program.clone();
                let eof = (d.len() % SECTOR) as u8;
                (d, eof, 0, 0)
            }
            Ftype::DisFix | Ftype::IntFix => {
                let rps = (SECTOR / f.reclen as usize) as u8;
                let mut d = Vec::new();
                for (r, rec) in f.records.iter().enumerate() {
                    if r % rps as usize == 0 && r > 0 {
                        d.resize(d.len().next_multiple_of(SECTOR), 0);
                    }
                    let mut cell = rec.clone();
                    cell.resize(f.reclen as usize, 0);
                    d.extend_from_slice(&cell);
                }
                (d, 0, rps, f.records.len() as u16)
            }
            Ftype::DisVar | Ftype::IntVar => {
                let mut d = Vec::new();
                let mut in_sec = 0usize;
                for rec in &f.records {
                    let need = rec.len() + 1;
                    if in_sec + need + 1 > SECTOR {
                        d.push(0xFF);
                        d.resize(d.len().next_multiple_of(SECTOR), 0);
                        in_sec = 0;
                    }
                    d.push(rec.len() as u8);
                    d.extend_from_slice(rec);
                    in_sec += need;
                }
                d.push(0xFF);
                let sectors = d.len().div_ceil(SECTOR) as u16;
                // EOF offset excludes the >FF terminator (authentic convention,
                // pinned by probe_write_var: "HELLO"+"WORLDLY" -> eof 14).
                let eof = ((d.len() - 1) % SECTOR) as u8;
                let rps = (SECTOR / (f.reclen as usize + 1)) as u8;
                (d, eof, rps, sectors)
            }
        };
        let nsec = data.len().div_ceil(SECTOR);
        let first = next_data;
        for (k, chunk) in data.chunks(SECTOR).enumerate() {
            img[(first + k) * SECTOR..(first + k) * SECTOR + chunk.len()].copy_from_slice(chunk);
            mark(&mut img, first + k);
        }
        next_data += nsec;

        // The FDR.
        let fdr = &mut img[fdr_sec * SECTOR..(fdr_sec + 1) * SECTOR];
        let n10 = format!("{: <10}", f.name);
        fdr[..10].copy_from_slice(&n10.as_bytes()[..10]);
        let mut flags = match f.ftype {
            Ftype::Program => 0x01,
            Ftype::DisFix => 0x00,
            Ftype::DisVar => 0x80,
            Ftype::IntFix => 0x02,
            Ftype::IntVar => 0x82,
        };
        if f.protected {
            flags |= 0x08;
        }
        fdr[0x0C] = flags;
        fdr[0x0D] = recs_per_sec;
        fdr[0x0E] = (nsec >> 8) as u8;
        fdr[0x0F] = nsec as u8;
        fdr[0x10] = eof;
        fdr[0x11] = f.reclen;
        // >12: little-endian (byte-swapped) count.
        let l3 = if matches!(f.ftype, Ftype::DisFix | Ftype::IntFix) {
            f.records.len() as u16
        } else {
            level3
        };
        fdr[0x12] = l3 as u8;
        fdr[0x13] = (l3 >> 8) as u8;
        // One contiguous cluster: start sector `first`, end offset nsec-1.
        if nsec > 0 {
            let start = first as u16;
            let endoff = (nsec - 1) as u16;
            fdr[0x1C] = start as u8;
            fdr[0x1D] = ((start >> 8) as u8 & 0x0F) | (((endoff & 0x0F) as u8) << 4);
            fdr[0x1E] = (endoff >> 4) as u8;
        }
        fdr_secs.push((n10, fdr_sec));
    }

    // --- FDIR: sorted by name ---
    fdr_secs.sort_by(|a, b| a.0.cmp(&b.0));
    for (i, (_, sec)) in fdr_secs.iter().enumerate() {
        img[SECTOR + 2 * i] = (*sec >> 8) as u8;
        img[SECTOR + 2 * i + 1] = *sec as u8;
    }
    img
}
