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

//! Does the console ROM's SCAN consult a THIRD (FCTN) key-table block for the
//! arrow keys (FCTN+S/D/E/X = left/right/up/down)? Boot the authentic GROM,
//! press FCTN+<key>, and record (a) which GROM >1700-block offset the ROM reads
//! and (b) the ASCII it deposits in >8375. Then repeat against OUR rewritten
//! GROM, which only emits the unshifted+shifted blocks (>1705/>1735).

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

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

/// Press FCTN+key against `grom`, return (grom offset read in >1700..>1800, ascii in >8375).
fn probe(grom: &[u8], key: TiKey) -> (Option<u16>, u8) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    for _ in 0..180 {
        m.run_frame();
    }
    m.bus_mut().grom_record(true);
    let start = m.bus().grom_log().len();
    m.set_key(TiKey::Fctn, true);
    m.set_key(key, true);
    let mut ascii = 0u8;
    for _ in 0..6 {
        m.run_frame();
        let k = m.bus().peek(0x8375);
        if k != 0xFF && k != 0x00 {
            ascii = k;
        }
    }
    m.set_key(key, false);
    m.set_key(TiKey::Fctn, false);
    let off = m.bus().grom_log()[start..]
        .iter()
        .map(|(a, _)| *a)
        .find(|a| (0x1700..0x1800).contains(a));
    (off, ascii)
}

fn main() {
    // FCTN + these keys are the four arrow directions on the TI-99/4A.
    let arrows = [
        (TiKey::S, "LEFT  (FCTN+S)"),
        (TiKey::D, "RIGHT (FCTN+D)"),
        (TiKey::E, "UP    (FCTN+E)"),
        (TiKey::X, "DOWN  (FCTN+X)"),
    ];
    let ours = our_grom();
    for (key, name) in arrows {
        let (ao, aa) = probe(&AUTHENTIC_GROM, key);
        let (oo, oa) = probe(&ours, key);
        println!(
            "{name}:  authentic read {:>7} -> >{aa:02X}    ours read {:>7} -> >{oa:02X}",
            ao.map(|o| format!(">{o:04X}")).unwrap_or("(none)".into()),
            oo.map(|o| format!(">{o:04X}")).unwrap_or("(none)".into()),
        );
    }
}
