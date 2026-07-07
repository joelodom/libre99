# MASTER OUTLINE
## *Programming the TI-99/4A: Assembly Language and GPL, from Silicon to Software*

**Working subtitle:** A complete course in TMS9900 assembly language and Graphics Programming Language for the Texas Instruments Home Computer — its history, its hardware, and the craft of writing software the way it was (and still is) done.

**Alternate titles for consideration:**
- *The 9900 Book: Assembly and GPL Programming on the TI-99/4A*
- *Sixteen Bits, Two Hundred Fifty-Six Bytes: Programming the TI-99/4A*
- *The Orphan's Handbook: A Modern Course in TI-99/4A System Programming*

**Target length:** ~1,050 pages (estimates per chapter below)
**Format:** One Markdown file per chapter; companion code repository
**Status:** Outline v1.0 — the roadmap for all subsequent writing sessions

---

## 1. Vision and Scope

### 1.1 The promise to the reader

A reader who begins this book with ordinary modern programming skills (some Python, some C, a data structures course) and works through every chapter and lab will finish able to **design, write, debug, optimize, package, and ship software of full commercial quality for the TI-99/4A** — equal in scope and polish to anything Texas Instruments or its third parties released between 1979 and 1984: arcade cartridges, speech-enabled games, data-driven RPG engines, disk-based productivity software, and system-level extensions.

Equally important, the reader will *understand the machine* — not as a black box behind an emulator, but down to the bus cycle: why the memory map looks the way it does, why TI BASIC was slow, what a GROM actually is, how a 16-bit minicomputer CPU ended up strangled by an 8-bit bus, and why a quarter-kilobyte of fast RAM shaped an entire software culture.

### 1.2 What this book covers

- The complete **TMS9900** processor: architecture, instruction set, timing, and idiom
- The complete **console environment**: memory map, scratchpad, console ROM services, interrupts, the TMS9901, keyboard, joysticks, cassette
- The **TMS9918A VDP** in every mode, including sprites, bitmap graphics, scrolling, and timing tricks
- The **TMS9919/SN94624 sound generator** and music/SFX engine construction
- The **Speech Synthesizer**: LPC theory, the resident vocabulary, allophones, and building new speech data with modern tools
- **GROM and GPL**: the hardware, the interpreter, the full instruction set, writing and assembling GPL with modern tools, and the console operating system as a GPL program
- **Peripherals and storage**: the CRU, DSR architecture, PABs and file I/O, disk internals at the sector level, RS-232, cassette, and modern devices (TIPI, SAMS, F18A, FinalGROM 99)
- **Cartridge engineering**: headers, bank switching schemes, ROM+GROM hybrids, building images for emulators and flash carts
- **Software engineering under scarcity**: memory budgeting, optimization and cycle counting, data compression, asset pipelines, and full-scale case studies that recreate the major genres of commercial TI software
- **History and culture** throughout: the corporate story, the people, the price war, Black Friday 1983, and the community that never stopped

### 1.3 What this book deliberately excludes

- **TI BASIC and Extended BASIC programming.** They appear only where system understanding demands it: TI BASIC is examined *as an artifact* (a GPL program that interprets tokens out of VDP RAM) in Chapter 28, and the Extended BASIC `CALL LINK` interface gets a short, clearly-fenced section in Chapter 36 because so much commercial software shipped as XB/assembly hybrids. No BASIC pedagogy anywhere.
- Detailed electronics/repair content (we describe hardware to the depth a programmer needs, with pointers to hardware references).
- The Geneve 9640 and TMS9995/99000 world, except as a survey chapter (Ch. 44).

### 1.4 The audience, precisely

Computer science undergraduates (or equivalent self-taught programmers) who are fluent in at least one modern high-level language and comfortable with binary/hex, but who have **never**:
- programmed in any assembly language,
- used a machine without an operating system, memory protection, or a file system by default,
- dealt with memory-mapped I/O, interrupts on bare metal, video generated from tables in RAM, or storage measured in kilobytes.

Every chapter therefore builds the *vintage* concept **and** the bridge from the modern one ("you know heap allocation; here there is no heap unless you write one — here's what that means in practice").

### 1.5 Three reading tracks (stated in the front matter)

- **Track A — Cover to cover** (the full course, recommended).
- **Track B — Game developer fast path:** Ch. 1–13, 16–17, 19, 21–22, 35–39 (then the remaining case studies).
- **Track C — Systems archaeology path:** Ch. 1–11, 22–34, 44 (GPL, OS internals, DSRs, storage).

---

## 2. Pedagogical Framework

Every chapter follows a fixed template so that sessions produced weeks apart feel like one book:

1. **Opening vignette** (½–1 page). A human story or period scene that motivates the chapter — an engineer, an ad, a magazine letter, a design meeting, a piece of famous software.
2. **Objectives** ("After this chapter you can…" — concrete, testable).
3. **The bridge** (1–2 paragraphs mapping modern concepts to vintage ones).
4. **Body sections** with worked, runnable examples. Every non-trivial listing exists in the companion repo and assembles with the standard toolchain.
5. **Field Notes sidebars** — boxed dissections of *real vintage code or data* (console ROM excerpts described behaviorally, published listings, magazine type-ins, disassembly-informed behavior studies) so the reader learns to read period software, not just write new software.
6. **Pitfalls** — a boxed list of the classic mistakes (byte-order surprises, VDP address races, R11 clobbering…).
7. **Lab** — one substantial build per chapter producing a running artifact. Labs accumulate: many contribute modules to **`lib99`**, the reader's personal runtime library, which the case studies later consume.
8. **Exercises** — 6–12 per chapter in three difficulty tiers (✦ warm-up, ✦✦ real work, ✦✦✦ challenge/open-ended). Selected solutions in the companion repo, not the book.
9. **Further reading** — pointers into the period literature (E/A manual sections, *TI Intern*, datasheets) and modern resources.
10. **Chapter summary block** — a 6–10 line recap **written to double as context priming for future writing sessions** (see Production Plan).

**Running projects:**
- **`lib99`** — grows from Ch. 11 onward: VDP I/O, text engine, sprite engine, input, sound driver, speech, timers, PAB file layer, compression.
- **Capstones** — the five Part IX case studies are complete, shippable programs.

---

## 3. Conventions and Style Guide (binding for all sessions)

