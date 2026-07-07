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

//! FD1771 / TI Disk Controller conformance tests — milestone 7.
//!
//! These drive the controller directly the way the DSR ROM does: through the
//! `>4000–5FFF` window with the card's `XOR >FF` data inversion. Helpers wrap the
//! inversion so the test reads as plain values.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::disk::Disk;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static DSR: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/Disk.Bin"));
static TUNNELS: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("disks/Tunnels.Dsk"));
static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));
static TUNDOOM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/tundoom.ctg"));

/// A controller with the DSR ROM installed and Tunnels.Dsk in DSK1, or `None`
/// (announcing the skip) when the third-party media is absent.
fn controller() -> Option<Disk> {
    let (Some(dsr), Some(tunnels)) = (DSR.as_deref(), TUNNELS.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return None;
    };
    let mut d = Disk::new();
    d.load_dsr(dsr);
    d.mount(0, tunnels.to_vec());
    d.write_cru(4, true); // select DSK1 (drive 0)
    Some(d)
}

/// Write a register the way the CPU does: the card inverts the bus.
fn put(d: &mut Disk, addr: u16, value: u8) {
    d.write_byte(addr, value ^ 0xFF);
}
/// Read a register, undoing the card's inversion.
fn get(d: &mut Disk, addr: u16) -> u8 {
    d.read_byte(addr) ^ 0xFF
}

#[test]
fn dsr_rom_maps_only_when_cru_bit0_is_set() {
    let Some(mut d) = controller() else { return };
    let dsr = DSR.as_deref().unwrap(); // present — `controller()` checked
    // Before ROM-enable the window is open bus.
    assert_eq!(d.read_byte(0x4000), 0x00);
    d.write_cru(0, true);
    // Disk.Bin opens with the >AA DSR header.
    assert_eq!(d.read_byte(0x4000), 0xAA);
    assert_eq!(d.read_byte(0x4001), dsr[1]);
    assert_eq!(d.read_byte(0x5000), dsr[0x1000]);
    // The FD1771 registers respond regardless of the ROM-enable bit.
    assert_eq!(get(&mut d, 0x5FF0), 0x00, "idle status");
}

#[test]
fn read_sector_zero_returns_the_volume_information_block() {
    let Some(mut d) = controller() else { return };
    let tunnels = TUNNELS.as_deref().unwrap(); // present — `controller()` checked
    put(&mut d, 0x5FF8, 0x00); // Restore → track 0
    put(&mut d, 0x5FFC, 0x00); // sector register = 0
    put(&mut d, 0x5FF8, 0x80); // Read Sector
    let sector: Vec<u8> = (0..256).map(|_| get(&mut d, 0x5FF6)).collect();
    assert_eq!(&sector[..], &tunnels[..256]);
    // The VIB names the volume "TUNNELS".
    assert_eq!(&sector[0..7], b"TUNNELS");
}

#[test]
fn lba_is_track_times_nine_plus_sector() {
    let Some(mut d) = controller() else { return };
    let tunnels = TUNNELS.as_deref().unwrap(); // present — `controller()` checked
    // Seek to track 2 (the target track is loaded into the data register first).
    put(&mut d, 0x5FFE, 2);
    put(&mut d, 0x5FF8, 0x10); // Seek
    assert_eq!(get(&mut d, 0x5FF2), 2, "track register follows the seek");
    put(&mut d, 0x5FFC, 3); // sector 3
    put(&mut d, 0x5FF8, 0x80); // Read Sector
    let lba = 2 * 9 + 3;
    let sector: Vec<u8> = (0..256).map(|_| get(&mut d, 0x5FF6)).collect();
    assert_eq!(&sector[..], &tunnels[lba * 256..lba * 256 + 256]);
}

// --------------------------------------------------------------------------
// Per-drive geometry from the VIB: the (track, side, sector) → LBA mapping must
// follow the mounted image's actual geometry, not a hardcoded 40×9 single side.
// --------------------------------------------------------------------------

/// Build a raw sector-dump with a valid VIB for the given geometry; each sector's
/// first two bytes hold its own LBA (big-endian) so a read reveals which absolute
/// sector the controller mapped.
fn synthetic_disk(spt: usize, tracks: usize, sides: usize) -> Vec<u8> {
    let total = spt * tracks * sides;
    let mut img = vec![0u8; total * 256];
    img[0x0A] = (total >> 8) as u8;
    img[0x0B] = total as u8;
    img[0x0C] = spt as u8;
    img[0x0D..0x10].copy_from_slice(b"DSK");
    img[0x11] = tracks as u8;
    img[0x12] = sides as u8;
    for lba in 0..total {
        let off = lba * 256;
        img[off] = (lba >> 8) as u8;
        img[off + 1] = lba as u8;
    }
    img
}

