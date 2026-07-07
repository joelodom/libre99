# A Software Engineer's Guide to TI-99 GROM Debugging

This is the *architecture-specific* debugging knowledge for the console-GROM
rewrite: how the TI-99/4A actually executes our firmware, where bugs hide in a
machine like this, and the methodologies that exploit its unusual
observability. It is written so that an engineer — or an AI session, including
smaller models — who has never touched a TI-99 can diagnose like someone who
has spent years inside one.

It pairs with two other documents. **[`DEBUGGING.md`](./DEBUGGING.md)** is the
*operational* side: the step-by-step protocol, the instrument list, the probe
inventory, test recipes. **[`RECON.md`](./RECON.md)** is the *facts*: every
verified interface detail this guide leans on. Read this guide once to build
the mental model; return to its spot-diagnosis table (§5) at the start of
every bug.

---

## 1. The machine you are actually debugging

The TI-99/4A runs **three nested execution engines**, and the first diagnostic
act on any bug is deciding which one you're in:

1. **The TMS9900 CPU** executes the console ROM (`994aROM.Bin`) — 8 KiB of
   machine code containing the **GPL interpreter**, the keyboard scanner
   (`KSCAN`), and the **VBLANK interrupt service routine**. We keep this ROM
   unmodified; it is essentially never the bug.
2. **The GPL interpreter** executes our GROM's bytecode — the operating
   system, i.e. *all the code we wrote* (`grom/console.gpl`). This is where
   the bug almost always is.
3. **The VBLANK ISR** is a concurrent actor: machine code that runs once per
   frame *between GPL instructions*, mutating shared scratchpad state — it
   drives sound lists, sprite auto-motion, cursor timers, the random byte,
   and QUIT.

The consequence that matters: **a bug's symptom layer is usually not its
cause layer**. "No sound" is a PSG-chip symptom, owned by the ISR (layer 3),
which was dead because a GPL boot instruction was missing (layer 2). If you
hypothesize at the symptom layer you will chase ghosts; walk down to the owner
first.

### State ownership — who writes what

Before forming any hypothesis, name the owner of the misbehaving state. If our
GPL is not the owner, the bug is in what we *feed* the owner (a table, a list,
a cell), not in the owner itself.

| State | Owner (writer) | Our GPL's role |
|---|---|---|
| VDP registers | GPL `MOVE …,#n`; the ISR re-asserts R1 from the `>83D4` image | program them at boot; keep `>83D4` in sync with R1 |
| VRAM (screen, font, colors) | GPL `MOVE`s via ROM helpers | everything drawn is ours |
| PSG (sound chip) | **the ISR only**, walking a sound list | point `>83CC/D` at a list and set `>83CE`; never touch the PSG directly |
| `>8375` (current key) | `KSCAN`, invoked by the GPL `SCAN` opcode | scan-loop discipline; copy the key out immediately |
| Keyboard decode table | *we* supply it (GROM `>1700`); KSCAN reads it | wrong/missing table = every key decodes wrong/`>00` |
| SYS scratchpad cells | interpreter / ISR / KSCAN | initialise per the contract (RECON scratchpad map); never repurpose |
| GROM address counter (= GPL PC) | the interpreter; the ISR saves/restores around excursions | our branches |
| 9901 / CRU (interrupt masks) | GPL `IO`; ROM `SBO/SBZ` | the boot **must** enable CRU bit 2 (RECON §11) |
| Cartridge space `>6000–7FFF` | the cartridge | read-only for us — **a write flips banks** on banked carts |

## 2. Total observability — and how it inverts the economics of debugging

Four properties make this system *radically* more debuggable than most
targets. Internalize them; every methodology in §4 is built on one of them.

1. **The entire GPL machine state is tiny and fully readable.** A GPL
   program's state is: 256 bytes of scratchpad (`>8300–>83FF`), VRAM, the GROM
   address counter, and a single condition bit. `bus().peek()` reads any of it
   without side effects. You can snapshot *the whole machine* every frame and
   diff it. There is no hidden state to guess about.
