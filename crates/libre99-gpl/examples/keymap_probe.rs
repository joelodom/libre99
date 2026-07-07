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

//! Discover the console ROM's KSCAN key-table INTERFACE: for every character
//! key (unshifted and shifted), boot the authentic GROM, press it, and record
//! (a) the GROM table offset the ROM reads and (b) the ASCII it deposits in
//! >8375. This maps scan-code offset -> key, so we can author an equivalent
//! table in our rewrite from the ASCII standard (clean-room: we take only the
//! functional offsets here, and fill the values ourselves).

use std::sync::LazyLock;

use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994AGROM.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

fn probe(key: TiKey, shift: bool) -> (Option<u16>, u8) {
    let mut m = Machine::new(&CONSOLE_ROM, &AUTHENTIC_GROM);
    for _ in 0..180 { m.run_frame(); }
    m.bus_mut().grom_record(true);
    if shift { m.set_key(TiKey::Shift, true); }
    m.set_key(key, true);
    let mut ascii = 0u8;
    for _ in 0..4 {
        m.run_frame();
        let k = m.bus().peek(0x8375);
        if k != 0xFF && k != 0x00 { ascii = k; }
    }
    m.set_key(key, false);
    if shift { m.set_key(TiKey::Shift, false); }
    let off = m.bus().grom_log().iter().map(|(a, _)| *a)
        .find(|a| (0x1700..0x1800).contains(a));
    (off, ascii)
}

fn main() {
    let keys = [
        (TiKey::Num0, '0'), (TiKey::Num1, '1'), (TiKey::Num2, '2'), (TiKey::Num3, '3'),
        (TiKey::Num4, '4'), (TiKey::Num5, '5'), (TiKey::Num6, '6'), (TiKey::Num7, '7'),
        (TiKey::Num8, '8'), (TiKey::Num9, '9'),
        (TiKey::A, 'A'), (TiKey::B, 'B'), (TiKey::C, 'C'), (TiKey::D, 'D'), (TiKey::E, 'E'),
        (TiKey::F, 'F'), (TiKey::G, 'G'), (TiKey::H, 'H'), (TiKey::I, 'I'), (TiKey::J, 'J'),
        (TiKey::K, 'K'), (TiKey::L, 'L'), (TiKey::M, 'M'), (TiKey::N, 'N'), (TiKey::O, 'O'),
        (TiKey::P, 'P'), (TiKey::Q, 'Q'), (TiKey::R, 'R'), (TiKey::S, 'S'), (TiKey::T, 'T'),
        (TiKey::U, 'U'), (TiKey::V, 'V'), (TiKey::W, 'W'), (TiKey::X, 'X'), (TiKey::Y, 'Y'),
        (TiKey::Z, 'Z'),
        (TiKey::Equals, '='), (TiKey::Period, '.'), (TiKey::Comma, ','),
        (TiKey::Semicolon, ';'), (TiKey::Slash, '/'), (TiKey::Space, ' '), (TiKey::Enter, '\n'),
    ];
    println!("=== UNSHIFTED (base >1705) ===");
    let mut rows: Vec<(u16, char, u8)> = Vec::new();
    for (k, c) in keys {
        let (off, a) = probe(k, false);
        if let Some(o) = off { rows.push((o, c, a)); }
    }
    rows.sort();
    for (o, c, a) in &rows {
        println!("  off >{o:04X} (+{:2}) key {c:?} -> ascii >{a:02X}", o - 0x1705);
    }
    println!("=== SHIFTED (base >1735) ===");
    let mut rows: Vec<(u16, char, u8)> = Vec::new();
    for (k, c) in keys {
        let (off, a) = probe(k, true);
        if let Some(o) = off { rows.push((o, c, a)); }
    }
    rows.sort();
    for (o, c, a) in &rows {
        println!("  off >{o:04X} (+{:2}) shift-{c:?} -> ascii >{a:02X}", o.wrapping_sub(0x1735) as i16);
    }
}
