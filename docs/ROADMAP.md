# Libre99 — Feature Roadmap

This document lays out where the emulator can go after the core machine is
faithful and playable (milestones 0–9 in the archived
[build plan](history/PLAN.md)). It is grounded in
two reference points the project already respects:

* **Classic99** (Tursi) — the gold-standard TI-99/4A emulator, whose *debugger*,
  *disk manager*, and *cartridge handling* are the features TI users reach for
  most. We already consult its source for hardware behavior (see
  [CLAUDE.md](../CLAUDE.md)); this roadmap borrows its *product* ideas too.
* **Modern multi-system emulators** (MAME, RetroArch, Mednafen, the bsnes
  family) — for conveniences that have become table stakes: save-state slots,
  rewind, fast-forward, run-ahead, shaders, input remapping, movie
  recording, and screenshots.

The goal of the roadmap is not only a list of features but a *shape* for adding
them so the codebase stays clean and each capability is independent.

---

## Road to 0.1.0 — the first public release (early testing)

**0.1.0 is the first source-available public drop, for early testers.** Its governing
constraint is simple: **ship only what the project owns** — the pure-Rust emulator, the
clean-room **libre99** firmware (ROM + GROM), the `libre99asm`/`libre99gpl` toolchain, and our
original cartridges (Titris, Sokoban), with **zero TI or third-party bytes** anywhere in
the tree, the binary, or the published history. The machine already boots and runs
*fully* on the clean-room firmware by default, so a TI-IP-free build is a working
product, not a stub. (Since 2026-07-06 that includes the disk: the clean-room
disk DSR reads *and writes* by default — formerly a row of this table.)

**Phase 1 is done** — three further rows completed 2026-07-06 and
left the table: **media loads on demand** (zero embedded media; the console
boots bare; `--cartridge`/`--disk` take file paths and `F9` opens the
**OS-native file chooser** — owner call: system chooser, not a custom
browser); **the public/local test split** (~90 files now load authentic
images at run time from the git-ignored `third-party/` directory via
`libre99_core::third_party`, skipping green when absent — the book's bench tool
included); and **the purge** (`roms/`, `cartridges/`, `disks/` moved out of
version control into `third-party/`; the gallery generator renders only our
titles; the two third-party README screenshots stay as static images per the
owner decision). The last row — the **verification gate** below — has cleared
too: this fork builds and tests green from a clean checkout, and its public CI
(GitHub Actions — `test` + `clippy` on Windows and macOS, on every push) passes with
**zero proprietary bytes** on the runners' fresh checkouts (owner-confirmed green,
2026-07-07). Everything left for 0.1.0 is Phase 3 polish.

**Order of battle — the sequence is deliberate.** All blocking IP must leave the working
tree **first**; *then* the rename to **Libre99**; and *immediately after the rename* the
project **forks to a brand-new `libre99` repository whose history has never contained
anyone's IP.** That fork is the decided answer to the git-history problem — a public repo
has to be clean all the way back to its first commit, which scrubbing `HEAD` alone cannot
achieve. So the work falls into three phases:

- **Phase 1 · Sever all IP** (in the predecessor repo) — make the tree build and test
  green with zero proprietary bytes. **Done 2026-07-06.**
- **Phase 2 · Rename & fork** — rename to Libre99, then snapshot into the fresh repo.
  **Done 2026-07-06.**
- **Phase 3 · Polish & ship 0.1.0** (this repo) — the remaining, IP-free work.

**Phases 1 and 2 completed 2026-07-06.** The rename made the project **Libre99**
everywhere — the crates (`libre99-core`/`-app`/`-asm`/`-gpl`), the binaries
(`libre99`, `libre99asm`, `libre99gpl`), the license's `Software:` line, the
data directory (`~/.libre99/`, adopted from `~/.ti-99-emulator/` by a one-time
automatic migration; the savestate kept its machine-named `savestate.ti99`
file and `TI99SAVE` magic, so old saves still load — the file was later renamed
`resume.ti99` on 2026-07-07, adopted the same way), all docs, and the book's
toolchain references — while the machine is still called the **TI-99/4A**
wherever the hardware is meant. The fork followed the same day: **this
repository** was created fresh and seeded with a snapshot of the predecessor's
IP-free tree (at its rebrand commit `b3db72f`, minus TI's Editor/Assembler
manual PDF, which moved to the git-ignored `third-party/`), so its history has
never contained a proprietary byte. The old private `ti-99-emulator`
repository is discontinued and must never be published — its *history* still
holds the firmware and media its tree shed.

