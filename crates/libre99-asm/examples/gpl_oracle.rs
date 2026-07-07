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

//! **The GPL oracle probe** (M1) — runs small hand-encoded GPL programs under
//! the AUTHENTIC console ROM and prints the machine state they leave, to pin
//! the interpreter semantics our rewrite must reproduce (clean-room: behavior
//! in → spec → original code; plan P5). Focus: the `>837C` status byte after
//! each operation class, stack-pointer conventions, and edge cases (RECON §3's
//! ❓ items).
//!
//! Programs run from GROM `>0020` with all-zero scratchpad; they end in a
//! self-loop (`BR $`). No ISR runs (the 9901 stays masked), so results are
//! deterministic. Run: `cargo run -p libre99-asm --example gpl_oracle`.

use std::sync::LazyLock;

use libre99_core::machine::Machine;

/// The authentic console ROM, loaded at run time; the probe exits with a
/// pointer message when the third-party media is absent.
static AUTH_ROM: LazyLock<Vec<u8>> = LazyLock::new(|| {
    libre99_core::third_party::load("roms/994aROM.Bin").unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/roms/994aROM.Bin)");
        std::process::exit(2);
    })
});

/// Run `prog` (placed at GROM >0020) for `frames`; return the machine.
fn run(prog: &[u8], frames: usize) -> Machine {
    let mut grom = vec![0u8; 0x6000];
    grom[0x20..0x20 + prog.len()].copy_from_slice(prog);
    let mut m = Machine::new(&AUTH_ROM, &grom);
    m.reset();
    for _ in 0..frames {
        m.run_frame();
    }
    m
}

/// A self-loop at GROM address `a` (13-bit slot-absolute BR to itself).
fn spin(a: u16) -> [u8; 2] {
    [0x40 | ((a >> 8) as u8 & 0x1F), a as u8]
}

fn show(name: &str, prog: &[u8], cells: &[u16]) {
    let m = run(prog, 12);
    print!("{name:38}");
    print!("  ST={:02X}", m.bus().peek(0x837C));
    for &c in cells {
        print!("  [{c:04X}]={:02X}", m.bus().peek(c));
    }
    println!();
}

