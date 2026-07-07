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

//! "User session" driver for the TI Invaders "text doesn't draw" bug.
//!
//! Joel: pressing 2 launches TI Invaders and reaches the opening screen, but the
//! TEXT doesn't draw right under our GROM (sprites are fine). It's correct under
//! the authentic GROM. This probe boots both, drives the same keystrokes
//! (leave title -> press 2), and at each stage dumps (a) the name table as
//! ASCII, (b) the font/pattern-table glyphs actually loaded, (c) a rendered
//! non-backdrop pixel count, and (d) the console-code call chain after launch
//! (which interconnect vector the cart CALLs and what authentic runs there).
//!
//! Run from the repo root:
//!   cargo run -p libre99-gpl --example invaders_probe

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994AGROM.Bin"));
const W: usize = 256;
const H: usize = 192;

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

/// How many distinct character CODES in the name table have a non-blank glyph in
/// the (current) pattern table. This is the key signal for "text doesn't draw":
/// if the name table holds real codes but their glyphs are all-zero, the codes
/// are invisible on screen.
fn font_report(m: &Machine) -> String {
    let name = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let pat = ((m.vdp().register(4) & 0x07) as u16) * 0x800;
    // Collect the codes actually placed on screen.
    let mut used = [false; 256];
    for i in 0..(24 * 32) {
        used[m.vdp().vram(name + i) as usize] = true;
    }
    let mut used_codes = 0;
    let mut blank_used = 0;
    for (code, &u) in used.iter().enumerate() {
        if !u {
            continue;
        }
        used_codes += 1;
        let g = pat + 8 * code as u16;
        let any = (0..8).any(|r| m.vdp().vram(g + r) != 0);
        if !any {
            blank_used += 1;
        }
    }
    // Also look specifically at the ASCII letter glyphs.
    let letters_loaded = (b'A'..=b'Z')
        .filter(|&c| (0..8).any(|r| m.vdp().vram(pat + 8 * c as u16 + r) != 0))
        .count();
    format!(
        "pat_base=>{pat:04X}  codes_on_screen={used_codes}  of_those_blank_glyph={blank_used}  ASCII_letters_with_glyph={letters_loaded}/26"
    )
}

/// Rendered non-backdrop pixel count — a display-independent proxy for "is
/// anything actually visible".
fn pixels(m: &mut Machine) -> String {
    let mut fb = vec![0u32; W * H];
    m.render(&mut fb);
    // backdrop = most common pixel
    let mut hist: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for &p in &fb {
        *hist.entry(p).or_default() += 1;
    }
    let (bg, _) = hist.iter().max_by_key(|(_, &c)| c).map(|(&v, &c)| (v, c)).unwrap();
    let non_bg = fb.iter().filter(|&&p| p != bg).count();
    format!("non_backdrop_pixels={non_bg}  distinct_colors={}", hist.len())
}

fn health(m: &Machine) -> String {
    let regs: Vec<String> = (0..8).map(|r| format!("{:02X}", m.vdp().register(r))).collect();
    format!(
        "PC={:04X}  VDP[{}]  ISR>8379={:02X} vdpint={}",
        m.cpu().pc(),
        regs.join(" "),
        m.bus().peek(0x8379),
        m.bus().tms9901.vdp_interrupt_enabled(),
    )
}

fn run(m: &mut Machine, frames: usize) {
    for _ in 0..frames {
        m.run_frame();
    }
}

fn press(m: &mut Machine, k: TiKey, settle: usize) {
    m.set_key(k, true);
    run(m, 4);
    m.set_key(k, false);
    run(m, settle);
}

fn checkpoint(label: &str, m: &mut Machine) {
    println!("\n---- {label} ----");
    println!("  {}", health(m));
    println!("  {}", font_report(m));
    println!("  {}", pixels(m));
    println!("{}", screen(m));
}

fn drive(label: &str, grom: &[u8], cart: &Cartridge) {
    println!("\n\n===================== {label} =====================");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    run(&mut m, 90);
    checkpoint("A. boot / title", &mut m);
    press(&mut m, TiKey::Space, 200);
    checkpoint("B. selection menu", &mut m);
    press(&mut m, TiKey::Num2, 300);
    checkpoint("C. TI Invaders +300f", &mut m);
    run(&mut m, 400);
    checkpoint("D. TI Invaders +700f", &mut m);
}

/// Boot, leave title, press 2, and record the GROM fetch log from the keypress
/// on. Returns the log.
fn launch_log(grom: &[u8], cart: &Cartridge, frames: usize) -> Vec<(u16, u8)> {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    run(&mut m, 90);
    press(&mut m, TiKey::Space, 200);
    m.bus_mut().grom_record(true);
    let start = m.bus().grom_log().len();
    m.set_key(TiKey::Num2, true);
    run(&mut m, 4);
    m.set_key(TiKey::Num2, false);
    run(&mut m, frames);
    m.bus().grom_log()[start..].to_vec()
}

fn decode_line(img: &[u8], a: usize) -> String {
    match libre99_gpl::decode::decode_at(img, a, a as u16) {
        Ok(d) => format!(">{a:04X}: {:<7} {:?}", d.mnemonic, d.operands),
        Err(_) => format!(">{a:04X}: <data> [{:02X}]", img[a]),
    }
}

