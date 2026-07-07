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

//! The **disk memory** overlay (`F4`): lists every disk image the emulator is
//! holding in memory — mounted in DSK1 or shelved after an eject — with its
//! CHANGED/CLEAN status, and drives the two actions on them: **export** the
//! delta-applied image to a host `.dsk` (via the OS-native save dialog, which
//! is what prompts before any overwrite) and **unload** it from memory so the
//! next mount re-reads the host file. The overlay itself is drawn with the
//! [`text::Canvas`] framework like the other HUD panels — only the file
//! dialogs are native. The machine keeps running underneath; TI key input is
//! suspended while it is open.

use libre99_core::disk::DiskInfo;
use winit::keyboard::KeyCode;

use crate::text::{self, Canvas, LINE};

/// What the app should do in response to a key routed to the open overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiskAction {
    None,
    /// Close the overlay.
    Close,
    /// Export the image with this key to a host `.dsk` (native save dialog).
    Export(String),
    /// Unload the image with this key from memory (confirming first if dirty).
    Unload(String),
}

/// Overlay state: whether it is showing and which row is selected.
#[derive(Default)]
pub struct DiskOverlay {
    pub open: bool,
    selected: usize,
}

impl DiskOverlay {
    /// Open the overlay with the selection reset to the first row.
    pub fn open(&mut self) {
        self.open = true;
        self.selected = 0;
    }

    /// Route a key press to the overlay. `disks` is the current in-memory list
    /// (fetched fresh by the caller, so the selection always acts on what is
    /// actually shown).
    pub fn handle_key(&mut self, code: KeyCode, disks: &[DiskInfo]) -> DiskAction {
        self.clamp(disks.len());
        let selected_key = || disks.get(self.selected).map(|d| d.key.clone());
        match code {
            KeyCode::Escape | KeyCode::F4 => DiskAction::Close,
            KeyCode::ArrowUp => {
                self.selected = self.selected.saturating_sub(1);
                DiskAction::None
            }
            KeyCode::ArrowDown => {
                if self.selected + 1 < disks.len() {
                    self.selected += 1;
                }
                DiskAction::None
            }
            KeyCode::Enter | KeyCode::KeyE => selected_key().map_or(DiskAction::None, DiskAction::Export),
            KeyCode::KeyU | KeyCode::Delete => selected_key().map_or(DiskAction::None, DiskAction::Unload),
            _ => DiskAction::None,
        }
    }

    /// Keep the selection on a real row as the list changes underneath it.
    fn clamp(&mut self, len: usize) {
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }

    /// Draw the overlay over the rendered frame.
    pub fn render(&mut self, canvas: &mut Canvas, disks: &[DiskInfo]) {
        const TITLE: u32 = 0x00FF_EE33;
        const INK: u32 = 0x00E8_E8E8;
        const DIM: u32 = 0x009B_A6C0;
        const CHANGED: u32 = 0x00FF_B347;
        const SELECT: u32 = 0x0000_FF66;

        self.clamp(disks.len());
        let x = 8;
        let w = 240;
        let rows = disks.len().max(1);
        let h = 3 * LINE + rows * LINE + 2 * LINE + 6;
        let y = 16;
        canvas.dim_rect(x, y, w, h, 3);

        canvas.draw_text(x + 4, y + 3, "DISK MEMORY", TITLE, 1);
        // Keep this line inside the 240px panel: 38 chars max at scale 1.
        canvas.draw_text(x + 4, y + 3 + LINE, "HOST .DSK FILES ARE NEVER WRITTEN", DIM, 1);

        let list_top = y + 3 + 3 * LINE;
        if disks.is_empty() {
            canvas.draw_text(x + 10, list_top, "(NO DISKS IN MEMORY)", DIM, 1);
        }
        for (i, d) in disks.iter().enumerate() {
            let row_y = list_top + i * LINE;
            let ink = if i == self.selected { INK } else { DIM };
            if i == self.selected {
                canvas.draw_text(x + 4, row_y, ">", SELECT, 1);
            }
            let mut name = file_name(&d.key).to_ascii_uppercase();
            name.truncate(18);
            canvas.draw_text(x + 10, row_y, &name, ink, 1);
            if let Some(drive) = d.drive {
                canvas.draw_text(x + 124, row_y, &format!("DSK{}", drive + 1), SELECT, 1);
            }
            let status = if d.dirty { "CHANGED" } else { "CLEAN" };
            canvas.draw_text(x + 154, row_y, status, if d.dirty { CHANGED } else { DIM }, 1);
            let size = format!("{:>4}K", d.len / 1024);
            canvas.draw_text(x + w - 4 - text::text_width(&size, 1), row_y, &size, ink, 1);
        }

        let footer_y = list_top + rows * LINE + LINE;
        canvas.draw_text(x + 4, footer_y, "ENTER EXPORT   U UNLOAD   ESC CLOSE", DIM, 1);
    }
}

/// The final path component of a disk key, for display and as the suggested
/// export file name (keys are path strings; both separators may appear).
pub fn file_name(key: &str) -> &str {
    key.rsplit(['/', '\\']).next().unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(key: &str, dirty: bool, drive: Option<usize>) -> DiskInfo {
        DiskInfo { key: key.into(), len: 92_160, dirty, drive }
    }

    #[test]
    fn file_name_takes_the_last_component_of_either_separator() {
        assert_eq!(file_name(r"C:\disks\GAME.dsk"), "GAME.dsk");
        assert_eq!(file_name("/Users/joel/disks/game.dsk"), "game.dsk");
        assert_eq!(file_name("bare.dsk"), "bare.dsk");
    }

    #[test]
    fn keys_navigate_act_and_close() {
        let disks = [info("a.dsk", false, Some(0)), info("b.dsk", true, None)];
        let mut ui = DiskOverlay::default();
        ui.open();
        assert_eq!(ui.handle_key(KeyCode::ArrowDown, &disks), DiskAction::None);
        assert_eq!(ui.handle_key(KeyCode::Enter, &disks), DiskAction::Export("b.dsk".into()));
        assert_eq!(ui.handle_key(KeyCode::KeyU, &disks), DiskAction::Unload("b.dsk".into()));
        assert_eq!(ui.handle_key(KeyCode::ArrowDown, &disks), DiskAction::None, "clamps at end");
        assert_eq!(ui.handle_key(KeyCode::Escape, &disks), DiskAction::Close);
    }

    #[test]
    fn an_empty_list_yields_no_actions_and_no_panic() {
        let mut ui = DiskOverlay::default();
        ui.open();
        assert_eq!(ui.handle_key(KeyCode::Enter, &[]), DiskAction::None);
        assert_eq!(ui.handle_key(KeyCode::KeyU, &[]), DiskAction::None);
        assert_eq!(ui.handle_key(KeyCode::ArrowDown, &[]), DiskAction::None);
    }

    #[test]
    fn the_selection_clamps_when_the_list_shrinks() {
        let two = [info("a.dsk", false, None), info("b.dsk", false, None)];
        let one = [info("a.dsk", false, None)];
        let mut ui = DiskOverlay::default();
        ui.open();
        ui.handle_key(KeyCode::ArrowDown, &two);
        assert_eq!(ui.handle_key(KeyCode::Enter, &one), DiskAction::Export("a.dsk".into()));
    }
}
