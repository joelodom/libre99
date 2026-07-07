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

//! Render OUR rewritten title + menu to PNG screenshots so the recreation can be
//! eyeballed with real colours. Writes to the path(s) given as CLI args (or the
//! temp dir). Run: `cargo run -p libre99-gpl --example title_shot -- title.png menu.png`.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;
use libre99_core::vdp::{HEIGHT, WIDTH};

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    libre99_core::third_party::load("roms/994aROM.Bin").unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/roms/994aROM.Bin)");
        std::process::exit(2)
    })
});

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut typed = kind.to_vec();
    typed.extend_from_slice(data);
    out.extend_from_slice(&typed);
    out.extend_from_slice(&crc32(&typed).to_be_bytes());
}

/// Encode an RGB framebuffer as an uncompressed (stored-DEFLATE) PNG.
fn png(fb: &[u32], w: usize, h: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity(h * (1 + w * 3));
    for y in 0..h {
        raw.push(0); // filter: none
        for x in 0..w {
            let px = fb[y * w + x];
            raw.push((px >> 16) as u8);
            raw.push((px >> 8) as u8);
            raw.push(px as u8);
        }
    }
    // zlib stream: header, stored blocks, adler32.
    let mut zlib = vec![0x78, 0x01];
    let mut i = 0;
    while i < raw.len() {
        let n = (raw.len() - i).min(0xFFFF);
        zlib.push(if i + n >= raw.len() { 1 } else { 0 });
        zlib.extend_from_slice(&(n as u16).to_le_bytes());
        zlib.extend_from_slice(&(!(n as u16)).to_le_bytes());
        zlib.extend_from_slice(&raw[i..i + n]);
        i += n;
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8-bit, truecolour RGB
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn shot(m: &mut Machine, path: &str, scale: usize) {
    let mut fb = vec![0u32; WIDTH * HEIGHT];
    m.render(&mut fb);
    // Nearest-neighbour upscale for a crisper screenshot.
    let (sw, sh) = (WIDTH * scale, HEIGHT * scale);
    let mut big = vec![0u32; sw * sh];
    for y in 0..sh {
        for x in 0..sw {
            big[y * sw + x] = fb[(y / scale) * WIDTH + x / scale];
        }
    }
    std::fs::write(path, png(&big, sw, sh)).unwrap();
    eprintln!("wrote {path} ({sw}x{sh})");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let title_path = args.first().cloned().unwrap_or_else(|| {
        std::env::temp_dir().join("title.png").to_string_lossy().into_owned()
    });
    let menu_path = args.get(1).cloned().unwrap_or_else(|| {
        std::env::temp_dir().join("menu.png").to_string_lossy().into_owned()
    });
    let scale: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);

    let grom = libre99_gpl::system_grom::build_console_grom().expect("assemble");
    let cart = ["cartridges/Parsec.ctg", "cartridges/centipe.ctg", "cartridges/tundoom.ctg"]
        .iter().find_map(|p| libre99_core::third_party::load(p))
        .map(|d| Cartridge::parse(&d).unwrap());
    let mut m = Machine::new(&CONSOLE_ROM, &grom);
    if let Some(c) = &cart { m.mount_cartridge(c); m.reset(); }
    for _ in 0..180 { m.run_frame(); }
    shot(&mut m, &title_path, scale);

    m.set_key(TiKey::Space, true);
    for _ in 0..4 { m.run_frame(); }
    m.set_key(TiKey::Space, false);
    for _ in 0..150 { m.run_frame(); }
    shot(&mut m, &menu_path, scale);
}
