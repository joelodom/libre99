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

//! **Chunk 2 A2 — the cartridge coverage sweep.** For every bundled cartridge,
//! boot → menu → launch → attract (+ an F5 warm-reset relaunch leg), recording
//! every console-GROM-0 (`>0000-1FFF`) address the run reads via the coverage
//! bitmap (`libre99-core` `grom_record_coverage`). Aggregates the results into
//! `grom/COVERAGE-REPORT.md` and asserts the load-bearing invariants:
//!
//! 1. **No bundled cartridge reboots to our title after launch** — an
//!    unimplemented GPLLNK service must RTN gracefully via `SVCBAD`, never reboot
//!    (the regression the loud-stub grid's earlier reboot form caused: Parsec).
//! 2. **No cart is *less alive* under our GROM than under the authentic one** —
//!    the differential health panel. Each cart is launched (and F5-relaunched)
//!    under both firmwares; ours must not leave the console wedged (display off, or
//!    the VBLANK ISR dead) where authentic runs it. "Didn't reboot" is weaker than
//!    "still running": a cart that hangs or goes dead after launch passes (1) but
//!    fails this. This is the automated replacement for "Joel plays each game until
//!    something looks wrong" (QUALITY-ASSESSMENT §C2 health panel). Judging it
//!    *differentially* is essential — many bundled carts (arcade ML titles) take
//!    the machine over and freeze the console ISR themselves, faithfully, under
//!    both firmwares; only a cart the *console* leaves dead is a bug. One is
//!    waived by name (VideovegasC, LIMITATIONS L8 — an on-demand L6-class gap).
//! 3. **Cart-facing interface data (the font homes) is shipped** — no cart may
//!    read a font home as zeros. Every *other* ours-zero read (the tripwire) is
//!    `CODE-REPLACED` monitor code the console sweeps during launch — classified in
//!    `grom/SURFACE-MAP.md`, completeness-gated by `census.rs`, and (2360/2518)
//!    read under the authentic console too. The tripwire is reported per cart for
//!    diagnostics; the font-home safety property is the assertion.
//!
//! Ignored by default (137 carts × launch + F5 under BOTH firmwares is ~4-5 min).
//! Run with `cargo test -p libre99-gpl --test coverage_sweep -- --ignored --nocapture`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use libre99_core::cartridge::Cartridge;
use libre99_core::keyboard::TiKey;
use libre99_core::machine::Machine;
use libre99_gpl::census;

static CONSOLE_ROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994aROM.Bin"));
static AUTHENTIC_GROM: LazyLock<Option<Vec<u8>>> =
    LazyLock::new(|| libre99_core::third_party::load("roms/994AGROM.Bin"));

/// Skip this test (pass, with a notice) when the third-party media is absent.
macro_rules! skip {
    () => {{
        eprintln!("SKIPPED: third-party media not present");
        return;
    }};
}

/// The loud-stub landing address: every unimplemented service entry `>0038-005F`
/// is `B SVCBAD`, and `SVCBAD` lives here (beside `ILRTN`). A fetch here means a
/// cart CALLed a service we do not implement (§5 item 7 / B2).
const SVCBAD: u16 = 0x1201;

fn our_grom() -> Vec<u8> {
    libre99_gpl::system_grom::build_console_grom().unwrap()
}

fn tap(m: &mut Machine, k: TiKey, hold: usize, settle: usize) {
    m.set_key(k, true);
    for _ in 0..hold { m.run_frame(); }
    m.set_key(k, false);
    for _ in 0..settle { m.run_frame(); }
}

fn name_table(m: &Machine) -> impl Iterator<Item = u8> + '_ {
    let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
    (0..24 * 32).map(move |i| m.vdp().vram(base + i))
}

/// Count name-table cells that are neither a space nor an unwritten `>00` — a
/// coarse "the screen is showing something" measure. A launched-and-running cart
/// draws a title/attract/playfield; a dead or blank-screen launch does not.
fn live_cells(m: &Machine) -> usize {
    name_table(m).filter(|&b| b != 0x20 && b != 0x00).count()
}

