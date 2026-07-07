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

//! **Milestone 2 gate** — the rewritten GROM boots to our title, and on a
//! keypress builds an original **selection list** that scans console GROM 1 and
//! every cartridge GROM/ROM base for `>AA` program headers, renders each as
//! `n FOR NAME`, and launches the chosen program: GPL (GROM) programs via the
//! GPL sub-stack trampoline, ML (ROM) programs via `XML >F0`.
//!
//! The menu scan copies a window of each base into VRAM and walks its program
//! list there; because the console ROM re-writes the GROM address per byte, the
//! window MOVEs are slow, so the tests give the build a generous frame budget.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom()
        .unwrap_or_else(|d| panic!("console GROM assembly failed: {d:?}"))
}

fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}

/// Read a name-table row as a string (identity-mapped ASCII).
fn row(m: &Machine, r: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..32).map(|i| m.vdp().vram(base + r * 32 + i) as char).collect()
}

/// The whole visible name table as one string (for substring checks).
fn screen(m: &Machine) -> String {
    (0..24).map(|r| row(m, r)).collect::<Vec<_>>().join("\n")
}

/// Boot our GROM with `cart` mounted, leave the title, and build the menu.
fn boot_to_menu(cart: &Cartridge) -> Machine {
    let console_rom = CONSOLE_ROM.as_deref().expect("presence checked by each test");
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.mount_cartridge(cart);
    m.reset();
    frames(&mut m, 40); // title
    // Leave the title (any key), then let the scan build the list.
    m.set_key(TiKey::Space, true);
    frames(&mut m, 3);
    m.set_key(TiKey::Space, false);
    frames(&mut m, 220); // window MOVEs are slow
    m
}

/// Press digit `d` (1..=9) and hold it long enough for the selection loop to
/// read it and dispatch.
fn press_digit(m: &mut Machine, d: u8) {
    let key = match d {
        1 => TiKey::Num1,
        2 => TiKey::Num2,
        3 => TiKey::Num3,
        _ => panic!("digit out of range"),
    };
    m.set_key(key, true);
    frames(m, 20);
    m.set_key(key, false);
}

#[test]
fn menu_lists_and_launches_grom_cart() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let Some(data) = libre99_core::third_party::load("cartridges/amazing.ctg") else { skip!() };
    let cart = Cartridge::parse(&data).unwrap();
    let mut m = boot_to_menu(&cart);

    // TI PYTHON is always entry 1; the GROM cartridge is entry 2. Its header
    // stores the name with surrounding quotes, so match the raw substring.
    let scr = screen(&m);
    assert!(scr.contains("1 FOR TI PYTHON"), "menu:\n{scr}");
    assert!(scr.contains("2 FOR") && scr.contains("A-MAZE-ING"), "menu:\n{scr}");

    // Record from the dispatch on; a GPL launch runs the cartridge's GPL, which
    // fetches sustainedly from cartridge GROM (>6000+), entering at >602A.
    m.bus_mut().grom_record(true);
    press_digit(&mut m, 2);
    frames(&mut m, 40);
    let log = m.bus().grom_log();
    let cart_fetches = log.iter().filter(|(a, _)| *a >= 0x6000).count();
    let first = log.iter().find(|(a, _)| *a >= 0x6000).map(|(a, _)| *a);
    assert!(cart_fetches > 500, "cartridge GPL should run (got {cart_fetches} fetches)");
    assert_eq!(first, Some(0x602A), "cartridge entered at its GPL entry >602A");
}

#[test]
fn menu_lists_and_launches_rom_cart() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let Some(data) = libre99_core::third_party::load("cartridges/centipe.ctg") else { skip!() };
    let cart = Cartridge::parse(&data).unwrap();
    let mut m = boot_to_menu(&cart);

    let scr = screen(&m);
    assert!(scr.contains("1 FOR TI PYTHON"), "menu:\n{scr}");
    assert!(scr.contains("2 FOR CENTIPEDE"), "menu:\n{scr}");

    // An ML launch transfers the CPU into the cartridge ROM window >6000..>7FFF.
    press_digit(&mut m, 2);
    let mut launched = false;
    for _ in 0..120 {
        m.run_frame();
        let pc = m.cpu().pc();
        if (0x6000..0x8000).contains(&pc) {
            launched = true;
            break;
        }
    }
    assert!(launched, "ML cartridge should run in the >6000-7FFF window");
}

#[test]
fn menu_lists_ti_python_with_no_cartridge() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    // With no cart, only console GROM 1 (TI PYTHON) lists.
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    frames(&mut m, 40);
    m.set_key(TiKey::Space, true);
    frames(&mut m, 3);
    m.set_key(TiKey::Space, false);
    frames(&mut m, 220);
    let scr = screen(&m);
    assert!(scr.contains("1 FOR TI PYTHON"), "menu:\n{scr}");
    assert!(!scr.contains("2 FOR"), "no second entry without a cart:\n{scr}");
}
