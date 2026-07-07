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

//! End-to-end Titris test: assemble the game, boot the real console, drive input
//! (keyboard and joystick), and inspect game state in RAM and on screen.
//!
//! It assembles the tracked, playable source at
//! `original-content/cartridges/titris/titris.asm`, so the game and its
//! regression test can never drift apart.
//!
//! Controls (after the conventional/SRS remap): arrows (TI joystick 1) move/
//! soft-drop/rotate-CW; X = rotate CW, Z = rotate CCW, SPACE = hard drop.

use std::sync::LazyLock;

use libre99_asm::{assemble, Options};
use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));
const SRC: &str = include_str!("../../../original-content/cartridges/titris/titris.asm");

const CURTYP: u16 = 0x8320;
const CURROT: u16 = 0x8322;
const CURX: u16 = 0x8324;
const CURY: u16 = 0x8326;
const TICK: u16 = 0x832E;
const SCORE: u16 = 0x8336;
const CURW: u16 = 0x8344;
const LEVEL: u16 = 0x8346;
const NEXTAT: u16 = 0x8354;
const FANCNT: u16 = 0x8356;

// Well geometry (see titris.asm): MAXW=20 wide, HEIGHT=20 tall, board stride 32.
const MAXW: u16 = 20;
const HEIGHT: u16 = 20;

/// Boot the console and select the cartridge; stop on the game's title screen.
/// `None` (announcing the skip) when the third-party console images are absent.
fn boot_to_title() -> Option<Machine> {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return None;
    };
    let asm = assemble(SRC, &Options::default()).expect("Titris assembles");
    assert_eq!(asm.title, "TITRIS");
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
    m.set_key(TiKey::Num2, true); // pick "2 FOR TITRIS"
    for _ in 0..20 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, false);
    for _ in 0..150 {
        m.run_frame();
    }
    Some(m)
}

/// Dismiss the title with SPACE — which is also the in-game hard-drop, so this
/// exercises that the starting keystroke is consumed, not passed to gameplay.
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

fn i16at(m: &Machine, a: u16) -> i16 {
    m.bus().peek_word(a) as i16
}

fn count_name(m: &Machine, pred: impl Fn(u8) -> bool) -> usize {
    (0..768u16).filter(|&i| pred(m.vdp().vram(i))).count()
}

