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

//! A **byte census** of the rewritten console GROM against the authentic image
//! — the measurement behind [`QUALITY-ASSESSMENT.md`] §3 and the source of
//! truth for [`grom/SURFACE-MAP.md`]. It classifies every byte of the two 24 KiB
//! images (authentic `994AGROM.Bin` vs our [`crate::system_grom::build_console_grom`]
//! output) and enumerates the **authentic-only runs**: contiguous stretches the
//! authentic firmware populates but ours ships as zeros. Each such run is a
//! fixed-address behaviour or data table a cartridge could reach directly — the
//! defect class of the TI-Invaders / joystick / keytab field bugs — so making
//! the set finite and classified is what bounds the compatibility surface.
//!
//! The functions here are **pure**: they take both images as slices, so the
//! library itself never embeds TI's copyrighted ROM. The census example
//! (`examples/grom_census.rs`) and the census gate (`tests/census.rs`) each
//! load the authentic bytes at run time from the git-ignored `third-party/`
//! directory (via `libre99_core::third_party`) and call in here, so the notion of
//! "authentic-only run" is defined in exactly one place and the map cannot
//! drift from the tool.
//!
//! [`QUALITY-ASSESSMENT.md`]: ../../../original-content/system-roms/history/QUALITY-ASSESSMENT.md
//! [`grom/SURFACE-MAP.md`]: ../../../original-content/system-roms/grom/SURFACE-MAP.md

/// A named, inclusive GROM address range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub name: &'static str,
    pub start: u16,
    pub end: u16,
}

/// The three real 6 KiB GROM chips carry the operating system's content at these
/// (inclusive) ranges. GROM 0 is the monitor (boot, title, menu, KSCAN tables,
/// GPLLNK library); GROM 1/2 are TI BASIC + the shared GPL library.
pub const GROM0: Region = Region { name: "GROM 0 >0000-17FF (monitor)", start: 0x0000, end: 0x17FF };
pub const GROM1: Region = Region { name: "GROM 1 >2000-37FF (BASIC)", start: 0x2000, end: 0x37FF };
pub const GROM2: Region = Region { name: "GROM 2 >4000-57FF (BASIC + GPL library)", start: 0x4000, end: 0x57FF };

/// The three content regions, in order.
pub const CONTENT_REGIONS: [Region; 3] = [GROM0, GROM1, GROM2];

/// The three **chip gaps**: the 2 KiB above each 6 KiB chip. Real console GROMs
/// are 6 KiB parts in 8 KiB slots, so `>1800-1FFF`, `>3800-3FFF`, and
/// `>5800-5FFF` do not exist on hardware — the authentic dump carries ghost
/// bytes there. Our image must stay all-zero across them (a strict emulator or
/// real hardware would not serve them); this is B4's invariant.
pub const CHIP_GAPS: [Region; 3] = [
    Region { name: "chip gap >1800-1FFF", start: 0x1800, end: 0x1FFF },
    Region { name: "chip gap >3800-3FFF", start: 0x3800, end: 0x3FFF },
    Region { name: "chip gap >5800-5FFF", start: 0x5800, end: 0x5FFF },
];

/// Per-region byte tallies — a partition of every byte in the range into exactly
/// one bucket (the five columns sum to `total`), matching QUALITY-ASSESSMENT.md
/// §3's table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub total: usize,
    /// Both images equal and non-zero (interface data we already reproduce).
    pub identical: usize,
    /// Both images zero (unpopulated on both sides).
    pub both_zero: usize,
    /// Authentic non-zero, ours zero — the hazard set (a table/routine we omit).
    pub authentic_only: usize,
    /// Ours non-zero, authentic zero — our original content in free space.
    pub ours_only: usize,
    /// Both non-zero but different (our code/content replacing authentic's).
    pub differ: usize,
}

