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

//! M1 gate for the Console ROM rewrite (Phase 2): our from-scratch **GPL
//! interpreter**, built by `libre99asm`, boots and executes GPL bytecode out of
//! GROM on our emulator.
//!
//! This is the ROM analogue of the GROM track's M0 (`libre99-gpl`'s
//! `boot_trivial.rs`, where our GPL reaches VDP R7 on the *authentic* ROM):
//! here our *own* console ROM interprets a trivial GROM. It proves the whole
//! reset → fetch → dispatch → opcode architecture — our machine code running
//! from `>0000`, fetching GPL through the `>9800` ports, dispatching via the
//! authentic table structure, and executing `BACK` to set the VDP backdrop.
//!
//! Increment 1 implements the skeleton + `BACK`; later M1 increments add the
//! remaining opcode families, each gated the same way (P9/P8).

use libre99_asm::system_rom::{build_console_rom, ROM_SIZE};
use libre99_core::machine::Machine;

/// A headerless trivial GROM whose fixed boot entry `>0020` is `BACK >07`
/// repeated. The console ROM hardcodes a jump to GROM `>0020` after reset
/// (RECON §1), so no `>AA` header is needed. Repeating `BACK` keeps the
/// interpreter on the one implemented opcode (it never falls into an
/// unimplemented handler), setting VDP R7 = `>07` every pass.
fn trivial_back_grom(color: u8) -> Vec<u8> {
    let mut grom = vec![0u8; 0x6000];
    let mut a = 0x20;
    while a < 0x60 {
        grom[a] = 0x04; // GPL BACK
        grom[a + 1] = color; // immediate: the backdrop colour
        a += 2;
    }
    grom
}

#[test]
fn our_rom_interprets_gpl_and_back_sets_the_backdrop() {
    let rom = build_console_rom().expect("console ROM assembles");
    assert_eq!(rom.len(), ROM_SIZE, "the console ROM is exactly 8 KiB");
    assert_eq!(&rom[0..4], [0x83, 0xE0, 0x00, 0x24], "reset vector WP=>83E0 PC=>0024");

    let grom = trivial_back_grom(0x07);
    let mut m = Machine::new(&rom, &grom);
    m.reset();
    // A few frames for the reset kernel + interpreter to fetch and run `BACK`.
    for _ in 0..4 {
        m.run_frame();
    }
    assert_eq!(
        m.vdp().register(7),
        0x07,
        "our ROM's GPL interpreter must execute BACK >07 from GROM and set VDP R7"
    );

    // And it is genuinely reading the operand: a different colour lands too.
    let grom = trivial_back_grom(0x0C);
    let mut m = Machine::new(&rom, &grom);
    m.reset();
    for _ in 0..4 {
        m.run_frame();
    }
    assert_eq!(m.vdp().register(7), 0x0C, "BACK must read its immediate operand from GROM");
}

/// The `L99R` self-identification marker: present at its fixed home, carrying
/// exactly this workspace's version (one number for the emulator, the console
/// ROM/GROM, and TI PYTHON — the system information screen shows them all).
/// Also the tripwire for the free gap the marker lives in: if ROM code ever
/// grows over `>0BF0`, the marker bytes get clobbered and this fails.
#[test]
fn rom_carries_the_l99r_version_marker() {
    use libre99_core::sysinfo;

    let rom = build_console_rom().expect("console ROM assembles");
    assert_eq!(
        &rom[sysinfo::ROM_MARKER_ADDR..sysinfo::ROM_MARKER_ADDR + 4],
        sysinfo::ROM_MARKER_MAGIC,
        "L99R marker missing at >0BF0 — did code grow over the free gap?"
    );
    assert_eq!(
        sysinfo::rom_marker_version(&rom).as_deref(),
        Some(env!("CARGO_PKG_VERSION")),
        "the ROM marker version must be the workspace version"
    );
}