2. **The GROM fetch log is a perfect instruction trace.** The GPL PC *is* the
   GROM chip's address counter, so `grom_record(true)` + `grom_log()` yields
   the complete, exact execution history — every instruction, every branch,
   every data fetch — for free. Most platforms would kill for this.
3. **The emulator is deterministic.** Same GROM + same scripted inputs → the
   identical run, every time. There are no flaky reproductions. If a bug
   happened once, it happens every time; if it stops happening, your last
   change is why.
4. **Experiments cost seconds.** `build_console_grom()` assembles the whole OS
   in-memory; boot + N frames runs in well under a second. Edit → run →
   observe is a seconds-scale loop.

The consequence — and this is the single most important piece of advice in
this guide — is that **an experiment is cheaper and more trustworthy than a
chain of reasoning**. On most systems you reason carefully because experiments
are expensive; here it is the reverse. When two explanations are plausible,
don't adjudicate by argument: run the probe that separates them. When the
correct encoding/recipe/value is unknown, don't derive it: **sweep all the
candidates** (that is how the CRU `IO` recipe was found — over a dozen
candidate list layouts executed in minutes, revealing a requirement no
document stated: the data byte must be non-zero). Smaller models especially:
you do not need to be clever here, you need to be *systematic*. Enumerate,
execute, observe, repeat.

One caveat: the ground truth is the *interpreter executing bytes*, never our
own tooling's opinion of the bytes. The disassembler has known approximations
(it renders `IO`'s function code as a memory operand). When a decode and an
execution disagree, the execution is right.

## 3. The boot-stage ladder

Every "the console is broken from startup" bug is located by finding the
**first boot stage that did not complete**. Each stage leaves cheap,
observable proof. Walk the ladder from the top; the first missing proof names
the guilty code block. (Stage order follows `console.gpl`'s `START`; read the
source alongside.)

| # | Stage | Proof it completed (one probe line each) |
|---|---|---|
| 0 | ROM self-test → GPL entry | `grom_log` shows a fetch at `>0020` (after the `>1FFF` dummy read) |
| 1 | VDP registers programmed | regs = `VREGS` table (`00 A0 F0 0E F9 86 F8 F7` — R1 `>A0`, display still off) |
| 2 | Font loaded | `vram(0x0800 + 8*0x41) != 0` (glyph 'A' has rows) |
| 3 | Color table loaded | `vram(0x0380)` = our entry |
| 4 | Title text drawn | name-table rows read back as our strings (recipe in DEBUGGING.md) |
| 5 | ISR cells initialised | `>837A=0, >83C2=0, >83C4/5=0, >83CC–CF=0, >83D4=>E0` |
| 6 | VDP interrupt enabled | `tms9901.vdp_interrupt_enabled() == true` |
| 7 | Display on | VDP R1 = `>E0` |
| 8 | ISR alive | `>8379` changes across consecutive frames |
| 9 | Key-wait reached | `grom_log` tail cycles through the `TWAIT` SCAN loop |

The same ladder idea applies to any flow: the menu (peek base → window MOVE →
`SCANW` walk → render → `SGET` key → dispatch) and the REPL (`RDK` echo →
ENTER → tokenize → `EVAL` → print) each have their own sequence of provable
stages. When debugging those, first write down their ladder from the source,
then find the first missing proof.

## 4. Core methodologies

### 4.1 Differential execution — the flagship

The same console ROM runs the authentic GROM and ours, in the same emulator.
That means **a reference implementation of the entire OS is always available
in your harness**. Run both through the same scenario and find the *first*
divergence. Variants, cheapest first:

- **Health-panel diff** — the six-signal table in DEBUGGING.md, per frame.
- **Wholesale scratchpad diff** — snapshot all 256 bytes `>8300–>83FF` each
  frame under both GROMs; report the first frame + cell where the *pattern*
  of behaviour diverges; decode the cell's meaning with the RECON scratchpad
  map. This is completely mechanical — no insight required to run it — and it
  has extremely high yield. It is the recommended first move for any
  "something is subtly wrong" bug.
- **VRAM row diff** — read name-table rows back as ASCII under both.
- **Branch-trace diff** — from `grom_log`, extract just the branch targets
  (fetch-address discontinuities); compare the two *sequences of decisions*.
  The first divergent decision is the first wrong compare/branch — then ask
  what cell that compare read, and who owns it.
- **CRU write-log / PSG register diff** — for interrupt and sound bugs.

**The one caveat:** our GROM is *intentionally different* (different text,
different menu code, different addresses, different timing). Diff
**invariants and event orderings, not raw values**. Under authentic, the
sound-list pointer *advances* — so ours must advance, not equal authentic's
addresses. Under authentic, key beep starts *then* the menu waits — ours may
legitimately skip both. Know which differences are design before calling one
a bug (STATUS.md and LIMITATIONS.md list the deliberate ones).

### 4.2 Last-fetch forensics — every hang is a readable loop

Because the fetch log is an instruction trace and the machine is
deterministic, a "hang" is never mysterious: it is a **spin loop whose
addresses are sitting in the tail of `grom_log`**. Take the tail, find the
repeating address cycle, decode it with `decode_at`, and read what it tests:

- Cycle contains `SCAN` + `CEQ @>8375,>FF` → it's a **key-wait** (`TWAIT`,
  `SGET`, `RDK`). Not a hang — feed it a key, or ask why the expected key
  never decoded (keymap? debounce? see §5).
