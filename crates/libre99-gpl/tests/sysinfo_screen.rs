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

//! The **system information screen** ((S) on the selection menu) and the
//! Libre99 emulator-identification block behind it (`libre99_core::sysinfo`).
//!
//! Three layers are gated here:
//!
//! 1. **The block's bytes** in the built image — magic `L99I` at `>5700`,
//!    format `>01`, flag `>00` (shipped unstamped), and the baked version
//!    strings (`VERSTR`/`PYVERS`/`PYBANR`) carrying this workspace's
//!    `CARGO_PKG_VERSION`. This is the version-synchronization gate: the
//!    emulator, this GROM, and TI PYTHON must all report the same number.
//! 2. **The unstamped screen** — booted anywhere the emulator did not stamp
//!    (Classic99, a GROM board), the host rows render `UNKNOWN` while the
//!    baked rows still show real versions.
//! 3. **The stamped screen** — after `libre99_core::sysinfo::stamp` (what
//!    `libre99-app` does at launch), the host rows show the stamped facts.
//!
//! The menu and screen run under the authentic console ROM (the tests' usual
//! oracle harness) and, as a smoke pass, under our rewritten ROM too.

use std::sync::LazyLock;

use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;
use libre99_core::sysinfo;

static TI_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom()
        .unwrap_or_else(|d| panic!("console GROM assembly failed: {d:?}"))
}

fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}

/// Read a name-table row as a string (identity-mapped ASCII).
fn row(m: &Machine, r: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..32).map(|i| m.vdp().vram(base + r * 32 + i) as char).collect()
}

/// The whole visible name table as one string (for substring checks).
fn screen(m: &Machine) -> String {
    (0..24).map(|r| row(m, r)).collect::<Vec<_>>().join("\n")
}

/// Boot `grom` on `rom` (bare console), leave the title, and build the menu.
fn boot_to_menu(rom: &[u8], grom: &[u8]) -> Machine {
    let mut m = Machine::new(rom, grom);
    m.reset();
    frames(&mut m, 40); // title
    m.set_key(TiKey::Space, true);
    frames(&mut m, 3);
    m.set_key(TiKey::Space, false);
    frames(&mut m, 220); // the menu's base scan is slow by design
    m
}

/// Press (S) on the menu and give the screen time to draw.
fn press_s(m: &mut Machine) {
    m.set_key(TiKey::S, true);
    frames(m, 20);
    m.set_key(TiKey::S, false);
    frames(m, 100);
}

// ---------------------------------------------------------------------------
// 1. The block's bytes in the built image.
// ---------------------------------------------------------------------------

#[test]
fn the_built_image_carries_an_unstamped_block_and_synchronized_versions() {
    let grom = our_grom();

    assert!(sysinfo::has_block(&grom), "L99I magic missing at >5700");
    assert_eq!(grom[sysinfo::FORMAT_ADDR], 0x01, "block format");
    assert_eq!(grom[sysinfo::FLAGS_ADDR], 0x00, "the image must ship unstamped");
    for (off, len) in [
        sysinfo::EMU_VERSION,
        sysinfo::BUILD_DATE,
        sysinfo::COMMIT,
        sysinfo::HOST,
        sysinfo::ROM_ID,
    ] {
        assert!(
            grom[off..off + len].iter().all(|&b| b == b' '),
            "stamped field at >{off:04X} must ship blank"
        );
    }

    // The baked strings right after the block: VERSTR (20) + PYVERS (8) +
    // PYBANR (20), each carrying the one workspace version.
    let text = |start: usize, len: usize| {
        String::from_utf8_lossy(&grom[start..start + len]).trim_end().to_string()
    };
    assert_eq!(text(sysinfo::BLOCK_END, 20), format!("LIBRE99 {VERSION}"));
    assert_eq!(text(sysinfo::BLOCK_END + 20, 8), VERSION);
    assert_eq!(text(sysinfo::BLOCK_END + 28, 20), format!("TI PYTHON {VERSION}"));
}

// ---------------------------------------------------------------------------
// 2. The unstamped screen (any emulator that does not know the block).
// ---------------------------------------------------------------------------

