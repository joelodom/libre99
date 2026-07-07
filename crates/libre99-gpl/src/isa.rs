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

//! The GPL instruction set — now sourced from the **authoritative 256-entry
//! opcode table** in Classic99's `addons/gpl.cpp:43-315` (a sibling checkout at
//! `../classic99`), with TI's per-opcode format specs in its comments, and
//! cross-checked against Nouspikel's GPL documentation. Encodings we emit are
//! additionally pinned by executing them on the real console ROM
//! (`examples/*_probe.rs`, `tests/`).
//!
//! ## The five operand formats (gpl.cpp:317-334, 1229-1242)
//!
//! * **Format 1** (two-operand, `>A0–>EB`): opcode `1xxxxx U W` — `W`=1 word,
//!   `U`=1 immediate source. Byte stream: **destination GAS, then source**
//!   (a GAS operand, or 1/2 immediate bytes when `U`).
//! * **Format 2** (immediate): opcode + 1 or 2 immediate bytes (`B`/`CALL`
//!   take a 16-bit GROM address).
//! * **Format 3** (no operands).
//! * **Format 4** (`BR`/`BS`): 13-bit slot-absolute target.
//! * **Format 5** (single-operand, `>80–>97`): opcode `100xxxx W`, one GAS.
//! * **Format 6** (`MOVE`, `>20–>3F`): opcode `001 G R V C N` — see
//!   [`MoveBits`]; stream is count, destination, source.

/// How an opcode's operands are laid out in the byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sig {
    /// No operands (format 3).
    None,
    /// One immediate byte (`BACK`, `ALL`, `RAND`, `PARSE`, `XML`).
    Imm8,
    /// A 16-bit absolute GROM address (`B`, `CALL`).
    Addr16,
    /// One GAS operand (format 5 single-operand group).
    Gas,
    /// Two GAS operands, destination first (format 1, U=0).
    GasGas,
    /// GAS destination + immediate byte (format 1, U=1 W=0).
    GasImm8,
    /// GAS destination + immediate word (format 1, U=1 W=1).
    GasImm16,
    /// 13-bit slot-absolute branch (`BR`, `BS`).
    Branch,
    /// The MOVE family (format 6) — decoded via [`MoveBits`].
    Move,
    /// FMT sub-language, terminated by FEND (`>FB`).
    Fmt,
    /// Not modelled.
    Unknown,
}

/// MOVE opcode bits (`001 G R V C N`, gpl.cpp:709-729). The operand stream is
/// `count, destination, source`:
///
/// * count: immediate word when `N`, else a GAS operand (byte count read from
///   CPU/VDP RAM — the menu uses this for name lengths).
/// * destination: one raw byte (starting VDP register) when `R`; a 16-bit GROM
///   address when `!G` (GRAM write — unused here); else a GAS operand.
/// * source: a GAS operand when `V` (CPU/VDP RAM); else a GROM source — an
///   immediate 16-bit GROM address when `!C`, or, when `C`, a GAS operand
///   naming the CPU cell that *holds* the GROM address (computed GROM source;
///   execution-verified in `examples/m2_probe.rs`).
#[derive(Debug, Clone, Copy)]
pub struct MoveBits {
    /// Destination is NOT GROM (set for everything we emit; clear = GRAM).
    pub not_grom_dst: bool,
    /// Destination is a VDP register number.
    pub reg_dst: bool,
    /// Source is CPU/VDP RAM (a GAS operand).
    pub ram_src: bool,
    /// GROM source address comes from a CPU cell (GAS operand) instead of an
    /// immediate 16-bit address. Only meaningful when `!ram_src`.
    pub cpu_held_grom_src: bool,
    /// Count is an immediate word (else a GAS operand).
    pub imm_count: bool,
}