- Cycle tests `@>83CE` → **sound-wait**. If `>83CE` never falls to 0, either
  the ISR is dead (health panel #1) or the sound list never terminates.
- Cycle tests a cell that *nothing in the trace writes* → the bug is the
  missing **writer**, not the loop. Consult the ownership table: who was
  supposed to write that cell, and why didn't it run?
- Single-instruction cycle (`B $` on itself) → a deliberate stop/stub. Ask
  which error path routed execution there.
- **No cycle — the log just stops** → GPL execution left the rails entirely:
  the interpreter is stuck in ROM (check `m.cpu().pc()`), or a dispatch
  transferred to a cartridge (fetches ≥ `>6000` — maybe that was correct!).

And remember **slow is not hung**: a GROM→VDP `MOVE` rewrites the GROM
address per byte (~250 CPU cycles/byte), so a 512-byte window copy is a
notable fraction of a frame, and the menu's per-cartridge scan takes 1–2
seconds of emulated time. Check the frame budgets (§7) before declaring
death.

### 4.3 Breadcrumb stores — GPL printf

GPL has no debugger, but every scratchpad cell is a **free output channel**
readable from the probe. Instrument `console.gpl` directly:

```
        ST   @>835E,>01     ; breadcrumb: reached stage 1   (2 bytes of GPL)
        ...
        ST   @>835E,>02     ; breadcrumb: took the ML-dispatch arm
```

then `m.bus().peek(0x835E)` per frame in the probe. Variants: **counters**
(`INC @cell` per loop pass — how many iterations really happened?),
**transient mirrors** (`ST @cell,@>8375` — capture a value the ISR will
overwrite before your probe can see it), **high-water marks**. Pick spare
cells by checking the cell-map comments in `console.gpl` first, and remove
the breadcrumbs after (or promote one to a permanent "boot progress" cell if
it keeps earning its place). Two bytes of GPL per probe point converts
invisible control flow into observable state — use this aggressively; it is
the cheapest localization tool in the box.

### 4.4 Stub-and-run bisection

Assembly is in-memory and instant, so **bisect by mutation**. Insert an
infinite loop (`HERE B HERE`) after successive stages of the suspect flow and
re-run: the observable goes bad exactly when the guilty block is allowed to
run. Binary-search over the boot = a handful of seconds-long experiments.
The complement: stub a suspect block *out* and see if the symptom vanishes.
For deep flows, build shortcut probes that skip the slow parts —
`tipython_probe` dispatches straight into the REPL without the menu scan;
write the analogous shortcut for whatever you're bisecting.

### 4.5 Minimal-repro mini-GROMs and candidate sweeps

When a *mechanism* is in doubt — an opcode's semantics, an operand encoding,
a list format the ROM consumes — do not debug it inside the full OS. Write a
10–30 line GROM that exercises **only that mechanism** and assert directly on
chip state (`boot_trivial.rs` and every `*_probe.rs` follow this shape). If
the correct form is unknown, **sweep the candidate space in a loop**:
assemble, boot, check, next. `cru_experiment.rs` is the canonical example —
and its lesson generalizes: the sweep surfaced a requirement (`IO` data byte
must be non-zero) that no amount of reading would have predicted. Sweeps beat
derivation in this environment; embrace them.

### 4.6 Invariant tripwires — for corruption and heisen-bugs

For "something corrupts something, sometime" bugs, don't hunt the cause —
**assert invariants every frame and let the first violation localize it**.
Good invariants for this firmware:

- `>8373` (sub-stack pointer) stays within `>7E..>BE` — beyond it, CALL
  nesting is running into the ISR area at `>83C0`;
- `>83D4` always equals the R1 value we programmed;
- ISR cells we zeroed (`>83C2`, `>83C4/5`) stay zero unless we set them;
- VDP R2–R6 never change after boot (nothing legitimate rewrites them);
- cells outside the documented cell maps (in `console.gpl` comments) hold
  their initial values.

First violating frame → turn on `grom_record` around it → the fetch trace
names the guilty instruction. This converts a needle-in-haystack bug into two
runs.

### 4.7 Reading the authentic firmware for behaviour — legally

When our behaviour must match the authentic console and RECON doesn't yet
record how it works: trigger the behaviour **under the authentic GROM** with
`grom_record(true)`. The fetch addresses show *where* the responsible GPL
lives; `decode_at` around those addresses tells you the **interface** — which
cells it reads and writes, in what order, with what values. Extract the
interface fact, prove it with a mini-GROM (§4.5), record it in `RECON.md`
with the probe that proves it. **Never transcribe the decoded GPL into our
source** — we take facts ("the list has a count byte at `+4`"), never
expression. This is exactly how the boot's `IO`/CRU chain was recovered
(decode over authentic `>0080–00B0`, then an empirical sweep).

### 4.8 User-session emulation — be the player

Most of §4 diffs *state*; this diffs *experience*. When a bug is reported as
"I did X, then the screen did Y" — a multi-screen flow, a cartridge
misbehaving several screens into itself, a menu that corrupts three keystrokes
deep — the fastest route to a repro is to **drive the real keystrokes and read
the screen back**, exactly as a human at the keyboard would:

- **Script the actual path.** Boot, leave the title, launch the cartridge, wait
  out the intro, press the keys the user pressed. Drive time with `run_frame()`
  only; hold keys ≥ 2 frames then release (§traps in DEBUGGING.md).
- **Read the name table back as ASCII every step** (the DEBUGGING.md
  screen-text recipe), and `m.render()` a PPM when you need pixels. That text
  *is* what the user sees; rendering non-printable codes as `.` makes
  corruption jump off the page.
- **Determinism means no "couldn't reproduce."** By §2, a scripted run is
  bit-for-bit the user's hardware run. If the user saw it, your probe sees it —
  every time.
- **Run the same script under both GROMs** and watch for the first step whose
  screen visibly differs. That frame is your entry point into differential
  trace (§4.1): you have narrowed a vague "it goes bonkers" to a single
  transition before writing one line of lower-level analysis.

This is the natural front end to §4.1 for any "a real user hit it" bug — it
turns a fuzzy narrative into a frame number and a screen you can see.
`tod_load_probe.rs` is the worked example: it walks Tunnels of Doom to its
"LOAD DATA FROM" screen under both GROMs, dumps each screen, and — once the
screens diverge — diffs the *cartridge's own* fetch stream (identical under
both images) to pin the exact console call that breaks the load. Copy it for
the next "do X and watch it break" report.

