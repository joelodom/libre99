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

//! End-to-end Sokoban test: assemble the game, boot the real console, and play.
//!
//! It assembles the tracked, playable source at
//! `original-content/cartridges/sokoban/sokoban.asm`, so the game and its
//! regression test can never drift apart — and the final test plays **every
//! shipped level to completion**, so a level-data transcription typo (or a
//! move/push/undo regression) cannot ship.
//!
//! Controls: E/S/D/X or joystick 1 move; U or fire undoes; R retries;
//! N/P skip; Q quits to the title; H (or AID) on the title opens help.

use std::sync::LazyLock;

use libre99_asm::{assemble, Options};
use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));
const SRC: &str = include_str!("../../../original-content/cartridges/sokoban/sokoban.asm");

// Scratchpad variables (see sokoban.asm).
const LVLNUM: u16 = 0x8320;
const PPOS: u16 = 0x8322;
const MOVES: u16 = 0x8324;
const PUSHES: u16 = 0x8326;
const BOXLEFT: u16 = 0x8328;
const UCNT: u16 = 0x832C;
const TICK: u16 = 0x832E;

const BOARD: u16 = 0xA000;

/// Move/push-optimal solutions for the twelve levels, in level order, as
/// produced by a BFS solver over the exact level text shipped in the ROM
/// (lowercase = walk, uppercase = push; the game doesn't distinguish).
const SOLUTIONS: [&str; 12] = [
    "rddLruulDuullddR",                          // 1  (Microban 2)
    "dlUrrrdLullddrUluRuulDrddrruLdlUU",         // 2  (Microban 1)
    "ullDLdRuurrdLLrrddlUruL",                   // 3  (Microban 4)
    "LuRllDrdRdrruuLLdlUddlluR",                 // 4  (Microban 5)
    "drDullDRddrruLUddlUUddlUU",                 // 5  (Microban 17)
    "uUlluurRDDlUdlddrUdrrUruuL",                // 6  (Microban 7)
    "urrDulldRdRluurDrDDlUruLdlUruL",            // 7  (Microban 9)
    "drUdrrURlLuurDldRdllluRluRRurD",            // 8  (Microban 34)
    "ruuLLLulDrrrrddlUruLLLddllluurRDrdLuuurDD", // 9  (Microban 3)
    "ulLrrddlLUdLuluurDldDrrrruulLLrddlluUrrdL", // 10 (Microban 33)
    "lluluuRDrDLddrruLdlUUruLuluurDrDLrDDlddrruLdlUUUUruLuurDDDDDlddrruLdlUUUUUruL", // 11 (Microban 35)
    "dllllllllllluurDldRRRRuLLdlluurDldRRurrdRRuLLLLdRRurrdRRuLLLLdRRlllllluurDldRRRRuLLdlluurDldRRurrrrrrdRRuLLLLLLLLdlluurDrrrdLurrrdLLurrrrdLLLurrrrrrrdLLLLLL", // 12 (Microban 36)
];

/// Boot the console and select the cartridge; stop on the game's title screen.
/// `None` (announcing the skip) when the third-party console images are absent.
fn boot_to_title() -> Option<Machine> {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return None;
    };
    let asm = assemble(SRC, &Options::default()).expect("Sokoban assembles");
    assert_eq!(asm.title, "SOKOBAN");
    let cart = Cartridge::parse(&asm.ctg()).unwrap();
    let mut m = Machine::new(rom, grom);
    m.mount_cartridge(&cart);
    m.reset();
    for _ in 0..180 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, true); // advance the console's own title
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..120 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, true); // pick "2 FOR SOKOBAN"
    for _ in 0..20 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, false);
    for _ in 0..150 {
        m.run_frame();
    }
    Some(m)
}

/// Dismiss the title with SPACE; the game consumes the starting keystroke.
fn start_game(m: &mut Machine) {
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..40 {
        m.run_frame();
    }
}

fn tap(m: &mut Machine, k: TiKey) {
    m.set_key(k, true);
    for _ in 0..4 {
        m.run_frame();
    }
    m.set_key(k, false);
    for _ in 0..3 {
        m.run_frame();
    }
}

/// Play an LURD solution string through joystick 1.
fn play(m: &mut Machine, solution: &str) {
    for c in solution.chars() {
        let k = match c.to_ascii_lowercase() {
            'l' => TiKey::Joy1Left,
            'r' => TiKey::Joy1Right,
            'u' => TiKey::Joy1Up,
            'd' => TiKey::Joy1Down,
            other => panic!("bad move {other:?}"),
        };
        tap(m, k);
    }
}

fn word(m: &Machine, a: u16) -> u16 {
    m.bus().peek_word(a)
}

fn count_name(m: &Machine, pred: impl Fn(u8) -> bool) -> usize {
    (0..768u16).filter(|&i| pred(m.vdp().vram(i))).count()
}

fn vram_text(m: &Machine, addr: u16, len: usize) -> Vec<u8> {
    (0..len as u16).map(|i| m.vdp().vram(addr + i)).collect()
}

