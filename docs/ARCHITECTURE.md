# Libre99 — Architecture

This document describes the emulated machine, how the code is organized, and
how the pieces fit together at run time. For build/run/controls see the
[User Guide](USER-GUIDE.md); for conventions and development workflow see
[DEVELOPMENT.md](DEVELOPMENT.md).

---

## 1. The machine we emulate

The TI-99/4A (1981) is built around Texas Instruments' own 16-bit CPU and a
small set of TI peripheral chips. Unusually, most of the operating system and
all built-in applications are stored not as CPU machine code but as **GPL**
(Graphics Programming Language) bytecode inside **GROM** chips, interpreted by
a small machine-code kernel in the system ROM. Cartridges like *Tunnels of
Doom* are likewise mostly GPL in GROM. This emulator therefore does **not**
reimplement GPL — it emulates the chips faithfully and runs genuine console
firmware (the project's own clean-room rewrite by default, or the authentic TI
ROMs), which interprets the GPL itself.

| Subsystem | Real chip | What it does |
|---|---|---|
| CPU | **TMS9900** | 16-bit, memory-to-memory; its registers live in RAM (the *workspace*). Runs the system ROM kernel and the GPL interpreter. |
| Video | **TMS9918A** (VDP) | 256×192, 15 colors + transparent; name/pattern/color/sprite tables in its own 16 KiB VRAM; one vblank interrupt per frame. |
| Sound | **SN76489** (PSG) | 3 square-wave tone channels + 1 noise channel. |
| Firmware store | **TMC0430 GROMs** | Serial, auto-incrementing ROMs holding the GPL OS, master title screen, and cartridge code. |
| I/O & keyboard | **TMS9901** + CRU bus | Bit-serial I/O ("CRU"); scans the keyboard matrix and delivers the VDP interrupt. |
| Mass storage | **TI Disk Controller** (FD1771 + DSR ROM) | Floppy controller plus a Device Service Routine ROM that implements the TI file system. |

### Console memory map (CPU address space)
```
>0000–1FFF  Console system ROM                        16-bit, fast
>2000–3FFF  Low RAM expansion (32K option)            8-bit, +waits
>4000–5FFF  Peripheral DSR ROM window (e.g. disk DSR) 8-bit, CRU-paged
            >5FF0–5FFE = FD1771 registers (data inverted)
>6000–7FFF  Cartridge ROM window (banked)             8-bit
>8000–83FF  Scratchpad RAM (256 B @ >8300, mirrored)  16-bit, fast (workspaces)
>8400       SN76489 sound (write-only)
>8800/>8802 VDP read data / read status
>8C00/>8C02 VDP write data / write address+register
>9800/>9802 GROM read data / read address
>9C00/>9C02 GROM write data / write address
>A000–FFFF  High RAM expansion (32K option)           8-bit, +waits
```
GROM lives in a *separate* 64 KiB address space reached only through the
>9800/>9C00 ports: console GROMs 0–2 occupy GROM >0000–5FFF, cartridge GROMs
start at GROM >6000.

### Hardware references

The chip behaviors the code encodes were taken from the TMS9900/TMS9918A/
SN76489/TMS9901/FD1771 datasheets, Thierry Nouspikel's *TI-99/4A Tech Pages*,
the MAME and `ti99sim` sources, and above all **Classic99** (Tursi's
hardware-verified emulator), which the project treats as the reference of
record for ambiguous behavior (see [DEVELOPMENT.md](DEVELOPMENT.md)). The
original fact dossier with per-quirk citations is preserved in
[docs/history/PLAN.md](history/PLAN.md) §2; the citations also live as
comments next to the code that encodes each quirk.

---

## 2. Code organization

A **Cargo workspace of four crates**. The split makes the core's "zero
third-party dependencies" rule structural rather than aspirational: only the
desktop app links windowing/audio/logging crates.

```
crates/libre99-core/   the emulator (pure std, zero deps, #![forbid(unsafe_code)])
crates/libre99-app/    desktop frontend (winit + softbuffer + cpal + log/simplelog + toml)
crates/libre99-asm/    libre99asm — TMS9900 assembler + .ctg packager + disassembler (pure std)
crates/libre99-gpl/    libre99gpl — GPL assembler/decoder/disassembler + console-GROM build (pure std)
```

