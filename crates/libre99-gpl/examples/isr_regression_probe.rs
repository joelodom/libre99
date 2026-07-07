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

//! Reproduce the one coverage-sweep health regression (LIMITATIONS L8): **Video
//! Vegas** (`VideovegasC`) launches to a live console under the authentic GROM but
//! wedges under ours — display off, 9901 VDP interrupt masked. Runs the cart under
//! both firmwares through the coverage-sweep flow and reports, per leg, the ISR
//! tick count and the end-state (9901 enable + VDP R1), then dumps the screen.
//!
//! Root cause (from the GROM fetch trail): under ours the cart's data-driven path
//! diverges early and it CALLs interconnect slots `>002C/>002D/>0032/>0033`, which
//! vector into the console's GROM-2 GPL library under authentic but are graceful
//! `ILRTN` no-ops in ours (the unshipped-library, on-demand L6-class gap). Without
//! the routine's side effect it runs on into a bad state and disables the display.
//!
//! `cargo run -p libre99-gpl --example isr_regression_probe`

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

fn cart(name: &str) -> Cartridge {
    Cartridge::parse(&require(&format!("cartridges/{name}.ctg"))).unwrap()
}

fn tap(m: &mut Machine, k: TiKey, hold: usize, settle: usize) {
    m.set_key(k, true);
    for _ in 0..hold { m.run_frame(); }
    m.set_key(k, false);
    for _ in 0..settle { m.run_frame(); }
}

/// Press `k`, then run `frames` counting console VBLANK (`>8379`) ticks.
fn count_press(m: &mut Machine, k: TiKey, hold: usize, frames: usize) -> usize {
    m.set_key(k, true);
    for _ in 0..hold { m.run_frame(); }
    m.set_key(k, false);
    let mut prev = m.bus().peek(0x8379);
    let mut ticks = 0;
    for _ in 0..frames {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev { ticks += 1; }
        prev = t;
    }
    ticks
}

/// Replicate the coverage-sweep flow (same frame counts) and report ISR ticks +
/// end-state per leg. Also records the interconnect/service slots the cart CALLed.
fn sweep_flow(name: &str, grom: &[u8], label: &str) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(&cart(name));
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 260);
    m.bus_mut().grom_record(true);
    let l = count_press(&mut m, TiKey::Num2, 6, 320);
    let (le, lr) = (m.bus().tms9901.vdp_interrupt_enabled(), m.vdp().register(1));
    tap(&mut m, TiKey::Space, 4, 30);
    tap(&mut m, TiKey::Num1, 4, 30);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 200);
    let f = count_press(&mut m, TiKey::Num2, 6, 220);
    let (fe, fr) = (m.bus().tms9901.vdp_interrupt_enabled(), m.vdp().register(1));
    let services: std::collections::BTreeSet<u16> = m.bus().grom_log().iter()
        .map(|(a, _)| *a).filter(|&a| (0x0010..=0x005F).contains(&a)).collect();
    println!(
        "  {label:10} launch: ticks={l:3} 9901={le} R1=>{lr:02X}   F5: ticks={f:3} 9901={fe} R1=>{fr:02X}\n\
         {:14} services CALLed: {:?}", "", services
    );
}

/// Launch and dump the screen (24 rows) to show whether the cart is running.
fn dump(name: &str, grom: &[u8], label: &str) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(&cart(name));
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 260);
    tap(&mut m, TiKey::Num2, 6, 300);
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    println!("--- {name} under {label}: R1=>{:02X} ---", m.vdp().register(1));
    for row in 0..24 {
        let line: String = (0..32)
            .map(|c| {
                let b = m.vdp().vram(base + row * 32 + c);
                if (0x20..0x7F).contains(&b) { b as char } else { ' ' }
            })
            .collect();
        if !line.trim().is_empty() {
            println!("    |{}|", line.trim_end());
        }
    }
    println!();
}

fn main() {
    let ours = our_grom();
    println!("== Video Vegas: coverage-sweep flow under both firmwares (LIMITATIONS L8) ==");
    sweep_flow("VideovegasC", &AUTHENTIC_GROM, "authentic");
    sweep_flow("VideovegasC", &ours, "ours");
    println!();
    dump("VideovegasC", &AUTHENTIC_GROM, "authentic");
    dump("VideovegasC", &ours, "ours");
}
