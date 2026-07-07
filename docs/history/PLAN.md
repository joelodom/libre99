> **ARCHIVED (2026-07-06).** This is the original implementation plan — "the
> contract for the build" — preserved as an engineering record. Every
> milestone it defines was completed except the macOS `.app` bundle
> (milestone 10, tracked in [../ROADMAP.md](../ROADMAP.md)); the workspace has
> since grown from two crates to four. Its §2 hardware-facts dossier (with
> citations) remains a useful reference. Current documentation:
> [../ARCHITECTURE.md](../ARCHITECTURE.md), [../STATUS.md](../STATUS.md),
> [../DEVELOPMENT.md](../DEVELOPMENT.md).

# Libre99 — Implementation Plan

A cycle-aware TI-99/4A emulator in pure Rust. The console firmware (system ROM +
GROMs containing the GPL interpreter and master title screen) is emulated
*faithfully* — we do **not** reimplement GPL. We emulate the hardware (TMS9900
CPU, TMS9918A VDP, SN76489 PSG, TMC0430 GROMs, TMS9901 + CRU keyboard, the
console memory map, and the TI Disk Controller FD1771 + DSR ROM) and let the real
firmware run.

This document records the architecture, the hardware facts that drive it (with
sources), the milestones and validation gates, the test-driven-development
strategy, and the dependency budget. It is the contract for the build.

---

## 1. Repository inventory (verified by byte inspection)

### `roms/`
| File | Size | Identity | Notes |
|---|---|---|---|
| `994aROM.Bin` | 8 KiB | Console system ROM → CPU `>0000–1FFF` | Big-endian, **not** byte-swapped. Reset vector at `>0000`: **WP=>83E0, PC=>0024** (verified). Level-1 vector at `>0004`: WP=>83C0, PC=>0900. |
| `994AGROM.Bin` | 24 KiB | Console GROMs 0/1/2 → GROM `>0000–5FFF` | Valid `>AA` GPL headers at file 0x0000 (GROM0) and 0x2000 (GROM1). Holds the GPL interpreter tables + master title screen. Loaded 1:1 at GROM addr 0. |
| `Disk.Bin` | 8 KiB | TI Disk Controller DSR ROM → CPU `>4000–5FEF` | `>AA` DSR header exposing devices DSK, DSK1, DSK2, DSK3 and subprogram FILES. CRU base `>1100`. |
| `RS232.Bin` | 8 KiB | RS232 card DSR ROM | Bundled & embedded; not emulated (no device wired). |
| `SpchROM.Bin` | 32 KiB | Speech synthesizer ROM | Bundled & embedded; speech not emulated (stub). |

### `cartridges/` (137 files, `.ctg`)
**Format = Marc Rousseau's `ti99sim` `.ctg`** (GPL, open source), *not* Win994a's
native format, despite the `"TI-99/4A Module - "` banner. Verified: a direct port
of ti99sim `cCartridge::LoadImageV1` parses **all 137 files byte-exact**
(bytes-consumed == filesize). Default cartridge = **Tunnels of Doom**
(`tundoom.ctg` ≡ `tunnelsofdoom.ctg`, identical), a pure-GROM cartridge (GROMs
3–7 at `>6000,>8000,>A000,>C000,>E000`).

### `disks/` (15 files, `.Dsk`)
Raw TI sector-dump images. Most are 90 KiB SSSD (40 trk × 9 sec × 256 B = 92160).
`FWeb501.Dsk` is 180 KiB (720 sectors). Default disk = **`Tunnels.Dsk`** (SSSD):
VIB verified (name "TUNNELS", 360 sectors, 9 sec/trk, "DSK" magic, 40 trk, 1 side,
single density). Holds two PROGRAM files **PENNIES** (AUs 34–84) and **QUEST**
(AUs 85–135) — the Tunnels-of-Doom scenario data the cartridge LOADs from disk.

---

## 2. Hardware reference (the facts the code encodes)

Sources consulted (cited inline in code where the quirk lives):
- **[NOUS]** Thierry Nouspikel, *TI-99/4A Tech Pages* (unige/nouspikel mirror): VDP,
  GROM, CRU, keyboard, sound, disk controller, DSR/headers — the authoritative
  community reference.
- **[MAME]** MAME source: `tms9900.cpp` (timing/microcode), `tms9928a.cpp` (VDP),
  `sn76496.cpp` (PSG + 15-bit LFSR), `tmc0430.cpp` (GROM prefetch),
  `bus/ti99/peb/ti_fdc.cpp` (disk card, FD1771, CRU, data inversion), `datamux.cpp`
  (wait states).
