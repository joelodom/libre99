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

//! Staleness gate for the committed console-ROM artifact.
//!
//! Every other test assembles the rewrite from source in memory, but the
//! emulator's `--system-rom` path loads the **committed**
//! `original-content/system-roms/rom/console-rom.bin`. Nothing else ties that
//! artifact to `console.asm`, so an edit that isn't followed by a rebuild
//! (`libre99asm rom …`) would ship a stale binary silently. This test is the tie:
//! it fails the build the moment the committed bytes and a fresh build of the
//! source disagree. (QUALITY-EVALUATION-2026-07-05.md, action item P4.2.)

use libre99_asm::system_rom::build_console_rom;

const COMMITTED: &[u8] =
    include_bytes!("../../../original-content/system-roms/rom/console-rom.bin");

#[test]
fn committed_console_rom_bin_matches_a_fresh_build_of_console_asm() {
    let built = build_console_rom().expect("console.asm must assemble cleanly");
    assert_eq!(built.len(), COMMITTED.len(), "image size drifted");
    if built != COMMITTED {
        let first = built
            .iter()
            .zip(COMMITTED)
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "committed console-rom.bin is stale: first difference at >{first:04X} \
             (built {:02X}, committed {:02X}). Rebuild it with \
             `cargo run -p libre99-asm -- rom original-content/system-roms/rom/console-rom.bin` \
             and commit the refreshed binary alongside the source change.",
            built[first], COMMITTED[first]
        );
    }
}