## 5. Spot diagnosis — symptom → ranked suspects

Start every bug here. Suspects are ordered by prior probability, informed by
what has actually bitten this project.

| Symptom | Check first | Usual culprits, in order |
|---|---|---|
| **Console "feels dead"**: no sound, no sprite motion, no cursor blink, QUIT ignored — any or all | `>8379` static across frames? | ISR not running: 9901 CRU bit 2 never enabled (the `IO` block in `START`), ISR cells uninitialised. The single most likely root cause in this project's history. DEBUGGING.md case study 1. |
| **Black/blank screen** | VDP R1 (`>E0`?); then ladder stages 1–4 | display-enable never raised (`DISPON` MOVE); VREGS MOVE mis-counted; name/pattern base wrong; font never loaded |
| **Text renders as blanks/boxes, layout right** | pattern table bytes at `>0800+8·code` | font MOVE count/destination; characters outside `>20–>5F` — the base set at `>1000` covers `>20–>5F` only; the lower-case small-caps set (`>60..>7E`) ships at GROM `>0874` and is loaded on request via GPLLNK `>004A` (LDLSET, 2026-07-06), so blanks there mean the program never called the loader |
| **Screen garbage / colored noise** | *which* VRAM region got clobbered | a `MOVE` with wrong destination (`@` vs `V@` confusion) or a count word read from the wrong cell; identify the smear's start address and match it to a MOVE |
| **Keys do nothing** | `>8375` after a held key: `>00` or `>FF`? | `>00` = KEYTAB missing/misplaced at GROM `>1700` (RECON §9) — check *which* block: the FCTN arrows live at `>1765` and the in-game joystick table at `>17C8`, both easy to omit past the shifted block; `>FF` = scan-loop discipline (re-scan before read — debounce, RECON §1), key not held ≥2 frames, or keyboard mode `>8374` ≠ 0. If menus scan but **gameplay** doesn't, read `>8374`: a cart in key-unit 1/2 reads the `>17C8` table, not the ASCII blocks |
| **Joystick dead, keyboard fine** | in key-unit 1/2, is `JOYX/JOYY` (`>8377/>8376`) moving? | `KEY` (`>8375`) alive but `JOYX/JOYY` stuck at `>00` = the **deflection table at `>16EA`** (11 `(Y,X)` pairs) is missing — it's the joystick's own path, separate from the arrows' `>17C8`→`KEY` path. First histogram the 9901 column to confirm the ROM reaches the joystick column (6/7) at all |
| **Wrong character for a key** | which table: unshifted `>1705` / shifted `>1735` | `keymap.rs` content (offset↔key mapping), shift table selection |
| **Menu missing an entry** | does the cart list under the authentic GROM? | far program list beyond the 512-byte window (LIMITATIONS L2); ROM-only cart and the CPU `>6000` scan path; multi-program `next`-chain walk; window pointer arithmetic (`ADJ` = base−window) |
| **Menu entry launches wrong/crashes** | `KIND` cell; the entry address bytes | GPL vs ML dispatch confusion; pre-launch cleanup skipped (RECON §5); entry address byte order; **any write into `>6000–7FFF`** (flips banks on 21 carts) |
| **Cart launches but misbehaves in-game** | authentic GROM same scenario | scratchpad left dirty vs. the cleanup contract; interconnect-table reads (LIMITATIONS L6); ISR cells |
| **Hang** | `grom_log` tail | §4.2 — read the loop; it's a key-wait, a sound-wait, or a missing writer |
| **TI PYTHON wrong numbers** | reproduce the arithmetic in a mini-GROM | `DMUL` result lands in dst *and* dst+2 (low word at dst+2 — clobbers!); `DDIV` quotient→dst, remainder→dst+2, **unsigned** (sign fix-up is manual); `INC` vs `DINC` on word cells; `-32768` print path |
| **Sporadic / rare corruption** | §4.6 invariant tripwires | byte op on a word cell; scratchpad cell collision (check *both* cell maps in `console.gpl`, and remember `>8300–>8304` is triple-duty: XML `>F0` vector, `IO` CRU list, REPL arithmetic scratch); sub-stack overrun into `>83C0+`; count-from-memory MOVE reads a **word** |
| **Sound wrong (vs. absent)** | is `>83CC` advancing? | advancing but sounds wrong = the list's *content* (frequency/attenuation bytes); frozen = ISR (row 1); never set = the program's own code path |
| **QUIT reboots to a broken title** | second boot's ladder | `START` not idempotent — warm reboot (`BLWP @>0000` from the ISR) re-enters `>0020` with *dirty* scratchpad; every cell we rely on must be re-initialised, not assumed zero |
| **Reset (F5) / reboot shows the previous program's graphics** | is the name table / sprite table cleared by the boot? | clean on a *cold* boot but dirty on reset = `START` repaints only its own title cells, not the whole screen. `reset()` is CPU-only (VRAM persists, as on hardware), so the boot must clear the name table (`ALL >20`) **and** neutralise sprites (`ST V@spriteattr,>D0`). The VRAM sibling of the QUIT row above. DEBUGGING.md case study 6 |
| **Works in the emulator, fails on hardware** (future) | any cell read before written? | the emulator zeroes RAM; real scratchpad powers up random. Init everything (RECON R1 ⚠) |

