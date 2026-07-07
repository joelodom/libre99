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

//! SN76489 PSG conformance tests — milestone 8.

use std::collections::BTreeSet;
use libre99_core::psg::Psg;

/// Latch byte: channel `ch`, type `t` (0 = tone/noise, 1 = attenuation), low
/// nibble `d`.
fn latch(ch: u8, t: u8, d: u8) -> u8 {
    0x80 | (ch << 5) | (t << 4) | (d & 0x0F)
}

#[test]
fn tone_period_and_frequency_decode() {
    let mut p = Psg::new(44_100);
    // Divider 254 (latch the low nibble, then the high 6 bits) ≈ A4 (440 Hz).
    p.write(latch(0, 0, 254 & 0x0F));
    p.write((254 >> 4) & 0x3F);
    assert_eq!(p.period(0), 254);
    assert!((p.frequency(0) - 440.3).abs() < 1.0, "freq = {}", p.frequency(0));
}

#[test]
fn attenuation_decode() {
    let mut p = Psg::new(44_100);
    p.write(latch(1, 1, 5)); // channel 1 attenuation = 5
    assert_eq!(p.volume(1), 5);
    p.write(latch(2, 1, 0)); // channel 2 fully loud
    assert_eq!(p.volume(2), 0);
}

#[test]
fn noise_control_selects_white_or_periodic() {
    let mut p = Psg::new(44_100);
    p.write(latch(3, 0, 0x04)); // white noise, rate 0
    assert!(p.noise_white());
    p.write(latch(3, 0, 0x01)); // periodic noise, rate 1
    assert!(!p.noise_white());
    assert_eq!(p.lfsr(), 0x4000, "a noise-control write resets the LFSR");
}

#[test]
fn periodic_noise_walks_a_single_bit_through_15_states() {
    let mut p = Psg::new(8_000); // low rate → the LFSR advances quickly
    p.write(latch(3, 0, 0x00)); // periodic noise, rate 0
    let mut seen = BTreeSet::new();
    for _ in 0..2_000 {
        p.next_sample();
        seen.insert(p.lfsr());
    }
    assert_eq!(seen.len(), 15, "states: {:?}", seen);
    for &v in &seen {
        assert_eq!(v.count_ones(), 1, "one bit set");
        assert!(v < 0x8000, "15-bit register");
    }
}

#[test]
fn white_noise_visits_many_states() {
    let mut p = Psg::new(8_000);
    p.write(latch(3, 0, 0x04)); // white noise, rate 0
    let mut seen = BTreeSet::new();
    for _ in 0..5_000 {
        p.next_sample();
        seen.insert(p.lfsr());
    }
    assert!(seen.len() > 50, "white noise should visit many states, got {}", seen.len());
}

#[test]
fn full_attenuation_is_silent() {
    let mut p = Psg::new(44_100);
    // All channels power up attenuated off; an explicit tone at atten 15 is
    // still silent.
    p.write(latch(0, 0, 0x0E));
    p.write(0x0F);
    p.write(latch(0, 1, 0x0F)); // channel 0 attenuation = 15 (off)
    for _ in 0..1_000 {
        assert_eq!(p.next_sample(), 0.0);
    }
}

#[test]
fn an_audible_tone_oscillates_at_its_frequency() {
    let mut p = Psg::new(44_100);
    p.write(latch(0, 0, 254 & 0x0F)); // ≈ 440 Hz
    p.write((254 >> 4) & 0x3F);
    p.write(latch(0, 1, 0x00)); // channel 0 fully loud

    let samples: Vec<f32> = (0..4_410).map(|_| p.next_sample()).collect(); // 0.1 s
    assert!(samples.iter().any(|&s| s.abs() > 0.1), "tone should be audible");

    // A 440 Hz square wave flips sign ~2·440·0.1 = 88 times in 0.1 s.
    let crossings = samples
        .windows(2)
        .filter(|w| (w[0] > 0.0) != (w[1] > 0.0))
        .count();
    assert!((crossings as i32 - 88).abs() < 10, "sign changes = {}", crossings);
}

#[test]
fn sound_port_routes_to_the_psg() {
    use libre99_core::bus::Bus;
    use libre99_core::machine::Tms9900Bus;
    let mut bus = Tms9900Bus::new(&[], &[]);
    bus.write_byte(0x8400, latch(0, 0, 254 & 0x0F));
    bus.write_byte(0x8400, (254 >> 4) & 0x3F);
    assert_eq!(bus.psg.period(0), 254, "writes to >8400 reach the sound chip");
}

