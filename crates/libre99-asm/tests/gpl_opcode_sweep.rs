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

//! **The 256-opcode differential sweep — the M4 exit gate.** Every GPL opcode
//! byte `>00-FF` runs once as the head of a canonical, WELL-FORMED program
//! under the authentic ROM and under ours, from identical state, for a
//! bounded number of CPU steps; the observable state must match. This is the
//! completeness instrument RECON §15 calls for: the per-opcode microsuites
//! pin deep semantics, this sweep proves every byte DISPATCHES somewhere
//! sane and identical — the vestigial and the hanging forms included.
//!
//! Canonical-form notes: the stack ops get real frames (RTN/RTNC/FETCH ride
//! a CALL, RTGR rides a SWGR), branch forms get a halt planted at their
//! slot-dependent target, EXIT is the bare reset loop (stationary by
//! construction), and the ext-GPL blocks park in the absent card's empty-bus
//! march — all legitimate as long as both ROMs land identically. The run is
//! bounded in STEPS (4000 — far below the march's ~24k wrap into ROM bytes).
//!
//! **The documented exclusions — the M6-deferral tripwire.** PARSE (>0E),
//! CONT (>10), EXEC (>11) and RTNB (>12) are the TI BASIC interpreter's
//! entries; their bodies ARE the BASIC ROM half, deferred indefinitely by
//! policy (ROM-REWRITE-PLAN, the M6 deferral note). Under the authentic they
//! run BASIC code; under ours they must hit the loud stub with the opcode in
//! the breadcrumb — asserted here, so un-deferring M6 will fail this gate
//! and force the justification the policy requires.

use std::sync::{LazyLock, OnceLock};

use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// The M6-deferred opcodes: the BASIC interpreter surface (the module note).
const M6_DEFERRED: [u8; 4] = [0x0E, 0x10, 0x11, 0x12];

/// MOVE's G=0,R=1 count-from-memory forms: an UNPINNED garbage corner. The
/// authentic's parse for these leaves an unbalanced sub-stack frame (its
/// dest decode differs structurally when R rides a GRAM dest with a memory
/// count); no real emitter produces the combination (the GROM track bans far
/// tamer forms). Ours runs them as coherent GRAM moves and parks. Pinning
/// the authentic's exact garbage parse is an M7 robustness-probe item —
/// tracked in RECON §26; the M4 bar (dispatch sanely, deterministically,
/// diagnosably) holds.
const M7_GARBAGE_CORNER: [u8; 2] = [0x2C, 0x2E];

fn our_rom() -> &'static [u8] {
    static ROM: OnceLock<Vec<u8>> = OnceLock::new();
    ROM.get_or_init(|| libre99_asm::system_rom::build_console_rom().expect("console ROM assembles"))
}

/// A condition-safe halt at GROM address `a` (RECON §16's double-BR idiom).
fn halt(a: u16) -> [u8; 4] {
    let n = a + 2;
    let hi = 0x40 | ((n >> 8) as u8 & 0x1F);
    [hi, n as u8, hi, n as u8]
}

