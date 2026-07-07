# CLAUDE.md — Project Instructions for *Programming the TI-99/4A*

You are the writing partner for a book-length technical manuscript: **"Programming the TI-99/4A: Assembly Language and GPL, from Silicon to Software."** This file tells you how to pick up the project and continue it. Read it fully before doing anything else.

## What this project is

A ~1,050-page textbook teaching TMS9900 assembly and GPL programming on the TI-99/4A, aimed at modern CS undergraduates with no vintage-systems background. It deliberately does NOT teach TI BASIC or Extended BASIC (TI BASIC is examined only as a GPL artifact in Ch. 28). The goal: a reader who finishes it could recreate any legacy commercial TI-99 program. It includes history and human-interest material, not just technical content.

**The book is founded on the Libre99 project** — the repository this folder lives inside (`docs/ti99book/`). The project's desktop emulator (`libre99`, crate `libre99-app`), assembler (`libre99asm`, crate `libre99-asm`), GPL toolchain (`libre99gpl`, crate `libre99-gpl`), and emulator core (`libre99-core`) are the book's machine and toolchain; **BENCH99** (`code/bench/`, a scriptable monitor over `libre99-core`) is its debugging/measurement instrument. Classic99, js99er.net, MAME, and the xdt99 suite are discussed tools with fixed shelf roles (see `_style.md` R-12). The book and the emulator co-evolve: when a chapter needs a capability the project lacks, the chapter states the gap and names the shelf tool that covers it (never assert unshipped features) — and the gap is worth surfacing to Joel as possible project roadmap work.

Full scope, all 45 chapters + 14 appendices, the pedagogical template, and the production plan live in **`manuscript/00-master-outline.md`** (v1.0 + the v1.1 re-founding amendment at the end). That file is the contract. Follow it.

## Session protocol — HOW TO CONTINUE (do this every time)

1. **Load context first, before writing anything.** Read, in this order:
   - `manuscript/00-master-outline.md` — the chapter's spec (find the `### Chapter N` section) **and the Amendments section at the end**.
   - `manuscript/_style.md` — binding style rules and accrued rulings R-1…R-n. Obey all of them (R-12…R-15 govern the toolchain/foundation).
   - `manuscript/_ledger.md` — every address, date, figure, and term the book has already asserted. **Never contradict it.** Reuse the exact forms logged there.
   - `manuscript/_summaries.md` — the summary block of every chapter written so far, for continuity and forward/back references.
   - The emulator project's current state where the chapter touches it: the repo `README.md`, `docs/STATUS.md`/`ROADMAP.md`, and the relevant `crates/` source. **Verify against HEAD, not memory** — the project is actively developed (often by parallel sessions).
2. **Confirm the next chapter.** Every chapter file now exists in one of three states (STATUS comment at the top of each file — see `manuscript/_stubs.md`, the stub-system manual): `DRAFTED` (ch01–ch04), `STUB` (Phase-2 populated: final-voice narrative + OPUS-TODO work orders), or `STUB-SKELETON` (structure only). The next chapter to DRAFT is the lowest-numbered non-DRAFTED one along the spine: 3→4→5→6→7→8→9→12→13→16→17… (dependency graph at the end of the outline). **Chapter 5 is next**, and it is a populated STUB — follow `_stubs.md` §2 to finish it. Alternatively a session may continue POPULATING skeletons into stubs per `manuscript/_stub-steering.md` (the exact template + per-chapter steering).
3. **Write one chapter per session** (target page counts are in the outline; heavy chapters may take two sessions — split at a section boundary and note it in the summary).
4. **Verify all code** (see "Toolchain" below). Every assembly listing in a chapter must assemble cleanly with `libre99asm` before the chapter is considered done — and behavioral claims should be exercised on BENCH99 where practical (the strongest evidence tier). Fix the manuscript if it doesn't check out.
5. **Update the three support files** at the end of every chapter:
   - Append any new rulings to `_style.md` (continue the R-n numbering; current max is **R-15**).
   - Append every new asserted address/date/figure/term to `_ledger.md`. If you machine-verified something, say so in the Notes column **with the repo commit** (this is the strongest evidence tier).
   - Append the chapter's `## Summary` block verbatim to `_summaries.md`, with a one-line status header (see existing entries for the format).
6. **Save each source listing** into `code/chNN/` as a real `.a99` file, so the companion tree grows with the book.

