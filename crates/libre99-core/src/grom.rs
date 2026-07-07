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

//! # GROM (Graphics Read-Only Memory, TMS/TMC 0430)
//!
//! GROMs are TI's bespoke serial ROM chips, and they are where the TI-99/4A keeps
//! almost everything interesting: the GPL interpreter's tables, the master title
//! screen, the built-in equation calculator, and the bulk of most cartridges
//! (including *Tunnels of Doom*). Understanding them is essential, because the
//! console firmware reads GROM constantly.
//!
//! ## How a GROM is accessed
//!
//! A GROM has **no parallel address bus**. Instead it exposes four byte-wide
//! ports in the CPU's memory map and an internal 16-bit address counter:
//!
//! | CPU address | Operation                                            |
//! |-------------|------------------------------------------------------|
//! | `>9800`     | read data byte (then auto-increment the counter)     |
//! | `>9802`     | read the address counter (high byte, then low)       |
//! | `>9C00`     | write data byte (GRAM only; mask ROMs ignore it)     |
//! | `>9C02`     | write the address counter (high byte, then low)      |
//!
//! To read from address `A` you write `A`'s high byte then its low byte to
//! `>9C02`, then read bytes from `>9800`; the counter advances on every read so a
//! whole table streams out with no further address writes. This serial design is
//! why GROM is slower than RAM and why GPL (which lives in GROM) is slow.
//!
//! ## The prefetch (the part that's easy to get wrong)
//!
//! The chip keeps a **one-byte prefetch buffer**. The "fetch one byte and
//! increment the counter" action happens:
//!
//! * once **automatically**, immediately after the second (low) address byte is
//!   written, and
//! * again after **every** data read.
//!
//! The net effect: after you set the address to `A`, the buffer holds `mem[A]`
//! and the counter already points at `A+1`. Your first `>9800` read returns
//! `mem[A]` (correct!) and refills the buffer with `mem[A+1]`. But if you read
//! the *address* counter back via `>9802` you see `A+1`, not `A`. This module
//! models that exactly so reads are never off by one.
//!
//! ## Slots and wrap-around
//!
//! GROM lives in its own 64 KiB address space, divided into eight 8 KiB **slots**
//! selected by the top three address bits. Each physical chip is only 6 KiB but
//! occupies a full 8 KiB slot. The auto-increment **wraps within the current
//! slot** — incrementing past `>1FFF` returns to `>0000` of the *same* slot, it
//! does not roll into the next GROM. Console GROMs 0–2 occupy slots `>0000–5FFF`;
//! cartridge GROMs begin at slot 3 (`>6000`).

/// A GROM array: the whole 64 KiB GROM address space plus the shared access
/// machinery (address counter, prefetch buffer, and the high/low byte phase
/// flip-flop). Real systems have several physical GROM chips that all watch the
/// same counter; modelling them as one flat space with one counter is
/// behaviorally identical from the CPU's point of view.
pub struct Grom {
    /// 64 KiB of GROM data. Console GROMs are loaded at offset 0; cartridge
    /// GROMs at `>6000` and up. Unpopulated bytes read as 0.
    data: Box<[u8; 0x1_0000]>,
    /// The 16-bit address counter.
    address: u16,
    /// The one-byte prefetch buffer (what the next data read will return).
    buffer: u8,
    /// Address-port byte phase: `false` ⇒ the next address byte written/read is
    /// the HIGH byte; `true` ⇒ the LOW byte. Reset to `false` (high) after a
    /// complete address is written and after every data read.
    low_byte_phase: bool,
    /// Diagnostic read log (address-of-byte, byte) when recording is enabled.
    record: bool,
    /// Recorded reads (see [`Grom::record`]).
    pub log: Vec<(u16, u8)>,
    /// Optional read-coverage bitmap: one bit per GROM address (65536 bits =
    /// 8 KiB), set for the true (prefetch-corrected) address of every byte
    /// returned by [`Grom::read_data`]. `None` (the default) disables coverage so
    /// normal runs allocate nothing and pay nothing. Independent of the
    /// [`Grom::record`] log; used by the cartridge coverage sweep to see which
    /// console-GROM addresses a cartridge actually exercises during
    /// boot/menu/launch.
    coverage: Option<Box<[u64; 1024]>>,
}

