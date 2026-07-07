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

---

## Part IV — Sound and Speech

### Chapter 19 — The Sound Generator

- **19-1 `[BUG]`** — §19.2: "Ten bits of `N` (1 to 1023) gives a range from
  about **3,494 Hz at `N = 1`** down to about 109 Hz at `N = 1023`."
  3,494 Hz is `N = 32` (3,579,545 / (32×32)); at `N = 1` the output is
  ~111,861 Hz — ultrasonic. The *musical* range the sentence means starts
  around `N = 32`. Fix: "about 3,494 Hz at `N = 32` (smaller values climb
  beyond hearing) down to about 109 Hz at `N = 1023`" — the "five octaves"
  conclusion survives unchanged. (The even-address-only write rule in §19.1
  was cross-checked against Classic99 `Tiemul.cpp` — "don't respond on odd
  addresses" — and is correct.)

### Chapter 20 — The Speech Synthesizer

- **20-1 `[BUG]``[INCONSISTENT]`** — The sidecar is placed on the console's
  **left** twice ("plug the Speech Synthesizer sidecar onto the left side,"
  prologue; "clips onto the console's **left** expansion port," §20.2). The
  I/O expansion port is on the console's **right** edge — as Ch. 2's own
  block diagram says ("Side port (right edge of console)").

---

## Part V — Input, Interrupts, and Console Services

### Chapter 21 — Keyboard, Joysticks, and the TMS9901

- **21-1 `[BUG]`** — §21.6: "there is a scratchpad flag the firmware checks,
  which a program **clears** to suppress the reset." Backwards. The ISR
  checks *disable* bits at `>83C2` (verified in the clean-room
  `rom/console.asm`, RECON §6: `>80` all VDP duties / `>40` sprite motion /
  `>20` sound / `>10` QUIT) — a program **sets** `>10` to suppress QUIT.
  Cross-check App. C's row for the same wording while fixing.
- **21-2 `[VERIFY]`** — §21.4 says Alpha Lock shares a line with
  "**joystick 1's** up direction." The community record generally has the
  latched Alpha Lock interfering with UP on **both** joysticks (the lock's
  select line back-feeds the shared row). Verify against Classic99's
  keyboard wiring or Nouspikel before print; if both, s/joystick 1's/the
  joysticks'/.

### Chapters 22–23 *(reviewed clean)*

`>8379` frame counter, `>83C4` hook, LOAD/RESET model, FAC/ARG addresses,
radix-100 description, and the E/A-environment honesty all check out
against the firmware source and App. C/K.

### Chapter 24 — The Scratchpad Atlas

- **24-1 `[INCONSISTENT]`** — The R10 stack top. Ch. 9's text and code
  canonize `STKTOP = >8380` ("R10 balanced to >8380"); the companion code
  then drifts — `>83FE` (ch10 `keyscan.a99` — *inside the `>83E0` GPL
  workspace this very chapter calls sacred*), `>8370` (ch11–12), `>8340`
  (ch13 onward, the settled value) — and Ch. 24 §24.4 declares ">8340 in
  most of this book's programs" as if it had always been the convention.
  Fix options: (a) add one sentence to Ch. 9 ("the top is per-program until
  the libraries settle on `>8340` in Ch. 13") and a footnote in §24.4
  acknowledging the early files; or (b) retrofit ch09–ch12 code + prose to
  `>8340`. Either way, reconsider ch10's `>83FE` — harmless bare-metal, but
  it models the opposite of the atlas's advice and will confuse a reader
  who diffs code against Ch. 24.

---

## Part VI — GROM, GPL, and the Operating System

### Chapter 25 — GROM *(reviewed clean — and note it explains the
address-readback/prefetch correctly; fixing 5-2 should point Ch. 5 at this
chapter's wording)*

### Chapter 26 — The GPL Language

- **26-1 `[BUG]`** — §26.5: "The core branches are **B** (unconditional
  branch to a GROM address), **BR** (a short relative branch), and
  **BS**/**BR** in their condition-testing forms." Muddled: GPL's `BR` is
  not a relative branch, and `BR`/`BS` *are* the conditional pair — there
  are no separate "condition-testing forms." Correct statement (per the
  ISA in `libre99-gpl/src/isa.rs` and App. B): `B` is the long
  unconditional branch; **`BR` (branch on status reset) and `BS` (branch
  on status set)** test — and consume — the COND bit of `>837C`, taking a
  13-bit within-slot GROM address.
- **26-2 `[BUG]`** — §26.5: "`CALL` pushes the GPL program counter onto the
  GPL **data stack**." GPL keeps two stacks — the data stack (pointer at
  `>8372`) and the **subroutine stack** (pointer at `>8373`); `CALL`/`RTN`
  use the subroutine stack. One-word fix, but readers cross-referencing
  App. C will notice.
- **26-3 `[VERIFY]`** — Lab 26 echoes the round-trip as "`ALL >20 / ST
  V@>0000, >48 / MOVE >0005, V@>0002, …`" — the `MOVE` operand order shown
  (count, *dest*, …) contradicts §26.7's own `MOVE >0005, G@MSG, V@>0002`
  (count, src, dest). Check the actual `dis` output and make the two
  agree.

### Chapter 27 — Writing GPL Today

- **27-1 `[STALE]`** — §27.2/Lab (and Ch. 28 §28.2/§28.7/Lab/Summary): the
  verified transcript "the booted **clean-room** console menu … lists
  `1 FOR TI BASIC`." At HEAD the clean-room GROM's program header is
  **`TI PYTHON`** (`console.gpl` line ~752; GROM 1), plus the
  system-information entry — TI BASIC appears only when booting authentic
  firmware. Update the transcripts and prose to the machine actually
  booted (the clean-room + TI PYTHON example is now the *better* story:
  the same `>AA` scan discovering an original program). The title-screen
  strings ("TEXAS INSTRUMENTS / HOME COMPUTER / READY-PRESS ANY KEY…")
  are still accurate at HEAD — only the menu-entry name changed.

### Chapter 28 — The OS in GROM

- **28-1 `[STALE]`** — Same as 27-1: the "`PRESS / 1 FOR TI BASIC`"
  transcript and the §28.7/summary claims that the clean-room menu lists
  TI BASIC. Fix together with 27-1.

### Chapter 29 — Hybrid Architecture

- **29-1 `[STYLE]`** — Further Reading: "Chapter 36 (Extended BASIC and
  the User)" — Ch. 36's title is *Program Architecture in 16–48K* (the
  `CALL LINK` page is §36.8 inside it). Fix the label.

---

## Part VII — Storage and Peripherals

### Chapters 30–33 *(reviewed clean, with one wording nit)*

- **30-1 `[SUGGEST]`** — §30.5: "the console's ISR runs on `/INT2` (the VDP's
  60 Hz), but the same interrupt line is shared by expansion cards." Loose:
  at the 9901's pins the VDP arrives on INT2 and expansion cards on INT1
  (EXTINT); what is shared is the CPU's single **level-1** line both funnel
  into (which the ISR's source check distinguishes — the firmware's `ISRVDP`
  test). One clarifying clause avoids teaching that cards share /INT2.
- The PAB error-code semantics in Ch. 31 (error 2 for a missing file on
  OPEN) were cross-checked against App. H's tier-1 matrix — consistent.

### Chapter 34 — Modern Peripherals

- **34-1 `[BUG]`** — §34.6 and Further Reading both say the consolidated
  capability-detection recipes live in "**App. I**" — App. I is *Media &
  File Formats* (and its outline spec contains no detection material).
  No appendix currently owns these recipes. Either add a short section to
  App. L (toolchain quick reference) or App. G, and point there, or cite
  §34.6 itself as the consolidation. (If App. I's finishing session wants
  to absorb them, update its stub spec first.)

---

## Part IX — Case Studies

Chapters 39, 40, 41, and 43 reviewed clean — and unusually deeply verifiable:
the printed traces, bit-packings (the FATE 5-bit trace decodes exactly), maze
algebra, and cycle budgets all reconcile. Two findings:

- **42-1 `[BUG]`** — Ch. 42 §42.3: "page the rest to **VRAM (24 kilobytes of
  it**, addressable a byte at a time through the port)." VRAM is **16 KB**
  total (roughly 15 KB free beyond text mode's tables). "24 KB" belongs to
  the *high expansion RAM*; as written it misstates the machine. Fix the
  number (and pick which far store the sentence means — VRAM through the
  port is the one the parenthetical describes).

## Part X — Beyond the Console

Chapters 44–45 reviewed clean (Geneve/F18A/99000 facts check against the
community record and are properly hedged).

- **45-1 `[STYLE]`** — §45.4: "the **byte-high law**" — the book's canon
  handle (R-8, App. M) is "the **high-byte law**."

---

## Appendices (drafted)

### Appendix B — GPL Reference *(reviewed clean)*
B.4.3's `BR`/`BS` rows are the correct semantics to align Ch. 26's fix
(26-1) against.

### Appendix C — Memory Maps & Scratchpad Atlas

- **C-1 `[INCONSISTENT]`** — Shares erratum 24-1 (the stack top): C.3/C.5
  print `>8340` as *the* stack top "(Ch. 9)" though Ch. 9 canonized `>8380`
  and the code drifted through four values. Additionally C.5 is internally
  contradictory: it assigns `>8320`–`>833F` to "hot variables" *and* has the
  stack growing **down from `>8340`** — i.e., into that same range. Resolve
  with 24-1: state the settled layout precisely (e.g., "the stack grows down
  from `>8340` into the *upper* end of `>8320`–`>833F`; keep hot variables
  at the low end, and keep the stack shallow — R-16 pushes are transient"),
  or renumber.

### Appendix D — TMS9918A Reference

- **D-1 `[BUG]`** — D.7 (the timing cookbook): "the whole **~4,000-cycle**
  blanking interval is yours." Same error as 12-2, propagated: the vertical
  blank is ~70 of 262 lines ≈ **13,400 CPU cycles** (≈ 4.5 ms at 3 MHz).
  Fix both places together; the recurring "4,x00" reads like a
  milliseconds-value mislabeled as cycles.

### Appendix E — Sound Reference *(reviewed clean)*
E.3 already prints the correct `N = 1` ≈ 111,861 Hz row — fix 19-1 to match
it. One nit: E.3 sets its formula in `$$…$$` LaTeX, the manuscript's only
use; the house style is code-block/ASCII math (R-5).

### Appendix G — CRU Map

- **G-a `[SUGGEST]`** — G.4: "The project emulator runs a **genuine** disk
  DSR" — ambiguous at HEAD, where the *default* is the clean-room DSR and
  `--disk-dsr` installs an authentic image. Reword ("a complete disk DSR —
  the clean-room rewrite by default"). G.3's Alpha-Lock note shares
  erratum 21-2 (joystick 1 vs. both joysticks — verify and fix together).

### Appendix H — DSR & PAB Reference *(reviewed clean; Ch. 31's error-code
usage checked against H.6 and consistent)*

### Appendix J — Character Sets & Key Codes *(reviewed clean)*

### Appendix K — Console Entry Points

- **K-1 `[VERIFY]`** — K.4/K.5's XTAB row names `XML >1A` = `SGROM` ("search
  GROM headers — the device/subprogram scan"), while Ch. 29 §29.2 glosses
  `XML >19` as "find the next card with a power-up routine" and `>1A` as
  "**call** it." One of the two mislabels which code searches and which
  calls. Reconcile both texts against `rom/console.asm`'s XTAB and
  `console.gpl`'s power-up walk, and use one set of glosses in both places.

### Appendix L — Toolchain Quick Reference *(reviewed clean — fully current
with HEAD; use it as the model for the Ch. 3 G-1 rewrite)*
One addition folded into G-2: L.3 should say where BENCH99's `boot` gets its
firmware (today: authentic images from the maintainer-local `third-party/`,
absent from a public clone).

### Appendix M — Glossary

- **M-1 `[BUG]`** — The **prefetch** entry: "…so the first read after
  setting an address is **stale**." Backwards: the prefetch is what makes
  the first *data* read correct; what is off-by-one is the *address
  counter read-back* (Ch. 25 §25.2 states it correctly). Rewrite the
  definition to match Ch. 25.
- **M-2 `[STYLE]`** — **CQ-82**'s origin pointer reads "(Ch. 36; Part IX)";
  the checklist is coined in **Ch. 2 §2.6** (Part IX enforces it). Also:
  the **high-byte law** entry points at "(Ch. 4, Ch. 8)" — Ch. 7 is the
  chapter that names it; and the **TMS5220** entry could note the console
  sidecar shipped the TMS5200 sibling (as Ch. 20 does).

### Appendix N — Bibliography *(reviewed clean)*

---

## Stubs — detailed guidance for the finishing sessions

### Appendix A — TMS9900 Instruction Reference `[STUB]`

The natural next appendix; opcodes/encodings/status are machine-verifiable
here (libre99asm round-trips + `cpu.rs` + Classic99's `WStatusLookup`); the
cycle-formula column needs the TMS9900 datasheet (Mac). Guidance from this
review pass:

- **A-s1** — App. A is the declared audit target for premises the body
  already printed. Reconcile these while drafting: Ex. 4.7's premises
  (erratum 4-1 — `MOVB Rs,@addr` = C 22 / M 5); Ch. 7 §7.1's mode-cost
  table (the "extra accesses" column counts only workspace-external
  accesses — footnote it or normalize); Ch. 8's shift model (12 + 2n, +8
  for the R0-count path) and `MPY` 52 / `DIV` 92–124 hardware ranges
  (Ch. 37 cites them); jumps 10 taken / 8 not.
- **A-s2** — Carry the emulator-deviation rows prominently: `MOV`/`MOVB`
  destination pre-read unmodeled (re-verified at HEAD in `cpu.rs`);
  `MPY`/`DIV` flat-cost timing (Ch. 8/37); DIV's data-dependent range is
  datasheet-only.
- **A-s3** — G-4 (the bit-numbering declaration) wants its permanent home
  in App. A's front matter.

### Appendix F — Speech Reference `[STUB]`

Correctly deferred: the project models no synthesizer, and no
TMS5200/5220 datasheet is on this PC — drafting now would violate R-21/R-15
(fabricated tables). When a session has the datasheet (or the project grows
speech): align with Ch. 20's asserted facts (TMS5200 in the sidecar, cousin
of the 5220; `>9000` read / `>9400` write; ~16-byte FIFO; buffer-low/empty
status bits; Speak External; energy + pitch + K1–K10, ~25 ms frames,
~1,200 bits/s; resident vocabulary "a few hundred" entries) and fix App. M's
TMS5220 entry in the same pass. Ch. 20's `SPKDET` "read-signature protocol"
should be documented as F's detection section.

### Appendix I — Media & File Formats `[STUB]`

Half is tier-1-ready **now**: the disk half (VIB/FDR/FDIR/cluster chains,
`.dsk` sector-dump geometry and v9t9/PC99 variants) can be drafted from
`disk.rs` + `RECON.md` §5 + Ch. 32's live `Tunnels.Dsk` decode (reuse that
worked example). The tagged-object/EA5/cassette half waits on Ch. 6 (Mac).
Two coordination items: resolve erratum 34-1 (Ch. 34 points "consolidated
detection recipes" at App. I, which its spec doesn't cover — either absorb
them deliberately or fix Ch. 34's pointer); and Ch. 6's §6.4/6.5 dissections
should feed I's tagged-object/EA5 sections from the same verified bytes.

### Chapter 6 `[STUB]` — see the Part II section (6-1…6-4).

### Front matter *(not yet begun)*

When `00a-preface.md`/`00b-how-to-use-this-book.md` are written: include
the three-reading-tracks statement (outline §1.5), and run the final-pass
forward-reference reconciliation the outline's review-pass ① and ④ call
for — this ERRATA's per-chapter entries are the input to that pass.
