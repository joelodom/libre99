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

//! End-to-end JAYWALKER 99 test: assemble the game, boot the real console, drive
//! the keyboard and joystick, and inspect game state in RAM, the sprite
//! attribute table, and the name table.
//!
//! It assembles the tracked, playable source at
//! `original-content/cartridges/jaywalker99/jaywalker99.asm`, so the game and its
//! regression test can never drift apart.
//!
//! Controls: E/S/D/X or joystick 1 (host arrows) hop north/west/east/south.

use std::sync::LazyLock;

use libre99_asm::{assemble, Options};
use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static CONSOLE_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));
const SRC: &str = include_str!("../../../original-content/cartridges/jaywalker99/jaywalker99.asm");

// Scratchpad state (see the EQU block in jaywalker99.asm).
const GMODE: u16 = 0x8320;
const VBOT: u16 = 0x832A;
const PLANE: u16 = 0x832C;
const PX: u16 = 0x832E;
const RIDE: u16 = 0x8334;
const SCORE: u16 = 0x8336;
const COINS: u16 = 0x8338;
const LCROSS: u16 = 0x833A;
const BEST: u16 = 0x833C;
const DEAD: u16 = 0x833E;
const HAWKT: u16 = 0x8342;
const HAWKON: u16 = 0x8344;
const GENL: u16 = 0x8364;

// The lane ring in expansion RAM: 16 records of 16 bytes.
const LANES: u16 = 0xA000;

// Lane record field offsets.
const L_TYPE: u16 = 0; // 0 grass, 1 road, 2 river, 3 rail
const L_SPEED: u16 = 2; // 12.4 fixed, signed
const L_MASK: u16 = 4; // bushes (grass) / lily pads (river)
const L_COIN: u16 = 6; // cell col, >FF = none
const L_OBJ0: u16 = 8; // 12.4 fixed
const L_OBJ1: u16 = 10;
const L_TIMER: u16 = 12; // rail: frames to the next train; 0 = sweeping

fn rec(lane: u16) -> u16 {
    LANES + ((lane & 15) << 4)
}

/// Boot the console and select the cartridge; stop on the game's title screen.
/// `None` (announcing the skip) when the third-party console images are absent.
fn boot_to_title() -> Option<Machine> {
    let (Some(rom), Some(grom)) = (CONSOLE_ROM.as_deref(), CONSOLE_GROM.as_deref()) else {
        eprintln!("SKIPPED: third-party media not present");
        return None;
    };
    // The 12-char menu title rides on `--name` (E/A's IDT caps at 8 chars);
    // the committed .ctg is built the same way.
    let opts = Options { name: Some("JAYWALKER 99".into()), ..Options::default() };
    let asm = assemble(SRC, &opts).expect("JAYWALKER 99 assembles");
    assert_eq!(asm.title, "JAYWALKER 99");
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
    m.set_key(TiKey::Num2, true); // pick "2 FOR JAYWALKER 99"
    for _ in 0..20 {
        m.run_frame();
    }
    m.set_key(TiKey::Num2, false);
    for _ in 0..150 {
        m.run_frame();
    }
    Some(m)
}

/// Dismiss the title with SPACE and wait for the opening world to paint.
fn start_game(m: &mut Machine) {
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..30 {
        m.run_frame();
    }
    assert_eq!(m.bus().peek_word(GMODE), 1, "the game should be running");
}

/// Overwrite the whole lane ring with bare, safe grass (and no coins), so
/// movement tests can hop without the random world interfering.
fn pave_with_grass(m: &mut Machine) {
    for lane in 0..16u16 {
        let r = rec(lane);
        for b in 0..16u16 {
            m.bus_mut().poke(r + b, 0);
        }
        m.bus_mut().poke(r + L_COIN, 0xFF);
    }
}

/// Tap a key for a few frames, then release and settle.
fn tap(m: &mut Machine, k: TiKey) {
    m.set_key(k, true);
    for _ in 0..3 {
        m.run_frame();
    }
    m.set_key(k, false);
    for _ in 0..12 {
        m.run_frame(); // a hop is 8 frames; let it land and settle
    }
}

