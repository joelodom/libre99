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

//! Wall-clock frame pacing arithmetic, factored out of the winit loop so it can
//! be reasoned about and unit-tested without an event loop.
//!
//! The application wakes on a timer (`ControlFlow::WaitUntil`) and asks
//! [`advance`] whether this wakeup is due and when the next one should fire. The
//! normal cadence steps the deadline forward one whole frame; if we wake up
//! already a full frame past the deadline (the app stalled — a slow redraw, or
//! the OS throttled our timer while backgrounded), we resync to `now + frame`
//! instead of replaying the backlog, so the loop can't spiral chasing missed
//! frames.

use std::time::{Duration, Instant};

/// Given `now`, the currently-scheduled frame `deadline`, and the frame period
/// `frame`, return the next deadline and whether a redraw is due this wakeup.
///
/// * Not yet due (`now < deadline`): keep the deadline, no redraw.
/// * Due: redraw, and advance the deadline one `frame` — unless that stepped
///   deadline is still in the past (we fell behind), in which case resync to
///   `now + frame`.
///
/// This is exactly the pacing the winit `about_to_wait` handler runs; keeping it
/// pure makes the fell-behind resync testable.
pub fn advance(now: Instant, deadline: Instant, frame: Duration) -> (Instant, bool) {
    if now < deadline {
        return (deadline, false);
    }
    let stepped = deadline + frame;
    let next = if stepped <= now { now + frame } else { stepped };
    (next, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Duration = Duration::from_micros(16_667);

    #[test]
    fn not_yet_due_keeps_the_deadline_and_skips_the_redraw() {
        let t0 = Instant::now();
        let deadline = t0 + FRAME;
        let (next, redraw) = advance(t0, deadline, FRAME);
        assert_eq!(next, deadline);
        assert!(!redraw);
    }

    #[test]
    fn on_time_redraws_and_advances_exactly_one_frame() {
        let t0 = Instant::now();
        let (next, redraw) = advance(t0, t0, FRAME);
        assert!(redraw);
        assert_eq!(next, t0 + FRAME);
    }

    #[test]
    fn slightly_late_keeps_cadence_from_the_old_deadline() {
        let t0 = Instant::now();
        let deadline = t0;
        // Less than a whole frame late: the stepped deadline is still ahead of
        // `now`, so we keep the fixed cadence (no drift accumulation).
        let now = t0 + FRAME / 2;
        let (next, redraw) = advance(now, deadline, FRAME);
        assert!(redraw);
        assert_eq!(next, deadline + FRAME);
    }

    #[test]
    fn a_long_stall_resyncs_to_now_plus_one_frame() {
        let t0 = Instant::now();
        let deadline = t0;
        // Stalled well over a frame: `deadline + FRAME` is still in the past, so
        // resync to `now + FRAME` instead of firing catch-up redraws.
        let now = t0 + FRAME * 10;
        let (next, redraw) = advance(now, deadline, FRAME);
        assert!(redraw);
        assert_eq!(next, now + FRAME);
    }
}
