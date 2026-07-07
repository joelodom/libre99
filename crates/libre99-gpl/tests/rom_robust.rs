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

//! **M7 robustness probes** (plan §8 "Robustness probes", the 1981 bar): the
//! console must survive hostile-but-legal use — QUIT/reset storms, a selection
//! menu stressed with a many-entry cartridge, and the VBLANK ISR fed a
//! pathological (never-terminating) sound list. Every probe runs under **both**
//! the authentic console ROM and our rewrite, on the same (our) GROM — the
//! firmware-matrix discipline of `device_io.rs`/`firmware_matrix.rs` — and the
//! outcomes are compared or sanity-asserted. All probes are deterministic
//! (frame-quantized, no wall-clock).
//!
//! The sound-list format and the ISR duty under test are documented at the
//! `ISRNSP` sound section of `original-content/system-roms/rom/console.asm`
//! (authentic `>09EC`): countdown `>83CE` -= SPEED; at zero, stream the next
//! block's N bytes from the `>83CC/D` list (GROM, or VDP per FLAGS `>01` — the
//! GPLWS R14 low byte `>83FD`) to the sound chip, then reload the countdown
//! with the block's duration. An N=0 control block reloads the pointer (a
//! jump); D=0 ends the list.

use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;

static AUTH_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

fn our_rom() -> Vec<u8> {
    libre99_asm::system_rom::build_console_rom().expect("console ROM assembles")
}

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().expect("console GROM assembles")
}

/// Firmware rows: the authentic ROM (oracle) and our rewrite, each on our GROM.
/// `None` when the authentic ROM is absent (the test then skips).
fn firmware() -> Option<Vec<(&'static str, Vec<u8>)>> {
    Some(vec![("TI_ROM", AUTH_ROM.as_deref()?.to_vec()), ("OUR_ROM", our_rom())])
}

/// The visible name table as printable text (identity-mapped ASCII).
fn screen_text(m: &Machine) -> String {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..24 * 32)
        .map(|i| {
            let c = m.vdp().vram(base + i);
            if (0x20..0x7F).contains(&c) {
                c as char
            } else {
                ' '
            }
        })
        .collect()
}

/// The raw name table `>0000-02FF` (both firmwares' title/menu keep it there).
fn name_table(m: &Machine) -> Vec<u8> {
    (0x0000u16..0x0300).map(|a| m.vdp().vram(a)).collect()
}

// ---------------------------------------------------------------------------
// Probe 1 — QUIT/reset storm
// ---------------------------------------------------------------------------

/// Boot to the settled title, then 5 cycles of: hold QUIT (`FCTN`+`=`, detected
/// in the ROM's VBLANK ISR, which soft-resets via `BLWP @>0000`) for 30 frames
/// — long enough that the re-boot re-arms the ISR and QUIT fires *again* mid-
/// hold — then release and give the final re-boot 60 frames. A `>8370` sentinel
/// (top-of-free-VRAM, rewritten to `>3FFF` only by the boot) proves each
/// cycle's QUIT genuinely reset the console rather than the storm being a
/// no-op.
fn quit_storm(name: &str, rom: &[u8], grom: &[u8]) -> Machine {
    let mut m = Machine::new(rom, grom);
    m.reset();
    for _ in 0..150 {
        m.run_frame(); // to the settled title key-wait (ISR armed)
    }
    for cycle in 0..5 {
        m.bus_mut().poke_word(0x8370, 0x1234); // sentinel: only a re-boot rewrites it
        m.set_key(TiKey::Fctn, true);
        m.set_key(TiKey::Equals, true);
        for _ in 0..30 {
            m.run_frame();
        }
        m.set_key(TiKey::Fctn, false);
        m.set_key(TiKey::Equals, false);
        for _ in 0..60 {
            m.run_frame();
        }
        assert_eq!(
            m.bus().peek_word(0x8370),
            0x3FFF,
            "{name}: storm cycle {cycle}: QUIT never reset the console (the >8370 sentinel survived)"
        );
    }
    m
}