fn main() {
    // Fail fast (with the pointer message) when the third-party media is absent.
    LazyLock::force(&AUTH_ROM);

    // Common prologue: none needed — scratchpad is zero. Programs store
    // results in >8340+ and spin.
    let end = |mut v: Vec<u8>| {
        let addr = 0x20 + v.len() as u16;
        v.extend_from_slice(&spin(addr));
        v
    };

    println!("== status byte >837C after arithmetic (DST >8340 := x; op) ==");
    // DADD >1111 + >2222 (no carry, no ovf, result != 0, positive)
    show("DADD 1111+2222", &end(vec![0xBF, 0x40, 0x11, 0x11, 0xA3, 0x40, 0x22, 0x22]), &[0x8340, 0x8341]);
    // DADD 8000+8000 (carry + ovf, result 0)
    show("DADD 8000+8000", &end(vec![0xBF, 0x40, 0x80, 0x00, 0xA3, 0x40, 0x80, 0x00]), &[0x8340, 0x8341]);
    // DADD FFFF+0001 (carry, result 0)
    show("DADD FFFF+0001", &end(vec![0xBF, 0x40, 0xFF, 0xFF, 0xA3, 0x40, 0x00, 0x01]), &[0x8340, 0x8341]);
    // DADD 7FFF+0001 (ovf, negative result)
    show("DADD 7FFF+0001", &end(vec![0xBF, 0x40, 0x7F, 0xFF, 0xA3, 0x40, 0x00, 0x01]), &[0x8340, 0x8341]);
    // byte ADD: 40+40 (result 80: ovf byte-wise?)
    show("ADD 40+40 (byte)", &end(vec![0xBE, 0x40, 0x40, 0xA2, 0x40, 0x40]), &[0x8340]);
    // byte ADD: FF+01 (carry, zero)
    show("ADD FF+01 (byte)", &end(vec![0xBE, 0x40, 0xFF, 0xA2, 0x40, 0x01]), &[0x8340]);
    // DSUB 2222-1111
    show("DSUB 2222-1111", &end(vec![0xBF, 0x40, 0x22, 0x22, 0xA7, 0x40, 0x11, 0x11]), &[0x8340, 0x8341]);
    // DSUB 1111-2222 (borrow)
    show("DSUB 1111-2222", &end(vec![0xBF, 0x40, 0x11, 0x11, 0xA7, 0x40, 0x22, 0x22]), &[0x8340, 0x8341]);

    println!("== status after logic (AND/OR/XOR) — do C/OV clear? ==");
    // Set carry+ovf first via DADD 8000+8000, then DAND.
    show(
        "DADD carry, then DAND F0F0&FF00",
        &end(vec![
            0xBF, 0x42, 0x80, 0x00, 0xA3, 0x42, 0x80, 0x00, // leave C/OV set
            0xBF, 0x40, 0xF0, 0xF0, 0xB3, 0x40, 0xFF, 0x00, // DAND
        ]),
        &[0x8340, 0x8341],
    );
    show("DOR 0|0 (zero result)", &end(vec![0xB7, 0x40, 0x00, 0x00]), &[0x8340, 0x8341]);
    show("DXOR AAAA^AAAA", &end(vec![0xBF, 0x40, 0xAA, 0xAA, 0xBB, 0x40, 0xAA, 0xAA]), &[0x8340, 0x8341]);

    println!("== ST / unary: status untouched? (set C/OV first, then op) ==");
    show(
        "carry, then DST >5A5A",
        &end(vec![0xBF, 0x42, 0x80, 0x00, 0xA3, 0x42, 0x80, 0x00, 0xBF, 0x40, 0x5A, 0x5A]),
        &[0x8340, 0x8341],
    );
    show(
        "carry, then INC",
        &end(vec![0xBF, 0x42, 0x80, 0x00, 0xA3, 0x42, 0x80, 0x00, 0x90, 0x40]),
        &[0x8340],
    );
    show("DINC FFFF", &end(vec![0xBF, 0x40, 0xFF, 0xFF, 0x91, 0x40]), &[0x8340, 0x8341]);
    show("DECT 0001", &end(vec![0xBF, 0x40, 0x00, 0x01, 0x97, 0x40]), &[0x8340, 0x8341]);
    show("ABS 0080 (byte -128)", &end(vec![0xBE, 0x40, 0x80, 0x80, 0x40]), &[0x8340]);
    show("DNEG 8000", &end(vec![0xBF, 0x40, 0x80, 0x00, 0x83, 0x40]), &[0x8340, 0x8341]);

    println!("== compares: cond bit ==");
    show("CEQ 5A==5A", &end(vec![0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5A]), &[0x8340]);
    show("CEQ 5A==5B", &end(vec![0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5B]), &[0x8340]);
    show("CH  80>7F (logical)", &end(vec![0xBE, 0x40, 0x80, 0xC6, 0x40, 0x7F]), &[0x8340]);
    show("CGT 80>7F (arith: neg<pos)", &end(vec![0xBE, 0x40, 0x80, 0xCE, 0x40, 0x7F]), &[0x8340]);
    show("CHE 7F>=7F", &end(vec![0xBE, 0x40, 0x7F, 0xCA, 0x40, 0x7F]), &[0x8340]);
    show("CGE 7F>=80 (arith)", &end(vec![0xBE, 0x40, 0x7F, 0xD2, 0x40, 0x80]), &[0x8340]);
    show("CLOG F0&0F (==0 -> set)", &end(vec![0xBE, 0x40, 0xF0, 0xDA, 0x40, 0x0F]), &[0x8340]);
    show("CZ 00", &end(vec![0x8E, 0x40]), &[0x8340]);
    show("CZ 5A", &end(vec![0xBE, 0x40, 0x5A, 0x8E, 0x40]), &[0x8340]);

    println!("== branches: does BR/BS reset cond? (CEQ-set, BS taken, then BS again) ==");
    // prog at >0020: CEQ(set) ; BS >0030 ; (skipped: ST >8350,=01; spin)
    // at >0030: BS >0040 (taken only if cond still set) ; ST >8351,=02; spin
    // at >0040: ST >8352,=03; spin
    let mut p = vec![0u8; 0x30];
    let seq: &[u8] = &[0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5A, 0x60, 0x30]; // CEQ set; BS >0030
    p[..seq.len()].copy_from_slice(seq);
    p[seq.len()..seq.len() + 5].copy_from_slice(&[0xBE, 0x50, 0x01, 0x40, 0x20]); // marker + spin(>0020-ish)
    // at offset >0030->0020 = 0x10 in prog:
    p[0x10..0x17].copy_from_slice(&[0x60, 0x40, 0xBE, 0x51, 0x02, 0x40, 0x37]); // BS >0040 ; ST >8351,02 ; spin
    // at offset >0040-> = 0x20:
    p[0x20..0x27].copy_from_slice(&[0xBE, 0x52, 0x03, 0x40, 0x45, 0x00, 0x00]); // ST >8352,03 ; spin
    show("BS chain (>8350/51/52 markers)", &p, &[0x8350, 0x8351, 0x8352]);

    println!("== CALL/RTN/FETCH/PUSH/stack ==");
    // init substack ptr like real GPL, CALL >0040, at target: markers; RTN.
    let mut p = vec![0u8; 0x40];
    let seq: &[u8] = &[
        0xBE, 0x73, 0x7E, // ST @>8373,>7E
        0x06, 0x00, 0x40, // CALL >0040
        0xBE, 0x50, 0x01, // ST @>8350,>01 (after return)
    ];
    p[..seq.len()].copy_from_slice(seq);
    p[seq.len()..seq.len() + 2].copy_from_slice(&spin(0x20 + seq.len() as u16));
    p[0x20..0x24].copy_from_slice(&[0xBE, 0x51, 0x02, 0x00]); // at >0040: ST >8351,02 ; RTN
    show("CALL/RTN (markers + ptr >8373)", &p, &[0x8350, 0x8351, 0x8373, 0x8380, 0x8381]);

    // FETCH: CALL >0040 with inline data byte after CALL; target FETCHes it.
    let mut p = vec![0u8; 0x40];
    let seq: &[u8] = &[
        0xBE, 0x73, 0x7E, // ST @>8373,>7E
        0x06, 0x00, 0x40, // CALL >0040
        0xC3,             // inline data byte >C3 (after the CALL)
        0xBE, 0x50, 0x01, // (resume lands here after RTN? depends on FETCH bump)
    ];
    p[..seq.len()].copy_from_slice(seq);
    p[seq.len()..seq.len() + 2].copy_from_slice(&spin(0x20 + seq.len() as u16));
    p[0x20..0x25].copy_from_slice(&[0x88, 0x54, 0x00, 0x00, 0x00]); // FETCH @>8354 ; RTN ; pad
    show("FETCH inline byte -> >8354", &p, &[0x8354, 0x8350, 0x8373]);

    // PUSH: ptr conventions on the data stack >8372.
    show(
        "PUSH >5A (ptr >8372, cells >8360s?)",
        &end(vec![0xBE, 0x72, 0x60, 0xBE, 0x40, 0x5A, 0x8C, 0x40]),
        &[0x8372, 0x8360, 0x8361, 0x8362],
    );

    println!("== MUL/DIV edges ==");
    show("MUL byte 7*6", &end(vec![0xBE, 0x40, 0x07, 0xAA, 0x40, 0x06]), &[0x8340, 0x8341]);
    show("DDIV 47/0 (div by zero!)", &end(vec![0xBF, 0x40, 0x00, 0x00, 0xBF, 0x42, 0x00, 0x2F, 0xAF, 0x40, 0x00, 0x00]), &[0x8340, 0x8341, 0x8342, 0x8343]);
    show("DIV byte 2F/05", &end(vec![0xBF, 0x40, 0x00, 0x2F, 0xAC, 0x40, 0x05]), &[0x8340, 0x8341]);

    println!("== shifts ==");
    show("DSRA 8010,1", &end(vec![0xBF, 0x40, 0x80, 0x10, 0xDF, 0x40, 0x00, 0x01]), &[0x8340, 0x8341]);
    show("DSLL 0101,4", &end(vec![0xBF, 0x40, 0x01, 0x01, 0xE3, 0x40, 0x00, 0x04]), &[0x8340, 0x8341]);
    show("SRL byte 81,1", &end(vec![0xBE, 0x40, 0x81, 0xE6, 0x40, 0x01]), &[0x8340]);
    show("DSRC 8001,1", &end(vec![0xBF, 0x40, 0x80, 0x01, 0xEB, 0x40, 0x00, 0x01]), &[0x8340, 0x8341]);
    show("DSLL 0101,0 (count 0?)", &end(vec![0xBF, 0x40, 0x01, 0x01, 0xE3, 0x40, 0x00, 0x00]), &[0x8340, 0x8341]);
    show("DSLL 0101,16", &end(vec![0xBF, 0x40, 0x01, 0x01, 0xE3, 0x40, 0x00, 0x10]), &[0x8340, 0x8341]);

    // ---- ROUND 2: capture >837C into >835E *before* the spin loop (the spin
    // is `BR $`, and BR resets the condition bit every pass — round 1's ST
    // column showed post-BR state). `cap` = ST @>835E,@>837C.
    let cap = |mut v: Vec<u8>| {
        v.extend_from_slice(&[0xBC, 0x5E, 0x7C]);
        end(v)
    };
    println!("== round 2: captured status (>835E) ==");
    show("cap: CEQ 5A==5A", &cap(vec![0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5A]), &[0x835E]);
    show("cap: CEQ 5A==5B", &cap(vec![0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5B]), &[0x835E]);
    // does a compare preserve C/OV? set carry first, then CEQ-false.
    show(
        "cap: carry, then CEQ false",
        &cap(vec![0xBF, 0x42, 0xFF, 0xFF, 0xA3, 0x42, 0x00, 0x01, 0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5B]),
        &[0x835E],
    );
    show("cap: CH 80>7F", &cap(vec![0xBE, 0x40, 0x80, 0xC6, 0x40, 0x7F]), &[0x835E]);
    show("cap: CH 7F>80", &cap(vec![0xBE, 0x40, 0x7F, 0xC6, 0x40, 0x80]), &[0x835E]);
    show("cap: CGT 80>7F (neg vs pos)", &cap(vec![0xBE, 0x40, 0x80, 0xCE, 0x40, 0x7F]), &[0x835E]);
    show("cap: CGT 7F>80", &cap(vec![0xBE, 0x40, 0x7F, 0xCE, 0x40, 0x80]), &[0x835E]);
    show("cap: CHE 7F>=7F", &cap(vec![0xBE, 0x40, 0x7F, 0xCA, 0x40, 0x7F]), &[0x835E]);
    show("cap: CHE 7E>=7F", &cap(vec![0xBE, 0x40, 0x7E, 0xCA, 0x40, 0x7F]), &[0x835E]);
    show("cap: CGE 7F>=80", &cap(vec![0xBE, 0x40, 0x7F, 0xD2, 0x40, 0x80]), &[0x835E]);
    show("cap: CGE 80>=7F", &cap(vec![0xBE, 0x40, 0x80, 0xD2, 0x40, 0x7F]), &[0x835E]);
    show("cap: CLOG F0&0F", &cap(vec![0xBE, 0x40, 0xF0, 0xDA, 0x40, 0x0F]), &[0x835E]);
    show("cap: CLOG F0&30", &cap(vec![0xBE, 0x40, 0xF0, 0xDA, 0x40, 0x30]), &[0x835E]);
    show("cap: CZ 00", &cap(vec![0x8E, 0x40]), &[0x835E]);
    show("cap: DCZ 0000", &cap(vec![0x8F, 0x40]), &[0x835E]);
    show("cap: byte AND F0&33 (odd result 30)", &cap(vec![0xBE, 0x40, 0xF0, 0xB2, 0x40, 0x33]), &[0x835E, 0x8340]);
    show("cap: byte OR 01|02 (result 03 even)", &cap(vec![0xBE, 0x40, 0x01, 0xB6, 0x40, 0x02]), &[0x835E, 0x8340]);
    show("cap: word arith DADD 0001+0001", &cap(vec![0xBF, 0x40, 0x00, 0x01, 0xA3, 0x40, 0x00, 0x01]), &[0x835E, 0x8340, 0x8341]);
    // EX with the REAL encoding (C1 = word mem-source EX).
    show(
        "cap: DEX 1122<->3344",
        &cap(vec![0xBF, 0x40, 0x11, 0x22, 0xBF, 0x42, 0x33, 0x44, 0xC1, 0x40, 0x42]),
        &[0x8340, 0x8341, 0x8342, 0x8343, 0x835E],
    );
    // DIV with correct imm encodings: success + statuses.
    show(
        "cap: DIV byte 2F/05 (AE imm)",
        &cap(vec![0xBF, 0x40, 0x00, 0x2F, 0xAE, 0x40, 0x05]),
        &[0x8340, 0x8341, 0x835E],
    );
    show(
        "cap: DIV byte 1000/2 (q>FF?)",
        &cap(vec![0xBF, 0x40, 0x10, 0x00, 0xAE, 0x40, 0x02]),
        &[0x8340, 0x8341, 0x835E],
    );
    show(
        "cap: DDIV 00010000/2 (hi>=div ovf?)",
        &cap(vec![0xBF, 0x40, 0x00, 0x01, 0xBF, 0x42, 0x00, 0x00, 0xAF, 0x40, 0x00, 0x02]),
        &[0x8340, 0x8341, 0x8342, 0x8343, 0x835E],
    );
    show(
        "cap: DDIV 00002F00/0100",
        &cap(vec![0xBF, 0x40, 0x00, 0x00, 0xBF, 0x42, 0x2F, 0x00, 0xAF, 0x40, 0x01, 0x00]),
        &[0x8340, 0x8341, 0x8342, 0x8343, 0x835E],
    );
    show(
        "cap: DMUL 1234*0100",
        &cap(vec![0xBF, 0x40, 0x12, 0x34, 0xAB, 0x40, 0x01, 0x00]),
        &[0x8340, 0x8341, 0x8342, 0x8343, 0x835E],
    );
    // Byte shifts, count edge cases.
    show("cap: SLL byte 01,0 (count0=?)", &cap(vec![0xBE, 0x40, 0x01, 0xE2, 0x40, 0x00]), &[0x8340, 0x835E]);
    show("cap: SRC byte 81,1", &cap(vec![0xBE, 0x40, 0x81, 0xEA, 0x40, 0x01]), &[0x8340, 0x835E]);
    show("cap: SRA byte 81,1", &cap(vec![0xBE, 0x40, 0x81, 0xDE, 0x40, 0x01]), &[0x8340, 0x835E]);
    // PUSH word form (8D).
    show(
        "cap: DPUSH >1122 (ptr >8372)",
        &cap(vec![0xBE, 0x72, 0x60, 0xBF, 0x40, 0x11, 0x22, 0x8D, 0x40]),
        &[0x8372, 0x8361, 0x8362, 0x8363],
    );
    // RTNC preserves cond; RTN clears it (markers around a CALL).
    let mut p = vec![0u8; 0x40];
    let seq: &[u8] = &[
        0xBE, 0x73, 0x7E, // ST @>8373,>7E
        0xBE, 0x40, 0x5A, 0xD6, 0x40, 0x5A, // CEQ true (cond set)
        0x06, 0x00, 0x40, // CALL >0040 (resets cond!)
        0xBC, 0x5E, 0x7C, // capture status after return
    ];
    p[..seq.len()].copy_from_slice(seq);
    p[seq.len()..seq.len() + 2].copy_from_slice(&spin(0x20 + seq.len() as u16));
    p[0x20..0x2A].copy_from_slice(&[
        0xBE, 0x41, 0x5A, 0xD6, 0x41, 0x5A, // CEQ true inside sub (cond set)
        0x01, 0x00, 0x00, 0x00, // RTNC (should preserve cond) ; pad
    ]);
    show("cap: RTNC preserves cond", &p, &[0x835E]);
    let mut p2 = p.clone();
    p2[0x26] = 0x00; // RTN instead of RTNC
    show("cap: RTN clears cond", &p2, &[0x835E]);
    // RAND with a known seed: seed=0 -> seed'=>7AB9; swpb=>B97A; %(FF+1)=>7A.
    show("cap: RAND FF (seed 0)", &cap(vec![0x02, 0xFF]), &[0x8378, 0x83C0, 0x83C1]);
    show("cap: RAND 07 (seed 0)", &cap(vec![0x02, 0x07]), &[0x8378, 0x83C0, 0x83C1]);

    println!("== EX / CASE / H-GT-CARRY-OVF ==");
    show("DEX 1122<->3344", &end(vec![0xBF, 0x40, 0x11, 0x22, 0xBF, 0x42, 0x33, 0x44, 0xA1 | 0x04, 0x40, 0x42]), &[0x8340, 0x8341, 0x8342, 0x8343]);
    // CASE 1: skips one 2-byte BR: prog: ST >8340,=1; CASE @>8340; BR a; BR b; b: marker
    let mut p = vec![0u8; 0x30];
    let seq: &[u8] = &[
        0xBE, 0x40, 0x01, // ST @>8340,>01
        0x8A, 0x40, // CASE @>8340
        0x40, 0x2D, // entry0: BR >002D
        0x40, 0x29, // entry1: BR >0029
    ];
    p[..seq.len()].copy_from_slice(seq);
    p[0x09..0x0e].copy_from_slice(&[0xBE, 0x50, 0x0B, 0x40, 0x2B]); // >0029: ST >8350,>0B ; spin
    p[0x0d..0x12].copy_from_slice(&[0xBE, 0x50, 0x0A, 0x40, 0x2F]); // >002D: ST >8350,>0A ; spin
    show("CASE value 1 -> entry1", &p, &[0x8350]);
    // CARRY after DADD FFFF+1: then CARRY op sets cond; check via markers.
    show(
        "CARRY op after carry-add",
        &end(vec![0xBF, 0x40, 0xFF, 0xFF, 0xA3, 0x40, 0x00, 0x01, 0x0C]),
        &[0x8340],
    );
    show("H op after CH-true", &end(vec![0xBE, 0x40, 0x80, 0xC6, 0x40, 0x7F, 0x09]), &[0x8340]);

    println!("== ALL (>07): which VRAM cells fill, and does it follow VDP reg2? ==");
    // Bare ALL >2A: report the filled range in VRAM.
    {
        let m = run(&end(vec![0x07, 0x2A]), 12);
        let (mut first, mut last, mut count) = (None, None, 0u32);
        for a in 0u16..0x1000 {
            if m.vdp().vram(a) == 0x2A {
                first.get_or_insert(a);
                last = Some(a);
                count += 1;
            }
        }
        println!("  bare ALL >2A: {count} cells; first={first:04X?} last={last:04X?}");
    }
    // Set VDP reg2 := >01 (would move a reg2-based name table to >0400), then
    // ALL >2A — if the fill stays at >0000 the base is hardcoded, not reg2.
    {
        // MOVE >0001,#2,G>0100 (reg-dest form), then ALL >2A ; spin.
        let mut grom = vec![0u8; 0x6000];
        let prog = end(vec![0x39, 0x00, 0x01, 0x02, 0x01, 0x00, 0x07, 0x2A]);
        grom[0x20..0x20 + prog.len()].copy_from_slice(&prog);
        grom[0x0100] = 0x01; // reg2 value
        let mut m = Machine::new(&AUTH_ROM, &grom);
        m.reset();
        for _ in 0..12 {
            m.run_frame();
        }
        let at0 = m.vdp().vram(0x0000);
        let at400 = m.vdp().vram(0x0400);
        println!("  reg2:=1 then ALL >2A: [>0000]={at0:02X} [>0400]={at400:02X} (>2A at >0000 => hardcoded base)");
    }
}
