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

//! End-to-end proof: assemble the §10.2 "HELLO" demo, pack a `.ctg`, boot the
//! real console, select the menu entry, and confirm the program ran.
//!
//! The source is embedded here (not read from the gitignored `assembler/poc/`
//! folder) so the test is self-contained and CI-safe.
//!
//! NOTE on the assertion: the demo loads custom glyphs at character codes
//! `>60..>63`, so the message cells hold `[60,61,62,62,63]` — which the usual
//! "name table as ASCII" helper would read as `` `abbc ``, not `HELLO`. The
//! pixels spell HELLO; the test therefore checks the raw cell codes and the
//! loaded glyph, not an ASCII read-back. (See assembler/POC_PLAN.md §1.)

use std::sync::LazyLock;

use libre99_asm::{assemble, Options};
use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

const HELLO_SRC: &str = r#"
        IDT  'HELLO'
VDPWD   EQU  >8C00
VDPWA   EQU  >8C02
START   LIMI 0
        LWPI >8300
        LI   R1,REGTAB
        LI   R2,16
RL      MOVB *R1+,@VDPWA
        DEC  R2
        JNE  RL
        LI   R1,>0800
        BL   @SETWR
        LI   R2,2048
        CLR  R0
PCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  PCLR
        LI   R1,>0300
        BL   @SETWR
        LI   R2,32
        LI   R0,>1F00
CCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  CCLR
        LI   R1,>0000
        BL   @SETWR
        LI   R2,768
        LI   R0,>2000
NCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  NCLR
        LI   R1,>0B00
        BL   @SETWR
        LI   R2,FONT
        LI   R3,32
        BL   @VMBW
        LI   R1,>018D
        BL   @SETWR
        LI   R2,MSG
        LI   R3,5
        BL   @VMBW
SPIN    JMP  SPIN
SETWR   SWPB R1
        MOVB R1,@VDPWA
        SWPB R1
        ORI  R1,>4000
        MOVB R1,@VDPWA
        ANDI R1,>3FFF
        RT
VMBW    MOVB *R2+,@VDPWD
        DEC  R3
        JNE  VMBW
        RT
REGTAB  BYTE >00,>80,>C0,>81,>00,>82,>0C,>83
        BYTE >01,>84,>00,>85,>00,>86,>17,>87
FONT    BYTE >88,>88,>88,>F8,>88,>88,>88,>00
        BYTE >F8,>80,>80,>F0,>80,>80,>F8,>00
        BYTE >80,>80,>80,>80,>80,>80,>F8,>00
        BYTE >70,>88,>88,>88,>88,>88,>70,>00
MSG     BYTE >60,>61,>62,>62,>63
        END  START
"#;

#[test]
fn hello_assembles_to_the_expected_layout() {
    let asm = assemble(HELLO_SRC, &Options::default()).expect("assembles");
    assert_eq!(asm.title, "HELLO");
    assert_eq!(asm.entry, 0x601A, "START is right after the 26-byte header prefix");
    let sym = |n: &str| asm.symbols.iter().find(|(s, _)| s == n).map(|(_, v)| *v);
    assert_eq!(sym("START"), Some(0x601A));
    assert_eq!(sym("SETWR"), Some(0x60A2));
    assert_eq!(sym("VMBW"), Some(0x60B8));
    assert_eq!(sym("REGTAB"), Some(0x60C2));
    assert_eq!(sym("FONT"), Some(0x60D2));
    assert_eq!(sym("MSG"), Some(0x60F2));

    // The synthesized header.
    assert_eq!(&asm.image[0..8], &[0xAA, 0x01, 0x01, 0x00, 0x00, 0x00, 0x60, 0x10]);
    // The program-list entry at >6010: next, entry=>601A, len=5, "HELLO".
    assert_eq!(&asm.image[0x10..0x1A], &[0x00, 0x00, 0x60, 0x1A, 0x05, b'H', b'E', b'L', b'L', b'O']);
}

#[test]
fn hello_cartridge_boots_and_writes_the_message() {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return;
    };
    let asm = assemble(HELLO_SRC, &Options::default()).expect("assembles");
    let ctg = asm.ctg();
    let cart = Cartridge::parse(&ctg).expect("our .ctg parses");
    assert_eq!(cart.rom_banks, 1);
    assert_eq!(cart.rom[0], 0xAA);

    let mut m = Machine::new(rom, grom);
    m.mount_cartridge(&cart);
    m.reset();

    // Boot to the master title screen, advance past it, then select "2 FOR HELLO".
    for _ in 0..180 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, true);
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..120 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, true);
    for _ in 0..20 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, false);
    for _ in 0..120 {
        m.run_frame();
    }

    // Graphics I, display on (the program's REGTAB programs the VDP this way).
    assert!(
        m.vdp().register(1) & 0x40 != 0,
        "display should be ON; program did not run (R1=>{:02X})",
        m.vdp().register(1)
    );
    // The message glyph codes at name-table cell >018D (row 12, col 13).
    let cells: Vec<u8> = (0..5).map(|i| m.vdp().vram(0x018D + i)).collect();
    assert_eq!(cells, vec![0x60, 0x61, 0x62, 0x62, 0x63], "HELLO message cells");
    // The H glyph loaded into pattern slot >60 (>0B00).
    let h: Vec<u8> = (0..8).map(|i| m.vdp().vram(0x0B00 + i)).collect();
    assert_eq!(h, vec![0x88, 0x88, 0x88, 0xF8, 0x88, 0x88, 0x88, 0x00], "H glyph");
}
