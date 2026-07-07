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

//! End-to-end: in LIVE TI Invaders gameplay, does the ship move with the joystick
//! and the keyboard arrows, under our rebuilt GROM vs authentic? Launch the wave
//! (fire), isolate the ship (the sprite that separates hold-right from hold-left),
//! and report its net X move for each control.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
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

fn run(m: &mut Machine, n: usize) { for _ in 0..n { m.run_frame(); } }
fn press(m: &mut Machine, k: TiKey, settle: usize) {
    m.set_key(k, true); run(m, 4); m.set_key(k, false); run(m, settle);
}
fn xs(m: &Machine) -> [i32; 32] {
    let base = ((m.vdp().register(5) & 0x7F) as u16) * 0x80;
    let mut v = [0i32; 32];
    for (i, x) in v.iter_mut().enumerate() {
        *x = m.vdp().vram(base + i as u16 * 4 + 1) as i32;
    }
    v
}
fn delta(a: [i32; 32], b: [i32; 32], i: usize) -> i32 {
    let mut d = b[i] - a[i];
    if d > 128 { d -= 256; }
    if d < -128 { d += 256; }
    d
}
fn live_game(grom: &[u8], cart: &Cartridge) -> Machine {
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(cart);
    m.reset();
    run(&mut m, 90);
    press(&mut m, TiKey::Space, 120);
    press(&mut m, TiKey::Num2, 200);
    press(&mut m, TiKey::Num1, 120);
    m.set_key(TiKey::Joy1Fire, true);
    run(&mut m, 60);
    m.set_key(TiKey::Joy1Fire, false);
    run(&mut m, 40);
    m
}
fn hold(m: &mut Machine, keys: &[TiKey], ship: usize) -> i32 {
    let a = xs(m);
    for &k in keys { m.set_key(k, true); }
    run(m, 40);
    for &k in keys { m.set_key(k, false); }
    let d = delta(a, xs(m), ship);
    run(m, 20);
    d
}
fn report(label: &str, grom: &[u8], cart: &Cartridge) {
    let mut m = live_game(grom, cart);
    // ship = sprite that separates right vs left the most.
    let a = xs(&m);
    m.set_key(TiKey::Joy1Right, true); run(&mut m, 40); m.set_key(TiKey::Joy1Right, false);
    let r = xs(&m); run(&mut m, 20);
    let b = xs(&m);
    m.set_key(TiKey::Joy1Left, true); run(&mut m, 40); m.set_key(TiKey::Joy1Left, false);
    let l = xs(&m); run(&mut m, 20);
    let (mut ship, mut best) = (0usize, -1i32);
    for i in 0..32 {
        let sep = delta(a, r, i) - delta(b, l, i);
        if sep > best { best = sep; ship = i; }
    }
    println!("{label}: ship=sprite#{ship}");
    println!("   JOY right          dX={:+}", hold(&mut m, &[TiKey::Joy1Right], ship));
    println!("   JOY left           dX={:+}", hold(&mut m, &[TiKey::Joy1Left], ship));
    println!("   KEY right (FCTN+D) dX={:+}", hold(&mut m, &[TiKey::Fctn, TiKey::D], ship));
    println!("   KEY left  (FCTN+S) dX={:+}", hold(&mut m, &[TiKey::Fctn, TiKey::S], ship));
}
fn main() {
    let cart = Cartridge::parse(&require("cartridges/TI-Invaders.ctg")).unwrap();
    report("AUTHENTIC", &AUTHENTIC_GROM, &cart);
    println!();
    let ours = libre99_gpl::system_grom::build_console_grom().unwrap();
    report("OURS     ", &ours, &cart);
}
