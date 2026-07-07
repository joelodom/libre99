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

//! Regression gate: cartridges that draw their text with the console's built-in
//! character sets must find those sets loaded. A cartridge points cell `>834A`
//! at a VDP pattern-table address and `CALL`s interconnect slot `>0016` (the
//! standard set), `>0018` (the thin "small" set), or the fixed service entry
//! `>004A` (the lower-case small-capitals set, `>60..>7E`); the console fills
//! the glyphs there. Our rewrite originally stubbed both interconnect slots to
//! a bare `RTN`, so **TI Invaders' opening-screen text rendered blank** (every
//! glyph all-zero) while its sprites — which the cartridge defines itself —
//! still showed. See `original-content/system-roms/DEBUGGING.md` "TI Invaders
//! text doesn't draw". Later, `>004A` (still a no-op stub then) did the same to
//! **Parsec's in-game small-caps text**: Parsec stages the lower-case set
//! through `>004A` and copies glyphs into its own character codes for "press
//! fire to begin", so the stub left those slots holding leftover full-size
//! patterns — the "random full-size characters" garble.
//!
//! The checks are differential and mechanical: drive the same cartridge (or a
//! synthetic one) under the authentic GROM and under ours, then assert the
//! observable results match. For TI Invaders: **every character code the
//! authentic opening screen draws with a non-blank glyph is also non-blank
//! under our GROM** (before the fix, 50+ codes are missing; after, 0). For
//! `>004A`: the staged bytes must be **identical**.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

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

/// Boot `grom`, mount the cart, leave the title, launch program 2 (TI Invaders),
/// and let its opening screen settle. Return `(pattern_table, name_table)` read
/// from the live VDP bases.
fn opening_screen(grom: &[u8], cart: &Cartridge) -> (Vec<u8>, Vec<u8>) {
    let console_rom = CONSOLE_ROM.as_deref().expect("presence checked by each test");
    let mut m = Machine::new(console_rom, grom);
    m.mount_cartridge(cart);
    m.reset();
    let run = |m: &mut Machine, n| (0..n).for_each(|_| m.run_frame());
    run(&mut m, 90);
    m.set_key(TiKey::Space, true);
    run(&mut m, 4);
    m.set_key(TiKey::Space, false);
    run(&mut m, 200);
    m.set_key(TiKey::Num2, true);
    run(&mut m, 4);
    m.set_key(TiKey::Num2, false);
    run(&mut m, 700);

    let pat = ((m.vdp().register(4) & 0x07) as u16) * 0x800;
    let name = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let pattern: Vec<u8> = (0..0x800).map(|i| m.vdp().vram(pat + i)).collect();
    let names: Vec<u8> = (0..0x300).map(|i| m.vdp().vram(name + i)).collect();
    (pattern, names)
}

fn glyph_present(pattern: &[u8], code: usize) -> bool {
    (0..8).any(|r| pattern[(8 * code + r) & 0x7FF] != 0)
}

fn ti_invaders() -> Option<Cartridge> {
    let data = libre99_core::third_party::load("cartridges/TI-Invaders.ctg")?;
    Some(Cartridge::parse(&data).unwrap())
}

/// **Gate: TI Invaders' opening-screen text draws under our GROM.** Every code
/// the authentic opening screen renders with a real glyph must also have a real
/// glyph in ours — i.e. the console's `>0016`/`>0018` character-set loaders did
/// their job. (Pre-fix this fails with dozens of blank codes.)
#[test]
fn ti_invaders_opening_text_glyphs_load_on_our_grom() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let Some(authentic_grom) = AUTHENTIC_GROM.as_deref() else { skip!() };
    let Some(cart) = ti_invaders() else { skip!() };
    let (auth_pat, auth_names) = opening_screen(authentic_grom, &cart);
    let ours = our_grom();
    let (our_pat, _) = opening_screen(&ours, &cart);

    // Codes actually placed on the authentic opening screen.
    let mut on_screen = [false; 256];
    for &c in &auth_names {
        on_screen[c as usize] = true;
    }

    let missing: Vec<usize> = (0..256)
        .filter(|&c| on_screen[c] && glyph_present(&auth_pat, c) && !glyph_present(&our_pat, c))
        .collect();

    assert!(
        missing.is_empty(),
        "these on-screen codes have glyphs under the authentic GROM but are blank under ours \
         (the >0016/>0018 char-set loaders aren't filling the pattern table): {:02X?}",
        missing
    );

    // Sanity: the screen really does draw a lot of text (guards against the test
    // trivially passing on a blank screen under both).
    let auth_glyphed = (0..256).filter(|&c| on_screen[c] && glyph_present(&auth_pat, c)).count();
    assert!(auth_glyphed > 30, "expected a text-rich opening screen, saw {auth_glyphed} glyphed codes");
}

