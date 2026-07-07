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

//! Throwaway experiment for M2: verify (1) the GPL `SCAN` opcode reads the
//! keyboard into scratchpad `>8375`, (2) a real ROM cartridge's header layout,
//! and (3) that the ML dispatch trampoline (`DST @>8380,entry ; XML >F0`) works
//! when executed from *our* rewritten GROM — the mechanism the selection list
//! will use to launch cartridges.

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

fn header(program: &str) -> String {
    format!(
        "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
{program}"
    )
}

fn build(program: &str) -> Vec<u8> {
    libre99_gpl::assemble(&header(program))
        .unwrap_or_else(|d| panic!("asm: {d:?}"))
        .image
}

fn main() {
    // ---- (1) SCAN reads the keyboard into >8375 --------------------------------
    // Set keyboard mode 0 (>8374=0), then loop SCAN forever.
    let prog = "        ST @>8374,>00
LOOP    SCAN
        B    LOOP
";
    let grom = build(prog);
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::Num5, true);
    for _ in 0..10 {
        m.run_frame();
    }
    println!(
        "SCAN: >8374(mode)=>{:02X} >8375(key)=>{:02X}  (want key >35 = '5')",
        m.bus().peek(0x8374),
        m.bus().peek(0x8375)
    );
    m.set_key(TiKey::Num5, false);

    // ---- (2) a real ROM cartridge's header (read from cart.rom directly) --------
    // cart.rom offset 0 == CPU >6000; header pointers are >6000-based CPU addrs.
    let rd = |rom: &[u8], addr: u16| -> u16 {
        let o = (addr - 0x6000) as usize;
        ((rom[o] as u16) << 8) | rom[o + 1] as u16
    };
    let mut centipe_entry = 0u16;
    for name in ["centipe", "moonpat"] {
        let data = require(&format!("cartridges/{name}.ctg"));
        let cart = Cartridge::parse(&data).unwrap();
        let rom = &cart.rom;
        let hdr: Vec<String> = (0..16).map(|i| format!("{:02X}", rom[i])).collect();
        let pl = rd(rom, 0x6006);
        let entry = rd(rom, pl + 2);
        let nlen = rom[(pl - 0x6000 + 4) as usize];
        let nm: String = (0..nlen)
            .map(|i| rom[(pl - 0x6000 + 5 + i as u16) as usize] as char)
            .collect();
        println!(
            "{name}: valid=>{:02X} hdr={} program-list=>{pl:04X} entry=>{entry:04X} name={nm:?}",
            rom[0],
            hdr.join(" ")
        );
        if name == "centipe" {
            centipe_entry = entry;
        }
    }

    // ---- (3) dispatch a mounted ROM cart from our GROM -------------------------
    let data = require("cartridges/centipe.ctg");
    let cart = Cartridge::parse(&data).unwrap();
    let entry = centipe_entry;

    let prog = format!("        DST @>8380,>{entry:04X}\n        XML >F0\nSPIN    B SPIN\n");
    let grom2 = build(&prog);
    let mut m = Machine::new(&CONSOLE_ROM, &grom2);
    m.mount_cartridge(&cart);
    m.reset();
    let mut in_cart = false;
    for _ in 0..30 {
        m.run_frame();
        if (0x6000..0x8000).contains(&m.cpu().pc()) {
            in_cart = true;
            break;
        }
    }
    println!("dispatch to centipe entry >{entry:04X}: in_cart={in_cart} PC=>{:04X}", m.cpu().pc());
}
