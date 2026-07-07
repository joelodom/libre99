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

//! Builds the rewritten console GROM image from its GPL source
//! (`original-content/system-roms/grom/console.gpl`) with the original font
//! (`font`) spliced in — the single source of truth for the system-GROM
//! rewrite. The emulator boots the result in place of `994AGROM.Bin` (via the
//! `--system-grom` flag or the embed).

use crate::asm::{assemble, Assembly, Diag};

/// The GPL source of the console GROM, with the spliced data blocks.
///
/// **Interface data at its authentic home addresses** (B1 — the surface map's
/// `DATA-MUST-MATCH` regions; see [`crate::census`] and `grom/SURFACE-MAP.md`).
/// A cartridge written against TI's shipped image can read these tables from the
/// documented GROM addresses directly (rather than through a loader), so we ship
/// them where the authentic firmware does. These regions are zero in our image
/// otherwise, so this is pure placement, gated byte-identical by
/// `tests/census.rs`:
///
/// * the **standard 8×8 character set** at GROM `>04B4` (label `FONTA`, 512
///   bytes) — byte-identical to authentic; the same content also lives at
///   `>1000` (label `FONT`) where our own title/menu read it;
/// * the **thin ("small") character set** at GROM `>06B4` (label `THINA`, 448
///   bytes, seven rows per glyph as stored) — byte-identical to authentic;
/// * the **lower-case (small capitals) character set** at GROM `>0874` (label
///   `LOWERA`, 217 bytes, seven rows per glyph as stored) — byte-identical to
///   authentic; the fixed service entry `>004A` loads it (see `LDLSET`); and
/// * an **original** menu-beep sound list at GROM `>0484` (label `BEEP0484`) —
///   the authentic firmware keeps its beep list here; we place an original tune
///   at the same address (creative content, deliberately *not* TI's bytes).
///
/// **Original on-screen content and decode tables:**
///
/// * the original **font** at GROM `>1000` (label `FONT`), read by the title and
///   menu with `MOVE …,G@FONT,…`;
/// * the original **Libre99 chip logo** at GROM `>1600` (label `LOGO`, glyphs
///   `>0B..>1E`), the title screen's original-content replacement for TI's "TI"
///   logo (see [`crate::logo`]);
/// * the **keyboard/joystick decode tables** at GROM `>1700` (label `KEYTAB`),
///   read by the console ROM's `SCAN`: the four ASCII blocks
///   (unshifted/shifted/FCTN/CTRL, `>1705`/`>1735`/`>1765`/`>1795`) and the
///   joystick / split-keyboard table at `>17C8` that joystick games read during
///   play. The FCTN block and `>17C8` table carry the arrow keys — without them
///   in-game movement is dead (see [`crate::keymap`]); and
/// * the **thin ("small") character set loader block** at GROM `>4000` (label
///   `FONT2`, eight rows per glyph, expanded), loaded by the char-set utility at
///   interconnect slot `>0018` for cartridges (e.g. TI Invaders) that use it
///   (see [`crate::font::emit_gpl_bytes_thin`]), and the **lower-case loader
///   block** at GROM `>4200` (label `FONT3`, likewise expanded), loaded by the
///   fixed service entry `>004A` for cartridges (e.g. Parsec) that stage the
///   small-caps set (see [`crate::font::emit_gpl_bytes_lower`]). They live in
///   empty GROM 2, **not** in the `>1800` chip gap: real GROMs are 6 KiB chips
///   in 8 KiB slots, so `>1800-1FFF` does not exist on hardware (B4 — the census
///   gate asserts all three chip gaps stay zero in our image).
///
/// Each block carries its own `GROM` directive, so the pieces may be concatenated
/// in any order and `console.gpl` need not end at any particular location counter.
pub fn console_gpl_source() -> String {
    let code = include_str!("../../../original-content/system-roms/grom/console.gpl");
    format!(
        "{code}\n\
         {beep}\
         \n        GROM >04B4\n{font_home}\
         \n        GROM >06B4\n{thin_home}\
         \n        GROM >0874\n{lower_home}\
         \n        GROM >1000\n{font}\
         \n{logo}\
         \n{keytab}\
         \n        GROM >4000\n{font2}\
         \n        GROM >4200\n{font3}\
         \n{sysinfo}",
        sysinfo = sysinfo_block(),
        beep = original_beep_list(),
        font_home = crate::font::emit_gpl_bytes("FONTA"),
        thin_home = crate::font::emit_gpl_bytes_thin_stored("THINA"),
        lower_home = crate::font::emit_gpl_bytes_lower_stored("LOWERA"),
        font = crate::font::emit_gpl_bytes("FONT"),
        logo = crate::logo::emit_gpl_bytes("LOGO"),
        keytab = crate::keymap::emit_gpl_bytes("KEYTAB"),
        font2 = crate::font::emit_gpl_bytes_thin("FONT2"),
        font3 = crate::font::emit_gpl_bytes_lower("FONT3"),
    )
}

