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

//! Staleness gate for the committed console-GROM artifact.
//!
//! The mirror of `libre99-asm`'s `committed_bin.rs`: tests build the GROM rewrite
//! from source in memory, but the emulator's `--system-grom` path loads the
//! **committed** `original-content/system-roms/grom/console-grom.bin`. This
//! test fails the build if that artifact and a fresh build of `console.gpl`
//! (plus its spliced data blocks) ever disagree.
//! (QUALITY-EVALUATION-2026-07-05.md, action item P4.2.)

use libre99_gpl::system_grom::build_console_grom;

const COMMITTED: &[u8] =
    include_bytes!("../../../original-content/system-roms/grom/console-grom.bin");

#[test]
fn committed_console_grom_bin_matches_a_fresh_build_of_console_gpl() {
    let built = build_console_grom().expect("console.gpl must assemble cleanly");
    assert_eq!(built.len(), COMMITTED.len(), "image size drifted");
    if built != COMMITTED {
        let first = built
            .iter()
            .zip(COMMITTED)
            .position(|(a, b)| a != b)
            .unwrap();
        panic!(
            "committed console-grom.bin is stale: first difference at GROM >{first:04X} \
             (built {:02X}, committed {:02X}). Rebuild it with \
             `cargo run -p libre99-gpl -- console original-content/system-roms/grom/console-grom.bin` \
             and commit the refreshed binary alongside the source change.",
            built[first], COMMITTED[first]
        );
    }
}
