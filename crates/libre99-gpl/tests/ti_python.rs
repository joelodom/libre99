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

//! **The TI PYTHON v1 gates** — the language's spec of record is
//! `docs/TI-PYTHON.md`; these tests pin it: the four-row banner and `>>> `
//! prompt, the KSCAN new-key input engine (overlapped typing, backspace,
//! ERASE, the input cap, blank lines), the terminal scroll and block cursor,
//! full-size variable names in the VRAM table, Python floor `/` `//` `%` and
//! real unary minus, `print(…)` with string literals, `#` comments,
//! `exit()`/`quit()`, and the five error messages. Launches from the menu
//! (entry 1) and reads everything back off the screen. Needs the authentic
//! console ROM only when the clean-room ROM is not used — these gates run on
//! the authentic ROM to match the rest of the boot-flow estate (skip green
//! without third-party media).

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
        // A trailing >1E is the blinking block cursor, not text.
        .trim_end_matches([' ', '\0', '\u{1e}'])
        .to_string()
}

fn rows(m: &Machine) -> Vec<String> {
    (0..24).map(|r| row(m, r)).collect()
}

/// Some screen row holds exactly `want` (output rows carry no prompt, so an
/// exact match can't be confused with the echoed input).
fn has_row(m: &Machine, want: &str) -> bool {
    rows(m).iter().any(|r| r == want)
}

/// ASCII → (modifier, key). The console keytab folds unshifted letters to
/// uppercase in scan state 0, so plain letter keys type A–Z.
fn key_for(c: char) -> (Option<TiKey>, TiKey) {
    use TiKey::*;
    let bare = |k: TiKey| (None, k);
    let shift = |k: TiKey| (Some(Shift), k);
    let fctn = |k: TiKey| (Some(Fctn), k);
    match c {
        'A' => bare(A), 'B' => bare(B), 'C' => bare(C), 'D' => bare(D), 'E' => bare(E),
        'F' => bare(F), 'G' => bare(G), 'H' => bare(H), 'I' => bare(I), 'J' => bare(J),
        'K' => bare(K), 'L' => bare(L), 'M' => bare(M), 'N' => bare(N), 'O' => bare(O),
        'P' => bare(P), 'Q' => bare(Q), 'R' => bare(R), 'S' => bare(S), 'T' => bare(T),
        'U' => bare(U), 'V' => bare(V), 'W' => bare(W), 'X' => bare(X), 'Y' => bare(Y),
        'Z' => bare(Z),
        '0' => bare(Num0), '1' => bare(Num1), '2' => bare(Num2), '3' => bare(Num3),
        '4' => bare(Num4), '5' => bare(Num5), '6' => bare(Num6), '7' => bare(Num7),
        '8' => bare(Num8), '9' => bare(Num9),
        ' ' => bare(Space), '=' => bare(Equals), '/' => bare(Slash),
        ',' => bare(Comma), '.' => bare(Period),
        '+' => shift(Equals), '-' => shift(Slash), '*' => shift(Num8),
        '%' => shift(Num5), '(' => shift(Num9), ')' => shift(Num0),
        '#' => shift(Num3),
        '"' => fctn(P), '\'' => fctn(O), '_' => fctn(U),
        other => panic!("test uses an unmapped key {other:?}"),
    }
}

