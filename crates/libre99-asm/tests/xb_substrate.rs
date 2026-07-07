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

//! **The XB-substrate differential gates** — the five BASIC-era console-ROM
//! helpers Extended BASIC calls directly by address (the F0 census,
//! `original-content/system-roms/XB-CENSUS.md`): SYMSRC `>15E0`, RDCELL
//! `>187C`/`>1880`, RDVAL8 `>1890`, WRWORD `>18AA`/`>18AE`, STKON/STKOFF
//! `>1E7A`/`>1E8C`, VPOPAG `>1FA8`.
//!
//! Each test plants identical machine state under the authentic ROM and ours,
//! runs a tiny hand-assembled driver that `BL`s the helper, and requires the
//! observable end state to match: the scratchpad (the FP suite's masks), the
//! working registers R0–R11, the final PC (which park loop was reached —
//! entry-protocol evidence), and the planted VRAM window. Semantic asserts on
//! our ROM alone document the contract even when the authentic image is
//! absent (the differential legs skip green without third-party media).

use std::sync::{LazyLock, OnceLock};

use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

fn our_rom() -> &'static [u8] {
    static ROM: OnceLock<Vec<u8>> = OnceLock::new();
    ROM.get_or_init(|| libre99_asm::system_rom::build_console_rom().expect("console ROM assembles"))
}

/// A condition-safe GPL halt at GROM address `a` (RECON §16's double-BR idiom)
/// — the boot parks here so the test owns the machine.
fn halt(a: u16) -> [u8; 4] {
    let n = a + 2;
    let hi = 0x40 | ((n >> 8) as u8 & 0x1F);
    [hi, n as u8, hi, n as u8]
}

/// Boot `rom` into a parked GPL halt, then plant deterministic register file +
/// the caller's state, point the CPU at a machine-code driver at `>2100`
/// (interrupt mask 0), run it to its park loop, and snapshot.
struct Run {
    m: Machine,
}

impl Run {
    fn new(rom: &[u8]) -> Run {
        let mut grom = vec![0u8; 0x6000];
        let h = halt(0x20);
        grom[0x20..0x24].copy_from_slice(&h);
        let mut m = Machine::new(rom, &grom);
        m.reset();
        for _ in 0..3 {
            m.run_frame();
        }
        // Deterministic working registers (R0-R11 are scratchpad words at
        // GPLWS >83E0+); the kernel-seeded port images R13-R15 stay.
        for r in 0..12u16 {
            m.bus_mut().poke_word(0x83E0 + 2 * r, 0x1110 + r);
        }
        Run { m }
    }

    fn plant_driver(&mut self, words: &[u16]) {
        for (i, &w) in words.iter().enumerate() {
            self.m.bus_mut().poke_word(0x2100 + 2 * i as u16, w);
        }
        // A separate park for miss/error continuations.
        self.m.bus_mut().poke_word(0x2200, 0x10FF); // JMP $
    }

    fn go(&mut self) {
        self.m.cpu_mut().set_wp(0x83E0);
        self.m.cpu_mut().set_st(0); // interrupt mask 0: nothing preempts
        self.m.cpu_mut().set_pc(0x2100);
        self.m.run_frame();
    }

    /// The compared observable: masked scratchpad, R0-R11, PC, VRAM window.
    fn snapshot(&self) -> (Vec<u8>, Vec<u16>, u16, Vec<u8>) {
        let pad: Vec<u8> = (0x8300u16..0x83E0)
            .map(|a| {
                let b = self.m.bus().peek(a);
                match a {
                    0x8300..=0x8307 => 0,
                    0x8372 | 0x8373 => b, // the stack bracket's own contract cell
                    0x837C => b & 0xF8,
                    0x83DA..=0x83DF => 0,
                    _ => b,
                }
            })
            .collect();
        let regs: Vec<u16> = (0..12).map(|r| self.m.reg(r)).collect();
        let vram: Vec<u8> = (0x1800u16..0x1900).map(|a| self.m.vdp().vram(a)).collect();
        (pad, regs, self.m.cpu().pc(), vram)
    }
}

