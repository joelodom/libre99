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

//! Build the rewritten TI-99/4A **console ROM** from the source in
//! `original-content/system-roms/rom/`, mirroring `libre99_gpl::system_grom`.
//!
//! The source is embedded with `include_str!` so [`build_console_rom`] is
//! deterministic and independent of the working directory (tests and probes
//! call it directly; no committed binary is needed to pass). If the source ever
//! grows past one file, embed each component the same way and concatenate here
//! — the on-disk `COPY` directive ([`crate::expand_includes`]) is for the
//! file/CLI workflow, not this reproducible in-memory build.

use crate::{assemble, Assembly, Diag, Options};

/// The console ROM occupies CPU `>0000..>1FFF` — 8 KiB.
pub const ROM_SIZE: usize = 0x2000;

/// Public entry points whose addresses are frozen (P8): external software and
/// TI's own GROMs branch to these, so a size regression that shifts any of them
/// must fail the build. The interpreter/KSCAN/ISR/XML entries are all pinned;
/// the layout-assertion gate (`Assembly::check_layout`) enforces the list.
/// Addresses are the authentic ones (RECON.md §2/§3).
pub const FROZEN_ENTRIES: &[(&str, u16)] = &[
    ("START", 0x0024),   // reset / EXIT routine
    ("SOFT", 0x006A),    // public soft entry: clear cond, run
    ("LOOP", 0x0070),    // GPL interpreter main loop
    ("FETCH", 0x0078),   // the fetch (public geometry: >001C stub)
    ("R9ENT", 0x007A),   // opcode-in-R9 entry (>0016 stub)
    ("MASK20", 0x011B),  // the NASTY condition-bit mask byte
    ("ISR", 0x0900),     // VBLANK interrupt handler
    ("SROM", 0x0AC0),    // XML >19 — peripheral-card ROM search
    ("SGROM", 0x0B24),   // XML >1A — GROM-header service search
    ("KSCAN", 0x02B2),   // keyboard scanner (behind >000E; SCAN opcode shim >02AE)
    ("GPOP", 0x0842),    // GROM-position pop helper (inverse of GPUSH)
    ("SPEC", 0x0270),    // specials sub-dispatch
    ("BACK", 0x029E),    // the BACK opcode handler
    ("CLEARH", 0x04B2),  // CLEAR/BREAK (FCTN-4) test (behind >0020)
    ("OPGET", 0x077A),   // the GAS operand engine
    ("RTN", 0x0838),     // RTN (clears cond, falls into RTNC)
    ("RTNC", 0x083E),    // RTNC
    ("CALLH2", 0x085A),  // CALL
    ("GPUSH", 0x0864),   // the GROM-position push helper
    ("VDPRL", 0x089A),   // VDP register-load helper
    ("NIBTAB", 0x0C36),  // first-nibble dispatch table
    ("SPCTAB", 0x0C3E),  // special-op dispatch table
    ("TAB7E", 0x0C7E),   // the >=>80 dispatch table
];

/// The top-level console-ROM source, with the `L99R` self-identification
/// marker appended (the assembler treats `END` as a directive, not a
/// terminator, so appended source still assembles).
///
/// The marker — `L99R` + a 12-byte space-padded version string at `>0BF0`
/// (`libre99_core::sysinfo::ROM_MARKER_ADDR`) — makes the image self-identifying:
/// the emulator's system-information stamp reads it to render the ROM row as
/// `LIBRE99 <version>`, and a hex dump shows it too. The version is this
/// workspace's `CARGO_PKG_VERSION`, the same number baked into the console
/// GROM and TI PYTHON. `>0BF0-0BFF` sits in the free gap between the SGROM
/// code (ends near `>0B4A`) and the fixed `>0C0C` interpreter home; the
/// marker-presence gate in `tests/rom.rs` fails if code ever grows over it.
pub fn console_asm_source() -> String {
    let code = include_str!("../../../original-content/system-roms/rom/console.asm");
    let version = env!("CARGO_PKG_VERSION");
    assert!(version.len() <= 12, "version does not fit the 12-byte ROM marker");
    format!(
        "{code}\n\
         * Libre99 self-identification marker (see libre99_core::sysinfo).\n\
         \x20       AORG >0BF0\n\
         \x20       TEXT 'L99R'\n\
         \x20       TEXT '{version:<12}'\n"
    )
}

/// Assemble the console ROM to its 8 KiB image, asserting the frozen public
/// entry points did not drift.
pub fn build_console_rom() -> Result<Vec<u8>, Vec<Diag>> {
    Ok(assemble_console_rom()?.rom)
}

/// As [`build_console_rom`] but returns the full [`Assembly`] (symbols, listing)
/// — used by the CLI to emit a listing / symbol map alongside the image.
pub fn assemble_console_rom() -> Result<Assembly, Vec<Diag>> {
    let asm = assemble(&console_asm_source(), &Options::absolute_image(ROM_SIZE))?;
    let drift = asm.check_layout(FROZEN_ENTRIES);
    if !drift.is_empty() {
        return Err(drift.into_iter().map(|message| Diag { line: 0, message }).collect());
    }
    Ok(asm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_rom_builds_to_8k_with_the_reset_vector() {
        let rom = build_console_rom().expect("console ROM assembles");
        assert_eq!(rom.len(), ROM_SIZE);
        // Reset vector: WP = >83E0, PC = >0024.
        assert_eq!(&rom[0..4], [0x83, 0xE0, 0x00, 0x24]);
    }
}
