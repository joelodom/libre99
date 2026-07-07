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

//! Regression gate for the console-ROM VBLANK **interrupt service routine**.
//!
//! The rewritten GROM's boot must enable the 9901's VDP interrupt (CRU bit 2)
//! so the ROM's ISR fires every frame. Everything ISR-driven depends on it:
//! GPL **sound lists**, sprite auto-motion, cursor/timers, and **QUIT**. An
//! early rewrite omitted the enable and booted to a silent, QUIT-less console
//! (the ISR never ran); these tests guard against that regressing. See
//! `original-content/system-roms/DEBUGGING.md` case study 1 ("no sound at the
//! Tunnels of Doom splash").

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
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

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

/// After boot the 9901's VDP interrupt must be enabled and the ISR must be
/// running — proven by the ROM's free-running interrupt counter (`>8379`)
/// advancing across frames (it only ticks inside the ISR).
#[test]
fn isr_runs_after_boot() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.reset();
    for _ in 0..20 {
        m.run_frame();
    }
    assert!(
        m.bus().tms9901.vdp_interrupt_enabled(),
        "boot must enable the 9901 VDP interrupt (CRU bit 2)"
    );
    // Sample the ISR-stirred timer across several frames; it must change.
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..10 {
        m.run_frame();
        seen.insert(m.bus().peek(0x8379));
    }
    assert!(
        seen.len() > 1,
        "the VBLANK ISR is not running (>8379 never changed: {seen:?})"
    );
}

/// A reset (F5) while a cartridge is mid-tune must **silence the stale
/// channels**. `reset()` re-runs the boot but does not clear the SN76489, so a
/// game's sound keeps playing until something mutes it. The boot's `SND` list
/// opens by muting generators 1-3 (matching the authentic power-on beep at GROM
/// `>0484`); without it the old tune drones over our title after F5
/// (QUALITY-ASSESSMENT §5 item 5). The channel-0 beep itself must still sound.
#[test]
fn reset_mutes_stale_sound_channels() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.reset();
    for _ in 0..60 { m.run_frame(); }
    // A cartridge leaves channels 1, 2, 3 loud (tone/noise + attenuation 0).
    {
        let psg = &mut m.bus_mut().psg;
        psg.write(0xA0); psg.write(0x10); psg.write(0xB0); // ch1 tone, loud
        psg.write(0xC0); psg.write(0x18); psg.write(0xD0); // ch2 tone, loud
        psg.write(0xE4); psg.write(0xF0); // ch3 noise, loud
    }
    assert!(
        (1..4).all(|ch| m.bus().psg.volume(ch) < 0x0F),
        "channels 1-3 should be audible before the reset"
    );
    // The F5 the user presses.
    m.reset();
    // The channel-0 beep sounds during the boot; sample it before it ends.
    let mut beeped = false;
    for _ in 0..80 {
        m.run_frame();
        if m.bus().psg.volume(0) < 0x0F { beeped = true; }
    }
    assert!(beeped, "the power-on beep (channel 0) must still sound after reset");
    assert!(
        (1..4).all(|ch| m.bus().psg.volume(ch) >= 0x0F),
        "reset must mute stale channels 1-3 (they droned over the title after F5)"
    );
}

/// The selection menu beeps on a **rejected** (out-of-range) key, matching the
/// authentic console — verified differentially (QUALITY-ASSESSMENT §7.5 /
/// `examples/menu_beep_probe`: authentic beeps on a rejected key, ours must too).
/// Our menu arms the `KBEEP` click on every key, valid or not (L7).
#[test]
fn menu_beeps_on_rejected_key() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.reset();
    for _ in 0..180 { m.run_frame(); } // title settles
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..200 { m.run_frame(); } // selection screen; the leave-title click drains
    // Confirm silence going in, so the beep we detect is from the rejected key.
    let mut quiet = true;
    for _ in 0..20 { m.run_frame(); if m.bus().psg.volume(0) < 0x0F { quiet = false; } }
    assert!(quiet, "expected the menu to be silent before the rejected key");
    // Press an out-of-range digit (9 — far beyond the one entry with no cart).
    m.set_key(TiKey::Num9, true);
    for _ in 0..6 { m.run_frame(); }
    m.set_key(TiKey::Num9, false);
    let mut beeped = false;
    for _ in 0..40 { m.run_frame(); if m.bus().psg.volume(0) < 0x0F { beeped = true; } }
    assert!(beeped, "the menu must beep on a rejected key (L7 parity with authentic)");
}

/// Launching a GROM cartridge that plays a splash tune (Tunnels of Doom) must
/// actually produce sound: the ISR walks the cartridge's GPL sound list and
/// writes the SN76489, so a channel becomes audible (attenuation < 15).
#[test]
fn tunnels_of_doom_plays_sound() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(cart) = ["cartridges/tundoom.ctg", "cartridges/tunnelsofdoom.ctg"]
        .iter()
        .find_map(|p| libre99_core::third_party::load(p))
        .map(|d| Cartridge::parse(&d).unwrap())
    else {
        skip!()
    };

    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..260 { m.run_frame(); }
    // Program 2 is the first cartridge program (Tunnels of Doom).
    m.set_key(TiKey::Num2, true);
    for _ in 0..6 { m.run_frame(); }
    m.set_key(TiKey::Num2, false);

    let mut audible = false;
    for _ in 0..120 {
        m.run_frame();
        if (0..4).any(|ch| m.bus().psg.volume(ch) < 0x0F) {
            audible = true;
            break;
        }
    }
    assert!(audible, "Tunnels of Doom splash tune never became audible");
}
