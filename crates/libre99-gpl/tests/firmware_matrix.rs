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

//! **The firmware matrix** (plan §8) — the shared `[TI_ROM, OUR_ROM]` conformance
//! harness. Each flow runs under the authentic ROM (the oracle) and our rewrite
//! on the same GROM, and the **conformance checkpoint** is diffed: the SYS
//! scratchpad `>8300-83DF`, the 8 VDP registers, and a VRAM window. The GPLWS
//! (`>83E0-83FF`) is interpreter-internal (the two implementations allocate
//! registers differently) and excluded by design; a small documented whitelist
//! covers the other intended differences.
//!
//! As milestones land, their flows are added here instead of accreting per-flow
//! `rom_*` twins (execution amendment 4). Today's rows cover what our ROM
//! implements through M2: the title, the selection menu, and a long idle over
//! which the VBLANK ISR runs its duties.

use std::sync::LazyLock;

use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static TI_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

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
    libre99_gpl::system_grom::build_console_grom().expect("console GROM")
}

/// The conformance checkpoint: the SYS scratchpad `>8300-83DF`, the 8 VDP
/// registers, and VRAM `>0000-1FFF` (name / color / pattern / sprite tables).
///
/// The scratchpad **whitelist** (the plan §8 "commented list of intended
/// differences") zeroes cells that legitimately differ at M2. Two groups:
///
/// *Interpreter-internal residue* (no software reads it; the M1 microsuite
/// documents the same set, RECON §16): `>8300-8307` operand-engine temporaries,
/// `>8372/8373` GPL data/sub-stack pointer bytes, `>8380-8383` sub-stack slots
/// (GPUSH/GPOP GROM-position residue), `>837C` the GPL status byte (transient at
/// a mid-flow frame boundary), `>83C7` the KSCAN modifier working cell, and
/// `>83DA-83DF` the INTWS RTWP frame (each ROM's own interrupted WP/PC/ST).
///
/// *ISR-counter cycle-timing offset* — with M3's full SGROM 16-base power-up
/// walk in place, the DSR power-up entry `>83D2` and the sound/timeout *pointers*
/// now realign (they left the whitelist). What remains is a pure timing offset:
/// our interpreter spends more CPU cycles per GPL instruction than authentic, so
/// our PUSCAN spans ~10 more frames before the boot arms the VDP interrupt, and
/// every ISR-driven counter then lags by that fixed offset — `>8379` the SPEED
/// timer, `>83D6/83D7` the screen-timeout counter, `>83CC-83CE` the sound-list
/// progress. This is frame-level-not-cycle-level parity (plan §2.4), a deliberate
/// non-goal, and the phantom power-up routines the walk runs off the absent GROM
/// at `>E000` (undefined GPL) add to it. Everything else must match byte-for-byte.
fn checkpoint(m: &Machine) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let pad: Vec<u8> = (0x8300u16..0x83E0)
        .map(|a| match a {
            0x8300..=0x8307 | 0x8372 | 0x8373 | 0x8380..=0x8383 => 0,
            0x837C | 0x83C7 | 0x83DA..=0x83DF => 0,
            0x8379 | 0x83CC..=0x83CE | 0x83D6 | 0x83D7 => 0,
            _ => m.bus().peek(a),
        })
        .collect();
    let regs: Vec<u8> = (0..8).map(|r| m.vdp().register(r)).collect();
    let vram: Vec<u8> = (0x0000u16..0x2000).map(|a| m.vdp().vram(a)).collect();
    (pad, regs, vram)
}

/// Run `grom` under `rom` for `frames`, optionally pressing `key` at frame
/// `press_at` (and holding it thereafter).
fn run(rom: &[u8], grom: &[u8], frames: usize, key: Option<(TiKey, usize)>) -> Machine {
    let mut m = Machine::new(rom, grom);
    m.reset();
    for f in 0..frames {
        if let Some((k, at)) = key {
            if f == at {
                m.set_key(k, true);
            }
        }
        m.run_frame();
    }
    m
}

