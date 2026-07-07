# The road to 0.1.0 — the executed release plan

> **ARCHIVED (2026-07-07).** This is the full phase-by-phase record of the
> 0.1.0 release plan as it stood in [ROADMAP.md](../ROADMAP.md), retired when
> the last [blocker] rows landed. Successors: the ROADMAP's compact
> **Road to 0.1.0** recap, [CHANGELOG.md](../../CHANGELOG.md) (what shipped),
> and [STATUS.md](../STATUS.md) (where the project stands). Statuses and
> paths below reflect 2026-07-07.

**0.1.0 is the first source-available public drop, for early testers.** Its governing
constraint is simple: **ship only what the project owns** — the pure-Rust emulator, the
clean-room **libre99** firmware (ROM + GROM), the `libre99asm`/`libre99gpl` toolchain, and our
original cartridges (Titris, Sokoban), with **zero TI or third-party bytes** anywhere in
the tree, the binary, or the published history. The machine already boots and runs
*fully* on the clean-room firmware by default, so a TI-IP-free build is a working
product, not a stub. (Since 2026-07-06 that includes the disk: the clean-room
disk DSR reads *and writes* by default.)

**Order of battle — the sequence was deliberate.** All blocking IP had to leave the
working tree **first**; *then* the rename to **Libre99**; and *immediately after the
rename* the project **forked to a brand-new `libre99` repository whose history has never
contained anyone's IP.** That fork was the decided answer to the git-history problem — a
public repo has to be clean all the way back to its first commit, which scrubbing `HEAD`
alone cannot achieve. So the work fell into three phases:

- **Phase 1 · Sever all IP** (in the predecessor repo) — make the tree build and test
  green with zero proprietary bytes. **Done 2026-07-06.**
- **Phase 2 · Rename & fork** — rename to Libre99, then snapshot into the fresh repo.
  **Done 2026-07-06.**
- **Phase 3 · Polish & ship 0.1.0** (this repo) — the remaining, IP-free work.
  **Done 2026-07-07** (tag + prebuilt binaries are the owner's final step).

## Phase 1 — sever all IP (done 2026-07-06)

Three rows completed 2026-07-06: **media loads on demand** (zero embedded
media; the console boots bare; `--cartridge`/`--disk` take file paths and
`F9` opens the **OS-native file chooser** — owner call: system chooser, not a
custom browser); **the public/local test split** (~90 files load authentic
images at run time from the git-ignored `third-party/` directory via
`libre99_core::third_party`, skipping green when absent — the book's bench
tool included); and **the purge** (`roms/`, `cartridges/`, `disks/` moved out
of version control into `third-party/`; the gallery generator renders only
our titles; the two third-party README screenshots stay as static images per
the owner decision). The **verification gate** cleared too: this fork builds
and tests green from a clean checkout, and its public CI (GitHub Actions —
`test` + `clippy` on Windows and macOS, on every push) passes with **zero
proprietary bytes** on the runners' fresh checkouts (owner-confirmed green,
2026-07-07).

## Phase 2 — rename & fork (done 2026-07-06)

The rename made the project **Libre99** everywhere — the crates
(`libre99-core`/`-app`/`-asm`/`-gpl`), the binaries (`libre99`, `libre99asm`,
`libre99gpl`), the license's `Software:` line, the data directory
(`~/.libre99/`, adopted from `~/.ti-99-emulator/` by a one-time automatic
migration; the savestate kept its machine-named `savestate.ti99` file and
`TI99SAVE` magic, so old saves still load — the file was later renamed
`resume.ti99` on 2026-07-07, adopted the same way), all docs, and the book's
toolchain references — while the machine is still called the **TI-99/4A**
wherever the hardware is meant. The fork followed the same day: **the
`libre99` repository** was created fresh and seeded with a snapshot of the
predecessor's IP-free tree (at its rebrand commit `b3db72f`, minus TI's
Editor/Assembler manual PDF, which moved to the git-ignored `third-party/`),
so its history has never contained a proprietary byte. The old private
`ti-99-emulator` repository is discontinued and must never be published — its
*history* still holds the firmware and media its tree shed.

