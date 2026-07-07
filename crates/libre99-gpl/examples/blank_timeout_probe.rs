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

//! Probe for the structure-handoff audit's HAZARD-1: `START` seeds every ISR
//! scratchpad cell EXCEPT `>83D6/7` (the screen-blank timeout the kept ROM's
//! VBLANK ISR advances every frame and, on wrap-to-0, uses to blank the display
//! via `>83D4`/R1). Since `reset()` (F5) preserves RAM, a game that left `>83D6`
//! high makes our un-seeded title blank before the user presses a key.
//!
//! Two questions, answered differentially (authentic ROM vs our GROM):
//!  (1) Is the blank mechanism live on this emulator? (poke `>83D6` near wrap at
//!      the title-wait, watch VDP R1 lose its display-enable bit.)
//!  (2) Does our boot leave `>83D6` un-seeded where authentic seeds it? (compare
//!      `>83D6` right after boot with RAM pre-dirtied, simulating F5-from-game.)

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994AGROM.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

/// R1 bit 6 (>40) is display-enable: >E0 = on, >A0 = blanked.
fn display_on(m: &Machine) -> bool {
    m.vdp().register(1) & 0x40 != 0
}

fn run(label: &str, grom: &[u8], cart: Option<&Cartridge>) {
    println!("\n===== {label} =====");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    if let Some(c) = cart {
        m.mount_cartridge(c);
    }
    m.reset();
    for _ in 0..180 { m.run_frame(); } // settle at the title-wait
    println!(
        "  at title-wait: R1={:02X} display_on={}  >83D6={:02X}{:02X}",
        m.vdp().register(1), display_on(&m), m.bus().peek(0x83D6), m.bus().peek(0x83D7)
    );

    // (1) Mechanism test: shove >83D6 near wrap and watch, without any keypress.
    m.bus_mut().poke_word(0x83D6, 0xFFF0);
    let mut blanked_at = None;
    for f in 0..90 {
        m.run_frame();
        if !display_on(&m) {
            blanked_at = Some(f);
            break;
        }
    }
    match blanked_at {
        Some(f) => println!(
            "  poked >83D6=FFF0 -> DISPLAY BLANKED after {f} frames (R1={:02X}, >83D6={:02X}{:02X}) — mechanism LIVE",
            m.vdp().register(1), m.bus().peek(0x83D6), m.bus().peek(0x83D7)
        ),
        None => println!(
            "  poked >83D6=FFF0 -> display stayed on over 90 frames (R1={:02X}, >83D6={:02X}{:02X}) — mechanism not observed",
            m.vdp().register(1), m.bus().peek(0x83D6), m.bus().peek(0x83D7)
        ),
    }
}

/// (2) F5-from-game: dirty RAM (as a running game would), reset, boot, and read
/// >83D6 — does our START re-seed it to 0 the way the neighbouring cells are?
fn f5_seed_check(label: &str, grom: &[u8]) {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.reset();
    for _ in 0..180 { m.run_frame(); }
    m.bus_mut().poke_word(0x83D6, 0xBEEF); // a game's leftover timeout
    m.reset(); // F5 — RAM preserved
    // Sample across a full boot-to-title-wait: if START (or the ISR) ever
    // re-seeds >83D6, it drops back near 0; if not, it keeps ticking from BEEF.
    let mut total = 0;
    for step in [3usize, 97, 200] {
        for _ in 0..step { m.run_frame(); }
        total += step;
        let d = m.bus().peek_word(0x83D6);
        println!(
            "  [{label}] F5(>83D6=BEEF) +{total:>3}f -> >83D6={d:04X} ({})",
            if d < 0x0400 { "RE-SEEDED near 0" } else { "un-seeded, ticking from BEEF" }
        );
    }
}

fn main() {
    let cart = libre99_core::third_party::load("cartridges/TI-Invaders.ctg")
        .map(|d| Cartridge::parse(&d).unwrap());
    let ours = libre99_gpl::system_grom::build_console_grom().unwrap();
    run("AUTHENTIC GROM (oracle)", &AUTHENTIC_GROM, cart.as_ref());
    run("OURS", &ours, cart.as_ref());
    println!("\n--- F5 re-seed check ---");
    f5_seed_check("AUTHENTIC", &AUTHENTIC_GROM);
    f5_seed_check("OURS", &ours);
}
