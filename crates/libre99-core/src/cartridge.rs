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

//! # Cartridge images — Marc Rousseau's `ti99sim` `.ctg` format
//!
//! TI-99/4A cartridges plug GROM and/or ROM chips onto the console bus. A `.ctg`
//! file is the open-source `ti99sim` container for those chip images. Despite the
//! `"TI-99/4A Module - "` banner it is **not** Win994a's native format; it is a
//! faithful port of `ti99sim`'s `cCartridge::LoadImageV1`, verified to parse all
//! 137 bundled images byte-exact (bytes-consumed == file size).
//!
//! ## File layout (version 1 — what every bundled image uses)
//! ```text
//!   0x00  80  Banner: "TI-99/4A Module - <title>\n\x1A", zero-padded
//!   0x50   1  Version marker; high nibble selects the version (0x1n = V1)
//!   0x51   2  CRU base (big-endian; >0000 for a plain ROM/GROM cartridge)
//!   0x53  ..  region records, repeated until end-of-file:
//!               1  index   (CPU 4K page 0..15, or 16+page for an 8K GROM page)
//!               1  #banks
//!               per bank:
//!                 1   bank type (2 = ROM → data follows; others carry no data)
//!                 ..  RLE-compressed bank data (see [`rle_decode`])
//! ```
//! A CPU index `i < 16` is a 4 KiB page at CPU `i·>1000` — cartridge ROM occupies
//! pages 6 and 7 (the `>6000–7FFF` window). An index `>= 16` is an 8 KiB GROM page
//! `i-16` at GROM `(i-16)·>2000`; cartridge GROMs start at page 3 (GROM `>6000`).
//!
//! ## RLE codec
//! `ti99sim`'s `compress.cpp` run-length scheme: a little-endian 16-bit tag, then
//! either one byte repeated (`tag & >8000`, count `tag & >7FFF`) or a literal run
//! of `tag` bytes. See [`rle_decode`].

use std::collections::BTreeMap;

/// Banner that every `.ctg` file begins with (the title follows it).
const BANNER: &[u8] = b"TI-99/4A Module - ";
/// CPU-ROM pages are 4 KiB; cartridge ROM uses pages 6 and 7 (`>6000–7FFF`).
const ROM_BANK_SIZE: usize = 0x1000;
/// GROM pages are 8 KiB.
const GROM_BANK_SIZE: usize = 0x2000;
/// Region indices `< GROM_0` are CPU-ROM pages; `>= GROM_0` are GROM pages. There
/// are 16 CPU 4 KiB pages (the full 64 KiB CPU space) before the first GROM page.
const GROM_0: u8 = 16;
/// Bank type that carries ROM data (other types — RAM, battery-backed — store no
/// bytes in the file and are not driven by the bundled images).
const BANK_ROM: u8 = 2;

/// A parsed cartridge: its title, CRU base, the `>6000–7FFF` ROM banks (assembled
/// as consecutive 8 KiB banks ready for the cartridge window), and its GROM pages.
#[derive(Debug, Clone)]
pub struct Cartridge {
    /// Human-readable title from the banner (e.g. `"TUNNELS OF DOOM"`).
    pub title: String,
    /// CRU base address from the header (`>0000` for plain ROM/GROM cartridges).
    pub cru_base: u16,
    /// Cartridge ROM as consecutive 8 KiB banks for the `>6000–7FFF` window;
    /// empty for a pure-GROM cartridge.
    pub rom: Vec<u8>,
    /// Number of 8 KiB ROM banks in [`rom`](Self::rom) (`0` when pure-GROM).
    pub rom_banks: usize,
    /// Cartridge GROM pages as `(grom_address, 8 KiB data)`, in file order.
    pub grom: Vec<(u16, Vec<u8>)>,
}

/// Why a `.ctg` image could not be parsed.
#[derive(Debug, PartialEq, Eq)]
pub enum CartridgeError {
    /// The file does not begin with the `"TI-99/4A Module - "` banner.
    BadBanner,
    /// The version marker is not a supported version (only V1 is bundled).
    UnsupportedVersion(u8),
    /// The file ended in the middle of a record.
    Truncated,
    /// A run-length record was malformed (zero-length literal, or it overran the
    /// bank).
    BadRle,
}

