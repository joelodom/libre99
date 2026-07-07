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

//! The **census gate** (QUALITY-ASSESSMENT.md Phase A1). Three invariants keep
//! the compatibility surface bounded and the surface map honest:
//!
//! (a) **DATA-MUST-MATCH regions are byte-identical.** Interface data a cartridge
//!     may read from its documented GROM address (the fonts) must reproduce the
//!     authentic bytes exactly — generalizing the font/keytab identity gates in
//!     `font.rs`/`keymap.rs` to the authentic *home* addresses B1 ships.
//!
//! (b) **The surface map can't rot.** Every authentic-only run (authentic
//!     non-zero, ours zero) of >= 8 bytes in GROM 0 must be classified in
//!     `grom/SURFACE-MAP.md`. A new such run — the fingerprint of an omitted
//!     table or routine, the defect class behind the field bugs — that nobody
//!     has classified fails this test until it is added to the map.
//!
//! (c) **The chip gaps stay zero.** `>1800-1FFF`, `>3800-3FFF`, `>5800-5FFF` do
//!     not exist on real hardware, so our image must never place content there
//!     (B4).
//!
//! The census logic itself lives in `libre99_gpl::census`; this file supplies the
//! authentic oracle and the map, and asserts.

use std::sync::LazyLock;

use libre99_gpl::census;
use libre99_gpl::system_grom::build_console_grom;

/// The authentic console GROM — the oracle. Loaded at run time from
/// `third-party/`; the gates that need it skip when it is absent.
static AUTH: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

/// The surface map, embedded so the completeness check compiles against the
/// committed classification (and fails to build if it is deleted).
const SURFACE_MAP: &str =
    include_str!("../../../original-content/system-roms/grom/SURFACE-MAP.md");

fn ours() -> Vec<u8> {
    build_console_grom().expect("assemble console GROM")
}

/// (a) The `DATA-MUST-MATCH` regions ship byte-identical to the authentic image
/// at their authentic home addresses. These are the fonts B1 places; each is
/// classified `DATA-MUST-MATCH` in `SURFACE-MAP.md`.
#[test]
fn data_must_match_regions_are_byte_identical() {
    let Some(auth) = AUTH.as_deref() else { skip!() };
    let ours = ours();
    // Standard 8x8 character set at its authentic home >04B4 (512 bytes); the
    // same content also lives at >1000 where our title/menu read it.
    assert_region_identical(&ours, auth, 0x04B4, 0x06B3, "standard font >04B4");
    // Thin "small" character set at its authentic home >06B4 (448 bytes, 7
    // rows/glyph as stored).
    assert_region_identical(&ours, auth, 0x06B4, 0x0873, "thin font >06B4");
    // Lower-case (small capitals) set at its authentic home >0874 (217 bytes,
    // 7 rows/glyph as stored, 31 glyphs >60..>7E) — the >004A loader's data.
    // Also guards the splice layout: our menu data block starts above at >0950,
    // and an overlap would silently clobber these bytes.
    assert_region_identical(&ours, auth, 0x0874, 0x094C, "lower-case font >0874");
}

fn assert_region_identical(ours: &[u8], auth: &[u8], start: usize, end: usize, what: &str) {
    assert_eq!(
        &ours[start..=end],
        &auth[start..=end],
        "{what} must be byte-identical to the authentic image (DATA-MUST-MATCH)"
    );
}

/// (b) Every GROM-0 authentic-only run >= 8 bytes is covered by a range listed
/// in `SURFACE-MAP.md`. If this fails, the census surfaced a stretch of
/// authentic content we ship as zeros that nobody has classified — add it to
/// the map (with a disposition) to make the omission a deliberate decision.
#[test]
fn surface_map_covers_every_authentic_only_run() {
    let Some(auth) = AUTH.as_deref() else { skip!() };
    let ours = ours();
    let ranges = parse_ranges(SURFACE_MAP);
    assert!(
        ranges.len() >= 50,
        "expected the surface map to enumerate the GROM-0 runs, found {} ranges — is the map populated?",
        ranges.len()
    );
    let runs = census::authentic_only_runs(&ours, auth, census::GROM0, 8);
    let mut uncovered = Vec::new();
    for run in &runs {
        let covered = ranges
            .iter()
            .any(|&(a, b)| a <= run.start && run.end <= b);
        if !covered {
            uncovered.push(format!(">{:04X}..>{:04X} ({})", run.start, run.end, run.len));
        }
    }
    assert!(
        uncovered.is_empty(),
        "these GROM-0 authentic-only runs are not classified in SURFACE-MAP.md:\n  {}",
        uncovered.join("\n  ")
    );
}

