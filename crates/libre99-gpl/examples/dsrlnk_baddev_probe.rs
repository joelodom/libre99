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

//! Probe for QUALITY-ASSESSMENT §5 item 6 (Chunk 3a): does requesting a device
//! we don't serve HANG, or does it error gracefully?
//!
//! §5 item 6 hypothesized that DSRLNK, by not checking the `XML >19` search
//! result, "barrels into `XML >1A`" and hangs on a bad device. But the authentic
//! DSRLNK (`>03DC`) is byte-identical to ours and does the same thing, so the
//! premise needs execution. This drives Tunnels of Doom to "LOAD DATA FROM" and
//! picks **CASSETTE (CS1)** — whose DSR is in the console ROM, not on a card, and
//! which our GROM does not ship — then watches for a hang (the ISR counter at
//! `>8379` stops advancing / PC frozen) versus a graceful return, authentic vs
//! ours.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994AGROM.Bin"));
static DSR: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/Disk.Bin"));
static TUNNELS: LazyLock<Vec<u8>> = LazyLock::new(|| require("disks/Tunnels.Dsk"));

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
        let t = line.trim_end();
        if !t.is_empty() {
            s.push_str(&format!("        |{}|\n", line));
        }
    }
    s
}

/// Hold a key 6 frames, release, run `settle` frames counting ISR ticks (>8379
/// changes). A live console keeps ticking; a hung one stops.
fn tap(m: &mut Machine, k: TiKey, settle: usize) -> u32 {
    m.set_key(k, true);
    for _ in 0..6 { m.run_frame(); }
    m.set_key(k, false);
    let mut prev = m.bus().peek(0x8379);
    let mut ticks = 0;
    for _ in 0..settle {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev { ticks += 1; }
        prev = t;
    }
    ticks
}

fn run(label: &str, grom: &[u8], cart: &Cartridge) {
    println!("\n================= {label} =================");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.load_disk_controller(&DSR);
    m.mount_disk(0, TUNNELS.to_vec());
    m.mount_cartridge(cart);
    m.reset();
    for _ in 0..180 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 40); // title -> selection list
    tap(&mut m, TiKey::Num2, 240); // select Tunnels of Doom
    let t = tap(&mut m, TiKey::Enter, 120); // -> "LOAD DATA FROM"
    println!("[at LOAD DATA FROM: {t} ticks over ~120 frames]");
    println!("{}", screen(&m));

    // Pick CASSETTE (option 1) instead of DISK, then proceed past any prompt.
    let t1 = tap(&mut m, TiKey::Num1, 120);
    println!("[after picking 1 (CASSETTE): {t1} ticks]");
    println!("{}", screen(&m));
    let t2 = tap(&mut m, TiKey::Enter, 300);
    println!("[after ENTER (proceed): {t2} ticks over ~300 frames]");
    let t3 = tap(&mut m, TiKey::Enter, 300);
    println!("[after ENTER again: {t3} ticks; PC={:04X}]", m.cpu().pc());
    println!("{}", screen(&m));

    let cs1_hung = t2 == 0 && t3 == 0;
    println!(
        "  CS1 VERDICT: {}",
        if cs1_hung { "HUNG (ISR stopped)" } else { "alive (graceful)" }
    );

    // --- Case 2: a garbage device via "OTHER" (option 3) ---
    // Recover to the LOAD DATA FROM menu, pick OTHER, type a device no card
    // serves, and submit. This is the "device not found by any card" case.
    tap(&mut m, TiKey::Enter, 60);
    tap(&mut m, TiKey::Num3, 120); // OTHER
    println!("\n[after picking 3 (OTHER) — prompt:]");
    println!("{}", screen(&m));
    for k in [TiKey::Z, TiKey::Z, TiKey::Num1, TiKey::Period, TiKey::X] {
        tap(&mut m, k, 8); // type "ZZ1.X"
    }
    let g2 = tap(&mut m, TiKey::Enter, 300);
    let g3 = tap(&mut m, TiKey::Enter, 300);
    println!("[after submitting garbage device ZZ1: {g2}+{g3} ticks; PC={:04X}]", m.cpu().pc());
    println!("{}", screen(&m));
    let other_hung = g2 == 0 && g3 == 0;
    println!(
        "  GARBAGE-DEVICE VERDICT: {}",
        if other_hung { "HUNG (ISR stopped)" } else { "alive (graceful)" }
    );
}

fn main() {
    let cart = ["cartridges/tundoom.ctg", "cartridges/tunnelsofdoom.ctg"]
        .iter()
        .find_map(|p| libre99_core::third_party::load(p))
        .map(|d| Cartridge::parse(&d).unwrap())
        .unwrap_or_else(|| {
            eprintln!("this probe needs third-party media (third-party/cartridges/tundoom.ctg)");
            std::process::exit(2)
        });
    run("AUTHENTIC GROM (oracle)", &AUTHENTIC_GROM, &cart);
    let ours = libre99_gpl::system_grom::build_console_grom().unwrap();
    run("OURS", &ours, &cart);
}
