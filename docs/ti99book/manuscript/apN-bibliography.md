# Appendix N — Annotated Bibliography and Resource Guide

<!-- Appendices · target ≈6 pp · drawn from every chapter's Further Reading · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. Period sources are the ledgered canon the chapters cite (Editor/Assembler Manual, TI Intern by Heiner Martin, the TI datasheets, MICROpendium) — named, not URL'd, as printed documents. The modern-corpus URLs were web-verified this session (js99er.net; Classic99 at harmlesslion.com; xdt99 at github.com/endlos99/xdt99 and endlos99.github.io/xdt99, GNU GPL, by Ralph Benzinger; Ninerpedia at ninerpedia.org; the AtariAge TI-99/4A Development forum; WHTech at ftp.whtech.com; 99er.net; nouspikel.com; MAME at mamedev.org). No unverified URL is printed (R-2); URLs are current as of writing and may move. The C-toolchain entries (tms9900-gcc, libti99) are cited as Ch. 44 verified them. Provenance/ethics (N.3) continue Ch. 1 and the repository's own clean-room stance. -->

A reading and resource list for going further, each entry with a note on *what to
read it for*. It gathers the Further Reading of forty-five chapters into two
shelves — the **period canon** (the documents written when the machine was
current) and the **modern corpus** (the tools, wikis, and communities that keep it
alive) — and closes on the provenance and ethics of using vintage materials
(N.3), the stance this whole project is built on. URLs were verified as this
appendix was written; the web moves, so treat a dead link as a search cue, not a
dead end.

## N.1 The period canon

The primary sources — tier-2 evidence throughout this book, and the final word
where the emulator and the datasheet disagree (Ch. 5's deviation rows).

- **_Editor/Assembler Manual_**, Texas Instruments. The single most-cited document
  in this book: the assembly-language environment, the console utilities (GPLLNK,
  XMLLNK), the file/PAB model, the floating-point package, KSCAN, and the object
  formats. *Read it for* the authoritative contract of every console service you
  call. The dialect `libre99asm` follows is this manual's (Ch. 6, 23, 31; App H, K).
- **_TI Intern_**, Heiner Martin. The definitive reverse-engineered map of the
  console ROM and hardware internals — scratchpad usage, the ROM/GROM routines,
  the timing. *Read it for* what TI never documented: the firmware's own byte-level
  behavior (Ch. 22–24; App C, K).
- **The TI datasheets** — TMS9900 (CPU), TMS9918A (VDP), TMS9901 (interface),
  SN76489 (sound), TMS5220 (speech). *Read them for* the ground-truth register,
  timing, and encoding facts the appendices tabulate (App A, D, E, F, G). Where
  this book prints a hardware number the emulator does not yet match, the datasheet
  is the authority (the `_ledger.md` deviation pattern).
- **_MICROpendium_** (1984–1999) and the period magazines. The community's
  contemporary record — type-in programs, hardware reviews, technique articles.
  *Read them for* the era's own voice and the problems people actually solved; much
  is now archived online (N.2). Earlier general-market coverage (*99'er* / *Home
  Computer Magazine*, and TI-related pieces in the general home-computer press of
  the day) rounds out the picture — hedge specific citations to the issue.

## N.2 The modern corpus

The living ecosystem — verify current addresses before relying on them.

**Cross-development tools**

- **xdt99 — TI 99 Cross-Development Tools** (Ralph Benzinger; GNU GPL; Python).
  `github.com/endlos99/xdt99`, docs at `endlos99.github.io/xdt99`. The suite the
  book reaches for beyond its own cartridge path: `xas99` (full E/A object model,
  EA5), `xga99` (GPL), `xdm99` (disk images), and disassemblers. *Read it for* the
  object formats and disk surgery `libre99asm` does not emit (Ch. 6, 31–32; App L).
- **A GCC port for the TMS9900** and **`libti99`** (Tursi's C library). The modern
  C path onto the machine, surveyed in Chapter 44 (verify the community
  repositories' current state before invoking — the toolchain moves). *Read them
  for* writing 4A code in C and calling `lib99`-style routines from it.

**Emulators (the reference shelf, Ch. 18/34/44; App L)**

- **Classic99** (Mike Brent, "Tursi"; Windows). `harmlesslion.com`. Hardware-verified
  against real iron — *the* arbiter this project consults for subtle CPU/VDP/GROM
  behavior. *Read/run it for* a second opinion trusted over our own when they
  differ, and for its debugger.
- **JS99'er** (Rasmus Moustgaard; browser). `js99er.net`. Runs the **F18A**
  enhanced VDP. *Run it for* the modern-VDP features (Ch. 18, 34) the project core
  does not emulate — no install needed.
- **MAME** (the TI-99 driver). `mamedev.org`. The breadth reference. *Run it for*
  the family's edges — the Geneve 9640, SAMS, peripheral cards (Ch. 34, 44).

**Community hubs and references**

- **AtariAge — TI-99/4A Development forum** (`forums.atariage.com`). The active
  center of gravity: current projects, hardware, and expertise. *Read it for* live
  answers and the state of the art.
- **Ninerpedia** (`ninerpedia.org`). A MediaWiki wiki of the whole TI-99 line.
  *Read it for* structured reference on hardware, software, and history.
- **WHTech** (`ftp.whtech.com`). The primary archive of nearly everything TI —
  software, manuals, magazines, hardware docs. Vast and lightly organized. *Read it
  for* the source scans the period canon (N.1) ultimately comes from.
- **99er.net** and **nouspikel's technical pages** (`nouspikel.com`). *Read the
  first for* general TI-99/4A history and links; *the second for* deep,
  well-regarded hardware technical write-ups.

## N.3 Provenance and ethics

A note the rest of the shelf earns. Much of what survives — ROM images, cartridge
dumps, scanned manuals — exists in a grey zone: preserved by a devoted community,
but not, in the main, cleared for redistribution by whoever holds the rights.
This book takes the honest position throughout (R-1, R-12): consult primary
sources, describe behavior, and **reproduce, never copy**.

- **Firmware and cartridge images** (`roms/*.Bin`, commercial `.ctg`) are *not*
  this project's to license or redistribute; they are excluded from any public
  release (the standing checklist is in the repository's `docs/DEVELOPMENT.md`).
  When the book studies a first-party title, it does so by *observation* —
  behavioral description, never a transcript of TI's code (the discipline of
  Part IX's archaeology sections).
- **The clean-room console ROM/GROM** under `original-content/system-roms/` is the
  ethical showpiece: an original firmware, written from behavior and differentially
  verified, that boots the machine without a proprietary byte (Ch. 28). It is *why*
  this project can be source-available (the Modified MIT + Commons Clause license,
  `LICENSE.md`) where a firmware dump could not.
- **When you build on the corpus**, credit maintainers, keep licenses with their
  code, and prefer the clean path — the project's own toolchain and clean-room
  firmware — so what you publish is yours to publish. That stance, more than any
  single tool, is what Chapter 45 argues keeps a dead machine alive honestly.

*See also:* every chapter's Further Reading (the per-topic sources), Appendix L
(the toolchain these resources complement), and Chapter 45 (preservation and the
community as the platform's future).
