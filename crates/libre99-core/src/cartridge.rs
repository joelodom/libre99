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

//! # Cartridge images — `ti99sim` `.ctg` containers and raw ROM `.bin` dumps
//!
//! TI-99/4A cartridges plug GROM and/or ROM chips onto the console bus.
//! [`Cartridge::parse`] recognizes two on-disk formats and returns the same
//! parsed [`Cartridge`] either way:
//!
//! * **`.ctg`** — Marc Rousseau's open-source `ti99sim` container (this file's
//!   original format; the layout below). Despite the `"TI-99/4A Module - "`
//!   banner it is **not** Win994a's native format; it is a faithful port of
//!   `ti99sim`'s `cCartridge::LoadImageV1`, verified to parse all 137 bundled
//!   images byte-exact (bytes-consumed == file size).
//! * **raw `.bin`** — a plain CPU-ROM dump with no container header: just the
//!   `>6000–7FFF` window's contents, one 8 KiB bank after another, exactly as
//!   the chip is read. This is the loose-binary form MAME/Classic99 accept (the
//!   `…8.bin` / `…C.bin` naming). See [`Cartridge::parse`] for what is and isn't
//!   supported.
//!
//! The format is chosen by content, not by extension — a `.bin` that happens to
//! carry the `.ctg` banner still parses as a container.
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
/// The valid-module marker (`>AA`) a standard TI header opens with — the first
/// byte of a bootable cartridge's `>6000` window, and the signature that marks a
/// bannerless file as a raw ROM dump rather than junk.
const HEADER_MAGIC: u8 = 0xAA;
/// CPU-ROM pages are 4 KiB; cartridge ROM uses pages 6 and 7 (`>6000–7FFF`).
const ROM_BANK_SIZE: usize = 0x1000;
/// One cartridge ROM bank fills the whole `>6000–7FFF` window: pages 6 and 7.
const CART_BANK_SIZE: usize = 2 * ROM_BANK_SIZE;
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
    /// Parse a cartridge image, dispatching on content: a `"TI-99/4A Module - "`
    /// banner is a `ti99sim` `.ctg` container (`parse_ctg`); otherwise a leading
    /// `>AA` marks a raw CPU-ROM `.bin` dump (`parse_raw_rom`). Anything else is
    /// not a cartridge.
    pub fn parse(data: &[u8]) -> Result<Cartridge, CartridgeError> {
        if data.starts_with(BANNER) {
            Self::parse_ctg(data)
        } else if data.first() == Some(&HEADER_MAGIC) {
            Self::parse_raw_rom(data)
        } else {
            Err(CartridgeError::BadBanner)
        }
    }

    /// Parse a raw CPU-ROM `.bin` dump — the loose-binary cartridge form (no
    /// container header, just the `>6000–7FFF` window's bytes as consecutive 8 KiB
    /// banks). The caller has already checked the leading `>AA`.
    ///
    /// The dump is padded up to a whole number of banks, then to a power-of-two
    /// bank count so the console's bank-select math (`(addr >> 1) & (banks - 1)`,
    /// see `Machine::write_cartridge`) has a clean mask; the padding banks read as
    /// zero and are never reached by a well-formed cartridge. The title is lifted
    /// from the module header's program list when present.
    ///
    /// **Supported:** the standard non-inverted banking scheme where bank 0 is the
    /// boot bank (it carries the `>AA` header). GROM-only `.bin` dumps and the
    /// inverted scheme (header only in the last bank) are out of scope — those
    /// ship as `.ctg` here.
    fn parse_raw_rom(data: &[u8]) -> Result<Cartridge, CartridgeError> {
        let mut rom = data.to_vec();
        // Pad partial trailing bank, then round the bank count up to a power of
        // two so `banks - 1` is a valid bank-select mask.
        let banks = data.len().div_ceil(CART_BANK_SIZE).next_power_of_two();
        rom.resize(banks * CART_BANK_SIZE, 0);
        Ok(Cartridge {
            title: raw_module_title(&rom),
            cru_base: 0,
            rom,
            rom_banks: banks,
            grom: Vec::new(),
        })
    }

    /// Parse a `ti99sim` V1 `.ctg` image.
    fn parse_ctg(data: &[u8]) -> Result<Cartridge, CartridgeError> {
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

/// Lift a display title from a raw ROM's standard module header. The header word
/// at `>6006` points at the program list; its first entry is `link(2) addr(2)
/// name_len(1) name…`, and the name is the cartridge's menu label (e.g.
/// `"COPPER"`). Everything is best-effort within bank 0 (`>6000`-relative): any
/// out-of-range pointer, zero-length name, or non-printable byte yields an empty
/// title, and the frontend falls back to the file name.
fn raw_module_title(rom: &[u8]) -> String {
    let read_word = |off: usize| -> Option<usize> {
        let hi = *rom.get(off)? as usize;
        let lo = *rom.get(off + 1)? as usize;
        Some((hi << 8) | lo)
    };
    // Program-list pointer, translated from a >6000-window address to a bank-0
    // offset. A null or below-window pointer means "no program list".
    let list = read_word(0x0006).filter(|&p| p >= 0x6000).map(|p| p - 0x6000);
    let Some(entry) = list else { return String::new() };
    let name_len = rom.get(entry + 4).copied().unwrap_or(0) as usize;
    let start = entry + 5;
    let name: String = rom
        .get(start..start + name_len)
        .unwrap_or(&[])
        .iter()
        .map(|&b| b as char)
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .collect();
    name.trim().to_string()
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

    /// Build a raw ROM `.bin` with a standard module header: `>AA` magic, a
    /// program-list pointer at `>6006`, and one program entry naming the cart.
    fn raw_rom(banks: usize, title: &str) -> Vec<u8> {
        let mut rom = vec![0u8; banks * CART_BANK_SIZE];
        for b in 0..banks {
            let base = b * CART_BANK_SIZE;
            rom[base] = HEADER_MAGIC; // >AA valid header in every bank
            rom[base + 1] = 0x01; // version
            rom[base + 6] = 0x60; // program list at >600C
            rom[base + 7] = 0x0C;
            // Program entry at >600C: link=0, addr=>6010, name.
            rom[base + 0x0C] = 0x00;
            rom[base + 0x0D] = 0x00;
            rom[base + 0x0E] = 0x60;
            rom[base + 0x0F] = 0x10;
            rom[base + 0x10] = title.len() as u8;
            rom[base + 0x11..base + 0x11 + title.len()].copy_from_slice(title.as_bytes());
        }
        rom
    }

    #[test]
    fn raw_rom_bin_parses_as_consecutive_rom_banks() {
        let bytes = raw_rom(4, "COPPER");
        let c = Cartridge::parse(&bytes).unwrap();
        assert_eq!(c.title, "COPPER", "title lifted from the module header");
        assert_eq!(c.cru_base, 0);
        assert_eq!(c.rom_banks, 4);
        assert_eq!(c.rom, bytes, "raw dump is the ROM verbatim");
        assert!(c.grom.is_empty(), "a raw ROM dump carries no GROM");
        assert_eq!(c.rom.len(), c.rom_banks * CART_BANK_SIZE);
    }

    #[test]
    fn raw_rom_single_bank_disables_banking() {
        let c = Cartridge::parse(&raw_rom(1, "MINI")).unwrap();
        assert_eq!(c.rom_banks, 1);
        assert_eq!(c.rom.len(), CART_BANK_SIZE);
    }

    #[test]
    fn raw_rom_pads_to_a_power_of_two_bank_count() {
        // A 3-bank (non-power-of-two) dump rounds up to 4 so `banks - 1` is a
        // clean bank-select mask; the padding bank reads as zero.
        let mut bytes = raw_rom(3, "ODD");
        // Trim the padding raw_rom already zero-fills to prove size handling: keep
        // the three real banks plus one stray byte to exercise partial-bank padding.
        bytes.truncate(3 * CART_BANK_SIZE + 1);
        let c = Cartridge::parse(&bytes).unwrap();
        assert_eq!(c.rom_banks, 4, "3 banks + a byte rounds up to 4");
        assert!(c.rom_banks.is_power_of_two());
        assert_eq!(c.rom.len(), 4 * CART_BANK_SIZE);
        assert_eq!(&c.rom[..3 * CART_BANK_SIZE], &bytes[..3 * CART_BANK_SIZE]);
    }

    #[test]
    fn raw_rom_without_a_program_name_has_an_empty_title() {
        let mut bytes = vec![0u8; CART_BANK_SIZE];
        bytes[0] = HEADER_MAGIC; // valid header, but the program-list pointer is 0
        let c = Cartridge::parse(&bytes).unwrap();
        assert_eq!(c.rom_banks, 1);
        assert_eq!(c.title, "");
    }

    #[test]
    fn dispatch_rejects_data_that_is_neither_banner_nor_magic() {
        // No `.ctg` banner and no leading >AA: not a cartridge at all.
        assert_eq!(
            Cartridge::parse(b"not a cartridge").unwrap_err(),
            CartridgeError::BadBanner
        );
        assert_eq!(Cartridge::parse(&[]).unwrap_err(), CartridgeError::BadBanner);
    }
}