/// Mount `image` in DSK1, seek to (track, side, sector), read the sector, and
/// return the LBA the controller mapped it to (recovered from the stamped
/// bytes) — or `None` (announcing the skip) when the DSR image is absent.
fn mapped_lba(image: Vec<u8>, track: u8, side: u8, sector: u8) -> Option<usize> {
    let Some(dsr) = DSR.as_deref() else {
        eprintln!("SKIPPED: third-party media not present");
        return None;
    };
    let mut d = Disk::new();
    d.load_dsr(dsr);
    d.mount(0, image);
    d.write_cru(4, true); // select DSK1 (drive 0)
    d.write_cru(7, side != 0); // side select
    put(&mut d, 0x5FFE, track); // Seek target track → data register
    put(&mut d, 0x5FF8, 0x10); // Seek
    put(&mut d, 0x5FFC, sector); // sector register
    put(&mut d, 0x5FF8, 0x80); // Read Sector
    let hi = get(&mut d, 0x5FF6) as usize;
    let lo = get(&mut d, 0x5FF6) as usize;
    Some((hi << 8) | lo)
}

#[test]
fn sssd_maps_track_times_nine_side_zero() {
    let Some(lba) = mapped_lba(synthetic_disk(9, 40, 1), 3, 0, 4) else { return };
    assert_eq!(lba, 3 * 9 + 4);
}

#[test]
fn dssd_side_one_uses_reverse_track_convention() {
    // Side 1 stored after side 0 with tracks reversed: 360 + (39 − T)·9 + sector.
    let Some(lba) = mapped_lba(synthetic_disk(9, 40, 2), 5, 1, 2) else { return };
    assert_eq!(lba, 360 + (39 - 5) * 9 + 2);
    // Side 0 of the same image is unchanged.
    assert_eq!(mapped_lba(synthetic_disk(9, 40, 2), 5, 0, 2), Some(5 * 9 + 2));
}

#[test]
fn ssdd_maps_eighteen_sectors_per_track() {
    // Double density: eighteen sectors per track, so track 3 sector 4 → 58.
    let Some(lba) = mapped_lba(synthetic_disk(18, 40, 1), 3, 0, 4) else { return };
    assert_eq!(lba, 3 * 18 + 4);
}

#[test]
fn status_tracks_busy_across_a_sector_transfer() {
    let Some(mut d) = controller() else { return };
    assert_eq!(get(&mut d, 0x5FF0) & 0x01, 0, "idle is not busy");
    put(&mut d, 0x5FFC, 0x00);
    put(&mut d, 0x5FF8, 0x80); // Read Sector
    assert!(get(&mut d, 0x5FF0) & 0x01 != 0, "busy during transfer");
    for _ in 0..256 {
        let _ = get(&mut d, 0x5FF6);
    }
    assert_eq!(get(&mut d, 0x5FF0) & 0x01, 0, "busy clears when the sector is done");
}

#[test]
fn write_sector_round_trips_through_the_image() {
    let Some(mut d) = controller() else { return };
    put(&mut d, 0x5FFC, 5); // sector 5 of track 0 → LBA 5
    put(&mut d, 0x5FF8, 0xA0); // Write Sector
    for i in 0..256u32 {
        put(&mut d, 0x5FFE, (i as u8) ^ 0x5A);
    }
    // Read it back.
    put(&mut d, 0x5FF8, 0x80); // Read Sector (same track/sector)
    for i in 0..256u32 {
        assert_eq!(get(&mut d, 0x5FF6), (i as u8) ^ 0x5A, "byte {}", i);
    }
}

#[test]
fn an_empty_drive_reads_not_ready() {
    let Some(dsr) = DSR.as_deref() else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut d = Disk::new();
    d.load_dsr(dsr);
    d.write_cru(5, true); // select DSK2 — no disk mounted there
    put(&mut d, 0x5FFC, 0x00);
    put(&mut d, 0x5FF8, 0x80); // Read Sector
    assert!(get(&mut d, 0x5FF0) & 0x80 != 0, "Not-Ready set with no disk");
}

