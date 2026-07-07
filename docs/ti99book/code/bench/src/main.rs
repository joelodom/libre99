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

//! BENCH99 — the book's lab bench: the emulator core on your desk, scriptable.
//!
//! The desktop emulator (`libre99`) is the machine you *use*; BENCH99 is
//! the machine you *interrogate*. It drives the same `libre99-core` crate the
//! desktop app runs, but from a line-oriented monitor, so a lab can set the
//! program counter, single-step instructions, read the workspace out of RAM,
//! and count cycles — interactively at a prompt, or reproducibly from a piped
//! script (which is how the book's own claims are machine-verified).
//!
//! ```text
//! bench99 [script...]      run command files, then read stdin (if a tty, prompt)
//!
//! Commands (one per line; '#' starts a comment; hex is TI-style, '>' optional):
//!   load <file.bin>        mount a raw 8 KiB ROM image as the cartridge at >6000
//!   boot [file.ctg|.bin]   full console boot (real firmware); optional cartridge
//!   pc <hex> | wp <hex>    set the CPU's PC / WP            (bare bench only)
//!   s [n]                  step n instructions (default 1), tracing each
//!   x [n]                  execute n instructions silently (default 1000)
//!   u <hex>                run until PC reaches the address (a breakpoint)
//!   f [n]                  run n whole frames (boot mode; default 1)
//!   k <key>                tap a TI key for a few frames: A–Z, 0–9, SPACE, ENTER, .
//!   press/rel <name>       hold/release a key or joystick switch (J1U/J1F/..) on this bench
//!   r                      registers: PC, WP, ST decoded, R0–R15 (read from RAM)
//!   m <hex> [n]            dump n bytes of memory (side-effect-free peek)
//!   pw <hex> <hex>         poke a word    (RAM only, no MMIO side effects)
//!   pb <hex> <hex>...      poke bytes     (RAM only)
//!   screen                 ASCII view of the VDP name table (32- or 40-wide by mode)
//!   vdp                    VDP write-registers R0–R7
//!   vram <hex> [n]         dump n bytes of the VDP's private VRAM (independent oracle)
//!   pixels [step]          render the picture: palette-index hex per sampled pixel
//!   sound                  SN76489 PSG state: per-channel period/frequency/attenuation
//!   gromlog [on|off]       trace GROM fetches (the GPL interpreter's instruction stream)
//!   cycles                 total elapsed CPU cycles
//!   q                      quit
//! ```
//!
//! Two modes. The **bare bench** (default) is a CPU wired to the console bus
//! with the firmware in ROM but nothing running: PC and WP are yours, nothing
//! else executes, and pokes/peeks touch RAM without device side effects — a
//! paper machine made of silicon. `boot` replaces it with the **full machine**
//! (the real firmware booting to the master title screen), for end-to-end runs.

use std::io::{BufRead, IsTerminal, Write};
use std::sync::LazyLock;

use libre99_asm::disasm;
use libre99_core::cartridge::Cartridge;
use libre99_core::cpu::Cpu;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::{Machine, Tms9900Bus};
use libre99_core::vdp::{HEIGHT, PALETTE, WIDTH};

/// The authentic console firmware, loaded at startup — never embedded. The
/// images live outside version control; the bench refuses to start without them.
static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| firmware("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| firmware("roms/994AGROM.Bin"));

fn firmware(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("bench99: the authentic TI console firmware is required — put {rel} under third-party/ at the repo root (or $LIBRE99_THIRD_PARTY)");
        std::process::exit(2);
    })
}

/// The bench is either bare (CPU + bus, nothing running) or a booted machine.
/// `cart` shadows bank 0 of the mounted cartridge ROM: the bus's side-effect-free
/// `peek` does not cover `>6000–7FFF`, so the bench keeps its own copy for the
/// trace disassembly and `m` dumps (the CPU itself reads the real thing).
enum Bench {
    Bare { cpu: Cpu, bus: Tms9900Bus, cart: Vec<u8> },
    Boot { m: Box<Machine>, cart: Vec<u8> },
}

