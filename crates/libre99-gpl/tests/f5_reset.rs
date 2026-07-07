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

//! Regression gate for the 2026-07-03 field bug: **play TI Invaders, press F5,
//! pass the title — then the menu can't launch anything** (and the post-F5
//! power-on beep is missing).
//!
//! Root cause: the GPL `IO` CRU-output list is FOUR fields, not three — the
//! ROM reads the output bit through a **data-ADDRESS byte at list+3 (`>8305`)**,
//! a `>83xx` scratchpad offset. Cold boots hid an uninitialized `>8305` because
//! the emulator zeroes RAM (offset 0 → data at `>8300`, exactly where `START`
//! puts it). After F5 — a CPU-only reset, RAM preserved — a game's leftover
//! `>8305` (TI Invaders leaves `>80`) makes `START`'s own arming `IO` read its
//! bit from the wrong cell and **write CRU bit 2 = 0, disarming the 9901 VDP
//! interrupt**: no ISR, no beeps, and the menu's `SBWAIT` beep-drain spins
//! forever on the first selection. See DEBUGGING.md case study 9 and
//! `examples/{f5_press2_probe,f5_mask_bisect}.rs` (libre99-core).

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static INVADERS: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/TI-Invaders.ctg"));
static DISK_DSR: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/Disk.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

fn tap(m: &mut Machine, k: TiKey, hold: usize, settle: usize) {
    m.set_key(k, true);
    for _ in 0..hold {
        m.run_frame();
    }
    m.set_key(k, false);
    for _ in 0..settle {
        m.run_frame();
    }
}

/// Press the menu digit `2` and report whether the cartridge actually starts
/// (sustained GPL fetches from the cartridge GROM slot `>6000-7FFF`).
fn press_2_launches(m: &mut Machine) -> bool {
    m.bus_mut().grom_record(true);
    tap(m, TiKey::Num2, 6, 0);
    for _ in 0..240 {
        m.run_frame();
    }
    let cart_fetches = m
        .bus_mut()
        .grom_log()
        .iter()
        .filter(|(a, _)| (0x6000..0x8000).contains(a))
        .count();
    m.bus_mut().grom_record(false);
    cart_fetches > 500
}

#[test]
fn f5_after_gameplay_still_launches_from_the_menu() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(invaders) = INVADERS.as_deref() else { skip!() };
    let Some(disk_dsr) = DISK_DSR.as_deref() else { skip!() };
    let grom = our_grom();
    let cart = Cartridge::parse(invaders).expect("parse TI-Invaders.ctg");
    let mut m = Machine::new(console_rom, &grom);
    m.mount_cartridge(&cart);
    m.load_disk_controller(disk_dsr); // match the app: DSR power-up runs at boot
    m.reset();
    for _ in 0..200 {
        m.run_frame();
    }

    // Cold path: title -> menu -> launch TI Invaders (entry 2).
    tap(&mut m, TiKey::Space, 3, 320);
    assert!(press_2_launches(&mut m), "cold-boot menu must launch TI Invaders");

    // "Play" a little so the game scribbles scratchpad the way a user's
    // session does (it leaves its own value in >8305, among others).
    tap(&mut m, TiKey::Num1, 3, 120);
    tap(&mut m, TiKey::S, 3, 60);
    tap(&mut m, TiKey::D, 3, 60);

    // F5. RAM survives (reset is CPU-only); the boot must re-arm the ISR
    // regardless of what the game left behind.
    m.reset();
    for _ in 0..200 {
        m.run_frame();
    }
    assert!(
        m.bus().tms9901.vdp_interrupt_enabled(),
        "the boot's arming IO disarmed the 9901 VDP interrupt after F5 \
         (uninitialized IO-list data-address byte at >8305?)"
    );

    // Title -> menu -> the selection must still work.
    tap(&mut m, TiKey::Space, 3, 320);
    assert_eq!(
        m.bus().peek(0x83CE),
        0,
        "the title-exit key click never drained after F5 — the ISR is dead"
    );
    assert!(
        press_2_launches(&mut m),
        "after F5 from gameplay, pressing 2 on the menu must relaunch TI Invaders"
    );
}
