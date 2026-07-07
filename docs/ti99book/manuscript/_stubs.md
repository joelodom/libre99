# `_stubs.md` — The Stub System: How the Remaining Chapters Get Finished

*Created 2026-07-05. Chapters 1–4 are drafted; chapters 5–45 and appendices A–N exist as stubs. This file is the operating manual for any session that touches a stub. Load it together with `CLAUDE.md`, `00-master-outline.md`, `_style.md`, `_ledger.md`, and `_summaries.md`.*

---

## 1. What a stub is

Every chapter file declares its state in a STATUS comment near the top:

| STATUS | Meaning | Produced by |
|---|---|---|
| `STUB-SKELETON` | Structure only — template headings plus a pointer to the chapter's outline spec. No content. | Phase 1 (2026-07-05) |
| `STUB` | Narrative fully written in the book's voice; every technical core is an `OPUS-TODO` work order. | Phase 2 (Fable, 2026-07-05) |
| `DRAFTED` | A real chapter: work orders executed, code machine-verified, support files updated. | A finishing session — you |

The narrative in a `STUB` was drafted by the same model that wrote ch01–ch04, specifically so the book keeps one voice even though many sessions will finish it. **Keep that prose.** Edit it only where a fact turns out wrong, where executed technical content forces a seam, or where flow demands a stitch — never for taste. Your job in a stub is the technical content, the verification, and the joinery.

If you arrive at a file still marked `STUB-SKELETON`, don't wait for a Phase 2 that will never come: write the chapter directly from its outline spec, imitating ch01–ch04 and the voice guide below.

## 2. Finishing protocol (STUB → DRAFTED)

`CLAUDE.md`'s session protocol applies in full — load order, one chapter per session, verify-at-HEAD, support-file updates, sibling-session etiquette. The stub-specific steps:

1. **Pick the chapter**: the lowest-numbered non-`DRAFTED` chapter along the outline's spine (3→4→5→6→7→8→9→12→13→16→17, then the dependency graph at the end of the outline). Never draft a chapter whose prerequisites are still stubs. **Chapter 5 is first.**
2. **Read the whole stub before writing anything.** Its OPUS-TODOs were written as one coherent plan; executing them piecemeal breaks cross-references between sections.
3. **Execute each `OPUS-TODO` in place**, then delete the tag. The order's VERIFY clause is binding: code that doesn't assemble with `libre99asm`, or a measurement you didn't actually take on BENCH99, may not be asserted in prose (R-15). Where the project toolchain lacks a capability an order assumes, follow R-12 — state the gap plainly, use the named shelf tool, log it in the ledger, and surface it in your final report as possible project roadmap work.
4. **Resolve every `OPUS-VERIFY` inline flag**: check the claim (ledger, primary source, or machine), fix or hedge the sentence per R-2, delete the tag.
5. **Save every listing** into `code/chNN/` as a real `.a99` (or `.gpl`) file and make `sh verify.sh` pass.
6. **Finalize the Summary** from the `SUMMARY-DRAFT`, append it verbatim to `_summaries.md` with a one-line status header (match the existing entries); append new facts to `_ledger.md` — machine-verified rows cite the repo commit — and new rulings to `_style.md` (numbering continues from the current max, R-15 as of this writing).
7. **Flip STATUS** to `DRAFTED (session N, YYYY-MM-DD) — pending review passes.` Then audit: a search for `OPUS-` in the file must return nothing.
8. **Commit** only your chapter file + `code/chNN/` + the three support files, message `book: draft chNN — <short title>`. Do not push over sibling sessions.

## 3. Tag grammar

Four tags, all HTML comments so they render invisibly:

**The STATUS header** (top of file, after the title/epigraph block):

```
<!-- STATUS: STUB (Phase 2 populated, 2026-07-05) — narrative is final-voice; technical cores are OPUS-TODO work orders. Protocol: _stubs.md. -->
```

**`OPUS-TODO` — a work order.** Self-contained: a finishing session should be able to execute it without asking anything. Anatomy:

