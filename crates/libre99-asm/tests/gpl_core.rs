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

//! **The M1 differential microsuite** — the P9 verification engine: every GPL
//! program below runs under the AUTHENTIC console ROM and under OUR rewrite,
//! from identical machine state, and the full observable state must match:
//! scratchpad `>8300-83DF` (the GPLWS `>83E0-83FF` is interpreter-internal by
//! design), the VDP registers, and a VRAM window. The authentic ROM is the
//! oracle; no cartridge is needed to drive any element (plan P9).
//!
//! Programs are hand-encoded GPL bytes (RECON §3's encoding, independently of
//! the libre99-gpl assembler), placed at the boot entry `>0020`, ending in a
//! self-loop. The status byte `>837C` is compared with its low three bits
//! masked (unspecified interpreter-internal bits — RECON §16); everything
//! else is compared exactly.

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

/// Run `prog` at GROM `>0020` under `rom` for `frames`; return the machine.
fn run(rom: &[u8], prog: &[u8], frames: usize) -> Machine {
    let mut grom = vec![0u8; 0x6000];
    grom[0x20..0x20 + prog.len()].copy_from_slice(prog);
    let mut m = Machine::new(rom, &grom);
    m.reset();
    for _ in 0..frames {
        m.run_frame();
    }
    m
}

/// The observable state: scratchpad >8300-83DF, the 8 VDP registers, and
/// VRAM >0000-04FF. Two masks (documented deviations, RECON §16): >837C's
/// low three bits are interpreter-internal, and >8300-8307 are the
/// documented interpreter temporaries where the authentic operand engine
/// leaves scratch residue (programs own them only when they write them —
/// keep test data at >8340+).
fn snapshot(m: &Machine) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let pad: Vec<u8> = (0x8300u16..0x83E0)
        .map(|a| {
            let b = m.bus().peek(a);
            match a {
                0x8300..=0x8307 => 0,
                // The GPL data/sub-stack pointer bytes carry the same engine
                // residue between uses; the stack tests copy them into
                // compared cells to keep the *behaviour* verified.
                0x8372 | 0x8373 => 0,
                0x837C => b & 0xF8,
                _ => b,
            }
        })
        .collect();
    let regs: Vec<u8> = (0..8).map(|r| m.vdp().register(r)).collect();
    let vram: Vec<u8> = (0x0000u16..0x0500).map(|a| m.vdp().vram(a)).collect();
    (pad, regs, vram)
}

/// A condition-safe terminator at GROM address `a`: a first `BR` to the
/// second, then a `BR` to itself. If the condition bit is SET the first BR
/// falls through (consuming the bit — BR/BS always reset it) straight into
/// the self-loop; if CLEAR it branches there. Either way the program parks.
/// (A bare `BR $` is NOT a self-loop when the condition bit is set!)
fn halt(a: u16) -> [u8; 4] {
    let n = a + 2;
    let hi = 0x40 | ((n >> 8) as u8 & 0x1F);
    [hi, n as u8, hi, n as u8]
}

/// Append the terminator to a program.
fn prog(mut v: Vec<u8>) -> Vec<u8> {
    let addr = 0x20 + v.len() as u16;
    v.extend_from_slice(&halt(addr));
    v
}

/// Run one case under both ROMs and diff the observables.
fn diff_case(name: &str, p: &[u8]) {
    let Some(auth_rom) = auth_rom() else { return };
    let (a_pad, a_regs, a_vram) = snapshot(&run(auth_rom, p, 10));
    let (o_pad, o_regs, o_vram) = snapshot(&run(our_rom(), p, 10));
    for i in 0..a_pad.len() {
        assert_eq!(
            a_pad[i],
            o_pad[i],
            "{name}: scratchpad >{:04X} differs (authentic {:02X} vs ours {:02X})",
            0x8300 + i,
            a_pad[i],
            o_pad[i]
        );
    }
    assert_eq!(a_regs, o_regs, "{name}: VDP registers differ");
    for i in 0..a_vram.len() {
        assert_eq!(
            a_vram[i], o_vram[i],
            "{name}: VRAM >{i:04X} differs (authentic {:02X} vs ours {:02X})",
            a_vram[i], o_vram[i]
        );
    }
}

macro_rules! cases {
    ($($test:ident : $bytes:expr;)*) => {
        $(
            #[test]
            fn $test() {
                diff_case(stringify!($test), &prog($bytes));
            }
        )*
    };
}

cases! {
    // ---- ST family: immediate + memory + word, CPU space ----
    st_imm_byte: vec![0xBE, 0x40, 0x5A];
    st_imm_word: vec![0xBF, 0x42, 0x12, 0x34];
    st_mem_byte: vec![0xBE, 0x40, 0x77, 0xBC, 0x41, 0x40];
    st_mem_word: vec![0xBF, 0x40, 0xAB, 0xCD, 0xBD, 0x44, 0x40];
    // 12-bit and 16-bit GAS forms (dest >83CE via the long forms).
    st_gas_12bit: vec![0xBE, 0x80, 0xCE, 0x69];
    st_gas_16bit: vec![0xBE, 0x8F, 0x00, 0xCC, 0x42];
    // CPU indirect: pointer byte at >8356 -> final >8341. (`*@>8356` is the
    // long GAS form with the I bit: `90 56`.)
    st_cpu_indirect: vec![0xBE, 0x56, 0x41, 0xBE, 0x90, 0x56, 0x5A];

    // ---- VDP space: direct write, readback, word, indirect ----
    vdp_write_byte: vec![0xBE, 0xA4, 0x00, 0x3C];
    vdp_write_word: vec![0xBF, 0xA4, 0x10, 0x11, 0x22];
    vdp_readback: vec![0xBE, 0xA4, 0x00, 0x3C, 0xBC, 0x40, 0xA4, 0x00];
    vdp_indirect_read: vec![
        0xBE, 0xA4, 0x50, 0x77,             // ST V@>0450,>77
        0xBF, 0x46, 0x04, 0x50,             // DST @>8346,>0450 (the pointer)
        0xBC, 0x40, 0xB0, 0x46,             // ST @>8340,*V@>8346
    ];

    // ---- arithmetic + status ----
    add_byte: vec![0xBE, 0x40, 0x40, 0xA2, 0x40, 0x40];
    add_byte_carry: vec![0xBE, 0x40, 0xFF, 0xA2, 0x40, 0x01];
    add_word: vec![0xBF, 0x40, 0x11, 0x11, 0xA3, 0x40, 0x22, 0x22];
    add_word_ovf: vec![0xBF, 0x40, 0x7F, 0xFF, 0xA3, 0x40, 0x00, 0x01];
    add_word_carry_zero: vec![0xBF, 0x40, 0xFF, 0xFF, 0xA3, 0x40, 0x00, 0x01];
    sub_word: vec![0xBF, 0x40, 0x22, 0x22, 0xA7, 0x40, 0x11, 0x11];
    sub_word_borrow: vec![0xBF, 0x40, 0x11, 0x11, 0xA7, 0x40, 0x22, 0x22];
    add_mem_word: vec![0xBF, 0x40, 0x00, 0x05, 0xBF, 0x42, 0x00, 0x03, 0xA1, 0x40, 0x42];

    // ---- logic + status (C/OV preservation) ----
    and_after_carry: vec![
        0xBF, 0x42, 0x80, 0x00, 0xA3, 0x42, 0x80, 0x00, // leave C+OV
        0xBF, 0x40, 0xF0, 0xF0, 0xB3, 0x40, 0xFF, 0x00, // DAND
    ];
    or_zero: vec![0xB7, 0x40, 0x00, 0x00];
    xor_word: vec![0xBF, 0x40, 0xAA, 0xAA, 0xBB, 0x40, 0xAA, 0xAA];
    and_byte: vec![0xBE, 0x40, 0xF0, 0xB2, 0x40, 0x33];
    or_byte: vec![0xBE, 0x40, 0x01, 0xB6, 0x40, 0x02];
    xor_byte: vec![0xBE, 0x40, 0xFF, 0xBA, 0x40, 0x0F];

    // ---- unaries ----
    abs_byte_neg: vec![0xBE, 0x40, 0x9C, 0x80, 0x40];
    abs_byte_min: vec![0xBE, 0x40, 0x80, 0x80, 0x40];
    neg_word: vec![0xBF, 0x40, 0x12, 0x34, 0x83, 0x40];
    inv_byte: vec![0xBE, 0x40, 0xF0, 0x84, 0x40];
    clr_word_after_val: vec![0xBF, 0x40, 0xAB, 0xCD, 0x87, 0x40];
    inc_byte: vec![0xBE, 0x40, 0x7F, 0x90, 0x40];
    dinc_word: vec![0xBF, 0x40, 0xFF, 0xFF, 0x91, 0x40];
    dec_byte: vec![0xBE, 0x40, 0x00, 0x92, 0x40];
    inct_word: vec![0xBF, 0x40, 0x7F, 0xFF, 0x95, 0x40];
    dect_word: vec![0xBF, 0x40, 0x00, 0x01, 0x97, 0x40];

    // ---- compares (cond + full-replace semantics) ----
    ceq_true: vec![0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5A, 0xBC, 0x5E, 0x7C];
    ceq_false_after_carry: vec![
        0xBF, 0x42, 0xFF, 0xFF, 0xA3, 0x42, 0x00, 0x01,
        0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5B, 0xBC, 0x5E, 0x7C,
    ];
    ch_true: vec![0xBE, 0x40, 0x80, 0xC6, 0x40, 0x7F, 0xBC, 0x5E, 0x7C];
    ch_false: vec![0xBE, 0x40, 0x7F, 0xC6, 0x40, 0x80, 0xBC, 0x5E, 0x7C];
    cgt_neg_vs_pos: vec![0xBE, 0x40, 0x80, 0xCE, 0x40, 0x7F, 0xBC, 0x5E, 0x7C];
    cgt_true: vec![0xBE, 0x40, 0x7F, 0xCE, 0x40, 0x80, 0xBC, 0x5E, 0x7C];
    che_equal: vec![0xBE, 0x40, 0x7F, 0xCA, 0x40, 0x7F, 0xBC, 0x5E, 0x7C];
    cge_word: vec![0xBF, 0x40, 0x00, 0x05, 0xD3, 0x40, 0x00, 0x05, 0xBC, 0x5E, 0x7C];
    clog_zero: vec![0xBE, 0x40, 0xF0, 0xDA, 0x40, 0x0F, 0xBC, 0x5E, 0x7C];
    clog_nonzero: vec![0xBE, 0x40, 0xF0, 0xDA, 0x40, 0x30, 0xBC, 0x5E, 0x7C];
    cz_zero: vec![0x8E, 0x40, 0xBC, 0x5E, 0x7C];
    cz_nonzero: vec![0xBE, 0x40, 0x5A, 0x8E, 0x40, 0xBC, 0x5E, 0x7C];
    dcz_nonzero: vec![0xBF, 0x40, 0x00, 0x01, 0x8F, 0x40, 0xBC, 0x5E, 0x7C];

    // ---- MUL / DIV ----
    mul_byte: vec![0xBE, 0x40, 0x07, 0xAA, 0x40, 0x06];
    dmul_word: vec![0xBF, 0x40, 0x12, 0x34, 0xAB, 0x40, 0x01, 0x00];
    div_byte: vec![0xBF, 0x40, 0x00, 0x2F, 0xAE, 0x40, 0x05];
    ddiv_word: vec![0xBF, 0x40, 0x00, 0x00, 0xBF, 0x42, 0x2F, 0x00, 0xAF, 0x40, 0x01, 0x00];
    ddiv_by_zero: vec![0xBF, 0x40, 0x00, 0x00, 0xBF, 0x42, 0x00, 0x2F, 0xAF, 0x40, 0x00, 0x00];

    // ---- shifts ----
    dsra: vec![0xBF, 0x40, 0x80, 0x10, 0xDF, 0x40, 0x00, 0x01];
    dsll: vec![0xBF, 0x40, 0x01, 0x01, 0xE3, 0x40, 0x00, 0x04];
    srl_byte: vec![0xBE, 0x40, 0x81, 0xE6, 0x40, 0x01];
    dsrc: vec![0xBF, 0x40, 0x80, 0x01, 0xEB, 0x40, 0x00, 0x01];
    dsll_count0: vec![0xBF, 0x40, 0x01, 0x01, 0xE3, 0x40, 0x00, 0x00];
    src_byte: vec![0xBE, 0x40, 0x81, 0xEA, 0x40, 0x01];
    sra_byte: vec![0xBE, 0x40, 0x81, 0xDE, 0x40, 0x01];
    shift_count_mem: vec![0xBE, 0x44, 0x03, 0xBF, 0x40, 0x01, 0x01, 0xE1, 0x40, 0x44];

    // ---- EX ----
    dex_words: vec![0xBF, 0x40, 0x11, 0x22, 0xBF, 0x42, 0x33, 0x44, 0xC1, 0x40, 0x42];
    ex_bytes: vec![0xBE, 0x40, 0xAA, 0xBE, 0x41, 0xBB, 0xC0, 0x40, 0x41];

    // ---- RAND (deterministic from the zeroed seed) ----
    rand_ff: vec![0x02, 0xFF, 0xBC, 0x5E, 0x7C];
    rand_07_twice: vec![0x02, 0x07, 0xBC, 0x50, 0x78, 0x02, 0x07, 0xBC, 0x51, 0x78];

    // ---- H / GT / CARRY / OVF ----
    carry_op: vec![0xBF, 0x40, 0xFF, 0xFF, 0xA3, 0x40, 0x00, 0x01, 0x0C, 0xBC, 0x5E, 0x7C];
    ovf_op: vec![0xBF, 0x40, 0x7F, 0xFF, 0xA3, 0x40, 0x00, 0x01, 0x0D, 0xBC, 0x5E, 0x7C];
    h_op: vec![0xBF, 0x40, 0x00, 0x01, 0xA3, 0x40, 0x00, 0x01, 0x09, 0xBC, 0x5E, 0x7C];
    gt_op: vec![0xBF, 0x40, 0x00, 0x01, 0xA3, 0x40, 0x00, 0x01, 0x0A, 0xBC, 0x5E, 0x7C];

    // ---- BACK ----
    back_color: vec![0x04, 0x0C];

    // ---- ALL: fill the 768-cell name table (VDP >0000..>02FF) ----
    all_fills_screen: vec![0x07, 0x2A];
    all_clears_screen: vec![0x07, 0x00];
}

