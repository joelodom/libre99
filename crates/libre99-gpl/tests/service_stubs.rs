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

//! Gate for the **service-entry stub grid** (QUALITY-ASSESSMENT §5 item 7 / B2).
//! The fixed GPLLNK/XMLLNK entries `>0038-005F` must be filled (no silent zero
//! tail) **and** an unimplemented-service CALL must degrade *gracefully* — a bare
//! RTN that lets the cart carry on, **not** a reboot.
//!
//! The reboot form was a real regression: the coverage sweep
//! (`tests/coverage_sweep.rs`) showed 16 bundled carts CALL an unimplemented
//! service and rely on the graceful no-op the old zero tail (RTN bytes) provided;
//! an earlier revision that rebooted at `SVCBAD` kicked **Parsec** to our title
//! mid-game. `console.gpl` now RTNs. This guards both halves.
//!
//! One grid entry has since graduated to a real service: `>004A` is the
//! lower-case character-set loader (`LDLSET`), which the 26 carts the sweep
//! counts on it — Parsec's in-game small-caps text among them — actually need
//! (`tests/char_set.rs` gates its behavior against the authentic console).

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

/// **Static:** the formerly-zero service tail is filled — no entry in `>004A..>005E`
/// is a silent `>00` (RTN). (`>005F` is an intentional 1-byte RTN pad to `>0060`.)
/// And each 3-byte entry is a `B` (opcode `>05`) — to `SVCBAD` for the stubs, to
/// the lower-case loader for the real service at `>004A`.
#[test]
fn service_grid_has_no_silent_zero_tail() {
    let grom = our_grom();
    let zeros = (0x004A..=0x005E).filter(|&a| grom[a] == 0).count();
    assert_eq!(
        zeros, 0,
        "the GPLLNK service tail >004A-005E must be stubs, not zero bytes; found {zeros} zeros"
    );
    for a in (0x0038..=0x005C).step_by(3) {
        assert_eq!(grom[a], 0x05, "service entry >{a:04X} should be a B (>05) entry");
    }
}

/// **Runtime regression guard:** a real cart that CALLs into the service grid
/// (Parsec — historically the cart the reboot-form stub kicked to our title
/// mid-game; its `>004A` call is nowadays the real lower-case loader) must keep
/// running after launch — it does **not** reboot the cart to our title.
#[test]
fn unimplemented_service_call_does_not_reboot_the_cart() {
    let Some(console_rom) = CONSOLE_ROM.as_deref() else { skip!() };
    let Some(data) = libre99_core::third_party::load("cartridges/Parsec.ctg") else { skip!() };
    let cart = Cartridge::parse(&data).unwrap();
    let grom = our_grom();
    let mut m = Machine::new(console_rom, &grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 260); // -> menu
    tap(&mut m, TiKey::Num2, 6, 260);  // launch Parsec
    tap(&mut m, TiKey::Space, 4, 120); // through its intro (which CALLs a service)
    tap(&mut m, TiKey::Num1, 4, 120);

    let s = screen(&m);
    assert!(
        !s.contains("JOEL ODOM"), // unique to our reboot title (not carts' own "TEXAS INSTRUMENTS")
        "Parsec rebooted to our title after launch — an unimplemented-service CALL must \
         RTN gracefully, not reboot. Screen:\n{s}"
    );
}
