# ERRATA — full-manuscript review pass

**What this is.** A cover-to-cover technical review of the manuscript (all
drafted chapters and appendices, plus detailed guidance for the stubs),
conducted as one continuous pass by a reviewer reading with deep TI-99/4A
domain knowledge and checking claims against the repository at HEAD
(2026-07-07, session 10). It is written to be **worked from**: each entry is
self-contained, names its location, states the problem, and proposes a fix.
Work an entry, then delete it (or mark it `DONE (commit)` if you want the
record); when a chapter's section is empty, the chapter is clean.

**Severity codes.**

| Code | Meaning |
|---|---|
| `[BUG]` | A factual/technical error a reader could be misled by |
| `[STALE]` | True when drafted, false at HEAD (usually: the book describes the predecessor repo) |
| `[INCONSISTENT]` | Contradicts the ledger, another chapter, or itself |
| `[STYLE]` | Violates `_style.md` / the outline's conventions |
| `[GAP]` | A required element is missing or an important caveat unstated |
| `[SUGGEST]` | Not wrong — a strong improvement worth the edit |
| `[VERIFY]` | Could not be confirmed from sources on this PC; verify before/while fixing |
| `[STUB]` | Guidance for an unfinished chapter/appendix |

**Review scope and method.** Every `.md` under `manuscript/` read in full,
cover-to-cover in book order. Technical claims spot-verified against the
tier-1 sources on this PC (the `libre99-core`/`-asm`/`-gpl` sources, the
clean-room firmware under `original-content/system-roms/`, the project README
and `docs/USER-GUIDE.md`, BENCH99's source in `code/bench/`) and against
Classic99 where the book cites it. Not re-verified: the bench transcripts and
cycle numbers already pinned in `_ledger.md` (they were machine-verified at
draft time and are internally consistent); prose history claims hedged per
R-2.

---

## G — Global / cross-cutting

**G-1 `[STALE]` — THE BIG ONE: the book describes the predecessor repository's
emulator, not this one.** The manuscript was drafted inside the private
`ti-99-emulator` repo; this repo (Libre99, born 2026-07-06) is its IP-clean
fork, and the machine changed in ways the book asserts confidently and often:

| The book says (Ch. 3 §3.2 et al.) | The truth at HEAD (README, USER-GUIDE) |
|---|---|
| Runs/embeds the **genuine TI firmware** | Boots the **clean-room firmware by default**; authentic firmware is user-supplied via `--system-rom`/`--system-grom` (needed only for TI BASIC; Extended BASIC runs on the clean-room substrate) |
| **137 period cartridges + 15 disk images embedded**; `--cartridge <name>`/`--disk <name>` pick from the library; `--list` prints it | **Zero third-party bytes embedded.** Media are files you supply: `--cartridge <path>` (alias `--cartridge-file`), `--disk <path>`, `--disk-dsr <path>` |
| **F9 = in-app media browser** with titles and ROM/GROM makeup | **F9 = the OS-native file chooser** (mount `.ctg`/`.dsk`); `F2`/`F3` eject; `F4` disk-memory overlay |
| "Disks mean the embedded library for now; arbitrary `.dsk` mounting is on the roadmap" | Inverted: arbitrary `.dsk` mounting **shipped**; there is no embedded library |
| Disk controller "running the genuine disk DSR ROM" | Default is the **clean-room disk DSR** (`--disk-dsr` swaps in an authentic one) |

*Fix direction (recommended):* re-found Ch. 3 §3.2/§3.5/§3.7/§3.8 and every
lab that says "pick X from the embedded library" on the IP-clean reality.
This is not damage control — it is an upgrade to the book's own ethics
narrative: the daily machine now *boots the book's showpiece* (the clean-room
firmware of §28.7) by default and embeds nothing it doesn't own. Labs that
need period titles (Ch. 1 artifact hunt, Field Notes throughout) should name
js99er.net (built-in software library) and Classic99 (licensed bundle) as the
supported paths, with "your own dumped images on libre99" as the third.
Per-chapter callouts below tagged `(G-1)`. R-12 already mandates citing the
README for usage details rather than duplicating them — apply it while fixing.

