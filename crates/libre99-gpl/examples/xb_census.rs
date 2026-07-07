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

//! **F0 — the Extended BASIC console-call census** (docs/TI-PYTHON.md §6.5).
//!
//! Launch an Extended BASIC cartridge, drive a scripted interactive session
//! (`PRINT`, float variables, a stored program, `RUN`, `LIST`), and record —
//! from the launch keypress onward —
//!
//! * every **console-GROM address fetched** (`>0000–>5FFF`, the GROM
//!   read-coverage bitmap), and
//! * every **console-ROM PC executed** (`>0000–>1FFF`, the CPU PC-coverage
//!   bitmap),
//!
//! then print both as compact runs, with the `>0010–005F` interconnect/GPLLNK
//! slot fetches and the known ROM regions called out. Under the **authentic**
//! firmware this enumerates the exact console surface XB depends on — the
//! contract the clean-room rewrite must provide (LIMITATIONS.md L9). Run it
//! against **our** firmware (`ours`) to watch the same script diverge and to
//! see which of the required addresses we serve as zeros/stubs.
//!
//! ```sh
//! cargo run -q -p libre99-gpl --example xb_census                 # authentic pair
//! cargo run -q -p libre99-gpl --example xb_census -- ours        # our ROM + our GROM
//! cargo run -q -p libre99-gpl --example xb_census -- ours-grom   # authentic ROM + our GROM
//! cargo run -q -p libre99-gpl --example xb_census -- ours-rom    # our ROM + authentic GROM
//! cargo run -q -p libre99-gpl --example xb_census -- authentic cartridges/sxba.ctg
//! ```
//!
//! Needs third-party media for the authentic images and the XB `.ctg`
//! (default `cartridges/xb25.ctg`). Exits 2 with a notice when absent.

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

fn need(path: &str) -> Vec<u8> {
    libre99_core::third_party::load(path).unwrap_or_else(|| {
        eprintln!("this census needs third-party media (third-party/{path})");
        std::process::exit(2)
    })
}

/// ASCII → (modifiers, key). Letters are typed **shifted** so they arrive as
/// uppercase under both the authentic keytab (unshifted = lowercase) and ours.
fn key_for(c: char) -> (Vec<TiKey>, TiKey) {
    use TiKey::*;
    let shift = |k: TiKey| (vec![Shift], k);
    let fctn = |k: TiKey| (vec![Fctn], k);
    let bare = |k: TiKey| (Vec::new(), k);
    match c {
        'A'..='Z' => shift(letter(c)),
        'a'..='z' => bare(letter(c.to_ascii_uppercase())),
        '0' => bare(Num0), '1' => bare(Num1), '2' => bare(Num2), '3' => bare(Num3),
        '4' => bare(Num4), '5' => bare(Num5), '6' => bare(Num6), '7' => bare(Num7),
        '8' => bare(Num8), '9' => bare(Num9),
        ' ' => bare(Space), '\n' => bare(Enter),
        '=' => bare(Equals), '.' => bare(Period), ',' => bare(Comma),
        ';' => bare(Semicolon), '/' => bare(Slash),
        '+' => shift(Equals), '-' => shift(Slash), '*' => shift(Num8),
        '(' => shift(Num9), ')' => shift(Num0), '^' => shift(Num6),
        '<' => shift(Comma), '>' => shift(Period), ':' => shift(Semicolon),
        '!' => shift(Num1), '@' => shift(Num2), '#' => shift(Num3),
        '$' => shift(Num4), '%' => shift(Num5), '&' => shift(Num7),
        '"' => fctn(P), '\'' => fctn(O), '_' => fctn(U), '?' => fctn(I),
        other => panic!("no TI keystroke mapped for {other:?}"),
    }
}

