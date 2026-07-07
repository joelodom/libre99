# Libre99 — User Guide

The complete manual for the desktop emulator (`libre99-app`). For a two-minute
introduction see the [README](../README.md); this document is the reference.

**A note on modifier keys.** The emulator's own shortcuts use the platform's
command modifier: **`Cmd` on macOS, `Ctrl` on Windows/Linux**. This guide
writes `Cmd/Ctrl` for those. (Only the two letters the emulator actually
claims — `S` and `D` — are withheld from the TI when `Ctrl` is held on
Windows/Linux; every other `Ctrl` chord still reaches the TI's own CTRL key.)

---

## Contents

- [Running it](#running-it)
- [Command-line options](#command-line-options)
- [Console firmware: clean-room default vs. authentic TI](#console-firmware-clean-room-default-vs-authentic-ti)
- [Choosing cartridges and disks](#choosing-cartridges-and-disks)
- [The keyboard](#the-keyboard)
- [Joystick](#joystick)
- [Emulator hotkeys](#emulator-hotkeys)
- [Mounting media (F9) and ejecting (F2/F3)](#mounting-media-f9-and-ejecting-f2f3)
- [Save state, auto-save, and resume](#save-state-auto-save-and-resume)
- [Screenshots](#screenshots)
- [Speed control](#speed-control-pause-frame-advance-fast-forward)
- [CPU inspector](#cpu-inspector)
- [Fullscreen and window scale](#fullscreen-and-window-scale)
- [Preferences file (TOML)](#preferences-file-toml)
- [Logs](#logs)
- [Where your files live](#where-your-files-live)
- [Known limitations](#known-limitations)

---

## Running it

You need a Rust toolchain (stable, edition 2021+) with Cargo. The clean-room
firmware is embedded in the binary — there is nothing else to install or
configure. **No cartridge or disk images are bundled**: media is loaded at run
time from files you supply.

```bash
cargo run --release -p libre99-app
```

The first build compiles the dependencies and takes a little while; subsequent
runs are fast. `--release` is recommended so the emulator runs at full speed.
A window opens at the **master title screen**; press any key for the selection
menu. With no arguments the console boots **bare** — or resumes your previous
session ([below](#save-state-auto-save-and-resume)). To run something, mount a
`.ctg` cartridge or `.dsk` disk image with
[`F9`](#mounting-media-f9-and-ejecting-f2f3) (your system's file chooser) or
the [command line](#command-line-options).

A double-clickable macOS `.app` bundle is planned
([roadmap](ROADMAP.md)); `cargo run` is the supported launcher today.

## Command-line options

Flags select media and display options for a single run and **override the
preferences file** for that run.

| Option | Effect |
|---|---|
| `--cartridge <path>` | Mount a `.ctg` cartridge image (e.g. `libre99asm` output). `--cartridge-file` is accepted as an alias. |
| `--disk <path>` | Insert a `.dsk` disk image into DSK1. |
| `--system-rom <path>` | Boot a console ROM image from disk in place of the default clean-room ROM — e.g. an authentic `994aROM.Bin`. |
| `--system-grom <path>` | Boot a console GROM image from disk in place of the default clean-room GROM — e.g. an authentic `994AGROM.Bin`. |
| `--disk-dsr <path>` | Install a disk-controller DSR ROM in place of the default clean-room DSR — e.g. an authentic `Disk.Bin`. |
| `--scale <n>` | Integer window scale, `1`–`8` (overrides the preferences file). |
| `--fullscreen` | Start fullscreen. |
| `--log-level <level>` | `error` / `warn` / `info` / `debug` / `trace` (overrides the preferences file). |
| `--help` | Print usage and exit. |

> Media flags take **file paths**. A path that doesn't exist or isn't usable
> media is a clear error at launch, before the window opens. `DSK2`/`DSK3`
> are on the [roadmap](ROADMAP.md).

## Console firmware: clean-room default vs. authentic TI

By default the emulator boots this project's **own clean-room firmware**
([Libre99](../original-content/system-roms/README.md)): an original console
ROM and GROM that boot to an original title screen, list and launch mounted
cartridges, and ship **TI PYTHON** (an original integer REPL) plus a
**system-information screen** (press `S` on the selection menu).

The **authentic TI firmware is not distributed with this project** — it is
Texas Instruments' copyrighted work. If you own the images, boot them with:

```bash
cargo run --release -p libre99-app -- --system-rom path/to/994aROM.Bin --system-grom path/to/994AGROM.Bin
```

Two things to know:

- **TI BASIC / Extended BASIC need the authentic firmware.** The clean-room
  rewrite deliberately does not reimplement TI BASIC or the console GPL
  library BASIC-family cartridges call into — under it, Extended BASIC reaches
  `READY` but entered lines do nothing. This is a documented firmware-rewrite
  limitation, not an emulator bug — see
  [KNOWN-ISSUES](KNOWN-ISSUES.md) and
  [LIMITATIONS L9](../original-content/system-roms/LIMITATIONS.md).
- **The system-information screen knows its host.** The emulator stamps its
  version/build/commit/host into an identification block in the clean-room
  GROM at launch; under other emulators those rows read `UNKNOWN`.

## Choosing cartridges and disks

Nothing is embedded; the console boots bare until you mount media, two ways:

1. **Command line** (`--cartridge <path>`, `--disk <path>`) — for a single
   run, e.g. straight out of `libre99asm`.
2. **In-app**, while running: `F9` opens your **system's file chooser**; pick
   any `.ctg` (cartridge port) or `.dsk` (DSK1) — the two are told apart by
   extension. `F2` ejects the cartridge, `F3` empties DSK1. Every in-app
   change **warm-boots** the console with the new media, and the window title
   always shows what is mounted.

## The keyboard

The TI-99/4A reaches its symbols and edit functions through `SHIFT` and `FCTN`
(function) combinations that a modern keyboard doesn't print. Press **`F1`**
(or `Esc`) any time for the help overlay — its **Keyboard** tab pictures the
TI-99/4A with every `SHIFT`/`FCTN` legend in place.

The TI's three modifiers always map to the same host keys, in both modes:

| TI key | Host key |
|---|---|
| **SHIFT** | Left Shift |
| **CTRL** | Left Control |
| **FCTN** (function) | Left Alt / Option |

### Character mode (the default)

Just type. Each keystroke maps to the TI key(s) that produce the **same
character** on any host layout (QWERTY, Dvorak, AZERTY, …): letters and digits
go straight through, and the TI's `SHIFT`/`FCTN` symbol combinations are
pressed for you — `@` sends `SHIFT`+`2`, `"` sends `FCTN`+`P`, `-` sends
`SHIFT`+`/` (the TI has no dedicated minus key), and the whole
`` ` `` `~` `[` `]` `{` `}` `\` `|` family types as its `FCTN` combinations.
Uppercase sends `SHIFT`+letter; lowercase goes through unshifted.

- **Backspace/Delete** sends the TI's backspace (`FCTN`+`S`, cursor-left).
- **Hold Left-Alt** (`FCTN`) or **Left-Ctrl** (`CTRL`) to reach the TI's
  function/control layers yourself — the edit keys, the cursor diamond,
  `QUIT`, and control codes.
- Characters the TI keyboard cannot produce are ignored.

### Positional mode (best for games): `F7`

**`F7`** toggles **positional** mode (a toast shows the change): host keys map
by *physical position* — the key in the QWERTY `Q` spot is TI `Q` regardless
of layout — and you press `SHIFT`/`FCTN` combinations yourself, exactly as on
hardware. Best for games and software that reads the keyboard positionally.
Set the startup default with `key_layout` in the preferences file.

### Edit and cursor keys (FCTN layer)

Hold **`FCTN`** (Left-Alt) with:

| Combination | Function | | Combination | Function |
|---|---|---|---|---|
| `FCTN`+`1` | DEL (delete) | | `FCTN`+`7` | AID |
| `FCTN`+`2` | INS (insert) | | `FCTN`+`8` | REDO |
| `FCTN`+`3` | ERASE | | `FCTN`+`9` | BACK |
| `FCTN`+`4` | CLEAR | | `FCTN`+`=` | QUIT |
| `FCTN`+`5` | BEGIN | | `FCTN`+`E`/`X` | cursor up / down |
| `FCTN`+`6` | PROC'D | | `FCTN`+`S`/`D` | cursor left / right |

The host **arrow keys drive joystick 1**, not the TI cursor — use the
`FCTN`+`E`/`S`/`D`/`X` cursor diamond for the TI's own cursor movement.

## Joystick

Joystick 1 maps to the arrow keys plus Right Alt, in both keyboard modes:

| Joystick 1 | Host key |
|---|---|
| Up / Down / Left / Right | Arrow keys |
| Fire | Right Alt / Right Option |

## Emulator hotkeys

These drive the emulator itself (not the TI). They are ignored while the help
overlay or media browser is open, except the keys that close those overlays.

| Key | Action |
|---|---|
| `F1` or `Esc` | **Help overlay** — five tabs (Start, Keyboard, Hotkeys, Media & State, Settings); switch with `1`–`5`, `Tab`, `←`/`→`. |
| `F2` | **Eject** the cartridge (warm reset — back to the bare console). |
| `F3` | **Empty DSK1** (warm reset). |
| `F5` | **Reset** the console. |
| `F6` | **Save state** — snapshot the whole machine to the save file. |
| `F7` | **Keyboard layout** — toggle positional ⇄ character. |
| `F8` | **Load state** — restore the machine from the save file. |
| `F9` | **Mount media** — pick a `.ctg`/`.dsk` with the system file chooser. |
| `F10` | **Pause / resume**. |
| `F11` (macOS also `Ctrl`+`Cmd`+`F`) | Toggle **fullscreen** (see note). |
| `F12` | **Frame advance** — run a single frame (pauses if running). |
| `Tab` (hold) | **Fast-forward** while held. |
| `Cmd/Ctrl`+`S` | **Screenshot** — save a PNG of the current frame. |
| `Cmd/Ctrl`+`D` | **CPU inspector** — toggle the live register/memory panel. |
| `Cmd`+`Q` (macOS) / `Alt`+`F4` / close window | **Quit** (auto-saves the session first). |

> **Fullscreen on macOS:** the system often swallows bare `F11`, so use
> **`Ctrl`+`Cmd`+`F`** or the green title-bar button; the emulator keeps its
> state in sync either way.

## Mounting media (`F9`) and ejecting (`F2`/`F3`)

`F9` opens your **operating system's native file chooser** (the standard
Open dialog on Windows and macOS), filtered to TI media. Pick any `.ctg`
cartridge or `.dsk` disk image — the extension decides which port it goes to
(cartridge slot vs. DSK1) — and the console warm-boots with it mounted. The
chooser opens in the folder you last mounted from (your home directory on
first run) and remembers the spot across sessions. The emulation pauses while
the dialog is up and resumes when it closes; canceling changes nothing.

`F2` **ejects the cartridge** (back to the bare console) and `F3` **empties
DSK1** — each a warm reset, confirmed by a toast. A file that can't be read
or isn't a usable image is a toast plus a log line, never a dead machine.

## Save state, auto-save, and resume

`F6` writes a **complete, self-contained snapshot** of the running machine —
all RAM and VRAM, the GROM image, the cartridge ROM, and the mounted disk
images **including any sectors a program has written** — to a single file:

```
~/.libre99/savestate.ti99
```

`F8` restores it. A toast confirms each action. The file is portable: it
carries the firmware/cartridge/disk images itself, so a reload does not depend
on which media are selected.

**Auto-save and resume.** On any exit the current state is written to this
same file, and the next launch loads it automatically — you pick up exactly
where you left off, with the window title restored to match. There is **one**
save state: `F6`, `F8`, the exit auto-save, and the startup resume all share
it. Launching with explicit media or firmware (`--cartridge`, `--disk`,
`--system-rom`, `--system-grom`) skips the resume and boots fresh.

> The snapshot itself is self-contained, but the frontend also re-reads the
> mounted media's *files* (recorded in the preferences) on resume, so a later
> in-app media change keeps the other side mounted. If one of those files has
> moved or gone, the resumed session still runs; the log notes the miss.

> **Disk writes live only in the save state.** A program's writes to a mounted
> disk go to the machine's **in-memory** copy of the image — never back to a
> host `.Dsk` file. They persist only through a save state (the exit auto-save
> keeps them across a normal quit-and-resume).

## Screenshots

**`Cmd/Ctrl`+`S`** saves a PNG of the current frame — the clean 256×192 image,
without overlays — to:

```
~/.libre99/screenshots/libre99-<timestamp>.png
```

## Speed control (pause, frame-advance, fast-forward)

| Key | Action |
|---|---|
| `F10` | Pause / resume (a `PAUSED` indicator shows). |
| `F12` | Frame advance — run exactly one frame, then stay paused. |
| `Tab` (hold) | Fast-forward while held. |

Audio is fed only while the machine is advancing, so a pause goes silent
instead of droning the last tone.

## CPU inspector

**`Cmd/Ctrl`+`D`** toggles a live, non-modal panel showing the TMS9900 as the
machine runs: `PC`/`WP`/`ST`, the cycle counter, all sixteen workspace
registers, and the memory words at `PC`. Read-only, updated every frame.

## Fullscreen and window scale

The emulated image is 256×192, upscaled by an integer factor — `window_scale`
in the preferences (default `3`) or `--scale <n>` (`1`–`8`). Toggle fullscreen
with `F11` (macOS: `Ctrl`+`Cmd`+`F`), or start fullscreen with `--fullscreen`
/ `fullscreen = true`.

## Preferences file (TOML)

```
~/.libre99/libre99.toml
```

Created with commented defaults on first run. Missing or malformed keys fall
back to defaults (and the file is rewritten clean), so hand edits can never
break startup.

| Key | Type | Meaning |
|---|---|---|
| `log_level` | string | Logging verbosity: `error` / `warn` / `info` / `debug` / `trace`. |
| `last_cartridge`, `last_disk` | string | File **paths** of the media mounted when the session was last saved — written on exit so a resume re-reads the same files. Managed by the app; no need to edit. |
| `browser_dir` | string | Where the `F9` file chooser opens — follows your last mount. Managed by the app. |
| `window_scale` | integer | Integer upscale of the 256×192 image (`1`–`8`). |
| `fullscreen` | bool | Start fullscreen. |
| `audio_enabled` | bool | Enable audio output. |
| `audio_volume` | float | Output volume, `0.0`–`1.0`. |
| `key_layout` | string | Startup keyboard mapping: `character` or `positional`. |
| `defeat_screen_blank` | bool | The authentic console ROM blanks an idle screen after ~9 minutes (real anti-burn-in behavior, faithfully reproduced — see [KNOWN-ISSUES](KNOWN-ISSUES.md)). `true` suppresses it. Default `false` (faithful). |

## Logs

```
~/.libre99/libre99.log      (appended across runs)
```

Leveled, human-timestamped, written to both the file and the terminal. The
default `info` is clean; **`debug` is the first thing to enable when something
misbehaves** — it adds detail at the subsystem seams (CPU traps, GROM address
sets, VDP register writes, DSR/disk calls). Set `log_level` in the preferences
or `--log-level` on the command line.

## Where your files live

Everything user-specific lives under one directory, created on first run:

```
~/.libre99/
├─ libre99.toml      preferences (commented; see above)
├─ libre99.log       run log (appended across runs)
├─ savestate.ti99           the single save state (F6/F8 + auto-save/resume)
└─ screenshots/             libre99-<timestamp>.png
```

Nothing is written anywhere else in your home directory. (On Windows, `~` is
your user profile directory.)

If you used a build from before the Libre99 rename, your old
`~/.ti-99-emulator/` directory is adopted automatically on first launch — the
directory and the preferences/log files are renamed in place, and the save
state (including its `savestate.ti99` name, which refers to the machine, not
the old project name) carries over untouched.

## Known limitations

- **TI BASIC / Extended BASIC don't execute under the default clean-room
  firmware** — boot user-supplied authentic ROMs for BASIC
  ([above](#console-firmware-clean-room-default-vs-authentic-ti)).
- **Speech** synthesizer and **RS232** are not emulated.
- The disk subsystem models the original **single-sided, single-density
  FD1771** card and its DSR (double-sided and other geometries are read from a
  disk's volume header when present). Only **DSK1** is wired; `DSK2`/`DSK3`
  are on the [roadmap](ROADMAP.md).
- Timing is **cycle-aware** (beam-accurate scanline rendering, wait-state
  counted, 60 Hz cadence), not transistor-exact; a handful of sub-instruction
  effects are on the [roadmap](ROADMAP.md).
- Cartridges needing hardware beyond standard ROM/GROM + bank switching (e.g.
  MiniMemory battery RAM) may not run.

See [KNOWN-ISSUES.md](KNOWN-ISSUES.md) for behaviors that *look* like bugs but
are the real machine (screen blanking, small-caps lowercase, …).
