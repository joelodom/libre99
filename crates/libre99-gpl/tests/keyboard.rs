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

//! Regression gate for the **keyboard/joystick decode tables** our rewritten
//! console GROM must carry at GROM `>1700` (see `crate::keymap`). The console
//! ROM's `SCAN` opcode looks these up at fixed addresses; omit them and every
//! keypress decodes to `>00`.
//!
//! Two paths matter, and TI Invaders exercised both: the **mode-0** blocks
//! (unshifted/shifted/FCTN/CTRL) that the menu, editors, and title-screen arrow
//! navigation use, and the **key-unit-1/2** joystick table at `>17C8` that
//! joystick games read *during play*. Before these landed, the master menu and
//! level select worked (number keys → unshifted block) but in-game movement was
//! dead (arrow keys → the missing FCTN block / `>17C8` table).

use std::sync::LazyLock;

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
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

/// Boot to the menu (mode-0 scan), hold FCTN+`key`, and return the ASCII the
/// ROM's `SCAN` deposits in KEY (`>8375`).
fn mode0_fctn(grom: &[u8], key: TiKey) -> u8 {
    let console_rom = CONSOLE_ROM.as_deref().expect("presence checked by each test");
    let mut m = Machine::new(console_rom, grom);
    for _ in 0..180 {
        m.run_frame();
    }
    m.set_key(TiKey::Fctn, true);
    m.set_key(key, true);
    let mut got = 0u8;
    for _ in 0..6 {
        m.run_frame();
        let k = m.bus().peek(0x8375);
        if k != 0xFF && k != 0x00 {
            got = k;
        }
    }
    got
}

#[test]
fn mode0_fctn_decodes_the_arrow_keys() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let grom = our_grom();
    // FCTN+S/D/E/X are the arrow keys; the ROM reads the FCTN block at >1765.
    assert_eq!(mode0_fctn(&grom, TiKey::S), 0x08, "FCTN+S = left arrow");
    assert_eq!(mode0_fctn(&grom, TiKey::D), 0x09, "FCTN+D = right arrow");
    assert_eq!(mode0_fctn(&grom, TiKey::E), 0x0B, "FCTN+E = up arrow");
    assert_eq!(mode0_fctn(&grom, TiKey::X), 0x0A, "FCTN+X = down arrow");
}

/// A minimal GROM: valid header + a key-unit-1 `SCAN` loop at the boot entry,
/// with our keytab tables optionally spliced at `>1700`.
fn scan_loop_grom(with_keytab: bool) -> Vec<u8> {
    let mut src = String::from(
        "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
        ST   @>8374,>01
LOOP    SCAN
        B    LOOP
",
    );
    if with_keytab {
        src.push_str(&libre99_gpl::keymap::emit_gpl_bytes("KEYTAB"));
    }
    libre99_gpl::assemble(&src).unwrap().image
}

/// Run the key-unit-1 SCAN loop with `keys` held; return the last non-idle value
/// seen in cell `addr` (KEY `>8375`, JOYY `>8376`, or JOYX `>8377`).
fn joystick_cell(grom: &[u8], keys: &[TiKey], addr: u16) -> u8 {
    let console_rom = CONSOLE_ROM.as_deref().expect("presence checked by each test");
    let mut m = Machine::new(console_rom, grom);
    for _ in 0..10 {
        m.run_frame();
    }
    for &k in keys {
        m.set_key(k, true);
    }
    let mut got = 0u8;
    for _ in 0..20 {
        m.run_frame();
        let v = m.bus().peek(addr);
        if v != 0xFF && v != 0x00 {
            got = v;
        }
    }
    got
}

fn joystick_key(grom: &[u8], keys: &[TiKey]) -> u8 {
    joystick_cell(grom, keys, 0x8375)
}

#[test]
fn joystick_mode_needs_the_17c8_table() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    // In key-unit 1 (what games select during play via >8374), SCAN translates
    // through the >17C8 table. Without it, arrow/fire input is dead (>00) — the
    // reported "keyboard does nothing in gameplay" bug. With it, they register.
    let with = scan_loop_grom(true);
    let without = scan_loop_grom(false);

    for keys in [
        vec![TiKey::Fctn, TiKey::S],
        vec![TiKey::Fctn, TiKey::D],
        vec![TiKey::Fctn, TiKey::E],
        vec![TiKey::Joy1Fire],
    ] {
        assert_eq!(joystick_key(&without, &keys), 0x00, "dead without the >17C8 table");
        assert_ne!(joystick_key(&with, &keys), 0x00, "live with the >17C8 table: {keys:?}");
    }
}

#[test]
fn joystick_directions_produce_deflection() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    // The joystick (as opposed to the keyboard arrows) decodes through the
    // deflection table at >16EA into JOYX/JOYY (>8377/>8376), signed +4/0/-4
    // (>04/>00/>FC). Without the table the joystick is "wired" but never deflects
    // (JOYX/JOYY stay 0) — the reported "joystick isn't hooked up" bug. This path
    // is independent of the keyboard, which still decodes to KEY (>8375).
    let with = scan_loop_grom(true);
    let without = scan_loop_grom(false);

    // JOYX (>8377): left = >FC (-4), right = >04 (+4).
    assert_eq!(joystick_cell(&with, &[TiKey::Joy1Left], 0x8377), 0xFC);
    assert_eq!(joystick_cell(&with, &[TiKey::Joy1Right], 0x8377), 0x04);
    // JOYY (>8376): up = >04 (+4), down = >FC (-4).
    assert_eq!(joystick_cell(&with, &[TiKey::Joy1Up], 0x8376), 0x04);
    assert_eq!(joystick_cell(&with, &[TiKey::Joy1Down], 0x8376), 0xFC);
    // Without the >16EA table, no deflection at all.
    assert_eq!(joystick_cell(&without, &[TiKey::Joy1Left], 0x8377), 0x00);
    assert_eq!(joystick_cell(&without, &[TiKey::Joy1Up], 0x8376), 0x00);
}
