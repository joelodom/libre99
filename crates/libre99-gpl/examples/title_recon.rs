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

//! Recon the **authentic** master title screen so the rewrite can recreate its
//! layout, colour bars, and beep faithfully (RECON R2). Boots the genuine
//! console ROM + GROM and dumps:
//!   1. the settled VDP registers and the 32-byte colour table (the colour-bar
//!      palette: groups 12..27 carry each bar's background colour);
//!   2. the name-table row positions of the banner text and the "TI" logo;
//!   3. the VBLANK ISR **sound-list format** — traced by watching which GROM
//!      bytes the ISR reads during the power-on beep, per frame.
//!
//! Run from the repo root: `cargo run -p libre99-gpl --example title_recon`.

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

fn main() {
    // ---- 1. Sound-list format: trace the ISR's GROM reads during the beep ----
    let mut m = Machine::new(&CONSOLE_ROM, &AUTHENTIC_GROM);
    m.bus_mut().grom_record(true);
    println!("=== power-on beep: PSG state + ISR sound-list reads (>0470..>04C0) ===");
    let mut seen = 0usize;
    for f in 0..55 {
        m.run_frame();
        let log = m.bus().grom_log();
        let reads: Vec<String> = log[seen..]
            .iter()
            .filter(|(a, _)| (0x0470..0x04C0).contains(a))
            .map(|(a, b)| format!("{a:04X}={b:02X}"))
            .collect();
        seen = log.len();
        let ch0 = (m.bus().psg.volume(0) < 0x0F)
            .then(|| format!("ch0 div={} vol={}", m.bus().psg.period(0), m.bus().psg.volume(0)));
        if ch0.is_some() || !reads.is_empty() {
            println!(
                "f{f:2}: {:<22} 83CC={:02X}{:02X} 83CE={:02X} | {}",
                ch0.unwrap_or_default(),
                m.bus().peek(0x83CC), m.bus().peek(0x83CD), m.bus().peek(0x83CE),
                reads.join(" ")
            );
        }
    }

    // ---- 2. Settled title: registers, colour table, banner/logo positions ----
    for _ in 0..130 { m.run_frame(); }
    let regs: Vec<String> = (0..8).map(|n| format!("R{n}=>{:02X}", m.vdp().register(n))).collect();
    println!("\n=== title settled: VDP {} ===", regs.join(" "));
    let ct = ((m.vdp().register(3) as u16) & 0xFF) * 0x40;
    let bars: Vec<String> = (12..28u16).map(|g| format!("{:X}", m.vdp().vram(ct + g) & 0x0F)).collect();
    println!("colour-bar bg palette (groups 12..27): {}", bars.join(" "));

    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    println!("=== name table (char codes / ASCII) ===");
    for row in 0..24u16 {
        let ascii: String = (0..32u16).map(|c| {
            let b = m.vdp().vram(base + row * 32 + c);
            match b { 0x01..=0x0A => '#', 0x20..0x60 => b as char, 0x60..=0xDF => '=', _ => '.' }
        }).collect();
        println!("{row:2}|{ascii}|");
    }
}
