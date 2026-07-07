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

//! Empirically pin the GPL `IO` (opcode >F6) CRU-output recipe that enables the
//! 9901 VDP vertical-blank interrupt (mask bit 2). Ground truth from the ROM
//! trace: the console executes `SBO` with R12=>0004 (CRU bit address 2). The
//! authentic console GROM boot does it with a chain of `ST @>8303,imm ; IO
//! @>8302,#3` (opcode >F6). We assemble a minimal GROM per candidate, boot it,
//! and read `tms9901.vdp_interrupt_enabled()`.
//! Run from repo root: `cargo run -p libre99-gpl --example cru_experiment`.

use std::sync::LazyLock;

use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

fn build(body: &str) -> Vec<u8> {
    let src = format!(
        "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
        B    START
        GROM >0060
START   ST   @>8373,>7E
{body}
        ST   @>8374,>00
        ST   @>8375,>FF
LOOP    SCAN
        B    LOOP
"
    );
    match libre99_gpl::asm::assemble(&src) {
        Ok(a) => a.image,
        Err(d) => panic!("assembly failed: {d:?}"),
    }
}

fn try_recipe(label: &str, body: &str) {
    let grom = build(body);
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    m.reset();
    for _ in 0..20 {
        m.run_frame();
    }
    let on = m.bus().tms9901.vdp_interrupt_enabled();
    let mask = m.bus().tms9901.int_mask();
    println!("{label:<44} mask_bit2={on}  full_mask={mask:04X}");
}

fn main() {
    // L: replay the authentic ST+IO chain verbatim (>008C..>00AD) as raw bytes.
    try_recipe(
        "L: authentic ST+IO chain (raw)",
        "        ST   @>8302,>00\n\
         \x20       BYTE >BF,>03,>03,>08\n        BYTE >F6,>02,>03\n\
         \x20       BYTE >BF,>03,>10,>01\n        BYTE >F6,>02,>03\n\
         \x20       BYTE >BE,>03,>18\n        BYTE >F6,>02,>03\n\
         \x20       BYTE >84,>00\n        BYTE >BE,>03,>02\n        BYTE >F6,>02,>03\n\
         \x20       BYTE >BE,>03,>01\n        BYTE >F6,>02,>03",
    );
    // M: just the interrupt-enable IO, with count primed via the word store that
    // sets >8304=01 (as the authentic chain does), then addr=2.
    try_recipe(
        "M: DST8303=1001; ST8303=02; IO",
        "        ST   @>8302,>00\n        DST  @>8303,>1001\n        ST   @>8303,>02\n        BYTE >F6,>02,>03",
    );
    // N: only the >18 (keyboard) IO then bit-2 IO — is a priming IO required?
    try_recipe(
        "N: ST8303=18;IO; ST8303=02;IO",
        "        ST   @>8302,>00\n        ST   @>8304,>01\n\
         \x20       ST   @>8303,>18\n        BYTE >F6,>02,>03\n\
         \x20       ST   @>8303,>02\n        BYTE >F6,>02,>03",
    );
    // O: single IO, addr=2, count=1 explicit (recipe F equivalent, re-confirm).
    try_recipe(
        "O: ST8302=0;ST8303=2;ST8304=1; IO",
        "        ST   @>8302,>00\n        ST   @>8303,>02\n        ST   @>8304,>01\n        BYTE >F6,>02,>03",
    );
    // Minimal-prefix search: which priming IO(s) before the bit-2 IO are needed?
    let io = |imm_word: &str| -> String {
        // ST @>8303,<imm> (word) then IO @>8302,#3
        format!("        BYTE >BF,>03,{imm_word}\n        BYTE >F6,>02,>03\n")
    };
    let io_b2 = "        BYTE >BE,>03,>02\n        BYTE >F6,>02,>03"; // ST@>8303,>02;IO
    try_recipe(
        "Pa: IO(0308) ; IO(02)",
        &format!("        ST   @>8302,>00\n{}{io_b2}", io(">03,>08")),
    );
    try_recipe(
        "Pb: IO(1001) ; IO(02)",
        &format!("        ST   @>8302,>00\n{}{io_b2}", io(">10,>01")),
    );
    try_recipe(
        "Pc: IO(0308) ; IO(1001) ; IO(02)",
        &format!("        ST   @>8302,>00\n{}{}{io_b2}", io(">03,>08"), io(">10,>01")),
    );
    // Pi: data byte at >8300 = FF (the INV target), addr=2, count=1, IO @>8302,#3.
    try_recipe(
        "Pi: ST8300=FF; addr=2; cnt=1; IO",
        "        ST   @>8300,>FF\n        ST   @>8302,>00\n        ST   @>8303,>02\n        ST   @>8304,>01\n        BYTE >F6,>02,>03",
    );
    // Pj: data word >8300=FFFF (both bytes), addr=2, count=1.
    try_recipe(
        "Pj: DST8300=FFFF; addr=2; cnt=1; IO",
        "        DST  @>8300,>FFFF\n        ST   @>8302,>00\n        ST   @>8303,>02\n        ST   @>8304,>01\n        BYTE >F6,>02,>03",
    );
    // Pe: full L minus the trailing bit-1 IO (still enable bit 2?).
    try_recipe(
        "Pe: L minus trailing IO(01)",
        "        ST   @>8302,>00\n\
         \x20       BYTE >BF,>03,>03,>08\n        BYTE >F6,>02,>03\n\
         \x20       BYTE >BF,>03,>10,>01\n        BYTE >F6,>02,>03\n\
         \x20       BYTE >BE,>03,>18\n        BYTE >F6,>02,>03\n\
         \x20       BYTE >84,>00\n        BYTE >BE,>03,>02\n        BYTE >F6,>02,>03",
    );
    // Pk: just the INV(sets >8300=FF) + bit-2 IO, count primed explicitly.
    try_recipe(
        "Pk: INV8300; addr=2; cnt=1; IO",
        "        ST   @>8300,>00\n        BYTE >84,>00\n        ST   @>8302,>00\n        ST   @>8303,>02\n        ST   @>8304,>01\n        BYTE >F6,>02,>03",
    );
}
