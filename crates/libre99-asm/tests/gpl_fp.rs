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

//! **The M5 floating-point differential suite.** Every XML table-0 routine
//! (FADD/FSUB/FMUL/FDIV/FCOMP + the S-variants, ROUND/ROUND1/STST/OVEXP/OV)
//! and the table-1 conversions (CSN/CSNGR/CFI) runs under the authentic ROM
//! and ours from identically planted FAC/ARG/stack state; the observable
//! state must match bit-for-bit. Radix-100 numbers: 8 bytes, exponent biased
//! `>40`, seven 0-99 digits, negatives with the first word negated, zero =
//! first word `>0000` (RECON §9).
//!
//! The planted-operand cases cover signs, magnitude order, zero operands,
//! carries across digit boundaries, exponent extremes (the `>8354` overflow
//! paths — `>836C` is seeded with a halt so the FP error warp is a
//! controlled, compared branch), and the S-forms' VDP value-stack pops. The
//! FP fuzz sweeps random operand patterns — valid and garbage alike (the
//! differential bar doesn't care which; garbage must diverge identically).

use std::sync::{LazyLock, OnceLock};

use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// The authentic console ROM, or `None` after announcing the skip on stderr —
/// each caller `return`s early so its test passes green without the media.
fn auth_rom() -> Option<&'static [u8]> {
    let rom = AUTH_ROM.as_deref();
    if rom.is_none() {
        eprintln!("SKIPPED: third-party media not present");
    }
    rom
}

fn our_rom() -> &'static [u8] {
    static ROM: OnceLock<Vec<u8>> = OnceLock::new();
    ROM.get_or_init(|| libre99_asm::system_rom::build_console_rom().expect("console ROM assembles"))
}

/// A condition-safe halt at GROM address `a` (RECON §16's double-BR idiom).
fn halt(a: u16) -> [u8; 4] {
    let n = a + 2;
    let hi = 0x40 | ((n >> 8) as u8 & 0x1F);
    [hi, n as u8, hi, n as u8]
}

/// Plant FAC/ARG (+ optional VDP value-stack top), run `XML >op`, snapshot.
/// The observable: the full compared pad (gpl_core's masks), FAC/ARG exact,
/// >8354/>8375/>8376 exact, the VDP regs, and the value-stack window in VRAM.
struct FpCase {
    fac: [u8; 8],
    arg: [u8; 8],
    /// Planted at VDP >03C0.. as the value-stack top (for the S-forms);
    /// >836E points just past it.
    stack_top: Option<[u8; 8]>,
    xml_op: u8,
}

fn run_fp(rom: &[u8], c: &FpCase) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    // GPL: [XML >op][halt]. >836C (the FP error warp) points at the halt so
    // an error exit is a compared, controlled branch.
    let p = {
        let mut v = vec![0x0F, c.xml_op];
        let at = 0x20 + v.len() as u16;
        v.extend_from_slice(&halt(at));
        v
    };
    let mut grom = vec![0u8; 0x6000];
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    let mut m = Machine::new(rom, &grom);
    m.reset();
    for (i, &b) in c.fac.iter().enumerate() {
        m.bus_mut().poke(0x834A + i as u16, b);
    }
    for (i, &b) in c.arg.iter().enumerate() {
        m.bus_mut().poke(0x835C + i as u16, b);
    }
    // The FP error warp target: the halt's GROM address.
    m.bus_mut().poke(0x836C, 0x00);
    m.bus_mut().poke(0x836D, 0x22);
    if let Some(top) = c.stack_top {
        for (i, &b) in top.iter().enumerate() {
            m.vdp_mut().set_vram(0x03C0 + i as u16, b);
        }
        m.bus_mut().poke(0x836E, 0x03);
        m.bus_mut().poke(0x836F, 0xC8); // just past the planted 8 bytes
    }
    for _ in 0..6 {
        m.run_frame();
    }
    let pad: Vec<u8> = (0x8300u16..0x83E0)
        .map(|a| {
            let b = m.bus().peek(a);
            match a {
                0x8300..=0x8307 => 0,
                0x8372 | 0x8373 => 0,
                0x837C => b & 0xF8,
                0x83DA..=0x83DF => 0,
                _ => b,
            }
        })
        .collect();
    let regs: Vec<u8> = (0..8).map(|r| m.vdp().register(r)).collect();
    let vram: Vec<u8> = (0x0380u16..0x0400).map(|a| m.vdp().vram(a)).collect();
    (pad, regs, vram)
}