/// An **original** menu-beep sound list spliced at GROM `>0484` — the address
/// the authentic firmware keeps its beep list at (QUALITY-ASSESSMENT §3, L7). We
/// ship our own short ~875 Hz chirp so a cartridge that points the ISR
/// sound-list cell at `>0484` finds a well-formed list rather than zeros; no TI
/// sound bytes are copied. Format (RECON): `[N][N PSG bytes][duration frames]`,
/// a duration-0 block ends the list. Kept to 8 bytes (`>0484-048B`) so it stays
/// clear of the pre-font region at `>048F` and the font at `>04B4`.
fn original_beep_list() -> String {
    // Built with explicit newlines (not `\`-continuations, which would strip the
    // second line's indent and push BYTE into the label column).
    let mut s = String::new();
    s.push_str("        GROM >0484\n");
    s.push_str("BEEP0484 BYTE >03,>80,>08,>92,>18   ; ch0 tone (divider 128), vol 2, 24 frames\n");
    s.push_str("        BYTE >01,>9F,>00            ; mute channel 0; duration 0 ends the list\n");
    s
}

/// The **Libre99 emulator-identification block** (GROM `>5700`) plus the baked
/// version strings that follow it — the data behind the system information
/// screen (`(S)` on the selection menu; the screen's code is the `SYSINF`
/// section of `console.gpl` at GROM `>4800`).
///
/// The block's layout is owned by `libre99_core::sysinfo` (the same constants the
/// frontend stamps through and the gates check), so the addresses used here
/// cannot drift from the stamper. The *stamped* fields ship as spaces with the
/// flag byte `>00`; an emulator that recognizes the magic fills them in its
/// in-memory copy at launch. The *baked* strings carry this workspace's
/// version (`CARGO_PKG_VERSION` — one number for the emulator, this GROM, and
/// TI PYTHON):
///
/// * `VERSTR` (20 bytes) — `LIBRE99 <version>`, the GROM row of the screen;
/// * `PYVERS` (8 bytes)  — `<version>`, the PYTHON row of the screen;
/// * `PYBANR` (20 bytes) — `TI PYTHON <version>`, the REPL's banner line.
fn sysinfo_block() -> String {
    use libre99_core::sysinfo as si;

    let version = env!("CARGO_PKG_VERSION");
    let pad = |text: String, width: usize| {
        assert!(
            text.len() <= width,
            "'{text}' does not fit its {width}-byte GROM field"
        );
        format!("{text:<width$}")
    };
    let blank = |width: usize| " ".repeat(width);

    let mut s = String::new();
    s.push_str(&format!("        GROM >{:04X}\n", si::BLOCK_ADDR));
    s.push_str("L99MAG  TEXT 'L99I'                 ; emulator-identification block magic\n");
    s.push_str("        BYTE >01                    ; block format\n");
    s.push_str("L99FLG  BYTE >00                    ; >01 = host fields stamped (the emulator writes it)\n");
    s.push_str(&format!(
        "L99EMU  TEXT '{}'          ; emulator version (stamped)\n",
        blank(si::EMU_VERSION.1)
    ));
    s.push_str(&format!(
        "L99BLD  TEXT '{}'        ; emulator build date (stamped)\n",
        blank(si::BUILD_DATE.1)
    ));
    s.push_str(&format!(
        "L99GIT  TEXT '{}'          ; emulator commit (stamped)\n",
        blank(si::COMMIT.1)
    ));
    s.push_str(&format!(
        "L99HST  TEXT '{}'      ; host OS/arch (stamped)\n",
        blank(si::HOST.1)
    ));
    s.push_str(&format!(
        "L99ROM  TEXT '{}'  ; mounted console-ROM identity (stamped)\n",
        blank(si::ROM_ID.1)
    ));
    s.push_str(&format!("VERSTR  TEXT '{}'  ; this GROM's own version\n", pad(format!("LIBRE99 {version}"), 20)));
    s.push_str(&format!("PYVERS  TEXT '{}'          ; TI PYTHON's version\n", pad(version.to_string(), 8)));
    s.push_str(&format!("PYBANR  TEXT '{}'  ; the REPL banner\n", pad(format!("TI PYTHON {version}"), 20)));
    s
}

