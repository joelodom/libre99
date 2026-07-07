# `_stub-steering.md` — Resuming Phase-2 Stub Population

*Written 2026-07-05 when the population run was cut short by token budget. Chapters 5–8 are populated STUBs; chapters 9–45 and appendices A–N are still STUB-SKELETONs. This file preserves the exact prompt template and the per-chapter steering notes so any future session — on either machine, with subagents or by hand — can continue producing stubs indistinguishable from ch05–ch08's.*

**Current state is always self-describing** — trust the files, not this page:

```
grep -l "STUB-SKELETON" manuscript/ch*.md manuscript/ap*.md   # not yet populated
grep -l "STUB (Phase 2" manuscript/ch*.md manuscript/ap*.md   # populated, awaiting finishing
```

Two ways to resume, both legitimate:
- **Population first (this pipeline):** turn skeletons into populated stubs (voice + work orders) using the template below — one chapter per subagent/session, commit per chapter (`book: stub chNN — <title> (Phase 2: narrative + work orders)`). Populated stubs were written by Fable to keep one voice; if a different model populates, imitate ch05–ch08 closely.
- **Finish directly:** a finishing session may take any skeleton straight to DRAFTED per `_stubs.md` §1 — the skeleton + outline spec + `_stubs.md` are sufficient. Chapter 5 (already populated) is first in line regardless.

## The population prompt template

Fill {NN} (e.g. 09), {N} (9), {TITLE}, {FILE}, {OFF}/{LIM} (outline Read offset/limit — valid for outline v1.1; if the outline has since changed, locate "### Chapter {N} —" instead), and {STEERING} from the table below.

> You are the Phase-2 stub writer for **Chapter {N} — {TITLE}** of *Programming the TI-99/4A* (the book in `docs/ti99book/` of the libre99 repo). Your job is to put the book's VOICE into this chapter now, and to leave razor-sharp work orders for the technical content that a later finishing session (a different model, with the toolchain running) will execute and verify.
>
> Read these four things, in order, before writing anything (and NOTHING else — no other chapters, no project source, no ledger):
> 1. `manuscript/_stubs.md` — the stub protocol: tag grammar (§3), voice guide (§4), canon card (§5), worked example (§7). It is binding.
> 2. `manuscript/_style.md` — rulings R-1…R-15.
> 3. Your chapter's spec: Read `manuscript/00-master-outline.md` with offset={OFF} limit={LIM} — that bullet list is the contract.
> 4. Your target file: `manuscript/{FILE}` (currently a skeleton).
>
> Then REWRITE the target file completely (one Write call) as a populated STUB.
>
> WRITE IN FULL, FINAL BOOK VOICE (this prose ships as-is):
> - Title line, a one-line italic epigraph, the `<!-- Part … · target … -->` comment, and a STATUS line per _stubs.md §3: `<!-- STATUS: STUB (Phase 2 populated, <date>) — narrative is final-voice; technical cores are OPUS-TODO work orders. Protocol: _stubs.md. -->`
> - The opening vignette (450–700 words, a real scene with a time and place, ends with `---`). Invented period documents must be labeled reconstructions (R-1); name real people only where the record does (R-3).
> - What You Will Learn (6–9 testable bullets).
> - The Bridge (1–3 paragraphs, modern-to-vintage mapping).
> - For EVERY numbered body section: a real section title (`## {N}.k Title`) and the section's narrative frame — motivation, concepts, connective argument: 1–4 paragraphs of real prose. Explain everything that can be explained WITHOUT unverified numbers or untested code; that is usually most of the ideas and none of the tables.
> - Sidebars / Field Notes whose content is history or culture: write them fully. Technical boxes become work orders.
> - Exercises: all 6–12 stems, tiered ✦/✦✦/✦✦✦ (no solutions; where one depends on to-be-measured data, phrase it as "using the §{N}.k table").
> - Further Reading (period + modern; ONLY sources the canon card or your spec names — never invent titles).
> - A `## Summary` block in the register of `_summaries.md`, marked `<!-- SUMMARY-DRAFT -->`.
>
> LEAVE AS WORK ORDERS — `OPUS-TODO` per _stubs.md §3 anatomy (DELIVER / CODE / VERIFY / LEDGER / PROSE):
> - Every code listing, register/format table, measured number, byte-level dissection, bench transcript, and any capability claim about the project's tools beyond the canon card. Orders must be executable without questions: name the source files (`code/ch{NN}/…`), give the exact bench workflow where you can, the expected evidence, the ledger rows to add, and a size budget. Where an order depends on a project capability NOT on the canon card, its VERIFY clause must START with "verify at HEAD (R-12)" and name the shelf tool covering a gap.
> - Typically 6–14 orders per chapter, each 6–15 lines, ids `ch{NN}-<slug>`.
>
> HARD RULES: narrative asserts only canon-card facts, your spec's facts, or safely hedged common history (R-2) — anything else gets an inline `OPUS-VERIFY` flag or moves into an order. Never invent numbers, quotes, credits, or sources. Em dashes set open (word — word). TI hex `>XXXX`, registers R0–R15, mnemonics UPPERCASE. Cross-refs "(Ch. 12)" / "(§17.5)", promising not spoiling (R-7). Flag beyond-E/A constructs `[libre99asm]` (R-13). Target 350–650 lines total.
>
> {STEERING}
>
> Write ONLY this one file. No commits, no builds, no reading or editing any other file. Your final message: one line — `ch{NN} stub populated: <n> lines, <n> OPUS-TODOs, <n> OPUS-VERIFYs` — plus at most two sentences flagging anything the orchestrator should know.

