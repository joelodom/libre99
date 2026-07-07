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

//! The **keyboard/joystick decode tables** the console ROM's `SCAN` reads out of
//! GROM (review finding: the ROM implements the GPL `SCAN` opcode by looking up
//! tables at *fixed* GROM addresses, so a GROM rewrite that omits them makes
//! keypresses decode to `>00` — every keyboard-driven cartridge is crippled).
//!
//! This is a **functional hardware interface**, not creative content: `SCAN`
//! scans the 8×8 key matrix into a scan code and indexes one of four 43-entry
//! ASCII blocks by the held modifier (none / SHIFT / FCTN / CTRL):
//!
//! * **unshifted** at GROM `>1705`,
//! * **shifted**   at GROM `>1735`,
//! * **FCTN**      at GROM `>1765`  — the *arrow keys* live here
//!   (FCTN+S/D/E/X → left `>08` / right `>09` / up `>0B` / down `>0A`), and
//! * **CTRL**      at GROM `>1795`.
//!
//! Each block is preceded by five unused entries (scan codes 0..=4), so a block
//! for scan codes 5..=47 occupies 48 bytes. The base addresses and scan-code
//! order are dictated by the console ROM (recovered by execution in
//! `examples/keymap_probe.rs` / `examples/fctn_probe.rs`: press each key against
//! the authentic GROM, observe which offset the ROM reads). The *values* are the
//! codes the TI-99/4A keyboard is documented to produce — the ASCII standard, the
//! printed FCTN legends (`[ ] { } \ | ' " _ ` ? ~` and the edit keys), and the
//! `CTRL+letter → >80+n` rule — so the tables are reconstructed from their
//! functional specification, never copied from TI's image.
//!
//! ## Joystick / split-keyboard scan (the in-game arrow keys)
//!
//! When a cartridge selects **key-unit 1 or 2** (writes `>01`/`>02` to the mode
//! cell `>8374` — what TI Invaders and other joystick games do *during play*),
//! `SCAN` does **not** use the ASCII blocks. Two decode tables serve this mode:
//!
//! * the **keyboard-arrow** path — the split-keyboard halves translate through a
//!   40-byte table at GROM **`>17C8`**, returning a direction/fire code (the TI
//!   edit/arrow codes `>00..>13`) in `KEY` (`>8375`); its two halves are each
//!   palindromic (the ROM walks them from both ends), and
//! * the **joystick** path — the joystick column is read and turned into signed
//!   X/Y deflections via [`JOY_DEFLECT`] at GROM **`>16EA`**, stored in
//!   `JOYY`/`JOYX` (`>8376`/`>8377`).
//!
//! Omitting `>17C8` kills in-game keyboard movement even though the mode-0 menu
//! and level select still work; omitting `>16EA` kills the *joystick* while
//! keyboard play still works. Both were recovered by execution
//! (`examples/joystick_scan_check.rs`, `examples/joy_gromtrace.rs`).
//!
//! [`emit_gpl_bytes`] renders the whole `>1700..>17EF` region as GPL `BYTE`
//! directives for splicing at GROM `>1700`.

/// GROM address of the whole table (5 unused entries, then the unshifted block).
pub const BASE: u16 = 0x1700;
/// Where `SCAN` starts the unshifted lookup (`BASE + 5`).
pub const UNSHIFTED: u16 = 0x1705;
/// Where `SCAN` starts the shifted lookup.
pub const SHIFTED: u16 = 0x1735;
/// Where `SCAN` starts the FCTN lookup (arrow keys and edit keys).
pub const FCTN: u16 = 0x1765;
/// Where `SCAN` starts the CTRL lookup.
pub const CTRL: u16 = 0x1795;
/// The joystick / split-keyboard translation table (key-units 1–2).
pub const JOYSTICK: u16 = 0x17C8;
/// The joystick **deflection** table, just below the keytab.
pub const JOY_DEFLECT_BASE: u16 = 0x16EA;

