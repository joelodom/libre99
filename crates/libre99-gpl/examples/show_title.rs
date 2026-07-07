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

//! Boot the rewritten console GROM on the real ROM and print the title screen
//! as ASCII (the VDP name table) plus a PPM screenshot to the temp dir, so the
//! Milestone-1 result can be eyeballed.

use std::sync::LazyLock;

use libre99_core::machine::Machine;
use libre99_core::vdp::{HEIGHT, WIDTH};

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    libre99_core::third_party::load("roms/994aROM.Bin").unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/roms/994aROM.Bin)");
        std::process::exit(2)
    })
});

fn main() {
    let grom = libre99_gpl::system_grom::build_console_grom().expect("assemble console GROM");
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    for _ in 0..60 {
        m.run_frame();
    }
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    println!("+{}+", "-".repeat(32));
    for row in 0..24u16 {
        let line: String = (0..32u16)
            .map(|c| {
                let b = m.vdp().vram(base + row * 32 + c);
                if (0x20..0x7F).contains(&b) {
                    b as char
                } else {
                    ' '
                }
            })
            .collect();
        println!("|{line}|");
    }
    println!("+{}+", "-".repeat(32));
    println!(
        "R1=>{:02X} R2=>{:02X} R7=>{:02X}",
        m.vdp().register(1),
        m.vdp().register(2),
        m.vdp().register(7)
    );

    let mut fb = vec![0u32; WIDTH * HEIGHT];
    m.render(&mut fb);
    let mut ppm = format!("P6\n{WIDTH} {HEIGHT}\n255\n").into_bytes();
    for &px in &fb {
        ppm.push((px >> 16) as u8);
        ppm.push((px >> 8) as u8);
        ppm.push(px as u8);
    }
    let path = std::env::temp_dir().join("rewrite_title.ppm");
    std::fs::write(&path, ppm).unwrap();
    eprintln!("wrote {}", path.display());
}