fn tap(m: &mut Machine, modifier: Option<TiKey>, k: TiKey) {
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

fn press_enter(m: &mut Machine) {
    m.set_key(TiKey::Enter, true);
    frames(m, 3);
    m.set_key(TiKey::Enter, false);
    frames(m, 40); // let the evaluator run and print
}

fn type_line(m: &mut Machine, line: &str) {
    for c in line.chars() {
        let (mo, k) = key_for(c);
        tap(m, mo, k);
    }
    press_enter(m);
}

/// Type with every adjacent pair of keys OVERLAPPED — each key goes down
/// while the previous one is still held, the way fast typists roll. Under
/// the v0 read loop (wait for a key, then wait for ALL keys up) the rolled
/// key was eaten whole; the KSCAN new-key protocol must deliver every one.
/// (Unshifted keys only: rollover of the plain rows is the reported bug.)
fn type_line_overlapped(m: &mut Machine, line: &str) {
    let keys: Vec<TiKey> = line
        .chars()
        .map(|c| {
            let (modifier, k) = key_for(c);
            assert!(modifier.is_none(), "overlapped typing helper takes unshifted keys only");
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

/// The four-row banner (docs/TI-PYTHON.md §3.1): the spliced version row,
/// three taglines, a blank row, the first prompt on row 5.
#[test]
fn banner_says_what_this_is() {
    let Some(m) = launch_ti_python() else { skip!() };
    assert_eq!(row(&m, 0), "TI PYTHON 0.0.1", "the spliced version row");
    assert_eq!(row(&m, 1), "A SUPER SIMPLE PYTHON-LIKE");
    assert_eq!(row(&m, 2), "INTERPRETER FOR THE TI-99/4A");
    assert_eq!(row(&m, 3), "EXIT() QUITS. 16-BIT INTEGERS.");
    assert_eq!(row(&m, 4), "", "a blank row before the prompt");
    assert!(row(&m, 5).starts_with(">>>"), "the Python prompt");
}

/// The reference session (docs/TI-PYTHON.md §2.3 core): precedence, names,
/// parentheses, Python floor division and modulo, 16-bit wrap, and the
/// errors. Expressions take two rows (input + output); assignments take one.
#[test]
fn ti_python_evaluates_the_reference_session() {
    let Some(mut m) = launch_ti_python() else { skip!() };

    let session: &[(&str, Option<&str>)] = &[
        ("2 + 3 * 4", Some("14")),               // precedence
        ("X = 7", None),                         // assignment prints nothing
        ("X * (X - 1)", Some("42")),             // variable + parentheses
        ("10 / 3", Some("3")),                   // floor divide
        ("-10 / 3", Some("-4")),                 // floors toward -inf (Python)
        ("-10 % 3", Some("2")),                  // Python modulo
        ("32767 + 1", Some("-32768")),           // 16-bit wrap
        ("Y", Some("NAME ERROR: Y")),            // undefined name, named
        ("10 / 0", Some("ZERO DIVISION ERROR")),
    ];

    let mut input_row = 5u16;
    for (expr, want) in session {
        type_line(&mut m, expr);
        assert_eq!(
            row(&m, input_row),
            format!(">>> {expr}"),
            "the echoed input line"
        );
        match want {
            Some(want) => {
                let got = row(&m, input_row + 1);
                assert_eq!(&got, want, "`{expr}` -> got {got:?}, want {want:?}");
                input_row += 2;
            }
            None => input_row += 1,
        }
    }
}

/// Full-size names (§3.2): up to 10 characters of letters/digits/underscore
/// in the 32-slot VRAM table; an 11th character is a SYNTAX ERROR; the 33rd
/// distinct name is a MEMORY ERROR; names stay distinct by full spelling.
#[test]
fn full_size_names_bind_and_overflow_honestly() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    type_line(&mut m, "RADIUS = 30");
    type_line(&mut m, "AREA = 3 * RADIUS * RADIUS");
    type_line(&mut m, "AREA");
    assert!(has_row(&m, "2700"), "3 * 30 * 30 through named variables\n{:#?}", rows(&m));

    type_line(&mut m, "_A1 = 5");
    type_line(&mut m, "_A1 * 2");
    assert!(has_row(&m, "10"), "underscores and digits in names");

    type_line(&mut m, "TOTAL = 1");
    type_line(&mut m, "TOTAL2 = 2");
    type_line(&mut m, "TOTAL + TOTAL2");
    assert!(has_row(&m, "3"), "TOTAL and TOTAL2 are distinct names");

    type_line(&mut m, "ABCDEFGHIJK = 1"); // 11 characters
    assert!(has_row(&m, "SYNTAX ERROR"), "an 11-character name is refused");
}

#[test]
fn the_33rd_name_is_a_memory_error() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    // 26 single letters + A0..A5 = 32 bindings; the 33rd distinct name fails.
    for c in b'A'..=b'Z' {
        type_line(&mut m, &format!("{} = 1", c as char));
    }
    for d in 0..6 {
        type_line(&mut m, &format!("A{d} = 1"));
    }
    assert!(!has_row(&m, "MEMORY ERROR"), "32 names must all bind");
    type_line(&mut m, "ZZ = 1");
    assert!(has_row(&m, "MEMORY ERROR"), "the 33rd name reports MEMORY ERROR");
    // And the table is intact: an existing name still reads and rebinds.
    type_line(&mut m, "A0 = 9");
    type_line(&mut m, "A0");
    assert!(has_row(&m, "9"), "rebinding an existing name still works\n{:#?}", rows(&m));
}

/// Python arithmetic semantics (§3.4): the unary-minus fix (v0 evaluated
/// 2*-3 as -3), floor `//`, and divisor-signed `%`.
#[test]
fn python_arithmetic_semantics() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    for (expr, want) in [
        ("2 * -3", "-6"),
        ("2 - -3", "5"),
        ("--5", "5"),
        ("10 // 3", "3"),
        ("-7 // 2", "-4"),
        ("7 % -2", "-1"),
        ("-5 % 3", "1"),
        ("7 % 2", "1"),
    ] {
        type_line(&mut m, expr);
        assert!(has_row(&m, want), "`{expr}` must print {want}\n{:#?}", rows(&m));
    }
    // The floor/mod identity a == (a//b)*b + a%b, spot-checked on a sign matrix.
    for (a, b) in [(7i32, 2i32), (-7, 2), (7, -2), (-7, -2)] {
        type_line(&mut m, &format!("{a} // {b} * {b} + {a} % {b}"));
        assert!(has_row(&m, &a.to_string()), "identity broke for a={a} b={b}");
    }
}

/// print(…) (§3.4): expression and string items, comma = single space,
/// print() = a blank row, unterminated strings and stray text are errors.
#[test]
fn print_items_strings_comments_and_exit() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    type_line(&mut m, "AREA = 2700");
    type_line(&mut m, "PRINT(\"AREA =\", AREA)");
    assert!(has_row(&m, "AREA = 2700"), "string + value items\n{:#?}", rows(&m));

    type_line(&mut m, "PRINT(1, 2+3, \"OK\")");
    assert!(has_row(&m, "1 5 OK"), "single-space separation");

    type_line(&mut m, "PRINT('SINGLE')");
    assert!(has_row(&m, "SINGLE"), "single-quoted strings work too");

    type_line(&mut m, "5 # A COMMENT");
    assert!(has_row(&m, "5"), "a # comment ends the line");

    type_line(&mut m, "PRINT(\"UNTERMINATED");
    assert!(has_row(&m, "SYNTAX ERROR"), "an unclosed string is refused");

    type_line(&mut m, "PRINT(1) 2");
    assert!(
        rows(&m).iter().filter(|r| *r == "SYNTAX ERROR").count() >= 2,
        "text after the close paren is refused"
    );

    // exit() returns to the selection menu (the same exit SYSINF takes).
    type_line(&mut m, "EXIT()");
    frames(&mut m, 260); // the menu redraws and rescans
    assert!(
        rows(&m).iter().any(|r| r.contains("FOR TI PYTHON")),
        "exit() must land on the selection menu\n{:#?}",
        rows(&m)
    );
}

