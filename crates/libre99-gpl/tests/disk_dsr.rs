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

//! Gates + recon probes for the **Disk Controller DSR rewrite** (Phase 3,
//! `original-content/system-roms/disk-dsr/`).
//!
//! The `#[ignore]`d `probe_*` tests are the D1 dossier instruments: they run
//! scripted PAB / subprogram operations against the **authentic** `Disk.Bin`
//! (via the `dsr_common` rig) and print the observed behavior — the empirical
//! spec our clean-room DSR is written from. Run them with
//! `cargo test -p libre99-gpl --test disk_dsr -- --ignored --nocapture`.
//! The un-ignored tests are the live differential gates.

mod dsr_common;

use dsr_common::*;
use libre99_core::machine::Machine;

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn our_rom() -> Vec<u8> {
    libre99_asm::system_rom::build_console_rom().expect("console ROM assembles")
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

// ---------------------------------------------------------------------------
// T1 gates (tracer bullet)
// ---------------------------------------------------------------------------

/// The image shape at the integration boundary: exactly 8 KiB, `>AA` at `>4000`.
#[test]
fn our_dsr_is_an_8k_image_with_the_aa_header() {
    let dsr = our_dsr();
    assert_eq!(dsr.len(), 0x2000, "the disk DSR is exactly 8 KiB");
    assert_eq!(dsr[0], 0xAA, ">4000 must be the >AA valid-DSR marker");
}

/// **T1 gate: our clean-room DSR's power-up reserves the VRAM buffer under both
/// console ROMs** — byte-for-byte the reservation `device_io.rs` asserts for
/// the genuine `Disk.Bin`.
#[test]
fn our_dsr_power_up_reserves_the_vram_buffer() {
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    let dsr = our_dsr();
    for (name, rom) in [("TI_ROM", ti_rom.to_vec()), ("OUR_ROM", our_rom())] {
        let grom = our_grom();
        let mut m = Machine::new(&rom, &grom);
        m.load_disk_controller(&dsr);
        m.reset();
        for _ in 0..120 {
            m.run_frame();
        }
        assert_eq!(
            m.bus().peek_word(0x8370),
            0x37D7,
            "{name}: our DSR's power-up did not reserve the VRAM buffer (>8370 != >37D7)"
        );
        assert_eq!(m.bus().peek(0x837D), 0, "{name}: a loud stub fired during boot");
    }
}

// ---------------------------------------------------------------------------
// M1/M2 differential gates: identical rig scripts under the authentic DSR and
// ours; every named observable must match. On failure the panic shows both
// sides — the authentic value IS the spec (RECON method note).
// ---------------------------------------------------------------------------

/// Compare a VDP range across the two machines.
fn diff_vram(a: &Machine, b: &Machine, addr: u16, len: usize, what: &str) {
    let (va, vb) = (vram(a, addr, len), vram(b, addr, len));
    assert_eq!(va, vb, "{what}: VDP >{addr:04X}+{len} differs\nauthentic: {}\nours:      {}", hex(&va), hex(&vb));
}

/// Compare a scratchpad cell (word) across the two machines.
fn diff_word(a: &Machine, b: &Machine, addr: u16, what: &str) {
    assert_eq!(
        a.bus().peek_word(addr),
        b.bus().peek_word(addr),
        "{what}: scratch word >{addr:04X} differs (authentic={:04X} ours={:04X})",
        a.bus().peek_word(addr),
        b.bus().peek_word(addr)
    );
}

/// Power-up parity: >8370 and the buffer-header bytes.
#[test]
fn dsr_powerup_and_header_match() {
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let rig = Rig::new();
    let Some((a, b)) = differential(&[(0, tunnels.to_vec())], &rig) else { skip!() };
    diff_word(&a, &b, 0x8370, "power-up");
    let top = a.bus().peek_word(0x8370);
    diff_vram(&a, &b, top + 1, 5, "buffer header");
}

/// SECTOR (>10) read + write parity: result cells, the read-back buffer, and
/// the on-disk image.
#[test]
fn dsr_sector_subprogram_matches() {
    let disk = build_disk("PROBE", &[FileSpec::var("VARF", false, 80, &[b"HELLO"])]);
    let rig = Rig::new()
        .stage(0x1400, &[0xEE; 8])
        .sub(0x10, &[(0x834C, 1), (0x834D, 1), (0x834E, 0x12), (0x834F, 0x00), (0x8350, 0x00), (0x8351, 0x01)])
        .sub(0x10, &[(0x834C, 1), (0x834D, 0), (0x834E, 0x14), (0x834F, 0x00), (0x8350, 0x00), (0x8351, 40)]);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_word(&a, &b, 0x834A, "SECTOR echo");
    diff_word(&a, &b, 0x8350, "SECTOR error");
    diff_vram(&a, &b, 0x1200, 256, "SECTOR read-back");
    assert_eq!(
        a.bus().disk.drive_image(0).unwrap(),
        b.bus().disk.drive_image(0).unwrap(),
        "SECTOR write: images differ"
    );
}

/// FILES(2) parity: the moved ceiling + header.
#[test]
fn dsr_files_subprogram_matches() {
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let rig = Rig::new().sub(0x16, &[(0x834C, 2)]);
    let Some((a, b)) = differential(&[(0, tunnels.to_vec())], &rig) else { skip!() };
    diff_word(&a, &b, 0x8370, "FILES(2)");
    let top = a.bus().peek_word(0x8370);
    diff_vram(&a, &b, top + 1, 5, "FILES(2) header");
}

/// STATUS parity on an existing (program) file and a missing one, incl. the
/// >8354/>8356 side-effects and the info record's location.
#[test]
fn dsr_status_matches() {
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    for name in ["DSK1.QUEST", "DSK1.NOFILE"] {
        let p = pab(9, 0, 0x1080, 0, 0, 0, name);
        let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
        let Some((a, b)) = differential(&[(0, tunnels.to_vec())], &rig) else { skip!() };
        diff_vram(&a, &b, 0x1000, 10, &format!("STATUS {name} PAB"));
        diff_word(&a, &b, 0x8354, &format!("STATUS {name} >8354"));
        if name.ends_with("QUEST") {
            diff_word(&a, &b, 0x8356, &format!("STATUS {name} >8356"));
        }
    }
}

/// LOAD parity: a builder-authored program file — PAB, the loaded bytes, and
/// the exact sector read order (the FDIR-bisect lockstep).
#[test]
fn dsr_load_program_matches() {
    let img: Vec<u8> = (0..600u16).map(|i| (i % 251) as u8).collect();
    let disk = build_disk("PROBE", &[FileSpec::program("PROG", &img)]);
    let p = pab(5, 0, 0x1400, 0, 0, 0x1000, "DSK1.PROG");
    let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1000, 12, "LOAD PAB");
    diff_vram(&a, &b, 0x1400, 600, "LOAD payload");
    assert_eq!(a.bus().disk.read_log(), b.bus().disk.read_log(), "LOAD sector read order");
}

/// OPEN/READ/CLOSE parity across VAR-sequential and FIX-relative files,
/// with per-op PAB snapshots.
#[test]
fn dsr_read_var_fix_matches() {
    let disk = build_disk(
        "PROBE",
        &[
            FileSpec::var("VARF", false, 80, &[b"ALPHA", b"BETA-BETA", b"GAMMA"]),
            FileSpec::fixed("FIXF", false, 20, &[b"REC0", b"REC1", b"REC2"]),
        ],
    );
    let p = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK1.VARF");
    let mut rig = Rig::new().stage(0x1000, &p).dev(0x1000).snap(0x1000, 0x1900, 10);
    for i in 0..4u16 {
        let buf = 0x1440 + i * 0x40;
        rig = rig
            .poke(0x1000, 2)
            .poke(0x1002, (buf >> 8) as u8)
            .poke(0x1003, buf as u8)
            .dev(0x1000)
            .snap(0x1000, 0x1910 + i * 16, 10);
    }
    let pc = pab(1, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK1.VARF");
    rig = rig.stage(0x10E0, &pc).dev(0x10E0);
    let pf = pab(0, F_REL | M_INPUT, 0x1700, 20, 0, 0, "DSK1.FIXF");
    rig = rig.stage(0x1100, &pf).dev(0x1100);
    rig = rig.poke(0x1100, 2).poke(0x1107, 2).dev(0x1100).snap(0x1100, 0x1960, 10);
    rig = rig.poke(0x1103, 0x40).poke(0x1107, 0).dev(0x1100).snap(0x1100, 0x1970, 10);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1900, 10, "VAR OPEN snap");
    for i in 0..4u16 {
        diff_vram(&a, &b, 0x1910 + i * 16, 10, &format!("VAR READ{i} snap"));
        diff_vram(&a, &b, 0x1440 + i * 0x40, 16, &format!("VAR READ{i} buffer"));
    }
    diff_vram(&a, &b, 0x1960, 10, "FIX READ rec2 snap");
    diff_vram(&a, &b, 0x1700, 20, "FIX rec2 buffer");
    diff_vram(&a, &b, 0x1970, 10, "FIX READ rec0 snap");
    diff_vram(&a, &b, 0x1740, 20, "FIX rec0 buffer");
}

