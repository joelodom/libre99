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

//! KSCAN differential gate (Console ROM M2). A GPL key-wait loop (`SCAN` +
//! `CEQ @>8375,>FF` + `BS`) idles until a key is pressed, then copies the
//! translated key (`>8375`) and the condition byte (`>837C`) into compared
//! cells. Run under OUR console ROM and the authentic ROM with the same GROM
//! (which carries the keyboard translation tables at `>1700+`): the key our
//! KSCAN reads, and the condition bit it raises, must match TI's byte-for-byte.

use std::sync::LazyLock;

use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static AUTH_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

/// A GROM with the real keyboard translation tables (spliced from our full
/// console GROM at `>1600..>1800`) and a custom key-wait program at `>0020`.
fn kscan_grom(program: &[u8]) -> Vec<u8> {
    let full = libre99_gpl::system_grom::build_console_grom().expect("console GROM");
    let mut grom = vec![0u8; 0x6000];
    grom[0x1600..0x1800].copy_from_slice(&full[0x1600..0x1800]);
    grom[0x20..0x20 + program.len()].copy_from_slice(program);
    grom
}

/// As [`kscan_grom`] but with the AUTHENTIC console GROM's keytabs — whose
/// unshifted table holds **lowercase** letters (ours holds uppercase), so the
/// KSCAN result-normalization paths (the state-0 a-z fold, and the states-1/2
/// alpha-lock switch read — RECON §23) actually fire.
fn kscan_grom_authentic_tables(program: &[u8]) -> Vec<u8> {
    let auth_grom = AUTH_GROM.as_deref().expect("presence checked by each test");
    let mut grom = vec![0u8; 0x6000];
    grom[0x1600..0x1800].copy_from_slice(&auth_grom[0x1600..0x1800]);
    grom[0x20..0x20 + program.len()].copy_from_slice(program);
    grom
}

/// Boot, idle `warm` frames waiting for a key, press `key`, idle `hold` frames
/// so KSCAN latches it; return `(>8375 key, >837C condition)`.
fn read_key(rom: &[u8], grom: &[u8], key: TiKey, warm: usize, hold: usize) -> (u8, u8) {
    let mut m = Machine::new(rom, grom);
    m.reset();
    for _ in 0..warm {
        m.run_frame();
    }
    m.set_key(key, true);
    for _ in 0..hold {
        m.run_frame();
    }
    (m.bus().peek(0x8360), m.bus().peek(0x8361))
}

// Key-wait: mode 0, wait for a key; capture KSCAN's own condition byte right
// after SCAN (before CEQ rewrites >837C), then latch the key and park.
//   ST @>8374,>00 ; ST @>8375,>FF
//   LP: SCAN ; ST @>8361,@>837C ; CEQ @>8375,>FF ; BS LP
//   ST @>8360,@>8375 ; <double-BR halt>
const KEYWAIT: &[u8] = &[
    0xBE, 0x74, 0x00, // ST @>8374,>00   (mode 0)
    0xBE, 0x75, 0xFF, // ST @>8375,>FF
    0x03, // LP (>0026): SCAN
    0xBC, 0x61, 0x7C, // ST @>8361,@>837C   (KSCAN's condition byte)
    0xD6, 0x75, 0xFF, // CEQ @>8375,>FF
    0x60, 0x26, // BS LP  (loop while no key)
    0xBC, 0x60, 0x75, // ST @>8360,@>8375   (latch the key)
    0x40, 0x34, 0x40, 0x34, // halt at >0032 (double-BR to >0034)
];

fn diff_key(name: &str, key: TiKey) {
    let auth_rom = AUTH_ROM.as_deref().expect("presence checked by each test");
    let grom = kscan_grom(KEYWAIT);
    let auth = read_key(auth_rom, &grom, key, 8, 8);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let ours = read_key(&our_rom, &grom, key, 8, 8);
    assert_ne!(auth.0, 0xFF, "{name}: sanity — the authentic ROM read a key");
    assert_eq!(
        ours, auth,
        "{name}: our KSCAN must match authentic — key/cond ours {ours:02X?} vs authentic {auth:02X?}"
    );
}

#[test]
fn kscan_reads_digit_5() {
    if AUTH_ROM.is_none() {
        skip!()
    }
    diff_key("Num5", TiKey::Num5);
}

#[test]
fn kscan_reads_letter_a() {
    if AUTH_ROM.is_none() {
        skip!()
    }
    diff_key("A", TiKey::A);
}

#[test]
fn kscan_reads_enter() {
    if AUTH_ROM.is_none() {
        skip!()
    }
    diff_key("Enter", TiKey::Enter);
}

#[test]
fn kscan_reads_space() {
    if AUTH_ROM.is_none() {
        skip!()
    }
    diff_key("Space", TiKey::Space);
}