/// **Gate: a QUIT/reset storm leaves both ROMs at a freshly painted title, and
/// the same one.** Five hold/release QUIT cycles (each triggering at least one
/// ISR-detected soft reset, several while held), then: the title repainted
/// (nonzero name table, the shared banner) and the two ROMs' name tables match
/// byte-for-byte.
#[test]
fn quit_reset_storm_recovers() {
    let Some(firmware) = firmware() else { skip!() };
    let grom = our_grom();
    let survivors: Vec<(&str, Machine)> = firmware
        .into_iter()
        .map(|(name, rom)| (name, quit_storm(name, &rom, &grom)))
        .collect();

    for (name, m) in &survivors {
        assert!(
            name_table(m).iter().any(|&b| b != 0),
            "{name}: the name table is all zero after the QUIT storm — no title painted"
        );
        assert!(
            screen_text(m).contains("TEXAS INSTRUMENTS"),
            "{name}: no title banner after the QUIT storm; screen:\n{}",
            screen_text(m)
        );
    }
    let (a, o) = (name_table(&survivors[0].1), name_table(&survivors[1].1));
    let diffs: Vec<String> = (0..a.len())
        .filter(|&i| a[i] != o[i])
        .take(8)
        .map(|i| format!(">{:04X} auth={:02X} ours={:02X}", i, a[i], o[i]))
        .collect();
    assert!(
        diffs.is_empty(),
        "post-storm name tables (>0000-02FF) diverge between the ROMs at {diffs:?}"
    );
}

// ---------------------------------------------------------------------------
// Probe 2 — ISR under a pathological (never-terminating) sound list
// ---------------------------------------------------------------------------

/// The self-looping sound list lives in high VRAM, untouched at the bare title
/// (top of free VRAM `>8370` = `>3FFF`, no disk mounted).
const LOOP_LIST: u16 = 0x3F00;

/// What a pathological-sound run left behind: the frames on which `>8379`
/// moved, the total (wrapping) `>8379` advance, and the final sound-list cells.
#[derive(Clone, Copy)]
struct SoundOutcome {
    ticks: u32,
    delta: u8,
    ptr: u16,
    count: u8,
}

/// Arm a degenerate, never-terminating sound list and idle 120 frames. The
/// list: one real block (N=1: `>9F` mute-ch0; D=1) followed by an N=0 control
/// block whose reload pointer targets the list head — so the ISR's sound duty
/// runs one block *every tick, forever* (D never reaches 0). Armed exactly as
/// the working idiom in `libre99-asm/tests/gpl_core.rs` arms poked lists —
/// pointer `>83CC/D`, countdown `>83CE` = 1 — but VDP-resident (the console
/// GROM occupies the low GROM space, so a poked *GROM* list has nowhere to
/// live): the list bytes go to VRAM and FLAGS bit 0 (`>83FD`, the source
/// select that GPL `IO #1` sets) is raised.
fn pathological_sound(name: &str, rom: &[u8], grom: &[u8]) -> SoundOutcome {
    let mut m = Machine::new(rom, grom);
    m.reset();
    for _ in 0..120 {
        m.run_frame();
    }
    assert_eq!(
        m.bus().peek(0x83CE),
        0,
        "{name}: the boot beep's sound list should have drained before arming ours"
    );

    let list = [0x01, 0x9F, 0x01, 0x00, (LOOP_LIST >> 8) as u8, LOOP_LIST as u8];
    for (i, &b) in list.iter().enumerate() {
        m.vdp_mut().set_vram(LOOP_LIST + i as u16, b);
    }
    let flags = m.bus().peek(0x83FD);
    m.bus_mut().poke(0x83FD, flags | 0x01); // FLAGS bit 0: the list is VDP-resident
    m.bus_mut().poke_word(0x83CC, LOOP_LIST); // sound-list pointer
    m.bus_mut().poke(0x83CE, 0x01); // countdown 1 -> process block 0 next tick

    // Positive signal that the duty is really walking our list (not gated off):
    // one frame in, the pointer has advanced past block 0.
    m.run_frame();
    assert_eq!(
        m.bus().peek_word(0x83CC),
        LOOP_LIST + 3,
        "{name}: the ISR sound duty never consumed block 0 of the armed list"
    );

    let before = m.bus().peek(0x8379);
    let mut prev = before;
    let mut ticks = 0u32;
    for _ in 0..120 {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev {
            ticks += 1;
        }
        prev = t;
    }
    SoundOutcome {
        ticks,
        delta: m.bus().peek(0x8379).wrapping_sub(before),
        ptr: m.bus().peek_word(0x83CC),
        count: m.bus().peek(0x83CE),
    }
}