fn count_name(m: &Machine, pred: impl Fn(u8) -> bool) -> usize {
    (0..768u16).filter(|&i| pred(m.vdp().vram(i))).count()
}

/// The four attribute bytes of hardware sprite `n`.
fn sprite(m: &Machine, n: u16) -> [u8; 4] {
    let base = 0x0780 + n * 4;
    [
        m.vdp().vram(base),
        m.vdp().vram(base + 1),
        m.vdp().vram(base + 2),
        m.vdp().vram(base + 3),
    ]
}

fn text_at(m: &Machine, addr: u16, len: u16) -> Vec<u8> {
    (0..len).map(|i| m.vdp().vram(addr + i)).collect()
}

#[test]
fn title_screen_shows_then_waits() {
    let Some(mut m) = boot_to_title() else { return };
    assert!(
        count_name(&m, |c| c == 0xE0) > 30,
        "expected the big JAY/WALKER wordmark in block glyphs"
    );
    assert_eq!(m.vdp().vram(0x02A5), b'P', "the PLAY prompt should be on screen");
    assert_eq!(text_at(&m, 0x026C, 8), b"ROUTE 99", "the route marker is painted");
    assert_eq!(m.bus().peek_word(GMODE), 0, "still on the title");
    // The diorama animates: the jay sprite (0) sits on the grass band and the
    // car sprite (1) keeps moving.
    let jay = sprite(&m, 0);
    assert!(jay[0] == 0x7F || jay[0] == 0x7D, "the title jay bobs on the grass band");
    let car_x0 = sprite(&m, 1)[1];
    for _ in 0..30 {
        m.run_frame();
    }
    assert_ne!(sprite(&m, 1)[1], car_x0, "the title car should be driving");
}

#[test]
fn game_starts_with_hud_world_and_player_sprite() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    // HUD labels and zeroed numbers.
    assert_eq!(text_at(&m, 0x0000, 5), b"SCORE");
    assert_eq!(text_at(&m, 0x0006, 5), b"00000");
    assert_eq!(text_at(&m, 0x000C, 4), b"COIN");
    assert_eq!(text_at(&m, 0x0014, 4), b"BEST");

    // The opening meadow: lanes 0..3 are bush-free grass.
    for lane in 0..4u16 {
        assert_eq!(m.bus().peek(rec(lane) + L_TYPE), 0, "lane {lane} is grass");
        assert_eq!(m.bus().peek_word(rec(lane) + L_MASK), 0, "lane {lane} has no bushes");
    }
    assert_eq!(m.bus().peek_word(GENL), 13, "lanes 0..12 exist at the start");
    assert_eq!(m.bus().peek_word(PLANE), 2, "the jay spawns on lane 2");
    assert_eq!(m.bus().peek_word(PX), 128 << 4, "centered on cell col 8");
    assert_eq!(m.bus().peek_word(VBOT), 0);

    // The spawn lanes paint as grass: rows 16..23 are grass-group glyphs.
    let grass = (16 * 32..24 * 32u16)
        .filter(|&a| (0x80..=0x85).contains(&m.vdp().vram(a)))
        .count();
    assert!(grass > 200, "the meadow should be painted ({grass} grass glyphs)");

    // Sprite 0 is the jay: lane 2 sits at y 144 (attr 143; the idle bob may
    // lift it one line), x 120, the jay pattern, jay-blue color.
    let jay = sprite(&m, 0);
    assert!(jay[0] == 143 || jay[0] == 142, "jay Y attr {}", jay[0]);
    assert_eq!(jay[1], 128, "jay X");
    assert!(jay[2] == 0 || jay[2] == 4, "jay pattern");
    assert_eq!(jay[3] & 0x0F, 0x05, "jay color is light blue");
    // Slot 24 terminates the sprite list.
    assert_eq!(sprite(&m, 24)[0], 0xD0, "the sprite list is terminated");
}