// --------------------------------------------------------------------------
// Type I stepping, Read Address, and Force Interrupt at the register level.
// --------------------------------------------------------------------------

/// Read all 256 bytes of the sector at absolute LBA `lba` (track 0) back from the
/// selected drive.
fn read_whole_sector(d: &mut Disk, lba: u8) -> Vec<u8> {
    put(d, 0x5FFC, lba);
    put(d, 0x5FF8, 0x80); // Read Sector
    (0..256).map(|_| get(d, 0x5FF6)).collect()
}

#[test]
fn step_latches_direction_and_clamps_at_the_track_extremes() {
    let Some(mut d) = controller() else { return };
    // Seek to track 5 (the target track is loaded into the data register first).
    put(&mut d, 0x5FFE, 5);
    put(&mut d, 0x5FF8, 0x10); // Seek
    assert_eq!(get(&mut d, 0x5FF2), 5);

    // Step In latches the +1 direction; a bare Step then *repeats* it.
    put(&mut d, 0x5FF8, 0x40); // Step In → track 6
    assert_eq!(get(&mut d, 0x5FF2), 6);
    put(&mut d, 0x5FF8, 0x20); // bare Step, repeat +1 → track 7
    assert_eq!(get(&mut d, 0x5FF2), 7, "bare Step repeats the latched +1 direction");

    // Step Out latches −1; a bare Step then repeats that.
    put(&mut d, 0x5FF8, 0x60); // Step Out → track 6
    assert_eq!(get(&mut d, 0x5FF2), 6);
    put(&mut d, 0x5FF8, 0x20); // bare Step, repeat −1 → track 5
    assert_eq!(get(&mut d, 0x5FF2), 5, "bare Step repeats the latched −1 direction");

    // Repeated −1 steps clamp at track 0 (and raise the Track-0 status bit).
    for _ in 0..10 {
        put(&mut d, 0x5FF8, 0x20);
    }
    assert_eq!(get(&mut d, 0x5FF2), 0, "steps clamp at track 0");
    assert!(get(&mut d, 0x5FF0) & 0x04 != 0, "Track-0 status bit set at track 0");

    // Latch +1 again and clamp at the maximum track (39).
    put(&mut d, 0x5FF8, 0x40); // Step In → +1, track 1
    for _ in 0..60 {
        put(&mut d, 0x5FF8, 0x20); // bare Step, repeat +1
    }
    assert_eq!(get(&mut d, 0x5FF2), 39, "steps clamp at the maximum track");
    assert_eq!(get(&mut d, 0x5FF0) & 0x04, 0, "Track-0 bit clear off track 0");
}

#[test]
fn read_address_returns_the_six_byte_id_field() {
    let Some(mut d) = controller() else { return };
    // Position the head: seek track 7, select side 1, sector register 3.
    put(&mut d, 0x5FFE, 7);
    put(&mut d, 0x5FF8, 0x10); // Seek → track 7
    d.write_cru(7, true); // side 1
    put(&mut d, 0x5FFC, 3); // sector register = 3
    put(&mut d, 0x5FF8, 0xC0); // Read Address

    // The FD1771 serves a 6-byte ID field: track, side, sector, size code, 2×CRC.
    assert_eq!(get(&mut d, 0x5FF6), 7, "cylinder = current track");
    assert_eq!(get(&mut d, 0x5FF6), 1, "side = selected side");
    assert_eq!(get(&mut d, 0x5FF6), 3, "sector = sector register");
    assert_eq!(get(&mut d, 0x5FF6), 0x01, "size code = 256-byte sectors");
    assert_eq!(get(&mut d, 0x5FF6), 0xFF, "CRC placeholder");
    assert_eq!(get(&mut d, 0x5FF6), 0xFF, "CRC placeholder");

    // Exactly six bytes: Busy clears once the ID field is exhausted.
    assert_eq!(get(&mut d, 0x5FF0) & 0x01, 0, "Busy clears after the ID field");
}