fn diff_fp(name: &str, c: &FpCase) {
    let Some(auth_rom) = auth_rom() else { return };
    let a = run_fp(auth_rom, c);
    let o = run_fp(our_rom(), c);
    for i in 0..a.0.len() {
        assert_eq!(
            a.0[i],
            o.0[i],
            "{name}: scratchpad >{:04X} differs (authentic {:02X} vs ours {:02X})\n  fac={:02X?}\n  arg={:02X?}",
            0x8300 + i,
            a.0[i],
            o.0[i],
            c.fac,
            c.arg
        );
    }
    assert_eq!(a.1, o.1, "{name}: VDP registers differ");
    assert_eq!(a.2, o.2, "{name}: the value-stack window differs");
}

/// Encode a simple radix-100 number: `digits` are 0-99 (first non-zero),
/// `exp` is the biased exponent byte, `neg` negates the first word.
fn num(exp: u8, digits: &[u8], neg: bool) -> [u8; 8] {
    let mut b = [0u8; 8];
    b[0] = exp;
    for (i, &d) in digits.iter().take(7).enumerate() {
        b[1 + i] = d;
    }
    if neg {
        let w = u16::from_be_bytes([b[0], b[1]]).wrapping_neg();
        b[0] = (w >> 8) as u8;
        b[1] = w as u8;
    }
    b
}

const ZERO: [u8; 8] = [0; 8];

macro_rules! fp_cases {
    ($($test:ident : $op:expr, $fac:expr, $arg:expr;)*) => {
        $(
            #[test]
            fn $test() {
                diff_fp(
                    stringify!($test),
                    &FpCase { fac: $fac, arg: $arg, stack_top: None, xml_op: $op },
                );
            }
        )*
    };
}