/// **Gate: a never-terminating sound list must not wedge the console.** Under
/// both ROMs the ISR keeps all its duties running (the `>8379` SPEED timer
/// advances every frame) while the loop list cycles forever, and the two ROMs
/// advance the timer by exactly the same amount. The list itself must still be
/// live at the end (countdown nonzero, pointer inside the loop) — the ISR
/// serviced it without ever "finishing" it — and byte-identically so under
/// both ROMs.
#[test]
fn pathological_sound_list_never_wedges() {
    let Some(firmware) = firmware() else { skip!() };
    let grom = our_grom();
    let outcomes: Vec<(&str, SoundOutcome)> = firmware
        .into_iter()
        .map(|(name, rom)| (name, pathological_sound(name, &rom, &grom)))
        .collect();
    let (auth, ours) = (outcomes[0].1, outcomes[1].1);

    for (name, s) in outcomes {
        assert!(
            s.ticks > 100,
            "{name}: the ISR stalled under the looping sound list (>8379 moved on only {}/120 frames)",
            s.ticks
        );
        assert_ne!(
            s.count, 0,
            "{name}: the never-terminating list terminated (>83CE reached 0)"
        );
        assert!(
            s.ptr == LOOP_LIST || s.ptr == LOOP_LIST + 3,
            "{name}: the sound-list pointer escaped the loop (>83CC = >{:04X})",
            s.ptr
        );
    }
    assert_eq!(
        ours.delta, auth.delta,
        "the two ROMs' >8379 SPEED-timer deltas diverged under the looping list (auth {} vs ours {})",
        auth.delta, ours.delta
    );
    assert_eq!(
        (ours.ptr, ours.count),
        (auth.ptr, auth.count),
        "the two ROMs' sound-list cells (>83CC, >83CE) diverged under the looping list"
    );
}

// ---------------------------------------------------------------------------
// Probe 3 — the selection menu under a many-entry cartridge
// ---------------------------------------------------------------------------

/// Walk a GROM page's standard `>AA`-header program chain (the `sweep.rs`
/// census idiom) and count the declared programs.
fn chain_entries(page: &[u8], base: u16) -> usize {
    let read = |a: u16| page.get(a.wrapping_sub(base) as usize).copied().unwrap_or(0);
    if read(base) != 0xAA {
        return 0;
    }
    let mut p = ((read(base + 6) as u16) << 8) | read(base + 7) as u16;
    let mut n = 0;
    while p != 0 && n < 16 {
        n += 1;
        p = ((read(p) as u16) << 8) | read(p + 1) as u16;
    }
    n
}

