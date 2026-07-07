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

//! Regenerate the README screenshot gallery (`docs/screenshots/*.png`).
//!
//! Boots the shipped clean-room firmware (the same committed artifacts the
//! desktop app embeds and boots by default), drives it and a few bundled
//! cartridges with scripted keys, and writes 4-bit-indexed PNGs at 2x scale.
//! Deterministic: the same tree produces the same bytes. Rerun whenever a
//! screenshot-visible surface changes (title screen, menu, TI PYTHON, …):
//!
//! ```text
//! cargo run -p libre99-gpl --example readme_gallery
//! ```
//!
//! The Parsec and Tunnels of Doom screenshots (`parsec.png`,
//! `tunnels-of-doom.png`) are static/historical: they exercise third-party
//! cartridges that are no longer bundled with the repo, so this tool no longer
//! regenerates them — the committed PNGs stay as-is.

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;
use libre99_core::sysinfo::{self as block, HostStamp};
use libre99_core::vdp::{HEIGHT, PALETTE, WIDTH};

/// The clean-room firmware the emulator boots by default (committed artifacts).
const CLEAN_ROM: &[u8] =
    include_bytes!("../../../original-content/system-roms/rom/console-rom.bin");
const CLEAN_GROM: &[u8] =
    include_bytes!("../../../original-content/system-roms/grom/console-grom.bin");

/// Bundled cartridges shown in the gallery.
const TITRIS: &[u8] = include_bytes!("../../../original-content/cartridges/titris/titris.ctg");
const SOKOBAN: &[u8] = include_bytes!("../../../original-content/cartridges/sokoban/sokoban.ctg");
const JAYWALKER99: &[u8] = include_bytes!("../../../original-content/cartridges/jaywalker99/jaywalker99.ctg");

// ---------------------------------------------------------------------------
// PNG writing: 4-bit indexed color over the fixed TMS9918A palette, stored
// (uncompressed) DEFLATE — small, dependency-free, deterministic.
// ---------------------------------------------------------------------------

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

/// Encode a palette-index image (one index byte per pixel) as an indexed PNG.
fn png_indexed(idx: &[u8], w: usize, h: usize) -> Vec<u8> {
    // Scanlines: filter byte 0, then two 4-bit indices per byte, high first.
    let mut raw = Vec::with_capacity(h * (1 + w / 2));
    for y in 0..h {
        raw.push(0);
        for x in (0..w).step_by(2) {
            raw.push((idx[y * w + x] << 4) | (idx[y * w + x + 1] & 0x0F));
        }
    }
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
    ihdr.extend_from_slice(&[4, 3, 0, 0, 0]); // 4-bit, indexed color
    chunk(&mut out, b"IHDR", &ihdr);
    let mut plte = Vec::with_capacity(48);
    for &c in &PALETTE {
        plte.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
    }
    chunk(&mut out, b"PLTE", &plte);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

/// Render the machine's frame, upscale, and write `docs/screenshots/<name>.png`.
fn shot(m: &mut Machine, name: &str) {
    const SCALE: usize = 2;
    let mut fb = vec![0u32; WIDTH * HEIGHT];
    m.render(&mut fb);
    // Map RGB back to palette indices (every rendered pixel is a PALETTE entry).
    let to_index = |px: u32| PALETTE.iter().position(|&p| p == px).unwrap_or(0) as u8;
    let (sw, sh) = (WIDTH * SCALE, HEIGHT * SCALE);
    let mut idx = vec![0u8; sw * sh];
    for y in 0..sh {
        for x in 0..sw {
            idx[y * sw + x] = to_index(fb[(y / SCALE) * WIDTH + x / SCALE]);
        }
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/screenshots");
    std::fs::create_dir_all(dir).expect("create docs/screenshots");
    let path = format!("{dir}/{name}.png");
    std::fs::write(&path, png_indexed(&idx, sw, sh)).expect("write PNG");
    eprintln!("wrote docs/screenshots/{name}.png ({sw}x{sh})");
}

// ---------------------------------------------------------------------------
// Machine driving.
// ---------------------------------------------------------------------------

fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame();
    }
}

fn tap(m: &mut Machine, k: TiKey, settle: usize) {
    m.set_key(k, true);
    frames(m, 3);
    m.set_key(k, false);
    frames(m, settle);
}

/// ASCII → (key, shift) for the TI PYTHON session (digits/operators/letters).
fn key_for(c: char) -> (TiKey, bool) {
    use TiKey::*;
    match c {
        '0'..='9' => {
            const DIGITS: [TiKey; 10] = [Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];
            (DIGITS[c as usize - '0' as usize], false)
        }
        ' ' => (Space, false),
        '=' => (Equals, false),
        '/' => (Slash, false),
        '+' => (Equals, true),
        '-' => (Slash, true),
        '*' => (Num8, true),
        '(' => (Num9, true),
        ')' => (Num0, true),
        'A'..='Z' => {
            const LETTERS: [TiKey; 26] = [
                A, B, C, D, E, F, G, H, I, J, K, L, M,
                N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
            ];
            (LETTERS[c as usize - 'A' as usize], false)
        }
        other => panic!("gallery session uses an unmapped key {other:?}"),
    }
}