```
<!-- OPUS-TODO ch07-memcpy [listing+measure]:
DELIVER: the section's technical core — a copy loop built up in three forms (indexed, autoincrement,
  unrolled×2), each shown as a fenced asm listing with surrounding explanation.
CODE: code/ch07/memlib.a99 — E/A-compatible, START entry, WS in scratchpad; assemble per R-14.
VERIFY: libre99asm clean; on BENCH99: load the .bin, `u` to the loop head, `s 20`, read per-instruction
  cycles off the trace. Expected: autoincrement form beats indexed by ~N cycles/word (measure N).
LEDGER: add rows for each measured loop cost, Notes = "machine-verified (bench), commit <hash>".
PROSE: results land in two short paragraphs after the third listing; keep the "pump" metaphor
  already set up by the section narrative. Budget ≈ 350 words + listings.
-->
```

Field meanings — `DELIVER` (what to write), `CODE` (file and constraints), `VERIFY` (the binding evidence bar), `LEDGER` (what the ledger gains), `PROSE` (where it lands, what tone/hooks to preserve, size). Not every order needs every field; every order needs DELIVER and VERIFY (VERIFY may be `n/a (prose only)` for pure-prose work like a Further Reading list).

**`OPUS-VERIFY` — an inline flag** on a single narrative claim the Phase-2 writer could not check:

```
...the controller retries a read reportedly up to five times <!-- OPUS-VERIFY: retry count — check FD1771 datasheet / Classic99 source --> before reporting an error.
```

**`SUMMARY-DRAFT`** — marks the Summary block as provisional until the chapter's measurements exist. Finalize, then remove the marker.

## 4. Voice — how this book sounds

The narrative register was set by ch01–ch04. Calibrate on these two excerpts before writing.

From the Ch. 1 prologue (history register):

> Here is the strange part, the part that makes this machine worth a thousand-page book: the TI-99/4A did not stop selling. It sped up. TI cut the console loose at clearance prices — $49.95 was a common sticker that Christmas — and Americans who had watched two years of brutal price-war advertising finally pounced. In the last weeks of 1983, a discontinued computer was one of the hottest gifts in the country.

From the Ch. 3 bridge (technical register):

> Two habits *don't* transfer, so install them now. First: here, **the emulator is not a compromise, and it is not a black box.** The machine you will run all book long is a from-scratch, cycle-aware software 99/4A whose source sits in the same repository as this book — when Chapter 5 measures wait states or Chapter 22 profiles an interrupt handler, you can read the very Rust that modeled them... "Runs in the emulator" is evidence, not hope.

The working rules behind that sound:

- **Prose, not bullet-salad.** Bullets are for objectives, checklists, and reference tables. Everything else is paragraphs that carry an argument. Second person for the reader ("you will measure"), first-person plural for the book's program ("we adopt one convention and keep it").
- **Rigorous but warm.** Humor and human texture live in vignettes and sidebars only — never in reference tables or instruction semantics. No exclamation points doing the work of evidence.
- **Concrete before general.** Lead with the artifact — an address, a trace, a price sheet, a listing — then name the principle. The book's beloved moves: the measured number over the folklore number; the "this is the paycheck" aside *sparingly*; naming a law once and reusing the handle (see canon card).
- **Em dashes set open** (word — word), per R-6. Cross-references as "(Ch. 12)" / "(§17.5)", forward references promise, never spoil (R-7). TI hex `>XXXX`; mnemonics UPPERCASE; K = 1024.
- **Honesty tics**: hedge non-primary figures ("reported," "commonly cited," R-2); label invented period artifacts as reconstructions (R-1); state project gaps plainly and shelve them (R-12); assert hardware truth where the emulator deviates, with the deviation noted (R-15).
- **Vignettes** are ½–1 page scenes with a specific time and place, real people named only where the record names them, ending with a `---` rule. They earn the chapter, not decorate it: the scene must set up the exact anxiety the chapter resolves.
- **Summary blocks** are 6–10 dense lines, written to prime future sessions (match the register of `_summaries.md`).

## 5. Canon card — facts and handles Phase 2 narratives may assert

Anything *not* on this card, not in your chapter's outline spec, and not common knowledge safely hedged → `OPUS-VERIFY` it or push it into an `OPUS-TODO`. The full record is `_ledger.md`; finishing sessions load it and must not contradict it.

