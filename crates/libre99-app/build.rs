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

//! Build script: stamp the build identity (commit + date) the system
//! information screen shows. The binary embeds **only the project's own
//! clean-room firmware** (via `include_bytes!` in `assets.rs`); no cartridge,
//! disk, or third-party image is baked in — media loads at run time from
//! user-given paths.

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let root = Path::new(&manifest).parent().unwrap().parent().unwrap();
    emit_build_identity(root);
}

/// Emit the commit id and build date the system information screen shows
/// (stamped into the Libre99 GROM's identification block — `src/sysinfo.rs`).
/// The date is the **commit** date, not wall clock, so identical sources build
/// identical binaries. A `+` on the commit marks an unclean tree. Both fall
/// back to `UNKNOWN` when git is unavailable (e.g. a source tarball).
fn emit_build_identity(root: &Path) {
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git").args(args).current_dir(root).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    };

    let commit = match git(&["rev-parse", "--short=7", "HEAD"]) {
        Some(hash) if !hash.is_empty() => {
            let dirty = git(&["status", "--porcelain"]).is_some_and(|s| !s.is_empty());
            if dirty { format!("{hash}+") } else { hash }
        }
        _ => "UNKNOWN".to_string(),
    };
    let date = match git(&["log", "-1", "--format=%cs"]) {
        Some(d) if !d.is_empty() => d,
        _ => "UNKNOWN".to_string(),
    };

    println!("cargo:rustc-env=LIBRE99_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=LIBRE99_BUILD_DATE={date}");
    // Re-run when the checked-out commit moves. (The dirty flag is best-effort:
    // it refreshes on rebuilds, which edits to tracked sources cause anyway.)
    println!("cargo:rerun-if-changed={}", root.join(".git/HEAD").display());
    println!("cargo:rerun-if-changed={}", root.join(".git/index").display());
}