#[test]
fn force_interrupt_aborts_a_write_without_flushing() {
    let Some(mut d) = controller() else { return };
    // Capture sector 5's original contents (the no-flush check compares against it).
    let original = read_whole_sector(&mut d, 5);

    // Begin a Write Sector to sector 5 but supply only a handful of the 256 bytes.
    put(&mut d, 0x5FFC, 5);
    put(&mut d, 0x5FF8, 0xA0); // Write Sector
    assert!(get(&mut d, 0x5FF0) & 0x01 != 0, "Busy during the write transfer");
    for _ in 0..10 {
        put(&mut d, 0x5FFE, 0x99); // partial, never-completed payload
    }

    // Force Interrupt aborts the in-progress transfer: Busy clears immediately.
    put(&mut d, 0x5FF8, 0xD0);
    assert_eq!(get(&mut d, 0x5FF0) & 0x01, 0, "Force Interrupt clears Busy");

    // And crucially the partial sector was never flushed — sector 5 still holds its
    // original bytes (a completed write would have overwritten them).
    let after = read_whole_sector(&mut d, 5);
    assert_eq!(after, original, "an aborted write must not flush to the image");
}

// --------------------------------------------------------------------------
// Gate (d): the genuine DSR loads a Tunnels of Doom scenario from disk.
// --------------------------------------------------------------------------

fn tap(m: &mut Machine, k: TiKey, settle: u32) {
    m.set_key(k, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(k, false);
    for _ in 0..settle {
        m.run_frame();
    }
}

fn screen_text(m: &Machine) -> String {
    let base = ((m.vdp().register(2) as usize) & 0x0F) << 10;
    (0..24 * 32)
        .map(|i| {
            let c = m.vdp().vram((base + i) as u16);
            if (0x20..0x7f).contains(&c) {
                c as char
            } else {
                ' '
            }
        })
        .collect()
}

/// **Gate (d): Tunnels of Doom loads a scenario from `Tunnels.Dsk`.** Mount the
/// cartridge, the disk controller, and the disk; drive the cartridge through its
/// "LOAD DATA FROM: DISK 1" prompt and type the QUEST scenario name. The console's
/// real DSR — running unmodified — then drives our FD1771 to read the file
/// descriptor index, the QUEST file descriptor, and the file's data sectors
/// (AUs 85–135), and the cartridge advances to its post-load game menu.
#[test]
fn tunnels_of_doom_loads_quest_scenario_from_disk() {
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
    m.bus_mut().disk.record(true);

    for _ in 0..180 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 40); // title → selection list
    tap(&mut m, TiKey::Num2, 240); // select Tunnels of Doom
    tap(&mut m, TiKey::Enter, 120); // → "LOAD DATA FROM"
    tap(&mut m, TiKey::Num2, 120); // 2 = DISK 1 → filename prompt
    for k in [TiKey::Q, TiKey::U, TiKey::E, TiKey::S, TiKey::T] {
        tap(&mut m, k, 10);
    }
    tap(&mut m, TiKey::Enter, 600); // submit → load the scenario

    // The QUEST scenario file occupies AUs 85–135 on the disk; the DSR must read
    // those data sectors (plus the file-descriptor index / descriptor).
    let read = m.bus().disk.read_log();
    assert!(
        read.contains(&85) && read.contains(&135),
        "the DSR did not read the QUEST file data sectors (85..=135); read {:?}",
        read
    );

    // Loading succeeded: the cartridge advanced to its game-selection menu.
    let screen = screen_text(&m);
    assert!(
        screen.contains("NEW DUNGEON"),
        "Tunnels of Doom did not reach its post-load menu; screen was:\n{}",
        screen
    );
}

/// Regression: with the TI Disk Controller installed, the console must still draw
/// its **master title screen**. The disk DSR's power-up routine reserves a VDP
/// buffer at the top of VRAM and clears it with a `CLR @>8C00` word-write loop;
/// when the bus turned each word write into two VDP writes, that loop cleared
/// twice as much as intended, ran past the end of VRAM, and wrapped its zeros back
/// over the freshly-drawn title — the screen went blank cyan. (Fixed by dropping
/// the odd half of a word access to the VDP; see `machine.rs`.)
#[test]
fn disk_controller_still_draws_the_master_title_screen() {
    let (Some(rom), Some(grom), Some(dsr)) =
        (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref(), DSR.as_deref())
    else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    m.load_disk_controller(dsr);
    m.reset();
    for _ in 0..180 {
        m.run_frame();
    }
    let screen = screen_text(&m);
    assert!(
        screen.contains("TEXAS INSTRUMENTS"),
        "master title screen missing with the disk controller installed; screen was:\n{}",
        screen
    );
}
