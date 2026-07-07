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

//! M4 (TI PYTHON) research probe: pin the arithmetic semantics that would
//! otherwise burn implementation time — DMUL product placement, DDIV
//! quotient/remainder placement, DADD word math, CASE jump tables, and
//! indexed GAS addressing — by executing them on the real console ROM.

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
{body}"
    );
    libre99_gpl::assemble(&src).unwrap_or_else(|d| panic!("asm: {d:?}")).image
}

fn run(grom: &[u8]) -> Machine {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    for _ in 0..8 {
        m.run_frame();
    }
    m
}

fn w(m: &Machine, a: u16) -> u16 {
    ((m.bus().peek(a) as u16) << 8) | m.bus().peek(a + 1) as u16
}

fn main() {
    // DMUL: word x word -> where does the 32-bit product land?
    // DDIV: what exactly is the dividend, where do quotient/remainder land?
    // DADD/DSUB word forms.
    let body = "        DST  @>8340,>0007
        DST  @>8342,>0006
        DMUL @>8340,@>8342      ; product of 7*6 -> ?
        DST  @>8348,>0000       ; dividend high word
        DST  @>834A,>002F       ; dividend low word (47)
        DST  @>834E,>0005
        DDIV @>8348,@>834E      ; 47/5 -> q?, r?
        DST  @>8350,>1234
        DADD @>8350,>0111       ; -> 1345
        DST  @>8352,>0005
        DSUB @>8352,>0008       ; -> FFFD (-3, two's complement)
SPIN    B    SPIN
";
    let m = run(&build(body));
    println!(
        "DMUL 7*6: >8340={:04X} >8342={:04X} >8344={:04X}",
        w(&m, 0x8340),
        w(&m, 0x8342),
        w(&m, 0x8344)
    );
    println!(
        "DDIV 47/5: >8348={:04X} >834A={:04X} >834C={:04X}  (looking for q=9 r=2)",
        w(&m, 0x8348),
        w(&m, 0x834A),
        w(&m, 0x834C)
    );
    println!("DADD: >8350={:04X} (want 1345)   DSUB: >8352={:04X} (want FFFD)", w(&m, 0x8350), w(&m, 0x8352));

    // CASE: jump table dispatch, and indexed GAS.
    let body = "        ST   @>8360,>02
        ST   @>8368,>03          ; index cell value
        ST   @>8300(@>8368),>77  ; indexed store -> >8303 expected
        CASE @>8360              ; PC += 2*2 -> skips two 2-byte branches
        BR   L0
        BR   L1
        BR   L2
L0      BACK >01
S0      B    S0
L1      BACK >02
S1      B    S1
L2      BACK >03
S2      B    S2
";
    let m = run(&build(body));
    println!(
        "CASE 2 -> backdrop=>{:02X} (want 03); indexed ST: >8303=>{:02X} (want 77)",
        m.vdp().register(7),
        m.bus().peek(0x8303)
    );

    // Indirect GAS: a pointer cell holding a full CPU address; ST through it,
    // read back through it, and bump the pointer to walk a table.
    // Hypothesis: indirection reads a BYTE pointer — final = >8300 + byte at
    // the cell (scratchpad-only pointers).
    let body = "        ST   @>8356,>62          ; byte pointer -> >8362
        ST   *@>8356,>55         ; store through the pointer
        INC  @>8356              ; pointer -> >8363
        ST   *@>8356,>66
        ST   @>8366,*@>8356      ; read back through the pointer
SPIN    B    SPIN
";
    let m = run(&build(body));
    println!(
        "indirect: >8362=>{:02X} (want 55) >8363=>{:02X} (want 66) >8366=>{:02X} (want 66)",
        m.bus().peek(0x8362),
        m.bus().peek(0x8363),
        m.bus().peek(0x8366)
    );

    // VDP indirect (*V@cell — a word cell holding a VDP address): hand-lay the
    // two candidate encodings of the source operand and see which reads the
    // VDP byte. Candidate A: 12-bit field = the CELL's scratchpad offset
    // (`B0 56`). Candidate B: 16-bit field = the cell's full CPU address
    // (`BF 83 56`).
    for (name, src) in [("A: B0 56", ">B0,>56"), ("B: BF 83 56", ">BF,>83,>56")] {
        let body = format!(
            "        MOVE >0004,G@SRC,V@>0123 ; DE AD BE EF into VDP >0123
        DST  @>8356,>0123        ; word cell holds the VDP address
        BYTE >BC,>60,{src}       ; ST @>8360, *V@(cell >8356)
SPIN    B    SPIN
        GROM >0100
SRC     BYTE >DE,>AD,>BE,>EF
"
        );
        let m = run(&build(&body));
        println!(
            "VDP indirect candidate {name}: @>8360=>{:02X} (want DE)",
            m.bus().peek(0x8360)
        );
    }
}