/// Catalog parity: the volume record, two file records, and the end record —
/// byte-exact INTERNAL packing incl. the radix-100 numbers.
#[test]
fn dsr_catalog_matches() {
    let disk = build_disk(
        "MYVOL",
        &[
            FileSpec::var("BFILE", false, 80, &[b"XY"]),
            FileSpec::program("APROG", &[0x55; 300]),
        ],
    );
    let p = pab(0, F_INT | M_INPUT, 0x1400, 38, 0, 0, "DSK1.");
    let mut rig = Rig::new().stage(0x1000, &p).dev(0x1000).snap(0x1000, 0x1900, 10);
    for rec in 0..4u16 {
        let buf = 0x1440 + rec * 0x40;
        rig = rig
            .poke(0x1000, 2)
            .poke(0x1002, (buf >> 8) as u8)
            .poke(0x1003, buf as u8)
            .poke(0x1006, (rec >> 8) as u8)
            .poke(0x1007, rec as u8)
            .dev(0x1000)
            .snap(0x1000, 0x1910 + rec * 16, 10);
    }
    let pc = pab(1, F_INT | M_INPUT, 0x1400, 38, 0, 0, "DSK1.");
    rig = rig.stage(0x10E0, &pc).dev(0x10E0);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1900, 10, "catalog OPEN snap");
    for rec in 0..4u16 {
        diff_vram(&a, &b, 0x1910 + rec * 16, 10, &format!("catalog READ{rec} snap"));
        diff_vram(&a, &b, 0x1440 + rec * 0x40, 38, &format!("catalog record {rec}"));
    }
}

/// Error parity: missing file, reclen mismatch, type mismatch, reclen-0
/// fill-in, protected-disk OPEN OUTPUT.
#[test]
fn dsr_errors_match() {
    let mut disk = build_disk("PROBE", &[FileSpec::var("VARF", false, 80, &[b"X"])]);
    let p1 = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK1.NOFILE");
    let p2 = pab(0, F_VAR | M_INPUT, 0x1400, 40, 0, 0, "DSK1.VARF");
    let p3 = pab(0, M_INPUT, 0x1400, 80, 0, 0, "DSK1.VARF");
    let p4 = pab(0, F_VAR | M_INPUT, 0x1400, 0, 0, 0, "DSK1.VARF");
    let rig = Rig::new()
        .stage(0x1000, &p1)
        .stage(0x1040, &p2)
        .stage(0x1080, &p3)
        .stage(0x10C0, &p4)
        .dev(0x1000)
        .dev(0x1040)
        .dev(0x1080)
        .dev(0x10C0);
    let Some((a, b)) = differential(&[(0, disk.clone())], &rig) else { skip!() };
    for (addr, what) in [(0x1000u16, "missing"), (0x1040, "bad reclen"), (0x1080, "fix-on-var"), (0x10C0, "reclen 0")] {
        diff_vram(&a, &b, addr, 10, what);
    }
    disk[0x10] = b'P';
    let p5 = pab(0, F_VAR | M_OUTPUT, 0x1400, 80, 0, 0, "DSK1.NEW");
    let rig2 = Rig::new().stage(0x1000, &p5).dev(0x1000);
    let Some((a2, b2)) = differential(&[(0, disk)], &rig2) else { skip!() };
    diff_vram(&a2, &b2, 0x1000, 10, "protected OPEN OUTPUT");
}

/// Volume-form parity ("DSK.VOL.FILE"), good and bad volumes.
#[test]
fn dsr_volume_form_matches() {
    let disk = build_disk("MYVOL", &[FileSpec::var("VARF", false, 80, &[b"HELLO-VOL"])]);
    let p = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK.MYVOL.VARF");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 2)
        .dev(0x1000);
    let Some((a, b)) = differential(&[(0, disk.clone())], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1900, 10, "volume OPEN snap");
    diff_vram(&a, &b, 0x1000, 10, "volume READ PAB");
    diff_vram(&a, &b, 0x1400, 16, "volume READ buffer");
    let pb = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK.NOVOL.VARF");
    let rig2 = Rig::new().stage(0x1000, &pb).dev(0x1000);
    let Some((a2, b2)) = differential(&[(0, disk)], &rig2) else { skip!() };
    diff_vram(&a2, &b2, 0x1000, 10, "bad volume PAB");
}

/// RESTORE + SCRATCH parity on a FIX file (SCRATCH = the authentic error 6).
#[test]
fn dsr_restore_scratch_match() {
    let disk = build_disk("PROBE", &[FileSpec::fixed("FIXF", false, 20, &[b"REC0", b"REC1", b"REC2"])]);
    let p = pab(0, F_REL | M_UPDATE, 0x1400, 20, 0, 0, "DSK1.FIXF");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 8) // SCRATCH -> error 6
        .poke(0x1007, 1)
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1000, 4) // RESTORE record 2 (error bits stay sticky)
        .poke(0x1007, 2)
        .dev(0x1000)
        .snap(0x1000, 0x1920, 10)
        .poke(0x1000, 2) // READ -> REC2
        .dev(0x1000);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    for (addr, what) in [(0x1900u16, "OPEN"), (0x1910, "SCRATCH"), (0x1920, "RESTORE")] {
        diff_vram(&a, &b, addr, 10, what);
    }
    diff_vram(&a, &b, 0x1000, 10, "final READ PAB");
    diff_vram(&a, &b, 0x1400, 20, "READ-after-RESTORE buffer");
}

/// **The flagship: Tunnels of Doom loads its QUEST scenario from disk with
/// OUR clean-room DSR — under both the authentic console ROM and ours.**
#[test]
fn tod_loads_quest_with_our_dsr() {
    use libre99_core::cartridge::Cartridge;
    use libre99_core::keyboard::TiKey;
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let Some(cart) = ["cartridges/tundoom.ctg", "cartridges/tunnelsofdoom.ctg"]
        .iter()
        .find_map(|p| libre99_core::third_party::load(p))
        .map(|d| Cartridge::parse(&d).unwrap())
    else {
        skip!()
    };
    let dsr = our_dsr();
    for (name, rom) in [("TI_ROM", ti_rom.to_vec()), ("OUR_ROM", our_rom())] {
        let grom = our_grom();
        let mut m = Machine::new(&rom, &grom);
        m.load_disk_controller(&dsr);
        m.mount_disk(0, tunnels.to_vec());
        m.mount_cartridge(&cart);
        m.reset();
        m.bus_mut().disk.record(true);
        let tap = |m: &mut Machine, k: TiKey, settle: usize| {
            m.set_key(k, true);
            for _ in 0..6 {
                m.run_frame();
            }
            m.set_key(k, false);
            for _ in 0..settle {
                m.run_frame();
            }
        };
        for _ in 0..180 {
            m.run_frame();
        }
        tap(&mut m, TiKey::Space, 40);
        tap(&mut m, TiKey::Num2, 240);
        tap(&mut m, TiKey::Enter, 120);
        tap(&mut m, TiKey::Num2, 120);
        for k in [TiKey::Q, TiKey::U, TiKey::E, TiKey::S, TiKey::T] {
            tap(&mut m, k, 10);
        }
        tap(&mut m, TiKey::Enter, 600);
        let read = m.bus().disk.read_log().to_vec();
        let screen: String = {
            let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
            (0..24 * 32)
                .map(|i| {
                    let c = m.vdp().vram(base + i);
                    if (0x20..0x7F).contains(&c) {
                        c as char
                    } else {
                        ' '
                    }
                })
                .collect()
        };
        assert!(
            read.contains(&85) && read.contains(&135),
            "{name}: our DSR did not read the QUEST data sectors; log {read:?}"
        );
        assert!(
            screen.contains("NEW DUNGEON"),
            "{name}: ToD did not reach its post-load menu under our DSR; screen:\n{screen}"
        );
    }
}