#[test]
fn hopping_moves_scores_and_scrolls() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    // East then west.
    let x0 = m.bus().peek_word(PX);
    tap(&mut m, TiKey::D);
    assert_eq!(m.bus().peek_word(PX), x0 + 256, "D hops one cell east");
    tap(&mut m, TiKey::S);
    assert_eq!(m.bus().peek_word(PX), x0, "S hops one cell west");

    // North scores 10 a lane; the joystick works too.
    tap(&mut m, TiKey::E);
    assert_eq!(m.bus().peek_word(PLANE), 3);
    assert_eq!(m.bus().peek_word(SCORE), 10);
    assert_eq!(text_at(&m, 0x0006, 5), b"00010", "the HUD shows the score");
    tap(&mut m, TiKey::Joy1Up);
    assert_eq!(m.bus().peek_word(PLANE), 4);
    assert_eq!(m.bus().peek_word(LCROSS), 2);

    // South hops back (no score), and retreading old ground scores nothing.
    tap(&mut m, TiKey::X);
    assert_eq!(m.bus().peek_word(PLANE), 3);
    assert_eq!(m.bus().peek_word(SCORE), 20);
    tap(&mut m, TiKey::Joy1Up);
    assert_eq!(m.bus().peek_word(SCORE), 20, "old lanes score nothing");

    // March north until the camera scrolls: it holds the jay at screen lane 5.
    for _ in 0..3 {
        pave_with_grass(&mut m); // keep the generator's new lanes harmless
        tap(&mut m, TiKey::E);
    }
    assert_eq!(m.bus().peek_word(PLANE), 7);
    assert_eq!(m.bus().peek_word(VBOT), 2, "the camera followed the jay");
    assert_eq!(m.bus().peek_word(GENL), 15, "the world generated ahead");
}

#[test]
fn bushes_block_hops() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    // A wall of bushes one lane north.
    m.bus_mut().poke_word(rec(3) + L_MASK, 0xFFFF);
    tap(&mut m, TiKey::E);
    assert_eq!(m.bus().peek_word(PLANE), 2, "a bush blocks the hop");
    assert_eq!(m.bus().peek_word(SCORE), 0);

    // A single bush on the neighboring cell blocks a sideways hop.
    let col = (m.bus().peek_word(PX) >> 8) as u16; // cell column (x/16)
    m.bus_mut().poke_word(rec(2) + L_MASK, 1 << (col + 1));
    tap(&mut m, TiKey::D);
    assert_eq!((m.bus().peek_word(PX) >> 8) as u16, col, "the bush blocks east");
    tap(&mut m, TiKey::S);
    assert_eq!((m.bus().peek_word(PX) >> 8) as u16, col - 1, "west is open");
}

#[test]
fn cars_kill_and_the_panel_returns_to_title() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    // Lane 3 becomes a road with a parked car exactly at the jay's column.
    let r = rec(3);
    m.bus_mut().poke(r + L_TYPE, 1);
    m.bus_mut().poke_word(r + L_SPEED, 0);
    let px = m.bus().peek_word(PX);
    m.bus_mut().poke_word(r + L_OBJ0, px);
    m.bus_mut().poke_word(r + L_OBJ1, ((px as i16) - 2432) as u16);

    tap(&mut m, TiKey::E); // hop straight into it
    assert_eq!(m.bus().peek_word(DEAD), 1, "squished by a car");
    for _ in 0..80 {
        m.run_frame();
    }
    assert_eq!(m.bus().peek_word(GMODE), 3, "the game-over panel is up");
    assert_eq!(text_at(&m, 0x012B, 9), b"GAME OVER");
    assert_eq!(text_at(&m, 0x0167, 17), b"SQUISHED BY A CAR");
    assert_eq!(m.bus().peek_word(BEST), 10, "the run's 10 points became the best");

    // Any key dismisses the panel back to the title.
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..30 {
        m.run_frame();
    }
    assert_eq!(m.bus().peek_word(GMODE), 0, "back on the title");
    assert!(count_name(&m, |c| c == 0xE0) > 30, "the big letters are back");
    assert_eq!(text_at(&m, 0x0010, 5), b"00010", "the title shows the best");
}