// ---- control flow (hand-built layouts, not simple linear programs) ----

/// Place `bytes` at program offset `off` (GROM `>0020 + off`).
fn place(p: &mut [u8], off: usize, bytes: &[u8]) {
    p[off..off + bytes.len()].copy_from_slice(bytes);
}

/// Place a marker (`ST @cell,val`) followed by a terminator at `off`.
fn mark_halt(p: &mut [u8], off: usize, cell: u8, val: u8) {
    place(p, off, &[0xBE, cell, val]);
    let a = 0x20 + off as u16 + 3;
    place(p, off + 3, &halt(a));
}

#[test]
fn bs_taken_and_cond_consumed() {
    // CEQ-true; BS >0030 (taken); at >0030: BS >0040 (must NOT be taken —
    // the first BS consumed the bit); markers tell the path.
    let mut p = vec![0u8; 0x40];
    place(&mut p, 0, &[0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5A, 0x60, 0x30]);
    mark_halt(&mut p, 8, 0x50, 0x01); // fall-through: must not run
    place(&mut p, 0x10, &[0x60, 0x40]); // at >0030: BS >0040
    mark_halt(&mut p, 0x12, 0x51, 0x02); // must run (bit consumed)
    mark_halt(&mut p, 0x20, 0x52, 0x03); // at >0040: must not run
    diff_case("bs_taken_and_cond_consumed", &p);
}

#[test]
fn br_taken_when_cond_clear() {
    let mut p = vec![0u8; 0x30];
    place(&mut p, 0, &[0x40, 0x30]); // BR >0030 (cond clear -> taken)
    mark_halt(&mut p, 2, 0x50, 0x01); // must not run
    mark_halt(&mut p, 0x10, 0x51, 0x02); // at >0030: must run
    diff_case("br_taken_when_cond_clear", &p);
}

#[test]
fn b_absolute() {
    let mut p = vec![0u8; 0x40];
    place(&mut p, 0, &[0x05, 0x00, 0x50]); // B >0050
    mark_halt(&mut p, 3, 0x50, 0x01); // must be skipped
    mark_halt(&mut p, 0x30, 0x51, 0x02); // at >0050: must run
    diff_case("b_absolute", &p);
}

#[test]
fn call_rtn_nested() {
    // Two-level CALL with markers at each stage + the stack cells compared.
    let mut p = vec![0u8; 0x60];
    place(
        &mut p,
        0,
        &[
            0xBE, 0x73, 0x7E, // ST @>8373,>7E
            0x06, 0x00, 0x40, // CALL >0040
        ],
    );
    mark_halt(&mut p, 6, 0x50, 0x01); // back at top level
    place(
        &mut p,
        0x20, // GROM >0040
        &[
            0xBE, 0x51, 0x02, // marker: level 1
            0x06, 0x00, 0x50, // CALL >0050
            0xBE, 0x52, 0x03, // marker: level 1 after return
            0x00, // RTN
        ],
    );
    place(&mut p, 0x30, &[0xBE, 0x53, 0x04, 0x00]); // >0050: marker; RTN
    // Copy the (masked) sub-stack pointer to a compared cell after the
    // balanced call chain — the pointer must be back at >7E on both ROMs.
    place(&mut p, 6, &[0xBC, 0x44, 0x73]);
    mark_halt(&mut p, 9, 0x50, 0x01);
    diff_case("call_rtn_nested", &p);
}

#[test]
fn rtnc_preserves_what_rtn_clears() {
    // Identical programs except RTN vs RTNC after an in-sub CEQ-true; the
    // captured status after return must match the authentic ROM's behaviour.
    for (name, ret) in [("rtn", 0x00u8), ("rtnc", 0x01u8)] {
        let mut p = vec![0u8; 0x40];
        place(
            &mut p,
            0,
            &[
                0xBE, 0x73, 0x7E,
                0x06, 0x00, 0x40, // CALL >0040 (resets cond)
                0xBC, 0x5E, 0x7C, // capture status after return
            ],
        );
        place(&mut p, 9, &halt(0x29));
        place(
            &mut p,
            0x20,
            &[
                0xBE, 0x41, 0x5A, 0xD6, 0x41, 0x5A, // cond set inside the sub
                ret,
            ],
        );
        diff_case(name, &p);
    }
}

#[test]
fn fetch_inline_data() {
    let mut p = vec![0u8; 0x40];
    place(
        &mut p,
        0,
        &[
            0xBE, 0x73, 0x7E,
            0x06, 0x00, 0x40, // CALL >0040
            0xC3, // inline data byte
        ],
    );
    mark_halt(&mut p, 7, 0x50, 0x01); // resumes here (past the data)
    place(&mut p, 0x20, &[0x88, 0x54, 0x00]); // FETCH @>8354 ; RTN
    diff_case("fetch_inline_data", &p);
}

