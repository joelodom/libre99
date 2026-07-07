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

//! **Console ROM chunk R-3 (D2) — the ROM entry census.**
//!
//! Records every address in the console ROM (`>0000-1FFF`) that
//! *machine-language* software branches into from outside the ROM — the
//! empirical public-entry set that validates the frozen-address table (P8,
//! `SURFACE-MAP.md`). Anything observed that is NOT a documented public entry
//! is a finding.
//!
//! Method: run the **authentic** ROM + GROM under `libre99_core::Machine`, single-
//! step, and log each transition of the CPU PC from an external region
//! (cart ROM `>6000-7FFF`, low/high expansion RAM, the DSR window `>4000-5FFF`)
//! *into* the ROM. Pure GPL never appears here — the GPL interpreter always runs
//! inside the ROM, so a PC that is external and then interior is a genuine
//! ML→ROM call, an interrupt vectoring, or a DSR/XML return. GROM-only carts
//! that run purely as GPL bytecode contribute only interrupts; ML carts and the
//! device (disk DSR) path contribute the real entries.
//!
//! This is a **representative baseline** over a few rich carts + the disk path.
//! The authoritative full-137-corpus run reuses the GROM track's
//! `coverage_sweep` launch driver and is deferred (see the archived
//! `system-roms/history/ROM-ENTRY-CENSUS.md`);
//! post-P9 the census orders work and validates P8, it does not gate what we
//! build. Run: `cargo run -p libre99-asm --example rom_entry_census` (repo root).

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

static AUTH_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| need("roms/994aROM.Bin"));
static AUTH_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| need("roms/994AGROM.Bin"));

/// Which external region a PC belongs to (None = inside the ROM, `>0000-1FFF`).
fn region(pc: u16) -> Option<&'static str> {
    match pc {
        0x0000..=0x1FFF => None,
        0x2000..=0x3FFF => Some("low-RAM >2000"),
        0x4000..=0x5FFF => Some("DSR/card >4000"),
        0x6000..=0x7FFF => Some("cart-ROM >6000"),
        0xA000..=0xFFFF => Some("high-RAM >A000"),
        _ => Some("mapped >8000"),
    }
}

/// A recorded ROM entry: how many times, and from which external regions.
#[derive(Default)]
struct Hits {
    count: u64,
    from: std::collections::BTreeSet<&'static str>,
}

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

/// Single-step `budget` instructions, mashing a little input, recording every
/// external→ROM PC transition into `census`.
fn census_steps(m: &mut Machine, budget: u64, census: &mut BTreeMap<u16, Hits>) {
    let keys = [TiKey::Num1, TiKey::Space, TiKey::S, TiKey::D, TiKey::E, TiKey::Enter];
    let mut prev = m.cpu().pc();
    for i in 0..budget {
        // Light scripted input so KSCAN / key paths are exercised.
        if i % 20_000 == 0 {
            let k = keys[(i / 20_000) as usize % keys.len()];
            m.set_key(k, true);
        } else if i % 20_000 == 4_000 {
            for k in keys {
                m.set_key(k, false);
            }
        }
        m.step();
        let pc = m.cpu().pc();
        if region(prev).is_some() && region(pc).is_none() {
            let h = census.entry(pc).or_default();
            h.count += 1;
            if let Some(r) = region(prev) {
                h.from.insert(r);
            }
        }
        prev = pc;
    }
}

/// Boot authentic firmware, mount `cart` (+ optional disk), reach + launch the
/// menu's entry `digit`, then census `budget` steps of the running cart.
fn run_cart(name: &str, cart_path: &str, digit: TiKey, disk: Option<&str>, budget: u64) -> BTreeMap<u16, Hits> {
    let mut census = BTreeMap::new();
    let bytes = match libre99_core::third_party::load(cart_path) {
        Some(b) => b,
        None => {
            eprintln!("  [{name}] skip: third-party media not present (third-party/{cart_path})");
            return census;
        }
    };
    let cart = Cartridge::parse(&bytes).expect("parse cart");
    let mut m = Machine::new(&AUTH_ROM, &AUTH_GROM);
    m.mount_cartridge(&cart);
    if let Some(d) = disk {
        if let Some(dsr) = libre99_core::third_party::load("roms/Disk.Bin") {
            m.load_disk_controller(&dsr);
        }
        let _ = d; // the disk image is selected by the cart's own load prompt
    }
    m.reset();
    for _ in 0..200 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 3, 320); // title -> menu
    tap(&mut m, digit, 6, 150); // launch
    census_steps(&mut m, budget, &mut census);
    census
}