**G-2 `[GAP]` — BENCH99 requires authentic firmware a fresh checkout doesn't
have.** `code/bench/src/main.rs` loads `roms/994aROM.Bin`/`994AGROM.Bin` via
`libre99_core::third_party::load` and **exits with an error** if `third-party/`
isn't populated. Every bench lab from Ch. 2 on ("bring-up is three lines")
silently assumes a maintainer-local directory that is git-ignored and absent
from a public clone. *Fix options:* (a) make the bench boot the clean-room
firmware by default (as the app does) and take authentic images via flag/env —
the better fix, but re-run any bench transcript whose numbers depend on
authentic GROM contents (boot-time instruction counts do; the bare-bench
cycle measurements don't); or (b) state the requirement plainly in Ch. 2's and
Ch. 3's lab setup and in `setup.sh`. Decide once; apply everywhere.

**G-4 `[SUGGEST]` — State the bit-numbering convention once.** The book
uses TI MSB-first numbering for status bits (ST0 = MSB, Ch. 4) and address
lines (A15 = LSB, Ch. 5 Pitfalls), but modern LSB-0 numbering for data-word
bits (Ch. 8's packed-word diagram labels the MSB "bit 15"; §8.5 "bit 15 of
the result is the sign"). Both choices are individually right; the *mix* is
never declared. Add one declaration — Ch. 4 §4.4 or App. A's front matter —
e.g. "data bits are numbered by value (bit n = 2^n); TI's hardware documents
number lines and status bits from the MSB, and we follow them there." Then
audit App. G (CRU bit numbers) and App. B/D tables for consistency with it.

**G-3 `[STALE]` — The Editor/Assembler manual's advertised location.** Ch. 3
Further Reading says "a copy lives in the repository's `assembler/` folder";
Ch. 6's sidebar says "a copy lives in the project repository." At HEAD it
lives at `third-party/Editor_Assembler_Manual.pdf` — maintainer-local,
git-ignored, absent from a public checkout. Reword both to "the community
archives host scans" (or similar), not a repo path.

---

## Part I — The Machine and Its World

### Chapter 1 — Genesis and Fall

- **1-1 `[STALE]` (G-1)** — Lab 1: "clone the libre99 repository … Parsec,
  Munch Man, and TI Invaders are all in its embedded library, an `F9`
  keypress away." No embedded library at HEAD; F9 is a file chooser. *Fix:*
  make js99er.net (which carries these titles in its built-in library) the
  primary artifact-hunt machine, Classic99's licensed bundle the second, and
  libre99-with-your-own-images the third.
- **1-2 `[STYLE]`** — Missing blank line between a blockquote and the next
  heading at two places (end of the "Decoding a Cartridge PCB" Field Notes →
  `## 1.5`; end of the "Retailer's Memo" sidebar → `## 1.8`). Renders fine in
  most parsers but is inconsistent with the file's own spacing everywhere else.

### Chapter 2 — Grand Tour

- **2-1 `[BUG]`** — Pitfalls box: "a **264-byte fast island** (8 K ROM + 256 B
  pad + nothing else)." Unit-mixing arithmetic: 8 K + 256 B = 8,448 bytes
  (8.25 K), not 264 bytes. Say "an 8.25 K fast island" or "a fast island of
  8 K ROM + 256 B RAM."
- **2-2 `[STALE]` (G-2)** — Lab 2 Part A: "`boot` — the real firmware boots to
  the master title screen." The bench needs `third-party/` populated (or the
  G-2 fix). Also, if G-2(a) is taken, "the real firmware" becomes the
  clean-room firmware and Part A's narration ("you have caught the GPL
  interpreter mid-heartbeat") still holds — the clean-room ROM's interpreter
  loop serves the same lesson.
- **2-3 `[SUGGEST]`** — §2.3 memory-map table, GROM row: the range
  `>9800`–`>9C02` is glossed "read data / read addr / write addr ports,"
  omitting `>9C00` (GROM/GRAM write **data**). The chapter summary lists all
  four ports; make the body row match (the omission will confuse exactly the
  reader who cross-checks against Ch. 25).