impl MoveBits {
    pub fn opcode(self) -> u8 {
        0x20 | (self.not_grom_dst as u8) << 4
            | (self.reg_dst as u8) << 3
            | (self.ram_src as u8) << 2
            | (self.cpu_held_grom_src as u8) << 1
            | self.imm_count as u8
    }
    pub fn from_opcode(op: u8) -> Self {
        MoveBits {
            not_grom_dst: op & 0x10 != 0,
            reg_dst: op & 0x08 != 0,
            ram_src: op & 0x04 != 0,
            cpu_held_grom_src: op & 0x02 != 0,
            imm_count: op & 0x01 != 0,
        }
    }
}

/// A format-1 two-operand family: `base` is the byte/memory-source opcode;
/// `base|1` the word form; `base|2` immediate source; `base|3` both.
pub struct TwoOp {
    pub name: &'static str,
    pub base: u8,
}

/// The format-1 families (gpl.cpp:214-293). Destination first in the stream.
pub const TWO_OPS: &[TwoOp] = &[
    TwoOp { name: "ADD", base: 0xA0 },
    TwoOp { name: "SUB", base: 0xA4 },
    TwoOp { name: "MUL", base: 0xA8 },
    TwoOp { name: "DIV", base: 0xAC },
    TwoOp { name: "AND", base: 0xB0 },
    TwoOp { name: "OR", base: 0xB4 },
    TwoOp { name: "XOR", base: 0xB8 },
    TwoOp { name: "ST", base: 0xBC },
    TwoOp { name: "EX", base: 0xC0 },
    TwoOp { name: "CH", base: 0xC4 },
    TwoOp { name: "CHE", base: 0xC8 },
    TwoOp { name: "CGT", base: 0xCC },
    TwoOp { name: "CGE", base: 0xD0 },
    TwoOp { name: "CEQ", base: 0xD4 },
    TwoOp { name: "CLOG", base: 0xD8 },
    TwoOp { name: "SRA", base: 0xDC },
    TwoOp { name: "SLL", base: 0xE0 },
    TwoOp { name: "SRL", base: 0xE4 },
    TwoOp { name: "SRC", base: 0xE8 },
];

/// A format-5 single-operand family: `base` is the byte form, `base|1` word.
pub struct OneOp {
    pub name: &'static str,
    pub base: u8,
}

/// The format-5 families (gpl.cpp:180-204).
pub const ONE_OPS: &[OneOp] = &[
    OneOp { name: "ABS", base: 0x80 },
    OneOp { name: "NEG", base: 0x82 },
    OneOp { name: "INV", base: 0x84 },
    OneOp { name: "CLR", base: 0x86 },
    OneOp { name: "FETCH", base: 0x88 },
    OneOp { name: "CASE", base: 0x8A },
    OneOp { name: "PUSH", base: 0x8C },
    OneOp { name: "CZ", base: 0x8E },
    OneOp { name: "INC", base: 0x90 },
    OneOp { name: "DEC", base: 0x92 },
    OneOp { name: "INCT", base: 0x94 },
    OneOp { name: "DECT", base: 0x96 },
];

/// Simple named opcodes (formats 2/3/4).
pub struct NamedOp {
    pub name: &'static str,
    pub opcode: u8,
    pub sig: Sig,
}

