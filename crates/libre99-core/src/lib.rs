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

//! # libre99-core — a TI-99/4A emulator core
//!
//! This crate emulates the hardware of the Texas Instruments TI-99/4A home
//! computer (1981) faithfully enough to run the machine's **real firmware**. The
//! TI-99/4A is unusual: most of its operating system and all of its built-in
//! applications are not CPU machine code but **GPL** (Graphics Programming
//! Language) *bytecode*, stored in serial **GROM** chips and interpreted by a
//! small machine-code kernel in the system ROM. Cartridges such as *Tunnels of
//! Doom* are likewise mostly GPL. We therefore do **not** reimplement GPL or the
//! TI OS; we emulate the chips correctly and let the genuine ROM/GROM images do
//! the work.
//!
//! ## The emulated machine
//!
//! | Subsystem        | Chip                | Module        |
//! |------------------|---------------------|---------------|
//! | CPU              | TMS9900             | [`cpu`]       |
//! | Memory map / CRU | (board logic)       | [`bus`]       |
//! | Video            | TMS9918A (VDP)      | `vdp`         |
//! | Sound            | SN76489 (PSG)       | `psg`         |
//! | Firmware store   | TMC0430 GROM        | `grom`        |
//! | I/O & keyboard   | TMS9901 + CRU       | `cru`         |
//! | Mass storage     | TI Disk Ctrl/FD1771 | `disk`        |
//! | Cartridge images | `.ctg` loader       | `cartridge`   |
//! | Key matrix       | (8×8 switch state)  | `keyboard`    |
//! | Save states      | (snapshot codec)    | `state`       |
//! | System-info block| (Libre99 contract)  | `sysinfo`     |
//! | Whole machine    | (wiring)            | `machine`     |
//!
//! The set is complete: every chip the console needs to boot firmware, run
//! cartridges, and load from disk is emulated and tested (see
//! `docs/ARCHITECTURE.md`).
//!
//! ## Design seam: the [`bus::Bus`] trait
//!
//! The CPU knows nothing about the rest of the machine. It performs all of its
//! work through the [`bus::Bus`] trait — word reads/writes and single-bit CRU
//! reads/writes — and asks the bus how many wait-state cycles each access costs.
//! This lets the CPU be unit-tested against a flat-RAM bus
//! ([`bus::FlatRam`]) in complete isolation, and keeps every machine-specific
//! detail (the console memory map, peripheral routing, wait states) in one place.
//!
//! ## Numbers and notation
//!
//! Following TI convention, hexadecimal constants are written `>ABCD` in
//! comments. The TMS9900 is **big-endian**: the most-significant byte of a 16-bit
//! word lives at the lower (even) address. All multi-byte values in ROMs, GROMs,
//! and on disk are big-endian unless explicitly noted.

// Deny a few classes of mistakes that are easy to make in low-level emulation
// code, while keeping the lints that would fight against faithful hardware
// modelling (e.g. lots of `as` casts between widths) at warn.
#![forbid(unsafe_code)]

pub mod bus;
pub mod cartridge;
pub mod cpu;
pub mod cru;
pub mod disk;
pub mod grom;
pub mod keyboard;
pub mod machine;
pub mod psg;
pub mod state;
pub mod sysinfo;
pub mod third_party;
pub mod vdp;