impl Cartridge {
    /// Parse a `ti99sim` V1 `.ctg` image.
    pub fn parse(data: &[u8]) -> Result<Cartridge, CartridgeError> {
        if data.len() < 0x53 || &data[..BANNER.len()] != BANNER {
            return Err(CartridgeError::BadBanner);
        }
        let title = parse_title(data);

        // Version marker at >50: V0 has the high bit set; V1/V2 use the high
        // nibble. Every bundled image is V1 (`0x1n`); reject anything else.
        let marker = data[0x50];
        if marker & 0x80 != 0 || marker & 0xF0 != 0x10 {
            return Err(CartridgeError::UnsupportedVersion(marker));
        }

        let mut p = 0x51;
        let cru_base = ((data[p] as u16) << 8) | data[p + 1] as u16;
        p += 2;

        // CPU-ROM banks collected by 4 KiB page; GROM pages collected in order.
        let mut cpu: BTreeMap<u8, Vec<Vec<u8>>> = BTreeMap::new();
        let mut grom: Vec<(u16, Vec<u8>)> = Vec::new();

        while p < data.len() {
            let index = read_u8(data, &mut p)?;
            let (is_grom, page, size) = if index < GROM_0 {
                (false, index, ROM_BANK_SIZE)
            } else {
                (true, index - GROM_0, GROM_BANK_SIZE)
            };
            let num_banks = read_u8(data, &mut p)?;
            for _ in 0..num_banks {
                let bank_type = read_u8(data, &mut p)?;
                let bank = if bank_type == BANK_ROM {
                    rle_decode(data, &mut p, size)?
                } else {
                    // RAM / battery-backed banks carry no data in the file.
                    vec![0u8; size]
                };
                if is_grom {
                    grom.push((page as u16 * GROM_BANK_SIZE as u16, bank));
                } else {
                    cpu.entry(page).or_default().push(bank);
                }
            }
        }

        // The cartridge ROM window `>6000–7FFF` is CPU page 6 (low 4 KiB) followed
        // by page 7 (high 4 KiB); assemble one 8 KiB image per bank.
        let zero = vec![0u8; ROM_BANK_SIZE];
        let low_banks = cpu.get(&6);
        let high_banks = cpu.get(&7);
        let rom_banks = low_banks
            .map_or(0, Vec::len)
            .max(high_banks.map_or(0, Vec::len));
        let mut rom = Vec::with_capacity(rom_banks * GROM_BANK_SIZE);
        for i in 0..rom_banks {
            let low = low_banks.and_then(|b| b.get(i)).unwrap_or(&zero);
            let high = high_banks.and_then(|b| b.get(i)).unwrap_or(&zero);
            rom.extend_from_slice(low);
            rom.extend_from_slice(high);
        }

        Ok(Cartridge {
            title,
            cru_base,
            rom,
            rom_banks,
            grom,
        })
    }
}

/// Serialize cartridge ROM (and optional GROM) into a `ti99sim` V1 `.ctg` image —
/// the inverse of [`Cartridge::parse`]. `rom` holds consecutive 8 KiB banks
/// (`rom.len()` must be a whole multiple of `2 * ROM_BANK_SIZE`); each bank is
/// split into its low (CPU page 6) and high (CPU page 7) 4 KiB halves, in the
/// order `parse` reassembles. `grom` pages are emitted as 8 KiB regions at their
/// GROM addresses. Every page is stored as a single RLE literal run, which
/// round-trips through [`rle_decode`].
pub fn write_v1(title: &str, cru_base: u16, rom: &[u8], grom: &[(u16, Vec<u8>)]) -> Vec<u8> {
    assert!(
        rom.len().is_multiple_of(2 * ROM_BANK_SIZE),
        "rom length must be a whole number of 8 KiB banks"
    );
    let banks = rom.len() / (2 * ROM_BANK_SIZE);

    let mut out = Vec::new();
    out.extend_from_slice(BANNER);
    out.extend_from_slice(title.as_bytes());
    out.push(0x0A);
    out.push(0x1A);
    out.resize(0x50, 0x00); // zero-pad the banner to 80 bytes
    out.push(0x10); // version marker: high nibble 1 => V1
    out.push((cru_base >> 8) as u8);
    out.push(cru_base as u8);

    // CPU ROM: page 6 carries every bank's low half, then page 7 every high half.
    if banks > 0 {
        for (index, half) in [(6u8, 0usize), (7u8, ROM_BANK_SIZE)] {
            out.push(index);
            out.push(banks as u8);
            for b in 0..banks {
                let start = b * (2 * ROM_BANK_SIZE) + half;
                out.push(BANK_ROM);
                rle_literal(&mut out, &rom[start..start + ROM_BANK_SIZE]);
            }
        }
    }
    // GROM pages, one region each.
    for (addr, page) in grom {
        out.push(GROM_0 + (addr / GROM_BANK_SIZE as u16) as u8);
        out.push(1);
        out.push(BANK_ROM);
        rle_literal(&mut out, page);
    }
    out
}

