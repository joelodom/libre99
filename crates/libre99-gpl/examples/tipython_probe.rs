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

//! Fast iteration harness for the TI PYTHON REPL: build our GROM, dispatch
//! straight into the TI PYTHON entry (skipping the slow menu) via the GPL
//! sub-stack trampoline, type a line, and read the screen back.

use std::sync::LazyLock;

use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    libre99_core::third_party::load("roms/994aROM.Bin").unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/roms/994aROM.Bin)");
        std::process::exit(2)
    })
});

fn build() -> (Vec<u8>, u16) {
    let src = libre99_gpl::system_grom::console_gpl_source();
    let a = libre99_gpl::assemble(&src).unwrap();
    let entry = a.symbols.iter().find(|(n, _)| n == "PYENTRY").map(|(_, v)| *v).unwrap();
    (a.image, entry)
}

/// ASCII -> (TiKey, shift). Uppercase letters and digits are unshifted; the
/// operators used below are shifted keys on the TI keyboard.
fn key_for(c: char) -> Option<(TiKey, bool)> {
    use TiKey::*;
    Some(match c {
        '0' => (Num0, false), '1' => (Num1, false), '2' => (Num2, false), '3' => (Num3, false),
        '4' => (Num4, false), '5' => (Num5, false), '6' => (Num6, false), '7' => (Num7, false),
        '8' => (Num8, false), '9' => (Num9, false),
        ' ' => (Space, false), '\n' => (Enter, false), '=' => (Equals, false), '/' => (Slash, false),
        '+' => (Equals, true), '-' => (Slash, true), '*' => (Num8, true), '%' => (Num5, true),
        '(' => (Num9, true), ')' => (Num0, true),
        'A' => (A, false), 'B' => (B, false), 'C' => (C, false), 'D' => (D, false), 'E' => (E, false),
        'X' => (X, false), 'Y' => (Y, false), 'Z' => (Z, false),
        _ => return None,
    })
}

fn typ(m: &mut Machine, line: &str) {
    for c in line.chars() {
        let (k, shift) = key_for(c).unwrap_or_else(|| panic!("no key for {c:?}"));
        if shift { m.set_key(TiKey::Shift, true); }
        m.set_key(k, true);
        for _ in 0..3 { m.run_frame(); }
        m.set_key(k, false);
        if shift { m.set_key(TiKey::Shift, false); }
        for _ in 0..3 { m.run_frame(); }
    }
}

pub fn row(m: &Machine, r: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..32).map(|i| m.vdp().vram(base + r * 32 + i) as char).collect::<String>()
        .trim_end().to_string()
}

fn main() {
    // Drive through the menu (no cart -> TI PYTHON is entry 1).
    let (grom, entry) = build();
    eprintln!("PYENTRY = >{entry:04X}");
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    for _ in 0..40 { m.run_frame(); }
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..260 { m.run_frame(); }
    // Select entry 1 (TI PYTHON).
    m.set_key(TiKey::Num1, true);
    for _ in 0..20 { m.run_frame(); }
    m.set_key(TiKey::Num1, false);
    for _ in 0..60 { m.run_frame(); }
    println!("after launch, rows:");
    for r in 0..8 { println!("  r{r}: {:?}", row(&m, r)); }

    // Type an expression.
    let expr = std::env::args().nth(1).unwrap_or_else(|| "2+3*4\n".to_string());
    typ(&mut m, &expr);
    for _ in 0..60 { m.run_frame(); }
    println!("after typing {expr:?}:");
    for r in 0..12 { println!("  r{r}: {:?}", row(&m, r)); }
    let _ = entry;
}
