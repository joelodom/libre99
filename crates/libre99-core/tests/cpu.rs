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

//! TMS9900 CPU conformance tests.
//!
//! These exercise the processor in isolation against a flat-RAM bus. Each test
//! assembles a tiny machine-code program as 16-bit words (the natural unit of
//! TMS9900 encoding), runs it, and checks the resulting registers, status flags,
//! and memory against the documented behavior of the real chip. The encodings in
//! the comments double as a compact opcode reference.
//!
//! Status-flag expectations follow the per-instruction effect table verified
//! during research (see `docs/history/PLAN.md` §2.1): e.g. `C` (compare) stores nothing,
//! `CLR`/`SETO`/`SWPB` touch no flags, `ABS` clears carry, `SLA` sets overflow on
//! a sign change, parity is computed only by byte instructions, and so on.

use libre99_core::bus::{Bus, FlatRam};
use libre99_core::cpu::{Cpu, ST_AGT, ST_C, ST_EQ, ST_LGT, ST_OP, ST_OV};

/// Workspace base used by most tests (registers live at `WP + 2*n`).
const WP: u16 = 0x0300;
/// Where test programs are loaded and PC starts.
const PROG: u16 = 0x1000;

/// Build a CPU + flat RAM, load `program` (as words) at [`PROG`], point the CPU
/// at it with workspace [`WP`], and clear the status register.
fn setup(program: &[u16]) -> (Cpu, FlatRam) {
    let mut ram = FlatRam::new();
    ram.load_words(PROG, program);
    let mut cpu = Cpu::new();
    cpu.set_wp(WP);
    cpu.set_pc(PROG);
    cpu.set_st(0x0000);
    (cpu, ram)
}

/// Read workspace register `n`.
fn reg(ram: &mut FlatRam, n: u16) -> u16 {
    ram.read_word(WP + 2 * n)
}
/// Write workspace register `n`.
fn set_reg(ram: &mut FlatRam, n: u16, v: u16) {
    ram.write_word(WP + 2 * n, v);
}

// --------------------------------------------------------------------------
// Reset and interrupt vectoring
// --------------------------------------------------------------------------

#[test]
fn reset_loads_workspace_and_pc_from_vector_0() {
    let mut ram = FlatRam::new();
    // Reset vector: WP at >0000, PC at >0002 — the real console's values.
    ram.load_words(0x0000, &[0x83E0, 0x0024]);
    let mut cpu = Cpu::new();
    cpu.reset(&mut ram);
    assert_eq!(cpu.wp(), 0x83E0);
    assert_eq!(cpu.pc(), 0x0024);
    // Interrupt mask cleared on reset (interrupts disabled until LIMI).
    assert_eq!(cpu.st() & 0x000F, 0);
}

// --------------------------------------------------------------------------
// Immediate operations
// --------------------------------------------------------------------------

#[test]
fn li_loads_immediate_and_sets_compare_flags() {
    // LI R1,>1234   (0x0201, 0x1234)
    let (mut cpu, mut ram) = setup(&[0x0201, 0x1234]);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x1234);
    // Positive, non-zero: L> and A> set, EQ clear.
    assert!(cpu.st() & ST_LGT != 0);
    assert!(cpu.st() & ST_AGT != 0);
    assert!(cpu.st() & ST_EQ == 0);
}

#[test]
fn li_zero_sets_equal() {
    // LI R0,0
    let (mut cpu, mut ram) = setup(&[0x0200, 0x0000]);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_EQ != 0);
    assert!(cpu.st() & ST_LGT == 0);
    assert!(cpu.st() & ST_AGT == 0);
}

#[test]
fn li_negative_sets_logical_but_not_arithmetic_greater() {
    // LI R0,>8000 — most negative. As unsigned it is > 0 (L> set); as signed it
    // is < 0 (A> clear).
    let (mut cpu, mut ram) = setup(&[0x0200, 0x8000]);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_LGT != 0);
    assert!(cpu.st() & ST_AGT == 0);
    assert!(cpu.st() & ST_EQ == 0);
}

#[test]
fn ai_adds_immediate_and_sets_carry() {
    // LI R1,>FFFF ; AI R1,1  -> 0 with carry out and (no signed overflow)
    let (mut cpu, mut ram) = setup(&[0x0201, 0xFFFF, 0x0221, 0x0001]);
    cpu.step(&mut ram);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x0000);
    assert!(cpu.st() & ST_C != 0, "carry out of bit 15");
    assert!(cpu.st() & ST_EQ != 0);
    assert!(cpu.st() & ST_OV == 0, "no signed overflow");
}

#[test]
fn ci_compares_without_storing() {
    // LI R1,5 ; CI R1,5  -> EQ, R1 unchanged
    let (mut cpu, mut ram) = setup(&[0x0201, 0x0005, 0x0281, 0x0005]);
    cpu.step(&mut ram);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 5);
    assert!(cpu.st() & ST_EQ != 0);
}

#[test]
fn andi_ori_mask_bits() {
    // LI R1,>FF0F ; ANDI R1,>0FF0 ; ORI R1,>00F0
    let (mut cpu, mut ram) = setup(&[0x0201, 0xFF0F, 0x0241, 0x0FF0, 0x0261, 0x00F0]);
    cpu.step(&mut ram);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x0F00);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x0FF0);
}

#[test]
fn lwpi_and_stwp_move_the_workspace_pointer() {
    // LWPI >2000 ; (now registers are at >2000)
    let (mut cpu, mut ram) = setup(&[0x02E0, 0x2000]);
    cpu.step(&mut ram);
    assert_eq!(cpu.wp(), 0x2000);
}

// --------------------------------------------------------------------------
// Word arithmetic / data movement
// --------------------------------------------------------------------------

#[test]
fn mov_sets_compare_flags_but_not_carry() {
    // MOV R1,R2 with R1 = >8000 (negative, non-zero)
    let (mut cpu, mut ram) = setup(&[0xC081]); // MOV R1,R2
    set_reg(&mut ram, 1, 0x8000);
    cpu.set_st(ST_C); // pre-set carry to prove MOV leaves it alone
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x8000);
    assert!(cpu.st() & ST_LGT != 0);
    assert!(cpu.st() & ST_AGT == 0);
    assert!(cpu.st() & ST_C != 0, "MOV must not disturb carry");
}

#[test]
fn add_produces_carry_and_overflow() {
    // A R1,R2 with R1=R2=>8000 -> 0, carry set, signed overflow set.
    let (mut cpu, mut ram) = setup(&[0xA081]); // A R1,R2
    set_reg(&mut ram, 1, 0x8000);
    set_reg(&mut ram, 2, 0x8000);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x0000);
    assert!(cpu.st() & ST_C != 0);
    assert!(cpu.st() & ST_OV != 0);
    assert!(cpu.st() & ST_EQ != 0);
}

#[test]
fn subtract_sets_carry_when_no_borrow() {
    // S R1,R2 with R2=5, R1=3 -> 2, carry set (no borrow), positive.
    let (mut cpu, mut ram) = setup(&[0x6081]); // S R1,R2
    set_reg(&mut ram, 1, 3);
    set_reg(&mut ram, 2, 5);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 2);
    assert!(cpu.st() & ST_C != 0, "no borrow -> carry set");
    assert!(cpu.st() & ST_AGT != 0);
}