/// Assemble the console GROM to a 24 KiB image.
pub fn build_console_grom() -> Result<Vec<u8>, Vec<Diag>> {
    assemble(&console_gpl_source()).map(|a: Assembly| a.image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembles_and_has_valid_header() {
        let img = build_console_grom().unwrap_or_else(|d| panic!("assembly failed: {d:?}"));
        assert_eq!(img.len(), crate::GROM_IMAGE_LEN);
        assert_eq!(img[0], 0xAA, "GROM 0 valid byte");
        assert_eq!(img[1], 0x02, "version 2");
        // The interconnect (GPLLNK) table >0010-0037 holds executable BR stubs.
        // Slot 8 (>0020) is the ROM's fixed GPL entry and BRanches to START;
        // slot 0 (>0010) is the DSRLNK device-I/O vector. (BR opcodes = >40..>5F.)
        assert!((0x40..=0x5F).contains(&img[0x0020]), "entry >0020 is a BR to START");
        assert!((0x40..=0x5F).contains(&img[0x0010]), "interconnect slot 0 (>0010) is a BR (DSRLNK)");
        // The font landed at >1000 (space glyph = all zero, 'A'>41 non-blank).
        let a_off = 0x1000 + (b'A' - crate::font::FIRST) as usize * 8;
        assert!(img[a_off..a_off + 8].iter().any(|&b| b != 0), "font 'A' present");
        // B1: the font also landed at its authentic home >04B4.
        let a_home = 0x04B4 + (b'A' - crate::font::FIRST) as usize * 8;
        assert!(img[a_home..a_home + 8].iter().any(|&b| b != 0), "font 'A' at >04B4");
        // B1: an original beep list at >0484 (first block header N=3).
        assert_eq!(img[0x0484], 0x03, "beep list at >0484");
        // B4: FONT2 moved out of the >1800 chip gap into GROM 2 at >4000.
        assert!(img[0x1800..0x2000].iter().all(|&b| b == 0), ">1800 chip gap is zero (FONT2 relocated)");
        assert!(img[0x4000..0x4200].iter().any(|&b| b != 0), "FONT2 present at >4000");
        // The keyboard table landed at >1700 ('2' at scan offset >170B; the
        // unshifted letters are lowercase, matching the authentic keytab).
        assert_eq!(img[0x170B], b'2', "keyboard table '2' at >170B");
        assert_eq!(img[0x172A], b'a', "keyboard table 'a' (lowercase) at >172A");
        // GROM 1 (>2000) carries the TI PYTHON program header.
        assert_eq!(img[0x2000], 0xAA, "GROM 1 valid byte");
        assert_eq!(img[0x2001], 0x01, "GROM 1 version 1");
    }
}