fn main() {
    let mut bench = new_bare();
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Command files first, then stdin.
    for path in &args {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                for line in text.lines() {
                    if !run_line(&mut bench, line) {
                        return;
                    }
                }
            }
            Err(e) => {
                eprintln!("bench99: cannot read {path}: {e}");
                std::process::exit(3);
            }
        }
    }

    let stdin = std::io::stdin();
    let interactive = stdin.is_terminal();
    if interactive {
        println!("BENCH99 — bare bench ready (set `pc`, `s` to step; `boot` for the full machine; `q` quits)");
    }
    loop {
        if interactive {
            print!("bench> ");
            let _ = std::io::stdout().flush();
        }
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => return, // EOF
            Ok(_) => {
                if !run_line(&mut bench, &line) {
                    return;
                }
            }
            Err(_) => return,
        }
    }
}

fn new_bare() -> Bench {
    Bench::Bare {
        cpu: Cpu::new(),
        bus: Tms9900Bus::new(&CONSOLE_ROM, &CONSOLE_GROM),
        cart: Vec::new(),
    }
}

/// Execute one command line. Returns `false` on `q`.
fn run_line(bench: &mut Bench, raw: &str) -> bool {
    let line = raw.split('#').next().unwrap_or("").trim();
    if line.is_empty() {
        return true;
    }
    let mut parts = line.split_whitespace();
    let cmd = parts.next().unwrap();
    let rest: Vec<&str> = parts.collect();
    match cmd {
        "q" | "quit" => return false,
        "load" => cmd_load(bench, &rest),
        "boot" => cmd_boot(bench, &rest),
        "pc" => cmd_set_pcwp(bench, &rest, true),
        "wp" => cmd_set_pcwp(bench, &rest, false),
        "s" | "step" => cmd_step(bench, &rest),
        "x" | "run" => cmd_exec(bench, &rest),
        "u" | "until" => cmd_until(bench, &rest),
        "f" | "frames" => cmd_frames(bench, &rest),
        "k" | "key" => cmd_key(bench, &rest),
        "press" => cmd_press(bench, &rest, true),
        "rel" | "release" => cmd_press(bench, &rest, false),
        "r" | "regs" => print_regs(bench),
        "m" | "mem" => cmd_mem(bench, &rest),
        "pw" => cmd_poke(bench, &rest, true),
        "pb" => cmd_poke(bench, &rest, false),
        "screen" => cmd_screen(bench),
        "vdp" => cmd_vdp(bench),
        "vram" | "vr" => cmd_vram(bench, &rest),
        "pixels" | "px" => cmd_pixels(bench, &rest),
        "sound" | "snd" => cmd_sound(bench),
        "gromlog" | "gl" => cmd_gromlog(bench, &rest),
        "cycles" => println!("cycles: {}", cycles(bench)),
        other => eprintln!("bench99: unknown command `{other}` (try: load boot pc wp s f k r m pw pb screen vdp vram pixels sound cycles q)"),
    }
    true
}

fn parse_hex(s: &str) -> Option<u16> {
    let t = s.trim_start_matches('>').trim_start_matches("0x");
    u16::from_str_radix(t, 16).ok()
}

fn parse_dec(s: &str) -> Option<usize> {
    s.parse().ok()
}

fn cycles(b: &Bench) -> u64 {
    match b {
        Bench::Bare { cpu, .. } => cpu.cycles(),
        Bench::Boot { m, .. } => m.cpu().cycles(),
    }
}

fn peek(b: &Bench, addr: u16) -> u8 {
    // The cartridge window is shadowed (see `Bench`); everything else is the
    // bus's own side-effect-free peek.
    let (peeked, cart) = match b {
        Bench::Bare { bus, cart, .. } => (bus.peek(addr), cart),
        Bench::Boot { m, cart } => (m.bus().peek(addr), cart),
    };
    if (0x6000..=0x7FFF).contains(&addr) {
        return cart.get((addr - 0x6000) as usize).copied().unwrap_or(0);
    }
    peeked
}