#[test]
fn title_screen_shows_then_waits() {
    let Some(m) = boot_to_title() else { return };
    assert!(
        count_name(&m, |c| c == 0xB8) > 40,
        "expected the big SOKOBAN title in solid blocks"
    );
    assert_eq!(
        vram_text(&m, 0x0126, 20),
        b"THE WAREHOUSE KEEPER",
        "the sub-title should be drawn"
    );
    assert_eq!(
        vram_text(&m, 0x0187, 18),
        b"BY DAVID W SKINNER",
        "the level-set credit should be drawn"
    );
    assert_eq!(m.vdp().vram(0x01C5), b'P', "the prompt should be on screen");
    assert_eq!(word(&m, TICK), 0, "the game loop should not be ticking yet");
}

#[test]
fn help_screen_opens_with_h_and_aid() {
    let Some(mut m) = boot_to_title() else { return };

    m.set_key(TiKey::H, true);
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::H, false);
    for _ in 0..20 {
        m.run_frame();
    }
    assert_eq!(vram_text(&m, 0x0082, 4), b"GOAL", "H should open the help screen");
    assert_eq!(vram_text(&m, 0x0202, 8), b"CONTROLS", "help lists the controls");
    assert_eq!(m.vdp().vram(0x0103), 0x98, "help legend should show the box tile");
    assert_eq!(m.vdp().vram(0x0163), 0xA8, "help legend should show the keeper");
    assert_eq!(word(&m, TICK), 0, "opening help must not start the game");

    // Any key returns to the title.
    tap(&mut m, TiKey::Space);
    for _ in 0..60 {
        m.run_frame();
    }
    assert!(count_name(&m, |c| c == 0xB8) > 40, "dismissing help returns to the title");

    // AID (FCTN+7) opens it too — and holding FCTN must not start a game.
    m.set_key(TiKey::Fctn, true);
    m.set_key(TiKey::Num7, true);
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::Fctn, false);
    m.set_key(TiKey::Num7, false);
    for _ in 0..20 {
        m.run_frame();
    }
    assert_eq!(vram_text(&m, 0x0082, 4), b"GOAL", "AID (FCTN+7) should open help too");
    assert_eq!(word(&m, TICK), 0, "AID must not start the game");
}

#[test]
fn level_one_is_parsed_drawn_and_centered() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    // Level 1 is Microban 2: a 6x7 box with the keeper at row 2, col 3.
    assert_eq!(word(&m, LVLNUM), 0);
    assert_eq!(word(&m, PPOS), (2 << 5) | 3, "the keeper starts at (2,3)");
    assert_eq!(word(&m, MOVES), 0);
    assert_eq!(word(&m, PUSHES), 0);
    assert_eq!(word(&m, BOXLEFT), 1, "one box starts off its spot");
    assert!(word(&m, TICK) > 0, "the game loop should be ticking");

    // The HUD frames the board.
    assert_eq!(vram_text(&m, 0x0000, 7), b"SOKOBAN");
    assert_eq!(vram_text(&m, 0x0012, 8), b"LEVEL 01");
    assert_eq!(vram_text(&m, 0x001B, 5), b"OF 12");
    assert_eq!(vram_text(&m, 0x0020, 5), b"MOVES");
    for i in 0..5 {
        assert_eq!(m.vdp().vram(0x0026 + i), b'0', "moves digit {i} reads 0");
    }

    // Exactly the level's tiles are on screen: 23 wall cells, one loose box,
    // two stored boxes, one open spot, one keeper.
    assert_eq!(count_name(&m, |c| c == 0x80), 23, "wall glyph count");
    assert_eq!(count_name(&m, |c| c == 0x98), 1, "loose-box glyph count");
    assert_eq!(count_name(&m, |c| c == 0xA0), 2, "stored-box glyph count");
    assert_eq!(count_name(&m, |c| c == 0x90), 1, "open-spot glyph count");
    assert_eq!(count_name(&m, |c| c == 0xA8), 1, "keeper glyph count");
    assert!(
        count_name(&m, |c| c == 0x88) > 5,
        "interior floor should render the dotted glyph"
    );

    // The 6x7 level is centered: board (0,0) maps to name cell (8,13), and the
    // top-left wall is there.
    assert_eq!(m.vdp().vram(8 * 32 + 13), 0x80, "the level is centered on screen");
}