#[test]
fn inc_and_dec() {
    // INC R1 ; DEC R1 (R1 starts 0x7FFF -> 0x8000 sets overflow; then back)
    let (mut cpu, mut ram) = setup(&[0x0581, 0x0601]); // INC R1 ; DEC R1
    set_reg(&mut ram, 1, 0x7FFF);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x8000);
    assert!(cpu.st() & ST_OV != 0, "0x7FFF+1 overflows signed");
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x7FFF);
}

#[test]
fn neg_negates_and_sets_overflow_on_min_int() {
    // NEG R1 with R1 = >8000 -> still >8000, overflow set.
    let (mut cpu, mut ram) = setup(&[0x0501]); // NEG R1
    set_reg(&mut ram, 1, 0x8000);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x8000);
    assert!(cpu.st() & ST_OV != 0);
}

#[test]
fn abs_clears_carry_and_uses_original_for_compare() {
    // ABS R1 with R1 = -3 (>FFFD). Result 3; compare flags reflect the ORIGINAL
    // (negative) value, so A> is clear; and ABS always clears carry.
    let (mut cpu, mut ram) = setup(&[0x0741]); // ABS R1
    set_reg(&mut ram, 1, 0xFFFD);
    cpu.set_st(ST_C);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x0003);
    assert!(cpu.st() & ST_C == 0, "ABS clears carry");
    assert!(cpu.st() & ST_AGT == 0, "compare uses original negative operand");
}

#[test]
fn compare_word_sets_flags_without_storing() {
    // C R1,R2 : R1=2, R2=5 -> R1 logically and arithmetically less than R2.
    let (mut cpu, mut ram) = setup(&[0x8081]); // C R1,R2
    set_reg(&mut ram, 1, 2);
    set_reg(&mut ram, 2, 5);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 2, "C stores nothing");
    assert_eq!(reg(&mut ram, 2), 5);
    assert!(cpu.st() & ST_EQ == 0);
    assert!(cpu.st() & ST_LGT == 0, "2 < 5 unsigned");
    assert!(cpu.st() & ST_AGT == 0, "2 < 5 signed");
}

#[test]
fn soc_and_szc_set_and_clear_bits() {
    // SOC R1,R2 ORs; SZC R1,R2 clears the masked bits.
    let (mut cpu, mut ram) = setup(&[0xE081, 0x4081]); // SOC R1,R2 ; SZC R1,R2
    set_reg(&mut ram, 1, 0x0F0F);
    set_reg(&mut ram, 2, 0xF000);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0xFF0F);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0xF000);
}

#[test]
fn clr_seto_swpb_touch_no_flags() {
    // Pre-set every compare flag, then run CLR/SETO/SWPB and prove they are
    // untouched (these instructions affect no status bits).
    let (mut cpu, mut ram) = setup(&[0x04C1, 0x0701, 0x06C1]); // CLR R1 ; SETO R1 ; SWPB R1
    let flags = ST_LGT | ST_AGT | ST_EQ | ST_C | ST_OV | ST_OP;
    cpu.set_st(flags);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x0000);
    set_reg(&mut ram, 1, 0xFFFF);
    cpu.step(&mut ram); // SETO
    assert_eq!(reg(&mut ram, 1), 0xFFFF);
    set_reg(&mut ram, 1, 0x1234);
    cpu.step(&mut ram); // SWPB -> 0x3412
    assert_eq!(reg(&mut ram, 1), 0x3412);
    assert_eq!(cpu.st(), flags, "no flag may change");
}

#[test]
fn xor_into_destination_register() {
    // XOR R1,R2 -> R2 ^= R1
    let (mut cpu, mut ram) = setup(&[0x2881]); // XOR R1,R2
    set_reg(&mut ram, 1, 0xAAAA);
    set_reg(&mut ram, 2, 0xFFFF);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x5555);
}

// --------------------------------------------------------------------------
// Addressing modes
// --------------------------------------------------------------------------

#[test]
fn indirect_addressing() {
    // MOV *R1,R2 : R1 points at >2000 which holds >BEEF.
    let (mut cpu, mut ram) = setup(&[0xC091]); // MOV *R1,R2
    set_reg(&mut ram, 1, 0x2000);
    ram.write_word(0x2000, 0xBEEF);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0xBEEF);
}

#[test]
fn autoincrement_advances_by_two_for_word() {
    // MOV *R1+,R2 : word access advances R1 by 2.
    let (mut cpu, mut ram) = setup(&[0xC0B1]); // MOV *R1+,R2
    set_reg(&mut ram, 1, 0x2000);
    ram.write_word(0x2000, 0x1111);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x1111);
    assert_eq!(reg(&mut ram, 1), 0x2002, "word autoincrement is +2");
}

#[test]
fn autoincrement_advances_by_one_for_byte() {
    // MOVB *R1+,R2 : byte access advances R1 by 1.
    let (mut cpu, mut ram) = setup(&[0xD0B1]); // MOVB *R1+,R2
    set_reg(&mut ram, 1, 0x2000);
    ram.write_word(0x2000, 0xABCD);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x2001, "byte autoincrement is +1");
}

#[test]
fn symbolic_addressing_reads_extension_word() {
    // MOV @>2000,R2  (0xC0A0, 0x2000)
    let (mut cpu, mut ram) = setup(&[0xC0A0, 0x2000]);
    ram.write_word(0x2000, 0xCAFE);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0xCAFE);
    assert_eq!(cpu.pc(), PROG + 4, "extension word consumed");
}

#[test]
fn indexed_addressing_adds_register() {
    // MOV @>2000(R1),R2 with R1=4 -> reads >2004
    let (mut cpu, mut ram) = setup(&[0xC0A1, 0x2000]);
    set_reg(&mut ram, 1, 4);
    ram.write_word(0x2004, 0xD00D);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0xD00D);
}

#[test]
fn indexed_addressing_with_r0_stays_pure_symbolic() {
    // `@>2000(R0)` encodes identically to `@>2000` (0xC0A0: a symbolic source, the
    // Ts=2/S=0 field). On the TMS9900 register 0 is NOT usable as an index, so even
    // a *nonzero* R0 must be ignored and the effective address stay >2000 — this
    // pins the `reg == 0` branch of `resolve`.
    let (mut cpu, mut ram) = setup(&[0xC0A0, 0x2000]); // MOV @>2000(R0),R2
    set_reg(&mut ram, 0, 4); // if R0 were used as an index, the EA would be >2004
    ram.write_word(0x2000, 0xBEEF);
    ram.write_word(0x2004, 0xDEAD); // the "indexed" cell must NOT be read
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0xBEEF, "R0 is not an index: EA stays >2000");
}

// --------------------------------------------------------------------------
// Byte operations and parity
// --------------------------------------------------------------------------