fn peek_word(b: &Bench, addr: u16) -> u16 {
    ((peek(b, addr) as u16) << 8) | peek(b, addr | 1) as u16
}

fn poke(b: &mut Bench, addr: u16, v: u8) {
    match b {
        Bench::Bare { bus, .. } => bus.poke(addr, v),
        Bench::Boot { m, .. } => m.bus_mut().poke(addr, v),
    }
}

fn cpu_state(b: &Bench) -> (u16, u16, u16) {
    match b {
        Bench::Bare { cpu, .. } => (cpu.pc(), cpu.wp(), cpu.st()),
        Bench::Boot { m, .. } => (m.cpu().pc(), m.cpu().wp(), m.cpu().st()),
    }
}

fn cmd_load(b: &mut Bench, args: &[&str]) {
    let Some(path) = args.first() else {
        eprintln!("usage: load <file.bin>");
        return;
    };
    let rom = match std::fs::read(path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("bench99: cannot read {path}: {e}");
            return;
        }
    };
    let banks = (rom.len() / 0x2000).max(1);
    match b {
        Bench::Bare { bus, cart, .. } => {
            *cart = rom.clone();
            bus.load_cartridge_rom(rom, banks);
        }
        Bench::Boot { m, cart } => {
            *cart = rom.clone();
            m.bus_mut().load_cartridge_rom(rom, banks);
        }
    }
    println!("loaded cartridge ROM at >6000 ({banks} bank(s))");
}

fn cmd_boot(b: &mut Bench, args: &[&str]) {
    let mut m = Box::new(Machine::new(&CONSOLE_ROM, &CONSOLE_GROM));
    let mut shadow = Vec::new();
    if let Some(path) = args.first() {
        let bytes = match std::fs::read(path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("bench99: cannot read {path}: {e}");
                return;
            }
        };
        if path.to_ascii_lowercase().ends_with(".ctg") {
            match Cartridge::parse(&bytes) {
                Ok(cart) => {
                    shadow = cart.rom.clone();
                    m.mount_cartridge(&cart);
                }
                Err(e) => {
                    eprintln!("bench99: bad .ctg: {e:?}");
                    return;
                }
            }
        } else {
            shadow = bytes.clone();
            let banks = (bytes.len() / 0x2000).max(1);
            m.bus_mut().load_cartridge_rom(bytes, banks);
        }
        println!("booted with cartridge {path}");
    } else {
        println!("booted bare console");
    }
    // Let the firmware reach the master title screen.
    for _ in 0..40 {
        m.run_frame();
    }
    *b = Bench::Boot { m, cart: shadow };
}

fn cmd_set_pcwp(b: &mut Bench, args: &[&str], is_pc: bool) {
    let Some(v) = args.first().and_then(|s| parse_hex(s)) else {
        eprintln!("usage: {} <hex-address>", if is_pc { "pc" } else { "wp" });
        return;
    };
    match b {
        Bench::Bare { cpu, .. } => {
            if is_pc {
                cpu.set_pc(v);
            } else {
                cpu.set_wp(v);
            }
        }
        Bench::Boot { .. } => {
            eprintln!("pc/wp are bare-bench commands (the booted machine's CPU belongs to the firmware)");
        }
    }
}

/// Disassemble the instruction at PC (side-effect-free), for the trace line.
fn disasm_at(b: &Bench, pc: u16) -> (String, String) {
    let mut buf = [0u8; 6];
    for (i, slot) in buf.iter_mut().enumerate() {
        *slot = peek(b, pc.wrapping_add(i as u16));
    }
    match disasm::decode_at(&buf, 0, pc) {
        Ok(d) => {
            let words: Vec<String> = (0..d.len / 2)
                .map(|i| format!("{:04X}", ((buf[2 * i] as u16) << 8) | buf[2 * i + 1] as u16))
                .collect();
            (words.join(" "), format!("{} {}", d.mnemonic, d.operands.join(",")))
        }
        Err(_) => (format!("{:04X}", ((buf[0] as u16) << 8) | buf[1] as u16), "?".into()),
    }
}

