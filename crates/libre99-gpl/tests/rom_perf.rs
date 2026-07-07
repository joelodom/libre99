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

//! **M7 performance-parity report** (plan §8 "Performance report"): the console
//! ROM rewrite must reach the settled title no slower than the authentic ROM
//! (frame-level parity) and cost the host no more wall-clock per emulated frame.
//! Both ROMs run on **our GROM** — the same isolation as `rom_title.rs`, so any
//! gap is our ROM's. This is the ROM-track sibling of `perf_parity.rs` (which
//! compares the two *GROMs* under the authentic ROM).
//!
//! Two metrics per ROM, printed as a table (run with `-- --nocapture`):
//!
//! 1. **frames-to-settled-title** — from `reset()`, frames until the VRAM name
//!    table (`>0000-02FF`) has been unchanged for 10 consecutive frames *with
//!    the title actually painted* (display enabled + the `TEXAS INSTRUMENTS`
//!    banner present — the paint guard keeps the pre-boot all-blank stretch
//!    from counting as "settled"). Deterministic; capped at 600 frames.
//! 2. **host wall-clock per emulated frame** — the time to run exactly 400
//!    frames of the settled title's idle (KSCAN key-wait + the VBLANK ISR).
//!    Wall-clock is noisy, so each ROM is timed 3 times — the batches
//!    interleaved authentic/ours to decorrelate host load drift — and the
//!    **minimum** is compared.
//!
//! Asserts (plan §8: our-ROM combos within ×1.25 of their authentic-ROM
//! counterparts): frames-to-settle `ours <= authentic × 1.25 + 30` (the +30 is
//! frame-level slack — plan §2.4 makes cycle-level timing a non-goal, and the
//! two boots legitimately spend different frame counts getting to the paint)
//! and wall-clock `ours <= authentic × 1.25`. No cartridge and no key input:
//! the bare title idles.

use std::sync::LazyLock;
use std::time::{Duration, Instant};
use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

/// Frame cap for the settle search — far past both boots' frames-to-title
/// (authentic ~41, ours ~11; see `rom_title.rs`) and far short of the
/// ~32768-frame screen-blank timeout.
const SETTLE_CAP: usize = 600;
/// "Settled" = the name table unchanged for this many consecutive frames.
const STABLE_FRAMES: usize = 10;
/// Frames per wall-clock batch, run after the title has settled.
const TIMED_FRAMES: usize = 400;
/// Wall-clock batches per ROM; the minimum is compared.
const WALL_RUNS: usize = 3;

fn our_rom() -> Vec<u8> {
    libre99_asm::system_rom::build_console_rom().expect("console ROM assembles")
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().expect("console GROM assembles")
}

/// The name table `>0000-02FF` (both firmwares' title keeps it at base `>0000`).
fn name_table(m: &Machine) -> Vec<u8> {
    (0x0000u16..0x0300).map(|a| m.vdp().vram(a)).collect()
}

/// Has the title genuinely painted? Display enabled (VDP R1 `>40`) and the
/// shared `TEXAS INSTRUMENTS` banner on screen (identity-mapped ASCII).
fn title_painted(m: &Machine) -> bool {
    if m.vdp().register(1) & 0x40 == 0 {
        return false;
    }
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    let screen: String = (0..24 * 32).map(|i| m.vdp().vram(base + i) as char).collect();
    screen.contains("TEXAS INSTRUMENTS")
}

/// Boot `rom` on `grom` and run until the title settles (metric 1). Returns the
/// machine (idling at the title key-wait) and the frames-to-settled-title.
fn boot_to_settled(name: &str, rom: &[u8], grom: &[u8]) -> (Machine, usize) {
    let mut m = Machine::new(rom, grom);
    m.reset();
    let mut prev = name_table(&m);
    let mut stable = 0usize;
    for frame in 1..=SETTLE_CAP {
        m.run_frame();
        let cur = name_table(&m);
        if cur == prev {
            stable += 1;
        } else {
            stable = 0;
        }
        prev = cur;
        if stable >= STABLE_FRAMES && title_painted(&m) {
            return (m, frame);
        }
    }
    panic!("{name}: title never settled within {SETTLE_CAP} frames (name table still changing, or the banner never painted)");
}

/// One timed batch: run exactly [`TIMED_FRAMES`] frames and return the elapsed
/// host wall-clock.
fn timed_batch(m: &mut Machine) -> Duration {
    let t0 = Instant::now();
    for _ in 0..TIMED_FRAMES {
        m.run_frame();
    }
    t0.elapsed()
}

fn per_frame_ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0 / TIMED_FRAMES as f64
}

/// The M7 performance-parity report: measure both ROMs on our GROM, print the
/// table, and assert our ROM within ×1.25 of authentic on both metrics.
#[test]
fn perf_parity_report() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else { skip!() };
    let grom = our_grom();
    let ours_rom = our_rom();

    // Metric 1 — frames-to-settled-title (deterministic).
    let (mut auth_m, auth_settle) = boot_to_settled("authentic", auth_rom, &grom);
    let (mut ours_m, ours_settle) = boot_to_settled("ours", &ours_rom, &grom);

    // Metric 2 — wall-clock for TIMED_FRAMES of settled-title idle. Three
    // batches per ROM, interleaved so a host load spike hits both ROMs alike;
    // the minimum of each ROM's batches is compared.
    let mut auth_wall = Duration::MAX;
    let mut ours_wall = Duration::MAX;
    for _ in 0..WALL_RUNS {
        auth_wall = auth_wall.min(timed_batch(&mut auth_m));
        ours_wall = ours_wall.min(timed_batch(&mut ours_m));
    }

    // Sanity: our ROM idled clean — no loud-stub breadcrumb during any of it.
    assert_eq!(
        ours_m.bus().peek(0x837D),
        0,
        "loud-stub breadcrumb >837D set: our ROM hit an unimplemented handler"
    );

    let settle_ratio = ours_settle as f64 / auth_settle as f64;
    let wall_ratio = ours_wall.as_secs_f64() / auth_wall.as_secs_f64();
    eprintln!("M7 performance parity (both ROMs on our GROM; wall = min of {WALL_RUNS} x {TIMED_FRAMES}-frame batches):");
    eprintln!("  ROM        settle(frames)  wall/{TIMED_FRAMES}f      ms/frame");
    eprintln!(
        "  authentic  {:>14}  {:>10.1?}  {:>8.4}",
        auth_settle,
        auth_wall,
        per_frame_ms(auth_wall)
    );
    eprintln!(
        "  ours       {:>14}  {:>10.1?}  {:>8.4}",
        ours_settle,
        ours_wall,
        per_frame_ms(ours_wall)
    );
    eprintln!("  ratios: settle {settle_ratio:.2}  wall-clock/frame {wall_ratio:.2}  (budget 1.25)");

    // Frames-to-settle: ours <= authentic * 1.25 + 30, in exact integer math
    // (cross-multiplied by 4: 4*ours <= 5*auth + 120).
    assert!(
        ours_settle * 4 <= auth_settle * 5 + 120,
        "frames-to-settled-title regressed past authentic x1.25 + 30: authentic={auth_settle} ours={ours_settle} (ratio {settle_ratio:.2})"
    );
    // Host wall-clock per frame: ours <= authentic * 1.25 (same frame count per
    // batch, so the batch totals compare directly).
    assert!(
        ours_wall.as_secs_f64() <= auth_wall.as_secs_f64() * 1.25,
        "host wall-clock per frame regressed past authentic x1.25: authentic={auth_wall:?} ours={ours_wall:?} per {TIMED_FRAMES} frames (ratio {wall_ratio:.2})"
    );
}
