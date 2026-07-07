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

//! Measure how many frames the console takes to reach the master title
//! screen — display enabled with ≥3 distinct framebuffer colors, the same
//! predicate as `tests/boot.rs`. Used to quantify GROM access-timing changes
//! (QUALITY-EVALUATION-2026-07-05.md, action item P2.1): run it before and
//! after a timing change and compare.
//!
//! ```sh
//! cargo run -p libre99-core --example boot_frames
//! ```

use libre99_core::machine::Machine;
use libre99_core::vdp::{HEIGHT, WIDTH};

/// Load one third-party image, or exit — this probe needs the authentic media.
fn need(rel: &str) -> Vec<u8> {
    libre99_core::third_party::load(rel).unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/{rel})");
        std::process::exit(2);
    })
}

fn main() {
    let rom = need("roms/994aROM.Bin");
    let grom = need("roms/994AGROM.Bin");
    let mut m = Machine::new(&rom, &grom);
    let mut fb = vec![0u32; WIDTH * HEIGHT];
    for frame in 1..=600 {
        m.run_frame();
        m.render(&mut fb);
        let display_on = m.vdp().register(1) & 0x40 != 0;
        let mut colors = fb.clone();
        colors.sort_unstable();
        colors.dedup();
        if display_on && colors.len() >= 3 {
            println!("title reached at frame {frame}");
            return;
        }
    }
    println!("title not reached within 600 frames");
}
