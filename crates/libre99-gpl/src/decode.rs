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

//! Decoding GPL bytes back into instructions — the inverse of the encoder, and
//! the engine behind the disassembler and the recon tooling. Signatures come
//! from the authoritative table in `isa` (Classic99 `gpl.cpp`); MOVE decodes
//! exactly via its bit field.

use crate::isa::{decode_sig, MoveBits, Sig, FMT};
use crate::operand::{decode_gas, Operand};

/// Where control goes after an instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Falls through to the next instruction.
    Fall,
    /// Unconditional branch to a GROM address (`B`).
    Jump(u16),
    /// Subroutine call (`CALL`) — falls through on return.
    Call(u16),
    /// Conditional branch (`BR`/`BS`) — may take the target or fall through.
    Cond(u16),
    /// Ends the run (`RTN`/`RTNC`/`EXIT`).
    Stop,
}

/// One decoded instruction.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub addr: u16,
    pub opcode: u8,
    pub mnemonic: &'static str,
    pub len: usize,
    pub operands: Vec<Operand>,
    pub flow: Flow,
}

/// Why decoding failed.
#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// Ran off the end of the image while reading operands.
    Truncated,
    /// The opcode has no modelled length.
    Unknown(u8),
}

/// Decode one instruction at slot-absolute address `addr`, reading bytes from
/// `img` at offset `off`. `addr` is used only to resolve branch targets.
pub fn decode_at(img: &[u8], off: usize, addr: u16) -> Result<Decoded, DecodeError> {
    let opcode = *img.get(off).ok_or(DecodeError::Truncated)?;
    let (mnemonic, sig) = decode_sig(opcode);
    let mut operands = Vec::new();
    let mut len = 1usize;
    let mut flow = Flow::Fall;

    let gas = |operands: &mut Vec<Operand>, len: &mut usize| -> Result<(), DecodeError> {
        let (op, l) = decode_gas(img, off + *len).map_err(|_| DecodeError::Truncated)?;
        operands.push(op);
        *len += l;
        Ok(())
    };
    let byte = |off2: usize| -> Result<u8, DecodeError> {
        img.get(off2).copied().ok_or(DecodeError::Truncated)
    };

    match sig {
        Sig::None => {
            flow = match opcode {
                0x00 | 0x01 | 0x0B | 0x10..=0x13 => Flow::Stop,
                _ => Flow::Fall,
            };
        }
        Sig::Imm8 => {
            operands.push(Operand::Imm8(byte(off + 1)?));
            len += 1;
        }
        Sig::Addr16 => {
            let target = ((byte(off + 1)? as u16) << 8) | byte(off + 2)? as u16;
            operands.push(Operand::Grom(target));
            len += 2;
            flow = if opcode == 0x06 { Flow::Call(target) } else { Flow::Jump(target) };
        }
        Sig::Gas => gas(&mut operands, &mut len)?,
        Sig::GasGas => {
            gas(&mut operands, &mut len)?;
            gas(&mut operands, &mut len)?;
        }
        Sig::GasImm8 => {
            gas(&mut operands, &mut len)?;
            operands.push(Operand::Imm8(byte(off + len)?));
            len += 1;
        }
        Sig::GasImm16 => {
            gas(&mut operands, &mut len)?;
            let v = ((byte(off + len)? as u16) << 8) | byte(off + len + 1)? as u16;
            operands.push(Operand::Imm16(v));
            len += 2;
        }
        Sig::Branch => {
            let disp = byte(off + 1)?;
            let target = (addr & 0xE000) | (((opcode & 0x1F) as u16) << 8) | disp as u16;
            operands.push(Operand::Grom(target));
            len += 1;
            flow = Flow::Cond(target);
        }
        Sig::Move => {
            // Exact decode from the bit field: count, destination, source.
            let bits = MoveBits::from_opcode(opcode);
            if bits.imm_count {
                let v = ((byte(off + 1)? as u16) << 8) | byte(off + 2)? as u16;
                operands.push(Operand::Imm16(v));
                len += 2;
            } else {
                gas(&mut operands, &mut len)?;
            }
            if bits.reg_dst {
                operands.push(Operand::Imm8(byte(off + len)?));
                len += 1;
            } else if !bits.not_grom_dst {
                // GRAM destination: 16-bit GROM address.
                let v = ((byte(off + len)? as u16) << 8) | byte(off + len + 1)? as u16;
                operands.push(Operand::Grom(v));
                len += 2;
            } else {
                gas(&mut operands, &mut len)?;
            }
            if bits.ram_src || bits.cpu_held_grom_src {
                gas(&mut operands, &mut len)?;
            } else {
                let v = ((byte(off + len)? as u16) << 8) | byte(off + len + 1)? as u16;
                operands.push(Operand::Grom(v));
                len += 2;
            }
        }
        Sig::Fmt => {
            debug_assert_eq!(opcode, FMT);
            // Scan to the FEND terminator (>FB).
            let mut j = off + 1;
            while j < img.len() && img[j] != 0xFB {
                j += 1;
            }
            len = (j - off) + 1; // include FEND
            if off + len > img.len() {
                return Err(DecodeError::Truncated);
            }
        }
        Sig::Unknown => return Err(DecodeError::Unknown(opcode)),
    }

    if off + len > img.len() {
        return Err(DecodeError::Truncated);
    }
    Ok(Decoded { addr, opcode, mnemonic, len, operands, flow })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_is_slot_absolute() {
        // `40 52` at >0020 => BR >0052 (RECON.md).
        let d = decode_at(&[0x40, 0x52], 0, 0x0020).unwrap();
        assert_eq!(d.mnemonic, "BR");
        assert_eq!(d.len, 2);
        assert_eq!(d.flow, Flow::Cond(0x0052));
        // `43 3C` at >033A => BR >033C (the sound-wait loop).
        let d = decode_at(&[0x43, 0x3C], 0, 0x033A).unwrap();
        assert_eq!(d.flow, Flow::Cond(0x033C));
    }

    #[test]
    fn absolute_branch_and_call() {
        let d = decode_at(&[0x05, 0x4D, 0x12], 0, 0x0038).unwrap();
        assert_eq!(d.mnemonic, "B");
        assert_eq!(d.flow, Flow::Jump(0x4D12));
        let d = decode_at(&[0x06, 0x12, 0x34], 0, 0).unwrap();
        assert_eq!(d.flow, Flow::Call(0x1234));
    }

    #[test]
    fn store_immediates_tile() {
        // `BE 81 00 9F` => ST @>8400, >9F (RECON.md sound-mute).
        let d = decode_at(&[0xBE, 0x81, 0x00, 0x9F], 0, 0x005A).unwrap();
        assert_eq!(d.mnemonic, "ST");
        assert_eq!(d.len, 4);
        assert_eq!(d.operands[0], Operand::Cpu { addr: 0x8400, indirect: false, index: None });
        assert_eq!(d.operands[1], Operand::Imm8(0x9F));
        // `BF 72 FF 7E` => DST @>8372, >FF7E.
        let d = decode_at(&[0xBF, 0x72, 0xFF, 0x7E], 0, 0x006A).unwrap();
        assert_eq!(d.mnemonic, "ST");
        assert_eq!(d.operands[1], Operand::Imm16(0xFF7E));
    }

    #[test]
    fn menu_trace_instructions_tile_exactly() {
        // The authentic menu's pre-launch sequence (RECON.md), byte-exact.
        // DCGT @>8370,>1000 = CF 70 10 00.
        let d = decode_at(&[0xCF, 0x70, 0x10, 0x00], 0, 0x0341).unwrap();
        assert_eq!(d.mnemonic, "CGT");
        assert_eq!(d.len, 4);
        // DST @>8300,@>8370 = BD 00 70 (word store, memory source).
        let d = decode_at(&[0xBD, 0x00, 0x70], 0, 0x0347).unwrap();
        assert_eq!(d.mnemonic, "ST");
        assert_eq!(d.len, 3);
        assert_eq!(d.operands[0], Operand::Cpu { addr: 0x8300, indirect: false, index: None });
        // DSUB @>8300,>0FFF = A7 00 0F FF.
        let d = decode_at(&[0xA7, 0x00, 0x0F, 0xFF], 0, 0x034A).unwrap();
        assert_eq!(d.mnemonic, "SUB");
        assert_eq!(d.len, 4);
        // MOVE with count-from-memory: 34 00 AF 10 00 AF 0F FF
        //   = MOVE @>8300 bytes, dst V@>1000, src V@>0FFF (the VRAM fill).
        let d = decode_at(&[0x34, 0x00, 0xAF, 0x10, 0x00, 0xAF, 0x0F, 0xFF], 0, 0x034E).unwrap();
        assert_eq!(d.mnemonic, "MOVE");
        assert_eq!(d.len, 8);
        assert_eq!(d.operands[0], Operand::Cpu { addr: 0x8300, indirect: false, index: None });
        assert_eq!(d.operands[1], Operand::Vdp { addr: 0x1000, indirect: false, index: None });
        assert_eq!(d.operands[2], Operand::Vdp { addr: 0x0FFF, indirect: false, index: None });
        // MOVE >006F, dst @>8301, src @>8300 (the scratchpad clear cascade).
        let d = decode_at(&[0x35, 0x00, 0x6F, 0x01, 0x00], 0, 0x0358).unwrap();
        assert_eq!(d.len, 5);
        assert_eq!(d.operands[0], Operand::Imm16(0x006F));
        // MOVE >0008 to VDP regs from GROM >0451 (the boot-trace form).
        let d = decode_at(&[0x39, 0x00, 0x08, 0x00, 0x04, 0x51], 0, 0x006E).unwrap();
        assert_eq!(d.len, 6);
        assert_eq!(d.operands[1], Operand::Imm8(0x00)); // starting VDP register
        assert_eq!(d.operands[2], Operand::Grom(0x0451));
        // XML >F0 (the ML dispatch).
        let d = decode_at(&[0x0F, 0xF0], 0, 0x0379).unwrap();
        assert_eq!(d.mnemonic, "XML");
        assert_eq!(d.operands[0], Operand::Imm8(0xF0));
    }

    #[test]
    fn backdrop_immediate() {
        let d = decode_at(&[0x04, 0x17], 0, 0).unwrap();
        assert_eq!(d.mnemonic, "BACK");
        assert_eq!(d.operands[0], Operand::Imm8(0x17));
    }
}