/// Report the console-code (< >6000) branch targets the cart reaches after
/// launch, in order — the routines it CALLs. Excludes the >1700 keytab noise.
fn call_chain(label: &str, log: &[(u16, u8)], img: &[u8]) {
    println!("\n--- {label}: console-code branch targets after launch (in order) ---");
    let mut seen: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
    let mut order: Vec<u16> = Vec::new();
    let mut prev = 0u16;
    let mut counts: std::collections::BTreeMap<u16, u32> = std::collections::BTreeMap::new();
    for (a, _) in log {
        if *a < 0x6000 && !(0x1700..0x1760).contains(a) {
            *counts.entry(*a).or_default() += 1;
            let is_target = *a != prev.wrapping_add(1) && *a != prev.wrapping_add(2);
            if is_target && seen.insert(*a) {
                order.push(*a);
            }
        }
        prev = *a;
    }
    for a in order.iter().take(40) {
        let blk: u32 = counts.range(*a..(a + 0x100)).map(|(_, c)| *c).sum();
        println!("  {}   (~{blk} fetches in page)", decode_line(img, *a as usize));
    }
    // Which interconnect-table slots (>0010-0037) were touched?
    let slots: Vec<String> = counts
        .range(0x0010u16..0x0038)
        .map(|(a, c)| format!(">{a:04X}×{c}"))
        .collect();
    println!("  interconnect-table fetches: {}", if slots.is_empty() { "none".into() } else { slots.join(" ") });
}

/// Boot, leave title, press 2, run `frames`, and return the full VDP pattern
/// table (2 KiB at the pattern base) plus the name table and cell >834A.
fn launch_state(grom: &[u8], cart: &Cartridge, frames: usize) -> (Vec<u8>, Vec<u8>, u16) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    run(&mut m, 90);
    press(&mut m, TiKey::Space, 200);
    press(&mut m, TiKey::Num2, 300);
    run(&mut m, frames);
    let patbase = ((m.vdp().register(4) & 0x07) as u16) * 0x800;
    let namebase = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let pat: Vec<u8> = (0..0x800).map(|i| m.vdp().vram(patbase + i)).collect();
    let names: Vec<u8> = (0..0x300).map(|i| m.vdp().vram(namebase + i)).collect();
    let c834a = ((m.bus().peek(0x834A) as u16) << 8) | m.bus().peek(0x834B) as u16;
    (pat, names, c834a)
}

/// Which char codes appear in the name table, and — for each — is its glyph
/// non-blank in the given pattern table? Also dump the raw glyph for the
/// on-screen codes so we can compare authentic vs our font.
fn glyph_coverage() {
    let cart = Cartridge::parse(&require("cartridges/TI-Invaders.ctg")).unwrap();
    let (apat, anames, ac) = launch_state(&AUTHENTIC_GROM, &cart, 400);
    let ours = our_grom();
    let (opat, _onames, oc) = launch_state(&ours, &cart, 400);
    println!("\n\n################ GLYPH COVERAGE (authentic, stage D) ################");
    println!("cell >834A: authentic=>{ac:04X}  ours=>{oc:04X}  (VDP dest the cart set for the font load)");

    let mut used = [0u32; 256];
    for &c in &anames {
        used[c as usize] += 1;
    }
    let glyph = |pat: &[u8], code: usize| -> bool { (0..8).any(|r| pat[(8 * code + r) & 0x7FF] != 0) };

    println!("\ncode  count  authGlyph  ourGlyph   authentic-rows");
    for code in 0..256 {
        if used[code] == 0 {
            continue;
        }
        let ag = glyph(&apat, code);
        let og = glyph(&opat, code);
        let rows: Vec<String> = (0..8).map(|r| format!("{:02X}", apat[(8 * code + r) & 0x7FF])).collect();
        let ch = if (0x20..0x7F).contains(&code) { code as u8 as char } else { '.' };
        let flag = if ag && !og { "  <-- MISSING in ours" } else { "" };
        println!("  >{code:02X} '{ch}'  {:4}   {}         {}      [{}]{flag}", used[code], ag, og, rows.join(" "));
    }
    // Ranges of codes that have glyphs in authentic.
    let auth_glyphed: Vec<usize> = (0..256).filter(|&c| glyph(&apat, c)).collect();
    println!("\nauthentic pattern table has glyphs for {} codes: {:02X?}", auth_glyphed.len(), auth_glyphed);
}

fn main() {
    let cart = Cartridge::parse(&require("cartridges/TI-Invaders.ctg")).unwrap();

    drive("AUTHENTIC GROM", &AUTHENTIC_GROM, &cart);
    let ours = our_grom();
    drive("OUR GROM", &ours, &cart);
    glyph_coverage();

    println!("\n\n################ CALL CHAINS ################");
    let la = launch_log(&AUTHENTIC_GROM, &cart, 250);
    let lo = launch_log(&ours, &cart, 250);
    call_chain("AUTHENTIC", &la, &AUTHENTIC_GROM);
    call_chain("OURS", &lo, &ours);

    // The authentic interconnect table, decoded (what each slot the cart uses
    // actually does).
    println!("\n--- authentic interconnect table >0010-0037 (decoded as BR) ---");
    for slot in (0x0010..0x0038).step_by(2) {
        let w = ((AUTHENTIC_GROM[slot] as u16) << 8) | AUTHENTIC_GROM[slot + 1] as u16;
        let target = (slot as u16 & 0xE000) | (w & 0x1FFF);
        println!("  >{slot:04X}: word=>{w:04X}  BR ->>{target:04X}   {}", decode_line(&AUTHENTIC_GROM, target as usize));
    }
}
