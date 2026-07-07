# Cross-Emulator Validation Plan (rewritten firmware)

*Created 2026-07-06. Addresses the standing circularity risk
[QUALITY-EVALUATION](history/QUALITY-EVALUATION-2026-07-05.md)
§3.2 **A2** / §8.2 **G2**, and the P5 "cross-emulator validation harness" bullet.*

## Why this exists

The from-scratch console firmware (`original-content/system-roms/rom/console.asm`
→ `console-rom.bin`, `grom/console.gpl` → `console-grom.bin`) is verified by a
**differential oracle: TI's authentic image executed by `libre99-core`**. Both
sides of every differential test therefore run inside *this* emulator, so a bug
in *our* hardware model is invisible to the differential suite by construction —
and worse, a firmware bug that only manifests under hardware-accurate execution
can pass here. This class is not hypothetical: the **FONT2 chip-gap** incident
(§8.2 G2) placed data in an address range that our GROM model reads
permissively but real hardware / MAME would not. The only durable mitigation is
to periodically execute the rewrite (or an independent golden reference of the
behavior it reproduces) **outside `libre99-core`**.

## What is on this machine today

| Reference | State here | Automation surface |
|---|---|---|
| **Classic99** (hardware-verified) | Built `classic99.exe` (x86 + x64) inside `C:\ClaudeShared\classic99\dist\classic99.zip` (2026-07-01); source tree unpacked | Windows **GUI**; `-rom <file>` loads a *cartridge* (not console ROM); screenshots via **Scroll-Lock + F5** (raw) / F6 (filtered), BMP only, or Video menu; paste-to-keyboard. **No headless/scripted boot→shot→exit.** |
| **MAME** (`ti99_4a`) | **Not installed** | Scriptable: `-autoboot_delay`/`-autoboot_command`, Lua `-autoboot_script`, PNG `-snapname`, near-headless `-video none`. Needs install + a `ti99_4a` romset. |
| **js99er** | **Not installed** (web app, js99er.net) | Browser + Node; automatable later with headless Playwright/Puppeteer. Needs network/npm. |
| **Real TI-99/4A hardware** | None | — |

Our side already produces the comparison artifacts: the app boots the rewrite
with `--system-rom original-content/system-roms/rom/console-rom.bin
--system-grom original-content/system-roms/grom/console-grom.bin`, screenshots
to PNG (`crates/libre99-app/src/screenshot.rs`), and `boot.rs` /
`examples/boot_frames.rs` render boot-to-title deterministically headless.

## What to compare

1. **Boot-to-title** — the master title screen (colors, "TEXAS INSTRUMENTS
   HOME COMPUTER", the two-choice menu). Pixel/visual diff.
2. **Selection-list / menu listing** — the cartridge/selection screen text and
   layout (exercises the GROM menu walk + FMT screen formatter).
3. **KSCAN behavior** — press a spread of keys (a letter, a digit, FCTN/CTRL
   combos, ENTER) and confirm the same character/keycode results and the same
   debounce/repeat feel.

## Procedure feasible **today** (manual, ~30 min, quarterly or per ROM milestone)

Two independent things are confirmed, in order:

**(A) Our emulator renders authentic firmware like a hardware-verified one.**
1. Unzip `classic99.zip` to a scratch dir; run `x64\classic99.exe`. It boots
   TI's **authentic** firmware. Capture title + menu + a KSCAN interaction
   (Scroll-Lock + F5 → BMP).
2. Boot **our** emulator on **TI's authentic** ROMs (the bundled `994aROM.Bin`
   / `994AGROM.Bin`, i.e. *no* `--system-*` flags), capture the same screens.
3. Diff. Any divergence here is an *emulator-rendering* bug (VDP/GROM/timing),
   independent of the rewrite — file it against `libre99-core`.

**(B) The rewrite reproduces authentic behavior end-to-end.**
4. Boot our emulator with `--system-rom`/`--system-grom` pointed at the rewrite
   binaries; capture the same three screens.
5. Diff (4) against the authentic captures from (2) *and* the Classic99 captures
   from (1). The rewrite passes when all three agree.

This is honest but partial: it transitively confirms *rewrite ≡ authentic ≡
Classic99* for what those screens exercise, and it catches emulator-rendering
divergences — but because our firmware bytes are still executed by *our* CPU/GROM
model, it does **not** catch the FONT2-class bug (firmware relying on memory our
model reads too permissively). For that, run the rewrite *bytes* elsewhere (below).

## Stronger checks that need tooling (the real circularity-breakers)

- **MAME romset swap (definitive).** Install MAME; build a `ti99_4a` romset zip
  whose console-ROM and console-GROM entries carry **our** bytes
  (`console-rom.bin`, `console-grom.bin` split to the driver's GROM regions).
  MAME flags the checksum mismatch but runs past the warning; then
  `-autoboot_delay N -snapname title` (or a Lua `-autoboot_script` that snaps
  after boot) yields a headless PNG executed by MAME's independent core. This
  runs *our firmware* on a second hardware model and is the FONT2-class catcher.
  Cost: MAME install + one romset-assembly script; re-runnable in CI later.
- **js99er headless.** Host js99er locally, drive it with Playwright: load the
  rewrite images, wait, `page.screenshot()`. A third independent core; cost is a
  Node/Playwright toolchain and a small page harness.
- **Real hardware.** Flash the rewrite to a GROM/ROM cart on a real console and
  photograph the title — the ground truth, when hardware becomes available.

## Cadence & ownership

- Run **(A)+(B) manually each time a ROM milestone (Mx) closes**, and at least
  **quarterly**, recording the BMP/PNG captures and a one-line pass/fail beside
  the milestone note. Small, cheap, and it closes G2 meaningfully today.
- Stand up the **MAME swap** once (highest value per hour); wire it into CI when
  the `libre99-firmware` crate is extracted (§12 P4.7) so every firmware change gets
  an independent-core smoke check automatically.

## Honest limitations

The manual Classic99 check validates *rendering + rewrite-vs-authentic* but not
hardware-accurate *execution of our bytes*; only the MAME/js99er/hardware paths
close that last gap, and all three need tooling not present on this machine
today. This document is the plan; none of the external-execution paths are wired
up yet.
