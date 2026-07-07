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

//! # SN76489 Programmable Sound Generator (PSG)
//!
//! The TI-99/4A's sound chip is a Texas Instruments **SN76489** (the TMS9919
//! variant). It has three square-wave **tone** channels and one **noise**
//! channel, and is written one byte at a time through the **write-only** port at
//! `>8400`.
//!
//! ## Register protocol
//! Each write is either a *latch* byte (bit 7 set) or a *data* byte (bit 7
//! clear):
//! * **Latch** `1 cc t dddd` selects channel `cc` (0–2 tone, 3 noise) and type
//!   `t` (0 = tone period / noise control, 1 = attenuation), and writes the low
//!   4 bits `dddd`.
//! * **Data** `0 _ dddddd` writes the high 6 bits of the **last latched** tone
//!   period (forming the 10-bit divider `N`).
//!
//! A tone channel's frequency is `f = 3 579 545 / (32·N)`. Attenuation is 4-bit,
//! 0 = loudest … 15 = silent (2 dB per step). The noise channel's 3-bit control
//! `fr` picks white (`fr & 4`) vs periodic feedback and a shift rate
//! (`>10`/`>20`/`>40`, or `3` = follow tone-2's period).
//!
//! ## Noise generator
//! A **15-bit** linear-feedback shift register (LFSR), reset to `>4000`. White
//! noise XORs taps at bits 0 and 1 back into bit 14; periodic noise feeds bit 0
//! back. The output is the (inverted) low bit. The register is clocked at half
//! the channel rate — matching the real chip's divide-by-two on the noise output.
//!
//! ## Synthesis
//! Samples are produced at the host rate with fractional clock counters: each
//! channel counts down `clock/16` ticks per sample and toggles its square wave
//! (or shifts the LFSR) when the counter underflows. Channels are summed and
//! scaled. This is band-unlimited but matches the chip's tones and noise closely
//! enough for faithful playback.

/// SN76489 master clock on the TI-99/4A (Hz).
const CLOCK: f64 = 3_579_545.0;

/// Per-channel output amplitude for each 4-bit attenuation value: 2 dB per step
/// (`10^(-i/10)`), with 15 = fully off.
const VOLUME: [f32; 16] = [
    1.0, 0.794_328_2, 0.630_957_3, 0.501_187_2, 0.398_107_2, 0.316_227_8,
    0.251_188_6, 0.199_526_2, 0.158_489_3, 0.125_892_5, 0.1, 0.079_432_82,
    0.063_095_73, 0.050_118_72, 0.039_810_72, 0.0,
];

/// The SN76489 sound chip: register state plus the per-channel oscillator state
/// used to synthesise samples.
pub struct Psg {
    /// 10-bit tone dividers for channels 0–2.
    period: [u16; 3],
    /// 4-bit attenuation for channels 0–3 (0 = loud, 15 = off).
    volume: [u8; 4],
    /// 3-bit noise control (channel 3): bit 2 = white, bits 0–1 = shift rate.
    noise_ctrl: u8,
    /// 15-bit noise LFSR.
    lfsr: u16,

    /// Channel selected by the last latch byte (for a following data byte).
    latched_channel: usize,
    /// Type selected by the last latch byte (0 = tone/noise, 1 = attenuation).
    latched_type: u8,

    // --- synthesis state ---
    /// Host sample rate (Hz).
    sample_rate: u32,
    /// Down-counters (in `clock/16` ticks) for the three tones + noise.
    counter: [f64; 4],
    /// Square-wave state (±1) for each tone channel.
    output: [f64; 3],
    /// Noise output sign toggle (the LFSR shifts on its rising edge).
    noise_phase: i8,
}

impl Default for Psg {
    fn default() -> Self {
        Self::new(44_100)
    }
}

impl Psg {
    /// A silent chip (all channels attenuated off) generating `sample_rate` Hz.
    pub fn new(sample_rate: u32) -> Self {
        Psg {
            period: [0; 3],
            volume: [0x0F; 4], // all channels off at power-up
            noise_ctrl: 0,
            lfsr: 0x4000,
            latched_channel: 0,
            latched_type: 0,
            sample_rate: sample_rate.max(1),
            counter: [0.0; 4],
            output: [1.0; 3],
            noise_phase: 1,
        }
    }

    /// Set the host sample rate used for synthesis.
    pub fn set_sample_rate(&mut self, rate: u32) {
        self.sample_rate = rate.max(1);
    }

    /// Handle a write to the sound port `>8400`.
    pub fn write(&mut self, byte: u8) {
        if byte & 0x80 != 0 {
            // Latch + low 4 data bits.
            self.latched_channel = ((byte >> 5) & 0x03) as usize;
            self.latched_type = (byte >> 4) & 0x01;
            self.apply((byte & 0x0F) as u16, true);
        } else {
            // Data byte: high 6 bits of the latched register.
            self.apply((byte & 0x3F) as u16, false);
        }
    }

