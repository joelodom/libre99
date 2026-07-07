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

//! The winit application: window creation, the ~60 Hz frame loop, presentation
//! via softbuffer, and keyboard input.

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use libre99_core::machine::Machine;
use libre99_core::state::StateError;
use libre99_core::vdp::{HEIGHT, WIDTH};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::audio::Audio;
use crate::disks::{self, DiskAction, DiskOverlay};
use crate::font::Fonts;
use crate::help::{self, HelpTab};
use crate::input::{self, KeyLayout};
use crate::media::{self, MediaItem, MediaKind, UnloadChoice};
use crate::pacing;
use crate::screenshot;
use crate::speed::Speed;
use crate::text;
use crate::video;

/// One emulated video frame (~59.92 Hz; close enough at 60).
const FRAME: Duration = Duration::from_micros(16_667);

/// Presentation/audio options plus the initially-mounted media, resolved from
/// config + the command line.
pub struct Options {
    pub scale: u32,
    pub fullscreen: bool,
    pub audio: bool,
    pub volume: f32,
    /// Initially-mounted cartridge (`None` = bare console).
    pub cart: Option<MediaItem>,
    /// Initially-inserted DSK1 image (`None` = empty drive).
    pub disk: Option<MediaItem>,
    /// Where the file browser (`F9`) opens.
    pub browser_dir: PathBuf,
    /// Host keyboard mapping at startup (toggle live with `F7`).
    pub key_layout: KeyLayout,
    /// Keep the authentic ~9-minute screen-blank timeout from ever firing
    /// (opt-in; default off preserves faithful hardware behavior).
    pub defeat_screen_blank: bool,
    /// Show the first-run `PRESS ESC FOR HELP` banner (no resume state on disk).
    pub show_help_banner: bool,
}

/// The desktop application: the emulated machine plus its window/surface and
/// frame pacing.
pub struct App {
    machine: Machine,
    scale: u32,
    fullscreen: bool,
    volume: f32,
    window: Option<Rc<Window>>,
    surface: Option<softbuffer::Surface<Rc<Window>, Rc<Window>>>,
    framebuffer: Vec<u32>,
    next_frame: Instant,
    audio: Option<Audio>,
    audio_buf: Vec<f32>,
    // The mounted media (path + bytes, so a warm rebuild re-mounts without
    // re-reading files). `None` = bare console / empty DSK1.
    cart: Option<MediaItem>,
    disk: Option<MediaItem>,
    // Where the file browser opens; follows the last successful mount and is
    // persisted across sessions on exit.
    browser_dir: PathBuf,
    // A transient on-screen status line ("STATE SAVED", …) and how many more
    // frames to show it.
    toast: Option<String>,
    toast_frames: u32,
    // When true, the Esc/F1 help overlay is shown (full-screen, native
    // resolution) and TI key input is suspended.
    keyboard_help: bool,
    // First-run onboarding cue: draw a persistent PRESS ESC FOR HELP banner.
    // True while there is no resume state (first launch, or right after a
    // Shift+F5 fresh start); retired for good the moment the user opens the
    // help or a resume state gets written.
    help_banner: bool,
    // The F4 disk-memory overlay (list / export / unload of in-memory disks).
    // While open it captures key input; the machine keeps running.
    disk_ui: DiskOverlay,
    // The window title last applied, so the per-frame refresh only touches the
    // window when the title actually changes (the DSK1 dirty marker can flip
    // on any frame the DSR writes).
    title_cache: String,
    // The embedded smooth fonts used by the native-resolution help overlay.
    fonts: Fonts,
    // The active help tab, and a cached render of it reused across frames until
    // the tab or window size changes (so presentation is just a memcpy).
    help_tab: HelpTab,
    help_image: Vec<u32>,
    help_image_key: Option<(usize, usize, HelpTab)>,
    // How host keys map onto the TI keyboard (positional vs character).
    layout: KeyLayout,
    // Pause / fast-forward / frame-advance state for the frame loop.
    speed: Speed,
    // When true, the live CPU inspector panel is drawn (non-modal).
    debug_overlay: bool,
    // Latest host modifier state, for the character layout's FCTN/CTRL layer.
    host_mods: input::HostMods,
    // The TI key(s) each currently-held host key pressed, so a release closes
    // exactly those cells — a character-mode combo may hold a synthesized
    // SHIFT/FCTN that must lift together with its base key.
    pressed: HashMap<KeyCode, input::TiPress>,
    // When true, hold off the authentic screen-blank timeout (see `Options`).
    defeat_screen_blank: bool,
    // Latches once we've logged a present-path failure (surface resize/acquire/
    // present), so a persistently-lost surface doesn't spam the log every frame.
    // Cleared on the next clean present so a fresh failure re-logs.
    present_error_logged: bool,
}

