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

//! **Milestone 1 gate** — the rewritten GROM boots the genuine console ROM to an
//! **original recreation of the master title screen**: the authentic layout
//! (colour bars top and bottom, the `TEXAS INSTRUMENTS` / `HOME COMPUTER`
//! banner, our own `LIBRE 99 ROMS` line, `READY-PRESS ANY KEY TO BEGIN`) drawn in
//! our own font, but with TI's copyrighted content removed — the "TI" logo
//! replaced by the original Libre99 chip logo (glyphs `>0B..>1E`) and the
//! `© 1981 TEXAS INSTRUMENTS` copyright replaced by
//! `© 2026 JOEL ODOM  LIBRE99.COM`.
//!
//! Keeping the words "TEXAS INSTRUMENTS HOME COMPUTER" is deliberate — it is the
//! machine this firmware runs on, not TI's creative expression; the chip logo,
//! the LIBRE 99 ROMS line, and the copyright, which are, are ours.

use std::collections::HashSet;
use std::sync::LazyLock;
use libre99_core::machine::Machine;
use libre99_core::vdp::{HEIGHT, WIDTH};

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

/// Boot our GROM on the authentic console ROM, or `None` when the ROM is
/// absent (the caller then skips).
fn boot_rewrite() -> Option<Machine> {
    let console_rom = CONSOLE_ROM.as_deref()?;
    let grom = libre99_gpl::system_grom::build_console_grom()
        .unwrap_or_else(|d| panic!("console GROM assembly failed: {d:?}"));
    let mut m = Machine::new(console_rom, &grom);
    for _ in 0..60 {
        m.run_frame();
    }
    Some(m)
}

/// Read a run of name-table cells as a string (identity-mapped ASCII).
fn name_text(m: &Machine, row: u16, col: u16, len: u16) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..len)
        .map(|i| m.vdp().vram(base + row * 32 + col + i) as char)
        .collect()
}

#[test]
fn boots_to_recreated_title_screen() {
    let Some(mut m) = boot_rewrite() else { skip!() };

    // The display must be enabled.
    assert!(
        m.vdp().register(1) & 0x40 != 0,
        "display should be enabled (R1=>{:02X})",
        m.vdp().register(1)
    );

    // The master-title layout: the banner bumped up a row to make room for our
    // own LIBRE 99 ROMS line between HOME COMPUTER and the key prompt.
    assert_eq!(name_text(&m, 8, 7, 17), "TEXAS INSTRUMENTS");
    assert_eq!(name_text(&m, 10, 9, 13), "HOME COMPUTER");
    assert_eq!(name_text(&m, 13, 9, 13), "LIBRE 99 ROMS");
    assert_eq!(name_text(&m, 16, 2, 28), "READY-PRESS ANY KEY TO BEGIN");

    // TI's copyright is replaced by ours; the "TI" logo year is gone.
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let screen: String = (0..24 * 32)
        .map(|i| m.vdp().vram(base + i) as char)
        .collect();
    assert!(screen.contains("2026 JOEL ODOM"), "our copyright is present");
    assert!(screen.contains("LIBRE99.COM"), "the project URL is present");
    assert!(!screen.contains("1981"), "TI's copyright year must be gone");

    // Copyright layout: the name hugs the left edge (space at col 0, the ringed-C
    // glyph >0A at col 1) and the URL hugs the right edge (ends col 30, space at
    // col 31), with spaces filling the middle.
    let row22 = base + 22 * 32;
    assert_eq!(m.vdp().vram(row22), 0x20, "col 0 is the left margin space");
    assert_eq!(m.vdp().vram(row22 + 1), 0x0A, "the (c) glyph sits at col 1");
    assert_eq!(name_text(&m, 22, 20, 11), "LIBRE99.COM");
    assert_eq!(m.vdp().vram(row22 + 31), 0x20, "col 31 is the right margin space");

    // The original "TI" logo is replaced by the Libre99 chip logo: glyphs >0B..>1E
    // occupy the 5x4 block at rows 3-6, cols 13-17 (never plain ASCII text).
    for (i, (r, c)) in [(3u16, 13u16), (4, 15), (5, 16), (6, 17)].iter().enumerate() {
        let code = m.vdp().vram(base + r * 32 + c);
        assert!(
            (0x0B..=0x1E).contains(&code),
            "logo cell {i} at ({r},{c}) should be a chip-logo glyph, got >{code:02X}"
        );
    }

    // Colour bars: rows 0-2 and 18-20 are filled with bar glyphs (>60..>DF).
    for &r in &[0u16, 1, 2, 18, 19, 20] {
        let all_bars = (0..32u16).all(|c| (0x60..=0xDF).contains(&m.vdp().vram(base + r * 32 + c)));
        assert!(all_bars, "row {r} should be a colour-bar row");
    }

    // A real, multi-coloured drawn screen (cyan backdrop + black text + the 16
    // bar colours ⇒ well over three distinct colours).
    let mut fb = vec![0u32; WIDTH * HEIGHT];
    m.render(&mut fb);
    let distinct: HashSet<u32> = fb.iter().copied().collect();
    assert!(distinct.len() >= 8, "expected the colour bars, saw {}", distinct.len());
}

