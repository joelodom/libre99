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

//! Analysis: for sample cartridges, enumerate the programs their headers
//! declare (the "census"), then boot our menu and read the name table to see
//! what it actually lists. Reveals whether the 512-byte scan window truncates
//! multi-program lists.

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

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

/// Walk a GROM/ROM program list from `data` (indexed by GROM/CPU address via
/// `base`), returning the program names. `read` fetches a byte at an address.
fn programs(read: &dyn Fn(u16) -> u8, base: u16) -> Vec<String> {
    let mut names = Vec::new();
    if read(base) != 0xAA {
        return names;
    }
    let mut p = ((read(base + 6) as u16) << 8) | read(base + 7) as u16;
    let mut guard = 0;
    while p != 0 && guard < 16 {
        guard += 1;
        let next = ((read(p) as u16) << 8) | read(p + 1) as u16;
        let len = read(p + 4) as u16;
        let name: String = (0..len).map(|i| read(p + 5 + i) as char).collect();
        names.push(name);
        p = next;
    }
    names
}

fn census(cart: &Cartridge) -> Vec<String> {
    let mut all = Vec::new();
    // GROM bases.
    for base in [0x6000u16, 0x8000, 0xA000, 0xC000, 0xE000] {
        if let Some((_, page)) = cart.grom.iter().find(|(a, _)| *a == base) {
            let read = |addr: u16| -> u8 {
                let off = addr.wrapping_sub(base) as usize;
                page.get(off).copied().unwrap_or(0)
            };
            all.extend(programs(&read, base));
        }
    }
    // CPU ROM base >6000 (bank 0).
    if cart.rom_banks > 0 {
        let read = |addr: u16| -> u8 {
            let off = addr.wrapping_sub(0x6000) as usize;
            cart.rom.get(off).copied().unwrap_or(0)
        };
        all.extend(programs(&read, 0x6000));
    }
    all
}

fn listed(cart: &Cartridge) -> Vec<String> {
    let grom = our_grom();
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    m.mount_cartridge(cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..260 { m.run_frame(); }
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let mut out = Vec::new();
    for r in 3..22u16 {
        let s: String = (0..32).map(|i| m.vdp().vram(base + r * 32 + i) as char).collect();
        let t = s.trim_end();
        if t.contains(" FOR ") {
            out.push(t.to_string());
        }
    }
    out
}

fn main() {
    let samples = [
        "amazing", "centipe", "HuntTheWumpus", "Parsec", "TI-Invaders",
        "DigDug", "MoonPatrol", "VideoGames1", "et", "Soccer", "mine",
    ];
    for name in samples {
        let data = match libre99_core::third_party::load(&format!("cartridges/{name}.ctg")) {
            Some(d) => d,
            None => { println!("{name}: (missing)"); continue; }
        };
        let cart = Cartridge::parse(&data).unwrap();
        let cen = census(&cart);
        let lst = listed(&cart);
        println!("{name}: census={} programs {:?}", cen.len(), cen);
        println!("    listed ({}): {:?}", lst.len(), lst);
    }
}
