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

//! **Chunk 2 state-contract harness** (§6 A3 / §7.8 amendment 1). Drives the
//! authentic GROM and our rewrite through the same user flow and checks the
//! **contract** — the observable behaviour both firmwares must produce — at three
//! checkpoints: (1) settled title, (2) selection menu, (3) **F5 warm-reset from
//! gameplay → title → menu** (the checkpoint that would have caught case studies
//! 6/7/9 — VRAM/PSG/scratchpad that survive a CPU-only reset).
//!
//! Our scratchpad *values* differ from the authentic firmware's by design (our
//! original boot/menu code uses different cells), so the vs-authentic assertions
//! are on the observable contract (the VBLANK ISR is live, the display is on, the
//! menu lists the cart), not a byte diff. The reset-drift assertion — the one the
//! field bugs needed — is **our cold-boot state vs our post-F5 state** on the cells
//! `START` initializes: F5 re-runs `START`, so those must come back identical, or a
//! program's leftover value survives into a ROM service (the `>8305` class).

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

fn cart() -> Option<Cartridge> {
    let data = ["cartridges/TI-Invaders.ctg", "cartridges/amazing.ctg"]
        .iter()
        .find_map(|p| libre99_core::third_party::load(p))?;
    Some(Cartridge::parse(&data).unwrap())
}

fn tap(m: &mut Machine, k: TiKey, hold: usize, settle: usize) {
    m.set_key(k, true);
    for _ in 0..hold { m.run_frame(); }
    m.set_key(k, false);
    for _ in 0..settle { m.run_frame(); }
}

fn screen(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..24 * 32)
        .map(|i| {
            let b = m.vdp().vram(base + i);
            if (0x20..0x7F).contains(&b) { b as char } else { ' ' }
        })
        .collect()
}

/// The VBLANK ISR is running: the ROM's frame counter `>8379` advances.
fn isr_live(m: &mut Machine) -> bool {
    let mut prev = m.bus().peek(0x8379);
    let mut ticks = 0;
    for _ in 0..40 {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev { ticks += 1; }
        prev = t;
    }
    ticks > 10
}

/// Display enabled: VDP R1 bit 6 (`>40`) set.
fn display_on(m: &Machine) -> bool {
    m.vdp().register(1) & 0x40 != 0
}

/// The selection menu has listed a program: a `"n FOR NAME"` row is present.
fn menu_listed(m: &Machine) -> bool {
    screen(m).contains(" FOR ")
}

/// **Checkpoint 1 — the title comes up live under both firmwares.** Display on,
/// ISR ticking, and each firmware's own title drawn.
#[test]
fn title_contract_holds_on_both_firmwares() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(authentic_grom) = AUTHENTIC_GROM.as_deref() else { skip!() };
    let Some(cart) = cart() else { skip!() };
    for (label, grom, marker) in [
        ("authentic", authentic_grom.to_vec(), "TEXAS INSTRUMENTS"),
        ("ours", our_grom(), "JOEL ODOM"),
    ] {
        let mut m = Machine::new(console_rom, &grom);
        m.mount_cartridge(&cart);
        m.reset();
        for _ in 0..180 { m.run_frame(); }
        assert!(display_on(&m), "{label}: display should be on at the title");
        assert!(
            screen(&m).contains(marker),
            "{label}: title not drawn (marker `{marker}` absent)"
        );
        assert!(isr_live(&mut m), "{label}: VBLANK ISR not running at the title");
    }
}

/// **Checkpoint 2 — the selection menu builds under both firmwares.** Both use the
/// same console-ROM menu mechanism, so both list the cart's program with the ISR
/// still live.
#[test]
fn menu_contract_holds_on_both_firmwares() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(authentic_grom) = AUTHENTIC_GROM.as_deref() else { skip!() };
    let Some(cart) = cart() else { skip!() };
    for (label, grom) in [("authentic", authentic_grom.to_vec()), ("ours", our_grom())] {
        let mut m = Machine::new(console_rom, &grom);
        m.mount_cartridge(&cart);
        m.reset();
        for _ in 0..40 { m.run_frame(); }
        tap(&mut m, TiKey::Space, 3, 300);
        assert!(menu_listed(&m), "{label}: menu did not list a program");
        assert!(display_on(&m), "{label}: display off at the menu");
        assert!(isr_live(&mut m), "{label}: ISR not running at the menu");
    }
}

/// The cells `START` initializes and then leaves put (the `IO`-arming list and the
/// sub-stack seed). After boot they hold their init values; a program does not need
/// them, so cold-boot and post-F5 must read identically — else F5 let a program's
/// leftover survive into what the ROM reads next (case study 9's `>8305`).
const START_INIT_CELLS: &[u16] = &[
    0x8300, 0x8301, 0x8302, 0x8303, 0x8304, 0x8305, // the IO CRU-output list (4 fields)
    0x8373, // GPL sub-stack pointer seed (>7E)
];

fn snapshot(m: &Machine) -> Vec<u8> {
    START_INIT_CELLS.iter().map(|&a| m.bus().peek(a)).collect()
}

/// Snapshot at the sub-stack's **seed phase**. While the menu idles, every
/// `SCAN` briefly saves the interpreter's GROM position through the sub-stack,
/// so `>8373` legitimately breathes `>7E` ↔ `>80` across frame boundaries — a
/// raw fixed-frame sample lands on either phase by timing luck (it flipped when
/// the menu gained its row-23 system-information line). Wait, bounded, for the
/// seed reading; on a genuine reset-drift bug `>8373` never returns to `>7E`
/// and the caller's compare still fails loudly.
fn settled_snapshot(m: &mut Machine) -> Vec<u8> {
    for _ in 0..32 {
        if m.bus().peek(0x8373) == 0x7E {
            break;
        }
        m.run_frame();
    }
    snapshot(m)
}

/// **Checkpoint 3 — F5 from gameplay re-establishes the boot contract (ours).**
/// Cold-boot to the menu and snapshot the `START`-init cells; launch the cart, play,
/// then F5 and return to the menu; the console must be live again *and* those cells
/// must read exactly as on cold boot. This is the general guard for the reset-drift
/// class (case studies 6/7/9) — an unwritten field would differ here.
#[test]
fn f5_from_gameplay_reestablishes_the_boot_contract() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(cart) = cart() else { skip!() };
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 300); // -> menu
    assert!(menu_listed(&m), "cold: menu did not build");
    let cold = settled_snapshot(&mut m);

    // Launch and play (dirty scratchpad/VRAM/chips the way a game does).
    tap(&mut m, TiKey::Num2, 6, 240);
    tap(&mut m, TiKey::Space, 4, 60);
    tap(&mut m, TiKey::Num1, 4, 60);

    // F5 (CPU-only reset — RAM/VRAM/chips survive), back through the title to the menu.
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 300);

    assert!(menu_listed(&m), "post-F5: menu did not rebuild");
    assert!(display_on(&m), "post-F5: display off");
    assert!(isr_live(&mut m), "post-F5: ISR not running");
    let warm = settled_snapshot(&mut m);
    assert_eq!(
        warm, cold,
        "post-F5 START-init cells drifted from cold boot — F5 left a program's stale \
         value where the ROM reads it (the >8305 class). cells={START_INIT_CELLS:02X?} \
         cold={cold:02X?} warm={warm:02X?}"
    );
}
