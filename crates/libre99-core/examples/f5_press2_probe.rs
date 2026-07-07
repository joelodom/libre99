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

//! Repro probe for Joel's 2026-07-03 report: "play TI INVADERS, F5 to reset,
//! pass the title with a key, press 2 — nothing happens."
//!
//! Protocol steps 1-3 (DEBUGGING.md): reproduce the exact flow cold (control)
//! and warm (F5 after playing), under BOTH our committed GROM artifact (the
//! bytes Joel actually ran — `grom/console-grom.bin`, read from disk, NOT
//! `build_console_grom()`, which would pick up the sibling session's in-flight
//! console.gpl edits) and the authentic GROM (the emulator-innocence control).
//! On the failing leg it prints the health panel, the scratchpad cells the
//! selection path reads, the VDP menu table at >3800, and a histogram of the
//! GROM fetch addresses after the '2' press — which pins the exact loop the
//! menu is stuck in (SGET wait vs SBWAIT beep-drain vs launched-and-crashed).
//!
//! Run from the repo root:  cargo run -p libre99-core --example f5_press2_probe

use std::collections::BTreeMap;
use std::sync::LazyLock;

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

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| need("roms/994aROM.Bin"));
static INVADERS: LazyLock<Vec<u8>> = LazyLock::new(|| need("cartridges/TI-Invaders.ctg"));

fn tap(m: &mut Machine, k: TiKey, hold: usize, settle: usize) {
    m.set_key(k, true);
    for _ in 0..hold {
        m.run_frame();
    }
    m.set_key(k, false);
    for _ in 0..settle {
        m.run_frame();
    }
}

fn row(m: &Machine, r: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..32)
        .map(|c| {
            let b = m.vdp().vram(base + r * 32 + c);
            if (0x20..0x7F).contains(&b) { b as char } else { '.' }
        })
        .collect()
}

fn pad_snapshot(m: &Machine) -> [u8; 0x100] {
    let mut s = [0u8; 0x100];
    for (i, cell) in s.iter_mut().enumerate() {
        *cell = m.bus().peek(0x8300 + i as u16);
    }
    s
}

/// Press '2' on the menu and watch what happens. Returns (launched, fetches
/// into the cartridge GROM slot, histogram of sub->2000 fetch addresses).
fn press2(m: &mut Machine) -> (bool, usize, BTreeMap<u16, usize>) {
    m.bus_mut().grom_record(true);
    tap(m, TiKey::Num2, 6, 0);
    for _ in 0..240 {
        m.run_frame();
    }
    let log: Vec<(u16, u8)> = m.bus_mut().grom_log().to_vec();
    m.bus_mut().grom_record(false);
    let cart_fetches = log.iter().filter(|(a, _)| (0x6000..0x8000).contains(a)).count();
    // Histogram of console-GROM fetches (the menu code region) — a tight
    // cluster = the loop we're stuck in.
    let mut hist: BTreeMap<u16, usize> = BTreeMap::new();
    for (a, _) in log.iter().filter(|(a, _)| *a < 0x2000) {
        *hist.entry(*a).or_default() += 1;
    }
    (cart_fetches > 500, cart_fetches, hist)
}

struct Leg {
    name: &'static str,
    launched: bool,
    cart_fetches: usize,
    menu_row7: String,
    menu_row9: String,
    pad_at_menu: [u8; 0x100],
    vdp_table: [u8; 16],
    isr_moving: bool,
    vdp_r1: u8,
    int9901: bool,
    int_pending: bool,
    hist: BTreeMap<u16, usize>,
}