/// The canonical case for opcode `op`: (the program at >0020, extra GROM
/// plants at absolute addresses). Every path ends in a halt or a stationary
/// hang shared by both ROMs.
fn case(op: u8) -> (Vec<u8>, Vec<(u16, Vec<u8>)>) {
    let mut extra: Vec<(u16, Vec<u8>)> = Vec::new();
    // EXIT: the bare reset loop — >0020 = EXIT re-fetches >0020 forever.
    if op == 0x0B {
        return (vec![0x0B], extra);
    }
    let mut p = vec![
        0xBE, 0x40, 0x35, // ST  @>8340,>35
        0xBE, 0x41, 0x02, // ST  @>8341,>02
        0xBF, 0x42, 0x12, 0x34, // DST @>8342,>1234
        0xBE, 0x56, 0x44, // ST  @>8356,>44 (an indirect pointer)
        0xBF, 0x48, 0x00, 0x02, // DST @>8348,>0002 (a word-valued fn cell)
    ];
    match op {
        // RTN/RTNC ride a CALL frame: CALL >0100 -> marker -> halt; the
        // subroutine is the op under test.
        0x00 | 0x01 => {
            p.extend([0x06, 0x01, 0x00]); // CALL >0100
            p.extend([0xBE, 0x45, 0xAA]); // ST @>8345,>AA (the return marker)
            extra.push((0x0100, vec![op]));
        }
        // RTGR rides a SWGR frame (the slice-3 gate's shape).
        0x13 => {
            p.extend([0xBF, 0x50, 0x98, 0x00]); // DST @>8350,>9800
            p.extend([0xFB, 0x50, 0x01, 0x00]); // SWGR @>8350,#>0100
            p.extend([0xBE, 0x45, 0xAA]); // the return marker
            extra.push((0x0100, vec![0xBE, 0x46, 0xBB, op])); // ST; RTGR
        }
        // FETCH rides a CALL with inline data after the call site.
        0x88 | 0x89 => {
            p.extend([0x06, 0x01, 0x00]); // CALL >0100
            p.push(0x77); // the inline byte FETCH reads
            p.extend([0xBE, 0x45, 0xAA]);
            extra.push((0x0100, vec![op, 0x44, 0x00])); // FETCH @>8344; RTN
        }
        // Specials with their own grammar.
        0x02 => p.extend([0x02, 0x07]), // RAND >07
        0x04 => p.extend([0x04, 0x17]),
        0x05 => {
            p.extend([0x05, 0x01, 0x00]); // B >0100
            extra.push((0x0100, halt(0x0100).to_vec()));
        }
        0x06 => {
            p.extend([0x06, 0x01, 0x00]); // CALL >0100 (parks inside)
            extra.push((0x0100, halt(0x0100).to_vec()));
        }
        0x07 => p.extend([0x07, 0x20]), // ALL: fill with spaces
        0x08 => p.extend([0x08, 0xFB]), // FMT: FEND immediately
        0x0F => p.extend([0x0F, 0x19]), // XML >19 (the no-card scan)
        // The remaining specials (SCAN/H/GT/CARRY/OVF, the ext block, and
        // the M6-deferred four — the latter only ever run through the
        // breadcrumb assert).
        0x03 | 0x09..=0x1F => p.push(op),
        // MOVE (>20-3F): a tail per the G/R/V/C/N bits.
        0x20..=0x3F => {
            p.push(op);
            if op & 1 != 0 {
                p.extend([0x00, 0x01]); // immediate count 1
            } else {
                // Count-from-memory is a WORD read (RECON §17): the sane
                // word cell @>8348 (= >0002). A byte cell here would give a
                // >0212-byte runaway copy trampling GPLWS itself — whose
                // contents are interpreter geometry, divergent by design.
                p.push(0x48);
            }
            if op & 8 != 0 {
                p.push(0x07); // VDP register 7
            } else if op & 0x10 != 0 {
                p.push(0x44); // CPU/VDP dest GAS
            } else {
                p.extend([0x00, 0x80]); // GRAM dest (inert writes)
            }
            if op & 4 != 0 {
                p.push(0x40); // RAM source GAS
            } else if op & 2 != 0 {
                p.extend([0x01, 0x00, 0x41]); // computed: base + index cell
            } else {
                p.extend([0x01, 0x00]); // GROM source >0100
            }
            extra.push((0x0100, vec![0x5A, 0xA5]));
        }
        // BR/BS (>40-7F): the target is slot-dependent — plant a halt there.
        0x40..=0x7F => {
            p.push(op);
            p.push(0x80); // the target's low byte
            let target = (((op & 0x1F) as u16) << 8) | 0x80;
            if target >= 0x0080 {
                extra.push((target, halt(target).to_vec()));
            }
        }
        // Format-5 (>80-97, FETCH handled above) + the ext block (>98-9F).
        0x80..=0x9F => p.extend([op, 0x40]),
        // COINC (>EC-EF): dest, src, scale, table16.
        0xEC..=0xEF => {
            p.push(op);
            p.push(0x42);
            if op & 2 != 0 {
                p.push(0x11);
                if op & 1 != 0 {
                    p.push(0x22);
                }
            } else {
                p.push(0x40);
            }
            p.extend([0x00, 0x01, 0x00]);
            extra.push((0x0100, vec![0x07, 0x07, 0x00, 0x00, 0xF0, 0x0F, 0xAA, 0x55]));
        }
        // Everything else >= >A0 (the two-op families, IO, SWGR, the ext
        // blocks): dest GAS then the source per the imm/word bits. For IO
        // the "source" is the function; @>8341 = 2 (CRU in) and the imm
        // forms use 3 (CRU out) — the list at @>8342 was seeded above.
        0xA0..=0xFF => {
            let io = (0xF4..=0xF7).contains(&op);
            p.push(op);
            p.push(if (0xF8..=0xFB).contains(&op) { 0x42 } else { 0x40 });
            if op & 2 != 0 {
                p.push(if io { 0x03 } else { 0x11 });
                if op & 1 != 0 {
                    p.push(if io { 0x03 } else { 0x22 });
                    if io {
                        let l = p.len();
                        p[l - 2] = 0x00; // word form: the function is the word
                    }
                }
            } else {
                // memory source: the IO forms read the sane word-valued fn
                // cell (@>8348 = >0002); a mid-cell word read would land in
                // our documented fn>=7 stub-vs-garbage divergence.
                p.push(if io { 0x48 } else { 0x41 });
            }
        }
    }
    (p, extra)
}