## 6. A taxonomy of GPL bugs — how this language goes wrong

GPL is a 1970s bytecode with sharp edges. These are the recurring
language-level failure modes; recognizing them turns "impossible" symptoms
into ten-minute fixes.

1. **Byte/word confusion.** `INC/DEC/ST/CEQ` touch one byte; `DINC/DDEC/DST/
   DCEQ` touch two. A byte `INC` on a word pointer corrupts silently at the
   `>FF` boundary — the classic "works for small values" bug.
2. **Condition-bit misuse.** There is exactly **one** status bit. Every
   compare overwrites it; `BR` branches when it is **clear** (reads
   backwards!), `BS` when set; `RTN` clears it, `RTNC` preserves it. Any
   instruction between the compare and the branch that touches the bit breaks
   the logic. Detect with breadcrumbs on both arms (§4.3).
3. **`BR`/`BS` are slot-absolute** (13-bit, same 8 KiB slot). The assembler
   rejects cross-slot targets — if it complains, restructure with `B`;
   don't fight it.
4. **Operand-encoding desync.** If an instruction's operands mis-assemble,
   the interpreter consumes the wrong byte count and *everything after it* is
   nonsense — the symptom appears downstream of the cause. Detect by decoding
   your own assembled bytes (`disasm`) and checking they tile; localize by
   cutting the program down.
