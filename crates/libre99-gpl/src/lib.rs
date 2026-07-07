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

//! # libre99-gpl — a GPL (Graphics Programming Language) toolchain for the TI-99/4A
//!
//! The TI-99/4A's operating system is almost entirely **GPL bytecode** stored in
//! serial GROM chips and interpreted by a small machine-code kernel in the
//! console ROM (`crates/libre99-core/src/lib.rs:6-11`). This crate is the toolchain
//! for the **system-GROM rewrite** — authoring original, TI-copyright-free
//! console firmware in GPL that the genuine ROM interpreter runs (see
//! `original-content/system-roms/`).
//!
//! Pieces:
//!
//! * [`operand`] — the GPL general-address (GAS) operand encoding, shared by the
//!   encoder and decoder.
//! * [`isa`] — opcode signatures.
//! * [`decode`] / [`disasm`] — turning GPL bytes back into instructions, for
//!   reconnaissance and for checking our own output.
//! * [`encode`] / [`asm`] — the assembler: original GPL source → a system-GROM
//!   image the emulator boots in place of `994AGROM.Bin`.
//!
//! The encoder targets a curated, **execution-validated** opcode subset: every
//! instruction we emit is proven either by a golden test or by running its bytes
//! on the real console ROM inside `libre99-core` (mirroring how `libre99-asm`'s output
//! is proven by booting it). The decoder additionally makes a best-effort at the
//! wider ISA for disassembly.

pub mod asm;
pub mod census;
pub mod decode;
pub mod disasm;
pub mod encode;
pub mod font;
pub mod keymap;
pub mod logo;
pub mod isa;
pub mod operand;
pub mod system_grom;

pub use asm::{assemble, Assembly, Diag, GROM_IMAGE_LEN};