#[test]
fn walking_pushing_blocking_and_undo() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    let start = word(&m, PPOS);

    // Undo with nothing recorded is a no-op.
    tap(&mut m, TiKey::U);
    assert_eq!(word(&m, MOVES), 0, "undo at the start must be a no-op");
    assert_eq!(word(&m, PPOS), start);

    // Left of the keeper is a wall: the move is refused.
    tap(&mut m, TiKey::Joy1Left);
    assert_eq!(word(&m, MOVES), 0, "walking into a wall must not count");
    assert_eq!(word(&m, PPOS), start);

    // Below the keeper is a stored box backed by another box: unpushable.
    tap(&mut m, TiKey::Joy1Down);
    assert_eq!(word(&m, MOVES), 0, "pushing a blocked box must not count");

    // A step right works, on the keyboard diamond as well as the joystick.
    tap(&mut m, TiKey::D);
    assert_eq!(word(&m, PPOS), start + 1, "D steps right");
    assert_eq!(word(&m, MOVES), 1);

    // r d d L: walk around and push the loose box one cell left, spot to spot.
    play(&mut m, "ddL");
    assert_eq!(word(&m, MOVES), 4);
    assert_eq!(word(&m, PUSHES), 1, "the fourth move is a push");
    assert_eq!(word(&m, UCNT), 4, "every move is recorded for undo");

    // Undo the push: the box and keeper step back.
    tap(&mut m, TiKey::U);
    assert_eq!(word(&m, MOVES), 3, "undo takes back the move");
    assert_eq!(word(&m, PUSHES), 0, "undo takes back the push");

    // Undo everything (hold-to-rewind exercises the auto-repeat path).
    m.set_key(TiKey::U, true);
    for _ in 0..120 {
        m.run_frame();
    }
    m.set_key(TiKey::U, false);
    assert_eq!(word(&m, MOVES), 0, "rewinding returns to the start");
    assert_eq!(word(&m, PPOS), start);
    assert_eq!(word(&m, BOXLEFT), 1);
}

#[test]
fn retry_and_level_skip() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    play(&mut m, "rd");
    assert_eq!(word(&m, MOVES), 2);
    tap(&mut m, TiKey::R);
    for _ in 0..20 {
        m.run_frame();
    }
    assert_eq!(word(&m, MOVES), 0, "R restarts the level");
    assert_eq!(word(&m, PPOS), (2 << 5) | 3);

    tap(&mut m, TiKey::N);
    for _ in 0..20 {
        m.run_frame();
    }
    assert_eq!(word(&m, LVLNUM), 1, "N skips to the next level");
    assert_eq!(vram_text(&m, 0x0012, 8), b"LEVEL 02");

    // Level 2 (Microban 1) has cells outside the walls: they stay void while
    // the reachable inside is flood-marked as floor (bit 4).
    assert_eq!(m.bus().peek(BOARD + 4), 0x00, "outside the hull stays void");
    assert_eq!(m.bus().peek(BOARD + (2 << 5) + 1), 0x10, "inside is interior floor");

    tap(&mut m, TiKey::P);
    for _ in 0..20 {
        m.run_frame();
    }
    assert_eq!(word(&m, LVLNUM), 0, "P returns to the previous level");

    tap(&mut m, TiKey::P);
    for _ in 0..20 {
        m.run_frame();
    }
    assert_eq!(word(&m, LVLNUM), 11, "P wraps from the first level to the last");
    assert_eq!(vram_text(&m, 0x0012, 8), b"LEVEL 12");

    tap(&mut m, TiKey::N);
    for _ in 0..20 {
        m.run_frame();
    }
    assert_eq!(word(&m, LVLNUM), 0, "N wraps from the last level to the first");
}

#[test]
fn quit_returns_to_the_title() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    play(&mut m, "r");
    tap(&mut m, TiKey::Q);
    for _ in 0..20 {
        m.run_frame();
    }
    assert!(
        count_name(&m, |c| c == 0xB8) > 40,
        "Q should return to the title screen"
    );
}

#[test]
fn solving_a_level_advances_to_the_next() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    play(&mut m, SOLUTIONS[0]);
    assert_eq!(word(&m, BOXLEFT), 0, "every box should be on a spot");
    assert_eq!(
        vram_text(&m, 0x02A8, 15),
        b"LEVEL COMPLETE!",
        "the completion banner should show"
    );
    for _ in 0..140 {
        m.run_frame();
    }
    assert_eq!(word(&m, LVLNUM), 1, "solving level 1 advances to level 2");
    assert_eq!(word(&m, MOVES), 0, "the new level starts with fresh counters");
    assert_eq!(word(&m, BOXLEFT), 1, "Microban 1 starts with one loose box");
}

#[test]
fn all_twelve_levels_are_winnable_as_shipped() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    for (i, solution) in SOLUTIONS.iter().enumerate() {
        assert_eq!(word(&m, LVLNUM) as usize, i, "should be on level {}", i + 1);
        play(&mut m, solution);
        assert_eq!(
            word(&m, BOXLEFT),
            0,
            "level {} should be solved by its scripted solution",
            i + 1
        );
        for _ in 0..140 {
            m.run_frame();
        }
    }

    // After the twelfth level the win panel appears with the totals.
    assert_eq!(vram_text(&m, 0x012C, 8), b"YOU WIN!", "the win panel should show");
    assert_eq!(vram_text(&m, 0x0168, 6), b"LEVELS");
    assert_eq!(vram_text(&m, 0x0171, 2), b"12", "all twelve levels completed");

    // Any key returns to the title for another run.
    tap(&mut m, TiKey::Space);
    for _ in 0..80 {
        m.run_frame();
    }
    assert!(
        count_name(&m, |c| c == 0xB8) > 40,
        "dismissing the win panel returns to the title"
    );
}