5. **Three address spaces, one number.** `@>1000` (CPU), `V@>1000` (VDP),
   `G@>1000` (GROM) are unrelated locations. A missing `V` prefix turns a
   screen write into a scratchpad/bias write. When VRAM or scratchpad is
   clobbered at a "familiar-looking" address, hunt for a prefix bug.
6. **The `+>8300` bias.** Far CPU operands encode biased (`@>6000` → `>DD00`).
   The assembler handles it — but hand-laid `BYTE` encodings must apply it
   manually, and a forgotten bias lands ~`>8300` bytes away from target.
7. **Scratchpad collisions.** Menu, REPL, ISR, interpreter, XML vector, and
   the `IO` list all share 256 bytes. The cell maps in `console.gpl`'s
   comments are the ledger — **update the map before claiming a cell**, and
   when corruption strikes, check the maps for overlap first.
8. **ISR races.** The ISR runs between any two GPL instructions: `>8375` can
   be overwritten, `>8378/>8379` change constantly. Copy transient values out
   the instruction after they appear.
9. **MOVE pitfalls.** Count is in bytes; a count-from-memory reads a **word**
   (zero the high byte first); GROM→VDP is slow (§4.2); `#n` destination
   means VDP *register* n — a stray `#` reprograms the display instead of
   writing memory.
10. **Uninitialized-cell luck.** The emulator's zeroed RAM can make missing
    initialisation *look* fine (hardware is random). Grep any cell you read
    for a preceding write; the F3 init block in `START` exists for exactly
    this reason.

## 7. Frame-time reasoning

GPL life is quantized in frames (one VBLANK ISR pass per frame, ~60/s):

| Event | Budget (frames) |
|---|---|
| ISR provably alive after boot | ~2–3 |
| Title fully drawn | ≤ 60 (tests use 60) |
| Menu built, per present cartridge | ~60–120 (the windowed MOVE scan) |
| Cartridge launch after keypress | ≤ 120 (tests assert this) |
| ToD splash tune audible after launch | ~16 observed |
| QUIT back to title | ≤ 60 |