#[test]
fn case_dispatch() {
    let mut p = vec![0u8; 0x40];
    place(
        &mut p,
        0,
        &[
            0xBE, 0x40, 0x01, // ST @>8340,>01
            0x8A, 0x40, // CASE @>8340
            0x40, 0x30, // entry 0 -> >0030
            0x40, 0x38, // entry 1 -> >0038  <- taken
        ],
    );
    mark_halt(&mut p, 0x10, 0x50, 0x0A); // >0030: wrong path
    mark_halt(&mut p, 0x18, 0x50, 0x0B); // >0038: right path
    diff_case("case_dispatch", &p);
}

#[test]
fn push_stack() {
    // The trailing `ST @>8344,@>8372` copies the (masked) stack pointer into
    // a compared cell, keeping the pointer behaviour verified.
    diff_case(
        "push_stack",
        &prog(vec![0xBE, 0x72, 0x60, 0xBE, 0x40, 0x5A, 0x8C, 0x40, 0xBC, 0x44, 0x72]),
    );
    diff_case(
        "dpush_stack",
        &prog(vec![0xBE, 0x72, 0x60, 0xBF, 0x40, 0x11, 0x22, 0x8D, 0x40, 0xBC, 0x44, 0x72]),
    );
}

#[test]
fn exit_reboots() {
    // EXIT re-enters the reset path: the GROM boot entry runs again; with a
    // marker before EXIT both ROMs should loop identically (the marker cell
    // simply ends up set, and the machine keeps re-running the program).
    diff_case("exit_reboots", &prog(vec![0xBE, 0x40, 0x01, 0x0B]));
}

// ---- MOVE (>20-3F) ---------------------------------------------------------
// Stream layout (execution-pinned in libre99-gpl's m2_probe/move_probe): opcode,
// count, destination, source. These programs sometimes need GROM source bytes
// placed away from the >0020 program, so `diff_move` seeds the GROM directly.

/// `prog(program)` at GROM >0020 plus raw `data` bytes at fixed GROM addresses;
/// run under both ROMs and diff the observables.
fn diff_move(name: &str, program: Vec<u8>, data: &[(u16, &[u8])]) {
    let mut grom = vec![0u8; 0x6000];
    let p = prog(program);
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    for (addr, bytes) in data {
        let a = *addr as usize;
        grom[a..a + bytes.len()].copy_from_slice(bytes);
    }
    let Some(auth_rom) = auth_rom() else { return };
    let go = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..10 {
            m.run_frame();
        }
        m
    };
    let (a_pad, a_regs, a_vram) = snapshot(&go(auth_rom));
    let (o_pad, o_regs, o_vram) = snapshot(&go(our_rom()));
    for i in 0..a_pad.len() {
        assert_eq!(
            a_pad[i],
            o_pad[i],
            "{name}: scratchpad >{:04X} differs (authentic {:02X} vs ours {:02X})",
            0x8300 + i,
            a_pad[i],
            o_pad[i]
        );
    }
    assert_eq!(a_regs, o_regs, "{name}: VDP registers differ");
    for i in 0..a_vram.len() {
        assert_eq!(
            a_vram[i], o_vram[i],
            "{name}: VRAM >{i:04X} differs (authentic {:02X} vs ours {:02X})",
            a_vram[i], o_vram[i]
        );
    }
}

#[test]
fn xml_srom_no_card_returns_not_found() {
    // XML >19 (SROM) on a bare console exercises the table-of-tables dispatch
    // (master[1]=XTAB, XTAB[9]=SROM); with no cards both ROMs scan, find no >AA
    // header, and leave >83D0 = 0 with the condition bit clear (captured to
    // >835E). DCLR @>83D0 ; ST @>836D,>04 ; XML >19 ; ST @>835E,@>837C.
    diff_case(
        "xml_srom_no_card",
        &prog(vec![0x87, 0x80, 0xD0, 0xBE, 0x6D, 0x04, 0x0F, 0x19, 0xBC, 0x5E, 0x7C]),
    );
}

#[test]
fn move_grom_to_cpu() {
    // MOVE >0004,@>8340,G>0100 (opcode >31: GROM src, CPU dst, imm count).
    diff_move(
        "move_grom_to_cpu",
        vec![0x31, 0x00, 0x04, 0x40, 0x01, 0x00],
        &[(0x0100, &[0xDE, 0xAD, 0xBE, 0xEF])],
    );
}

#[test]
fn move_computed_grom_to_cpu() {
    // MOVE with C=1 (computed-GROM source, opcode >33): the GROM source is a
    // 16-bit inline base (>0100) PLUS an indexed offset — the index selector
    // byte (>62) names cell >8362 (planted = word >0002), so the effective
    // source is G>0102. Copy 4 bytes to CPU >8340. This is the ToD LOAD-path
    // form (M4) and it shares the >8300-indexed-word mechanism with indexed GAS.
    diff_move(
        "move_computed_grom_to_cpu",
        vec![
            0xBF, 0x62, 0x00, 0x02, // ST word >8362 := >0002 (the index)
            0x33, 0x00, 0x04, 0x40, 0x01, 0x00, 0x62, // MOVE >0004,@>8340,C G>0100(idx@>8362)
        ],
        &[(0x0102, &[0xDE, 0xAD, 0xBE, 0xEF])],
    );
}

#[test]
fn indexed_gas_destination() {
    // Indexed GAS (the long-form X bit): a ST byte-immediate to an indexed
    // destination. The long operand >C0,>40 = CPU base >8340 with X set; the
    // index selector >62 names cell >8362 (planted = word >0004), so the store
    // lands at >8340+4 = >8344, not >8340. Exercises OPGIDX via OPGET (M4).
    diff_move(
        "indexed_gas_destination",
        vec![
            0xBF, 0x62, 0x00, 0x04, // ST word >8362 := >0004 (the index)
            0xBE, 0xC0, 0x40, 0x62, 0xAB, // ST >AB -> [X]@>8340(idx@>8362) = >8344
        ],
        &[],
    );
}

#[test]
fn move_cpu_to_gram() {
    // MOVE with G=0 (GRAM destination, opcode >25): CPU source, a 16-bit inline
    // GRAM address dest. The emulator's mask ROMs no-op the >9C00 GRAM write
    // (no GRAM chips), so this pins the *control flow* — the GRAM dest drives
    // the shared GROM address counter, so the interpreter's fetch position must
    // be saved/restored around the copy exactly as authentic does, or the next
    // opcode diverges. Copy 2 bytes from CPU >8340 to GRAM >6000 (M4).
    diff_move(
        "move_cpu_to_gram",
        vec![
            0xBE, 0x40, 0xAA, // ST >8340 := >AA
            0xBE, 0x41, 0xBB, // ST >8341 := >BB
            0x25, 0x00, 0x02, 0x60, 0x00, 0x40, // MOVE >0002,GRAM>6000,@>8340
        ],
        &[],
    );
}

#[test]
fn move_grom_to_gram() {
    // MOVE >20 (GRAM dest, GROM immediate source, count-from-memory) — the ToD
    // LOAD-path form. Both source and destination drive the shared GROM address
    // counter, so the per-byte re-addressing must alternate correctly and the
    // interpreter position be restored. The >9C00 GRAM write is an emulator
    // no-op, so this pins the control flow. count@>8360, G>6000 <- G>0100.
    diff_move(
        "move_grom_to_gram",
        vec![
            0xBF, 0x60, 0x00, 0x02, // ST word >8360 := >0002 (the count)
            0x20, 0x60, 0x60, 0x00, 0x01, 0x00, // MOVE cnt@>8360,GRAM>6000,G>0100
        ],
        &[(0x0100, &[0x11, 0x22])],
    );
}

#[test]
fn move_grom_to_vdp() {
    // MOVE >0004,V@>0010,G>0100 (>31: GROM src, VDP dst, imm count).
    diff_move(
        "move_grom_to_vdp",
        vec![0x31, 0x00, 0x04, 0xA0, 0x10, 0x01, 0x00],
        &[(0x0100, &[0x11, 0x22, 0x33, 0x44])],
    );
}

#[test]
fn move_grom_to_vdp_registers() {
    // MOVE >0008,#0,G>0100 (>39: the boot register-setup form).
    diff_move(
        "move_grom_to_vdp_registers",
        vec![0x39, 0x00, 0x08, 0x00, 0x01, 0x00],
        &[(0x0100, &[0x00, 0x80, 0xF0, 0x0E, 0xF9, 0x86, 0xF8, 0xF7])],
    );
}

#[test]
fn move_cpu_cascade() {
    // ST @>8348,>AB ; MOVE >0003,@>8349,@>8348 (>35: the CPU->CPU ascending
    // cascade fill — >8348..>834B all become >AB).
    diff_move("move_cpu_cascade", vec![0xBE, 0x48, 0xAB, 0x35, 0x00, 0x03, 0x49, 0x48], &[]);
}

#[test]
fn move_vdp_overlap_fill() {
    // ST V@>0100,>5A ; MOVE >0004,V@>0101,V@>0100 (>35 VDP->VDP overlap fill,
    // the title's `MOVE >03FF,V@>0B00,V@>0B01` idiom in miniature).
    diff_move(
        "move_vdp_overlap_fill",
        vec![0xBE, 0xA1, 0x00, 0x5A, 0x35, 0x00, 0x04, 0xA1, 0x01, 0xA1, 0x00],
        &[],
    );
}

#[test]
fn move_vdp_to_cpu() {
    // ST V@>0010,>77 ; ST V@>0011,>88 ; MOVE >0002,@>8360,V@>0010 (>35).
    diff_move(
        "move_vdp_to_cpu",
        vec![0xBE, 0xA0, 0x10, 0x77, 0xBE, 0xA0, 0x11, 0x88, 0x35, 0x00, 0x02, 0x60, 0xA0, 0x10],
        &[],
    );
}

