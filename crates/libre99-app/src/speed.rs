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

//! Emulation speed control: **pause**, single-**frame advance**, and
//! **fast-forward**.
//!
//! The frame loop asks [`Speed::frames_this_tick`] how many emulated frames to
//! run for each displayed (≈60 Hz) frame: `1` normally, `0` while paused (the
//! picture freezes but overlays keep updating), a burst while fast-forwarding,
//! or exactly `1` for a queued single step. Keeping the policy here (rather than
//! sprinkled through `app.rs`) makes the behavior testable and the feature
//! self-contained.

/// Emulated frames run per displayed frame while fast-forwarding.
const TURBO_FACTOR: usize = 8;

/// Speed/pause state for the frame loop.
#[derive(Default)]
pub struct Speed {
    paused: bool,
    turbo: bool,
    step: bool,
}

impl Speed {
    pub fn new() -> Self {
        Speed::default()
    }

    /// Toggle pause (clearing any queued single step).
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        self.step = false;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Hold-to-fast-forward: set while the key is down, cleared on release.
    pub fn set_turbo(&mut self, on: bool) {
        self.turbo = on;
    }

    /// Pause (if not already) and queue exactly one frame to run — the classic
    /// debugging "advance one frame" control.
    pub fn frame_advance(&mut self) {
        self.paused = true;
        self.step = true;
    }

    /// How many emulated frames to run for the next displayed frame. Consumes a
    /// queued single step.
    pub fn frames_this_tick(&mut self) -> usize {
        if self.paused {
            if self.step {
                self.step = false;
                1
            } else {
                0
            }
        } else if self.turbo {
            TURBO_FACTOR
        } else {
            1
        }
    }

    /// A short HUD label for the current state, or `None` at normal speed.
    pub fn indicator(&self) -> Option<&'static str> {
        if self.paused {
            Some("|| PAUSED")
        } else if self.turbo {
            Some(">> FAST FWD")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacing_follows_pause_turbo_and_step() {
        let mut s = Speed::new();
        assert_eq!(s.frames_this_tick(), 1, "normal speed runs one frame");

        s.set_turbo(true);
        assert_eq!(s.frames_this_tick(), TURBO_FACTOR, "fast-forward bursts frames");
        s.set_turbo(false);

        s.toggle_pause();
        assert!(s.is_paused());
        assert_eq!(s.frames_this_tick(), 0, "paused freezes emulation");
        assert_eq!(s.frames_this_tick(), 0);

        s.frame_advance();
        assert_eq!(s.frames_this_tick(), 1, "a queued step runs exactly one frame");
        assert_eq!(s.frames_this_tick(), 0, "and then stays paused");

        s.toggle_pause();
        assert_eq!(s.frames_this_tick(), 1, "resumed");
    }

    #[test]
    fn indicator_reflects_state() {
        let mut s = Speed::new();
        assert_eq!(s.indicator(), None);
        s.set_turbo(true);
        assert_eq!(s.indicator(), Some(">> FAST FWD"));
        s.toggle_pause();
        assert_eq!(s.indicator(), Some("|| PAUSED"), "pause takes precedence");
    }
}