#[test]
fn movb_moves_high_byte_and_sets_parity() {
    // MOVB R1,R2 : the high byte of R1 (odd parity 0x07 = 3 ones) moves to the
    // high byte of R2; OP (parity) is set for byte ops.
    let (mut cpu, mut ram) = setup(&[0xD081]); // MOVB R1,R2
    set_reg(&mut ram, 1, 0x0700); // high byte 0x07 -> three 1 bits -> odd
    set_reg(&mut ram, 2, 0x00FF); // low byte must survive
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x07FF);
    assert!(cpu.st() & ST_OP != 0, "odd parity high byte sets OP");
}

#[test]
fn movb_even_parity_clears_op() {
    // high byte 0x03 = two 1 bits = even parity -> OP clear
    let (mut cpu, mut ram) = setup(&[0xD081]);
    set_reg(&mut ram, 1, 0x0300);
    cpu.set_st(ST_OP);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_OP == 0);
}

// --------------------------------------------------------------------------
// Jumps and branches
// --------------------------------------------------------------------------

#[test]
fn jeq_taken_and_not_taken() {
    // CI R1,1 (sets EQ) ; JEQ +2 ; LI R5,>DEAD ; (skipped) ; LI R6,>BEEF
    // JEQ disp = +1 word means skip the next 1-word... displacement is in words.
    // Program: CI R1,1 (2 words) ; JEQ +2 ; LI R5,>DEAD (2 words) ; LI R6,>BEEF
    let (mut cpu, mut ram) = setup(&[
        0x0281, 0x0001, // CI R1,1
        0x1302, // JEQ +2  (skip the LI R5)
        0x0205, 0xDEAD, // LI R5,>DEAD  (should be skipped)
        0x0206, 0xBEEF, // LI R6,>BEEF
    ]);
    set_reg(&mut ram, 1, 1);
    cpu.step(&mut ram); // CI -> EQ set
    cpu.step(&mut ram); // JEQ taken
    cpu.step(&mut ram); // executes LI R6 (R5 skipped)
    assert_eq!(reg(&mut ram, 5), 0, "LI R5 was skipped");
    assert_eq!(reg(&mut ram, 6), 0xBEEF);
}

#[test]
fn jmp_backwards_displacement_is_signed() {
    // A genuinely negative displacement: LI R1,1 (two words), then JMP -3.
    // The jump sits at PROG+4; after its fetch PC = PROG+6, and -3 words is
    // PROG+6 - 6 = PROG — back to the start of the program.
    let (mut cpu, mut ram) = setup(&[0x0201, 0x0001, 0x10FD]); // LI R1,1 ; JMP -3
    cpu.step(&mut ram); // LI
    cpu.step(&mut ram); // JMP -3
    assert_eq!(cpu.pc(), PROG, "JMP -3 lands back on the first word");
}

/// Run a single conditional-jump instruction with the status register forced
/// to `st`, and report whether the jump was taken (displacement +4 words).
fn jump_taken(insn: u16, st: u16) -> bool {
    let (mut cpu, mut ram) = setup(&[insn | 0x04]);
    cpu.set_st(st);
    cpu.step(&mut ram);
    match cpu.pc() {
        p if p == PROG + 2 + 8 => true,
        p if p == PROG + 2 => false,
        p => panic!("insn >{insn:04X} with ST >{st:04X} left PC at >{p:04X}"),
    }
}

#[test]
fn conditional_jumps_take_and_fall_through_per_the_datasheet() {
    // Every jump opcode (>10xx..>1Cxx), with flag states that must take it and
    // flag states that must not. The L>/A>/EQ boolean combinations (JLE, JHE,
    // JL, JH especially) are the classic TMS9900 emulator bug site.
    #[rustfmt::skip]
    let cases: &[(&str, u16, &[u16], &[u16])] = &[
        // name    opcode   taken when ..............   not taken when ........
        ("JMP",   0x1000, &[0, ST_LGT | ST_EQ][..],    &[][..]),
        ("JLT",   0x1100, &[0],                        &[ST_AGT, ST_EQ]),
        ("JLE",   0x1200, &[0, ST_EQ, ST_LGT | ST_EQ], &[ST_LGT]),
        ("JEQ",   0x1300, &[ST_EQ],                    &[0, ST_LGT]),
        ("JHE",   0x1400, &[ST_LGT, ST_EQ],            &[0]),
        ("JGT",   0x1500, &[ST_AGT],                   &[0, ST_EQ]),
        ("JNE",   0x1600, &[0, ST_LGT],                &[ST_EQ]),
        ("JNC",   0x1700, &[0],                        &[ST_C]),
        ("JOC",   0x1800, &[ST_C],                     &[0]),
        ("JNO",   0x1900, &[0],                        &[ST_OV]),
        ("JL",    0x1A00, &[0],                        &[ST_LGT, ST_EQ, ST_LGT | ST_EQ]),
        ("JH",    0x1B00, &[ST_LGT],                   &[0, ST_EQ, ST_LGT | ST_EQ]),
        ("JOP",   0x1C00, &[ST_OP],                    &[0]),
    ];
    for (name, opcode, taken, not_taken) in cases {
        for &st in *taken {
            assert!(jump_taken(*opcode, st), "{name} must jump when ST=>{st:04X}");
        }
        for &st in *not_taken {
            assert!(!jump_taken(*opcode, st), "{name} must fall through when ST=>{st:04X}");
        }
    }
}

#[test]
fn ab_sets_carry_overflow_and_result_parity() {
    // AB R1,R2 — byte add on the registers' HIGH bytes. >FF + >01 wraps to
    // >00 with a carry out and even (zero) parity.
    let (mut cpu, mut ram) = setup(&[0xB081]);
    ram.write_word(WP + 2, 0xFF00); // R1 byte >FF
    ram.write_word(WP + 4, 0x0100); // R2 byte >01
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2) & 0xFF00, 0x0000);
    assert!(cpu.st() & ST_C != 0, "byte carry out");
    assert!(cpu.st() & ST_EQ != 0);
    assert!(cpu.st() & ST_OP == 0, ">00 parity is even");

    // >7F + >01 = >80: signed byte overflow, no carry, odd parity, negative.
    let (mut cpu, mut ram) = setup(&[0xB081]);
    ram.write_word(WP + 2, 0x0100);
    ram.write_word(WP + 4, 0x7F00);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2) & 0xFF00, 0x8000);
    assert!(cpu.st() & ST_OV != 0, "signed byte overflow");
    assert!(cpu.st() & ST_C == 0);
    assert!(cpu.st() & ST_OP != 0, ">80 parity is odd");
    assert!(cpu.st() & ST_AGT == 0);
    assert!(cpu.st() & ST_LGT != 0);
}