fn board_filled(m: &Machine) -> usize {
    let mut n = 0;
    for r in 0..HEIGHT {
        for c in 0..MAXW {
            // Board cell stride is 32 (cell = base + (row<<5) + col).
            if m.bus().peek(0xA000 + (r << 5) + c) != 0 {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn title_screen_shows_then_waits() {
    let Some(m) = boot_to_title() else { return };
    assert!(
        count_name(&m, |c| c == 0x88) > 30,
        "expected the big TITRIS title in block glyphs"
    );
    assert_eq!(m.vdp().vram(0x01C5), b'P', "prompt text should be on screen");
    assert_eq!(m.bus().peek_word(TICK), 0, "game loop should not be ticking yet");
}

#[test]
fn game_has_u_shaped_border_score_and_preview() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    // The well is a U: side walls and a bottom, but no ceiling.
    assert_eq!(m.vdp().vram(0x0040), 0x80, "left wall, top (row 2, col 0)");
    assert_eq!(m.vdp().vram(0x02A0), 0x80, "left wall, bottom (row 21, col 0)");
    assert_eq!(m.vdp().vram(0x0055), 0x80, "right wall at full width (row 2, col 21)");
    assert_eq!(m.vdp().vram(0x02C0), 0x80, "bottom-left corner (row 22, col 0)");
    assert_eq!(m.vdp().vram(0x02D5), 0x80, "bottom-right at full width (row 22, col 21)");
    // No top line: the row above the well (row 1) has no wall glyphs.
    let ceiling = (0x0020..0x0040u16).filter(|&a| m.vdp().vram(a) == 0x80).count();
    assert_eq!(ceiling, 0, "the well must be open at the top (no ceiling)");

    assert_eq!(m.bus().peek_word(CURW), MAXW, "the well starts at full width");
    assert_eq!(m.bus().peek_word(LEVEL), 0, "the game starts at level 0");

    // Empty play-area cells render the faint column-guide glyph (char >C0).
    let guides = (0..768u16).filter(|&a| m.vdp().vram(a) == 0xC0).count();
    assert!(guides > 300, "the empty well should be filled with column guides");

    // SCORE reads 00000 (label row 6, value row 7, col 23).
    assert_eq!(m.vdp().vram(0x00D7), b'S', "the SCORE label should be drawn");
    for i in 0..5 {
        assert_eq!(m.vdp().vram(0x00F7 + i), b'0', "score digit {i} should be '0'");
    }
    // "NEXT LEVEL AT" readout (value at row 13): the next level is reached at 1000.
    let nextat: Vec<u8> = (0..5).map(|i| m.vdp().vram(0x01B7 + i)).collect();
    assert_eq!(&nextat, b"01000", "next-level-at shows the first threshold at start");
    // LEVEL is shown as two digits (label row 16, value row 17).
    let level: Vec<u8> = (0..2).map(|i| m.vdp().vram(0x0237 + i)).collect();
    assert_eq!(&level, b"00", "level is shown as two digits");

    // The next-piece preview (4 cells) sits at the top, rows 1..4, cols 23..26.
    let mut preview = 0;
    for r in 1..5u16 {
        for c in 23..27u16 {
            if (0x88..=0xBF).contains(&m.vdp().vram(r * 32 + c)) {
                preview += 1;
            }
        }
    }
    assert_eq!(preview, 4, "the preview should show the next tetromino (4 cells)");
}

#[test]
fn gameplay_moves_rotates_falls_and_locks() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    assert!(m.bus().peek_word(CURTYP) <= 6, "a piece should have spawned (type 0..6)");
    assert!(m.bus().peek_word(TICK) > 0, "the game loop should be ticking");

    // Move left/right with the arrows (TI joystick 1).
    let x0 = i16at(&m, CURX);
    tap(&mut m, TiKey::Joy1Left);
    let x1 = i16at(&m, CURX);
    assert_eq!(x1, x0 - 1, "arrow-left should move the piece left");
    tap(&mut m, TiKey::Joy1Right);
    assert_eq!(i16at(&m, CURX), x1 + 1, "arrow-right should move the piece right");

    // Rotate CW (X) advances the state; CCW (Z) reverses it.
    let r0 = m.bus().peek_word(CURROT);
    tap(&mut m, TiKey::X);
    assert_eq!(m.bus().peek_word(CURROT), (r0 + 1) & 3, "X should rotate clockwise");
    let r1 = m.bus().peek_word(CURROT);
    tap(&mut m, TiKey::Z);
    assert_eq!(m.bus().peek_word(CURROT), (r1 + 3) & 3, "Z should rotate counter-clockwise");
    // Up-arrow is a second clockwise rotate.
    let r2 = m.bus().peek_word(CURROT);
    tap(&mut m, TiKey::Joy1Up);
    assert_eq!(m.bus().peek_word(CURROT), (r2 + 1) & 3, "arrow-up should rotate clockwise");

    // Gravity advances the piece.
    let y0 = i16at(&m, CURY);
    for _ in 0..40 {
        m.run_frame();
    }
    assert_ne!(i16at(&m, CURY), y0, "gravity should advance the piece");

    for _ in 0..600 {
        m.run_frame();
    }
    assert!((0x601C..0x8000).contains(&m.cpu().pc()), "PC stays in the cartridge");
    assert!(board_filled(&m) > 0, "at least one piece should have locked into the board");
}

#[test]
fn srs_wall_kick_off_the_left_wall() {
    // Deterministic RNG seed -> the first piece is L. In state R it sits with its
    // box flush at column -1 against the left wall; rotating CW would push a cell
    // off the wall, so SRS kicks the piece one column right (JLSTZ 1->2 test (1,0)).
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    assert_eq!(m.bus().peek_word(CURTYP), 6, "first piece should be L for this seed");

    tap(&mut m, TiKey::X); // 0 -> R
    assert_eq!(m.bus().peek_word(CURROT), 1);
    // The wider well spawns the piece nearer the middle, so tap left generously;
    // COLLIDE clamps it at the wall, so the extra taps are harmless no-ops.
    for _ in 0..14 {
        tap(&mut m, TiKey::Joy1Left);
    }
    assert_eq!(i16at(&m, CURX), -1, "L in state R rests at box col -1 against the wall");

    tap(&mut m, TiKey::X); // R -> 2, needs a kick to fit
    assert_eq!(m.bus().peek_word(CURROT), 2, "the rotation should succeed via a wall kick");
    assert_eq!(i16at(&m, CURX), 0, "the wall kick shifts the piece one column right");
}

#[test]
fn topping_out_shows_the_game_over_overlay_then_returns_to_title() {
    const GMOVER: u16 = 0x8330;
    const NPIECE: u16 = 0x8340;
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    // Pile pieces with soft-drop (Down-arrow held, no sideways moves) until top-out.
    // The well is taller now, so allow more frames to fill the spawn columns.
    m.set_key(TiKey::Joy1Down, true);
    let mut topped = false;
    for _ in 0..2400 {
        m.run_frame();
        if m.bus().peek_word(GMOVER) != 0 {
            topped = true;
            break;
        }
    }
    m.set_key(TiKey::Joy1Down, false);
    for _ in 0..30 {
        m.run_frame();
    }
    assert!(topped, "the stack should have topped out");

    let heading: Vec<u8> = (0..9).map(|i| m.vdp().vram(0x0106 + i)).collect();
    assert_eq!(&heading, b"GAME OVER", "the game-over heading should be drawn");
    assert!(m.bus().peek_word(NPIECE) > 0, "the pieces-placed stat should be > 0");

    // Any key dismisses it and returns to the title screen.
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..60 {
        m.run_frame();
    }
    assert!(
        count_name(&m, |c| c == 0x88) > 30,
        "dismissing the overlay should return to the title screen"
    );
}

#[test]
fn starting_keystroke_is_not_passed_to_gameplay() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m); // dismisses the title with SPACE (the in-game hard-drop)
    assert_eq!(board_filled(&m), 0, "the starting keystroke must not hard-drop the piece");
    assert!(
        i16at(&m, CURY) < 5,
        "the first piece should still be near the top"
    );
}