Rules: drive time **only** with `run_frame()` (bare `step()` starves the ISR —
the canonical sound-wait hang); when a probe sees "nothing yet", quadruple the
frame budget once before concluding anything; in regression tests, use double
the observed-good budget so timing drift doesn't flake.

## 8. The ten questions — a condensed diagnostic card

Ask in order; each is one probe line or one document lookup.

1. Is it a **known limitation**? (`LIMITATIONS.md` — 2 min)
2. Does the **authentic GROM** do it too? (then it's the emulator or the
   expectation — different playbook)
3. Is the **ISR alive**? (`>8379` moving)
4. Is the **9901 armed**? (`vdp_interrupt_enabled()`; `interrupt_line()` vs
   `interrupt_pending()`)
5. Are the **VDP registers** sane? (R1 = `>E0`; R2–R6 per `VREGS`)
6. Which **boot-ladder stage** is the first incomplete? (§3)
7. **Who owns** the bad state? (§1 table — then ask what we feed the owner)
8. What does the **fetch-log tail** say? (§4.2 — hangs are readable)
9. What is the **first diverging scratchpad cell** vs. authentic? (§4.1)
10. Can a **20-line mini-GROM** reproduce the mechanism? (§4.5)

If all ten come back clean and the bug persists, you have an interesting bug:
write up exactly what the ten checks showed in DEBUGGING.md's *Open
investigations* and let the next session stand on it.

---

## 9. Lessons-learned register

Hard-won, transferable lessons. **Append here** when a bug teaches something
general (the narrative goes in a DEBUGGING.md case study; the distilled rule
goes here).

- **"Dead console" is one bug, not four.** Sound, sprite motion, cursor
  blink, and QUIT all die together when the ISR never runs — and the ISR only
  runs if the *GROM boot* enables 9901 CRU bit 2. Check ISR-liveness before
  anything else. *(2026-07, ToD sound bug)*
- **The consumer defines the format — empirically.** The ROM's `IO` CRU-output
  list required a non-zero data byte; nothing documented it; only a candidate
  sweep found it. When the ROM consumes a structure you built, sweep the
  layout space instead of trusting inference. *(2026-07)*
- **Cold-boot zeros are camouflage; F5-from-a-game is the adversary.** The
  emulator zero-fills RAM, so any list field or cell the ROM reads that you
  forgot to write still "works" on every cold boot — until a warm reset (F5,
  CPU-only; RAM survives) hands the ROM a real game's leftovers. The `IO`
  list's data-address byte (`>8305`) hid this way and let `START`'s own arming
  instruction *disarm* the ISR. Write every field of every structure, every
  boot — and test warm resets from real gameplay, not just cold boots.
  *(2026-07, F5 menu bug; case study 9)*
- **"Did not reproduce" describes the harness, not the bug.** A
  state-dependent symptom needs the failing *state* reproduced — longer,
  realistic play, the user's exact flow — before its report can be closed.
  Case study 7 closed the press-2 half as non-reproducing; the state that
  triggers it just hadn't been generated. *(2026-07, case studies 7→9)*
- **Distrust the disassembler at the edges.** It models `IO`'s immediate
  function code as a memory operand. Execution evidence outranks decode
  output. *(2026-07)*
- **`bus().peek()` is blind to `>6000–7FFF`.** Read cartridge space via
  `cart.rom` or `Bus::read_byte`, or you'll "prove" a header absent that is
  present. *(M2)*
- **A stalled trace usually means a starved ISR.** Tests that step the CPU
  without `run_frame()` hang in sound-waits and key-waits that would resolve
  in one frame of real time. *(M2, recon)*
- **The screen is a fine data structure.** The row-as-line-buffer trick (REPL
  input read back from VRAM) and the VDP-window header scan both use VRAM as
  working memory — plentiful and probe-visible. Consider it before inventing
  scratchpad structures. *(M2/M4 design, useful in debugging too)*
- **Reproduce as the user before you theorize.** Driving the real keystroke
  path and reading the screen back as ASCII turns a vague "the display goes
  bonkers" into a specific frame and a specific call — a screen you can *see*
  beats a cell you infer. Script it under both GROMs; the first
  visibly-different frame is the entry to differential trace (§4.8). *(2026-07,
  ToD cassette-load bug; `tod_load_probe.rs`)*
