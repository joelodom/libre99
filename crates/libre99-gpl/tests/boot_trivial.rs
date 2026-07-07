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

//! **Milestone 0 gate** — the whole premise of the GROM rewrite in one test:
//! our own assembled GPL bytes, loaded as the system GROM, executed by the
//! *genuine* console ROM's GPL interpreter, producing an observable effect.
//!
//! We assemble a minimal GROM — a valid `>AA` header plus `BACK` at the ROM's
//! hardcoded GPL entry `>0020` (RECON.md R1) — boot it against the real
//! `994aROM.Bin`, and assert the VDP backdrop register took our value. If this
//! passes, the assembler emits real GPL and the real interpreter runs it.

use std::sync::LazyLock;

use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

/// The smallest bootable original GROM: header + `BACK imm` at the fixed entry.
fn trivial_grom(backdrop: u8) -> Vec<u8> {
    let src = format!(
        "
        GROM >0000
        BYTE >AA,>02,>00,>00      ; valid, version 2, 0 programs, reserved
        DATA >0000               ; power-up list = none
        DATA >0000               ; program list = none
        DATA >0000               ; DSR list = none
        DATA >0000               ; subprogram list = none
        DATA >0000               ; interrupt link
        DATA >0000               ; reserved

        GROM >0020               ; the ROM's fixed GPL entry
START   BACK >{backdrop:02X}
LOOP    B    LOOP               ; spin so the interpreter stays put
"
    );
    libre99_gpl::assemble(&src)
        .unwrap_or_else(|d| panic!("assembly failed: {d:?}"))
        .image
}

#[test]
fn our_gpl_sets_the_backdrop() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let grom = trivial_grom(0x17); // light-blue backdrop
    let mut m = Machine::new(console_rom, &grom);
    for _ in 0..10 {
        m.run_frame();
    }
    assert_eq!(
        m.vdp().register(7),
        0x17,
        "the real interpreter should have executed our BACK and set VDP R7"
    );
}

#[test]
fn backdrop_value_is_ours_not_a_coincidence() {
    // A different value proves the effect tracks our source, not a fixed boot state.
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let grom = trivial_grom(0x0C);
    let mut m = Machine::new(console_rom, &grom);
    for _ in 0..10 {
        m.run_frame();
    }
    assert_eq!(m.vdp().register(7), 0x0C);
}