/// Find a bundled cartridge whose GROM header declares >= 3 programs (checked
/// by walking the chain — nothing fabricated). Preference order: the deepest
/// menus first (`alpiner`/`et`/`mine` declare 7 language entries; `mbgames` 5).
fn many_entry_cart() -> Option<(&'static str, Cartridge, usize)> {
    let candidates = [
        "cartridges/alpiner.ctg",
        "cartridges/et.ctg",
        "cartridges/mine.ctg",
        "cartridges/mbgames.ctg",
        "cartridges/Soccer.ctg",
        "cartridges/VideoGames1.ctg",
        "cartridges/HuntTheWumpus.ctg",
    ];
    for path in candidates {
        let Some(data) = libre99_core::third_party::load(path) else { continue };
        let Ok(cart) = Cartridge::parse(&data) else { continue };
        let declared: usize =
            cart.grom.iter().map(|(base, page)| chain_entries(page, *base)).sum();
        if declared >= 3 {
            return Some((path, cart, declared));
        }
    }
    None
}

/// Count menu entries drawn as `n FOR NAME` (the `sweep.rs` signal).
fn listed_count(m: &Machine) -> usize {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (3..22u16)
        .filter(|r| {
            let s: String = (0..32).map(|i| m.vdp().vram(base + r * 32 + i) as char).collect();
            s.contains(" FOR ")
        })
        .count()
}

/// Boot `rom` with `cart`, leave the title at frame 180 with a SPACE tap, give
/// the menu 60 frames, then run to a settled screen (name table unchanged for
/// 10 consecutive frames — the two ROMs' menu-build *transients* differ by
/// design, ours running the visible `SCANNING` pass, so the comparison is on
/// the settled screen).
fn menu_with(rom: &[u8], grom: &[u8], cart: &Cartridge) -> Machine {
    let mut m = Machine::new(rom, grom);
    m.mount_cartridge(cart);
    m.reset();
    for _ in 0..180 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, true);
    for _ in 0..6 {
        m.run_frame();
    }
    m.set_key(TiKey::Space, false);
    for _ in 0..60 {
        m.run_frame();
    }
    let mut prev = name_table(&m);
    let mut stable = 0;
    for _ in 0..300 {
        if stable >= 10 {
            break;
        }
        m.run_frame();
        let cur = name_table(&m);
        if cur == prev {
            stable += 1;
        } else {
            stable = 0;
        }
        prev = cur;
    }
    assert!(stable >= 10, "menu screen never settled within the frame budget");
    m
}

/// **Gate: the master menu under a many-entry cartridge lists the same screen
/// under both ROMs.** A single machine cannot mount 10 cartridges, so the menu
/// stress is a real bundled cartridge whose GROM header chains many programs
/// (alpiner declares 7 — with the console's built-in entry, 8 of the menu's
/// 9-entry cap; see `menu_cap.rs`). Both ROMs must list >= 3 entries and paint
/// byte-identical settled menus.
#[test]
fn menu_lists_many_entries() {
    let Some(firmware) = firmware() else { skip!() };
    let Some((path, cart, declared)) = many_entry_cart() else {
        eprintln!(
            "menu_lists_many_entries: SKIPPED — no bundled .ctg with >= 3 chained programs found"
        );
        return;
    };
    let grom = our_grom();
    let menus: Vec<Machine> =
        firmware.into_iter().map(|(_, rom)| menu_with(&rom, &grom, &cart)).collect();
    let (auth, ours) = (&menus[0], &menus[1]);

    // Sanity: the stress is real — a many-entry list actually built (under our
    // ROM: the cartridge's programs beside the console's built-in entry).
    let listed = listed_count(ours);
    assert!(
        listed >= 3,
        "{path} declares {declared} programs but our ROM's menu listed only {listed}; screen:\n{}",
        screen_text(ours)
    );
    assert_eq!(
        ours.bus().peek(0x837D),
        0,
        "loud-stub breadcrumb >837D set during the menu build under our ROM"
    );
    assert_eq!(
        screen_text(ours),
        screen_text(auth),
        "{path} ({declared} programs): the settled menu screens diverge between the ROMs"
    );
    eprintln!(
        "menu_lists_many_entries: {path} declares {declared} programs; both ROMs listed {listed} identical entries"
    );
}
