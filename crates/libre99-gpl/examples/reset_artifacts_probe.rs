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

//! F5-reset artifact bug: after playing TI Invaders and resetting, the title
//! screen shows leftover game graphics. m.reset() only resets the CPU — VRAM
//! persists — so the GROM boot (START) must repaint the *whole* screen. This
//! probe drives authentic vs ours into gameplay, resets, re-runs boot, and dumps
//! the name table + sprite attribute table so we can see exactly what authentic
//! clears that ours leaves dirty.

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

fn run(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}
fn press(m: &mut Machine, k: TiKey, settle: usize) {
    m.set_key(k, true);
    run(m, 4);
    m.set_key(k, false);
    run(m, settle);
}

/// Boot, enter TI Invaders gameplay (dirtying VRAM with tiles + sprites), then
/// F5-reset and let the title boot repaint.
fn into_game_then_reset(grom: &[u8], cart: &Cartridge) -> Machine {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    run(&mut m, 90);
    press(&mut m, TiKey::Space, 120); // leave title -> menu
    press(&mut m, TiKey::Num2, 200); // select TI Invaders
    press(&mut m, TiKey::Num1, 120); // pick a skill level -> gameplay
    m.set_key(TiKey::Joy1Fire, true);
    run(&mut m, 60);
    m.set_key(TiKey::Joy1Fire, false);
    run(&mut m, 40);
    // Now F5.
    m.reset();
    run(&mut m, 120);
    m
}

/// Count non-space, non-zero cells in the 768-byte name table (>0000..>02FF).
fn dirty_name_cells(m: &Machine) -> usize {
    (0..0x300)
        .filter(|&a| {
            let c = m.vdp().vram(a);
            c != 0x20 && c != 0x00
        })
        .count()
}

/// Report on the sprite attribute table (>0300). A leading Y of >D0 disables all
/// sprites; anything else means sprites will render. Returns (first_y, active).
fn sprite_state(m: &Machine) -> (u8, usize) {
    let first_y = m.vdp().vram(0x300);
    let mut active = 0;
    for s in 0..32 {
        let y = m.vdp().vram(0x300 + s * 4);
        if y == 0xD0 {
            break;
        }
        active += 1;
    }
    (first_y, active)
}

fn report(label: &str, grom: &[u8], cart: &Cartridge) {
    let m = into_game_then_reset(grom, cart);
    let (first_y, active) = sprite_state(&m);
    println!(
        "{label}: name-table dirty cells (not space/0) = {}",
        dirty_name_cells(&m)
    );
    println!(
        "        sprite[0].Y = >{first_y:02X}  ({}), active sprites before >D0 = {active}",
        if first_y == 0xD0 { "disabled" } else { "LIVE" }
    );
    // Row 23 (>02E0..>02FF) is below the copyright line — a good tell for
    // leftover game tiles.
    print!("        row 23:");
    for a in 0x2E0..0x300 {
        print!(" {:02X}", m.vdp().vram(a));
    }
    println!();
}

fn main() {
    let cart = Cartridge::parse(&require("cartridges/TI-Invaders.ctg")).unwrap();
    report("AUTHENTIC", &AUTHENTIC_GROM, &cart);
    println!();
    let ours = libre99_gpl::system_grom::build_console_grom().unwrap();
    report("OURS     ", &ours, &cart);
}