#[test]
fn sb_borrow_and_no_borrow_drive_the_carry_flag() {
    // SB R1,R2 : R2 -= R1 on the high bytes. >00 - >01 = >FF borrows, so the
    // TMS9900's C ("no borrow") flag is CLEAR.
    let (mut cpu, mut ram) = setup(&[0x7081]);
    ram.write_word(WP + 2, 0x0100);
    ram.write_word(WP + 4, 0x0000);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2) & 0xFF00, 0xFF00);
    assert!(cpu.st() & ST_C == 0, "borrow clears carry");
    assert!(cpu.st() & ST_OP == 0, ">FF parity is even");
    assert!(cpu.st() & ST_LGT != 0);
    assert!(cpu.st() & ST_AGT == 0);

    // >05 - >01 = >04: no borrow sets carry; >04 has odd parity.
    let (mut cpu, mut ram) = setup(&[0x7081]);
    ram.write_word(WP + 2, 0x0100);
    ram.write_word(WP + 4, 0x0500);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2) & 0xFF00, 0x0400);
    assert!(cpu.st() & ST_C != 0, "no borrow sets carry");
    assert!(cpu.st() & ST_OP != 0, ">04 parity is odd");
}

#[test]
fn cb_compares_and_takes_parity_from_the_source_byte() {
    // CB R1,R2 with source >07, destination >90: flags compare src to dst,
    // OP comes from the SOURCE byte (not a result), and nothing is stored.
    let (mut cpu, mut ram) = setup(&[0x9081]);
    ram.write_word(WP + 2, 0x0700);
    ram.write_word(WP + 4, 0x9000);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x9000, "CB stores nothing");
    assert!(cpu.st() & ST_OP != 0, "parity from source >07 (three bits)");
    assert!(cpu.st() & ST_LGT == 0, ">07 is not logically higher than >90");
    assert!(cpu.st() & ST_AGT != 0, ">07 beats >90 signed (>90 is negative)");
    assert!(cpu.st() & ST_EQ == 0);
}

#[test]
fn socb_and_szcb_byte_logic_set_result_flags_and_parity() {
    // SOCB R1,R2 : R2 |= R1 -> >F0 | >0F = >FF (even parity, L> set).
    let (mut cpu, mut ram) = setup(&[0xF081]);
    ram.write_word(WP + 2, 0xF000);
    ram.write_word(WP + 4, 0x0F00);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2) & 0xFF00, 0xFF00);
    assert!(cpu.st() & ST_OP == 0, ">FF parity is even");
    assert!(cpu.st() & ST_LGT != 0);

    // SZCB R1,R2 : R2 &= !R1 -> >FF & !>0F = >F0.
    let (mut cpu, mut ram) = setup(&[0x5081]);
    ram.write_word(WP + 2, 0x0F00);
    ram.write_word(WP + 4, 0xFF00);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2) & 0xFF00, 0xF000);
    assert!(cpu.st() & ST_OP == 0, ">F0 parity is even");
    assert!(cpu.st() & ST_EQ == 0);
}

#[test]
fn movb_to_an_odd_address_writes_the_low_byte_and_preserves_the_high() {
    // MOVB R1,@>2001 — the byte lands in the LOW half of the word at >2000
    // via read-modify-write; the high byte must survive. No prior test ever
    // drove a byte access at an odd address.
    let (mut cpu, mut ram) = setup(&[0xD801, 0x2001]);
    ram.write_word(WP + 2, 0xAB00); // R1 byte >AB
    ram.write_word(0x2000, 0x1234);
    cpu.step(&mut ram);
    assert_eq!(ram.read_word(0x2000), 0x12AB);
    assert!(cpu.st() & ST_OP != 0, ">AB has odd parity (five bits)");
    assert!(cpu.st() & ST_AGT == 0, ">AB is negative as a byte");
    assert!(cpu.st() & ST_LGT != 0);
}

#[test]
fn byte_autoincrement_steps_the_register_by_one() {
    // MOVB *R3+,R4 : loads the byte at >2000, then R3 = >2001 (byte mode
    // increments by 1, not 2); the byte write to R4 preserves its low byte.
    let (mut cpu, mut ram) = setup(&[0xD133]);
    ram.write_word(WP + 6, 0x2000); // R3
    ram.write_word(WP + 8, 0x00CD); // R4 low byte must survive
    ram.write_word(0x2000, 0x5678);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 3), 0x2001, "byte mode increments by 1");
    assert_eq!(reg(&mut ram, 4), 0x56CD, "high byte loaded, low byte kept");
}

#[test]
fn x_executes_the_sourced_instruction_with_its_flag_effects() {
    // X R5 where R5 holds "A R1,R2". The addition must run exactly as if it
    // were inline: result stored, carry/EQ set, PC advanced past X only.
    let (mut cpu, mut ram) = setup(&[0x0485]); // X R5
    ram.write_word(WP + 10, 0xA081); // R5 = A R1,R2
    ram.write_word(WP + 2, 0xFFFF); // R1
    ram.write_word(WP + 4, 0x0001); // R2
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x0000, ">FFFF + 1 wraps");
    assert!(cpu.st() & ST_C != 0, "carry from the executed A");
    assert!(cpu.st() & ST_EQ != 0);
    assert_eq!(cpu.pc(), PROG + 2, "X itself is one word");
}

#[test]
fn x_fetches_the_executed_instructions_extension_words_from_pc() {
    // X R5 where R5 holds "LI R1,imm". Per the TMS9900 data manual the
    // executed instruction's extension words come from PC — the word AFTER
    // the X — and PC advances past them.
    let (mut cpu, mut ram) = setup(&[0x0485, 0x1234]); // X R5 ; (imm)
    ram.write_word(WP + 10, 0x0201); // R5 = LI R1,imm
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x1234, "immediate taken from after the X");
    assert_eq!(cpu.pc(), PROG + 4, "PC advanced past X and the immediate");
}

#[test]
fn autoincrement_source_and_dest_sharing_a_register() {
    // MOV *R1+,*R1+ : the source uses R1 then bumps it; the destination uses
    // the BUMPED R1 and bumps it again — so buf[0] is copied to buf[1] and R1
    // ends +4. This ordering is the one CLAUDE.md flags as subtle.
    let (mut cpu, mut ram) = setup(&[0xCC71]);
    ram.write_word(WP + 2, 0x2000); // R1
    ram.write_word(0x2000, 0xBEEF);
    ram.write_word(0x2002, 0x0000);
    cpu.step(&mut ram);
    assert_eq!(ram.read_word(0x2002), 0xBEEF, "buf[0] copied into buf[1]");
    assert_eq!(reg(&mut ram, 1), 0x2004, "R1 bumped once per operand");

    // C *R2+,*R2+ : compares buf[0] to buf[1]; R2 also ends +4.
    let (mut cpu, mut ram) = setup(&[0x8CB2]);
    ram.write_word(WP + 4, 0x2000); // R2
    ram.write_word(0x2000, 0x1111);
    ram.write_word(0x2002, 0x1111);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_EQ != 0, "equal words compare equal");
    assert_eq!(reg(&mut ram, 2), 0x2004);
}