## Style rules that matter most (full set in `_style.md`)

- TI hex is `>XXXX`. Registers `R0`–`R15`. Mnemonics/directives UPPERCASE.
- Assembler baseline is Editor/Assembler-compatible source assembled with `libre99asm`. Anything beyond E/A (long labels, `LABEL:`, `;` comments, predefined registers leaned on, binary `:` constants) is flagged inline with `[libre99asm]` (R-13).
- Entry point is the label `START` by convention (libre99asm's default); `DEF` belongs to Ch. 6's E/A world (R-15). Trailing comments on no-operand mnemonics need `;`.
- Em dashes are set OPEN with spaces (word — word).
- Diagrams are ASCII art + Markdown tables only. No image assets. (PAL coverage = sidebars only.)
- Chapter file template (opening vignette → What You Will Learn → The Bridge → numbered sections → sidebars/Field Notes/Pitfalls as blockquotes → Lab → Exercises with ✦/✦✦/✦✦✦ tiers → Further Reading → Summary). Match the structure of ch01–ch04 exactly.
- Prose, not bullet-salad. The tone is rigorous but warm; humor stays in vignettes and sidebars.
- Any invented period document (memo, ad, price sheet) MUST be labeled a reconstruction/composite in its first line (ruling R-1).
- Hedge figures not traceable to a primary source ("reported," "roughly") and log the hedged form in the ledger (R-2).
- Cite the project README for emulator hotkey/flag detail instead of restating it at fragile precision (R-12).

## Toolchain — how to verify code

The toolchain is the enclosing repository's own, built from source (no downloads):

```
sh setup.sh        # builds libre99asm, the desktop emulator, and BENCH99; smoke-tests hello.a99
sh verify.sh       # assembles every .a99 under code/ with libre99asm + builds bench99; run before finishing a chapter
```

(`make setup` / `make verify` delegate to the same scripts where `make` exists; the scripts need only `sh` + `cargo`.)

Canonical invocations (R-14):

```
../../target/release/libre99asm src/foo.a99 --name 'TITLE' -o build/foo.ctg --listing build/foo.lst --symbols build/foo.map.json
../../target/release/libre99asm src/foo.a99 --name 'TITLE' --format bin -o build/FOOC.bin    # 8 KiB-padded ROM image
code/bench/target/release/bench99 [script]   # the lab bench: load/boot, pc/wp, s/u/x, r/m/pw/pb, screen, vdp
```

Run the emulator itself with `cargo run --release -p libre99-app -- --cartridge-file build/foo.ctg` (from the repo root). If a listing in the manuscript won't assemble, the manuscript is wrong — fix the prose/code, don't fudge the check. Measured figures printed in the book should come from actual bench transcripts (see the `Machine-verified (session 3)` ledger rows for the pattern, including how to log an emulator-vs-hardware deviation — one is currently open: libre99-core's `MOV`/`MOVB` omit the destination pre-read, so placement timings can read 4 cycles low per re-read destination; Ch. 5 must confront it).

**Sibling-session etiquette:** the emulator is developed in this same repository, often concurrently. Book sessions write only under `docs/ti99book/`; check `git status` at session start and never commit/push over a sibling's in-flight work.

## What NOT to do

- Don't teach TI BASIC or Extended BASIC as languages (Ch. 28 dissects TI BASIC as an artifact; one fenced XB CALL LINK page appears in Ch. 36 — that's the whole allowance).
- Don't contradict `_ledger.md`. If new research forces a change, update the ledger entry AND note the correction.
- Don't assert emulator/assembler capabilities beyond what the project ships at HEAD (R-12). State gaps plainly; name the shelf tool that covers them.
- Don't invent quotes, fabricate credits, or present reconstructed documents as authentic.
- Don't reformat or "improve" already-drafted chapters unless explicitly asked; they're done pending a later review pass.
- Don't skip the support-file updates. They are the memory that makes the next session work.
- Don't touch anything outside `docs/ti99book/` from a book session.

## Licensing and versioning

**Licensing.** The project is the **Modified MIT License with Commons Clause**
(`LICENSE.md` at the repo root). Every companion-code source we author carries
a license header — no "all rights reserved" notices anywhere. The header form
by file kind: the assembly/GPL listings (`code/ch*/*.a99`, `*.inc`, `*.gpl`)
carry the **two-line pointer**

```
* Copyright (c) 2026 Joel Odom. Licensed under the Modified MIT License
* with Commons Clause — see LICENSE.md at the repository root.
```

(matching the project's firmware `.asm`/`.gpl` convention); the bench Rust
(`code/bench/**/*.rs`) carries the **full text of `LICENSE.md`** as a `//`
header. **Add the right header to every new source file.**

**Versioning.** The book is versioned to **track the Libre99 project version** —
the single source of truth is the workspace `version` in the repo-root
`Cargo.toml` (**currently 0.0.1**). The book's `README.md` shows that number;
keep it in sync when the project bumps (the book releases in lock-step with the
toolchain it documents). This is a *release* version — distinct from the
outline's editorial `v1.x` amendments, which track manuscript revisions, not
releases.

## Current status (update this line each session)

Drafted: **Ch. 1–5, 7–40** — Part I complete; **Part II complete except Ch. 6**; **Part III COMPLETE**; **Part IV COMPLETE**; **Part V COMPLETE — Ch. 21-24 done**; **Part VI COMPLETE — Ch. 25-29 done**; **Part VII COMPLETE**; **Part VIII COMPLETE**; **Part IX UNDERWAY — Ch. 39-40 (METEOR BELT, GRIDRUNNER 99) done** (2026-07-07). Rulings through **R-19** (R-16 = R10 software stack + calling convention, Ch. 9; R-17 = include architecture + `lib99`, Ch. 11; R-18 = Part III pixel-oracle graphics verification, Ch. 14; R-19 = the Part IX capstone standard — case-study arc + deterministic-engine bench verification + archaeology honesty, Ch. 39). Ledger/summaries current, Part III rows commit-stamped: Ch. 12 @ e97e8ce, Ch. 13 @ 31417ef, Ch. 14/15 @ 0d3e5d5 (re-confirmed against the sibling's beam-accurate rasterizer bd1bbb6). **`lib99` grows**: `memlib`/`mathlib` (Ch. 7–8), `equates`/`assert` (Ch. 11), `vdplib` (Ch. 12), `textlib` (Ch. 13), `textlib40`/`mcolib` (Ch. 14), `bmplib` (Ch. 15), `spritelib` (Ch. 16), `sndlib` (Ch. 19), `spklib` (Ch. 20), `inplib` (Ch. 21), `gromlib` (Ch. 25); profile99/fxcalc/padwatch/gromdump + scroll demos. **BENCH99 gained** `vram` (Ch. 12) , `pixels`+mode-aware `screen` (Ch. 14), `sound` (Ch. 19), `press`/`rel` (Ch. 21), and `gromlog` (Ch. 26) — the graphics oracles (R-18), both using only libre99-core's public API. **Ch. 6 is DEFERRED** — its object-format/loader legs need python/xdt99 + Classic99, absent on the PC workstation; draft it on the Mac (E/A manual PDF: keep a local copy in the git-ignored `third-party/` — it is TI's and is not tracked; the ch06 stub carries the run-where-python-exists notes). **Parts III-VI are COMPLETE; Part VII COMPLETE; Part VIII COMPLETE (Ch. 35-38 done); Part IX UNDERWAY — Ch. 39-40 (Capstone I: The Scrolling Shooter, METEOR BELT; Capstone II: The Fixed-Screen Arcade, GRIDRUNNER 99) done (2026-07-07).** Next along the spine: **Chapter 41 (Capstone III: The Data-Driven RPG Engine, DUNGEONS OF FATE)** — `[disk, 32K]`, an engine that plays a quest database (Tunnels of Doom lineage); the capstones instantiate SKELETON99 (code/ch36) + lib99 into complete games (Archaeology/Spec/Construction/Postmortem arc, codified as **R-19**). **Ch. 40 (GRIDRUNNER 99) is machine-verified at commit 408a451**: a console-only maze chase whose WALLS ARE AN ALGORITHM (WALLAT = border + odd/odd posts, zero bytes of RAM) and whose PELLET DATABASE IS THE NAME TABLE (VSBR to test, VSBW to eat) — so all logic fits the 256-byte pad (5 actors + ~16 scalars); target-tile AI with four personalities (chase/ambush/flank/shy) + scatter/chase mode timer + frightened flee (the min-distance loop with the compare flipped); three-channel sound + recorded-input attract; assembles to an 8,192-byte SINGLE bank (entry >61FE, ~3.4 KB, no banking needed), 11-part deterministic self-test GREEN (VR7=>02, FAILID=0). §40.6 lesson: the R-16 calling convention is load-bearing under famine (three real clobber bugs — R11 in fake-leaf sound helpers, R3 across CELLAD, R4/R8 across STONE/ENAI — all fixed). **Ch. 39 (METEOR BELT) is machine-verified at commit 18c069e**: meteorbelt.a99 = a complete scrolling-shooter engine (state machine + data-driven wave director + fixed-slot entity tables + 8.8 STEP + bounding-box collision + laser-heat lockout + fuel + lives), assembling to an 8,192-byte single-bank image, deterministic self-test GREEN (FAILID=0, VR7=>02), one PLAY frame ~16,000 cyc (~⅓ of the ~50,000-cyc budget), SCROLL (Ch.17 pattern-shift) 988 cyc. **R-19 archaeology honesty**: the project ships NO cartridge image (cartridges/ empty, IP-clean) → the genre is reconstructed from the record + Part III measurements, NOT a playthrough. Placement is bench-verified (2818 vs 3622 cyc); R-15 MPY/DIV flat-cost deviation logged. SKELETON99 (code/ch36) = the Part IX chassis; new Part VII-VIII code: dsrlib/filelib/seclib/termlib/samslib/bankcart/skeleton99/placebench; Ch. 39-40 add `meteorbelt`/`gridrunner` (Ch. 6 still DEFERRED to the Mac). **Part VII verification tier (important):** disk/DSR/PAB is machine-verified via the **Rust test harness** (`cargo test -p libre99-gpl --test device_io` + `-p libre99-core --test disk`, both GREEN at HEAD) and by **decoding real `.dsk` images** (`disks/*.Dsk`) + the probe-pinned `original-content/system-roms/disk-dsr/RECON.md` facts — **NOT BENCH99** (no disk/DSR command; `>4000` is open bus there — a stated gap). Code artifacts (`dsrlib` etc.) assemble via `libre99asm` and follow the verified contract. **RS-232/9902, SAMS, F18A (plain 9918A only), TIPI, cassette, the 9901 interval timer are all UNEMULATED** — describe from the hardware record + name shelf tools (Classic99/MAME/js99er). The emulator models the **disk card only** at `>4000` (no general DSR bus). **Sibling is actively building the clean-room disk DSR** (`disk-dsr.asm` etc. uncommitted in-tree) — cite committed HEAD only; stage ONLY `docs/ti99book/` files. **`libre99gpl` in play** (Ch. 26-29): asm/dis/console; the clean-room console GROM (`console.gpl`) boots + its menu discovers programs by the >AA scan. New R-12 gaps surfaced this session: **libre99gpl builds console GROMs (24 KiB) not plug-in cartridge GROMs at >6000 / no .ctg-.rpk emitter** (Ch. 27 §27.5 — a clear roadmap item); 9901 interval timer + speech synth still unemulated (Parts IV-V). Part III added `pixels`+`vram`+mode-aware `screen` to BENCH99 and libs `textlib40`/`mcolib`/`bmplib`/`spritelib` to lib99; all Part III numbers re-confirmed against the sibling beam rasterizer bd1bbb6. Open for Joel: two libre99-core deviations (MOV/MOVB omit the destination pre-read; DIV success cost modeled flat) + three R-12 bench gaps (no unexpanded-console model; no DSR-ROM install from BENCH99; 9901 cassette-relay output unemulated); NOTE a sibling session is mid-refactor of the VDP rasterizer (whole-frame → per-scanline beam), behavior-preserving. State check: `grep -l "STUB-SKELETON" manuscript/*.md` lists what still needs population.

## Repo layout

```
manuscript/   the book: 00-master-outline.md, chNN-*.md, apX-*.md, _style.md, _ledger.md, _summaries.md, _stubs.md (stub-system manual), _stub-steering.md (population template + steering)
code/         bench/ (BENCH99) · chNN/ per-chapter .a99 sources · lib99/ the running library (starts Ch. 11) · tools/ asset scripts (Ch. 38)
assets/       binaries, images produced by builds
build/        scratch build output (git-ignored)
setup.sh      one-time toolchain bring-up (builds the enclosing repo's tools)
verify.sh     assemble all chapter code + build the bench; the pre-finish check
Makefile      thin delegate to the two scripts, for `make` users
```
