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

//! Which CRU keyboard COLUMN does the console ROM select while scanning in
//! key-unit 1 (joystick 1) vs key-unit 0 (normal)? Step the CPU one instruction
//! at a time through a SCAN loop and histogram the 9901's selected column. This
//! rules the CRU layer in or out: if the ROM never selects the column our
//! keyboard matrix puts the joystick on (col 6/7), the joystick is unreachable.
//! (It does — the ROM even parks on col 6 when a Joy1 key is held.)

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

fn scan_loop_grom(mode: u8) -> Vec<u8> {
    let src = format!(
        "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
        ST   @>8374,>{mode:02X}
LOOP    SCAN
        B    LOOP
{}",
        libre99_gpl::keymap::emit_gpl_bytes("KEYTAB")
    );
    libre99_gpl::assemble(&src).unwrap().image
}

fn column_histogram(mode: u8, held: &[TiKey]) -> [u32; 8] {
    let grom = scan_loop_grom(mode);
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    for _ in 0..10 {
        m.run_frame();
    }
    for &k in held {
        m.set_key(k, true);
    }
    let mut hist = [0u32; 8];
    for _ in 0..200_000 {
        m.step();
        hist[m.bus().tms9901.selected_column() as usize] += 1;
    }
    hist
}

fn main() {
    println!("selected-column histogram over ~200k CPU steps\n");
    for (mode, name) in [(0u8, "key-unit 0 (normal)"), (1, "key-unit 1 (joystick 1)"), (2, "key-unit 2 (joystick 2)")] {
        println!("mode {mode} {name:<24}: {:?}", column_histogram(mode, &[]));
    }
    println!("\n(our keyboard.rs: cols 0-5 = keyboard, col 6 = Joy1, col 7 = Joy2)\n");

    let base = column_histogram(1, &[]);
    let held = column_histogram(1, &[TiKey::Joy1Left, TiKey::Joy1Fire]);
    println!("mode 1, columns touched (idle vs Joy1 held) — the ROM parks on the joystick column:");
    for c in 0..8 {
        if base[c] > 0 || held[c] > 0 {
            println!("  col {c}: idle {} / held {}", base[c], held[c]);
        }
    }
}