#[test]
fn source_value_is_read_before_a_dest_autoincrement_rewrites_it() {
    // A R3,*R3+ : the source operand is R3's register cell, and the
    // destination autoincrement REWRITES that cell. The hardware fetches the
    // source value before destination decode, so the OLD R3 is what gets
    // added at *old R3 (Classic99 fixS/op/fixD order agrees).
    let (mut cpu, mut ram) = setup(&[0xACC3]);
    ram.write_word(WP + 6, 0x2000); // R3
    ram.write_word(0x2000, 0x0011);
    cpu.step(&mut ram);
    assert_eq!(
        ram.read_word(0x2000),
        0x2011,
        "the pre-increment R3 (>2000) is the addend, not >2002"
    );
    assert_eq!(reg(&mut ram, 3), 0x2002);

    // MOV R3,*R3+ : same shape — the OLD R3 value is what lands at *old R3.
    let (mut cpu, mut ram) = setup(&[0xCCC3]);
    ram.write_word(WP + 6, 0x2000);
    ram.write_word(0x2000, 0xFFFF);
    cpu.step(&mut ram);
    assert_eq!(ram.read_word(0x2000), 0x2000, "the pre-increment R3 is stored");
    assert_eq!(reg(&mut ram, 3), 0x2002);
}

#[test]
fn illegal_opcodes_are_counted_but_change_no_state() {
    // >0000 is undefined. Each one costs a nominal 6 cycles, bumps the
    // diagnostic counter (so a program wedged in data is visible from
    // outside), and leaves the architectural state alone.
    let (mut cpu, mut ram) = setup(&[0x0000, 0x0000]);
    assert_eq!(cpu.illegal_count(), 0);
    cpu.step(&mut ram);
    cpu.step(&mut ram);
    assert_eq!(cpu.illegal_count(), 2);
    assert_eq!(cpu.pc(), PROG + 4, "each fetch still advances PC");
    assert_eq!(cpu.st(), 0, "flags untouched");
}

#[test]
fn coc_and_czc_test_bit_subsets_into_eq() {
    // COC R1,R2 : EQ iff every 1 bit of the source is also 1 in R2.
    let (mut cpu, mut ram) = setup(&[0x2081]);
    ram.write_word(WP + 2, 0x0030);
    ram.write_word(WP + 4, 0x0070);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_EQ != 0, "subset -> EQ set");

    let (mut cpu, mut ram) = setup(&[0x2081]);
    ram.write_word(WP + 2, 0x0031);
    ram.write_word(WP + 4, 0x0070);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_EQ == 0, "stray bit 0 -> EQ clear");

    // CZC R1,R2 : EQ iff every 1 bit of the source is 0 in R2.
    let (mut cpu, mut ram) = setup(&[0x2481]);
    ram.write_word(WP + 2, 0x000F);
    ram.write_word(WP + 4, 0xFF00);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_EQ != 0, "disjoint -> EQ set");

    let (mut cpu, mut ram) = setup(&[0x2481]);
    ram.write_word(WP + 2, 0x000F);
    ram.write_word(WP + 4, 0xFF08);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_EQ == 0, "shared bit 3 -> EQ clear");
}

#[test]
fn xop_vectors_and_passes_the_source_address_in_the_new_r11() {
    // XOP R1,2 vectors through >0040 + 4*2 = >0048, saves the caller in the
    // new R13/R14/R15, puts the source's EFFECTIVE ADDRESS in the new R11,
    // and sets ST's X bit.
    let (mut cpu, mut ram) = setup(&[0x2C81]);
    ram.write_word(0x0048, 0x0400); // vector: new WP
    ram.write_word(0x004A, 0x2000); // vector: new PC
    cpu.set_st(0x8003); // distinctive caller status (mask 3)
    cpu.step(&mut ram);
    assert_eq!(cpu.wp(), 0x0400);
    assert_eq!(cpu.pc(), 0x2000);
    assert!(cpu.st() & 0x0200 != 0, "ST_X set while in the XOP routine");
    assert_eq!(ram.read_word(0x0400 + 22), WP + 2, "new R11 = source EA (R1)");
    assert_eq!(ram.read_word(0x0400 + 26), WP, "new R13 = caller WP");
    assert_eq!(ram.read_word(0x0400 + 28), PROG + 2, "new R14 = caller PC");
    assert_eq!(ram.read_word(0x0400 + 30), 0x8003, "new R15 = caller ST");
    assert_eq!(cpu.st() & 0x000F, 0x0003, "XOP leaves the interrupt mask alone");
}

#[test]
fn stwp_and_stst_store_the_internal_registers() {
    let (mut cpu, mut ram) = setup(&[0x02A3, 0x02C4]); // STWP R3 ; STST R4
    cpu.set_st(0x8002);
    cpu.step(&mut ram);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 3), WP, "STWP stores the workspace pointer");
    assert_eq!(reg(&mut ram, 4), 0x8002, "STST stores the status register");
}

#[test]
fn limi_gates_interrupt_acceptance_and_idle_waits_for_one() {
    // LIMI 2 ; IDLE — with a level-1 request pending the whole time. The
    // request is not honored while the mask is 0, LIMI raises the mask, IDLE
    // parks the CPU (burning cycles, PC pinned), and the next boundary
    // accepts the interrupt through the >0004 vector.
    let (mut cpu, mut ram) = setup(&[0x0300, 0x0002, 0x0340]);
    ram.write_word(0x0004, 0x0500); // level-1 vector: new WP
    ram.write_word(0x0006, 0x2000); // level-1 vector: new PC
    cpu.set_interrupt_request(Some(1));

    cpu.step(&mut ram); // LIMI 2 (not preempted: level 1 > mask 0)
    assert_eq!(cpu.pc(), PROG + 4, "LIMI ran despite the pending request");
    assert_eq!(cpu.st() & 0x000F, 2, "mask loaded");

    // The very next boundary accepts (level 1 <= mask 2) — IDLE never runs.
    cpu.step(&mut ram);
    assert_eq!(cpu.wp(), 0x0500, "interrupt context switch");
    assert_eq!(cpu.pc(), 0x2000);
    assert_eq!(cpu.st() & 0x000F, 0, "mask lowered to level-1");

    // IDLE on its own: parks at the same PC, burning cycles each step.
    let (mut cpu, mut ram) = setup(&[0x0340]);
    cpu.step(&mut ram); // IDLE
    let parked_pc = cpu.pc();
    let c0 = cpu.cycles();
    cpu.step(&mut ram);
    cpu.step(&mut ram);
    assert_eq!(cpu.pc(), parked_pc, "IDLE holds the PC");
    assert!(cpu.cycles() > c0, "idling still consumes cycles");
    // An accepted interrupt is what wakes it.
    ram.write_word(0x0004, 0x0500);
    ram.write_word(0x0006, 0x2000);
    cpu.set_st(0x0002);
    cpu.set_interrupt_request(Some(1));
    cpu.step(&mut ram);
    assert_eq!(cpu.pc(), 0x2000, "interrupt acceptance ends IDLE");
}

#[test]
fn inv_inct_dect_flags() {
    // INV R1 : ones complement, L>/A>/EQ from the result.
    let (mut cpu, mut ram) = setup(&[0x0541]);
    ram.write_word(WP + 2, 0x00FF);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0xFF00);
    assert!(cpu.st() & ST_LGT != 0);
    assert!(cpu.st() & ST_AGT == 0, "result is negative");

    // INCT R1 : >7FFF + 2 overflows into the sign bit.
    let (mut cpu, mut ram) = setup(&[0x05C1]);
    ram.write_word(WP + 2, 0x7FFF);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x8001);
    assert!(cpu.st() & ST_OV != 0, "signed overflow");
    assert!(cpu.st() & ST_C == 0);

    // DECT R1 : 1 - 2 borrows (C clear on the TMS9900).
    let (mut cpu, mut ram) = setup(&[0x0641]);
    ram.write_word(WP + 2, 0x0001);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0xFFFF);
    assert!(cpu.st() & ST_C == 0, "borrow clears carry");
}