/// Run the same plant+driver under both ROMs; assert the snapshots match.
/// Returns our run for the semantic asserts (always available).
fn differential(plant: impl Fn(&mut Run), driver: &[u16]) -> Run {
    let mut ours = Run::new(our_rom());
    plant(&mut ours);
    ours.plant_driver(driver);
    ours.go();
    if let Some(auth) = AUTH_ROM.as_deref() {
        let mut a = Run::new(auth);
        plant(&mut a);
        a.plant_driver(driver);
        a.go();
        let (ap, ar, apc, av) = a.snapshot();
        let (op, or, opc, ov) = ours.snapshot();
        for i in 0..ap.len() {
            assert_eq!(
                ap[i],
                op[i],
                "scratchpad >{:04X} differs (authentic {:02X} vs ours {:02X})",
                0x8300 + i,
                ap[i],
                op[i]
            );
        }
        assert_eq!(ar, or, "working registers R0-R11 differ");
        assert_eq!(apc, opc, "final PC differs (a different exit was taken)");
        assert_eq!(av, ov, "the planted VRAM window differs");
    } else {
        eprintln!("SKIPPED differential leg: third-party media not present");
    }
    ours
}

const BL: u16 = 0x06A0; // BL @addr (addr word follows)
const PARK: u16 = 0x10FF; // JMP $

/// Build a two-entry symbol chain in VRAM:
///   >1800: len 2, link >1810, text >1880 = "AB"
///   >1810: len 3, link 0,     text >1888 = "XYZ"
fn plant_chain(r: &mut Run) {
    let entries: [(u16, u8, u16, u16, &[u8]); 2] =
        [(0x1800, 2, 0x1810, 0x1880, b"AB"), (0x1810, 3, 0x0000, 0x1888, b"XYZ")];
    for (base, len, link, text, name) in entries {
        r.m.vdp_mut().set_vram(base, 0x00);
        r.m.vdp_mut().set_vram(base + 1, len);
        r.m.vdp_mut().set_vram(base + 2, (link >> 8) as u8);
        r.m.vdp_mut().set_vram(base + 3, link as u8);
        r.m.vdp_mut().set_vram(base + 4, (text >> 8) as u8);
        r.m.vdp_mut().set_vram(base + 5, text as u8);
        for (i, &b) in name.iter().enumerate() {
            r.m.vdp_mut().set_vram(text + i as u16, b);
        }
    }
    r.m.bus_mut().poke_word(0x833E, 0x1800); // the chain head
}

fn seek(r: &mut Run, name: &[u8]) {
    r.m.bus_mut().poke(0x8359, name.len() as u8);
    for (i, &b) in name.iter().enumerate() {
        r.m.bus_mut().poke(0x834A + i as u16, b);
    }
}

#[test]
fn symsrc_finds_a_chained_name() {
    // BL @>15E0 / DATA >2200 (miss park) / JMP $ (found park at >2106).
    let driver = [BL, 0x15E0, 0x2200, PARK];
    let r = differential(
        |r| {
            plant_chain(r);
            seek(r, b"XYZ");
        },
        &driver,
    );
    assert_eq!(r.m.cpu().pc(), 0x2106, "found must resume past the DATA word");
    assert_eq!(r.m.bus().peek_word(0x834A), 0x1810, ">834A must hold the entry's base");
}

#[test]
fn symsrc_misses_to_the_continuation_word() {
    let driver = [BL, 0x15E0, 0x2200, PARK];
    let r = differential(
        |r| {
            plant_chain(r);
            seek(r, b"QQQ");
        },
        &driver,
    );
    assert_eq!(r.m.cpu().pc(), 0x2200, "a miss must branch to the DATA word's address");
}

#[test]
fn symsrc_length_gates_the_text_compare() {
    // "AB" (len 2) lives in the first entry; "ABX" (len 3) must miss it AND
    // the len-3 "XYZ" entry (text differs).
    let driver = [BL, 0x15E0, 0x2200, PARK];
    let r = differential(
        |r| {
            plant_chain(r);
            seek(r, b"AB");
        },
        &driver,
    );
    assert_eq!(r.m.cpu().pc(), 0x2106);
    assert_eq!(r.m.bus().peek_word(0x834A), 0x1800, "the len-2 entry is the match");

    let r = differential(
        |r| {
            plant_chain(r);
            seek(r, b"ABX");
        },
        &driver,
    );
    assert_eq!(r.m.cpu().pc(), 0x2200, "no len-3 entry is named ABX");
}

#[test]
fn symsrc_empty_chain_misses() {
    let driver = [BL, 0x15E0, 0x2200, PARK];
    let r = differential(
        |r| {
            r.m.bus_mut().poke_word(0x833E, 0x0000);
            seek(r, b"XYZ");
        },
        &driver,
    );
    assert_eq!(r.m.cpu().pc(), 0x2200);
}