fn letter(c: char) -> TiKey {
    use TiKey::*;
    match c {
        'A' => A, 'B' => B, 'C' => C, 'D' => D, 'E' => E, 'F' => F, 'G' => G,
        'H' => H, 'I' => I, 'J' => J, 'K' => K, 'L' => L, 'M' => M, 'N' => N,
        'O' => O, 'P' => P, 'Q' => Q, 'R' => R, 'S' => S, 'T' => T, 'U' => U,
        'V' => V, 'W' => W, 'X' => X, 'Y' => Y, 'Z' => Z,
        _ => unreachable!(),
    }
}

fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}

fn press(m: &mut Machine, mods: &[TiKey], k: TiKey, hold: usize, settle: usize) {
    for &mo in mods {
        m.set_key(mo, true);
    }
    m.set_key(k, true);
    frames(m, hold);
    m.set_key(k, false);
    for &mo in mods {
        m.set_key(mo, false);
    }
    frames(m, settle);
}

/// Type a line and press ENTER, then let the interpreter chew (`settle`).
fn type_line(m: &mut Machine, line: &str, settle: usize) {
    for c in line.chars() {
        let (mods, k) = key_for(c);
        press(m, &mods, k, 3, 3);
    }
    press(m, &[], TiKey::Enter, 3, settle);
}

/// One screen row, decoded. TI BASIC-family screens store ASCII **biased by
/// `+>60`** (space = `>80`); decode the biased band back to ASCII and show
/// anything else as-is (menu/title screens are unbiased).
fn row(m: &Machine, r: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..32)
        .map(|i| {
            let b = m.vdp().vram(base + r * 32 + i);
            let c = if (0x80..=0xDF).contains(&b) { b - 0x60 } else { b };
            if (0x20..0x7F).contains(&c) { c as char } else { '·' }
        })
        .collect::<String>()
        .trim_end_matches([' ', '·'])
        .to_string()
}

fn dump(m: &Machine, label: &str) {
    println!("--- screen: {label}");
    for r in 0..24 {
        let t = row(m, r);
        if !t.is_empty() {
            println!("{r:2} |{t}");
        }
    }
}

/// Fold sorted addresses into inclusive runs.
fn runs(addrs: &[u16]) -> Vec<(u16, u16)> {
    let mut out: Vec<(u16, u16)> = Vec::new();
    for &a in addrs {
        match out.last_mut() {
            Some(last) if a == last.1 + 1 || a == last.1 => last.1 = a,
            _ => out.push((a, a)),
        }
    }
    out
}

/// Fold sorted word-aligned addresses into runs (stride 2).
fn word_runs(addrs: &[u16]) -> Vec<(u16, u16)> {
    let mut out: Vec<(u16, u16)> = Vec::new();
    for &a in addrs {
        match out.last_mut() {
            Some(last) if a == last.1 + 2 || a == last.1 => last.1 = a,
            _ => out.push((a, a)),
        }
    }
    out
}

