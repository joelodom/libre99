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

//! Guard for QUALITY-ASSESSMENT §5 item 4 (Chunk 3c): the selection menu must
//! **cap the listed programs at 9**.
//!
//! Menu entry lines start at VDP `>00E4` (row 7) and advance `>40` (two rows)
//! each with no cap; the name table ends at `>02FF` (row 23). Entry 10 lands at
//! `>0324` — inside the **sprite attribute table** (`>0300`) — so a cartridge (or
//! GROM base) declaring ten-plus programs scribbles the SAT and beyond, and the
//! entry digit runs past `'9'` into `':' ';' …` while staying selectable. No
//! bundled cartridge declares that many, so this needs a synthetic one.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

/// A pure-GROM cartridge at base `>6000` whose header declares `n` programs, all
/// chained within the first 512 bytes (so the scan window sees them all), with
/// short names (so entry lines don't overflow their row on their own).
fn synthetic_multi_program_cart(n: usize) -> Cartridge {
    let mut page = vec![0u8; 0x2000];
    page[0] = 0xAA; // valid header
    page[1] = 0x01; // version 1
    let list: u16 = 0x6010;
    page[6] = (list >> 8) as u8;
    page[7] = list as u8;
    let mut off = 0x10usize;
    for i in 1..=n {
        let name = format!("PROG{i:02}"); // 6 chars
        let entry_len = 5 + name.len();
        let next: u16 = if i < n { 0x6000 + (off + entry_len) as u16 } else { 0 };
        page[off] = (next >> 8) as u8;
        page[off + 1] = next as u8;
        page[off + 2] = 0x60; // entry address (dummy — the test never launches)
        page[off + 3] = 0x00;
        page[off + 4] = name.len() as u8;
        page[off + 5..off + 5 + name.len()].copy_from_slice(name.as_bytes());
        off += entry_len;
    }
    Cartridge {
        title: "SYNTH MULTI".into(),
        cru_base: 0,
        rom: Vec::new(),
        rom_banks: 0,
        grom: vec![(0x6000, page)],
    }
}

/// Build the menu for a cartridge with 12 programs and return `(cnt, sat_dirty)`:
/// the final entry count (`>8350`) and whether the menu scribbled text into the
/// sprite attribute table (`>0300–037F`).
fn build_menu_over_12() -> (u8, bool) {
    let console_rom = CONSOLE_ROM.as_deref().expect("presence checked by the test");
    let cart = synthetic_multi_program_cart(12);
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..320 { m.run_frame(); }

    let cnt = m.bus().peek(0x8350);
    // The menu writes "N FOR NAME" text; any of those bytes landing in the SAT
    // (>0300-037F) is corruption. Look for the 'F' of " FOR " or a digit there.
    let sat_dirty = (0x0300..0x0380u16).any(|a| {
        let b = m.vdp().vram(a);
        b == b'F' || b == b'O' || b == b'R' || b.is_ascii_digit()
    });
    (cnt, sat_dirty)
}

#[test]
fn menu_caps_listed_programs_at_nine() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let (cnt, sat_dirty) = build_menu_over_12();
    assert!(
        cnt <= 9,
        "menu must cap the list at 9 entries (a 10th lands in the sprite table); \
         a 12-program cart produced CNT={cnt}"
    );
    assert!(
        !sat_dirty,
        "menu text scribbled into the sprite attribute table (>0300-037F) — \
         entries past 9 overflow the name table"
    );
}
