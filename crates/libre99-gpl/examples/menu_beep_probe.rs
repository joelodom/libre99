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

//! L7 probe (QUALITY-ASSESSMENT §7.5): does the menu beep on a *rejected*
//! keypress? Our menu beeps on leaving the title and on a valid selection but
//! not on an out-of-range key. To match the authentic console we must know what
//! it does — so this drives both GROMs to the selection screen, settles any
//! prior click, presses an out-of-range digit, and reports whether channel 0
//! beeps in the frames after.

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

fn ch0_beeps_over(m: &mut Machine, n: usize) -> bool {
    let mut beeped = false;
    for _ in 0..n {
        m.run_frame();
        if m.bus().psg.volume(0) < 0x0F {
            beeped = true;
        }
    }
    beeped
}

fn probe(label: &str, grom: &[u8]) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.reset();
    for _ in 0..180 { m.run_frame(); } // title settles
    // Leave the title.
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..200 { m.run_frame(); } // selection screen builds; any click drains

    let quiet_before = !ch0_beeps_over(&mut m, 20); // confirm silence going in

    // Press an out-of-range digit (9 — far more than any menu offers here).
    m.set_key(TiKey::Num9, true);
    for _ in 0..6 { m.run_frame(); }
    m.set_key(TiKey::Num9, false);
    let beeped = ch0_beeps_over(&mut m, 40);

    println!(
        "{label:26}  quiet-before={:5}  beep-after-rejected-key={}",
        quiet_before, beeped
    );
}

fn main() {
    let ours = libre99_gpl::system_grom::build_console_grom().unwrap();
    println!("L7 reject-key beep probe (no cartridge; press 9 = out of range):\n");
    probe("AUTHENTIC GROM (oracle)", &AUTHENTIC_GROM);
    probe("OURS", &ours);
}
