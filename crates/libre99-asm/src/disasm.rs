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

//! A TMS9900 disassembler — the inverse of [`crate::isa`], driven by the *same*
//! instruction table so the two can never drift (NFR-4). It exists to read the
//! authentic console ROM while mapping it (the D1 dossier work) and to eyeball
//! our own image; round-trip tests (assemble → disassemble → assemble) keep it
//! honest.
//!
//! [`decode_at`] decodes one instruction; [`linear`] tiles sequentially. Both
//! render operands in this assembler's own syntax (`R1`, `*R3+`, `@>83C4`,
//! `@>2(R5)`) so a listing reads back the way it was written.

use crate::isa::{self, Fmt, Insn};

/// One decoded instruction.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub mnemonic: &'static str,
    pub operands: Vec<String>,
    /// Total length in bytes (2, 4, or 6).
    pub len: usize,
}

/// Why a word could not be decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// Not enough bytes remain for the opcode or an expected extension word.
    Truncated,
    /// The opcode word matches no instruction.
    Unknown(u16),
}

/// The bits that must equal an instruction's `base` for a word to be that
/// instruction — the complement of its operand fields.
fn fmt_mask(f: Fmt) -> u16 {
    match f {
        Fmt::Dual => 0xF000,
        Fmt::Jump | Fmt::CruBit | Fmt::Shift => 0xFF00,
        Fmt::Single => 0xFFC0,
        Fmt::RegSrc | Fmt::CruMulti => 0xFC00,
        Fmt::ImmReg | Fmt::RegOnly => 0xFFF0,
        Fmt::Imm => 0xFFE0,
        Fmt::Pseudo | Fmt::Control => 0xFFFF,
    }
}

fn find(w: u16) -> Option<&'static Insn> {
    // Skip the pseudo aliases (NOP == JMP $+2, RT == B *R11): they share their
    // encoding with a real instruction, which decodes them fine.
    isa::table()
        .iter()
        .filter(|i| i.fmt != Fmt::Pseudo)
        .find(|i| w & fmt_mask(i.fmt) == i.base)
}

fn read_word(img: &[u8], off: usize) -> Option<u16> {
    if off + 2 <= img.len() {
        Some(((img[off] as u16) << 8) | img[off + 1] as u16)
    } else {
        None
    }
}

/// Render a general operand `(T, reg)` with its optional extension word.
fn general(t: u16, reg: u16, ext: u16) -> String {
    match t {
        0 => format!("R{reg}"),
        1 => format!("*R{reg}"),
        3 => format!("*R{reg}+"),
        2 if reg == 0 => format!("@>{ext:04X}"),
        2 => format!("@>{ext:04X}(R{reg})"),
        _ => unreachable!("T is 2 bits"),
    }
}

/// Decode the instruction at byte offset `off`; `addr` is that byte's runtime
/// address (used to resolve jump targets).
pub fn decode_at(img: &[u8], off: usize, addr: u16) -> Result<Decoded, DecodeError> {
    let w = read_word(img, off).ok_or(DecodeError::Truncated)?;
    let insn = find(w).ok_or(DecodeError::Unknown(w))?;
    let mut len = 2usize;

    // Pull the next extension word, advancing `len`.
    let take_ext = |len: &mut usize| -> Result<u16, DecodeError> {
        let e = read_word(img, off + *len).ok_or(DecodeError::Truncated)?;
        *len += 2;
        Ok(e)
    };

    let operands = match insn.fmt {
        Fmt::Dual => {
            let ts = (w >> 4) & 3;
            let s = w & 0xF;
            let td = (w >> 10) & 3;
            let d = (w >> 6) & 0xF;
            // Source extension word precedes the destination's.
            let sext = if ts == 2 { take_ext(&mut len)? } else { 0 };
            let dext = if td == 2 { take_ext(&mut len)? } else { 0 };
            vec![general(ts, s, sext), general(td, d, dext)]
        }
        Fmt::Single => {
            let ts = (w >> 4) & 3;
            let s = w & 0xF;
            let ext = if ts == 2 { take_ext(&mut len)? } else { 0 };
            vec![general(ts, s, ext)]
        }
        Fmt::Jump => {
            let disp = (w & 0xFF) as u8 as i8;
            let target = addr.wrapping_add(2).wrapping_add((disp as i16 as u16).wrapping_mul(2));
            vec![format!(">{target:04X}")]
        }
        Fmt::CruBit => {
            let disp = (w & 0xFF) as u8 as i8;
            vec![format!("{disp}")]
        }
        Fmt::Shift => {
            let reg = w & 0xF;
            let count = (w >> 4) & 0xF;
            vec![format!("R{reg}"), format!("{count}")]
        }
        Fmt::RegSrc => {
            let ts = (w >> 4) & 3;
            let s = w & 0xF;
            let field = (w >> 6) & 0xF;
            let ext = if ts == 2 { take_ext(&mut len)? } else { 0 };
            // XOP's field is a vector number; the rest name a destination register.
            let second = if insn.name == "XOP" { format!("{field}") } else { format!("R{field}") };
            vec![general(ts, s, ext), second]
        }
        Fmt::CruMulti => {
            let ts = (w >> 4) & 3;
            let s = w & 0xF;
            let count = (w >> 6) & 0xF;
            let ext = if ts == 2 { take_ext(&mut len)? } else { 0 };
            vec![general(ts, s, ext), format!("{count}")]
        }
        Fmt::ImmReg => {
            let reg = w & 0xF;
            let imm = take_ext(&mut len)?;
            vec![format!("R{reg}"), format!(">{imm:04X}")]
        }
        Fmt::Imm => {
            let imm = take_ext(&mut len)?;
            vec![format!(">{imm:04X}")]
        }
        Fmt::RegOnly => vec![format!("R{}", w & 0xF)],
        Fmt::Control | Fmt::Pseudo => vec![],
    };

    Ok(Decoded { mnemonic: insn.name, operands, len })
}