// Mode-5 key-wait: re-select mode 5 (the 99/4A-native translation state)
// before EVERY scan — KSCAN rewrites >8374 to the state number (mode-3), so a
// caller wanting the native state re-sets 5 each call, as TI BASIC does.
//   LP: ST @>8374,>05 ; SCAN ; ST @>8361,@>837C ; CEQ @>8375,>FF ; BS LP
const KEYWAIT5: &[u8] = &[
    0xBE, 0x75, 0xFF, // ST @>8375,>FF
    0xBE, 0x74, 0x05, // LP (>0023): ST @>8374,>05
    0x03, // SCAN
    0xBC, 0x61, 0x7C, // ST @>8361,@>837C
    0xD6, 0x75, 0xFF, // CEQ @>8375,>FF
    0x60, 0x23, // BS LP
    0xBC, 0x60, 0x75, // ST @>8360,@>8375
    0x40, 0x34, 0x40, 0x34, // halt at >0032
];

/// State 0 (the zeroed->83C6 boot default = the 99/4 state) with the AUTHENTIC
/// lowercase keytab: KSCAN must fold a-z to uppercase — without reading the
/// alpha-lock switch — identically under both ROMs. This is the fold branch
/// actually firing (our own GROM's uppercase table never enters the a-z range).
#[test]
fn kscan_state0_folds_authentic_lowercase_table_to_uppercase() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    if AUTH_GROM.is_none() {
        skip!()
    }
    let grom = kscan_grom_authentic_tables(KEYWAIT);
    let auth = read_key(auth_rom, &grom, TiKey::A, 8, 8);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let ours = read_key(&our_rom, &grom, TiKey::A, 8, 8);
    assert_eq!(auth.0, 0x41, "sanity: authentic folds the table's 'a' to 'A' in state 0");
    assert_eq!(ours, auth, "our state-0 fold must match authentic (ours {ours:02X?})");
}

/// Mode 5 (the 99/4A-native state) with the authentic lowercase keytab: KSCAN
/// reads the alpha-lock switch (SBZ 21 / TB 7 / SBO 21). Our 9901 has no switch
/// input — the line idles high = "not locked" — so the lowercase table byte is
/// kept, identically under both ROMs (they read the same emulated line). This
/// differentially pins the switch-read path itself.
#[test]
fn kscan_native_state_reads_the_switch_and_keeps_lowercase() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    if AUTH_GROM.is_none() {
        skip!()
    }
    let grom = kscan_grom_authentic_tables(KEYWAIT5);
    let auth = read_key(auth_rom, &grom, TiKey::A, 8, 8);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let ours = read_key(&our_rom, &grom, TiKey::A, 8, 8);
    assert_eq!(auth.0, 0x61, "sanity: authentic keeps lowercase 'a' (switch reads unlocked)");
    assert_eq!(ours, auth, "our native-state switch read must match authentic (ours {ours:02X?})");
}

/// **Fix gate (2026-07-06): OUR shipped keytab types lowercase in Extended BASIC.**
/// The two tests above use the authentic keytab to prove the *mechanism*; this one
/// uses **our own shipped GROM** (`kscan_grom`, `keymap.rs`) — now carrying the
/// authentic *lowercase* unshifted table — to prove the *outcome*: native mode 5
/// (Extended BASIC's state) types lowercase `>61`, while state 0 (the menu /
/// TI-BASIC state) still folds to uppercase `>41` so the menu keeps working. This
/// is the regression guard for the "Extended BASIC types uppercase" bug
/// (`docs/KNOWN-ISSUES.md`); before the keytab flip, native mode returned `>41`.
#[test]
fn our_keytab_types_lowercase_in_native_state_and_folds_in_state0() {
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let native = read_key(&our_rom, &kscan_grom(KEYWAIT5), TiKey::A, 8, 8);
    assert_eq!(native.0, 0x61, "native-state 'a' must type lowercase >61 (the EB fix)");
    let state0 = read_key(&our_rom, &kscan_grom(KEYWAIT), TiKey::A, 8, 8);
    assert_eq!(state0.0, 0x41, "state-0 'a' must still fold to uppercase >41 (menu unaffected)");
}

/// A split-mode (1 or 2) key-wait GROM: set the mode, then loop `SCAN` while no
/// split-keyboard key is found — so each frame re-scans the joystick, leaving the
/// latest deflections in >8376/>8377 and the split key (or >FF) in >8375.
///   ST @>8374,mode ; LP: SCAN ; CEQ @>8375,>FF ; BS LP ; <halt>
fn kscan_grom_split(mode: u8) -> Vec<u8> {
    let program = [
        0xBE, 0x74, mode, // ST @>8374,mode
        0x03, // LP (>0023): SCAN
        0xD6, 0x75, 0xFF, // CEQ @>8375,>FF
        0x60, 0x23, // BS LP  (loop while no split key)
        0x40, 0x2B, 0x40, 0x2B, // halt (self-loop at >002B; unreached while joystick-only)
    ];
    kscan_grom(&program)
}

