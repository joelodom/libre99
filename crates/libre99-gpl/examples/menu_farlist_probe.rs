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

//! Probe for LIMITATIONS L2 (Chunk 4a): far-list carts now *list* under our
//! GROM — confirm they also record the right menu-table entry and launch.
//! Boots STARSHIP PEGASUS (list at slot >7801) and EXTENDED BASIC V2.5 (>6A01),
//! prints the menu, the menu-table {KIND,ENTRY} recorded for slot 2, then
//! presses "2" and reports the first GROM reads >=6000 (should begin at the
//! cart's entry) and the resulting screen.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

fn screen(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let mut s = String::new();
    for row in 0..24u16 {
        let line: String = (0..32u16)
            .map(|c| {
                let b = m.vdp().vram(base + row * 32 + c);
                if (0x20..0x7F).contains(&b) { b as char } else { '.' }
            })
            .collect();
        if !line.trim_end().is_empty() {
            s.push_str(&format!("    |{}|\n", line));
        }
    }
    s
}

fn run(name: &str) {
    println!("\n================= {name} =================");
    let data = require(&format!("cartridges/{name}.ctg"));
    let cart = Cartridge::parse(&data).unwrap();
    let grom = libre99_gpl::system_grom::build_console_grom().unwrap();
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..320 { m.run_frame(); }

    println!("{}", screen(&m));

    // Menu table lives at VDP >3800; each slot is {KIND(1?)/word, ENTRY word}.
    // SCANW writes KIND at +0 (a word store of the byte) and ENTRY at +2.
    let mt = |o: u16| m.vdp().vram(0x3800 + o);
    for slot in 0..3u16 {
        let b = slot * 4;
        let kind = (mt(b) as u16) << 8 | mt(b + 1) as u16;
        let entry = (mt(b + 2) as u16) << 8 | mt(b + 3) as u16;
        println!("  menu-table slot {slot}: KIND={kind:04X} ENTRY={entry:04X}");
    }

    let pk = |m: &Machine, a: u16| m.bus().peek(a);
    println!("  pre-'2':  CNT(>8350)={:02X} beep(>83CE)={:02X} sound-ptr >83CC={:02X}{:02X} (= KBEEP addr)",
        pk(&m, 0x8350), pk(&m, 0x83CE), pk(&m, 0x83CC), pk(&m, 0x83CD));

    m.bus_mut().grom_record(true);
    m.set_key(TiKey::Num2, true);
    for _ in 0..20 { m.run_frame(); }
    m.set_key(TiKey::Num2, false);
    for _ in 0..200 { m.run_frame(); }
    let log = m.bus().grom_log();
    let hi: Vec<String> = log.iter().filter(|(a, _)| *a >= 0x6000).take(8)
        .map(|(a, _)| format!("{a:04X}")).collect();
    let n = log.iter().filter(|(a, _)| *a >= 0x6000).count();
    println!("  after '2': {n} GROM reads >=6000; first few: {hi:?}");
    let isr_a = pk(&m, 0x8379);
    for _ in 0..10 { m.run_frame(); }
    let isr_b = pk(&m, 0x8379);
    println!("  post-'2': KEY(>8354)={:02X} CNT(>8350)={:02X} MAXK(>8355)={:02X} beep(>83CE)={:02X} KIND(>8352)={:02X} ENTRY(>8344)={:02X}{:02X} PC={:04X}",
        pk(&m, 0x8354), pk(&m, 0x8350), pk(&m, 0x8355), pk(&m, 0x83CE),
        pk(&m, 0x8352), pk(&m, 0x8344), pk(&m, 0x8345), m.cpu().pc());
    println!("  ISR counter >8379: {isr_a:02X} then {isr_b:02X} ({}), sound-ptr >83CC={:02X}{:02X}",
        if isr_a != isr_b { "ALIVE" } else { "STALLED" }, pk(&m, 0x83CC), pk(&m, 0x83CD));
    println!("{}", screen(&m));
}

fn main() {
    run("HuntTheWumpus"); // near-list control: '2' launches normally
    run("starpeg");
    run("xb25");
}