/// Disassemble sequentially from `off` (runtime address `addr`) for up to `max`
/// instructions, stopping at an unknown opcode or the end of `img`. Returns the
/// listing plus the number of bytes tiled.
pub fn linear(img: &[u8], off: usize, addr: u16, max: usize) -> (String, usize) {
    let mut out = String::new();
    let mut o = off;
    let mut a = addr;
    let mut tiled = 0usize;
    for _ in 0..max {
        match decode_at(img, o, a) {
            Ok(d) => {
                // Operands are comma-joined with no space: classic E/A ends the
                // operand field at the first whitespace (the rest is a comment),
                // so a spaced join would not reassemble.
                out.push_str(&format!(">{:04X}  {:<6} {}\n", a, d.mnemonic, d.operands.join(",")));
                o += d.len;
                a = a.wrapping_add(d.len as u16);
                tiled += d.len;
            }
            Err(DecodeError::Unknown(op)) => {
                out.push_str(&format!(">{a:04X}  DATA   >{op:04X}   ; unknown opcode\n"));
                break;
            }
            Err(DecodeError::Truncated) => break,
        }
    }
    (out, tiled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assemble, Options};

    fn one(src: &str) -> Decoded {
        let img = assemble(src, &Options { auto_header: false, ..Default::default() })
            .expect("assembles")
            .image;
        decode_at(&img, 0, 0x6000).expect("decodes")
    }

    #[test]
    fn decodes_representative_instructions() {
        let d = one("   MOV R1,R2");
        assert_eq!((d.mnemonic, d.len), ("MOV", 2));
        assert_eq!(d.operands, ["R1", "R2"]);

        let d = one("   MOVB *R1+,@>8C02");
        assert_eq!((d.mnemonic, d.len), ("MOVB", 4));
        assert_eq!(d.operands, ["*R1+", "@>8C02"]);

        let d = one("   LI R0,>1234");
        assert_eq!(d.operands, ["R0", ">1234"]);

        let d = one("   LIMI 2");
        assert_eq!((d.mnemonic, d.operands.first().map(String::as_str)), ("LIMI", Some(">0002")));

        let d = one("   SBO 2");
        assert_eq!((d.mnemonic, d.operands[0].as_str()), ("SBO", "2"));

        let d = one("   LDCR R0,3");
        assert_eq!(d.operands, ["R0", "3"]);

        assert_eq!(one("   RTWP").mnemonic, "RTWP");
        assert_eq!(one("   X R4").operands, ["R4"]);
    }

    #[test]
    fn jump_target_is_absolute() {
        // JMP $ encodes displacement -1; from >6000 that targets >6000.
        let img = assemble("   JMP $", &Options { auto_header: false, ..Default::default() })
            .unwrap()
            .image;
        let d = decode_at(&img, 0, 0x6000).unwrap();
        assert_eq!((d.mnemonic, d.operands[0].as_str()), ("JMP", ">6000"));
    }

    /// Assemble a small program, disassemble it, reassemble the disassembly, and
    /// require the bytes to match — the encoder/decoder agree on every op used.
    #[test]
    fn round_trips_through_the_assembler() {
        let src = "\
            \x20   LI   R0,>1700\n\
            \x20   MOVB R0,@>8C02\n\
            \x20   MOV  R1,*R2+\n\
            \x20   A    @>0004(R3),R4\n\
            \x20   JEQ  $\n\
            \x20   SBO  2\n\
            \x20   LDCR R0,3\n\
            \x20   RTWP\n";
        let opts = Options { auto_header: false, ..Default::default() };
        let img = assemble(src, &opts).unwrap().image;
        let (listing, tiled) = linear(&img, 0, 0x6000, 64);
        assert_eq!(tiled, img.len(), "did not tile the whole image:\n{listing}");
        // Strip the fixed 7-char address column (">XXXX  ") from each listing
        // line to recover clean source, reassemble it, and require byte-for-byte
        // identity. (A naive "first letter" strip would trip over hex digits
        // A–F in the address itself.)
        let clean: String =
            listing.lines().map(|l| format!("   {}\n", l.get(7..).unwrap_or(""))).collect();
        let img2 = assemble(&clean, &opts).expect("reassembles").image;
        assert_eq!(img, img2, "round-trip mismatch\nlisting:\n{listing}");
    }
}
