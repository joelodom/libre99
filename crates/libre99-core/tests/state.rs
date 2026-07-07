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

//! Save-state conformance tests.
//!
//! The headline guarantee is the user's scenario: boot a program, **write to a
//! mounted disk**, save the machine, throw it away, and reload into a *fresh*
//! machine — and find the program and the written disk data exactly where they
//! were left. These tests prove the snapshot is self-contained (it carries the
//! ROM/GROM/cartridge/disk images itself), byte-exact (a reload re-serializes
//! identically and evolves identically), and robust (a bad/foreign/truncated file
//! is rejected without touching the live machine).

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::disk::Disk;
use libre99_core::machine::Machine;
use libre99_core::state::StateError;
use libre99_core::vdp::{HEIGHT, WIDTH};

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));
static DSR: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/Disk.Bin"));
static TUNNELS: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("disks/Tunnels.Dsk"));
static TUNDOOM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/tundoom.ctg"));

/// Write an FD1771 register the way the CPU does (the card inverts the bus).
fn put(d: &mut Disk, addr: u16, value: u8) {
    d.write_byte(addr, value ^ 0xFF);
}
/// Read an FD1771 register, undoing the card's inversion.
fn get(d: &mut Disk, addr: u16) -> u8 {
    d.read_byte(addr) ^ 0xFF
}

/// Drive a 256-byte Write Sector to absolute sector `lba` (track 0) on drive 0.
fn write_sector(d: &mut Disk, lba: u8, payload: &[u8; 256]) {
    d.write_cru(0, true); // ROM enable
    d.write_cru(4, true); // select drive 0
    put(d, 0x5FFA, 0); // track register = 0
    put(d, 0x5FFC, lba); // sector register = lba (track 0 ⇒ absolute lba)
    put(d, 0x5FF8, 0xA0); // Write Sector
    for &b in payload.iter() {
        put(d, 0x5FFE, b);
    }
}

/// Read absolute sector `lba` (track 0) back from drive 0.
fn read_sector(d: &mut Disk, lba: u8) -> [u8; 256] {
    d.write_cru(0, true);
    d.write_cru(4, true);
    put(d, 0x5FFA, 0);
    put(d, 0x5FFC, lba);
    put(d, 0x5FF8, 0x80); // Read Sector
    let mut out = [0u8; 256];
    for b in out.iter_mut() {
        *b = get(d, 0x5FF6);
    }
    out
}

fn framebuffer(m: &mut Machine) -> Vec<u32> {
    let mut fb = vec![0u32; WIDTH * HEIGHT];
    m.render(&mut fb);
    fb
}

/// A reload re-serializes to the identical bytes **and** the restored machine
/// runs forward identically to one that was never saved.
#[test]
fn save_load_round_trip_is_exact() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    for _ in 0..150 {
        m.run_frame(); // boot to the master title screen
    }

    let snapshot = m.save_state();
    let mut restored = Machine::new(rom, grom);
    restored.load_state(&snapshot).expect("load a valid snapshot");

    // Byte-for-byte: re-saving the restored machine reproduces the file exactly.
    assert_eq!(restored.save_state(), snapshot, "reload is not byte-identical");

    // Functionally exact: advancing both by the same amount keeps them in lockstep
    // (any drift in CPU, GROM, VDP, or the interrupt timing would diverge here).
    for _ in 0..20 {
        m.run_frame();
        restored.run_frame();
    }
    assert_eq!(framebuffer(&mut m), framebuffer(&mut restored), "frames diverged");
    assert_eq!(m.save_state(), restored.save_state(), "state diverged after replay");
}

/// A whole cartridge+disk session reloads into a machine that never mounted
/// either — the snapshot carries the cartridge ROM/GROM and the disk image with
/// it.
#[test]
fn restores_a_cartridge_and_disk_session() {
    let (Some(rom), Some(grom), Some(dsr), Some(tunnels), Some(tundoom)) = (
        CONSOLE_ROM.as_deref(),
        CONSOLE_GROM.as_deref(),
        DSR.as_deref(),
        TUNNELS.as_deref(),
        TUNDOOM.as_deref(),
    ) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    m.load_disk_controller(dsr);
    m.mount_disk(0, tunnels.to_vec());
    m.mount_cartridge(&Cartridge::parse(tundoom).unwrap());
    m.reset();
    for _ in 0..200 {
        m.run_frame();
    }
    let snapshot = m.save_state();

    // A pristine bare console — no disk controller, no cartridge.
    let mut restored = Machine::new(rom, grom);
    restored.load_state(&snapshot).expect("load the session");

    assert_eq!(restored.save_state(), snapshot, "cartridge/disk session not restored");
    assert_eq!(
        framebuffer(&mut m),
        framebuffer(&mut restored),
        "restored cartridge screen differs"
    );
}