fn cmd_step(b: &mut Bench, args: &[&str]) {
    let n = args.first().and_then(|s| parse_dec(s)).unwrap_or(1);
    for _ in 0..n {
        let (pc, _, _) = cpu_state(b);
        let (hex, text) = disasm_at(b, pc);
        let spent = match b {
            Bench::Bare { cpu, bus, .. } => cpu.step(bus),
            Bench::Boot { m, .. } => m.step(),
        };
        let (npc, _, nst) = cpu_state(b);
        println!(
            ">{pc:04X}  {hex:<14} {text:<22} {spent:>3} cycles   ST={} PC→>{npc:04X}",
            st_string(nst)
        );
    }
}

/// `x [n]` — execute n instructions (default 1000) without tracing each one;
/// prints only the end state. The workhorse for "run this program a while".
fn cmd_exec(b: &mut Bench, args: &[&str]) {
    let n = args.first().and_then(|s| parse_dec(s)).unwrap_or(1000);
    let before = cycles(b);
    for _ in 0..n {
        match b {
            Bench::Bare { cpu, bus, .. } => {
                cpu.step(bus);
            }
            Bench::Boot { m, .. } => {
                m.step();
            }
        }
    }
    let (pc, _, st) = cpu_state(b);
    println!("ran {n} instructions ({} cycles)   PC=>{pc:04X}  ST={}", cycles(b) - before, st_string(st));
}

/// `u <hex>` — a breakpoint, bench style: execute silently until PC equals the
/// given address (or a safety cap of one million instructions is reached).
fn cmd_until(b: &mut Bench, args: &[&str]) {
    let Some(target) = args.first().and_then(|s| parse_hex(s)) else {
        eprintln!("usage: u <hex-address>");
        return;
    };
    let before = cycles(b);
    for n in 0..1_000_000u32 {
        let (pc, _, _) = cpu_state(b);
        if pc == target {
            println!("break at >{pc:04X} after {n} instructions ({} cycles)", cycles(b) - before);
            return;
        }
        match b {
            Bench::Bare { cpu, bus, .. } => {
                cpu.step(bus);
            }
            Bench::Boot { m, .. } => {
                m.step();
            }
        }
    }
    let (pc, _, _) = cpu_state(b);
    println!("gave up after 1,000,000 instructions; PC=>{pc:04X} (target >{target:04X} never reached)");
}

/// Render ST's defined bits: set bits by name, clear as '-', plus the mask.
fn st_string(st: u16) -> String {
    let names = ["L>", "A>", "EQ", "C", "OV", "OP", "X"];
    let mut out = String::new();
    for (i, n) in names.iter().enumerate() {
        if st & (0x8000 >> i) != 0 {
            out.push_str(n);
        } else {
            out.push('-');
        }
        out.push(' ');
    }
    out.push_str(&format!("mask={:X}", st & 0xF));
    out
}

fn cmd_frames(b: &mut Bench, args: &[&str]) {
    let n = args.first().and_then(|s| parse_dec(s)).unwrap_or(1);
    match b {
        Bench::Boot { m, .. } => {
            for _ in 0..n {
                m.run_frame();
            }
        }
        Bench::Bare { .. } => eprintln!("f runs whole frames — boot the machine first (`boot`)"),
    }
}