`libre99-asm` and `libre99-gpl` are both stand-alone tools **and** the build +
verification harness for the clean-room firmware in
[`original-content/system-roms/`](../original-content/system-roms/README.md):
they embed the firmware sources, build them deterministically in memory, and
host the differential test suites that verify the rewrite against the
authentic images. (`libre99-gpl` depends on `libre99-asm` to build the console ROM
its tests run against.)

### `libre99-core` modules
| Module | Responsibility |
|---|---|
| `cpu` | TMS9900: instruction decode/execute, status flags, BLWP/RTWP context switches, interrupt acceptance, per-instruction cycle counting. |
| `bus` | The `Bus` trait the CPU drives, plus `FlatRam` for isolated CPU testing. |
| `vdp` | TMS9918A: port protocol, VRAM + registers, status/interrupt, and the beam-accurate scanline rasterizer (RGBA framebuffer). |
| `psg` | SN76489: register latches, tone/noise generation, sample synthesis at the host rate. |
| `grom` | TMC0430 GROM array: address counter, prefetch buffer, auto-increment, 8 KiB-slot wrap. |
| `cru` | TMS9901 + the CRU bit bus: keyboard column/row scanning, the VDP interrupt line, the interval timer. |
| `keyboard` | The 8×8 key/joystick matrix as pure state. |
| `cartridge` | The cartridge parser: the `ti99sim` `.ctg` container (RLE + region records) and raw `.bin` CPU-ROM dumps, both → CPU-ROM banks and GROM blobs; ROM bank switching. |
| `disk` | The FD1771 controller registers, disk-image sector access (geometry from the VIB), DSR-ROM paging, and the disk-persistence model: keyed in-memory images, the eject shelf, dirty tracking, export/forget accessors. |
| `state` | Versioned, `std`-only (de)serialization of a complete machine snapshot behind a magic+version header (no `serde`). |
| `sysinfo` | The Libre99 emulator-identification block: the layout contract shared by the GROM build, the frontend's stamp, and the firmware gates. |
| `machine` | `Tms9900Bus` (the console memory map + CRU routing) and `Machine`: wires everything, `run_frame()`, framebuffer/audio/key access, mounting, save/load state. |

### `libre99-app` modules
| Module | Responsibility |
|---|---|
| `main` | Parses the CLI, loads config, initializes logging, stamps the system-info block, mounts media, optionally resumes the saved session, runs the window loop. |
| `cli` | Hand-rolled parser: `--cartridge`/`--disk`/`--system-rom`/`--system-grom`/`--disk-dsr`/`--scale`/`--fullscreen`/`--log-level`/`--version`/`--help` (media flags take file paths; nothing is embedded). |
| `config` | The preferences TOML (resilient parse, clean rewrite); owns the data-dir/log/resume-state/screenshot paths and the atomic file write (`write_atomic`) every state/preferences save goes through. |
| `assets` | The **clean-room firmware** embedded in the binary (console ROM/GROM + disk DSR) — and nothing else. |
| `media` | Runtime media loading: the OS-native dialogs (`rfd`) — file chooser, disk-export and snapshot save/open, unload/snapshot/fresh-start warnings — `.ctg`/`.bin`/`.dsk` type detection, size guard, media identity keys (canonical paths), read-and-validate shared by the CLI and `F9`. |
| `disks` | The `F4` disk-memory overlay: lists the in-memory disk images (mounted + shelved, `CHANGED`/`CLEAN`) and drives export/unload. |
| `logging` | Leveled logging to terminal + run-log file. |
| `app` | The winit application: window, ~60 Hz frame loop, input routing, hotkeys, overlays; the resume state (save/load, exit auto-save, fresh start) and snapshot save/load. |
| `pacing` | The frame-pacing arithmetic (pure, unit-tested). |
| `video` | Scales/blits the core's framebuffer to the window via softbuffer. |
| `audio` | A cpal output stream pulling PSG samples (lock-contention-safe callback). |
| `input` | Host key events → TI matrix, in character or positional (`KeyLayout`) mapping; platform command-modifier policy. |
| `text` | A tiny bitmap-font `Canvas` (text/rects/dim) painting every overlay — no GUI toolkit. |
| `font` | Rasterizes the embedded Silkscreen / IBM Plex Mono faces for the native-resolution help overlay. |
| `help` | The four-tab `Esc`/`F1` help overlay, including the pictured TI keyboard. |
| `speed` | Pause / frame-advance / fast-forward state. |
| `screenshot` | The built-in PNG encoder for `Cmd/Ctrl`+`S`. |
| `debug` | The live CPU-inspector overlay. |
| `sysinfo` | Fills the core's system-info block with this build's version/date/commit/host at launch. |