#[test]
fn menu_offers_the_screen_and_unstamped_rows_render_unknown() {
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    let mut m = boot_to_menu(ti_rom, &our_grom());

    // The offer line fills menu row 23 edge to edge (exactly 32 characters).
    assert_eq!(row(&m, 23), "PRESS (S) FOR SYSTEM INFORMATION", "menu:\n{}", screen(&m));

    press_s(&mut m);
    let scr = screen(&m);
    assert!(scr.contains("SYSTEM INFORMATION"), "title:\n{scr}");
    // Host rows: UNKNOWN, because nothing stamped the block.
    assert!(scr.contains("VERSION  UNKNOWN"), "emulator version row:\n{scr}");
    assert!(scr.contains("BUILD    UNKNOWN"), "build row:\n{scr}");
    assert!(scr.contains("COMMIT   UNKNOWN"), "commit row:\n{scr}");
    assert!(scr.contains("HOST     UNKNOWN"), "host row:\n{scr}");
    assert!(scr.contains("ROM      UNKNOWN"), "ROM row:\n{scr}");
    // Baked rows: real versions, valid anywhere this image runs.
    assert!(scr.contains(&format!("GROM     LIBRE99 {VERSION}")), "GROM row:\n{scr}");
    assert!(scr.contains(&format!("PYTHON   {VERSION}")), "PYTHON row:\n{scr}");
    // The static hardware description (two-column chip list) and the exit hint.
    assert!(scr.contains("CPU TMS9900    VDP TMS9918A"), "hardware rows:\n{scr}");
    assert!(scr.contains("PRESS ANY KEY TO RETURN"), "return hint:\n{scr}");

    // Any key returns to the menu (which redraws and rescans).
    m.set_key(TiKey::Space, true);
    frames(&mut m, 20);
    m.set_key(TiKey::Space, false);
    frames(&mut m, 240);
    let scr = screen(&m);
    assert!(scr.contains("1 FOR TI PYTHON"), "back on the menu:\n{scr}");
    assert_eq!(row(&m, 23), "PRESS (S) FOR SYSTEM INFORMATION", "offer line redrawn");
}

// ---------------------------------------------------------------------------
// 3. The stamped screen (what libre99-app produces at launch).
// ---------------------------------------------------------------------------

#[test]
fn stamped_rows_render_the_hosts_facts() {
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    let mut grom = our_grom();
    let stamped = sysinfo::stamp(
        &mut grom,
        &sysinfo::HostStamp {
            emu_version: "9.9.9",
            build_date: "2026-12-31",
            commit: "abc1234+",
            host: "TESTOS X64",
            rom_id: "TI 1981",
        },
    );
    assert!(stamped, "the built image must accept a stamp");

    let mut m = boot_to_menu(ti_rom, &grom);
    press_s(&mut m);
    let scr = screen(&m);
    assert!(scr.contains("VERSION  9.9.9"), "emulator version row:\n{scr}");
    assert!(scr.contains("BUILD    2026-12-31"), "build row:\n{scr}");
    assert!(scr.contains("COMMIT   ABC1234+"), "commit row (uppercased):\n{scr}");
    assert!(scr.contains("HOST     TESTOS X64"), "host row:\n{scr}");
    assert!(scr.contains("ROM      TI 1981"), "ROM row:\n{scr}");
    // Baked rows are unaffected by stamping.
    assert!(scr.contains(&format!("GROM     LIBRE99 {VERSION}")), "GROM row:\n{scr}");
}

// ---------------------------------------------------------------------------
// The full Libre99 stack: our ROM interpreting the screen too.
// ---------------------------------------------------------------------------

#[test]
fn the_screen_also_runs_under_our_rewritten_rom() {
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM assembles");
    let mut m = boot_to_menu(&our_rom, &our_grom());
    assert_eq!(row(&m, 23), "PRESS (S) FOR SYSTEM INFORMATION", "menu:\n{}", screen(&m));

    press_s(&mut m);
    let scr = screen(&m);
    assert!(scr.contains("SYSTEM INFORMATION"), "title:\n{scr}");
    assert!(scr.contains(&format!("GROM     LIBRE99 {VERSION}")), "GROM row:\n{scr}");
    assert!(scr.contains("VERSION  UNKNOWN"), "unstamped host row:\n{scr}");
}
