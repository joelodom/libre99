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

//! The **Libre99 emulator-identification block** — the data contract behind the
//! rewritten console GROM's system information screen (`(S)` on the selection
//! menu).
//!
//! The screen splits its facts into two kinds. *Baked* facts (the GROM's own
//! version, TI PYTHON's version) are assembled into the image by the
//! `libre99-gpl` build and are correct wherever the image runs. *Stamped* facts
//! (the emulator's version, build date, commit, host, and which console ROM is
//! actually mounted) are known only to the running emulator, so the GROM image
//! reserves a block of GROM 2 for them and the frontend fills it in — in its
//! in-memory copy of the image, before the machine powers on. A real TMC0430
//! is mask-programmed, so pre-loading different bytes is indistinguishable
//! from shipping a different chip; nothing about the hardware model changes.
//!
//! Under an emulator that does not stamp (Classic99, a GROM board, …) the
//! block's flag byte stays `>00` and the firmware renders those rows as
//! `UNKNOWN` — the degraded mode is the default, not an error path.
//!
//! This module is the single home of the block's layout; the GROM build
//! (`libre99-gpl`), the frontend (`libre99-app`), and the firmware gates all read
//! these constants so the three can never drift apart. The block lives at
//! GROM `>5700-576F`: inside GROM 2's real 6 KiB (the `>5800-5FFF` chip gap
//! must stay zero — census invariant B4), above the thin-font loader block at
//! `>4000` and the system-information screen's own code at `>4800`.
//!
//! Layout (all text fields are space-padded ASCII in the console font's
//! `>20-5F` range — uppercase only):
//!
//! | GROM addr | len | field | filled by |
//! |-----------|-----|-------|-----------|
//! | `>5700`   | 4   | magic `L99I` | GROM build |
//! | `>5704`   | 1   | block format (`>01`) | GROM build |
//! | `>5705`   | 1   | flags: `>01` = host fields stamped | emulator |
//! | `>5706`   | 8   | emulator version | emulator |
//! | `>570E`   | 10  | emulator build date (commit date, `YYYY-MM-DD`) | emulator |
//! | `>5718`   | 8   | commit (short hash, `+` suffix if dirty) | emulator |
//! | `>5720`   | 12  | host OS/arch | emulator |
//! | `>572C`   | 20  | mounted console-ROM identity | emulator |
//!
//! The baked version strings (`VERSTR`, `PYVERS`, `PYBANR`) follow the block
//! from `>5740`; they are the GROM build's business and are not written here.
//!
//! The rewritten console **ROM** carries a marker of its own so the emulator
//! (or a hex dump) can identify it: `L99R` + a 12-byte version string at
//! `>0BF0`, in the free gap below the fixed `>0C0C` interpreter home.

/// Offset of the block in the 24 KiB console-GROM image.
pub const BLOCK_ADDR: usize = 0x5700;

/// The block's magic, at [`BLOCK_ADDR`]. If these four bytes are absent the
/// image carries no block (e.g. the authentic TI GROM) and stamping is a no-op.
pub const MAGIC: &[u8; 4] = b"L99I";

/// Offset of the one-byte block-format number (currently `0x01`).
pub const FORMAT_ADDR: usize = 0x5704;

/// Offset of the flags byte: `0x01` = the host fields below are stamped.
pub const FLAGS_ADDR: usize = 0x5705;

/// The stamped text fields as `(offset, length)`.
pub const EMU_VERSION: (usize, usize) = (0x5706, 8);
pub const BUILD_DATE: (usize, usize) = (0x570E, 10);
pub const COMMIT: (usize, usize) = (0x5718, 8);
pub const HOST: (usize, usize) = (0x5720, 12);
pub const ROM_ID: (usize, usize) = (0x572C, 20);

/// First byte past the stamped block (where the GROM build's baked version
/// strings begin).
pub const BLOCK_END: usize = 0x5740;

/// Offset of the `L99R` marker in the 8 KiB rewritten console-ROM image.
pub const ROM_MARKER_ADDR: usize = 0x0BF0;

/// The console-ROM marker magic.
pub const ROM_MARKER_MAGIC: &[u8; 4] = b"L99R";

/// Length of the version string that follows the ROM marker magic.
pub const ROM_MARKER_VERSION_LEN: usize = 12;

/// The host-side facts an emulator stamps into the block. Free-form strings;
/// [`stamp`] uppercases, sanitizes to the console font's range, truncates, and
/// space-pads each into its fixed-width field.
pub struct HostStamp<'a> {
    pub emu_version: &'a str,
    pub build_date: &'a str,
    pub commit: &'a str,
    pub host: &'a str,
    pub rom_id: &'a str,
}

/// Does this console-GROM image carry the identification block?
pub fn has_block(grom: &[u8]) -> bool {
    grom.len() >= BLOCK_END && &grom[BLOCK_ADDR..BLOCK_ADDR + 4] == MAGIC
}