#[test]
fn io_cru_output_arms_vdp_interrupt() {
    // The console boot's ISR-arming sequence (RECON shared §11): a four-field
    // CRU-output list + `IO @>8302,#3` must SBO CRU bit 2 (the 9901 VDP
    // interrupt mask). Verified functionally via the 9901 mask, not scratchpad,
    // to sidestep the still-stubbed ISR. Both ROMs must end with bit 2 set.
    //   ST  @>8300,>FF     data byte (low bit 1 -> SBO)
    //   DST @>8302,>0002   CRU bit address = 2
    //   DST @>8304,>0100   count 1, data-address byte >00 (data at >8300)
    //   IO  @>8302,#3      function 3 = CRU output
    let program = vec![
        0xBE, 0x00, 0xFF, 0xBF, 0x02, 0x00, 0x02, 0xBF, 0x04, 0x01, 0x00, 0xF6, 0x02, 0x03,
    ];
    let Some(auth_rom) = auth_rom() else { return };
    let mut grom = vec![0u8; 0x6000];
    let p = prog(program);
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    let run3 = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        // A few frames: enough to boot and run the IO; the mask persists even
        // once the first post-arm interrupt fires.
        for _ in 0..3 {
            m.run_frame();
        }
        m.bus().tms9901.int_mask()
    };
    let authentic = run3(auth_rom);
    assert_eq!(authentic, 0x0004, "authentic IO must arm 9901 mask bit 2");
    assert_eq!(run3(our_rom()), authentic, "our IO must arm the VDP interrupt like authentic");
}

#[test]
fn isr_advances_timer_and_timeout() {
    // Enable VDP interrupts (reg1 := >E0) and arm the 9901 (CRU bit 2), then
    // idle 12 frames: the ISR runs each vblank. With no sound list, no sprites,
    // and no key, its SPEED timer >8379 and screen-timeout >83D6 (and the VDP
    // status copy >837B) must advance exactly like the authentic ROM's idle ISR,
    // which does the same timer/timeout/status/CLR-R8 duties. (The RTWP frame
    // >83DA-DF holds each interpreter's own interrupted PC, so it is not compared.)
    let program = vec![
        0x39, 0x00, 0x01, 0x01, 0x01, 0x00, // MOVE >0001,#1,G>0100  (reg1 := >E0)
        0xBE, 0x00, 0xFF, // ST  @>8300,>FF
        0xBF, 0x02, 0x00, 0x02, // DST @>8302,>0002
        0xBF, 0x04, 0x01, 0x00, // DST @>8304,>0100
        0xF6, 0x02, 0x03, // IO  @>8302,#3
    ];
    let Some(auth_rom) = auth_rom() else { return };
    let mut grom = vec![0u8; 0x6000];
    let p = prog(program);
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    grom[0x0100] = 0xE0; // the reg1 value (16K + display + interrupt enable)
    let run12 = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..12 {
            m.run_frame();
        }
        m
    };
    let a = run12(auth_rom);
    let o = run12(our_rom());
    // The interrupt actually fired (timer advanced), and it matches authentic.
    let a_timer = a.bus().peek(0x8379);
    assert_ne!(a_timer, 0, "sanity: the authentic ISR advanced the timer");
    assert_eq!(o.bus().peek(0x8379), a_timer, "SPEED timer >8379");
    assert_eq!(
        o.bus().peek_word(0x83D6),
        a.bus().peek_word(0x83D6),
        "screen-timeout counter >83D6"
    );
    assert_eq!(o.bus().peek(0x837B), a.bus().peek(0x837B), "VDP status copy >837B");
}

#[test]
fn isr_sound_list_drains_to_the_chip_like_authentic() {
    let Some(auth_rom) = auth_rom() else { return };
    // Arm the VDP interrupt (as isr_advances does), plant a two-block GROM sound
    // list, then point >83CC/D at it with the countdown >83CE = 1 and idle: each
    // vblank the ISR's sound duty drains the next block's bytes to the sound chip
    // >8400 and reloads the countdown with the block's duration, until the D=0
    // block ends the list. The pointer bookkeeping (>83CC/D, >83CE) AND the
    // resulting SN76489 register state must match the authentic engine exactly.
    let program = vec![
        0x39, 0x00, 0x01, 0x01, 0x01, 0x00, // MOVE >0001,#1,G>0100  (reg1 := >E0)
        0xBE, 0x00, 0xFF, // ST  @>8300,>FF
        0xBF, 0x02, 0x00, 0x02, // DST @>8302,>0002
        0xBF, 0x04, 0x01, 0x00, // DST @>8304,>0100
        0xF6, 0x02, 0x03, // IO  @>8302,#3   (arm the 9901 VDP interrupt)
    ];
    let mut grom = vec![0u8; 0x6000];
    let p = prog(program);
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    grom[0x0100] = 0xE0; // reg1 value (16K + display + interrupt enable)
    // A two-block list at GROM >1000. Block 0 (4 bytes, duration 3): latch ch0
    // tone period then attenuation. Block 1 (2 bytes, duration 0 = end): mute
    // ch0 and ch1. FLAGS bit 0 is clear at boot, so the source is GROM.
    const LIST: u16 = 0x1000;
    let list = [0x04, 0x83, 0x0F, 0x90, 0x00, 0x03, 0x02, 0x9F, 0xBF, 0x00];
    grom[LIST as usize..LIST as usize + list.len()].copy_from_slice(&list);

    let run = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..3 {
            m.run_frame(); // boot + arm the interrupt
        }
        m.bus_mut().poke_word(0x83CC, LIST); // sound-list pointer
        m.bus_mut().poke(0x83CE, 0x01); // countdown 1 -> drain on the next tick
        for _ in 0..10 {
            m.run_frame(); // the ISR drains the list over these vblanks
        }
        m
    };
    let a = run(auth_rom);
    let o = run(our_rom());

    // Sanity: the authentic engine actually ran the whole list.
    assert_eq!(a.bus().peek(0x83CE), 0x00, "authentic: the D=0 block ended the list");
    assert_eq!(a.bus().peek_word(0x83CC), LIST + 10, "authentic: pointer advanced past both blocks");
    assert_ne!(a.bus().psg.period(0), 0, "authentic: ch0 tone period was written");

    // Our engine matches the authentic bookkeeping and the byte stream it emitted
    // (the SN76489 register state is a faithful function of the bytes at >8400).
    assert_eq!(o.bus().peek_word(0x83CC), a.bus().peek_word(0x83CC), "sound-list pointer >83CC");
    assert_eq!(o.bus().peek(0x83CE), a.bus().peek(0x83CE), "sound-list countdown >83CE");
    for ch in 0..3 {
        assert_eq!(o.bus().psg.period(ch), a.bus().psg.period(ch), "PSG tone period ch{ch}");
    }
    for ch in 0..4 {
        assert_eq!(o.bus().psg.volume(ch), a.bus().psg.volume(ch), "PSG attenuation ch{ch}");
    }
    assert_eq!(o.bus().psg.lfsr(), a.bus().psg.lfsr(), "PSG noise LFSR");
}

#[test]
fn isr_sprite_motion_integrates_like_authentic() {
    let Some(auth_rom) = auth_rom() else { return };
    // Arm the VDP interrupt, plant a sprite motion table (SMT at VDP >0780) and a
    // sprite attribute table (SAT at VDP >0300) for three auto-motion sprites —
    // one moving down-right, one up-left, and one positioned to trigger the
    // vertical screen-edge wrap — then idle. Each vblank the ISR integrates every
    // sprite's velocity into its position; the resulting SAT positions and SMT
    // accumulators must match the authentic ROM's motion engine byte-for-byte.
    let program = vec![
        0x39, 0x00, 0x01, 0x01, 0x01, 0x00, // MOVE >0001,#1,G>0100  (reg1 := >E0)
        0xBE, 0x00, 0xFF, // ST  @>8300,>FF
        0xBF, 0x02, 0x00, 0x02, // DST @>8302,>0002
        0xBF, 0x04, 0x01, 0x00, // DST @>8304,>0100
        0xF6, 0x02, 0x03, // IO  @>8302,#3   (arm the 9901 VDP interrupt)
    ];
    let mut grom = vec![0u8; 0x6000];
    let p = prog(program);
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    grom[0x0100] = 0xE0; // reg1 value (16K + display + interrupt enable)
    // SAT: [Y, X, pattern, colour] per sprite. SMT: [Yvel, Xvel, Yacc, Xacc].
    // Sprite 2 sits at Y=>D0 so a downward velocity drives it into the wrap band.
    let sat: [u8; 12] = [40, 80, 0, 0, 100, 100, 0, 0, 0xD0, 50, 0, 0];
    let smt: [u8; 12] = [0x20, 0x10, 0, 0, 0xF0, 0xE0, 0, 0, 0x40, 0x00, 0, 0];

    let run = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..3 {
            m.run_frame(); // boot + arm the interrupt
        }
        for (i, &b) in sat.iter().enumerate() {
            m.vdp_mut().set_vram(0x0300 + i as u16, b);
        }
        for (i, &b) in smt.iter().enumerate() {
            m.vdp_mut().set_vram(0x0780 + i as u16, b);
        }
        m.bus_mut().poke(0x837A, 3); // three auto-motion sprites
        for _ in 0..5 {
            m.run_frame(); // integrate over five vblanks
        }
        m
    };
    let a = run(auth_rom);
    let o = run(our_rom());

    // Sanity: the authentic engine actually moved the sprites.
    assert_ne!(a.vdp().vram(0x0300), 40, "authentic: sprite 0 Y advanced");

    // Our engine matches the authentic SAT positions and SMT accumulators exactly.
    for i in 0..12u16 {
        assert_eq!(o.vdp().vram(0x0300 + i), a.vdp().vram(0x0300 + i), "SAT byte >{:04X}", 0x0300 + i);
        assert_eq!(o.vdp().vram(0x0780 + i), a.vdp().vram(0x0780 + i), "SMT byte >{:04X}", 0x0780 + i);
    }
}

