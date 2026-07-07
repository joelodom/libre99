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

//! Sound bug trace: launch Tunnels of Doom under the authentic GROM and under
//! our rewrite, and watch (a) whether the SN76489 ever becomes audible (any
//! channel attenuation < 15) and (b) the console ISR's sound/interrupt
//! scratchpad cells. The ToD splash tune is played by the console ROM's VBLANK
//! ISR from a sound list (`>83CC/D` ptr, `>83CE` count), gated by the disable
//! flags at `>83C2`; comparing the two runs shows which cell our boot leaves
//! wrong. Run from the repo root: `cargo run -p libre99-gpl --example sound_probe`.

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

/// True if any PSG channel is currently audible (attenuation below 15).
fn audible(m: &Machine) -> bool {
    (0..4).any(|ch| m.bus().psg.volume(ch) < 0x0F)
}

/// The ISR sound/interrupt scratchpad cells, as a compact string.
fn cells(m: &Machine) -> String {
    let b = |a: u16| m.bus().peek(a);
    format!(
        "STmask={:X} 83CC={:02X}{:02X} 83CE={:02X} | isr[79={:02X}] R14(83FC)={:02X}{:02X}",
        m.cpu().st() & 0x000F,
        b(0x83CC), b(0x83CD), b(0x83CE),
        b(0x8379),
        b(0x83FC), b(0x83FD),
    )
}

/// Boot `grom` with ToD mounted, leave the title, launch program 2, then run
/// ~360 more frames sampling sound + the ISR cells.
/// First frame (from reset) at which the 9901 VDP interrupt mask turns on.
fn mask_on(m: &Machine) -> bool {
    m.bus().tms9901.vdp_interrupt_enabled()
}

fn run(label: &str, grom: &[u8], cart: &Cartridge) {
    println!("\n===== {label} =====");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    let mut mask_first = None;
    for f in 0..60 {
        m.run_frame();
        if mask_on(&m) && mask_first.is_none() { mask_first = Some(f); }
    }
    println!("9901 VDP-int mask on after boot: {} (first at frame {:?})", mask_on(&m), mask_first);

    // Leave the title screen.
    m.set_key(TiKey::Space, true);
    for _ in 0..4 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..200 { m.run_frame(); }
    println!("at menu:        {}", cells(&m));

    // Select program 2 (the first cartridge program = Tunnels of Doom).
    m.set_key(TiKey::Num2, true);
    for _ in 0..6 { m.run_frame(); }
    m.set_key(TiKey::Num2, false);

    let mut first_audible = None;
    let mut any_sound_write_cell = false;
    let mut isr_timer_prev = m.bus().peek(0x8379);
    let mut isr_ticks = 0u32; // how many frames >8379 changed (ISR liveness)
    for f in 0..380 {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != isr_timer_prev { isr_ticks += 1; }
        isr_timer_prev = t;
        if f < 4 {
            println!(
                "  launch+{f}: PC={:04X} int_line={:?} vdp_pending={} {}",
                m.cpu().pc(), m.bus().interrupt_line(), m.vdp().interrupt_pending(), cells(&m),
            );
        }
        if audible(&m) && first_audible.is_none() {
            first_audible = Some(f);
            println!("AUDIBLE @+{f:3}:  {}", cells(&m));
            for ch in 0..4 {
                println!(
                    "    ch{ch}: vol={:X} period={:03X}",
                    m.bus().psg.volume(ch),
                    if ch < 3 { m.bus().psg.period(ch) } else { 0 }
                );
            }
        }
        // Note if the cart ever installs a sound list.
        if m.bus().peek(0x83CE) != 0 || m.bus().peek(0x83CC) != 0 || m.bus().peek(0x83CD) != 0 {
            any_sound_write_cell = true;
        }
        if f % 60 == 0 {
            println!("  +{f:3}: audible={} {}", audible(&m), cells(&m));
        }
    }
    println!("ISR liveness: >8379 changed on {isr_ticks}/380 frames");
    match first_audible {
        Some(f) => println!("RESULT: sound became audible at +{f} frames"),
        None => println!("RESULT: SILENT for the whole run (sound-list cell touched: {any_sound_write_cell})"),
    }
}