fn key_by_name(name: &str) -> Option<TiKey> {
    use TiKey::*;
    let k = match name.to_ascii_uppercase().as_str() {
        "A" => A, "B" => B, "C" => C, "D" => D, "E" => E, "F" => F, "G" => G,
        "H" => H, "I" => I, "J" => J, "K" => K, "L" => L, "M" => M, "N" => N,
        "O" => O, "P" => P, "Q" => Q, "R" => R, "S" => S, "T" => T, "U" => U,
        "V" => V, "W" => W, "X" => X, "Y" => Y, "Z" => Z,
        "0" => Num0, "1" => Num1, "2" => Num2, "3" => Num3, "4" => Num4,
        "5" => Num5, "6" => Num6, "7" => Num7, "8" => Num8, "9" => Num9,
        "SPACE" => Space, "ENTER" => Enter, "." => Period, "," => Comma,
        "=" => Equals, ";" => Semicolon, "/" => Slash,
        // Joystick directions/buttons (columns 6–7), for `press`/`rel`.
        "J1F" => Joy1Fire, "J1L" => Joy1Left, "J1R" => Joy1Right,
        "J1D" => Joy1Down, "J1U" => Joy1Up,
        "J2F" => Joy2Fire, "J2L" => Joy2Left, "J2R" => Joy2Right,
        "J2D" => Joy2Down, "J2U" => Joy2Up,
        _ => return None,
    };
    Some(k)
}

/// `press`/`rel <name>` — hold or release a key or joystick switch on the
/// *current* bench (bare or boot), so code that scans the 9901 matrix reads it.
/// Unlike `k` (which taps + runs frames in boot mode), this just sets the switch
/// state and returns, for verifying input reads on the bare bench.
fn cmd_press(b: &mut Bench, args: &[&str], down: bool) {
    let Some(k) = args.first().and_then(|s| key_by_name(s)) else {
        eprintln!("usage: {} <A–Z 0–9 SPACE ENTER . , = ; / | J1U J1D J1L J1R J1F J2..>", if down { "press" } else { "rel" });
        return;
    };
    match b {
        Bench::Bare { bus, .. } => bus.keyboard.set_key(k, down),
        Bench::Boot { m, .. } => m.bus_mut().keyboard.set_key(k, down),
    }
}

fn cmd_key(b: &mut Bench, args: &[&str]) {
    let Some(k) = args.first().and_then(|s| key_by_name(s)) else {
        eprintln!("usage: k <A–Z, 0–9, SPACE, ENTER, . , = ; />");
        return;
    };
    match b {
        Bench::Boot { m, .. } => {
            // Hold for ~10 frames so the firmware's KSCAN debounce sees it
            // (the same rhythm the core's own integration tests use).
            m.set_key(k, true);
            for _ in 0..10 {
                m.run_frame();
            }
            m.set_key(k, false);
            for _ in 0..10 {
                m.run_frame();
            }
        }
        Bench::Bare { .. } => eprintln!("k needs the booted machine (`boot`)"),
    }
}

fn print_regs(b: &Bench) {
    let (pc, wp, st) = cpu_state(b);
    println!("PC=>{pc:04X}  WP=>{wp:04X}  ST={}", st_string(st));
    for row in 0..4 {
        let mut line = String::new();
        for col in 0..4 {
            let n = row * 4 + col;
            let v = peek_word(b, wp.wrapping_add(2 * n));
            line.push_str(&format!("R{n:<2}=>{v:04X}   "));
        }
        println!("{line}");
    }
}

fn cmd_mem(b: &Bench, args: &[&str]) {
    let Some(addr) = args.first().and_then(|s| parse_hex(s)) else {
        eprintln!("usage: m <hex-address> [bytes]");
        return;
    };
    let n = args.get(1).and_then(|s| parse_dec(s)).unwrap_or(16);
    let mut i = 0usize;
    while i < n {
        let base = addr.wrapping_add(i as u16);
        let mut hex = String::new();
        let mut txt = String::new();
        for j in 0..16.min(n - i) {
            let v = peek(b, base.wrapping_add(j as u16));
            hex.push_str(&format!("{v:02X} "));
            txt.push(if (0x20..0x7F).contains(&v) { v as char } else { '.' });
        }
        println!(">{base:04X}  {hex:<48} {txt}");
        i += 16;
    }
}

