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

//! The M2 research probe: pin, by execution on the real console ROM, every
//! mechanism the selection list needs (see RECON.md "M2 mechanisms"):
//!
//! 1. the key-wait idiom (`SCAN` + `CEQ @>8375,>FF` + `BS`),
//! 2. MOVE variants: GROM→CPU, CPU→CPU cascade, VDP→CPU, and the
//!    computed-GROM-source form (`G*@cell`, C=1) used to walk cart headers,
//! 3. ROM-cart dispatch: entry word at `>8300`, `XML >F0` (Nouspikel: "vector
//!    in >8300"),
//! 4. GROM-cart dispatch: push the entry on the GPL subroutine stack
//!    (`>8380`, pointer byte `>8373`) and `RTN`.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
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

fn run_frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}

/// Step until PC lands in [lo,hi) or `steps` exhausted; return Some(pc).
fn step_until_pc(m: &mut Machine, lo: u16, hi: u16, steps: usize) -> Option<u16> {
    for _ in 0..steps {
        m.step();
        let pc = m.cpu().pc();
        if (lo..hi).contains(&pc) {
            return Some(pc);
        }
    }
    None
}

fn main() {
    // ---- 1. key-wait idiom ----------------------------------------------------
    let body = "        ST   @>8374,>00
        ST   @>8375,>FF
KEYLP   SCAN
        CEQ  @>8375,>FF
        BS   KEYLP
        BACK >05
SPIN    B    SPIN
";
    let grom = build(body);
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    run_frames(&mut m, 8);
    let before = m.vdp().register(7);
    m.set_key(TiKey::Num5, true);
    run_frames(&mut m, 6);
    let after = m.vdp().register(7);
    m.set_key(TiKey::Num5, false);
    println!(
        "1. key-wait: R7 before=>{before:02X} after=>{after:02X} key=>{:02X}  (want after=>05, key=>35)",
        m.bus().peek(0x8375)
    );

    // ---- 2. MOVE variants -------------------------------------------------------
    // GROM->CPU, CPU->CPU cascade, computed-GROM-source, VDP->CPU in one program.
    let body = "        MOVE >0004,G@SRC,@>8340      ; GROM -> CPU scratchpad
        ST   @>8348,>AB
        MOVE >0003,@>8348,@>8349     ; CPU -> CPU cascade fill
        DST  @>8356,SRC              ; cell >8356 holds the GROM address of SRC
        MOVE >0004,G*@>8356,@>8360   ; computed GROM source (C=1)
        MOVE >0004,G@SRC,V@>0010     ; GROM -> VDP
        MOVE >0004,V@>0010,@>8368    ; VDP -> CPU
SPIN    B    SPIN
        GROM >0100
SRC     BYTE >DE,>AD,>BE,>EF
";
    let grom = build(body);
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    run_frames(&mut m, 8);
    let rd4 = |m: &Machine, a: u16| -> Vec<u8> { (0..4).map(|i| m.bus().peek(a + i)).collect() };
    println!("2. MOVE GROM->CPU  @>8340: {:02X?} (want DE AD BE EF)", rd4(&m, 0x8340));
    println!("   MOVE CPU->CPU   @>8348: {:02X?} (want AB AB AB AB)", rd4(&m, 0x8348));
    println!("   MOVE G*@ (C=1)  @>8360: {:02X?} (want DE AD BE EF)", rd4(&m, 0x8360));
    println!("   MOVE VDP->CPU   @>8368: {:02X?} (want DE AD BE EF)", rd4(&m, 0x8368));

    // ---- 3. ROM-cart dispatch ---------------------------------------------------
    let data = require("cartridges/centipe.ctg");
    let cart = Cartridge::parse(&data).unwrap();
    // centipe: program list >6010, entry word at >6012 (RECON.md) = >6056.
    let entry = ((cart.rom[0x12] as u16) << 8) | cart.rom[0x13] as u16;
    let body = format!(
        "        DST  @>8300,>{entry:04X}
        XML  >F0
SPIN    B    SPIN
"
    );
    let grom = build(&body);
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    let hit = step_until_pc(&mut m, 0x6000, 0x8000, 3_000_000);
    println!("3. ROM dispatch via >8300 + XML >F0: PC-in-cart={hit:04X?} (want Some(~{entry:04X}))");

    // ---- 4. GROM-cart dispatch ----------------------------------------------------
    // amazing.ctg is GROM-only; its GROM >6000 header's program list gives the
    // GPL entry. Push it on the GPL subroutine stack and RTN.
    let data = require("cartridges/amazing.ctg");
    let cart = Cartridge::parse(&data).unwrap();
    let g6000 = &cart.grom.iter().find(|(a, _)| *a == 0x6000).unwrap().1;
    let pl = ((g6000[6] as u16) << 8) | g6000[7] as u16;
    let off = (pl - 0x6000) as usize;
    let gentry = ((g6000[off + 2] as u16) << 8) | g6000[off + 3] as u16;
    let nlen = g6000[off + 4] as usize;
    let name: String = g6000[off + 5..off + 5 + nlen].iter().map(|&b| b as char).collect();
    println!("4. amazing: program-list=>{pl:04X} entry=>{gentry:04X} name={name:?}");

    let body = format!(
        "        DST  @>8380,>{gentry:04X}   ; fake one frame on the GPL sub stack
        ST   @>8373,>80          ; stack pointer -> that frame
        RTN                      ; 'return' into the cartridge GPL
SPIN    B    SPIN
"
    );
    let grom = build(&body);
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    m.bus_mut().grom_record(true);
    run_frames(&mut m, 30);
    // Success = sustained GPL fetches from the cartridge GROM (>6000+).
    let log = m.bus().grom_log();
    let cart_fetches = log.iter().filter(|(a, _)| *a >= 0x6000).count();
    let first = log.iter().find(|(a, _)| *a >= 0x6000).map(|(a, _)| *a);
    println!(
        "   GROM dispatch via push+RTN: cart-GROM fetches={cart_fetches} first=>{first:04X?} (want first ~ >{gentry:04X})"
    );
}