/// The screen scrolls like a terminal (§3.1): past the bottom, everything
/// moves up one row, the banner eventually leaves, and the prompt sits on
/// the bottom row instead of the screen clearing (the v0 wrap, B4).
#[test]
fn the_screen_scrolls_instead_of_clearing() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    for _ in 0..12 {
        type_line(&mut m, "7"); // two rows per interaction
    }
    assert_ne!(row(&m, 0), "TI PYTHON 0.0.1", "the banner scrolled off");
    assert_eq!(row(&m, 23), ">>>", "the prompt rides the bottom row");
    assert_eq!(row(&m, 22), "7", "the last answer is right above it");
    assert_eq!(row(&m, 21), ">>> 7", "and its echo above that");
    // History is preserved in order up the screen, not cleared.
    assert_eq!(row(&m, 20), "7");
    assert_eq!(row(&m, 19), ">>> 7");
}

/// The block cursor (§3.1): char >1E blinks at the input cell, phased by the
/// ISR tick — over 40 frames the cell must show both the block and a space.
#[test]
fn the_cursor_blinks_at_the_input_cell() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let cell = base + 5 * 32 + 4; // row 5, column 4 — the input position
    let mut saw_block = false;
    let mut saw_space = false;
    for _ in 0..40 {
        m.run_frame();
        match m.vdp().vram(cell) {
            0x1E => saw_block = true,
            0x20 => saw_space = true,
            _ => {}
        }
    }
    assert!(saw_block, "the block cursor must appear");
    assert!(saw_space, "and blink off again");
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
    assert_eq!(row(&m, 5), ">>> 144 / 12", "every rolled key must echo");
    assert_eq!(row(&m, 6), "12", "and the line must evaluate");
}