- **[TI99SIM]** Marc Rousseau's `ti99sim` `core/cartridge.cpp` + `core/compress.cpp`
  — authoritative `.ctg` loader (RLE codec, region records).
- **[JS]** js99er (Rasmus M) — readable JS implementations cross-checked.
- Datasheets: TMS9900, TMS9918A, SN76489AN, TMS9901, FD1771.

### 2.1 TMS9900 CPU
- Memory-to-memory; registers R0–R15 live in RAM at **WP + 2·n**. State = WP, PC,
  ST (+ cycle counter, interrupt-request latch).
- **Reset/interrupt vectoring**: a context switch loads WP from `[vec]`, PC from
  `[vec+2]`, saving old WP→newR13, old PC→newR14, old ST→newR15. Reset uses
  `vec=>0000`. Level-N interrupt uses `vec=>4N`; on accept, mask←N−1. LOAD uses
  `>FFFC`. **All 99/4A interrupts reach the CPU as level 1** (vector `>0004`).
- **Status register** (TI bit order, MSB=bit0): `L>`(0x8000) `A>`(0x4000)
  `EQ`(0x2000) `C`(0x1000) `OV`(0x0800) `OP`(0x0400) `X`(0x0200), interrupt
  mask = low 4 bits.
- **Flag-effect table** (the error-prone part — encoded & unit-tested):
  CLR/SETO/SWPB/B/BL/BLWP/LWPI/LIMI/STST/STWP/SBO/SBZ set **no** compare flags;
  **C** is compare-only (no store, no C/OV); **ABS** clears C, sets OV iff
  operand=>8000, compares set from the *original* operand; **NEG/INC/DEC/AI/A/S**
  set C and OV; **SLA** sets OV if the sign changes during the shift; SRA/SRL/SRC
  set C=last-bit-out, no OV; **OP** (parity) only on byte ops; **MPY** sets no
  flags; **DIV** sets only OV (and aborts if divisor ≤ high word); TB sets EQ;
  LDCR/STCR set OP only if ≤8 bits.
- **Addressing** (Ts/Td): `Rn`, `*Rn`, `@addr`/`@addr(Rn)` (extension word),
  `*Rn+` (autoincrement +2 word / +1 byte). Extension-word fetch order =
  **source then destination**. Byte ops act on the high byte.
- **Timing** (cycle-aware): base clocks per instruction + addressing add-ons
  (`*Rn`+4, `@`+8, `*Rn+`+8/+6) per the datasheet/MAME table, **plus wait
  states**: console ROM `>0000–1FFF` and scratchpad RAM `>8000–83FF` are 16-bit &
  fast (0 wait); everything else (VDP/GROM/sound/cartridge/DSR/8-bit RAM) is the
  multiplexed 8-bit bus → **+4 cycles per access**. We count fetch + operand
  accesses in slow space. Frame budget ≈ 50 000 cycles (3.0 MHz / 60).

### 2.2 TMS9918A VDP
- CPU ports: read VRAM `>8800`, read status `>8802`, write VRAM `>8C00`, write
  addr/reg `>8C02`. Address setup = **two bytes to >8C02, low then high**; high
  bits 7–6 select `00`=read-setup, `01`=write-setup, `10`=register write
  (`reg=high&7`). 14-bit auto-incrementing address counter; read-setup prefetches.
- Registers R0–R7 (write masks `{03,FB,0F,FF,07,7F,07,FF}`): mode bits M1(R1 b4)
  M2(R1 b3) M3(R0 b1); 16K/BLANK/IE/size/mag in R1; table bases — Name
  `(R2&0x0F)<<10`, Color `R3<<6`, Pattern `(R4&7)<<11`, SpriteAttr `(R5&0x7F)<<7`,
  SpritePat `(R6&7)<<11`, backdrop/text color R7. Graphics-II masking quirk for
  R3/R4 encoded explicitly.
- Status: F(0x80 vblank) 5S(0x40) C(0x20) + 5th-sprite index; reading clears them
  & de-asserts INT. **INT asserted iff F & (R1&0x20 IE)**.
- Modes: Graphics I (32×24), Graphics II bitmap (256×192, 3-way split tables),
  Multicolor, Text (40×24, no sprites). Sprites: 4 bytes (Y,X,pattern,EC|color);
  **Y displayed at Y+1; Y=>D0 ends the list**; early-clock X−32; 4/line + 5S flag;
  8×8/16×16 × 1×/2× mag. 16-color palette (standard emulator RGB set) encoded.
- Timing: 262 lines, ~59.92 Hz, one level-1 interrupt per frame.