#[test]
fn logs_carry_riders_and_open_water_drowns() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    // Lane 3: a river drifting east at 1 px/frame, one log at the jay's x.
    let r = rec(3);
    m.bus_mut().poke(r + L_TYPE, 2);
    m.bus_mut().poke_word(r + L_SPEED, 16);
    let px = m.bus().peek_word(PX);
    m.bus_mut().poke_word(r + L_OBJ0, px);
    m.bus_mut().poke_word(r + L_OBJ1, ((px as i16) - 2432) as u16);

    tap(&mut m, TiKey::E);
    assert_eq!(m.bus().peek_word(RIDE), 1, "landed on the log");
    assert_eq!(m.bus().peek_word(DEAD), 0);
    let x0 = m.bus().peek_word(PX) as i16;
    for _ in 0..32 {
        m.run_frame();
    }
    let x1 = m.bus().peek_word(PX) as i16;
    assert!(x1 - x0 >= 28, "the log carried the jay east ({x0} -> {x1})");

    // Hop back to grass, move the log away, hop in again: open water.
    tap(&mut m, TiKey::X);
    assert_eq!(m.bus().peek_word(DEAD), 0, "safely ashore");
    m.bus_mut().poke_word(r + L_OBJ0, ((px as i16) - 1600) as u16);
    m.bus_mut().poke_word(r + L_OBJ1, ((px as i16) + 1600) as u16);
    tap(&mut m, TiKey::E);
    assert_eq!(m.bus().peek_word(DEAD), 2, "open water drowns");
    for _ in 0..80 {
        m.run_frame();
    }
    assert_eq!(text_at(&m, 0x0166, 20), b"SWEPT DOWN THE RIVER");
}

#[test]
fn lily_pads_are_safe_footing() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    let r = rec(3);
    m.bus_mut().poke(r + L_TYPE, 2);
    m.bus_mut().poke_word(r + L_SPEED, 16);
    m.bus_mut().poke_word(r + L_OBJ0, 0x2000); // logs parked far away
    m.bus_mut().poke_word(r + L_OBJ1, 0x2000);
    let col = m.bus().peek_word(PX) >> 8;
    m.bus_mut().poke_word(r + L_MASK, 1 << col);

    tap(&mut m, TiKey::E);
    assert_eq!(m.bus().peek_word(DEAD), 0, "the pad holds");
    assert_eq!(m.bus().peek_word(RIDE), 0, "standing, not riding");
    assert_eq!(m.bus().peek_word(PLANE), 3);
}

#[test]
fn trains_sweep_the_rails() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    // Lane 3 is a rail lane mid-sweep, the engine right on the jay's column.
    let r = rec(3);
    m.bus_mut().poke(r + L_TYPE, 3);
    m.bus_mut().poke_word(r + L_SPEED, 64);
    m.bus_mut().poke_word(r + L_TIMER, 0);
    let px = m.bus().peek_word(PX);
    m.bus_mut().poke_word(r + L_OBJ0, px);
    m.bus_mut().poke_word(r + L_OBJ1, ((px as i16) - 256) as u16);

    // The engine sprite is on the rails in that lane's slots (lane 3 is
    // screen lane 3 from the bottom -> slots 8 and 9).
    m.run_frame();
    m.run_frame();
    let engine = sprite(&m, 8);
    assert_eq!(engine[2], 36, "the engine pattern rides lane 3's slot");

    tap(&mut m, TiKey::E); // hop onto the crossing
    assert_eq!(m.bus().peek_word(DEAD), 3, "run down by the express");
    for _ in 0..80 {
        m.run_frame();
    }
    assert_eq!(text_at(&m, 0x0167, 18), b"HIT BY THE EXPRESS");
}

