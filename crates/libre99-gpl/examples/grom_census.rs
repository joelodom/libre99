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

//! `grom_census` — the byte census of the rewritten console GROM against the
//! authentic image (QUALITY-ASSESSMENT.md §3, Phase A1). Prints the per-region
//! tally table and enumerates every **authentic-only run** (authentic non-zero,
//! ours zero) of at least 8 bytes — the work list `grom/SURFACE-MAP.md`
//! classifies. The census logic lives in [`libre99_gpl::census`] so the gate
//! (`tests/census.rs`) measures with the same definitions.
//!
//! ```text
//!   cargo run -p libre99-gpl --example grom_census            # stats + GROM-0 runs
//!   cargo run -p libre99-gpl --example grom_census -- all     # also GROM 1/2 runs
//! ```
//!
//! To identify a run, disassemble the authentic bytes at its address:
//!   `cargo run -p libre99-gpl --bin libre99gpl -- dis third-party/roms/994AGROM.Bin >04B4`.

use std::sync::LazyLock;

use libre99_gpl::census::{self, Region};
use libre99_gpl::system_grom::build_console_grom;

/// The authentic console GROM — the oracle. Loaded at run time from the
/// git-ignored `third-party/` directory (see `libre99_core::third_party`), never
/// embedded in this tool or the shipped firmware image.
static AUTH: LazyLock<Vec<u8>> = LazyLock::new(|| {
    libre99_core::third_party::load("roms/994AGROM.Bin").unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/roms/994AGROM.Bin)");
        std::process::exit(2)
    })
});

fn main() {
    let all = std::env::args().nth(1).as_deref() == Some("all");
    let ours = build_console_grom().expect("assemble console GROM");

    println!("Byte census — authentic 994AGROM.Bin vs our console-grom.bin\n");
    println!(
        "{:<40} {:>6} {:>9} {:>9} {:>14} {:>9} {:>6}",
        "Region", "bytes", "identical", "both zero", "authentic-only", "ours-only", "differ"
    );
    for r in census::CONTENT_REGIONS {
        let s = census::stats(&ours, &AUTH, r);
        println!(
            "{:<40} {:>6} {:>9} {:>9} {:>14} {:>9} {:>6}",
            r.name, s.total, s.identical, s.both_zero, s.authentic_only, s.ours_only, s.differ
        );
    }
    // Chip gaps as one combined row (they must be all-zero in ours — B4).
    let mut gap = census::Stats::default();
    let mut gaps_clean = true;
    for r in census::CHIP_GAPS {
        let s = census::stats(&ours, &AUTH, r);
        gap.total += s.total;
        gap.identical += s.identical;
        gap.both_zero += s.both_zero;
        gap.authentic_only += s.authentic_only;
        gap.ours_only += s.ours_only;
        gap.differ += s.differ;
        gaps_clean &= census::all_zero(&ours, r);
    }
    println!(
        "{:<40} {:>6} {:>9} {:>9} {:>14} {:>9} {:>6}",
        "chip gaps >1800/>3800/>5800 (+2 KiB)",
        gap.total,
        gap.identical,
        gap.both_zero,
        gap.authentic_only,
        gap.ours_only,
        gap.differ
    );
    println!(
        "\nchip gaps all-zero in ours (B4 invariant): {}",
        if gaps_clean { "yes" } else { "NO — content leaked into a ghost region" }
    );

    print_runs("GROM 0", census::GROM0, &ours);
    if all {
        print_runs("GROM 1", census::GROM1, &ours);
        print_runs("GROM 2", census::GROM2, &ours);
    } else {
        println!("\n(pass `all` to also list GROM 1/2 runs)");
    }
}

fn print_runs(label: &str, region: Region, ours: &[u8]) {
    let runs = census::authentic_only_runs(ours, &AUTH, region, 8);
    let bytes: usize = runs.iter().map(|r| r.len).sum();
    println!(
        "\n{label} authentic-only runs >= 8 bytes: {} runs, {bytes} bytes",
        runs.len()
    );
    for (i, r) in runs.iter().enumerate() {
        print!(">{:04X}..>{:04X} ({:<3})", r.start, r.end, r.len);
        if i % 3 == 2 {
            println!();
        } else {
            print!("   ");
        }
    }
    if !runs.len().is_multiple_of(3) {
        println!();
    }
}