fn print_runs(title: &str, rs: &[(u16, u16)]) {
    println!("--- {title} ({} runs)", rs.len());
    for &(s, e) in rs {
        if s == e {
            println!(">{s:04X}");
        } else {
            println!(">{s:04X}-{e:04X}  ({} bytes)", (e - s) as u32 + 1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let firmware = args.first().map(String::as_str).unwrap_or("authentic");
    let cart_path = args.get(1).map(String::as_str).unwrap_or("cartridges/xb25.ctg");

    let (console_rom, grom) = match firmware {
        "authentic" => (need("roms/994aROM.Bin"), need("roms/994AGROM.Bin")),
        "ours" => (
            libre99_asm::system_rom::build_console_rom().unwrap(),
            libre99_gpl::system_grom::build_console_grom().unwrap(),
        ),
        "ours-grom" => (
            need("roms/994aROM.Bin"),
            libre99_gpl::system_grom::build_console_grom().unwrap(),
        ),
        "ours-rom" => (
            libre99_asm::system_rom::build_console_rom().unwrap(),
            need("roms/994AGROM.Bin"),
        ),
        other => {
            eprintln!("unknown firmware {other:?} (use: authentic | ours | ours-grom | ours-rom)");
            std::process::exit(2)
        }
    };
    let cart_bytes = need(cart_path);
    let cart = Cartridge::parse(&cart_bytes).expect("cartridge parses");

    let mut m = Machine::new(&console_rom, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    frames(&mut m, 40);
    press(&mut m, &[], TiKey::Space, 3, 300); // title -> selection menu
    dump(&m, "selection menu");

    // Find the cartridge's menu entry (the row naming EXTENDED/BASIC), else 2.
    let mut entry = '2';
    for r in 0..24 {
        let t = row(&m, r);
        if t.contains("EXTENDED") || t.contains("XB") {
            if let Some(d) = t.trim_start().chars().next().filter(char::is_ascii_digit) {
                entry = d;
                break;
            }
        }
    }
    println!("--- launching menu entry {entry} from {cart_path} under {firmware} firmware");

    // Record everything from the launch keypress onward.
    m.bus_mut().grom_record_coverage(true);
    m.record_pc_coverage(true);
    let (mods, k) = key_for(entry);
    press(&mut m, &mods, k, 6, 500);
    dump(&m, "after launch (expect the XB banner / READY)");

    // The scripted session — immediate PRINT, floats, a stored program, RUN,
    // LIST. Settle frames are generous: crunch + execute + GC are slow.
    for (line, settle, label) in [
        ("PRINT \"HELLO\"", 150, "PRINT \"HELLO\""),
        ("X=1.5", 150, "X=1.5"),
        ("PRINT X*2", 150, "PRINT X*2"),
        ("10 PRINT \"HI\"", 120, "10 PRINT \"HI\""),
        ("20 END", 120, "20 END"),
        ("RUN", 250, "RUN"),
        ("LIST", 200, "LIST"),
    ] {
        type_line(&mut m, line, settle);
        dump(&m, label);
    }

    // ---- the census ---------------------------------------------------------
    let grom_reads: Vec<u16> = m
        .bus()
        .grom_coverage_addresses()
        .into_iter()
        .filter(|&a| a < 0x6000)
        .collect();
    let (g0, rest): (Vec<u16>, Vec<u16>) = grom_reads.iter().partition(|&&a| a < 0x2000);
    let (g1, g2): (Vec<u16>, Vec<u16>) = rest.iter().partition(|&&a| a < 0x4000);

    println!();
    println!("================= XB CONSOLE-CALL CENSUS ({firmware}) =================");
    let slots: Vec<u16> = g0.iter().copied().filter(|&a| (0x0010..=0x005F).contains(&a)).collect();
    print_runs("interconnect/GPLLNK slot fetches (>0010-005F)", &runs(&slots));
    print_runs("console GROM 0 reads (>0000-1FFF)", &runs(&g0));
    print_runs("console GROM 1 reads (>2000-3FFF)", &runs(&g1));
    print_runs("console GROM 2 reads (>4000-5FFF)", &runs(&g2));

    let pcs: Vec<u16> = m.pc_coverage_addresses().into_iter().filter(|&a| a < 0x2000).collect();
    print_runs("console ROM PCs executed (>0000-1FFF)", &word_runs(&pcs));
    let in_range = |lo: u16, hi: u16| pcs.iter().any(|&a| (lo..=hi).contains(&a));
    println!("--- known-region summary");
    println!("FP package  >0D3A-11A1 executed: {}", in_range(0x0D3A, 0x11A1));
    println!("FLTAB       >0D1A-0D39 (data — see GROM/PC runs above)");
    println!("BASIC XMLs  >15D6-18C7 executed: {}", in_range(0x15D6, 0x18C7));
    let cart_pc: Vec<u16> =
        m.pc_coverage_addresses().into_iter().filter(|&a| (0x6000..0x8000).contains(&a)).collect();
    println!("cartridge ROM (>6000-7FFF) executed: {}", !cart_pc.is_empty());
    println!("illegal opcodes executed: {}", m.cpu().illegal_count());
}