### 2.3 GROM (TMC0430) — prefetch model is the subtle part
- Ports: read data `>9800`, read addr `>9802` (returns counter **+1**, high byte
  first), write data `>9C00` (GRAM only), write addr `>9C02`.
- Set address = **two writes to >9C02, high then low**. After the 2nd byte the
  GROM does an **automatic dummy prefetch** (buffer←mem[A], counter←A+1). Each
  `>9800` read returns the buffer then refills (buffer←mem[counter], counter++).
  Net: first read after setting A returns mem[A] correctly, but the address read-
  back already shows A+1. Encoded exactly to avoid off-by-one.
- 13-bit counter **wraps within the 8K slot** (`(a+1)&0x1FFF | a&0xE000`), never
  rolling into the next GROM. 6K→8K gap reads as `>00`. Slot = top 3 address bits;
  console = GROM 0–2 (`>0000–5FFF`), cartridge GROMs start at GROM 3 (`>6000`).

### 2.4 SN76489 PSG
- Write-only `>8400`. Latch byte `1 cc t dddd` (cc=channel, t=0 freq/1 atten);
  data byte `0 _ dddddd` supplies the high 6 bits of the 10-bit tone period.
  `f = 3 579 545 / (32·N)`, N=0→1024. Attenuation 4-bit, 0=loud…15=silent
  (2 dB/step). Noise control `0xE0|FB<<2|NF`: FB 0=periodic/1=white, NF rate
  (11 = use tone-2 period). **TI LFSR = 15-bit, taps bits 0+1, inverted output,
  reset 0x4000** (not the Sega 16-bit variant).

### 2.5 TMS9901 + CRU keyboard
- 9901 CRU base `>0000`; CRU bit = R12/2. **Keyboard**: write 3-bit column to
  output bits 18–20 (P2=LSB…P4), read rows on input bits 3–10 (active **LOW**).
  Full 8×8 matrix (cols 0–5 keys, col 6 = joystick-1, col 7 = joystick-2) encoded
  from MAME/Nouspikel. Modifiers SHIFT/CTRL/FCTN are matrix cells (col 0).
  **VDP vblank interrupt = /INT2 = CRU input bit 2 (R12 >0004)**. Interval timer
  on the 9901 (level-3) implemented minimally.

### 2.6 TI Disk Controller (FD1771) + DSR — emulate the hardware, run the real ROM
- DSR ROM `Disk.Bin` maps `>4000–5FEF` gated by **CRU >1100 bit 0**. FD1771
  registers overlay `>5FF0–5FFE`: **read** Status/Track/Sector/Data at
  `>5FF0/2/4/6`, **write** Command/Track/Sector/Data at `>5FF8/A/C/E`. **The card
  one's-complements the data bus → every register byte is `XOR 0xFF`.**
- CRU `>1100`: bit0 ROM-enable, bit1 motor, bit2 wait-states (no-op for us),
  bit3 head-load, bits4–6 drive select, bit7 side. INTRQ/DRQ are **not** CRU-
  readable — completion is read from the **status register Busy bit**.
- Minimal FD1771 to satisfy the real DSR: Restore (trk←0), Seek (trk←Data),
  Step (opt), **Read Sector** (`lba = track·9 + sector`; serve 256 bytes via Data
  reg, each `^0xFF`; then clear Busy, raise INTRQ), **Write Sector** (inverse).
  Synchronous — wait states/DRQ/motor are no-ops. Side-1 mapping for DSSD:
  `360 + (39−track)·9 + sector` (for the one 180 KiB image).
- The console GPL DSRLNK scans CRU bases, enables the card, matches the device
  name in the ROM header, and calls the ROM routine, which drives our FD1771.
  We therefore need *no* DSRLNK special-casing — the real firmware does the file
  system (VIB/FDI/FDR parsing, LOAD/SAVE) itself.
- On-disk format (for tooling/validation): VIB sector 0; File Descriptor Index
  sector 1 (list of FDR sector #s); FDR (name, flags@+12, sectors-allocated@+14
  **big-endian**, EOF offset, record len, record count@+18 **byte-swapped**,
  3-byte cluster pointers @+0x1C: `first_AU = b0|((b1&0x0F)<<8)`,
  `last_off = (b1>>4)|(b2<<4)`).

---

## 3. Architecture & module layout

A **Cargo workspace** of two crates, which makes the "core is std-only" guarantee
structural and verifiable:

