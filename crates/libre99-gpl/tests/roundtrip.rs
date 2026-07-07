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

//! Encode→decode roundtrips for the curated opcode set: assembling an
//! instruction and decoding the bytes back must recover the same mnemonic,
//! operands, and length. This guards the encoder and decoder against drifting
//! apart.

use libre99_gpl::decode::decode_at;
use libre99_gpl::operand::Operand;

/// Assemble a one-instruction program at GROM `>0000` and decode it back.
fn one(src_line: &str) -> libre99_gpl::decode::Decoded {
    let src = format!("        GROM >0000\n        {src_line}\n");
    let img = libre99_gpl::assemble(&src).expect("assembles").image;
    decode_at(&img, 0, 0x0000).expect("decodes")
}

#[test]
fn immediate_ops_roundtrip() {
    let d = one("BACK >17");
    assert_eq!(d.mnemonic, "BACK");
    assert_eq!(d.operands, [Operand::Imm8(0x17)]);

    let d = one("ALL >20");
    assert_eq!(d.mnemonic, "ALL");
    assert_eq!(d.operands, [Operand::Imm8(0x20)]);
}

#[test]
fn store_ops_roundtrip() {
    let d = one("ST @>83CE,>05");
    assert_eq!(d.mnemonic, "ST");
    assert_eq!(
        d.operands,
        [Operand::Cpu { addr: 0x83CE, indirect: false, index: None }, Operand::Imm8(0x05)]
    );

    // Family decode names DST by its stem "ST" (word form via the W bit).
    let d = one("DST @>8372,>3FFF");
    assert_eq!(d.mnemonic, "ST");
    assert_eq!(d.opcode, 0xBF);
    assert_eq!(d.operands[1], Operand::Imm16(0x3FFF));
}

#[test]
fn vdp_store_roundtrip() {
    let d = one("ST V@>0000,>41");
    assert_eq!(d.mnemonic, "ST");
    assert_eq!(d.operands[0], Operand::Vdp { addr: 0x0000, indirect: false, index: None });
    assert_eq!(d.operands[1], Operand::Imm8(0x41));
}

#[test]
fn absolute_and_relative_branches() {
    let d = one("B >4D12");
    assert_eq!(d.mnemonic, "B");
    assert_eq!(d.operands, [Operand::Grom(0x4D12)]);

    // A BR to a nearby absolute target within the slot.
    let src = "        GROM >0100\n        BR >0180\n";
    let img = libre99_gpl::assemble(src).unwrap().image;
    let d = decode_at(&img, 0x0100, 0x0100).unwrap();
    assert_eq!(d.mnemonic, "BR");
    assert_eq!(d.operands, [Operand::Grom(0x0180)]);
}
