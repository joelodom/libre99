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

//! Phase-2 bisect for the F5 bug: find the exact moment (frame, then CPU
//! instruction) where the 9901 VDP-interrupt mask (CRU bit 2) goes OFF during
//! our warm boot — and who wrote it.
//!
//! Run from the repo root:  cargo run -p libre99-core --example f5_mask_bisect

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

/// Load one third-party image, or exit — this probe needs the authentic media.
fn need(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2);
    })
}

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| need("roms/994aROM.Bin"));
static INVADERS: LazyLock<Vec<u8>> = LazyLock::new(|| need("cartridges/TI-Invaders.ctg"));

fn tap(m: &mut Machine, k: TiKey, hold: usize, settle: usize) {
    m.set_key(k, true);
    for _ in 0..hold {
        m.run_frame();
    }
    m.set_key(k, false);
    for _ in 0..settle {
        m.run_frame();
    }
}

/// Boot our committed GROM with Invaders + DSR, launch, play a bit. Leaves the
/// machine mid-game, exactly as the user's F5 finds it.
fn to_gameplay(grom: &[u8]) -> Machine {
    let cart = Cartridge::parse(&INVADERS).expect("parse");
    let mut m = Machine::new(&CONSOLE_ROM, grom);
    m.mount_cartridge(&cart);
    if let Some(dsr) = libre99_core::third_party::load("roms/Disk.Bin") {
        m.load_disk_controller(&dsr);
    }
    m.reset();
    for _ in 0..200 {
        m.run_frame();
    }
    tap(&mut m, TiKey::Space, 3, 320);
    tap(&mut m, TiKey::Num2, 6, 300);
    tap(&mut m, TiKey::Num1, 3, 120);
    tap(&mut m, TiKey::S, 3, 60);
    tap(&mut m, TiKey::D, 3, 60);
    m
}

fn main() {
    // Fail fast (with the pointer message) when the third-party media is absent.
    LazyLock::force(&CONSOLE_ROM);
    LazyLock::force(&INVADERS);

    let ours = std::fs::read("original-content/system-roms/grom/console-grom.bin").expect("committed GROM");

    // --- Phase A: frame-level scan --------------------------------------
    let mut m = to_gameplay(&ours);
    println!("during gameplay: 9901int={}", m.bus().tms9901.vdp_interrupt_enabled());
    m.reset(); // F5
    println!("right after m.reset(): 9901int={}", m.bus().tms9901.vdp_interrupt_enabled());
    let mut flip_frame = None;
    let mut last = m.bus().tms9901.vdp_interrupt_enabled();
    for f in 0..250 {
        m.run_frame();
        let now = m.bus().tms9901.vdp_interrupt_enabled();
        if now != last {
            println!("frame {f}: 9901int {last} -> {now}   (>83CE={:02X} >8379={:02X})", m.bus().peek(0x83CE), m.bus().peek(0x8379));
            if !now && flip_frame.is_none() {
                flip_frame = Some(f);
            }
            last = now;
        }
    }
    println!("state at +250 frames: 9901int={}", m.bus().tms9901.vdp_interrupt_enabled());

    // --- Phase C: watch >8300 around the arming IO (>0105..>0112) --------
    // Step from the F5 and print every change of (grom_addr window, >8300,
    // mask) while the GPL PC is inside the IO block — the CPU PC at the
    // moment >8300 changes identifies the writer (ISR? card DSR? interpreter?).
    let mut m = to_gameplay(&ours);
    m.reset();
    let mut last_cell = (m.bus().peek(0x8300), m.bus().peek(0x8301));
    let mut last_int = m.bus().tms9901.vdp_interrupt_enabled();
    let mut lines = 0;
    for i in 0..4_000_000u64 {
        m.step();
        let ga = m.bus().grom_address();
        let cell = (m.bus().peek(0x8300), m.bus().peek(0x8301));
        let int = m.bus().tms9901.vdp_interrupt_enabled();
        let interesting = (0x0100..=0x0118).contains(&ga);
        if (cell != last_cell || int != last_int) && (interesting || cell != last_cell) {
            println!(
                "step {i}: PC={:04X} grom_addr={:04X}  >8300={:02X}{:02X} -> {:02X}{:02X}  9901int={}",
                m.cpu().pc(),
                ga,
                last_cell.0,
                last_cell.1,
                cell.0,
                cell.1,
                int
            );
            last_cell = cell;
            last_int = int;
            lines += 1;
            if lines > 60 {
                println!("(output capped)");
                break;
            }
        }
        if !int && i > 0 {
            println!("mask now OFF at step {i}, PC={:04X}, grom_addr={:04X}", m.cpu().pc(), ga);
            break;
        }
    }

    // --- Phase B: instruction-level bisect within the run ----------------
    // Re-run the same deterministic flow, stepping instruction by instruction
    // from the F5 onward; report every transition of the mask with the PC,
    // R12, and the GROM address at that moment.
    let mut m = to_gameplay(&ours);
    m.reset();
    let mut last = m.bus().tms9901.vdp_interrupt_enabled();
    let mut transitions = 0;
    for i in 0..4_000_000u64 {
        m.step();
        let now = m.bus().tms9901.vdp_interrupt_enabled();
        if now != last {
            let pc = m.cpu().pc();
            println!(
                "step {i}: 9901int {last} -> {now}  PC={:04X}  R12={:04X}  grom_addr={:04X}",
                pc,
                m.reg(12),
                m.bus().grom_address()
            );
            last = now;
            transitions += 1;
            if transitions > 12 {
                println!("(more transitions; stopping the report)");
                break;
            }
        }
    }
    println!("final: 9901int={}", m.bus().tms9901.vdp_interrupt_enabled());
}