### Chapter 3 — The Workshop

- **3-1 `[STALE]` (G-1)** — §3.2 "the daily driver" paragraph is the epicenter
  of G-1: genuine-firmware claim, 137 cartridges/15 disks, `--cartridge`/
  `--disk` by *name*, `--list`, F9 media browser "with each cartridge's title
  and ROM/GROM makeup." Rewrite on the IP-clean reality (see G-1 table).
  Same paragraph's honesty notes: the disk note is inverted (arbitrary `.dsk`
  now mounts; no embedded library); the speech and GUI-breakpoint gaps are
  still true — keep them.
- **3-2 `[STALE]` (G-1)** — §3.8 Pitfall "ROM provenance, said plainly":
  "This project embeds the authentic console firmware and a library of period
  cartridges…" Now false — and the truth is stronger: the binary carries zero
  third-party bytes, and the clean-room firmware is the default boot. Rewrite
  the pitfall to state the current provenance model (`third-party/` is
  maintainer-local for differential tests only), keeping the Classic99-bundle
  and clean-room-GROM sentences, which are still accurate.
- **3-3 `[STALE]` (G-1)** — §3.7 "Build and run, every way," path 1: with the
  clean-room default boot, the menu that enumerates HELLO is the *Libre99*
  selection menu, not "the real 1981 firmware" — either note that, or note
  that `--system-rom`/`--system-grom` restores the authentic experience.
  (`--cartridge-file` still works as an alias; the primary flag is now
  `--cartridge <path>`.)
- **3-4 `[VERIFY]`/`[BUG]`** — §3.7 header dump, the `>6015` line: "'HELLO,
  1981' (+ a pad byte to stay even)". The arithmetic contradicts the pad
  byte: the 11-byte name occupies `>6015`–`>601F`, so the next address is
  `>6020` — already even, no pad — and the dump itself puts `START` at
  `>6020`. Either the pad-byte parenthetical is wrong or the entry address
  is. Rebuild HELLO and check the actual bytes; fix whichever is wrong.
- **3-5 `[STALE]` (G-3)** — Further Reading: E/A manual "copy lives in the
  repository's `assembler/` folder" — see G-3.
- **3-6 `[STALE]` (G-1)** — Lab 3 step 1: "run one period title from Chapter
  1's artifact hunt" via the F9 media browser — needs the G-1 fix (user
  supplies the image, or the step moves to js99er/Classic99). The summary
  bullet for §3.2 repeats the 137/15 counts and the F9-browser claim; fix the
  summary in the same edit (and the copy in `_summaries.md`).

---

## Part II — The TMS9900 and Assembly Fundamentals

*(Cross-cutting note added during this part: see G-4, bit-numbering
convention.)*

### Chapter 4 — The TMS9900

- **4-1 `[BUG]`** — Exercise 4.7's stated premises: "`MOVB Rs,@addr`
  C=14/M=4." That is `MOV R,R`'s cost, not this instruction's. Correct
  figures: **C=22** (14 base + 8 symbolic destination), **M=5** on hardware
  (fetch, operand word, source-register read, destination pre-read,
  destination write) — M=4 on the current core, which skips the pre-read.
  As written, a student's loop total cannot reconcile with Ch. 3 §3.8's own
  bench transcript (which prices the analogous `MOVB` at 34 from cartridge)
  or Ch. 7 §7.6's tables. The `DEC` and `JNE` premises are correct.
- **4-2 `[SUGGEST]`** — §4.4's drill table and Pitfalls say "A15 ignored" —
  correct in TI numbering (A0 = MSB … A15 = LSB), but Ch. 4 never states
  that address-line convention; a newcomer reads A15 as the top bit. Add
  one parenthetical at first use. (Ch. 5's Pitfalls says "A15 in TI's
  numbering — Ch. 4," pointing at an explanation Ch. 4 doesn't quite give.)
- **4-3 `[VERIFIED-OK]`** — §4.8's honesty note (core executes `MOV`/`MOVB`
  without the destination pre-read) re-verified true at HEAD
  (`cpu.rs` — MOV/MOVB write without reading dst). No book change needed;
  remove this row only when the core fix lands (then Chs. 4, 5, 7 all need
  their deviation notes retired together — grep for "pre-read").