/// Boot, idle `warm`, press a joystick/keyboard input, idle `hold`; return
/// KSCAN's (>8376 Y deflection, >8377 X deflection, >8375 split key).
fn read_split(rom: &[u8], grom: &[u8], key: TiKey, warm: usize, hold: usize) -> (u8, u8, u8) {
    let mut m = Machine::new(rom, grom);
    m.reset();
    for _ in 0..warm {
        m.run_frame();
    }
    m.set_key(key, true);
    for _ in 0..hold {
        m.run_frame();
    }
    (m.bus().peek(0x8376), m.bus().peek(0x8377), m.bus().peek(0x8375))
}

#[test]
fn kscan_mode1_joystick1_up_reads_deflection() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    // Mode 1 scans joystick 1 (column 6). Pressing Up yields the GROM deflection
    // pair in >8376/>8377 and no split-keyboard key (>8375 = >FF), byte-identical
    // under both ROMs reading the same deflection table (>16E0).
    let grom = kscan_grom_split(1);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let auth = read_split(auth_rom, &grom, TiKey::Joy1Up, 8, 4);
    let ours = read_split(&our_rom, &grom, TiKey::Joy1Up, 8, 4);
    assert_ne!((auth.0, auth.1), (0, 0), "sanity: authentic read a nonzero Up deflection");
    assert_eq!(auth.2, 0xFF, "sanity: a pure joystick press leaves no split-keyboard key");
    assert_eq!(ours, auth, "mode-1 joystick Up: ours {ours:02X?} vs authentic {auth:02X?}");
}

#[test]
fn kscan_mode1_joystick1_down_differs_from_up() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    // Down must produce a different deflection than Up (proving the direction
    // index reaches distinct table entries), and match authentic.
    let grom = kscan_grom_split(1);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let up = read_split(auth_rom, &grom, TiKey::Joy1Up, 8, 4);
    let down_auth = read_split(auth_rom, &grom, TiKey::Joy1Down, 8, 4);
    let down_ours = read_split(&our_rom, &grom, TiKey::Joy1Down, 8, 4);
    assert_ne!(down_auth.0, up.0, "sanity: Down's Y deflection differs from Up's");
    assert_eq!(down_ours, down_auth, "mode-1 joystick Down: ours {down_ours:02X?} vs authentic {down_auth:02X?}");
}

#[test]
fn kscan_mode2_joystick2_up_reads_deflection() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    // Mode 2 scans joystick 2 (column 7) — the >0407 selector + unit-2 path.
    let grom = kscan_grom_split(2);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let auth = read_split(auth_rom, &grom, TiKey::Joy2Up, 8, 4);
    let ours = read_split(&our_rom, &grom, TiKey::Joy2Up, 8, 4);
    assert_ne!((auth.0, auth.1), (0, 0), "sanity: authentic read joystick 2 Up");
    assert_eq!(ours, auth, "mode-2 joystick Up: ours {ours:02X?} vs authentic {auth:02X?}");
}

#[test]
fn kscan_mode1_split_keyboard_matches_authentic() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    // Mode 1 also scans the left split-keyboard half (mask >0FFF, base >17C0).
    // Whatever key (or >FF) authentic reports for each press, ours must match —
    // exercising the split mask + the >17C0 split translation base differentially.
    let grom = kscan_grom_split(1);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    for key in [TiKey::A, TiKey::S, TiKey::D, TiKey::X, TiKey::Num1, TiKey::Q] {
        let auth = read_split(auth_rom, &grom, key, 8, 4).2;
        let ours = read_split(&our_rom, &grom, key, 8, 4).2;
        assert_eq!(ours, auth, "mode-1 split key {key:?}: ours {ours:02X} vs authentic {auth:02X}");
    }
}

#[test]
fn kscan_no_key_is_ff_and_clear() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    // With no key pressed the whole time, the wait loop never exits (>8360 stays
    // its initial 0), and KSCAN leaves >8375 = >FF with the condition clear —
    // identically under both ROMs.
    let grom = kscan_grom(KEYWAIT);
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");
    let read = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..16 {
            m.run_frame();
        }
        (m.bus().peek(0x8375), m.bus().peek(0x837C) & 0x20)
    };
    assert_eq!(read(auth_rom), (0xFF, 0x00), "authentic idle KSCAN");
    assert_eq!(read(&our_rom), (0xFF, 0x00), "our idle KSCAN matches");
}
