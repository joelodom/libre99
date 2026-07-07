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

//! Throwaway experiment: pin the GPL MOVE encoding by executing candidates on
//! the real console ROM and observing VDP registers / VRAM.
//!
//! Step 1 verifies the register-setup MOVE hand-laid from the boot trace
//! (`39 count:imm16 reg:byte src:grom16`, RECON.md). Step 2 brute-forces the
//! opcode + operand layout for a GROM→VDP-RAM block move by checking whether a
//! distinctive source pattern lands in VRAM.

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

fn header() -> String {
    "        GROM >0000
        BYTE >AA,>02,>00,>00
        DATA >0000,>0000,>0000,>0000,>0000,>0000
"
    .to_string()
}

/// Assemble `body` (placed at >0020) into a GROM image, or print diagnostics.
fn build(body: &str) -> Option<Vec<u8>> {
    let src = format!("{}        GROM >0020\n{body}", header());
    match libre99_gpl::assemble(&src) {
        Ok(a) => Some(a.image),
        Err(d) => {
            eprintln!("asm error: {d:?}");
            None
        }
    }
}

fn boot(grom: &[u8], frames: usize) -> Machine {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    for _ in 0..frames {
        m.run_frame();
    }
    m
}

fn main() {
    // ---- Step 1: register-setup MOVE (opcode 0x39, hand-laid) ----------------
    let body = "        BYTE >39
        DATA >0008
        BYTE >00
        DATA VREGS
LOOP    B    LOOP
VREGS   BYTE >00,>E0,>F0,>0E,>F9,>86,>F8,>F7
";
    if let Some(grom) = build(body) {
        let m = boot(&grom, 8);
        let regs: Vec<String> = (0..8).map(|n| format!("R{}=>{:02X}", n, m.vdp().register(n))).collect();
        println!("step1 register move => {}", regs.join(" "));
        println!("  (want R1=>E0 R2=>F0 R3=>0E R4=>F9 R5=>86 R6=>F8 R7=>F7)");
    }

    // ---- Step 2: brute-force a GROM->VDP-RAM block move ----------------------
    // First set registers (so VDP is sane), then try a candidate move of 4
    // distinctive bytes from GROM label SRC to VDP >0000, and read VRAM back.
    let want = [0xDEu8, 0xAD, 0xBE, 0xEF];
    // Candidate operand layouts, given (count, vdp_dst, grom_src):
    // each returns the operand bytes following the opcode.
    type Layout = fn(u16, u16, u16) -> Vec<u8>;
    let layouts: &[(&str, Layout)] = &[
        ("cnt16,vdpGAS,grom16", |c, d, s| {
            let mut v = vec![(c >> 8) as u8, c as u8];
            v.extend(vdp_gas(d));
            v.extend([(s >> 8) as u8, s as u8]);
            v
        }),
        ("cnt16,grom16,vdpGAS", |c, d, s| {
            let mut v = vec![(c >> 8) as u8, c as u8, (s >> 8) as u8, s as u8];
            v.extend(vdp_gas(d));
            v
        }),
        ("cnt16,vdp16raw,grom16", |c, d, s| {
            vec![(c >> 8) as u8, c as u8, (d >> 8) as u8, d as u8, (s >> 8) as u8, s as u8]
        }),
        ("cnt8,vdpGAS,grom16", |c, d, s| {
            let mut v = vec![c as u8];
            v.extend(vdp_gas(d));
            v.extend([(s >> 8) as u8, s as u8]);
            v
        }),
        ("vdpGAS,grom16,cnt16", |c, d, s| {
            let mut v = vdp_gas(d);
            v.extend([(s >> 8) as u8, s as u8, (c >> 8) as u8, c as u8]);
            v
        }),
    ];

    for op in 0x20u8..0x40 {
        for (name, layout) in layouts {
            // Build: reg-setup move, then candidate move (op + operands), then loop.
            // We assemble by hand-laying bytes; SRC/DST are known GROM offsets we
            // compute after placing labels, so use a fixed layout with a data
            // block at a known address.
            //
            // Program layout at >0020:
            //   reg move (7 bytes)         >0020..>0026
            //   candidate move             >0027..
            //   B LOOP
            //   VREGS (8)
            //   SRC (4 = want)
            // We must know SRC's GROM address to encode grom16. Assemble twice:
            // once to learn SRC, but simpler — put SRC at a FIXED padded address.
            let src_addr = 0x0100u16; // we will DATA-place `want` at >0100
            let cand = {
                let mut v = vec![op];
                v.extend(layout(4, 0x0000, src_addr));
                v
            };
            let cand_bytes: Vec<String> = cand.iter().map(|b| format!(">{b:02X}")).collect();
            let body = format!(
                "        BYTE >39
        DATA >0008
        BYTE >00
        DATA VREGS
        BYTE {}
LOOP    B    LOOP
VREGS   BYTE >00,>E0,>F0,>0E,>F9,>86,>F8,>F7
        GROM >0100
SRC     BYTE >DE,>AD,>BE,>EF
",
                cand_bytes.join(",")
            );
            let Some(grom) = build(&body) else { continue };
            let m = boot(&grom, 8);
            let got = [m.vdp().vram(0), m.vdp().vram(1), m.vdp().vram(2), m.vdp().vram(3)];
            if got == want {
                println!("MATCH op=>{op:02X} layout={name} => VRAM {got:02X?}");
            }
        }
    }
    println!("step2 done");
}

/// Encode a VDP address as a GAS operand (12/16-bit with the V bit set).
fn vdp_gas(addr: u16) -> Vec<u8> {
    let mut out = Vec::new();
    libre99_gpl::operand::encode_gas(
        &libre99_gpl::operand::Operand::Vdp { addr, indirect: false, index: None },
        &mut out,
    )
    .unwrap();
    out
}
