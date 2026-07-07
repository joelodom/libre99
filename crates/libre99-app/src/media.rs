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

/// A loaded media file: where it came from and its bytes. A cartridge item
/// keeps its bytes so the cold-boot rebuild a cartridge change requires can
/// re-mount without re-reading the file; a *disk* item hands its bytes to the
/// machine at mount time (the in-memory image lives there from then on) and
/// keeps only the path, for the window title and the exit bookkeeping.
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

/// Ask the OS where to write an exported disk image with its **native save
/// dialog** (blocking, like [`pick_media_file`]). The dialog itself asks
/// "replace existing file?" when the user picks a name that exists — that is
/// the app's overwrite guarantee: no host `.dsk` is ever overwritten without
/// that prompt. The returned path is written **exactly as the dialog returned
/// it** (no extension fix-ups afterwards, which would dodge the check the OS
/// just performed on the name the user confirmed).
pub fn save_dsk_file(start_dir: &Path, suggested_name: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .add_filter("Disk images (*.dsk)", &["dsk"])
        .set_title("Export disk image (.dsk)")
        .set_file_name(suggested_name);
    if start_dir.is_dir() {
        dialog = dialog.set_directory(start_dir);
    }
    dialog.save_file()
}

/// The user's answer to the native "unsaved disk changes" prompt shown before
/// unloading a modified in-memory disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnloadChoice {
    /// Export the image to a `.dsk` file first, then unload.
    Save,
    /// Unload without exporting — the in-memory changes are discarded.
    Discard,
    /// Keep the disk in memory; nothing happens.
    Cancel,
}

/// Ask — with the OS's **native message dialog** — whether to export a
/// modified disk image before unloading it from memory (the point of no
/// return for its in-memory changes; the host file was never touched).
pub fn confirm_unload(name: &str) -> UnloadChoice {
    let result = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Unload disk from memory")
        .set_description(format!(
            "{name} has in-memory changes that have not been exported.\n\n\
             Save it to a .dsk file before unloading?\n\n\
             Yes: choose where to save, then unload.\n\
             No: unload and discard the changes (the original file on disk is untouched).\n\
             Cancel: keep the disk in memory."
        ))
        .set_buttons(rfd::MessageButtons::YesNoCancel)
        .show();
    match result {
        rfd::MessageDialogResult::Yes => UnloadChoice::Save,
        rfd::MessageDialogResult::No => UnloadChoice::Discard,
        _ => UnloadChoice::Cancel,
    }
}

/// Ask the OS where to write a snapshot — a user-named save state — with its
/// **native save dialog**. As with [`save_dsk_file`], the dialog's own
/// replace-prompt is the overwrite guarantee, and the returned path is
/// written exactly as returned (no extension fix-ups afterwards).
pub fn save_snapshot_file(start_dir: &Path, suggested_name: &str) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .add_filter("Libre99 save states (*.ti99)", &["ti99"])
        .set_title("Save snapshot (.ti99)")
        .set_file_name(suggested_name);
    if start_dir.is_dir() {
        dialog = dialog.set_directory(start_dir);
    }
    dialog.save_file()
}

/// Ask the OS for a snapshot file to load with its **native open dialog**
/// (blocking, like [`pick_media_file`]).
pub fn pick_snapshot_file(start_dir: &Path) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new()
        .add_filter("Libre99 save states (*.ti99)", &["ti99"])
        .set_title("Load snapshot (.ti99)");
    if start_dir.is_dir() {
        dialog = dialog.set_directory(start_dir);
    }
    dialog.pick_file()
}

/// The warning shown before a snapshot load (pure, so it is unit-testable;
/// the dialog itself is native): loading replaces the running machine *and*
/// the resume state — plus what that costs when the session holds unexported
/// disk changes the snapshot won't have.
pub fn snapshot_load_warning(name: &str, dirty_disks: usize) -> String {
    let mut text = format!(
        "Load the snapshot {name}?\n\n\
         It replaces the running machine and becomes the resume state — the \
         automatic save Libre99 reloads at startup — so the session running \
         now is overwritten."
    );
    if dirty_disks > 0 {
        text.push_str(&format!(
            "\n\nThis session holds {dirty_disks} in-memory disk image(s) with \
             unexported changes; loading the snapshot discards them. Export \
             them first (F4) to keep them."
        ));
    }
    text
}