#[test]
fn blwp_rtwp_round_trips_the_full_status_including_the_mask() {
    // BLWP @>3000 ; ... vector WP=>0400, PC=>2000 where an RTWP sits. The
    // caller's ST — flags AND interrupt mask — must come back exactly.
    let (mut cpu, mut ram) = setup(&[0x0420, 0x3000]);
    ram.write_word(0x3000, 0x0400);
    ram.write_word(0x3002, 0x2000);
    ram.write_word(0x2000, 0x0380); // RTWP
    cpu.set_st(0x9005); // L> + C + mask 5
    cpu.step(&mut ram); // BLWP
    assert_eq!(ram.read_word(0x0400 + 30), 0x9005, "new R15 holds caller ST");
    cpu.step(&mut ram); // RTWP
    assert_eq!(cpu.wp(), WP);
    assert_eq!(cpu.pc(), PROG + 4);
    assert_eq!(cpu.st(), 0x9005, "ST restored exactly, mask included");
}

#[test]
fn x_of_a_jump_takes_the_displacement_from_the_post_x_pc() {
    // X R5 where R5 holds "JMP +2". The displacement is relative to the PC
    // after the X instruction word (PROG+2), so the jump lands at PROG+6.
    let (mut cpu, mut ram) = setup(&[0x0485]);
    ram.write_word(WP + 10, 0x1002); // R5 = JMP +2
    cpu.step(&mut ram);
    assert_eq!(cpu.pc(), PROG + 6);
}

#[test]
fn branch_sets_pc_to_effective_address() {
    // B @>2000
    let (mut cpu, mut ram) = setup(&[0x0460, 0x2000]);
    cpu.step(&mut ram);
    assert_eq!(cpu.pc(), 0x2000);
}

#[test]
fn bl_saves_return_address_in_r11() {
    // BL @>2000 — R11 receives the address of the word after the instruction.
    let (mut cpu, mut ram) = setup(&[0x06A0, 0x2000]);
    cpu.step(&mut ram);
    assert_eq!(cpu.pc(), 0x2000);
    assert_eq!(reg(&mut ram, 11), PROG + 4, "R11 = return address");
}

#[test]
fn blwp_then_rtwp_round_trips_context() {
    // BLWP @>2000 where >2000 holds new WP, >2002 holds new PC.
    // The new workspace gets old WP/PC/ST in R13/R14/R15. RTWP restores them.
    let new_wp = 0x2100u16;
    let new_pc = 0x2500u16;
    let (mut cpu, mut ram) = setup(&[0x0420, 0x2000]); // BLWP @>2000
    ram.write_word(0x2000, new_wp);
    ram.write_word(0x2002, new_pc);
    // Put an RTWP at the new PC.
    ram.write_word(new_pc, 0x0380); // RTWP
    let old_wp = cpu.wp();
    cpu.step(&mut ram); // BLWP
    assert_eq!(cpu.wp(), new_wp);
    assert_eq!(cpu.pc(), new_pc);
    assert_eq!(ram.read_word(new_wp + 2 * 13), old_wp, "R13 = old WP");
    assert_eq!(ram.read_word(new_wp + 2 * 14), PROG + 4, "R14 = return PC");
    cpu.step(&mut ram); // RTWP
    assert_eq!(cpu.wp(), old_wp, "RTWP restores WP");
    assert_eq!(cpu.pc(), PROG + 4, "RTWP restores PC");
}

// --------------------------------------------------------------------------
// Shifts
// --------------------------------------------------------------------------

#[test]
fn sla_shifts_left_sets_carry_and_overflow_on_sign_change() {
    // SLA R1,1 with R1 = >4000 -> >8000: the sign bit changed, so OV set; the
    // bit shifted out of the MSB was 0, so C clear.
    let (mut cpu, mut ram) = setup(&[0x0A11]); // SLA R1,1
    set_reg(&mut ram, 1, 0x4000);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x8000);
    assert!(cpu.st() & ST_OV != 0, "sign changed during shift");
    assert!(cpu.st() & ST_C == 0);
}

#[test]
fn srl_logical_right_fills_zero_and_sets_carry() {
    // SRL R1,1 with R1 = >0001 -> 0, carry = the 1 shifted out.
    let (mut cpu, mut ram) = setup(&[0x0911]); // SRL R1,1
    set_reg(&mut ram, 1, 0x0001);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x0000);
    assert!(cpu.st() & ST_C != 0);
}

#[test]
fn sra_arithmetic_right_sign_extends() {
    // SRA R1,1 with R1 = >8000 -> >C000 (sign preserved).
    let (mut cpu, mut ram) = setup(&[0x0811]); // SRA R1,1
    set_reg(&mut ram, 1, 0x8000);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0xC000);
}

#[test]
fn src_rotates_right() {
    // SRC R1,4 with R1 = >000F -> >F000 (the low nibble rotates to the top).
    let (mut cpu, mut ram) = setup(&[0x0B41]); // SRC R1,4
    set_reg(&mut ram, 1, 0x000F);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0xF000);
}

#[test]
fn shift_count_zero_comes_from_r0() {
    // SLA R1,0 uses R0's low nibble as the count. R0=4 -> shift left 4.
    let (mut cpu, mut ram) = setup(&[0x0A01]); // SLA R1,0
    set_reg(&mut ram, 0, 0x0004);
    set_reg(&mut ram, 1, 0x0001);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 1), 0x0010);
}

// --------------------------------------------------------------------------
// Multiply / divide
// --------------------------------------------------------------------------

#[test]
fn mpy_unsigned_32bit_product() {
    // MPY R1,R2 -> R2:R3 = R2 * R1 (unsigned). 0x1000 * 0x0010 = 0x0001_0000.
    let (mut cpu, mut ram) = setup(&[0x3881]); // MPY R1,R2
    set_reg(&mut ram, 1, 0x1000);
    set_reg(&mut ram, 2, 0x0010);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x0001, "high word");
    assert_eq!(reg(&mut ram, 3), 0x0000, "low word");
}

#[test]
fn div_quotient_and_remainder() {
    // DIV R1,R2 -> R2:R3 / R1; quotient->R2, remainder->R3.
    // dividend 0x0001_0001 (=65537) / 0x10 = quotient 0x1000, remainder 1.
    let (mut cpu, mut ram) = setup(&[0x3C81]); // DIV R1,R2
    set_reg(&mut ram, 1, 0x0010);
    set_reg(&mut ram, 2, 0x0001);
    set_reg(&mut ram, 3, 0x0001);
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 2), 0x1000, "quotient");
    assert_eq!(reg(&mut ram, 3), 0x0001, "remainder");
}

