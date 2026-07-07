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

//! Isolate the SCAN blocker: does the AUTHENTIC firmware deposit a pressed key
//! into >8375, and does driving KSCAN the way the ROM expects register it? If
//! the authentic menu populates >8375 but our bare-SCAN GROM does not, the fix
//! is setup, not the opcode.

use std::sync::LazyLock;

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

fn scan_for(m: &Machine, want: u8) -> Vec<u16> {
    (0x8300..0x8400u16).filter(|&a| m.bus().peek(a) == want).collect()
}

fn main() {
    // Authentic firmware: boot to the title/menu, hold '5', pump frames.
    let mut m = Machine::new(&CONSOLE_ROM, &CONSOLE_GROM);
    for _ in 0..120 {
        m.run_frame();
    }
    m.set_key(TiKey::Num5, true);
    for _ in 0..30 {
        m.run_frame();
    }
    println!(
        "authentic: >8374=>{:02X} >8375=>{:02X} >8376=>{:02X} >837C=>{:02X}",
        m.bus().peek(0x8374),
        m.bus().peek(0x8375),
        m.bus().peek(0x8376),
        m.bus().peek(0x837C)
    );
    println!("authentic cells holding >35 ('5'): {:04X?}", scan_for(&m, 0x35));
    println!("authentic cells holding >B5/>05: {:04X?} {:04X?}", scan_for(&m, 0xB5), scan_for(&m, 0x05));
    m.set_key(TiKey::Num5, false);

    // Our GROM: a CONDITIONAL scan loop that stops when SCAN reports a new key.
    // BR branches on condition-reset; if SCAN sets the condition on a new key,
    // `SCAN ; BR LOOP` loops while no key and falls through to a spin when one
    // arrives, leaving the key latched in >8375.
    let src = "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
        ST   @>8374,>00
LOOP    SCAN
        BR   LOOP
DONE    B    DONE
";
    let _ = src;
    for polarity in ["BR", "BS"] {
        let src = format!(
            "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
        ST   @>8374,>00
LOOP    SCAN
        {polarity}   LOOP
DONE    B    DONE
"
        );
        let grom = libre99_gpl::assemble(&src).unwrap().image;
        let mut m = Machine::new(&CONSOLE_ROM, &grom);
        for _ in 0..8 {
            m.run_frame();
        }
        m.set_key(TiKey::Num5, true);
        for _ in 0..20 {
            m.run_frame();
        }
        // Peek >8375 and scan the whole pad for the key code, since the loop may
        // stash it elsewhere.
        println!(
            "ours ({polarity} loop): >8375=>{:02X}  cells w/ >35: {:04X?}",
            m.bus().peek(0x8375),
            scan_for(&m, 0x35)
        );
        m.set_key(TiKey::Num5, false);
    }
}
