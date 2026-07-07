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

//! User preferences, stored as a small commented TOML file.
//!
//! Every user-specific file lives in **`~/.libre99/`** (created on first
//! run; see [`data_dir`] / [`ensure_data_dir`]): the preferences
//! (`libre99.toml`), the log (`libre99.log`), save states, and
//! screenshots. The TOML is created with documented defaults on first run, and
//! missing or malformed keys fall back to defaults rather than failing — after a
//! load the file is rewritten clean so new keys appear. Parsing reads a generic
//! TOML table (no derive macros), keeping the core dependency-light promise local
//! to the frontend.

use std::path::{Path, PathBuf};

/// Resolved user preferences.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub log_level: String,
    /// Path of the cartridge mounted when the session was last saved, written
    /// on exit ([`update_session`]); empty = bare console. On a
    /// resume-from-snapshot the frontend re-reads this file so the session's
    /// media survives a later in-app media change.
    pub last_cartridge: String,
    /// Path of the disk in DSK1 when the session was last saved (see
    /// [`last_cartridge`](Self::last_cartridge)); empty = none.
    pub last_disk: String,
    /// The directory the in-app file browser (`F9`) last mounted from; it opens
    /// there next time. Empty = fall back to the home directory.
    pub browser_dir: String,
    pub window_scale: u32,
    pub fullscreen: bool,
    pub audio_enabled: bool,
    pub audio_volume: f32,
    /// Host keyboard mapping at startup: `"character"` (default) or
    /// `"positional"` (see [`crate::input::KeyLayout`]). Toggle live with `F7`.
    pub key_layout: String,
    /// Defeat the authentic console screen-blank (anti-burn-in) timeout. The
    /// genuine TI-99/4A ROM blanks the display to the backdrop after ~9 minutes
    /// idle — its VBLANK ISR advances a counter at scratchpad `>83D6` (+2/tick)
    /// and clears the VDP display-enable bit when it wraps; any key restores it.
    /// When `true`, the frontend keeps that counter from wrapping so the picture
    /// never blanks. Default `false` (faithful hardware behavior).
    pub defeat_screen_blank: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            log_level: "info".into(),
            // No media is embedded or configured-in: the console boots bare
            // until the command line or the file browser mounts something.
            last_cartridge: String::new(),
            last_disk: String::new(),
            browser_dir: String::new(),
            window_scale: 3,
            fullscreen: false,
            audio_enabled: true,
            audio_volume: 0.8,
            key_layout: "character".into(),
            defeat_screen_blank: false,
        }
    }
}

