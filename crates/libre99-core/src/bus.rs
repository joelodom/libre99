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

//! The system bus: the seam between the [`crate::cpu::Cpu`] and everything else.
//!
//! # What the TMS9900 can do to the outside world
//!
//! The TMS9900 has exactly two ways to touch the world beyond its own registers:
//!
//! 1. **The memory bus** — 16-bit, word-addressed, big-endian. The CPU reads and
//!    writes 16-bit words. Byte instructions (`MOVB`, `AB`, …) are built on top of
//!    word accesses by the CPU itself (read the containing word, replace the
//!    selected byte, write it back), because the physical bus only moves words.
//!
//! 2. **The CRU** (Communications Register Unit) — a *bit-serial* I/O bus. The CPU
//!    can set, reset, or test a single addressed bit, or transfer 1–16 bits at a
//!    time (`LDCR`/`STCR`). The keyboard, the 9901 interrupt/timer chip, and every
//!    peripheral card (including the disk controller) are wired to the CRU.
//!
//! The [`Bus`] trait below is exactly those two capabilities, plus a hook for
//! wait-state accounting. The CPU depends only on this trait, so it can be tested
//! against [`FlatRam`] with no other hardware present.
//!
//! # The TI-99/4A console memory map (implemented by `Tms9900Bus` in `machine.rs`)
//!
//! ```text
//! >0000–1FFF  Console system ROM (8 KiB)                     16-bit, 0 wait
//! >2000–3FFF  Low  RAM expansion (32K option, 8 KiB)          8-bit, +waits
//! >4000–5FFF  Peripheral DSR ROM window (paged in by CRU)     8-bit
//!               >5FF0–5FFE  TI Disk Controller FD1771 regs (data inverted)
//! >6000–7FFF  Cartridge ROM window (8 KiB, optionally banked) 8-bit
//! >8000–83FF  Scratchpad RAM (256 bytes at >8300, mirrored)  16-bit, 0 wait
//!               This fast RAM is where CPU workspaces normally live.
//! >8400       SN76489 sound chip (write-only)                 8-bit
//! >8800       VDP read data        >8802  VDP read status      8-bit
//! >8C00       VDP write data       >8C02  VDP write addr/reg   8-bit
//! >9000/>9002 Speech read          >9400  Speech write         8-bit (not emulated)
//! >9800       GROM read data       >9802  GROM read address    8-bit
//! >9C00       GROM write data      >9C02  GROM write address   8-bit
//! >A000–FFFF  High RAM expansion (32K option, 24 KiB)         8-bit, +waits
//! ```
//!
//! Two important timing facts (see [`Bus::wait_states`]):
//! * The console ROM (`>0000–1FFF`) and the 256-byte scratchpad RAM
//!   (`>8000–83FF`) sit on the CPU's native 16-bit bus and are accessed with **no
//!   wait states**.
//! * Everything else is reached through a multiplexer that converts the CPU's
//!   16-bit accesses into pairs of 8-bit accesses, which **stretches each access
//!   by ~4 clock cycles**. This is why VDP/GROM/cartridge/sound code runs visibly
//!   slower than code in scratchpad RAM, and why we model wait states at all.
//!
//! # The CRU address space
//!
//! `SBO`/`SBZ`/`TB` and `LDCR`/`STCR` form a *bit address* from register R12:
//! the CRU bit address is `(R12 >> 1) + displacement`. (R12 holds twice the bit
//! address; only its bits 1..15 matter — the hardware decodes 12 address lines,
//! so there are 4096 CRU bits.) The console wires:
//!
//! ```text
//! bit >0000–001F  TMS9901: keyboard scan, VDP interrupt input, interval timer
//! bit >0800       PEB card slot at CRU base >1000 (R12 = >1000)
//! bit >0880       TI Disk Controller card (CRU base >1100, R12 = >1100)
//! …               further cards at >1200,>1300,… (bit >0900,>0980,…)
//! ```
//!
//! (Recall: "CRU base >1100" is the *R12 value*; the bit address is half of it,
//! `>1100 >> 1 = >0880`.)

