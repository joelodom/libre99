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

//! Reconnaissance / partial fetch-differential check: disassemble the **real**
//! console GROM and confirm our decoder tiles the same instruction boundaries
//! the interpreter fetched (RECON.md's hand-tiled boot sequence).
//!
//! This is the honest, partial form of the review's "fetch-differential oracle":
//! GAS-lengthed opcodes tile exactly; MOVE variants and a handful of unmodelled
//! ALU opcodes are best-effort, so full-trace tiling is future work. The test
//! asserts the confidently-decodable prefix and prints a listing for eyeballing
//! (`cargo test -p libre99-gpl --test disasm_recon -- --nocapture`).

use std::sync::LazyLock;

use libre99_gpl::decode::{decode_at, Flow};

static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

#[test]
fn entry_is_a_branch_to_0052() {
    let Some(console_grom) = CONSOLE_GROM.as_deref() else { skip!() };
    // RECON.md R1: the ROM enters GPL at >0020, whose bytes `40 52` are BR >0052.
    let d = decode_at(console_grom, 0x0020, 0x0020).unwrap();
    assert_eq!(d.mnemonic, "BR");
    assert_eq!(d.flow, Flow::Cond(0x0052));
}

#[test]
fn boot_prefix_tiles_like_recon() {
    let Some(console_grom) = CONSOLE_GROM.as_deref() else { skip!() };
    // From >0052 our decoder should reproduce RECON.md's hand-tiling of the
    // sound-mute stores before the first MOVE.
    let (listing, tiled) = libre99_gpl::disasm::linear(console_grom, 0x0052, 0x0052, 12);
    println!("--- disassembly from >0052 ---\n{listing}(tiled {tiled} bytes)");

    // The four sound-chip mute stores (`BE 81 00 9F/BF/DF/FF` = ST @>8400, …).
    let d = decode_at(console_grom, 0x005A, 0x005A).unwrap();
    assert_eq!(d.mnemonic, "ST");
    assert_eq!(d.len, 4);

    assert!(tiled >= 12, "expected to tile the store prefix, only {tiled} bytes");
}

#[test]
fn header_decodes() {
    let Some(console_grom) = CONSOLE_GROM.as_deref() else { skip!() };
    // GROM 0 header valid byte.
    assert_eq!(console_grom[0], 0xAA);
    assert_eq!(console_grom[1], 0x02);
}
