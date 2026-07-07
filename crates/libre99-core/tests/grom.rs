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

//! GROM (TMC0430) conformance tests.
//!
//! The GROM is a serial, auto-incrementing ROM. The tricky part — and the reason
//! these tests exist — is the **prefetch**: setting the address performs an
//! automatic dummy read so that the *first* data read returns the byte at the
//! address you set, even though the internal counter has already advanced. Get
//! the ordering wrong and every GROM read is off by one, which would break the
//! GPL interpreter and thus the whole machine.

use libre99_core::grom::Grom;

/// Set the 16-bit GROM address the way software does: write the HIGH byte then
/// the LOW byte to the address port.
fn set_address(g: &mut Grom, a: u16) {
    g.write_address((a >> 8) as u8);
    g.write_address(a as u8);
}

#[test]
fn first_read_returns_byte_at_the_set_address() {
    let mut g = Grom::new();
    g.load(0x0000, &[0x11, 0x22, 0x33, 0x44]);
    set_address(&mut g, 0x0000);
    // Despite the prefetch having advanced the counter, the first read must
    // return mem[0], then mem[1], mem[2], ...
    assert_eq!(g.read_data(), 0x11);
    assert_eq!(g.read_data(), 0x22);
    assert_eq!(g.read_data(), 0x33);
    assert_eq!(g.read_data(), 0x44);
}

#[test]
fn read_from_arbitrary_address() {
    let mut g = Grom::new();
    g.load(0x1234, &[0xDE, 0xAD]);
    set_address(&mut g, 0x1234);
    assert_eq!(g.read_data(), 0xDE);
    assert_eq!(g.read_data(), 0xAD);
}

#[test]
fn address_readback_reflects_the_prefetch_increment_high_byte_first() {
    let mut g = Grom::new();
    set_address(&mut g, 0x1234);
    // After setting A the prefetch has bumped the counter to A+1; the read-back
    // returns that, high byte first.
    assert_eq!(g.read_address(), 0x12);
    assert_eq!(g.read_address(), 0x35);
}

#[test]
fn autoincrement_wraps_within_the_8k_slot() {
    let mut g = Grom::new();
    g.load(0x1FFF, &[0x99]); // last byte of slot 0
    g.load(0x0000, &[0x77]); // first byte of slot 0
    set_address(&mut g, 0x1FFF);
    assert_eq!(g.read_data(), 0x99);
    // The counter must wrap to >0000 (staying in slot 0), NOT advance to >2000.
    assert_eq!(g.read_data(), 0x77);
}

#[test]
fn cartridge_groms_live_at_6000_and_up() {
    let mut g = Grom::new();
    g.load(0x6000, &[0xAB, 0xCD]); // GROM 3 = first cartridge GROM
    set_address(&mut g, 0x6000);
    assert_eq!(g.read_data(), 0xAB);
    assert_eq!(g.read_data(), 0xCD);
}

#[test]
fn writes_to_mask_rom_groms_are_ignored() {
    let mut g = Grom::new();
    g.load(0x0000, &[0x11]);
    set_address(&mut g, 0x0000);
    g.write_data(0xFF); // a real mask-ROM GROM ignores data writes
    set_address(&mut g, 0x0000);
    assert_eq!(g.read_data(), 0x11);
}

#[test]
fn reading_data_resets_the_address_byte_phase() {
    // If only a single (high) address byte was written and then a data read
    // occurs, the chip's high/low phase is reset, so the next address pair is
    // still interpreted high-then-low.
    let mut g = Grom::new();
    g.load(0x4000, &[0x5A]);
    g.write_address(0x40); // stray high byte
    let _ = g.read_data(); // resets the phase
    set_address(&mut g, 0x4000);
    assert_eq!(g.read_data(), 0x5A);
}

#[test]
fn reading_address_does_not_corrupt_the_next_address_write() {
    // The GPL interpreter reads the address port (e.g. a GPL branch reads it to
    // recover the current GROM slot) and then writes a brand-new address. A
    // single (odd) address read must NOT leave the chip half-way through an
    // address-write pair — the following high/low write pair must still be
    // interpreted high-then-low, landing on the address that was written.
    let mut g = Grom::new();
    g.load(0x2000, &[0xAB]);
    set_address(&mut g, 0x0100); // some current address
    let _ = g.read_address(); // read ONE byte (the high byte) of the counter
    set_address(&mut g, 0x2000); // now point somewhere new
    assert_eq!(g.read_data(), 0xAB, "first read must return mem[>2000]");
}