/// **docs/TI-PYTHON.md §4 B2 — backspace.** FCTN+S (>08 — what the desktop
/// frontend sends for host Backspace) must rub out the character left of the
/// cursor; ERASE (FCTN+3, >07) must clear the whole line; neither may echo
/// junk. Arrows and other control codes are ignored outright.
#[test]
fn backspace_erase_and_control_keys_edit_cleanly() {
    let Some(mut m) = launch_ti_python() else { skip!() };

    // "12<BS>3" reads back as "13" (input row 5, answer row 6).
    for c in "12".chars() {
        let (mo, k) = key_for(c);
        tap(&mut m, mo, k);
    }
    tap(&mut m, Some(TiKey::Fctn), TiKey::S); // backspace
    let (mo, k3) = key_for('3');
    tap(&mut m, mo, k3);
    press_enter(&mut m);
    assert_eq!(row(&m, 5), ">>> 13", "backspace must rub out the 2");
    assert_eq!(row(&m, 6), "13");

    // ERASE clears the whole line; retype and evaluate (rows 7/8).
    for c in "999".chars() {
        let (mo, k) = key_for(c);
        tap(&mut m, mo, k);
    }
    tap(&mut m, Some(TiKey::Fctn), TiKey::Num3); // ERASE
    for c in "42".chars() {
        let (mo, k) = key_for(c);
        tap(&mut m, mo, k);
    }
    press_enter(&mut m);
    assert_eq!(row(&m, 7), ">>> 42", "ERASE must clear the 999");
    assert_eq!(row(&m, 8), "42");

    // An arrow key (FCTN+E, >0B) echoes nothing; backspace on an empty line
    // is ignored; ENTER on the empty line re-prompts without an error.
    tap(&mut m, Some(TiKey::Fctn), TiKey::E);
    tap(&mut m, Some(TiKey::Fctn), TiKey::S);
    assert_eq!(row(&m, 9), ">>>", "control codes must not echo junk");
    press_enter(&mut m);
    assert!(!has_row(&m, "SYNTAX ERROR"), "a blank line must not error");
    assert_eq!(row(&m, 10), ">>>", "the REPL just re-prompts");
}

/// **docs/TI-PYTHON.md §4 B3 — the input cap.** Typing past the row edge is
/// swallowed: the echo stops at column 31 and the next row stays untouched
/// until the result prints there.
#[test]
fn input_stops_at_the_row_edge() {
    let Some(mut m) = launch_ti_python() else { skip!() };
    let (_, k) = key_for('1');
    for _ in 0..40 {
        tap(&mut m, None, k);
    }
    // 28 digits fit after ">>> "; the 40 presses must not scribble row 6.
    assert_eq!(row(&m, 5), format!(">>> {}", "1".repeat(28)));
    assert_eq!(row(&m, 6), "", "no VRAM scribble past the input row");
    press_enter(&mut m);
    // The 28-digit literal wraps mod 2^16 like all arithmetic.
    let mut v: u16 = 0;
    for _ in 0..28 {
        v = v.wrapping_mul(10).wrapping_add(1);
    }
    assert_eq!(row(&m, 6), format!("{}", v as i16), "the capped line still evaluates");
}

/// **Regression for QUALITY-EVALUATION G1 — unguarded evaluator stacks.**
///
/// The shunting-yard evaluator's operand stack (`>8350–835F`, 8 words) and
/// operator stack (`>8360–836F`, 16 bytes) sit directly below the GPL
/// interpreter's own cells: `>8370` (VDP top-of-memory) and `>8372/>8373` (the
/// data- and sub-stack pointers). Without a bound, ~17 nested `(` walks the
/// operator-stack pointer past `>836F` into `>8370+`, corrupting the interpreter
/// and derailing `CALL`/`RTN` mid-evaluation. The `console.gpl` fix guards every
/// push site and aborts to a `TOO COMPLEX` error (`EV_OVF`) before any push
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
        row(&m, 6),
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
        row(&m, 8),
        "2",
        "the REPL must survive the overflow and keep evaluating (1+1 = 2)"
    );
}