#[test]
fn scoring_levels_up_and_shrinks_the_well() {
    // Levels are score-driven (LEVTHR = 1000 points each), and each level narrows
    // the well by one column from the right. Four levels' worth of points (4000)
    // shrinks the well from 20 to 16 the next time a piece locks, and marches the
    // right wall inward.
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    assert_eq!(m.bus().peek_word(CURW), MAXW, "the well starts at full width");

    // Force the score; the next lock recomputes the level and width from it.
    m.bus_mut().poke_word(SCORE, 4000);

    // Soft-drop until a piece locks and the well shrinks (bounded wait).
    m.set_key(TiKey::Joy1Down, true);
    let mut shrank = false;
    for _ in 0..400 {
        m.run_frame();
        if m.bus().peek_word(CURW) != MAXW {
            shrank = true;
            break;
        }
    }
    m.set_key(TiKey::Joy1Down, false);
    assert!(shrank, "crossing a score threshold should shrink the well");

    // UPDLVL arms the fanfare before it shrinks the well, so the counter is already
    // set the moment the width change is observed.
    assert!(m.bus().peek_word(FANCNT) > 0, "a level-up should kick off the fanfare");

    // The level-up repaint is slow and can span a couple of frames; let it settle
    // before inspecting the wall it draws.
    for _ in 0..5 {
        m.run_frame();
    }
    assert_eq!(m.bus().peek_word(LEVEL), 4, "4000 / 1000 = level 4");
    assert_eq!(m.bus().peek_word(CURW), 16, "level 4 -> width 20 - 4 = 16");
    assert_eq!(m.bus().peek_word(NEXTAT), 5000, "next level (5) is reached at 5*1000");

    // The right wall has marched inward to col 17 (= 1 + width 16); the old wall
    // column (21) is now open background.
    assert_eq!(m.vdp().vram(0x0051), 0x80, "right wall now at row 2, col 17");
    assert_eq!(m.vdp().vram(0x0055), 0x20, "the old right wall (col 21) is cleared");
}

#[test]
fn help_screen_opens_with_h_and_aid() {
    let Some(mut m) = boot_to_title() else { return };

    // H at the title opens help (scoring + controls) without starting the game.
    m.set_key(TiKey::H, true);
    for _ in 0..10 {
        m.run_frame();
    }
    m.set_key(TiKey::H, false);
    for _ in 0..20 {
        m.run_frame();
    }
    let scoring: Vec<u8> = (0..7).map(|i| m.vdp().vram(0x00A4 + i)).collect();
    assert_eq!(&scoring, b"SCORING", "H should open the help screen");
    let controls: Vec<u8> = (0..8).map(|i| m.vdp().vram(0x0184 + i)).collect();
    assert_eq!(&controls, b"CONTROLS", "help should list the controls");
    assert_eq!(m.vdp().vram(0x006C), 0x88, "help should show the colored accent bar");
    assert_eq!(m.bus().peek_word(TICK), 0, "opening help must not start the game");

    // Any key returns to the title.
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..60 {
        m.run_frame();
    }
    assert!(count_name(&m, |c| c == 0x88) > 30, "dismissing help returns to the title");

    // AID (FCTN+7) opens it too — and holding FCTN must not start the game.
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
    let scoring: Vec<u8> = (0..7).map(|i| m.vdp().vram(0x00A4 + i)).collect();
    assert_eq!(&scoring, b"SCORING", "AID (FCTN+7) should open help too");
    assert_eq!(m.bus().peek_word(TICK), 0, "AID must not start the game");
}