/// The joystick deflection table at GROM `>16EA` (22 bytes = 11 `(Y, X)` pairs).
/// In key-units 1–2 `SCAN` reads the joystick column, forms an index from the
/// pressed directions, and reads a `(Y, X)` pair here — the signed deflections
/// `+4`/`0`/`-4` (`>04`/`>00`/`>FC`) it stores in `JOYY`(`>8376`)/`JOYX`(`>8377`).
/// Without it every joystick direction yields zero deflection — the joystick is
/// "wired" (the ROM reads column 6/7) but never moves anything. This is a
/// *different* path from the keyboard arrows, which decode through [`JOYSTICK`]
/// into `KEY`; that is why keyboard play works while the joystick is dead.
///
/// Index = `vert + horiz`, where `vert` ∈ {up:0, down:4, none:8} and
/// `horiz` ∈ {right:0, left:1, none:2}; the two `vert+3` slots are unused. Values
/// are the deflections themselves (recovered by execution in
/// `examples/joy_gromtrace.rs`), reconstructed from that functional spec.
const JOY_DEFLECT: [u8; 22] = [
    // Y,    X       index  direction
    0x04, 0x04, //   0      up + right
    0x04, 0xFC, //   1      up + left
    0x04, 0x00, //   2      up
    0x00, 0x00, //   3      (unused)
    0xFC, 0x04, //   4      down + right
    0xFC, 0xFC, //   5      down + left
    0xFC, 0x00, //   6      down
    0x00, 0x00, //   7      (unused)
    0x00, 0x04, //   8      right
    0x00, 0xFC, //   9      left
    0x00, 0x00, //  10      centered
];

/// The 43 keys in scan-code order (offset 0 == scan code 5), each as
/// `(unshifted, shifted, fctn, ctrl)`. `unshifted`/`shifted` are the printable
/// ASCII the key emits; `fctn`/`ctrl` are the raw byte codes (many are control
/// codes or codes >127), given as `u8`. This layout mirrors the TI-99/4A key
/// matrix as the ROM's `SCAN` walks it.
///
/// **The unshifted letters are LOWERCASE** (`a`–`z`), matching the authentic
/// console GROM's unshifted keytab byte-for-byte (`>1708` = `x w s …`, RECON §23).
/// The console-ROM KSCAN normalizes them per translation state: state 0 (the menu
/// / TI-BASIC "99/4" state) folds `a`–`z` **to uppercase** unconditionally, so the
/// menu still selects on `S`/etc.; the native state (mode 5, what Extended BASIC
/// uses) **keeps lowercase** — so typing in Extended BASIC shows lowercase (small
/// caps) exactly as the real machine does. Shipping uppercase here (the pre-fix
/// bug) masked the fold in state 0 but left native-mode input stuck uppercase.
/// The fold direction and state-gating are gated by `tests/rom_kscan.rs`.
///
/// The FCTN column carries the arrow/edit codes the console produces:
/// AID `>01`, CLEAR `>02`, DEL `>03`, INS `>04`, QUIT `>05`, REDO `>06`,
/// ERASE `>07`, LEFT `>08`, RIGHT `>09`, DOWN `>0A`, UP `>0B`, PROC'D `>0C`,
/// BEGIN `>0E`, BACK `>0F`, the printed symbol legends, and the >127 glyph keys.
/// The CTRL column is `>80 + n` for the letters and TI's documented control codes
/// for the digits and punctuation.
const KEYS: [(char, char, u8, u8); 43] = [
    ('\r', '\r', 0x0D, 0x0D), (' ', ' ', 0x20, 0x20), ('=', '+', 0x05, 0x9D), ('x', 'X', 0x0A, 0x98),
    ('w', 'W', 0x7E, 0x97), ('s', 'S', 0x08, 0x93), ('2', '@', 0x04, 0xB2), ('9', '(', 0x0F, 0x9F),
    ('o', 'O', 0x27, 0x8F), ('l', 'L', 0xC2, 0x8C), ('.', '>', 0xB9, 0x9B), ('c', 'C', 0x60, 0x83),
    ('e', 'E', 0x0B, 0x85), ('d', 'D', 0x09, 0x84), ('3', '#', 0x07, 0xB3), ('8', '*', 0x06, 0x9E),
    ('i', 'I', 0x3F, 0x89), ('k', 'K', 0xC1, 0x8B), (',', '<', 0xB8, 0x80), ('v', 'V', 0x7F, 0x96),
    ('r', 'R', 0x5B, 0x92), ('f', 'F', 0x7B, 0x86), ('4', '$', 0x02, 0xB4), ('7', '&', 0x01, 0xB7),
    ('u', 'U', 0x5F, 0x95), ('j', 'J', 0xC0, 0x8A), ('m', 'M', 0xC3, 0x8D), ('b', 'B', 0xBE, 0x82),
    ('t', 'T', 0x5D, 0x94), ('g', 'G', 0x7D, 0x87), ('5', '%', 0x0E, 0xB5), ('6', '^', 0x0C, 0xB6),
    ('y', 'Y', 0xC6, 0x99), ('h', 'H', 0xBF, 0x88), ('n', 'N', 0xC4, 0x8E), ('z', 'Z', 0x5C, 0x9A),
    ('q', 'Q', 0xC5, 0x91), ('a', 'A', 0x7C, 0x81), ('1', '!', 0x03, 0xB1), ('0', ')', 0xBC, 0xB0),
    ('p', 'P', 0x22, 0x90), (';', ':', 0xBD, 0x9C), ('/', '-', 0xBA, 0xBB),
];

