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

//! Boot integration test — **Gate (b): the machine boots to the master title
//! screen** using only the console ROM + GROM.
//!
//! Reaching the title screen exercises the CPU, GROM access, and the VDP all
//! together: the console's GPL interpreter (machine code in ROM, bytecode in
//! GROM) has to run correctly enough to set up the VDP and draw the screen, and
//! the vertical-blank interrupt has to be delivered for its timing. This is the
//! single most important correctness milestone.
//!
//! The real ROM/GROM images are loaded at run time from the git-ignored
//! `third-party/` directory (or `$LIBRE99_THIRD_PARTY`); when they are absent the
//! test skips green with a notice.

use std::collections::HashSet;
use std::sync::LazyLock;
use libre99_core::machine::Machine;
use libre99_core::vdp::{HEIGHT, WIDTH};

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

/// Write the framebuffer as a binary PPM (P6) for manual inspection.
fn dump_ppm(fb: &[u32], name: &str) {
    let mut out = format!("P6\n{} {}\n255\n", WIDTH, HEIGHT).into_bytes();
    for &px in fb {
        out.push((px >> 16) as u8);
        out.push((px >> 8) as u8);
        out.push(px as u8);
    }
    let path = std::env::temp_dir().join(name);
    let _ = std::fs::write(&path, out);
    eprintln!("wrote {}", path.display());
}

// Gate (b): the console boots into its GPL interpreter, which reads the master
// title-screen program out of console GROM and draws it into VRAM. This proves
// the CPU, GROM access (prefetch + address-port semantics), and VDP are all
// sound together.
#[test]
fn boots_to_master_title_screen() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);

    // Run ~3 seconds of emulated time to let the power-up self-test and the GPL
    // title program finish.
    for _ in 0..180 {
        m.run_frame();
    }

    let mut fb = vec![0u32; WIDTH * HEIGHT];
    m.render(&mut fb);
    dump_ppm(&fb, "libre99_boot.ppm");

    eprintln!(
        "after boot: PC=>{:04X} WP=>{:04X} VDP R0=>{:02X} R1=>{:02X} R7=>{:02X}",
        m.cpu().pc(),
        m.cpu().wp(),
        m.vdp().register(0),
        m.vdp().register(1),
        m.vdp().register(7),
    );

    // The console must have enabled the display.
    assert!(
        m.vdp().register(1) & 0x40 != 0,
        "VDP display should be enabled after boot"
    );

    // The title screen has real, multi-colored content rather than a uniform
    // backdrop.
    let distinct: HashSet<u32> = fb.iter().copied().collect();
    assert!(
        distinct.len() >= 3,
        "expected a drawn title screen with several colors, saw {} distinct colors",
        distinct.len()
    );
}
