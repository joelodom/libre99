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

//! Beam-accuracy conformance: mid-frame VRAM writes must appear exactly where
//! the beam is, and the frame interrupt must rise at the end of active
//! display — the hardware contract that games like Parsec rely on when they
//! draw flashing text from their vblank handlers. These tests pin the model
//! that fixed the Parsec "PRESS FIRE TO BEGIN" garble: the whole-frame
//! renderer sampled VRAM at the slice boundary (an arbitrary point in the
//! game's loop) and raised vblank at the slice *start*, so it kept catching
//! name tables mid-update.
//!
//! The machine-level tests hand-assemble a tiny TMS9900 program into a custom
//! console ROM (`Machine::new` takes any ROM image; the reset vector at
//! `>0000` points WP/PC at it) — the program syncs to the frame flag by
//! polling the real status port, waits a counted number of cycles, and
//! rewrites VRAM through the real VDP ports, so the whole CPU→bus→VDP timing
//! path is exercised, wait states and all.

use libre99_core::machine::Machine;
use libre99_core::vdp::{Vdp, HEIGHT, WIDTH};

const WHITE: u32 = 0x00FF_FFFF; // palette 15
const DARK_BLUE: u32 = 0x0054_55ED; // palette 4

/// Write VDP register `reg` with `val` (low byte first, then `0x80|reg`).
fn write_reg(v: &mut Vdp, reg: u8, val: u8) {
    v.write_control(val);
    v.write_control(0x80 | reg);
}

/// Graphics I stage shared by every test here: name table `>0000` (all char
/// 0), pattern table `>0800` (char 0 = all-zero rows), color group 0 =
/// white-on-dark-blue, an empty sprite list at `>0700`. Every pixel renders
/// dark blue until char 0's pattern is rewritten to `>FF`, when every pixel
/// flips white — so "old vs new" is visible on any scanline.
fn stage(v: &mut Vdp) {
    write_reg(v, 0, 0x00);
    write_reg(v, 1, 0x40); // display on
    write_reg(v, 2, 0x00); // name table @ >0000
    write_reg(v, 3, 0x10); // color table @ >0400
    write_reg(v, 4, 0x01); // pattern table @ >0800
    write_reg(v, 5, 0x0E); // sprite attributes @ >0700
    write_reg(v, 7, 0x01); // backdrop black
    v.set_vram(0x0400, 0xF4); // group 0: fg white, bg dark blue
    v.set_vram(0x0700, 0xD0); // empty sprite list
}

/// A bare `Vdp` driven per line: a pattern rewrite between lines 99 and 100
/// shows on line 100 and below, never above — the beam does not rewrite
/// history.
#[test]
fn render_line_shows_a_mid_frame_write_from_the_next_line_down() {
    let mut v = Vdp::new();
    stage(&mut v);

    for y in 0..100 {
        v.render_line(y);
    }
    // Mid-frame: char 0's pattern flips from blank to solid.
    for row in 0..8 {
        v.set_vram(0x0800 + row, 0xFF);
    }
    for y in 100..HEIGHT {
        v.render_line(y);
    }

    let mut fb = vec![0u32; WIDTH * HEIGHT];
    v.copy_frame(&mut fb);
    assert_eq!(fb[50 * WIDTH], DARK_BLUE, "above the write: old pixels stay");
    assert_eq!(fb[150 * WIDTH], WHITE, "below the write: new pixels show");
}

/// Build a console ROM whose reset vector runs this program (WP `>8300`):
///
/// ```text
/// POLL: MOVB @>8802,R1    ; read VDP status (clears F when seen)
///       JLT WAIT          ; F (bit >80) makes the byte negative
///       JMP POLL
/// WAIT: LI   R2,wait      ; counted delay: 20 cycles per iteration
/// LOOP: DEC  R2
///       JNE  LOOP
///       LI   R3,>FF00     ; data byte >FF (ports latch the high byte)
///       LI   R4,>0000     ; VRAM address >0800 for writing:
///       LI   R5,>4800     ;   low >00, then >08 | >40
///       MOVB R4,@>8C02
///       MOVB R5,@>8C02
///       LI   R6,8
/// DATA: MOVB R3,@>8C00    ; char 0's eight pattern rows := >FF
///       DEC  R6
///       JNE  DATA
///       JMP  $
/// ```
///
/// Synced to the F edge, `wait` iterations of 20 cycles place the rewrite at
/// a chosen beam line: F rises at line 192, and the beam re-enters active
/// display 70 lines (~13,360 cycles) later.
fn rewrite_rom(wait: u16) -> Vec<u8> {
    let words: &[u16] = &[
        0x8300, 0x0010, // reset vector: WP >8300, PC >0010
        // >0010
        0xD060, 0x8802, // MOVB @>8802,R1
        0x1101, // JLT >0018
        0x10FC, // JMP >0010
        0x0202, wait, // LI R2,wait
        0x0602, // DEC R2
        0x16FE, // JNE >001C
        0x0203, 0xFF00, // LI R3,>FF00
        0x0204, 0x0000, // LI R4,>0000
        0x0205, 0x4800, // LI R5,>4800
        0xD804, 0x8C02, // MOVB R4,@>8C02
        0xD805, 0x8C02, // MOVB R5,@>8C02
        0x0206, 0x0008, // LI R6,8
        0xD803, 0x8C00, // MOVB R3,@>8C00
        0x0606, // DEC R6
        0x16FC, // JNE >0038
        0x10FF, // JMP $
    ];
    let mut rom = vec![0u8; 0x60];
    // The program starts at >0010; the vector words sit at >0000/>0002.
    let mut addr = 0usize;
    for (i, &w) in words.iter().enumerate() {
        if i == 2 {
            addr = 0x10;
        }
        rom[addr] = (w >> 8) as u8;
        rom[addr + 1] = w as u8;
        addr += 2;
    }
    rom
}