// ---------------------------------------------------------------------------
// M3/M4 write-side differential gates
// ---------------------------------------------------------------------------

/// Compare the final disk images byte-for-byte, reporting the first sector
/// that differs.
fn diff_image(a: &Machine, b: &Machine, what: &str) {
    let (ia, ib) = (a.bus().disk.drive_image(0).unwrap(), b.bus().disk.drive_image(0).unwrap());
    for s in 0..ia.len() / 256 {
        let (sa, sb) = (&ia[s * 256..(s + 1) * 256], &ib[s * 256..(s + 1) * 256]);
        if sa != sb {
            let off = sa.iter().zip(sb).position(|(x, y)| x != y).unwrap();
            let lo = off & !0xF;
            let hi = (lo + 48).min(256);
            panic!(
                "{what}: sector {s} differs first at byte >{off:02X}\nauthentic[{lo:02X}..]: {}\nours     [{lo:02X}..]: {}",
                hex(&sa[lo..hi]),
                hex(&sb[lo..hi])
            );
        }
    }
}

/// VAR write end-to-end: OPEN OUTPUT, two WRITEs, CLOSE — per-op PAB snaps
/// plus the whole resulting image.
#[test]
fn dsr_write_var_matches() {
    let disk = build_disk("PROBE", &[]);
    let p = pab(0, F_VAR | M_OUTPUT, 0x1400, 80, 0, 0, "DSK1.TEST");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .stage(0x1400, b"HELLO")
        .stage(0x1500, b"WORLDLY")
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 3)
        .poke(0x1005, 5)
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1002, 0x15)
        .poke(0x1003, 0x00)
        .poke(0x1005, 7)
        .dev(0x1000)
        .snap(0x1000, 0x1920, 10)
        .poke(0x1000, 1)
        .dev(0x1000);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    for (addr, what) in [(0x1900u16, "OPEN snap"), (0x1910, "WRITE1 snap"), (0x1920, "WRITE2 snap")] {
        diff_vram(&a, &b, addr, 10, what);
    }
    diff_vram(&a, &b, 0x1000, 12, "final PAB");
    diff_word(&a, &b, 0x8354, ">8354");
    diff_word(&a, &b, 0x8356, ">8356");
    diff_image(&a, &b, "VAR write image");
}

/// FIX writes: sequential creation, then a relative rewrite of record 1 and
/// an extension write at record 30 (sparse growth across sectors).
#[test]
fn dsr_write_fix_matches() {
    let disk = build_disk("PROBE", &[]);
    let p = pab(0, M_OUTPUT, 0x1400, 20, 0, 0, "DSK1.FIXED");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .stage(0x1400, b"REC-ZERO")
        .stage(0x1500, b"PATCHED-")
        .dev(0x1000)
        .poke(0x1000, 3)
        .poke(0x1005, 8)
        .dev(0x1000)
        .dev(0x1000)
        .dev(0x1000)
        .poke(0x1000, 1) // CLOSE
        .dev(0x1000)
        // Re-open UPDATE (relative), rewrite record 1, write record 30.
        .poke(0x1000, 0)
        .poke(0x1001, F_REL | M_UPDATE)
        .dev(0x1000)
        .snap(0x1000, 0x1930, 10)
        .poke(0x1000, 3)
        .poke(0x1002, 0x15)
        .poke(0x1003, 0x00)
        .poke(0x1005, 8)
        .poke(0x1007, 1)
        .dev(0x1000)
        .snap(0x1000, 0x1940, 10)
        .poke(0x1007, 30)
        .dev(0x1000)
        .snap(0x1000, 0x1950, 10)
        .poke(0x1000, 1)
        .dev(0x1000);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    for (addr, what) in [(0x1930u16, "reopen snap"), (0x1940, "rewrite snap"), (0x1950, "extend snap")] {
        diff_vram(&a, &b, addr, 10, what);
    }
    diff_vram(&a, &b, 0x1000, 12, "final PAB");
    diff_image(&a, &b, "FIX write image");
}

/// SAVE: a new program, then SAVE-over with a different length.
#[test]
fn dsr_save_matches() {
    let disk = build_disk("PROBE", &[FileSpec::var("AFILE", false, 80, &[b"X"])]);
    let img: Vec<u8> = (0..600u16).map(|i| (i % 199) as u8).collect();
    let p1 = pab(6, 0, 0x1400, 0, 0, 600, "DSK1.PROG");
    let p2 = pab(6, 0, 0x1400, 0, 0, 300, "DSK1.PROG");
    let rig = Rig::new()
        .stage(0x1000, &p1)
        .stage(0x1040, &p2)
        .stage(0x1400, &img)
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .dev(0x1040); // overwrite with the shorter image
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1900, 10, "SAVE snap");
    diff_vram(&a, &b, 0x1040, 10, "re-SAVE PAB");
    diff_image(&a, &b, "SAVE image");
}

/// DELETE: an existing file (image effects) and a missing one (the error).
#[test]
fn dsr_delete_matches() {
    let disk = build_disk(
        "PROBE",
        &[
            FileSpec::var("AAA", false, 80, &[b"1"]),
            FileSpec::var("BBB", false, 80, &[b"22"]),
            FileSpec::var("CCC", false, 80, &[b"333"]),
        ],
    );
    let p = pab(7, 0, 0, 0, 0, 0, "DSK1.BBB");
    let pm = pab(7, 0, 0, 0, 0, 0, "DSK1.NADA");
    let rig = Rig::new().stage(0x1000, &p).stage(0x1040, &pm).dev(0x1000).dev(0x1040);
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1000, 10, "DELETE existing PAB");
    diff_vram(&a, &b, 0x1040, 10, "DELETE missing PAB");
    diff_image(&a, &b, "DELETE image");
}

/// Open modes: APPEND to an existing VAR file (the reopened-tail flush),
/// OUTPUT-truncate of an existing file, UPDATE on a missing file, and a
/// create with record length 0 (the default-80 fill).
#[test]
fn dsr_open_modes_match() {
    let disk = build_disk("PROBE", &[FileSpec::var("VARF", false, 80, &[b"ALPHA", b"BETA"])]);
    // APPEND "GAMMA-XYZ" to VARF, close; then OUTPUT-truncate VARF and write
    // "FRESH"; then UPDATE a missing file; then create with reclen 0.
    let pa = pab(0, F_VAR | M_APPEND, 0x1400, 80, 0, 0, "DSK1.VARF");
    let pu = pab(0, F_VAR | M_UPDATE, 0x1400, 80, 0, 0, "DSK1.NOPE");
    let pc = pab(0, F_VAR | M_OUTPUT, 0x1400, 0, 0, 0, "DSK1.DEF80");
    let rig = Rig::new()
        .stage(0x1000, &pa)
        .stage(0x1040, &pu)
        .stage(0x1080, &pc)
        .stage(0x1400, b"GAMMA-XYZ")
        .stage(0x1500, b"FRESH")
        .dev(0x1000) // APPEND open
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 3)
        .poke(0x1005, 9)
        .dev(0x1000) // WRITE GAMMA-XYZ
        .poke(0x1000, 1)
        .dev(0x1000) // CLOSE
        .poke(0x1000, 0) // reopen OUTPUT (truncate)
        .poke(0x1001, F_VAR | M_OUTPUT)
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1000, 3)
        .poke(0x1002, 0x15)
        .poke(0x1003, 0x00)
        .poke(0x1005, 5)
        .dev(0x1000) // WRITE FRESH
        .poke(0x1000, 1)
        .dev(0x1000) // CLOSE
        .dev(0x1040) // UPDATE on missing
        .dev(0x1080) // create with reclen 0
        .snap(0x1080, 0x1920, 10)
        .poke(0x1080, 1)
        .dev(0x1080); // CLOSE it
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1900, 10, "APPEND open snap");
    diff_vram(&a, &b, 0x1910, 10, "truncate open snap");
    diff_vram(&a, &b, 0x1040, 10, "UPDATE-missing PAB");
    diff_vram(&a, &b, 0x1920, 10, "reclen-0 create snap");
    diff_image(&a, &b, "modes image");
}

