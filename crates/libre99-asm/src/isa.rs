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

//! The TMS9900 instruction table the encoder and the [`crate::disasm`] decoder
//! are both driven by (NFR-4: one diffable source of truth).
//!
//! Complete: all 69 base opcodes across all nine instruction formats —
//! dual-operand (I), jumps (II) and the CRU single-bit ops that share its shape,
//! `COC`/`CZC`/`XOR` + `MPY`/`DIV`/`XOP` (III/IX), CRU multi-bit (IV), shifts (V),
//! single-operand (VI), no-operand control (VII), and the immediate /
//! store-register forms (VIII) — plus the `NOP`/`RT` pseudo-ops. Bases are
//! cross-checked against `libre99-core`'s `cpu.rs` decoder and `ASSEMBLER.md`
//! Appendix A.

/// Instruction format (operand shape), selecting which encoder runs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Fmt {
    /// Format I: two general operands; the byte variant is baked into `base`.
    Dual,
    /// Format II: a single signed word displacement (jumps).
    Jump,
    /// Format VI: one general operand.
    Single,
    /// Format VIII: a register field plus an immediate word.
    ImmReg,
    /// Format VIII: an immediate word with no register field (`LWPI`/`LIMI`).
    Imm,
    /// A pseudo-instruction emitting `base` verbatim with no operands.
    Pseudo,
    /// Formats III/IX: a general source plus a 4-bit field in bits 6–9 — a
    /// register (`COC`/`CZC`/`XOR`/`MPY`/`DIV`) or a count (`XOP`).
    RegSrc,
    /// Format IV: a general source plus a 0–15 transfer count (`LDCR`/`STCR`).
    CruMulti,
    /// Format V: a workspace register plus a 0–15 shift count (`SLA`/`SRA`/…).
    Shift,
    /// Format II shape, but the operand is a signed CRU-bit *displacement value*
    /// added to `R12>>1` — not a jump target (`SBO`/`SBZ`/`TB`).
    CruBit,
    /// Format VIII: a register field, no immediate (`STWP`/`STST`).
    RegOnly,
    /// Format VII: no operands (`RTWP`/`IDLE`/`RSET`/`CKON`/`CKOF`/`LREX`).
    Control,
}

/// One instruction definition.
pub struct Insn {
    pub name: &'static str,
    pub base: u16,
    pub fmt: Fmt,
}

/// Look up an instruction by its (already upper-cased) mnemonic.
pub fn lookup(name_upper: &str) -> Option<&'static Insn> {
    TABLE.iter().find(|i| i.name == name_upper)
}

/// The full instruction table — the single source of truth the encoder and the
/// [`crate::disasm`] decoder both drive from (NFR-4: one diffable table).
pub fn table() -> &'static [Insn] {
    TABLE
}

use Fmt::*;