/// Drive one leg: boot, (optionally launch Invaders + play + F5), reach the
/// menu, snapshot, press 2.
fn leg(grom: &[u8], warm: bool, name: &'static str) -> Leg {
    let cart = Cartridge::parse(&INVADERS).expect("parse");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(&cart);
    if let Some(dsr) = libre99_core::third_party::load("roms/Disk.Bin") {
        m.load_disk_controller(&dsr);
    }
    m.reset();
    for _ in 0..200 {
        m.run_frame();
    } // title settles, boot beep drains

    // Title -> menu.
    tap(&mut m, TiKey::Space, 3, 320);

    if warm {
        // Launch TI Invaders (entry 2), "play" a bit, then F5.
        tap(&mut m, TiKey::Num2, 6, 300);
        tap(&mut m, TiKey::Num1, 3, 120); // start / fire
        tap(&mut m, TiKey::S, 3, 60); // some movement input
        tap(&mut m, TiKey::D, 3, 60);
        m.reset(); // F5
        for _ in 0..200 {
            m.run_frame();
        }
        tap(&mut m, TiKey::Space, 3, 320); // past the title again
    }

    // At the menu. Snapshot everything the selection path depends on.
    let pad_at_menu = pad_snapshot(&m);
    let mut vdp_table = [0u8; 16];
    for (i, b) in vdp_table.iter_mut().enumerate() {
        *b = m.vdp().vram(0x3800 + i as u16);
    }
    let menu_row7 = row(&m, 7);
    let menu_row9 = row(&m, 9);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..8 {
        m.run_frame();
        seen.insert(m.bus().peek(0x8379));
    }
    let isr_moving = seen.len() > 1;
    let vdp_r1 = m.vdp().register(1);
    let int9901 = m.bus().tms9901.vdp_interrupt_enabled();
    let int_pending = m.vdp().interrupt_pending();

    let (launched, cart_fetches, hist) = press2(&mut m);
    Leg {
        name,
        launched,
        cart_fetches,
        menu_row7,
        menu_row9,
        pad_at_menu,
        vdp_table,
        isr_moving,
        vdp_r1,
        int9901,
        int_pending,
        hist,
    }
}

fn report(l: &Leg) {
    println!(
        "  {:34} launched={:5}  cart_fetches={:6}  isr_moving={:5}  R1={:02X}  9901int={:5}  vdp_pending={:5}",
        l.name, l.launched, l.cart_fetches, l.isr_moving, l.vdp_r1, l.int9901, l.int_pending
    );
    println!("      row7: {}", l.menu_row7);
    println!("      row9: {}", l.menu_row9);
}

fn main() {
    // Fail fast (with the pointer message) when the third-party media is absent.
    LazyLock::force(&CONSOLE_ROM);
    LazyLock::force(&INVADERS);
    let auth = need("roms/994AGROM.Bin");

    let ours = std::fs::read("original-content/system-roms/grom/console-grom.bin").expect("committed GROM artifact");

    println!("F5-then-press-2 probe (TI Invaders + disk DSR mounted):");
    let legs = [
        leg(&ours, false, "OURS  cold  -> press 2"),
        leg(&ours, true, "OURS  F5-from-playing -> press 2"),
        leg(&auth, false, "AUTH  cold  -> press 2"),
        leg(&auth, true, "AUTH  F5-from-playing -> press 2"),
    ];
    for l in &legs {
        report(l);
    }

    // If our warm leg failed where cold worked, print the differential detail.
    let (cold, warm) = (&legs[0], &legs[1]);
    if cold.launched && !warm.launched {
        println!("\nOUR WARM LEG FAILED — differential detail:");
        println!("  VDP menu table >3800..>380F:");
        println!("    cold: {:02X?}", cold.vdp_table);
        println!("    warm: {:02X?}", warm.vdp_table);
        println!("  scratchpad cells differing at menu-ready (cold vs warm):");
        for i in 0..0x100 {
            let (a, b) = (cold.pad_at_menu[i], warm.pad_at_menu[i]);
            if a != b {
                println!("    >{:04X}: cold={:02X} warm={:02X}", 0x8300 + i, a, b);
            }
        }
        println!("  console-GROM fetch histogram after '2' (warm), top 12:");
        let mut v: Vec<_> = warm.hist.iter().collect();
        v.sort_by(|x, y| y.1.cmp(x.1));
        for (a, n) in v.into_iter().take(12) {
            println!("    >{a:04X} x{n}");
        }
        if let (Some(lo), Some(hi)) = (
            warm.hist.keys().next(),
            warm.hist.keys().next_back(),
        ) {
            println!("  fetch address span: >{lo:04X}..>{hi:04X}");
        }
    }
}
