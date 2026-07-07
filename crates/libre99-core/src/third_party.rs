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

//! Development-only loader for **third-party media** — the authentic TI
//! firmware images and commercial cartridge/disk images that are *not* this
//! project's to redistribute. They live **outside version control** in
//! `third-party/` at the workspace root (git-ignored; `roms/`, `cartridges/`,
//! and `disks/` subdirectories), or wherever `$LIBRE99_THIRD_PARTY` points.
//!
//! The differential firmware suites, probe examples, and the book's bench tool
//! load these images **at run time** through this module and skip (or exit
//! with a message) when they are absent, so a public checkout builds and tests
//! green with zero proprietary bytes. Nothing in the emulator itself — the
//! `libre99-app` binary or this crate's runtime — reads these paths; the shipped
//! product loads media only from explicit user-given paths.

use std::path::PathBuf;

/// The third-party media directory, if it exists: `$LIBRE99_THIRD_PARTY` when
/// set, else `<workspace root>/third-party`. The workspace root is resolved
/// from this crate's compile-time manifest directory, which is correct for
/// the only intended users — in-repo tests, examples, and tools.
pub fn dir() -> Option<PathBuf> {
    let dir = match std::env::var_os("LIBRE99_THIRD_PARTY") {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../third-party"),
    };
    dir.is_dir().then_some(dir)
}

/// The bytes of `<third-party dir>/<rel>` (e.g. `roms/994aROM.Bin`), or `None`
/// when the directory or file is absent.
pub fn load(rel: &str) -> Option<Vec<u8>> {
    std::fs::read(dir()?.join(rel)).ok()
}

/// [`load`], announcing the skip on stderr when the image is unavailable, so
/// a green-but-skipped differential test says why. Callers do
/// `let Some(bytes) = third_party::load_or_skip("roms/994aROM.Bin") else { return };`
pub fn load_or_skip(rel: &str) -> Option<Vec<u8>> {
    let bytes = load(rel);
    if bytes.is_none() {
        eprintln!("SKIPPED: third-party media not present (third-party/{rel})");
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_image_is_none_not_a_panic() {
        assert!(load("roms/no-such-image-ever.Bin").is_none());
        assert!(load_or_skip("roms/no-such-image-ever.Bin").is_none());
    }
}