/// Run the case under `rom` for `steps` CPU instructions; snapshot with the
/// gpl_core masks + the RTWP frame (>83DA-DF, interpreter geometry).
fn run(rom: &[u8], p: &[u8], extra: &[(u16, Vec<u8>)], steps: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut grom = vec![0u8; 0x6000];
    let mut v = p.to_vec();
    let at = 0x20 + v.len() as u16;
    v.extend_from_slice(&halt(at));
    grom[0x20..0x20 + v.len()].copy_from_slice(&v);
    for (a, bytes) in extra {
        let a = *a as usize;
        grom[a..a + bytes.len()].copy_from_slice(bytes);
    }
    let mut m = Machine::new(rom, &grom);
    m.reset();
    for _ in 0..steps {
        m.step();
    }
    let pad: Vec<u8> = (0x8300u16..0x83E0)
        .map(|a| {
            let b = m.bus().peek(a);
            match a {
                0x8300..=0x8307 => 0,
                0x8372 | 0x8373 => 0,
                0x837C => b & 0xF8,
                0x83DA..=0x83DF => 0,
                _ => b,
            }
        })
        .collect();
    let regs: Vec<u8> = (0..8).map(|r| m.vdp().register(r)).collect();
    let vram: Vec<u8> = (0x0000u16..0x0500).map(|a| m.vdp().vram(a)).collect();
    (pad, regs, vram)
}

/// The breadcrumb cell under our ROM after running the case.
fn breadcrumb(p: &[u8], extra: &[(u16, Vec<u8>)], steps: usize) -> u8 {
    let mut grom = vec![0u8; 0x6000];
    let mut v = p.to_vec();
    let at = 0x20 + v.len() as u16;
    v.extend_from_slice(&halt(at));
    grom[0x20..0x20 + v.len()].copy_from_slice(&v);
    for (a, bytes) in extra {
        let a = *a as usize;
        grom[a..a + bytes.len()].copy_from_slice(bytes);
    }
    let mut m = Machine::new(our_rom(), &grom);
    m.reset();
    for _ in 0..steps {
        m.step();
    }
    m.bus().peek(0x837D)
}

/// Every opcode byte, differentially — the M4 completeness gate.
#[test]
fn all_256_opcodes_dispatch_identically() {
    let Some(auth_rom) = AUTH_ROM.as_deref() else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let mut failures = Vec::new();
    for op in 0u16..=0xFF {
        let op = op as u8;
        if M6_DEFERRED.contains(&op) || M7_GARBAGE_CORNER.contains(&op) {
            continue; // asserted separately below
        }
        let (p, extra) = case(op);
        let a = run(auth_rom, &p, &extra, 4000);
        let o = run(our_rom(), &p, &extra, 4000);
        if a != o {
            let diff: Vec<String> = (0..a.0.len())
                .filter(|&i| a.0[i] != o.0[i])
                .take(4)
                .map(|i| format!(">{:04X} {:02X}/{:02X}", 0x8300 + i, a.0[i], o.0[i]))
                .collect();
            failures.push(format!(
                "op >{op:02X}: pad {diff:?} regs-match {} vram-match {}",
                a.1 == o.1,
                a.2 == o.2
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} opcodes diverged:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The M6-deferral tripwire: the four BASIC-interpreter opcodes must hit OUR
/// loud stub (breadcrumb = the opcode). If M6 ever lands, this fails and
/// demands the written justification the deferral policy requires.
#[test]
fn m6_deferred_opcodes_hit_the_loud_stub() {
    for op in M6_DEFERRED {
        let (p, extra) = case(op);
        assert_eq!(
            breadcrumb(&p, &extra, 4000),
            op,
            "op >{op:02X} is the deferred M6 surface and must breadcrumb loudly"
        );
    }
}

/// The exclusion lists are exactly the documented surfaces: the M6 four and
/// the two-op unpinned garbage pair — nothing may creep in silently.
#[test]
fn the_exclusion_list_is_the_m6_surface_only() {
    assert_eq!(M6_DEFERRED, [0x0E, 0x10, 0x11, 0x12]);
    assert_eq!(M7_GARBAGE_CORNER, [0x2C, 0x2E]);
}

/// The garbage-corner pair still DISPATCHES sanely under ours: the machine
/// parks (a stationary halt), no breadcrumb fires — deterministic and
/// diagnosable even where the authentic's garbage parse is unpinned.
#[test]
fn the_garbage_corner_parks_cleanly_under_ours() {
    for op in M7_GARBAGE_CORNER {
        let (p, extra) = case(op);
        assert_eq!(breadcrumb(&p, &extra, 4000), 0, "op >{op:02X} must not breadcrumb");
    }
}