/// **The headline scenario.** Write data to a mounted disk, save, and reload into
/// a machine that has *no* disk controller — the controller, the image, and the
/// written sector all come back.
#[test]
fn disk_writes_survive_save_and_reload() {
    let (Some(rom), Some(grom), Some(dsr), Some(tunnels)) = (
        CONSOLE_ROM.as_deref(),
        CONSOLE_GROM.as_deref(),
        DSR.as_deref(),
        TUNNELS.as_deref(),
    ) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let payload: [u8; 256] = std::array::from_fn(|i| (i as u8) ^ 0xC3);
    const LBA: u8 = 5;

    let mut m = Machine::new(rom, grom);
    m.load_disk_controller(dsr);
    m.mount_disk(0, tunnels.to_vec());
    write_sector(&mut m.bus_mut().disk, LBA, &payload);

    // Sanity: we actually changed the image (the payload differs from the original
    // contents of that sector), so the test below proves persistence of a *write*.
    assert_ne!(
        &payload[..],
        &tunnels[LBA as usize * 256..LBA as usize * 256 + 256],
        "payload coincidentally equals the original sector"
    );

    let snapshot = m.save_state();

    let mut restored = Machine::new(rom, grom);
    restored.load_state(&snapshot).expect("load the disk session");

    let read_back = read_sector(&mut restored.bus_mut().disk, LBA);
    assert_eq!(read_back, payload, "the written sector did not survive save/reload");
}

/// The in-memory disk shelf rides along in the save state: eject a keyed disk,
/// save, load into a fresh machine, and remounting the same file reattaches
/// the shelved image — no host media required (bare firmware images suffice).
#[test]
fn the_disk_shelf_survives_save_and_reload() {
    let mut image = vec![0u8; 4 * 256];
    image[0] = 0xA5;

    let mut m = Machine::new(&[], &[]);
    assert!(!m.mount_disk_keyed(0, "C:/disks/game.dsk", image.clone()));
    m.eject_disk(0);

    let snapshot = m.save_state();
    let mut restored = Machine::new(&[], &[]);
    restored.load_state(&snapshot).expect("load the shelf session");
    assert_eq!(restored.save_state(), snapshot, "shelf session not byte-identical");

    let shelf = restored.bus().disk.in_memory_disks();
    assert_eq!(shelf.len(), 1, "the ejected disk did not come back");
    assert_eq!(shelf[0].key, "C:/disks/game.dsk");
    assert_eq!(shelf[0].drive, None);

    // Remounting the same file reattaches the restored in-memory copy (the
    // host bytes offered here are different, and lose).
    assert!(restored.mount_disk_keyed(0, "C:/disks/game.dsk", vec![0u8; 4 * 256]));
    assert_eq!(restored.bus().disk.drive_image(0).unwrap()[0], 0xA5);
}

/// A mounted keyed disk's identity survives save/reload, so the frontend can
/// still export it and the window title can still name it.
#[test]
fn a_mounted_disk_identity_survives_save_and_reload() {
    let mut m = Machine::new(&[], &[]);
    m.mount_disk_keyed(0, "K", vec![0u8; 4 * 256]);

    let mut restored = Machine::new(&[], &[]);
    restored.load_state(&m.save_state()).expect("load");
    assert_eq!(restored.bus().disk.drive_key(0), Some("K"));
}

#[test]
fn rejects_a_file_that_is_not_a_save_state() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    assert_eq!(m.load_state(b"not a TI99 save"), Err(StateError::BadMagic));
}

#[test]
fn rejects_an_unsupported_version() {
    // Correct magic, but a version this build does not know.
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut bytes = b"TI99SAVE".to_vec();
    bytes.extend_from_slice(&999u32.to_le_bytes());
    let mut m = Machine::new(rom, grom);
    assert_eq!(m.load_state(&bytes), Err(StateError::UnsupportedVersion(999)));
}

#[test]
fn rejects_a_truncated_file() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    let snapshot = m.save_state();
    // Keep the valid magic+version but cut the body short.
    assert_eq!(m.load_state(&snapshot[..20]), Err(StateError::Truncated));
}

/// A failed load must not disturb the running machine.
#[test]
fn a_failed_load_leaves_the_machine_untouched() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    for _ in 0..120 {
        m.run_frame();
    }
    let before = m.save_state();
    assert!(m.load_state(b"garbage").is_err());
    assert_eq!(m.save_state(), before, "a rejected load mutated the machine");
}

// --------------------------------------------------------------------------
// Sanitizer paths: a *well-formed* file (correct magic/version/length) can still
// carry out-of-range field values (a corrupt or foreign save). `load_state` must
// clamp them so a later access cannot index out of bounds or panic. These paths
// are otherwise never exercised by a normal round trip.
//
// Field byte offsets are located by construction rather than hardcoded: we build
// two real snapshots that differ in *only* the target field and diff them. The
// first differing byte is the low (little-endian) byte of that field, and the
// serialization order (`disk.rs`/`psg.rs` `save_state`) fixes the field's width
// and its neighbor. This keeps the tests valid across serialization-format edits.
// --------------------------------------------------------------------------

