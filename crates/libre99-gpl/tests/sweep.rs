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

//! **Milestone 3 gate** — the cartridge compatibility sweep. For a spread of
//! bundled cartridges (GROM-only, ROM-only, GROM+ROM, banked, and multi-program
//! menus) assert that our selection list enumerates exactly the programs the
//! cartridge's headers declare, and that the first cartridge program launches by
//! its kind (GPL programs run cartridge GROM; ML programs run the >6000 ROM
//! window). An `#[ignore]`d test sweeps every `.ctg` in the bundle.

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

fn our_rom() -> Vec<u8> {
    libre99_asm::system_rom::build_console_rom().expect("console ROM assembles")
}

/// Walk a program list, returning `(names, first_entry_addr)`.
fn walk(read: &dyn Fn(u16) -> u8, base: u16) -> (Vec<String>, Option<u16>) {
    let mut names = Vec::new();
    let mut first = None;
    if read(base) != 0xAA {
        return (names, first);
    }
    let mut p = ((read(base + 6) as u16) << 8) | read(base + 7) as u16;
    let mut guard = 0;
    while p != 0 && guard < 16 {
        guard += 1;
        let next = ((read(p) as u16) << 8) | read(p + 1) as u16;
        let entry = ((read(p + 2) as u16) << 8) | read(p + 3) as u16;
        let len = read(p + 4) as u16;
        let name: String = (0..len).map(|i| read(p + 5 + i) as char).collect();
        if first.is_none() {
            first = Some(entry);
        }
        names.push(name);
        p = next;
    }
    (names, first)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Kind {
    Gpl,
    Ml,
}

/// The programs a cartridge declares (in the menu's scan order) and the kind +
/// entry address of the first one.
fn census(cart: &Cartridge) -> (Vec<String>, Option<(Kind, u16)>) {
    let mut names = Vec::new();
    let mut first: Option<(Kind, u16)> = None;
    for base in [0x6000u16, 0x8000, 0xA000, 0xC000, 0xE000] {
        if let Some((_, page)) = cart.grom.iter().find(|(a, _)| *a == base) {
            let read = |addr: u16| page.get(addr.wrapping_sub(base) as usize).copied().unwrap_or(0);
            let (ns, f) = walk(&read, base);
            if first.is_none() {
                if let Some(e) = f {
                    first = Some((Kind::Gpl, e));
                }
            }
            names.extend(ns);
        }
    }
    if cart.rom_banks > 0 {
        let read = |addr: u16| cart.rom.get(addr.wrapping_sub(0x6000) as usize).copied().unwrap_or(0);
        let (ns, f) = walk(&read, 0x6000);
        if first.is_none() {
            if let Some(e) = f {
                first = Some((Kind::Ml, e));
            }
        }
        names.extend(ns);
    }
    (names, first)
}

fn boot_to_menu(cart: &Cartridge) -> Machine {
    boot_to_menu_on(cart, CONSOLE_ROM.as_deref().expect("presence checked by each test"))
}

fn boot_to_menu_on(cart: &Cartridge, rom: &[u8]) -> Machine {
    let grom = our_grom();
    let mut m = Machine::new(rom, &grom);
    m.mount_cartridge(cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    m.set_key(TiKey::Space, true);
    for _ in 0..3 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..260 { m.run_frame(); }
    m
}

/// Count menu entries drawn as "n FOR NAME".
fn listed_count(m: &Machine) -> usize {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (3..22u16)
        .filter(|r| {
            let s: String = (0..32).map(|i| m.vdp().vram(base + r * 32 + i) as char).collect();
            s.contains(" FOR ")
        })
        .count()
}

/// Launch entry 2 (the first cartridge program) and confirm it started per kind.
fn launches(m: &mut Machine, kind: Kind, entry: u16) -> bool {
    m.bus_mut().grom_record(true);
    m.set_key(TiKey::Num2, true);
    for _ in 0..20 { m.run_frame(); }
    m.set_key(TiKey::Num2, false);
    match kind {
        Kind::Gpl => {
            for _ in 0..60 { m.run_frame(); }
            let log = m.bus().grom_log();
            let n = log.iter().filter(|(a, _)| *a >= 0x6000).count();
            let first = log.iter().find(|(a, _)| *a >= 0x6000).map(|(a, _)| *a);
            n > 200 && first == Some(entry)
        }
        Kind::Ml => {
            for _ in 0..120 {
                m.run_frame();
                if (0x6000..0x8000).contains(&m.cpu().pc()) {
                    return true;
                }
            }
            false
        }
    }
}

/// The M3 firmware matrix over the cart sweep: every class sample must list and
/// launch under **both** the authentic ROM (the oracle) and our rewrite. The
/// GPL-cart launch runs the cartridge GROM; the ML-cart launch is the console's
/// `XML >F0` trampoline through the `>8300` vector into the `>6000` ROM window —
/// so these ML samples (centipe, MoonPatrol) are the M3 `XML >F0` launch gate.
fn check(name: &str) {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    check_on(name, console_rom, "TI_ROM", true);
    check_on(name, &our_rom(), "OUR_ROM", true);
}

fn check_on(name: &str, rom: &[u8], label: &str, launch: bool) {
    let path = format!("cartridges/{name}.ctg");
    let Some(data) = libre99_core::third_party::load(&path) else { skip!() };
    let cart = Cartridge::parse(&data).unwrap();
    let (programs, first) = census(&cart);
    let (kind, entry) = first.unwrap_or_else(|| panic!("{name}: no launchable program"));

    let mut m = boot_to_menu_on(&cart, rom);
    // TI PYTHON (entry 1) plus every cartridge program.
    assert_eq!(
        listed_count(&m),
        programs.len() + 1,
        "{label} {name}: listed count != census+1 (census {:?})",
        programs
    );
    if launch {
        assert!(
            launches(&mut m, kind, entry),
            "{label} {name}: entry 2 ({kind:?} at >{entry:04X}) did not launch",
        );
    }
}

#[test]
fn sweep_grom_only_amazing() { check("amazing"); }
#[test]
fn sweep_rom_only_centipe() { check("centipe"); }
#[test]
fn sweep_grom_rom_parsec() { check("Parsec"); }
#[test]
fn sweep_grom_rom_invaders() { check("TI-Invaders"); }
#[test]
fn sweep_rom_banked_moonpatrol() { check("MoonPatrol"); }
#[test]
fn sweep_multi_huntthewumpus() { check("HuntTheWumpus"); }
#[test]
fn sweep_multi_videogames1() { check("VideoGames1"); }
#[test]
fn sweep_multi_banked_et() { check("et"); }
// Far-list carts (LIMITATIONS L2): their program list is stored past the old
// 512-byte scan window (`starpeg` at slot >7801, `xb25` at >6A01). `SCANW`'s
// widen-on-far path (SFAR) now re-copies the full 8 KiB slot for these, so both
// list and launch their program. These were the two `FAR_LIST_CARTS` exceptions.
#[test]
fn sweep_farlist_starpeg() { check("starpeg"); }
#[test]
fn sweep_farlist_xb25() { check("xb25"); }

/// QUIT (`FCTN`+`=`) soft-resets the console back to our title. This is detected
/// in the console ROM's VBLANK ISR, which only runs once our boot enables the
/// 9901's VDP interrupt (CRU bit 2) — the same fix that restored sound. See
/// `DEBUGGING.md` "no sound at Tunnels of Doom" and `console.gpl` START.
#[test]
fn quit_returns_to_our_title() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let Some(data) = libre99_core::third_party::load("cartridges/centipe.ctg") else { skip!() };
    let cart = Cartridge::parse(&data).unwrap();
    let mut m = boot_to_menu(&cart);
    // At the menu, press QUIT.
    m.set_key(TiKey::Fctn, true);
    m.set_key(TiKey::Equals, true);
    for _ in 0..10 { m.run_frame(); }
    m.set_key(TiKey::Fctn, false);
    m.set_key(TiKey::Equals, false);
    let mut back = false;
    for _ in 0..90 {
        m.run_frame();
        let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
        let s: String = (0..24 * 32).map(|i| m.vdp().vram(base + i) as char).collect();
        if s.contains("JOEL ODOM") {
            back = true;
            break;
        }
    }
    assert!(back, "QUIT should reboot to our title");
}

/// Full bundle sweep — enumerates every `.ctg`, compares listed vs. census.
/// Ignored by default (137 cartridges × a slow menu build is ~10 s). Asserts
/// **every** cartridge lists exactly its declared programs.
///
/// This used to carry two `FAR_LIST_CARTS` exceptions — `starpeg` (program list
/// at GROM slot >7801) and `xb25` (>6A01) — whose lists sit past the old 512-byte
/// scan window. `SCANW`'s far-list path (see `console.gpl` `SFAR`) now re-copies
/// the full 8 KiB slot for a base whose list pointer lands beyond the window, so
/// both list and launch; the exceptions are gone and the sweep expects 137/137
/// (LIMITATIONS L2 resolved). Fast per-cart gates: `sweep_farlist_starpeg`,
/// `sweep_farlist_xb25`.
#[test]
#[ignore]
fn sweep_all_cartridges() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let Some(third_party) = libre99_core::third_party::dir() else { skip!() };
    let mut pass = 0;
    let mut unexpected = Vec::new();
    let mut entries = std::fs::read_dir(third_party.join("cartridges")).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "ctg").unwrap_or(false))
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let data = std::fs::read(&path).unwrap();
        let cart = match Cartridge::parse(&data) {
            Ok(c) => c,
            Err(_) => { println!("SKIP {name}: parse error"); continue; }
        };
        let (programs, first) = census(&cart);
        let m = boot_to_menu(&cart);
        let got = listed_count(&m);
        let want = programs.len() + 1;
        let ok = got == want && (programs.is_empty() || first.is_some());
        if ok {
            pass += 1;
        } else {
            unexpected.push(format!("{name}: listed {got} want {want} (census {:?})", programs));
        }
    }
    println!("sweep: {pass} pass");
    assert!(unexpected.is_empty(), "unexpected sweep failures:\n{}", unexpected.join("\n"));
    assert!(pass >= 137, "expected all 137 cartridges to list correctly, got {pass}");
}