### Chapter 5 — The Console Memory Map

- **5-1 `[BUG]`** — §5.4, the mirror-decode explanation: "the two bits that
  would tell `>8000` from `>8100` from `>8200` from `>8300` — **A8 and A9 in
  the CPU's numbering**." Under TI numbering — which this same chapter's
  Pitfalls box explicitly uses ("A15 in TI's numbering") — the `>0200` and
  `>0100` address lines are **A6 and A7**. "A8/A9" is modern LSB-0
  numbering. Fix to "A6 and A7 (the `>0200` and `>0100` lines)" and check
  Ex. 5.9's answer key expectation.
- **5-2 `[BUG]`** — §5.6, the GROM demo's explanation: "the two reads of the
  same address `>9802` return different bytes … **because each read advances
  the port**." Wrong mechanism: reading the GROM *address* port does not
  advance an address — the port returns the internal counter's high byte,
  then its low byte (a byte-selector toggle). Also worth one forward-looking
  clause: the value that reads back (`>60` then `>01` = `>6001`) is the
  written address **+1** — the prefetch off-by-one the trace displays but the
  prose never mentions (Ch. 25 owns it; a "(why +1? Ch. 25)" would turn a
  confusing datum into a hook).

### Chapter 6 — Assembling *(STUB — guidance for the finishing session)*

- **6-1 `[STUB]` (G-1)** — The work orders `ch06-ea-ritual`, `ch06-minimem`,
  and `ch06-lab` each say "verify at HEAD whether the project's embedded
  media include the E/A cartridge (and any usable E/A disk among the 15
  embedded disks / 137 embedded cartridges)." At HEAD the answer is fixed
  and permanent: **the project embeds no media at all.** Rewrite those
  VERIFY clauses now so the finisher doesn't chase them: legs (a)–(c) and
  the Mini Memory session run on Classic99's licensed bundle (its canon
  shelf role); libre99 participates only with user-supplied images.
- **6-2 `[STUB]` (G-3)** — Both OPUS-VERIFY hedges about the E/A manual
  "copy in this book's repository": the copy is at
  `third-party/Editor_Assembler_Manual.pdf` — maintainer-local and
  git-ignored, absent from a public clone. Cite it as the working copy used
  for page-cites, but don't promise readers a repo path.
- **6-3 `[STUB]`** — Reviewer's seeds for the §6.4/§6.5/§6.6 work orders
  (tier-4 recollection — verify against the E/A manual exactly as the
  orders already demand, but these give the finisher the expected shape):
  tagged-object tags ≈ `0` header/IDT+length, `9`/`A` abs/rel load address,
  `B`/`C` abs/rel data word, `3`/`4` REF, `5`/`6` DEF, `1`/`2` abs/rel
  entry, `7` checksum, `8` ignore-checksum, `F` end-of-record, `:`
  end-of-file. EA5 header = three words (more-files flag, total length,
  load address); continuation files increment the last filename character;
  Option 5 begins execution at the first file's load address. REF/DEF
  table: grows downward from `>4000` in low expansion, 8-byte records
  (6-char blank-padded name + word address).
- **6-4 `[STUB]``[STYLE]`** — The vignette (the Elkhart teacher) is written
  as dated reportage with no composite/reconstruction signal. Ch. 5's and
  Ch. 7's vignettes both flag themselves ("we have to reconstruct the
  scene"; "the room is a composite"). Add one clause in the same spirit
  (R-1's principle, applied to narrative).

### Chapter 7 — Instruction Set I

- **7-1 `[BUG]`** — Lab 7's measurement table, `MEMSCN` row: printed
  **40** island / **44** buffers-in-expansion cycles per byte. The actual
  loop in `code/ch07/memlib.a99` (`CB *R1+,R2` / `JEQ` / `DEC` / `JNE`)
  costs, by the core's own cycle model (Format I base 14, byte
  autoincrement +6, jumps 10 taken / 8 not), **48 / 52** — the printed
  figures look like the not-taken `JEQ` (8 cycles) was dropped from the
  count. Consequence: the reading "`MEMSCN` is the cheapest" is then false
  (`MEMCPB` = 46 island). Re-measure on the bench, fix the row, and rewrite
  that sentence (the true claim that survives: `MEMSCN` is the only routine
  immune to the dest-pre-read deviation, and its *expansion delta* is the
  smallest because it only reads).