// ---- FADD (>06): ARG + FAC -> FAC ----
fp_cases! {
    fadd_simple:            0x06, num(0x40, &[5], false),        num(0x40, &[3], false);
    fadd_carry_digit:       0x06, num(0x40, &[99], false),       num(0x40, &[2], false);
    fadd_carry_top:         0x06, num(0x40, &[99, 99, 99, 99, 99, 99, 99], false), num(0x3A, &[99], false);
    fadd_align_small:       0x06, num(0x40, &[1], false),        num(0x3D, &[42], false);
    fadd_align_vanish:      0x06, num(0x40, &[1], false),        num(0x30, &[42], false);
    fadd_opposite_signs:    0x06, num(0x40, &[7], false),        num(0x40, &[3], true);
    fadd_negative_result:   0x06, num(0x40, &[3], false),        num(0x40, &[7], true);
    fadd_cancel_to_zero:    0x06, num(0x40, &[5, 25], false),    num(0x40, &[5, 25], true);
    fadd_cancel_normalize:  0x06, num(0x40, &[5, 25], false),    num(0x40, &[5, 24], true);
    fadd_fac_zero:          0x06, ZERO,                          num(0x41, &[12, 34], true);
    fadd_arg_zero:          0x06, num(0x41, &[12, 34], false),   ZERO;
    fadd_both_zero:         0x06, ZERO,                          ZERO;
    fadd_both_negative:     0x06, num(0x40, &[50], true),        num(0x40, &[60], true);
    fadd_exp_overflow:      0x06, num(0x7F, &[99, 99, 99, 99, 99, 99, 99], false), num(0x7F, &[99], false);

    // ---- FSUB (>07): ARG - FAC -> FAC ----
    fsub_simple:            0x07, num(0x40, &[3], false),        num(0x40, &[8], false);
    fsub_borrow:            0x07, num(0x40, &[1, 1], false),     num(0x40, &[2], false);
    fsub_to_negative:       0x07, num(0x40, &[9], false),        num(0x40, &[4], false);
    fsub_negative_fac:      0x07, num(0x40, &[9], true),         num(0x40, &[4], false);
    fsub_zero_fac:          0x07, ZERO,                          num(0x40, &[4], false);

    // ---- FMUL (>08): ARG * FAC -> FAC ----
    fmul_simple:            0x08, num(0x40, &[3], false),        num(0x40, &[4], false);
    fmul_digit_carry:       0x08, num(0x40, &[50], false),       num(0x40, &[50], false);
    fmul_multi_digit:       0x08, num(0x40, &[12, 34, 56], false), num(0x40, &[98, 76], false);
    fmul_signs:             0x08, num(0x40, &[7], true),         num(0x40, &[6], false);
    fmul_both_negative:     0x08, num(0x40, &[7], true),         num(0x40, &[6], true);
    fmul_fac_zero:          0x08, ZERO,                          num(0x40, &[6], false);
    fmul_arg_zero:          0x08, num(0x40, &[6], false),        ZERO;
    fmul_exponent_sum:      0x08, num(0x45, &[2], false),        num(0x43, &[3], false);
    fmul_exp_overflow:      0x08, num(0x7F, &[99], false),       num(0x7F, &[99], false);
    fmul_tiny_underflow:    0x08, num(0x01, &[1], false),        num(0x01, &[1], false);

    // ---- FDIV (>09): ARG / FAC -> FAC ----
    fdiv_simple:            0x09, num(0x40, &[4], false),        num(0x40, &[12], false);
    fdiv_repeating:         0x09, num(0x40, &[3], false),        num(0x40, &[1], false);
    fdiv_signs:             0x09, num(0x40, &[4], true),         num(0x40, &[12], false);
    fdiv_multi_digit:       0x09, num(0x40, &[12, 34], false),   num(0x41, &[56, 78, 90], false);
    fdiv_by_zero:           0x09, ZERO,                          num(0x40, &[5], false);
    fdiv_zero_dividend:     0x09, num(0x40, &[5], false),        ZERO;
    fdiv_exp_range:         0x09, num(0x30, &[2], false),        num(0x50, &[8], false);

    // ---- FCOMP (>0A): compare ARG vs FAC -> >837C ----
    fcomp_equal:            0x0A, num(0x40, &[42, 17], false),   num(0x40, &[42, 17], false);
    fcomp_arg_greater:      0x0A, num(0x40, &[10], false),       num(0x40, &[20], false);
    fcomp_fac_greater:      0x0A, num(0x40, &[20], false),       num(0x40, &[10], false);
    fcomp_signs:            0x0A, num(0x40, &[10], true),        num(0x40, &[10], false);
    fcomp_both_negative:    0x0A, num(0x40, &[10], true),        num(0x40, &[20], true);
    fcomp_zeroes:           0x0A, ZERO,                          ZERO;
    fcomp_exponent_only:    0x0A, num(0x41, &[1], false),        num(0x42, &[1], false);

    // ---- ROUND1 (>01) / ROUND (>02) / STST (>03) / OVEXP (>04) / OV (>05) ----
    round1_basic:           0x01, num(0x40, &[12, 34, 56, 78, 90, 12, 51], false), ZERO;
    round1_ripple:          0x01, num(0x40, &[99, 99, 99, 99, 99, 99, 99], false), ZERO;
    round_basic:            0x02, num(0x40, &[12, 34, 56, 78, 90, 12, 51], false), ZERO;
    round_negative:         0x02, num(0x40, &[12, 34, 56, 78, 90, 12, 51], true),  ZERO;
    stst_positive:          0x03, num(0x40, &[5], false),        ZERO;
    stst_negative:          0x03, num(0x40, &[5], true),         ZERO;
    stst_zero:              0x03, ZERO,                          ZERO;
    ovexp_case:             0x04, num(0x7F, &[99], false),       ZERO;
    ov_case:                0x05, num(0x40, &[5], false),        ZERO;
}

/// The S-forms pop ARG from the VDP value stack first (>836E). One planted
/// case per S-op; the pop protocol (pointer adjustment + the popped bytes)
/// is part of the compared state.
#[test]
fn s_forms_pop_the_value_stack() {
    for (name, op) in [
        ("sadd", 0x0Bu8),
        ("ssub", 0x0C),
        ("smul", 0x0D),
        ("sdiv", 0x0E),
        ("scomp", 0x0F),
    ] {
        diff_fp(
            name,
            &FpCase {
                fac: num(0x40, &[7, 50], false),
                arg: num(0x40, &[99, 99], true), // overwritten by the pop
                stack_top: Some(num(0x41, &[3, 25], false)),
                xml_op: op,
            },
        );
    }
}

