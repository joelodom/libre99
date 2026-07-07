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
    press_enter(m);
}

fn press_enter(m: &mut Machine) {
    m.set_key(TiKey::Enter, true);
    frames(m, 3);
    m.set_key(TiKey::Enter, false);
    frames(m, 40); // let the evaluator run and print
}

/// Type with every adjacent pair of keys OVERLAPPED — each key goes down
/// while the previous one is still held, the way fast typists roll. Under
/// the v0 read loop (wait for a key, then wait for ALL keys up) the rolled
/// key was eaten whole; the KSCAN new-key protocol must deliver every one.
/// (Uses no shifted characters: SHIFT going up and down mid-roll is its own
/// scenario, and unshifted rollover is the reported bug.)
fn type_line_overlapped(m: &mut Machine, line: &str) {
    let keys: Vec<TiKey> = line
        .chars()
        .map(|c| {
            let (k, shift) = key_for(c);
            assert!(!shift, "overlapped typing helper takes unshifted keys only");
            k
        })
        .collect();
    let mut held: Option<TiKey> = None;
    for k in keys {
        if held == Some(k) {
            // A key can't roll over itself: a double-tap needs the up-edge
            // (one all-keys-up scan resets KSCAN's debounce cell).
            m.set_key(k, false);
            frames(m, 2);
            held = None;
        }
        m.set_key(k, true); // down while the previous key is still down
        frames(m, 2);
        if let Some(prev) = held {
            m.set_key(prev, false);
        }
        frames(m, 2);
        held = Some(k);
    }
    if let Some(prev) = held {
        m.set_key(prev, false);
    }
    frames(m, 3);
    press_enter(m);
}

/// Tap a key with an optional modifier held (FCTN combos: backspace/ERASE).
fn tap_combo(m: &mut Machine, modifier: Option<TiKey>, k: TiKey) {
    if let Some(mo) = modifier {
        m.set_key(mo, true);
    }
    m.set_key(k, true);
    frames(m, 3);
    m.set_key(k, false);
    if let Some(mo) = modifier {
        m.set_key(mo, false);
    }
    frames(m, 3);
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

/// **Regression for docs/TI-PYTHON.md §4 B1 — fast (overlapped) typing
/// dropped characters.** The v0 read loop waited for a key, then spun until
/// NO key was held before accepting the next; rolling a second key down
/// before releasing the first never presents an all-keys-up instant between
/// them, so the rolled key was eaten. The KSCAN new-key protocol (one
/// condition-bit event per changed key) must deliver every character.
#[test]
fn overlapped_typing_delivers_every_character() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    type_line_overlapped(&mut m, "144 / 12");
    assert_eq!(row(&m, 2), "> 144 / 12", "every rolled key must echo");
    assert_eq!(row(&m, 3), "12", "and the line must evaluate");
}

/// **docs/TI-PYTHON.md §4 B2 — backspace.** FCTN+S (>08 — what the desktop
/// frontend sends for host Backspace) must rub out the character left of the
/// cursor; ERASE (FCTN+3, >07) must clear the whole line; neither may echo
/// junk. Arrows and other control codes are ignored outright.
#[test]
fn backspace_erase_and_control_keys_edit_cleanly() {
    let Some(mut m) = launch_ti_python() else { skip!() };

    // "12<BS>3" reads back as "13".
    type_line(&mut m, "12");
    // (type_line pressed ENTER: that line evaluated to 12 on row 3. Now edit
    // a fresh line on row 4.)
    for c in ["1", "2"] {
        let (k, _) = key_for(c.chars().next().unwrap());
        tap_combo(&mut m, None, k);
    }
    tap_combo(&mut m, Some(TiKey::Fctn), TiKey::S); // backspace
    let (k3, _) = key_for('3');
    tap_combo(&mut m, None, k3);
    press_enter(&mut m);
    assert_eq!(row(&m, 4), "> 13", "backspace must rub out the 2");
    assert_eq!(row(&m, 5), "13");

    // ERASE clears the whole line; retype and evaluate.
    for c in "999".chars() {
        let (k, _) = key_for(c);
        tap_combo(&mut m, None, k);
    }
    tap_combo(&mut m, Some(TiKey::Fctn), TiKey::Num3); // ERASE
    for c in "42".chars() {
        let (k, _) = key_for(c);
        tap_combo(&mut m, None, k);
    }
    press_enter(&mut m);
    assert_eq!(row(&m, 6), "> 42", "ERASE must clear the 999");
    assert_eq!(row(&m, 7), "42");

    // An arrow key (FCTN+E, >0B) echoes nothing.
    tap_combo(&mut m, Some(TiKey::Fctn), TiKey::E);
    assert_eq!(row(&m, 8), ">", "control codes must not echo junk");

    // Backspace with nothing typed is ignored (the prompt survives).
    tap_combo(&mut m, Some(TiKey::Fctn), TiKey::S);
    assert_eq!(row(&m, 8), ">");
    press_enter(&mut m); // blank line: just re-prompts, no SYNTAX ERROR
    assert_eq!(row(&m, 9), "", "a blank line must not print an error");
    assert_eq!(row(&m, 10), ">", "and the REPL re-prompts");
}

/// **docs/TI-PYTHON.md §4 B3 — the input cap.** Typing past the row edge is
/// swallowed: the echo stops at the last cell and the next row stays
/// untouched until the result prints there.
#[test]
fn input_stops_at_the_row_edge() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    let (k, _) = key_for('1');
    for _ in 0..40 {
        tap_combo(&mut m, None, k);
    }
    // 30 digits fit after "> "; the 40 presses must not scribble row 3.
    assert_eq!(row(&m, 2), format!("> {}", "1".repeat(30)));
    assert_eq!(row(&m, 3), "", "no VRAM scribble past the input row");
    press_enter(&mut m);
    // The 30-digit literal wraps mod 2^16 like all arithmetic.
    let mut v: u16 = 0;
    for _ in 0..30 {
        v = v.wrapping_mul(10).wrapping_add(1);
    }
    let want = format!("{}", v as i16);
    assert_eq!(row(&m, 3), want, "the capped line still evaluates");
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