> The git-history question was **resolved and executed**: rather than scrub or
> squash the old repository, the project forked — this repo's history starts at
> its own commit 1 and has never contained a proprietary byte. That is why IP
> removal had to fully precede the fork.

## Phase 3 — polish & ship (done 2026-07-07)

**Landed 2026-07-07 — Docs, in-app help & first-run (the former row 1
[blocker]).** The five-tab `Esc`/`F1` help was revamped and verified
tab-by-tab against the code and USER-GUIDE (Esc is now the advertised key —
reconciling the banner prompt with the binding, which had accepted Esc all
along); first-run onboarding landed as the **`PRESS ESC FOR HELP` banner**,
shown whenever there is no resume state (first launch, or after a
`Shift`+`F5` fresh start), drawn via the `text::Canvas` overlay framework and
retired the moment the user opens the help or saves a state; the README +
USER-GUIDE pass swept every doc outside the book for stale facts; and both
docs state plainly that **TI BASIC needs user-supplied authentic ROMs**
(Extended BASIC runs on the clean-room pair since 2026-07-07; the former
Video Vegas health waiver cleared the same day — zero waivers).

**Landed 2026-07-07 — version 0.1.0 + CHANGELOG (the former row 2 [blocker],
minus the owner's tag/package step).** The workspace version — the single
number the emulator, the firmware markers, and TI PYTHON's banner all
inherit — was set to **0.1.0**, both committed firmware artifacts were
rebuilt (their staleness gates tie them to source), a root
[CHANGELOG.md](../../CHANGELOG.md) records the release, and the new
`--version` flag reports it. Remaining with the owner: the final hands-on
testing pass, `git tag v0.1.0`, and prebuilt Windows/macOS binaries on a
GitHub Release.

**Landed 2026-07-07 — TI PYTHON v1 + Extended BASIC on the clean-room
firmware.** The v1 spec ([docs/TI-PYTHON.md](../TI-PYTHON.md)) was
implemented in full the day it was written: the input-bug fixes (fast typing
dropped keys, backspace was ignored — the KSCAN new-key protocol replaced the
v0 wait-for-release loop), the four-row banner + `>>> ` prompt, a scrolling
terminal screen with a blinking cursor, full-size variable names (32 VRAM
slots), Python floor `/`·`//`·`%` and a real unary minus (v0 silently
mis-evaluated `2*-3`), `print(…)` with string literals, `#` comments, and
`exit()`/`quit()` — twelve gates. **The spec's §6 feasibility study also paid
off immediately**: its census-first plan (F0) found Extended BASIC needs just
**five small console-ROM helpers** — not the feared interpreter's worth — and
the **XB substrate** implements them at their authentic addresses, so a
user-supplied XB cartridge now runs end-to-end (PRINT, floats, RUN, LIST) on
the default clean-room boot, differentially gated. TI BASIC proper stays
deferred by policy. Dossier:
[XB-CENSUS.md](../../original-content/system-roms/XB-CENSUS.md); ledger:
[LIMITATIONS](../../original-content/system-roms/LIMITATIONS.md) L3
(resolved) + L9 (XB resolved).

**Landed 2026-07-07 — save states: atomic, portable, snapshots.** Every state
file is written **atomically** (temp file + rename, `config::write_atomic` —
the preferences use it too), so a crash or full disk mid-save can never
destroy the previous save. The **portability** half: the state file was
already self-contained and little-endian; format **v3** adds the
*cartridge's* host identity alongside the disks' (v2), so a loaded state
names its own media — the frontend no longer re-reads the cartridge file on
resume, and `last_cartridge`/`last_disk` in the preferences are just the
fallback identities for pre-v3 files. Identities are opaque labels, never
re-opened as paths; a regression test loads Windows- and POSIX-keyed states
regardless of host. The UX (Joel's spec, 2026-07-07): the automatic save is
named the **resume state** (`~/.libre99/resume.ti99`, adopted once from the
old `savestate.ti99` name) — auto-saved on exit, auto-loaded at launch,
saved/loaded live with `F6`/`F8`; **snapshots** are user-named `.ti99` files
through the OS-native dialogs (`Shift`+`F6` save, `Shift`+`F8` load, with a
native warning that loading replaces the resume state — which is rewritten
immediately after a successful load); and **`Shift`+`F5` is the fresh
start** — it deletes the resume state after a native warning that counts the
in-memory disk images (and unexported changes) it unloads, then restarts bare
like a first run. `F8` also warns first when loading would roll back
unexported disk changes.

**Landed 2026-07-07 — live disk mounting + disk persistence.** The former
**"mount a disk without rebooting"** blocker (*bug, Joel 2026-07-07*) and the
**"disk persistence — original untouched · tracked delta · export"** target
landed together. Disks now mount (`F9`) and eject (`F3`) **live** into the
running machine, like a real floppy; only a *cartridge* change still
cold-boots (the console scans cartridge ROM at power-up), and even that
reboot carries the whole disk subsystem across intact. The persistence model:
the host `.dsk` is **never written** — writes mutate the machine's in-memory
image (a whole mutated copy, keyed by the file's canonical path); an ejected
image moves to an in-memory **shelf** and reattaches, edits intact, when the
same file is remounted; save states (format v2 — v1 still loads) serialize
drives *and* shelf, so changed disks survive quit-and-resume. The new **`F4`
disk-memory overlay** lists every in-memory image (`CHANGED`/`CLEAN`),
**exports** one through the OS-native save dialog — whose own replace-prompt
is the guarantee that **no host `.dsk` is ever overwritten unprompted** — and
**unloads** one (native save-first/discard/cancel dialog when dirty) so the
next mount re-reads the host file. The `F1` help's stale "warm reset" wording
went with it.

**Landed 2026-07-07 — the Legal-notices blocker:** a single root
[`NOTICE.md`](../../NOTICE.md) now consolidates all legal notices, kept
distinct from `LICENSE.md` (the project's own grant): the **"Not affiliated
with or endorsed by Texas Instruments"** trademark disclaimer (TI marks used
nominatively only); the **Silkscreen** and **IBM Plex Mono** attributions
under the SIL **Open Font License 1.1** (pointing to the full texts already
shipped beside the fonts, not duplicating them); and the **Microban** level
credit (David W. Skinner) that the Sokoban cartridge already shows on screen.

## Decisions of record (all resolved)

> **Decided 2026-07-07 (owner):** TI PYTHON's 0.1.0 target — **grow it** into the small
> Python-like language specified in **[docs/TI-PYTHON.md](../TI-PYTHON.md)** (v1: full-size
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

> **Decided 2026-07-07 (release docs sweep):** the system-roms README's "no TI
> bytes" headline wording. The headline keeps its precise claim — the firmware
> contains **no Texas Instruments copyrighted bytes** — and the provenance
> section now states outright that **no authentic TI image is part of the
> repository** (user-supplied via flags; dev-machine tests load them from the
> git-ignored `third-party/`, skipping green when absent). This retired the
> last open decision on the 0.1.0 list.

## The former fast-cartridge-scan row (fidelity, firmware)

*(bug/roadmap note, Joel 2026-07-06; resolved 2026-07-07.)* Our rewritten
console GROM menu painted a **`SCANNING`** row while it built a cartridge's
program list; the **authentic** menu is fast and shows **no such word**.
Measurement (`tests/perf_parity.rs`) **corrected the "~1–2 s" premise**: the
isolated menu-build segment is only ~7 frames (~0.12 s), and our rewrite
already reaches the menu *sooner* than the authentic firmware overall (reset
→ cart listed ~30 vs ~54 frames). The banner was the only artifact that read
as slow, so it was **removed**, and the build is now hidden: `MENU` blanks
the display (VDP R1 `>A0`) while it scans and reveals the whole list at once
(`SDONE`/`DISPON`, the title screen's own idiom) instead of painting entries
in one at a time
(`original-content/system-roms/LIMITATIONS.md` **L5**). Speeding the `SCANW`
walk further was declined: ~0.12 s is imperceptible once hidden, and a
window-size change is not worth risking the 137-cart enumeration gate.