#[test]
fn io_sound_arm_function0_sets_the_list_cells() {
    // GPL `IO @cell,#0` arms a GROM sound list for the ISR: the operand cell holds
    // the list address, which the handler stores into >83CC/D while setting the
    // countdown >83CE = SPEED. (No interrupt needed — this isolates the arm path.)
    //   DST @>8340,>1000   the cell := the list address
    //   IO  @>8340,#0      function 0 = arm a GROM sound list
    let Some(auth_rom) = auth_rom() else { return };
    let program = vec![0xBF, 0x40, 0x10, 0x00, 0xF6, 0x40, 0x00];
    let a = run(auth_rom, &prog(program.clone()), 3);
    let o = run(our_rom(), &prog(program), 3);
    assert_eq!(a.bus().peek_word(0x83CC), 0x1000, "authentic: >83CC := the operand cell's list pointer");
    assert_eq!(a.bus().peek(0x83CE), 0x01, "authentic: >83CE := SPEED");
    assert_eq!(o.bus().peek_word(0x83CC), a.bus().peek_word(0x83CC), "our sound-list pointer >83CC");
    assert_eq!(o.bus().peek(0x83CE), a.bus().peek(0x83CE), "our sound-list countdown >83CE");
}

#[test]
fn io_sound_function1_drains_from_vdp() {
    let Some(auth_rom) = auth_rom() else { return };
    // `IO @cell,#1` arms a VDP-resident sound list (function 1 sets the source bit
    // in FLAGS). Arm the VDP interrupt, plant the list in VRAM, arm via IO #1, and
    // idle: the ISR's sound duty must drain it through the SNDVDP path — the VDP
    // source that the poke-armed sound test does not exercise — like authentic.
    let program = vec![
        0x39, 0x00, 0x01, 0x01, 0x01, 0x00, // MOVE >0001,#1,G>0100  (reg1 := >E0)
        0xBE, 0x00, 0xFF, // ST  @>8300,>FF
        0xBF, 0x02, 0x00, 0x02, // DST @>8302,>0002
        0xBF, 0x04, 0x01, 0x00, // DST @>8304,>0100
        0xF6, 0x02, 0x03, // IO  @>8302,#3   (arm the 9901 VDP interrupt)
        0xBF, 0x40, 0x10, 0x00, // DST @>8340,>1000  (cell := the VDP list address)
        0xF6, 0x40, 0x01, // IO  @>8340,#1   (arm a VDP sound list)
    ];
    let mut grom = vec![0u8; 0x6000];
    let p = prog(program);
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    grom[0x0100] = 0xE0; // reg1 value (16K + display + interrupt enable)
    const LIST: u16 = 0x1000;
    let list = [0x04, 0x83, 0x0F, 0x90, 0x00, 0x03, 0x02, 0x9F, 0xBF, 0x00];
    let run_vdp = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for (i, &b) in list.iter().enumerate() {
            m.vdp_mut().set_vram(LIST + i as u16, b); // the list lives in VRAM
        }
        for _ in 0..12 {
            m.run_frame(); // boot arms the interrupt + IO #1, then the ISR drains
        }
        m
    };
    let a = run_vdp(auth_rom);
    let o = run_vdp(our_rom());
    assert_eq!(a.bus().peek(0x83CE), 0x00, "authentic: the VDP list ran to its D=0 end");
    assert_ne!(a.bus().psg.period(0), 0, "authentic: ch0 tone period written from the VDP list");
    assert_eq!(o.bus().peek_word(0x83CC), a.bus().peek_word(0x83CC), "sound-list pointer >83CC");
    assert_eq!(o.bus().peek(0x83CE), a.bus().peek(0x83CE), "sound-list countdown >83CE");
    for ch in 0..3 {
        assert_eq!(o.bus().psg.period(ch), a.bus().psg.period(ch), "PSG tone period ch{ch}");
    }
    for ch in 0..4 {
        assert_eq!(o.bus().psg.volume(ch), a.bus().psg.volume(ch), "PSG attenuation ch{ch}");
    }
}

#[test]
fn move_count_from_memory() {
    // DST @>8346,>0003 ; ST @>8348,>AB ; MOVE @>8346,@>8349,@>8348 (>34: the
    // count is a WORD read from a memory cell — pins the from-memory width;
    // authentic fills >8349..>834B, leaving >834C clear).
    diff_move(
        "move_count_from_memory",
        vec![0xBF, 0x46, 0x00, 0x03, 0xBE, 0x48, 0xAB, 0x34, 0x46, 0x49, 0x48],
        &[],
    );
}

// ============================================================================
// M4 slice 3 — COINC, SWGR/RTGR, IO CRU-in + the uniform source discipline,
// and the ext-GPL vestige (RECON §25).
// ============================================================================

/// SWGR (word-imm form >FB) switches interpretation to base >9800 at >0100,
/// where a marker store and RTGR run; RTGR restores the base and RTN-pops back
/// to the instruction after the SWGR. Both markers must land, and the whole
/// pad (sub-stack mechanics included) must match the authentic.
#[test]
fn swgr_rtgr_switch_and_return() {
    diff_move(
        "swgr_rtgr_switch_and_return",
        vec![
            0xBF, 0x50, 0x98, 0x00, // DST @>8350,>9800 (the base cell)
            0xFB, 0x50, 0x01, 0x00, // SWGR @>8350,#>0100
            0xBE, 0x41, 0xCD, // ST @>8341,>CD (the return marker)
        ],
        &[(0x0100, &[0xBE, 0x40, 0xAB, 0x13])], // ST @>8340,>AB ; RTGR
    );
}

/// SWGR's word-MEM source form (>F9): the switch address comes from a cell —
/// the uniform imm/mem source discipline for a >=>EC opcode (RECON §25; the
/// pre-M4 driver special-cased these and would have misread this form).
#[test]
fn swgr_mem_source_form() {
    diff_move(
        "swgr_mem_source_form",
        vec![
            0xBF, 0x50, 0x98, 0x00, // DST @>8350,>9800
            0xBF, 0x52, 0x01, 0x00, // DST @>8352,>0100 (the target cell)
            0xF9, 0x50, 0x52, // SWGR @>8350,@>8352
            0xBE, 0x41, 0xCD, // ST @>8341,>CD
        ],
        &[(0x0100, &[0xBE, 0x40, 0xAB, 0x13])],
    );
}

/// COINC word-mem form (>ED), inside the box, target bit SET -> coincidence
/// (>837C := >20 wholesale). Points (>10,>10)/(>12,>13): deltas 2,3 -> bit
/// 2*8+3 = 19 -> table byte 2, MSB-first bit 3 (>10).
#[test]
fn coinc_word_hit() {
    diff_move(
        "coinc_word_hit",
        vec![
            0xBF, 0x40, 0x10, 0x10, // DST @>8340,>1010 (point 1: Y,X)
            0xBF, 0x42, 0x12, 0x13, // DST @>8342,>1213 (point 2)
            0xED, 0x40, 0x42, 0x00, 0x01, 0x00, // COINC @>8340,@>8342,0,>0100
        ],
        &[(0x0100, &[0x07, 0x07, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00])],
    );
}

/// Same geometry, target bit CLEAR -> no coincidence (>837C := >00).
#[test]
fn coinc_word_miss_bit_clear() {
    diff_move(
        "coinc_word_miss_bit_clear",
        vec![
            0xBF, 0x40, 0x10, 0x10, //
            0xBF, 0x42, 0x12, 0x13, //
            0xED, 0x40, 0x42, 0x00, 0x01, 0x00,
        ],
        &[(0x0100, &[0x07, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])],
    );
}

/// Outside the box (Y delta beyond the limit) -> no coincidence, and the
/// bitmap is never consulted.
#[test]
fn coinc_word_miss_outside() {
    diff_move(
        "coinc_word_miss_outside",
        vec![
            0xBF, 0x40, 0x10, 0x10, //
            0xBF, 0x42, 0x1A, 0x13, // Y delta = >0A > the >07 limit
            0xED, 0x40, 0x42, 0x00, 0x01, 0x00,
        ],
        &[(0x0100, &[0x07, 0x07, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF])],
    );
}

/// A negative X delta (point2 left of point1) -> outside, no coincidence.
#[test]
fn coinc_word_miss_negative_delta() {
    diff_move(
        "coinc_word_miss_negative_delta",
        vec![
            0xBF, 0x40, 0x10, 0x10, //
            0xBF, 0x42, 0x12, 0x0C, // X delta = -4
            0xED, 0x40, 0x42, 0x00, 0x01, 0x00,
        ],
        &[(0x0100, &[0x07, 0x07, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF])],
    );
}

/// The scale byte SRAs both deltas (deltas 4,6 >> 1 = 2,3 -> the same set bit
/// as coinc_word_hit).
#[test]
fn coinc_scaled_hit() {
    diff_move(
        "coinc_scaled_hit",
        vec![
            0xBF, 0x40, 0x10, 0x10, //
            0xBF, 0x42, 0x14, 0x16, // deltas 4,6
            0xED, 0x40, 0x42, 0x01, 0x01, 0x00, // scale 1
        ],
        &[(0x0100, &[0x07, 0x07, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00])],
    );
}

