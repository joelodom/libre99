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

//! Bus multiplexer conformance: the VDP, GROM, and sound chips sit on the **high**
//! byte of the 8-bit multiplexed bus and answer only at even addresses, so a
//! *word* access reaches each chip exactly **once** — the odd half is discarded.
//! This is why TI software drives these ports with byte instructions; getting it
//! wrong double-strobes the chip (sound) or double-advances the GROM address
//! counter (GROM), corrupting whatever the port drives. Mirrors Classic99, which
//! `return`s on `x & 1` for every device port.

use std::sync::LazyLock;

use libre99_core::bus::Bus;
use libre99_core::machine::{Machine, Tms9900Bus};

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

/// A *word* read of the GROM data port advances the address counter by **one**,
/// not two: the odd half of the access is open bus and performs no GROM read.
#[test]
fn word_read_of_grom_data_port_advances_counter_once() {
    let mut bus = Tms9900Bus::new(&[], &[]);
    bus.grom.load(0x0100, &[0x11, 0x22, 0x33, 0x44]);
    // Point the counter at >0100 the way software does: two byte writes.
    bus.write_byte(0x9C02, 0x01);
    bus.write_byte(0x9C02, 0x00);
    let before = bus.grom_address();
    let _ = bus.read_word(0x9800);
    assert_eq!(
        bus.grom_address().wrapping_sub(before),
        1,
        "a word read must auto-increment the GROM counter once, not twice"
    );
}

/// A single *word* write to the GROM address port latches only the high byte, so
/// it cannot stand in for the two byte writes that load a full 16-bit address.
#[test]
fn word_write_to_grom_address_is_not_two_byte_writes() {
    let mut by_bytes = Tms9900Bus::new(&[], &[]);
    by_bytes.write_byte(0x9C02, 0x12);
    by_bytes.write_byte(0x9C02, 0x34);

    let mut by_word = Tms9900Bus::new(&[], &[]);
    by_word.write_word(0x9C02, 0x1234);

    // The byte path completed a 16-bit address load (and prefetched); the word
    // path latched only the high byte. The counters must therefore differ — before
    // the odd-half guard they were identical (the bug).
    assert_ne!(by_bytes.grom_address(), by_word.grom_address());
}

/// A *word* write to the sound port strobes the SN76489 only once (the high
/// byte); the odd half is ignored, so it cannot smuggle a second (data) byte in.
#[test]
fn word_write_to_sound_port_reaches_psg_once() {
    let mut bus = Tms9900Bus::new(&[], &[]);
    // Latch tone channel 0 with period low-nibble 0 (>80 = `1 00 0 0000`).
    bus.write_byte(0x8400, 0x80);
    assert_eq!(bus.psg.period(0), 0);
    // Word write: the high byte >80 re-latches ch0 tone; the odd-half low byte >3F
    // is a data byte that, if it reached the chip, would set period bits 4..9.
    bus.write_word(0x8400, 0x803F);
    assert_eq!(
        bus.psg.period(0),
        0,
        "the odd half of a word write must not reach the sound chip"
    );
}

/// The hardware decodes 12 CRU address lines, so software bit addresses above
/// >0FFF alias back into the 4096-bit space — a write to bit >1012 lands on
/// bit >012 (a keyboard column-select pin), and a read of bit >1008 samples
/// bit >008 (a keyboard row).
#[test]
fn cru_bit_addresses_alias_into_the_12_bit_space() {
    use libre99_core::keyboard::TiKey;
    let mut bus = Tms9900Bus::new(&[], &[]);
    bus.keyboard.set_key(TiKey::A, true); // matrix cell (column 5, row 5)

    // Select column 5 (0b101 on P2..P4 = bits 18..20) via ALIASED addresses.
    bus.write_cru_bit(0x1012, true);
    bus.write_cru_bit(0x1013, false);
    bus.write_cru_bit(0x1014, true);

    // Row 5 reads on bit 3+5 = 8, active low — through the alias and directly.
    assert!(!bus.read_cru_bit(0x1008), "aliased row read sees the key");
    assert!(!bus.read_cru_bit(0x0008), "canonical row read agrees");
}