pub const NAMED: &[NamedOp] = &[
    NamedOp { name: "RTN", opcode: 0x00, sig: Sig::None },
    NamedOp { name: "RTNC", opcode: 0x01, sig: Sig::None },
    NamedOp { name: "RAND", opcode: 0x02, sig: Sig::Imm8 },
    NamedOp { name: "SCAN", opcode: 0x03, sig: Sig::None },
    NamedOp { name: "BACK", opcode: 0x04, sig: Sig::Imm8 },
    NamedOp { name: "B", opcode: 0x05, sig: Sig::Addr16 },
    NamedOp { name: "CALL", opcode: 0x06, sig: Sig::Addr16 },
    NamedOp { name: "ALL", opcode: 0x07, sig: Sig::Imm8 },
    NamedOp { name: "H", opcode: 0x09, sig: Sig::None },
    NamedOp { name: "GT", opcode: 0x0A, sig: Sig::None },
    NamedOp { name: "EXIT", opcode: 0x0B, sig: Sig::None },
    NamedOp { name: "CARRY", opcode: 0x0C, sig: Sig::None },
    NamedOp { name: "OVF", opcode: 0x0D, sig: Sig::None },
    NamedOp { name: "PARSE", opcode: 0x0E, sig: Sig::Imm8 },
    NamedOp { name: "XML", opcode: 0x0F, sig: Sig::Imm8 },
    NamedOp { name: "CONT", opcode: 0x10, sig: Sig::None },
    NamedOp { name: "EXEC", opcode: 0x11, sig: Sig::None },
    NamedOp { name: "RTNB", opcode: 0x12, sig: Sig::None },
    NamedOp { name: "RTGR", opcode: 0x13, sig: Sig::None },
    NamedOp { name: "BR", opcode: 0x40, sig: Sig::Branch },
    NamedOp { name: "BS", opcode: 0x60, sig: Sig::Branch },
];

/// Assembler lookup: resolve a mnemonic to `(opcode-base, sig-kind)`.
/// Two-op/one-op mnemonics accept a `D` prefix for the word form (TI GPL
/// convention: `ST`/`DST`, `CEQ`/`DCEQ`, `CLR`/`DCLR`, …); the immediate-source
/// bit of format 1 is chosen by the assembler from the source operand's shape.
pub enum Lookup {
    Named(&'static NamedOp),
    /// Format-1 family opcode with the W bit applied; U applied by the caller.
    Two { base_w: u8, word: bool },
    /// Format-5 opcode with the W bit applied.
    One { op: u8, word: bool },
}

pub fn lookup(mnemonic: &str) -> Option<Lookup> {
    if let Some(n) = NAMED.iter().find(|o| o.name == mnemonic) {
        return Some(Lookup::Named(n));
    }
    let (word, stem) = match mnemonic.strip_prefix('D') {
        // `DIV`/`DEC`/`DECT` begin with D but are their own stems; only treat
        // the prefix as "double" when the remainder is a known stem.
        Some(rest)
            if TWO_OPS.iter().any(|t| t.name == rest) || ONE_OPS.iter().any(|o| o.name == rest) =>
        {
            (true, rest)
        }
        _ => (false, mnemonic),
    };
    if let Some(t) = TWO_OPS.iter().find(|t| t.name == stem) {
        return Some(Lookup::Two { base_w: t.base | word as u8, word });
    }
    if let Some(o) = ONE_OPS.iter().find(|o| o.name == stem) {
        return Some(Lookup::One { op: o.base | word as u8, word });
    }
    None
}

/// The disassembler's view: mnemonic + signature for any opcode byte.
pub fn decode_sig(op: u8) -> (&'static str, Sig) {
    match op {
        0x00..=0x13 => {
            let n = NAMED.iter().find(|o| o.opcode == op);
            match n {
                Some(n) => (n.name, n.sig),
                None => ("?", Sig::Unknown), // >08 FMT handled below; >14+ XGPL
            }
        }
        0x14..=0x1F => ("XGPL", Sig::Unknown),
        0x20..=0x3F => ("MOVE", Sig::Move),
        0x40..=0x5F => ("BR", Sig::Branch),
        0x60..=0x7F => ("BS", Sig::Branch),
        0x80..=0x97 => {
            let fam = ONE_OPS.iter().find(|o| o.base == op & 0xFE).map(|o| o.name).unwrap_or("?");
            (fam, Sig::Gas)
        }
        0x98..=0x9F => ("XGPL", Sig::Unknown),
        0xA0..=0xEB => {
            let fam = TWO_OPS
                .iter()
                .find(|t| t.base == op & 0xFC)
                .map(|t| t.name)
                .unwrap_or("?");
            let sig = if op & 0x02 != 0 {
                if op & 0x01 != 0 {
                    Sig::GasImm16
                } else {
                    Sig::GasImm8
                }
            } else {
                Sig::GasGas
            };
            (fam, sig)
        }
        0xEC..=0xEF => ("COINC", Sig::Unknown),
        0xF4..=0xF7 => ("IO", Sig::GasGas),
        0xF8..=0xFB => ("SWGR", Sig::Unknown),
        _ => ("?", Sig::Unknown),
    }
}

/// FMT's opcode (decode-only; the assembler rejects it).
pub const FMT: u8 = 0x08;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_bit_layout() {
        // ST: BC mem-byte, BD mem-word, BE imm-byte, BF imm-word.
        assert_eq!(decode_sig(0xBC), ("ST", Sig::GasGas));
        assert_eq!(decode_sig(0xBD), ("ST", Sig::GasGas));
        assert_eq!(decode_sig(0xBE), ("ST", Sig::GasImm8));
        assert_eq!(decode_sig(0xBF), ("ST", Sig::GasImm16));
        // CEQ family at D4; DCGT imm-word = CF (seen in the menu trace).
        assert_eq!(decode_sig(0xD6), ("CEQ", Sig::GasImm8));
        assert_eq!(decode_sig(0xCF), ("CGT", Sig::GasImm16));
        // Single-op: CLR 86 byte, DCLR 87 word; DECT 96.
        assert_eq!(decode_sig(0x86), ("CLR", Sig::Gas));
        assert_eq!(decode_sig(0x87), ("CLR", Sig::Gas));
        assert_eq!(decode_sig(0x96), ("DECT", Sig::Gas));
    }