**Named handles (R-8 canon — reuse, never re-coin):** the **funnel** / mail slot (8-bit multiplexer between the 16-bit CPU and most of the machine); the **fast island / fast domain** (console ROM + scratchpad, the only 16-bit zero-wait territory); the **tower of interpreters** (Floor 1 = 9900, Floor 2 = GPL, Floor 3 = TI BASIC); **CQ-82** (the 9-item 1982 commercial-quality checklist, Part IX's rubric); the law **"speed is a property of addresses, not instructions"**; the **high-byte law** (byte ops on registers use the HIGH byte); **R10 = software stack pointer** (book-wide, built Ch. 9); **`lib99`** (the reader's accumulating library, born Ch. 11).

**Running artifacts by origin chapter:** BENCH99 (Ch. 2–3, exists in `code/bench/`), HELLO (Ch. 3), trace.a99 (Ch. 4), timing rig (Ch. 5), `memlib` (7), `mathlib` (8), `task99` (9), CRU explorer (10), `SYSCHK` + lib99 formalized (11), `vdplib` (12), `textlib` + `MONITOR99` (13), `textlib40` + `PLOT64` (14), `bmplib` (15), `spritelib` + **DODGE** (16), time system + **TERRAIN** (17), split-screen scoreboard (18), `sndlib` (19), `spklib` (20), `inplib` (21), user-ISR music + `PROFILE99` (22), FP calculator (23), `PADWATCH` (24), `gromlib` + `GROMDUMP` (25), GPL hand-assembly (26), **QUIZMASTER** (27), boot trace (28), XML hybrid (29), `dsrlib` + `NULCARD` (30), `filelib` + **FILER99** (31), `seclib` + **DISKDOC** (32), **TERM99** (33), SAMS streamer (34), banked DODGE (35), **SKELETON99** (36), optimization dojo (37), asset crunch (38), **METEOR BELT** (39), **GRIDRUNNER 99** (40), **DUNGEONS OF FATE** (41), **AUTHOR99** (42), the port (43). Case-study names are R-4 placeholders — use them.

**Machine facts (ledgered):** CPU TMS9900 @ 3.0 MHz; PC/WP/ST only; Rn = WP+2n; big-endian; word ops ignore A15; vectors `>0000`–`>003F`, XOP `>0040`–`>007F`; RT = `B *R11` = `>045B`; `JMP $` = `>10FF`. Memory: console ROM 8 K `>0000` (16-bit, holds GPL interpreter + ISR + FP pkg); low expansion 8 K `>2000`; DSR window `>4000`; cart ROM `>6000`–`>7FFF`; scratchpad **256 B `>8300`–`>83FF`, 16-bit zero-wait**; high expansion 24 K `>A000`. MMIO: sound `>8400`(w); VDP `>8800` rd data / `>8802` rd status / `>8C00` wr data / `>8C02` wr addr; speech `>9000` rd / `>9400` wr; GROM `>9800` rd data / `>9802` rd addr / `>9C00` wr data / `>9C02` wr addr — all side-effectful. GPL workspace `>83E0`, GPL status byte `>837C`. VRAM = 16 K private to the VDP; frame = 60 Hz, **50,000 CPU cycles**. Timing model **T = C + 4 × (accesses in the 8-bit domain)**; measured: `JMP $` 10 cycles in pad / 14 in expansion; `MOV R,R` 14 (all-pad) / 18 (code slow, WS pad) / 30 all-slow (datasheet truth — see the OPEN deviation below).

**History facts (ledgered, hedged forms):** TMS9900 1976; 99/4 June 1979, $1,150 with monitor; 99/4A June 1981, $525; price path $525→~$299→rebates→$99→$49 clearance; Black Friday = Friday, **October 28, 1983**; write-off reported ≈$330 M; ≈2.8 M consoles commonly cited; Speak & Spell 1978 (Breedlove/Wiggins/Frantz/Brantingham, LPC); *TI Intern* (Heiner Martin); MICROpendium 1984–99; Chicago Faire annual since Nov 1983; Geneve 9640 (Myarc, 1987). First-party titles safe to name: Parsec, TI Invaders, Munch Man, Alpiner, Tunnels of Doom, TI-Writer, Editor/Assembler, Mini Memory, Terminal Emulator II.

