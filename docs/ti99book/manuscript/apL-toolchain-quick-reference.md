# Appendix L — Toolchain Quick Reference

<!-- Appendices · target ≈6 pp · companion to Ch. 3, 6 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. Every project-tool surface below was verified at HEAD by running the tool or reading its source: `libre99asm --help` and `libre99gpl --help` (the invocation and option tables), `code/bench/src/main.rs` (the BENCH99 command dispatch), `docs/USER-GUIDE.md` (the emulator CLI and hotkeys), and `assembler/ASSEMBLER.md` (the assembler's own language/directive reference, which this page points to rather than duplicates). Tier-1 evidence for the project tools; the xdt99 and reference-emulator cribs (L.5–L.6) are tier-4 orientation, hedged per R-2 and owned in full by their subject chapters (Ch. 6, 18, 34, 44). -->

This is the card you keep by the keyboard: the invocations, flags, and command
sets of the four programs this book runs — the assembler `libre99asm`, the GPL
tool `libre99gpl`, the scriptable monitor **BENCH99**, and the desktop emulator
`libre99` — plus a one-screen crib for the outside tools (xdt99 and the reference
emulators) the book reaches for at the edges. It is a *quick* reference: where a
subject has a fuller treatment, this page gives the everyday shape and names the
authority. The assembler's complete language and directive reference is
`assembler/ASSEMBLER.md`; the emulator's complete manual is `docs/USER-GUIDE.md`;
the instruction and GPL opcode tables are Appendices A and B.

Everything here was confirmed by running the tools at the repository's current
commit (R-12): flags drift, so when this card and a live `--help` disagree, the
tool wins, and a patch to this page is welcome.

## L.1 `libre99asm` — the assembler and cartridge packager

A two-pass TMS9900 assembler that emits a runnable cartridge. Invocation forms:

| Form | What it does |
|---|---|
| `libre99asm [OPTIONS] <input.asm>` | Assemble a cartridge (the default mode) |
| `libre99asm rom <out.bin>` | Build the project's rewritten clean-room console ROM |
| `libre99asm dsr <out.bin>` | Build the rewritten disk-controller DSR |
| `libre99asm dis <file.bin> [addr]` | Disassemble from an address (default `>6000`) |

Cartridge-mode options:

| Option | Effect |
|---|---|
| `-o`, `--output <file>` | Output path (default: input name with `.ctg`/`.bin` extension) |
| `--format <fmt>` | `ctg` or `bin` (default `ctg`) |
| `--bin` | Shorthand for `--format bin` — a raw `>6000` image |
| `--name <title>` | Cartridge menu title (default: the `IDT` string, then `"CART"`) |
| `--entry <symbol>` | Entry-point symbol (default: the `END` operand, then `START`) |
| `--listing <file>` | Also write an address/object/source listing |
| `--symbols <file>` | Also write the symbol table as JSON |
| `-h`, `--help` | Print usage |

The two output formats: **`.ctg`** is the project-native cartridge container (the
form `F9` and `--cartridge` mount, and BENCH99's `boot` reads); **`--format bin`**
is a raw, `8,192`-byte, zero-padded `>6000`–`>7FFF` image — the form BENCH99's
`load` expects. The assembler synthesizes the standard cartridge header
automatically from `--name`/`IDT` (Ch. 3, Ch. 35); you write only your code.

Language facts worth keeping on the card (the full reference is
`assembler/ASSEMBLER.md`, §§5–6):

- **Dialect** — the TI *Editor/Assembler* source format: `>XXXX` hex, registers
  `R0`–`R15` predefined, mnemonics and directives case-insensitive.
- **Cartridge origin** — a cartridge assembles `AORG >6000` (absolute); for a
  cartridge every symbol is absolute and relocation never runs.
- **Includes** — `COPY 'file'` (single-quoted) and `INCLUDE`; paths resolve
  relative to the *including* file's directory. This is how `lib99` modules are
  pulled in (Ch. 11). There is **no** macro facility in v1 — the `COPY`-include
  discipline stands in for one (Ch. 11, §11.2).
- **Banking** — the assembler emits **single-bank** images at HEAD; the
  `--banks` flag and `BANK` directive are designed and documented but not yet
  wired to multi-bank output (R-12 — see Ch. 35's build-script workaround).
- **Relocatable object** (`RORG`/tagged object/`DEF`/`REF` and the E/A loaders)
  is Chapter 6's territory, and is where `xas99` (L.5) covers what the project's
  cartridge path does not.

The **canonical build** this book uses everywhere (R-14) — a `.ctg` for the
emulator plus a listing and symbol map for the bench:

```sh
libre99asm src/foo.a99 --name 'TITLE' -o build/foo.ctg \
    --listing build/foo.lst --symbols build/foo.map.json
```

…and the raw image for BENCH99's `load`:

```sh
libre99asm src/foo.a99 --name 'TITLE' --format bin -o build/FOOC.bin
```

## L.2 `libre99gpl` — the GPL assembler and console-GROM harness

The GPL side of the toolchain (Ch. 26–27): a GPL assembler, disassembler, and
the builder that produces the project's clean-room console GROM.

| Form | What it does |
|---|---|
| `libre99gpl asm <src.gpl> <out.bin>` | Assemble GPL source to a GROM image |
| `libre99gpl dis <grom.bin> [hexaddr]` | Disassemble a GROM from an address |
| `libre99gpl console <out.bin>` | Build the clean-room console GROM (with its verification harness) |

`dis` on a console GROM is the ground-truth instrument of Chapter 26 — it turns
"what does GPL *mean*" into an observation rather than an argument. The GPL
opcode and `FMT` grammar reference is Appendix B.

## L.3 BENCH99 — the scriptable monitor

BENCH99 (`code/bench/`) drives the same emulator core from a script of one-line
commands — the instrument every code chapter verifies against. It reads commands
from a file argument or from standard input; `#` begins a comment; hex takes an
optional `>` prefix. Two worlds: a **bare** CPU+bus (for timing and unit work,
where `pc`/`wp` are yours to set) and a **booted** machine (`boot`, which runs
the firmware to the title screen and owns the CPU). On Windows, script paths want
`C:/...` form.

| Command | Args | Does |
|---|---|---|
| `load` | `<file.bin>` | Load a raw cartridge image at `>6000` (bare) |
| `boot` | `[file.ctg\|.bin]` | Boot a machine through the firmware; optional cartridge |
| `pc` / `wp` | `<hex>` | Set PC / workspace pointer (bare only) |
| `s`, `step` | `[n]` | Single-step `n` (default 1), tracing each instruction with cycles + status |
| `x`, `run` | `[n]` | Execute `n` instructions (default 1000), print end state only |
| `u`, `until` | `<hex>` | Run until PC reaches an address |
| `f`, `frames` | `[n]` | Run `n` whole frames (drives the ISR / 60 Hz world) |
| `k`, `key` | `<K>` | Hold a key down for the coming frames |
| `press` / `rel`, `release` | `<K>` | Press / release a key (edge control) |
| `r`, `regs` | | Print R0–R15, PC, WP, and the decoded status register |
| `m`, `mem` | `<hex> [n]` | Dump `n` bytes of CPU memory |
| `pw` / `pb` | `<hex> <val>…` | Poke word / byte(s) into memory |
| `screen` | | Print the screen image (name table) as text |
| `vdp` | | Print the VDP registers (incl. VR7, the border/background) |
| `vram`, `vr` | `<hex> [n]` | Dump VRAM |
| `pixels`, `px` | `<args>` | Read back bitmap-mode pixels |
| `sound`, `snd` | | Read back what the PSG is playing (per-channel Hz + attenuation) |
| `gromlog`, `gl` | `<args>` | Dump the GROM-access log (Ch. 25–26) |
| `cycles` | | Print the cumulative CPU cycle count |
| `q`, `quit` | | End the script |

The register line decodes the status bits as `L> A> EQ C OV OP X` (logical-greater,
arithmetic-greater, equal, carry, overflow, odd-parity, XOP). The `sound` oracle
is what makes a write-only chip testable (Ch. 19); `screen`/`vdp`/`vram`/`px` are
how a headless run proves what reached the glass.

## L.4 `libre99` — the desktop emulator

Launch from the repository root:

```sh
cargo run --release -p libre99-app -- [OPTIONS]
```

Command-line options (the full list is in `docs/USER-GUIDE.md`):

| Option | Effect |
|---|---|
| `--cartridge <path>` | Mount a `.ctg` (alias `--cartridge-file`) |
| `--disk <path>` | Insert a `.dsk` image into DSK1 |
| `--system-rom <path>` | Boot a console ROM image in place of the clean-room default |
| `--system-grom <path>` | Boot a console GROM image in place of the default |
| `--disk-dsr <path>` | Install a disk-controller DSR ROM in place of the default |
| `--scale <n>` | Integer window scale, `1`–`8` |
| `--fullscreen` | Start fullscreen |
| `--log-level <level>` | `error` / `warn` / `info` / `debug` / `trace` |
| `--help` | Print usage |

The everyday hotkeys (the complete table, including the keyboard modes and the
save-state details, is `docs/USER-GUIDE.md`):

| Key | Action | | Key | Action |
|---|---|---|---|---|
| `F1` / `Esc` | Help overlay (five tabs) | | `F7` | Keyboard: positional ⇄ character |
| `F2` | Eject cartridge (reboots) | | `F8` | Load the resume state |
| `F3` | Eject DSK1 (live) | | `Shift`+`F8` | Load a `.ti99` snapshot |
| `F4` | Disk-memory overlay (export/unload) | | `F9` | Mount media (file chooser) |
| `F5` | Reset the console | | `F10` | Pause / resume |
| `Shift`+`F5` | Fresh start (delete resume state) | | `F11` | Fullscreen toggle |
| `F6` | Save the resume state | | `F12` | Frame advance (one frame) |
| `Shift`+`F6` | Save a `.ti99` snapshot | | `Tab` (hold) | Fast-forward |
| | | | `Cmd/Ctrl`+`S` / `+D` | Screenshot / CPU inspector |

All user files live under `~/.libre99/` — preferences (`libre99.toml`), the run
log (`libre99.log`), the auto-saved resume state (`resume.ti99`), snapshots, and
screenshots. Mounting media on the command line skips the resume-state restore.

## L.5 xdt99 interchange crib

**xdt99** is Ralph Benzinger's well-established, freely available cross-development
suite — the book's bridge to the wider TI world for the things the project's own
cartridge path does not (yet) emit: relocatable/tagged object, the `EA5` program
image, and disk-image surgery. Its programs, and when you reach for each:

| Program | Role | Book uses it in |
|---|---|---|
| `xas99` | TMS9900 cross-assembler — the full E/A object model: `RORG`, `DEF`/`REF`, tagged object, `EA5` images | Ch. 6 (object formats & loaders) |
| `xga99` | GPL cross-assembler — the counterpart for GROM code | Ch. 26–27 |
| `xdm99` | Disk-image (`.dsk`) manager — list, add, extract, and build TI disk volumes on the host | Ch. 31–32 |
| `xhm99` | HFE / hardware-image helper for real floppy hardware | Ch. 33 (the hardware bridge) |

Names and roles here are the stable, long-documented shape of the suite; specific
flags change between releases, so consult xdt99's own documentation for exact
invocation (R-2). The division of labor is the thing to remember: `libre99asm`
owns the cartridge, `xas99` owns the E/A object formats, and Chapter 6 is where
the two paths meet.

## L.6 The reference emulators — a shelf cheat sheet

When the project emulator does not model something — because the hardware is
outside its scope, or a behavior is contested — the book names a *shelf* tool and
says so (R-12). One line each:

- **Classic99** (Mike Brent / "Tursi," Windows, C++) — hardware-verified against
  real iron; the arbiter this project itself consults for subtle CPU / VDP / GROM
  behavior. When our emulator and Classic99 disagree, Classic99 is almost
  certainly right. The reference behind the deviation rows in `_ledger.md`.
- **js99er.net** (Rasmus Moustgaard, browser) — runs the **F18A** enhanced VDP
  and is the easy way to see the modern-VDP features Chapters 18 and 34 describe
  but the project core does not emulate.
- **MAME** (the TI-99 driver) — the breadth reference: the Geneve 9640, SAMS, and
  the peripheral cards that live at the family's edges (Ch. 34, 44), and a second
  opinion on timing when the datasheet is ambiguous.

Use them in that spirit: **stock first** — write and prove on the project
toolchain — then enhance-if-present, degrading gracefully on plain hardware (the
R-12 doctrine, Ch. 34).

## L.7 Build-script patterns (R-14)

Every script in this book is `sh` + `cargo` only — no `make`, no `python` in the
build path (generated assets are committed, never regenerated at build time;
Ch. 38). The two scripts that anchor the companion code:

- **`setup.sh`** — once per checkout: `cargo build --release` for the assembler,
  the emulator, and BENCH99, so the binaries exist for the loop below.
- **`verify.sh`** — the CI of 1981 code: assemble *every* `code/**` source with
  `libre99asm` and (re)build the bench; a non-zero exit is a broken book. This is
  the same discipline Chapter 11 (§11.6) makes into a method — scripted BENCH99
  runs as regression tests for vintage assembly.

The everyday inner loop is three moves — assemble to a raw image, then drive it
headlessly and assert on the read-back:

```sh
libre99asm src/foo.a99 --name 'FOO' --format bin -o build/FOOC.bin
cat > run.txt <<'EOF'
load build/FOOC.bin
pc >6000
x 500
vdp          # or: screen / sound / m <addr> <n> — whatever proves the result
EOF
code/bench/target/release/bench99 run.txt
```

That loop — build, run, read back a machine-checkable fact — is the whole method
of this book, and the reason "it runs" is evidence rather than hope (Ch. 3).