/// A bare console with the disk controller + Tunnels.Dsk installed, its DSK1
/// selected, then a Read Sector of sector 0 partially drained by `bytes_read` data
/// reads — leaving the transfer cursor (`buf_pos`, `buf_left`) mid-transfer.
/// The caller has already skip-checked the third-party media.
fn machine_mid_disk_read(bytes_read: usize) -> Machine {
    let mut m = Machine::new(CONSOLE_ROM.as_deref().unwrap(), CONSOLE_GROM.as_deref().unwrap());
    m.load_disk_controller(DSR.as_deref().unwrap());
    m.mount_disk(0, TUNNELS.as_deref().unwrap().to_vec());
    let d = &mut m.bus_mut().disk;
    d.write_cru(0, true); // ROM enable
    d.write_cru(4, true); // select DSK1
    put(d, 0x5FFA, 0); // track register = 0
    put(d, 0x5FFC, 0); // sector register = 0
    put(d, 0x5FF8, 0x80); // Read Sector → buf_pos=0, buf_left=256
    for _ in 0..bytes_read {
        let _ = get(d, 0x5FF6); // advance the cursor (stays busy while < 256)
    }
    m
}

#[test]
fn a_corrupt_disk_transfer_cursor_is_clamped_on_load() {
    let (Some(rom), Some(grom), Some(_), Some(_)) = (
        CONSOLE_ROM.as_deref(),
        CONSOLE_GROM.as_deref(),
        DSR.as_deref(),
        TUNNELS.as_deref(),
    ) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    // Locate the cursor: two snapshots that read 40 vs 60 of the 256 bytes differ
    // *only* in buf_pos/buf_left (same sector ⇒ identical buffer, still busy ⇒
    // identical status). The first differing byte is buf_pos's low byte, i.e. the
    // start of the two consecutive u32 fields `buf_pos` then `buf_left`.
    let a = machine_mid_disk_read(40).save_state();
    let b = machine_mid_disk_read(60).save_state();
    assert_eq!(a.len(), b.len(), "same layout, different cursor");
    let start = (0..a.len()).find(|&i| a[i] != b[i]).expect("cursor bytes differ");

    // A real snapshot mid-read, then force buf_pos and buf_left (the two u32s at
    // `start`) to huge values — length-preserving.
    let mut snap = machine_mid_disk_read(40).save_state();
    assert_eq!(snap[start], 40, "located field holds buf_pos (layout check)");
    for byte in &mut snap[start..start + 8] {
        *byte = 0xFF; // buf_pos = buf_left = 0xFFFF_FFFF
    }

    // The load must succeed (well-formed file) and the sanitizer must reset the
    // cursor: no panic, no out-of-bounds.
    let mut restored = Machine::new(rom, grom);
    restored.load_state(&snap).expect("a well-formed corrupt file still loads");

    // Observable proof of the reset: DRQ (set only while bytes remain) is clear, and
    // a data-register read serves the idle data byte instead of indexing the buffer
    // at 0xFFFF_FFFF (which would panic if the cursor had survived).
    let d = &mut restored.bus_mut().disk;
    assert_eq!(get(d, 0x5FF0) & 0x02, 0, "DRQ clear ⇒ transfer cursor was reset");
    let _ = get(d, 0x5FF6); // must not panic
}

#[test]
fn a_corrupt_psg_latched_channel_is_masked_on_load() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    // Latch attenuation (type 1) = 15 — the power-up default — on a channel: this
    // changes only latched_channel (volumes stay 15), so two snapshots latching
    // channel 1 vs 2 differ in exactly the latched_channel u32.
    let latch_atten = |ch: u8| -> Vec<u8> {
        let mut m = Machine::new(rom, grom);
        m.bus_mut().psg.write(0x80 | (ch << 5) | (1 << 4) | 0x0F);
        m.save_state()
    };
    let a = latch_atten(1);
    let b = latch_atten(2);
    let start = (0..a.len()).find(|&i| a[i] != b[i]).expect("latched_channel differs");

    // A real snapshot with channel 2 latched, then force latched_channel to 7 —
    // out of range for the 4-entry volume array.
    let mut snap = latch_atten(2);
    assert_eq!(snap[start], 2, "located field holds latched_channel (layout check)");
    snap[start] = 7;
    snap[start + 1] = 0;
    snap[start + 2] = 0;
    snap[start + 3] = 0;

    let mut restored = Machine::new(rom, grom);
    restored.load_state(&snap).expect("a well-formed corrupt file still loads");

    // load_state masks latched_channel to 0..3 (7 & 3 = 3). A following data byte
    // must land on channel 3's attenuation — not panic indexing `volume[7]`.
    restored.bus_mut().psg.write(0x00); // data byte → volume[latched_channel] = 0
    assert_eq!(
        restored.bus().psg.volume(3),
        0,
        "masked latched_channel (7 & 3 = 3) received the following write"
    );
}