/// (c) The three chip gaps are all-zero in our image (B4). Content here would
/// break on real hardware or a strict emulator, which do not serve the ghost
/// 2 KiB above each 6 KiB GROM chip.
#[test]
fn chip_gaps_are_zero_in_our_image() {
    let ours = ours();
    for gap in census::CHIP_GAPS {
        assert!(
            census::all_zero(&ours, gap),
            "our image must be all-zero across {} (real hardware has no bytes there)",
            gap.name
        );
    }
}

/// Sanity: the map uses the four classification labels and documents the font
/// homes B1 ships, so a mangled or truncated map is caught rather than silently
/// passing the coverage check.
#[test]
fn surface_map_is_well_formed() {
    let Some(auth) = AUTH.as_deref() else { skip!() };
    for label in ["DATA-MUST-MATCH", "CODE-REPLACED", "SERVICE-ENTRY", "DEAD"] {
        assert!(SURFACE_MAP.contains(label), "SURFACE-MAP.md should use the {label} classification");
    }
    // The census-stat cross-check pins the map to the measured hazard set.
    let ours = ours();
    let s = census::stats(&ours, auth, census::GROM0);
    assert!(s.authentic_only > 0, "GROM 0 still has authentic-only content to classify");
}

/// Extract every `>hhhh..>hhhh` range (1–4 hex digits per address, two dots)
/// from the map text, as inclusive `(start, end)` address pairs.
///
/// Each address is read greedily up to four hex digits, so the map's canonical
/// four-digit forms parse exactly as before while a shorter hand-written form
/// (`>4B4`) now parses instead of being silently dropped. Once a range opener
/// (`>hhhh..>`) is matched, a non-hex closing address is a **corrupt map entry**
/// and panics loudly: the census gate must never quietly skip a range it
/// half-recognised, because a dropped range can leave an authentic-only run
/// looking uncovered — or hide one that should have been flagged.
fn parse_ranges(text: &str) -> Vec<(u16, u16)> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Look for the range shape ">hhhh..>hhhh".
        if bytes[i] == b'>' {
            if let Some((start, j)) = hex_run(bytes, i + 1) {
                if bytes.get(j) == Some(&b'.')
                    && bytes.get(j + 1) == Some(&b'.')
                    && bytes.get(j + 2) == Some(&b'>')
                {
                    // Committed to a range opener; the closing address must be hex.
                    let (end, k) = hex_run(bytes, j + 3).unwrap_or_else(|| {
                        panic!(
                            "malformed range in SURFACE-MAP.md near `{}`: `>hhhh..>` \
                             not followed by hex digits",
                            snippet(text, i)
                        )
                    });
                    if start <= end {
                        out.push((start, end));
                    }
                    i = k;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Read 1–4 hex digits at `pos`, returning their value and the index just past
/// them, or `None` if the byte at `pos` is not a hex digit.
fn hex_run(bytes: &[u8], pos: usize) -> Option<(u16, usize)> {
    let mut v: u16 = 0;
    let mut end = pos;
    while end < bytes.len() && end - pos < 4 {
        match (bytes[end] as char).to_digit(16) {
            Some(d) => {
                v = (v << 4) | d as u16;
                end += 1;
            }
            None => break,
        }
    }
    if end == pos { None } else { Some((v, end)) }
}

/// A short, single-line text snippet starting at byte offset `at`, for panic
/// messages (best-effort; returns "" if `at` is not on a char boundary).
fn snippet(text: &str, at: usize) -> String {
    let stop = text.len().min(at + 16);
    text.get(at..stop).unwrap_or("").replace('\n', " ")
}