fn cmd_poke(b: &mut Bench, args: &[&str], word: bool) {
    let Some(addr) = args.first().and_then(|s| parse_hex(s)) else {
        eprintln!("usage: {} <hex-address> <hex-value>...", if word { "pw" } else { "pb" });
        return;
    };
    let mut a = addr;
    for v in &args[1..] {
        let Some(x) = parse_hex(v) else {
            eprintln!("bad hex value `{v}`");
            return;
        };
        if word {
            poke(b, a, (x >> 8) as u8);
            poke(b, a.wrapping_add(1), x as u8);
            a = a.wrapping_add(2);
        } else {
            poke(b, a, x as u8);
            a = a.wrapping_add(1);
        }
    }
}

/// ASCII view of the name table (32×24). Screen codes on this machine are true
/// ASCII for the standard character set, so a plain byte→char map reads fine.
fn cmd_screen(b: &Bench) {
    let vdp = match b {
        Bench::Bare { bus, .. } => &bus.vdp,
        Bench::Boot { m, .. } => m.vdp(),
    };
    // Text mode (R1 bit 4 = M1) shows a 40-column name table; every other mode
    // is 32 wide. Reading the wrong width turns a legible screen into diagonal
    // hash — which is exactly what a 32-wide reader makes of a 40-column mode.
    let cols: u16 = if vdp.register(1) & 0x10 != 0 { 40 } else { 32 };
    let base = (vdp.register(2) as u16 & 0x0F) * 0x400;
    println!("+{}+", "-".repeat(cols as usize));
    for row in 0..24u16 {
        let mut line = String::new();
        for col in 0..cols {
            let v = vdp.vram(base + row * cols + col);
            line.push(if (0x20..0x7F).contains(&v) { v as char } else { '.' });
        }
        println!("|{line}|");
    }
    println!("+{}+", "-".repeat(cols as usize));
}

fn cmd_vdp(b: &Bench) {
    let vdp = match b {
        Bench::Bare { bus, .. } => &bus.vdp,
        Bench::Boot { m, .. } => m.vdp(),
    };
    let regs: Vec<String> = (0..8).map(|n| format!("R{n}=>{:02X}", vdp.register(n))).collect();
    println!("VDP {}", regs.join("  "));
}

/// Dump the VDP's private VRAM — an independent oracle for graphics work, since
/// the CPU cannot address VRAM directly. Mirrors `cmd_mem`'s format but reads
/// through `Vdp::vram` (14-bit address space, wraps mod 16 KiB).
fn cmd_vram(b: &Bench, args: &[&str]) {
    let Some(addr) = args.first().and_then(|s| parse_hex(s)) else {
        eprintln!("usage: vram <hex-address> [bytes]");
        return;
    };
    let vdp = match b {
        Bench::Bare { bus, .. } => &bus.vdp,
        Bench::Boot { m, .. } => m.vdp(),
    };
    let n = args.get(1).and_then(|s| parse_dec(s)).unwrap_or(16);
    let mut i = 0usize;
    while i < n {
        let base = addr.wrapping_add(i as u16);
        let mut hex = String::new();
        let mut txt = String::new();
        for j in 0..16.min(n - i) {
            let v = vdp.vram(base.wrapping_add(j as u16));
            hex.push_str(&format!("{v:02X} "));
            txt.push(if (0x20..0x7F).contains(&v) { v as char } else { '.' });
        }
        println!(">{base:04X}  {hex:<48} {txt}");
        i += 16;
    }
}