impl Config {
    /// Load preferences from the platform config path, filling defaults for any
    /// missing/invalid keys, then rewrite a clean file. Always succeeds.
    pub fn load() -> Config {
        let path = config_path();
        let cfg = match path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(text) => Config::from_toml_str(&text),
            None => Config::default(),
        };
        if let Some(path) = &path {
            cfg.save(path);
        }
        cfg
    }

    /// Write the preferences back as a clean, commented file (best effort,
    /// atomically — a crash mid-write must not eat the preferences).
    pub fn save(&self, path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = write_atomic(path, self.to_toml_string().as_bytes());
    }

    /// Parse preferences from TOML text, using defaults for missing/invalid keys.
    pub fn from_toml_str(text: &str) -> Config {
        let table: toml::Table = text.parse().unwrap_or_default();
        let d = Config::default();
        let s = |k: &str, def: &str| {
            table
                .get(k)
                .and_then(|v| v.as_str())
                .unwrap_or(def)
                .to_string()
        };
        Config {
            log_level: s("log_level", &d.log_level),
            // Media keys hold file PATHS (a config predating the media rework
            // held embedded names here; those no longer resolve and the resume
            // simply reports the file as missing — a one-time migration blip).
            last_cartridge: s("last_cartridge", &d.last_cartridge),
            last_disk: s("last_disk", &d.last_disk),
            browser_dir: s("browser_dir", &d.browser_dir),
            window_scale: table
                .get("window_scale")
                .and_then(|v| v.as_integer())
                .map(|n| n.clamp(1, 8) as u32)
                .unwrap_or(d.window_scale),
            fullscreen: table
                .get("fullscreen")
                .and_then(|v| v.as_bool())
                .unwrap_or(d.fullscreen),
            audio_enabled: table
                .get("audio_enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(d.audio_enabled),
            audio_volume: table
                .get("audio_volume")
                .and_then(|v| v.as_float())
                .map(|f| f.clamp(0.0, 1.0) as f32)
                .unwrap_or(d.audio_volume),
            key_layout: s("key_layout", &d.key_layout),
            defeat_screen_blank: table
                .get("defeat_screen_blank")
                .and_then(|v| v.as_bool())
                .unwrap_or(d.defeat_screen_blank),
        }
    }

    /// Render the preferences as a commented TOML file.
    pub fn to_toml_string(&self) -> String {
        format!(
            "# Libre99 preferences. Edit freely; missing/invalid keys use defaults.\n\
             \n\
             # Logging verbosity: error | warn | info | debug | trace\n\
             log_level = \"{log_level}\"\n\
             \n\
             # Auto-written on exit: file paths of the media mounted last, so a resumed\n\
             # session keeps its cartridge/disk, and the directory the file browser (F9)\n\
             # opens in. Usually no need to edit by hand. Empty = none / home directory.\n\
             last_cartridge = \"{last_cart}\"\n\
             last_disk      = \"{last_disk}\"\n\
             browser_dir    = \"{browser}\"\n\
             \n\
             # Display\n\
             window_scale = {scale}        # integer upscale of the 256x192 image\n\
             fullscreen   = {full}\n\
             \n\
             # Audio\n\
             audio_enabled = {aud}\n\
             audio_volume  = {vol}     # 0.0 .. 1.0\n\
             \n\
             # Input — host keyboard mapping (toggle live with F7):\n\
             #   character  = type normally; each keystroke maps to the TI keys\n\
             #                that produce the same character (default)\n\
             #   positional = map by host key position (best for games)\n\
             key_layout = \"{layout}\"\n\
             \n\
             # The genuine console blanks the screen (anti-burn-in) after ~9 min\n\
             # idle; pressing any key restores it. Set true to keep it always on.\n\
             defeat_screen_blank = {blank}\n",
            log_level = toml_escape(&self.log_level),
            last_cart = toml_escape(&self.last_cartridge),
            last_disk = toml_escape(&self.last_disk),
            browser = toml_escape(&self.browser_dir),
            scale = self.window_scale,
            full = self.fullscreen,
            aud = self.audio_enabled,
            vol = self.audio_volume,
            layout = toml_escape(&self.key_layout),
            blank = self.defeat_screen_blank,
        )
    }
}

/// Escape a string for a double-quoted TOML value: Windows paths carry `\`,
/// which TOML basic strings treat as an escape character.
fn toml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Persist the session's media state into the preferences file: the file paths
/// of the mounted cartridge/disk (empty = none) and the file browser's last
/// directory. Merges into the existing file (re-parsed then rewritten clean),
/// best effort — a missing data dir or unwritable file is silently ignored.
pub fn update_session(cartridge: &str, disk: &str, browser_dir: &str) {
    let Some(path) = config_path() else {
        return;
    };
    let mut cfg = match std::fs::read_to_string(&path) {
        Ok(text) => Config::from_toml_str(&text),
        Err(_) => Config::default(),
    };
    cfg.last_cartridge = cartridge.to_string();
    cfg.last_disk = disk.to_string();
    cfg.browser_dir = browser_dir.to_string();
    cfg.save(&path);
}

/// The user's home directory: `$HOME`, falling back to `%USERPROFILE%` (a
/// native Windows launch has no `HOME`). No external crates.
pub fn home_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home))
}

/// `~/.libre99` — the directory holding every user-specific file for
/// this app (preferences, log, save states, screenshots).
pub fn data_dir() -> Option<PathBuf> {
    Some(home_dir()?.join(".libre99"))
}

/// Create the user data directory if it doesn't exist (best effort). Called once
/// at startup so the preferences and log have somewhere to land. Adopts the
/// pre-rebrand `~/.ti-99-emulator` directory first, so existing preferences,
/// save state, and screenshots survive the rename to Libre99 — and then the
/// resume state's pre-2026-07-07 file name.
pub fn ensure_data_dir() {
    if let (Some(home), Some(dir)) = (home_dir(), data_dir()) {
        migrate_legacy_data_dir(&home.join(".ti-99-emulator"), &dir);
        let _ = std::fs::create_dir_all(&dir);
        adopt_legacy_state_file(&dir);
    }
}

