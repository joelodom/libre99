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

//! Recon probe for the system-GROM rewrite (see
//! `original-content/system-roms/RECON.md`, which records this program's
//! evidence; the original review it fed is archived in
//! `original-content/system-roms/history/`). Three probes against the real
//! console ROM + GROM:
//!
//! 1. Where does the ROM enter GPL, and what machine state does it establish?
//!    (Answer: GROM `>0020`, executed as `BR >0052`; scratchpad/VDP snapshots.)
//! 2. What VDP + scratchpad state does the finished title screen leave?
//! 3. How does the master menu list and dispatch a ROM-only cartridge?
//!    (Answer: it scans the CPU `>6000` header too, stages the entry address
//!    at `>8380`, copies it to the `>8300` vector, and launches via `XML >F0` —
//!    the dispatch vector is `>8300`; RECON Verified Mechanisms §2.)
//!
//! Run from the repo root: `cargo run -p libre99-core --example recon_probe`.
//! Keep pumping `run_frame()` (not bare `step()`) whenever a key is involved —
//! the menu's key beep is a GROM sound list drained by the VBLANK ISR, and the
//! menu polls `>83CE` until it finishes; without vblanks it waits forever.
use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

/// Load one third-party image, or exit — this probe needs the authentic media.
fn need(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2);
    })
}

fn dump_scratchpad(m: &Machine, title: &str) {
    println!("=== scratchpad >8300->83FF {title} ===");
    for row in 0..16u16 {
        let base = 0x8300 + row * 16;
        let bytes: Vec<String> = (0..16)
            .map(|i| format!("{:02X}", m.bus().peek(base + i)))
            .collect();
        println!(">{:04X}: {}", base, bytes.join(" "));
    }
}

fn dump_vdp_regs(m: &Machine, title: &str) {
    let regs: Vec<String> = (0..8).map(|n| format!("R{}=>{:02X}", n, m.vdp().register(n))).collect();
    println!("=== VDP {title}: {} ===", regs.join(" "));
}

fn dump_name_table(m: &Machine, title: &str) {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    println!("=== name table @>{base:04X} {title} (raw, and as ASCII / ASCII+>60) ===");
    for row in 0..24u16 {
        let mut raw = String::new();
        let mut plain = String::new();
        let mut off60 = String::new();
        for col in 0..32u16 {
            let b = m.vdp().vram(base + row * 32 + col);
            raw.push_str(&format!("{b:02X}"));
            let p = b as char;
            plain.push(if (0x20..0x7F).contains(&b) { p } else { '.' });
            let q = b.wrapping_sub(0x60);
            off60.push(if (0x20..0x7F).contains(&q) { q as char } else { '.' });
        }
        println!("{row:2}: {raw} | {plain} | {off60}");
    }
}

fn press(m: &mut Machine, key: TiKey, hold_frames: usize, release_frames: usize) {
    m.set_key(key, true);
    for _ in 0..hold_frames {
        m.run_frame();
    }
    m.set_key(key, false);
    for _ in 0..release_frames {
        m.run_frame();
    }
}

fn main() {
    let console_rom = need("roms/994aROM.Bin");
    let console_grom = need("roms/994AGROM.Bin");
    let centipede = need("cartridges/centipe.ctg");

    // ---- Probe 1: entry into GPL -------------------------------------------
    let mut m = Machine::new(&console_rom, &console_grom);
    m.bus_mut().grom_record(true);
    let mut steps = 0u64;
    while m.bus().grom_log().is_empty() {
        m.step();
        steps += 1;
    }
    println!("first GROM read after {steps} CPU instructions, PC=>{:04X}", m.cpu().pc());
    dump_scratchpad(&m, "at FIRST GROM read");
    dump_vdp_regs(&m, "at FIRST GROM read");
    for _ in 0..40_000 {
        m.step();
    }
    println!("=== first 400 GROM reads (addr:byte) ===");
    let log = m.bus().grom_log();
    for (i, (a, b)) in log.iter().take(400).enumerate() {
        print!("{i:>4}: >{a:04X}={b:02X}  ");
        if i % 6 == 5 {
            println!();
        }
    }
    println!();

    // ---- Probe 2: settled title-screen state --------------------------------
    let mut m = Machine::new(&console_rom, &console_grom);
    for _ in 0..180 {
        m.run_frame();
    }
    dump_vdp_regs(&m, "after 180 frames (title)");
    dump_scratchpad(&m, "after 180 frames (title)");
    dump_name_table(&m, "title");

    // ---- Probe 3: ROM-only cartridge listing + dispatch ---------------------
    let cart = Cartridge::parse(&centipede).expect("parse centipe");
    println!("=== centipe: rom_banks={} grom_pages={} ===", cart.rom_banks, cart.grom.len());
    let mut m = Machine::new(&console_rom, &console_grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..180 {
        m.run_frame();
    }
    // Leave the title screen.
    press(&mut m, TiKey::Space, 5, 60);
    dump_name_table(&m, "menu (centipe mounted)");
    // Choose entry 2 and watch for the CPU to land in cartridge ROM.
    m.bus_mut().grom_record(true);
    m.set_key(TiKey::Num2, true);
    let mut in_cart = false;
    for frame in 0..120 {
        if frame == 20 {
            m.set_key(TiKey::Num2, false);
        }
        m.run_frame();
        if (0x6000..0x8000).contains(&m.cpu().pc()) {
            in_cart = true;
            break;
        }
    }
    m.set_key(TiKey::Num2, false);
    println!("dispatched into cart ROM: {in_cart}, PC=>{:04X}", m.cpu().pc());
    let log = m.bus().grom_log();
    println!("=== last 240 GROM reads before dispatch ===");
    let start = log.len().saturating_sub(240);
    for (i, (a, b)) in log[start..].iter().enumerate() {
        print!("{:>4}: >{a:04X}={b:02X}  ", start + i);
        if i % 6 == 5 {
            println!();
        }
    }
    println!();
    dump_scratchpad(&m, "at dispatch");
}
