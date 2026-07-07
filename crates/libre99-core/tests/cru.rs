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

//! TMS9901 + keyboard-matrix conformance tests.
//!
//! The CRU (Communications Register Unit) is the TI's bit-serial I/O bus. The
//! TMS9901 chip on it scans the keyboard, delivers the VDP interrupt, and runs an
//! interval timer. These tests pin down the parts the firmware depends on: the
//! keyboard column/row addressing (active low), the VDP interrupt input and mask,
//! and the matrix layout.

use libre99_core::cru::Tms9901;
use libre99_core::keyboard::{Keyboard, TiKey};

#[test]
fn keyboard_matrix_positions_match_hardware() {
    let mut kb = Keyboard::new();
    // A handful of well-known cells from the 99/4A matrix (column, row).
    kb.set_key(TiKey::A, true);
    assert!(kb.is_pressed(5, 5), "A is column 5, row 5");
    kb.set_key(TiKey::Enter, true);
    assert!(kb.is_pressed(0, 2), "ENTER is column 0, row 2");
    kb.set_key(TiKey::Equals, true);
    assert!(kb.is_pressed(0, 0), "= is column 0, row 0");
    kb.set_key(TiKey::Num8, true);
    assert!(kb.is_pressed(2, 3), "8 is column 2, row 3");
    kb.set_key(TiKey::A, false);
    assert!(!kb.is_pressed(5, 5), "release clears the cell");
}

#[test]
fn column_select_via_p2_p4() {
    let mut t = Tms9901::new();
    // Column number is written to CRU output bits 18,19,20 (P2=LSB..P4=MSB).
    // Select column 5 = 0b101.
    t.write_bit(18, true);
    t.write_bit(19, false);
    t.write_bit(20, true);
    assert_eq!(t.selected_column(), 5);
}

#[test]
fn keyboard_rows_read_active_low_on_bits_3_to_10() {
    let mut t = Tms9901::new();
    let mut kb = Keyboard::new();
    // Select column 2 = 0b010.
    t.write_bit(18, false);
    t.write_bit(19, true);
    t.write_bit(20, false);
    assert_eq!(t.selected_column(), 2);
    // Nothing pressed: every row reads idle-high (true).
    assert!(t.read_bit(3, &kb, false), "row 0 idle high");
    assert!(t.read_bit(6, &kb, false), "row 3 idle high");
    // '8' is column 2, row 3 — read on CRU input bit 6 (= 3 + row).
    kb.set_key(TiKey::Num8, true);
    assert!(!t.read_bit(6, &kb, false), "pressed key reads active-low");
    assert!(t.read_bit(3, &kb, false), "a different row stays high");
}

#[test]
fn vdp_interrupt_input_is_bit2_active_low() {
    let t = Tms9901::new();
    let kb = Keyboard::new();
    assert!(t.read_bit(2, &kb, false), "no VDP interrupt -> high");
    assert!(!t.read_bit(2, &kb, true), "VDP interrupt asserted -> low");
}

#[test]
fn vdp_interrupt_mask_gates_the_cpu_request() {
    let mut t = Tms9901::new();
    t.write_bit(0, false); // select I/O (interrupt) mode
    assert!(
        !t.pending_interrupt(true),
        "mask not set yet, so no CPU interrupt"
    );
    t.write_bit(2, true); // enable /INT2 (the VDP)
    assert!(t.pending_interrupt(true), "enabled + active -> CPU interrupt");
    assert!(!t.pending_interrupt(false), "enabled but inactive -> none");
    t.write_bit(2, false); // disable again
    assert!(!t.pending_interrupt(true), "disabled -> none");
}