/// The header offsets shift the deltas before the bound checks (offsets 1,1
/// on deltas 1,2 -> bit 2*8+3 again).
#[test]
fn coinc_offset_hit() {
    diff_move(
        "coinc_offset_hit",
        vec![
            0xBF, 0x40, 0x10, 0x10, //
            0xBF, 0x42, 0x11, 0x12, // deltas 1,2
            0xED, 0x40, 0x42, 0x00, 0x01, 0x00,
        ],
        &[(0x0100, &[0x07, 0x07, 0x01, 0x01, 0x00, 0x00, 0x10, 0x00])],
    );
}

/// COINC's byte form (>EC): byte operands arrive right-justified and
/// sign-extended (the authentic >07AA discipline) before the same math —
/// the normalization gate.
#[test]
fn coinc_byte_form() {
    diff_move(
        "coinc_byte_form",
        vec![
            0xBE, 0x40, 0x05, // ST @>8340,>05 (point 1, byte view)
            0xBE, 0x42, 0x08, // ST @>8342,>08 (point 2)
            0xEC, 0x40, 0x42, 0x00, 0x01, 0x00,
        ],
        &[(0x0100, &[0x07, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF])],
    );
}

/// IO function 2 (CRU input): arm the 9901 VDP-interrupt mask with an output
/// list, then STCR three bits from CRU 2 back into a pad cell — the read-back
/// must see the armed bit (and the zero-filled rest) identically.
#[test]
fn io_cru_input_reads_back() {
    diff_case(
        "io_cru_input_reads_back",
        &prog(vec![
            0xBE, 0x40, 0x01, // ST  @>8340,>01 (the output data bit)
            0xBF, 0x42, 0x00, 0x02, // DST @>8342,>0002 (out list: CRU addr 2)
            0xBF, 0x44, 0x01, 0x40, // count 1, data cell >40
            0xF6, 0x42, 0x03, // IO  @>8342,#3 -> arm bit 2
            0xBF, 0x46, 0x00, 0x02, // DST @>8346,>0002 (in list: CRU addr 2)
            0xBF, 0x48, 0x03, 0x41, // count 3, data cell >41
            0xF6, 0x46, 0x02, // IO  @>8346,#2 -> >8341 := the 3 bits
        ]),
    );
}

/// IO with word counts: a 12-bit STCR (word store, zero-filled) and a 10-bit
/// LDCR (word read) — the > 8-bit forms access a WORD at the data cell, the
/// 9900's own LDCR/STCR semantics (RECON §25 pins the count > 1 open item).
#[test]
fn io_cru_word_counts() {
    diff_case(
        "io_cru_word_counts",
        &prog(vec![
            0xBF, 0x42, 0x00, 0x00, // DST @>8342,>0000 (in list: CRU addr 0)
            0xBF, 0x44, 0x0C, 0x4A, // count 12, data cell >4A (word)
            0xF6, 0x42, 0x02, // IO  @>8342,#2 -> >834A/B := 12 CRU bits
            0xBF, 0x4C, 0x55, 0x02, // DST @>834C,>5502 (the out word)
            0xBF, 0x46, 0x00, 0x10, // DST @>8346,>0010 (out list: CRU addr >10)
            0xBF, 0x48, 0x0A, 0x4C, // count 10, data cell >4C (word)
            0xF6, 0x46, 0x03, // IO  @>8346,#3 -> 10 bits out
        ]),
    );
}

/// The IO function code from MEMORY (the word-mem form >F5) — the uniform
/// source discipline again; the pre-M4 driver would have read this stream
/// wrongly.
#[test]
fn io_function_from_memory() {
    diff_case(
        "io_function_from_memory",
        &prog(vec![
            0xBE, 0x40, 0x01, // ST  @>8340,>01
            0xBF, 0x42, 0x00, 0x02, // DST @>8342,>0002
            0xBF, 0x44, 0x01, 0x40, // count 1, data cell >40
            0xBF, 0x4A, 0x00, 0x03, // DST @>834A,>0003 (the function cell)
            0xF5, 0x42, 0x4A, // IO  @>8342,@>834A -> function 3
        ]),
    );
}

/// The ext-GPL vestige: special op >14 and two-op >98 dispatch through the
/// >0C0C card trampoline — CRU >1B00 on, then a branch into the absent card's
/// >4000 space (an empty-bus no-op march, the authentic accident). The run is
/// bounded in INSTRUCTIONS (an unbounded march wraps past >FFFE and starts
/// executing the ROM itself as code, where the two images legitimately
/// differ): after the departure both machines must be marching the card
/// space with identical observable state.
#[test]
fn ext_gpl_ops_take_the_card_trampoline() {
    let Some(auth_rom) = auth_rom() else { return };
    for (name, program) in [
        ("ext_gpl_special_14", vec![0xBE, 0x40, 0x77, 0x14]),
        ("ext_gpl_twoop_98", vec![0xBE, 0x40, 0x77, 0x98, 0x40]),
    ] {
        let p = prog(program);
        let go = |rom: &[u8]| {
            let mut grom = vec![0u8; 0x6000];
            grom[0x20..0x20 + p.len()].copy_from_slice(&p);
            let mut m = Machine::new(rom, &grom);
            m.reset();
            for _ in 0..400 {
                m.step();
            }
            m
        };
        let a = go(auth_rom);
        let o = go(our_rom());
        assert!(
            (0x4000..0x6000).contains(&a.cpu().pc()) && (0x4000..0x6000).contains(&o.cpu().pc()),
            "{name}: both machines must have departed into the card space \
             (authentic PC >{:04X}, ours >{:04X})",
            a.cpu().pc(),
            o.cpu().pc()
        );
        // (No PC-lockstep assert: the two interpreters spend different
        // instruction counts reaching the departure — frame-level parity is
        // the documented bar, and the march itself writes nothing.)
        let (a_pad, a_regs, a_vram) = snapshot(&a);
        let (o_pad, o_regs, o_vram) = snapshot(&o);
        assert_eq!(a_pad, o_pad, "{name}: scratchpad differs at the departure state");
        assert_eq!(a_regs, o_regs, "{name}: VDP registers differ");
        assert_eq!(a_vram, o_vram, "{name}: VRAM differs");
    }
}

// ============================================================================
// M4 slice 4 — the fuzz-logged per-opcode-form semantics (RECON §25): MUL/DIV
// byte forms, EX's immediate accident, SUB's NEG-then-ADD status, CEQ's
// raw-STST tail, and the >837D character buffer + *@>837C pop quirk.
// ============================================================================

