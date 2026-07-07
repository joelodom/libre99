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

//! # TMS9901 — the CRU's programmable systems interface
//!
//! The **CRU** (Communications Register Unit) is the TMS9900's bit-serial I/O
//! bus: the CPU can set/reset/test individual numbered bits (`SBO`/`SBZ`/`TB`) or
//! shift groups of them (`LDCR`/`STCR`). The CRU bit address is `(R12 >> 1) +
//! displacement`. The **TMS9901** sits at CRU base 0 and provides three things
//! the firmware leans on constantly:
//!
//! * the **keyboard scan** — write a 3-bit column to output pins P2–P4 (CRU bits
//!   18–20), then read the eight rows back on input bits 3–10 (active low);
//! * the **interrupt controller** — the VDP's vertical-blank interrupt arrives on
//!   `/INT2` (CRU input bit 2, active low); a per-interrupt enable **mask** lets
//!   software choose which inputs reach the CPU;
//! * an **interval timer** (selected by writing CRU bit 0 = 1), used for the
//!   sound-list/cassette timing — modeled minimally here.
//!
//! ## Read vs. write are different functions of the same bit number
//!
//! Like the real chip, a given CRU bit means one thing when written and another
//! when read. Writing bit 2 in interrupt mode sets the **mask** that enables the
//! VDP interrupt; reading bit 2 returns the **state** of the `/INT2` pin. This
//! module keeps them straight: [`Tms9901::write_bit`] updates configuration,
//! [`Tms9901::read_bit`] samples live inputs (the keyboard and the VDP line).
//!
//! ## Interrupt levels on the bare console
//!
//! The 99/4A leaves the 9901's priority-encode pins unconnected, so every enabled
//! interrupt reaches the CPU as **level 1**. [`Tms9901::pending_interrupt`]
//! therefore answers a single yes/no question — "should the CPU take its level-1
//! interrupt?" — which the machine wiring feeds to the CPU each step.

use crate::keyboard::Keyboard;

/// The TMS9901 interface chip's state relevant to the console: the selected
/// keyboard column, the interrupt-enable mask, the timer/I-O mode flag, and the
/// alpha-lock output.
#[derive(Default)]
pub struct Tms9901 {
    /// Currently selected keyboard column (0–7), from output bits P2–P4.
    column: u8,
    /// Interrupt-enable mask: bit *n* enables interrupt input `/INTn`. The
    /// console enables bit 2 (the VDP).
    int_mask: u16,
    /// CRU bit 0: `true` selects timer mode, `false` selects I/O (interrupt)
    /// mode. The console writes mask bits only in I/O mode.
    timer_mode: bool,
    /// Alpha-lock output latch (P5). Tracked for completeness; not otherwise used.
    alpha_lock: bool,
}

impl Tms9901 {
    pub fn new() -> Self {
        Tms9901::default()
    }

    /// The currently selected keyboard column.
    pub fn selected_column(&self) -> u8 {
        self.column
    }

    /// Drive a CRU output bit (an `SBO`/`SBZ`, or one bit of an `LDCR`).
    pub fn write_bit(&mut self, bit: u16, value: bool) {
        match bit {
            // Bit 0 selects timer (1) vs. I/O / interrupt (0) mode.
            0 => self.timer_mode = value,
            // Bits 1–15 are the interrupt-enable mask, but only in I/O mode (in
            // timer mode these same bits load the interval timer, which we don't
            // model — so they fall through to the catch-all and are ignored).
            1..=15 if !self.timer_mode => {
                let m = 1u16 << bit;
                if value {
                    self.int_mask |= m;
                } else {
                    self.int_mask &= !m;
                }
            }
            // Bits 18–20 (pins P2–P4) are the 3-bit keyboard column select,
            // P2 the least-significant bit.
            18..=20 => {
                let b = 1u8 << (bit - 18);
                if value {
                    self.column |= b;
                } else {
                    self.column &= !b;
                }
            }
            // Bit 21 (pin P5) is the alpha-lock select.
            21 => self.alpha_lock = value,
            // Other P-port outputs (cassette motor/level, etc.) are not wired.
            // Decision (Joel, 2026-07-02): cassette stays unemulated for now — the
            // console-ROM rewrite ships interface-correct CS1/CS2 error stubs only,
            // and its tape transport is deferred until these bits (plus the 9901
            // interval timer and a tape source) exist. See docs/ROADMAP.md §6 and
            // original-content/system-roms/history/ROM-REWRITE-PLAN.md §10.2.
            _ => {}
        }
    }

    /// Sample a CRU input bit (a `TB`, or one bit of an `STCR`). Needs the live
    /// keyboard state and the current VDP interrupt line.
    ///
    /// CRU inputs idle **high**; pressed keys and asserted interrupts read low.
    pub fn read_bit(&self, bit: u16, keyboard: &Keyboard, vdp_int: bool) -> bool {
        match bit {
            // /INT2 = the VDP vertical-blank interrupt (active low).
            2 => !vdp_int,
            // Bits 3–10 read the eight keyboard rows of the selected column
            // (active low: pressed = 0).
            3..=10 => !keyboard.is_pressed(self.column as usize, (bit - 3) as usize),
            // Everything else idles high.
            _ => true,
        }
    }

    /// Should the CPU take its level-1 interrupt right now? True when the VDP
    /// interrupt is asserted *and* enabled in the mask (bit 2). (Peripheral-card
    /// interrupts on `/INT1` would also count, but the bundled cards poll rather
    /// than interrupt.)
    pub fn pending_interrupt(&self, vdp_int: bool) -> bool {
        (self.int_mask & (1 << 2)) != 0 && vdp_int
    }

    /// Is the VDP vertical-blank interrupt (mask bit 2) currently enabled?
    /// (Diagnostics: the console ROM sets this at power-up; a rewritten GROM that
    /// never triggers that path leaves the VBLANK ISR — and hence sound, QUIT,
    /// sprite motion — dead.)
    pub fn vdp_interrupt_enabled(&self) -> bool {
        (self.int_mask & (1 << 2)) != 0
    }

    /// The raw interrupt-enable mask (diagnostics).
    pub fn int_mask(&self) -> u16 {
        self.int_mask
    }

    /// Serialize the 9901's latched state into a save state.
    pub(crate) fn save_state(&self, w: &mut crate::state::StateWriter) {
        w.u8(self.column);
        w.u16(self.int_mask);
        w.bool(self.timer_mode);
        w.bool(self.alpha_lock);
    }

    /// Restore the 9901's latched state from a save state.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
    ) -> Result<(), crate::state::StateError> {
        self.column = r.u8()?;
        self.int_mask = r.u16()?;
        self.timer_mode = r.bool()?;
        self.alpha_lock = r.bool()?;
        Ok(())
    }
}