/// Step one instruction at a time, reporting: the step at which the boot first
/// reads GROM data port `>9800` (GPL entry), whether/when ROM PC `>0604` (the
/// 9901 mask-enable instruction) is reached, and when the mask actually turns on.
fn trace_first_mask_enable(label: &str, grom: &[u8], cart: &Cartridge) {
    println!("\n===== TRACE: {label} =====");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    let mut steps = 0u64;
    let mut first_grom_read = None;
    let mut hit_0604 = None;
    let mut mask_step = None;
    let mut budget = 0u64;
    let mut prev_grom_log_len = 0;
    m.bus_mut().grom_record(true);
    while steps < 400_000 {
        if budget == 0 { m.bus_mut().vdp.vblank(); budget = libre99_core::machine::CYCLES_PER_FRAME; }
        let pc = m.cpu().pc();
        if pc == 0x0604 && hit_0604.is_none() { hit_0604 = Some(steps); }
        let mask_before = m.bus().tms9901.vdp_interrupt_enabled();
        let c = m.step();
        if !mask_before && m.bus().tms9901.vdp_interrupt_enabled() {
            // R12 holds the TMS9900 CRU base (bit address << 1).
            println!("  SBO at PC={pc:04X}  R12={:04X}", m.reg(12));
        }
        budget = budget.saturating_sub(c as u64);
        steps += 1;
        let ll = m.bus().grom_log().len();
        if ll > prev_grom_log_len && first_grom_read.is_none() {
            first_grom_read = Some((steps, m.bus().grom_log()[0].0));
        }
        prev_grom_log_len = ll;
        if mask_step.is_none() && m.bus().tms9901.vdp_interrupt_enabled() {
            mask_step = Some(steps);
            // The GPL instruction stream (GROM fetches) just before the enable.
            let log = m.bus().grom_log();
            let tail: Vec<String> = log.iter().rev().take(12).rev()
                .map(|(a, b)| format!("{a:04X}={b:02X}")).collect();
            println!("GPL (GROM) fetches just before enable: {}", tail.join(" "));
        }
        if mask_step.is_some() && steps > mask_step.unwrap() + 1 {
            let sp: Vec<String> = (0x8300u16..=0x830A).map(|a| format!("{:02X}", m.bus().peek(a))).collect();
            println!("scratchpad >8300..830A at enable: {}", sp.join(" "));
            break;
        }
    }
    println!("first GROM read:      {first_grom_read:?} (step, addr)");
    println!("reached ROM PC >0604: {hit_0604:?}");
    println!("9901 mask enabled at: {mask_step:?} steps");
}

fn main() {
    // tundoom.ctg / tunnelsofdoom.ctg are both Tunnels of Doom; use whichever
    // is present.
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
    trace_first_mask_enable("AUTHENTIC", &AUTHENTIC_GROM, &cart);
    trace_first_mask_enable("OURS", &ours, &cart);

    println!("\n===== DISASM: authentic GROM >0080..>00B0 =====");
    let mut a = 0x0080usize;
    while a < 0x00B0 {
        match libre99_gpl::decode::decode_at(&AUTHENTIC_GROM, a, a as u16) {
            Ok(d) => {
                let raw: Vec<String> = (a..a + d.len).map(|o| format!("{:02X}", AUTHENTIC_GROM[o])).collect();
                println!(">{a:04X}: {:<8} {:?}   [{}]  flow={:?}", d.mnemonic, d.operands, raw.join(" "), d.flow);
                a += d.len;
            }
            Err(e) => { println!(">{a:04X}: <decode error {e:?}>"); a += 1; }
        }
    }
}
