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

//! M7 step 1 — how does the authentic console boot run the **peripheral DSR
//! power-up**? That scan enables each card, and the disk card's power-up routine
//! reserves a VRAM buffer and lowers `>8370` (top of free VRAM) from `>3FFF` to
//! `>37D7` (docs/STATUS.md). Our `START` skips it, so `>8370 = 0` and the disk
//! DSR later stalls. This probe boots the authentic GROM WITH the disk controller
//! and finds the moment the card is enabled / `>8370` is set, plus the GPL/GROM
//! context that drove it — the interface our boot must reproduce.
//!
//! Run: cargo run -p libre99-gpl --example powerup_probe

use std::sync::LazyLock;

use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994AGROM.Bin"));
static DSR: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/Disk.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

fn main() {
    let mut m = Machine::new(&CONSOLE_ROM, &AUTHENTIC_GROM);
    m.load_disk_controller(&DSR);
    m.reset();
    m.bus_mut().grom_record(true);
    m.bus_mut().disk.record(true);

    let mut prev8370 = m.bus().peek_word(0x8370);
    let mut prev_dt = 0usize;
    println!("boot: >8370 starts = >{prev8370:04X}");
    for f in 0..200 {
        let before = m.bus().grom_log().len();
        m.run_frame();
        let log = m.bus().grom_log();

        // First disk CRU/register activity this frame => the power-up touched it.
        let dt = m.bus().disk.trace();
        if dt.len() != prev_dt {
            let news: Vec<String> = dt[prev_dt..]
                .iter()
                .map(|(k, a, v)| format!("{}{:04X}={:02X}", *k as char, a, v))
                .collect();
            println!("frame {f}: disk activity: {}", news.join(" "));
            // GROM fetch tail this frame — the GPL that drove it.
            let lo = before.max(log.len().saturating_sub(24));
            let tail: Vec<String> = log[lo..].iter().map(|(a, b)| format!("{a:04X}={b:02X}")).collect();
            println!("  grom tail: {}", tail.join(" "));
            prev_dt = dt.len();
        }

        let now = m.bus().peek_word(0x8370);
        if now != prev8370 {
            println!("frame {f}: >8370 {:04X} -> {:04X}  (top-of-free-VRAM reserved)", prev8370, now);
            prev8370 = now;
        }
    }
    println!("final >8370 = >{:04X}", m.bus().peek_word(0x8370));

    // All XML opcodes the boot GPL executed, in order (0F nn in the fetch stream)
    // — the ROM-delegation points; one of these is likely the power-up scanner.
    let log = m.bus().grom_log();
    let mut xmls: Vec<(u16, u8)> = Vec::new();
    let mut i = 0;
    while i + 1 < log.len() {
        if log[i].1 == 0x0F && log[i].0 < 0x6000 {
            xmls.push((log[i].0, log[i + 1].1));
        }
        i += 1;
    }
    xmls.dedup();
    println!("\nXML calls during boot (addr -> XML #): {:02X?}", xmls);

    // Disk CRU latch summary + any FD1771 commands during boot.
    let dt = m.bus().disk.trace();
    let cru: Vec<String> = dt.iter().filter(|(k, _, _)| *k == b'C').map(|(_, a, v)| format!("bit{a}={v}")).collect();
    println!("disk CRU writes during boot: {}", cru.join(" "));

    // Decode the authentic peripheral power-up scanner (~>0183) to reproduce it,
    // and the boot code just before that reaches it.
    let dump = |from: usize, to: usize| {
        let mut a = from;
        while a < to {
            match libre99_gpl::decode::decode_at(&AUTHENTIC_GROM, a, a as u16) {
                Ok(d) => {
                    let raw: Vec<String> = (a..(a + d.len).min(AUTHENTIC_GROM.len()))
                        .map(|o| format!("{:02X}", AUTHENTIC_GROM[o])).collect();
                    println!("  >{a:04X}: {:<7} {:<26} [{}]", d.mnemonic, format!("{:?}", d.operands), raw.join(" "));
                    a += d.len.max(1);
                }
                Err(_) => { println!("  >{a:04X}: <data> [{:02X}]", AUTHENTIC_GROM[a]); a += 1; }
            }
        }
    };
    println!("\n--- authentic power-up scanner >0180..>01C0 ---");
    dump(0x0180, 0x01C0);
    println!("\n--- how it's reached: search for CALL/B >0180 or >0183 in GROM 0 ---");
    for a in 0x0060..0x0800 {
        // CALL (06) or B (05) with a 16-bit target of 0180/0183
        if (AUTHENTIC_GROM[a] == 0x06 || AUTHENTIC_GROM[a] == 0x05) && a + 2 < 0x0800 {
            let t = ((AUTHENTIC_GROM[a + 1] as u16) << 8) | AUTHENTIC_GROM[a + 2] as u16;
            if t == 0x0180 || t == 0x0183 {
                println!("  >{a:04X}: {} >{t:04X}", if AUTHENTIC_GROM[a] == 6 { "CALL" } else { "B" });
            }
        }
    }
}
