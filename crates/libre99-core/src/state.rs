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

//! # Save-state serialization (`std`-only, zero dependencies)
//!
//! A *save state* is a complete, self-contained binary snapshot of the running
//! machine: every byte of RAM and VRAM, the full GROM image, the cartridge ROM,
//! the mounted disk images (**including any sectors the program has written**),
//! and every chip's internal register/latch state. Restoring one reproduces the
//! machine *exactly* — boot a program, write to a disk, save, quit, relaunch, and
//! [`Machine::load_state`](crate::machine::Machine::load_state) puts you back
//! where you left off.
//!
//! Because the core forbids third-party crates, there is no `serde`: each chip
//! module implements `save_state`/`load_state` against the tiny [`StateWriter`] /
//! [`StateReader`] cursors below. The format is deliberately simple and
//! little-endian; it is versioned with a magic number so a foreign or truncated
//! file is rejected cleanly rather than mis-read.
//!
//! The snapshot is self-contained on purpose: it embeds the ROM/GROM/cartridge
//! images so a reload depends on nothing but the file itself. That trades a larger
//! file (a few hundred KiB, dominated by the 64 KiB GROM image and the disk
//! images) for robustness — exactly the right trade for "resume my session".

use std::fmt;

/// Appends primitive values to a growing byte buffer, little-endian. The mirror
/// image of [`StateReader`].
#[derive(Default)]
pub struct StateWriter {
    buf: Vec<u8>,
}

impl StateWriter {
    /// A fresh, empty writer.
    pub fn new() -> Self {
        StateWriter { buf: Vec::new() }
    }

    /// Consume the writer and return the encoded bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }
    pub fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    pub fn bool(&mut self, v: bool) {
        self.u8(v as u8);
    }
    /// A float, stored by its IEEE-754 bit pattern (exact, including NaN/inf).
    pub fn f64(&mut self, v: f64) {
        self.u64(v.to_bits());
    }
    /// A `usize`, narrowed to 32 bits (all of ours are tiny indices/counts).
    pub fn usize(&mut self, v: usize) {
        self.u32(v as u32);
    }
    /// Raw bytes with no length prefix — for fixed-size arrays whose length is
    /// known at both ends.
    pub fn raw(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }
    /// A length-prefixed byte blob — for `Vec<u8>` whose length must be recovered.
    pub fn blob(&mut self, bytes: &[u8]) {
        self.u32(bytes.len() as u32);
        self.raw(bytes);
    }
    /// `Option<u8>` as a presence flag followed by the value when present.
    pub fn opt_u8(&mut self, v: Option<u8>) {
        match v {
            Some(x) => {
                self.u8(1);
                self.u8(x);
            }
            None => self.u8(0),
        }
    }
    /// `Option<usize>` as a presence flag followed by the value when present.
    pub fn opt_usize(&mut self, v: Option<usize>) {
        match v {
            Some(x) => {
                self.u8(1);
                self.usize(x);
            }
            None => self.u8(0),
        }
    }
}

/// Reads primitive values from a byte slice, little-endian, with bounds checks
/// that turn a short read into [`StateError::Truncated`] instead of a panic.
pub struct StateReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> StateReader<'a> {
    /// A reader positioned at the start of `data`.
    pub fn new(data: &'a [u8]) -> Self {
        StateReader { data, pos: 0 }
    }

    /// Borrow the next `n` bytes, advancing the cursor; error if fewer remain.
    fn take(&mut self, n: usize) -> Result<&'a [u8], StateError> {
        let end = self.pos.checked_add(n).ok_or(StateError::Truncated)?;
        let slice = self.data.get(self.pos..end).ok_or(StateError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8, StateError> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, StateError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    pub fn u32(&mut self) -> Result<u32, StateError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn u64(&mut self) -> Result<u64, StateError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }
    pub fn bool(&mut self) -> Result<bool, StateError> {
        Ok(self.u8()? != 0)
    }
    pub fn f64(&mut self) -> Result<f64, StateError> {
        Ok(f64::from_bits(self.u64()?))
    }
    pub fn usize(&mut self) -> Result<usize, StateError> {
        Ok(self.u32()? as usize)
    }
    /// Fill `dst` from the stream (the read counterpart of [`StateWriter::raw`]).
    pub fn fill(&mut self, dst: &mut [u8]) -> Result<(), StateError> {
        let n = dst.len();
        dst.copy_from_slice(self.take(n)?);
        Ok(())
    }
    /// Read a length-prefixed blob (the counterpart of [`StateWriter::blob`]).
    pub fn blob(&mut self) -> Result<Vec<u8>, StateError> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }
    pub fn opt_u8(&mut self) -> Result<Option<u8>, StateError> {
        if self.u8()? != 0 {
            Ok(Some(self.u8()?))
        } else {
            Ok(None)
        }
    }
    pub fn opt_usize(&mut self) -> Result<Option<usize>, StateError> {
        if self.u8()? != 0 {
            Ok(Some(self.usize()?))
        } else {
            Ok(None)
        }
    }
}

/// Why a [`Machine::load_state`](crate::machine::Machine::load_state) failed. The
/// machine is left untouched on any of these (the load builds a fresh machine and
/// only swaps it in on success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    /// The file does not start with the save-state magic — not one of ours.
    BadMagic,
    /// A save-state of a format version this build does not understand.
    UnsupportedVersion(u32),
    /// The data ended before a fully-formed snapshot could be read.
    Truncated,
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateError::BadMagic => write!(f, "not a TI-99 save state (bad magic)"),
            StateError::UnsupportedVersion(v) => {
                write!(f, "unsupported save-state version {v}")
            }
            StateError::Truncated => write!(f, "save state is truncated or corrupt"),
        }
    }
}

impl std::error::Error for StateError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_round_trip() {
        let mut w = StateWriter::new();
        w.u8(0xAB);
        w.u16(0x1234);
        w.u32(0xDEAD_BEEF);
        w.u64(0x0102_0304_0506_0708);
        w.bool(true);
        w.bool(false);
        w.f64(-1.5);
        w.usize(4096);
        w.raw(&[1, 2, 3, 4]);
        w.blob(&[9, 8, 7]);
        w.opt_u8(Some(42));
        w.opt_u8(None);
        w.opt_usize(Some(7));
        w.opt_usize(None);
        let bytes = w.into_bytes();

        let mut r = StateReader::new(&bytes);
        assert_eq!(r.u8().unwrap(), 0xAB);
        assert_eq!(r.u16().unwrap(), 0x1234);
        assert_eq!(r.u32().unwrap(), 0xDEAD_BEEF);
        assert_eq!(r.u64().unwrap(), 0x0102_0304_0506_0708);
        assert!(r.bool().unwrap());
        assert!(!r.bool().unwrap());
        assert_eq!(r.f64().unwrap(), -1.5);
        assert_eq!(r.usize().unwrap(), 4096);
        let mut four = [0u8; 4];
        r.fill(&mut four).unwrap();
        assert_eq!(four, [1, 2, 3, 4]);
        assert_eq!(r.blob().unwrap(), vec![9, 8, 7]);
        assert_eq!(r.opt_u8().unwrap(), Some(42));
        assert_eq!(r.opt_u8().unwrap(), None);
        assert_eq!(r.opt_usize().unwrap(), Some(7));
        assert_eq!(r.opt_usize().unwrap(), None);
    }

    #[test]
    fn reading_past_the_end_is_truncated_not_a_panic() {
        let bytes = [0u8; 2];
        let mut r = StateReader::new(&bytes);
        assert_eq!(r.u32(), Err(StateError::Truncated));
    }
}