/// Render the actual 256×192 picture the VDP would show from the current VRAM
/// and registers, and print it as ASCII — one character per sampled pixel, the
/// character being that pixel's palette index as a hex digit (0–F). The default
/// step of 4 collapses the screen to 64×48 characters, which maps one glyph to
/// one multicolor "fat pixel" and keeps a bitmap picture legible. This is the
/// pixel-level oracle for Part III: `vram`/`vdp` prove the bytes and registers;
/// `pixels` proves they actually paint the intended picture. Sprites are drawn
/// (the whole-frame render includes them).
///
/// Palette indices 0 (transparent) and 1 (black) share the same RGB, so both
/// read as `0` here — an ambiguity that never matters for the shape/colour
/// checks this view is for.
fn cmd_pixels(b: &mut Bench, args: &[&str]) {
    let step = args.first().and_then(|s| parse_dec(s)).unwrap_or(4).max(1);
    let vdp = match b {
        Bench::Bare { bus, .. } => &mut bus.vdp,
        Bench::Boot { m, .. } => &mut m.bus_mut().vdp,
    };
    let mut fb = vec![0u32; WIDTH * HEIGHT];
    vdp.render(&mut fb);
    let cols = WIDTH.div_ceil(step);
    println!("+{}+", "-".repeat(cols));
    let mut y = 0;
    while y < HEIGHT {
        let mut line = String::new();
        let mut x = 0;
        while x < WIDTH {
            let idx = palette_index(fb[y * WIDTH + x]);
            line.push(std::char::from_digit(idx as u32, 16).unwrap_or('?'));
            x += step;
        }
        println!("|{line}|");
        y += step;
    }
    println!("+{}+", "-".repeat(cols));
}

/// Map a rendered `0x00RRGGBB` pixel back to its TMS9918A palette index (the
/// first entry that matches; 0/1 both being black resolve to 0). The bench's
/// display-only inverse of `PALETTE` — no core change needed.
fn palette_index(px: u32) -> u8 {
    let rgb = px & 0x00FF_FFFF;
    PALETTE.iter().position(|&c| c == rgb).unwrap_or(0) as u8
}

/// Dump the SN76489 PSG's state. The chip is write-only (a `>8400` read is open
/// bus), so its registers are read through `Psg`'s diagnostic accessors — the
/// sound-work oracle for Part IV: prove a note driver latched the frequency and
/// attenuation it intended. Attenuation is 0 (loud) .. 15 (silent).
fn cmd_sound(b: &Bench) {
    let psg = match b {
        Bench::Bare { bus, .. } => &bus.psg,
        Bench::Boot { m, .. } => &m.bus().psg,
    };
    print!("PSG ");
    for ch in 0..3 {
        print!(
            " ch{ch}: N={:>4} f={:>7.1}Hz att={:>2}  ",
            psg.period(ch),
            psg.frequency(ch),
            psg.volume(ch)
        );
    }
    let kind = if psg.noise_white() { "white" } else { "periodic" };
    println!("noise: {kind} att={}", psg.volume(3));
}

/// `gromlog [on|off]` — trace GROM data fetches. With the GPL interpreter
/// running (boot mode), the fetch stream IS the interpreter reading its bytecode
/// out of GROM, so `gromlog on`, a few steps, then `gromlog` shows the GPL
/// instruction bytes the interpreter consumed — the "other side" of the act that
/// the 9900 trace (`s`) shows. Each entry is `>addr:byte`.
fn cmd_gromlog(b: &mut Bench, args: &[&str]) {
    let Bench::Boot { m, .. } = b else {
        eprintln!("gromlog needs the booted machine (`boot`) — the GPL interpreter must be running");
        return;
    };
    match args.first().copied() {
        Some("on") => {
            m.bus_mut().grom_record(true);
            println!("grom log: on");
        }
        Some("off") => {
            m.bus_mut().grom_record(false);
            println!("grom log: off");
        }
        _ => {
            let log = m.bus().grom_log();
            println!("grom fetches: {}", log.len());
            for (a, v) in log.iter().take(40) {
                print!(">{a:04X}:{v:02X} ");
            }
            println!();
        }
    }
}