/// The interface the CPU uses to reach memory and the CRU.
///
/// Implementors:
/// * [`FlatRam`] — 64 KiB of flat RAM + 4096 CRU bits, no wait states. Used by
///   the CPU unit/integration tests so the processor can be exercised in
///   isolation.
/// * [`crate::machine::Tms9900Bus`] — the real console memory map and CRU
///   routing, owning the VDP/GROM/PSG/CRU/cartridge/disk.
///
/// All addresses are the raw 16-bit values the CPU computes. Implementors are
/// responsible for any masking (e.g. words ignore address bit 0; RAM mirrors).
pub trait Bus {
    /// Read a 16-bit word. `addr` bit 0 is ignored (words are even-aligned); the
    /// returned value is big-endian (the byte at the even address is the MSB).
    fn read_word(&mut self, addr: u16) -> u16;

    /// Write a 16-bit word. `addr` bit 0 is ignored.
    fn write_word(&mut self, addr: u16, value: u16);

    /// Read a single byte (the high byte at an even address, the low byte at an
    /// odd one). The default expresses a byte read in terms of the word access,
    /// which is correct for RAM-like buses. The real console bus overrides this
    /// because its 8-bit-mapped device ports (VDP, GROM, sound) respond to a byte
    /// access as a **single** transfer — unlike a word access, which the bus
    /// hardware splits into two byte transfers to the same port.
    fn read_byte(&mut self, addr: u16) -> u8 {
        let w = self.read_word(addr & 0xFFFE);
        if addr & 1 == 0 {
            (w >> 8) as u8
        } else {
            (w & 0xFF) as u8
        }
    }

    /// Write a single byte (read-modify-write over the containing word by
    /// default). Overridden by the console bus for its device ports, where a byte
    /// write is one transfer and must not be expanded into the two-transfer
    /// sequence a word write implies.
    fn write_byte(&mut self, addr: u16, value: u8) {
        let w = self.read_word(addr & 0xFFFE);
        let nw = if addr & 1 == 0 {
            (w & 0x00FF) | ((value as u16) << 8)
        } else {
            (w & 0xFF00) | (value as u16)
        };
        self.write_word(addr & 0xFFFE, nw);
    }

    /// Read one CRU bit at the given *bit address* (`(R12 >> 1) + displacement`).
    /// Returns `true` for a 1 bit. Inputs that are not driven read back as `true`
    /// on real hardware (the CRU input lines idle high); test buses may differ.
    fn read_cru_bit(&mut self, bit_addr: u16) -> bool;

    /// Write one CRU bit at the given bit address.
    fn write_cru_bit(&mut self, bit_addr: u16, value: bool);

    /// Number of extra clock cycles ("wait states") that an access to `addr`
    /// costs beyond the CPU's datasheet timing (which assumes 0-wait memory).
    ///
    /// The real console returns ~4 for the 8-bit-multiplexed regions and 0 for
    /// console ROM and scratchpad RAM. The default is 0, which is correct for a
    /// flat-RAM test bus where all memory is "fast".
    fn wait_states(&self, _addr: u16) -> u32 {
        0
    }

    /// Like [`Bus::wait_states`], but told whether the access is a write —
    /// some device ports stall the CPU differently per operation (the GROM
    /// ports most of all: its serial fetch adds 13–22 cycles on top of the
    /// multiplexer, measured on hardware). The CPU calls this for every
    /// access; the default forwards to the direction-blind [`Bus::wait_states`],
    /// which is correct for plain memory.
    fn wait_states_rw(&self, addr: u16, _is_write: bool) -> u32 {
        self.wait_states(addr)
    }
}

/// A 64 KiB flat-RAM bus with 4096 CRU bits and no wait states.
///
/// This is the harness the CPU is tested against: every address is plain
/// readable/writable RAM, so a test can poke an instruction stream and initial
/// register values into memory, step the CPU, and read the results straight back
/// out. It deliberately models nothing about the real console — that is the whole
/// point of testing the processor in isolation.
pub struct FlatRam {
    /// 64 KiB, byte-addressed, big-endian word access.
    mem: Box<[u8; 0x1_0000]>,
    /// 4096 CRU bits (the full 12-bit CRU address space).
    cru: Box<[bool; 0x1000]>,
}