/// Encode one page as a single RLE literal run (`ti99sim` `compress.cpp` format):
/// a little-endian count (bit 15 clear) followed by the bytes verbatim. Page sizes
/// (4 KiB / 8 KiB) fit the 15-bit literal count.
fn rle_literal(out: &mut Vec<u8>, page: &[u8]) {
    let count = page.len();
    debug_assert!(count > 0 && count < 0x8000);
    out.push((count & 0xFF) as u8);
    out.push((count >> 8) as u8);
    out.extend_from_slice(page);
}

/// Pull the title out of the banner: the bytes after `"TI-99/4A Module - "` up to
/// the trailing `\n` (or NUL padding).
fn parse_title(data: &[u8]) -> String {
    let start = BANNER.len();
    let end = data[start..0x50]
        .iter()
        .position(|&b| b == 0x0A || b == 0x00)
        .map_or(0x50, |i| start + i);
    String::from_utf8_lossy(&data[start..end])
        .trim()
        .to_string()
}

fn read_u8(data: &[u8], p: &mut usize) -> Result<u8, CartridgeError> {
    let b = *data.get(*p).ok_or(CartridgeError::Truncated)?;
    *p += 1;
    Ok(b)
}

/// Decode one RLE-compressed bank of exactly `size` bytes, advancing `*p`.
///
/// `ti99sim` `compress.cpp`: read a **little-endian** 16-bit tag. If bit 15 is
/// set it is a *run* — one following byte repeated `tag & >7FFF` times; otherwise
/// it is a *literal* — the next `tag` bytes verbatim (a zero tag is illegal).
/// Records repeat until the bank is full.
fn rle_decode(data: &[u8], p: &mut usize, size: usize) -> Result<Vec<u8>, CartridgeError> {
    let mut out = Vec::with_capacity(size);
    while out.len() < size {
        let lo = read_u8(data, p)? as u16;
        let hi = read_u8(data, p)? as u16;
        let tag = lo | (hi << 8);
        if tag & 0x8000 != 0 {
            let count = (tag & 0x7FFF) as usize;
            let ch = read_u8(data, p)?;
            if out.len() + count > size {
                return Err(CartridgeError::BadRle);
            }
            out.resize(out.len() + count, ch);
        } else {
            let count = tag as usize;
            if count == 0 || out.len() + count > size {
                return Err(CartridgeError::BadRle);
            }
            let end = p.checked_add(count).ok_or(CartridgeError::Truncated)?;
            let bytes = data.get(*p..end).ok_or(CartridgeError::Truncated)?;
            out.extend_from_slice(bytes);
            *p = end;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_v1_round_trips_single_bank() {
        let mut rom = vec![0u8; 2 * ROM_BANK_SIZE];
        rom[0] = 0xAA; // low-page marker (>6000)
        rom[ROM_BANK_SIZE] = 0x55; // high-page marker (>7000)
        rom[2 * ROM_BANK_SIZE - 1] = 0x99;
        let bytes = write_v1("HELLO", 0, &rom, &[]);
        let c = Cartridge::parse(&bytes).unwrap();
        assert_eq!(c.title, "HELLO");
        assert_eq!(c.cru_base, 0);
        assert_eq!(c.rom_banks, 1);
        assert_eq!(c.rom, rom);
        assert!(c.grom.is_empty());
    }

    #[test]
    fn write_v1_round_trips_two_banks_with_cru_base() {
        let mut rom = vec![0u8; 4 * ROM_BANK_SIZE];
        for (i, b) in rom.iter_mut().enumerate() {
            *b = (i % 251) as u8; // deterministic, distinguishes every page
        }
        let bytes = write_v1("X", 0x1234, &rom, &[]);
        let c = Cartridge::parse(&bytes).unwrap();
        assert_eq!(c.cru_base, 0x1234);
        assert_eq!(c.rom_banks, 2);
        assert_eq!(c.rom, rom);
    }
}