/// Write one VRAM byte through the CPU-facing VDP ports (address setup + data),
/// the way running software dirties the screen — so a plain `m.reset()` leaves
/// it in VRAM (reset touches only the CPU, not VRAM).
fn poke_vram(m: &mut Machine, addr: u16, byte: u8) {
    let vdp = &mut m.bus_mut().vdp;
    vdp.write_control((addr & 0xFF) as u8);
    vdp.write_control(((addr >> 8) as u8 & 0x3F) | 0x40); // 0x40 = address-for-write
    vdp.write_data(byte);
}

/// Reset regression: F5 (`m.reset()`) re-runs the boot but does NOT clear VRAM,
/// so a reset from a running cartridge left that program's tiles and sprites on
/// the title screen. START must repaint the *whole* screen — clear the name
/// table to spaces and disable sprites — the way the authentic GROM does.
#[test]
fn reset_from_a_dirty_screen_repaints_it_clean() {
    let Some(mut m) = boot_rewrite() else { skip!() };

    // Dirty the screen the way a game would: fill the whole name table with a
    // junk tile and load a live sprite (visible Y, opaque colour) at entry 0.
    for a in 0..0x300u16 {
        poke_vram(&mut m, a, 0xCA);
    }
    poke_vram(&mut m, 0x300, 0x40); // sprite 0: Y (on-screen, not the >D0 terminator)
    poke_vram(&mut m, 0x301, 0x40); // X
    poke_vram(&mut m, 0x302, 0x01); // pattern
    poke_vram(&mut m, 0x303, 0x0F); // early-clock|colour 15 (opaque -> would show)

    // F5.
    m.reset();
    for _ in 0..60 {
        m.run_frame();
    }

    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;

    // No junk tile survives anywhere in the name table — every cell is either a
    // space or a title glyph the boot drew, never the >CA we planted.
    let leftover = (0..0x300u16).filter(|&i| m.vdp().vram(base + i) == 0xCA).count();
    assert_eq!(leftover, 0, "{leftover} junk cells survived the reset");

    // Row 23 sits below the copyright line, so the boot never writes it: it must
    // be all spaces, proving the name table was cleared rather than just painted.
    for c in 0..32u16 {
        let cell = m.vdp().vram(base + 23 * 32 + c);
        assert_eq!(cell, 0x20, "row 23 col {c} should be a space, got >{cell:02X}");
    }

    // The leftover sprite is gone: entry 0's Y is the >D0 list terminator, so the
    // VDP draws no sprites.
    assert_eq!(
        m.vdp().vram(0x300),
        0xD0,
        "sprite list should be terminated (entry 0 Y = >D0) after reset"
    );

    // And the boot actually completed — the title is on screen.
    assert_eq!(name_text(&m, 8, 7, 17), "TEXAS INSTRUMENTS");
}

#[test]
fn font_and_emblem_loaded_into_pattern_table() {
    let Some(m) = boot_rewrite() else { skip!() };
    // Pattern table at >0800; 'T' (>54) glyph must be non-blank.
    let t = 0x0800 + (b'T' as u16) * 8;
    assert!((0..8).any(|i| m.vdp().vram(t + i) != 0), "'T' glyph should be loaded");
    // The chip-logo glyph >0B (top-left of the logo) must be non-blank too.
    let e = 0x0800 + 0x0B * 8;
    assert!((0..8).any(|i| m.vdp().vram(e + i) != 0), "chip-logo glyph >0B should be loaded");
}
