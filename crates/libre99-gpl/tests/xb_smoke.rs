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

//! **The Extended BASIC smoke gate** — the end-to-end proof that the
//! clean-room firmware is an XB substrate (`XB-CENSUS.md`; the L9 closure
//! for XB-class cartridges). Launches the third-party Extended BASIC
//! cartridge under OUR console ROM + OUR console GROM and drives a scripted
//! session: an immediate string `PRINT`, float assignment + arithmetic, a
//! stored two-line program, `RUN`, and `LIST` — asserting each step's output
//! on screen. When the authentic images are also present, the same script
//! runs under them and the produced output rows must agree (the differential
//! leg). Skips green when the third-party media is absent.

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

/// ASCII → (modifiers, key). Letters are typed shifted so they arrive as
/// uppercase under both keytabs.
fn key_for(c: char) -> (Option<TiKey>, TiKey) {
    use TiKey::*;
    match c {
        'A'..='Z' => (
            Some(Shift),
            match c {
                'A' => A, 'B' => B, 'C' => C, 'D' => D, 'E' => E, 'F' => F, 'G' => G,
                'H' => H, 'I' => I, 'J' => J, 'K' => K, 'L' => L, 'M' => M, 'N' => N,
                'O' => O, 'P' => P, 'Q' => Q, 'R' => R, 'S' => S, 'T' => T, 'U' => U,
                'V' => V, 'W' => W, 'X' => X, 'Y' => Y, 'Z' => Z,
                _ => unreachable!(),
            },
        ),
        '0' => (None, Num0), '1' => (None, Num1), '2' => (None, Num2),
        '3' => (None, Num3), '4' => (None, Num4), '5' => (None, Num5),
        '6' => (None, Num6), '7' => (None, Num7), '8' => (None, Num8),
        '9' => (None, Num9),
        ' ' => (None, Space), '=' => (None, Equals), '.' => (None, Period),
        '*' => (Some(Shift), Num8), '"' => (Some(Fctn), P),
        other => panic!("no TI keystroke mapped for {other:?}"),
    }
}

fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}

fn press(m: &mut Machine, modifier: Option<TiKey>, k: TiKey, hold: usize, settle: usize) {
    if let Some(mo) = modifier {
        m.set_key(mo, true);
    }
    m.set_key(k, true);
    frames(m, hold);
    m.set_key(k, false);
    if let Some(mo) = modifier {
        m.set_key(mo, false);
    }
    frames(m, settle);
}

fn type_line(m: &mut Machine, line: &str, settle: usize) {
    for c in line.chars() {
        let (mo, k) = key_for(c);
        press(m, mo, k, 3, 3);
    }
    press(m, None, TiKey::Enter, 3, settle);
}

/// One decoded screen row. XB screens store ASCII biased by `+>60`
/// (space = `>80`); decode that band, blank everything else, trim the edges.
fn row(m: &Machine, r: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..32)
        .map(|i| {
            let b = m.vdp().vram(base + r * 32 + i);
            let c = if (0x80..=0xDF).contains(&b) { b - 0x60 } else { b };
            if (0x20..0x7F).contains(&c) { c as char } else { ' ' }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn rows(m: &Machine) -> Vec<String> {
    (0..24).map(|r| row(m, r)).collect()
}

fn has_row(m: &Machine, want: &str) -> bool {
    rows(m).iter().any(|r| r == want)
}

/// Boot `rom`+`grom` with the XB cart, launch its menu entry, and run the
/// scripted session, asserting each step's output. Returns the final screen.
fn run_session(rom: &[u8], grom: &[u8], cart: &Cartridge, label: &str) -> Vec<String> {
    let mut m = Machine::new(rom, grom);
    m.mount_cartridge(cart);
    m.reset();
    frames(&mut m, 40);
    press(&mut m, None, TiKey::Space, 3, 300); // title -> selection menu

    // The cart's menu entry (the row naming EXTENDED), else entry 2.
    let mut entry = '2';
    for r in 0..24 {
        let t = row(&m, r);
        if t.contains("EXTENDED") {
            if let Some(d) = t.chars().next().filter(char::is_ascii_digit) {
                entry = d;
            }
            break;
        }
    }
    let (mo, k) = key_for(entry);
    press(&mut m, mo, k, 6, 500);

    type_line(&mut m, "PRINT \"HELLO\"", 150);
    assert!(has_row(&m, "HELLO"), "[{label}] PRINT \"HELLO\" must print HELLO\n{:#?}", rows(&m));

    type_line(&mut m, "X=1.5", 150);
    type_line(&mut m, "PRINT X*2", 150);
    assert!(has_row(&m, "3"), "[{label}] PRINT X*2 must print 3 (floats!)\n{:#?}", rows(&m));

    type_line(&mut m, "10 PRINT \"HI\"", 120);
    type_line(&mut m, "20 END", 120);
    type_line(&mut m, "RUN", 250);
    assert!(has_row(&m, "HI"), "[{label}] RUN must print HI\n{:#?}", rows(&m));

    type_line(&mut m, "LIST", 200);
    assert!(
        has_row(&m, "10 PRINT \"HI\"") && has_row(&m, "20 END"),
        "[{label}] LIST must list both lines\n{:#?}",
        rows(&m)
    );
    rows(&m)
}

/// The gate: Extended BASIC executes under the all-clean-room firmware.
/// The 2026-07-06 L9 symptom — "READY appears but nothing executes" — is the
/// exact failure mode the PRINT/RUN asserts would reproduce.
#[test]
fn extended_basic_executes_under_the_clean_room_firmware() {
    let Some(cart_bytes) = libre99_core::third_party::load("cartridges/xb25.ctg") else { skip!() };
    let cart = Cartridge::parse(&cart_bytes).expect("the XB cartridge parses");
    let our_rom = libre99_asm::system_rom::build_console_rom().unwrap();
    let our_grom = libre99_gpl::system_grom::build_console_grom().unwrap();
    let ours = run_session(&our_rom, &our_grom, &cart, "ours");

    // Differential leg: the same script under the authentic pair must produce
    // the same output rows (the non-empty screen content, order preserved).
    let (Some(auth_rom), Some(auth_grom)) = (
        libre99_core::third_party::load("roms/994aROM.Bin"),
        libre99_core::third_party::load("roms/994AGROM.Bin"),
    ) else {
        return;
    };
    let auth = run_session(&auth_rom, &auth_grom, &cart, "authentic");
    let content = |v: &[String]| v.iter().filter(|r| !r.is_empty()).cloned().collect::<Vec<_>>();
    assert_eq!(content(&auth), content(&ours), "the sessions' final screens must agree");
}
