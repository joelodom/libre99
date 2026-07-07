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

//! Extract the authentic 8x8 character set (codes >20..>5F) as loaded by the
//! genuine console ROM+GROM, and compare it pixel-by-pixel with ours. Bitmap
//! typefaces are not subject to copyright, so the rewrite reproduces this
//! character set faithfully. Prints, per glyph: the authentic bytes + art, a
//! DIFFERS flag vs our current font, and (at the end) a ready-to-paste Rust
//! table of the authentic glyphs. Also locates the font block inside the raw
//! GROM image so a test can read it directly.
//!
//! Run from the repo root: `cargo run -p libre99-gpl --example font_extract`.

use std::sync::LazyLock;

use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Vec<u8>> = LazyLock::new(|| require("roms/994AGROM.Bin"));

/// Load one third-party image at run time (`third-party/` is git-ignored; see
/// `libre99_core::third_party`), exiting with a notice when the media is absent.
fn require(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2)
    })
}

fn art(g: &[u8; 8]) -> Vec<String> {
    g.iter()
        .map(|&b| (0..8).map(|c| if b & (0x80 >> c) != 0 { '#' } else { '.' }).collect())
        .collect()
}

fn main() {
    let mut m = Machine::new(&CONSOLE_ROM, &AUTHENTIC_GROM);
    for _ in 0..180 { m.run_frame(); }
    let pt = ((m.vdp().register(4) & 0x07) as u16) * 0x800;

    // Pull >20..>5F out of the VDP pattern table.
    let mut auth = [[0u8; 8]; 64];
    for (i, glyph) in auth.iter_mut().enumerate() {
        let code = 0x20 + i as u16;
        for r in 0..8u16 {
            glyph[r as usize] = m.vdp().vram(pt + code * 8 + r);
        }
    }

    let mut nonblank = 0;
    let mut differ = 0;
    for (i, a) in auth.iter().enumerate() {
        let code = 0x20 + i as u8;
        if a.iter().any(|&b| b != 0) { nonblank += 1; }
        let ours = libre99_gpl::font::glyph(code);
        let d = ours != *a;
        if d && a.iter().any(|&b| b != 0) { differ += 1; }
        let shown = if code == b' ' { "sp".into() } else { (code as char).to_string() };
        let hex: Vec<String> = a.iter().map(|b| format!("{b:02X}")).collect();
        println!("{code:#04X} {shown:>2}  [{}]  {}", hex.join(" "), if d { "DIFFERS" } else { "same" });
        if a.iter().any(|&b| b != 0) {
            let la = art(a);
            let lo = art(&ours);
            for r in 0..8 { println!("     {}    {}", la[r], lo[r]); }
        }
    }
    println!("\n{nonblank}/64 authentic glyphs non-blank; {differ} differ from ours (of the non-blank)");

    // Locate the contiguous font block in the raw GROM (search for space..'A').
    let needle: Vec<u8> = (0x20u16..=0x41).flat_map(|c| auth[(c - 0x20) as usize]).collect();
    let at = AUTHENTIC_GROM.windows(needle.len()).position(|w| w == needle);
    match at {
        Some(off) => println!("authentic font block for >20.. starts at GROM >{off:04X}"),
        None => println!("font block not contiguous in GROM at the obvious offset"),
    }

    // Emit a ready-to-paste Rust table of the authentic glyphs.
    println!("\n// ---- paste into font.rs ----");
    println!("const GLYPHS: [[u8; 8]; COUNT] = [");
    for (i, a) in auth.iter().enumerate() {
        let code = 0x20 + i as u8;
        let shown = if code == b' ' { "space".into() } else { (code as char).to_string() };
        let hex: Vec<String> = a.iter().map(|b| format!("0x{b:02X}")).collect();
        println!("    [{}], // >{:02X} {}", hex.join(", "), code, shown);
    }
    println!("];");
}