/// PROTECT + RENAME subprograms: set protection (visible via STATUS), rename
/// (visible via the catalog + lookups), both against the authentic DSR.
#[test]
fn dsr_protect_rename_match() {
    let disk = build_disk(
        "PROBE",
        &[FileSpec::var("AAA", false, 80, &[b"1"]), FileSpec::var("ZZZ", false, 80, &[b"9"])],
    );
    let ps = pab(9, 0, 0x1080, 0, 0, 0, "DSK1.AAA");
    let rig = Rig::new()
        .stage(0x1000, &ps)
        .stage(0x1200, b"AAA       ") // old name (10 bytes, VDP)
        .stage(0x1210, b"MMM       ") // new name
        // PROTECT AAA (flag >FF).
        .sub(0x12, &[(0x834C, 1), (0x834D, 0xFF), (0x834E, 0x12), (0x834F, 0x00)])
        .dev(0x1000) // STATUS AAA -> protected bit
        // RENAME AAA -> MMM (old at >8350 -> VDP >1200, new at >834E -> >1210).
        .sub(
            0x13,
            &[(0x834C, 1), (0x834E, 0x12), (0x834F, 0x10), (0x8350, 0x12), (0x8351, 0x00)],
        );
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1000, 10, "STATUS-after-PROTECT PAB");
    // >8350 after RENAME is an internal scratch artifact under the authentic
    // DSR (observed >20xx on success) — the image is the real contract.
    diff_image(&a, &b, "protect/rename image");
}

/// FILEIN/FILEOUT (>14/>15): info request + sector reads of a program file;
/// FILEOUT overwrite of one sector.
#[test]
fn dsr_fileio_subs_match() {
    let img: Vec<u8> = (0..512u16).map(|i| (i % 97) as u8).collect();
    let disk = build_disk("PROBE", &[FileSpec::program("PROG", &img)]);
    let patch = [0xA5u8; 256];
    let rig = Rig::new()
        .stage(0x1200, b"PROG      ")
        .stage(0x1600, &patch)
        // Info request: N=0, block at >8300+>26 = >8326 (neutral — a block inside
        // either DSR's own >834A->836D workspace is out of contract; buf >1400).
        .sub(
            0x14,
            &[
                (0x834C, 1),
                (0x834D, 0),
                (0x834E, 0x12),
                (0x834F, 0x00),
                (0x8350, 0x26),
                (0x8326, 0x14),
                (0x8327, 0x00),
                (0x8328, 0x00),
                (0x8329, 0x00),
            ],
        )
        // Read 2 sectors to >1400.
        .sub(
            0x14,
            &[
                (0x834C, 1),
                (0x834D, 2),
                (0x834E, 0x12),
                (0x834F, 0x00),
                (0x8350, 0x26),
                (0x8326, 0x14),
                (0x8327, 0x00),
                (0x8328, 0x00),
                (0x8329, 0x00),
            ],
        )
        // FILEOUT: overwrite file sector 1 from >1600.
        .sub(
            0x15,
            &[
                (0x834C, 1),
                (0x834D, 1),
                (0x834E, 0x12),
                (0x834F, 0x00),
                (0x8350, 0x26),
                (0x8326, 0x16),
                (0x8327, 0x00),
                (0x8328, 0x00),
                (0x8329, 0x01),
            ],
        );
    let Some((a, b)) = differential(&[(0, disk)], &rig) else { skip!() };
    // The info block lands in scratchpad: compare >8326..>8330.
    for off in 0..10u16 {
        assert_eq!(
            a.bus().peek(0x8326 + off),
            b.bus().peek(0x8326 + off),
            "FILEIN info block byte {off} differs (authentic={:02X} ours={:02X})",
            a.bus().peek(0x8326 + off),
            b.bus().peek(0x8326 + off)
        );
    }
    diff_vram(&a, &b, 0x1400, 512, "FILEIN payload");
    diff_word(&a, &b, 0x8350, "FILEOUT error cell");
    diff_image(&a, &b, "FILEOUT image");
}

/// **Cross-oracle interop**: a disk written by OUR DSR is read back — catalog
/// and contents — by the AUTHENTIC DSR (and the reverse). The strongest
/// on-disk-format gate (plan §4 P2).
#[test]
fn dsr_cross_oracle_write() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let write_rig = |_: ()| {
        let p = pab(0, F_VAR | M_OUTPUT, 0x1400, 80, 0, 0, "DSK1.NOTE");
        let ps = pab(6, 0, 0x1500, 0, 0, 300, "DSK1.CODE");
        Rig::new()
            .stage(0x1000, &p)
            .stage(0x1040, &ps)
            .stage(0x1400, b"FIRST-LINE")
            .stage(0x1500, &(0..300u16).map(|i| (i % 89) as u8).collect::<Vec<_>>())
            .dev(0x1000)
            .poke(0x1000, 3)
            .poke(0x1005, 10)
            .dev(0x1000)
            .poke(0x1000, 1)
            .dev(0x1000)
            .dev(0x1040)
    };
    let read_rig = || {
        let po = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK1.NOTE");
        let pl = pab(5, 0, 0x1600, 0, 0, 0x1000, "DSK1.CODE");
        Rig::new()
            .stage(0x1000, &po)
            .stage(0x1080, &pl)
            .dev(0x1000)
            .poke(0x1000, 2)
            .dev(0x1000)
            .snap(0x1000, 0x1900, 10)
            .dev(0x1080)
    };
    for (writer_name, writer, reader) in
        [("ours->authentic", our_dsr(), ti_dsr.to_vec()), ("authentic->ours", ti_dsr.to_vec(), our_dsr())]
    {
        let disk = build_disk("XCHG", &[]);
        let Some(m1) = run_rig(&writer, &[(0, disk)], &write_rig(())) else { skip!() };
        let written = m1.bus().disk.drive_image(0).unwrap().to_vec();
        let Some(m2) = run_rig(&reader, &[(0, written)], &read_rig()) else { skip!() };
        assert_eq!(
            vram(&m2, 0x1400, 10),
            b"FIRST-LINE".to_vec(),
            "{writer_name}: the VAR record did not read back"
        );
        assert_eq!(m2.vdp().vram(0x1905), 10, "{writer_name}: record length wrong");
        let code = vram(&m2, 0x1600, 300);
        let want: Vec<u8> = (0..300u16).map(|i| (i % 89) as u8).collect();
        assert_eq!(code, want, "{writer_name}: the program did not LOAD back");
    }
}

/// FORMAT: ours re-initializes a junk image in place; the AUTHENTIC DSR then
/// catalogs it and SAVEs a program onto it (the cross-oracle validation —
/// authentic FORMAT itself cannot run on this card, RECON §7).
#[test]
fn dsr_format_cross_oracle() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let junk: Vec<u8> = (0..360 * 256).map(|i| (i * 7 % 251) as u8).collect();
    let rig = Rig::new().sub(0x11, &[(0x834C, 1), (0x834D, 40), (0x8350, 1), (0x8351, 1)]);
    let Some(m1) = run_rig(&our_dsr(), &[(0, junk)], &rig) else { skip!() };
    assert_eq!(m1.bus().peek(0x8350), 0, "FORMAT reported an error");
    assert_eq!(m1.bus().peek_word(0x834A), 360, "FORMAT total-sectors echo");
    let formatted = m1.bus().disk.drive_image(0).unwrap().to_vec();
    // The authentic DSR now uses the disk we formatted.
    let ps = pab(6, 0, 0x1400, 0, 0, 200, "DSK1.SAVED");
    let pl = pab(5, 0, 0x1600, 0, 0, 0x1000, "DSK1.SAVED");
    let rig2 = Rig::new()
        .stage(0x1000, &ps)
        .stage(0x1040, &pl)
        .stage(0x1400, &[0x5A; 200])
        .dev(0x1000)
        .dev(0x1040);
    let Some(m2) = run_rig(ti_dsr, &[(0, formatted)], &rig2) else { skip!() };
    assert_eq!(
        vram(&m2, 0x1600, 200),
        vec![0x5A; 200],
        "the authentic DSR could not save+load on our formatted disk"
    );
}

// ---------------------------------------------------------------------------
// M5 robustness gates
// ---------------------------------------------------------------------------

