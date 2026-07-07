# System ROM Rewrite — original, compatible TI-99/4A console firmware

This folder is a project to **re-create the TI-99/4A's system firmware from
scratch** — as original work — using this repository's own toolchain, so the
emulator can boot a console whose operating system contains **no Texas
Instruments copyrighted bytes**, yet still lists and launches the real cartridges
exactly as the hardware did.

It is the firmware counterpart to **[`titris`](../cartridges/titris/)**. Titris
proved that this repo can author an original *cartridge* end to end (source →
our `libre99asm` → `.ctg` → our emulator). This project proves the same for the
*firmware itself*: original *system software*, assembled by our own tools,
running on our own emulated chips.

**Current state: both halves of the rewrite are complete, and the emulator
boots them by default (since 2026-07-06).** The **console GROM** (all
milestones M0–M7) boots to an original title screen, **lists and launches all
137** cartridges of the differential test corpus from an original selection
menu — **zero waivers** since 2026-07-07 — ships **TI PYTHON** (an original
Python-like mini-language, v1 — [spec](../../docs/TI-PYTHON.md)) in TI BASIC's
menu slot plus a **system-information screen**, and the VBLANK ISR is armed so
sound, sprite motion, and QUIT work. The **console ROM** — the 8 KiB TMS9900
OS (GPL interpreter, ISR, KSCAN, device linkage, FMT, cassette modem layer,
and the radix-100 floating-point package) — is likewise complete and
differentially verified (M1–M5, M7, M8; **M6, TI BASIC, is deferred
indefinitely by policy**), and since 2026-07-07 its **XB substrate** (five
census-pinned helpers, [`XB-CENSUS.md`](./XB-CENSUS.md)) runs a user-supplied
**Extended BASIC** cartridge end-to-end on the clean-room pair
([`LIMITATIONS.md`](./LIMITATIONS.md) **L9**, resolved for XB). By default no
TI console bytes execute at all; **TI BASIC itself** is the one thing that
still needs the authentic firmware, selected via `--system-rom` /
`--system-grom` (user-supplied files — nothing TI ships in this repository).
See [`STATUS.md`](./STATUS.md) and [`rom/README.md`](./rom/README.md).

---

## Which document do I read?

Start here, then go by task. **A new session should read this file, then
`STATUS.md`, then `LIMITATIONS.md`** — that's the full current picture in ~5
minutes. **A debugging session should additionally read
`GROM-DEBUGGING-GUIDE.md` (once) and `DEBUGGING.md`** before touching
anything. Bugs are reported by Joel directly in the session — the conversation
is the ticket; the docs are the method.

