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

//! Isolate the remaining MOVE variants, one program per experiment so a bad
//! encoding can't poison the next: (a) VDP→CPU with opcode >35, (b) the
//! computed-GROM-source form — brute-forcing the opcode among the C=1
//! candidates and both plausible source layouts.

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

fn build_raw(body: &str) -> Option<Vec<u8>> {
    let src = format!(
        "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
        GROM >0020
{body}"
    );
    libre99_gpl::assemble(&src).ok().map(|a| a.image)
}

fn run(grom: &[u8], frames: usize) -> Machine {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    for _ in 0..frames {
        m.run_frame();
    }
    m
}

fn rd4(m: &Machine, a: u16) -> Vec<u8> {
    (0..4).map(|i| m.bus().peek(a + i)).collect()
}

fn main() {
    // (a) VDP -> CPU in isolation.
    let body = "        MOVE >0004,G@SRC,V@>0010
        MOVE >0004,V@>0010,@>8360
SPIN    B    SPIN
        GROM >0100
SRC     BYTE >DE,>AD,>BE,>EF
";
    let m = run(&build_raw(body).unwrap(), 8);
    println!("(a) VDP->CPU @>8360: {:02X?} (want DE AD BE EF)", rd4(&m, 0x8360));

    // (b) computed GROM source: brute-force opcode x layout.
    // Cell >8356 holds >0100 (the GROM address of SRC). Candidate opcodes:
    // C=1 with/without V, i.e. >33, >37, >3B, >3F; layouts: src as GAS cell
    // (one byte >56), indirect GAS (>90 >56), or raw 16-bit cell addr.
    for op in [0x33u8, 0x37, 0x3B, 0x3F, 0x32, 0x36] {
        for (lname, src_bytes) in [
            ("GAS-cell", vec![0x56u8]),
            ("GAS-indirect", vec![0x90u8, 0x56]),
            ("raw16", vec![0x83u8, 0x56]),
        ] {
            let src_list: Vec<String> = src_bytes.iter().map(|b| format!(">{b:02X}")).collect();
            let body = format!(
                "        DST  @>8356,>0100
        BYTE >{op:02X}
        DATA >0004
        BYTE >60                 ; dst GAS = @>8360
        BYTE {}
SPIN    B    SPIN
        GROM >0100
SRC     BYTE >DE,>AD,>BE,>EF
",
                src_list.join(",")
            );
            let Some(grom) = build_raw(&body) else { continue };
            let m = run(&grom, 8);
            let got = rd4(&m, 0x8360);
            if got == [0xDE, 0xAD, 0xBE, 0xEF] {
                println!("(b) MATCH op=>{op:02X} layout={lname} @>8360: {got:02X?}");
            }
        }
    }
    println!("(b) sweep done");

    // (c) Read a mounted ROM cart's CPU header with MOVE (CPU source, biased
    //     16-bit GAS) and compare a VDP-buffer byte with CEQ — the two menu
    //     building blocks the VDP-window strategy needs.
    let data = require("cartridges/centipe.ctg");
    let cart = libre99_core::cartridge::Cartridge::parse(&data).unwrap();
    let body = "        MOVE >0004,@>6000,@>8360     ; CPU cart window -> scratchpad
        MOVE >0004,G@SRC,V@>0010     ; place DE AD BE EF in VDP
        CEQ  V@>0010,>DE             ; compare a VDP byte with an immediate
        BS   YES
        BACK >0E
SPIN1   B    SPIN1
YES     BACK >03
SPIN2   B    SPIN2
        GROM >0100
SRC     BYTE >DE,>AD,>BE,>EF
";
    let grom = build_raw(body).unwrap();
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..8 {
        m.run_frame();
    }
    println!(
        "(c) CPU cart header via MOVE @>8360: {:02X?} (want AA FF 00 00); CEQ-on-VDP backdrop=>{:02X} (want 03)",
        rd4(&m, 0x8360),
        m.vdp().register(7)
    );
}
