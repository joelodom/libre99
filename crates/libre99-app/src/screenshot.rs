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

//! A tiny, dependency-free **PNG encoder** for screenshots.
//!
//! The frontend deliberately avoids an image crate, so this writes PNG by hand.
//! It uses *stored* (uncompressed) DEFLATE blocks inside the zlib stream — a
//! valid, universally-readable PNG that needs no compressor. Screenshots of a
//! 256×192 frame are ~150 KB, which is fine for the convenience. Color type is
//! truecolor (RGB, 8-bit); the core's framebuffer words are `0x00RRGGBB`.

/// Encode a `width × height` framebuffer (`0x00RRGGBB` words) as a PNG file.
pub fn encode_png(width: usize, height: usize, pixels: &[u32]) -> Vec<u8> {
    let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]; // signature

    // IHDR: dimensions, 8-bit depth, color type 2 (truecolor), no interlace.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &ihdr);

    // Raw image data: each scanline prefixed with filter byte 0 ("none").
    let mut raw = Vec::with_capacity(height * (1 + width * 3));
    for row in pixels.chunks(width).take(height) {
        raw.push(0);
        for &px in row {
            raw.push((px >> 16) as u8);
            raw.push((px >> 8) as u8);
            raw.push(px as u8);
        }
    }
    chunk(&mut png, b"IDAT", &zlib_stored(&raw));
    chunk(&mut png, b"IEND", &[]);
    png
}

/// Append a PNG chunk: length, type, data, and a CRC over (type ++ data).
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = Crc::new();
    crc.update(kind);
    crc.update(data);
    out.extend_from_slice(&crc.finish().to_be_bytes());
}

/// Wrap `data` in a zlib stream of *stored* (uncompressed) DEFLATE blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01]; // zlib header (32K window, no preset dict)
    if data.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
    } else {
        let mut i = 0;
        while i < data.len() {
            let n = (data.len() - i).min(0xFFFF);
            let last = i + n >= data.len();
            out.push(last as u8); // BFINAL bit, BTYPE = 00 (stored)
            out.extend_from_slice(&(n as u16).to_le_bytes());
            out.extend_from_slice(&(!(n as u16)).to_le_bytes());
            out.extend_from_slice(&data[i..i + n]);
            i += n;
        }
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// The zlib Adler-32 running checksum of `data`.
fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

/// The standard PNG/zlib CRC-32 (reflected, polynomial `0xEDB88320`).
struct Crc(u32);

impl Crc {
    fn new() -> Self {
        Crc(0xFFFF_FFFF)
    }
    fn update(&mut self, data: &[u8]) {
        for &byte in data {
            self.0 ^= byte as u32;
            for _ in 0..8 {
                self.0 = if self.0 & 1 != 0 {
                    (self.0 >> 1) ^ 0xEDB8_8320
                } else {
                    self.0 >> 1
                };
            }
        }
    }
    fn finish(self) -> u32 {
        self.0 ^ 0xFFFF_FFFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_and_adler_match_known_vectors() {
        // CRC-32 of "IEND" is the well-known PNG end-chunk CRC 0xAE426082.
        let mut c = Crc::new();
        c.update(b"IEND");
        assert_eq!(c.finish(), 0xAE42_6082);
        // Adler-32 of "Wikipedia" is 0x11E60398 (the canonical example).
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn encodes_a_well_formed_png() {
        let (w, h) = (4, 3);
        let pixels: Vec<u32> = (0..(w * h) as u32).map(|i| i * 0x0011_2233).collect();
        let png = encode_png(w, h, &pixels);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        // IHDR length (13) + type at bytes 8..16; ends with an IEND chunk.
        assert_eq!(&png[12..16], b"IHDR");
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }
}