fn main() {
    // Fail fast (with the pointer message) when the third-party media is absent.
    LazyLock::force(&AUTH_ROM);
    LazyLock::force(&AUTH_GROM);

    println!("ROM entry census — authentic ROM + GROM, representative baseline\n");

    let scenarios: Vec<(&str, BTreeMap<u16, Hits>)> = vec![
        // Centipede: a ROM-only ML cart (its own machine code at >6000) — the
        // case that calls the ROM's public entries directly.
        ("centipede (ML)", run_cart("centipede", "cartridges/centipe.ctg", TiKey::Num2, None, 3_000_000)),
        // TI Invaders: exercises KSCAN + char-set + sound during gameplay.
        ("ti-invaders", run_cart("ti-invaders", "cartridges/TI-Invaders.ctg", TiKey::Num2, None, 3_000_000)),
        // Tunnels of Doom + disk controller: the device-I/O (DSRLNK/XML >19/1A +
        // the >4000 disk DSR) path.
        ("tunnels+disk", run_cart("tunnels", "cartridges/tunnelsofdoom.ctg", TiKey::Num2, Some("disks/Tunnels.Dsk"), 3_000_000)),
    ];

    // Merge into one table.
    let mut all: BTreeMap<u16, Hits> = BTreeMap::new();
    for (_, c) in &scenarios {
        for (pc, h) in c {
            let e = all.entry(*pc).or_default();
            e.count += h.count;
            for r in &h.from {
                e.from.insert(r);
            }
        }
    }

    // The documented public entries (SURFACE-MAP frozen-address table) — an entry
    // outside this set is a finding worth investigating.
    let known: &[(u16, &str)] = &[
        (0x0000, "reset vector"),
        (0x0004, "L1 interrupt vector"),
        (0x0008, "L2 vector"),
        (0x000E, "KSCAN"),
        (0x0016, "interp entry (R9)"),
        (0x001C, "interp entry (fetch)"),
        (0x0020, "CLEAR test"),
        (0x0024, "reset/EXIT"),
        (0x006A, "interp soft entry"),
        (0x0070, "interp main loop"),
        (0x0900, "VBLANK ISR"),
    ];
    let is_known = |pc: u16| known.iter().any(|(a, _)| *a == pc);

    for (label, c) in &scenarios {
        println!("== {label}: {} distinct ROM entries ==", c.len());
        let mut v: Vec<_> = c.iter().collect();
        v.sort_by_key(|(_, h)| std::cmp::Reverse(h.count));
        for (pc, h) in v.into_iter().take(12) {
            let tag = known.iter().find(|(a, _)| a == pc).map(|(_, n)| *n).unwrap_or("? UNLISTED");
            let from: Vec<&str> = h.from.iter().copied().collect();
            println!("   >{pc:04X}  x{:<8} {tag:22} from {from:?}", h.count);
        }
        println!();
    }

    println!("== merged: entries NOT in the documented public set (findings) ==");
    let mut findings = 0;
    for (pc, h) in &all {
        if !is_known(*pc) {
            let from: Vec<&str> = h.from.iter().copied().collect();
            println!("   >{pc:04X}  x{:<8} from {from:?}", h.count);
            findings += 1;
        }
    }
    if findings == 0 {
        println!("   (none — every observed ROM entry is a documented public address; P8 validated on this sample)");
    }
    println!("\ntotal distinct ROM entries across scenarios: {}", all.len());
}
