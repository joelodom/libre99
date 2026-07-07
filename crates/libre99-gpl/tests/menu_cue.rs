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

//! Gate for **L5**: how the selection menu appears. Two guarantees:
//!
//! 1. **No "SCANNING" cue.** The build was once thought "visibly slow," so `MENU`
//!    painted a "SCANNING" row (row 6) while it walked the bases. Measurement
//!    (`perf_parity.rs`) showed the isolated build is only ~7 frames and we reach
//!    the menu *sooner* than the authentic firmware overall — the banner was the
//!    only thing that read as slow, and the authentic menu shows no such word. It
//!    was removed so the list simply appears. Guarded: row 6 stays blank.
//! 2. **Atomic reveal.** So the per-byte base scan does not paint the program
//!    lines in one at a time, `MENU` blanks the display (VDP R1 `>A0`, the title
//!    screen's own idiom) and reveals it whole (`DISPON`, R1 `>E0`) only once the
//!    scan is complete. Guarded: the display is off during the build and turns on
//!    with the **full** list already present — never a partial paint.
//!
//! Both keep the settled menu identical to before (`LIMITATIONS.md` L5).

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

/// Read 16 characters of row 6 (VDP name-table offset >00C4, col 4).
fn row6(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..16u16)
        .map(|c| {
            let b = m.vdp().vram(base + 0x00C4 + c);
            if (0x20..0x7F).contains(&b) { b as char } else { ' ' }
        })
        .collect()
}

/// Full screen text, for the settled-menu assertions.
fn screen(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..24 * 32)
        .map(|i| {
            let b = m.vdp().vram(base + i);
            if (0x20..0x7F).contains(&b) { b as char } else { ' ' }
        })
        .collect()
}

/// Is the VDP display enabled? (R1 bit 6, `>40`, the blanking bit.)
fn display_on(m: &Machine) -> bool {
    m.vdp().register(1) & 0x40 != 0
}

/// How many `n FOR NAME` program lines the name table holds right now. VRAM is
/// written during the (blanked) build, so this counts entries whether or not the
/// display is currently on.
fn listed_count(m: &Machine) -> usize {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..24u16)
        .filter(|r| {
            let row: String = (0..32)
                .map(|c| m.vdp().vram(base + r * 32 + c) as char)
                .collect();
            row.contains(" FOR ")
        })
        .count()
}

/// Boot a cart, leave the title, and sample row 6 across the whole build — the
/// "SCANNING" cue must never appear (it was removed), yet the list still renders.
#[test]
fn menu_builds_with_no_scanning_cue() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(cart) = ["cartridges/HuntTheWumpus.ctg", "cartridges/amazing.ctg"]
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

    // Sample row 6 every frame across the entire build+settle window. The banner
    // used to flash here for ~7 frames; it must now never show — the row 6 slot
    // (between PRESS and the first entry) stays blank the whole way.
    for _ in 0..320 {
        m.run_frame();
        let r6 = row6(&m);
        assert!(
            !r6.contains("SCANNING"),
            "the removed SCANNING cue must never appear; row 6 was `{r6}`"
        );
    }
    let settled = row6(&m);
    assert!(
        settled.trim().is_empty(),
        "row 6 must stay blank once the menu is ready; row 6 was `{settled}`"
    );
    let screen = screen(&m);
    assert!(
        screen.contains(" FOR "),
        "the settled menu should still list programs; screen:\n{screen}"
    );
}

/// The menu is revealed **atomically**: `MENU` blanks the display while the base
/// scan runs and raises it (`SDONE`/`DISPON`) only once every entry is drawn, so
/// the user sees the whole list appear at once rather than lines paint in one at
/// a time. Assert the display goes off during the build and turns back on with
/// the *complete* list already present — never a partial paint.
#[test]
fn menu_reveals_atomically_with_full_list() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(cart) = ["cartridges/HuntTheWumpus.ctg", "cartridges/amazing.ctg"]
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

    // Walk the build+settle window. Record: whether we ever saw the display
    // blanked (the build), the frame it first turned back on after a blank (the
    // reveal) with the list count at that instant, and the final list count.
    let mut off_seen = false;
    let mut reveal_listed = None;
    let mut on_after_reveal_always = true;
    let mut max_listed = 0;
    for _ in 0..320 {
        m.run_frame();
        let on = display_on(&m);
        let listed = listed_count(&m);
        max_listed = max_listed.max(listed);
        if !on {
            off_seen = true;
            if reveal_listed.is_some() {
                on_after_reveal_always = false; // flickered back off after revealing
            }
        } else if off_seen && reveal_listed.is_none() {
            reveal_listed = Some(listed); // first on-frame after the build blank
        }
    }

    assert!(off_seen, "the display should blank while the menu builds");
    let reveal_listed = reveal_listed.expect("the display should turn back on once built");
    assert!(max_listed >= 2, "the cart entry should be listed (got {max_listed})");
    assert_eq!(
        reveal_listed, max_listed,
        "the menu was revealed with a partial list ({reveal_listed} of {max_listed}); \
         discovery must finish before the DISPON reveal"
    );
    assert!(on_after_reveal_always, "the display must stay on once the menu is revealed");
}