/// The display is enabled: VDP R1 bit 6 (`>40`). A launched cart that draws
/// anything has it on; a console left wedged with the screen off does not. This is
/// the robust, physically-meaningful health signal — unlike an ISR-tick *count*,
/// it is a discrete end-state, not a phase-sensitive rate.
fn display_on(m: &Machine) -> bool {
    m.vdp().register(1) & 0x40 != 0
}

/// Press `k`, then run `frames` counting how many advanced the console VBLANK
/// counter `>8379` (health-panel check #1, DEBUGGING.md — the "dead console"
/// signal: no sound/sprite-motion/cursor/QUIT). Counted from the launch keypress
/// so a machine-takeover cart's brief setup ticks are captured under both
/// firmwares. Used only in the *strong* form below (authentic clearly running the
/// ISR as primary timekeeper), because a raw threshold misclassifies carts that
/// tick only ~10-20 times during setup then mask the interrupt (burgerbeta,
/// subcom, …) — their tick count straddles any fixed cutoff even though the
/// machine ends in the identical state under both firmwares.
fn launch_isr_ticks(m: &mut Machine, k: TiKey, hold: usize, frames: usize) -> (usize, usize) {
    m.set_key(k, true);
    for _ in 0..hold { m.run_frame(); }
    m.set_key(k, false);
    let mut prev = m.bus().peek(0x8379);
    let mut ticks = 0;
    let mut max_cells = 0;
    for _ in 0..frames {
        m.run_frame();
        let t = m.bus().peek(0x8379);
        if t != prev { ticks += 1; }
        prev = t;
        max_cells = max_cells.max(live_cells(m));
    }
    (ticks, max_cells)
}

/// The per-cart result of a launch+attract+F5 run under one firmware.
struct Run {
    /// console-GROM-0 addresses read from the launch onward.
    read: BTreeSet<u16>,
    /// rebooted to our title after launch (an unimplemented service kicked it
    /// out). Only meaningful under our GROM (the "JOEL ODOM" marker is ours).
    rebooted: bool,
    /// ISR ticks over the launch window and over the F5-relaunch window.
    ticks_launch: usize,
    ticks_f5: usize,
    /// the display was enabled at the end of the launch / F5 windows.
    display_launch: bool,
    display_f5: bool,
    /// non-blank name-table cells at the post-launch checkpoint.
    cells_launch: usize,
}

/// Boot the cart to the menu on `console_grom`, then — with coverage recording on
/// — launch entry 2, let it run a light attract/input mash, sample health, then F5
/// and relaunch and sample health again. `reboot_marker` (our title's unique text)
/// detects a post-launch reboot; `None` for the authentic firmware (no clean
/// unique marker). Returns the read set and the health signals.
fn cover_cart(console_grom: &[u8], cart: &Cartridge, reboot_marker: Option<&str>) -> Run {
    let console_rom = CONSOLE_ROM.as_deref().expect("presence checked by the sweep");
    let mut m = Machine::new(console_rom, console_grom);
    m.mount_cartridge(cart);
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 260); // title -> menu

    // Record from the launch onward: the cart's own console-GROM usage
    // (char-set loaders, any GPLLNK service, fonts read by address). Launch entry
    // 2 and count ISR ticks across the whole launch+settle window, then read the
    // end-state (display).
    m.bus_mut().grom_record_coverage(true);
    let (ticks_launch, cells_window) = launch_isr_ticks(&mut m, TiKey::Num2, 6, 320);
    let display_launch = display_on(&m);
    tap(&mut m, TiKey::Space, 4, 30); // a little input
    tap(&mut m, TiKey::Num1, 4, 30);

    let rebooted = reboot_marker.is_some_and(|marker| {
        name_table(&m).map(|b| b as char).collect::<String>().contains(marker)
    });
    // "Did the launched cart draw?" — take the best of the whole launch window
    // and the post-mash checkpoint. The mash can legitimately START a game
    // (frogger/popeye do, under our GROM, since beam-accurate interrupt
    // timing), and the fixed checkpoint then lands inside the game's
    // screen-clear → playfield-build moment; a transient clear is not a blank
    // launch when the cart demonstrably drew its screen all window long.
    let cells_launch = live_cells(&m).max(cells_window);

    // F5 warm-reset leg: reset (RAM survives), back to the menu, relaunch.
    m.reset();
    for _ in 0..40 { m.run_frame(); }
    tap(&mut m, TiKey::Space, 3, 200);
    let (ticks_f5, _) = launch_isr_ticks(&mut m, TiKey::Num2, 6, 220);
    let display_f5 = display_on(&m);

    let read = m.bus()
        .grom_coverage_addresses()
        .into_iter()
        .filter(|&a| a < 0x2000)
        .collect();
    Run { read, rebooted, ticks_launch, ticks_f5, display_launch, display_f5, cells_launch }
}

