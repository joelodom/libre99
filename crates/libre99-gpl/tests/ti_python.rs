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

//! **Milestone 4 gate** — TI PYTHON, the integer-expression REPL that replaces
//! TI BASIC as console GROM 1's program. Launch it from the menu (entry 1), type
//! expressions, and read the answers back off the screen. Covers precedence,
//! parentheses, truncating division/modulo, 16-bit wrap, variables, and the
//! three error messages (the session in the plan §9 plus the sign/overflow
//! edge cases).

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

fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}

fn row(m: &Machine, r: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..32)
        .map(|i| m.vdp().vram(base + r * 32 + i) as char)
        .collect::<String>()
        .trim_end_matches([' ', '\0'])
        .to_string()
}

/// ASCII → (key, shift). Uppercase letters and digits are unshifted; the
/// arithmetic operators are shifted keys on the TI-99/4A keyboard.
fn key_for(c: char) -> (TiKey, bool) {
    use TiKey::*;
    match c {
        '0' => (Num0, false), '1' => (Num1, false), '2' => (Num2, false), '3' => (Num3, false),
        '4' => (Num4, false), '5' => (Num5, false), '6' => (Num6, false), '7' => (Num7, false),
        '8' => (Num8, false), '9' => (Num9, false),
        ' ' => (Space, false), '\n' => (Enter, false), '=' => (Equals, false), '/' => (Slash, false),
        '+' => (Equals, true), '-' => (Slash, true), '*' => (Num8, true), '%' => (Num5, true),
        '(' => (Num9, true), ')' => (Num0, true),
        'X' => (X, false), 'Y' => (Y, false),
        other => panic!("test uses an unmapped key {other:?}"),
    }
}

fn type_line(m: &mut Machine, line: &str) {
    for c in line.chars() {
        let (k, shift) = key_for(c);
        if shift { m.set_key(TiKey::Shift, true); }
        m.set_key(k, true);
        frames(m, 3);
        m.set_key(k, false);
        if shift { m.set_key(TiKey::Shift, false); }
        frames(m, 3);
    }
    m.set_key(TiKey::Enter, true);
    frames(m, 3);
    m.set_key(TiKey::Enter, false);
    frames(m, 40); // let the evaluator run and print
}

/// Boot, walk the menu, and launch TI PYTHON (entry 1) — `None` when the
/// authentic console ROM is absent (the caller then skips).
fn launch_ti_python() -> Option<Machine> {
    let console_rom = CONSOLE_ROM.as_deref()?;
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    frames(&mut m, 40);
    m.set_key(TiKey::Space, true);
    frames(&mut m, 3);
    m.set_key(TiKey::Space, false);
    frames(&mut m, 220);
    m.set_key(TiKey::Num1, true);
    frames(&mut m, 20);
    m.set_key(TiKey::Num1, false);
    frames(&mut m, 40);
    Some(m)
}

#[test]
fn ti_python_evaluates_the_reference_session() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    assert_eq!(row(&m, 0), "TI PYTHON 0.0.1", "versioned banner should be on screen");

    // Each expression's answer lands on the row just below its input; prompts
    // step down two rows per line (input rows 2,4,6,…, answers 3,5,7,…).
    let session: &[(&str, &str)] = &[
        ("2 + 3 * 4", "14"),        // precedence
        ("X = 7", ""),              // assignment prints nothing
        ("X * (X - 1)", "42"),      // variable + parentheses
        ("10 / 3", "3"),            // truncating divide
        ("-10 / 3", "-3"),          // truncates toward zero
        ("-10 % 3", "-1"),          // remainder takes the dividend's sign
        ("32767 + 1", "-32768"),    // 16-bit wrap
        ("Y", "NAME ERROR"),        // undefined variable
        ("10 / 0", "ZERO DIVISION ERROR"),
    ];

    let mut input_row = 2u16;
    for (expr, want) in session {
        type_line(&mut m, expr);
        let got = row(&m, input_row + 1);
        assert_eq!(&got, want, "`{expr}` -> got {got:?}, want {want:?}");
        input_row += 2;
    }
}

/// **Regression for QUALITY-EVALUATION G1 — unguarded evaluator stacks.**
///
/// The shunting-yard evaluator's operand stack (`>8350–835F`, 8 words) and
/// operator stack (`>8360–836F`, 16 bytes) sit directly below the GPL
/// interpreter's own cells: `>8370` (VDP top-of-memory) and `>8372/>8373` (the
/// data- and sub-stack pointers). Without a bound, ~17 nested `(` walks the
/// operator-stack pointer past `>836F` into `>8370+`, corrupting the interpreter
/// and derailing `CALL`/`RTN` mid-evaluation. The `console.gpl` fix guards every
/// push site and aborts to a new `TOO COMPLEX` error (`EV_OVF`) before any push
/// reaches those cells. This pins it: the overflow is reported, the interpreter
/// cells are untouched, and the REPL keeps evaluating.
#[test]
fn deep_nesting_overflows_cleanly_and_the_repl_survives() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    assert_eq!(row(&m, 0), "TI PYTHON 0.0.1", "versioned banner should be on screen");

    // >8370 (VDP top-of-memory) is a stable cell — set at power-up, not a
    // moving stack pointer — so it can be compared before/after exactly. The
    // stack POINTERS (>8372/>8373) fluctuate legitimately at every sampling
    // instant (the key-wait loop itself uses the sub-stack), so for them we
    // assert the corruption signature instead: an overflow would have pushed
    // operator bytes — '(' = >28 — over the pointers, dropping them far below
    // their working bands (sub-stack ≈ >80 ± call depth, data ≈ >FE ± depth).
    // A fixed-instant equality check here regressed when CPU cycle-count
    // corrections shifted which interpreter phase the sample landed in.
    let vdp_top_before = m.bus().peek_word(0x8370);

    // ~20 nested '(' — well past the 16-byte operator stack. The 17th '(' would
    // push a byte at >8370 (VDP top-of-mem) and keep walking into >8372/>8373;
    // the guard must fire first and report the overflow instead.
    type_line(&mut m, "((((((((((((((((((((");
    assert_eq!(
        row(&m, 3),
        "TOO COMPLEX",
        "deep nesting must report the overflow, not crash or corrupt memory"
    );

    // The interpreter cells must not carry the overflow's signature — the
    // guard aborted before any push reached them (on the unguarded ROM the
    // pushed '(' bytes land here and the pointers read ≈ >28).
    let sub_stack = m.bus().peek(0x8373);
    let data_stack = m.bus().peek(0x8372);
    assert!(
        (0x60..=0x9F).contains(&sub_stack),
        ">8373 (GPL sub-stack pointer) out of its working band: >{sub_stack:02X}"
    );
    assert!(
        data_stack >= 0xA0,
        ">8372 (GPL data-stack pointer) out of its working band: >{data_stack:02X}"
    );
    assert_eq!(
        m.bus().peek_word(0x8370),
        vdp_top_before,
        ">8370 (VDP top-of-memory) must be intact after the overflow"
    );

    // And the REPL is still healthy: a plain expression evaluates normally.
    type_line(&mut m, "1+1");
    assert_eq!(
        row(&m, 5),
        "2",
        "the REPL must survive the overflow and keep evaluating (1+1 = 2)"
    );
}