```
libre99/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ libre99-core/              # PURE std, ZERO third-party deps  (the emulator)
│  │  └─ src/
│  │     ├─ lib.rs            # re-exports; Machine
│  │     ├─ cpu.rs            # TMS9900 (decode/execute/flags/timing/interrupts)
│  │     ├─ bus.rs            # Bus trait + Tms9900Bus impl (console memory map + CRU)
│  │     ├─ vdp.rs            # TMS9918A (ports, regs, VRAM, renderer→framebuffer)
│  │     ├─ psg.rs            # SN76489 (latches, tone/noise, sample synthesis)
│  │     ├─ grom.rs           # TMC0430 GROM array (prefetch/auto-increment)
│  │     ├─ cru.rs            # TMS9901 + CRU bus + interrupt + interval timer
│  │     ├─ keyboard.rs       # 8×8 matrix model (set/clear key, column scan)
│  │     ├─ cartridge.rs      # ti99sim .ctg parser (RLE) → ROM banks + GROM blobs
│  │     ├─ disk.rs           # FD1771 controller + disk image + DSR mapping
│  │     └─ machine.rs        # wires everything; run_frame(); mount cart/disk
│  └─ libre99-app/               # binary frontend (uses the allowed crates)
│     ├─ build.rs             # scans roms/cartridges/disks → embedded_assets.rs
│     └─ src/
│        ├─ main.rs           # config+logging → pick cart/disk → run window loop
│        ├─ assets.rs         # access embedded ROM/cart/disk blobs
│        ├─ config.rs         # preferences TOML (load/save/defaults, resilient)
│        ├─ logging.rs        # leveled logs → terminal + platform log file
│        ├─ app.rs            # winit ApplicationHandler: window + main loop
│        ├─ video.rs          # 256×192 framebuffer → softbuffer (scaled)
│        ├─ audio.rs          # cpal output stream fed by the PSG
│        └─ input.rs          # winit key → TI keyboard matrix
├─ roms/ cartridges/ disks/   # embedded at build time (include_bytes!)
├─ scripts/make_app.sh        # builds the Apple-Silicon .app bundle (planned; milestone 10)
└─ docs/ (PLAN.md, ARCHITECTURE.md, ROADMAP.md, STATUS.md) + README.md (user manual)
```

**Bus trait** is the seam: the CPU talks to `read_word/write_word/read_cru/
write_cru` and a cycle/interrupt hook; `Tms9900Bus` implements the console map and
owns VDP/PSG/GROM/CRU/cartridge/disk. This keeps the CPU testable against a flat
RAM bus and keeps wiring in one place.

---

## 4. Milestones, validation gates & TDD strategy

**Test-driven**: for each behavior write a failing test, then implement to green.
Unit tests live in-module (`#[cfg(test)]`); integration/boot tests in
`crates/libre99-core/tests/`. Gates reported as we pass them.

0. **Scaffold** workspace, `Bus` trait, flat-RAM test harness.
1. **CPU core** — TDD every instruction class against known encodings/results &
   flag effects; reset/interrupt context switch; cycle counts.
   **Gate (a): CPU sane** — known-good instruction sequences (arithmetic, BLWP/
   RTWP, CRU, shifts, addressing modes) produce exact register/flag/memory state.
2. **GROM** — TDD prefetch/auto-increment/address-readback/wrap.
3. **VDP** — TDD port protocol, VRAM auto-increment, register decode, table-base
   math, status/INT; software renderer for Graphics I/II + Text + sprites.
4. **Console bus + CRU/9901 + keyboard** — TDD address decode, CRU column/row,
   VDP-INT bit, interrupt delivery.
5. **Machine + interrupts** — assemble console ROM+GROM.
   **Gate (b) — KEY MILESTONE: boots to the master title screen. ✅ PASSED.**
   Integration test (`tests/boot.rs::boots_to_master_title_screen`) runs 180
   frames and asserts the VDP display is enabled and the framebuffer has several
   distinct colors (the drawn title screen); a PPM framebuffer dump is produced
   for visual confirmation. (Proves CPU + GROM access + VDP are sound together.)
   See `docs/STATUS.md` for the boot bug that gated this and its fix.
6. **Cartridge** — TDD `.ctg` RLE + region parse against the *exact* validated
   breakdowns (tundoom=5 GROM; blasto=1 GROM; Parsec=ROM+3 GROM; xb25=2-bank
   ROM+5 GROM; sizes reconcile). Mount into bus; bank switching for ROM carts.
   **Gate (c): Tunnels of Doom mounted reaches its title/menu. ✅ PASSED.** All
   137 bundled images parse byte-exact; `tunnels_of_doom_appears_on_the_selection_screen`
   (`tests/cartridge.rs`) mounts the cartridge, boots, and asserts the GROM menu
   entry "TUNNELS OF DOOM" is listed on the master selection screen.
