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

//! GPL operands and the **general address format (GAS)** — the variable-length,
//! self-describing operand encoding every memory-referencing GPL instruction
//! uses.
//!
//! The format byte's top bit distinguishes two shapes (recovered empirically in
//! `RECON.md`; the plan's §5.1 table was the mis-stated one — see the review's
//! finding F7):
//!
//! ```text
//!   0aaaaaaa                          direct CPU cell >8300+a         (1 byte)
//!   1 X V I nnnn , lo                 12-bit address (nnnn:lo)        (2 bytes)
//!   1 X V I 1111 , hi , lo            16-bit address                 (3 bytes)
//!     X = indexed  (one index byte, a CPU cell, appended LAST)
//!     V = VDP RAM  (else CPU RAM)
//!     I = indirect (through the addressed word)
//! ```
//!
//! CPU addresses are **biased by `>8300`** (the short form reaches
//! `>8300–837F`; the 12-bit form `>8300–92FF`; the 16-bit form is
//! `>8300 + value`, wrapping). VDP addresses are used verbatim (0–`>3FFF`).
//! The `>8300` bias on the 16-bit CPU form is the single easiest thing to get
//! wrong; it is asserted by an executed `MOVE` in the M1 tests, not just here.

/// A decoded/encodable GPL operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operand {
    /// A CPU-RAM / scratchpad reference. `addr` is the full 16-bit CPU address
    /// (already de-biased, i.e. `>83CE` not `>00CE`).
    Cpu {
        addr: u16,
        indirect: bool,
        /// The CPU cell used as an index, if any (form with X=1).
        index: Option<u8>,
    },
    /// A VDP-RAM reference. Direct (`indirect == false`): `addr` is a
    /// 0–`>3FFF` VDP address, encoded verbatim. Indirect (`*V@cell`): `addr`
    /// is the **CPU scratchpad cell** (e.g. `>8356`) holding a word VDP
    /// address; the encoded field is the cell's `>8300` offset
    /// (execution-verified: `examples/m4_probe.rs` candidate A).
    Vdp {
        addr: u16,
        indirect: bool,
        index: Option<u8>,
    },
    /// An immediate byte (for ops like `BACK`, `ALL`, `ST`).
    Imm8(u8),
    /// An immediate word (for `DST`).
    Imm16(u16),
    /// An absolute GROM address (for `B`, `CALL`, and MOVE GROM operands).
    Grom(u16),
}

/// The CPU-address bias applied to GAS CPU operands.
pub const CPU_BIAS: u16 = 0x8300;

/// Error from decoding a GAS operand (ran off the end of the image).
#[derive(Debug, PartialEq, Eq)]
pub struct GasTruncated;

/// Decode one GAS operand from `img` at offset `at`, returning the operand and
/// the number of bytes it consumed. `vdp_forced` selects VDP addressing even
/// when the format byte's V bit is clear (unused today; the V bit is authoritative).
pub fn decode_gas(img: &[u8], at: usize) -> Result<(Operand, usize), GasTruncated> {
    let b0 = *img.get(at).ok_or(GasTruncated)?;
    if b0 & 0x80 == 0 {
        // Short direct CPU form.
        return Ok((
            Operand::Cpu {
                addr: CPU_BIAS + b0 as u16,
                indirect: false,
                index: None,
            },
            1,
        ));
    }
    let indexed = b0 & 0x40 != 0;
    let vdp = b0 & 0x20 != 0;
    let indirect = b0 & 0x10 != 0;
    let nib = b0 & 0x0F;
    let (raw, core_len) = if nib == 0x0F {
        let hi = *img.get(at + 1).ok_or(GasTruncated)?;
        let lo = *img.get(at + 2).ok_or(GasTruncated)?;
        (((hi as u16) << 8) | lo as u16, 3)
    } else {
        let lo = *img.get(at + 1).ok_or(GasTruncated)?;
        (((nib as u16) << 8) | lo as u16, 2)
    };
    let index = if indexed {
        Some(*img.get(at + core_len).ok_or(GasTruncated)?)
    } else {
        None
    };
    let len = core_len + if indexed { 1 } else { 0 };
    let op = if vdp {
        Operand::Vdp {
            // Indirect VDP fields reference the scratchpad cell (>8300-based);
            // direct fields are raw VDP addresses.
            addr: if indirect { CPU_BIAS.wrapping_add(raw) } else { raw },
            indirect,
            index,
        }
    } else {
        Operand::Cpu {
            addr: CPU_BIAS.wrapping_add(raw),
            indirect,
            index,
        }
    };
    Ok((op, len))
}