/// Fold a sorted address list into inclusive `(start, end)` runs for compact
/// reporting.
fn runs(addrs: &BTreeSet<u16>) -> Vec<(u16, u16)> {
    let mut out: Vec<(u16, u16)> = Vec::new();
    for &a in addrs {
        match out.last_mut() {
            Some(last) if a == last.1 + 1 => last.1 = a,
            _ => out.push((a, a)),
        }
    }
    out
}

#[test]
#[ignore]
fn grom_coverage_sweep() {
    if CONSOLE_ROM.is_none() {
        skip!()
    }
    let Some(authentic_grom) = AUTHENTIC_GROM.as_deref() else { skip!() };
    let Some(third_party) = libre99_core::third_party::dir() else { skip!() };
    let grom = our_grom();
    let mut entries = std::fs::read_dir(third_party.join("cartridges"))
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "ctg").unwrap_or(false))
        .collect::<Vec<_>>();
    entries.sort();

    // The census authentic-only runs (authentic non-zero, ours zero): the surface
    // we ship as zeros. A read landing in one is a tripwire hit.
    let zero_runs = census::authentic_only_runs(&grom, authentic_grom, census::GROM0, 8);
    let in_zero_run = |a: u16| zero_runs.iter().any(|r| a >= r.start && a <= r.end);

    let mut carts = 0usize;
    let mut all_read: BTreeSet<u16> = BTreeSet::new();
    let mut all_tripwire: BTreeSet<u16> = BTreeSet::new();
    let mut auth_read_of_tripwire: BTreeSet<u16> = BTreeSet::new();
    // service/interconnect entry address -> carts that fetched it
    let mut service_hits: BTreeMap<u16, Vec<String>> = BTreeMap::new();
    let mut reboot_hits: Vec<String> = Vec::new();
    // carts where OUR console left the ISR/screen worse off than the authentic
    // console did (the real "our GROM went dead" bug — differential, so carts that
    // legitimately take over the machine under BOTH firmwares don't false-fail).
    let mut isr_regressions: Vec<String> = Vec::new();
    let mut screen_regressions: Vec<String> = Vec::new();
    // carts whose ISR freezes under BOTH firmwares (they take over the machine) —
    // reported for context, not a failure.
    let mut isr_takeover: Vec<String> = Vec::new();
    // per-cart tripwire hit count, for attribution
    let mut cart_tripwire: Vec<(String, usize)> = Vec::new();
    let mut font_home = 0usize; // read >04B4 (authentic std-font home)
    let mut font_1000 = 0usize; // read >1000 (our FONT)
    let mut thin_home = 0usize; // read >06B4 (authentic thin-font home)
    let mut lower_home = 0usize; // read >0874 (authentic lower-case-font home)

    for path in entries {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let data = std::fs::read(&path).unwrap();
        let cart = match Cartridge::parse(&data) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Differential: the same cart under our GROM and the authentic GROM, driven
        // through the identical flow. Health is judged relative to authentic.
        let ours = cover_cart(&grom, &cart, Some("JOEL ODOM"));
        let auth = cover_cart(authentic_grom, &cart, None);
        carts += 1;
        all_read.extend(&ours.read);

        for &a in ours.read.range(0x0010..=0x005F) {
            service_hits.entry(a).or_default().push(name.clone());
        }
        if ours.rebooted { reboot_hits.push(name.clone()); }

        // Health, judged differentially against the authentic console. Two robust
        // signals (a fixed ISR-tick threshold is NOT robust — carts that tick only
        // ~10-20 times during setup then mask the interrupt straddle any cutoff):
        //   (a) DISPLAY: authentic leaves the display on, ours off — the console is
        //       wedged with a black screen (VideovegasC: R1>05 vs >E0).
        //   (b) ISR, strong form: authentic clearly runs the console ISR as the
        //       primary timekeeper (>60 ticks over the window) yet ours ticks under
        //       a quarter as often — the "dead console" class (L1). The >60 floor
        //       excludes machine-takeover carts (authentic itself ticks only ~12-22
        //       before the cart masks the interrupt — inconclusive, not a
        //       regression).
        let display_regr = (auth.display_launch && !ours.display_launch)
            || (auth.display_f5 && !ours.display_f5);
        let isr_regr = |a: usize, o: usize| a > 60 && o * 4 < a;
        let isr_dead = isr_regr(auth.ticks_launch, ours.ticks_launch)
            || isr_regr(auth.ticks_f5, ours.ticks_f5);
        if display_regr || isr_dead { isr_regressions.push(name.clone()); }
        // A cart the console leaves *fully* to itself under both firmwares (display
        // matches, neither runs the ISR as primary) — faithful takeover, context.
        else if auth.ticks_launch <= 60 && ours.ticks_launch <= 60
            && auth.display_launch == ours.display_launch {
            isr_takeover.push(name.clone());
        }
        // Screen, differentially: ours must not be blank where authentic drew.
        if auth.cells_launch >= 8 && ours.cells_launch < 8 { screen_regressions.push(name.clone()); }

        let hits: BTreeSet<u16> = ours.read.iter().copied().filter(|&a| in_zero_run(a)).collect();
        if !hits.is_empty() {
            cart_tripwire.push((name.clone(), hits.len()));
            all_tripwire.extend(&hits);
        }
        // Evidence for the waiver: the authentic console reads the same addresses
        // (there they hold monitor code; here, zeros) — proof it's console
        // execution common to both firmwares, not our-image data corruption.
        auth_read_of_tripwire.extend(auth.read.iter().copied().filter(|&a| in_zero_run(a)));

        if ours.read.contains(&0x04B4) { font_home += 1; }
        if ours.read.contains(&0x1000) { font_1000 += 1; }
        if ours.read.contains(&0x06B4) { thin_home += 1; }
        if ours.read.contains(&0x0874) { lower_home += 1; }
    }

    // The safety property behind the tripwire: no cart may read cart-facing
    // *interface data* (the font homes) as zeros — that data must be shipped. All
    // other ours-zero reads are `CODE-REPLACED` monitor code the console sweeps
    // during launch (classified in SURFACE-MAP, gated for completeness by
    // census.rs, and read under authentic too — see `auth_read_of_tripwire`), so
    // they carry no interface obligation. A hit here means a font home (B1)
    // regressed to zero and a cart noticed.
    let font_home_reads: Vec<u16> = all_tripwire
        .iter()
        .copied()
        .filter(|&a| FONT_HOMES.iter().any(|&(s, e)| a >= s && a <= e))
        .collect();

    // ---- report ----
    let mut r = String::new();
    r.push_str("# GROM coverage report\n\n");
    r.push_str("*Generated by `crates/libre99-gpl/tests/coverage_sweep.rs` (Chunk 2 A2). ");
    r.push_str("Do not edit by hand — re-run the sweep.*\n\n");
    r.push_str(&format!(
        "Ran **{carts} cartridges** (boot → menu → launch entry 2 → attract, then \
         F5 → menu → relaunch), recording every console-GROM-0 (`>0000-1FFF`) address \
         read from the launch onward. **{} distinct addresses** read across all carts.\n\n",
        all_read.len()
    ));

    // Health panel — the C2 automated "play until it looks wrong" replacement,
    // judged *differentially* against the authentic console (each cart run under
    // both firmwares through the identical flow).
    r.push_str("## Post-launch health (C2 health panel, differential vs authentic)\n\n");
    r.push_str(
        "Each cart is launched (and F5-relaunched) under both our GROM and the authentic \
         one, and health is judged *differentially*: our console must never leave a cart \
         **less alive** than the authentic console does. Two robust signals — the display \
         is on (VDP R1 bit 6) and, when authentic clearly drives the console ISR as the \
         primary timekeeper, ours does too. A cart that legitimately takes over the machine \
         (LIMI-masks and runs its own frame loop) does so under *both* firmwares and is \
         reported as \"takes over,\" not a failure.\n\n",
    );
    r.push_str(&format!(
        "- Carts that **rebooted** to our title after launch: **{}**\n",
        if reboot_hits.is_empty() { "none".into() } else { reboot_hits.join(", ") }
    ));
    r.push_str(&format!(
        "- **Health regressions** (authentic runs the console alive — display on / ISR \
         ticking — but ours leaves it wedged): **{}**\n",
        if isr_regressions.is_empty() { "none".into() } else { isr_regressions.join(", ") }
    ));
    r.push_str(&format!(
        "- **Screen regressions** (authentic draws, ours blank): **{}**\n",
        if screen_regressions.is_empty() { "none".into() } else { screen_regressions.join(", ") }
    ));
    r.push_str(&format!(
        "- Carts that **take over the machine** (ISR frozen under *both* firmwares — \
         faithful, for context): **{}** — {}\n\n",
        isr_takeover.len(),
        if isr_takeover.is_empty() { "none".into() } else { isr_takeover.join(", ") }
    ));

    r.push_str("## Service surface `>0010-005F` — what carts actually exercise\n\n");
    r.push_str("Which fixed interconnect/GPLLNK entry addresses a launched cart fetched. \
                This prioritizes Chunk 5 (implement only what carts use).\n\n");
    if service_hits.is_empty() {
        r.push_str("_No cart fetched any `>0010-005F` entry after launch._\n\n");
    } else {
        r.push_str("| Entry | # carts | examples |\n|---|---:|---|\n");
        for (a, cs) in &service_hits {
            let ex: Vec<&str> = cs.iter().take(4).map(|s| s.as_str()).collect();
            r.push_str(&format!("| `>{a:04X}` | {} | {} |\n", cs.len(), ex.join(", ")));
        }
        r.push('\n');
    }
    r.push_str(&format!(
        "(The `>0038-005F` rows above count *reads*, which include carts that data-copy \
         the table — not necessarily service CALLs. An unimplemented service CALL now RTNs \
         gracefully via `SVCBAD` `>{SVCBAD:04X}`, so a launched cart should never be kicked \
         to our title; several bundled carts do CALL unimplemented services and carry on — \
         a Chunk 5 signal, not a failure.)\n\n"
    ));

    r.push_str("## Font homes (B1) — carts that read them\n\n");
    r.push_str(&format!("- `>04B4` standard-font home: **{font_home}** carts\n"));
    r.push_str(&format!("- `>06B4` thin-font home: **{thin_home}** carts\n"));
    r.push_str(&format!("- `>0874` lower-case-font home (the `>004A` loader's data): **{lower_home}** carts\n"));
    r.push_str(&format!("- `>1000` FONT (title/menu + loaders): **{font_1000}** carts\n\n"));

    // Tripwire — attributed per cart, then rolled up into runs.
    r.push_str("## Tripwire — reads landing where we ship zeros (authentic-only runs)\n\n");
    r.push_str(&format!(
        "**{}** distinct ours-zero addresses were read across all carts, by **{}** carts; \
         **{}** of those same addresses are also read under the *authentic* console driven \
         through the identical flow. Every such address falls in a `CODE-REPLACED` (or \
         `SERVICE-ENTRY`) run per `SURFACE-MAP.md` — authentic monitor code our original \
         boot/menu/dispatch replaced by function, which the console's own GPL execution \
         sweeps while launching. That the authentic console reads the *same* addresses \
         (there monitor code, here zeros) is the proof it is console execution common to \
         both firmwares, not our-image data corruption — and no cart's observable health \
         differs. **None is cart-facing interface data**: the only data carts address \
         directly (the fonts) ships byte-identical, so it is not in the ours-zero set. \
         That safety property is the asserted one — no tripwire hit may land in a font \
         home (`census.rs` separately gates the fonts byte-identical + every ours-zero run \
         classified). This section is otherwise diagnostic.\n\n",
        all_tripwire.len(),
        cart_tripwire.len(),
        auth_read_of_tripwire.len(),
    ));
    if !cart_tripwire.is_empty() {
        cart_tripwire.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        r.push_str("Per-cart attribution (ours-zero addresses each cart read):\n\n");
        r.push_str("| Cartridge | # ours-zero reads |\n|---|---:|\n");
        for (name, n) in cart_tripwire.iter().take(20) {
            r.push_str(&format!("| {name} | {n} |\n"));
        }
        if cart_tripwire.len() > 20 {
            r.push_str(&format!("| _(+{} more carts)_ | |\n", cart_tripwire.len() - 20));
        }
        r.push('\n');
    }
    r.push_str("Runs touched (union across all carts):\n\n");
    for (s, e) in runs(&all_tripwire) {
        if s == e {
            r.push_str(&format!("- `>{s:04X}`\n"));
        } else {
            r.push_str(&format!("- `>{s:04X}..>{e:04X}` ({} bytes)\n", e - s + 1));
        }
    }

    std::fs::write("../../original-content/system-roms/grom/COVERAGE-REPORT.md", &r)
        .expect("write COVERAGE-REPORT.md");
    println!("{r}");
    eprintln!(
        "MEASURE carts={carts} tripwire_addrs={} auth_also={} font_home_reads={} \
         health_regressions={:?} screen_regressions={:?} takeover_n={}",
        all_tripwire.len(), auth_read_of_tripwire.len(), font_home_reads.len(),
        isr_regressions, screen_regressions, isr_takeover.len()
    );

    // Invariant 1 — no cart rebooted to our title after launch (an unimplemented
    // service CALL must RTN gracefully, not reboot).
    assert!(
        reboot_hits.is_empty(),
        "these carts rebooted to our title after launch — an unimplemented-service CALL \
         must RTN gracefully, not reboot (regression): {reboot_hits:?}"
    );
    // Invariant 2 — the differential health panel: our console must never leave a
    // cart *less* alive than the authentic console does (the "our GROM went dead
    // after launch" bug class — VBLANK/ISR, case studies 1/6/7/9). The waiver
    // list is empty since 2026-07-07: VideovegasC's dead-console symptom cleared
    // when the XB substrate populated formerly-zero console-ROM addresses on its
    // data-driven launch path (LIMITATIONS L8) — so all 137 carts now gate.
    let unexpected: Vec<&String> = isr_regressions
        .iter()
        .filter(|n| !KNOWN_ISR_REGRESSIONS.contains(&n.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "these carts run the console alive under authentic but wedged under ours (display \
         off or the VBLANK ISR dead after launch/F5) — a dead-console regression: \
         {unexpected:?}"
    );
    assert!(
        screen_regressions.is_empty(),
        "these carts draw a screen under authentic but launch blank under ours: {screen_regressions:?}"
    );
    // Invariant 3 — cart-facing interface data (the font homes) is shipped: no cart
    // may read one as zeros. Every *other* ours-zero read is CODE-REPLACED monitor
    // code (classified in SURFACE-MAP, completeness-gated by census.rs, and read
    // under authentic too), carrying no interface obligation.
    assert!(
        font_home_reads.is_empty(),
        "carts read a console font home (interface data) that is ZERO in our image — B1 \
         regressed; ship the font at its authentic home: {font_home_reads:02X?}"
    );
}

/// Carts whose console the authentic GROM keeps alive but ours leaves wedged —
/// documented, waived regressions (each a `LIMITATIONS.md` entry). A regression
/// *not* in this list fails the gate. **Empty since 2026-07-07** (VideovegasC's
/// wedge cleared with the XB substrate — L8): every bundled cart now gates.
const KNOWN_ISR_REGRESSIONS: &[&str] = &[];

/// Cart-facing interface-data (DATA-MUST-MATCH) homes: the standard font at
/// `>04B4`, the thin font at `>06B4`, and the lower-case (small caps) font at
/// `>0874` — the `>004A` loader's data (B1). These must be shipped byte-identical
/// (`census.rs` gates that); a tripwire hit here means one regressed to zero.
const FONT_HOMES: &[(u16, u16)] = &[(0x04B4, 0x06B3), (0x06B4, 0x0873), (0x0874, 0x094C)];
