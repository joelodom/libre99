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

//! Repro probe for the reported F5 bug: "after F5 I get no beep and can't press
//! 2 to play TI Invaders." Runs the exact user flow — boot → menu → launch cart
//! → **F5 (second reset)** → menu → launch again — and reports the ISR / sound
//! health at each stage, for BOTH our new console GROM and the pre-commit one
//! (`/tmp/old-grom.bin`), so we can tell whether Chunk 1 changed this behaviour.

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

#[derive(Debug, Default)]
struct Health {
    isr_enabled: bool,
    isr_timer_moving: bool,
    boot_beep: bool,   // did the power-on SND beep sound after this boot/F5?
    audible: bool,     // did sound play after pressing 2?
    menu_has_entry2: bool,
    drone_1_3: usize,  // frames channels 1-3 stayed audible after F5 (droning)
}

/// Run `n` frames; return true if any PSG channel is audible during them (the
/// power-on beep is brief, so we must sample while it plays).
fn frames_audible(m: &mut Machine, n: usize) -> bool {
    let mut any = false;
    for _ in 0..n {
        m.run_frame();
        if (0..4).any(|ch| m.bus().psg.volume(ch) < 0x0F) { any = true; }
    }
    any
}

/// Run `n` frames; return the per-channel count of frames each PSG channel was
/// audible. A high count on channels 1-3 while sitting on the title = the game's
/// sound droning past an F5 (QUALITY-ASSESSMENT §5 item 5).
fn channel_audible_frames(m: &mut Machine, n: usize) -> [usize; 4] {
    let mut c = [0usize; 4];
    for _ in 0..n {
        m.run_frame();
        for ch in 0..4 {
            if m.bus().psg.volume(ch) < 0x0F { c[ch] += 1; }
        }
    }
    c
}

#[derive(Clone, Copy)]
enum F5From {
    ColdOnly,     // no F5 — plain first boot
    Title,        // F5 while on the title
    Menu,         // F5 after reaching the menu
    Playing,      // F5 after launching + playing TI Invaders
}

/// Reach the menu from the title (space, then wait for the scan).
fn to_menu(m: &mut Machine) {
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..260 { m.run_frame(); }
}

/// Press 2 and run; returns whether any PSG channel became audible.
fn press2_and_listen(m: &mut Machine) -> bool {
    m.set_key(TiKey::Num2, true);
    for _ in 0..6 { m.run_frame(); }
    m.set_key(TiKey::Num2, false);
    for _ in 0..180 {
        m.run_frame();
        if (0..4).any(|ch| m.bus().psg.volume(ch) < 0x0F) { return true; }
    }
    false
}

/// Boot, optionally F5 from a given depth, reach the menu, then press 2.
fn run(grom: &[u8], cart: &Cartridge, from: F5From) -> Health {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    // Match the real app: load the TI Disk Controller DSR (CPU >4000-5FFF),
    // whose window sits at the same *address* as our relocated FONT2 (GROM
    // >4000) — different address space, but verify the app config, not a bare one.
    if let Some(dsr) = libre99_core::third_party::load("roms/Disk.Bin") {
        m.load_disk_controller(&dsr);
    }
    m.reset();
    let mut h = Health::default();
    // Cold-boot beep sounds during the first settle (unless we F5 later, in which
    // case the beep we care about is the post-F5 one, captured below).
    h.boot_beep = frames_audible(&mut m, 40);

    match from {
        F5From::ColdOnly | F5From::Title => {}
        F5From::Menu => { to_menu(&mut m); }
        F5From::Playing => {
            to_menu(&mut m);
            // Launch TI Invaders and "play" a bit with some input.
            m.set_key(TiKey::Num2, true);
            for _ in 0..6 { m.run_frame(); }
            m.set_key(TiKey::Num2, false);
            for _ in 0..200 { m.run_frame(); }
            m.set_key(TiKey::Num1, true); // fire / start in Invaders
            for _ in 0..60 { m.run_frame(); }
            m.set_key(TiKey::Num1, false);
            for _ in 0..120 { m.run_frame(); }
        }
    }

    if !matches!(from, F5From::ColdOnly) {
        // The F5 the user pressed — capture the post-F5 power-on beep, and watch
        // for the previous game's sound droning on channels 1-3 past the reset.
        m.reset();
        let ch = channel_audible_frames(&mut m, 90);
        h.boot_beep = ch[0] > 0;
        h.drone_1_3 = ch[1] + ch[2] + ch[3];
        print!("      [post-F5 audible frames/90: ch0={} ch1={} ch2={} ch3={}]  ", ch[0], ch[1], ch[2], ch[3]);
    }

    // From the (possibly repainted) title, go to the menu.
    to_menu(&mut m);

    h.isr_enabled = m.bus().tms9901.vdp_interrupt_enabled();
    // Is the ISR timer >8379 advancing?
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..10 { m.run_frame(); seen.insert(m.bus().peek(0x8379)); }
    h.isr_timer_moving = seen.len() > 1;
    // Does the menu list a second entry? Scan the name table for two " FOR ".
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let mut for_rows = 0;
    for r in 3..22u16 {
        let s: String = (0..32).map(|i| m.vdp().vram(base + r * 32 + i) as char).collect();
        if s.contains(" FOR ") { for_rows += 1; }
    }
    h.menu_has_entry2 = for_rows >= 2;
    h.audible = press2_and_listen(&mut m);
    h
}

fn main() {
    let cart = Cartridge::parse(&require("cartridges/TI-Invaders.ctg")).unwrap();
    let grom = libre99_gpl::system_grom::build_console_grom().unwrap();

    println!("F5 (reset) health — our console GROM, TI Invaders mounted, disk controller loaded:");
    for (stage, from) in [
        ("COLD first boot", F5From::ColdOnly),
        ("F5 from title", F5From::Title),
        ("F5 from menu", F5From::Menu),
        ("F5 while playing Invaders", F5From::Playing),
    ] {
        let h = run(&grom, &cart, from);
        let ok = h.isr_enabled && h.isr_timer_moving && h.menu_has_entry2 && h.audible && h.boot_beep;
        println!(
            "  {stage:28}  boot_beep={:5}  isr_moving={:5}  menu_entry2={:5}  press2_sound={:5}  => {}",
            h.boot_beep, h.isr_timer_moving, h.menu_has_entry2, h.audible,
            if ok { "OK" } else { "BROKEN" }
        );
    }
}
