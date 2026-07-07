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

//! Media files loaded at run time — no cartridge or disk image is embedded in
//! the binary. A cartridge (`.ctg`) or disk (`.dsk`) comes from an explicit
//! user-given path: the `--cartridge` / `--disk` command-line flags or the
//! OS-native file chooser ([`pick_media_file`], on `F9`). This module owns the
//! chooser, the file-type detection, the size guard, and the read-and-validate
//! step both entry points share.

use std::path::{Path, PathBuf};

use libre99_core::cartridge::Cartridge;

/// Refuse to slurp files beyond this size (16 MiB). Real TI media is tiny — a
/// cartridge image tops out around half a megabyte and a floppy image around
/// three — so anything huge is a mis-pick in the browser, not media.
pub const MAX_MEDIA_BYTES: u64 = 16 * 1024 * 1024;

/// What kind of media a file is, judged by its extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Cartridge,
    Disk,
}

impl MediaKind {
    /// Short uppercase label for toasts and list rows.
    pub fn label(self) -> &'static str {
        match self {
            MediaKind::Cartridge => "CART",
            MediaKind::Disk => "DISK",
        }
    }
}

/// A loaded media file: where it came from and its bytes (kept in memory so a
/// warm machine rebuild re-mounts without re-reading the file).
#[derive(Clone, Debug)]
pub struct MediaItem {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

impl MediaItem {
    /// The file name, for the window title and toasts.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }
}

/// Ask the OS for a media file with its **native open dialog** (blocking —
/// the frame loop, and so the machine, pauses while it is up). Filtered to TI
/// media; opens in `start_dir`, the last directory a mount came from.
pub fn pick_media_file(start_dir: &Path) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .add_filter("TI media (*.ctg, *.dsk)", &["ctg", "dsk"])
        .add_filter("Cartridges (*.ctg)", &["ctg"])
        .add_filter("Disk images (*.dsk)", &["dsk"])
        .set_title("Mount TI media — cartridge (.ctg) or disk (.dsk)");
    if start_dir.is_dir() {
        dialog = dialog.set_directory(start_dir);
    }
    dialog.pick_file()
}

/// The media kind of `path` by extension (case-insensitive): `.ctg` is a
/// cartridge, `.dsk` a disk image; anything else is not mountable media.
pub fn kind_of(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?;
    if ext.eq_ignore_ascii_case("ctg") {
        Some(MediaKind::Cartridge)
    } else if ext.eq_ignore_ascii_case("dsk") {
        Some(MediaKind::Disk)
    } else {
        None
    }
}

/// Read and validate a media file. Errors are one-line, user-facing strings
/// (they go to the CLI on `--cartridge`/`--disk` and to the on-screen toast
/// from the browser); this function never panics on foreign input.
pub fn load(kind: MediaKind, path: &Path) -> Result<MediaItem, String> {
    let bytes = read_guarded(path)?;
    match kind {
        MediaKind::Cartridge => {
            // Parse up front so a bad pick is a message, not a dead machine.
            Cartridge::parse(&bytes)
                .map_err(|e| format!("not a usable cartridge image: {e:?} ({})", path.display()))?;
        }
        MediaKind::Disk => {
            if bytes.is_empty() {
                return Err(format!("empty disk image: {}", path.display()));
            }
        }
    }
    Ok(MediaItem {
        path: path.to_path_buf(),
        bytes,
    })
}

/// Read a file, refusing anything over [`MAX_MEDIA_BYTES`].
fn read_guarded(path: &Path) -> Result<Vec<u8>, String> {
    let meta =
        std::fs::metadata(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if meta.len() > MAX_MEDIA_BYTES {
        return Err(format!(
            "{} is {} MiB — too large to be TI media",
            path.display(),
            meta.len() / (1024 * 1024)
        ));
    }
    std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_is_judged_by_extension_case_insensitively() {
        assert_eq!(kind_of(Path::new("a/b/Game.ctg")), Some(MediaKind::Cartridge));
        assert_eq!(kind_of(Path::new("A.CTG")), Some(MediaKind::Cartridge));
        assert_eq!(kind_of(Path::new("Vol.Dsk")), Some(MediaKind::Disk));
        assert_eq!(kind_of(Path::new("vol.DSK")), Some(MediaKind::Disk));
        assert_eq!(kind_of(Path::new("readme.txt")), None);
        assert_eq!(kind_of(Path::new("no-extension")), None);
    }

    #[test]
    fn a_missing_file_is_an_error_message_not_a_panic() {
        let err = load(MediaKind::Disk, Path::new("no/such/file.dsk")).unwrap_err();
        assert!(err.contains("cannot read"), "{err}");
    }

    #[test]
    fn garbage_is_rejected_as_a_cartridge_but_named_clearly() {
        let dir = std::env::temp_dir();
        let path = dir.join("libre99-media-test-garbage.ctg");
        std::fs::write(&path, [0u8; 16]).unwrap();
        let err = load(MediaKind::Cartridge, &path).unwrap_err();
        assert!(err.contains("not a usable cartridge"), "{err}");
        let _ = std::fs::remove_file(&path);
    }
}
