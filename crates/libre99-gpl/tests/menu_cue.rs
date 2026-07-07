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

//! Gate for **L5** (Chunk 4b): the menu's "SCANNING" progress cue. The base
//! scan is visibly slow (the console ROM re-writes the GROM address per byte —
//! `LIMITATIONS.md` L5), so `MENU` draws an original "SCANNING" row (row 6)
//! before walking the bases and `SGET` erases it once the list is ready. This
//! asserts both halves: the cue shows while the scan runs, and it is gone (and
//! the list is intact) once the menu settles.

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

/// Read 16 characters of row 6 (VDP name-table offset >00C4, col 4).
fn row6(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..16u16)
        .map(|c| {
            let b = m.vdp().vram(base + 0x00C4 + c);
            if (0x20..0x7F).contains(&b) { b as char } else { ' ' }
        })
        .collect()
}

/// Full screen text, for the settled-menu assertions.
fn screen(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..24 * 32)
        .map(|i| {
            let b = m.vdp().vram(base + i);
            if (0x20..0x7F).contains(&b) { b as char } else { ' ' }
        })
        .collect()
}

/// The cue is drawn at `MENU` (before the scan) and cleared at `SGET` (after).
/// Boot a cart, leave the title, sample row 6 early (cue up) and once settled
/// (cue gone, list drawn).
#[test]
fn scanning_cue_shows_during_scan_then_clears() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(cart) = ["cartridges/HuntTheWumpus.ctg", "cartridges/amazing.ctg"]
        .iter()
        .find_map(|p| libre99_core::third_party::load(p))
        .map(|d| Cartridge::parse(&d).unwrap())
    else {
        skip!()
    };

    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);

    // Early in the build (cue drawn at MENU, before/while the bases are scanned).
    for _ in 0..6 { m.run_frame(); }
    let during = row6(&m);
    assert!(
        during.starts_with("SCANNING"),
        "the progress cue should show while the scan runs; row 6 was `{during}`"
    );

    // Settle the menu; the cue must be erased and the list intact.
    for _ in 0..300 { m.run_frame(); }
    let settled = row6(&m);
    assert!(
        settled.trim().is_empty(),
        "the cue must be cleared once the menu is ready; row 6 was `{settled}`"
    );
    let screen = screen(&m);
    assert!(
        screen.contains(" FOR "),
        "the settled menu should still list programs; screen:\n{screen}"
    );
}