/// An empty drive, an unformatted disk, and a protected file — every case
/// must degrade exactly like the authentic DSR, and never hang.
#[test]
fn dsr_robustness_matches() {
    // OPEN on a drive with nothing mounted.
    let disk = build_disk("PROBE", &[FileSpec::var("VARF", false, 80, &[b"X"])]);
    let p = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK2.VARF");
    let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
    let Some((a, b)) = differential(&[(0, disk.clone())], &rig) else { skip!() };
    diff_vram(&a, &b, 0x1000, 10, "empty-drive OPEN PAB");

    // A completely unformatted (all-zero) disk: catalog open + STATUS.
    let blank = vec![0u8; 360 * 256];
    let pc = pab(0, F_INT | M_INPUT, 0x1400, 38, 0, 0, "DSK1.");
    let ps = pab(9, 0, 0x1080, 0, 0, 0, "DSK1.ANY");
    let rig2 = Rig::new()
        .stage(0x1000, &pc)
        .stage(0x1040, &ps)
        .dev(0x1000)
        .poke(0x1000, 2)
        .dev(0x1000)
        .dev(0x1040);
    let Some((a2, b2)) = differential(&[(0, blank)], &rig2) else { skip!() };
    diff_vram(&a2, &b2, 0x1000, 10, "unformatted catalog PAB");
    diff_vram(&a2, &b2, 0x1040, 10, "unformatted STATUS PAB");

    // WRITE to a protected file.
    let mut spec = FileSpec::fixed("LOCKED", false, 20, &[b"REC0"]);
    spec.protected = true;
    let disk3 = build_disk("PROBE", &[spec]);
    let pw = pab(0, F_REL | M_UPDATE, 0x1400, 20, 0, 0, "DSK1.LOCKED");
    let rig3 = Rig::new()
        .stage(0x1000, &pw)
        .stage(0x1400, b"NEWREC")
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 3)
        .poke(0x1005, 6)
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1000, 7) // DELETE the protected file
        .dev(0x1000);
    let Some((a3, b3)) = differential(&[(0, disk3)], &rig3) else { skip!() };
    diff_vram(&a3, &b3, 0x1900, 10, "protected OPEN snap");
    diff_vram(&a3, &b3, 0x1910, 10, "protected WRITE snap");
    diff_vram(&a3, &b3, 0x1000, 10, "protected DELETE PAB");
    diff_image(&a3, &b3, "protected image");
}

/// The committed artifact must match the source build (the staleness gate,
/// house pattern).
#[test]
fn committed_disk_dsr_artifact_is_fresh() {
    let committed: &[u8] =
        include_bytes!("../../../original-content/system-roms/disk-dsr/disk-dsr.bin");
    assert_eq!(
        committed,
        our_dsr().as_slice(),
        "disk-dsr.bin is stale — rebuild it: cargo run -p libre99-asm --bin libre99asm -- dsr original-content/system-roms/disk-dsr/disk-dsr.bin"
    );
}

// ---------------------------------------------------------------------------
// D1 probes (authentic DSR = the empirical spec). All #[ignore]d; run with
// --ignored --nocapture and read the output.
// ---------------------------------------------------------------------------

/// Debug probe: run STATUS under OUR DSR alone and dump the trail.
#[test]
#[ignore]
fn probe_ours_status_trace() {
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let p = pab(9, 0, 0x1080, 0, 0, 0, "DSK1.QUEST");
    let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
    let Some(m) = run_rig(&our_dsr(), &[(0, tunnels.to_vec())], &rig) else { skip!() };
    println!("read log: {:?}", m.bus().disk.read_log());
    println!("PAB: {}", hex(&vram(&m, 0x1000, 24)));
    println!("FNAME cells >8360..: {}", hex(&(0..10).map(|i| m.bus().peek(0x8360 + i)).collect::<Vec<_>>()));
    println!(
        ">834C drive={:02X} >8354={:04X} >8356={:04X}",
        m.bus().peek(0x834C),
        m.bus().peek_word(0x8354),
        m.bus().peek_word(0x8356)
    );
    println!("bufB FDIR >3F00: {}", hex(&vram(&m, 0x3F00, 8)));
    println!("bufA FDR  >3E00: {}", hex(&vram(&m, 0x3E00, 16)));
}

/// Debug probe: OPEN-create under OUR DSR alone.
#[test]
#[ignore]
fn probe_ours_create_trace() {
    let disk = build_disk("PROBE", &[]);
    let p = pab(0, F_VAR | M_OUTPUT, 0x1400, 80, 0, 0, "DSK1.NEW");
    let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
    let Some(m) = run_rig(&our_dsr(), &[(0, disk.clone())], &rig) else { skip!() };
    println!("read log: {:?}", m.bus().disk.read_log());
    println!("PAB: {}", hex(&vram(&m, 0x1000, 12)));
    let after = m.bus().disk.drive_image(0).unwrap();
    for s in 0..360 {
        if disk[s * 256..(s + 1) * 256] != after[s * 256..(s + 1) * 256] {
            println!("== sector {s} changed ==\n{}", hex(&after[s * 256..s * 256 + 48]));
        }
    }
}

/// Debug probe: VAR create+write+close under OUR DSR alone, with the FD trace.
#[test]
#[ignore]
fn probe_ours_close_trace() {
    let disk = build_disk("PROBE", &[]);
    let p = pab(0, F_VAR | M_OUTPUT, 0x1400, 80, 0, 0, "DSK1.TEST");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .stage(0x1400, b"HELLO")
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 3)
        .poke(0x1005, 5)
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1000, 1)
        .dev(0x1000);
    let Some(m) = run_rig(&our_dsr(), &[(0, disk.clone())], &rig) else { skip!() };
    println!("OPEN snap:  {}", hex(&vram(&m, 0x1900, 10)));
    println!("WRITE snap: {}", hex(&vram(&m, 0x1910, 10)));
    println!("final PAB:  {}", hex(&vram(&m, 0x1000, 12)));
    println!("read log: {:?}", m.bus().disk.read_log());
    // The last 30 register/CRU trace entries show where the CLOSE died.
    let tr = m.bus().disk.trace();
    for (k, a, v) in tr.iter().rev().take(30).rev() {
        println!("{} {:04X} {:02X}", *k as char, a, v);
    }
    let after = m.bus().disk.drive_image(0).unwrap();
    for s in 0..360 {
        if disk[s * 256..(s + 1) * 256] != after[s * 256..(s + 1) * 256] {
            println!("== sector {s} changed ==\n{}", hex(&after[s * 256..s * 256 + 48]));
        }
    }
    // The slot region: control (top+40), the FDR copy head, the data head.
    let top = m.bus().peek_word(0x8370);
    println!("slot ctl  @top+40: {}", hex(&vram(&m, top + 40, 16)));
    println!("slot copy @+6:     {}", hex(&vram(&m, top + 46, 32)));
    println!("slot data @+254:   {}", hex(&vram(&m, top + 40 + 254, 16)));
    println!("buffer A  >3E00:   {}", hex(&vram(&m, 0x3E00, 32)));
}

/// Debug probe: the SAVE + re-SAVE flow under OUR DSR alone.
#[test]
#[ignore]
fn probe_ours_save_trace() {
    let disk = build_disk("PROBE", &[FileSpec::var("AFILE", false, 80, &[b"X"])]);
    let img: Vec<u8> = (0..600u16).map(|i| (i % 199) as u8).collect();
    let p1 = pab(6, 0, 0x1400, 0, 0, 600, "DSK1.PROG");
    let p2 = pab(6, 0, 0x1400, 0, 0, 300, "DSK1.PROG");
    let rig = Rig::new()
        .stage(0x1000, &p1)
        .stage(0x1040, &p2)
        .stage(0x1400, &img)
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .dev(0x1040);
    let Some(m) = run_rig(&our_dsr(), &[(0, disk)], &rig) else { skip!() };
    println!("save1 snap: {}", hex(&vram(&m, 0x1900, 10)));
    println!("save2 PAB:  {}", hex(&vram(&m, 0x1040, 10)));
    println!("read log: {:?}", m.bus().disk.read_log());
    let a = m.bus().disk.drive_image(0).unwrap();
    println!("bitmap: {}", hex(&a[0x38..0x48]));
    println!("FDIR:   {}", hex(&a[256..256 + 12]));
    for s in [2usize, 3, 4] {
        println!("s{s}: {}", hex(&a[s * 256..s * 256 + 34]));
    }
}

