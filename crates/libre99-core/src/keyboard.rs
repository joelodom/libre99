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

//! # The TI-99/4A keyboard matrix
//!
//! The keyboard is a passive 8×8 switch matrix scanned over the CRU by the
//! TMS9901 (see [`crate::cru`]). Software selects a **column** (0–7) by writing
//! three CRU output bits, then reads eight **rows** back on CRU input bits 3–10.
//! A pressed key pulls its row **low**, so the firmware sees a pressed key as a
//! `0` (active low). Columns 0–5 are the keyboard proper; columns 6 and 7 are the
//! two joystick ports, which share the same row lines.
//!
//! ## The full matrix (column × row → key)
//!
//! Row *r* is read on CRU input bit `3 + r`. `FCTN`, `SHIFT`, and `CTRL` are
//! ordinary matrix cells in column 0 — software reads them like any other key and
//! combines them itself.
//!
//! ```text
//!        Col0    Col1  Col2  Col3  Col4  Col5    Col6(Joy1)  Col7(Joy2)
//! Row0    =       .     ,     M     N     /       FIRE        FIRE
//! Row1   SPACE    L     K     J     H     ;       LEFT        LEFT
//! Row2   ENTER    O     I     U     Y     P       RIGHT       RIGHT
//! Row3    —       9     8     7     6     0       DOWN        DOWN
//! Row4   FCTN     2     3     4     5     1       UP          UP
//! Row5   SHIFT    S     D     F     G     A        —           —
//! Row6   CTRL     W     E     R     T     Q        —           —
//! Row7    —       X     C     V     B     Z        —           —
//! ```
//!
//! This module models the matrix as pure state — which switches are closed — and
//! the host frontend maps PC keys onto [`TiKey`] values. Modifier glyphs (e.g.
//! the symbol printed above a number, reached with `FCTN`/`SHIFT`) are produced
//! by the firmware from the raw key plus the modifier cell, exactly as on real
//! hardware; we only report which physical switches are down.

/// A physical key (or joystick direction/button) on the TI-99/4A.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TiKey {
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    Equals, Period, Comma, Semicolon, Slash, Space, Enter,
    /// The `FCTN` (function) modifier — column 0, row 4.
    Fctn,
    /// The `SHIFT` modifier — column 0, row 5.
    Shift,
    /// The `CTRL` modifier — column 0, row 6.
    Ctrl,
    Joy1Fire, Joy1Left, Joy1Right, Joy1Down, Joy1Up,
    Joy2Fire, Joy2Left, Joy2Right, Joy2Down, Joy2Up,
}

impl TiKey {
    /// The `(column, row)` cell this key occupies in the scan matrix.
    pub fn position(self) -> (usize, usize) {
        use TiKey::*;
        match self {
            // Column 0 — punctuation/control column.
            Equals => (0, 0), Space => (0, 1), Enter => (0, 2),
            Fctn => (0, 4), Shift => (0, 5), Ctrl => (0, 6),
            // Column 1.
            Period => (1, 0), L => (1, 1), O => (1, 2), Num9 => (1, 3),
            Num2 => (1, 4), S => (1, 5), W => (1, 6), X => (1, 7),
            // Column 2.
            Comma => (2, 0), K => (2, 1), I => (2, 2), Num8 => (2, 3),
            Num3 => (2, 4), D => (2, 5), E => (2, 6), C => (2, 7),
            // Column 3.
            M => (3, 0), J => (3, 1), U => (3, 2), Num7 => (3, 3),
            Num4 => (3, 4), F => (3, 5), R => (3, 6), V => (3, 7),
            // Column 4.
            N => (4, 0), H => (4, 1), Y => (4, 2), Num6 => (4, 3),
            Num5 => (4, 4), G => (4, 5), T => (4, 6), B => (4, 7),
            // Column 5.
            Slash => (5, 0), Semicolon => (5, 1), P => (5, 2), Num0 => (5, 3),
            Num1 => (5, 4), A => (5, 5), Q => (5, 6), Z => (5, 7),
            // Column 6 — joystick 1.
            Joy1Fire => (6, 0), Joy1Left => (6, 1), Joy1Right => (6, 2),
            Joy1Down => (6, 3), Joy1Up => (6, 4),
            // Column 7 — joystick 2.
            Joy2Fire => (7, 0), Joy2Left => (7, 1), Joy2Right => (7, 2),
            Joy2Down => (7, 3), Joy2Up => (7, 4),
        }
    }
}

/// The state of the 8×8 key-switch matrix (`true` = the switch is closed/pressed).
/// Indexed `[column][row]`.
#[derive(Default)]
pub struct Keyboard {
    state: [[bool; 8]; 8],
}

impl Keyboard {
    pub fn new() -> Self {
        Keyboard {
            state: [[false; 8]; 8],
        }
    }

    /// Press (`down = true`) or release a key.
    pub fn set_key(&mut self, key: TiKey, down: bool) {
        let (col, row) = key.position();
        self.state[col][row] = down;
    }

    /// Is the switch at `(col, row)` currently closed?
    pub fn is_pressed(&self, col: usize, row: usize) -> bool {
        self.state[col & 7][row & 7]
    }

    /// Release every key (used on focus loss so keys don't "stick").
    pub fn release_all(&mut self) {
        self.state = [[false; 8]; 8];
    }

    /// Serialize the switch matrix (one byte per switch) into a save state.
    pub(crate) fn save_state(&self, w: &mut crate::state::StateWriter) {
        for col in &self.state {
            for &pressed in col {
                w.bool(pressed);
            }
        }
    }

    /// Restore the switch matrix from a save state.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
    ) -> Result<(), crate::state::StateError> {
        for col in &mut self.state {
            for pressed in col.iter_mut() {
                *pressed = r.bool()?;
            }
        }
        Ok(())
    }
}