#[test]
fn div_overflow_aborts_and_sets_ov() {
    // If the divisor <= the high word of the dividend, the quotient would not fit
    // 16 bits: DIV sets OV and does NOT modify the registers.
    let (mut cpu, mut ram) = setup(&[0x3C81]); // DIV R1,R2
    set_reg(&mut ram, 1, 0x0010); // divisor
    set_reg(&mut ram, 2, 0x0010); // high word == divisor -> overflow
    set_reg(&mut ram, 3, 0x0000);
    cpu.step(&mut ram);
    assert!(cpu.st() & ST_OV != 0);
    assert_eq!(reg(&mut ram, 2), 0x0010, "registers unchanged on overflow");
}

#[test]
fn mpy_r15_low_word_lands_past_workspace_not_r0() {
    // MPY R1,R15 — the second result word lives at the next *address*, not the
    // next register number modulo 16. Registers are memory: the low product
    // word lands at WP+32 (one word past the workspace), never in R0.
    // (Classic99 cpu9900.cpp op_mpy writes WRWORD(D+2, ...) — plain address
    // arithmetic.)
    let (mut cpu, mut ram) = setup(&[0x3BC1]); // MPY R1,R15
    set_reg(&mut ram, 1, 3); // source
    set_reg(&mut ram, 15, 5); // destination (product high lands here)
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 15), 0, "product high word in R15");
    assert_eq!(
        ram.read_word(WP + 32),
        15,
        "product low word at WP+32 (past R15), not in R0"
    );
    assert_eq!(reg(&mut ram, 0), 0, "R0 must be untouched");
}

#[test]
fn div_r15_uses_the_word_past_workspace_not_r0() {
    // DIV R1,R15 — the dividend's low word is read from WP+32 and the
    // remainder is written back there; R0 plays no part (same address
    // arithmetic as MPY above).
    let (mut cpu, mut ram) = setup(&[0x3FC1]); // DIV R1,R15
    set_reg(&mut ram, 1, 7); // divisor
    set_reg(&mut ram, 15, 0); // dividend high word
    ram.write_word(WP + 32, 23); // dividend low word, past the workspace
    set_reg(&mut ram, 0, 0xABCD); // sentinel: must be neither read nor written
    cpu.step(&mut ram);
    assert_eq!(reg(&mut ram, 15), 3, "quotient in R15 (23 / 7)");
    assert_eq!(ram.read_word(WP + 32), 2, "remainder at WP+32 (23 % 7)");
    assert_eq!(reg(&mut ram, 0), 0xABCD, "R0 must be untouched");
}

// --------------------------------------------------------------------------
// CRU instructions
// --------------------------------------------------------------------------

#[test]
fn sbo_sbz_tb_single_bits() {
    // R12 = >0020 -> CRU bit base = >0010. SBO 1 sets bit >0011; TB 1 reads it.
    let (mut cpu, mut ram) = setup(&[0x1D01, 0x1F01]); // SBO 1 ; TB 1
    set_reg(&mut ram, 12, 0x0020);
    cpu.step(&mut ram); // SBO 1
    assert!(ram.get_cru(0x0011), "SBO drove the CRU bit high");
    cpu.step(&mut ram); // TB 1
    assert!(cpu.st() & ST_EQ != 0, "TB copies the tested bit into EQ");
}

#[test]
fn ldcr_and_stcr_transfer_multiple_bits() {
    // R12=>0020 -> base >0010. For <=8 bits the source/destination is a *byte*,
    // and a register's byte is its HIGH byte (even address). So the value 0xA5
    // must sit in the high byte: R1 = >A500. LDCR sends its 8 bits LSB-first to
    // CRU bits >0010..>0017; STCR reads them back into R2's high byte.
    let (mut cpu, mut ram) = setup(&[0x3201, 0x3602]); // LDCR R1,8 ; STCR R2,8
    set_reg(&mut ram, 12, 0x0020);
    set_reg(&mut ram, 1, 0xA500); // high byte 1010_0101
    cpu.step(&mut ram); // LDCR
    assert!(ram.get_cru(0x0010), "bit0 = 1");
    assert!(!ram.get_cru(0x0011), "bit1 = 0");
    assert!(ram.get_cru(0x0017), "bit7 = 1");
    cpu.step(&mut ram); // STCR
    assert_eq!(reg(&mut ram, 2) >> 8, 0xA5, "round-trip 8 bits into high byte");
}

// --------------------------------------------------------------------------
// Interrupts
// --------------------------------------------------------------------------

#[test]
fn interrupt_taken_when_mask_allows() {
    // Vector for level 1 is at >0004 (WP) / >0006 (PC).
    let mut ram = FlatRam::new();
    ram.load_words(0x0004, &[0x2100, 0x2500]); // ISR WP, PC
    ram.load_words(PROG, &[0x0205, 0x1111]); // LI R5 — would run if no interrupt
    let mut cpu = Cpu::new();
    cpu.set_wp(WP);
    cpu.set_pc(PROG);
    cpu.set_st(0x000F); // mask = 15, so level-1 is allowed
    let old_wp = cpu.wp();
    cpu.set_interrupt_request(Some(1));
    cpu.step(&mut ram); // should take the interrupt, not run LI R5
    assert_eq!(cpu.wp(), 0x2100);
    assert_eq!(cpu.pc(), 0x2500);
    assert_eq!(ram.read_word(0x2100 + 2 * 13), old_wp, "R13 = old WP");
    assert_eq!(ram.read_word(0x2100 + 2 * 14), PROG, "R14 = interrupted PC");
    assert_eq!(cpu.st() & 0x000F, 0, "mask lowered to level-1 minus 1 = 0");
}

#[test]
fn interrupt_ignored_when_masked() {
    let mut ram = FlatRam::new();
    ram.load_words(0x0004, &[0x2100, 0x2500]);
    ram.load_words(PROG, &[0x0205, 0x1111]); // LI R5,>1111
    let mut cpu = Cpu::new();
    cpu.set_wp(WP);
    cpu.set_pc(PROG);
    cpu.set_st(0x0000); // mask = 0 -> even level 1 is masked
    cpu.set_interrupt_request(Some(1));
    cpu.step(&mut ram); // runs the normal instruction
    assert_eq!(reg(&mut ram, 5), 0x1111);
    assert_eq!(cpu.wp(), WP, "no context switch");
}

// --------------------------------------------------------------------------
// Cycle counts
//
// The test bus is `FlatRam`, which has **zero wait states**, so the TMS9900
// data-manual base cycle counts apply directly: `cpu.step()` returns exactly
// the instruction's base cost plus its addressing-mode add-ons, with no memory
// wait states mixed in. Addressing add-ons charged by `resolve`: register `Rn`
// +0, indirect `*Rn` +4, symbolic/indexed `@A(Rn)` +8, autoincrement `*Rn+` +6
// (byte) / +8 (word). Base costs verified against Classic99 `cpu9900.cpp`.
// --------------------------------------------------------------------------

#[test]
fn mov_register_mode_costs_14_cycles() {
    // MOV R1,R2 (0xC081): base 14 + src Rn +0 + dst Rn +0 = 14.
    let (mut cpu, mut ram) = setup(&[0xC081]);
    set_reg(&mut ram, 1, 0x1234);
    assert_eq!(cpu.step(&mut ram), 14);
}