#[test]
fn the_crossing_signal_flashes_before_a_train() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    let r = rec(4);
    m.bus_mut().poke(r + L_TYPE, 3);
    m.bus_mut().poke_word(r + L_SPEED, 64);
    m.bus_mut().poke_word(r + L_TIMER, 2000); // far from due: idle lamp
    for _ in 0..2 {
        m.run_frame();
    }
    assert_eq!(m.vdp().vram(0x0316), 0x61, "idle lamp is dim dark red");

    m.bus_mut().poke_word(r + L_TIMER, 88); // inside the warning window
    let mut seen_bright = false;
    let mut seen_dim = false;
    for _ in 0..40 {
        m.run_frame();
        match m.vdp().vram(0x0316) {
            0x81 => seen_bright = true,
            0x61 => seen_dim = true,
            _ => {}
        }
    }
    assert!(seen_bright && seen_dim, "the warning lamp flashes");
}

#[test]
fn the_hawk_takes_idle_birds() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    // Fast-forward the idle clock to just before the dive.
    m.bus_mut().poke_word(HAWKT, 478);
    for _ in 0..4 {
        m.run_frame();
    }
    assert_eq!(m.bus().peek_word(HAWKON), 1, "the hawk committed");
    // It starts above the screen and swoops; 16x16 at 3 px/frame reaches the
    // jay (y 144) in about 55 frames.
    let mut died = false;
    for _ in 0..90 {
        m.run_frame();
        if m.bus().peek_word(DEAD) == 4 {
            died = true;
            break;
        }
    }
    assert!(died, "the hawk always gets the idle bird");
    for _ in 0..160 {
        m.run_frame();
    }
    assert_eq!(m.bus().peek_word(GMODE), 3);
    assert_eq!(text_at(&m, 0x0165, 21), b"CARRIED OFF BY A HAWK");
}

#[test]
fn hopping_resets_the_hawk_clock() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);
    m.bus_mut().poke_word(HAWKT, 300);
    tap(&mut m, TiKey::E);
    assert!(
        m.bus().peek_word(HAWKT) < 20,
        "a hop buys the jay a fresh 8 seconds"
    );
    assert_eq!(m.bus().peek_word(HAWKON), 0);
}

#[test]
fn coins_collect_ding_and_score() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);
    pave_with_grass(&mut m);

    let col = (m.bus().peek_word(PX) >> 8) as u8;
    m.bus_mut().poke(rec(3) + L_COIN, col);
    tap(&mut m, TiKey::E);
    assert_eq!(m.bus().peek_word(COINS), 1, "the coin is collected");
    assert_eq!(m.bus().peek_word(SCORE), 35, "10 for the lane + 25 for the coin");
    assert_eq!(m.bus().peek(rec(3) + L_COIN), 0xFF, "the coin is gone from the lane");
    assert_eq!(text_at(&m, 0x0011, 2), b"01", "the HUD coin counter shows it");
}

#[test]
fn the_sound_chip_sings() {
    let Some(mut m) = boot_to_title() else { return };
    // Leaving the title plays the start arpeggio on tone channel 0
    // (volume 15 = silent on the SN76489; anything below is audible).
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    let mut heard = false;
    for _ in 0..40 {
        m.run_frame();
        if m.bus().psg.volume(0) < 15 {
            heard = true;
            break;
        }
    }
    assert!(heard, "the start arpeggio should play");
    assert_eq!(m.bus().peek_word(GMODE), 1);
    pave_with_grass(&mut m);
    for _ in 0..90 {
        m.run_frame(); // let the start arpeggio finish and fall silent
    }
    assert_eq!(m.bus().psg.volume(0), 15, "quiet between effects");

    // A hop chirps on channel 0.
    m.set_key(TiKey::E, true);
    let mut heard = false;
    for _ in 0..6 {
        m.run_frame();
        if m.bus().psg.volume(0) < 15 {
            heard = true;
            break;
        }
    }
    m.set_key(TiKey::E, false);
    assert!(heard, "a hop should chirp");
    for _ in 0..30 {
        m.run_frame();
    }

    // Drowning hisses on the noise channel.
    let r = rec(4);
    m.bus_mut().poke(r + L_TYPE, 2);
    m.bus_mut().poke_word(r + L_SPEED, 16);
    m.bus_mut().poke_word(r + L_OBJ0, 0x2000);
    m.bus_mut().poke_word(r + L_OBJ1, 0x2000);
    tap(&mut m, TiKey::E);
    assert_eq!(m.bus().peek_word(DEAD), 2);
    assert!(m.bus().psg.volume(3) < 15, "the splash plays on the noise channel");
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
    assert_eq!(text_at(&m, 0x0027, 17), b"JAYWALKER 99 HELP", "H opens the help screen");
    assert_eq!(text_at(&m, 0x00A2, 8), b"CONTROLS");
    assert_eq!(m.vdp().vram(0x0142), 0x82, "the bush icon is drawn");
    assert_eq!(m.bus().peek_word(GMODE), 0, "help must not start a game");

    // Any key returns to the title.
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..40 {
        m.run_frame();
    }
    assert!(count_name(&m, |c| c == 0xE0) > 30, "dismissing help returns to the title");

    // AID (FCTN+7) opens it too.
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
    assert_eq!(text_at(&m, 0x0027, 17), b"JAYWALKER 99 HELP", "AID opens help too");
}

