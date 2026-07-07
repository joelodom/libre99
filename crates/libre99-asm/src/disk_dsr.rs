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

//! Build the rewritten TI Disk Controller **DSR ROM** from the source in
//! `original-content/system-roms/disk-dsr/`, mirroring [`crate::system_rom`].
//!
//! The disk card maps its DSR ROM at CPU `>4000..>5FFF`; the top 16 bytes
//! (`>5FF0..5FFF`) are the FD1771 register overlay, never ROM. The image is
//! assembled at an **absolute base of `>4000`** so the `>AA` header's chain /
//! entry pointers resolve to their real `>40xx` addresses — the console's SROM
//! (`XML >19`) reads those words and `BL *R9`-calls them. Source is embedded
//! with `include_str!` so [`build_disk_dsr`] is deterministic and independent
//! of the working directory (tests and probes call it directly; the committed
//! `disk-dsr.bin` is a convenience artifact, not a build input).
//!
//! Phase 3 of the system-ROM project. Status: the T1 tracer bullet — a valid
//! header + an idempotent power-up + a DSK1 stub node. The FD1771 driver, the
//! on-disk file system, the PAB opcodes, and the subprograms arrive in
//! milestones M1-M4 (see the plan in that folder).

use crate::{assemble, Assembly, Diag, Options};

/// The disk DSR is mapped at CPU `>4000`.
pub const DSR_BASE: u16 = 0x4000;

/// The DSR ROM window is 8 KiB (`>4000..>5FFF`); [`libre99_core`]'s disk card uses
/// the first this-many bytes. The top 16 (`>5FF0..5FFF`) are shadowed by the
/// FD1771 registers at runtime, so nothing live may sit there (enforced below).
pub const DSR_SIZE: usize = 0x2000;

/// Image offset of the FD1771 register shadow (`>5FF0..5FFF` → offsets
/// `>1FF0..1FFF`): those bytes are never visible as ROM, so they stay zero.
const REGISTER_SHADOW: usize = 0x1FF0;

/// Structurally-pinned addresses — the DSR analogue of the console ROM's frozen
/// entries, but far smaller: the console discovers every routine by walking the
/// `>AA` header chains (plan §2.5, P8-DISK), so only the header itself is fixed.
/// The layout-assertion gate ([`Assembly::check_layout`]) enforces the list.
pub const FROZEN_ENTRIES: &[(&str, u16)] = &[
    ("HDR", 0x4000), // the >AA peripheral-card header
];

/// The disk-DSR source, embedded for a reproducible in-memory build.
pub fn disk_dsr_source() -> String {
    include_str!("../../../original-content/system-roms/disk-dsr/disk-dsr.asm").to_string()
}

/// Assemble the disk DSR to its 8 KiB `>4000`-based [`Assembly`] (symbols,
/// listing), asserting the header did not drift and the FD1771 register shadow
/// stays clear. The CLI uses this to emit a listing / symbol map alongside the
/// image; [`build_disk_dsr`] returns just the bytes.
pub fn assemble_disk_dsr() -> Result<Assembly, Vec<Diag>> {
    // `Options::absolute_image` hardcodes base 0 (the console ROM's shape), so
    // construct directly with the DSR's `>4000` origin.
    let opts = Options {
        name: None,
        entry: None,
        base: DSR_BASE,
        auto_header: false,
        absolute: true,
        image_size: DSR_SIZE,
    };
    let asm = assemble(&disk_dsr_source(), &opts)?;

    let drift = asm.check_layout(FROZEN_ENTRIES);
    if !drift.is_empty() {
        return Err(drift.into_iter().map(|message| Diag { line: 0, message }).collect());
    }
    if asm.rom.get(REGISTER_SHADOW..DSR_SIZE).is_none_or(|s| s.iter().any(|&b| b != 0)) {
        return Err(vec![Diag {
            line: 0,
            message: "disk-dsr: content in the FD1771 register shadow (>5FF0..5FFF) — \
                      those bytes are never visible as ROM"
                .into(),
        }]);
    }
    Ok(asm)
}

/// Build the disk DSR's 8 KiB image. `image[0]` is the `>AA` valid-DSR marker;
/// it drops straight into `libre99_core`'s `Disk::load_dsr`.
pub fn build_disk_dsr() -> Result<Vec<u8>, Vec<Diag>> {
    Ok(assemble_disk_dsr()?.rom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disk_dsr_builds_to_8k_with_the_aa_header() {
        let rom = build_disk_dsr().expect("disk DSR assembles");
        assert_eq!(rom.len(), DSR_SIZE, "the disk DSR is exactly 8 KiB");
        assert_eq!(rom[0], 0xAA, ">4000 must be the >AA valid-DSR marker");
    }

    #[test]
    fn the_chain_heads_point_into_the_image() {
        let rom = build_disk_dsr().expect("disk DSR assembles");
        let word = |o: usize| ((rom[o] as u16) << 8) | rom[o + 1] as u16;
        let in_image = |a: u16| (DSR_BASE..DSR_BASE + DSR_SIZE as u16).contains(&a);
        // Power-up head (>4004) and device head (>4008) resolve into the image;
        // program / subprogram / interrupt heads are the empty chain (>0000).
        assert!(in_image(word(0x0004)), "power-up chain head must point into the DSR");
        assert!(in_image(word(0x0008)), "device chain head must point into the DSR");
        assert_eq!(word(0x0006), 0x0000, "program chain is empty for the disk card");
        assert_eq!(word(0x000C), 0x0000, "interrupt chain is empty for the disk card");
    }
}