/// Debug probe: the open-modes truncate flow under OUR DSR alone.
#[test]
#[ignore]
fn probe_ours_modes_trace() {
    let disk = build_disk("PROBE", &[FileSpec::var("VARF", false, 80, &[b"ALPHA", b"BETA"])]);
    let pa = pab(0, F_VAR | M_APPEND, 0x1400, 80, 0, 0, "DSK1.VARF");
    let rig = Rig::new()
        .stage(0x1000, &pa)
        .stage(0x1400, b"GAMMA-XYZ")
        .stage(0x1500, b"FRESH")
        .dev(0x1000)
        .poke(0x1000, 3)
        .poke(0x1005, 9)
        .dev(0x1000)
        .poke(0x1000, 1)
        .dev(0x1000)
        .poke(0x1000, 0)
        .poke(0x1001, F_VAR | M_OUTPUT)
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1000, 3)
        .poke(0x1002, 0x15)
        .poke(0x1003, 0x00)
        .poke(0x1005, 5)
        .dev(0x1000)
        .poke(0x1000, 1)
        .dev(0x1000);
    let Some(m) = run_rig(&our_dsr(), &[(0, disk)], &rig) else { skip!() };
    println!("truncate-open snap: {}", hex(&vram(&m, 0x1910, 10)));
    println!("final PAB: {}", hex(&vram(&m, 0x1000, 10)));
    println!("read log: {:?}", m.bus().disk.read_log());
    let a = m.bus().disk.drive_image(0).unwrap();
    println!("bitmap: {}", hex(&a[0x38..0x44]));
    println!("FDR s2: {}", hex(&a[2 * 256..2 * 256 + 34]));
    println!("s34: {}", hex(&a[34 * 256..34 * 256 + 24]));
    println!("s35: {}", hex(&a[35 * 256..35 * 256 + 24]));
}

/// Ground truth straight from a real TI-formatted disk: the VIB (incl. the
/// bitmap tail convention), the FDIR, and each file's FDR.
#[test]
#[ignore]
fn probe_tunnels_layout() {
    let Some(img) = TUNNELS.as_deref() else { skip!() };
    println!("== VIB (sector 0) ==\n{}", hex(&img[..256]));
    println!("== FDIR (sector 1, first 32 bytes) ==\n{}", hex(&img[256..288]));
    for i in 0..8 {
        let ptr = ((img[256 + 2 * i] as usize) << 8) | img[256 + 2 * i + 1] as usize;
        if ptr == 0 {
            break;
        }
        let fdr = &img[ptr * 256..ptr * 256 + 64];
        println!(
            "== FDR @sector {ptr} name={:?} ==\n{}",
            String::from_utf8_lossy(&fdr[..10]),
            hex(fdr)
        );
    }
}

/// Rig smoke test + STATUS ground truth: STATUS (op 9) on DSK1.QUEST.
/// Validates the whole rig (power-up, PAB staging, DSRLNK, skip-return).
#[test]
#[ignore]
fn probe_rig_smoke_status() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let p = pab(9, 0, 0x1080, 0, 0, 0, "DSK1.QUEST");
    let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
    let Some(m) = run_rig(ti_dsr, &[(0, tunnels.to_vec())], &rig) else { skip!() };
    println!("done={:02X} notfound={:02X}", m.bus().peek(DONE_CELL), m.bus().peek(NOTFOUND_CELL));
    println!("PAB after: {}", hex(&vram(&m, 0x1000, 24)));
    println!(
        ">8350={:02X} >8354={:04X} >8356={:04X} >83D0={:04X}",
        m.bus().peek(0x8350),
        m.bus().peek_word(0x8354),
        m.bus().peek_word(0x8356),
        m.bus().peek_word(0x83D0)
    );
    // What does >8356 point at after the call?
    let p = m.bus().peek_word(0x8356);
    println!("VDP @>8356 [{p:04X}]: {}", hex(&vram(&m, p.saturating_sub(4), 20)));
    // STATUS on a missing file too.
    let p2 = pab(9, 0, 0x1080, 0, 0, 0, "DSK1.NOFILE");
    let rig2 = Rig::new().stage(0x1000, &p2).dev(0x1000);
    let Some(m2) = run_rig(ti_dsr, &[(0, tunnels.to_vec())], &rig2) else { skip!() };
    println!("missing-file PAB after: {}", hex(&vram(&m2, 0x1000, 24)));
}

/// Boot only (power-up): the VRAM buffer header's exact bytes.
#[test]
#[ignore]
fn probe_powerup_vram() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let rig = Rig::new();
    let Some(m) = run_rig(ti_dsr, &[(0, tunnels.to_vec())], &rig) else { skip!() };
    println!(">8370={:04X}", m.bus().peek_word(0x8370));
    println!("VRAM >37D0..>3800:\n{}", hex(&vram(&m, 0x37D0, 0x30)));
    println!("VRAM >3FE0..>4000:\n{}", hex(&vram(&m, 0x3FE0, 0x20)));
}

/// The catalog: OPEN "DSK1." INT/FIX 38 INPUT, READ x5, CLOSE — record layout,
/// radix-100 numbers, the end marker, per-read charcount.
#[test]
#[ignore]
fn probe_catalog() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let p = pab(0, F_INT | M_INPUT, 0x1400, 38, 0, 0, "DSK1.");
    // One PAB per READ (fresh buffer each) so every result stays inspectable.
    let mut rig = Rig::new().stage(0x1000, &p);
    rig = rig.dev(0x1000); // OPEN
    for i in 0..5u16 {
        let buf = 0x1400 + i * 0x40;
        let pr = pab(2, F_INT | M_INPUT, buf, 38, 0, 0, "DSK1.");
        rig = rig.stage(0x1040 + i * 0x20, &pr).dev(0x1040 + i * 0x20);
    }
    let pc = pab(1, F_INT | M_INPUT, 0x1400, 38, 0, 0, "DSK1."); // CLOSE
    rig = rig.stage(0x1200, &pc).dev(0x1200);
    let Some(m) = run_rig(ti_dsr, &[(0, tunnels.to_vec())], &rig) else { skip!() };
    println!("OPEN PAB after: {}", hex(&vram(&m, 0x1000, 16)));
    for i in 0..5u16 {
        println!(
            "READ{} PAB {} | rec: {}",
            i,
            hex(&vram(&m, 0x1040 + i * 0x20, 10)),
            hex(&vram(&m, 0x1400 + i * 0x40, 38))
        );
    }
    println!("CLOSE PAB after: {}", hex(&vram(&m, 0x1200, 16)));
}

/// ToD's actual load: run the real cartridge flow under the authentic DSR,
/// then read the PAB it staged (via the >8354 return side-effect) — pins the
/// opcode/flags/name ToD uses and QUEST's type end-to-end.
#[test]
#[ignore]
fn probe_tod_pab() {
    use libre99_core::cartridge::Cartridge;
    use libre99_core::keyboard::TiKey;
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let Some(cart) = ["cartridges/tundoom.ctg", "cartridges/tunnelsofdoom.ctg"]
        .iter()
        .find_map(|p| libre99_core::third_party::load(p))
        .map(|d| Cartridge::parse(&d).unwrap())
    else {
        skip!()
    };
    let grom = our_grom();
    let mut m = Machine::new(ti_rom, &grom);
    m.load_disk_controller(ti_dsr);
    m.mount_disk(0, tunnels.to_vec());
    m.mount_cartridge(&cart);
    m.reset();
    let tap = |m: &mut Machine, k: TiKey, settle: usize| {
        m.set_key(k, true);
        for _ in 0..6 {
            m.run_frame();
        }
        m.set_key(k, false);
        for _ in 0..settle {
            m.run_frame();
        }
    };
    for _ in 0..180 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 40);
    tap(&mut m, TiKey::Num2, 240);
    tap(&mut m, TiKey::Enter, 120);
    tap(&mut m, TiKey::Num2, 120);
    for k in [TiKey::Q, TiKey::U, TiKey::E, TiKey::S, TiKey::T] {
        tap(&mut m, k, 10);
    }
    tap(&mut m, TiKey::Enter, 600);
    let pabp = m.bus().peek_word(0x8354);
    println!(">8354={pabp:04X} >8356={:04X}", m.bus().peek_word(0x8356));
    println!("PAB region: {}", hex(&vram(&m, pabp, 32)));
}