/// Encode a CPU/VDP GAS operand into `out`. Immediates and GROM addresses are
/// not GAS and are rejected (callers emit those directly).
pub fn encode_gas(op: &Operand, out: &mut Vec<u8>) -> Result<(), String> {
    let (vdp, addr, indirect, index) = match *op {
        Operand::Cpu {
            addr,
            indirect,
            index,
        } => (false, addr.wrapping_sub(CPU_BIAS), indirect, index),
        Operand::Vdp {
            addr,
            indirect,
            index,
        } => {
            // Indirect VDP operands name the scratchpad cell; the field is the
            // cell's >8300 offset. Direct fields are raw VDP addresses.
            let field = if indirect { addr.wrapping_sub(CPU_BIAS) } else { addr };
            (true, field, indirect, index)
        }
        _ => return Err("operand is not a memory (GAS) operand".into()),
    };

    // Short form: CPU, direct, un-indexed, value fits 7 bits.
    if !vdp && !indirect && index.is_none() && addr <= 0x7F {
        out.push(addr as u8);
        return Ok(());
    }

    let mut b0 = 0x80u8;
    if index.is_some() {
        b0 |= 0x40;
    }
    if vdp {
        b0 |= 0x20;
    }
    if indirect {
        b0 |= 0x10;
    }
    // Use the 12-bit form when the value fits 12 bits, else the 16-bit form.
    if addr <= 0x0FFF {
        b0 |= (addr >> 8) as u8 & 0x0F;
        // A low nibble of 0xF would be read as the 16-bit escape, so bump such
        // addresses to the 16-bit form.
        if b0 & 0x0F == 0x0F {
            b0 |= 0x0F;
            out.push(b0);
            out.push((addr >> 8) as u8);
            out.push(addr as u8);
        } else {
            out.push(b0);
            out.push(addr as u8);
        }
    } else {
        b0 |= 0x0F;
        out.push(b0);
        out.push((addr >> 8) as u8);
        out.push(addr as u8);
    }
    if let Some(ix) = index {
        out.push(ix);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(op: Operand) {
        let mut b = Vec::new();
        encode_gas(&op, &mut b).unwrap();
        let (back, len) = decode_gas(&b, 0).unwrap();
        assert_eq!(len, b.len(), "length mismatch for {op:?} -> {b:02X?}");
        assert_eq!(back, op, "roundtrip mismatch, bytes {b:02X?}");
    }

    #[test]
    fn short_cpu_form() {
        // >83CE encodes 12-bit (>0CE has low nibble != F): `80 CE`.
        let (op, len) = decode_gas(&[0x80, 0xCE], 0).unwrap();
        assert_eq!(len, 2);
        assert_eq!(
            op,
            Operand::Cpu {
                addr: 0x83CE,
                indirect: false,
                index: None
            }
        );
        // >8370 fits 7 bits (0x70) => short 1-byte form.
        let mut b = Vec::new();
        encode_gas(
            &Operand::Cpu {
                addr: 0x8370,
                indirect: false,
                index: None,
            },
            &mut b,
        )
        .unwrap();
        assert_eq!(b, [0x70]);
    }

    #[test]
    fn extended_16bit_form() {
        // `8F 11 00` => CPU 16-bit, value >1100, biased => >9400.
        let (op, len) = decode_gas(&[0x8F, 0x11, 0x00], 0).unwrap();
        assert_eq!(len, 3);
        assert_eq!(
            op,
            Operand::Cpu {
                addr: 0x9400,
                indirect: false,
                index: None
            }
        );
    }

    #[test]
    fn roundtrips() {
        rt(Operand::Cpu {
            addr: 0x8300,
            indirect: false,
            index: None,
        });
        rt(Operand::Cpu {
            addr: 0x837F,
            indirect: false,
            index: None,
        });
        rt(Operand::Cpu {
            addr: 0x83CE,
            indirect: false,
            index: None,
        });
        rt(Operand::Cpu {
            addr: 0x8400,
            indirect: false,
            index: None,
        });
        rt(Operand::Cpu {
            addr: 0x9400,
            indirect: true,
            index: None,
        });
        rt(Operand::Vdp {
            addr: 0x0000,
            indirect: false,
            index: None,
        });
        rt(Operand::Vdp {
            addr: 0x0380,
            indirect: false,
            index: None,
        });
        rt(Operand::Vdp {
            addr: 0x3FFF,
            indirect: false,
            index: None,
        });
        rt(Operand::Cpu {
            addr: 0x8340,
            indirect: false,
            index: Some(0x20),
        });
        // Indirect VDP: `addr` names the scratchpad cell holding the VDP addr.
        rt(Operand::Vdp {
            addr: 0x8356,
            indirect: true,
            index: None,
        });
    }

    #[test]
    fn vdp_indirect_encodes_the_cell_offset() {
        // *V@>8356 => `B0 56` (V=1, I=1, 12-bit field = cell offset >056) —
        // execution-verified in examples/m4_probe.rs (candidate A).
        let mut b = Vec::new();
        encode_gas(
            &Operand::Vdp { addr: 0x8356, indirect: true, index: None },
            &mut b,
        )
        .unwrap();
        assert_eq!(b, [0xB0, 0x56]);
    }
}
