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

//! Console ROM chunk R-3 (D2) — the **P8 tripwire**: real ML cartridges enter
//! the console ROM (`>0000-1FFF`) only at documented public addresses (the
//! `rom/SURFACE-MAP.md` frozen-address table). A cart branching into a ROM
//! *interior* address would be a finding — an entry point the rewrite must also
//! freeze. Runs under the **authentic** ROM (the oracle); see
//! the archived `system-roms/history/ROM-ENTRY-CENSUS.md` and
//! `examples/rom_entry_census.rs`.
//!
//! `#[ignore]`d (single-stepping a launched cart is a few seconds). Run:
//! `cargo test -p libre99-asm --test entry_census -- --ignored --nocapture`.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static AUTH_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));
static CENTIPEDE: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("cartridges/centipe.ctg"));

/// The documented public entries external ML software may branch to
/// (SURFACE-MAP frozen-address table). Interrupt/soft entries included.
const PUBLIC_ENTRIES: &[u16] =
    &[0x0000, 0x0004, 0x0008, 0x000E, 0x0016, 0x001C, 0x0020, 0x0024, 0x006A, 0x0070, 0x0900];

#[test]
#[ignore = "single-steps a launched cart; run in the deep tier"]
fn ml_cart_enters_rom_only_at_public_entries() {
    let (Some(rom), Some(grom), Some(centipede)) =
        (AUTH_ROM.as_deref(), AUTH_GROM.as_deref(), CENTIPEDE.as_deref())
    else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let cart = Cartridge::parse(centipede).expect("parse centipe.ctg");
    let mut m = Machine::new(rom, grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..200 {
        m.run_frame();
    }
    // Title -> menu -> launch Centipede (entry 2).
    m.set_key(TiKey::Space, true);
    for _ in 0..3 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..320 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, false);
    for _ in 0..150 {
        m.run_frame();
    }

    // Single-step the running cart, recording every external->ROM entry, with a
    // little input so the keyboard path is exercised.
    let external = |pc: u16| pc >= 0x2000;
    let mut entries: BTreeSet<u16> = BTreeSet::new();
    let mut prev = m.cpu().pc();
    for i in 0..3_000_000u64 {
        if i % 20_000 == 0 {
            m.set_key(TiKey::Num1, true);
        } else if i % 20_000 == 4_000 {
            m.set_key(TiKey::Num1, false);
        }
        m.step();
        let pc = m.cpu().pc();
        if external(prev) && !external(pc) {
            entries.insert(pc);
        }
        prev = pc;
    }

    assert!(!entries.is_empty(), "the ML cart never called the ROM — check the launch/step drive");
    let unlisted: Vec<u16> = entries.iter().copied().filter(|pc| !PUBLIC_ENTRIES.contains(pc)).collect();
    assert!(
        unlisted.is_empty(),
        "ML cart entered the ROM at UNDOCUMENTED interior address(es): {:04X?} \
         — a new public entry to freeze (P8). Observed set: {:04X?}",
        unlisted,
        entries
    );
}