/// CSN (XML >10): string -> float from a VDP text buffer at *>8356. The
/// text is planted at VDP >0200 followed by >2C (a terminator the grammar
/// rejects); the pointer cell, FAC, >8354/>8375/>8376, and the pointer's
/// advance are all in the compared pad.
fn run_csn(rom: &[u8], text: &[u8], op: u8, grom_text: bool) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let p = {
        let mut v = vec![0x0F, op];
        let at = 0x20 + v.len() as u16;
        v.extend_from_slice(&halt(at));
        v
    };
    let mut grom = vec![0u8; 0x6000];
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    if grom_text {
        // The GROM-text variant at GROM >0300. The conversion leaves the GPL
        // PC aimed at the terminator, so the HALT BYTES themselves terminate
        // the grammar (BR opcodes are '@'-'_' characters, outside it) AND
        // park the interpreter — a ',' here would execute as opcode >2C,
        // the documented unpinned garbage corner.
        grom[0x0300..0x0300 + text.len()].copy_from_slice(text);
        let h = halt(0x0300 + text.len() as u16);
        grom[0x0300 + text.len()..0x0304 + text.len()].copy_from_slice(&h);
    }
    let mut m = Machine::new(rom, &grom);
    m.reset();
    if grom_text {
        m.bus_mut().poke(0x8389, 0x01);
        m.bus_mut().poke(0x8356, 0x03);
        m.bus_mut().poke(0x8357, 0x00);
    } else {
        for (i, &b) in text.iter().enumerate() {
            m.vdp_mut().set_vram(0x0200 + i as u16, b);
        }
        m.vdp_mut().set_vram(0x0200 + text.len() as u16, 0x2C);
        m.bus_mut().poke(0x8389, 0x00);
        m.bus_mut().poke(0x8356, 0x02);
        m.bus_mut().poke(0x8357, 0x00);
    }
    for _ in 0..6 {
        m.run_frame();
    }
    let pad: Vec<u8> = (0x8300u16..0x83E0)
        .map(|a| {
            let b = m.bus().peek(a);
            match a {
                0x8300..=0x8307 => 0,
                0x8372 | 0x8373 => 0,
                0x837C => b & 0xF8,
                0x83DA..=0x83DF => 0,
                _ => b,
            }
        })
        .collect();
    let regs: Vec<u8> = (0..8).map(|r| m.vdp().register(r)).collect();
    let vram: Vec<u8> = (0x0380u16..0x0400).map(|a| m.vdp().vram(a)).collect();
    (pad, regs, vram)
}

fn diff_csn(name: &str, text: &[u8], op: u8, grom_text: bool) {
    let Some(auth_rom) = auth_rom() else { return };
    let a = run_csn(auth_rom, text, op, grom_text);
    let o = run_csn(our_rom(), text, op, grom_text);
    for i in 0..a.0.len() {
        assert_eq!(
            a.0[i],
            o.0[i],
            "{name} {:?}: scratchpad >{:04X} differs (authentic {:02X} vs ours {:02X})",
            String::from_utf8_lossy(text),
            0x8300 + i,
            a.0[i],
            o.0[i]
        );
    }
    assert_eq!(a.1, o.1, "{name}: VDP registers differ");
    assert_eq!(a.2, o.2, "{name}: VRAM differs");
}

#[test]
fn csn_parses_numbers_identically() {
    for text in [
        &b"123"[..],
        b"0.5",
        b"-12.34",
        b"+7",
        b"1E-99",
        b"1.5E2",
        b"9999999999999999", // 16 nines: the full round ripple
        b"0",
        b"-0",   // sign residue with a zero FAC
        b"00.00",
        b".",    // the no-pointer-update zero
        b"+.",
        b"1E",   // the FULL ABORT (nothing written but the sign)
        b"1E+",
        b"1E999",   // exponent overflow: saturation + >01
        b"1E-999",  // exponent underflow: silent zero
        b"1E32768", // the huge-literal path
        b"1E-32768",
        b"0.0005",
        b"12.",
        b"--1",
        b"2.5.7", // a second '.' terminates
        b"3,",
    ] {
        diff_csn("csn", text, 0x10, false);
    }
}