/// The 40-byte joystick / split-keyboard translation table at GROM `>17C8`,
/// read by `SCAN` in key-units 1–2 to map a scanned direction to its TI
/// direction/fire code. Two palindromic halves (24 + 16 bytes): the ROM walks
/// each from both ends for the two mirrored scan orders.
const JOYSTICK_TABLE: [u8; 40] = [
    0x00, 0x04, 0x02, 0x07, 0x09, 0x06, 0x0C, 0x0D, 0x0E, 0x05, 0x03, 0x08,
    0x08, 0x05, 0x03, 0x0E, 0x0D, 0x06, 0x0C, 0x09, 0x07, 0x04, 0x02, 0x00,
    0x10, 0x0B, 0x11, 0x0A, 0x13, 0x12, 0x01, 0x0F,
    0x0F, 0x12, 0x01, 0x13, 0x0A, 0x0B, 0x11, 0x10,
];

/// Number of unused scan codes (0..=4) before each lookup block.
const PAD: usize = 5;
/// Unused entries between the CTRL block (ends `>17BF`) and the joystick table
/// (`>17C8`).
const JOY_GAP: usize = 8;

/// The packed table image, `>1700` up to the end of the joystick table
/// (`>17EF`): four `PAD`-then-43 ASCII blocks, `JOY_GAP` `>FF`, then the 40-byte
/// joystick table.
pub fn packed() -> Vec<u8> {
    let mut out = Vec::new();
    for col in 0..4 {
        out.extend([0xFFu8; PAD]);
        out.extend(KEYS.iter().map(|k| match col {
            0 => k.0 as u8,
            1 => k.1 as u8,
            2 => k.2,
            _ => k.3,
        }));
    }
    out.extend([0xFFu8; JOY_GAP]);
    out.extend_from_slice(&JOYSTICK_TABLE);
    out
}

/// Render the tables as GPL source: the joystick deflection table (`GROM >16EA`)
/// followed by the four ASCII blocks and joystick table (`GROM >1700`), as `BYTE`
/// directives.
pub fn emit_gpl_bytes(label: &str) -> String {
    let mut s = String::new();
    // Joystick deflection table, just below the keytab (>16EA..>16FF).
    s.push_str("* joystick deflection table (>16EA, key-units 1-2): (Y,X) pairs\n");
    s.push_str("        GROM >16EA\n");
    for chunk in JOY_DEFLECT.chunks(2) {
        s.push_str(&format!("        BYTE >{:02X},>{:02X}\n", chunk[0], chunk[1]));
    }
    s.push_str("        GROM >1700\n");
    let blocks = [
        (label, "unshifted", 0usize),
        ("", "shifted", 1),
        ("", "FCTN (arrow/edit keys)", 2),
        ("", "CTRL", 3),
    ];
    for (lbl, name, col) in blocks {
        s.push_str(&format!("* {name} block\n"));
        s.push_str(&format!("{lbl:<7} BYTE >FF,>FF,>FF,>FF,>FF        ; scan codes 0-4 (unused)\n"));
        for (i, k) in KEYS.iter().enumerate() {
            let byte = match col {
                0 => k.0 as u8,
                1 => k.1 as u8,
                2 => k.2,
                _ => k.3,
            };
            s.push_str(&byte_line(byte, i + PAD, k.0));
        }
    }
    // The joystick / split-keyboard table at >17C8 (8 unused entries first).
    s.push_str("* joystick / split-keyboard table (>17C8, key-units 1-2)\n");
    s.push_str("        BYTE >FF,>FF,>FF,>FF,>FF,>FF,>FF,>FF\n");
    for chunk in JOYSTICK_TABLE.chunks(8) {
        let bytes: Vec<String> = chunk.iter().map(|b| format!(">{b:02X}")).collect();
        s.push_str(&format!("        BYTE {}\n", bytes.join(",")));
    }
    s
}