/// Peak `|sample|` of a single audible tone at attenuation `atten` (all other
/// channels stay off, so the mix is just this channel scaled by `VOLUME[atten]`).
fn tone_peak(atten: u8) -> f32 {
    let mut p = Psg::new(44_100);
    p.write(latch(0, 0, 8)); // divider 40 → an audible ~2.8 kHz square
    p.write(0x02);
    p.write(latch(0, 1, atten)); // channel 0 attenuation
    (0..2_000).map(|_| p.next_sample().abs()).fold(0.0f32, f32::max)
}

#[test]
fn mid_table_attenuation_step_is_two_decibels_down() {
    // Attenuation is 2 dB per step: index 1 is 10^(−1/10) ≈ 0.7943 of full scale.
    // Testing the *ratio* pins VOLUME[1] independently of the mixer's output scale.
    let full = tone_peak(0);
    let step1 = tone_peak(1);
    assert!(full > 0.0, "index 0 must be audible");
    let ratio = step1 / full;
    assert!(
        (ratio - 0.794_328).abs() < 0.01,
        "attenuation step 1 ratio = {ratio}, expected ≈ 0.794"
    );
}

/// Count LFSR shifts of the white-noise channel over a fixed window at a fixed
/// sample rate: a smaller noise divisor clocks the shift register faster. The
/// sample rate is chosen so at most one shift falls in any single sample, so
/// counting register-value changes counts shifts. `ctrl_low` selects the shift
/// rate (1 → >20, 2 → >40, 3 → follow tone-2); `tone2_period` sets channel 2's
/// divider for the follow-tone-2 mode.
fn noise_shifts(ctrl_low: u8, tone2_period: u16) -> usize {
    let mut p = Psg::new(40_000);
    if tone2_period != 0 {
        p.write(latch(2, 0, (tone2_period & 0x0F) as u8));
        p.write(((tone2_period >> 4) & 0x3F) as u8);
    }
    p.write(latch(3, 0, 0x04 | ctrl_low)); // white noise + shift-rate select
    let mut count = 0usize;
    let mut prev = p.lfsr();
    for _ in 0..20_000 {
        p.next_sample();
        if p.lfsr() != prev {
            count += 1;
            prev = p.lfsr();
        }
    }
    count
}

#[test]
fn noise_shift_rate_tracks_control_bits_and_follows_tone_two() {
    let r20 = noise_shifts(1, 0); // divisor >20 = 32
    let r40 = noise_shifts(2, 0); // divisor >40 = 64
    let follow_fast = noise_shifts(3, 0x10); // follow tone-2, small period → fast
    let follow_slow = noise_shifts(3, 0x100); // follow tone-2, large period → slow

    assert!(r20 > 0 && r40 > 0, "both fixed rates must clock the LFSR");
    // Halving the divisor doubles the shift rate: >20 clocks ~2× as often as >40.
    let ratio = r20 as f64 / r40 as f64;
    assert!((ratio - 2.0).abs() < 0.3, ">20/>40 shift-rate ratio = {ratio}");
    // Follow-tone-2 tracks channel 2's divider, so its rate moves with the period:
    // a period below 32 clocks faster than >20; a period above 64 slower than >40.
    assert!(follow_fast > r20, "follow(16)={follow_fast} faster than >20={r20}");
    assert!(follow_slow < r40, "follow(256)={follow_slow} slower than >40={r40}");
}

#[test]
fn tone_divider_zero_reloads_as_1024_not_silence() {
    // A zero divider reloads as 1024 (Classic99 `sound.cpp:443`), producing a
    // ~109 Hz square (3 579 545 / 32 / 1024) — NOT silence and NOT a divide-by-zero
    // that would hang the reload loop. Channel 0 powers up with divider 0; just make
    // it loud and listen.
    let mut p = Psg::new(44_100);
    p.write(latch(0, 1, 0x00)); // channel 0 fully loud; divider left at 0
    let samples: Vec<f32> = (0..44_100).map(|_| p.next_sample()).collect(); // 1 s
    assert!(samples.iter().any(|&s| s.abs() > 0.1), "N=0 must not be silent");

    // The square flips sign at 3 579 545 / 16 / 1024 ≈ 218.5 Hz (half-cycles/s).
    let crossings = samples
        .windows(2)
        .filter(|w| (w[0] > 0.0) != (w[1] > 0.0))
        .count();
    assert!(
        (crossings as i32 - 218).abs() < 15,
        "sign changes = {crossings}, expected ≈ 218 (divider 0 ⇒ 1024)"
    );
}
