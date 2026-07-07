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

//! The firmware embedded in the binary — and nothing else.
//!
//! Only the project's own **clean-room firmware** is baked in: the console
//! ROM/GROM and the disk-controller DSR from the rewrite under
//! `original-content/system-roms/`. No cartridge, disk, or third-party image
//! of any kind is embedded; media is loaded at run time from user-given paths
//! (see [`crate::media`]), and authentic TI firmware can be supplied per-run
//! with `--system-rom` / `--system-grom` / `--disk-dsr`.

/// The default console ROM: the project's clean-room rewrite (Libre99), the
/// compiled `original-content/system-roms/rom/console-rom.bin`. Boots unless a
/// `--system-rom` override is given. Carries the `L99R` marker (its own baked
/// version), so the system information screen names it `LIBRE99 <version>`.
pub const DEFAULT_CONSOLE_ROM: &[u8] =
    include_bytes!("../../../original-content/system-roms/rom/console-rom.bin");

/// The default console GROM: the project's clean-room rewrite (Libre99), the
/// compiled `original-content/system-roms/grom/console-grom.bin`. Boots unless a
/// `--system-grom` override is given. Carries the `L99I` identification block,
/// so [`crate::sysinfo::stamp`] fills its host fields at launch.
pub const DEFAULT_CONSOLE_GROM: &[u8] =
    include_bytes!("../../../original-content/system-roms/grom/console-grom.bin");

/// The default disk-controller DSR: the project's clean-room rewrite, the
/// compiled `original-content/system-roms/disk-dsr/disk-dsr.bin` (Phase 3).
/// Installed unless a `--disk-dsr` override is given.
pub const DEFAULT_DISK_DSR: &[u8] =
    include_bytes!("../../../original-content/system-roms/disk-dsr/disk-dsr.bin");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_console_firmware_is_the_clean_room_rewrite() {
        // Same sizes as the console it emulates.
        assert_eq!(DEFAULT_CONSOLE_ROM.len(), 8192);
        assert_eq!(DEFAULT_CONSOLE_GROM.len(), 24576);
        // It self-identifies as ours: the GROM carries the L99I block, the ROM
        // the L99R marker — neither exists in authentic TI firmware. (Its CRC
        // also differs from the pinned authentic one; see `crate::sysinfo`.)
        assert!(libre99_core::sysinfo::has_block(DEFAULT_CONSOLE_GROM));
        assert!(libre99_core::sysinfo::rom_marker_version(DEFAULT_CONSOLE_ROM).is_some());
        assert_ne!(
            libre99_core::sysinfo::crc32(DEFAULT_CONSOLE_ROM),
            crate::sysinfo::AUTHENTIC_ROM_CRC32
        );
    }

    #[test]
    fn the_default_disk_dsr_is_the_clean_room_rewrite() {
        assert_eq!(DEFAULT_DISK_DSR.len(), 8192);
        assert_eq!(DEFAULT_DISK_DSR[0], 0xAA, "the >AA valid-DSR marker");
        // Not the authentic TI DSR (a well-known image, pinned by CRC).
        assert_ne!(
            libre99_core::sysinfo::crc32(DEFAULT_DISK_DSR),
            crate::sysinfo::AUTHENTIC_DISK_DSR_CRC32
        );
    }
}
