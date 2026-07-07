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

//! Encoding resolved GPL instructions to bytes — the inverse of [`crate::decode`].
//!
//! Branch encoding (`BR`/`BS`) needs the location counter, so it lives in the
//! assembler; everything else is a pure function of the opcode, its signature,
//! and its resolved operands.

use crate::isa::Sig;
use crate::operand::{encode_gas, Operand};

/// Encode `opcode` with signature `sig` and resolved `ops` into bytes. Returns
/// an error if the operand kinds don't match the signature.
pub fn encode(opcode: u8, sig: Sig, ops: &[Operand]) -> Result<Vec<u8>, String> {
    let mut out = vec![opcode];
    match sig {
        Sig::None => expect(ops, 0)?,
        Sig::Imm8 => {
            expect(ops, 1)?;
            out.push(imm(&ops[0])? as u8);
        }
        Sig::Addr16 => {
            expect(ops, 1)?;
            let a = addr(&ops[0])?;
            out.push((a >> 8) as u8);
            out.push(a as u8);
        }
        Sig::Gas => {
            expect(ops, 1)?;
            encode_gas(&ops[0], &mut out)?;
        }
        Sig::GasGas => {
            expect(ops, 2)?;
            encode_gas(&ops[0], &mut out)?;
            encode_gas(&ops[1], &mut out)?;
        }
        Sig::GasImm8 => {
            expect(ops, 2)?;
            encode_gas(&ops[0], &mut out)?;
            out.push(imm(&ops[1])? as u8);
        }
        Sig::GasImm16 => {
            expect(ops, 2)?;
            encode_gas(&ops[0], &mut out)?;
            let v = imm(&ops[1])?;
            out.push((v >> 8) as u8);
            out.push(v as u8);
        }
        Sig::Branch => {
            return Err("branch encoding is handled by the assembler (needs the LC)".into())
        }
        Sig::Move => return Err("MOVE encoding is handled by the assembler".into()),
        Sig::Fmt => return Err("FMT is not supported by the v0 assembler".into()),
        Sig::Unknown => return Err(format!("opcode >{opcode:02X} has no encoding")),
    }
    Ok(out)
}

fn expect(ops: &[Operand], n: usize) -> Result<(), String> {
    if ops.len() == n {
        Ok(())
    } else {
        Err(format!("expected {n} operand(s), found {}", ops.len()))
    }
}

/// Coerce an operand used as an immediate value.
fn imm(op: &Operand) -> Result<u16, String> {
    match *op {
        Operand::Imm8(v) => Ok(v as u16),
        Operand::Imm16(v) => Ok(v),
        Operand::Grom(v) => Ok(v),
        _ => Err("expected an immediate value, found a memory operand".into()),
    }
}

/// Coerce an operand used as an absolute GROM address.
fn addr(op: &Operand) -> Result<u16, String> {
    match *op {
        Operand::Grom(v) | Operand::Imm16(v) => Ok(v),
        Operand::Imm8(v) => Ok(v as u16),
        _ => Err("expected an address, found a memory operand".into()),
    }
}
