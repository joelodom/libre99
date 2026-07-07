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

//! Map the joystick deflection table below the keytab: for each Joy1 direction
//! (and diagonals), which GROM >16xx addresses does the ROM's SCAN read, and
//! what value does the AUTHENTIC GROM hold there? Reveals the table base(s),
//! extent, and index order so we can reconstruct it.

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

fn scan_loop_grom() -> Vec<u8> {
    let src = format!(
        "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
        ST   @>8374,>01
LOOP    SCAN
        B    LOOP
{}",
        libre99_gpl::keymap::emit_gpl_bytes("KEYTAB")
    );
    libre99_gpl::assemble(&src).unwrap().image
}

/// Addresses in >16E0..>1700 that SCAN reads while `held`, minus the idle set.
fn joy_reads(grom: &[u8], held: &[TiKey], idle: &[u16]) -> Vec<u16> {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    for _ in 0..12 { m.run_frame(); }
    for &k in held { m.set_key(k, true); }
    for _ in 0..6 { m.run_frame(); }
    m.bus_mut().grom_record(true);
    let start = m.bus().grom_log().len();
    for _ in 0..4 { m.run_frame(); }
    let mut v: Vec<u16> = m.bus().grom_log()[start..].iter().map(|(a, _)| *a)
        .filter(|a| (0x16C0..0x1700).contains(a) && !idle.contains(a)).collect();
    v.sort(); v.dedup();
    v
}

fn main() {
    let grom = scan_loop_grom();
    // idle baseline in the >16C0..>1700 window
    let idle = {
        let mut m = Machine::new(&CONSOLE_ROM, &grom);
        for _ in 0..12 { m.run_frame(); }
        m.bus_mut().grom_record(true);
        let s = m.bus().grom_log().len();
        for _ in 0..4 { m.run_frame(); }
        let mut v: Vec<u16> = m.bus().grom_log()[s..].iter().map(|(a, _)| *a)
            .filter(|a| (0x16C0..0x1700).contains(a)).collect();
        v.sort(); v.dedup(); v
    };

    let cases: [(&str, Vec<TiKey>); 8] = [
        ("Left",       vec![TiKey::Joy1Left]),
        ("Right",      vec![TiKey::Joy1Right]),
        ("Up",         vec![TiKey::Joy1Up]),
        ("Down",       vec![TiKey::Joy1Down]),
        ("Up+Left",    vec![TiKey::Joy1Up, TiKey::Joy1Left]),
        ("Up+Right",   vec![TiKey::Joy1Up, TiKey::Joy1Right]),
        ("Down+Left",  vec![TiKey::Joy1Down, TiKey::Joy1Left]),
        ("Down+Right", vec![TiKey::Joy1Down, TiKey::Joy1Right]),
    ];
    println!("direction -> (addr = authentic_value) pairs SCAN reads in >16xx\n");
    for (name, keys) in cases {
        let addrs = joy_reads(&grom, &keys, &idle);
        let shown: Vec<String> = addrs.iter()
            .map(|a| format!(">{a:04X}=>{:02X}", AUTHENTIC_GROM[*a as usize])).collect();
        println!("  {name:<11}: {}", shown.join("  "));
    }
    println!("\nAUTHENTIC >16E8..>1700:");
    let row: Vec<String> = (0x16E8..0x1700).map(|a| format!("{:02X}", AUTHENTIC_GROM[a])).collect();
    println!("  {}", row.join(" "));
}