7. **Disk** — TDD FD1771 commands, LBA mapping, `^0xFF` inversion, CRU >1100,
   DSR mapping. **Gate (d): Tunnels of Doom loads game data from `Tunnels.Dsk`.
   ✅ PASSED.** `tunnels_of_doom_loads_quest_scenario_from_disk` (`tests/disk.rs`)
   drives the cartridge's "LOAD DATA FROM: DISK 1" prompt, types `QUEST`, and the
   genuine DSR reads the file-descriptor index, the QUEST descriptor, and the
   file's data sectors (AUs 85–135), after which the cartridge reaches its
   post-load game menu. (Read Address, not just Read Sector, turned out to be
   required by the DSR's sector-finding.)
8. **PSG** — TDD latch/freq/atten/noise; cpal playback. ✅ **core done.** The
   SN76489 register decode, tone frequencies, attenuation, and the 15-bit TI
   noise LFSR are implemented and unit-tested (`tests/psg.rs`), with a software
   synthesizer producing host-rate samples (`Machine::fill_audio`). `cpal`
   playback is wired up in the frontend (milestone 9).
9. **Frontend** — winit window + softbuffer present + input + audio + pickers;
   config + logging; default = Tunnels of Doom + Tunnels.Dsk. ✅ **done.** The
   `libre99-app` crate embeds all media (`build.rs`), opens a window, runs at ~60 Hz,
   presents via softbuffer, plays the PSG through `cpal`, maps the host keyboard +
   joystick, reads a preferences TOML, logs to terminal + file, selects media from
   the command line (`--cartridge`/`--disk`/`--list`/…), and switches media in-app
   (`F2`/`F3`/`F4` with a warm reset). The in-app picker is keyboard cycling with
   the selection in the title bar rather than a mouse overlay.
10. **Package** — `.app` bundle (Apple Silicon) + finalized README (user manual) +
    ARCHITECTURE.md; everything embedded, no external files. ⬜ The docs are done;
    the `scripts/make_app.sh` bundle is the remaining piece.

---

## 5. Dependency budget (one line of justification each)

**`libre99-core`: zero third-party crates — std only.** (CPU/VDP/PSG/GROM/CRU/disk/
cartridge/bus/machine.) Enforced structurally by the workspace split.

**`libre99-app` frontend/support crates:**
| Crate | Why (one line) |
|---|---|
| `winit` | De-facto-standard cross-platform windowing + keyboard input with first-class current-macOS / Apple-Silicon support. |
| `softbuffer` | Minimal CPU-side framebuffer presentation (same maintainers as winit); blits our 256×192 image with no GPU/wgpu stack. |
| `cpal` | Standard Rust cross-platform audio output; streams the SN76489-generated samples. |
| `log` | Ubiquitous logging facade so core/app emit leveled records without binding an implementation. |
| `simplelog` | Tiny `log` implementation that writes leveled, human-timestamped output to **both** the terminal and a platform log file. |
| `toml` | Small, well-maintained parser for the user-editable preferences file; robust against hand edits (we write a commented template and fill defaults for missing/partial keys). |

No `serde_derive`, no `dirs`/`directories` (platform paths computed from `$HOME`),
no GPU stack. Core stays dependency-free.

---

## 6. Bundling, paths & packaging
- **Embedding**: `libre99-app/build.rs` scans `roms/`, `cartridges/`, `disks/` and
  emits `include_bytes!` tables (name → bytes). The app is fully standalone; the
  cartridge/disk pickers choose among the embedded blobs. Default selection:
  Tunnels of Doom + Tunnels.Dsk.
- **Config**: `~/.libre99/libre99.toml` (created with commented
  defaults if missing/partial; `log_level=info` default; also default
  cartridge/disk, window scale, fullscreen, audio on/off + volume).
- **Logs**: `~/.libre99/libre99.log` (appended across runs), level from config;
  INFO clean, DEBUG liberal at subsystem seams (CPU traps, GROM address sets, VDP
  register writes, DSR/disk calls). Paths documented in the README.
- **.app**: `scripts/make_app.sh` builds `--release` for `aarch64-apple-darwin`,
  assembles `Libre99.app/Contents/{MacOS,Resources}` + `Info.plist`. No
  external data files (all embedded).

---

## 7. Known scope boundaries
- Speech synthesizer and RS232 are embedded but not functionally emulated
  (stubbed; no device wired) — not needed for the gates.
- Cycle model is cycle-*aware* (correct relative timing, wait-state-counted, 60 Hz
  interrupt cadence), not transistor-exact; sufficient for correct firmware/game
  behavior.
- Disk is the original SSSD FD1771 card (matches the bundled DSR + images).