/// GROM accesses stall the CPU far beyond the multiplexer's 4 cycles —
/// Classic99's hardware-measured values, stacked on the mux wait exactly as
/// Classic99 stacks them. The second address byte costs more than the first
/// (it completes the address and triggers the prefetch), so the cost model
/// tracks the GROM's write-latch phase.
#[test]
fn grom_port_accesses_stall_beyond_the_multiplexer() {
    let mut bus = Tms9900Bus::new(&[], &[]);
    assert_eq!(bus.wait_states_rw(0x9800, false), 23, "data read: 4 + 19");
    assert_eq!(bus.wait_states_rw(0x9802, false), 17, "address read: 4 + 13");
    assert_eq!(bus.wait_states_rw(0x9C00, true), 26, "data write: 4 + 22");
    assert_eq!(bus.wait_states_rw(0x9C02, true), 19, "first address byte: 4 + 15");
    bus.write_byte(0x9C02, 0x12);
    assert_eq!(bus.wait_states_rw(0x9C02, true), 25, "second address byte: 4 + 21");
    bus.write_byte(0x9C02, 0x34);
    assert_eq!(bus.wait_states_rw(0x9C02, true), 19, "phase resets after a full address");
    // The odd half of a word access is open bus; everything else is unchanged.
    assert_eq!(bus.wait_states_rw(0x9801, false), 4);
    assert_eq!(bus.wait_states_rw(0x8C00, true), 4, "VDP keeps the plain mux wait");
    assert_eq!(bus.wait_states_rw(0x0000, false), 0, "console ROM is fast");
    assert_eq!(bus.wait_states_rw(0x8300, false), 0, "scratchpad is fast");
}

/// The stall reaches instruction timing through the CPU: a GROM data-port
/// read inside MOVB costs 14 (base) + 8 (symbolic operand) + 23 (port) = 45.
#[test]
fn a_grom_data_read_charges_the_stall_through_the_cpu() {
    use libre99_core::cpu::Cpu;
    let mut bus = Tms9900Bus::new(&[], &[]);
    bus.poke_word(0x8320, 0xD060); // MOVB @>9800,R1 — program in scratchpad
    bus.poke_word(0x8322, 0x9800);
    let mut cpu = Cpu::new();
    cpu.set_wp(0x8300);
    cpu.set_pc(0x8320);
    assert_eq!(cpu.step(&mut bus), 45);
}

// A *word* write to the VDP data port must land a single byte and advance the
// VRAM address only once. The 9918A hangs off the high byte of the console's data
// bus, so the odd half of a word access never reaches the chip — the bus drops
// it. (When the bus instead wrote both halves, every word write double-advanced
// the address; the disk DSR's power-up `CLR @>8C00` VRAM-clear loop then ran off
// the end of VRAM and wrapped its zeros back over the master title screen.) This
// behavior lives in the console bus, not the bare `Vdp`, so the test drives a
// `Machine` (which is why it lives here and not in the VDP unit tests).
#[test]
fn word_write_to_vdp_data_port_lands_one_byte() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut m = Machine::new(rom, grom);
    // Point the VRAM write address at >0100 (low byte, then high byte | >40).
    m.bus_mut().write_byte(0x8C02, 0x00);
    m.bus_mut().write_byte(0x8C02, 0x41);

    // One word write of >ABCD: only the high byte (>AB) is latched.
    m.bus_mut().write_word(0x8C00, 0xABCD);
    assert_eq!(m.vdp().vram(0x0100), 0xAB, "word write latches the high byte");
    assert_eq!(
        m.vdp().vram(0x0101),
        0x00,
        "the low half of the word must not be written to VRAM"
    );

    // The address advanced exactly once: the next byte write goes to >0101.
    m.bus_mut().write_byte(0x8C00, 0xEE);
    assert_eq!(
        m.vdp().vram(0x0101),
        0xEE,
        "a word access advances the VRAM address by one, not two"
    );
}