/// Write scenario, DIS/VAR: OPEN OUTPUT, WRITE "HELLO", WRITE "WORLDLY",
/// CLOSE — then dump every changed sector (bitmap, FDIR, FDR, data) of the
/// image. The empirical write/alloc/FDR spec.
#[test]
#[ignore]
fn probe_write_var() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk("PROBE", &[]);
    let p = pab(0, F_VAR | M_OUTPUT, 0x1400, 80, 0, 0, "DSK1.TEST");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .stage(0x1400, b"HELLO")
        .stage(0x1500, b"WORLDLY")
        .dev(0x1000) // OPEN
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 3) // WRITE
        .poke(0x1005, 5) // count
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1002, 0x15) // buffer -> >1500
        .poke(0x1003, 0x00)
        .poke(0x1005, 7)
        .dev(0x1000)
        .snap(0x1000, 0x1920, 10)
        .poke(0x1000, 1) // CLOSE
        .dev(0x1000);
    let Some(m) = run_rig(ti_dsr, &[(0, disk.clone())], &rig) else { skip!() };
    println!("OPEN snap:  {}", hex(&vram(&m, 0x1900, 10)));
    println!("WRITE1 snap:{}", hex(&vram(&m, 0x1910, 10)));
    println!("WRITE2 snap:{}", hex(&vram(&m, 0x1920, 10)));
    println!("final PAB:  {}", hex(&vram(&m, 0x1000, 20)));
    let after = m.bus().disk.drive_image(0).unwrap();
    for s in 0..360 {
        if disk[s * 256..(s + 1) * 256] != after[s * 256..(s + 1) * 256] {
            println!("== sector {s} changed ==\n{}", hex(&after[s * 256..s * 256 + 64]));
            if s == 0 {
                println!("bitmap >38..: {}", hex(&after[0x38..0x60]));
            }
        }
    }
    println!(">8354={:04X} >8356={:04X}", m.bus().peek_word(0x8354), m.bus().peek_word(0x8356));
    let p6 = m.bus().peek_word(0x8356);
    println!("VDP @>8356: {}", hex(&vram(&m, p6, 16)));
}

/// Write scenario, DIS/FIX: 3 records of a FIX/20 file (incl. a rewrite of
/// record 1) — record-count and EOF conventions for FIXED files.
#[test]
#[ignore]
fn probe_write_fix() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk("PROBE", &[]);
    let p = pab(0, M_OUTPUT, 0x1400, 20, 0, 0, "DSK1.FIXED");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .stage(0x1400, b"REC-ZERO")
        .dev(0x1000) // OPEN OUTPUT
        .poke(0x1000, 3)
        .poke(0x1005, 8)
        .dev(0x1000) // WRITE rec (seq 0)
        .dev(0x1000) // WRITE rec (seq 1)
        .dev(0x1000) // WRITE rec (seq 2)
        .poke(0x1000, 1)
        .dev(0x1000); // CLOSE
    let Some(m) = run_rig(ti_dsr, &[(0, disk.clone())], &rig) else { skip!() };
    println!("final PAB: {}", hex(&vram(&m, 0x1000, 20)));
    let after = m.bus().disk.drive_image(0).unwrap();
    for s in 0..360 {
        if disk[s * 256..(s + 1) * 256] != after[s * 256..(s + 1) * 256] {
            println!("== sector {s} changed ==\n{}", hex(&after[s * 256..s * 256 + 96]));
        }
    }
}

/// SAVE (op 6): a 300-byte program image — PROGRAM-file FDR conventions.
#[test]
#[ignore]
fn probe_save_program() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk("PROBE", &[]);
    let img: Vec<u8> = (0..300u16).map(|i| i as u8).collect();
    let p = pab(6, 0, 0x1400, 0, 0, 300, "DSK1.PROG");
    let rig = Rig::new().stage(0x1000, &p).stage(0x1400, &img).dev(0x1000);
    let Some(m) = run_rig(ti_dsr, &[(0, disk.clone())], &rig) else { skip!() };
    println!("final PAB: {}", hex(&vram(&m, 0x1000, 20)));
    let after = m.bus().disk.drive_image(0).unwrap();
    for s in 0..360 {
        if disk[s * 256..(s + 1) * 256] != after[s * 256..(s + 1) * 256] {
            println!("== sector {s} changed ==\n{}", hex(&after[s * 256..s * 256 + 64]));
        }
    }
}

/// LOAD (op 5) of a program file authored by our Rust builder — validates the
/// builder against the authentic DSR and pins LOAD's PAB conventions.
#[test]
#[ignore]
fn probe_load_program() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let img: Vec<u8> = (0..600u16).map(|i| (i % 251) as u8).collect();
    let disk = build_disk("PROBE", &[FileSpec::program("PROG", &img)]);
    let p = pab(5, 0, 0x1400, 0, 0, 0x1000, "DSK1.PROG");
    let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
    let Some(m) = run_rig(ti_dsr, &[(0, disk)], &rig) else { skip!() };
    println!("final PAB: {}", hex(&vram(&m, 0x1000, 20)));
    let got = vram(&m, 0x1400, 600);
    println!("loaded == authored: {}", got == img);
    println!("first 32: {}", hex(&got[..32]));
    println!("read log: {:?}", m.bus().disk.read_log());
}

/// Sequential VAR + relative FIX reads of builder-authored files — validates
/// the builder's record packing against the authentic DSR.
#[test]
#[ignore]
fn probe_read_builder_files() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk(
        "PROBE",
        &[
            FileSpec::var("VARF", false, 80, &[b"ALPHA", b"BETA-BETA", b"GAMMA"]),
            FileSpec::fixed("FIXF", false, 20, &[b"REC0", b"REC1", b"REC2"]),
        ],
    );
    // VAR sequential: OPEN INPUT, READ x4 (4th -> EOF), CLOSE.
    let p = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK1.VARF");
    let mut rig = Rig::new().stage(0x1000, &p).dev(0x1000).snap(0x1000, 0x1900, 10);
    for i in 0..4u16 {
        let buf = 0x1440 + i * 0x40;
        rig = rig
            .poke(0x1000, 2)
            .poke(0x1002, (buf >> 8) as u8)
            .poke(0x1003, buf as u8)
            .dev(0x1000)
            .snap(0x1000, 0x1910 + i * 16, 10);
    }
    // FIX relative: READ record 2 then record 0.
    let pf = pab(0, F_REL | M_INPUT, 0x1700, 20, 0, 0, "DSK1.FIXF");
    rig = rig.stage(0x1100, &pf).dev(0x1100);
    rig = rig.poke(0x1100, 2).poke(0x1107, 2).dev(0x1100).snap(0x1100, 0x1960, 10);
    rig = rig.poke(0x1103, 0x40).poke(0x1107, 0).dev(0x1100).snap(0x1100, 0x1970, 10);
    let Some(m) = run_rig(ti_dsr, &[(0, disk)], &rig) else { skip!() };
    println!("VAR OPEN snap: {}", hex(&vram(&m, 0x1900, 10)));
    for i in 0..4u16 {
        println!(
            "VAR READ{i} snap {} | buf {}",
            hex(&vram(&m, 0x1910 + i * 16, 10)),
            hex(&vram(&m, 0x1440 + i * 0x40, 12))
        );
    }
    println!("FIX READ rec2 snap {} | buf {}", hex(&vram(&m, 0x1960, 10)), hex(&vram(&m, 0x1700, 20)));
    println!("FIX READ rec0 snap {} | buf {}", hex(&vram(&m, 0x1970, 10)), hex(&vram(&m, 0x1740, 20)));
}

/// The SECTOR subprogram (>10): read sector 1, then write a pattern to sector
/// 40 — parameter/result cells and the >834A/>8350 ambiguity, settled.
#[test]
#[ignore]
fn probe_sector_sub() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk("PROBE", &[]);
    let rig = Rig::new()
        .stage(0x1400, &[0xEE; 8]) // pattern that must land in sector 40
        // READ sector 1 into >1200: unit 1, rw=1 (read), buf >1200, sector 1.
        .sub(
            0x10,
            &[
                (0x834C, 1),
                (0x834D, 1),
                (0x834E, 0x12),
                (0x834F, 0x00),
                (0x8350, 0x00),
                (0x8351, 0x01),
            ],
        )
        // WRITE >1400.. to sector 40: rw=0.
        .sub(
            0x10,
            &[
                (0x834C, 1),
                (0x834D, 0),
                (0x834E, 0x14),
                (0x834F, 0x00),
                (0x8350, 0x00),
                (0x8351, 40),
            ],
        );
    let Some(m) = run_rig(ti_dsr, &[(0, disk.clone())], &rig) else { skip!() };
    println!(
        ">834A={:04X} >834C={:02X} >834D={:02X} >8350={:04X}",
        m.bus().peek_word(0x834A),
        m.bus().peek(0x834C),
        m.bus().peek(0x834D),
        m.bus().peek_word(0x8350)
    );
    println!("sector-1 read -> VDP >1200: {}", hex(&vram(&m, 0x1200, 16)));
    let after = m.bus().disk.drive_image(0).unwrap();
    println!("disk sector 40 after write: {}", hex(&after[40 * 256..40 * 256 + 16]));
    println!("done={:02X} notfound={:02X}", m.bus().peek(DONE_CELL), m.bus().peek(NOTFOUND_CELL));
}