- **"Text doesn't draw" with a right layout is a font bug — read pixels, not
  codes.** The name-table ASCII dump is blind to it: identical codes render
  identically as text whether or not their glyphs exist. A rendered
  non-backdrop pixel count and a per-code "is this glyph non-blank?" check
  expose it in one run. And a GROM cart's text is often drawn by **console**
  char-set loaders it reaches through the interconnect table (`>0016`/`>0018`),
  not by the cart itself — so an unimplemented console utility surfaces as
  *missing glyphs*, far downstream of the `RTN` stub that caused it. Sprites
  keep showing because the cart defines those itself. *(2026-07, TI Invaders
  text bug; `invaders_probe.rs`)*
- **The interconnect table is not "tolerate the zeros" — some slots are load-
  bearing.** DSRLNK (`>0010`) and the two char-set loaders (`>0016`/`>0018`)
  are hard dependencies for the carts that call them; a slot stubbed to `RTN`
  fails *silently and downstream*. When a cart misbehaves after launch, trace
  which interconnect vectors it CALLs and diff what authentic runs there. *(2026-07)*
- **"Some keys work, others are dead" splits by keytab block, and gameplay uses
  a different one.** The ROM's `SCAN` reads *four* ASCII blocks at
  `>1705/>1735/>1765/>1795` (unshifted/shifted/**FCTN**/CTRL) *and* a joystick /
  split-keyboard table at `>17C8`. Number keys → unshifted; arrow keys → FCTN;
  **in-game** arrows/fire → the `>17C8` table, selected when a cart sets key-unit
  1/2 in the mode cell `>8374`. A rewrite that stops after the shifted block
  leaves everything from `>1765` on as `>00`, so the menu (numbers) works while
  gameplay (arrows) is dead. When menus scan but gameplay doesn't, read `>8374`
  first — it names which table the ROM is about to consult. *(2026-07, TI Invaders
  gameplay-keyboard bug; `fctn_probe.rs`, `joystick_scan_check.rs`)*
- **The joystick and the keyboard arrows are two different decode paths.** In
  key-units 1/2 the keyboard arrows go `>17C8` → `KEY` (`>8375`), but the physical
  **joystick** goes through a *second* table — the deflection table at **`>16EA`**
  (11 `(Y,X)` pairs, `+4/0/-4` = `>04/>00/>FC`) — into `JOYY/JOYX` (`>8376/>8377`).
  So "keyboard plays but the joystick is dead" (or the reverse) is a *missing
  table*, not a wiring fault. Confirm the CRU first (histogram the 9901 column —
  the ROM parks on the joystick column when a stick is held), then diff the result
  cells: `KEY` alive but `JOYX/JOYY` zero ⇒ the `>16EA` table is missing. *(2026-07,
  TI Invaders joystick bug; `joy_gromtrace.rs`, `joy_gameplay_probe.rs`)*
- **Drive the game only when it's actually running.** A movement probe proves
  nothing if the game loop is frozen. TI Invaders parks every sprite until **fire**
  launches the wave; scripted probes that never press fire saw zero motion under
  *both* GROMs and looked like "nothing works". Check liveness (a sprite/`>8379`
  actually changing with no input) before trusting a "no response" result. *(2026-07)*
- **`reset()` is CPU-only — the boot owns the screen clear.** A reset (F5) and a
  warm reboot (QUIT) re-enter the boot with VRAM *unchanged*, exactly as on
  hardware; nothing zeroes the name table or the sprite table for you. So the
  boot/title code must repaint the **whole** screen — clear the name table
  (`ALL >20`) *and* neutralise sprites (`ST V@spriteattr,>D0`) — not just write
  its own cells. The tell is a bug that is invisible on a **cold** boot (blank
  VRAM makes the gaps look clean) but appears on **reset from a running cart**
  (its tiles and sprites survive in the gaps). Differential-trace the *reset*
  path, not only the cold boot. The VRAM analogue of the scratchpad-idempotency
  lesson above. *(2026-07, TI Invaders reset-artifacts bug; `reset_artifacts_probe.rs`)*
