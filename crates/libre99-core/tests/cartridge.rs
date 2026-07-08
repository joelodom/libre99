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

//! `.ctg` cartridge-parser conformance tests — milestone 6.
//!
//! These parse the real commercial images (loaded at run time from the
//! git-ignored `third-party/` directory; the tests skip green when they are
//! absent) and check the region breakdown against the validated reference:
//! `blasto` is a single GROM, `tundoom` five GROMs, `Parsec` a ROM plus three
//! GROMs, and `xb25` a two-bank ROM plus five GROMs. Every decoded GROM page
//! must carry a valid `>AA` GPL header, and the cartridge ROM banks must be a
//! whole number of 8 KiB.

use std::sync::LazyLock;

use libre99_core::cartridge::{Cartridge, CartridgeError};
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));
static BLASTO: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/blasto.ctg"));
static TUNDOOM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/tundoom.ctg"));
static PARSEC: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/Parsec.ctg"));
static XB25: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/xb25.ctg"));

#[test]
fn blasto_is_a_single_grom() {
    let Some(blasto) = BLASTO.as_deref() else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let c = Cartridge::parse(blasto).unwrap();
    assert_eq!(c.title, "BLASTO");
    assert_eq!(c.cru_base, 0x0000);
    assert_eq!(c.rom_banks, 0, "pure-GROM cartridge has no ROM");
    assert!(c.rom.is_empty());
    assert_eq!(c.grom.len(), 1);
    let (addr, page) = &c.grom[0];
    assert_eq!(*addr, 0x6000, "cartridge GROM starts at GROM >6000");
    assert_eq!(page.len(), 0x2000);
    assert_eq!(page[0], 0xAA, "valid GPL header magic");
}

#[test]
fn mounting_a_cartridge_replaces_the_previous_one() {
    use libre99_core::bus::Bus;
    // Cart A: one ROM bank + a GROM page at >6000. Cart B: GROM-only at >8000.
    // Mounting B over A must unmap A's ROM and erase A's GROM page — layering
    // the two would let stale code shadow the new cartridge.
    let a = Cartridge {
        title: "A".into(),
        cru_base: 0,
        rom: vec![0xAA; 0x2000],
        rom_banks: 1,
        grom: vec![(0x6000, vec![0xBB; 0x2000])],
    };
    let b = Cartridge {
        title: "B".into(),
        cru_base: 0,
        rom: Vec::new(),
        rom_banks: 0,
        grom: vec![(0x8000, vec![0xCC; 0x2000])],
    };
    let mut m = Machine::new(&[], &[]);
    m.mount_cartridge(&a);
    m.mount_cartridge(&b);
    let bus = m.bus_mut();
    assert_eq!(bus.read_byte(0x6000), 0, "cart A's ROM must be unmapped");
    bus.write_byte(0x9C02, 0x60); // GROM address >6000, high then low
    bus.write_byte(0x9C02, 0x00);
    assert_eq!(bus.read_byte(0x9800), 0x00, "cart A's GROM page is gone");
    bus.write_byte(0x9C02, 0x80); // GROM address >8000
    bus.write_byte(0x9C02, 0x00);
    assert_eq!(bus.read_byte(0x9800), 0xCC, "cart B's GROM page is present");
}

#[test]
fn tundoom_is_five_groms() {
    let Some(tundoom) = TUNDOOM.as_deref() else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let c = Cartridge::parse(tundoom).unwrap();
    assert_eq!(c.title, "TUNNELS OF DOOM");
    assert_eq!(c.rom_banks, 0);
    assert_eq!(c.grom.len(), 5);
    // Five consecutive 8 KiB GROM pages at >6000, >8000, >A000, >C000, >E000.
    for (i, (addr, page)) in c.grom.iter().enumerate() {
        assert_eq!(*addr, 0x6000 + i as u16 * 0x2000);
        assert_eq!(page.len(), 0x2000);
    }
    assert_eq!(c.grom[0].1[0], 0xAA, "first GROM page has the GPL header");
}

#[test]
fn parsec_is_rom_plus_three_groms() {
    let Some(parsec) = PARSEC.as_deref() else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let c = Cartridge::parse(parsec).unwrap();
    assert_eq!(c.title, "PARSEC");
    assert_eq!(c.rom_banks, 1, "single 8 KiB ROM bank");
    assert_eq!(c.rom.len(), 0x2000);
    assert_eq!(c.grom.len(), 3);
    assert_eq!(c.grom[0].0, 0x6000);
    assert_eq!(c.grom[0].1[0], 0xAA);
}

#[test]
fn xb25_is_two_bank_rom_plus_five_groms() {
    let Some(xb25) = XB25.as_deref() else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let c = Cartridge::parse(xb25).unwrap();
    assert!(c.title.starts_with("EXTENDED BASIC"), "title was {:?}", c.title);
    assert_eq!(c.rom_banks, 2, "bank-switched ROM cartridge");
    assert_eq!(c.rom.len(), 2 * 0x2000);
    assert_eq!(c.grom.len(), 5);
}