/// One-time rename of the resume state's old file name (`savestate.ti99`,
/// pre-2026-07-07) to `resume.ti99`, so an existing session keeps resuming
/// across the rename. Never clobbers: an existing `resume.ti99` wins and the
/// old file is then left where it is.
fn adopt_legacy_state_file(dir: &Path) {
    let old = dir.join("savestate.ti99");
    let new = dir.join("resume.ti99");
    if old.is_file() && !new.exists() {
        let _ = std::fs::rename(&old, &new);
    }
}

/// Write `bytes` to `path` **atomically**: write a sibling `<name>.tmp` file
/// first, then rename it over the target — `rename` replaces the destination
/// as one operation on both Windows (`MOVEFILE_REPLACE_EXISTING`) and POSIX,
/// so a crash or full disk mid-write leaves the previous file intact instead
/// of a truncated one. Used for everything worth protecting: the resume
/// state, snapshots, and the preferences.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// One-time migration from the pre-rebrand data directory: if `new` doesn't
/// exist yet and `old` does, rename the directory, then the project-named
/// files inside (`ti-99-emulator.toml` → `libre99.toml`, `.log` likewise —
/// `savestate.ti99` is picked up afterwards by [`adopt_legacy_state_file`]).
/// Best effort: any failure just leaves a fresh directory to be created.
fn migrate_legacy_data_dir(old: &Path, new: &Path) {
    if new.exists() || !old.is_dir() || std::fs::rename(old, new).is_err() {
        return;
    }
    for (from, to) in [
        ("ti-99-emulator.toml", "libre99.toml"),
        ("ti-99-emulator.log", "libre99.log"),
    ] {
        let _ = std::fs::rename(new.join(from), new.join(to));
    }
}

/// `~/.libre99/libre99.toml` — the preferences file.
pub fn config_path() -> Option<PathBuf> {
    Some(data_dir()?.join("libre99.toml"))
}

/// `~/.libre99/libre99.log` — the run log (appended across runs).
pub fn log_path() -> Option<PathBuf> {
    Some(data_dir()?.join("libre99.log"))
}

/// `~/.libre99/resume.ti99` — the **resume state**: the one automatic save
/// state, written on exit and by Save (`F6`), loaded at startup and by Load
/// (`F8`). (Named `savestate.ti99` before 2026-07-07; `ensure_data_dir`
/// adopts the old name once.) User-named snapshots (`Shift`+`F6`/`F8`) are
/// separate `.ti99` files wherever the user puts them.
pub fn state_path() -> Option<PathBuf> {
    Some(data_dir()?.join("resume.ti99"))
}

