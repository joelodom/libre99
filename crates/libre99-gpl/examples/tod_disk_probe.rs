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

//! Disk-load investigation: does our rewritten console GROM let Tunnels of Doom
//! load a QUEST scenario from `Tunnels.Dsk` (DSK1) the way the authentic GROM
//! does (`crates/libre99-core/tests/disk.rs` gate d)? Drives the full user path —
//! title → select ToD → LOAD DATA FROM → 2 (DISK 1) → type "QUEST" → ENTER — and
//! reports the screen + the disk sector-read log under authentic vs. ours.
//!
//! Run from the repo root: `cargo run -p libre99-gpl --example tod_disk_probe`

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

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

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

fn health(m: &Machine) -> String {
    format!(
        "PC={:04X} R1={:02X} ISR>8379={:02X} vdpint={} pending={}",
        m.cpu().pc(),
        m.vdp().register(1),
        m.bus().peek(0x8379),
        m.bus().tms9901.vdp_interrupt_enabled(),
        m.vdp().interrupt_pending(),
    )
}

/// Hold `k` for 6 frames, release, settle `settle` frames counting ISR ticks.
fn tap(m: &mut Machine, k: TiKey, settle: usize) -> u32 {
    m.set_key(k, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(k, false);
    let mut prev = m.bus().peek(0x8379);
    let mut ticks = 0;
    for _ in 0..settle {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev {
            ticks += 1;
        }
        prev = t;
    }
    ticks
}

fn checkpoint(label: &str, m: &Machine) {
    println!("\n---- {label} ----");
    println!("  {}", health(m));
    println!("{}", screen(m));
}

fn run(label: &str, grom: &[u8], cart: &Cartridge) {
    println!("\n\n========================= {label} =========================");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.load_disk_controller(&DSR);
    m.mount_disk(0, TUNNELS.to_vec());
    m.mount_cartridge(cart);
    m.reset();
    m.bus_mut().disk.record(true);

    for _ in 0..180 {
        m.run_frame();
    }
    let t = tap(&mut m, TiKey::Space, 40); // title → selection list
    println!("[Space: {t} ticks]");
    let t = tap(&mut m, TiKey::Num2, 240); // select Tunnels of Doom
    println!("[Num2: {t} ticks]");
    let t = tap(&mut m, TiKey::Enter, 120); // → "LOAD DATA FROM"
    println!("[Enter: {t} ticks]");
    checkpoint("A. LOAD DATA FROM prompt", &m);

    let t = tap(&mut m, TiKey::Num2, 120); // 2 = DISK 1 → filename prompt
    println!("[Num2 (DISK 1): {t} ticks]");
    checkpoint("B. after selecting DISK 1 (filename prompt?)", &m);

    for k in [TiKey::Q, TiKey::U, TiKey::E, TiKey::S, TiKey::T] {
        tap(&mut m, k, 10);
    }
    checkpoint("C. after typing QUEST", &m);

    let t = tap(&mut m, TiKey::Enter, 600); // submit → load the scenario
    println!("[Enter (submit): {t} ticks over 606 frames]");
    checkpoint("D. after submit (load attempt)", &m);

    let read = m.bus().disk.read_log();
    println!("  disk sectors read: {} total", read.len());
    println!("  read log: {:?}", read);
    // Disk activity: CRU bits toggled (drive select/ROM enable) + FD1771 commands.
    let tr = m.bus().disk.trace();
    let mut cru: std::collections::BTreeMap<u16, u8> = std::collections::BTreeMap::new();
    let mut cmds: Vec<u8> = Vec::new();
    for (kind, a, v) in tr {
        match *kind {
            b'C' => {
                cru.insert(*a, *v);
            }
            b'W' if *a == 0x5FF8 => cmds.push(*v),
            _ => {}
        }
    }
    println!("  disk trace: {} accesses; CRU latches={:?}; FD1771 cmds(first 12)={:02X?}",
        tr.len(), cru, &cmds[..cmds.len().min(12)]);
    let reached_menu = screen(&m).contains("NEW DUNGEON");
    println!(
        "  RESULT: read QUEST data (85 & 135)? {} ; reached NEW DUNGEON? {}",
        read.contains(&85) && read.contains(&135),
        reached_menu
    );
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

/// Drive to the QUEST filename prompt, then press ENTER (submit) with the GROM
/// fetch log + disk trace on, and print the console-GROM routines ToD runs to
/// open/read the file — the DSRLNK service the fix must provide.
fn trace_submit(label: &str, grom: &[u8], img: &[u8], cart: &Cartridge) {
    println!("\n========== TRACE submit under {label} ==========");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.load_disk_controller(&DSR);
    m.mount_disk(0, TUNNELS.to_vec());
    m.mount_cartridge(cart);
    m.reset();
    for _ in 0..180 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 40);
    tap(&mut m, TiKey::Num2, 240);
    tap(&mut m, TiKey::Enter, 120);
    tap(&mut m, TiKey::Num2, 120);
    for k in [TiKey::Q, TiKey::U, TiKey::E, TiKey::S, TiKey::T] {
        tap(&mut m, k, 10);
    }
    // At the filename prompt with QUEST typed. Dump the PAB the caller built.
    let pab_name = m.bus().peek_word(0x8356);
    let vbyte = |m: &Machine, a: u16| m.vdp().vram(a);
    let pab: String = (0..24)
        .map(|i| {
            let b = vbyte(&m, pab_name.wrapping_sub(9).wrapping_add(i));
            if (0x20..0x7F).contains(&b) { b as char } else { '.' }
        })
        .collect();
    println!("  >8356 (PAB name ptr) = >{pab_name:04X}; PAB[-9..+15] = \"{pab}\"");

    // Wholesale scratchpad snapshot going into the DSR (diff authentic vs ours).
    for row in 0..16u16 {
        let base = 0x8300 + row * 16;
        let hex: Vec<String> = (0..16).map(|c| format!("{:02X}", m.bus().peek(base + c))).collect();
        println!("  SP {:04X}: {}", base, hex.join(" "));
    }

    // Record the submit.
    m.bus_mut().grom_record(true);
    m.bus_mut().disk.record(true);
    let start = m.bus().grom_log().len();
    tap(&mut m, TiKey::Enter, 150);
    // Which interconnect-table slots (>0010-0037) did the run execute?
    let mut slots: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
    for (a, _) in &m.bus().grom_log()[start..] {
        if (0x0010..0x0038).contains(a) {
            *slots.entry(*a & 0xFFFE).or_default() += 1;
        }
    }
    println!("  interconnect slots executed: {slots:?}");

    let log = m.bus().grom_log()[start..].to_vec();
    // First cart(>=6000) -> console(<6000, not keytab) transfer = the call site.
    let mut prev_cart = false;
    let mut call_site = None;
    for w in log.windows(1).enumerate() {
        let (i, s) = w;
        let (a, _) = s[0];
        let console = a < 0x6000 && !(0x1700..0x1760).contains(&a);
        if console && prev_cart {
            call_site = Some(i);
            break;
        }
        if a >= 0x6000 {
            prev_cart = true;
        } else if console {
            prev_cart = false;
        }
    }
    if let Some(i) = call_site {
        println!("  first cart->console call: cart >{:04X} --> console >{:04X}", log[i - 1].0, log[i].0);
    }
    // Ordered console-code branch targets (the routines run), with fetch counts.
    let mut seen: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    let mut order: Vec<u16> = Vec::new();
    let mut counts: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
    let mut prev = 0u16;
    for (a, _) in &log {
        if *a < 0x6000 && !(0x1700..0x1760).contains(a) {
            *counts.entry(*a).or_default() += 1;
            if *a != prev.wrapping_add(1) && *a != prev.wrapping_add(2) && seen.insert(*a) {
                order.push(*a);
            }
        }
        prev = *a;
    }
    println!("  console-code branch targets (in order):");
    for a in order.iter().take(40) {
        let blk: u32 = counts.range(*a..(a + 0x100)).map(|(_, c)| *c).sum();
        println!("    {}   (~{blk}/page)", decode_line(img, *a as usize));
    }
    println!("  disk sectors read during submit: {:?}", m.bus().disk.read_log());
}

/// Experiment: boot ours, drive to the QUEST prompt, poke the SYS cells the
/// authentic boot/DSR-power-up sets that ours leaves 0, then submit — does the
/// disk read now?  Isolates whether the peripheral power-up (>8370 top-of-VRAM)
/// is the remaining blocker.
fn experiment(cart: &Cartridge, grom: &[u8]) {
    println!("\n\n################ EXPERIMENT: poke >8370 then load ################");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.load_disk_controller(&DSR);
    m.mount_disk(0, TUNNELS.to_vec());
    m.mount_cartridge(cart);
    m.reset();
    for _ in 0..180 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 40);
    tap(&mut m, TiKey::Num2, 240);
    tap(&mut m, TiKey::Enter, 120);
    tap(&mut m, TiKey::Num2, 120);
    for k in [TiKey::Q, TiKey::U, TiKey::E, TiKey::S, TiKey::T] {
        tap(&mut m, k, 10);
    }
    // Poke the top-of-free-VRAM pointer to a reserved-buffer value like authentic.
    m.bus_mut().poke_word(0x8370, 0x37D7);
    m.bus_mut().disk.record(true);
    let t = tap(&mut m, TiKey::Enter, 600);
    let read = m.bus().disk.read_log();
    println!("  ISR ticks: {t}/606; disk sectors read: {} = {:?}", read.len(), read);
    println!("  reached NEW DUNGEON? {}", screen(&m).contains("NEW DUNGEON"));
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

    run("AUTHENTIC GROM", &AUTHENTIC_GROM, &cart);
    let ours = our_grom();
    run("OUR GROM", &ours, &cart);
    experiment(&cart, &ours);

    println!("\n\n################ CONFIRM THE SUBMIT CALL ################");
    trace_submit("AUTHENTIC GROM", &AUTHENTIC_GROM, &AUTHENTIC_GROM, &cart);
    trace_submit("OUR GROM", &ours, &ours, &cart);

    // Decode the authentic DSRLNK dispatch + routine to gauge the fix scope.
    // Looking for: XML/CALL into the console ROM (thin wrapper) vs. all-GPL CRU
    // scanning (large). XML is GPL opcode >0F.
    let dump = |from: usize, to: usize| {
        let mut a = from;
        while a < to {
            let line = decode_line(&AUTHENTIC_GROM, a);
            println!("    {line}");
            match libre99_gpl::decode::decode_at(&AUTHENTIC_GROM, a, a as u16) {
                Ok(d) if d.len > 0 => a += d.len,
                _ => a += 1,
            }
        }
    };
    println!("\n--- authentic interconnect table >0010..>0038 (executed as BR stubs) ---");
    dump(0x0010, 0x0038);
    println!("\n--- authentic DSRLNK routine >03DC..>0468 (the wrapper to reimplement) ---");
    dump(0x03DC, 0x0468);
    // Count XML opcodes (>0F) across GROM 0 low code as a delegation signal.
    let xml = (0x0060..0x0800).filter(|&i| AUTHENTIC_GROM[i] == 0x0F).count();
    println!("\n  (>0F bytes in >0060..0800: {xml} — potential XML-to-ROM delegations)");

    println!("\n--- OUR DSRLNK routine >1201..>1250 (vs authentic >03DC) ---");
    let mut a = 0x1201usize;
    while a < 0x1250 {
        println!("    {}", decode_line(&ours, a));
        match libre99_gpl::decode::decode_at(&ours, a, a as u16) {
            Ok(d) if d.len > 0 => a += d.len,
            _ => a += 1,
        }
    }
}
