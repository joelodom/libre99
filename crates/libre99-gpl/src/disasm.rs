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

//! A small GPL disassembler over [`decode_at`], for reconnaissance (reading the
//! authentic firmware's title/menu code) and for eyeballing our own output.
//!
//! Two modes: [`linear`] tiles instructions sequentially from a start offset
//! until it hits an unknown opcode, and [`format_operand`] renders GAS operands
//! in the assembler's own syntax so listings round-trip mentally.

use crate::decode::{decode_at, DecodeError, Flow};
use crate::operand::Operand;

/// Render one operand in assembler syntax (`@>83CE`, `V@>0380`, `*@…`, `G@…`).
pub fn format_operand(op: &Operand) -> String {
    match *op {
        Operand::Cpu { addr, indirect, index } => decorate("@", addr, indirect, index),
        Operand::Vdp { addr, indirect, index } => decorate("V@", addr, indirect, index),
        Operand::Imm8(v) => format!(">{v:02X}"),
        Operand::Imm16(v) => format!(">{v:04X}"),
        Operand::Grom(v) => format!(">{v:04X}"),
    }
}

fn decorate(prefix: &str, addr: u16, indirect: bool, index: Option<u8>) -> String {
    let star = if indirect { "*" } else { "" };
    let base = format!("{star}{prefix}>{addr:04X}");
    match index {
        Some(ix) => format!("{base}(@>{:04X})", 0x8300u16.wrapping_add(ix as u16)),
        None => base,
    }
}

/// Disassemble sequentially from `off` (slot-absolute `addr`) for up to `max`
/// instructions, stopping at an unknown opcode or the end of `img`. Returns the
/// listing plus the number of bytes successfully tiled.
pub fn linear(img: &[u8], off: usize, addr: u16, max: usize) -> (String, usize) {
    let mut out = String::new();
    let mut o = off;
    let mut a = addr;
    let mut tiled = 0usize;
    for _ in 0..max {
        match decode_at(img, o, a) {
            Ok(d) => {
                let ops: Vec<String> = d.operands.iter().map(format_operand).collect();
                out.push_str(&format!(">{:04X}  {:<6} {}\n", a, d.mnemonic, ops.join(", ")));
                o += d.len;
                a = a.wrapping_add(d.len as u16);
                tiled += d.len;
                if matches!(d.flow, Flow::Stop) {
                    break;
                }
            }
            Err(DecodeError::Unknown(op)) => {
                out.push_str(&format!(">{a:04X}  .byte  >{op:02X}   ; unknown opcode\n"));
                break;
            }
            Err(DecodeError::Truncated) => break,
        }
    }
    (out, tiled)
}