**The project (verify at HEAD before asserting more — R-12):** pure-Rust from-scratch 4A running the real firmware; crates `libre99-core` (pure std), `libre99-app` (desktop `libre99`), `libre99-asm` (`libre99asm`), `libre99-gpl` (`libre99gpl`); 137 cartridges + 15 disks embedded; F9 media browser, F5 reset, F6/F8 savestates, F10/F12/Tab time control, CPU inspector, logging (cite README for detail). **Gaps as of 2026-07-05:** no speech emulation; embedded disks only (DSK1); GUI breakpoints on roadmap — BENCH99 covers break/trace/poke. **libre99asm:** E/A-compatible; R0–R15 predefined; entry = `START`/`END` operand/`--entry`; auto-synthesized `>6000` header; `.ctg` default, `--format bin` = 8,192-byte padded image; `--listing`/`--symbols`; `dis` subcommand; `COPY 'file'` single-quoted; **no** DEF/REF/RORG/DORG/BES/macros/tagged-object/EA5 (xas99 covers those, Ch. 6); `AORG` absolute mode only; trailing comment on a no-operand mnemonic needs `;`. Project originals: **TITRIS** (complete libre99asm-built game, `original-content/cartridges/titris/`) and the **clean-room console GROM** (original title/menu + TI PYTHON REPL, boots via `--system-grom`; Ch. 28 §28.7 tours it).

**OPEN DEVIATION (ledgered):** libre99-core executes `MOV`/`MOVB` without the 9900's destination pre-read — all-expansion `MOV R,R` measures 26 where hardware says 30 (`A R1,R2` correctly measures 30). Body prose asserts hardware truth; bench numbers that re-read a destination inherit the caveat. Ch. 5 confronts this first; check whether HEAD has fixed it before writing any placement-timing number.

## 6. Verification quick reference

```
sh setup.sh                        # once: builds libre99asm, the emulator, BENCH99
sh verify.sh                       # before finishing: assembles all code/, builds bench
../../target/release/libre99asm src/foo.a99 --name 'TITLE' -o build/foo.ctg \
    --listing build/foo.lst --symbols build/foo.map.json          # R-14 canonical
../../target/release/libre99asm src/foo.a99 --name 'TITLE' --format bin -o build/FOOC.bin
cargo run --release -p libre99-app -- --cartridge-file build/foo.ctg  # from repo root
code/bench/target/release/bench99 script.txt   # load/boot · pc/wp · s/x/u · r/m/pw/pb · f/k · screen · vdp · cycles
```

Evidence tiers, strongest first: (1) machine-verified on the project toolchain at a stated commit; (2) primary period source (datasheet, E/A manual, *TI Intern*); (3) Classic99/MAME behavior; (4) community record, hedged. Print numbers only from tier 1–2; when tiers disagree, say so (the deviation-row pattern in `_ledger.md` is the template). Bench scripts on Windows want `C:/...` paths, not `/c/...`.

## 7. Worked example — what a populated section looks like

Narrative (final voice, stays):

> The race is almost embarrassingly easy to set up, which is the point: on this machine you do not *theorize* about memory speed, you clock it. We plant the same two-instruction loop at `>8300` and at `>A000`, aim the bench at each, and let the cycle counter arbitrate. The loop does no work at all — it just runs — so every cycle it costs is pure geography.

Work order (executes, then vanishes):

```
<!-- OPUS-TODO ch05-race [measure]:
DELIVER: the §5.3 measured table — one loop, every region: >8300, >2000, >6000 (cart), >A000, plus
  fetch-vs-operand split demonstrated with MOV variants.
CODE: code/ch05/rig.a99 (the timing rig per §5.7's worksheet hooks).
VERIFY: BENCH99 bare mode; `pw`/`pc`/`s` per region; record per-instruction cycles from the trace.
  CHECK FIRST whether the MOV dest-pre-read deviation (canon card §5) is fixed at HEAD; if not,
  use A/S-family probes where a destination re-read matters and state the caveat once.
LEDGER: upgrade the "Multiplexer cost ≈ +4 nominal" row to measured per-region figures, commit-stamped.
PROSE: table + ~200 words of reading-the-table; land the law "speed is a property of addresses."
-->
```

## 8. Don'ts

- Don't rewrite Phase-2 narrative for taste; don't let a chapter contradict `_ledger.md`; don't teach BASIC.
- Don't assert unverified numbers, unshipped capabilities (R-12), invented quotes, or unlabeled reconstructions.
- Don't touch anything outside `docs/ti99book/`; don't commit over sibling sessions; don't skip support-file updates.
- Don't leave any `OPUS-` tag in a file you mark `DRAFTED`.