cases! {
    // MUL byte: the SOURCE keeps its right-justified sign extension, so
    // >09 * >96 = >0009 * >FF96 -> the product's low word >FC46 at D, D+1.
    mul_byte_signed_source: vec![0xBE, 0x40, 0x09, 0xAA, 0x40, 0x96];
    mul_byte_mem: vec![0xBE, 0x40, 0x07, 0xBE, 0x42, 0x85, 0xA8, 0x40, 0x42];
    mul_byte_both_high: vec![0xBE, 0x40, 0xC8, 0xAA, 0x40, 0xC8];
    // DIV byte: dividend = sext(D)::(D:(D+1)), divisor sign-extended;
    // q -> D, r -> D+1; a >= >80 dest byte makes the high word >FFFF ->
    // guaranteed overflow (OR >08, unchanged halves still stored).
    div_byte_basic: vec![0xBE, 0x40, 0x63, 0xBE, 0x41, 0x07, 0xBE, 0x42, 0x05, 0xAC, 0x40, 0x42];
    div_byte_negative_dest_overflows:
        vec![0xBE, 0x40, 0x90, 0xBE, 0x41, 0x07, 0xBE, 0x42, 0x05, 0xAC, 0x40, 0x42];
    div_byte_negative_divisor:
        vec![0xBE, 0x40, 0x10, 0xBE, 0x41, 0x07, 0xBE, 0x42, 0x9C, 0xAC, 0x40, 0x42];
    // DIV word: quotient overflow (high >= divisor) and divide-by-zero both
    // take the 9900's own JNO path — status OR >08 over the >01 preset, and
    // the unchanged dividend halves still store back.
    div_word_overflow_stores:
        vec![0xBF, 0x40, 0x00, 0x05, 0xBF, 0x42, 0x12, 0x34, 0xAF, 0x40, 0x00, 0x04];
    div_word_by_zero:
        vec![0xBF, 0x40, 0x00, 0x01, 0xBF, 0x42, 0x00, 0x10, 0xAF, 0x40, 0x00, 0x00];
    div_word_clean: vec![0xBF, 0x40, 0x00, 0x00, 0xBF, 0x42, 0x30, 0x39, 0xAF, 0x40, 0x00, 0x07];
    // DIV presets >837C wholesale (>01 word / >00 byte) — the prior status
    // must NOT survive a DIV (an INC's carry here, then a clean divide).
    div_wipes_prior_status:
        vec![0xBE, 0x40, 0xFF, 0x90, 0x40, 0xBE, 0x40, 0x08, 0xBE, 0x41, 0x04, 0xBE, 0x42, 0x02,
             0xAC, 0x40, 0x42];
    // EX immediate forms: the immediate stores to the dest; the second store
    // goes to the imm path's leftover pointer (the speech-write region) —
    // inert here, so the observable is dest := imm (RECON §25).
    ex_imm_byte: vec![0xBE, 0x40, 0x55, 0xC2, 0x40, 0x77];
    ex_imm_word: vec![0xBF, 0x40, 0x12, 0x34, 0xC3, 0x40, 0x56, 0x78];
    ex_mem_still_swaps: vec![0xBE, 0x40, 0x11, 0xBE, 0x42, 0x22, 0xC0, 0x40, 0x42];
    // SUB is NEG-then-ADD (authentic >0186): with a ZERO source the carry is
    // an ADD's (clear), not a subtract's no-borrow (set) — the deep-fuzz
    // seed-306 regression; and subtracting >8000 overflows differently too.
    sub_byte_zero_source: vec![0xBE, 0x40, 0x42, 0xBE, 0x42, 0x00, 0xA4, 0x40, 0x42];
    sub_word_zero_source: vec![0xBF, 0x40, 0x12, 0x34, 0xA7, 0x40, 0x00, 0x00];
    sub_word_8000_source: vec![0xBF, 0x40, 0x00, 0x01, 0xA7, 0x40, 0x80, 0x00];
    // CEQ replaces >837C wholesale (the CZ raw-STST tail): a prior carry
    // dies, EQ/H/GT come from the compare, and the visible C bit is the
    // word-form bit (opcode bit 0).
    ceq_word_equal_after_carry:
        vec![0xBE, 0x40, 0xFF, 0x90, 0x40, 0xBF, 0x42, 0x11, 0x11, 0xD7, 0x42, 0x11, 0x11];
    ceq_byte_equal: vec![0xBE, 0x40, 0x66, 0xD6, 0x40, 0x66];
    ceq_word_greater: vec![0xBF, 0x40, 0x22, 0x22, 0xD7, 0x40, 0x11, 0x11];
    // The jump-family compares PRESERVE the rest of >837C (an INC's H/C
    // survive a true CH; only the condition bit moves).
    ch_preserves_status: vec![0xBE, 0x40, 0xFF, 0x90, 0x40, 0xBE, 0x42, 0x01, 0xC4, 0x40, 0x42];
    cge_preserves_status: vec![0xBE, 0x40, 0x7F, 0x90, 0x40, 0xD0, 0x40, 0x40];

    // ---- the >837D character buffer (RECON §25) ----
    // A read naming >837D fetches the screen byte at the cursor first: plant
    // >41 at screen cell 5, point the cursor there, read *the cell* through a
    // memory ST — >8340 and >837D both become >41.
    chbuf_read_fetches_screen: vec![
        0xBE, 0xA0, 0x05, 0x41, // ST V@>0005,>41
        0xBE, 0x7E, 0x00, // ST @>837E,>00 (row)
        0xBE, 0x7F, 0x05, // ST @>837F,>05 (col)
        0xBC, 0x40, 0x7D, // ST @>8340,@>837D -> the screen byte
    ];
    // A store whose last byte lands at >837D paints it at the cursor: cell
    // row 1 col 2 = >0022 becomes >42 (the VRAM window shows it).
    chbuf_write_echoes: vec![
        0xBE, 0x7E, 0x01, // row 1
        0xBE, 0x7F, 0x02, // col 2
        0xBE, 0x7D, 0x42, // ST @>837D,>42 -> echoed to VRAM >0022
    ];
    // A word store to >837C/D echoes its LOW byte (the one that landed on
    // >837D).
    chbuf_word_store_echoes: vec![
        0xBE, 0x7E, 0x00, //
        0xBE, 0x7F, 0x00, //
        0xBF, 0x7C, 0x12, 0x34, // DST @>837C,>1234 -> >34 echoed to cell 0
    ];
    // Arithmetic stores echo too (the authentic >0232 tail serves the whole
    // two-op family).
    chbuf_add_echoes: vec![
        0xBE, 0x7E, 0x00, //
        0xBE, 0x7F, 0x03, //
        0xBE, 0x7D, 0x20, // >837D := >20 (echoes >20)
        0xA2, 0x7D, 0x11, // ADD @>837D,>11 -> reads the screen (>20), +>11,
    ]; // stores >31, echoes >31 to cell 3
    // *@>837C is the data-stack pop quirk: the "pointer" is the stack byte
    // >8372, post-decremented — the operand is the popped stack cell.
    stack_pop_indirect: vec![
        0xBE, 0x72, 0x45, // ST @>8372,>45 (the stack pointer)
        0xBE, 0x45, 0xAB, // ST @>8345,>AB (the top slot)
        0xBC, 0x40, 0x90, 0x7C, // ST @>8340,*@>837C -> pops >AB, >8372 -> >44
    ];
    // The multicolour fork (FLAGS >02): reads extract the (row,col) nibble,
    // writes read-modify-write it — path parity under both ROMs (the pattern
    // cell lives above the compared VRAM window; the scratchpad, the >837D
    // nibble value, and the VDP port traffic still discriminate).
    chbuf_multicolour_write: vec![
        0xBE, 0xFD, 0x02, // FLAGS := >02 (multicolour)
        0xBE, 0x7E, 0x02, // row 2
        0xBE, 0x7F, 0x05, // col 5 (odd -> the low nibble)
        0xBE, 0x7D, 0x07, // >837D := >07 -> RMW the pattern nibble
        0xBC, 0x41, 0x7D, // read it back through the buffer
    ];
}

// ============================================================================
// M4 slice 5 — the cassette modem layer (RECON §26): the IO 4/5/6 engines'
// observable surface + the FLAGS->20 cassette timer ISR fork. The emulator
// has no 9901 interval timer and no tape line, so the engines park on their
// first half-cell wait — identically under both ROMs; what IS observable
// (the list parse, FLAGS, the >837C markers, the 9901 programming, the fork's
// PC warp) is gated here.
// ============================================================================

/// Drive one cassette IO function under `rom` and return (pad snapshot,
/// the 9901 interrupt mask, FLAGS). The engine parks in its first stepped
/// half-cell; four frames are plenty.
fn cassette_run(rom: &[u8], func: u8) -> ((Vec<u8>, Vec<u8>, Vec<u8>), bool, u8) {
    let p = prog(vec![
        0xBF, 0x42, 0x00, 0x80, // DST @>8342,>0080 (byte count 128 = 2 records)
        0xBF, 0x44, 0x10, 0x00, // DST @>8344,>1000 (the VDP window)
        0xF6, 0x42, func, // IO  @>8342,#func
        0xBE, 0x41, 0xEE, // ST  @>8341,>EE — must never run (the engine parks)
    ]);
    let mut grom = vec![0u8; 0x6000];
    grom[0x20..0x20 + p.len()].copy_from_slice(&p);
    let mut m = Machine::new(rom, &grom);
    m.reset();
    for _ in 0..4 {
        m.run_frame();
    }
    let flags = m.bus().peek(0x83FD);
    (snapshot(&m), m.bus().tms9901.int_mask() & 0x0004 != 0, flags)
}

/// IO #4 (cassette write): both ROMs must arm identically — FLAGS >20 set
/// (the timer-ISR fork), the VDP interrupt mask dropped, the marker after
/// the IO never reached (the engine owns the CPU), and the whole pad equal.
#[test]
fn cassette_write_arms_and_parks() {
    let Some(auth_rom) = auth_rom() else { return };
    let (a, a_vdp, a_flags) = cassette_run(auth_rom, 4);
    let (o, o_vdp, o_flags) = cassette_run(our_rom(), 4);
    assert_eq!(a.0, o.0, "cassette #4: scratchpad differs");
    assert_eq!((a.1.clone(), a.2.clone()), (o.1, o.2), "cassette #4: VDP state differs");
    assert!(!a_vdp && !o_vdp, "cassette #4: the VDP interrupt must be masked off");
    assert_eq!(a_flags & 0x20, 0x20, "cassette #4: FLAGS >20 (the timer fork) must be set");
    assert_eq!(a_flags, o_flags, "cassette #4: FLAGS differ");
    assert_eq!(a.0[0x41], 0x00, "cassette #4: the post-IO marker must never run");
}

/// IO #5 (read) clears the verify bit; IO #6 (verify) sets it; both set the
/// >20 fork and the >21 record-phase marker in >837C (visible as >20 under
/// the &F8 mask), park, and match across the ROMs.
#[test]
fn cassette_read_and_verify_flags() {
    let Some(auth_rom) = auth_rom() else { return };
    for (func, v10) in [(5u8, 0x00u8), (6, 0x10)] {
        let (a, _, a_flags) = cassette_run(auth_rom, func);
        let (o, _, o_flags) = cassette_run(our_rom(), func);
        assert_eq!(a.0, o.0, "cassette #{func}: scratchpad differs");
        assert_eq!(a_flags & 0x30, 0x20 | v10, "cassette #{func}: FLAGS >20/>10 wrong");
        assert_eq!(a_flags, o_flags, "cassette #{func}: FLAGS differ");
        assert_eq!(a.0[0x7C] & 0xF8, 0x20, "cassette #{func}: the >21 phase marker");
    }
}