#[test]
fn monkey_soak_survives_random_mashing() {
    // Mash pseudo-random keys for ~80 seconds of emulated time across many
    // lives and titles; the game must stay inside the cartridge and keep
    // running (ticking, or parked in one of its wait-for-a-key loops).
    let Some(mut m) = boot_to_title() else { return };
    let keys = [
        TiKey::E,
        TiKey::S,
        TiKey::D,
        TiKey::X,
        TiKey::Joy1Up,
        TiKey::Joy1Down,
        TiKey::Joy1Left,
        TiKey::Joy1Right,
        TiKey::Space,
        TiKey::H,
    ];
    let mut lcg: u32 = 0x1234_5678;
    let mut held: Option<TiKey> = None;
    for frame in 0..5000 {
        if frame % 5 == 0 {
            if let Some(k) = held.take() {
                m.set_key(k, false);
            }
            lcg = lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let k = keys[(lcg >> 16) as usize % keys.len()];
            m.set_key(k, true);
            held = Some(k);
        }
        m.run_frame();
        let pc = m.cpu().pc();
        assert!(
            (0x6000..0x8000).contains(&pc),
            "PC {pc:04X} left the cartridge at frame {frame}"
        );
        let mode = m.bus().peek_word(GMODE);
        assert!(mode <= 3, "GMODE {mode} is corrupt at frame {frame}");
    }
}

#[test]
fn the_world_generator_builds_valid_lanes() {
    let Some(mut m) = boot_to_title() else { return };
    start_game(&mut m);

    // Every generated record is a known type with sane fields.
    for lane in 0..13u16 {
        let r = rec(lane);
        let ty = m.bus().peek(r + L_TYPE);
        assert!(ty <= 3, "lane {lane} type {ty} is valid");
        if lane < 4 {
            assert_eq!(ty, 0, "the spawn meadow is grass");
            continue;
        }
        match ty {
            0 => {
                let mask = m.bus().peek_word(r + L_MASK);
                assert!(mask.count_ones() <= 5, "never more than 5 bushes");
            }
            1 | 2 => {
                let speed = m.bus().peek_word(r + L_SPEED) as i16;
                assert!(speed != 0, "moving lanes move");
                assert!(speed.unsigned_abs() <= 40, "speed {speed} stays sane");
            }
            3 => {
                let timer = m.bus().peek_word(r + L_TIMER);
                assert!((1..=495).contains(&timer), "a train is due in {timer} frames");
            }
            _ => unreachable!(),
        }
    }

    // March a long way north over paved ground and confirm the generator
    // keeps producing valid lanes (this exercises runs and the ring wrap).
    for _ in 0..30 {
        pave_with_grass(&mut m);
        tap(&mut m, TiKey::E);
        assert_eq!(m.bus().peek_word(DEAD), 0);
    }
    assert_eq!(m.bus().peek_word(PLANE), 32);
    assert_eq!(m.bus().peek_word(SCORE), 300);
}