- **Hex notation:** TI convention `>` prefix (`>8300`, `>AA`). Binary written `0b...` only in bridges; period style avoided ambiguity by using hex.
- **Registers:** `R0`–`R15`; workspace pointer `WP`; program counter `PC`; status `ST`.
- **Mnemonics and directives:** UPPERCASE (`MOV`, `BLWP`, `AORG`) for period authenticity; labels uppercase ≤6 chars in "classic style" listings, longer labels permitted and noted as an extension.
- **Assembler baseline:** source is **Editor/Assembler-compatible**, assembled with **`libre99asm`** — the assembler of the libre99 project this book lives inside (GPL: **`libre99gpl`**). Divergences from E/A are always flagged `[libre99asm]`. The community's **xdt99** suite (xas99/xga99/xdm99) is the interchange toolchain: it produces the period formats (tagged object, EA5) in Ch. 6 and drives disk imagery in Part VII, and its dialect is close enough that the book's source assembles under both except where flagged.
- **Numbers:** decimal unless prefixed `>`; sizes in bytes with K = 1024.
- **Memory maps and register tables:** Markdown tables; timing diagrams and bus diagrams as labeled ASCII art (portable across renderers).
- **Code fences:** ```asm for 9900 assembly, ```gpl for GPL source (libre99gpl syntax), ```text for listings/output, ```sh/```make, ```python for tooling.
- **Hardware baseline:** "**Standard system**" = console + 32K expansion + disk (emulated or real). Chapters/labs that run on a **bare console** are badged `[console-only]`; cartridge-target material badged `[cart]`.
- **Emulator baseline:** the **libre99 project** (primary — the book is developed inside its repository; the desktop app `libre99` is the daily machine, and **BENCH99**, a scriptable monitor over the same `libre99-core` crate, is the debugging/measurement instrument). Reference shelf, all covered in Ch. 3: **Classic99** (interactive GUI debugger, licensed TI software bundle, home of Ch. 6's period E/A workflow), **js99er.net** (zero-install; speech and F18A until ours grows them), **MAME** (cycle-accuracy referee). Instructions elsewhere reference the project emulator/bench first, with a one-line shelf equivalent where one matters. Labs must not assert emulator capabilities beyond what the project ships at writing time — gaps are stated and shelved explicitly.
- **Terminology ledger:** a living `_ledger.md` in the repo records every address, symbol, and term of art the manuscript has asserted (e.g., "user ISR hook = `>83C4`") so later chapters never contradict earlier ones.
- **Tone:** rigorous but warm; humor allowed in vignettes and sidebars, never in reference tables. American English.

---

## 4. Production Plan (how we actually write this across sessions)

### 4.1 Repository layout

```
/manuscript/
  00-master-outline.md          (this file — the contract)
  00a-preface.md
  00b-how-to-use-this-book.md
  ch01-...md … ch45-...md
  apA-...md … apN-...md
  _style.md                     (distilled from §3, updated as rulings accrue)
  _ledger.md                    (terminology/address/consistency ledger)
  _summaries.md                 (all chapter summary blocks, concatenated)
/code/
  lib99/                        (the accumulating library)
  bench/                        (BENCH99 — the lab bench over libre99-core; built Ch. 3)
  ch03/ … ch43/                 (per-chapter labs; build.sh per dir, libre99asm/libre99gpl)
  tools/                        (PC-side Python asset tools written in Ch. 38)
/assets/                        (diagrams, tables source data)
```

### 4.2 File naming

`chNN-short-slug.md` (e.g., `ch07-instruction-set-i-moving-data.md`), appendices `apA-tms9900-reference.md`, etc. Two-digit chapter numbers; no renumbering after Part IX begins — insertions become `chNNb-`.

### 4.3 Session protocol (because long-running tasks are hard)

- **One chapter per session** as the default unit of work (18–28 pp ≈ 6,000–10,000 words + code). Heavy chapters (7, 9, 15–17, 19–20, 26, 37, 39, 41) may take two sessions: draft + polish.
- **Every session begins** by loading: this outline, `_style.md`, `_ledger.md`, and `_summaries.md` (not full prior chapters — the summary blocks are written to make that sufficient).
- **Every session ends** by appending the new chapter's summary block to `_summaries.md` and any new rulings to `_style.md`/`_ledger.md`.
- **Appendices A–N** are drafted in parallel with their subject chapters (App. A alongside Ch. 7–9; App. B alongside Ch. 26) and finalized in dedicated batch sessions.
- **Review passes** (separate sessions, after the draft is complete): ① technical audit against datasheets/E-A manual (cycle counts, addresses), ② pedagogy pass (do the labs actually build on each other?), ③ copyedit/consistency pass driven by `_ledger.md`, ④ cross-reference & index pass.
- **Estimated session count:** ~45 chapter sessions + ~10 heavy-chapter second passes + ~6 appendix batches + 4 review passes + front/back matter ≈ **65–70 sessions**.

### 4.4 Page budget by part (targets, not straitjackets)

| Part | Content | Chapters | Est. pages |
|---|---|---|---|
| Front matter | Preface, how-to, acknowledgments | — | 8 |
| I | The Machine and Its World | 1–3 | 60 |
| II | The TMS9900 and Assembly Fundamentals | 4–11 | 180 |
| III | The Video Display Processor | 12–18 | 154 |
| IV | Sound and Speech | 19–20 | 52 |
| V | Input, Interrupts, and Console Services | 21–24 | 82 |
| VI | GROM, GPL, and the Operating System | 25–29 | 102 |
| VII | Storage and Peripherals | 30–34 | 92 |
| VIII | Cartridge and Software Engineering | 35–38 | 86 |
| IX | Case Studies: Recreating the Classics | 39–43 | 118 |
| X | Beyond the Console | 44–45 | 24 |
| Appendices | A–N | — | 104 |
| **Total** | | **45 + 14 app.** | **≈ 1,062** |

---

# THE OUTLINE

---

## PART I — THE MACHINE AND ITS WORLD *(≈60 pp)*

### Chapter 1 — Genesis and Fall: A History of the TI-99/4A *(22 pp)*
*The full corporate and cultural story, told for readers born decades after Black Friday.*

- 1.1 Texas Instruments before home computers: transistors, calculators, *Speak & Spell*, and the TI-990 minicomputer line — the two bloodlines (CPU and speech) that meet in the 99/4
- 1.2 The 9900 gamble: putting a minicomputer CPU on one chip (1976), and why the world's first mainstream 16-bit micro didn't conquer it
- 1.3 Building the 99/4 (1979): the Lubbock consumer group, design-by-committee, the calculator-key fiasco, the FCC problem and the bundled monitor, the $1,150 sticker shock
- 1.4 The closed box: GROM as a business strategy — controlling the cartridge market, the third-party lockout, and how that decision shaped (and shrank) the software library
- 1.5 The 99/4A (June 1981): real keyboard, the 9918**A** and bitmap mode, lowercase (sort of), and a machine finally ready — at $525
- 1.6 The home computer war: Jack Tramiel, the VIC-20, Bill Cosby vs. William Shatner, rebates, and the death spiral to a $49 sixteen-bit computer
- 1.7 Black Friday: October 28, 1983 — inside TI's exit, the quarter-billion-dollar write-off, and the strangest Christmas in computer retail
- 1.8 Life after death: user groups, *99'er* / *Home Computer Magazine*, *MICROpendium*, Millers Graphics, the fairware era, Myarc and the Geneve, four decades of faires — and why this community never dissolved
- 1.9 The machine today: emulation, FPGA rebirths, new hardware, new commercial-quality homebrew — setting up the book's claim that the platform is *alive*
- **Vignettes/sidebars:** the Lubbock plant; "How do you sell a computer with Bill Cosby?"; a retailer's memo from November 1983; the first Chicago Faire
- **Lab:** none (reading chapter) — but a guided "artifact hunt": run three famous titles in an emulator and record observations we'll explain by book's end
- **Field Notes:** reading a 1983 price sheet; decoding a cartridge PCB photo

### Chapter 2 — Grand Tour: The Architecture at 10,000 Feet *(18 pp)*
*Every major component introduced once, honestly, before we zoom in for the rest of the book.*

- 2.1 The cast of chips: TMS9900 (CPU), TMS9901 (I/O + interrupts), TMS9918A (video, *with its own 16K*), TMS9919/SN94624 (sound), console ROM, console GROMs, and 256 bytes — really — of CPU RAM
- 2.2 The block diagram walk: who talks to whom, and the fateful 8-bit multiplexed data bus between the 16-bit CPU and almost everything
- 2.3 Where the memory *is*: why the computer's main RAM (16K) belongs to the video chip, and what that means for every program ever written on this machine
- 2.4 The tower of interpreters: GPL over 9900, BASIC over GPL — the architectural reason "TI BASIC is slow" and the doorway this book walks through instead
- 2.5 A modern programmer's disorientation kit: no OS, no scheduler, no memory protection, no file abstraction, no stack (!), everything memory-mapped — and why that's liberating
- 2.6 What "commercial quality" meant in 1982: a checklist we'll keep returning to (attract modes, polish, speech, packaging, manuals)
- 2.7 The road map of this book, mapped onto the block diagram
- **Sidebar:** Karl Guttag and the 9918 — designing the chip that quietly powered MSX, ColecoVision, and the arcades' cousins
- **Lab:** on BENCH99 (bare bench, ahead of the toolchain by design), *watch* the multiplexer: plant `JMP $` in scratchpad and in expansion RAM, single-step each, and read the +4-cycle funnel toll off the trace

### Chapter 3 — The Workshop: A Modern Development Environment *(20 pp)*
*Set up once, used for 42 chapters. Cross-development first; period workflow honored later (Ch. 6).*

- 3.1 The strategy: edit and assemble on a modern machine, run emulated constantly, verify on real hardware occasionally; every target, one source tree
- 3.2 The machine: the **libre99 project** as daily driver (from-scratch pure-Rust core running the real firmware; embedded cartridge/disk library; media browser, save states, CPU inspector, time control, logging; `--cartridge-file` for our builds; stated gaps: speech, arbitrary disks, GUI breakpoints) — and the reference shelf: **Classic99** (GUI debugger, heat maps, licensed bundle), **js99er.net** (instant, in-browser), **MAME** (reference accuracy)
- 3.3 The project toolchain: `libre99asm` (assembler + cartridge packager: `.ctg`/`.bin`, listings, JSON symbol maps, synthesized header, `dis`), `libre99gpl` (GPL, sleeps till Ch. 27), **BENCH99** (the scriptable lab bench over `libre99-core`); the **xdt99** suite shelved for period formats (Ch. 6) and disk tools (Part VII)
- 3.4 Editor setup: syntax highlighting for 9900 assembly; project skeleton with `build.sh` (`LIBRE99ASM`/`NAME`/`TITLE` variables)
- 3.5 Build artifacts: ship formats (`.ctg` cartridge container vs. raw padded `.bin`) + instrument files (listing, symbol map); the period package formats (tagged object, EA5, `.dsk`) previewed and deferred to Ch. 6 / Part VII
- 3.6 Real hardware on your desk in 2026: consoles, the 32K question, **FinalGROM 99 / FlashROM 99**, **TIPI**, CF/nanoPEB, F18A-modded consoles; a sane shopping list by budget
- 3.7 **First light:** a complete two-dozen-instruction program — clear the screen, print `HELLO, 1981`, set the backdrop, spin; entry-by-convention (`START`); run in the project emulator (real firmware lists it on the master menu), js99er, Classic99, and a flash cart; the synthesized `>6000` header dissected byte by byte
- 3.8 Debugger literacy, six instruments: pause/frame-advance/fast-forward; the live CPU inspector; bench break-and-trace (`u`/`s`, cycle prices); memory watch-and-poke (`r`/`m`/`pw`/`screen` — registers edited by street address); listing-vs-trace reading (+ `libre99asm dis`); `--log-level debug` narration — the skills every later lab assumes (Classic99/MAME GUIs noted as equivalents)
- **Sidebar:** what it cost and what it took to do this in 1982 (E/A cartridge, memory expansion, two disk drives, a printer — a $1,500 "IDE")
- **Lab:** full toolchain bring-up (`setup.sh`: assembler + emulator + bench from source); modify HELLO to greet you by name; ship to every target; instrument drills incl. a bench transcript
- **Pitfalls:** `;` before comments on no-operand lines `[libre99asm]`; `AORG` is absolute-mode-only; TI filename law preview; `>` vs. the shell; ROM provenance stated honestly (project embeds authentic firmware; Classic99's bundle licensed; the clean-room GROM as the ethical showpiece)

---

## PART II — THE TMS9900 AND ASSEMBLY FUNDAMENTALS *(≈180 pp)*

### Chapter 4 — The TMS9900: A Minicomputer on a Chip *(24 pp)*
- 4.1 The TI-990 inheritance: why this CPU thinks like a process-control minicomputer
- 4.2 The programmer's model: exactly three on-chip registers — **PC**, **WP**, **ST** — and the radical idea of **workspaces**: your sixteen "registers" R0–R15 live in RAM
- 4.3 Consequences of workspaces: context switches by changing one pointer; registers with addresses; the cost — every register access is a memory access
- 4.4 Memory model: 64K bytes as 32K **words**; word alignment; **big-endian** byte order; how byte instructions select the *high* byte of a register — the single biggest source of newcomer bugs, treated with care
- 4.5 The status register bit by bit: L>, A>, EQ, C, OV, OP, X, and the interrupt mask — with the comparison-semantics table (logical vs. arithmetic) every later chapter leans on
- 4.6 No hardware stack: the `R11` link convention, and a preview of building stacks by hand (Ch. 9)
- 4.7 Instruction formats and the general-address philosophy (memory-to-memory operations — no "load/store" bottleneck, at a price)
- 4.8 Timing model: clock (3 MHz), machine cycles, memory cycles, and where wait states come from (deep measurement in Ch. 5, 37)
- 4.9 The 9900 among its peers: vs. 8080, 6502, Z80, 68000 — an honest scorecard, and the tragedy of the 64K address space
- **Sidebar:** the 64-pin package and three supply voltages — why the 9900 was expensive to design in
- **Lab:** paper machine first: hand-execute a 6-instruction trace, predicting PC/WP/ST and memory; then verify every value in the debugger

### Chapter 5 — The Console Memory Map: Geography Is Destiny *(20 pp)*
- 5.1 The complete map, `>0000`–`>FFFF`, region by region: console ROM; low expansion (`>2000`); the peripheral/DSR window (`>4000`); cartridge ROM (`>6000`); the `>8000` block (scratchpad and its mirrors); memory-mapped ports — sound `>8400`, VDP `>8800/>8C00`, speech `>9000/>9400`, GROM `>9800/>9C00`; high expansion (`>A000`)
- 5.2 **The scratchpad**: 256 bytes at `>8300` — the only 16-bit RAM in the machine; first tour of its system-reserved regions (full atlas: Ch. 24 / App. C)
- 5.3 The multiplexer measured: a timing experiment proving the 8-bit-bus penalty on expansion RAM, cartridge ROM, and everything else — and the two golden territories (console ROM, scratchpad)
- 5.4 Partial address decoding and mirrors: why `>8300` ≡ `>83xx` mirrors exist, why writing "empty" space isn't always harmless
- 5.5 Where programs live: the `>2000`/`>A000` split (24K + 8K, and why it's split), cartridge space, running code *in the scratchpad*
- 5.6 Memory-mapped I/O as a worldview: "peripherals are just addresses" — bridging from modern MMIO the reader may know only abstractly
- 5.7 Planning worksheet: the memory budget template used by every project in this book
- **Sidebar:** why 256 bytes? The economics of the 1979 bill of materials
- **Lab:** write the timing rig; produce a measured table of access costs per region; race the same loop in `>8300` vs `>A000`
- **Field Notes:** how period software detected the 32K expansion

### Chapter 6 — Assembling: Source, Object, and Loaders *(24 pp)*
- 6.1 Source anatomy: labels, mnemonics, operands, comments; the 80-column heritage
- 6.2 Directives that matter: `AORG`/`RORG`/`DORG`, `BSS`/`BES`, `DATA`/`BYTE`/`TEXT`, `EQU`, `DEF`/`REF`, `EVEN`, `END` — relocatable vs. absolute thinking
- 6.3 Expressions, the location counter `$`, and classic idioms
- 6.4 Object code demystified: the **tagged object format** read line by line (a beautiful, human-readable relic); compressed object; why it matters for tooling and archaeology
- 6.5 Program images: **EA5** memory-image files, multi-file images, autostart conventions
- 6.6 The historic environment, honored properly: the **Editor/Assembler** cartridge walkthrough — editing, assembling (two passes, listing files), Option 3 vs. Option 5 loading; the **REF/DEF table** and how `DEF`'d names become callable
- 6.7 The E/A utility vocabulary (`VSBW`, `VMBW`, `VSBR`, `VMBR`, `VWTR`, `KSCAN`, `GPLLNK`, `XMLLNK`, `DSRLNK`) — introduced as *names and contracts now*, implemented ourselves later; environment-dependence warnings
- 6.8 Mini Memory: the 4K battery-backed cartridge and line-by-line assembler — the $99 on-ramp thousands actually used
- 6.9 Multi-module builds: `COPY`-include architecture (libre99asm); xas99's object-module path for the linking-loader world; listings, symbol tables, and map files as debugging instruments
- **Sidebar:** the Editor/Assembler manual itself — anatomy of the most-thumbed 400 pages in TI history (a copy lives in the project repository)
- **Lab:** one program, four ways: (a) E/A Option 3 object and (b) Option 5 image — xas99-built, loaded through Classic99's bundled E/A cartridge, honoring the period path; (c) the full 1982 ritual once, on the emulated console itself; (d) the modern cartridge via libre99asm — same source, one build script; loader-linkage directives (`DEF`/`REF`) met in their native habitat
- **Pitfalls:** forgetting `EVEN` after `TEXT`; `AORG` overlap; symbol length assumptions

### Chapter 7 — Instruction Set I: Moving Data *(26 pp)*
- 7.1 The five general addressing modes, each with its assembly syntax, encoding, cost, and canonical idiom: `Rn`, `*Rn`, `*Rn+`, `@LABEL`, `@LABEL(Rn)`
- 7.2 `MOV`/`MOVB` in depth; the big-endian byte-in-high-half rule, illustrated until it cannot be forgotten
- 7.3 Immediates and register setup: `LI`, `LWPI`, `STWP`, `STST`; `CLR`, `SETO`, `SWPB`
- 7.4 Autoincrement as the machine's inner loop: copying, filling, scanning — the memcpy/memset/strlen family written well
- 7.5 Structures and arrays without a compiler: indexed mode patterns, record layouts, tables of words vs. tables of bytes
- 7.6 First cycle-counting: reading the instruction timing table; why `MOV *R1+,*R2+` is the pump at the heart of the machine
- 7.7 Self-test scaffolding: comparing memory regions and reporting via screen color (we still have no console I/O of our own — deliberately)
- **Lab:** `memlib` for `lib99`: copy/fill/compare/scan primitives, benchmarked with the Ch. 5 rig in 16-bit vs 8-bit RAM
- **Field Notes:** a data-mover loop from a published 1983 listing, annotated

### Chapter 8 — Instruction Set II: Arithmetic, Logic, and Bits *(24 pp)*
- 8.1 Add/subtract families (`A`, `AB`, `S`, `SB`, `INC`, `INCT`, `DEC`, `DECT`, `NEG`, `ABS`) and flag effects that later branches depend on
- 8.2 Comparison done right: `C`, `CB`, `CI` and the logical-vs-arithmetic flag pairs; `COC`/`CZC` — the bit-mask comparators nobody's home CPU has today
- 8.3 Boolean surgery: `ANDI`, `ORI`, `XOR`, `INV`, and the mask writers `SOC`/`SZC` ("set ones corresponding") — flags, fields, and packed data
- 8.4 Shifts: `SLA`, `SRA`, `SRL`, `SRC`; shift-count-in-R0; multiply/divide by powers of two; the carry as a data channel
- 8.5 `MPY` and `DIV`: unsigned 16×16→32 and 32÷16; signed multiplication/division built honestly; overflow traps for the unwary
- 8.6 Multi-precision: 32-bit add/sub/compare patterns; a 32-bit accumulator module
- 8.7 Decimal output without hardware BCD: divide-by-10 chains vs. subtract-tables; hex output as a bonus
- 8.8 Pseudo-random numbers: LFSRs on the 9900; the console's own seed byte
- **Lab:** `mathlib` for `lib99`: 32-bit ops, signed MPY/DIV wrappers, `U16→decimal ASCII`, LFSR — with a test harness that proves them
- **Pitfalls:** `DIV` overflow semantics; flags `INCT` does and doesn't set

### Chapter 9 — Control Flow and Program Shape *(26 pp)*
- 9.1 Jumps: the `J**` family, ±128-word displacement, and the two-jump idiom when the target is far
- 9.2 `B` (go anywhere), `BL` (call, link in R11), `RT` — and the discipline of R11: nesting, saving, the bug every beginner writes
- 9.3 **Software stacks**: choosing a stack register, PUSH/POP macros, calling conventions we adopt for the whole book (documented, then obeyed)
- 9.4 `BLWP`/`RTWP`: the context-switch call — vectors, the R13/R14/R15 linkage, writing your own `BLWP`-style services; when workspace switching pays and when it's ceremony
- 9.5 Structured shapes in an unstructured language: if/else, while, repeat, switch — as named jump patterns; **jump tables** and `B @TABLE(Rn)` dispatch
- 9.6 The `X` instruction: execute-an-instruction-somewhere-else — legitimate uses (dispatch, patching) of a famous oddity
- 9.7 `XOP`: the supervisor-call that consoles barely use — how it works, why it's rare on the 4A, where it shines (Geneve/990 heritage)
- 9.8 Coroutines and state machines via workspace switching — the pattern behind smooth game logic
- 9.9 Error handling conventions for a machine with no exceptions
- **Lab:** `task99`: a tiny cooperative multitasker (two coroutines sharing the screen) in under 200 bytes — the chapter's ideas made undeniable
- **Field Notes:** BLWP vector tables in the console ROM, read cold

### Chapter 10 — The CRU: TI's Serial Nervous System *(14 pp)*
- 10.1 What the Communications Register Unit is: a 4K-bit address space of single-bit I/O, entirely separate from memory
- 10.2 The R12 base-address convention (and the ×2 shift that confuses everyone once)
- 10.3 Single-bit ops: `SBO`, `SBZ`, `TB` — set, clear, test
- 10.4 Multi-bit transfers: `LDCR`/`STCR`, the ≤8-bit byte-operand rule, and bit-order gotchas
- 10.5 First real device: reading console keys directly via the 9901 (a taste — full treatment Ch. 21)
- 10.6 The CRU map of a loaded system (preview of App. G): who lives at `>1100`, `>1300`, `>1E00`…
- **Lab:** a CRU explorer: interactive bit-poker with live display (and an audible clicker via the cassette relay — the machine's most satisfying output device)
- **Sidebar:** why a serial bit bus? 990 industrial-control DNA

### Chapter 11 — Craftsmanship: Style, Macros, Debugging, and `lib99` *(22 pp)*
- 11.1 A style guide for 9900 assembly (ours, stated once, used everywhere): naming, workspace discipline, comment density, file layout, register-use maps per routine
- 11.2 Assembler-assisted idiom: PUSH/POP and structured-flow patterns via `COPY`-includes and disciplined convention; the macro question examined honestly (E/A had none; xas99 and TI's Macro Assembler as the macro traditions; what macros buy, what they hide, and what our macro-free baseline demands instead)
- 11.3 Include architecture: `equates.a99` (every magic address named once), `lib99` module conventions, linking discipline
- 11.4 Debugging as a method: hypothesis → breakpoint → evidence (bench `u`/`s` transcripts as lab notebook); watchpoints on scratchpad state (bench diffing; Classic99's watchpoints and heat map on the shelf); MAME as second opinion
- 11.5 Instrumentation you leave in: assertion macros, debug-build screen flashes, a panic dump routine
- 11.6 Testing assembly in 2026: **scripted BENCH99 runs** (assemble, boot, drive, assert on RAM/VRAM) — a real CI for a 1981 target, native to the emulator-as-library; the same technique the emulator project uses to test itself
- 11.7 Reading the ancients: how to approach *TI Intern*-style ROM listings and period source without drowning
- **Lab:** formalize `lib99` (repo layout, versioning, test harness); write its first consumer: `SYSCHK`, a system-info card (CPU RAM found, VDP type, peripherals responding) we'll extend all book long
- **Sidebar:** Millers Graphics' *Explorer* — single-stepping the whole machine in 1985

---

## PART III — THE VIDEO DISPLAY PROCESSOR *(≈154 pp)*

### Chapter 12 — Inside the TMS9918A *(20 pp)*
- 12.1 The VDP as a computer of its own: 16K private VRAM, its own bus, its own clock; the CPU as a guest
- 12.2 The four ports (`>8800/>8802` read data/status, `>8C00/>8C02` write data/address) and the **address-setup protocol**: two bytes, the read/write flag, autoincrement — performed slowly, then macro-ized forever
- 12.3 Write registers R0–R7: every bit named (mode bits M1–M3, 4K/16K, blank, interrupt enable, sprite size/mag, table bases, colors)
- 12.4 The status byte: frame flag **F**, fifth-sprite flag & number, coincidence **C** — and why reading it has side effects you must design around
- 12.5 Table-driven video: the philosophy (name→pattern→color) that the next five chapters instantiate mode by mode
- 12.6 VRAM timing truths: CPU access windows, how fast you may hammer the ports (and how the 4A's own slowness mostly protects you — with the exceptions that bite)
- 12.7 Planning VRAM: the layout worksheet (a sibling of Ch. 5's) used in every project
- **Lab:** `vdplib` core for `lib99`: `VWTR`-like register write, address set, single/multi read & write (our own `VSBW/VMBW/VSBR/VMBR`), verified against the E/A-manual behavior contract
- **Field Notes:** the 9918A datasheet — learning to read a 1980 datasheet as literature

### Chapter 13 — Graphics I Mode and a Real Text Engine *(24 pp)*
- 13.1 Graphics I structure: 32×24 name table, 256 patterns, the 32-entry color table (color by *group of eight* characters — the constraint that defines TI aesthetics)
- 13.2 Character sets: borrowing the console's font from GROM vs. shipping your own; the small-caps "lowercase" story
- 13.3 Building `textlib`: cursor state, PUTC/PUTS, newline & wrap, clear, positioning — a println for a machine that never had one
- 13.4 Number & hex formatters joined to Ch. 8's math: the debugging dashboard pattern
- 13.5 Color-table strategy: palettes, inverse video, "highlight by group" tricks within the 8-character rule
- 13.6 Screen composition: borders, panels, simple windows; a status-line convention for the rest of the book
- 13.7 Name-table double buffering (two screens in VRAM, flip by register) — smooth full-screen updates before we ever touch sprites
- **Lab:** `MONITOR99`: a live memory viewer/editor (hex dump, poke, goto) built entirely on this chapter — genuinely useful for the rest of the course
- **Sidebar:** why 32 columns? TVs, overscan, and the 40-column temptation

### Chapter 14 — Text Mode and Multicolor Mode *(14 pp)*
- 14.1 Text mode: 40×24, 6-pixel cells, two colors from R7, **no sprites** — the productivity mode; setting it up, fonts that read at 6 pixels
- 14.2 A 40-column text engine variant (`textlib40`) — foundation for Ch. 42's editor
- 14.3 Multicolor mode: the 64×48 fat-pixel canvas; table layout; who ever used it (and why so few)
- 14.4 Mode switching cleanly at runtime; keeping VRAM plans compatible across modes
- **Lab:** a mode carousel demo + `PLOT64` mini-API for multicolor; port `MONITOR99` to 40 columns
- **Field Notes:** TI-Writer's screen handling, observed

### Chapter 15 — Bitmap Mode: Graphics II *(26 pp)*
- 15.1 How bitmap mode really works: the screen in thirds; name table as 768 sequential indices; 6K pattern + 6K color tables; the 8×1 color-pair rule (per-line attribute clash, mastered not mourned)
- 15.2 Register setup incl. the notorious mask bits in R3/R4 (the `>7F`/`>03` values that must be exactly right)
- 15.3 Pixel addressing math: from (x,y) to byte & bit; a fast PSET/PRESET with lookup tables
- 15.4 Lines, rectangles, fills: Bresenham with 9900-friendly fixed point; span fills that respect color cells
- 15.5 Text over bitmap: rendering fonts into the pixel canvas (variable-width teaser)
- 15.6 Full-screen images: the modern pipeline (convert PC images → 9918 constraints), dithering and palette realities; loading from disk vs. cart
- 15.7 The cost ledger: 12K of 16K gone — hybrid layouts and "half-bitmap" style tricks used by the pros
- 15.8 Performance truth-telling: what full-screen bitmap animation can and cannot do at 3 MHz (setting up Ch. 17's answers)
- **Lab:** `bmplib` + a slideshow viewer with fades; a plotting demo (starfield / function grapher)
- **Sidebar:** the 99/4 owners who couldn't run any of this — the 9918 (no A) and the upgrade culture

### Chapter 16 — Sprites *(26 pp)*
- 16.1 Sprite hardware model: 32 sprites, the Sprite Attribute List (Y-first order, the `>D0` terminator, early-clock bit), 8×8 vs 16×16, magnification
- 16.2 Sprite patterns and the descriptor table; sharing vs. splitting pattern space with characters
- 16.3 The law of four: only four sprites per scanline; the fifth-sprite status field; **flicker multiplexing** — rotating SAL priority like the professionals did
- 16.4 Movement: position update strategies; sub-pixel velocity with 8.8 fixed point; the console ISR's **automatic motion** feature (the VDP `>0780` motion table) — how it works, why serious games often bypass it, and when it's exactly right
- 16.5 Collisions: the coincidence flag's honest limits; bounding boxes; hybrid strategies (hardware hint → software confirm)
- 16.6 Sprite priorities, transparency, and layering against Graphics I/II backgrounds
- 16.7 Animation: frame tables, ping-pong cycles, state-driven animation controllers
- **Lab:** `spritelib` (spawn/kill/move/animate/multiplex/collide) + **DODGE**, a complete minigame: player vs. meteor field, score, lives, restart — our first *game*
- **Field Notes:** counting sprites in Parsec's asteroid belt frame by frame in the debugger

### Chapter 17 — Motion: Game Loops, Scrolling, and the 60 Hz Contract *(26 pp)*
- 17.1 The frame as the unit of time: the VDP interrupt / status-F heartbeat; **50,000 cycles per frame** (60 Hz) — the budget every design negotiates with
- 17.2 Two orthodox loop shapes: interrupt-driven vs. poll-synchronized; choosing per project; keeping logic deterministic
- 17.3 Update/render separation on a machine with no vsync'd GPU: batching VRAM writes into the safe window; dirty-rectangle bookkeeping
- 17.4 Coarse scrolling: shifting the name table (all four directions), wraparound worlds, column/row feeders from map data
- 17.5 **Smooth character scrolling**: the pattern-shift technique (redrawing character definitions to slide pixels) — the Parsec method, built step by step; cost analysis and how much screen can actually move
- 17.6 Vertical variants and split scrolling; parallax illusions with color and pattern cycling
- 17.7 Page flipping (Ch. 13's double buffer) under motion; tear-free updates
- 17.8 A time system for `lib99`: frame counters, timers, scheduled events; slow-motion & pause done right
- **Lab:** **TERRAIN**: a smooth-scrolling landscape flyover with parallax stars and a frame-budget HUD showing cycles spent — the chapter's claims, measured on screen
- **Pitfalls:** VDP address register races between ISR and main loop (the classic crash, finally explained)

### Chapter 18 — Advanced and Modern VDP Topics *(18 pp)*
- 18.1 Mid-frame register tricks: split screens (score panel in text mode over bitmap play field), timing splits by polling/timer; stability limits on a stock console
- 18.2 The status-read hazards catalog; interrupt-enable etiquette when the ISR isn't yours
- 18.3 4K/16K mode bit archaeology; the external-video bit; undocumented corners worth knowing exist
- 18.4 NTSC composite artifacts: the colors between the colors; PAL differences (50 Hz frame budgets — porting math)
- 18.5 Successors as *targets you may meet*: 9938/9958 (80 columns, more VRAM) and the **F18A** FPGA VDP (scanline registers, 80-col, its embedded GPU) — writing 4A-first code that degrades/enhances gracefully; full F18A programming deferred to Ch. 34/44
- 18.6 A bandwidth cookbook: measured VRAM throughput tables for every technique in Part III — the reference card game programmers actually want
- **Lab:** a split-screen scoreboard atop the Ch. 17 scroller; run identical code on the 9918A (project emulator) and an F18A config (js99er or real hardware — ours models the stock chip) and document the differences
- **Sidebar:** demo-scene proof: what "impossible on a 9918A" looked like before someone did it


---

## PART IV — SOUND AND SPEECH *(≈52 pp)*

### Chapter 19 — The Sound Generator: Music and Effects Engineering *(26 pp)*
- 19.1 The TMS9919/SN94624 (SN76489 family): three square-wave tone channels + one noise channel; the register/command byte format written to `>8400`
- 19.2 Frequency math: from the 3.58 MHz-derived clock to divider codes; building an equal-temperament note table (generated by a 20-line Python tool — our first PC-side asset script)
- 19.3 Attenuation: 2 dB steps, silence at 15; software **envelopes** (attack/decay shapes as tables)
- 19.4 Noise: periodic vs. white, the three rates + channel-3-tracked mode; drums, engines, explosions, surf
- 19.5 Effects cookbook: pitch slides, vibrato, arpeggios (the poor chip's chords), laser zaps, alarms — each as a reusable `sfxlib` patch
- 19.6 **The console sound-list format**: duration-tagged register streams; the built-in ISR auto-player (pointer & flags in scratchpad); the same lists GPL plays — one format across the whole platform, and our assembler macros for writing it musically
- 19.7 A real music driver: patterns/orders (tracker-style) vs. straight streams; priorities and channel stealing when SFX interrupt music
- 19.8 Modern pipeline: composing in a tracker → VGM capture → converting/compressing for the 9919 (the vgm-to-TI workflow); size/quality tradeoffs
- **Lab:** `sndlib` (driver + SFX kit) for `lib99`; score DODGE (Ch. 16) with title music, in-game loop, and six effects — priorities audibly correct
- **Field Notes:** dissecting TI Invaders' descending-invader loop from its register writes (debugger log → notation)
- **Sidebar:** one chip, many childhoods — the SN76489 across ColecoVision, Sega, and the BBC Micro

### Chapter 20 — The Speech Synthesizer: Making the Machine Talk *(26 pp)*
- 20.1 Why a home computer spoke in 1979: the Speak & Spell lineage, Gene Frantz & the LPC team, and speech as TI's signature move
- 20.2 Hardware & protocol: the sidecar unit, TMS5200/5220, ports `>9000` (read/status) and `>9400` (write); status bits, the 16-byte FIFO, **Speak External** streaming; timing discipline (polling without starving the frame)
- 20.3 The resident vocabulary: the speech ROM's phrase set; addressing and triggering built-in words/phrases from assembly; chaining phrases into sentences ("THAT WAS A NICE TRY" as data)
- 20.4 LPC, honestly explained for CS students: source-filter model, frames (energy, pitch, K1–K10), interpolation, why 1,200-ish bits/second of speech is possible — enough theory to *use* it fearlessly
- 20.5 Making **new** speech in 2026: the modern encoder toolchain (BlueWizard/python_wizard-class tools), recording guidelines, tuning frames by hand; building `.sph` data into your program
- 20.6 The allophone route: Terminal Emulator II's text-to-speech, phoneme stringing, and when unlimited-but-robotic beats natural-but-canned
- 20.7 Integration engineering: speech + music + gameplay concurrently — buffer feeding inside the frame budget; graceful behavior when no synthesizer is attached (detection!)
- **Lab:** `spklib`: detect, speak-resident, stream-external; give TERRAIN (Ch. 17) a talking mission-control; encode one custom phrase from your own voice and ship it
- **Sidebar:** "Aliens approaching" — why Parsec's voice sold consoles
- **Field Notes:** Alpiner's insult library, catalogued

---

## PART V — INPUT, INTERRUPTS, AND CONSOLE SERVICES *(≈82 pp)*

### Chapter 21 — Keyboard, Joysticks, and the TMS9901 *(20 pp)*
- 21.1 The 9901 Programmable Systems Interface: one chip, three jobs (interrupt controller, I/O ports, timer) — pinout-level map of what the console wires where
- 21.2 The keyboard matrix: column select via CRU bits, row reads; scanning it *yourself*, debouncing, ghosting/masking realities of the 4A matrix
- 21.3 The console's `KSCAN` routine: modes (`>8374`/`>8375` interface), key codes, what it costs, and what it gives (repeat, shift states, the GPL status byte)
- 21.4 Joysticks: reading both sticks through the matrix; fire buttons; diagonals; **the Alpha Lock trap** (why UP dies with the lock down — the hardware story and every known workaround)
- 21.5 Rolling your own input layer: per-frame snapshot, edge detection (pressed/held/released), redefinable keys — `inplib` design
- 21.6 Conventions players expected: FCTN-based QUIT etiquette (and disabling it deliberately), pause keys, two-player patterns
- **Lab:** `inplib` for `lib99` + an input visualizer; retrofit DODGE with keyboard *and* joystick + redefinable controls
- **Field Notes:** how Munch Man reads the sticks (and why it feels tight)
- **Sidebar:** the membrane-vs-real-keys war of 1979–81

### Chapter 22 — Interrupts and Time *(24 pp)*
- 22.1 The 9900's 16-level vectored interrupt model — and the console's decision to wire effectively **one** level (plus RESET and LOAD); `LIMI` as the on/off switch of daily practice
- 22.2 Sources into level 1 via the 9901: VDP vertical interrupt (the 60/50 Hz metronome), 9901 timer, external — identification and dispatch
- 22.3 **The console ISR, walked line by line** (behaviorally): what fires every frame — sprite auto-motion, sound-list service, QUIT scan, screen-blank timeout — its scratchpad footprint, and its cost in your budget
- 22.4 Taming it: the control flags (all-off vs. feature masks), and the **user ISR hook at `>83C4`** — contract, register/GPL-state preservation rules, safe patterns, chaining
- 22.5 Going bare: pure-poll architectures with interrupts masked — when the ISR is worth firing at all (many cartridges said no)
- 22.6 The 9901 timer: programming it, reading it, its resolution and quirks; uses — music tempo independent of frame rate, mid-frame splits, input sampling
- 22.7 **Profiling**: a timer-based measurement harness; measuring routines in cycles; Classic99 cross-checks — the tool Ch. 37 will lean on
- 22.8 Critical sections: the VDP-address race and friends, formally stated; masking discipline; a checklist for ISR-safe libraries (applied back onto `vdplib`/`sndlib`)
- **Lab:** install a user ISR that runs the Ch. 19 music driver at rock-solid tempo while main code sleeps; build `PROFILE99` and publish measured costs of ten `lib99` calls
- **Sidebar:** LOAD, the nonmaskable back door — single-step hardware and the Explorer trick

### Chapter 23 — Console ROM Services: GPLLNK, XMLLNK, and Floating Point *(22 pp)*
- 23.1 What the console ROM/GROM already does for you (and what it costs to ask): the service landscape and the two doorways from assembly
- 23.2 `GPLLNK`: borrowing GPL-side routines — loading the standard character sets, cassette DSR entry, bit-reversal and friends; the scratchpad/GPL-state contract behind every call
- 23.3 `XMLLNK`: the ROM routine table; calling conventions; direct-address XML as an escape hatch
- 23.4 The **floating-point package**: radix-100 reals, the 8-byte format dissected; FAC/ARG scratchpad areas; add/sub/mul/div, compares, conversions (CFI/CIF), and number↔string via the value/display routines
- 23.5 Judgment: when floating point earns its cycles (rarely, in games) vs. Ch. 8/36 fixed point; a conversion cost table
- 23.6 Environment honesty: these E/A-utility doorways assume the E/A memory image — what breaks in a bare cartridge, and **re-implementing minimal GPLLNK/XMLLNK shims** for cart-only programs (built here, reused in Ch. 35)
- **Lab:** a scientific-notation calculator over `textlib` using the ROM FP package; then the same math in 8.8 fixed point, benchmarked side by side
- **Field Notes:** radix-100 spotted in the wild — reading a BASIC program's numbers out of VDP RAM

### Chapter 24 — The Scratchpad Atlas *(16 pp)*
- 24.1 Why 256 bytes deserve a chapter: the fastest — and most political — memory in the machine
- 24.2 Guided tour `>8300`→`>83FF` in narrative form: the conventionally free zones; GPL's variables; KSCAN's state; cassette/DSR usage; the ISR's territory; the system workspaces (`>83C0` ISR / `>83E0` GPL) register by register (what R13/R14/R15 of each *mean* to the OS)
- 24.3 Occupancy by environment: bare console vs. E/A-loaded vs. cartridge-with-GPL-alive — three annotated maps; what you may steal in each world and what you must preserve
- 24.4 Standard layouts for this book's projects: where `lib99` puts its workspace(s), stack, and hot variables — the template all case studies inherit
- 24.5 Running *code* in the pad: carving 40–80 bytes for an inner loop; loaders that copy hot code in (technique perfected in Ch. 37)
- **Lab:** `PADWATCH`: a live scratchpad visualizer (before/after diffing around any routine) — instantly reveals what any console service really touches
- **Sidebar:** archaeology of `>83C0` as the random-number seed — one byte, many stories

---

## PART VI — GROM, GPL, AND THE OPERATING SYSTEM *(≈102 pp)*

### Chapter 25 — GROM: The Strangest Memory You'll Ever Address *(16 pp)*
- 25.1 What a GROM physically is: a ROM with its own internal address counter, read serially through two memory-mapped ports — and why TI built such a thing (density, cost, and *control*)
- 25.2 The port protocol at `>9800`/`>9802` (read data / read address) and `>9C02` (write address): setting the 16-bit address high-byte-first, auto-increment on every read, the read-address off-by-one to know cold
- 25.3 The 64K GROM space: 6K devices in 8K slots; console GROMs 0–2 (monitor, TI BASIC); cartridge GROMs 3–7 at `>6000`+
- 25.4 **GROM bases**: the sixteen parallel address spaces (`>9800`, `>9804`, …) — why they exist, who used them (p-code card!), and library-style GROM selection
- 25.5 GRAM: writable GROM-space devices from the GRAM Kracker to the FinalGROM 99 — the door through which *we* will run our own GPL
- 25.6 Reading GROM from assembly: a clean `gromlib` (save/restore address discipline so the OS never notices you were there)
- **Lab:** `GROMDUMP`: dump and checksum the console GROMs to disk; hex-browse TI BASIC's own source of being with MONITOR99
- **Sidebar:** the lockout that backfired — GROM licensing, Atarisoft's ROM-only end-run, and the 1983 cartridge politics

### Chapter 26 — The GPL Language *(26 pp)*
- 26.1 What GPL is: a bytecode language + interpreter (in console ROM) whose "machine" is the whole console — GROM for code, VDP for screen-and-data, scratchpad for variables; why TI wrote its OS and flagship software in it (density, portability across a planned family, and GROM economics)
- 26.2 Execution model: the interpreter fetch loop; the GPL workspace (`>83E0`) as the VM's registers; the GPL status byte (`>837C`); cost per opcode — an honest overhead measurement up front
- 26.3 Addressing in GPL: immediate, GROM, **CPU-RAM offsets (the `>8300`-based pad variables)**, VDP RAM, indexed & indirect flavors, VDP-register access — the operand-byte encodings decoded once, referenced forever (full tables App. B)
- 26.4 Instruction tour I — data & arithmetic: `MOVE` (the block-mover between *any* two spaces — GPL's superpower), loads/stores, ADD/SUB/MUL/DIV/INC/DEC, logic ops, compares
- 26.5 Instruction tour II — flow: `BS`/`BR` (branch on status set/reset), `B`, `CASE`/`DCASE` jump tables, `CALL`/`RTN`(`FETCH` for parameters), `EXIT`
- 26.6 Instruction tour III — the console-flavored ops: `FMT` (the screen-layout **sub-language**: rows, columns, repeated characters, text runs — a DSL inside the bytecode, given full treatment), `ALL` (fill screen), `BACK` (border color), `SCAN` (keyboard), `RAND`, sound-list dispatch, I/O hooks
- 26.7 Reading real GPL: conventions of TI's own code; a Rosetta listing (same routine in 9900 asm, GPL, and pseudo-C)
- 26.8 GPL's costs and sweet spots, measured: menus/UI/text — superb; inner game loops — never; the numbers behind the folklore
- **Lab:** hand-assemble a dozen GPL instructions into GRAM with MONITOR99 and single-step the *interpreter* interpreting them (BENCH99 traces both sides of the act: the 9900 instruction stream and the core's GROM-fetch log) — the moment GPL stops being magic
- **Sidebar:** the leaked GPL manuals and the fan reconstruction of a language TI never sold

### Chapter 27 — Writing GPL Today: libre99gpl and Building GROM Cartridges *(22 pp)*
- 27.1 The project's `libre99gpl` assembler (`asm`/`dis`; the tool that built the clean-room console GROM): source format, directives, symbol handling; project scaffolding alongside libre99asm; the xga99 dialect noted for interchange
- 27.2 **The GPL cartridge header** at `G>6000` byte by byte: `>AA`, version, program list / power-up list / DSR-and-subprogram list pointers; how the master menu discovers you; multiple entries from one cartridge
- 27.3 Program structure in practice: screens with `FMT`, pad-variable planning, subroutine style, keyboard loops with `SCAN`
- 27.4 Multi-GROM programs: crossing the 6K boundaries; data GROMs; base-address hygiene
- 27.5 Shipping formats: `.ctg` with GROM regions for the project emulator, images for the **FinalGROM 99**, Classic99 configs, MAME `.rpk` — one build, several targets (tool growth expected here; the book drives the project's roadmap)
- 27.6 Debugging GPL: BENCH99 with the core's GROM-fetch log and `libre99gpl dis`; Classic99's GPL-aware single-stepper and GROM viewer on the shelf; tracing FMT; classic GPL bugs (status-byte confusion, MOVE direction slips)
- **Lab:** **QUIZMASTER**, a complete GPL cartridge: title screen, menu entry, FMT-drawn UI, question data in a second GROM, scores in pad — runs from the real console menu
- **Field Notes:** the header of a genuine 1981 TI cartridge, annotated byte-by-byte

### Chapter 28 — The Operating System in GROM: Boot, Menu, and TI BASIC as Artifact *(22 pp)*
- 28.1 Power-on, precisely: RESET vector → console ROM init → the leap into GPL at the title screen; what's initialized where (a scratchpad birth certificate)
- 28.2 The master menu algorithm: scanning GROM bases and headers, building the selection list, dispatching — why "PRESS 2 FOR TI BASIC" is just a list entry, and how *your* cartridge joined it in Ch. 27
- 28.3 Power-up hooks: code that runs before the menu (peripheral GPL and cartridge power-up lists) — legitimate uses and famous abuses
- 28.4 System services living GPL-side: cassette dialogs, the character sets, pieces of KSCAN's personality — the console as a GPL application suite
- 28.5 **TI BASIC dissected as a GPL program** (not taught as a language): tokens crunched into VDP RAM, the CRUNCH/EXECUTE loop, why variables live behind the video chip, the two-interpreter tax quantified — the machine's slowest feature explained by everything the reader now knows
- 28.6 Field guide to console versions: 99/4 vs 4A vs v2.2 GROM differences that occasionally bite software
- 28.7 Proof of understanding: the project's **clean-room console GROM** — an original title screen, menu, and REPL written with Ch. 27's tools, booting on the same emulator via `--system-grom` — toured as a living demonstration that the boot contract of §28.1–28.3 is fully specified (and as this book's answer to the preservation question)
- **Lab:** trace a full boot on the bench and annotate the first 500 GPL instructions; then boot with your Ch. 27 cart and watch yourself get enumerated — under the authentic GROM *and* the clean-room one
- **Sidebar:** "READY" — what the beep meant to a generation

### Chapter 29 — Hybrid Architecture: GPL and Assembly Together *(16 pp)*
- 29.1 The TI house pattern: GPL for shell/menus/text/data, 9900 for kernels — observed across the first-party library; economics of GROM bytes vs. ROM bytes
- 29.2 Calling assembly from GPL: `XML` — the ROM tables, **user XML vector tables in scratchpad**, parameter passing conventions across the boundary
- 29.3 Calling GPL from assembly re-examined (Ch. 23's GPLLNK, now from the GPL side); state contracts in both directions, stated once as law
- 29.4 Packaging hybrids: ROM+GROM cartridges — build system, header choreography, who initializes whom
- 29.5 Case dissection: how a first-party game divides labor (menu/UI/attract in GPL; motion/collision/sound service in 9900) — reconstructed behaviorally in the debugger
- 29.6 A reusable **hybrid skeleton**: this book's template cart (GPL shell + asm core) — the chassis for Part IX
- **Lab:** rebuild QUIZMASTER's timing-critical round as a 9900 core invoked by `XML`, measure the difference, keep the GPL shell byte-identical
- **Sidebar:** why Extended BASIC is *itself* a hybrid — one sentence of respect, and we move on

---

## PART VII — STORAGE AND PERIPHERALS *(≈92 pp)*

### Chapter 30 — DSRs: How Peripherals Introduce Themselves *(18 pp)*
- 30.1 The Device Service Routine idea: every peripheral brings its own driver in ROM, mapped — one card at a time — into `>4000`, gated by its CRU bit
- 30.2 The DSR header: `>AA` and the linked lists (device names, subprograms, power-up, card ISRs); name matching ("DSK1.", "RS232/1", "PIO")
- 30.3 CRU geography of a loaded PEB: the classic base map (`>1100` disk, `>1300`/`>1500` RS232, …) and polite multi-card citizenship
- 30.4 **`DSRLNK` from the inside**: we *write our own* robust DSRLNK (search, page-in, call, page-out, error return) rather than treat the E/A one as a spell — cart-safe by construction
- 30.5 Card-side interrupts: how a DSR hooks the ISR chain (the RS232 pattern)
- 30.6 Writing a DSR of your own: a virtual card in the emulator (name, one subprogram, power-up message) — the rite of passage
- **Lab:** `dsrlib` (our DSRLNK) + **NULCARD**, a build-it-yourself DSR that answers to "NUL." and logs calls — then call it from a test program like any real device
- **Sidebar:** the Peripheral Expansion Box — eight pounds of steel and the "fire hose" cable

### Chapter 31 — File I/O: PABs and the Filesystem Contract *(22 pp)*
- 31.1 The Peripheral Access Block: every field (opcode, flags/type, VDP buffer pointer, record length, count, status, name) laid out and macro-ized
- 31.2 The opcode set in practice: OPEN/CLOSE/READ/WRITE/UPDATE, RESTORE, LOAD/SAVE, DELETE, STATUS — call sequences and error codes (the full matrix → App. H)
- 31.3 File types that matter: DIS/VAR 80 (the text lingua franca), DIS/FIX 80 (object code), INT/VAR & INT/FIX records, PROGRAM images — who reads what, interchange with modern machines via xdm99/TIPI
- 31.4 VRAM etiquette for I/O: buffers, the `>8356` name-pointer dance, coexisting with your own screen data
- 31.5 `filelib`: open/read/write/close wrappers with error surfaces a game can actually use; text-file reader (config/level loader) and writer (high-score persistence)
- 31.6 Catalog reading: enumerating a disk from assembly (the "DSK1." trick) for file-picker UIs
- 31.7 LOAD/SAVE of PROGRAM images from your own code — writing a loader like E/A Option 5's
- **Lab:** **FILER99**: a two-pane file manager (catalog, view DV80, copy, delete) over `textlib40` — the course's most practically reusable artifact yet
- **Field Notes:** reading a real TI-Writer document file byte-by-byte

### Chapter 32 — Disk Internals: Sectors, Structures, and Controllers *(20 pp)*
- 32.1 The single-density world: 90K per side — 40 tracks × 9 sectors × 256 bytes; how the TI controller (FD1771) framed a floppy
- 32.2 On-disk anatomy: **VIB** (volume info + allocation bitmap), the file-descriptor index, **FDRs** (metadata + cluster chains) — the whole filesystem on one diagram, then verified live
- 32.3 Sector-level access: the disk DSR's subprograms (direct read/write and friends) — `seclib` wrappers
- 32.4 Tools you can now write: disk mapper, un-deleter, catalog repairer, sector editor — each sketched, one built
- 32.5 The controller zoo and its dialects: TI vs. CorComp vs. Myarc (densities, sector counts), and the modern replacements (CF7/nanoPEB volumes, TIPI-mapped drives) — capability detection instead of assumptions
- 32.6 Image formats for interchange: `.dsk` sector dumps and their variants; xdm99 and TIImageTool workflows; getting bits between 1982 media and 2026 laptops
- 32.7 A note on copy protection (there was barely any) and on preservation ethics done right
- **Lab:** **DISKDOC**: sector editor with VIB/FDR decoding overlays — then deliberately corrupt a scratch disk and repair it by hand
- **Sidebar:** why TI shipped single-density in a double-density world

### Chapter 33 — Wires Out: RS-232, Parallel, and Cassette *(14 pp)*
- 33.1 The RS232 card: two serial ports + PIO; the TMS9902 in one page; DSR-level use ("RS232.BA=9600.DA=8") vs. register-level control for interrupt-driven buffers
- 33.2 A tiny terminal: TTY in, TTY out — talking to a modern USB-serial adapter; file transfer to a PC without any disk at all
- 33.3 PIO printing: line-printer conventions; formatting output for paper like it's 1983 (because sometimes it is)
- 33.4 Cassette, the people's storage: 9901 lines (motor relays, mag-tape in/out), the encoding scheme, and the console's cassette DSR reached via GPLLNK — reading/writing tape blocks from assembly
- 33.5 Modern tape tricks: mastering program audio as WAV/FLAC; loading a real console from a phone's headphone jack (yes, we do this)
- **Lab:** **TERM99** (mini terminal with XON/XOFF) + cassette save/load of a data block, round-tripped through an audio file
- **Sidebar:** 1370 baud and patience — the sound of loading, remembered

### Chapter 34 — Modern Peripherals: TIPI, SAMS, and the F18A *(18 pp)*
- 34.1 Why this chapter exists: the platform's *current* hardware ecosystem is part of "recreating commercial software" — today's releases target it
- 34.2 **TIPI** (Raspberry-Pi bridge): the message protocol at the register level; the mapped filesystem (DSK-style paths to a Pi); the extension calls — files, and onward to TCP and web APIs *from a 1981 console*; coding defensively when TIPI is absent
- 34.3 **SAMS** memory (1M+ paged RAM): the mapper model (4K pages into the `>2000`/`>3000`/`>A000`… windows), CRU enable, register programming; far-data patterns, bank-aware code rules, size detection; what SAMS makes possible (streaming worlds, giant assets)
- 34.4 **F18A** as a programmable target: unlock sequence, the extended register world, scanline interrupt-style tricks, 80-column text — and a first bow to its onboard **GPU** (a 9900-class core inside the VDP; programming it in earnest → Ch. 44)
- 34.5 FinalGROM 99 in developer mode: iterating cartridge builds fast on real hardware
- 34.6 Compatibility doctrine for this book: stock-first, enhance-if-present — detection recipes consolidated
- **Lab:** SAMS-backed level streamer bolted onto TERRAIN (a world far bigger than 32K); stretch goal: TIPI-powered online high-score table for DODGE
- **Sidebar:** the community as hardware company — how these boards get designed, built, and supported

---

## PART VIII — CARTRIDGE AND SOFTWARE ENGINEERING *(≈86 pp)*

### Chapter 35 — Cartridge Engineering *(20 pp)*
- 35.1 What a cartridge is electrically (a programmer's-eye schematic) and in the memory map: ROM at `>6000`–`>7FFF`, the standard **ROM header** (`>AA`, program list, power-up) mirrored from Ch. 27's GPL version
- 35.2 The 8K problem and its solutions — a complete catalog of **bank-switching schemes**: the write-to-`>60xx` 2-bank style; the modern 378/379 "inverted" multi-bank standard (32K → 512K → megabytes); ROM+GROM hybrids; RAM-bearing carts (Mini Memory's lineage)
- 35.3 Bank-safe program design: the resident common segment, trampolines, per-bank vector tables, data banking, bank-aware build maps (libre99asm's multi-bank support per its spec; xas99 equivalents noted) — the discipline, with build templates
- 35.4 Startup without the E/A comfort blanket: cart-side init (workspace, scratchpad claims, VDP from cold), the Ch. 23 shims in their natural habitat
- 35.5 Build & ship: one build → `.ctg` for the project emulator, padded `.bin`s for FinalGROM, `.rpk` for MAME, Classic99 configs; version/reset etiquette; testing across console revisions
- 35.6 Multi-program carts and menus of your own
- **Lab:** rebuild DODGE as a 32K four-bank cartridge — music data banked, common kernel resident — byte-identical behavior on emulator and flash cart
- **Field Notes:** tour of a modern homebrew megacart's bank map (with the author's permission)

### Chapter 36 — Program Architecture in 16–48K *(22 pp)*
- 36.1 Memory budgeting as design: the worksheet (CPU RAM / VRAM / GROM / cart banks / disk) filled out for three archetypes — cart action game, disk RPG, productivity tool
- 36.2 VRAM as data warehouse: parking tables, maps, even rarely-run code behind the VDP; costs and access patterns (when it's brilliant, when it's a trap)
- 36.3 Overlays from disk: loader design, phase-based programs (title → game → editor), keeping `lib99` resident
- 36.4 Entity/state architecture in assembly: fixed-slot object tables, component-lite layouts, iteration patterns that respect the cache-free world; the game-state machine formalized
- 36.5 Fixed-point doctrine: 8.8 as the house standard, when 12.4/4.12 win, angle systems (256-degree circles), sine tables and their generators
- 36.6 Data-driven everything: level formats, tuning tables, text banks — designed for the Ch. 38 toolchain to fill
- 36.7 Saving state: high scores/config to disk and cassette; the no-storage fallback (codes/passwords, computed & verified)
- 36.8 *Fenced sidebar (scope-honoring):* the Extended BASIC hybrid pattern — `CALL LINK`'s ABI in one page, because half the commercial catalog shipped that way; no XB taught
- **Lab:** **SKELETON99**: the official project template (phases, budgets, entity table, fixed-point, data hooks) that all Part IX studies instantiate
- **Sidebar:** reading a 1983 design doc — how pros planned before typing

### Chapter 37 — Optimization: Making 3 MHz Feel Fast *(26 pp)*
- 37.1 Doctrine: measure (Ch. 22's profiler), then move memory, then choose instructions — in that order; Amdahl in 256 bytes
- 37.2 **Placement is king**: the wait-state ledger revisited; workspaces *always* in scratchpad; hot loops copied into pad RAM (the loader pattern, perfected); the surprise that cartridge ROM is slow too — and what's actually fast
- 37.3 Instruction-level play: addressing-mode costs table; strength reduction; `MPY`/`DIV` avoidance kit (shifts, tables, reciprocal tricks); flag-riding to skip compares; fallthrough-friendly loop shapes
- 37.4 Unrolling and specializing: when 4× unroll pays on this bus; compile-time specialization via macros; jump-table dispatch vs. chained compares (measured crossover)
- 37.5 Self-modifying code, respectfully: patching immediates in RAM-resident loops, `X`-based dispatch — with bright warnings for ROM targets
- 37.6 VDP throughput engineering: batched writes, address-set amortization, the interleave patterns behind every smooth TI game (Part III's cookbook, now weaponized)
- 37.7 Table-ism as philosophy: precompute everything (the Python side generates; the 9900 side indexes)
- 37.8 Case study in tuning: the Ch. 17 scroller taken from 65% to 31% of frame budget, decision by decision, with the profiler as referee
- 37.9 When to stop: correctness, maintainability, and the retro-dev virtue of *shipping*
- **Lab:** optimization dojo — five given routines, target cycle counts to beat, scoreboard checked by the test harness
- **Field Notes:** a legendary inner loop from a shipped 1982 title, reconstructed and explained cycle-by-cycle

### Chapter 38 — Data, Compression, and the Asset Pipeline *(18 pp)*
- 38.1 The two-machine workflow matured: Python tools in `/code/tools` as first-class citizens; `make` as the conveyor belt from PNG/tracker/text to `DATA` statements and binary blobs
- 38.2 Graphics conversion: PC image → 9918 constraints (Graphics I tiles, bitmap thirds, sprite sheets); map editors (Magellan-class) and their export formats into SKELETON99's loaders
- 38.3 Compression that earns its keep at 3 MHz: RLE (and where it's enough), an LZ77-family decompressor for the 9900 (size/speed measured), tiny-alphabet text packing (5-bit + digraphs) for RPG-scale prose
- 38.4 Music & speech assets: the Ch. 19 VGM pipeline and Ch. 20 LPC builds integrated into the same Makefile
- 38.5 Level/data schemas: binary layouts with versioning; generators that emit both the binary *and* the matching `EQU` header (single source of truth)
- 38.6 Budget accounting automated: the build prints a memory report against Ch. 36's worksheet — red ink before run-time surprises
- **Lab:** crunch a 24K asset pack (tiles, two maps, three songs, 4K of text) into a 12K bank + resident 600-byte decompressor; the report proves it
- **Sidebar:** how 1982 pros did this with graph paper, and what we owe them

---

## PART IX — CASE STUDIES: RECREATING THE CLASSICS *(≈118 pp)*

*Each case study follows the same arc: **(1) Archaeology** — play and instrument the original genre-definer in the debugger, recover its architecture behaviorally (we reimplement mechanics and techniques, never copy code or assets); **(2) Specification** — write the design doc a 1982 team would have; **(3) Construction** — build our own complete title on SKELETON99 + `lib99`, chapter code = full shippable source; **(4) Postmortem** — budgets hit, tricks used, what the originals knew that we had to rediscover.*

### Chapter 39 — Capstone I: The Scrolling Shooter *(34 pp)* `[cart]`
*Genre defined by Parsec: smooth scroll, waves, speech, heat, refueling.*
- 39.1 Archaeology: instrumenting the genre-definer — scroll method confirmed, sprite multiplexing observed, speech cue timing logged
- 39.2 Spec: **METEOR BELT** — feature list, difficulty curve tables, memory budget (32K bank map), 60 Hz budget allocation
- 39.3 Terrain & scroll engine (Ch. 17 tech productionized); collision-with-terrain
- 39.4 Wave director: data-driven enemy scripts; movement pattern library; fairness rules
- 39.5 Player systems: laser heat, lives, fuel tunnel mechanic reimagined; tuning tables exposed
- 39.6 Presentation: attract mode, high-score table (disk + TIPI online variant), speech callouts, full soundtrack integration
- 39.7 Ship it: four-bank cart build, manual (yes, we write the 1982-style manual), release checklist
- **Deliverable:** a complete, polished cartridge game of first-party scope

### Chapter 40 — Capstone II: The Fixed-Screen Arcade *(20 pp)* `[console-only + cart]`
*The Munch Man / TI Invaders school — and a constraint study: bare console, no 32K.*
- 40.1 Archaeology: grid logic, chase behaviors, and the tricks of running entirely from cartridge with 256 bytes of CPU RAM
- 40.2 Spec: **GRIDRUNNER 99** — maze chase with per-enemy personalities (the four-behavior pattern), speed tiers, bonus items
- 40.3 Engine under famine: state in scratchpad + VRAM only; character-based rendering with sprite garnish; the console-only toolkit consolidated
- 40.4 Enemy AI as tables: target-tile personalities, mode timers, difficulty ramp
- 40.5 Sound identity on three channels; attract/demo playback via recorded inputs
- 40.6 Postmortem: what 16K-of-nothing forces you to invent
- **Deliverable:** an 8K/16K cart that runs on any unexpanded 4A ever made

### Chapter 41 — Capstone III: The Data-Driven RPG Engine *(30 pp)* `[disk, 32K]`
*The Tunnels of Doom inheritance: an engine that plays databases.*
- 41.1 Archaeology: how ToD separated engine from quest file — the platform's most modern architecture, mapped
- 41.2 Spec: **DUNGEONS OF FATE** — engine + quest-file format (monsters, items, maps, text, tuning all as data), so readers can ship *new games* without reassembly
- 41.3 Dungeon generation & map rendering (Graphics I tile world; the 3D-corridor view as an optional stretch build)
- 41.4 Party & combat resolvers: stats, turn engine, ranged/melee, morale — all table-driven; balancing workflow
- 41.5 Text subsystem: Ch. 38's packed prose + windowed dialogue UI; item/monster catalogs from disk
- 41.6 Persistence: save games (multiple slots, versioned), quest loading via `filelib`; memory choreography between disk phases
- 41.7 The quest-builder PC tool (Python): author a new adventure and run it, no assembler required
- **Deliverable:** engine + two playable quests + authoring tool — the book's largest single artifact

### Chapter 42 — Capstone IV: The Productivity Program *(18 pp)* `[disk, 32K]`
*The TI-Writer tradition: honest tools, 40 columns, real files.*
- 42.1 Archaeology: editor feel at 3 MHz — what made TI-Writer usable; buffer and file behaviors observed
- 42.2 Spec: **AUTHOR99** — a 40-column text editor: insert/overwrite, block ops, search, DV80 files, printer output
- 42.3 The text buffer: gap buffer on the 9900 (why it wins here); VRAM as swap for large documents
- 42.4 Screen engine: `textlib40` under continuous editing — dirty-line rendering, cursor discipline, status line
- 42.5 Files & printing: DV80 round-trips that TI-Writer itself accepts; PIO/RS232 output with margins and page breaks (a nod to the formatter tradition)
- 42.6 Responsiveness engineering: keystroke-to-glass latency measured and tuned — productivity's version of the frame budget
- **Deliverable:** an editor you can actually keep notes in (this book's own errata were drafted in it — we make that true)

### Chapter 43 — Capstone V: The Port — Bringing a Modern Game Back in Time *(16 pp)*
- 43.1 Choosing a portable modern indie design (simple rules, deep play) and mapping it to 9918A/9919 reality
- 43.2 The constraint translation table: resolution, palette, channels, RAM, input — decision by decision
- 43.3 Build fast on SKELETON99: what four parts of accumulated `lib99` buy us (a complete game in one chapter)
- 43.4 Postmortem & ledger: cycles/bytes spent vs. Part IX siblings; what porting teaches that green-field doesn't
- 43.5 The graduation exercise: reader's choice port, with a published rubric — the book's final exam
- **Deliverable:** the port, plus the reader's launch checklist for their *own* first release

---

## PART X — BEYOND THE CONSOLE *(≈24 pp)*

### Chapter 44 — The Extended Family *(14 pp)*
- 44.1 The 99/8 and the Hexbus world: the machine TI cancelled on the loading dock — architecture notes and what its GPL heritage tells us
- 44.2 The **Geneve 9640**: TMS9995, MDOS, 9938 video — porting 4A knowledge upward; what changes (real RAM! faster CPU! new timing bugs!)
- 44.3 The 99000 family and the road not taken
- 44.4 Other languages on the iron, surveyed with samples: UCSD Pascal (the p-code card's parallel universe), TI Forth → fbForth/TurboForth (why Forth loved this machine), c99, and modern **GCC for the TMS9900** — mixing C with `lib99`
- 44.5 The F18A GPU in earnest: programming the VDP's inner 9900-class core; a scanline effect impossible on stock hardware
- **Lab:** compile a `lib99`-calling C module with GCC-9900; run one Part IX game on a Geneve/emulated 9938 and document the port sheet

### Chapter 45 — The Living Platform *(10 pp)*
- 45.1 The community atlas: forums, the annual faires, developer contests, where finished software gets released and celebrated
- 45.2 Publishing your work in 2026: cart runs, disk/digital releases, manuals and box art, licensing your own code honorably
- 45.3 Preservation and stewardship: dumping, documenting, contributing to emulators and the shared record; the ethics we practiced all book
- 45.4 Contributing back: tool patches, hardware docs, mentoring the next reader
- 45.5 Closing essay — *Why program a dead machine?* On constraint, comprehension, and the pleasure of holding an entire computer in your head
- **Final artifact:** the reader's portfolio: five capstones, a library, a toolchain — and somewhere, a 45-year-old machine saying their words out loud

---

## APPENDICES *(≈104 pp — drafted alongside their subject chapters, finalized in batch sessions)*

- **A. TMS9900 Instruction Reference** *(24 pp)* — every instruction: syntax, operation, encoding, status effects, cycle formulas (with wait-state math), examples; opcode matrix; addressing-mode cost tables. *(Companion to Ch. 7–9; the book's most-thumbed pages.)*
- **B. GPL Instruction Reference** *(14 pp)* — full opcode set with operand encodings, status behavior, FMT sub-language grammar, libre99gpl syntax mapping (xga99 deltas noted). *(Ch. 26–27.)*
- **C. Memory Maps & the Scratchpad Atlas** *(10 pp)* — console map poster-page; `>8300`–`>83FF` byte-by-byte table with per-environment ownership; standard `lib99` layouts. *(Ch. 5, 24.)*
- **D. TMS9918A Reference** *(8 pp)* — registers, status, mode table layouts, VRAM timing cookbook, color palette (with RGB approximations). *(Part III.)*
- **E. Sound Generator Reference** *(4 pp)* — command bytes, frequency/attenuation tables, note table, sound-list format grammar. *(Ch. 19.)*
- **F. Speech Reference** *(8 pp)* — 5220 command/status detail, resident-vocabulary catalog with addresses, LPC frame format, allophone set. *(Ch. 20.)*
- **G. CRU Map** *(4 pp)* — system allocation `>0000`–`>1FFE`, 9901 bit map, classic card bases. *(Ch. 10, 21, 30.)*
- **H. DSR & PAB Reference** *(6 pp)* — PAB layout card, opcode/error matrices, DSR header structures, standard device names & options. *(Ch. 30–31.)*
- **I. Media & File Formats** *(10 pp)* — disk VIB/FDR structures, file-type matrix, tagged object format, EA5 image format, cassette block format, cartridge image conventions (.bin/.rpk). *(Ch. 6, 32, 35.)*
- **J. Character Sets & Key Codes** *(4 pp)* — standard patterns, KSCAN code tables per mode, keyboard matrix diagram. *(Ch. 13, 21.)*
- **K. Console Entry Points & Service Codes** *(8 pp)* — documented ROM vectors, GPLLNK/XMLLNK routine catalogs with contracts, floating-point package summary, scratchpad interface variables. *(Ch. 22–24.)*
- **L. Toolchain Quick Reference** *(6 pp)* — libre99asm/libre99gpl flags & directives (E/A deltas flagged), the BENCH99 command set, emulator hotkeys/CLI, xdt99 (xas99/xga99/xdm99) interchange crib, Classic99/js99er/MAME cheat sheets, build-script patterns. *(Ch. 3, 6.)*
- **M. Glossary** *(6 pp)* — every term of art, vintage and modern, one book-wide vocabulary.
- **N. Annotated Bibliography & Resource Guide** *(6 pp)* — the period canon (E/A manual, *TI Intern*, key datasheets, magazines) and the modern corpus (community sites, tool repos, forums), each with "read this for X" notes; provenance/ethics notes on scanned materials.
- *(Exercise solutions and full lab sources live in the companion repository, not the page count.)*

---

## Chapter Dependency Graph (planning aid)

- **Spine (strict order):** 3 → 4 → 5 → 6 → 7 → 8 → 9 → 12 → 13 → 16 → 17 — everything else hangs off this.
- Ch. 1–2 float (any time, ideally first). Ch. 10 needs 7; Ch. 11 needs 7–9.
- Part III internal: 12→13→(14,15)→16→17→18.
- Ch. 19 needs 12–13, 22(§ISR) *referenced forward — keep the sound-list ISR material self-contained in 19, deep dive in 22.*
- Ch. 20 needs 19. Ch. 21 needs 10. Ch. 22 needs 12, 19, 21. Ch. 23 needs 8–9. Ch. 24 needs 22–23.
- Part VI: 25 → 26 → 27 → 28 → 29 (26 also wants 24).
- Part VII: 30 → 31 → 32; 33 needs 10, 21–22; 34 needs 30–31 (+18 for F18A).
- Part VIII: 35 needs 27, 29; 36 needs 31, 35; 37 needs 22; 38 needs 36.
- Part IX needs essentially all of II–VIII per its build sheet (39: +34 optional; 40: deliberately *excludes* 32K material; 41: 31–32, 38; 42: 14, 31, 33; 43: everything).
- Appendix A drafts alongside 7–9; B alongside 26–27; others per their subject chapters.

## Open Questions for the Author (to resolve before Ch. 1)

1. **Title** — pick from the candidates above (affects preface voice).
2. **Case-study naming** — placeholder titles (METEOR BELT, etc.) fine, or brand them now?
3. **PAL coverage depth** — sidebars only (current plan) or first-class dual-timing throughout Part III?
4. **Companion repo hosting** — public from day one (recommended: readers test as we write) or on completion?
5. **Original-hardware photography/diagrams** — ASCII-art only (current plan keeps Markdown pure) or budget for image assets?

*— End of Master Outline v1.0. This document is the contract; propose amendments here before deviating in a chapter session.*

---

## Amendments

**v1.1 (2026-07-05) — Re-founded on the libre99 project.** The book is developed inside the libre99 repository (`docs/ti99book/`), and that project is now the book's foundation: its desktop emulator (`libre99`) is the daily machine, its assembler (`libre99asm`) and GPL toolchain (`libre99gpl`) are the toolchain baseline, and **BENCH99** (a scriptable monitor over the project's `libre99-core` crate, in `code/bench/`) is the debugging/measurement instrument. Classic99, js99er.net, MAME, and the xdt99 suite remain first-class *discussed* tools with explicit roles (GUI debugging and the Ch. 6 period workflow; zero-install/speech/F18A; reference accuracy; period-format and disk interchange). Changes: §3 conventions (assembler/emulator baselines, `[libre99asm]` flag), §4.1 layout (`code/bench/`), Ch. 2 lab, Ch. 3 throughout, Ch. 6 §6.9/lab, Ch. 11 §11.2/11.4/11.6, Ch. 18 lab, Ch. 26 lab, Ch. 27 (libre99gpl), new §28.7 (the project's clean-room console GROM as a case study), Ch. 35 §35.3/35.5, App. B/L. Chapter-facing rule added to the emulator baseline: labs may not assert emulator/toolchain capabilities beyond what the project ships when the chapter is written; gaps are stated and covered by the shelf. Rulings R-12…R-15 in `_style.md` bind the details.

**v1.2 (2026-07-05) — The stub production system.** To let the remaining chapters be finished across many sessions (and by different models) without losing one voice, every not-yet-drafted chapter and appendix now exists as a file in a declared state: `STUB-SKELETON` (template structure + spec pointer), `STUB` (opening vignette, bridge, all section narrative, sidebars, exercises, further reading, and a draft summary written in final book voice; every code listing, table, measurement, and tool-capability claim left as a self-contained, verification-bearing `OPUS-TODO` work order), then `DRAFTED` (orders executed, code machine-verified, support files updated). The system's manual is **`_stubs.md`** (finishing protocol, tag grammar, voice guide, canon card — §4.3's session protocol still applies in full); **`_stub-steering.md`** preserves the population template and per-chapter steering notes for the stubs not yet populated. Ch. 5–8 were populated 2026-07-05 as the models to imitate. The spine order and one-chapter-per-session rule are unchanged; a `DRAFTED` chapter must contain no `OPUS-` tag.
