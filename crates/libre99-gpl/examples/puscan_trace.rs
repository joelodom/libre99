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

//! Trace the authentic ROM's power-up scan (PUSCAN) on our no-card GROM: detect
//! each entry into SROM (>0AC0) and SGROM (>0B24) and log the GPL-visible state
//! (cond bit, cursor cells), then report when the boot reaches the title
//! key-wait. Pins the no-card contract our ROM's XML >19/>1A must reproduce.

use std::sync::LazyLock;

use libre99_core::machine::Machine;

static AUTH: LazyLock<Vec<u8>> = LazyLock::new(|| {
    libre99_core::third_party::load("roms/994aROM.Bin").unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/roms/994aROM.Bin)");
        std::process::exit(2)
    })
});

fn main() {
    let grom = libre99_gpl::system_grom::build_console_grom().expect("console GROM");
    let mut m = Machine::new(&AUTH, &grom);
    m.reset();

    let cell = |m: &Machine, a: u16| m.bus().peek(a);
    let mut srom_hits = 0;
    let mut sgrom_hits = 0;
    let mut reached_twait = None;
    let mut prev_pc = 0u16;

    for step in 0..4_000_000usize {
        m.step();
        let pc = m.cpu().pc();
        // Log the first few entries into each service (PC transition into it).
        if pc == 0x0AC0 && prev_pc != 0x0AC0 {
            srom_hits += 1;
            if srom_hits <= 4 {
                println!(
                    "  SROM  #{srom_hits} @step {step}: cond>837C={:02X} >83D0={:04X} >83D2={:04X} >836D={:02X}",
                    cell(&m, 0x837C), m.bus().peek_word(0x83D0), m.bus().peek_word(0x83D2), cell(&m, 0x836D),
                );
            }
        }
        if pc == 0x0B24 && prev_pc != 0x0B24 {
            sgrom_hits += 1;
            if sgrom_hits <= 4 {
                println!(
                    "  SGROM #{sgrom_hits} @step {step}: cond>837C={:02X} >83D0={:04X} >83D2={:04X} >836D={:02X}",
                    cell(&m, 0x837C), m.bus().peek_word(0x83D0), m.bus().peek_word(0x83D2), cell(&m, 0x836D),
                );
            }
        }
        prev_pc = pc;
        // KSCAN entry (>02B2) reached from the GROM TWAIT loop = boot finished PUSCAN.
        if pc == 0x02B2 && reached_twait.is_none() {
            reached_twait = Some(step);
            println!(
                "  reached KSCAN (TWAIT) @step {step}: SROM hits={srom_hits} SGROM hits={sgrom_hits} \
                 cond>837C={:02X} >83D0={:04X}",
                cell(&m, 0x837C), m.bus().peek_word(0x83D0),
            );
            break;
        }
    }
    if reached_twait.is_none() {
        println!("  never reached KSCAN; SROM hits={srom_hits} SGROM hits={sgrom_hits}");
    }
}