/// Confirm a snapshot load with the OS's **native message dialog**.
pub fn confirm_snapshot_load(name: &str, dirty_disks: usize) -> bool {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Load snapshot")
        .set_description(snapshot_load_warning(name, dirty_disks))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

/// The warning shown before Load (`F8`) *only* when the session holds
/// unexported in-memory disk changes that the resume state would roll back
/// (pure, for tests). A clean session loads without a prompt.
pub fn resume_load_warning(dirty_disks: usize) -> String {
    format!(
        "Load the resume state?\n\n\
         This session holds {dirty_disks} in-memory disk image(s) with \
         unexported changes; loading rolls every disk back to the resume \
         state's copy. Export them first (F4) to keep them."
    )
}

/// Confirm rolling back unexported disk changes with the OS's **native
/// message dialog** (only shown when there are some — see
/// [`resume_load_warning`]).
pub fn confirm_resume_load(dirty_disks: usize) -> bool {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Load resume state")
        .set_description(resume_load_warning(dirty_disks))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

/// The warning shown before a fresh start (pure, for tests): the resume
/// state is deleted permanently and the console restarts bare, spelling out
/// exactly what is lost — every in-memory disk image, and unexported changes
/// in particular. Host files are never touched.
pub fn fresh_start_warning(disks: usize, dirty_disks: usize) -> String {
    let mut text = String::from(
        "Delete the resume state and start fresh?\n\n\
         The resume state is deleted permanently, and the console restarts \
         bare — no cartridge, no disk — as on a first run.",
    );
    if disks > 0 {
        text.push_str(&format!(
            "\n\nThe {disks} disk image(s) held in memory are unloaded"
        ));
        if dirty_disks > 0 {
            text.push_str(&format!(
                " — {dirty_disks} of them carry unexported changes that will be \
                 lost. Export them first (F4) to keep them"
            ));
        }
        text.push('.');
    }
    text.push_str(
        "\n\nFiles on your computer (.dsk, .ctg, snapshot .ti99) are never touched.",
    );
    text
}

/// Confirm a fresh start with the OS's **native message dialog**.
pub fn confirm_fresh_start(disks: usize, dirty_disks: usize) -> bool {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Warning)
        .set_title("Fresh start")
        .set_description(fresh_start_warning(disks, dirty_disks))
        .set_buttons(rfd::MessageButtons::YesNo)
        .show()
        == rfd::MessageDialogResult::Yes
}

/// The host identity a media file is remembered by — disks across ejects, in
/// save states, and in the disk-memory overlay; cartridges in save states
/// (format v3): the canonicalized absolute path when the file resolves (so
/// case and relative-vs-absolute spellings of the same file collapse to one
/// identity), else the path as given. Windows' canonical form carries the
/// `\\?\` verbatim prefix; it is stripped so keys stay readable (the key is
/// an identity and display string, never re-opened as a path).
pub fn file_key(path: &Path) -> String {
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = canon.display().to_string();
    match s.strip_prefix(r"\\?\") {
        Some(stripped) => stripped.to_string(),
        None => s,
    }
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
    fn file_key_collapses_spellings_of_the_same_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("libre99-media-test-key.dsk");
        std::fs::write(&path, [0u8; 4]).unwrap();
        let key = file_key(&path);
        // A redundant `.` component resolves to the same identity.
        assert_eq!(file_key(&dir.join(".").join("libre99-media-test-key.dsk")), key);
        assert!(!key.starts_with(r"\\?\"), "verbatim prefix leaked into the key: {key}");
        let _ = std::fs::remove_file(&path);
        // A file that doesn't resolve still yields a stable identity.
        assert_eq!(
            file_key(Path::new("no/such.dsk")),
            Path::new("no/such.dsk").display().to_string()
        );
    }

    #[test]
    fn the_snapshot_warning_names_the_file_and_the_resume_state() {
        let clean = snapshot_load_warning("parsec.ti99", 0);
        assert!(clean.contains("parsec.ti99"), "{clean}");
        assert!(clean.contains("resume state"), "{clean}");
        assert!(!clean.contains("unexported"), "a clean session must not warn: {clean}");
        let dirty = snapshot_load_warning("parsec.ti99", 2);
        assert!(dirty.contains("2 in-memory disk image(s)"), "{dirty}");
        assert!(dirty.contains("F4"), "{dirty}");
    }

    #[test]
    fn the_fresh_start_warning_spells_out_what_is_lost() {
        let bare = fresh_start_warning(0, 0);
        assert!(bare.contains("deleted permanently"), "{bare}");
        assert!(!bare.contains("disk image(s) held in memory"), "no disks, no disk line: {bare}");
        assert!(bare.contains("never touched"), "{bare}");

        let loaded = fresh_start_warning(3, 1);
        assert!(loaded.contains("3 disk image(s)"), "{loaded}");
        assert!(loaded.contains("1 of them carry unexported changes"), "{loaded}");
        assert!(loaded.contains("F4"), "{loaded}");

        // Disks in memory but none dirty: mention the unload, skip the alarm.
        let clean = fresh_start_warning(2, 0);
        assert!(clean.contains("2 disk image(s)"), "{clean}");
        assert!(!clean.contains("unexported"), "{clean}");
    }

    #[test]
    fn the_resume_load_warning_counts_the_dirty_disks() {
        let text = resume_load_warning(1);
        assert!(text.contains("1 in-memory disk image(s)"), "{text}");
        assert!(text.contains("rolls every disk back"), "{text}");
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