impl Default for FlatRam {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatRam {
    /// A zero-initialized flat memory with all CRU input bits low.
    pub fn new() -> Self {
        FlatRam {
            mem: Box::new([0u8; 0x1_0000]),
            cru: Box::new([false; 0x1000]),
        }
    }

    /// Load `bytes` into memory starting at `addr` (wrapping at 64 KiB). Handy
    /// for placing an instruction stream or a data table in a test.
    pub fn load(&mut self, addr: u16, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            self.mem[addr.wrapping_add(i as u16) as usize] = b;
        }
    }

    /// Load a slice of big-endian 16-bit words starting at `addr`. Most CPU tests
    /// describe programs as words (the natural unit of TMS9900 encoding), so this
    /// keeps them readable.
    pub fn load_words(&mut self, addr: u16, words: &[u16]) {
        for (i, &w) in words.iter().enumerate() {
            self.write_word(addr.wrapping_add((i as u16) * 2), w);
        }
    }

    /// Read a byte directly (test inspection helper).
    pub fn peek(&self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }

    /// Directly set a CRU input bit the program will read with `TB`/`STCR`.
    pub fn set_cru_input(&mut self, bit_addr: u16, value: bool) {
        self.cru[(bit_addr & 0x0FFF) as usize] = value;
    }

    /// Read back a CRU bit the program drove with `SBO`/`SBZ`/`LDCR`.
    pub fn get_cru(&self, bit_addr: u16) -> bool {
        self.cru[(bit_addr & 0x0FFF) as usize]
    }
}

impl Bus for FlatRam {
    fn read_word(&mut self, addr: u16) -> u16 {
        let a = (addr & 0xFFFE) as usize;
        // Big-endian: high byte at the even address.
        ((self.mem[a] as u16) << 8) | (self.mem[a + 1] as u16)
    }

    fn write_word(&mut self, addr: u16, value: u16) {
        let a = (addr & 0xFFFE) as usize;
        self.mem[a] = (value >> 8) as u8;
        self.mem[a + 1] = (value & 0xFF) as u8;
    }

    fn read_cru_bit(&mut self, bit_addr: u16) -> bool {
        self.cru[(bit_addr & 0x0FFF) as usize]
    }

    fn write_cru_bit(&mut self, bit_addr: u16, value: bool) {
        self.cru[(bit_addr & 0x0FFF) as usize] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_access_is_big_endian() {
        let mut ram = FlatRam::new();
        ram.write_word(0x1000, 0x1234);
        // High byte at the even (lower) address.
        assert_eq!(ram.peek(0x1000), 0x12);
        assert_eq!(ram.peek(0x1001), 0x34);
        assert_eq!(ram.read_word(0x1000), 0x1234);
    }

    #[test]
    fn word_access_ignores_address_bit_0() {
        let mut ram = FlatRam::new();
        ram.write_word(0x2000, 0xABCD);
        // Reading at the odd address still returns the even-aligned word.
        assert_eq!(ram.read_word(0x2001), 0xABCD);
    }

    #[test]
    fn load_words_places_big_endian_stream() {
        let mut ram = FlatRam::new();
        ram.load_words(0x0000, &[0x0460, 0x0024]);
        assert_eq!(ram.peek(0x0000), 0x04);
        assert_eq!(ram.peek(0x0001), 0x60);
        assert_eq!(ram.peek(0x0002), 0x00);
        assert_eq!(ram.peek(0x0003), 0x24);
    }

    #[test]
    fn cru_bits_round_trip() {
        let mut ram = FlatRam::new();
        assert!(!ram.read_cru_bit(0x10));
        ram.write_cru_bit(0x10, true);
        assert!(ram.read_cru_bit(0x10));
        assert!(ram.get_cru(0x10));
    }

    #[test]
    fn default_bus_has_no_wait_states() {
        let ram = FlatRam::new();
        assert_eq!(ram.wait_states(0x0000), 0);
        assert_eq!(ram.wait_states(0x8800), 0);
    }
}
