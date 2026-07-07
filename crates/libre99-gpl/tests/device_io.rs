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

//! Regression gate for **console device I/O** — a cartridge loads a file from
//! disk and runs it, exactly as the authentic console does
//! (`crates/libre99-core/tests/disk.rs` proves the disk stack + authentic GROM).
//! This is the end-to-end proof of LIMITATIONS L6's device-I/O path: the
//! interconnect table + our original DSRLNK routine (interconnect slot `>0010`)
//! plus the boot's peripheral DSR power-up scan (which reserves the disk DSR's
//! VRAM buffer — without it the load stalls at 0 sectors). See
//! `original-content/system-roms/DEBUGGING.md` "M7 — console device I/O".
//!
//! **The M3 device linkage.** Our ROM's SROM (`XML >19`) does the real
//! found+call: the boot power-up scan finds the disk card and calls its power-up
//! routine (reserving the VRAM buffer / lowering `>8370`), and a `DSK1` DSRLNK
//! finds and calls the disk DSR — the console-internal DSRLNK the M3 SROM
//! implements (RECON §24). The FMT-free `disk_power_up_reserves_vram` gate below
//! proves this end-to-end under **both** ROMs.
//!
//! **The M4 closure (2026-07-05).** The full *Tunnels of Doom* load exercised a
//! whole chain of M4 elements: FMT (the selection screens), MOVE C=1 /
//! indexed-GAS / GRAM-dest (the LOAD path), the uniform imm/mem source
//! discipline, the `>837D` character buffer — and finally the **DSR
//! skip-return exit** (authentic `>0B16`): a DSR that handled the request
//! returns to `BL *R9`+2, and SROM must turn the card off, POP the GPL
//! DSRLNK's CALL frame (resuming DSRLNK's *caller* directly), and clear the
//! condition bit. Without it the GPL fell into DSRLNK's error tail and ToD
//! hung retrying (the old "GRAM readback" hypothesis was wrong — the GRAM
//! writes were never the issue). With the skip-exit in, both ToD flows run
//! under **both** ROMs below.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static TI_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static DSR: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/Disk.Bin"));
static TUNNELS: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("disks/Tunnels.Dsk"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn our_rom() -> Vec<u8> {
    libre99_asm::system_rom::build_console_rom().expect("console ROM assembles")
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

/// Firmware rows for FMT-free device-linkage flows: the authentic ROM (oracle)
/// and our rewrite, each on our GROM. (The ToD end-to-end flows run TI_ROM-only
/// until M4's FMT — see the module note.) `None` when the authentic ROM is
/// absent (the test then skips).
fn firmware() -> Option<Vec<(&'static str, Vec<u8>)>> {
    Some(vec![("TI_ROM", TI_ROM.as_deref()?.to_vec()), ("OUR_ROM", our_rom())])
}

fn tunnels_cart() -> Option<Cartridge> {
    ["cartridges/tundoom.ctg", "cartridges/tunnelsofdoom.ctg"]
        .iter()
        .find_map(|p| libre99_core::third_party::load(p))
        .map(|d| Cartridge::parse(&d).unwrap())
}

fn tap(m: &mut Machine, k: TiKey, settle: usize) {
    m.set_key(k, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(k, false);
    for _ in 0..settle {
        m.run_frame();
    }
}

fn screen_text(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..24 * 32)
        .map(|i| {
            let c = m.vdp().vram(base + i);
            if (0x20..0x7F).contains(&c) {
                c as char
            } else {
                ' '
            }
        })
        .collect()
}

/// Drive Tunnels of Doom to load the QUEST scenario from `Tunnels.Dsk` under
/// `rom` (on our GROM). Returns the disk read-sector log and the final screen.
fn load_quest(rom: &[u8]) -> (Vec<usize>, String) {
    let dsr = DSR.as_deref().expect("presence checked by each test");
    let tunnels = TUNNELS.as_deref().expect("presence checked by each test");
    let grom = our_grom();
    let mut m = Machine::new(rom, &grom);
    m.load_disk_controller(dsr);
    m.mount_disk(0, tunnels.to_vec());
    m.mount_cartridge(&tunnels_cart().expect("presence checked by each test"));
    m.reset();
    m.bus_mut().disk.record(true);

    for _ in 0..180 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 40); // title -> selection list
    tap(&mut m, TiKey::Num2, 240); // select Tunnels of Doom
    tap(&mut m, TiKey::Enter, 120); // -> "LOAD DATA FROM"
    tap(&mut m, TiKey::Num2, 120); // 2 = DISK 1 -> filename prompt
    for k in [TiKey::Q, TiKey::U, TiKey::E, TiKey::S, TiKey::T] {
        tap(&mut m, k, 10);
    }
    tap(&mut m, TiKey::Enter, 600); // submit -> load the scenario

    (m.bus().disk.read_log().to_vec(), screen_text(&m))
}

/// **Gate: Tunnels of Doom loads a QUEST scenario from disk — under both the
/// authentic ROM and our rewrite.** Mount the cartridge, the disk controller,
/// and the disk; drive the "LOAD DATA FROM: DISK 1" prompt, type the QUEST
/// filename, submit. The DSRLNK hands off to the disk DSR via the ROM's device
/// linkage (`XML >19`, the real `Disk.Bin` DSR reads the file), and — because
/// the boot's peripheral power-up reserves the DSR's VRAM buffer — the read
/// completes: the DSR reads the QUEST data sectors (AUs 85..=135) and the
/// cartridge reaches its post-load game-selection menu ("NEW DUNGEON").
#[test]
fn tunnels_of_doom_loads_quest_scenario_from_disk() {
    let Some(firmware) = firmware() else { skip!() };
    if DSR.is_none() || TUNNELS.is_none() || tunnels_cart().is_none() {
        skip!()
    }
    for (name, rom) in firmware {
        let (read, screen) = load_quest(&rom);
        assert!(
            read.contains(&85) && read.contains(&135),
            "{name}: the DSR did not read the QUEST file data sectors (85..=135); read {read:?}"
        );
        assert!(
            screen.contains("NEW DUNGEON"),
            "{name}: Tunnels of Doom did not reach its post-load menu; screen was:\n{screen}"
        );
    }
}

/// Drive Tunnels of Doom's "LOAD DATA FROM → CASSETTE" (CS1, a device we do not
/// serve) under `rom`, then measure whether the console stays alive (its VBLANK
/// ISR keeps ticking `>8379`) and what the screen shows.
fn bad_device(rom: &[u8]) -> (u32, String) {
    let dsr = DSR.as_deref().expect("presence checked by each test");
    let tunnels = TUNNELS.as_deref().expect("presence checked by each test");
    let grom = our_grom();
    let mut m = Machine::new(rom, &grom);
    m.load_disk_controller(dsr);
    m.mount_disk(0, tunnels.to_vec());
    m.mount_cartridge(&tunnels_cart().expect("presence checked by each test"));
    m.reset();

    for _ in 0..180 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 40); // title -> selection list
    tap(&mut m, TiKey::Num2, 240); // select Tunnels of Doom
    tap(&mut m, TiKey::Enter, 120); // -> "LOAD DATA FROM"
    tap(&mut m, TiKey::Num1, 120); // 1 = CASSETTE (CS1) — a device we don't serve

    // The console must stay alive: the ROM's VBLANK ISR counter (>8379) keeps
    // advancing. A hang (barreling into a dead DSR) would freeze it at one value.
    let mut ticks = 0;
    let mut prev = m.bus().peek(0x8379);
    for _ in 0..150 {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev {
            ticks += 1;
        }
        prev = t;
    }
    (ticks, screen_text(&m))
}