/// CSNGR (XML >11): the same body with the source switched by >8389 —
/// the VDP row (flag 0) and the GROM row (flag 1, where the text cursor IS
/// the GPL PC and the planted halt catches the post-conversion fetch).
#[test]
fn csngr_switches_sources_identically() {
    for text in [&b"42.5"[..], b"-1E3", b"0"] {
        diff_csn("csngr-vdp", text, 0x11, false);
        diff_csn("csngr-grom", text, 0x11, true);
    }
}

/// CFI (XML >12): float -> the signed 16-bit integer at >834A; round to
/// nearest, exact halves toward +infinity; overflow = >8354 := >03 with no
/// result stored. The agent-verified edge set.
#[test]
fn cfi_converts_identically() {
    for (name, fac) in [
        ("zero", ZERO),
        ("one", num(0x40, &[1], false)),
        ("neg_one", num(0x40, &[1], true)),
        ("half_up", num(0x3F, &[50], false)),           // +0.5 -> 1
        ("neg_half_tie", num(0x3F, &[50], true)),       // -0.5 -> 0
        ("neg_past_half", num(0x3F, &[70], true)),      // -0.7 -> -1
        ("tiny", num(0x3E, &[50], false)),              // 0.005 -> 0
        ("max", num(0x42, &[3, 27, 67], false)),        // 32767
        ("max_49", num(0x42, &[3, 27, 67, 49], false)), // 32767.49 -> 32767
        ("max_50", num(0x42, &[3, 27, 67, 50], false)), // 32767.5 -> err >03
        ("min", num(0x42, &[3, 27, 68], true)),         // -32768
        ("min_50", num(0x42, &[3, 27, 68, 50], true)),  // -32768.5 -> -32768
        ("min_51", num(0x42, &[3, 27, 68, 51], true)),  // -> err >03
        ("pos_32768", num(0x42, &[3, 27, 68], false)),  // +32768 -> err >03
        ("huge", num(0x50, &[1], false)),               // exp > >42 -> err
        ("hundred", num(0x41, &[1], false)),
        ("9999", num(0x41, &[99, 99], false)),
    ] {
        diff_fp(
            &format!("cfi_{name}"),
            &FpCase { fac, arg: ZERO, stack_top: None, xml_op: 0x12 },
        );
    }
}

/// The CSN text fuzz: random strings over the number-ish alphabet — valid
/// and garbage alike must parse identically.
#[test]
fn csn_text_fuzz() {
    const ALPHA: &[u8] = b"0123456789+-.E,X 5";
    let mut s: u64 = 0xF00D_FACE_0BAD_CAFE;
    let mut next = move || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    for i in 0..80 {
        let r = next();
        let len = 1 + (r % 10) as usize;
        let mut text = Vec::with_capacity(len);
        let mut v = next();
        for _ in 0..len {
            text.push(ALPHA[(v % ALPHA.len() as u64) as usize]);
            v /= ALPHA.len() as u64;
        }
        diff_csn(&format!("csn_fuzz[{i}]"), &text, 0x10, false);
    }
}