#[rustfmt::skip]
static TABLE: &[Insn] = &[
    // Format I — two general operands.
    Insn { name: "SZC",  base: 0x4000, fmt: Dual }, Insn { name: "SZCB", base: 0x5000, fmt: Dual },
    Insn { name: "S",    base: 0x6000, fmt: Dual }, Insn { name: "SB",   base: 0x7000, fmt: Dual },
    Insn { name: "C",    base: 0x8000, fmt: Dual }, Insn { name: "CB",   base: 0x9000, fmt: Dual },
    Insn { name: "A",    base: 0xA000, fmt: Dual }, Insn { name: "AB",   base: 0xB000, fmt: Dual },
    Insn { name: "MOV",  base: 0xC000, fmt: Dual }, Insn { name: "MOVB", base: 0xD000, fmt: Dual },
    Insn { name: "SOC",  base: 0xE000, fmt: Dual }, Insn { name: "SOCB", base: 0xF000, fmt: Dual },
    // Format II — jumps.
    Insn { name: "JMP", base: 0x1000, fmt: Jump }, Insn { name: "JLT", base: 0x1100, fmt: Jump },
    Insn { name: "JLE", base: 0x1200, fmt: Jump }, Insn { name: "JEQ", base: 0x1300, fmt: Jump },
    Insn { name: "JHE", base: 0x1400, fmt: Jump }, Insn { name: "JGT", base: 0x1500, fmt: Jump },
    Insn { name: "JNE", base: 0x1600, fmt: Jump }, Insn { name: "JNC", base: 0x1700, fmt: Jump },
    Insn { name: "JOC", base: 0x1800, fmt: Jump }, Insn { name: "JNO", base: 0x1900, fmt: Jump },
    Insn { name: "JL",  base: 0x1A00, fmt: Jump }, Insn { name: "JH",  base: 0x1B00, fmt: Jump },
    Insn { name: "JOP", base: 0x1C00, fmt: Jump },
    // Format VI — single general operand.
    Insn { name: "BLWP", base: 0x0400, fmt: Single }, Insn { name: "B",    base: 0x0440, fmt: Single },
    Insn { name: "X",    base: 0x0480, fmt: Single }, Insn { name: "CLR",  base: 0x04C0, fmt: Single },
    Insn { name: "NEG",  base: 0x0500, fmt: Single }, Insn { name: "INV",  base: 0x0540, fmt: Single },
    Insn { name: "INC",  base: 0x0580, fmt: Single }, Insn { name: "INCT", base: 0x05C0, fmt: Single },
    Insn { name: "DEC",  base: 0x0600, fmt: Single }, Insn { name: "DECT", base: 0x0640, fmt: Single },
    Insn { name: "BL",   base: 0x0680, fmt: Single }, Insn { name: "SWPB", base: 0x06C0, fmt: Single },
    Insn { name: "SETO", base: 0x0700, fmt: Single }, Insn { name: "ABS",  base: 0x0740, fmt: Single },
    // Format VIII — immediate.
    Insn { name: "LI",   base: 0x0200, fmt: ImmReg }, Insn { name: "AI",   base: 0x0220, fmt: ImmReg },
    Insn { name: "ANDI", base: 0x0240, fmt: ImmReg }, Insn { name: "ORI",  base: 0x0260, fmt: ImmReg },
    Insn { name: "CI",   base: 0x0280, fmt: ImmReg },
    Insn { name: "LWPI", base: 0x02E0, fmt: Imm },    Insn { name: "LIMI", base: 0x0300, fmt: Imm },
    // Format III / IX — general source + register or count field.
    Insn { name: "COC", base: 0x2000, fmt: RegSrc }, Insn { name: "CZC", base: 0x2400, fmt: RegSrc },
    Insn { name: "XOR", base: 0x2800, fmt: RegSrc }, Insn { name: "XOP", base: 0x2C00, fmt: RegSrc },
    Insn { name: "MPY", base: 0x3800, fmt: RegSrc }, Insn { name: "DIV", base: 0x3C00, fmt: RegSrc },
    // Format IV — multi-bit CRU.
    Insn { name: "LDCR", base: 0x3000, fmt: CruMulti }, Insn { name: "STCR", base: 0x3400, fmt: CruMulti },
    // Format V — shifts.
    Insn { name: "SRA", base: 0x0800, fmt: Shift }, Insn { name: "SRL", base: 0x0900, fmt: Shift },
    Insn { name: "SLA", base: 0x0A00, fmt: Shift }, Insn { name: "SRC", base: 0x0B00, fmt: Shift },
    // Format II shape — single-bit CRU (operand is a displacement value).
    Insn { name: "SBO", base: 0x1D00, fmt: CruBit }, Insn { name: "SBZ", base: 0x1E00, fmt: CruBit },
    Insn { name: "TB",  base: 0x1F00, fmt: CruBit },
    // Format VIII — store internal register (no immediate).
    Insn { name: "STWP", base: 0x02A0, fmt: RegOnly }, Insn { name: "STST", base: 0x02C0, fmt: RegOnly },
    // Format VII — control, no operands.
    Insn { name: "RTWP", base: 0x0380, fmt: Control }, Insn { name: "IDLE", base: 0x0340, fmt: Control },
    Insn { name: "RSET", base: 0x0360, fmt: Control }, Insn { name: "CKON", base: 0x03A0, fmt: Control },
    Insn { name: "CKOF", base: 0x03C0, fmt: Control }, Insn { name: "LREX", base: 0x03E0, fmt: Control },
    // Pseudo-instructions.
    Insn { name: "NOP",  base: 0x1000, fmt: Pseudo }, // = JMP $+2
    Insn { name: "RT",   base: 0x045B, fmt: Pseudo }, // = B *R11
];