### `libre99-asm` modules
| Module | Responsibility |
|---|---|
| `lex`, `expr`, `front` | Tokenizer, E/A-style left-to-right expressions, and the front-end helpers shared with `libre99-gpl`. |
| `isa` | The TMS9900 instruction table (all 69 base opcodes) driving both assembler and disassembler. |
| `lib` | The two-pass assembler, directives, `COPY` includes, cartridge header synthesis, `.ctg`/`.bin` output, listings/symbols. |
| `disasm` | Table-driven disassembler (kept honest by a round-trip test). |
| `system_rom` | Builds the clean-room console ROM from source; the frozen-entry layout gate. |
| `main` | The `libre99asm` CLI: assemble (default), `rom`, `dis`. |

### `libre99-gpl` modules
| Module | Responsibility |
|---|---|
| `asm`, `encode`, `operand` | The GPL assembler: statement parsing and the execution-validated opcode encoder. |
| `decode`, `disasm` | The full-256-opcode GPL decoder and disassembler (reconnaissance tooling). |
| `isa` | The GPL opcode table. |
| `font`, `logo`, `keymap` | Generators for the spliced GROM data blocks (character sets, the Texas-99 emblem, key tables). |
| `census` | Byte-census tooling: classifies our image against the authentic one and gates unclassified divergence. |
| `system_grom` | Builds the clean-room console GROM from `console.gpl` + spliced blocks. |
| `main` | The `libre99gpl` CLI. |

---

## 3. How the pieces fit at run time

### The `Bus` seam
The CPU is deliberately ignorant of the rest of the machine: it only calls
`read_word`, `write_word`, `read_cru_bit`, `write_cru_bits`, and a hook to
learn what each access costs in wait states. `Tms9900Bus` decodes the address
and routes to console ROM, scratchpad RAM, expansion RAM, the VDP/GROM/sound
ports, the cartridge window, or the DSR/FD1771 window, and routes CRU bit
addresses to the TMS9901 or a card's CRU base (the disk controller at >1100).
This seam is what lets the CPU be tested in isolation against a flat-RAM bus,
and keeps all machine-specific wiring in one place.

### A frame
`Machine::run_frame()` walks the beam through the 262-scanline NTSC frame
(≈50 000 CPU cycles at 3 MHz/60, ~190.84 per line, with each memory access
charged its wait states):
1. For each of the 192 active lines, the VDP rasterizes that one line from
   **live** VRAM into its internal framebuffer (or, when nothing has asked for
   pixels, evaluates just the line's sprite status flags), then the CPU runs
   that line's share of cycles. Mid-frame VRAM writes therefore show up
   exactly where the beam is — a name-table write while the beam is at line
   100 appears from line 101 down, never above.
2. At end of active display (line 192), the VDP raises its interrupt flag; if
   enabled, the CPU takes the level-1 interrupt (vector >0004) on the next
   instruction boundary and runs the console's ISR (key scan, sound list,
   timers) inside the real 70-line vertical-blanking window — updates it makes
   are invisible until the next frame's beam, exactly the contract game code
   is written against.
3. The PSG produces the frame's audio samples.
4. The app copies the frame out and presents it (softbuffer), feeds the
   samples to cpal, and maps host key changes into the matrix for the next
   frame.

### Boot to the master title screen
On reset the CPU loads WP/PC from the system ROM's reset vector and begins the
kernel. The kernel initializes the 9901 and VDP, then runs the GPL
interpreter, which reads the title-screen program out of console GROM and
writes the title, color bars, and selection list into VRAM — relying on
correct CPU arithmetic, correct GROM prefetch/auto-increment, correct VDP port
behavior, and the vblank interrupt for timing. Reaching this screen proves
those four are sound together; it is pinned by integration tests for both the
clean-room and authentic firmware.

### Cartridges
The parser accepts two on-disk formats and hands the machine the same parsed
cartridge either way; the format is chosen by content, not extension.

A **`.ctg`** file is the `ti99sim` format: an 80-byte banner, a version/CRU
header, then RLE-compressed *region records* — each a 4 KiB CPU-ROM page
(loaded into the >6000–7FFF window, possibly one of several banks) or an
8 KiB GROM page (loaded into cartridge GROM space at >6000 and up).