/// FILES (>16) with n=2: how >8370 and the buffer header move.
#[test]
#[ignore]
fn probe_files_sub() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let Some(tunnels) = TUNNELS.as_deref() else { skip!() };
    let rig = Rig::new().sub(0x16, &[(0x834C, 2)]);
    let Some(m) = run_rig(ti_dsr, &[(0, tunnels.to_vec())], &rig) else { skip!() };
    println!(">8370={:04X} >8350={:02X}", m.bus().peek_word(0x8370), m.bus().peek(0x8350));
    let top = m.bus().peek_word(0x8370);
    println!("header @top+1: {}", hex(&vram(&m, top + 1, 8)));
}

/// Error matrix: OPEN INPUT on a missing file; OPEN with a mismatched record
/// length; WRITE to a protected disk; OPEN FIX on a VAR file.
#[test]
#[ignore]
fn probe_errors() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let mut disk = build_disk("PROBE", &[FileSpec::var("VARF", false, 80, &[b"X"])]);
    // OPEN INPUT missing.
    let p1 = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK1.NOFILE");
    // OPEN with wrong reclen (40 vs 80).
    let p2 = pab(0, F_VAR | M_INPUT, 0x1400, 40, 0, 0, "DSK1.VARF");
    // OPEN FIX on the VAR file.
    let p3 = pab(0, M_INPUT, 0x1400, 80, 0, 0, "DSK1.VARF");
    // Correct reclen 0 -> filled in?
    let p4 = pab(0, F_VAR | M_INPUT, 0x1400, 0, 0, 0, "DSK1.VARF");
    let rig = Rig::new()
        .stage(0x1000, &p1)
        .stage(0x1040, &p2)
        .stage(0x1080, &p3)
        .stage(0x10C0, &p4)
        .dev(0x1000)
        .dev(0x1040)
        .dev(0x1080)
        .dev(0x10C0);
    let Some(m) = run_rig(ti_dsr, &[(0, disk.clone())], &rig) else { skip!() };
    println!("missing:      {}", hex(&vram(&m, 0x1000, 10)));
    println!("bad reclen:   {}", hex(&vram(&m, 0x1040, 10)));
    println!("fix-on-var:   {}", hex(&vram(&m, 0x1080, 10)));
    println!("reclen 0:     {}", hex(&vram(&m, 0x10C0, 10)));
    // Protected disk: 'P' in the VIB protection byte, then OPEN OUTPUT.
    disk[0x10] = b'P';
    let p5 = pab(0, F_VAR | M_OUTPUT, 0x1400, 80, 0, 0, "DSK1.NEW");
    let rig2 = Rig::new().stage(0x1000, &p5).dev(0x1000);
    let Some(m2) = run_rig(ti_dsr, &[(0, disk)], &rig2) else { skip!() };
    println!("prot output:  {}", hex(&vram(&m2, 0x1000, 10)));
}

/// SCRATCH (op 8) + RESTORE (op 4) semantics on a FIX file.
#[test]
#[ignore]
fn probe_scratch_restore() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk(
        "PROBE",
        &[FileSpec::fixed("FIXF", false, 20, &[b"REC0", b"REC1", b"REC2"])],
    );
    let p = pab(0, F_REL | M_UPDATE, 0x1400, 20, 0, 0, "DSK1.FIXF");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 8) // SCRATCH record 1
        .poke(0x1007, 1)
        .dev(0x1000)
        .snap(0x1000, 0x1910, 10)
        .poke(0x1000, 4) // RESTORE to record 2
        .poke(0x1007, 2)
        .dev(0x1000)
        .snap(0x1000, 0x1920, 10)
        .poke(0x1000, 2) // READ (should be rec 2)
        .dev(0x1000);
    let Some(m) = run_rig(ti_dsr, &[(0, disk)], &rig) else { skip!() };
    println!("OPEN:    {}", hex(&vram(&m, 0x1900, 10)));
    println!("SCRATCH: {}", hex(&vram(&m, 0x1910, 10)));
    println!("RESTORE: {}", hex(&vram(&m, 0x1920, 10)));
    println!("READ:    {} buf {}", hex(&vram(&m, 0x1000, 10)), hex(&vram(&m, 0x1400, 20)));
}

/// The volume-name device form: "DSK.<volume>.<file>".
#[test]
#[ignore]
fn probe_volume_form() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk("MYVOL", &[FileSpec::var("VARF", false, 80, &[b"HELLO-VOL"])]);
    let p = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK.MYVOL.VARF");
    let rig = Rig::new()
        .stage(0x1000, &p)
        .dev(0x1000)
        .snap(0x1000, 0x1900, 10)
        .poke(0x1000, 2)
        .dev(0x1000);
    let Some(m) = run_rig(ti_dsr, &[(0, disk)], &rig) else { skip!() };
    println!("OPEN snap: {}", hex(&vram(&m, 0x1900, 10)));
    println!("READ PAB:  {} buf {}", hex(&vram(&m, 0x1000, 12)), hex(&vram(&m, 0x1400, 12)));
    // And a bad volume.
    let pb = pab(0, F_VAR | M_INPUT, 0x1400, 80, 0, 0, "DSK.NOVOL.VARF");
    let rig2 = Rig::new().stage(0x1000, &pb).dev(0x1000);
    let Some(m2) = run_rig(ti_dsr, &[(0, build_disk("MYVOL", &[]))], &rig2) else { skip!() };
    println!(
        "bad vol:   {} notfound={:02X}",
        hex(&vram(&m2, 0x1000, 10)),
        m2.bus().peek(NOTFOUND_CELL)
    );
}

/// The FDIR search order (binary search?): STATUS a middle file on a 5-file
/// disk and read the FDR probe order out of the sector read log.
#[test]
#[ignore]
fn probe_fdir_search_order() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk(
        "PROBE",
        &[
            FileSpec::var("AAA", false, 80, &[b"1"]),
            FileSpec::var("CCC", false, 80, &[b"2"]),
            FileSpec::var("EEE", false, 80, &[b"3"]),
            FileSpec::var("GGG", false, 80, &[b"4"]),
            FileSpec::var("III", false, 80, &[b"5"]),
        ],
    );
    for name in ["DSK1.GGG", "DSK1.AAA", "DSK1.ZZZ"] {
        let p = pab(9, 0, 0x1080, 0, 0, 0, name);
        let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
        let Some(m) = run_rig(ti_dsr, &[(0, disk.clone())], &rig) else { skip!() };
        println!(
            "{name}: PAB {} read_log {:?}",
            hex(&vram(&m, 0x1000, 10)),
            m.bus().disk.read_log()
        );
    }
}

/// DELETE (op 7): bitmap/FDIR effects.
#[test]
#[ignore]
fn probe_delete() {
    let Some(ti_dsr) = TI_DSR.as_deref() else { skip!() };
    let disk = build_disk(
        "PROBE",
        &[
            FileSpec::var("AAA", false, 80, &[b"1"]),
            FileSpec::var("BBB", false, 80, &[b"22"]),
            FileSpec::var("CCC", false, 80, &[b"333"]),
        ],
    );
    let p = pab(7, 0, 0, 0, 0, 0, "DSK1.BBB");
    let rig = Rig::new().stage(0x1000, &p).dev(0x1000);
    let Some(m) = run_rig(ti_dsr, &[(0, disk.clone())], &rig) else { skip!() };
    println!("PAB after: {}", hex(&vram(&m, 0x1000, 10)));
    let after = m.bus().disk.drive_image(0).unwrap();
    for s in 0..360 {
        if disk[s * 256..(s + 1) * 256] != after[s * 256..(s + 1) * 256] {
            println!("== sector {s} changed ==\n{}", hex(&after[s * 256..s * 256 + 48]));
            if s == 0 {
                println!("bitmap: {}", hex(&after[0x38..0x48]));
            }
        }
    }
}