fn type_line(m: &mut Machine, line: &str) {
    for c in line.chars() {
        let (k, shift) = key_for(c);
        if shift {
            m.set_key(TiKey::Shift, true);
        }
        m.set_key(k, true);
        frames(m, 3);
        m.set_key(k, false);
        if shift {
            m.set_key(TiKey::Shift, false);
        }
        frames(m, 3);
    }
    tap(m, TiKey::Enter, 40);
}

/// A clean-room GROM image with the system-information block stamped the way
/// the desktop app stamps it at launch (true version; host facts from git/OS).
fn stamped_grom() -> Vec<u8> {
    let mut grom = CLEAN_GROM.to_vec();
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let date = std::process::Command::new("git")
        .args(["log", "-1", "--format=%cs"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    // Fit the block's 12-char HOST field ("WINDOWS X64", "MACOS ARM64").
    let arch = match std::env::consts::ARCH {
        "x86_64" => "X64",
        "aarch64" => "ARM64",
        other => other,
    };
    let host = format!("{} {}", std::env::consts::OS.to_uppercase(), arch);
    let rom_id = block::rom_marker_version(CLEAN_ROM)
        .map(|v| format!("LIBRE99 ROM {v}"))
        .unwrap_or_else(|| "UNKNOWN".into());
    block::stamp(
        &mut grom,
        &HostStamp {
            emu_version: env!("CARGO_PKG_VERSION"),
            build_date: &date,
            commit: &commit,
            host: &host,
            rom_id: &rom_id,
        },
    );
    grom
}

/// Boot the default (clean-room) machine, optionally with a cartridge, and run
/// it to the master title screen.
fn boot(cart: Option<&[u8]>) -> Machine {
    let grom = stamped_grom();
    let mut m = Machine::new(CLEAN_ROM, &grom);
    if let Some(bytes) = cart {
        let c = Cartridge::parse(bytes).expect("parse cartridge");
        m.mount_cartridge(&c);
        m.reset();
    }
    frames(&mut m, 180);
    m
}

/// Title screen → master selection menu.
fn to_menu(m: &mut Machine) {
    tap(m, TiKey::Space, 220);
}

fn main() {
    // 1-2. The clean-room title screen, then the selection menu (Titris
    //      mounted so the menu shows an original cartridge entry).
    let mut m = boot(Some(TITRIS));
    shot(&mut m, "title");
    to_menu(&mut m);
    shot(&mut m, "menu");

    // 3. Titris (original cartridge, built by the bundled assembler): start a
    //    game and play a few moves so a piece is on the board.
    tap(&mut m, TiKey::Num2, 150); // "2 FOR TITRIS"
    tap(&mut m, TiKey::Space, 30); // leave the Titris title: start the game
    for _ in 0..3 {
        tap(&mut m, TiKey::Joy1Left, 10);
    }
    tap(&mut m, TiKey::Joy1Up, 10); // rotate
    frames(&mut m, 240); // let a couple of pieces fall
    shot(&mut m, "titris");

    // 4. Sokoban (the second original cartridge): start level 1 and make the
    //    first few moves of the solution, ending on a push.
    let mut m = boot(Some(SOKOBAN));
    to_menu(&mut m);
    tap(&mut m, TiKey::Num2, 150); // "2 FOR SOKOBAN"
    tap(&mut m, TiKey::Space, 30); // leave the Sokoban title: start level 1
    tap(&mut m, TiKey::Joy1Right, 10);
    tap(&mut m, TiKey::Joy1Down, 10);
    tap(&mut m, TiKey::Joy1Down, 10);
    tap(&mut m, TiKey::Joy1Left, 10); // push the loose box onto a spot
    frames(&mut m, 30);
    shot(&mut m, "sokoban");

    // 5. Jaywalker 99 (the third original cartridge): start a run and hop north
    //    few times so the camera has scrolled fresh lanes into view.
    let mut m = boot(Some(JAYWALKER99));
    to_menu(&mut m);
    tap(&mut m, TiKey::Num2, 150); // "2 FOR JAYWALKER 99"
    tap(&mut m, TiKey::Space, 30); // leave the Jaywalker 99 title: start a run
    for _ in 0..2 {
        tap(&mut m, TiKey::Joy1Up, 12); // hop north onto the meadow's edge
    }
    frames(&mut m, 30);
    shot(&mut m, "jaywalker99");

    // 6. TI PYTHON: launch from the menu and evaluate a tiny session.
    let mut m = boot(Some(TITRIS));
    to_menu(&mut m);
    tap(&mut m, TiKey::Num1, 60); // "1 FOR TI PYTHON"
    type_line(&mut m, "2 + 3 * 4");
    type_line(&mut m, "X = 7");
    type_line(&mut m, "X * 6");
    shot(&mut m, "ti-python");

    // 7. The system-information screen ((S) on the menu), stamped like the app.
    let mut m = boot(Some(TITRIS));
    to_menu(&mut m);
    tap(&mut m, TiKey::S, 90);
    shot(&mut m, "system-info");
}