/// Classify every byte of `[start, end]` (inclusive) of the two images.
pub fn stats(ours: &[u8], auth: &[u8], region: Region) -> Stats {
    let mut s = Stats::default();
    for addr in region.start..=region.end {
        let o = byte(ours, addr);
        let a = byte(auth, addr);
        s.total += 1;
        match (o == 0, a == 0) {
            (true, true) => s.both_zero += 1,
            (true, false) => s.authentic_only += 1,
            (false, true) => s.ours_only += 1,
            (false, false) if o == a => s.identical += 1,
            (false, false) => s.differ += 1,
        }
    }
    s
}

/// A maximal contiguous stretch where the authentic image is non-zero and ours
/// is zero: `[start, end]` inclusive, `len` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub start: u16,
    pub end: u16,
    pub len: usize,
}

/// Every authentic-only run of length `>= min_len` within `[region.start,
/// region.end]` (inclusive), in address order. An authentic-only byte is
/// `auth != 0 && ours == 0` — content the authentic firmware provides at a fixed
/// address that our image leaves blank.
pub fn authentic_only_runs(ours: &[u8], auth: &[u8], region: Region, min_len: usize) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut run_start: Option<u16> = None;
    for addr in region.start..=region.end {
        let is_ao = byte(auth, addr) != 0 && byte(ours, addr) == 0;
        match (is_ao, run_start) {
            (true, None) => run_start = Some(addr),
            (false, Some(s)) => {
                push_run(&mut runs, s, addr - 1, min_len);
                run_start = None;
            }
            _ => {}
        }
    }
    // A run that reaches the end of the region.
    if let Some(s) = run_start {
        push_run(&mut runs, s, region.end, min_len);
    }
    runs
}

fn push_run(runs: &mut Vec<Run>, start: u16, end: u16, min_len: usize) {
    let len = end as usize - start as usize + 1;
    if len >= min_len {
        runs.push(Run { start, end, len });
    }
}

/// True iff every byte of `region` is zero in `img` — the chip-gap invariant.
pub fn all_zero(img: &[u8], region: Region) -> bool {
    (region.start..=region.end).all(|addr| byte(img, addr) == 0)
}

/// Read the byte at GROM address `addr`, treating out-of-bounds as zero.
fn byte(img: &[u8], addr: u16) -> u8 {
    img.get(addr as usize).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use super::*;

    /// The authentic console GROM, loaded at run time from the git-ignored
    /// `third-party/` directory (`None` when the media are absent — the tests
    /// below then skip with a notice).
    static AUTH: LazyLock<Option<Vec<u8>>> =
        LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

    /// The authentic GROM, or `None` (announced on stderr) when unavailable.
    fn auth() -> Option<&'static [u8]> {
        let auth = AUTH.as_deref();
        if auth.is_none() {
            eprintln!("SKIPPED: third-party media not present (third-party/roms/994AGROM.Bin)");
        }
        auth
    }

    fn ours() -> Vec<u8> {
        crate::system_grom::build_console_grom().unwrap()
    }

    #[test]
    fn stats_columns_partition_the_region() {
        let Some(auth) = auth() else { return };
        let ours = ours();
        for r in CONTENT_REGIONS {
            let s = stats(&ours, auth, r);
            assert_eq!(
                s.identical + s.both_zero + s.authentic_only + s.ours_only + s.differ,
                s.total,
                "the five buckets must partition {}",
                r.name
            );
            assert_eq!(s.total, (r.end - r.start) as usize + 1);
        }
    }

    #[test]
    fn runs_are_ordered_disjoint_and_authentic_only() {
        let Some(auth) = auth() else { return };
        let ours = ours();
        let runs = authentic_only_runs(&ours, auth, GROM0, 8);
        let mut prev_end = None;
        for run in &runs {
            assert!(run.len >= 8);
            assert_eq!(run.end as usize - run.start as usize + 1, run.len);
            if let Some(pe) = prev_end {
                assert!(run.start > pe, "runs must be ordered and disjoint");
            }
            for addr in run.start..=run.end {
                assert_ne!(auth[addr as usize], 0, "run byte must be authentic non-zero");
                assert_eq!(ours[addr as usize], 0, "run byte must be ours-zero");
            }
            prev_end = Some(run.end);
        }
    }
}