#[test]
fn every_bundled_cartridge_parses() {
    // Every commercial image the suite exercises, loaded at run time.
    let (Some(blasto), Some(tundoom), Some(parsec), Some(xb25)) = (
        BLASTO.as_deref(),
        TUNDOOM.as_deref(),
        PARSEC.as_deref(),
        XB25.as_deref(),
    ) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let images: &[&[u8]] = &[blasto, tundoom, parsec, xb25];
    for img in images {
        let c = Cartridge::parse(img).expect("bundled image parses");
        // Either ROM or GROM must be present, ROM is a whole number of banks.
        assert!(c.rom_banks > 0 || !c.grom.is_empty());
        assert_eq!(c.rom.len(), c.rom_banks * 0x2000);
        for (_, page) in &c.grom {
            assert_eq!(page.len(), 0x2000);
        }
    }
}

/// Read the VDP name table back as text. On the console's title/menu screens the
/// name-table cell value is the character's ASCII code, so the screen reads out
/// directly.
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

/// **Gate (c): a mounted cartridge reaches the master selection list.** With
/// Tunnels of Doom mounted, the console scans the cartridge GROM, finds its menu
/// entry, and — once a key advances past the title screen — lists
/// "2 FOR TUNNELS OF DOOM" on the selection screen.
#[test]
fn tunnels_of_doom_appears_on_the_selection_screen() {
    let (Some(rom), Some(grom), Some(tundoom)) =
        (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref(), TUNDOOM.as_deref())
    else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    let cart = Cartridge::parse(tundoom).unwrap();
    m.mount_cartridge(&cart);
    m.reset(); // restart the console now that the cartridge is present

    // Boot to the master title screen ("PRESS ANY KEY TO BEGIN").
    for _ in 0..180 {
        m.run_frame();
    }
    // Press a key (SPACE — not a menu number, so it only advances the screen),
    // then let the selection list draw.
    m.set_key(TiKey::Space, true);
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..120 {
        m.run_frame();
    }

    let text = screen_text(&m);
    assert!(
        text.contains("TUNNELS OF DOOM"),
        "cartridge menu entry not on the selection screen; screen was:\n{}",
        text
    );
}

/// Build a raw CPU-ROM `.bin` dump (the loose-binary cartridge form, no `.ctg`
/// container): `banks` 8 KiB banks, each opening with a standard `>AA` module
/// header whose program list names the cartridge — the shape `copper8.bin` has.
fn synth_raw_rom(title: &str, banks: usize) -> Vec<u8> {
    let mut rom = vec![0u8; banks * 0x2000];
    for b in 0..banks {
        let base = b * 0x2000;
        rom[base] = 0xAA; // valid module header, present in every bank
        rom[base + 1] = 0x01; // version
        rom[base + 6] = 0x60; // program list pointer -> >600C
        rom[base + 7] = 0x0C;
        rom[base + 0x0E] = 0x60; // program entry address -> >6016 (unused here)
        rom[base + 0x0F] = 0x16;
        rom[base + 0x10] = title.len() as u8;
        rom[base + 0x11..base + 0x11 + title.len()].copy_from_slice(title.as_bytes());
    }
    rom
}

/// A raw ROM `.bin` dump parses to consecutive ROM banks with no GROM, and its
/// title is lifted from the module header — no `.ctg` container required.
#[test]
fn raw_rom_bin_parses_without_a_container() {
    let bytes = synth_raw_rom("RAW ROM CART", 2);
    let c = Cartridge::parse(&bytes).unwrap();
    assert_eq!(c.title, "RAW ROM CART");
    assert_eq!(c.cru_base, 0);
    assert_eq!(c.rom_banks, 2, "16 KiB dump = two 8 KiB banks");
    assert_eq!(c.rom, bytes, "the raw dump is the ROM verbatim");
    assert!(c.grom.is_empty(), "a raw ROM dump carries no GROM");
}

/// **A raw `.bin` ROM cartridge reaches the selection screen** exactly as a
/// `.ctg` cartridge does: the console's power-up scan reads the module header
/// from cartridge ROM (bank 0) and lists the cartridge's name. This is the
/// `copper8.bin` path, exercised here with a synthesized raw ROM so no
/// third-party image need be committed.
#[test]
fn a_raw_rom_cartridge_appears_on_the_selection_screen() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party console firmware not present");
        return;
    };
    let cart = Cartridge::parse(&synth_raw_rom("RAW ROM CART", 2)).unwrap();

    let mut m = Machine::new(rom, grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..180 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, true);
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..120 {
        m.run_frame();
    }

    let text = screen_text(&m);
    assert!(
        text.contains("RAW ROM CART"),
        "raw-ROM cartridge menu entry not on the selection screen; screen was:\n{}",
        text
    );
}

#[test]
fn rejects_non_cartridge_data() {
    assert_eq!(
        Cartridge::parse(b"not a cartridge").unwrap_err(),
        CartridgeError::BadBanner
    );
    // A valid banner but a bogus version marker is rejected.
    let mut bad = vec![0u8; 0x54];
    bad[..18].copy_from_slice(b"TI-99/4A Module - ");
    bad[0x50] = 0x99;
    assert_eq!(
        Cartridge::parse(&bad).unwrap_err(),
        CartridgeError::UnsupportedVersion(0x99)
    );
}
