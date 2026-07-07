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

//! Audio output: a `cpal` stream that plays the SN76489's samples.
//!
//! The emulator runs on the main (winit) thread and produces mono samples a
//! frame at a time; cpal's callback runs on a separate audio thread. They are
//! decoupled by a small shared queue: each frame the main thread tops the queue
//! up to a short target latency, and the callback drains it (emitting silence on
//! underrun). The device sample rate is whatever cpal reports, which the PSG is
//! told to synthesize at.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// A live audio output stream plus the queue feeding it.
pub struct Audio {
    // The stream must stay alive to keep playing; it is never touched again.
    _stream: cpal::Stream,
    queue: Arc<Mutex<VecDeque<f32>>>,
    sample_rate: u32,
}

impl Audio {
    /// Open the default output device and start a stream, or `None` if no device
    /// / supported format is available (the app then runs silently).
    pub fn new() -> Option<Audio> {
        let device = cpal::default_host().default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let sample_rate = supported.sample_rate().0;
        let channels = supported.channels() as usize;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
        let q = queue.clone();
        let on_error = |e| log::error!("audio stream error: {e}");

        // The real-time callbacks never block or panic: they `try_lock` the shared
        // queue and, on contention or a poisoned lock, output a buffer of silence
        // for this tick rather than `unwrap`-panicking in the audio thread.
        let stream = match format {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config,
                move |data: &mut [f32], _: &_| {
                    let Ok(mut queue) = q.try_lock() else {
                        data.fill(0.0);
                        return;
                    };
                    for frame in data.chunks_mut(channels) {
                        let s = queue.pop_front().unwrap_or(0.0);
                        frame.iter_mut().for_each(|o| *o = s);
                    }
                },
                on_error,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config,
                move |data: &mut [i16], _: &_| {
                    let Ok(mut queue) = q.try_lock() else {
                        data.fill(0);
                        return;
                    };
                    for frame in data.chunks_mut(channels) {
                        let s = queue.pop_front().unwrap_or(0.0);
                        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        frame.iter_mut().for_each(|o| *o = v);
                    }
                },
                on_error,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config,
                move |data: &mut [u16], _: &_| {
                    // Unsigned silence is the mid-scale value (0.0 maps to ~32767).
                    let Ok(mut queue) = q.try_lock() else {
                        data.fill(u16::MAX / 2);
                        return;
                    };
                    for frame in data.chunks_mut(channels) {
                        let s = queue.pop_front().unwrap_or(0.0);
                        let v = ((s.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16;
                        frame.iter_mut().for_each(|o| *o = v);
                    }
                },
                on_error,
                None,
            ),
            _ => return None,
        }
        .ok()?;

        stream.play().ok()?;
        Some(Audio {
            _stream: stream,
            queue,
            sample_rate,
        })
    }

    /// The device's sample rate (Hz) — synthesize the PSG at this rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// How many samples are currently queued for playback.
    pub fn queued(&self) -> usize {
        // Poison-tolerant: a panic in the audio thread must not wedge the main
        // thread's pacing — recover the guard and read the length anyway.
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Enqueue freshly generated samples, dropping them if the queue is already
    /// far ahead (a guard against unbounded latency after a stall).
    pub fn push(&self, samples: &[f32]) {
        let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        if q.len() < self.sample_rate as usize {
            q.extend(samples.iter().copied());
        }
    }
}