/// Stamp the host fields into a console-GROM image and raise the stamped flag.
/// Returns `false` (leaving the image untouched) if the image carries no block
/// — stamping an arbitrary image is always safe.
pub fn stamp(grom: &mut [u8], host: &HostStamp) -> bool {
    if !has_block(grom) {
        return false;
    }
    write_field(grom, EMU_VERSION, host.emu_version);
    write_field(grom, BUILD_DATE, host.build_date);
    write_field(grom, COMMIT, host.commit);
    write_field(grom, HOST, host.host);
    write_field(grom, ROM_ID, host.rom_id);
    grom[FLAGS_ADDR] = 0x01;
    true
}

/// The version string baked into a rewritten console ROM, if this image is one
/// (identified by the `L99R` marker at [`ROM_MARKER_ADDR`]).
pub fn rom_marker_version(rom: &[u8]) -> Option<String> {
    let start = ROM_MARKER_ADDR + ROM_MARKER_MAGIC.len();
    let end = start + ROM_MARKER_VERSION_LEN;
    if rom.len() < end || &rom[ROM_MARKER_ADDR..start] != ROM_MARKER_MAGIC {
        return None;
    }
    Some(String::from_utf8_lossy(&rom[start..end]).trim_end().to_string())
}

/// Uppercase, sanitize to the console font's `>20-5F` glyph range, truncate to
/// the field width, and space-pad. The GROM ships no lowercase glyphs, so this
/// is what makes an arbitrary host string renderable by the firmware.
fn write_field(grom: &mut [u8], (offset, len): (usize, usize), text: &str) {
    let field = &mut grom[offset..offset + len];
    field.fill(b' ');
    for (slot, ch) in field.iter_mut().zip(text.chars()) {
        let up = ch.to_ascii_uppercase();
        *slot = match up {
            ' '..='_' => up as u8,
            _ => b'?',
        };
    }
}

/// CRC-32 (IEEE 802.3, the `cksum`/zlib polynomial) of an image — how the
/// frontend names a console ROM it does not recognize (`CUSTOM CRC XXXXXXXX`).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let low = crc & 1;
            crc >>= 1;
            if low != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal image carrying an unstamped block, as the GROM build ships it.
    fn image_with_block() -> Vec<u8> {
        let mut img = vec![0u8; 0x6000];
        img[BLOCK_ADDR..BLOCK_ADDR + 4].copy_from_slice(MAGIC);
        img[FORMAT_ADDR] = 0x01;
        for (off, len) in [EMU_VERSION, BUILD_DATE, COMMIT, HOST, ROM_ID] {
            img[off..off + len].fill(b' ');
        }
        img
    }

    #[test]
    fn stamping_fills_fields_and_raises_the_flag() {
        let mut img = image_with_block();
        let ok = stamp(
            &mut img,
            &HostStamp {
                emu_version: "0.0.1",
                build_date: "2026-07-05",
                commit: "2b109d4+",
                host: "windows x64",
                rom_id: "LIBRE99 0.0.1",
            },
        );
        assert!(ok);
        assert_eq!(img[FLAGS_ADDR], 0x01);
        assert_eq!(&img[EMU_VERSION.0..EMU_VERSION.0 + EMU_VERSION.1], b"0.0.1   ");
        assert_eq!(&img[BUILD_DATE.0..BUILD_DATE.0 + BUILD_DATE.1], b"2026-07-05");
        assert_eq!(&img[COMMIT.0..COMMIT.0 + COMMIT.1], b"2B109D4+");
        assert_eq!(&img[HOST.0..HOST.0 + HOST.1], b"WINDOWS X64 ");
        assert_eq!(&img[ROM_ID.0..ROM_ID.0 + ROM_ID.1], b"LIBRE99 0.0.1       ");
    }

    #[test]
    fn images_without_the_magic_are_left_untouched() {
        let mut img = vec![0u8; 0x6000];
        let before = img.clone();
        let ok = stamp(
            &mut img,
            &HostStamp {
                emu_version: "0.0.1",
                build_date: "2026-07-05",
                commit: "2b109d4",
                host: "macos arm64",
                rom_id: "TI 1981",
            },
        );
        assert!(!ok);
        assert_eq!(img, before, "an authentic image must never be modified");
    }

    #[test]
    fn overlong_and_unrenderable_text_is_truncated_and_sanitized() {
        let mut img = image_with_block();
        stamp(
            &mut img,
            &HostStamp {
                emu_version: "0.0.1-very-long-prerelease",
                build_date: "2026-07-05",
                commit: "2b109d4",
                host: "host\u{00E9}",
                rom_id: "TI 1981",
            },
        );
        assert_eq!(&img[EMU_VERSION.0..EMU_VERSION.0 + EMU_VERSION.1], b"0.0.1-VE");
        assert_eq!(&img[HOST.0..HOST.0 + 5], b"HOST?");
    }

    #[test]
    fn rom_marker_version_reads_only_marked_images() {
        let mut rom = vec![0u8; 0x2000];
        assert_eq!(rom_marker_version(&rom), None);
        let start = ROM_MARKER_ADDR;
        rom[start..start + 4].copy_from_slice(ROM_MARKER_MAGIC);
        rom[start + 4..start + 4 + ROM_MARKER_VERSION_LEN].copy_from_slice(b"0.0.1       ");
        assert_eq!(rom_marker_version(&rom).as_deref(), Some("0.0.1"));
    }

    #[test]
    fn crc32_matches_the_reference_vector() {
        // The canonical IEEE CRC-32 check value.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