A raw **`.bin`** file is a plain CPU-ROM dump with no container: just the
>6000–7FFF window's bytes, one 8 KiB bank after another, opening with the
standard `>AA` module header (the signature that tells a raw dump from junk).
It is the loose-binary form MAME/Classic99 accept (the `…8.bin`/`…C.bin`
naming). The dump is padded up to a power-of-two bank count so the console's
`(addr >> 1) & (banks − 1)` bank-select mask stays clean. Supported: the
standard non-inverted scheme where bank 0 is the boot bank; GROM-only dumps
and the inverted scheme (header only in the last bank) ship as `.ctg` instead.

When a cartridge is mounted its GROMs join the GROM array and its ROM banks
back the cartridge window (replacing any prior cartridge's state); the
console's menu then lists it by its header name. *Tunnels of Doom* is pure
GROM: five GROM pages holding the game engine, which loads scenario data from
disk. *Copper* is a 128 KiB raw ROM `.bin`: sixteen bank-switched 8 KiB banks.

### Disk
The TI Disk Controller is emulated at the hardware level so the **real**
bundled DSR ROM runs: the ROM is paged into >4000–5FFF by CRU bit 0 at base
>1100, and the FD1771 registers at >5FF0–5FFE are modeled (with the card's
data-bus inversion). The console's `DSRLNK` mechanism — plain CPU code that
scans peripheral cards, matches a device name like `DSK1`, and calls the ROM
routine — runs unmodified, and the DSR itself parses the TI on-disk structures
(volume block, file descriptors, sector clusters) and drives our FD1771. Disk
geometry is read from a mounted image's Volume Information Block, with the
classic 40×9 single-sided layout as the fallback.

**Persistence:** the host `.dsk` file is never written back. Sector writes
mutate the machine's in-memory image; every image mounted with a host identity
(its file's canonical path) stays in memory for the life of the machine — an
eject moves it to a **shelf**, a remount of the same file reattaches it, edits
intact, and save states carry drives and shelf alike. Getting edits onto the
host filesystem is an explicit frontend export (the `F4` overlay + the
OS-native save dialog). Disks mount and eject **live**; only a cartridge
change rebuilds the machine, and the frontend transplants the whole `Disk`
across that rebuild.

### Save states
`state.rs` serializes a complete, self-contained snapshot (RAM, VRAM, GROM,
cartridge ROM, every in-memory disk image — mounted drives and the eject shelf,
writes and host identities included) behind a magic+version header (format v3,
which added the cartridge's host identity; v2 added the disk identities/shelf;
v1 files still load, with none of those). The format is portable across
hosts: little-endian everywhere, and the identities are opaque labels never
re-opened as paths. Loads are **staged** — the snapshot decodes into a scratch
machine and is swapped in only on success, so a corrupt file can never
half-corrupt a session — and per-device sanitizers clamp restored
cursors/indices. The frontend keeps **one automatic state, the resume state**
(`~/.libre99/resume.ti99`: exit auto-save, startup resume, `F6`/`F8`), plus
user-named snapshot files (`Shift`+`F6`/`F8`, native dialogs); every state
file is written atomically (temp file + rename) by `config::write_atomic`.

---

## 4. Timing model
The emulator is *cycle-aware* rather than transistor-exact: every instruction
is charged its datasheet base cycles plus addressing-mode and wait-state costs.
Accesses through the console's 8-bit multiplexer (VDP/cartridge/DSR/expansion
RAM) cost ~4 extra cycles; **GROM port accesses carry the much larger measured
stalls of the real chip** (13–22 cycles by port and phase, per Classic99's
hardware-verified numbers), which is what paces GPL-interpreted code — i.e.
the whole OS. Cycles drive the beam: the VDP steps one scanline per ~190.84
CPU cycles (262 lines/frame), the frame interrupt rises at end of active
display, and sprite 5S/coincidence status latches on the very line the beam
meets them. Deliberately deferred (on the [ROADMAP](ROADMAP.md)): Classic99's
sub-instruction status-read catch-up and the real-time fifth-sprite-number
count.

## 5. Rendering & audio
The VDP rasterizes scanline-by-scanline into an internal 256×192 RGBA
framebuffer using the standard TMS9918A palette; the app upscales it by an
integer factor. Headless machines (tests, sweeps) that never ask for pixels
skip rasterizing entirely — sprite status flags are still evaluated per line,
so nothing observable diverges. The PSG mixes its four channels into mono
samples at the host audio rate; cpal plays them. Both are pull-based from the
app's frame loop, so emulation speed and presentation stay decoupled from the
audio callback.

## 6. The clean-room firmware track

The emulator boots original, from-scratch console firmware by default — in the
default configuration **no TI-copyrighted byte executes at all**, console or
disk. Authentic TI images remain loadable (`--system-rom` / `--system-grom` /
`--disk-dsr`, always user-supplied files — the repository carries none), and
they double as the verification oracle the rewrite is tested against. This
section is the architectural overview; the subproject's own documentation tree
(interface dossiers, surface maps, debugging guides) starts at
[original-content/system-roms/README.md](../original-content/system-roms/README.md).

### What each image is

**The console ROM** — 8 KiB of TMS9900 machine code at >0000–1FFF, from
[`rom/console.asm`](../original-content/system-roms/rom/console.asm) — is the
machine's actual operating system: the reset/boot kernel, the **GPL
interpreter** that executes everything living in GROM, the VBLANK ISR
(keyboard scan, sound-list player, timers), the KSCAN keyboard scanner, the
device-linkage scan that finds and calls peripheral-card DSRs, the FMT
screen-format sub-language, the cassette modem layer, and the radix-100
floating-point package (bit-exact against the authentic implementation,
garbage inputs included). The one part *not* rewritten is the ROM's TI BASIC
half, deferred indefinitely by policy — with one justified exception, the
**XB substrate**: the five BASIC-era helpers that Extended BASIC calls
directly by address, reimplemented at those addresses, which is enough to run
an Extended BASIC cartridge end-to-end on the clean-room pair.

**The console GROM** — 24 KiB of GPL bytecode filling GROMs 0–2, from
[`grom/console.gpl`](../original-content/system-roms/grom/console.gpl) — is
the OS layer the interpreter runs: power-up (VDP and scratchpad setup, arming
the VBLANK ISR), the master title screen, the selection-list menu (scan every
GROM and cartridge header, list the programs, dispatch the chosen one), the
GPL-side DSRLNK and GPLLNK service grid, the character-set loaders — and, in
TI BASIC's menu slot, **TI PYTHON** (an original mini-language,
[spec](TI-PYTHON.md)) plus a system-information screen.