/// ROUND (XML >02) takes its digit position from >8354; the gates above
/// never plant it, so the position axis gets its own sweep. Positions that
/// keep the ripple inside the FP scratch are bit-exact; garbage positions
/// >= >96 walk the ripple through the LIVE GPLWS itself, where the outcome
/// depends on the interpreter's transient register file — three of those
/// walks (>AA/>AB/>B1, starting on the R10/R13 cells) diverge from the
/// authentic and are KEPT as a ledgered garbage corner (RECON §27, ROUNDH's
/// header comment). This test pins the whole contract: every other
/// position byte must match, and the three ledgered ones must still
/// diverge — if they start matching, the ledger is stale.
fn run_round_at(rom: &[u8], fac: [u8; 8], pos: u8) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let p = {
        let mut v = vec![0x0F, 0x02];
        let at = 0x20 + v.len() as u16;
        v.extend_from_slice(&halt(at));
        v
    };
    let mut grom = vec![0u8; 0x6000];
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    let mut m = Machine::new(rom, &grom);
    m.reset();
    for (i, &b) in fac.iter().enumerate() {
        m.bus_mut().poke(0x834A + i as u16, b);
    }
    m.bus_mut().poke(0x8354, pos);
    m.bus_mut().poke(0x836C, 0x00);
    m.bus_mut().poke(0x836D, 0x22);
    for _ in 0..6 {
        m.run_frame();
    }
    let pad: Vec<u8> = (0x8300u16..0x83E0)
        .map(|a| {
            let b = m.bus().peek(a);
            match a {
                0x8300..=0x8307 => 0,
                0x8372 | 0x8373 => 0,
                0x837C => b & 0xF8,
                0x83DA..=0x83DF => 0,
                _ => b,
            }
        })
        .collect();
    let regs: Vec<u8> = (0..8).map(|r| m.vdp().register(r)).collect();
    let vram: Vec<u8> = (0x0380u16..0x0400).map(|a| m.vdp().vram(a)).collect();
    (pad, regs, vram)
}

#[test]
fn round_position_sweep_pins_the_contract() {
    let Some(auth_rom) = auth_rom() else { return };
    const LEDGERED: [u8; 3] = [0xAA, 0xAB, 0xB1];
    let sweep_facs = [
        num(0x40, &[12, 34, 56, 78, 90, 12, 51], false),
        num(0x40, &[99, 99, 99, 99, 99, 99, 99], false),
    ];
    let trip_facs = [
        num(0x40, &[12, 34, 56, 78, 90, 12, 51], false),
        num(0x40, &[99, 99, 99, 99, 99, 99, 99], false),
        num(0x45, &[1], true),
        num(0x40, &[50, 49, 99, 0, 99, 99, 98], false),
    ];
    for pos in 0..=255u16 {
        let pos = pos as u8;
        if LEDGERED.contains(&pos) {
            let diverges = trip_facs
                .iter()
                .any(|fac| run_round_at(auth_rom, *fac, pos) != run_round_at(our_rom(), *fac, pos));
            assert!(
                diverges,
                "ROUND at position >{pos:02X} is ledgered as divergent but now matches — \
                 retire the ledger entry (ROUNDH comment + RECON §27) and pin it clean"
            );
        } else {
            for fac in &sweep_facs {
                let a = run_round_at(auth_rom, *fac, pos);
                let o = run_round_at(our_rom(), *fac, pos);
                assert_eq!(
                    a, o,
                    "ROUND at position >{pos:02X} diverged (fac={fac:02X?}) — \
                     outside the ledgered >AA/>AB/>B1 garbage corner"
                );
            }
        }
    }
}

/// The FP fuzz: deterministic random FAC/ARG patterns — valid-ish and raw
/// garbage alike — across the arithmetic ops. Both ROMs must agree on
/// everything, error paths included.
#[test]
fn fp_fuzz_fast() {
    let mut s: u64 = 0x00DD_BA11_CAFE_F00D;
    let mut next = move || {
        s = s.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = s;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    for i in 0..160 {
        let r = next();
        let op = [0x06u8, 0x07, 0x08, 0x09, 0x0A, 0x01, 0x02][(r % 7) as usize];
        let mut fac = [0u8; 8];
        let mut arg = [0u8; 8];
        let (a, b) = (next(), next());
        for j in 0..8 {
            fac[j] = (a >> (8 * j)) as u8;
            arg[j] = (b >> (8 * j)) as u8;
        }
        if r & 0x10 != 0 {
            // Half the corpus: plausibly valid numbers (biased exponents,
            // 0-99 digits) so the deep arithmetic paths get real traffic.
            fac[0] = 0x38 + (fac[0] & 0x0F);
            arg[0] = 0x38 + (arg[0] & 0x0F);
            for j in 1..8 {
                fac[j] %= 100;
                arg[j] %= 100;
            }
        }
        diff_fp(&format!("fp_fuzz[{i}] op>{op:02X}"), &FpCase { fac, arg, stack_top: None, xml_op: op });
    }
}