## Per-chapter fills and steering

Format: `NN | FILE | OFF/LIM | steering`. (Offsets valid at outline v1.1.)

- 05 | ch05-console-memory-map.md | 220/11 — **DONE** (commit 74424e1)
- 06 | ch06-assembling-source-object-loaders.md | 232/13 — **DONE** (fa3f88c)
- 07 | ch07-instruction-set-i-moving-data.md | 246/10 — **DONE** (120cdb6)
- 08 | ch08-instruction-set-ii-arithmetic-logic-bits.md | 257/11 — **DONE** (c7e1748)

- 09 | ch09-control-flow-and-program-shape.md | 269/12
  STEERING: R10 = software stack pointer is canon — this chapter BUILDS it; the book-wide calling convention is stated once here, so one order must demand the convention as a boxed contract plus ledger rows. task99 (two coroutines sharing the screen, under 200 bytes) is the lab — machine-verifiable on the bench. BLWP/RTWP linkage facts are ledgered and on the canon card (old WP/PC/ST land in the NEW workspace's R13/R14/R15; RTWP inverts; callee reads caller's Rn at @2n(R13)). R11's discipline (BL link depth one, the bug every beginner writes) continues Ch. 4's setup — lean on it as established.

- 10 | ch10-the-cru.md | 282/9
  STEERING: The 9901/CRU is emulated in the project core (it is how the emulator reads keys) but the lab's "audible clicker via the cassette relay" depends on cassette-relay modeling — that order's VERIFY starts with verify-at-HEAD and shelves to Classic99/real hardware if absent. The R12 ×2 base-address shift is THE classic confusion — give the narrative room to teach it slowly. LDCR/STCR ≤8-bit byte-operand rule + bit-order gotchas = datasheet-verify orders. R12 = CRU base is already canon. The keyboard taste here stays a taste — full treatment is Ch. 21 (promise, don't spoil).

- 11 | ch11-craftsmanship.md | 292/10
  STEERING: §11.6 (scripted BENCH99 runs as CI for 1981 code) is the project's unique strength — give it the chapter's best narrative energy; the same technique the emulator uses to test itself. lib99 is FORMALIZED here: one order defines code/lib99/ layout, module conventions, equates.a99, and the test harness that all later chapters cite. §11.2: our baseline has no macros (canon) — honest treatment of what macros buy and what our COPY-include discipline demands instead.

- 12 | ch12-inside-the-tms9918a.md | 307/10
  STEERING: Ports and side-effectful reads are canon; the core models VDP prefetch — orders can lean on BENCH99 `vdp`/`screen` and must verify VRAM-timing claims at HEAD vs datasheet vs Classic99 (tivdp.cpp is the repo's stated reference — name it for the finisher). vdplib is lib99's first module: entry points mirror the E/A vocabulary (our own VSBW/VMBW/VSBR/VMBR) verified against the E/A-manual behavior contract. Ch. 4's dest-read-before-write MMIO implication pays off here — cash that promise.

- 13 | ch13-graphics-i-and-a-text-engine.md | 318/10
  STEERING: MONITOR99 (live hex viewer/editor) is the lab and a recurring instrument — its order specifies the minimal command set later chapters (25, 26) rely on. The color-by-group-of-eight rule defines TI aesthetics — teach it with love. Borrowing the console font from GROM must NOT get ahead of Ch. 23/25 — recipe-level here with a promise. Screen codes: true ASCII for us; the +>60 bias is TI BASIC's convention only (ledgered).

- 14 | ch14-text-mode-and-multicolor.md | 329/7
  STEERING: Short chapter. Text mode: 40×24, 6-pixel cells, colors from VR7 only, NO sprites — foundation for Ch. 42's editor (promise it). textlib40 variant + PLOT64 multicolor mini-API + mode-carousel lab; port MONITOR99 to 40 columns. Multicolor's obscurity is a story the narrative can enjoy — who used it and why so few (hedge, R-2).

- 15 | ch15-bitmap-mode.md | 337/11
  STEERING: The notorious R3/R4 mask values (>7F/>03) must be machine-verified on the core AND against the datasheet before printing (order). Pixel-address math with lookup tables, Bresenham in 9900-friendly fixed point — listing orders with bench verification. The PC-image pipeline previews Ch. 38 — tool specifics in an order with verify-at-HEAD; python exists Mac-side only, generated data gets committed. The 12K-of-16K cost ledger and "half-bitmap" hybrid layouts are the honest heart.

- 16 | ch16-sprites.md | 349/10
  STEERING: The console ISR's automatic motion (VDP >0780 motion table) is spec, not ledger — verify against TI Intern/Classic99 and the core before asserting. Fifth-sprite flag semantics were recently reworked in the core (5S evaluated at vblank, cleared by status read) — verify at HEAD and machine-verify the flicker-multiplex demo. DODGE is the book's first complete game: order demands a full shippable tree under code/ch16/ plus CQ-82 self-scoring in the postlab prose.

- 17 | ch17-motion-game-loops-scrolling.md | 360/11
  STEERING: 50,000 cycles/frame is canon — the whole chapter negotiates with it. TERRAIN lab with an on-screen frame-budget HUD: orders demand measured cycle budgets from bench traces. The ISR-vs-main VDP address race (the classic crash) is the crown-jewel pitfall — narrative sets it up, an order reproduces the failure mode on the bench deterministically. Pattern-shift smooth scrolling (the Parsec method) built step by step with measured cost per moving region.

- 18 | ch18-advanced-and-modern-vdp.md | 372/9
  STEERING: F18A and 9938 are NOT in the project emulator — the lab explicitly runs stock (project) vs F18A (js99er/hardware); orders shelve accordingly (R-12). The §18.6 bandwidth cookbook = measured VRAM-throughput tables on the bench (same caveats as Ch. 5). Mid-frame register tricks need verify-at-HEAD for timing fidelity — MAME as referee where contested. PAL sidebar only (R-5): 50 Hz budget math.

- 19 | ch19-the-sound-generator.md | 387/12
  STEERING: The PSG is emulated (canon). The note-table generator is a Python tool: note that the PC workstation lacks python (Mac side has it) and the GENERATED table must be committed to code/ch19/ so builds never need python. The console sound-list format + ISR auto-player (pointer/flags in scratchpad) is the platform-unifying story — its exact bytes need TI Intern/E-A verification orders. TI Invaders Field Notes = debugger-log archaeology (project log or Classic99); verify the title is in the embedded 137 before asserting.

- 20 | ch20-the-speech-synthesizer.md | 400/11
  STEERING: The project does NOT emulate speech (canon gap) — every lab order routes VERIFY through js99er or real hardware, and the narrative states the gap once, plainly (the R-12 exemplar chapter; check at HEAD in case speech landed). LPC theory + Speak & Spell history are safe narrative (ledgered: Breedlove/Wiggins/Frantz/Brantingham, 1978). Ports >9000/>9400 are canon. Modern encoder tools (BlueWizard / python_wizard class): orders verify current names/availability before printing.

- 21 | ch21-keyboard-joysticks-9901.md | 416/10
  STEERING: KSCAN interface bytes (>8374/>8375) and the Alpha Lock trap are spec facts needing verification orders (E/A manual, TI Intern, Classic99 source). The bench `k` command holds a key for whole frames (canon) — inplib's edge-detection can be machine-verified with it; spell that workflow. Ghosting/masking of the real matrix: verify how faithfully the core models the matrix at HEAD before asserting demos.

- 22 | ch22-interrupts-and-time.md | 427/11
  STEERING: The user ISR hook (spec says >83C4) is NOT yet ledgered — verify before asserting (TI Intern/Classic99). The console ISR walk (§22.3) is behavioral description — never reproduce TI's listing (same discipline the repo uses). 9901 timer emulation fidelity: verify at HEAD; MAME referee if contested. PROFILE99's interface must be pinned (order) because Ch. 37 consumes it. LOAD/nonmaskable sidebar: Explorer trick (ledgered, Millers Graphics).

- 23 | ch23-console-rom-services.md | 439/10
  STEERING: All entry addresses and contracts are verify-first orders (E/A manual + TI Intern; machine-verify calls on the bench under the real firmware where possible). The E/A-environment dependence is the honest core: what breaks in a bare cartridge, and the §23.6 cart-safe shims — built as lib99 modules with documented contracts, REUSED in Ch. 35 (make the interface order explicit). Radix-100 FP format dissected via bench memory dumps of FAC/ARG.

- 24 | ch24-the-scratchpad-atlas.md | 449/9
  STEERING: Table-heavy chapter: the three environment maps (bare console / E/A-loaded / cartridge-with-GPL) are the deliverable — orders source every byte range (TI Intern + E/A manual + observed PADWATCH diffs) and mark each row's evidence tier; the full byte table lands in App. C, this chapter narrates the geography. PADWATCH lab = before/after scratchpad diffing around any console call — pure BENCH99, spell the workflow. >83C0 ISR ws / >83E0 GPL ws are canon.

- 25 | ch25-grom.md | 462/10
  STEERING: The port protocol (read-address off-by-one, prefetch) is modeled in the core (canon: "real prefetch quirk"; the repo's own boot bug was the address-byte-selector reset — cite repo CLAUDE.md's note for the finisher) — orders machine-verify port semantics on the bench and cross-cite Classic99's ReadValidGrom. GROMDUMP "to disk" hits the embedded-disks-only gap — verify at HEAD; shelf = dump via bench `m` to a host file instead, honestly stated. GROM bases ×16 and the p-code card story: hedge + verify.

- 26 | ch26-the-gpl-language.md | 472/12
  STEERING: GPL semantics come from fan-reconstructed manuals plus the project's OWN proof (libre99gpl built a working console GROM — canon) — orders lean on `libre99gpl dis` of console GROMs for ground truth and on tracing the interpreter on the bench (the lab: both sides of the act). The spec's "GROM-fetch log" capability claim needs verify-at-HEAD. §26.8's cost measurements = bench cycle counts of the fetch-dispatch loop per opcode class — doable and delicious; spell the workflow. FMT gets full sub-language treatment (grammar table → App. B order).

- 27 | ch27-writing-gpl-today.md | 484/10
  STEERING: libre99gpl's actual CLI/directives are NOT on the canon card — the FIRST order must be "verify libre99gpl at HEAD (crates/libre99-gpl README/source; the clean-room system GROM source is the style exemplar)" and every later order builds on that verified reality. QUIZMASTER must run from the real console menu — machine-verifiable end-to-end like HELLO's "2 FOR HELLO, 1981" (canon pattern). The GPL header at G>6000 mirrors Ch. 3's ROM header dissection — same pedagogy, GROM twin. Shipping formats beyond .ctg (FinalGROM images, .rpk): verify what the tools emit at HEAD; state gaps + shelf (xga99).

- 28 | ch28-the-os-in-grom.md | 494/11
  STEERING: §28.7 (the clean-room console GROM tour) is the chapter's unique asset — the narrative may celebrate it (canon: original title/menu + TI PYTHON REPL, boots via --system-grom, one documented cart incompatibility); orders verify details against original-content/system-roms docs at HEAD. Boot-trace lab: bench `boot` + `s`/`u`, annotating the first ~500 GPL instructions BEHAVIORALLY (describe, never transcribe TI's code). TI BASIC dissection: tokens in VRAM observed live via bench `m`/VRAM reads — quantify the two-interpreter tax with a measured order. Console-version field guide: hedge + verify (community record).

- 29 | ch29-hybrid-architecture.md | 505/10
  STEERING: XML user vector tables in scratchpad = verify-first (TI Intern). The .ctg container supports GROM regions (canon) but the AUTHORING path for ROM+GROM hybrids (libre99asm + libre99gpl choreography) needs verify-at-HEAD — it is the lab's heart and possibly a project roadmap surface. The hybrid skeleton chassis is REUSED by Part IX — its order defines the template contract precisely. Case dissection of a first-party game's labor split: debugger observation, behavioral description only.

- 30 | ch30-dsrs.md | 519/10
  STEERING: The project runs the genuine disk DSR (canon) so our hand-written DSRLNK is machine-verifiable against a real device on the bench — spell that workflow. NULCARD (a build-it-yourself virtual card) needs a core capability that may not exist at HEAD (mounting a custom DSR ROM): the order must verify first and, if absent, surface it to Joel as roadmap work and restructure onto the shelf (verify Classic99/MAME custom-DSR paths before naming them). CRU geography of a loaded PEB: App. G feeds from here.

- 31 | ch31-file-io-pabs.md | 529/11
  STEERING: PAB fields/opcodes are E/A-manual facts — verification orders against the manual, then machine-verified end-to-end via the emulator's real disk DSR (FILER99 can genuinely run). The embedded-disks/DSK1-only gap constrains WRITE tests — verify at HEAD whether writes persist (likely in-memory only); orders route around honestly, and persistent-write support may be roadmap surface for Joel. The >8356 name-pointer dance: TI Intern verification. FILER99 over textlib40 is the most reusable artifact yet — full tree order.

- 32 | ch32-disk-internals.md | 540/11
  STEERING: VIB/FDR structures triangulate three ways: xdm99 (canon interchange), the project's embedded disk images (bench `m` reads through the DSR), Classic99. SSSD geometry (90 KB = 40×9×256) is ledgered. DISKDOC's corrupt-and-repair exercise needs a scratch disk — embedded-only gap: shelf = xdm99-built image on Classic99, or surface .dsk-mounting as roadmap. Controller zoo (CorComp/Myarc densities): hedge + verify; capability detection over assumptions.

- 33 | ch33-wires-out-rs232-pio-cassette.md | 551/9
  STEERING: RS-232/PIO/cassette are very likely NOT emulated at HEAD (verify; canon lists the core's devices) — a shelf-heavy chapter: verify EACH shelf claim (Classic99's RS232/PIO redirection, MAME, js99er cassette) before naming it; real hardware is the honest target, TIPI the modern alternative (promise Ch. 34). The WAV-mastering tape trick (load a real console from a phone) is delightful and real — order it with verification of the encoding scheme (E/A manual/TI Intern) and a committed known-good audio asset.

- 34 | ch34-modern-peripherals-tipi-sams-f18a.md | 560/10
  STEERING: None of the three is in the project emulator — verify at HEAD, state plainly; js99er has F18A (canon), MAME may have SAMS/Geneve-side pieces (verify), TIPI needs real hardware (or its own emulation ecosystem — verify before naming). This is the R-12 doctrine chapter: stock-first, enhance-if-present, detection recipes consolidated — the narrative should own that stance proudly. SAMS mapper model + CRU enable: community-docs verification orders; the lab's SAMS streamer must degrade gracefully on stock hardware by design.

- 35 | ch35-cartridge-engineering.md | 574/10
  STEERING: The emulator embeds 137 carts including banked ones, so the core models banking — verify WHICH schemes at HEAD (write-to->60xx 2-bank; 378/379 inverted multi-bank) and machine-verify the banked-DODGE lab end-to-end. libre99asm multi-bank authoring ("per its spec" in the outline) = verify-first; if absent, honest gap + build-script workaround + roadmap surface. Manual headers return here via AORG absolute mode (canon: AORG is absolute-only) — hello_cart's retirement (ledgered) gets its payoff. Ship-format matrix: .ctg / padded .bin (canon) / .rpk + FinalGROM naming (verify).

- 36 | ch36-program-architecture.md | 584/11
  STEERING: SKELETON99 is the template every capstone instantiates — its order is a specification document (phases, budgets, entity-table layout, 8.8 fixed-point house standard, 256-degree angles, sine tables, data hooks); demand precision, it gets consumed four times. The XB CALL LINK fenced sidebar is the book's ONLY Extended BASIC allowance — one page, ABI only (scope law; say so in the order). VRAM-as-warehouse costs cite Ch. 12/18 measured tables — don't re-measure. Save-state codes/passwords fallback: computed and verified — a listing order.

- 37 | ch37-optimization.md | 596/12
  STEERING: The most measurement-dense chapter: nearly every section's order ends on the bench with cycle numbers. §37.8's case study must be REAL — take Ch. 17's TERRAIN from ~65% to ~31% of frame budget decision-by-decision with the profiler as referee (order: publish actual before/after numbers; the percentages are outline TARGETS — verify what reality gives and keep the narrative honest). Placement doctrine inherits Ch. 5's measured rows; respect the MOV dest-pre-read caveat wherever destinations re-read. Self-modifying code with bright ROM-target warnings. The dojo lab: five routines, target counts, harness-checked scoreboard.

- 38 | ch38-data-compression-asset-pipeline.md | 609/10
  STEERING: Python tools live in code/tools (outline §4.1) — but the PC workstation has no python (Mac does); therefore every generated artifact is COMMITTED and sh verify.sh never invokes python (state this in the orders; it is already the repo's law). The 24K→12K crunch lab needs honest measured ratios (orders: measure, never assume) and the 600-byte decompressor budget is a target to verify. LZ77-family decompressor on the 9900: size/speed measured on the bench. The build's memory report: let the finisher choose sh-compatible tooling.

- 39 | ch39-capstone-scrolling-shooter.md | 621/15 (Read covers the Part IX preamble at 621–623)
  STEERING: The four-beat arc (Archaeology / Specification / Construction / Postmortem) REPLACES the standard body shape; keep the template's other fixtures (vignette, WYWL, bridge, exercises, further reading, summary). Reimplement mechanics and techniques, NEVER copy code or assets. Archaeology instruments the genre-definer: verify Parsec is in the embedded 137 at HEAD (else Classic99's licensed bundle) and describe observations behaviorally. METEOR BELT ships as a four-bank cart with a written 1982-style manual (label it a reconstruction, R-1). A 34-pp two-session chapter — say so in a header note and split the orders into Session A (arch+spec+engine) / Session B (systems+presentation+ship).

- 40 | ch40-capstone-fixed-screen-arcade.md | 636/9 (also read the preamble: offset 621 limit 3)
  STEERING: Part IX arc again. The console-only constraint IS the pedagogy: GRIDRUNNER 99 uses scratchpad + VRAM only, no 32K anywhere — orders enforce it and machine-verify on the emulator configured as an unexpanded console (verify at HEAD how to disable 32K, else surface to Joel; the game must also RUN with 32K present, simply unused). Munch Man / TI Invaders archaeology via the embedded library (verify presence). Attract/demo playback via recorded inputs — verify input capture on the bench; else deterministic scripted replay.

- 41 | ch41-capstone-rpg-engine.md | 646/11 (also read the preamble: offset 621 limit 3)
  STEERING: Part IX arc. DUNGEONS OF FATE = engine + QUEST-FILE format so readers ship new games without reassembly — the versioned binary schema + matching EQU header (single source of truth, Ch. 38 pattern) is the chapter's heart; demand a precise format-spec order. Disk-based: the embedded-DSK1-only gap shapes everything — verify at HEAD whether arbitrary-disk mounting landed; if not, quest files may ship embedded via a build step, and .dsk mounting becomes an explicit roadmap surface for Joel. The Python quest-builder runs Mac-side; its OUTPUT is committed. Tunnels of Doom archaeology: verify presence in embedded library.

- 42 | ch42-capstone-productivity.md | 657/9 (also read the preamble: offset 621 limit 3)
  STEERING: Part IX arc. AUTHOR99: gap buffer on the 9900 (why it wins at 3 MHz — the narrative argument is yours now), textlib40 under continuous editing, DV80 files TI-Writer itself accepts — the DV80 round-trip order is machine-verifiable if TI-Writer is in the embedded library (verify; else xdm99-level verification of the file bytes). Keystroke-to-glass latency measured on the bench = productivity's frame budget — spell the measurement. Printing via PIO/RS232 hits Ch. 33's emulation gap — orders shelve honestly. "This book's errata were drafted in it" stays an aspiration, not an assertion.

- 43 | ch43-capstone-the-port.md | 667/7 (also read the preamble: offset 621 limit 3)
  STEERING: Part IX arc. The port needs a CHOSEN modern indie design — an authorial decision: the candidate-selection order must present 2–3 candidate designs (simple rules / deep play / NO IP entanglement — original mechanics only, or the finisher designs an original with modern sensibilities) and instruct the finisher to confirm with Joel or default to the safest original design. The constraint-translation table (resolution/palette/channels/RAM/input) is the reusable artifact. The graduation rubric ties back to CQ-82.

- 44 | ch44-the-extended-family.md | 679/8
  STEERING: Survey chapter, shelf-heavy and honest: Geneve/9938 = MAME territory; F18A GPU = js99er/real hardware; GCC-for-TMS9900 = verify the community toolchain's current state before printing ANY invocation; UCSD Pascal / Forth / c99 samples: verify sources. The lab (C module calling lib99; run a Part IX game on emulated 9938) may exceed available tooling — orders verify-first and scale the lab honestly to what exists. Keep it light: 14 pp.

- 45 | ch45-the-living-platform.md | 687/8
  STEERING: The most narrative chapter in the book — write nearly all of it NOW. Community atlas: hedge names per R-2 with OPUS-VERIFY flags on anything specific (forums, faires, contests). Publishing/preservation ethics continue Ch. 1's and this repo's own stance (clean-room GROM as the ethical showpiece — canon). The closing essay ("Why program a dead machine?") is the book's last word; land the holding-a-whole-computer-in-your-head theme seeded in Ch. 1–2. Few orders: name/URL verification, the portfolio checklist cross-checked against actual chapter deliverables, and a final-pass order to reconcile forward references made by earlier chapters.

## Appendix batches (voice-light; suit a cheaper model)

Common frame: read _stubs.md (§3/§5/§6), _style.md, and outline offset=697 limit=17; rewrite each target as: 1-paragraph scope intro, a complete `## Contents plan` (every table/catalog as headings with 1–2-sentence descriptions), and dense OPUS-TODO orders (ids `apX-<slug>`), each naming DELIVER / VERIFY (primary source; machine-verify on the toolchain where possible; evidence tiers per _stubs.md §6) / LEDGER / PROSE-size, and noting WHICH chapter's drafting session feeds it. STATUS line: `<!-- STATUS: STUB (Phase 2 populated, <date>) — reference-appendix stub; populate alongside subject chapters. Protocol: _stubs.md. -->`

- **A/B/C** — apA: per-instruction reference template + opcode matrix + cycle formulas with wait-state math (TMS9900 datasheet; Classic99 cpu9900.cpp WStatusLookup/BStatusLookup as the repo's cross-check; bench-verify formulas; alongside Ch. 7–9; footnote the MOV dest-pre-read deviation until fixed). apB: GPL opcodes/encodings/FMT grammar + libre99gpl syntax mapping, xga99 deltas (fan-reconstructed manuals + libre99gpl at HEAD; alongside Ch. 26–27). apC: map poster + >8300–>83FF byte table with per-environment ownership + lib99 layouts (feeds from Ch. 5/24; evidence tier per row).
- **D/E/F** — apD: 9918A registers/status/mode layouts/VRAM timing cookbook/palette with RGB approximations (datasheet + core + Ch. 12/18 measurements; RGB values are community-measured — hedge). apE: PSG command bytes/frequency-attenuation/note table (committed Ch. 19 artifact)/sound-list grammar. apF: 5200/5220 commands/status/resident-vocabulary catalog with addresses/LPC frames/allophones (community docs + datasheets; NO project bench path — speech not emulated; tier-2/3 evidence stated).
- **G/H/I** — apG: CRU allocation >0000–>1FFE + 9901 bit map + card bases (Ch. 10/21/30; TI Intern + card manuals). apH: PAB layout card/opcode-error matrices/DSR headers/device names (E/A manual; bench-verify against the live disk DSR). apI: VIB/FDR + file-type matrix + tagged-object + EA5 + cassette block + cart image conventions incl. the project-native .ctg spec (verify at HEAD; alongside Ch. 6/32/35).
- **J/K/L** — apJ: character patterns + KSCAN code tables + matrix diagram (E/A manual; bench `k` + KSCAN verification). apK: ROM vectors + GPLLNK/XMLLNK catalogs with contracts + FP summary + scratchpad interface variables (TI Intern + E/A manual + Ch. 22–24 rows). apL: toolchain quick reference — libre99asm/libre99gpl flags AT HEAD (run the tools), BENCH99 command set (canon), emulator hotkeys/CLI (cite README per R-12), xdt99 crib, shelf cheat sheets, R-14 build patterns — much is verifiable immediately; mark those orders executable now.
- **M/N** — apM: glossary seeded from every canon-card handle + per-part term batches; standing order: harvest terms as each chapter drafts. apN: annotated bibliography — period canon (E/A manual — the repo recently gained a TI-99 book download; check what it actually is before citing it as the in-repo copy; TI Intern; datasheets; magazines) + modern corpus with "read this for X" notes + provenance/ethics notes; verify every name/edition (R-2); no unverified URLs.
