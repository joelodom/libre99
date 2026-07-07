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

//! TI-99/4A emulator — desktop frontend.
//!
//! Resolves settings (preferences file overridden by command-line flags), starts
//! logging, assembles the [`libre99_core`] machine, and runs the winit window
//! loop. **No media is embedded**: with no flags the console boots bare (to the
//! master title screen); a cartridge or disk comes from `--cartridge` /
//! `--disk` file paths or the in-app file browser (`F9`).

mod app;
mod assets;
mod audio;
mod cli;
mod config;
mod debug;
mod disks;
mod font;
mod help;
mod input;
mod logging;
mod media;
mod pacing;
mod screenshot;
mod speed;
mod sysinfo;
mod text;
mod video;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libre99_core::machine::Machine;
use winit::event_loop::EventLoop;

use app::{App, Options};
use cli::Args;
use config::Config;
use media::{MediaItem, MediaKind};

/// Optional firmware overrides from `--system-grom` / `--system-rom`, read once
/// at startup. Stored here (rather than threaded through `Options`) so warm
/// media changes (`rebuild_machine`) keep booting the overridden firmware.
static SYSTEM_GROM: OnceLock<Vec<u8>> = OnceLock::new();
static SYSTEM_ROM: OnceLock<Vec<u8>> = OnceLock::new();
static DISK_DSR: OnceLock<Vec<u8>> = OnceLock::new();

fn main() {
    let args = match Args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}\n\n{}", cli::USAGE);
            std::process::exit(2);
        }
    };
    if args.help {
        println!("{}", cli::USAGE);
        return;
    }

    // All user-specific files live in ~/.libre99; make sure it exists
    // before we read preferences or open the log.
    config::ensure_data_dir();

    // Firmware overrides (read before logging is up, so report to stderr).
    let grom_override = match &args.system_grom {
        Some(path) => match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(e) => {
                eprintln!("could not read --system-grom {path}: {e}");
                std::process::exit(2);
            }
        },
        None => None,
    };
    if let Some(path) = &args.system_rom {
        match std::fs::read(path) {
            Ok(bytes) => {
                let _ = SYSTEM_ROM.set(bytes);
            }
            Err(e) => {
                eprintln!("could not read --system-rom {path}: {e}");
                std::process::exit(2);
            }
        }
    }
    if let Some(path) = &args.disk_dsr {
        match std::fs::read(path) {
            Ok(bytes) => {
                let _ = DISK_DSR.set(bytes);
            }
            Err(e) => {
                eprintln!("could not read --disk-dsr {path}: {e}");
                std::process::exit(2);
            }
        }
    }

    // Resolve the console GROM this session boots — the project's own clean-room
    // rewrite by default, or a `--system-grom` override. If it is a Libre99
    // image (it carries the L99I identification block, as our default does),
    // stamp the host facts its system information screen shows — in this
    // in-memory copy only, before the machine powers on. An authentic TI GROM
    // (e.g. supplied via the override) has no block and is untouched.
    let mut grom = grom_override.unwrap_or_else(|| assets::DEFAULT_CONSOLE_GROM.to_vec());
    let rom: &[u8] = SYSTEM_ROM.get().map(Vec::as_slice).unwrap_or(assets::DEFAULT_CONSOLE_ROM);
    let stamped_rom_id = sysinfo::stamp(&mut grom, rom);
    let _ = SYSTEM_GROM.set(grom);

    let config = Config::load();
    let level = args.log_level.as_deref().unwrap_or(&config.log_level);
    logging::init(logging::level_from_str(level), config::log_path().as_deref());
    if let Some(rom_id) = &stamped_rom_id {
        log::info!("stamped the Libre99 system-information block (console ROM: {rom_id})");
    }

    // Command-line media: file paths, read and validated up front. A path the
    // user explicitly gave that doesn't load is a launch error worth stopping
    // for, matching the firmware-override flags above.
    let mut cart = args
        .cartridge
        .as_deref()
        .map(|p| load_cli_media(MediaKind::Cartridge, p));
    let mut disk = args.disk.as_deref().map(|p| load_cli_media(MediaKind::Disk, p));

    let mut machine = build_machine(cart.as_ref().map(|m| m.bytes.as_slice()));
    // A command-line disk mounts keyed (by its canonical path), like an F9
    // mount, so its writes are remembered across ejects and in save states.
    if let Some(item) = &mut disk {
        let key = media::disk_key(&item.path);
        // The image now lives in the machine; the item keeps only the path
        // (for the window title and the exit bookkeeping).
        machine.mount_disk_keyed(0, &key, std::mem::take(&mut item.bytes));
    }

    // Resume the previous session: load the save state written on the last exit,
    // unless the user explicitly chose media on the command line (then honor it).
    if args.cartridge.is_none()
        && args.disk.is_none()
        && args.system_grom.is_none()
        && args.system_rom.is_none()
    {
        if let Some(path) = config::state_path() {
            match std::fs::read(&path) {
                Ok(bytes) => match machine.load_state(&bytes) {
                    Ok(()) => {
                        log::info!("resumed previous session from {}", path.display());
                        // The snapshot carries the session's media inside it; the
                        // frontend re-reads the cartridge *file* recorded at exit
                        // so a later in-app media change can keep it mounted. A
                        // file gone missing only degrades that case — the resumed
                        // session itself is intact (warn and go on).
                        cart = reload_session_media(MediaKind::Cartridge, &config.last_cartridge);
                        // The disk needs no re-read: its image (and host
                        // identity) are inside the snapshot. A version-1 save
                        // predates identities — adopt the path recorded at exit
                        // so its disk joins the keyed persistence model.
                        if !config.last_disk.is_empty() {
                            let key = media::disk_key(Path::new(&config.last_disk));
                            machine.bus_mut().disk.adopt_drive_key(0, &key);
                        }
                        disk = machine.bus().disk.drive_key(0).map(|k| MediaItem {
                            path: PathBuf::from(k),
                            bytes: Vec::new(),
                        });
                    }
                    Err(e) => log::warn!("ignoring unreadable save state ({e:?}); starting fresh"),
                },
                Err(_) => log::info!("no previous session to resume; starting fresh"),
            }
        }
    }

    // The file browser opens where it last mounted from (home on first run).
    let browser_dir = Some(PathBuf::from(&config.browser_dir))
        .filter(|p| !config.browser_dir.is_empty() && p.is_dir())
        .or_else(config::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));

    let options = Options {
        scale: args.scale.unwrap_or(config.window_scale),
        fullscreen: args.fullscreen || config.fullscreen,
        audio: config.audio_enabled,
        volume: config.audio_volume,
        cart,
        disk,
        browser_dir,
        key_layout: input::KeyLayout::from_config(&config.key_layout),
        defeat_screen_blank: config.defeat_screen_blank,
    };

    let event_loop = EventLoop::new().expect("create event loop");
    let mut app = App::new(machine, options);
    event_loop.run_app(&mut app).expect("run event loop");
}