impl App {
    /// Build the application around a ready-to-run machine. Opens an audio output
    /// (if enabled and available) and tells the machine to synthesize at the
    /// device's sample rate.
    pub fn new(mut machine: Machine, opts: Options) -> Self {
        let audio = if opts.audio { Audio::new() } else { None };
        match &audio {
            Some(a) => {
                machine.set_audio_sample_rate(a.sample_rate());
                log::info!("audio output at {} Hz", a.sample_rate());
            }
            None => log::info!("running without audio"),
        }
        log::info!("key layout: {}", opts.key_layout.as_config());
        App {
            machine,
            scale: opts.scale.clamp(1, 8),
            fullscreen: opts.fullscreen,
            volume: opts.volume.clamp(0.0, 1.0),
            window: None,
            surface: None,
            framebuffer: vec![0; WIDTH * HEIGHT],
            next_frame: Instant::now(),
            audio,
            audio_buf: Vec::new(),
            cart: opts.cart,
            disk: opts.disk,
            browser_dir: opts.browser_dir,
            toast: None,
            toast_frames: 0,
            keyboard_help: false,
            help_banner: opts.show_help_banner,
            disk_ui: DiskOverlay::default(),
            title_cache: String::new(),
            fonts: Fonts::new(),
            help_tab: HelpTab::Start,
            help_image: Vec::new(),
            help_image_key: None,
            layout: opts.key_layout,
            speed: Speed::new(),
            debug_overlay: false,
            host_mods: input::HostMods::default(),
            pressed: HashMap::new(),
            defeat_screen_blank: opts.defeat_screen_blank,
            present_error_logged: false,
        }
    }

    /// The mounted cartridge's file name, `(none)` when the console is bare.
    fn cart_name(&self) -> String {
        self.cart.as_ref().map(MediaItem::name).unwrap_or_else(|| "(none)".into())
    }

    /// The DSK1 image's file name, `(none)` when the drive is empty.
    fn disk_name(&self) -> String {
        self.disk.as_ref().map(MediaItem::name).unwrap_or_else(|| "(none)".into())
    }

    /// Window title showing the current media. A `*` after the disk name marks
    /// in-memory changes that have not been exported to a host file.
    fn title(&self) -> String {
        let star = if self.machine.bus().disk.drive_dirty(0) { "*" } else { "" };
        format!("Libre99  —  {}  ·  DSK1: {}{star}", self.cart_name(), self.disk_name())
    }

    /// Apply the title to the window if it changed since last applied (called
    /// once per frame — the disk dirty marker can flip on any frame).
    fn sync_title(&mut self) {
        let title = self.title();
        if title != self.title_cache {
            if let Some(window) = &self.window {
                window.set_title(&title);
            }
            self.title_cache = title;
        }
    }

    /// Rebuild the machine around a **cartridge** change — mounting or ejecting
    /// a cartridge is a cold boot, since the console scans cartridge ROM at
    /// power-up. The whole disk subsystem — mounted image, the in-memory shelf,
    /// controller state — is carried over intact, so a cartridge swap never
    /// costs disk edits. (Disks themselves mount and eject **live**, without
    /// coming through here.)
    fn rebuild_machine(&mut self) {
        let disk = std::mem::take(&mut self.machine.bus_mut().disk);
        self.machine = crate::build_machine(self.cart.as_ref());
        self.machine.bus_mut().disk = disk;
        if let Some(audio) = &self.audio {
            self.machine.set_audio_sample_rate(audio.sample_rate());
        }
        self.sync_title();
        log::info!("media: cartridge={:?} disk={:?}", self.cart_name(), self.disk_name());
    }

    /// Top the audio queue up to a short target latency with freshly synthesized
    /// samples; generating only the shortfall self-syncs to the device rate.
    fn feed_audio(&mut self) {
        let Some(audio) = &self.audio else {
            return;
        };
        let target = (audio.sample_rate() / 20) as usize; // ~50 ms
        let want = target.saturating_sub(audio.queued());
        if want > 0 {
            self.audio_buf.resize(want, 0.0);
            self.machine.fill_audio(&mut self.audio_buf);
            if self.volume != 1.0 {
                self.audio_buf.iter_mut().for_each(|s| *s *= self.volume);
            }
            audio.push(&self.audio_buf);
        }
    }

