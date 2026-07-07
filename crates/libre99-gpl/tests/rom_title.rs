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

//! **Console ROM M1 exit gate.** Our GROM's master title screen must paint
//! *pixel-identically* under OUR console ROM (`libre99_asm::build_console_rom` —
//! our from-scratch GPL interpreter) and under the authentic `994aROM.Bin`:
//! every VDP register and all of VRAM match byte-for-byte. This is the ROM-track
//! analogue of the GROM track's `title_screen.rs`, but it isolates the ROM —
//! **same GROM, our interpreter vs TI's** — so any divergence is our ROM's.
//!
//! Scope note (M1): the visible title (colour bars, banner, chip logo, prompt,
//! copyright, fonts, colours) is fully drawn and the display is enabled before
//! the boot's peripheral power-up scan (`XML >19`, still an M3 element) runs, so
//! this gate covers the whole *painted* title. After it, our ROM halts cleanly
//! at the unimplemented `XML` (a loud breadcrumb, not a crash); the authentic
//! ROM continues into its key-wait. Neither writes VRAM/registers past the
//! paint, so the two states are identical here. The full boot-to-key-wait lands
//! with XML/KSCAN (M3/M2).

use std::sync::LazyLock;

use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn boot(rom: &[u8], grom: &[u8], frames: usize) -> Machine {
    let mut m = Machine::new(rom, grom);
    m.reset();
    for _ in 0..frames {
        m.run_frame();
    }
    m
}

#[test]
fn our_rom_paints_the_title_pixel_identically() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    let grom = libre99_gpl::system_grom::build_console_grom()
        .unwrap_or_else(|d| panic!("console GROM assembly failed: {d:?}"));
    let our_rom = libre99_asm::system_rom::build_console_rom()
        .unwrap_or_else(|d| panic!("console ROM assembly failed: {d:?}"));

    // 60 frames: past both boots' frames-to-title (authentic ~41, ours ~11) and
    // far short of the ~32768-frame screen-blank timeout.
    let authentic = boot(auth_rom, &grom, 60);
    let ours = boot(&our_rom, &grom, 60);

    // Sanity: the title actually drew under our ROM (not a both-blank match).
    assert!(
        ours.vdp().register(1) & 0x40 != 0,
        "our ROM must enable the display (R1=>{:02X})",
        ours.vdp().register(1)
    );
    let base = ((ours.vdp().register(2) & 0x0F) as u16) * 0x400;
    let banner: String =
        (0..17).map(|i| ours.vdp().vram(base + 8 * 32 + 7 + i) as char).collect();
    assert_eq!(banner, "TEXAS INSTRUMENTS", "our ROM must paint the banner");

    // The whole boot ran clean: no loud-stub breadcrumb — our ROM reached the
    // title's key-wait without hitting an unimplemented handler.
    assert_eq!(
        ours.bus().peek(0x837D),
        0,
        "loud-stub breadcrumb >837D set: an unimplemented handler was hit during boot"
    );

    // Pixel-identity: every VDP register…
    for r in 0..8 {
        assert_eq!(
            ours.vdp().register(r),
            authentic.vdp().register(r),
            "VDP register {r} differs (ours >{:02X} vs authentic >{:02X})",
            ours.vdp().register(r),
            authentic.vdp().register(r),
        );
    }
    // …and all 16 KiB of VRAM.
    for a in 0x0000u16..0x4000 {
        assert_eq!(
            ours.vdp().vram(a),
            authentic.vdp().vram(a),
            "VRAM >{a:04X} differs (ours >{:02X} vs authentic >{:02X})",
            ours.vdp().vram(a),
            authentic.vdp().vram(a),
        );
    }
}

/// The next screen too: press (and release) a key at the title and let the
/// master selection menu build — the chip logo, the banner, the scan over
/// every GROM/cart base, and the `1 FOR TI PYTHON` entry. The
/// settled menu must be pixel-identical under our ROM vs the authentic ROM
/// (the transient timing differs — ours skips the authentic 16-base SGROM
/// walk — but both run the same GPL to the same settled screen).
#[test]
fn our_rom_reaches_the_menu_pixel_identically() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    let grom = libre99_gpl::system_grom::build_console_grom()
        .unwrap_or_else(|d| panic!("console GROM assembly failed: {d:?}"));
    let our_rom = libre99_asm::system_rom::build_console_rom()
        .unwrap_or_else(|d| panic!("console ROM assembly failed: {d:?}"));

    let drive = |rom: &[u8]| {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..60 {
            m.run_frame(); // to the title key-wait
        }
        m.set_key(TiKey::Space, true);
        for _ in 0..8 {
            m.run_frame();
        }
        m.set_key(TiKey::Space, false);
        for _ in 0..150 {
            m.run_frame(); // key release + the menu build + its key-wait
        }
        m
    };
    let authentic = drive(auth_rom);
    let ours = drive(&our_rom);

    // Sanity: the menu genuinely built under our ROM (not a both-stuck match).
    let base = ((ours.vdp().register(2) & 0x0F) as u16) * 0x400;
    let screen: String = (0..24 * 32).map(|i| ours.vdp().vram(base + i) as char).collect();
    assert!(
        screen.contains("TI PYTHON"),
        "the selection list should offer TI PYTHON under our ROM"
    );
    assert_eq!(ours.bus().peek(0x837D), 0, "no loud stub during the menu build");

    for r in 0..8 {
        assert_eq!(
            ours.vdp().register(r),
            authentic.vdp().register(r),
            "VDP register {r} differs on the menu screen"
        );
    }
    for a in 0x0000u16..0x4000 {
        assert_eq!(
            ours.vdp().vram(a),
            authentic.vdp().vram(a),
            "menu VRAM >{a:04X} differs (ours >{:02X} vs authentic >{:02X})",
            ours.vdp().vram(a),
            authentic.vdp().vram(a),
        );
    }
}