| Document | What it is | Read it when |
|---|---|---|
| [`STATUS.md`](./STATUS.md) | what is built and verified, with test evidence | orienting; checking what *should* work |
| [`LIMITATIONS.md`](./LIMITATIONS.md) | known, designed-in gaps — each with a path forward | before shipping/demoing; before treating a symptom as a bug |
| [`GROM-DEBUGGING-GUIDE.md`](./GROM-DEBUGGING-GUIDE.md) | **the whitepaper**: "A Software Engineer's Guide to TI-99 GROM Debugging" — the machine's mental model, boot-stage ladder, spot-diagnosis table, GPL bug taxonomy, methodologies, lessons-learned register | once before your first bug; its §5 table at the start of *every* bug |
| [`DEBUGGING.md`](./DEBUGGING.md) | the operational playbook — protocol, health panel, instruments, probe inventory, traps, test recipes, open investigations, case studies | while actively debugging; *append to it* when you learn something |
| [`RECON.md`](./RECON.md) | the GROM-side interface dossier — every empirically-verified fact about the authentic firmware (headers, dispatch, scratchpad map, GPL semantics) | writing or reviewing GPL; checking a mechanism |
| [`grom/README.md`](./grom/README.md) | the GROM artifact — build/run, address map, source layout, TI PYTHON spec | building, booting, or editing the GROM |
| [`grom/SURFACE-MAP.md`](./grom/SURFACE-MAP.md) | the classified authentic-GROM byte surface (the census gate's authority) | judging a divergence from the authentic image |
| [`rom/README.md`](./rom/README.md) | **the console-ROM track's front door** — status, method, test estate, maintenance notes (layout ledger, house rules) | anything touching `console.asm` |
| [`rom/RECON.md`](./rom/RECON.md) | the ROM-side interface dossier (dispatch tables, ISR, FMT, XML, FP, execution-pinned semantics) | writing or reviewing console-ROM code |
| [`rom/KSCAN-SPEC.md`](./rom/KSCAN-SPEC.md) | the deep keyboard-scanner subsystem spec | KSCAN/keyboard work |
| [`rom/SURFACE-MAP.md`](./rom/SURFACE-MAP.md) | the authentic-ROM byte classification + the frozen-address table (the layout gate's input) | layout questions; "can this move?" |
| [`disk-dsr/README.md`](./disk-dsr/README.md) | **Phase 3 (complete 2026-07-06)** — the clean-room Disk Controller DSR, the emulator's default; execution ledger in [`disk-dsr/PROGRESS.md`](./disk-dsr/PROGRESS.md), deep-tier follow-ups in [`disk-dsr/DSR-ASSURANCE-PLAN.md`](./disk-dsr/DSR-ASSURANCE-PLAN.md) | anything touching the disk-DSR track |
| [`history/`](./history/) | the archived plans, reviews, quality assessments, and execution ledgers that got us here | curiosity about *why* decisions were made; not for current facts |

The GROM source is [`grom/console.gpl`](./grom/console.gpl) (its comment blocks
document the scratchpad cell layouts — they are the authority for "OURS" cells),
built by the [`libre99-gpl`](../../crates/libre99-gpl) crate into the committed
artifact [`grom/console-grom.bin`](./grom/console-grom.bin).

---

## The two system ROMs, and what they do

A bare TI-99/4A needs exactly two firmware images to boot (the authentic TI
images are not part of this repository — on development machines they live in
the git-ignored `third-party/roms/`):

| Image | Size | What it is | Contents |
|---|---|---|---|
| `994aROM.Bin` | 8 KiB | **Console ROM** — TMS9900 machine code | The reset/boot kernel **and the GPL interpreter**. This is the program the CPU actually runs. |
| `994AGROM.Bin` | 24 KiB | **Console GROM** (GROMs 0–2) — **GPL bytecode** | The operating system written in GPL: the master title screen, the selection-list "shell," and TI BASIC. The ROM's interpreter executes this. |

The crucial relationship: **the GROM is not machine code.** Almost all of the TI
operating system is **GPL** (Graphics Programming Language) bytecode, and the
thing that *runs* that bytecode is the small machine-code interpreter inside the
console ROM. Our emulator deliberately does **not** reimplement GPL — it emulates
the chips and runs the genuine firmware, and the genuine firmware interprets
itself (`crates/libre99-core/src/lib.rs:6-11`).

**Strategy: rewrite the GROM first, then the ROM.** Phase 1 kept TI's console
ROM (the GPL interpreter) so the rewritten GROM was guaranteed to run the way
real GROMs do — genuine GPL bytecode behind a genuine `>AA` header, produced by
the `libre99-gpl` toolchain built for this project. Phase 2 then rewrote the
console ROM itself the same clean-room way (recon → spec → reimplement →
differentially verify; see [`rom/README.md`](./rom/README.md)). **Both phases
are complete**, and the pair boots as the emulator's default firmware.

## What we replaced, and what we preserved

The goal is to respect the **spirit of TI's intellectual property**: replace the
*copyrighted creative content* with original content, while reproducing only the
**uncopyrightable interface** required for interoperability — the GROM header
format, the GPL entry contract, the chip port protocol, and the **functional
interface data** a compatible OS must present at fixed addresses (the 8×8 and
thin character-set bitmaps, the keyboard/joystick decode tables). That interface
data is reproduced byte-identically and each block is gated by an identity test;
it is enumerated with its authentic address and disposition in
[`grom/SURFACE-MAP.md`](./grom/SURFACE-MAP.md)'s `DATA-MUST-MATCH` set (see also
the interface-data policy note in [`grom/README.md`](./grom/README.md)).

**Replaced with original content:**

- The **title screen** — an original recreation of the authentic master title
  screen. We keep the layout (the colour bars, the `TEXAS INSTRUMENTS HOME
  COMPUTER` banner, and the `READY-PRESS ANY KEY TO BEGIN` prompt) and replace
  TI's creative content: the stylized "TI" logo becomes an **original Texas + 99
  emblem** (a Texas outline with a stylized "99" — Texas-99, a nod to the TI-99),
  and `© 1981 TEXAS INSTRUMENTS` becomes **`© 2026 JOEL ODOM`**. The master
  selection screen and the power-on beep are recreated the same way.
- **TI BASIC** → **TI PYTHON**, a deliberately tiny, Python-flavored interactive
  calculator (spec in [`grom/README.md`](./grom/README.md)). It takes BASIC's
  place on the selection screen as **`1 FOR TI PYTHON`**. We do **not**
  reproduce TI's ~12 KiB BASIC interpreter at all.

**Preserved (so everything still works):**

- The **boot/powerup contract** with the real console ROM (the fixed GPL entry
  point `>0020`, the VDP/scratchpad setup conventions, arming the VBLANK ISR).
- The **master selection list** — scanning every GROM/cartridge header, listing
  each program, reading the keyboard, and dispatching the chosen entry.
- **Cartridge compatibility** — the 137-cartridge differential corpus (loaded
  at run time from the git-ignored `third-party/`) still appears on the menu
  and launches healthy, **137/137 with zero waivers** (the former Video Vegas
  exception cleared 2026-07-07 — `LIMITATIONS.md` L8). This is a tested
  regression gate, not an aspiration.

> This is a **clean reimplementation for interoperability**, not a copy. We
> wrote all-new GPL and all-new creative on-screen content (the Texas + 99
> emblem, the copyright line, TI PYTHON), and we consult the real firmware's
> *behavior* (via Classic99 and the emulator's own GROM tracer) only to reproduce
> the documented interface a compatible OS must honor.

## Folder layout

```
original-content/system-roms/
├─ README.md                 this document — overview + doc map
├─ STATUS.md                 what's built and verified
├─ LIMITATIONS.md            known gaps, each with a path forward
├─ GROM-DEBUGGING-GUIDE.md   the debugging whitepaper (mental model + methodologies)
├─ DEBUGGING.md              the operational debugging playbook (living doc)
├─ RECON.md                  the GROM-side interface dossier (verified firmware facts)
├─ history/                  archived plans/reviews/assessments/ledgers (provenance only)
├─ grom/
│  ├─ console.gpl            the rewritten console GROM source (GPL)
│  ├─ console-grom.bin       the built 24 KiB image (committed artifact)
│  ├─ README.md              build/run, address map, TI PYTHON spec
│  ├─ SURFACE-MAP.md         the classified authentic-GROM byte surface
│  └─ COVERAGE-REPORT.md     the generated coverage-sweep artifact (never hand-edit)
├─ rom/
│  ├─ console.asm            the rewritten console ROM source (TMS9900)
│  ├─ console-rom.bin        the built 8 KiB image (committed artifact)
│  ├─ README.md              the ROM track's front door + maintenance notes
│  ├─ RECON.md               the ROM-side interface dossier
│  ├─ KSCAN-SPEC.md          the keyboard-scanner subsystem spec
│  └─ SURFACE-MAP.md         byte classification + the frozen-address table
└─ disk-dsr/
   ├─ disk-dsr.asm           the rewritten disk-controller DSR source (TMS9900)
   ├─ disk-dsr.bin           the built 8 KiB image (committed artifact)
   ├─ README.md              the DSR track's front door
   ├─ RECON.md / SURFACE-MAP.md  the DSR-side interface dossier artifacts
   ├─ PROGRESS.md            execution ledger (complete 2026-07-06)
   └─ DSR-ASSURANCE-PLAN.md  deep-tier follow-ups (future work; plan archived in history/)
```

The GPL toolchain is the [`libre99-gpl`](../../crates/libre99-gpl) crate (assembler,
decoder, disassembler, the `libre99gpl` CLI, and the font/keymap generators); the
console ROM is built by [`libre99-asm`](../../crates/libre99-asm) (`libre99asm rom`).
Both binaries are committed and embedded as the emulator's defaults, with
staleness gates tying them to their sources.

## Provenance and license

The rewritten firmware and the toolchain that builds it are **original work**,
licensed with the rest of the project under the Modified MIT License with
Commons Clause ([LICENSE.md](../../LICENSE.md)). By default the emulator
executes no TI bytes at all — console **or disk**: the **Phase 3 clean-room
Disk Controller DSR** ([`disk-dsr/`](./disk-dsr/README.md), complete
2026-07-06) installs by default, with a user-supplied genuine `Disk.Bin`
selectable via `--disk-dsr`. **No authentic TI image is part of this
repository**: for comparison, differential testing, and TI BASIC they are
supplied by the user (`--system-rom` / `--system-grom`) — on development
machines the test suites load them at run time from the git-ignored
`third-party/`, skipping green when absent.
Hardware/firmware *behavior* is cross-checked against Classic99
(checked out on both workstations — `C:\ClaudeShared\classic99` on the PC,
`/Users/Shared/classic99` on the Mac; consult, never copy) and the emulator's
GROM tracer, as the repo's `CLAUDE.md` directs.
