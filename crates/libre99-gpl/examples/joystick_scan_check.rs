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

//! Deterministic test of the >17C8 joystick/split-keyboard table: assemble a
//! minimal GROM that carries OUR keytab tables (via keymap::emit_gpl_bytes) and
//! runs a key-unit-1 SCAN loop. Hold each arrow key (and the joystick) and read
//! the direction cells the ROM's SCAN deposits: >8375 KEY, >8376 JOYY, >8377
//! JOYX. Compare with the same loop against a keytab-LESS GROM (the pre-fix
//! state) to show the table is what makes the directions register.

use std::sync::LazyLock;

use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

/// A minimal GROM: valid header, a key-unit-1 SCAN loop at the boot entry, and
/// (optionally) our keytab tables spliced at >1700.
fn scan_loop_grom(with_keytab: bool) -> Vec<u8> {
    let mut src = String::from(
        "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
        ST   @>8374,>01             ; key-unit 1 (joystick / arrow mode)
LOOP    SCAN
        B    LOOP
",
    );
    if with_keytab {
        src.push_str(&libre99_gpl::keymap::emit_gpl_bytes("KEYTAB"));
    }
    libre99_gpl::assemble(&src).unwrap().image
}

fn read_dir(grom: &[u8], keys: &[TiKey]) -> (u8, u8, u8) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    for _ in 0..10 { m.run_frame(); }
    for &k in keys { m.set_key(k, true); }
    // Sample the max-magnitude direction seen over a few frames (the loop keeps
    // overwriting the cells; a held key should read consistently).
    let (mut key, mut joyy, mut joyx) = (0u8, 0u8, 0u8);
    for _ in 0..20 {
        m.run_frame();
        let (k, y, x) = (m.bus().peek(0x8375), m.bus().peek(0x8376), m.bus().peek(0x8377));
        if k != 0xFF && k != 0x00 { key = k; }
        if y != 0x00 { joyy = y; }
        if x != 0x00 { joyx = x; }
    }
    (key, joyy, joyx)
}

fn main() {
    let with = scan_loop_grom(true);
    let without = scan_loop_grom(false);
    let cases = [
        ("LEFT  (FCTN+S)", vec![TiKey::Fctn, TiKey::S]),
        ("RIGHT (FCTN+D)", vec![TiKey::Fctn, TiKey::D]),
        ("UP    (FCTN+E)", vec![TiKey::Fctn, TiKey::E]),
        ("DOWN  (FCTN+X)", vec![TiKey::Fctn, TiKey::X]),
        ("JOY1 LEFT", vec![TiKey::Joy1Left]),
        ("JOY1 RIGHT", vec![TiKey::Joy1Right]),
        ("JOY1 FIRE", vec![TiKey::Joy1Fire]),
    ];
    println!("key-unit-1 SCAN results  (KEY>8375 / JOYY>8376 / JOYX>8377)\n");
    println!("{:<16} {:<26} {:<26}", "input", "WITH our keytab", "WITHOUT (pre-fix)");
    for (name, keys) in cases {
        let (k1, y1, x1) = read_dir(&with, &keys);
        let (k0, y0, x0) = read_dir(&without, &keys);
        println!(
            "{name:<16} KEY>{k1:02X} JOYY>{y1:02X} JOYX>{x1:02X}      KEY>{k0:02X} JOYY>{y0:02X} JOYX>{x0:02X}",
        );
    }
}