impl Default for Grom {
    fn default() -> Self {
        Self::new()
    }
}

impl Grom {
    /// A blank GROM array.
    pub fn new() -> Self {
        Grom {
            data: Box::new([0u8; 0x1_0000]),
            address: 0,
            buffer: 0,
            low_byte_phase: false,
            record: false,
            log: Vec::new(),
            coverage: None,
        }
    }

    /// Enable/disable the diagnostic read log.
    pub fn record(&mut self, on: bool) {
        self.record = on;
        if on {
            self.log.clear();
        }
    }

    /// Enable/disable the read-coverage bitmap. Enabling installs a fresh, zeroed
    /// bitmap (so each enable starts a clean sweep); disabling drops it, freeing
    /// the 8 KiB and making every coverage query inert.
    pub fn record_coverage(&mut self, on: bool) {
        self.coverage = if on { Some(Box::new([0u64; 1024])) } else { None };
    }

    /// Load `bytes` into the GROM address space starting at `addr` (wrapping at
    /// 64 KiB). Used to install the console GROMs (at `>0000`) and a cartridge's
    /// GROMs (at `>6000`, `>8000`, …).
    pub fn load(&mut self, addr: u16, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            self.data[addr.wrapping_add(i as u16) as usize] = b;
        }
    }

    /// Erase the cartridge region of the GROM space (`>6000` and up), leaving
    /// the console GROMs (`>0000–5FFF`) in place. Replacing a cartridge must
    /// not leave the previous one's pages readable where the new one has none.
    pub fn clear_cartridge_space(&mut self) {
        self.data[0x6000..].fill(0);
    }

    /// Is the next address-port write the LOW (second) byte of an address?
    /// The bus consults this for timing: on real hardware the second address
    /// byte triggers the prefetch and stalls the CPU longer than the first.
    pub fn expecting_low_address_byte(&self) -> bool {
        self.low_byte_phase
    }

    /// Inspect the current address counter (diagnostics).
    pub fn address(&self) -> u16 {
        self.address
    }

    /// Fetch `data[counter]` into the buffer and advance the counter, wrapping
    /// within the current 8 KiB slot. This is the GROM's fundamental action.
    fn prefetch_and_increment(&mut self) {
        self.buffer = self.data[self.address as usize];
        let slot = self.address & 0xE000;
        let next = (self.address.wrapping_add(1)) & 0x1FFF;
        self.address = slot | next;
    }

    /// The true GROM address of the byte currently in the prefetch buffer — the
    /// byte the next data read will return. Because the prefetch has already
    /// advanced the counter, that address is one *before* the counter, wrapping
    /// within the same 8 KiB slot (see the module-level prefetch note). Both the
    /// diagnostic log and the coverage bitmap attribute a read to this address.
    fn buffered_read_addr(&self) -> u16 {
        (self.address & 0xE000) | (self.address.wrapping_sub(1) & 0x1FFF)
    }

    /// Read a data byte from `>9800`: return the prefetch buffer, then refill it
    /// from the (already-incremented) counter. Also resets the address byte phase
    /// to "high".
    pub fn read_data(&mut self) -> u8 {
        let value = self.buffer;
        if self.record || self.coverage.is_some() {
            // The byte being returned was prefetched from one address before the
            // current (already-incremented) counter, within the same 8K slot.
            let read_addr = self.buffered_read_addr();
            if self.record {
                self.log.push((read_addr, value));
            }
            if let Some(coverage) = self.coverage.as_mut() {
                let (word, mask) = Self::coverage_pos(read_addr);
                coverage[word] |= mask;
            }
        }
        self.prefetch_and_increment();
        self.low_byte_phase = false;
        value
    }

    /// Write a data byte to `>9C00`. Mask-ROM GROMs (everything we ship) ignore
    /// this — only GRAM chips are writable, and none of the bundled images use
    /// GRAM. Documented and intentionally a no-op apart from resetting the phase.
    pub fn write_data(&mut self, _byte: u8) {
        self.low_byte_phase = false;
    }

    /// Read the address counter from `>9802`: high byte first, then low byte.
    /// Because the counter was already advanced by the prefetch, this returns the
    /// post-increment value (the documented "counter + 1" behavior).
    ///
    /// The read is *destructive*, exactly like the real TMC0430: returning the
    /// high byte leaves the counter holding `low:low`, so an immediately
    /// following read returns the low byte. Crucially, touching the address port
    /// also resets the address-*write* byte selector ([`Grom::low_byte_phase`]),
    /// so a subsequent two-byte address write is still interpreted high-then-low
    /// even after an *odd* number of address reads. The GPL interpreter relies on
    /// this: a GPL branch reads the address port (to recover the current GROM
    /// slot) and then writes the branch target; without the reset that write
    /// would be mis-sequenced and the branch would land on the wrong address.
    pub fn read_address(&mut self) -> u8 {
        let value = (self.address >> 8) as u8;
        let low = self.address & 0x00FF;
        self.address = (low << 8) | low;
        self.low_byte_phase = false;
        value
    }

    /// Write a byte of the address counter to `>9C02`. Two writes (high then low)
    /// set the full address; the second write triggers the automatic prefetch.
    ///
    /// Each write shifts the new byte into the low end of the counter, exactly as
    /// the hardware does — after two writes the counter equals `high:low`
    /// regardless of its previous contents.
    pub fn write_address(&mut self, byte: u8) {
        self.address = (self.address << 8) | (byte as u16);
        if self.low_byte_phase {
            // That was the low byte: the address is complete. Prefetch the byte
            // at the new address so the first data read returns it.
            self.low_byte_phase = false;
            self.prefetch_and_increment();
        } else {
            // That was the high byte; the low byte comes next.
            self.low_byte_phase = true;
        }
    }

    /// The `(word index, bit mask)` position of GROM address `addr` in the
    /// coverage bitmap. The single source of truth for the bitmap's layout,
    /// shared by the set site in [`Grom::read_data`] and every query below.
    fn coverage_pos(addr: u16) -> (usize, u64) {
        ((addr >> 6) as usize, 1u64 << (addr & 63))
    }

    /// Whether the byte at GROM address `addr` has been returned by a data read
    /// since coverage was last enabled. Always `false` when coverage is disabled.
    pub fn was_read(&self, addr: u16) -> bool {
        let (word, mask) = Self::coverage_pos(addr);
        self.coverage
            .as_ref()
            .is_some_and(|bits| bits[word] & mask != 0)
    }

    /// Count the distinct addresses in `range` whose byte has been read. The
    /// coverage sweep uses this to measure how much of a routine's address span
    /// (e.g. a console-GROM entry point) a cartridge exercised. `0` when coverage
    /// is disabled.
    pub fn coverage_count(&self, range: std::ops::RangeInclusive<u16>) -> usize {
        range.filter(|&addr| self.was_read(addr)).count()
    }

    /// Every GROM address read since coverage was enabled, in ascending order.
    /// Empty when coverage is disabled. Allocates a fresh `Vec`; intended for
    /// end-of-sweep report generation, not hot paths.
    pub fn read_addresses(&self) -> Vec<u16> {
        (0..=u16::MAX).filter(|&addr| self.was_read(addr)).collect()
    }

    /// Serialize the GROM image and counter state into a save state. The full
    /// 64 KiB image is included so the snapshot is self-contained (no need to
    /// re-mount the cartridge to reload its GROM pages).
    pub(crate) fn save_state(&self, w: &mut crate::state::StateWriter) {
        w.raw(&self.data[..]);
        w.u16(self.address);
        w.u8(self.buffer);
        w.bool(self.low_byte_phase);
    }

    /// Restore the GROM image and counter state from a save state.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
    ) -> Result<(), crate::state::StateError> {
        r.fill(&mut self.data[..])?;
        self.address = r.u16()?;
        self.buffer = r.u8()?;
        self.low_byte_phase = r.bool()?;
        // Diagnostics never persist across a load.
        self.record = false;
        self.log.clear();
        self.coverage = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set the 16-bit GROM address the way software does: HIGH byte then LOW byte
    /// to the address port. Mirrors the conformance-test helper so the prefetch
    /// lands on the same address the log and coverage bitmap must record.
    fn set_address(g: &mut Grom, a: u16) {
        g.write_address((a >> 8) as u8);
        g.write_address(a as u8);
    }

    #[test]
    fn coverage_bitmap_marks_the_true_prefetch_corrected_read_addresses() {
        let mut g = Grom::new();
        g.load(0x1234, &[0xDE, 0xAD, 0xBE, 0xEF]);
        // Record both instruments so we can assert they agree on the addresses.
        g.record(true);
        g.record_coverage(true);

        set_address(&mut g, 0x1234);
        assert_eq!(g.read_data(), 0xDE);
        assert_eq!(g.read_data(), 0xAD);
        assert_eq!(g.read_data(), 0xBE);

        // The three bytes returned came from >1234, >1235, >1236 (prefetch-
        // corrected — the same addresses the diagnostic log records), so exactly
        // those bits must be set.
        assert!(g.was_read(0x1234));
        assert!(g.was_read(0x1235));
        assert!(g.was_read(0x1236));
        // The byte just past what we read (still buffered, not yet returned) and
        // the byte just before the start are untouched.
        assert!(!g.was_read(0x1233));
        assert!(!g.was_read(0x1237));

        // The bitmap agrees with the diagnostic log, and the range/list queries
        // report the same set of addresses.
        let logged: Vec<u16> = g.log.iter().map(|&(addr, _)| addr).collect();
        assert_eq!(logged, vec![0x1234, 0x1235, 0x1236]);
        assert_eq!(g.read_addresses(), vec![0x1234, 0x1235, 0x1236]);
        assert_eq!(g.coverage_count(0x1234..=0x1236), 3);
        assert_eq!(g.coverage_count(0x1234..=0x1237), 3);
        assert_eq!(g.coverage_count(0x0000..=0x1233), 0);
    }

    #[test]
    fn coverage_is_off_by_default_and_dropped_on_disable() {
        let mut g = Grom::new();
        g.load(0x0000, &[0x01, 0x02]);

        // Disabled by default: reads mark nothing and queries stay inert.
        set_address(&mut g, 0x0000);
        let _ = g.read_data();
        assert!(!g.was_read(0x0000));
        assert!(g.read_addresses().is_empty());
        assert_eq!(g.coverage_count(0x0000..=0xFFFF), 0);

        // Enable, read, then disable: the bitmap is dropped and queries go inert.
        g.record_coverage(true);
        set_address(&mut g, 0x0000);
        let _ = g.read_data();
        assert!(g.was_read(0x0000));
        g.record_coverage(false);
        assert!(!g.was_read(0x0000));
        assert!(g.read_addresses().is_empty());
    }

    #[test]
    fn load_state_drops_coverage() {
        let mut g = Grom::new();
        g.load(0x0000, &[0x01]);
        g.record_coverage(true);
        set_address(&mut g, 0x0000);
        let _ = g.read_data();
        assert!(g.was_read(0x0000));

        // Round-trip through a save state: like `record`/`log`, coverage is a
        // diagnostic and must not survive a load.
        let mut w = crate::state::StateWriter::new();
        g.save_state(&mut w);
        let bytes = w.into_bytes();
        let mut r = crate::state::StateReader::new(&bytes);
        g.load_state(&mut r).unwrap();

        assert!(!g.was_read(0x0000), "coverage must be dropped by load_state");
    }
}