    #[test]
    fn lookup_d_prefix() {
        assert!(matches!(lookup("ST"), Some(Lookup::Two { base_w: 0xBC, word: false })));
        assert!(matches!(lookup("DST"), Some(Lookup::Two { base_w: 0xBD, word: true })));
        assert!(matches!(lookup("CEQ"), Some(Lookup::Two { base_w: 0xD4, word: false })));
        assert!(matches!(lookup("DCEQ"), Some(Lookup::Two { base_w: 0xD5, word: true })));
        // DIV starts with D but is its own family; DDIV is its word form.
        assert!(matches!(lookup("DIV"), Some(Lookup::Two { base_w: 0xAC, word: false })));
        assert!(matches!(lookup("DDIV"), Some(Lookup::Two { base_w: 0xAD, word: true })));
        // DEC/DECT vs DDEC/DDECT.
        assert!(matches!(lookup("DEC"), Some(Lookup::One { op: 0x92, word: false })));
        assert!(matches!(lookup("DDEC"), Some(Lookup::One { op: 0x93, word: true })));
        assert!(matches!(lookup("DECT"), Some(Lookup::One { op: 0x96, word: false })));
        assert!(matches!(lookup("CLR"), Some(Lookup::One { op: 0x86, word: false })));
        assert!(matches!(lookup("DCLR"), Some(Lookup::One { op: 0x87, word: true })));
    }

    #[test]
    fn move_bits_roundtrip() {
        // 0x31: dst GAS (not GROM, not reg), src GROM imm, count imm — the
        // execution-verified GROM->VRAM form.
        let b = MoveBits::from_opcode(0x31);
        assert!(b.not_grom_dst && !b.reg_dst && !b.ram_src && !b.cpu_held_grom_src && b.imm_count);
        assert_eq!(b.opcode(), 0x31);
        // 0x39: VDP-register dest (the boot-trace form).
        let b = MoveBits::from_opcode(0x39);
        assert!(b.reg_dst && b.imm_count);
        // 0x35: RAM source (GAS), count imm.
        let b = MoveBits::from_opcode(0x35);
        assert!(b.ram_src && b.imm_count);
        // 0x34: RAM source, count from memory (the menu's VRAM-fill form).
        let b = MoveBits::from_opcode(0x34);
        assert!(b.ram_src && !b.imm_count);
        // 0x33: computed GROM source (address held in a CPU cell).
        let b = MoveBits::from_opcode(0x33);
        assert!(!b.ram_src && b.cpu_held_grom_src && b.imm_count);
    }
}
