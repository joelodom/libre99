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

//! Trace how the AUTHENTIC menu reads cartridge GROM headers: boot the real
//! GROM with a GROM-only cart (amazing), record all GROM fetches from the
//! title keypress on, and print the fetch stream around the first reads of the
//! cartridge header region (>6000-60FF). The instruction bytes fetched from
//! console GROM 0 just before those reads reveal the exact computed-GROM-read
//! mechanism (opcode + operand layout) the menu uses.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994AGROM.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

fn main() {
    let data = require("cartridges/amazing.ctg");
    let cart = Cartridge::parse(&data).unwrap();
    let mut m = Machine::new(&CONSOLE_ROM, &CONSOLE_GROM);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..150 {
        m.run_frame();
    }
    // Leave the title screen; the menu scan happens right after the keypress.
    m.bus_mut().grom_record(true);
    m.set_key(TiKey::Space, true);
    for _ in 0..5 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..60 {
        m.run_frame();
    }

    let log = m.bus().grom_log();
    // Find the first fetch in the cartridge header region.
    let first_hdr = log.iter().position(|(a, _)| (0x6000..0x6100).contains(a));
    match first_hdr {
        None => println!("no cartridge-region GROM reads recorded ({} fetches)", log.len()),
        Some(i) => {
            let lo = i.saturating_sub(60);
            let hi = (i + 60).min(log.len());
            println!("fetch stream around first >6000 header read (index {i} of {}):", log.len());
            for (j, (a, b)) in log[lo..hi].iter().enumerate() {
                let mark = if lo + j == i { " <== first header read" } else { "" };
                print!("{:>6}: >{a:04X}={b:02X}{mark}  ", lo + j);
                if j % 4 == 3 || !mark.is_empty() {
                    println!();
                }
            }
            println!();
        }
    }
}