    fn present(&mut self) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (w, h) = (size.width.max(1) as usize, size.height.max(1) as usize);
        // Softbuffer can lose its backing (device reset, minimize, occlusion): on
        // any error log once and skip this frame rather than panic the whole app.
        // `present_error_logged` is a disjoint field, so touching it while `surface`
        // is borrowed is fine.
        let (Some(nw), Some(nh)) = (NonZeroU32::new(w as u32), NonZeroU32::new(h as u32)) else {
            return;
        };
        if let Err(e) = surface.resize(nw, nh) {
            if !self.present_error_logged {
                log::warn!("surface resize failed, skipping frame: {e}");
                self.present_error_logged = true;
            }
            return;
        }
        let mut buffer = match surface.buffer_mut() {
            Ok(buffer) => buffer,
            Err(e) => {
                if !self.present_error_logged {
                    log::warn!("could not acquire present buffer, skipping frame: {e}");
                    self.present_error_logged = true;
                }
                return;
            }
        };
        if self.keyboard_help {
            // The help overlay is drawn at the window's native resolution (so its
            // smooth fonts stay crisp) rather than upscaled from 256×192. Re-render
            // only when the tab or window size changes; otherwise reuse the cache.
            let key = (w, h, self.help_tab);
            if self.help_image_key != Some(key) {
                self.help_image.resize(w * h, 0);
                help::render(&mut self.fonts, &mut self.help_image, w, h, self.help_tab);
                self.help_image_key = Some(key);
            }
            buffer.copy_from_slice(&self.help_image);
        } else {
            video::blit(&self.framebuffer, &mut buffer, w, h);
        }
        if let Err(e) = buffer.present() {
            if !self.present_error_logged {
                log::warn!("buffer present failed, skipping frame: {e}");
                self.present_error_logged = true;
            }
            return;
        }
        // A clean frame got through — re-arm the one-shot so a later failure logs.
        self.present_error_logged = false;
    }

    fn toggle_fullscreen(&mut self) {
        // Derive the next state from the window's *actual* fullscreen status,
        // not a local flag: the user can also enter/leave fullscreen with the
        // green title-bar button, which never runs this code, so a separate
        // bool would drift out of sync and make the next toggle a no-op.
        let going_fs = match &self.window {
            Some(window) => {
                let going_fs = window.fullscreen().is_none();
                window.set_fullscreen(going_fs.then_some(Fullscreen::Borderless(None)));
                going_fs
            }
            None => return,
        };
        self.fullscreen = going_fs;
        self.flash(if going_fs { "Fullscreen" } else { "Windowed" });
    }

    /// Flash a short status message on screen for ~2 seconds.
    fn flash(&mut self, message: impl Into<String>) {
        self.toast = Some(message.into());
        self.toast_frames = 120;
    }

    /// Release every held TI key and forget the host-key → TI-key records (used
    /// on focus loss and whenever TI input is otherwise interrupted).
    fn release_all_keys(&mut self) {
        self.machine.bus_mut().keyboard.release_all();
        self.pressed.clear();
    }

    /// Apply a host key press/release to the TI matrix under the current layout.
    /// On release we replay exactly the keys recorded at press time, so a
    /// character-mode `SHIFT`/`FCTN` synthesized for this key can't stick if the
    /// host modifier state changed before the release arrived.
    fn set_ti_keys(&mut self, code: KeyCode, event: &KeyEvent, down: bool) {
        let press = if down {
            let press = input::resolve(event, self.layout, self.host_mods);
            self.pressed.insert(code, press);
            press
        } else {
            self.pressed
                .remove(&code)
                .unwrap_or_else(|| input::resolve(event, self.layout, self.host_mods))
        };
        for key in press.keys() {
            self.machine.set_key(key, down);
        }
    }

    /// Toggle between positional and character host-key mapping. Held keys are
    /// released first so none stick across the remap.
    fn toggle_layout(&mut self) {
        self.release_all_keys();
        self.layout = match self.layout {
            KeyLayout::Positional => KeyLayout::Character,
            KeyLayout::Character => KeyLayout::Positional,
        };
        let label = match self.layout {
            KeyLayout::Positional => "INPUT: POSITIONAL (QWERTY)",
            KeyLayout::Character => "INPUT: CHARACTER (HOST LAYOUT)",
        };
        log::info!("key layout: {}", self.layout.as_config());
        self.flash(label);
    }

    /// Save a PNG screenshot of the current (clean, overlay-free) frame to the
    /// screenshots directory.
    fn screenshot(&mut self) {
        let Some(dir) = crate::config::screenshot_dir() else {
            self.flash("SCREENSHOT FAILED");
            return;
        };
        let _ = std::fs::create_dir_all(&dir);
        let mut frame = vec![0u32; WIDTH * HEIGHT];
        self.machine.render(&mut frame);
        let png = screenshot::encode_png(WIDTH, HEIGHT, &frame);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let path = dir.join(format!("libre99-{stamp}.png"));
        match std::fs::write(&path, &png) {
            Ok(()) => {
                log::info!("screenshot -> {}", path.display());
                self.flash("SCREENSHOT SAVED");
            }
            Err(e) => {
                log::error!("screenshot failed: {e}");
                self.flash("SCREENSHOT FAILED");
            }
        }
    }

    /// Toggle pause and confirm it on screen (a persistent indicator also shows
    /// while paused).
    fn toggle_pause(&mut self) {
        self.speed.toggle_pause();
        self.flash(if self.speed.is_paused() {
            "PAUSED"
        } else {
            "RESUMED"
        });
    }

    /// `F9`: ask the OS for a media file with its **native open dialog**, then
    /// mount it in the port its extension implies (`.ctg` → cartridge port,
    /// `.dsk` → DSK1). The dialog is modal: the frame loop — and so the
    /// machine — stalls while it is up, and pacing resyncs afterwards. A file
    /// that won't load is a toast, not a dead machine.
    fn mount_via_dialog(&mut self) {
        // No key-release events arrive while the modal dialog has focus, so
        // lift everything first (the F9 press itself included).
        self.release_all_keys();
        self.speed.set_turbo(false);
        self.keyboard_help = false;
        let Some(path) = media::pick_media_file(&self.browser_dir) else {
            return; // canceled
        };
        let Some(kind) = media::kind_of(&path) else {
            self.flash("NOT TI MEDIA (.CTG / .DSK)");
            return;
        };
        match media::load(kind, &path) {
            Ok(mut item) => {
                if let Some(dir) = item.path.parent() {
                    self.browser_dir = dir.to_path_buf();
                }
                let name = item.name();
                match kind {
                    // A cartridge mount is a cold boot (the console scans
                    // cartridge ROM at power-up).
                    MediaKind::Cartridge => {
                        self.cart = Some(item);
                        self.rebuild_machine();
                        self.flash(format!("CART: {name}"));
                    }
                    // A disk slots into the *running* machine, like a real
                    // floppy — no reboot. If this same file was mounted (and
                    // written to) earlier this session, the in-memory copy
                    // reattaches instead of the host bytes.
                    MediaKind::Disk => {
                        let key = media::file_key(&item.path);
                        let bytes = std::mem::take(&mut item.bytes);
                        let resumed = self.machine.mount_disk_keyed(0, &key, bytes);
                        self.disk = Some(item);
                        self.sync_title();
                        log::info!(
                            "mounted DSK1 live: {key}{}",
                            if resumed { " (in-memory copy reattached)" } else { "" }
                        );
                        self.flash(if resumed {
                            format!("DSK1: {name} (IN-MEMORY CHANGES APPLY)")
                        } else {
                            format!("DSK1: {name}")
                        });
                    }
                }
            }
            Err(message) => {
                log::warn!("mount failed: {message}");
                self.flash("CANNOT LOAD (SEE LOG)");
            }
        }
    }

    /// `F2`: unmount the cartridge (a cold boot, back to the bare console).
    /// `F3`: empty DSK1 **live** — no reboot; the image stays in memory (see
    /// the `F4` overlay), so remounting the same file brings its edits back.
    fn eject(&mut self, kind: MediaKind) {
        match kind {
            MediaKind::Cartridge => {
                if self.cart.take().is_some() {
                    self.rebuild_machine();
                    self.flash("CARTRIDGE EJECTED");
                } else {
                    self.flash("ALREADY EMPTY");
                }
            }
            MediaKind::Disk => {
                // The machine is the source of truth for whether a disk is in
                // the drive (a resumed session can have one the app never
                // mounted itself).
                if self.machine.bus().disk.drive_image(0).is_none() {
                    self.flash("ALREADY EMPTY");
                    return;
                }
                let dirty = self.machine.bus().disk.drive_dirty(0);
                self.machine.eject_disk(0);
                self.disk = None;
                self.sync_title();
                self.flash(if dirty {
                    "DSK1 EJECTED - CHANGES KEPT IN MEMORY (F4)"
                } else {
                    "DSK1 EJECTED"
                });
            }
        }
    }

    /// `F4`: toggle the disk-memory overlay (in-memory disks: export / unload).
    fn toggle_disk_ui(&mut self) {
        if self.disk_ui.open {
            self.disk_ui.open = false;
            return;
        }
        // Like the file dialog: lift held keys (the F4 press included) since
        // TI input is suspended while the overlay captures the keyboard.
        self.release_all_keys();
        self.speed.set_turbo(false);
        self.disk_ui.open();
    }

    /// Route a key to the open disk-memory overlay and run the action it asks
    /// for.
    fn disk_ui_key(&mut self, code: KeyCode) {
        let disks = self.machine.bus().disk.in_memory_disks();
        match self.disk_ui.handle_key(code, &disks) {
            DiskAction::Close => self.disk_ui.open = false,
            DiskAction::Export(key) => {
                self.export_disk(&key);
            }
            DiskAction::Unload(key) => self.unload_disk(&key),
            DiskAction::None => {}
        }
    }

    /// Export the in-memory disk image identified by `key` to a host `.dsk`
    /// file the user picks with the **native save dialog** — which is what
    /// confirms an overwrite of an existing file; the app never overwrites a
    /// host `.dsk` without that prompt. On success the image is marked clean
    /// (its bytes are now safe on the host filesystem). Returns whether the
    /// file was written.
    fn export_disk(&mut self, key: &str) -> bool {
        let Some(image) = self.machine.bus().disk.image_for_key(key).map(<[u8]>::to_vec) else {
            self.flash("NO SUCH DISK IN MEMORY");
            return false;
        };
        // Modal dialog: same input caveats as the F9 chooser.
        self.release_all_keys();
        self.speed.set_turbo(false);
        let Some(path) = media::save_dsk_file(&self.browser_dir, disks::file_name(key)) else {
            return false; // canceled
        };
        match std::fs::write(&path, &image) {
            Ok(()) => {
                if let Some(dir) = path.parent() {
                    self.browser_dir = dir.to_path_buf();
                }
                self.machine.bus_mut().disk.mark_clean(key);
                self.sync_title();
                log::info!("exported disk {key} -> {}", path.display());
                self.flash("DISK EXPORTED");
                true
            }
            Err(e) => {
                log::error!("disk export to {} failed: {e}", path.display());
                self.flash("EXPORT FAILED (SEE LOG)");
                false
            }
        }
    }

    /// Unload the in-memory disk image identified by `key`, so the next mount
    /// of that file starts over from the host file's bytes. If the image has
    /// unexported changes, a **native message dialog** offers to export it
    /// first (canceling either dialog keeps the disk in memory — no data is
    /// lost without an explicit choice). Unloading a disk that is mounted also
    /// empties the drive.
    fn unload_disk(&mut self, key: &str) {
        let disks = self.machine.bus().disk.in_memory_disks();
        let Some(info) = disks.iter().find(|d| d.key == key) else {
            self.flash("NO SUCH DISK IN MEMORY");
            return;
        };
        if info.dirty {
            self.release_all_keys();
            self.speed.set_turbo(false);
            match media::confirm_unload(disks::file_name(key)) {
                UnloadChoice::Save => {
                    // A canceled or failed export aborts the unload — the user
                    // asked to save first, and nothing has been saved.
                    if !self.export_disk(key) {
                        return;
                    }
                }
                UnloadChoice::Discard => {}
                UnloadChoice::Cancel => return,
            }
        }
        let was_mounted = info.drive == Some(0);
        self.machine.bus_mut().disk.forget(key);
        if was_mounted {
            self.disk = None;
        }
        self.sync_title();
        log::info!("unloaded disk from memory: {key}");
        self.flash("DISK UNLOADED FROM MEMORY");
    }

    /// Write the machine to the **resume state** (`~/.libre99/resume.ti99`)
    /// atomically, logging the outcome and returning whether it succeeded.
    /// Shared by Save (F6), the auto-save on exit, and a snapshot load (which
    /// makes the snapshot the resume state).
    fn write_state(&mut self) -> bool {
        let Some(path) = crate::config::state_path() else {
            return false;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let data = self.machine.save_state();
        match crate::config::write_atomic(&path, &data) {
            Ok(()) => {
                log::info!("saved resume state: {} bytes -> {}", data.len(), path.display());
                // A resume state exists now — the first-run help cue is over.
                self.help_banner = false;
                true
            }
            Err(e) => {
                log::error!("resume-state save failed: {e}");
                false
            }
        }
    }

    /// How many in-memory disk images carry unexported changes — what a state
    /// load silently rolls back, so the warnings count them first.
    fn dirty_disk_count(&self) -> usize {
        self.machine.bus().disk.in_memory_disks().iter().filter(|d| d.dirty).count()
    }

    /// Swap the running machine for the state in `bytes` (the resume state or
    /// a snapshot file) and resync everything that hangs off it: the audio
    /// rate, the media bookkeeping, and the window title. The state names its
    /// own media (host identities, save format v3); the machine — not
    /// whatever was mounted a moment ago — drives the title and the exit
    /// bookkeeping. Bytes stay empty in the items: the images live in the
    /// machine, and a later cartridge change replaces the item, bytes and
    /// all, before any rebuild reads it.
    fn apply_state(&mut self, bytes: &[u8]) -> Result<(), StateError> {
        self.machine.load_state(bytes)?;
        // The restored PSG carries the sample rate from the save file;
        // re-point it at the live audio device.
        if let Some(audio) = &self.audio {
            self.machine.set_audio_sample_rate(audio.sample_rate());
        }
        self.cart = match (self.machine.cart_key(), self.machine.has_cartridge()) {
            (Some(k), _) => Some(MediaItem { path: PathBuf::from(k), bytes: Vec::new() }),
            (None, false) => None,
            // A pre-v3 state carries a cartridge but no identity — keep the
            // current bookkeeping rather than claim the console is bare.
            (None, true) => self.cart.take(),
        };
        self.disk = self.machine.bus().disk.drive_key(0).map(|k| MediaItem {
            path: PathBuf::from(k),
            bytes: Vec::new(),
        });
        self.sync_title();
        Ok(())
    }

    /// Save (`F6`): write the whole machine to the resume state and confirm on
    /// screen.
    fn save_state(&mut self) {
        let ok = self.write_state();
        self.flash(if ok { "RESUME STATE SAVED" } else { "SAVE FAILED" });
    }

    /// Load (`F8`): restore the machine from the resume state, replacing the
    /// running session. If the session holds unexported disk changes the load
    /// would roll back, a **native warning** asks first.
    fn load_state(&mut self) {
        let Some(path) = crate::config::state_path() else {
            self.flash("LOAD FAILED");
            return;
        };
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::warn!("no resume state at {}: {e}", path.display());
                self.flash("NO RESUME STATE");
                return;
            }
        };
        let dirty = self.dirty_disk_count();
        if dirty > 0 {
            // Modal dialog: same input caveats as the F9 chooser.
            self.release_all_keys();
            self.speed.set_turbo(false);
            if !media::confirm_resume_load(dirty) {
                return;
            }
        }
        match self.apply_state(&bytes) {
            Ok(()) => {
                log::info!("loaded resume state from {}", path.display());
                self.flash("RESUME STATE LOADED");
            }
            Err(e) => {
                log::error!("resume-state load failed: {e}");
                self.flash("LOAD FAILED");
            }
        }
    }

    /// Default file name offered for a new snapshot: the mounted cartridge's
    /// stem (`parsec.ti99`), else `console.ti99`.
    fn snapshot_suggested_name(&self) -> String {
        let stem = self
            .cart
            .as_ref()
            .and_then(|m| m.path.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "console".into());
        format!("{stem}.ti99")
    }

    /// `Shift`+`F6`: save a **snapshot** — the whole machine, exactly like the
    /// resume state — to a user-named `.ti99` file via the native save dialog
    /// (whose replace-prompt guards overwrites). The resume state itself is
    /// untouched.
    fn save_snapshot(&mut self) {
        // Modal dialog: same input caveats as the F9 chooser.
        self.release_all_keys();
        self.speed.set_turbo(false);
        let suggested = self.snapshot_suggested_name();
        let Some(path) = media::save_snapshot_file(&self.browser_dir, &suggested) else {
            return; // canceled
        };
        let data = self.machine.save_state();
        match crate::config::write_atomic(&path, &data) {
            Ok(()) => {
                if let Some(dir) = path.parent() {
                    self.browser_dir = dir.to_path_buf();
                }
                log::info!("saved snapshot: {} bytes -> {}", data.len(), path.display());
                self.flash("SNAPSHOT SAVED");
            }
            Err(e) => {
                log::error!("snapshot save to {} failed: {e}", path.display());
                self.flash("SNAPSHOT FAILED (SEE LOG)");
            }
        }
    }

    /// `Shift`+`F8`: load a snapshot file picked with the native open dialog.
    /// A **native warning** first: the snapshot replaces the running machine
    /// and becomes the resume state — which is rewritten immediately after a
    /// successful load, so the warning is true the moment it happens, not
    /// only at exit.
    fn load_snapshot(&mut self) {
        self.release_all_keys();
        self.speed.set_turbo(false);
        let Some(path) = media::pick_snapshot_file(&self.browser_dir) else {
            return; // canceled
        };
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) => {
                log::warn!("cannot read snapshot {}: {e}", path.display());
                self.flash("CANNOT READ SNAPSHOT (SEE LOG)");
                return;
            }
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        if !media::confirm_snapshot_load(&name, self.dirty_disk_count()) {
            return;
        }
        match self.apply_state(&bytes) {
            Ok(()) => {
                if let Some(dir) = path.parent() {
                    self.browser_dir = dir.to_path_buf();
                }
                self.write_state(); // the snapshot is the resume state now
                log::info!("loaded snapshot {}", path.display());
                self.flash("SNAPSHOT LOADED");
            }
            Err(e) => {
                log::error!("snapshot load of {} failed: {e}", path.display());
                self.flash("NOT A USABLE SAVE STATE");
            }
        }
    }

    /// `Shift`+`F5`: **fresh start** — delete the resume state and restart as
    /// a first run, after a **native warning** that spells out what goes: the
    /// resume state permanently, and every in-memory disk image (the machine
    /// restarts bare; files on the host are never touched).
    fn fresh_start(&mut self) {
        self.release_all_keys();
        self.speed.set_turbo(false);
        let disks = self.machine.bus().disk.in_memory_disks();
        let dirty = disks.iter().filter(|d| d.dirty).count();
        if !media::confirm_fresh_start(disks.len(), dirty) {
            return;
        }
        if let Some(path) = crate::config::state_path() {
            match std::fs::remove_file(&path) {
                Ok(()) => log::info!("deleted the resume state at {}", path.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => log::warn!("could not delete the resume state at {}: {e}", path.display()),
            }
        }
        self.cart = None;
        self.disk = None;
        // A genuinely fresh machine: no cartridge, no disk, empty disk memory —
        // deliberately *not* the disk-preserving rebuild a cartridge change uses.
        self.machine = crate::build_machine(None);
        if let Some(audio) = &self.audio {
            self.machine.set_audio_sample_rate(audio.sample_rate());
        }
        // Session bookkeeping starts over too; the browser directory is a
        // preference, not session state, and stays.
        crate::config::update_session("", "", &self.browser_dir.display().to_string());
        self.sync_title();
        // Back to a true first run — the onboarding banner returns with it.
        self.help_banner = true;
        log::info!("fresh start: session reset to a bare console");
        self.flash("FRESH START - RESUME STATE DELETED");
    }

    /// Open the help overlay (on the Start tab), releasing any held keys first so
    /// none stick while TI input is suspended.
    fn open_keyboard_help(&mut self) {
        self.release_all_keys();
        self.keyboard_help = true;
        self.help_tab = HelpTab::Start;
        // The banner's one job — pointing at this screen — is done.
        self.help_banner = false;
    }

    /// Route a key to the open help overlay: close it, or switch tabs (`1`–`4`
    /// jump, `Tab`/`Shift`+`Tab` and `←`/`→` cycle).
    fn help_key(&mut self, code: KeyCode) {
        use KeyCode::*;
        match code {
            F1 | Escape => self.keyboard_help = false,
            Tab => self.help_tab = self.help_tab.cycle(if self.host_mods.shift { -1 } else { 1 }),
            ArrowRight => self.help_tab = self.help_tab.cycle(1),
            ArrowLeft => self.help_tab = self.help_tab.cycle(-1),
            Digit1 => self.help_tab = HelpTab::Start,
            Digit2 => self.help_tab = HelpTab::Keyboard,
            Digit3 => self.help_tab = HelpTab::Hotkeys,
            Digit4 => self.help_tab = HelpTab::Settings,
            _ => {}
        }
    }

    /// Paint on-screen overlays into the framebuffer after the machine has
    /// rendered the frame: the keyboard reference card (when open) takes the whole
    /// screen; otherwise the transient status toast.
    fn draw_overlays(&mut self) {
        // Live CPU inspector (top-left), non-modal so the machine keeps running.
        if self.debug_overlay {
            let mut canvas = text::Canvas::new(&mut self.framebuffer, WIDTH, HEIGHT);
            crate::debug::render(&mut canvas, &self.machine);
        }
        // Persistent speed indicator (top-right) while paused or fast-forwarding.
        if let Some(label) = self.speed.indicator() {
            let w = text::text_width(label, 1) + 6;
            let x = WIDTH.saturating_sub(w + 2);
            let mut canvas = text::Canvas::new(&mut self.framebuffer, WIDTH, HEIGHT);
            canvas.dim_rect(x, 2, w, text::GLYPH_H + 4, 2);
            canvas.draw_text(x + 3, 4, label, 0x00FF_EE33, 1);
        }
        // The disk-memory overlay (F4): in-memory disks, export/unload.
        if self.disk_ui.open {
            let disks = self.machine.bus().disk.in_memory_disks();
            let mut canvas = text::Canvas::new(&mut self.framebuffer, WIDTH, HEIGHT);
            self.disk_ui.render(&mut canvas, &disks);
        }
        // First-run onboarding banner (bottom center, persistent): shown while
        // there is no resume state, yielding to the F4 overlay and to a live
        // toast (which uses the same bottom band).
        if self.help_banner && !self.disk_ui.open && self.toast_frames == 0 {
            let label = "PRESS ESC FOR HELP";
            let w = text::text_width(label, 1) + 6;
            let x = WIDTH.saturating_sub(w) / 2;
            let band = text::GLYPH_H + 4;
            let y = HEIGHT.saturating_sub(band + 6);
            let mut canvas = text::Canvas::new(&mut self.framebuffer, WIDTH, HEIGHT);
            canvas.dim_rect(x, y, w, band, 2);
            canvas.draw_text(x + 3, y + 2, label, 0x00FF_FFFF, 1);
        }
        if self.toast_frames == 0 {
            return;
        }
        self.toast_frames -= 1;
        let Some(message) = self.toast.clone() else {
            return;
        };
        let scale = 2;
        let pad = 4;
        let band = text::GLYPH_H * scale + pad * 2;
        let y = HEIGHT.saturating_sub(band + 6);
        let tw = text::text_width(&message, scale);
        let x = WIDTH.saturating_sub(tw) / 2;
        let mut canvas = text::Canvas::new(&mut self.framebuffer, WIDTH, HEIGHT);
        canvas.dim_rect(0, y, WIDTH, band, 2);
        canvas.draw_text(x, y + pad, &message, 0x0000_FF66, scale);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let size = LogicalSize::new(WIDTH as u32 * self.scale, HEIGHT as u32 * self.scale);
        let attributes = Window::default_attributes()
            .with_title("Libre99")
            .with_inner_size(size);
        // Window/graphics-context creation failing is unrecoverable — there is no
        // app without a window — so log a clear fatal message and ask the event
        // loop to exit cleanly instead of unwinding with a raw backtrace.
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Rc::new(window),
            Err(e) => {
                log::error!("fatal: could not create the window: {e}");
                event_loop.exit();
                return;
            }
        };
        window.set_title(&self.title());
        if self.fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(context) => context,
            Err(e) => {
                log::error!("fatal: could not create the graphics context: {e}");
                event_loop.exit();
                return;
            }
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(surface) => surface,
            Err(e) => {
                log::error!("fatal: could not create the drawing surface: {e}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window);
        self.surface = Some(surface);
        self.next_frame = Instant::now();
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                let down = event.state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = event.physical_key {
                    // While the help overlay is open it captures input: its keys
                    // switch tabs or close it, and nothing reaches the TI.
                    if self.keyboard_help {
                        if down {
                            self.help_key(code);
                        }
                        return;
                    }
                    // Same for the disk-memory overlay (the machine keeps
                    // running underneath; TI input is suspended).
                    if self.disk_ui.open {
                        if down && !event.repeat {
                            self.disk_ui_key(code);
                        }
                        return;
                    }
                    // One-shot emulator hotkeys. Skip auto-repeat events so a
                    // held key can't fire them many times — e.g. F11 toggling
                    // fullscreen on and straight back off mid-transition.
                    if !event.repeat {
                        match code {
                            KeyCode::F1 | KeyCode::Escape if down => self.open_keyboard_help(),
                            KeyCode::F9 if down => self.mount_via_dialog(),
                            KeyCode::F2 if down => self.eject(MediaKind::Cartridge),
                            KeyCode::F3 if down => self.eject(MediaKind::Disk),
                            KeyCode::F4 if down => self.toggle_disk_ui(),
                            // Shift-guarded arms must precede their bare keys.
                            KeyCode::F5 if down && self.host_mods.shift => self.fresh_start(),
                            KeyCode::F5 if down => self.machine.reset(),
                            KeyCode::F7 if down => self.toggle_layout(),
                            // Fullscreen: F11 (cross-platform) or the macOS-standard
                            // Ctrl+Cmd+F. macOS binds bare F11 to Mission Control's
                            // "Show Desktop" by default and swallows it, so Ctrl+Cmd+F
                            // is the binding that reliably reaches us there.
                            KeyCode::F11 if down => self.toggle_fullscreen(),
                            KeyCode::KeyF if down && self.host_mods.cmd && self.host_mods.ctrl => {
                                self.toggle_fullscreen()
                            }
                            KeyCode::F6 if down && self.host_mods.shift => self.save_snapshot(),
                            KeyCode::F6 if down => self.save_state(),
                            KeyCode::F8 if down && self.host_mods.shift => self.load_snapshot(),
                            KeyCode::F8 if down => self.load_state(),
                            KeyCode::F10 if down => self.toggle_pause(),
                            KeyCode::F12 if down => self.speed.frame_advance(),
                            // Hold Tab to fast-forward (acts on press *and* release).
                            KeyCode::Tab => self.speed.set_turbo(down),
                            // Command-modifier shortcuts, so the letter keys stay free
                            // to type to the TI: Cmd+S / Cmd+D on macOS, Ctrl+S / Ctrl+D
                            // elsewhere (Win+key is OS-reserved). `HostMods::command()`
                            // picks the modifier; `input::resolve` withholds exactly
                            // these keys from the TI so the TI CTRL layer keeps working
                            // for every other Ctrl chord. S = screenshot, D = CPU panel.
                            KeyCode::KeyS if down && self.host_mods.command() => self.screenshot(),
                            KeyCode::KeyD if down && self.host_mods.command() => {
                                self.debug_overlay = !self.debug_overlay
                            }
                            _ => {}
                        }
                    }
                    self.set_ti_keys(code, &event, down);
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                self.host_mods = input::HostMods {
                    alt: state.alt_key(),
                    ctrl: state.control_key(),
                    cmd: state.super_key(),
                    shift: state.shift_key(),
                };
            }
            WindowEvent::Focused(false) => {
                // Don't let keys or modifiers stick when we lose focus mid-press.
                self.release_all_keys();
                self.host_mods = input::HostMods::default();
                // Tab (fast-forward) is hold-to-run state, not a matrix key:
                // its release also goes elsewhere on focus loss, so drop turbo
                // here or the machine keeps racing in the background.
                self.speed.set_turbo(false);
            }
            // Coming back from another app (regaining focus) or having the window
            // revealed after being hidden behind one (occlusion ending) can leave
            // the softbuffer surface showing a stale/blank backing: our frame loop
            // is timer-driven, and macOS throttles or drops those timer-driven
            // redraws while we're in the background — and winit doesn't request a
            // repaint on its own when we return. Without this, the window sits on a
            // solid background color until the next input event nudges the loop.
            // Resync pacing and force an immediate repaint.
            WindowEvent::Focused(true) | WindowEvent::Occluded(false) => {
                self.next_frame = Instant::now();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                // Run 0 (paused), 1 (normal), or several (fast-forward) emulated
                // frames for this displayed frame.
                let frames = self.speed.frames_this_tick();
                for _ in 0..frames {
                    self.machine.run_frame();
                }
                // Opt-in: hold the console's anti-burn-in screen-blank off. The
                // ROM's VBLANK ISR advances a timeout counter at scratchpad
                // `>83D6` (+2/tick) and blanks the display when it wraps; keeping
                // it near zero is exactly what a keypress does, so the picture
                // never blanks while idle. Only when the machine actually advanced.
                if self.defeat_screen_blank && frames > 0 {
                    self.machine.bus_mut().poke_word(0x83D6, 0);
                }
                // The help overlay covers the screen at native resolution (drawn in
                // present), so skip the hidden 256×192 frame and its overlays while
                // it is open — the machine still advances for audio/timing.
                if !self.keyboard_help {
                    self.machine.render(&mut self.framebuffer);
                    self.draw_overlays();
                }
                // Only feed audio when the machine advanced, so a pause goes
                // silent instead of droning the last tone.
                if frames > 0 {
                    self.feed_audio();
                }
                // The DSK1 dirty marker in the title can flip on any frame the
                // DSR writes; this only touches the window on a real change.
                self.sync_title();
                self.present();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Pure pacing arithmetic (schedule the next frame; resync if we fell far
        // behind) lives in `pacing::advance` so it can be unit-tested.
        let (next_frame, redraw) = pacing::advance(Instant::now(), self.next_frame, FRAME);
        self.next_frame = next_frame;
        if redraw {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
    }

    /// The event loop is shutting down (window close, `Cmd`+`Q` on macOS / `Alt`+`F4`
    /// on Windows, any exit) — write the resume state so the next launch
    /// resumes here.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.write_state();
        // Record the mounted media's file paths and the browser directory.
        // The resume state itself names its media since format v3; these
        // preferences remain the fallback identities for pre-v3 states, and
        // the browser reopens where the user was working.
        let cart = self.cart.as_ref().map(|m| m.path.display().to_string()).unwrap_or_default();
        let disk = self.disk.as_ref().map(|m| m.path.display().to_string()).unwrap_or_default();
        crate::config::update_session(&cart, &disk, &self.browser_dir.display().to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A windowless, audioless app around a bare machine — enough to exercise
    /// overlay and banner logic without a desktop session.
    fn headless(show_help_banner: bool) -> App {
        App::new(
            crate::build_machine(None),
            Options {
                scale: 1,
                fullscreen: false,
                audio: false,
                volume: 1.0,
                cart: None,
                disk: None,
                browser_dir: PathBuf::from("."),
                key_layout: KeyLayout::Character,
                defeat_screen_blank: false,
                show_help_banner,
            },
        )
    }

    #[test]
    fn the_help_banner_retires_when_the_help_opens() {
        let mut app = headless(true);
        assert!(app.help_banner, "no resume state at launch shows the banner");
        app.open_keyboard_help();
        assert!(app.keyboard_help);
        assert!(!app.help_banner, "opening the help retires the banner");
        assert!(!headless(false).help_banner, "a resumed session shows no banner");
    }

    #[test]
    fn the_help_banner_paints_the_bottom_band_only_while_active() {
        let flat = 0x0012_3456u32;
        let mut app = headless(true);
        app.framebuffer.fill(flat);
        app.draw_overlays();
        assert!(
            app.framebuffer.iter().any(|&p| p != flat),
            "an active banner paints the frame"
        );
        let mut app = headless(false);
        app.framebuffer.fill(flat);
        app.draw_overlays();
        assert!(
            app.framebuffer.iter().all(|&p| p == flat),
            "without the banner the frame is untouched"
        );
    }
}