/// **Gate: the two char-set loader slots are wired to real routines, not the
/// no-op `RTN` stub.** The interconnect slots at `>0016`/`>0018` must each be a
/// `BR` (opcode `>40..>5F`) to a loader — a direct structural check on the image.
#[test]
fn char_set_loader_slots_are_wired() {
    let img = our_grom();
    for slot in [0x0016usize, 0x0018] {
        assert!(
            (0x40..=0x5F).contains(&img[slot]),
            "interconnect slot >{slot:04X} must be a BR to a char-set loader, found >{:02X}",
            img[slot]
        );
    }
    // The fixed service entry >004A must be a 3-byte B (opcode >05) to the
    // lower-case loader — anywhere but SVCBAD (>1201), the stub-grid target.
    assert_eq!(img[0x004A], 0x05, "service entry >004A must be a B to the lower-case loader");
    let target = ((img[0x004B] as u16) << 8) | img[0x004C] as u16;
    assert_ne!(
        target, 0x1201,
        "service entry >004A must reach the lower-case loader, not the SVCBAD stub"
    );
}

/// A synthetic pure-GROM cartridge that does exactly what Parsec does at
/// startup: point `>834A` at a VDP pattern-table address and `CALL` the fixed
/// lower-case loader entry `>004A`, then spin. Assembled with our own GPL
/// assembler and launched like any cart.
fn lower_loader_cart() -> Cartridge {
    // The assembler's image covers the three console GROMs (24 KiB), so the
    // page is assembled at slot 0 and mounted at >6000 — position-independent
    // by construction: the two header pointers are written as numeric >6xxx
    // literals (their offsets are fixed by the layout above them, not by
    // instruction encodings), and the self-loop is a BR, which the interpreter
    // resolves within the current 8 KiB bank.
    let src = "
        GROM >0000
        BYTE >AA,>01,>00,>00     ; valid header, version 1
        DATA >0000               ; power-up list = none
        DATA >600C               ; program list (MENU, at the fixed offset >0C)
        DATA >0000               ; DSR list = none
        DATA >0000               ; subprogram list = none
MENU    DATA >0000               ; no next entry
        DATA >6016               ; program entry (ENTRY, at the fixed offset >16)
        BYTE >05
        TEXT 'LOWER'
ENTRY   DST  @>834A,>0B00        ; stage at PDT >0800 + >60*8, like a real cart
        CALL >004A               ; load the lower-case (small caps) set
HANG    BR   HANG
";
    let assembly = libre99_gpl::assemble(src).unwrap_or_else(|d| panic!("cart assembly failed: {d:?}"));
    let mut page = assembly.image;
    page.truncate(0x2000);
    page.resize(0x2000, 0);
    Cartridge {
        title: "LOWER SVC".into(),
        cru_base: 0,
        rom: Vec::new(),
        rom_banks: 0,
        grom: vec![(0x6000, page)],
    }
}

/// Launch [`lower_loader_cart`] on `grom` and return the 248 staged bytes at
/// VDP `>0B00` plus the two bytes of `>834A` (the loader advances it).
fn staged_lower(grom: &[u8], cart: &Cartridge) -> (Vec<u8>, [u8; 2]) {
    let console_rom = CONSOLE_ROM.as_deref().expect("presence checked by each test");
    let mut m = Machine::new(console_rom, grom);
    m.mount_cartridge(cart);
    m.reset();
    let run = |m: &mut Machine, n| (0..n).for_each(|_| m.run_frame());
    run(&mut m, 90);
    m.set_key(TiKey::Space, true);
    run(&mut m, 4);
    m.set_key(TiKey::Space, false);
    run(&mut m, 200);
    m.set_key(TiKey::Num2, true);
    run(&mut m, 4);
    m.set_key(TiKey::Num2, false);
    run(&mut m, 120);

    let block: Vec<u8> = (0..0xF8).map(|i| m.vdp().vram(0x0B00 + i)).collect();
    (block, [m.bus().peek(0x834A), m.bus().peek(0x834B)])
}

/// **Gate: the lower-case loader at the fixed service entry `>004A` matches the
/// authentic console byte-for-byte.** The synthetic cart stages the set under
/// both firmwares; the 31 glyphs (blank top row + seven stored rows each, 248
/// bytes) and the advanced `>834A` must be identical — and must equal the
/// glyphs our `font` module ships, which are themselves gated byte-identical to
/// the authentic GROM's `>0874` block. This is the Parsec "press fire to begin"
/// contract: before the loader existed, the `>004A` stub no-opped and the
/// staged area kept its previous contents.
#[test]
fn lower_case_loader_matches_the_authentic_console() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let Some(authentic_grom) = AUTHENTIC_GROM.as_deref() else { skip!() };
    let cart = lower_loader_cart();
    let (auth_block, auth_ptr) = staged_lower(authentic_grom, &cart);
    let ours = our_grom();
    let (our_block, our_ptr) = staged_lower(&ours, &cart);

    // Sanity: the authentic loader really ran (the staging area is not blank
    // and >834A advanced past the 248 bytes written).
    assert!(auth_block.iter().any(|&b| b != 0), "authentic >004A staged nothing");
    assert_eq!(auth_ptr, [0x0B, 0xF8], "authentic loader advances >834A by >F8");

    assert_eq!(our_block, auth_block, ">004A must stage the same bytes as the authentic console");
    assert_eq!(our_ptr, auth_ptr, ">004A must leave >834A as the authentic console does");

    // And the staged bytes are exactly the expanded font module data.
    let expected: Vec<u8> = libre99_gpl::font::packed_lower()
        .chunks(7)
        .flat_map(|rows| std::iter::once(0u8).chain(rows.iter().copied()))
        .collect();
    assert_eq!(our_block, expected, "staged block must be the expanded lower-case font");
}