#[test]
fn mov_symbolic_source_costs_14_plus_8() {
    // MOV @>1100,R2 (0xC0A0, 0x1100): base 14 + symbolic src +8 + dst Rn +0 = 22.
    let (mut cpu, mut ram) = setup(&[0xC0A0, 0x1100]);
    ram.write_word(0x1100, 0x55AA);
    assert_eq!(cpu.step(&mut ram), 22);
    assert_eq!(reg(&mut ram, 2), 0x55AA, "sanity: the symbolic MOV ran");
}

#[test]
fn shift_immediate_count_costs_12_plus_2c() {
    // SLA R1,4 (0x0A41): count = 4 from the instruction field, so 12 + 2*4 = 20.
    let (mut cpu, mut ram) = setup(&[0x0A41]);
    set_reg(&mut ram, 1, 0x0001);
    assert_eq!(cpu.step(&mut ram), 20);
}

#[test]
fn shift_count_from_r0_costs_20_plus_2c() {
    // SLA R1,0 (0x0A01) with R0 low nibble = 4: the count comes from R0, which
    // adds +8 over the immediate form, so 12 + 8 + 2*4 = 28 = 20 + 2*4.
    let (mut cpu, mut ram) = setup(&[0x0A01]);
    set_reg(&mut ram, 0, 0x0004);
    set_reg(&mut ram, 1, 0x0001);
    assert_eq!(cpu.step(&mut ram), 28);
}

#[test]
fn shift_r0_low_nibble_zero_means_count_16() {
    // SLA R1,0 (0x0A01) with R0 = >0010: the low nibble is 0, which means a
    // count of 16 (not 0). Cost = 12 + 8 (R0 read) + 2*16 = 52, and R1's single
    // set bit shifts entirely out (>0001 << 16 = 0), pinning the count at 16.
    let (mut cpu, mut ram) = setup(&[0x0A01]);
    set_reg(&mut ram, 0, 0x0010); // nonzero word, but low nibble = 0
    set_reg(&mut ram, 1, 0x0001);
    assert_eq!(cpu.step(&mut ram), 52);
    assert_eq!(reg(&mut ram, 1), 0x0000, "shifted left 16 -> all bits gone");
}

#[test]
fn mpy_costs_52_cycles() {
    // MPY R1,R2 (0x3881): base 52 + src Rn +0 = 52.
    let (mut cpu, mut ram) = setup(&[0x3881]);
    set_reg(&mut ram, 1, 0x1000);
    set_reg(&mut ram, 2, 0x0010);
    assert_eq!(cpu.step(&mut ram), 52);
}

#[test]
fn div_success_costs_92_cycles() {
    // DIV R1,R2 (0x3C81), successful: a flat 92 (a deliberate approximation of
    // hardware's data-dependent 92–124) + src Rn +0 = 92.
    let (mut cpu, mut ram) = setup(&[0x3C81]);
    set_reg(&mut ram, 1, 0x0010);
    set_reg(&mut ram, 2, 0x0001);
    set_reg(&mut ram, 3, 0x0001);
    assert_eq!(cpu.step(&mut ram), 92);
}

#[test]
fn div_overflow_abort_costs_16_cycles() {
    // DIV R1,R2 (0x3C81) with divisor <= high word: the overflow abort short-
    // circuits at 16 cycles + src Rn +0 = 16.
    let (mut cpu, mut ram) = setup(&[0x3C81]);
    set_reg(&mut ram, 1, 0x0010);
    set_reg(&mut ram, 2, 0x0010); // high word == divisor -> overflow
    set_reg(&mut ram, 3, 0x0000);
    assert_eq!(cpu.step(&mut ram), 16);
    assert!(cpu.st() & ST_OV != 0, "sanity: overflow path was taken");
}

#[test]
fn interrupt_acceptance_costs_22_cycles() {
    // Accepting an interrupt is a context switch charged at a flat 22 cycles
    // (the memory transfers add only wait states, which are 0 on FlatRam).
    let mut ram = FlatRam::new();
    ram.load_words(0x0004, &[0x2100, 0x2500]); // level-1 ISR WP, PC
    ram.load_words(PROG, &[0x0205, 0x1111]);
    let mut cpu = Cpu::new();
    cpu.set_wp(WP);
    cpu.set_pc(PROG);
    cpu.set_st(0x000F); // mask allows level 1
    cpu.set_interrupt_request(Some(1));
    assert_eq!(cpu.step(&mut ram), 22);
    assert_eq!(cpu.pc(), 0x2500, "sanity: the interrupt was accepted");
}

#[test]
fn ldcr_costs_20_plus_2c() {
    // LDCR R1,8 (0x3201): base 20 + 2*8 = 36 + src Rn +0 = 36. LDCR keeps the
    // 20+2C formula (unlike STCR, which is a fixed table).
    let (mut cpu, mut ram) = setup(&[0x3201]);
    set_reg(&mut ram, 12, 0x0020);
    set_reg(&mut ram, 1, 0xA500);
    assert_eq!(cpu.step(&mut ram), 36);
}

#[test]
fn ldcr_symbolic_source_adds_addressing_cycles() {
    // LDCR @>1100,8 (0x3220, 0x1100): base 20 + 2*8 = 36 + symbolic src +8 = 44.
    // (8 bits => byte operand; the byte read is the high byte of the word.)
    let (mut cpu, mut ram) = setup(&[0x3220, 0x1100]);
    set_reg(&mut ram, 12, 0x0020);
    ram.write_word(0x1100, 0xA500);
    assert_eq!(cpu.step(&mut ram), 44);
}

#[test]
fn stcr_cycle_table_by_count() {
    // STCR uses fixed base costs by count (NOT LDCR's 20+2C). Register operand
    // (+0), so `step()` returns the base directly. Encoding STCR Rs,C is
    // 0x3400 | (C<<6) | s, with a count field of 0 meaning 16.
    //   C=7  (STCR R2,7  = 0x35C2) -> 42   (C≤7)
    //   C=8  (STCR R2,8  = 0x3602) -> 44
    //   C=15 (STCR R2,15 = 0x37C2) -> 58   (9≤C≤15)
    //   C=16 (STCR R2,0  = 0x3402) -> 60   (field 0 == 16 bits)
    let cases = [
        (0x35C2u16, 42u32),
        (0x3602, 44),
        (0x37C2, 58),
        (0x3402, 60),
    ];
    for (insn, expected) in cases {
        let (mut cpu, mut ram) = setup(&[insn]);
        set_reg(&mut ram, 12, 0x0020);
        assert_eq!(cpu.step(&mut ram), expected, "STCR insn {insn:#06X}");
    }
}

#[test]
fn stcr_symbolic_dest_adds_addressing_cycles() {
    // STCR @>1102,7 (0x35E0, 0x1102): base 42 (C≤7) + symbolic dst +8 = 50.
    let (mut cpu, mut ram) = setup(&[0x35E0, 0x1102]);
    set_reg(&mut ram, 12, 0x0020);
    assert_eq!(cpu.step(&mut ram), 50);
}