**The disk DSR** — 8 KiB, CRU-paged into the >4000–5FFF window — is the
clean-room Device Service Routine for the TI Disk Controller: the TI file
system implemented against our FD1771 model (§3 "Disk").

### Method: recon → reimplement → verify differentially

The build is clean-room but oracle-driven. First **recon**: the authentic
firmware's *interface* was pinned empirically — with the emulator's GROM
tracer, targeted probes, and disassembly used only to recover contracts
(entry addresses, table geometry, register conventions, algorithms restated
as behavioral specs) — into per-track dossiers (each track's `RECON.md`).
Code was then written from the dossier, never from TI's expression. Then
**differential verification**: the same GPL programs execute under the
authentic image and under ours from identical machine state, and the complete
observable state — scratchpad, VDP registers, VRAM, CRU — must match. The
acceptance bar is bug-for-bug *behavior* compatibility, not resemblance. The
differential suites load the authentic images at run time from the git-ignored
`third-party/` (skipping green when absent), so the repository itself stays
free of TI bytes.

### How it compares with the TI original

Side by side, the two consoles are hard to tell apart — until you look at
exactly the places the project chose to differ. Power on the Libre99 firmware
and you get the familiar startup screen: the same color bars, the same `TEXAS
INSTRUMENTS HOME COMPUTER` banner, the same `READY-PRESS ANY KEY TO BEGIN`
prompt, a power-on beep. But the stylized "TI" logo has become an original
**Texas + 99 emblem** (a Texas outline around a stylized 99), the copyright
line reads `© 2026 JOEL ODOM`, and the beep is an original tune. Press a key
and the master selection menu appears, built the way the original builds it —
by scanning every GROM and cartridge header and listing what it finds — except
that slot 1 reads `1 FOR TI PYTHON` where TI BASIC used to live. Those are the
only intentional visible differences, and their placement is the point: they
are precisely the elements that were TI's creative expression rather than
machine interface.

Below the surface, the resemblance is contractual rather than cosmetic.
Cartridges call console routines by absolute address and read console data
from documented locations, so the rewrite pins both. Every public ROM entry —
the vectors, the dispatch tables, the floating-point routines, the helpers
Extended BASIC calls directly — sits at its authentic address (handler
*bodies* float free, and the ROM track keeps a layout ledger of where each one
landed). The functional data a compatible OS must present is byte-identical at
its authentic home: the 8×8 and thin character sets, and the keyboard and
joystick decode tables that KSCAN reads out of GROM to turn scan codes into
characters. A cartridge that peeks at the font, jumps into the ROM, or expects
a key table where the manual says one lives cannot tell the difference.

Compatibility goes deeper than addresses — the bar is *bug-for-bug behavior*.
The authentic ROM, for instance, deliberately exploits the fact that
scratchpad bytes >83E0–>83FF *are* the GPL interpreter's workspace registers,
reading a register's low byte as a memory cell and vice versa; software can
observe that, so the rewrite reproduces the same habits. The limit of the
principle is one documented residual: a garbage-corner parse of a malformed
MOVE encoding that no real program emits, excluded on purpose with a tripwire
test recording the exclusion. Every other divergence from the authentic images
must be classified in the surface maps — an unclassified difference fails the
build.

What the rewrite deliberately does not reproduce is **TI BASIC** — roughly
12 KiB of GPL plus the ROM's BASIC-support half, deferred indefinitely by
policy (running it means supplying authentic images via the flags). That
deferral is also what makes the ROM fit: the non-BASIC OS packs into the same
8 KiB with its entries pinned only because the BASIC half's territory could
absorb relocated code. The practical sting was pulled in the meantime — the
XB substrate is enough for an Extended BASIC cartridge to run end-to-end on
the clean-room pair.

### Performance against the original

A rewrite could easily have been *too fast*, and that would have been a bug.
On this machine the OS's speed is set less by code quality than by a
bottleneck: nearly the whole OS is GPL bytecode pulled one byte at a time
through the GROM ports, and every port access stalls the CPU 13–22 cycles
(§4). The rewrite deliberately keeps the authentic access pattern — per-byte
GROM addressing, no streaming shortcuts — so GPL executes at the authentic
pace under either firmware, and anything timed against the OS behaves the
same.

Parity is enforced by a performance gate (`rom_perf.rs`) holding both of its
metrics to ×1.25 of the authentic firmware, with a fixed 30-frame allowance on
the boot metric. Measured: host wall-clock per emulated frame is ~0.22 ms
under either firmware (ratio 0.99–1.04), and the boot reaches a settled title
screen in 32 frames against the authentic 22 — a fixed ~10-frame offset in
the boot path, about a sixth of a second, after which the two run at
frame-for-frame parity. As predicted at design time, the per-byte GROM
addressing dominates the cost; the code around it is noise.

### The gate architecture

The comparison above is measured, not asserted — every guarantee is a cargo
test over committed artifacts. Staleness gates fail the suite if a committed
`.bin` lags its source. Byte-census gates classify every divergence from the
authentic images against the surface maps. Entry-census tripwires fail if a
pinned address moves (one of them enforces the TI BASIC deferral paperwork).
The differential suites — per-element microtests, the exhaustive 256-opcode
GPL sweep, random-program fuzz with deep soaks, fetch-stream lockstep — run
the same code under both firmwares and diff the observable state. And the
cartridge sweep runs the 137-cartridge corpus through our menu, with a
differential health panel that launches each cartridge under both GROMs and
asserts ours is never *less alive* afterward (display on, ISR ticking):
**137/137, zero waivers**.

### Toolchain wiring

The firmware is built by the workspace's own tools, deterministically:
`libre99asm rom` assembles `console.asm`; `libre99gpl console` assembles
`console.gpl` and splices in the Rust-generated data blocks (the character
sets from `font.rs`, the emblem from `logo.rs`, the key tables from
`keymap.rs`, the system-info block). Tests never read the committed images —
they rebuild the firmware in memory from the embedded sources — while the
desktop binary embeds the committed images via `libre99-app`'s `assets`
module. The system-information screen shows the cross-crate wiring in
miniature: `libre99-core`'s `sysinfo` module defines the block layout once,
the GROM build bakes the block and version strings into the image, and the
frontend stamps build/host fields into its in-memory copy at launch — run the
same GROM on another emulator and those rows degrade to `UNKNOWN` by design.