    fn apply(&mut self, data: u16, latch: bool) {
        let ch = self.latched_channel;
        if self.latched_type == 1 {
            self.volume[ch] = (data & 0x0F) as u8;
        } else if ch < 3 {
            self.period[ch] = if latch {
                (self.period[ch] & 0x03F0) | (data & 0x0F)
            } else {
                (self.period[ch] & 0x000F) | (data << 4)
            };
        } else {
            self.noise_ctrl = (data & 0x07) as u8;
            self.lfsr = 0x4000; // a noise-control write resets the shift register
        }
    }

    /// Generate one mono sample in `[-1.0, 1.0]`.
    pub fn next_sample(&mut self) -> f32 {
        let ticks = CLOCK / 16.0 / self.sample_rate as f64;
        let mut mix = 0.0f32;

        for ch in 0..3 {
            self.counter[ch] -= ticks;
            while self.counter[ch] <= 0.0 {
                let reload = if self.period[ch] != 0 {
                    self.period[ch]
                } else {
                    0x400
                };
                self.counter[ch] += reload as f64;
                self.output[ch] = -self.output[ch];
            }
            mix += self.output[ch] as f32 * VOLUME[self.volume[ch] as usize];
        }

        let noise_clk = match self.noise_ctrl & 0x03 {
            0 => 0x10,
            1 => 0x20,
            2 => 0x40,
            _ if self.period[2] != 0 => self.period[2],
            _ => 0x400,
        } as f64;
        self.counter[3] -= ticks;
        while self.counter[3] <= 0.0 {
            self.counter[3] += noise_clk;
            self.noise_phase = -self.noise_phase;
            if self.noise_phase > 0 {
                self.shift_lfsr();
            }
        }
        let noise = if self.lfsr & 1 != 0 { -1.0 } else { 1.0 }; // inverted output
        mix += noise * VOLUME[self.volume[3] as usize];

        // Average of four full-scale channels keeps the mix within range.
        mix * 0.25
    }

    /// Fill `buffer` with mono samples.
    pub fn fill(&mut self, buffer: &mut [f32]) {
        for s in buffer.iter_mut() {
            *s = self.next_sample();
        }
    }

    fn shift_lfsr(&mut self) {
        let feedback = if self.noise_ctrl & 0x04 != 0 {
            // White noise: XOR taps at bits 0 and 1.
            (self.lfsr ^ (self.lfsr >> 1)) & 1
        } else {
            // Periodic noise: feed bit 0 back.
            self.lfsr & 1
        };
        self.lfsr = (self.lfsr >> 1) | (feedback << 14);
    }

    // --- accessors (diagnostics / tests) ---

    /// The 10-bit divider of tone channel `ch` (0–2).
    pub fn period(&self, ch: usize) -> u16 {
        self.period[ch]
    }
    /// Tone channel `ch`'s frequency in Hz (`0` if the divider is 0).
    pub fn frequency(&self, ch: usize) -> f64 {
        let n = self.period[ch];
        if n == 0 {
            0.0
        } else {
            CLOCK / (32.0 * n as f64)
        }
    }
    /// The 4-bit attenuation of channel `ch` (0–3).
    pub fn volume(&self, ch: usize) -> u8 {
        self.volume[ch]
    }
    /// Is the noise channel in white-noise (vs periodic) mode?
    pub fn noise_white(&self) -> bool {
        self.noise_ctrl & 0x04 != 0
    }
    /// The current 15-bit noise LFSR value.
    pub fn lfsr(&self) -> u16 {
        self.lfsr
    }

    /// Serialize the chip's register and oscillator state into a save state.
    pub(crate) fn save_state(&self, w: &mut crate::state::StateWriter) {
        for &p in &self.period {
            w.u16(p);
        }
        for &v in &self.volume {
            w.u8(v);
        }
        w.u8(self.noise_ctrl);
        w.u16(self.lfsr);
        w.usize(self.latched_channel);
        w.u8(self.latched_type);
        w.u32(self.sample_rate);
        for &c in &self.counter {
            w.f64(c);
        }
        for &o in &self.output {
            w.f64(o);
        }
        w.u8(self.noise_phase as u8);
    }

    /// Restore the chip's state from a save state. `latched_channel` is masked to
    /// its valid 0–3 range so a corrupt file cannot index out of bounds on a
    /// subsequent register write.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
    ) -> Result<(), crate::state::StateError> {
        for p in self.period.iter_mut() {
            *p = r.u16()?;
        }
        for v in self.volume.iter_mut() {
            *v = r.u8()?;
        }
        self.noise_ctrl = r.u8()?;
        self.lfsr = r.u16()?;
        self.latched_channel = r.usize()? & 0x03;
        self.latched_type = r.u8()?;
        self.sample_rate = r.u32()?.max(1);
        for c in self.counter.iter_mut() {
            *c = r.f64()?;
        }
        for o in self.output.iter_mut() {
            *o = r.f64()?;
        }
        self.noise_phase = r.u8()? as i8;
        Ok(())
    }
}
