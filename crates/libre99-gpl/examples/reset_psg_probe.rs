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

//! Differential probe for QUALITY-ASSESSMENT §5 item 5 (Chunk 3b): does a
//! console reset (F5) silence a cartridge's sound that was mid-play?
//!
//! The VBLANK ISR keeps writing the SN76489 from a GPL sound list. When a game
//! is playing a multi-channel tune and the user presses F5, `reset()` re-runs
//! the boot but does NOT clear the PSG (the sound chip keeps its latches). The
//! authentic boot's power-on beep list at GROM `>0484` opens with `BF DF FF` —
//! mute channels 1/2/3 — so the drone stops. Our `SND` list did not, so the old
//! tune droned over our beep on the title screen ("no fun beep" after F5).
//!
//! This probe injects a loud 3-channel tone (as a cartridge would, by writing
//! the sound chip), resets, runs the boot, and samples the PSG — under the
//! authentic GROM and ours — so the fix can be verified against the oracle.

use std::sync::LazyLock;

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

/// Latch loud tones on channels 1, 2, 3 (and noise), as a game's sound would —
/// low attenuation = audible. Channel select is bits 6-5; bit 4 picks tone(0)/
/// volume(1); mute = attenuation >0F.
fn start_three_channels(m: &mut Machine) {
    let psg = &mut m.bus_mut().psg;
    // Channel 1 tone + loud.
    psg.write(0xA0); psg.write(0x10); psg.write(0xB0);
    // Channel 2 tone + loud.
    psg.write(0xC0); psg.write(0x18); psg.write(0xD0);
    // Channel 3 noise + loud.
    psg.write(0xE4); psg.write(0xF0);
}

fn audible(m: &Machine, ch: usize) -> bool {
    m.bus().psg.volume(ch) < 0x0F
}

fn probe(label: &str, grom: &[u8]) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.reset();
    for _ in 0..60 { m.run_frame(); } // settle the boot

    start_three_channels(&mut m);
    let before: Vec<bool> = (0..4).map(|ch| audible(&m, ch)).collect();

    // The F5 the user presses. reset() does not clear the PSG.
    m.reset();
    for _ in 0..80 { m.run_frame(); } // run the boot; the beep list should mute

    let after: Vec<bool> = (0..4).map(|ch| audible(&m, ch)).collect();
    let drone = (1..4).any(|ch| after[ch]);
    println!(
        "{label:26}  injected ch1-3 audible={:?}  |  after F5 ch0-3 audible={:?}  =>  {}",
        &before[1..4],
        after,
        if drone { "DRONE (channels 1-3 not muted on reset)" } else { "clean (channels 1-3 muted)" }
    );
}

fn main() {
    let ours = libre99_gpl::system_grom::build_console_grom().unwrap();
    println!("Reset/F5 PSG-mute differential probe (QUALITY-ASSESSMENT §5 item 5):\n");
    probe("AUTHENTIC GROM (oracle)", &AUTHENTIC_GROM);
    probe("OURS", &ours);
}