/// Run `frames` full frames and return each rendered frame's first-column
/// pixel at `y_probe` — the cheap signature of old (dark blue) vs new (white).
fn run_and_probe(wait: u16, frames: usize) -> Vec<Vec<u32>> {
    let mut m = Machine::new(&rewrite_rom(wait), &[]);
    stage(&mut m.bus_mut().vdp);
    let mut fb = vec![0u32; WIDTH * HEIGHT];
    m.render(&mut fb); // first render: switches the machine to beam accumulation
    let mut shots = Vec::new();
    for _ in 0..frames {
        m.run_frame();
        m.render(&mut fb);
        shots.push(fb.clone());
    }
    shots
}

/// The Parsec regression class, mid-frame edition: a VRAM rewrite landing at
/// ~line 96 splits that frame at the beam — lines already drawn keep the old
/// picture, lines still to come show the new one — and the next frame is
/// uniformly new. The write is timed from the F edge: 70 vblank lines plus
/// 96 active lines ≈ 166 × 190.84 cycles ≈ 31,680 ≈ 1,584 20-cycle waits.
/// (On the old whole-frame renderer this frame could never split.)
#[test]
fn a_mid_frame_rewrite_splits_the_frame_at_the_beam() {
    let shots = run_and_probe(1584, 3);
    // Frame 1: F is seen at line 192, after the whole frame drew old.
    assert_eq!(shots[0][8 * WIDTH], DARK_BLUE, "frame 1 is entirely old");
    assert_eq!(shots[0][184 * WIDTH], DARK_BLUE, "frame 1 is entirely old");
    // Frame 2: the rewrite lands ~line 96..99. Assert with a generous margin
    // (±30 lines) — the split's existence is the contract, not its exact row.
    assert_eq!(shots[1][64 * WIDTH], DARK_BLUE, "frame 2, above the beam: old");
    assert_eq!(shots[1][128 * WIDTH], WHITE, "frame 2, below the beam: new");
    // Frame 3: the beam has repainted everything.
    assert_eq!(shots[2][8 * WIDTH], WHITE, "frame 3 is entirely new");
}

/// The Parsec contract itself: a rewrite performed **during vblank** (right
/// after the F edge) is invisible mid-update — the frame that just finished
/// is uniformly old, the next frame uniformly new, and no frame ever shows a
/// half-written screen.
#[test]
fn a_vblank_rewrite_renders_clean_on_the_next_frame() {
    let shots = run_and_probe(1, 2);
    for y in [0usize, 64, 128, 191] {
        assert_eq!(shots[0][y * WIDTH], DARK_BLUE, "frame 1 uniformly old (y={y})");
        assert_eq!(shots[1][y * WIDTH], WHITE, "frame 2 uniformly new (y={y})");
    }
}

/// The frame flag must rise at the **end of active display** (~73% into the
/// frame), not at the start of the machine's frame slice. Two identical poll
/// loops count iterations: R1 from program start to the first F edge, R2
/// between the first and second F edges (one full frame period). Their ratio
/// is cycle-cost-free: 192/262 ≈ 0.73. The old model set F at slice start,
/// which would make the first count ~1.
#[test]
fn frame_flag_rises_at_the_end_of_active_display() {
    let words: &[u16] = &[
        0x8300, 0x0010, // reset vector
        // >0010  phase 1: count to the first F edge in R1
        0x0581, // INC R1
        0xD260, 0x8802, // MOVB @>8802,R9
        0x1101, // JLT >001A
        0x10FB, // JMP >0010
        // >001A  phase 2: count one full F-to-F period in R2
        0x0582, // INC R2
        0xD260, 0x8802, // MOVB @>8802,R9
        0x1101, // JLT >0024
        0x10FB, // JMP >001A
        // >0024  park the counts where the harness can read them
        0xC801, 0x8320, // MOV R1,@>8320
        0xC802, 0x8322, // MOV R2,@>8322
        0x10FF, // JMP $
    ];
    let mut rom = vec![0u8; 0x60];
    let mut addr = 0usize;
    for (i, &w) in words.iter().enumerate() {
        if i == 2 {
            addr = 0x10;
        }
        rom[addr] = (w >> 8) as u8;
        rom[addr + 1] = w as u8;
        addr += 2;
    }
    let mut m = Machine::new(&rom, &[]);
    stage(&mut m.bus_mut().vdp);
    // Two frames: the first F edge lands in frame 1, the second in frame 2.
    m.run_frame();
    m.run_frame();
    let c1 = m.bus().peek_word(0x8320) as u32;
    let c2 = m.bus().peek_word(0x8322) as u32;
    assert!(c2 > 0, "the program must have seen two F edges");
    assert!(
        c1 > 100,
        "F rose almost immediately (count {c1}) — the slice-start vblank bug"
    );
    let percent = 100 * c1 / c2;
    assert!(
        (65..=80).contains(&percent),
        "first F edge at {percent}% of a frame period (counts {c1}/{c2}); \
         the beam model puts it at 192/262 = 73%"
    );
}