/// `~/.libre99/screenshots` — where `encode_png` screenshots are written.
pub fn screenshot_dir() -> Option<PathBuf> {
    Some(data_dir()?.join("screenshots"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_through_toml() {
        let d = Config::default();
        let parsed = Config::from_toml_str(&d.to_toml_string());
        assert_eq!(parsed, d);
    }

    #[test]
    fn missing_and_invalid_keys_fall_back_to_defaults() {
        let cfg = Config::from_toml_str("window_scale = \"oops\"\nfullscreen = true\n");
        let d = Config::default();
        assert_eq!(cfg.window_scale, d.window_scale, "invalid int → default");
        assert!(cfg.fullscreen, "valid bool is taken");
        assert_eq!(cfg.last_cartridge, d.last_cartridge, "missing → default");
    }

    #[test]
    fn garbage_input_yields_defaults() {
        assert_eq!(Config::from_toml_str("@@ not toml @@"), Config::default());
    }

    #[test]
    fn session_media_paths_round_trip_including_windows_paths() {
        // Paths are taken verbatim (empty = bare console / no disk).
        let cfg = Config::from_toml_str(
            "last_cartridge = \"/media/parsec.ctg\"\nlast_disk = \"\"\n",
        );
        assert_eq!(cfg.last_cartridge, "/media/parsec.ctg");
        assert_eq!(cfg.last_disk, "");

        // Full round-trip through the serializer preserves the session keys —
        // including Windows paths, whose backslashes need TOML escaping.
        let mut d = Config::default();
        d.last_cartridge = r"C:\Users\ti\media\parsec.ctg".into();
        d.last_disk = String::new();
        d.browser_dir = r"C:\Users\ti\media".into();
        assert_eq!(Config::from_toml_str(&d.to_toml_string()), d);
    }

    #[test]
    fn values_are_clamped() {
        let cfg = Config::from_toml_str("window_scale = 99\naudio_volume = 5.0\n");
        assert_eq!(cfg.window_scale, 8);
        assert_eq!(cfg.audio_volume, 1.0);
    }

    #[test]
    fn user_files_live_under_the_dot_dir() {
        // Every user-specific path shares the ~/.libre99 base.
        let base = data_dir().expect("HOME or USERPROFILE is set in the test environment");
        assert!(base.ends_with(".libre99"));
        assert_eq!(config_path().unwrap(), base.join("libre99.toml"));
        assert_eq!(log_path().unwrap(), base.join("libre99.log"));
        assert_eq!(state_path().unwrap(), base.join("resume.ti99"));
        assert_eq!(screenshot_dir().unwrap(), base.join("screenshots"));
    }

    #[test]
    fn legacy_data_dir_migrates_once_and_never_clobbers() {
        let scratch =
            std::env::temp_dir().join(format!("libre99-migration-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        let old = scratch.join(".ti-99-emulator");
        let new = scratch.join(".libre99");

        // A pre-rebrand directory: prefs + log under the old names, plus the
        // old-named savestate (renamed later by `adopt_legacy_state_file`,
        // not by the directory move tested here) and a screenshot.
        std::fs::create_dir_all(old.join("screenshots")).unwrap();
        std::fs::write(old.join("ti-99-emulator.toml"), "fullscreen = true\n").unwrap();
        std::fs::write(old.join("ti-99-emulator.log"), "log line\n").unwrap();
        std::fs::write(old.join("savestate.ti99"), b"state").unwrap();

        migrate_legacy_data_dir(&old, &new);
        assert!(!old.exists(), "the old directory is renamed away");
        assert_eq!(
            std::fs::read_to_string(new.join("libre99.toml")).unwrap(),
            "fullscreen = true\n"
        );
        assert_eq!(std::fs::read_to_string(new.join("libre99.log")).unwrap(), "log line\n");
        assert_eq!(std::fs::read(new.join("savestate.ti99")).unwrap(), b"state");
        assert!(new.join("screenshots").is_dir());

        // A second migration attempt (old dir reappears) must not touch the
        // adopted directory.
        std::fs::create_dir_all(&old).unwrap();
        std::fs::write(old.join("ti-99-emulator.toml"), "fullscreen = false\n").unwrap();
        migrate_legacy_data_dir(&old, &new);
        assert!(old.exists(), "nothing moves once ~/.libre99 exists");
        assert_eq!(
            std::fs::read_to_string(new.join("libre99.toml")).unwrap(),
            "fullscreen = true\n"
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn the_legacy_resume_state_name_is_adopted_once_and_never_clobbers() {
        let scratch =
            std::env::temp_dir().join(format!("libre99-state-adopt-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).unwrap();

        // The old name is renamed to resume.ti99.
        std::fs::write(scratch.join("savestate.ti99"), b"old state").unwrap();
        adopt_legacy_state_file(&scratch);
        assert!(!scratch.join("savestate.ti99").exists());
        assert_eq!(std::fs::read(scratch.join("resume.ti99")).unwrap(), b"old state");

        // An existing resume.ti99 wins; a reappearing old file is left alone.
        std::fs::write(scratch.join("savestate.ti99"), b"stale").unwrap();
        adopt_legacy_state_file(&scratch);
        assert_eq!(std::fs::read(scratch.join("resume.ti99")).unwrap(), b"old state");
        assert!(scratch.join("savestate.ti99").exists(), "the stale file was not consumed");

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn write_atomic_replaces_the_target_and_leaves_no_temp_file() {
        let scratch =
            std::env::temp_dir().join(format!("libre99-atomic-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(&scratch).unwrap();
        let target = scratch.join("resume.ti99");

        write_atomic(&target, b"first").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"first");
        write_atomic(&target, b"second").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"second", "existing file is replaced");
        assert!(
            !scratch.join("resume.ti99.tmp").exists(),
            "the temp file must not outlive the write"
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }
}