- **7-2 `[SUGGEST]`** — §7.1's mode-cost table column "Extra accesses"
  silently counts only accesses that can land outside the workspace (e.g.
  `*Rn+` shows "+1" where the datasheet counts the register write-back
  too). Operationally right for funnel accounting, but App. A will
  reconcile against datasheet access counts — add a one-line footnote
  saying which accounting the column uses.

### Chapters 8–11 (reviewed clean)

Reviewed closely (flag tables, DIV/MPY register-pair choreography, shift
cost slopes, the R-16/R-17 statements, CRU bit addressing, keyscan method,
COPY resolution, SYSCHK design) against the datasheet model, the core
source, and Classic99 lore — **no errata found**. Two notes: Ch. 8's
packed-word diagram is where the bit-numbering convention question (G-4)
first bites; Ch. 10's and Ch. 11's R-12 gap statements (no keypress+step
bench mode; no DSR ROM installable from the bench; no unexpanded-console
model; cassette relay unemulated) should be spot-checked against HEAD in a
later pass — if the project has since grown any of these, the gap notes
become stale in the *good* direction.


---

## Part III — The Video Display Processor

### Chapter 12 — Inside the TMS9918A

- **12-1 `[BUG]`** — §12.2, the one hand-done handshake example ("aiming the
  counter at VRAM address `>0100` for writing"): the code loads
  `LI R0,>0040`, which sends `>00` then (after `SWPB`) `>40` — that aims
  **`>0000`**, not `>0100`. The in-line comment even says the second byte is
  ">41 → 01_000001", contradicting the constant two lines above. Fix:
  `LI R0,>0041` (then the `MOVB R1,@VDPWD` comment ">AA lands in
  VRAM[>0100]" becomes true).
- **12-2 `[BUG]`** — §12.6 "When it is safe": "the **~4,300-cycle** gap each
  frame when the beam is off the visible area." The vertical-blank interval
  is ~70 of 262 lines ≈ **13,400 CPU cycles** (≈ 4.5 ms at 3 MHz). The
  printed figure looks like the *millisecond* value mislabeled as cycles.
  Fix to "~13,000-cycle (≈ 4.5 ms) window" — and note Ch. 18's cookbook
  derives budgets from the full 50,000-cycle frame, so nothing downstream
  depends on the wrong number.
- **12-3 `[TYPO]`** — Lab 12: "The self-test **dogfriends** the library" →
  "dogfoods".

### Chapter 13 — Graphics I *(reviewed clean)*

### Chapter 14 — Text Mode and Multicolor

- **14-1 `[STYLE]`** — The Bridge: "the layout we first met in §12.4" — the
  register layout is **§12.3**; §12.4 is the status byte.
- **14-2 `[SUGGEST]`** — Ch. 13's `TXMODE` sets `R1 = >E0` (interrupt-enable
  **on**), while Ch. 14's text setup uses `>D0` and says "bit 5 (interrupt
  enable) is off; we are not using the frame interrupt yet." The asymmetry
  is real but unexplained; one clause in Ch. 13 ("IE set to match the
  console's own Graphics I convention — harmless under `LIMI 0`") would
  disarm the sharp reader who diffs the two setups.

### Chapters 15–18 *(reviewed clean)*

Bitmap addressing math, mask-bit values (`R3=>FF`/`R4=>03`), sprite
SAL/law-of-four semantics, frame/scroll cycle economics, and the §18.6
cookbook all reconcile internally and against `vdp.rs`/the datasheet
model. (The `cycles` bench command referenced by Ex. 14.6/15.4/18.6 exists —
verified in `code/bench/src/main.rs`.)
