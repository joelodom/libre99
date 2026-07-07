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

//! Stamping of the Libre99 **emulator-identification block** (see
//! [`libre99_core::sysinfo`]) — the host-side half of the console firmware's
//! system information screen (`(S)` on the selection menu).
//!
//! At startup `main` resolves the console GROM this session will boot (the
//! `--system-grom` override or the embed). If that image carries the `L99I`
//! block — i.e. it is a Libre99 GROM — [`stamp`] fills the block's host fields
//! in the in-memory copy before the machine powers on: the emulator's version,
//! its build date and commit (from `build.rs`), the host OS/arch, and the
//! identity of the console ROM being mounted. The authentic TI GROM has no
//! block and passes through byte-identical. Under an emulator that does not
//! stamp, the firmware renders the host rows as `UNKNOWN` by design.

use libre99_core::sysinfo as block;

/// Stamp the block in `grom` (a console-GROM image about to boot) with this
/// build's identity and the identity of `rom` (the console ROM mounted beside
/// it). Returns the ROM identity string when the image carried a block, `None`
/// when it did not (authentic or custom GROM — left untouched).
pub fn stamp(grom: &mut [u8], rom: &[u8]) -> Option<String> {
    if !block::has_block(grom) {
        return None;
    }
    let rom_id = rom_identity(rom);
    block::stamp(
        grom,
        &block::HostStamp {
            emu_version: env!("CARGO_PKG_VERSION"),
            build_date: env!("LIBRE99_BUILD_DATE"),
            commit: env!("LIBRE99_GIT_COMMIT"),
            host: &host_identity(),
            rom_id: &rom_id,
        },
    );
    Some(rom_id)
}

/// CRC-32 (IEEE, as [`libre99_core::sysinfo::crc32`]) of the authentic TI-99/4A
/// console ROM (`994aROM.Bin`). The image itself is not distributed with this
/// project; the checksum — a public, well-known fact about it — lets the
/// system information screen recognize a user-supplied `--system-rom`.
pub const AUTHENTIC_ROM_CRC32: u32 = 0xDB8F_33E5;

/// CRC-32 of the authentic console GROM (`994AGROM.Bin`); see
/// [`AUTHENTIC_ROM_CRC32`] for why a checksum and not the bytes. (Referenced
/// by tests pinning that the clean-room firmware is not TI's.)
#[cfg(test)]
pub const AUTHENTIC_GROM_CRC32: u32 = 0xAF5C_2449;

/// CRC-32 of the authentic TI Disk Controller DSR (`Disk.Bin`); see above.
#[cfg(test)]
pub const AUTHENTIC_DISK_DSR_CRC32: u32 = 0x8F7D_F93F;

/// Name the console ROM for the screen's ROM row: the authentic image
/// (recognized by its well-known CRC-32), a Libre99 rewrite (self-identified
/// by its `L99R` marker, which also carries its own version — an older rewrite
/// under a newer emulator reports its own number), or an unrecognized image
/// named by its CRC-32.
fn rom_identity(rom: &[u8]) -> String {
    if block::crc32(rom) == AUTHENTIC_ROM_CRC32 {
        return "TI 1981".to_string();
    }
    if let Some(version) = block::rom_marker_version(rom) {
        return format!("LIBRE99 {version}");
    }
    format!("CUSTOM CRC {:08X}", block::crc32(rom))
}

/// The host OS and architecture, in the console font's uppercase-only range.
fn host_identity() -> String {
    let os = match std::env::consts::OS {
        "macos" => "MACOS",
        "windows" => "WINDOWS",
        "linux" => "LINUX",
        other => return format!("{} {}", other.to_ascii_uppercase(), arch()),
    };
    format!("{os} {}", arch())
}

fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "X64",
        "aarch64" => "ARM64",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal Libre99-style GROM image: just the unstamped block.
    fn libre_grom() -> Vec<u8> {
        let mut img = vec![0u8; 0x6000];
        img[block::BLOCK_ADDR..block::BLOCK_ADDR + 4].copy_from_slice(block::MAGIC);
        img[block::FORMAT_ADDR] = 0x01;
        img
    }

    #[test]
    fn the_authentic_grom_is_never_stamped() {
        // Needs the authentic image; loaded from third-party/ when present.
        let Some(authentic) = libre99_core::third_party::load_or_skip("roms/994AGROM.Bin") else {
            return;
        };
        assert_eq!(block::crc32(&authentic), AUTHENTIC_GROM_CRC32, "pinned CRC");
        let mut grom = authentic.clone();
        assert_eq!(stamp(&mut grom, &[0u8; 0x2000]), None);
        assert_eq!(grom, authentic, "the authentic image must boot byte-identical");
    }

    #[test]
    fn a_libre99_grom_is_stamped_with_this_build() {
        let mut grom = libre_grom();
        // Any ROM works for the stamping mechanics; the identity row is pinned
        // separately in `the_authentic_rom_is_recognized_by_crc`.
        let rom = vec![0xA5u8; 0x2000];
        let expected_id = rom_identity(&rom);
        let rom_id = stamp(&mut grom, &rom).expect("block present");
        assert_eq!(rom_id, expected_id);
        assert_eq!(grom[block::FLAGS_ADDR], 0x01);
        let version = env!("CARGO_PKG_VERSION").as_bytes();
        let at = block::EMU_VERSION.0;
        assert_eq!(&grom[at..at + version.len()], version);
        let at = block::ROM_ID.0;
        assert_eq!(&grom[at..at + expected_id.len()], expected_id.as_bytes());
    }

    #[test]
    fn the_authentic_rom_is_recognized_by_crc() {
        // Needs the authentic image; loaded from third-party/ when present.
        let Some(rom) = libre99_core::third_party::load_or_skip("roms/994aROM.Bin") else {
            return;
        };
        assert_eq!(rom_identity(&rom), "TI 1981");
    }

    #[test]
    fn a_libre99_rom_reports_its_own_baked_version() {
        let mut rom = vec![0u8; 0x2000];
        let at = block::ROM_MARKER_ADDR;
        rom[at..at + 4].copy_from_slice(block::ROM_MARKER_MAGIC);
        rom[at + 4..at + 16].copy_from_slice(b"9.9.9       ");
        assert_eq!(rom_identity(&rom), "LIBRE99 9.9.9");
    }

    #[test]
    fn an_unknown_rom_is_named_by_its_crc() {
        let rom = vec![0xA5u8; 0x2000];
        let id = rom_identity(&rom);
        assert!(id.starts_with("CUSTOM CRC "), "{id}");
        assert_eq!(id.len(), "CUSTOM CRC ".len() + 8);
    }

    #[test]
    fn host_identity_fits_its_field_and_font() {
        let host = host_identity();
        assert!(host.len() <= block::HOST.1, "{host} overflows the field");
        assert!(host.bytes().all(|b| (0x20..=0x5F).contains(&b)), "{host}");
    }
}
