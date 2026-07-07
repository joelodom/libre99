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

//! **M4 FMT differential microsuite** — the `FMT` (`>08`) screen-format
//! sub-interpreter, one differential case per sub-op of the authentic `>0CDC`
//! table (grammar pinned by disassembling authentic `>04DE-05B7` as a spec —
//! RECON §7, plan P5/P9). Each program runs the same `FMT … FEND` under the
//! AUTHENTIC console ROM and OUR rewrite from identical state; the full
//! observable state (scratchpad `>8300-83DF`, the VDP registers, and the VRAM
//! window that holds FMT's screen output) must match. The authentic ROM is the
//! oracle — no cartridge is needed to drive any sub-op.
//!
//! A program is `[0x08, <format bytes…>, 0xFB]` (FEND leaves the sub-language);
//! `prog()` appends the interpreter self-loop terminator after it.

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
fn run(rom: &[u8], program: &[u8], frames: usize) -> Machine {
    let mut grom = vec![0u8; 0x6000];
    grom[0x20..0x20 + program.len()].copy_from_slice(program);
    let mut m = Machine::new(rom, &grom);
    m.reset();
    for _ in 0..frames {
        m.run_frame();
    }
    m
}

/// The observable state: scratchpad `>8300-83DF`, the 8 VDP registers, and
/// VRAM `>0000-04FF` (the name table FMT writes to, plus margin). `>837C`'s low
/// three bits are interpreter-internal (RECON §16); `>8300-8307` and the stack
/// pointer bytes carry engine residue (kept out of comparison — FMT keeps its
/// working data at `>8340+`).
fn snapshot(m: &Machine) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let pad: Vec<u8> = (0x8300u16..0x83E0)
        .map(|a| {
            let b = m.bus().peek(a);
            match a {
                0x8300..=0x8307 => 0,
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

/// A condition-safe interpreter terminator at GROM address `a` (double-BR — a
/// bare `BR $` is not a self-loop when the condition bit is set; RECON §16).
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

/// Run one FMT case under both ROMs and diff scratchpad + VDP regs + VRAM.
fn diff_case(name: &str, p: &[u8]) {
    let Some(auth_rom) = auth_rom() else { return };
    let (a_pad, a_regs, a_vram) = snapshot(&run(auth_rom, p, 10));
    let (o_pad, o_regs, o_vram) = snapshot(&run(our_rom(), p, 10));
    for i in 0..a_pad.len() {
        assert_eq!(
            a_pad[i], o_pad[i],
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

macro_rules! case {
    ($test:ident, $bytes:expr) => {
        #[test]
        fn $test() {
            diff_case(stringify!($test), &prog($bytes));
        }
    };
}

// 0/1 HTEXT — n+1 inline chars written horizontally (byte >01 = 2 chars).
case!(fmt_htext_two_chars, vec![0x08, 0x01, 0x41, 0x42, 0xFB]);
// A longer HTEXT string exercises the multi-char horizontal advance.
case!(
    fmt_htext_run,
    vec![0x08, 0x05, 0x48, 0x45, 0x4C, 0x4C, 0x4F, 0x21, 0xFB]
);
// 2/3 VTEXT — n+1 inline chars written down a column (byte >21 = 2 chars).
case!(fmt_vtext_two_chars, vec![0x08, 0x21, 0x41, 0x42, 0xFB]);
// 4/5 HCHAR — repeat one char horizontally (byte >42 = 3 copies of 'X').
case!(fmt_hchar_repeat, vec![0x08, 0x42, 0x58, 0xFB]);
// 6/7 VCHAR — repeat one char vertically (byte >62 = 3 copies of 'Y').
case!(fmt_vchar_repeat, vec![0x08, 0x62, 0x59, 0xFB]);
// 8/9 HMOVE — advance the cursor n+1 columns (>82 = +3), then a char lands there.
case!(fmt_hmove_then_char, vec![0x08, 0x82, 0x40, 0x5A, 0xFB]);
// A/B VMOVE — advance the cursor n+1 rows (>A1 = +2 rows), then a char lands there.
case!(fmt_vmove_then_char, vec![0x08, 0xA1, 0x40, 0x5B, 0xFB]);
// >FF COL then >FE ROW — set the cursor, then a char lands at (row 2, col 5).
case!(
    fmt_row_col_set,
    vec![0x08, 0xFF, 0x05, 0xFE, 0x02, 0x40, 0x51, 0xFB]
);
// >FC BIAS immediate — the bias adds to each emitted char ('A' + >10 = >51).
case!(fmt_bias_immediate, vec![0x08, 0xFC, 0x10, 0x00, 0x41, 0xFB]);
// A bias larger than the char exercises the 8-bit wrap of the addition.
case!(fmt_bias_wraps, vec![0x08, 0xFC, 0xE0, 0x02, 0x30, 0x31, 0x32, 0xFB]);
// C/D RPTB — repeat a one-char block n+1 passes (>C2 = 3); the FEND at >0024
// loops back to the HCHAR at GROM >0022; the trailing >FB (nesting 0) exits.
case!(
    fmt_rptb_loop,
    vec![0x08, 0xC2, 0x40, 0x41, 0xFB, 0x00, 0x22, 0xFB]
);
// The 768-cell wrap: from (row 23, col 30) an HTEXT of 4 rolls past the last
// cell back to the top of the screen.
case!(
    fmt_htext_wrap,
    vec![0x08, 0xFF, 0x1E, 0xFE, 0x17, 0x03, 0x41, 0x42, 0x43, 0x44, 0xFB]
);
// FEND with an empty program returns cleanly (cursor unchanged at 0,0).
case!(fmt_empty_fend, vec![0x08, 0xFB]);

// >E0-FA — a string from a GAS operand: plant 'M','N' at CPU >8340 with two
// byte-immediate STs, then emit 2 chars from >8340 (operand >40 = short-form
// >8300+40). Differential, plus an explicit check that the data really flowed.
#[test]
fn fmt_mem_string_from_cpu() {
    let Some(auth_rom) = auth_rom() else { return };
    let p = prog(vec![
        0xBE, 0x40, 0x4D, // ST byte-imm  >8340 := 'M'
        0xBE, 0x41, 0x4E, // ST byte-imm  >8341 := 'N'
        0x08, 0xE1, 0x40, 0xFB, // FMT: string(2) from >8340 ; FEND
    ]);
    diff_case("fmt_mem_string_from_cpu", &p);
    for rom in [auth_rom, our_rom()] {
        let m = run(rom, &p, 10);
        assert_eq!(m.vdp().vram(0), 0x4D, "mem-string char 0");
        assert_eq!(m.vdp().vram(1), 0x4E, "mem-string char 1");
    }
}

// >FD — BIAS from a GAS operand: plant >10 at CPU >8340, then an HTEXT 'A' is
// biased by it ('A' >41 + >10 = >51).
#[test]
fn fmt_bias_from_cpu() {
    let Some(auth_rom) = auth_rom() else { return };
    let p = prog(vec![
        0xBE, 0x40, 0x10, // ST byte-imm  >8340 := >10
        0x08, 0xFD, 0x40, 0x00, 0x41, 0xFB, // FMT: bias := @>8340 ; HTEXT 'A'
    ]);
    diff_case("fmt_bias_from_cpu", &p);
    for rom in [auth_rom, our_rom()] {
        let m = run(rom, &p, 10);
        assert_eq!(m.vdp().vram(0), 0x51, "biased char");
    }
}