/// The cassette timer ISR fork (FLAGS >20 + a level-1 interrupt): the ISR
/// must branch to the cassette handler, which — interrupted outside a JMP-$
/// wait — warps the PC to the resume address parked at >83EC (GPL R6's
/// cell). R6 is interpreter scratch, so the harness pokes the warp target at
/// a frame boundary and raises the vblank itself, so the pending interrupt
/// is taken at the first LIMI window — before any GPL op can scribble R6.
/// (The beam-accurate `run_frame` raises its own vblank only at line 192,
/// two-thirds of a frame of GPL execution later — far too late for the poke
/// to survive.) The planted ML stub clears the FLAGS word and re-enters the
/// interpreter; without the fork (an acknowledge-only ISR) FLAGS would
/// keep >20.
#[test]
fn cassette_timer_isr_fork_warps() {
    let Some(auth_rom) = auth_rom() else { return };
    let p = prog(vec![
        0x39, 0x00, 0x01, 0x01, 0x01, 0x00, // MOVE >0001,#1,G>0100 (VDP R1 IE)
        0xBE, 0x40, 0x01, // the VDP-interrupt arming list...
        0xBF, 0x42, 0x00, 0x02, //
        0xBF, 0x44, 0x01, 0x40, //
        0xF6, 0x42, 0x03, // IO @>8342,#3 -> the normal ISR ticks from here
    ]);
    let run = |rom: &[u8]| {
        let mut grom = vec![0u8; 0x6000];
        grom[0x20..0x20 + p.len()].copy_from_slice(&p);
        grom[0x0100] = 0xE0; // VDP R1: 16K + display + interrupt enable
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..3 {
            m.run_frame();
        }
        // A two-rung JMP-$ ladder at >8390 (each cassette-fork interrupt
        // INCTs the saved PC past one rung — the authentic single-step),
        // then the stub: CLR @>83FC (the SPEED/FLAGS word), B @>0070.
        // (No GPL-R1 poke: whatever its parked sign, the FIRST interrupt
        // warps — both CASTIM arcs end at the >83EC target when the
        // interrupted instruction is not a JMP-$.)
        for (a, w) in [
            (0x8390u16, 0x10FFu16),
            (0x8392, 0x10FF),
            (0x8394, 0x04E0),
            (0x8396, 0x83FC),
            (0x8398, 0x0460),
            (0x839A, 0x0070),
        ] {
            m.bus_mut().poke(a, (w >> 8) as u8);
            m.bus_mut().poke(a + 1, w as u8);
        }
        m.bus_mut().poke(0x83EC, 0x83); // the parked warp target := >8390
        m.bus_mut().poke(0x83ED, 0x90);
        m.bus_mut().poke(0x83FD, 0x20); // FLAGS: the cassette fork on
        // Fire the vblank NOW, while the pokes are fresh — the level-held
        // line is taken at the first LIMI window of the next frame.
        m.bus_mut().vdp.vblank();
        // Int 1 warps to the ladder; ints 2-3 step the rungs (the fork path
        // never reads the VDP status, so they re-fire back to back); the
        // stub clears FLAGS and re-enters the interpreter — where the normal
        // ISR finally clears the line.
        for _ in 0..3 {
            m.run_frame();
        }
        m
    };
    let a = run(auth_rom);
    let o = run(our_rom());
    let (mut a_pad, a_regs, a_vram) = snapshot(&a);
    let (mut o_pad, o_regs, o_vram) = snapshot(&o);
    // The RTWP frame (>83DA-DF) holds each interpreter's own interrupted
    // WP/PC/ST — implementation geometry, not compared (the idle-ISR gates
    // document the same exclusion).
    for pad in [&mut a_pad, &mut o_pad] {
        for cell in &mut pad[0xDA..=0xDF] {
            *cell = 0;
        }
    }
    assert_eq!(a_pad, o_pad, "timer-fork: scratchpad differs");
    assert_eq!(a_regs, o_regs, "timer-fork: VDP registers differ");
    assert_eq!(a_vram, o_vram, "timer-fork: VRAM differs");
    // The fork ran under BOTH ROMs — its race-free signature is the 9901
    // interrupt-mask bit 3 that the cassette timer ISR's SBO 3 arms (an
    // acknowledge-only ISR never touches it).
    for (name, m) in [("authentic", &a), ("ours", &o)] {
        assert!(
            m.bus().tms9901.int_mask() & 0x0008 != 0,
            "timer-fork: {name} must have taken the cassette fork (SBO 3)"
        );
    }
    // The warp mechanism rides the JMP-$ ladder: every interrupt in the
    // ensuing livelock (the VDP line is level-held; on hardware the TIMER
    // interrupt self-clears and the ladder steps on) saves a ladder address
    // as the interrupted PC. Asserted on the authentic only: the poked warp
    // target races with GPL R6 rewrites, and WHICH interpreter-internal ops
    // rewrite R6 is excluded from parity by design (our BR does, theirs
    // does not) — ours takes the same warp with the same >83EC contents.
    let saved_pc = a.bus().peek_word(0x83DC);
    assert!(
        (0x8390..=0x839A).contains(&saved_pc),
        "timer-fork: the authentic must be cycling the warp ladder \
         (RTWP-frame PC >{saved_pc:04X})"
    );
}

/// The pinned vestigial surfaces are byte-identical to the authentic: the
/// ext-GPL trampoline block (>0C0C-0C2F, address-forced like the vectors),
/// the XOP-0 vector (>0040), and XTAB's >1C-1F tail (the harvested dispatch
/// constants that index into CFI's future home — RECON §8/§25).
#[test]
fn vestigial_surfaces_match_authentic_bytes() {
    let Some(auth_rom) = auth_rom() else { return };
    let ours = our_rom();
    assert_eq!(
        &ours[0x0C0C..0x0C30],
        &auth_rom[0x0C0C..0x0C30],
        "the ext-GPL trampoline block must be byte-identical"
    );
    assert_eq!(&ours[0x0040..0x0044], &auth_rom[0x0040..0x0044], "the XOP-0 vector");
    assert_eq!(
        &ours[0x12B8..0x12C0],
        &auth_rom[0x12B8..0x12C0],
        "XTAB's >1C-1F vestigial tail (harvested constants)"
    );
}

/// The >837D character-buffer window (§25) must preserve the program's own
/// scratchpad. Real GPL uses the window as a drawing primitive with live
/// state around it — Tunnels of Doom plots its corridor floor-edge chars
/// through >837D with renderer state in >8306, and our old echo tails
/// spilled interpreter temporaries into >8302-8306, which snapped the
/// floor-edge loop after one segment (the "blocky hallway floor" bug). The
/// standard `snapshot` masks >8300-8307 as engine residue, so this runner
/// compares those cells EXACTLY: sentinels are planted test-side, the
/// program drives one window access, and the authentic — which preserves
/// the whole pad on these paths — is the oracle.
fn diff_chb_case(name: &str, p: &[u8]) {
    let Some(auth_rom) = auth_rom() else { return };
    let run_chb = |rom: &[u8]| -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let full = prog(p.to_vec());
        let mut grom = vec![0u8; 0x6000];
        grom[0x20..0x20 + full.len()].copy_from_slice(&full);
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for i in 0..16u16 {
            m.bus_mut().poke(0x8300 + i, 0xA0 + i as u8);
        }
        m.bus_mut().poke(0x837E, 5); // cursor row
        m.bus_mut().poke(0x837F, 7); // cursor column
        for _ in 0..10 {
            m.run_frame();
        }
        let pad: Vec<u8> = (0x8300u16..0x83E0)
            .map(|a| {
                let b = m.bus().peek(a);
                match a {
                    // NO >8300-8307 mask here — preserving these is the point.
                    0x8372 | 0x8373 => 0,
                    0x837C => b & 0xF8,
                    _ => b,
                }
            })
            .collect();
        let regs: Vec<u8> = (0..8).map(|r| m.vdp().register(r)).collect();
        // The window paints the name table (standard) or the pattern table
        // at >0800+ (multicolour) — cover both.
        let vram: Vec<u8> = (0x0000u16..0x0C00).map(|a| m.vdp().vram(a)).collect();
        (pad, regs, vram)
    };
    let (a_pad, a_regs, a_vram) = run_chb(auth_rom);
    let (o_pad, o_regs, o_vram) = run_chb(our_rom());
    for i in 0..a_pad.len() {
        assert_eq!(
            a_pad[i],
            o_pad[i],
            "{name}: scratchpad >{:04X} differs (authentic {:02X} vs ours {:02X})",
            0x8300 + i,
            a_pad[i],
            o_pad[i]
        );
    }
    assert_eq!(a_regs, o_regs, "{name}: VDP registers differ");
    for i in 0..a_vram.len() {
        assert_eq!(
            a_vram[i], o_vram[i],
            "{name}: VRAM >{i:04X} differs (authentic {:02X} vs ours {:02X})",
            a_vram[i], o_vram[i]
        );
    }
}

#[test]
fn chb_write_preserves_program_scratchpad() {
    diff_chb_case("chb write", &[0xBE, 0x7D, 0x41]); // ST >837D,>41
}

#[test]
fn chb_read_preserves_program_scratchpad() {
    diff_chb_case("chb read", &[0xD6, 0x7D, 0x00]); // CEQ >837D,>00
}

#[test]
fn chb_word_write_ending_at_837d_echoes_identically() {
    diff_chb_case("chb word write", &[0xBF, 0x7C, 0x12, 0x34]); // ST(w) >837C,>1234
}

#[test]
fn chb_mul_store_echo_preserves_the_parked_second_word() {
    // Word MUL with D = >837C: the high-word store's last byte lands on
    // >837D and fires the echo MID-SEQUENCE, while the low word waits in
    // R0 for the second store at D+2 — the echo must not clobber it.
    diff_chb_case("chb mul echo", &[0xAB, 0x7C, 0x00, 0x02]);
}

#[test]
fn chb_after_gpl_write_to_r14_flags_matches() {
    // Writing GPL R14's flag byte (>83DD) from GPL derails the interpreter
    // before the >837D access completes — identically under both ROMs, so
    // the derailment itself is pinned here. It does NOT reach the
    // multicolour nibble protocol: that path (FLAGS >02 set by the console
    // GROM's own mode code) has no differential pin yet — the nibble
    // algorithm is carried by review against the authentic >08D6-08FE.
    diff_chb_case("chb r14-write write", &[0xBE, 0xDD, 0x02, 0xBE, 0x7D, 0x41]);
    diff_chb_case("chb r14-write read", &[0xBE, 0xDD, 0x02, 0xD6, 0x7D, 0x00]);
}