/// Load a `--cartridge`/`--disk` file or exit with the loader's one-line error
/// (before the window opens; logging is already up, so record it there too).
fn load_cli_media(kind: MediaKind, path: &str) -> MediaItem {
    match media::load(kind, Path::new(path)) {
        Ok(item) => {
            log::info!("command-line {}: {}", kind.label(), item.path.display());
            item
        }
        Err(message) => {
            log::error!("{message}");
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}

/// Re-read a media file recorded in the preferences at last exit (empty = none
/// was mounted). Missing/unloadable files are logged and dropped — the resumed
/// snapshot still runs; only a *later* media change would lose this side.
fn reload_session_media(kind: MediaKind, path: &str) -> Option<MediaItem> {
    if path.is_empty() {
        return None;
    }
    match media::load(kind, Path::new(path)) {
        Ok(item) => Some(item),
        Err(message) => {
            log::warn!("resumed session's {} not reloadable: {message}", kind.label());
            None
        }
    }
}

/// Build the emulated console with the session firmware and the given
/// cartridge bytes (`None` = bare console). Reused for the cold boot a
/// cartridge change requires — disks are not mounted here: they slot into the
/// running machine live (`Machine::mount_disk_keyed`), and a rebuild carries
/// the whole disk subsystem over from the old machine.
pub(crate) fn build_machine(cartridge: Option<&[u8]>) -> Machine {
    // Firmware: a `--system-rom` / `--system-grom` override, else the clean-room
    // default (`SYSTEM_GROM` always holds the stamped default set up in `main`).
    let rom: &[u8] = SYSTEM_ROM.get().map(Vec::as_slice).unwrap_or(assets::DEFAULT_CONSOLE_ROM);
    let grom: &[u8] = SYSTEM_GROM.get().map(Vec::as_slice).unwrap_or(assets::DEFAULT_CONSOLE_GROM);
    let mut machine = Machine::new(rom, grom);

    // The disk controller runs the clean-room DSR by default (Phase 3); a
    // `--disk-dsr` override (e.g. an authentic `Disk.Bin`) replaces it.
    let dsr: &[u8] = DISK_DSR.get().map(Vec::as_slice).unwrap_or(assets::DEFAULT_DISK_DSR);
    machine.load_disk_controller(dsr);
    if let Some(bytes) = cartridge {
        // The browser and the CLI both validated the image with
        // `media::load`, so a parse failure here means the file changed
        // underneath us mid-session — log it and boot bare.
        match libre99_core::cartridge::Cartridge::parse(bytes) {
            Ok(cart) => {
                log::info!("mounted cartridge: {}", cart.title);
                machine.mount_cartridge(&cart);
            }
            Err(e) => log::error!("could not parse the cartridge image: {e:?}"),
        }
    }

    machine.reset();
    machine
}