/// Assert our ROM and the authentic ROM reach a byte-identical checkpoint after
/// the same flow — the whole-firmware conformance signal.
fn assert_parity(name: &str, ti_rom: &[u8], grom: &[u8], frames: usize, key: Option<(TiKey, usize)>) {
    let auth = checkpoint(&run(ti_rom, grom, frames, key));
    let ours = checkpoint(&run(&our_rom(), grom, frames, key));
    let diffs: Vec<String> = (0..auth.0.len())
        .filter(|&i| auth.0[i] != ours.0[i])
        .map(|i| format!(">{:04X} auth={:02X} ours={:02X}", 0x8300 + i, auth.0[i], ours.0[i]))
        .collect();
    assert!(diffs.is_empty(), "{name}: SYS scratchpad diverged at {diffs:?}");
    assert_eq!(ours.1, auth.1, "{name}: VDP registers diverged");
    assert_eq!(ours.2, auth.2, "{name}: VRAM >0000-1FFF diverged");
}

#[test]
fn matrix_title_parity() {
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    // Boot to the settled master title: full checkpoint parity vs authentic.
    assert_parity("title", ti_rom, &our_grom(), 60, None);
}

#[test]
fn matrix_menu_parity() {
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    // Title -> keypress -> the selection menu (the SCANNING pass + the cart list).
    assert_parity("menu", ti_rom, &our_grom(), 200, Some((TiKey::Space, 90)));
}

#[test]
fn matrix_idle_isr_parity() {
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    // A long idle at the title: the VBLANK ISR runs every frame (the SPEED timer
    // >8379, the screen-timeout >83D6, the VDP-status copy >837B, and the gated
    // sound/sprite duties over an empty list). After 150 frames the whole
    // checkpoint must still match authentic — the ISR duties in integration.
    assert_parity("idle-isr", ti_rom, &our_grom(), 150, None);
}

/// **Gate (M3): the boot power-up scan runs the authentic PUSCAN shape.** Count
/// entries into SROM (`>0AC0`) and SGROM (`>0B24`) from reset until the boot
/// settles into the title key-wait (KSCAN `>02B2`). Our ROM's device linkage
/// must reproduce it exactly: **SROM once** (the peripheral scan, no card) then
/// the **SGROM 16-base power-up walk**, converging with the cursor `>83D0` back
/// to 0. This is the M3 replacement for the M1 minimal not-found SGROM (whose
/// single iteration was the boot-timing whitelist's original cause).
#[test]
fn matrix_puscan_walk_matches_authentic() {
    let Some(ti_rom) = TI_ROM.as_deref() else { skip!() };
    fn walk(rom: &[u8], grom: &[u8]) -> (u32, u32, u16) {
        let mut m = Machine::new(rom, grom);
        m.reset();
        let (mut srom, mut sgrom, mut prev) = (0u32, 0u32, 0u16);
        for _ in 0..6_000_000usize {
            m.step();
            let pc = m.cpu().pc();
            if pc == 0x0AC0 && prev != 0x0AC0 {
                srom += 1;
            }
            if pc == 0x0B24 && prev != 0x0B24 {
                sgrom += 1;
            }
            if pc == 0x02B2 {
                return (srom, sgrom, m.bus().peek_word(0x83D0));
            }
            prev = pc;
        }
        (srom, sgrom, 0xFFFF) // never reached the key-wait
    }
    let grom = our_grom();
    let auth = walk(ti_rom, &grom);
    assert_eq!(auth, (1, 16, 0x0000), "authentic PUSCAN baseline moved: {auth:?}");
    let ours = walk(&our_rom(), &grom);
    assert_eq!(
        ours, auth,
        "our SGROM power-up walk diverged from authentic (SROM count, SGROM count, final >83D0)"
    );
}