/// One `BYTE` line for scan code `scan`, annotated with the key it belongs to.
fn byte_line(byte: u8, scan: usize, key: char) -> String {
    let shown = match key {
        ' ' => "space".to_string(),
        '\r' => "enter".to_string(),
        c => c.to_string(),
    };
    format!("        BYTE >{byte:02X}                       ; scan {scan}: {shown}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_layout_and_values() {
        let t = packed();
        // Region spans >1700..>17EF (four 48-byte blocks, 8-byte gap, 40-byte table).
        assert_eq!(t.len(), 4 * (PAD + 43) + JOY_GAP + 40);
        assert_eq!(t.len(), (0x17F0 - BASE) as usize);
        // Unshifted lookups the ROM makes (from keymap_probe): '2' at >170B, 'a'
        // (lowercase — matches the authentic keytab) at >172A, '1' at >172B, Space
        // at >1706, Enter at >1705.
        assert_eq!(t[(0x170B - BASE) as usize], b'2');
        assert_eq!(t[(0x172A - BASE) as usize], b'a', "unshifted letters are lowercase (authentic)");
        assert_eq!(t[(0x172B - BASE) as usize], b'1');
        assert_eq!(t[(0x1706 - BASE) as usize], b' ');
        assert_eq!(t[(0x1705 - BASE) as usize], b'\r');
        // Shifted: '2'->'@' at >173B, '9'->'(' at >173C, '='->'+' at >1737.
        assert_eq!(t[(0x173B - BASE) as usize], b'@');
        assert_eq!(t[(0x173C - BASE) as usize], b'(');
        assert_eq!(t[(0x1737 - BASE) as usize], b'+');
    }

    #[test]
    fn fctn_block_has_the_arrow_keys() {
        // The console ROM's mode-0 SCAN reads the FCTN block for FCTN+key; the
        // arrow keys are FCTN+S/D/E/X (recovered by execution in fctn_probe:
        // left reads >176A, right >1772, up >1771, down >1768).
        let t = packed();
        assert_eq!(t[(0x176A - BASE) as usize], 0x08, "FCTN+S = left arrow");
        assert_eq!(t[(0x1772 - BASE) as usize], 0x09, "FCTN+D = right arrow");
        assert_eq!(t[(0x1771 - BASE) as usize], 0x0B, "FCTN+E = up arrow");
        assert_eq!(t[(0x1768 - BASE) as usize], 0x0A, "FCTN+X = down arrow");
    }

    #[test]
    fn ctrl_block_follows_the_letter_rule() {
        // CTRL+letter = >80 + alphabetical position (A=1..Z=26). Unshifted key
        // chars are lowercase now, so look them up by lowercase letter.
        let t = packed();
        for (ch, code) in [('a', 0x81u8), ('s', 0x93), ('z', 0x9A), ('m', 0x8D)] {
            let idx = KEYS.iter().position(|k| k.0 == ch).unwrap();
            // CTRL is the 4th block: 3 full 48-byte blocks, then this block's PAD.
            assert_eq!(t[3 * (PAD + 43) + PAD + idx], code, "CTRL+{ch}");
        }
    }

    #[test]
    fn joystick_deflection_table_lands_at_16ea() {
        // The full assembled console GROM must carry the (Y,X) deflection table at
        // >16EA (recovered by execution in joy_gromtrace: Left reads >16FD=>FC,
        // Right >16FB=>04, Up >16EE=>04, Down >16F6=>FC).
        let img = crate::system_grom::build_console_grom().unwrap();
        assert_eq!(&img[0x16EA..0x1700], &JOY_DEFLECT, "deflection table at >16EA");
        assert_eq!(img[0x16FD], 0xFC, "Joy1Left X deflection = -4");
        assert_eq!(img[0x16FB], 0x04, "Joy1Right X deflection = +4");
        assert_eq!(img[0x16EE], 0x04, "Joy1Up Y deflection = +4");
        assert_eq!(img[0x16F6], 0xFC, "Joy1Down Y deflection = -4");
    }

    #[test]
    fn joystick_table_lands_at_17c8() {
        // In-game (key-unit 1-2) SCAN reads this table; recovered by execution in
        // invaders_play_probe (down reads >17C8, left >17CA, up >17D1, right >17D2).
        let t = packed();
        assert_eq!(t[(JOYSTICK - BASE) as usize], 0x00, ">17C8 table present");
        assert_eq!(t[(0x17CA - BASE) as usize], JOYSTICK_TABLE[2]);
        assert_eq!(t[(0x17D2 - BASE) as usize], JOYSTICK_TABLE[10]);
    }
}