/// **Gate: a device our GROM does not serve fails *gracefully*, never hangs** —
/// the 1981 bar (QUALITY-ASSESSMENT §5 item 6), under both ROMs. Tunnels of
/// Doom's "LOAD DATA FROM → CASSETTE" asks for `CS1`, whose DSR lives in the
/// console ROM (not on a card) and which our GROM does not ship. §5 item 6
/// feared DSRLNK would skip the `XML >19` search result and "barrel into
/// `XML >1A`", wedging the console — but execution refutes it
/// (`examples/dsrlnk_baddev_probe`): the ROM's `XML >19/>1A` return the DSR
/// error, the cartridge shows `DEVICE ERROR`, and the console stays alive (the
/// ISR keeps ticking). This guards that behaviour on our rewrite too.
#[test]
fn bad_device_errors_gracefully_without_hanging() {
    let Some(firmware) = firmware() else { skip!() };
    if DSR.is_none() || TUNNELS.is_none() || tunnels_cart().is_none() {
        skip!()
    }
    for (name, rom) in firmware {
        let (ticks, screen) = bad_device(&rom);
        assert!(
            ticks > 10,
            "{name}: console hung after a bad-device (CS1) request — the ISR stalled ({ticks} ticks)"
        );
        assert!(
            screen.contains("ERROR") || screen.contains("LOAD DATA FROM"),
            "{name}: expected a graceful device error or recovery to the load menu; screen:\n{screen}"
        );
    }
}

/// **Gate (M3, FMT-free): SROM's found+call peripheral power-up reserves the
/// disk DSR's VRAM buffer — under both the authentic ROM and our rewrite.** With
/// the disk controller present, the boot's power-up scan (`XML >19`) walks the
/// CRU bases, finds the disk card's `>AA` header, follows its power-up chain, and
/// **calls** the disk power-up routine, which lowers `>8370` (top of free VRAM)
/// from `>3FFF` to `>37D7` (docs/STATUS.md). This is the whole M3 device-linkage
/// found+call path — the CRU scan, the header walk, the name/key match, and the
/// `BL *R9` call with the DSR-call invariants — exercised end-to-end with real
/// card ROM, and it needs no FMT. Our ROM must reserve the buffer identically.
#[test]
fn disk_power_up_reserves_vram() {
    let Some(firmware) = firmware() else { skip!() };
    let Some(dsr) = DSR.as_deref() else { skip!() };
    for (name, rom) in firmware {
        let grom = our_grom();
        let mut m = Machine::new(&rom, &grom);
        m.load_disk_controller(dsr);
        m.reset();
        for _ in 0..120 {
            m.run_frame();
        }
        assert_eq!(
            m.bus().peek_word(0x8370),
            0x37D7,
            "{name}: the disk power-up did not reserve the DSR VRAM buffer (>8370 != >37D7)"
        );
        assert_eq!(
            m.bus().peek(0x837D),
            0,
            "{name}: a loud stub fired during the power-up boot (>837D breadcrumb set)"
        );
    }
}
