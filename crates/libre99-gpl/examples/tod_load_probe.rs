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

//! "User session" driver for the Tunnels of Doom cassette-load bug.
//!
//! This probe *acts as a human at the keyboard*: it boots the genuine console
//! ROM in the emulator, mounts Tunnels of Doom, and drives the exact keystrokes
//! a user would — leave our title, launch ToD, let the title/music play, advance
//! to the "load a game" prompt, then press `1` (load from cassette). At every
//! step it dumps what is literally on the screen (the VDP name table read back
//! as ASCII) plus a health panel, so we can *watch* the "display goes bonkers"
//! moment and see the machine state at it.
//!
//! It runs the identical script under the **authentic** GROM and under **ours**
//! (DEBUGGING.md protocol Step 2) — if authentic goes bonkers too, the cause is
//! the emulator/expectation, not our GPL.
//!
//! Run from the repo root:
//!   cargo run -p libre99-gpl --example tod_load_probe

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
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

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

/// The 24×32 name table read back as ASCII — literally what is on the display.
/// Non-printable character codes render as `.` so corruption is visible.
fn screen(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let mut s = String::new();
    s.push_str(&format!("      +{}+\n", "-".repeat(32)));
    for row in 0..24u16 {
        let line: String = (0..32u16)
            .map(|c| {
                let b = m.vdp().vram(base + row * 32 + c);
                if (0x20..0x7F).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        s.push_str(&format!("      |{line}|\n"));
    }
    s.push_str(&format!("      +{}+", "-".repeat(32)));
    s
}

/// A crude "is this screen sane?" fingerprint of the name table: how many
/// distinct byte values it holds and the most common one. A bonkers screen is
/// often a flood of one garbage value or fully random noise.
fn fingerprint(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let mut hist = [0u32; 256];
    for i in 0..(24 * 32) {
        hist[m.vdp().vram(base + i) as usize] += 1;
    }
    let distinct = hist.iter().filter(|&&c| c > 0).count();
    let (top_val, top_cnt) = hist
        .iter()
        .enumerate()
        .max_by_key(|(_, &c)| c)
        .map(|(v, &c)| (v, c))
        .unwrap();
    format!("distinct={distinct:3} top=>{top_val:02X}×{top_cnt}")
}

/// One-line machine health panel.
fn health(m: &Machine) -> String {
    let regs: Vec<String> = (0..8).map(|r| format!("{:02X}", m.vdp().register(r))).collect();
    format!(
        "PC={:04X}  VDP[{}]  ISR>8379={:02X} vdpint={} pending={}",
        m.cpu().pc(),
        regs.join(" "),
        m.bus().peek(0x8379),
        m.bus().tms9901.vdp_interrupt_enabled(),
        m.vdp().interrupt_pending(),
    )
}

/// ISR liveness across `frames` frames: how many frames `>8379` changed.
fn run_counting_isr(m: &mut Machine, frames: usize) -> u32 {
    let mut prev = m.bus().peek(0x8379);
    let mut ticks = 0;
    for _ in 0..frames {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev {
            ticks += 1;
        }
        prev = t;
    }
    ticks
}

/// Hold `k` for 3 frames, release, then settle `frames` frames counting ISR ticks.
fn press(m: &mut Machine, k: TiKey, frames: usize) -> u32 {
    m.set_key(k, true);
    let a = run_counting_isr(m, 3);
    m.set_key(k, false);
    a + run_counting_isr(m, frames)
}

fn checkpoint(label: &str, m: &Machine) {
    println!("\n---- {label} ----");
    println!("  {}", health(m));
    println!("  screen {}", fingerprint(m));
    println!("{}", screen(m));
}

fn drive(label: &str, grom: &[u8], cart: &Cartridge) {
    println!("\n\n========================= {label} =========================");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();

    // Stage A — boot to our title.
    let t = run_counting_isr(&mut m, 90);
    println!("[boot: ISR ticked {t}/90 frames]");
    checkpoint("A. boot / our title", &m);

    // Stage B — leave the title (any key), land on the selection menu.
    let t = press(&mut m, TiKey::Space, 200);
    println!("[leave title: ISR ticked {t}/203]");
    checkpoint("B. selection menu", &m);

    // Stage C — launch program 2 (the first cartridge program = ToD).
    let t = press(&mut m, TiKey::Num2, 300);
    println!("[launch ToD: ISR ticked {t}/303]");
    checkpoint("C. ToD after launch (+300f)", &m);

    // Stage D — give the title/music more time; the load prompt may appear on
    // its own after the intro.
    let t = run_counting_isr(&mut m, 300);
    println!("[settle: ISR ticked {t}/300]");
    checkpoint("D. ToD +600f", &m);

    // Stage E — press ENTER in case the intro waits for a key to reach the
    // load-a-game prompt.
    let t = press(&mut m, TiKey::Enter, 180);
    println!("[press ENTER: ISR ticked {t}/183]");
    checkpoint("E. after ENTER", &m);

    // Stage F — THE BUG: press 1 = load from cassette.
    let t = press(&mut m, TiKey::Num1, 180);
    println!("[press 1 (cassette): ISR ticked {t}/183]");
    checkpoint("F. after press 1 (cassette)", &m);

    // Stage G — let it run further; corruption may keep spreading.
    let t = run_counting_isr(&mut m, 240);
    println!("[after +240f: ISR ticked {t}/240]");
    checkpoint("G. cassette +240f", &m);
}

/// Boot `grom`, drive to ToD's load-data prompt, then press `key` and return the
/// GROM fetch log recorded from the keypress onward (`frames` frames of it).
fn post_key_log(grom: &[u8], cart: &Cartridge, key: TiKey, frames: usize) -> Vec<(u16, u8)> {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    run_counting_isr(&mut m, 90);
    press(&mut m, TiKey::Space, 200);
    press(&mut m, TiKey::Num2, 300);
    run_counting_isr(&mut m, 300); // at "LOAD DATA FROM"
    m.bus_mut().grom_record(true);
    let start = m.bus().grom_log().len();
    m.set_key(key, true);
    for _ in 0..3 {
        m.run_frame();
    }
    m.set_key(key, false);
    for _ in 0..frames {
        m.run_frame();
    }
    m.bus().grom_log()[start..].to_vec()
}

fn decode_line(img: &[u8], a: usize) -> String {
    match libre99_gpl::decode::decode_at(img, a, a as u16) {
        Ok(d) => {
            let raw: Vec<String> = (a..(a + d.len).min(img.len()))
                .map(|o| format!("{:02X}", img[o]))
                .collect();
            format!(">{a:04X}: {:<7} {:<26} [{}]", d.mnemonic, format!("{:?}", d.operands), raw.join(" "))
        }
        Err(_) => format!(">{a:04X}: <data> [{:02X}]", img[a]),
    }
}

/// ToD's own code (>=>6000) is byte-identical under both GROMs, so its fetch
/// stream must match until a console call returns something different. Diff the
/// cart-only fetch subsequences to find that FIRST divergence — the exact call
/// that breaks under our GROM — and print the console context on each side.
fn diff_load(cart: &Cartridge, ours: &[u8], key: TiKey, keyname: &str) {
    println!("\n========== DIFF: press {keyname} — authentic vs ours ==========");
    let la = post_key_log(&AUTHENTIC_GROM, cart, key, 150);
    let lo = post_key_log(ours, cart, key, 150);

    // Cart-only subsequences, remembering each fetch's index in the full log.
    let cart: fn(&Vec<(u16, u8)>) -> Vec<(usize, u16, u8)> =
        |l| l.iter().enumerate().filter(|(_, (a, _))| *a >= 0x6000).map(|(i, (a, b))| (i, *a, *b)).collect();
    let ca = cart(&la);
    let co = cart(&lo);
    println!("  cart fetches after key: authentic={} ours={}", ca.len(), co.len());

    let n = ca.len().min(co.len());
    let mut div = None;
    for k in 0..n {
        if (ca[k].1, ca[k].2) != (co[k].1, co[k].2) {
            div = Some(k);
            break;
        }
    }
    let Some(k) = div else {
        println!("  cart streams identical for {n} fetches — divergence is later/elsewhere");
        return;
    };
    println!(
        "  ToD ran IDENTICALLY for {k} cart-fetches, then diverged:\n    authentic cart fetch #{k} = >{:04X}={:02X}\n    ours      cart fetch #{k} = >{:04X}={:02X}",
        ca[k].1, ca[k].2, co[k].1, co[k].2
    );

    // Show the full fetch stream (incl. console excursions) just before each
    // side's diverging cart fetch — the console call that returned differently.
    let dump = |label: &str, log: &[(u16, u8)], full_idx: usize| {
        println!("  --- {label}: fetch stream into the divergence ---");
        let lo = full_idx.saturating_sub(28);
        let s: Vec<String> = log[lo..(full_idx + 2).min(log.len())]
            .iter()
            .map(|(a, b)| format!("{a:04X}={b:02X}"))
            .collect();
        println!("    {}", s.join(" "));
        // The last console-GROM (<6000, and not the >1700 keytab) address touched
        // before the divergence — the routine ToD called.
        if let Some((a, _)) = log[lo..full_idx]
            .iter()
            .rev()
            .find(|(a, _)| *a < 0x6000 && !(0x1700..0x1760).contains(a))
        {
            println!("    last console-code fetch before divergence: >{a:04X}");
        }
    };
    dump("AUTHENTIC", &la, ca[k].0);
    dump("OURS", &lo, co[k].0);

    // Distinct console-code regions (excluding keytab) each side executed.
    let regions = |l: &Vec<(u16, u8)>| {
        let mut pages: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
        for (a, _) in l {
            if *a < 0x6000 && !(0x1700..0x1760).contains(a) {
                *pages.entry(*a & 0xFF00).or_default() += 1;
            }
        }
        pages
    };
    println!("  authentic console-code pages: {:?}", regions(&la));
    println!("  ours      console-code pages: {:?}", regions(&lo));
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

    drive("AUTHENTIC GROM", &AUTHENTIC_GROM, &cart);
    let ours = our_grom();
    drive("OUR GROM", &ours, &cart);

    // Confirm the exact call: first cart-fetch divergence, for all three devices.
    println!("\n\n################ CONFIRM THE EXACT CALL ################");
    diff_load(&cart, &ours, TiKey::Num1, "1 (cassette)");
    diff_load(&cart, &ours, TiKey::Num2, "2 (disk)");
    diff_load(&cart, &ours, TiKey::Num3, "3 (other)");

    // Decode what authentic runs at its console-code entry vs. what we have there.
    println!("\n--- console GROM 0 header DSR-list pointer (offset >08) ---");
    println!("  ours:      >{:02X}{:02X}", ours[0x08], ours[0x09]);
    println!("  authentic: >{:02X}{:02X}", AUTHENTIC_GROM[0x08], AUTHENTIC_GROM[0x09]);

    // The authentic call chain: in order, the console-code basic-block entries
    // (branch targets) ToD reaches after pressing 1 — the routines the fix must
    // stand in for. Excludes the >1700 keytab (KSCAN) noise.
    println!("\n--- AUTHENTIC console-code call chain after press 1 (branch targets, in order) ---");
    let la = post_key_log(&AUTHENTIC_GROM, &cart, TiKey::Num1, 150);
    let mut seen: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    let mut order: Vec<(u16, u32)> = Vec::new();
    let mut prev = 0u16;
    let mut counts: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
    for (a, _) in &la {
        if *a < 0x6000 && !(0x1700..0x1760).contains(a) {
            *counts.entry(*a).or_default() += 1;
            let is_target = *a != prev.wrapping_add(1) && *a != prev.wrapping_add(2);
            if is_target && seen.insert(*a) {
                order.push((*a, 0));
            }
        }
        prev = *a;
    }
    for (a, _) in order.iter().take(30) {
        // total fetches in the 256-byte block starting here = rough routine size
        let blk: u32 = counts.range(*a..(a + 0x100)).map(|(_, c)| *c).sum();
        println!("  >{a:04X}  {}   (~{blk} fetches in this page)", decode_line(&AUTHENTIC_GROM, *a as usize));
    }

    // Walk the authentic console GROM 0 DSR list (the devices ToD's load finds).
    println!("\n--- authentic console GROM 0 DSR list (from header offset >08) ---");
    let img: &[u8] = &AUTHENTIC_GROM;
    let mut p = ((img[0x08] as usize) << 8) | img[0x09] as usize;
    let mut guard = 0;
    while p != 0 && p + 5 < img.len() && guard < 12 {
        let next = ((img[p] as usize) << 8) | img[p + 1] as usize;
        let entry = ((img[p + 2] as usize) << 8) | img[p + 3] as usize;
        let nlen = img[p + 4] as usize;
        let name: String = (0..nlen).map(|i| img[p + 5 + i] as char).collect();
        println!("  >{p:04X}: next=>{next:04X}  dsr_entry=>{entry:04X}  name=\"{name}\"");
        p = next;
        guard += 1;
    }
}