Gate tags below: **[blocker]** must land before 0.1.0 · **[target]** a 0.1.0 goal, not
release-gating · **[decide]** an owner decision gates it.

| # | Phase | Work item | What it involves — and why it sits here | Gate |
|:--:|---|---|---|:--:|
| 1 | **3 · Polish & ship** | **TI PYTHON — implement the v1 spec** | **Decided 2026-07-07 (owner): grow it.** The spec of record is now **[docs/TI-PYTHON.md](TI-PYTHON.md)** — a user's guide + language specification for **TI PYTHON v1**, *a language very loosely based on Python 3* (full-size variable names, `print(…)` with string literals, comments, `exit()`, Python floor `/`·`%`, and a startup banner that says plainly it's a super-simple Python-like interpreter), plus the **input-bug fixes** (fast typing drops keys; backspace is ignored — both root-caused in the spec's §4 to the v0 read loop, fixed by the KSCAN new-key protocol) and a milestone implementation plan (**§5, P1–P6**) a working session can execute directly. The same document's **§6 feasibility study** maps how TI PYTHON's primitives grow toward running **TI Extended BASIC** (the M6/L9 gap) — census-first, explicitly post-0.1.0. Occupies the same slot as the deferred M6 BASIC (callout below). | [target] |
| 2 | **3 · Polish & ship** | **Docs, in-app help & first-run** | **Revamp the `F1` help (`help.rs`) — explicitly still required before 0.1.0** (the 2026-07-06 media rework and rename made only accuracy edits to it — Joel; the 2026-07-07 disk-persistence and save-state work likewise touched only its media/hotkey/state facts); first-run onboarding on an empty console; README + USER-GUIDE pass; state plainly that **BASIC/XB need user-supplied authentic ROMs** and note the Video Vegas GROM-2 exception. | [blocker] |
| 3 | **3 · Polish & ship** | **Package & release** | Set the workspace to **0.1.0**, add a `CHANGELOG`, tag; ship prebuilt Windows + macOS binaries via GitHub Releases (incl. the macOS `.app` bundle); final crash/robustness pass (first run, missing dir, bad input, no media). | [blocker] |

**Landed 2026-07-07 — save states: atomic, portable, snapshots (the former row 1
[blocker] left the table).** Every state file is now written **atomically**
(temp file + rename, `config::write_atomic` — the preferences use it too), so a
crash or full disk mid-save can never destroy the previous save. The
**portability** half: the state file was already self-contained and
little-endian; format **v3** adds the *cartridge's* host identity alongside the
disks' (v2), so a loaded state names its own media — the frontend no longer
re-reads the cartridge file on resume, and `last_cartridge`/`last_disk` in the
preferences are just the fallback identities for pre-v3 files. Identities are
opaque labels, never re-opened as paths; a regression test loads Windows- and
POSIX-keyed states regardless of host. The UX (Joel's spec, 2026-07-07): the
automatic save is named the **resume state** (`~/.libre99/resume.ti99`, adopted
once from the old `savestate.ti99` name) — auto-saved on exit, auto-loaded at
launch, saved/loaded live with `F6`/`F8`; **snapshots** are user-named `.ti99`
files through the OS-native dialogs (`Shift`+`F6` save, `Shift`+`F8` load, with
a native warning that loading replaces the resume state — which is rewritten
immediately after a successful load); and **`Shift`+`F5` is the fresh start** —
it deletes the resume state after a native warning that counts the in-memory
disk images (and unexported changes) it unloads, then restarts bare like a
first run. `F8` also warns first when loading would roll back unexported disk
changes.

**Landed 2026-07-07 — live disk mounting + disk persistence (two rows left the
table).** The former **"mount a disk without rebooting"** blocker (*bug, Joel
2026-07-07*) and the **"disk persistence — original untouched · tracked delta ·
export"** target landed together. Disks now mount (`F9`) and eject (`F3`)
**live** into the running machine, like a real floppy; only a *cartridge*
change still cold-boots (the console scans cartridge ROM at power-up), and even
that reboot carries the whole disk subsystem across intact. The persistence
model: the host `.dsk` is **never written** — writes mutate the machine's
in-memory image (a whole mutated copy, keyed by the file's canonical path); an
ejected image moves to an in-memory **shelf** and reattaches, edits intact,
when the same file is remounted; save states (format v2 — v1 still loads)
serialize drives *and* shelf, so changed disks survive quit-and-resume. The new
**`F4` disk-memory overlay** lists every in-memory image (`CHANGED`/`CLEAN`),
**exports** one through the OS-native save dialog — whose own replace-prompt is
the guarantee that **no host `.dsk` is ever overwritten unprompted** — and
**unloads** one (native save-first/discard/cancel dialog when dirty) so the
next mount re-reads the host file. The `F1` help's stale "warm reset" wording
went with it.

**Landed 2026-07-07 — the Legal-notices blocker (left the table):** a single root
[`NOTICE.md`](../NOTICE.md) now consolidates all legal notices, kept distinct from
`LICENSE.md` (the project's own grant): the **"Not affiliated with or endorsed by Texas
Instruments"** trademark disclaimer (TI marks used nominatively only); the **Silkscreen**
and **IBM Plex Mono** attributions under the SIL **Open Font License 1.1** (pointing to the
full texts already shipped beside the fonts, not duplicating them); and the **Microban**
level credit (David W. Skinner) that the Sokoban cartridge already shows on screen.

**Open decisions (owner)** — each rides on the row noted:
- **system-roms README "no TI bytes" headline wording** — still open from earlier.

> **Decided 2026-07-07 (owner):** TI PYTHON's 0.1.0 target — **grow it** into the small
> Python-like language specified in **[docs/TI-PYTHON.md](TI-PYTHON.md)** (v1: full-size
> variable names, `print(…)` with string literals, comments, `exit()`, Python floor
> division/modulo, a proper multi-row banner), fixing the fast-typing dropped-keys and
> backspace input bugs on the way; the TI PYTHON name stays. The spec doubles as the
> implementation plan (its §5) and carries the **TI Extended BASIC feasibility study**
> (its §6): grow the console primitives behind TI PYTHON's own milestones, census-first,
> as the long road to closing the M6/L9 BASIC gap — post-0.1.0 by construction. This
> retired the former "TI PYTHON — what does complete mean" [decide] row.

> **Decided 2026-07-07 (owner, via the save-state requirements):** the save-slot
> shape. **One automatic slot — the resume state** (auto-save on exit /
> auto-load at startup, `F6`/`F8` live) — plus **user-named snapshot files**
> through the OS-native save/open dialogs, rather than multiple internal slots:
> the file name *is* the slot name, which makes naming cross-platform-safe by
> construction. Loading a snapshot replaces the resume state (native warning
> first), and `Shift`+`F5` deletes the resume state for a first-run fresh start
> (warning spells out what is lost). This retired the former "save-slot split"
> open decision.

> **Decided 2026-07-07 (owner, via the disk-persistence requirements):** the disk
> delta's shape. The machine keeps a **whole mutated copy** of each disk image in
> memory (not a sparse sector map — simpler, and the images are ≤ ~1.4 MB); an
> image is **keyed by its source file's canonical path**, which is how an export
> or a remount associates back to the original `.dsk`. Persistence is **the live
> session plus its save states** (the exit auto-save already covers a plain
> quit-and-relaunch); there is **no separate write-through** under `~/.libre99/`
> and no opt-in write-through to the source — export is **on-demand only**
> (`F4`), through the OS-native save dialog, whose replace-prompt enforces the
> never-overwrite-unprompted rule.

> **Decided 2026-07-06 (owner, via "rebrand everything"):** the data directory is
> `~/.libre99/` with the `libre99.toml` / `libre99.log` names; startup adopts an
> existing `~/.ti-99-emulator/` automatically (one-time rename), and the
> savestate kept its machine-named `savestate.ti99` file, so nothing was lost in
> the move (renamed once more to `resume.ti99` on 2026-07-07, adopted the same
> way). This retired the former "data-dir rename & migration" open decision.

> **Decided 2026-07-06 (owner):** *no* content is bundled with the application —
> not even our own cartridges. Media enters only via the command line or the
> system file chooser. (This retired the former "default bundled content"
> decision; Titris and Sokoban stay in the repo as sources + built `.ctg`s to
> mount by hand.)

> The git-history question is **resolved and executed**: rather than scrub or squash
> the old repository, the project forked — this repo's history starts at its own
> commit 1 and has never contained a proprietary byte. That is why IP removal had to
> fully precede the fork.

### Definition of done for 0.1.0
The fresh **libre99** repository — history clean back to commit 1 — builds from a clean
checkout a binary named **libre99** that contains **no TI or third-party bytes**, boots
the clean-room firmware, loads cartridges and disks from user-supplied files (command
line + the system file chooser; nothing bundled), reads *and writes* disks on the
clean-room DSR (mounting a disk **without rebooting** the running console), saves and
restores state portably across macOS and Windows, passes a public CI with no proprietary
inventory (green as of 2026-07-07), ships refreshed docs and in-app help, and is
downloadable as a prebuilt binary for both platforms.

---

## Design principles (so features stay modular and cherry-pickable)

1. **The core stays pure `std`, zero-dependency.** Anything that needs a crate
   (file dialogs, gamepads, image/audio encoders) lives in `libre99-app`. New
   *emulated hardware* (speech, cassette, more RAM) belongs in `libre99-core` and
   must come with unit tests, like every existing chip.
2. **Each feature is a self-contained module.** A feature adds a module
   (`input_layout`, `menu`, `speed`, …) and wires into `app.rs` in one small,
   additive place. That keeps feature commits independent enough to **cherry-pick
   onto `main` individually**.
3. **UI is drawn, not toolkited.** On-screen overlays use the `text::Canvas`
   framework (a bitmap font + rectangle/dim helpers) introduced with the
   keyboard reference. No GUI dependency is required for menus, the debugger
   view, or HUD indicators.
4. **The save-state format is versioned.** `state.rs` carries a magic + version;
   any change to what is serialized bumps the version and stays
   backward-detectable (a foreign/old file is rejected cleanly, never
   mis-read).
5. **Persisted choices go through `config.rs`.** New preferences extend the
   resiliently-parsed TOML (missing/invalid keys fall back to defaults).

---

## Themes & features

Each item is tagged: **[done]** implemented and merged to `main` ·
**[next]** designed and high-priority · **[later]** valuable, larger ·
**[stretch]** ambitious.

> **⚠ Clean-room firmware — the big gap: TI BASIC (milestone M6).** The clean-room
> rewrite (our from-scratch console firmware, **now the default**)
> **does not run TI BASIC or Extended BASIC programs.** It deliberately ships
> **TI PYTHON in TI BASIC's menu slot** and has not reimplemented the shared
> **BASIC-era GPL library** those interpreters call to tokenize and execute a line
> (console GROM 2, ~5.5 KiB, currently 0 bytes implemented). So by default,
> Extended BASIC reaches `READY` but `PRINT "HELLO"` does nothing.
> This is **milestone M6 (BASIC)** of the ROM-rewrite track — the largest remaining
> piece and a major effort (a whole interpreter's worth of console services; the
> superset of the L8 Video Vegas one-routine gap). Until it lands, **BASIC needs the
> authentic ROMs** (now selected via `--system-rom` / `--system-grom` — this is a
> firmware-rewrite limitation, not an emulator one). Detail:
> [`original-content/system-roms/LIMITATIONS.md`](../original-content/system-roms/LIMITATIONS.md)
> L9; user-facing note in [`KNOWN-ISSUES.md`](KNOWN-ISSUES.md). A **feasibility study**
> for closing this gap *incrementally* — building the missing console primitives behind
> TI PYTHON's own growth, census-first (phases F0–F5, sized, with the address-space
> conflict and the ROM-M6 policy gate called out) — is **[TI-PYTHON.md §6](TI-PYTHON.md)**
> (2026-07-07). **[later — large]**

### 1. Input & control
- **Host keyboard layout translation (QWERTY / Dvorak / …).** Toggle between
  *positional* mapping (host physical key → same TI position, best for games and
  joystick-style control) and *character* mapping (host **logical** key → TI key,
  so a Dvorak or AZERTY typist gets the letters they actually typed). **[done]**
- Full key/joystick **remapping** from the config file. **[next]**
- **Gamepad** support (via a frontend crate such as `gilrs`). **[later]**
- **Paste-to-type**: inject host clipboard text as TI keystrokes. **[later]**

### 2. Media management (Classic99-grade)
- **Media from arbitrary file paths** — `--cartridge <path>` / `--disk <path>`
  on the command line, and the **OS-native file chooser** on `F9` (`rfd`;
  starts in the last-mounted-from directory, remembered in the preferences);
  `F2`/`F3` eject. This **replaced** the earlier embedded-media pickers (the
  on-screen browser and F2/F3/F4 cycling) when the media embeds were removed
  with the third-party IP (2026-07-06; owner call: system chooser over a
  custom browser). Nothing is bundled — the console boots bare. **[done]**
- **Disks mount and eject live — no reboot** *(was: bug, Joel 2026-07-07)*. `F9`
  slots a disk into the **running** machine and `F3` pulls it just as live,
  like a real floppy; only a *cartridge* change cold-boots (the console scans
  cartridge ROM at power-up), and the rebuild carries the disk subsystem across
  intact. **[done]** (2026-07-07)
- **Disk persistence — the source `.dsk` is never written; in-memory images +
  on-demand export.** Writes mutate the machine's in-memory copy, keyed by the
  file's canonical path; ejected images shelve in memory and reattach on
  remount; save states carry all of it (format v2+). The **`F4` disk-memory
  overlay** exports an image via the OS-native save dialog (its replace-prompt
  means no host `.dsk` is ever overwritten unprompted) or unloads one (with a
  native save-first prompt when changed). **[done]** (2026-07-07)
- **Create / format blank disks**; **import/export** files to and from TI disk
  images (TIFILES / FIAD). **[later]**
- **Recently-used** media list; per-title default disk. **[later]**
- **Fast console-menu cartridge scan (fidelity, firmware).** *(bug/roadmap note,
  Joel 2026-07-06.)* Our rewritten console GROM menu takes ~1–2 s to build a
  cartridge's program list and paints a **`SCANNING`** row while it works. The
  **authentic** menu is fast and shows **no such word** — so both the slowness
  and the cue are our-side artifacts, not fidelity. Our scan is slow because it
  re-writes the GROM address per byte over a 512-byte (or full-slot) window
  (`RECON.md` §10); the `SCANNING` cue was added deliberately to mask that wait
  (`original-content/system-roms/LIMITATIONS.md` **L5**, whose "the authentic
  menu has the same cost" assumption this report **corrects**). Fix: speed the
  `SCANW` walk in `original-content/system-roms/grom/console.gpl` (e.g. bulk
  copy / fewer address rewrites) to authentic speed, then **remove the
  `SCANNING`/`BLANK8` cue** so the list simply appears. **[later]**

### 3. Emulation control
- **Variable speed**: fast-forward / turbo, slow motion, **pause**, and
  **single-frame advance**, with an on-screen state indicator. **[done]**
- **Save states**: the automatic **resume state** (auto-saved on exit, resumed
  at launch, `F6`/`F8` live, `Shift`+`F5` fresh-start delete) plus user-named
  **snapshot** files via the OS-native dialogs (`Shift`+`F6`/`F8`); atomic
  writes, portable format (v3), and **screenshots** (PNG). **[done]** (2026-07-07)
- **Rewind**: a ring buffer of recent save states scrubbed with a key. **[later]**
- **Run-ahead** for lower input latency. **[stretch]**

### 4. Debugging & development (Classic99's debugger is the standout)
- On-screen **debugger overlay**: CPU registers (WP/PC/ST), the workspace, and a
  memory peek window — a live, non-modal panel. **[done]** (live disassembly at
  PC and breakpoints are the natural next step.)
- **Breakpoints**, single-step, and watchpoints driven from the overlay. **[later]**
- **VDP inspector**: pattern/sprite/name-table and palette viewers. **[later]**
- **GROM/GPL trace** and a memory editor. **[later]**
- **Assembler & cartridge builder** (`libre99asm`, crate `libre99-asm`): a from-scratch,
  Editor/Assembler-compatible TMS9900 assembler that emits bootable `.ctg`
  cartridges, so new software — and AI agents — can author cartridges for the
  emulator end to end. User guide and language reference:
  **[ASSEMBLER.md](../assembler/ASSEMBLER.md)**; pairs with the `libre99-app
  --cartridge-file <path>` flag to close the build-run loop (relates to §2's
  "arbitrary file paths"). **[done]** (full TMS9900 ISA; the playable
  [Titris](../original-content/cartridges/titris/README.md) and
  [Sokoban](../original-content/cartridges/sokoban/README.md) cartridges prove
  the pipeline; the bootstrap record is archived at
  [docs/history/ASSEMBLER-POC-PLAN.md](history/ASSEMBLER-POC-PLAN.md).)

### 5. Video & audio
- **Beam-accurate (scanline) VDP rendering**: each of the 192 active lines is
  rasterized from live VRAM at the moment the beam crosses it, interleaved with
  that line's ~190.84 CPU cycles; the frame interrupt rises at end of active
  display (line 192 of 262) and 5S/coincidence latch per line — so mid-frame
  VRAM writes, flashing text, and screen splits render as on hardware (gates in
  `libre99-core/tests/beam.rs`; the Parsec in-game garble first pinned on this
  turned out to be firmware — the `>004A` lower-case loader, see
  KNOWN-ISSUES — but the beam model is hardware-true and stands). **[done]**
  (2026-07-06 — was QUALITY-EVALUATION §3.2 A1 / the "scanline-stepped VDP
  behind the existing seam" deferral.) Two Classic99 refinements deliberately
  remain:
  - **Sub-instruction status-read catch-up** (Classic99 `Tiemul.cpp:5155`):
    advance the VDP to the exact mid-instruction cycle on status reads, for
    software that polls the F bit with interrupts enabled (fbForth's RNG) —
    our F/5S/C changes are quantized to instruction boundaries. **[later]**
  - **Real-time fifth-sprite-number counting** (Classic99 `tivdp.cpp:2641`):
    when no fifth sprite exists the status low bits count the last sprite
    scanned, in real time; Miner 2049er reads it mid-frame. We latch the
    number only with 5S (P2.3). **[later]**
- **CRT presentation**: aspect correction, selectable smooth scaling, and
  optional scanline/shader *filters* (a look, unrelated to the beam-accurate
  rasterizer above). **[later]**
- Selectable **palettes** (TI, the perceptual Classic99 set, greyscale). **[later]**
- **Screenshot** (PNG) — see §3 — and **GIF/video** capture. **[later]**
- **Audio recording** to WAV. **[later]**
- **F18A** enhanced VDP (extra modes, palette, GPU). **[stretch]**

### 6. Hardware & peripherals
- **TMS5220 speech synthesizer** (the Speech Synthesizer module). **[later]**
- **Cassette (CS1/CS2)** in/out (load/save programs to a `.wav`/sound file).
  **[later]** — *not yet emulated:* `crates/libre99-core/src/cru.rs` leaves the
  cassette motor/level CRU outputs "not wired" and provides no read-data input,
  so the console's cassette DSR can prompt but never transfers a byte. This
  blocks loading a file from **CS1** (e.g. a Tunnels of Doom scenario from tape);
  the **disk (DSK1)** path is the supported alternative. Building this means
  modeling the CRU cassette bits (22/23 motor, 24 audio gate, 25 out, 27 in),
  the 9901 interval timer the tape loops rely on, + a tape image/`.wav` source
  feeding the read-data bit the DSR polls. **Decision (Joel, 2026-07-02):**
  the console-ROM rewrite (decision record in the archived
  `original-content/system-roms/history/ROM-REWRITE-PLAN.md`
  §10.2) ships **interface-correct CS1/CS2 error behavior only** — its ROM-side
  tape transport (bit engines + timer ISR) is deferred until this hardware
  exists; when someone builds this item, commission the ROM transport with it.
- **Alpha-lock switch input.** `cru.rs` models alpha-lock only as the P5 output
  latch; the console ROM reads the physical switch by driving P5 low and testing
  CRU bit 7 (`SBZ 21 / TB 7 / SBO 21` — pinned in
  `original-content/system-roms/rom/RECON.md` §23). With no switch input the
  line idles high ("not locked"), so lowercase-capable keytabs never fold — under
  the authentic ROM and the rewrite alike. Building this: latch a host toggle
  (Caps Lock) as the switch state, return it on CRU bit 7 while the P5 latch is
  low (keyboard row 4 otherwise), and reproduce the real-hardware quirk that an
  engaged alpha lock interferes with joystick-up (Classic99 models it). The ROM
  side is already written and differentially gated (`libre99-gpl/tests/rom_kscan.rs`);
  this item makes the fold functional under both ROMs. **[later]**
  - **Decoupled prerequisite win (do first): ship the authentic lowercase
    keytab.** Extended BASIC currently types **uppercase** because our GROM's
    unshifted keytab (`crates/libre99-gpl/src/keymap.rs`) stores uppercase where the
    real machine stores lowercase (see `docs/KNOWN-ISSUES.md` "Extended BASIC …
    types UPPERCASE"). Because the switchless line already idles "not locked,"
    flipping the table to lowercase makes native-mode typing lowercase (real
    behavior) **with zero new hardware** — the menu still folds to uppercase in
    state 0. The alpha-lock *host toggle* above is only needed to let a user
    *lock* back to uppercase; it is **not** required for lowercase-by-default.
    **[next]**
- Memory options: **SAMS/AMS** expansion beyond the 32K. **[later]**
- Multiple **console ROM revisions**, and the TI-99/4 (vs 4A) keyboard. **[later]**
- More **PEB cards**: RS232/serial, p-code. **[stretch]**

### 7. Quality of life & packaging
- macOS **`.app` bundle** (milestone 10) and an optional **native menu bar** that
  mirrors the in-app overlays. **[next]**
- A **settings overlay** to edit preferences without leaving the app. **[later]**
- **Cheats / poke editor** (RAM patches with a small database). **[later]**
- **Netplay** over the `Bus`/input seam. **[stretch]**

### 8. Assurance & hardening
- **Disk-DSR assurance follow-through.** The clean-room disk DSR shipped as the
  default (2026-07-06) with 24 differential gates green, but several
  instruments from its plan's own definition-of-done remain: the random-PAB
  differential **fuzz** (the highest-value item — corruption protection beyond
  the hand-written gates), parameterizing the pre-existing disk test estate
  over `[TI_DSR, OUR_DSR]`, the all-bundled-disks catalog/read sweep, a TI
  BASIC scripted file-I/O parity gate, the op-surface completeness sweep +
  entry-census tripwire, the perf tripwire, a manual xdt99 round-trip, plus
  source hygiene (mojibake repair in `disk-dsr.asm` + an encoding tripwire)
  and doc sync. **Explicitly not required for 0.1.0** (Joel, 2026-07-06) —
  the DSR's differential evidence already supports daily use; this deepens
  it. Full execution plan for a working session:
  [`original-content/system-roms/disk-dsr/DSR-ASSURANCE-PLAN.md`](../original-content/system-roms/disk-dsr/DSR-ASSURANCE-PLAN.md)
  (its A0 hygiene/doc chunk is cheap and pull-forward-able any time). **[later]**

---

## Status — implemented (merged to `main`)

Each feature below landed as a single, self-contained commit that **builds and
runs on its own** — designed so any one could be cherry-picked independently — and
the slice has since been merged to `main`. This table is updated in the same commit
that lands a feature.

| Feature | Module(s) | Commit |
|---|---|---|
| Roadmap (this document) | `docs/ROADMAP.md` | `a81148c` |
| Host keyboard layout translation (Dvorak/QWERTY) | `input` (`KeyLayout`), `config` | `e698ff8` |
| In-app media browser + metadata | `menu` | `22a644d` |
| Variable speed (turbo / pause / frame-advance) | `speed` | `4bfba1d` |
| Screenshot (built-in PNG encoder) | `screenshot` | `980550b` |
| Save state (single; auto-save on exit + resume at launch) | `app`, `config` | `2ad3d05` |
| Live CPU inspector overlay (registers + memory) | `debug` | `e681a7a` |
| Assembler & cartridge builder — full TMS9900 ISA (`libre99asm`) | `libre99-asm` | `93055a6` |
| Mount a `.ctg` from disk (`--cartridge-file`) | `cli`, `app` (main) | `a034949` |
| Public/local test split — runtime `third-party/` media loading, skip-when-absent | `libre99-core` (`third_party`), all test suites | `e5fef69` |
| Zero embedded media — CLI paths + the system file chooser (`F9`), `F2`/`F3` eject | `media`, `cli`, `app`, `config` | `b41a03e` |
| Live disk mount/eject + in-memory disk persistence, `F4` export/unload (save format v2) | `disk`, `machine`, `state` (core); `disks`, `media`, `app` | `c153aa8` |
| Save states: resume state + snapshots + fresh start, atomic writes, portable format v3 (cartridge identity) | `machine` (core); `app`, `config`, `media`, `help` | `09f8fd8` |

> The list above is the *committed* slice of the roadmap; everything tagged
> **[next]/[later]/[stretch]** is future work, captured here so the design intent
> is recorded even where the code isn't written yet.