#[test]
fn rdcell_reads_the_byte_at_the_named_cell() {
    // BL @>187C / DATA >8320 (the cell) / JMP $ at >2106.
    let driver = [BL, 0x187C, 0x8320, PARK];
    let r = differential(
        |r| {
            r.m.bus_mut().poke_word(0x8320, 0x1840);
            r.m.vdp_mut().set_vram(0x1840, 0x5A);
        },
        &driver,
    );
    assert_eq!(r.m.cpu().pc(), 0x2106, "control resumes past the DATA word");
    assert_eq!(r.m.reg(1) >> 8, 0x5A, "the byte lands in R1's high byte");
}

#[test]
fn rdval8_copies_the_vdp_value_into_fac() {
    let pattern = [0x40u8, 12, 34, 56, 78, 90, 12, 51];
    let driver = [BL, 0x1890, PARK];
    let r = differential(
        |r| {
            r.m.bus_mut().poke_word(0x834E, 0x1850);
            for (i, &b) in pattern.iter().enumerate() {
                r.m.vdp_mut().set_vram(0x1850 + i as u16, b);
            }
        },
        &driver,
    );
    for (i, &b) in pattern.iter().enumerate() {
        assert_eq!(r.m.bus().peek(0x834A + i as u16), b, "FAC byte {i}");
    }
}

#[test]
fn wrword_writes_r6_at_r1_and_biases() {
    // >18AE: write at R1 as given; R1 comes back with the >4000 write bit.
    let driver = [BL, 0x18AE, PARK];
    let r = differential(
        |r| {
            r.m.bus_mut().poke_word(0x83E2, 0x1860); // R1
            r.m.bus_mut().poke_word(0x83EC, 0xBEEF); // R6
        },
        &driver,
    );
    assert_eq!(r.m.vdp().vram(0x1860), 0xBE);
    assert_eq!(r.m.vdp().vram(0x1861), 0xEF);
    assert_eq!(r.m.reg(1), 0x5860, "R1 returns ORed with the write bit");

    // >18AA: the same write with the address backed up by 3.
    let driver = [BL, 0x18AA, PARK];
    let r = differential(
        |r| {
            r.m.bus_mut().poke_word(0x83E2, 0x1873); // R1: 3 past the target
            r.m.bus_mut().poke_word(0x83EC, 0xCAFE);
        },
        &driver,
    );
    assert_eq!(r.m.vdp().vram(0x1870), 0xCA);
    assert_eq!(r.m.vdp().vram(0x1871), 0xFE);
}

#[test]
fn stack_bracket_pushes_through_the_gpl_substack() {
    // BL @>1E7A / INCT R9 / MOV R5,*R9 / BL @>1E8C / JMP $.
    let driver = [BL, 0x1E7A, 0x05C9, 0xC645, BL, 0x1E8C, PARK];
    let r = differential(
        |r| {
            r.m.bus_mut().poke(0x8342, 0x77);
            r.m.bus_mut().poke(0x8373, 0x20); // sub-stack top at >8320
            r.m.bus_mut().poke_word(0x83EA, 0x1234); // R5: the pushed word
        },
        &driver,
    );
    assert_eq!(r.m.bus().peek_word(0x8322), 0x1234, "the word pushed above the old top");
    assert_eq!(r.m.bus().peek(0x8373), 0x22, "STKOFF writes the advanced pointer back");
    assert_eq!(r.m.bus().peek(0x8342), 0x77, ">8342 restored");
}

#[test]
fn vpopag_pops_the_value_stack_into_arg() {
    let pattern = [0x41u8, 3, 14, 15, 92, 65, 35, 89];
    let driver = [BL, 0x1FA8, PARK];
    let r = differential(
        |r| {
            r.m.bus_mut().poke_word(0x836E, 0x1880); // the top element's base
            for (i, &b) in pattern.iter().enumerate() {
                r.m.vdp_mut().set_vram(0x1880 + i as u16, b);
            }
        },
        &driver,
    );
    for (i, &b) in pattern.iter().enumerate() {
        assert_eq!(r.m.bus().peek(0x835C + i as u16), b, "ARG byte {i}");
    }
    assert_eq!(r.m.bus().peek_word(0x836E), 0x1878, ">836E popped by 8");
}
