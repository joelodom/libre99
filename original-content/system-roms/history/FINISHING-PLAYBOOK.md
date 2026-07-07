> **ARCHIVED (2026-07-02).** Phases A–D were all completed and shipped. Kept for
> provenance; do not work from it. Live docs: [`../STATUS.md`](../STATUS.md),
> [`../RECON.md`](../RECON.md), [`../DEBUGGING.md`](../DEBUGGING.md).

# FINISHING PLAYBOOK — M2 → M6 in one session

The step-by-step guide to complete the GROM rewrite. Written after the
2026-07-02 research session so that **every mechanism used here is already
execution-verified** — see `RECON.md` "VERIFIED MECHANISMS" (§1–§8, referenced
below as V1–V8). No new reverse-engineering should be needed: if something
seems to require it, re-read RECON first; if genuinely new, pin it with a probe
(`crates/libre99-gpl/examples/*_probe.rs` shows the pattern) before writing GPL.

**Rules** (unchanged): TDD; gates are mandatory and never weakened; `cargo test
-p libre99-gpl -p libre99-core` green + clippy clean before each commit; commit at
each milestone; keep `console-grom.bin`, README/help, and these docs in sync.
All GPL in `original-content/system-roms/grom/console.gpl` (font is spliced at
build; see `system_grom.rs`). Never copy TI bytes; the RECON trace excerpts are
behavioral references only.

**Environment**: prepend `$env:USERPROFILE\.cargo\bin` to PATH (PowerShell).
Classic99 source is checked out on both workstations — `C:\ClaudeShared\classic99`
on the PC and `/Users/Shared/classic99` on the Mac (sibling of the repo) — if an
opcode question survives RECON+isa.rs; consult, never copy. Run everything from
the repo root.

---

## Phase A — M2: the selection list

All in `console.gpl`, replacing the `WAIT B WAIT` hold loop after the title.

### A.1 Data layout (our choices; scratchpad map in the review §8)

| Cell(s) | Use |
|---|---|
| `>8310` | key scratch (copy of `>8375`) |
| `>8312` | menu entry count (byte) |
| `>8314` | scan cursor: current screen row (byte) |
| `>8316/7` | word scratch: current GROM base |
| `>8318/9` | word scratch: pointer into the VDP window / list walking |
| `>831A` | entry kind array base… — simpler: see A.2's table-in-VDP note |
| `V@>2000` | 4 KiB cartridge-GROM window (V4) |
| `V@>3800+` | menu table: per entry 4 bytes `{kind, unused, entry-addr word}` (VDP is plentiful; read back with `MOVE …,V@…,@…` or `*V@` pointers) |

### A.2 Scan + render (V4)

1. Title waits for a key (V1). On key: clear screen (`ALL >20`), draw a
   header row (`MOVE` a `TEXT` from GROM, like the title strings).
2. **Console GROM 1 first**: our own TI PYTHON program list lives at `>2000`
   (Phase C adds it) — handle it the same way as cartridges by scanning base
   `>2000` too. Menu numbering therefore starts with `1 FOR TI PYTHON`.
3. **Per GROM base** `>2000, >6000, >8000, >A000, >C000, >E000` (unrolled —
   yes, one code block per base or a small `CASE`-driven loop; immediate GROM
   addresses are required, V7):
   - `MOVE >1000, G@base, V@>2000` (4 KiB window).
   - `CEQ V@>2000,>AA` / `BR next-base`.
   - Program-list pointer = window bytes `+6,+7` → `@>8318` via
     `MOVE >0002, V@>2006, @>8318`. Zero → next base.
   - Walk entries: convert GROM pointer P to window offset `P - base + >2000`
     (`DSUB @>8318,base` / `DADD @>8318,>2000`), then with `*V@>8318`-style
     word pointers read `{next(2), entry(2), len(1), name(len)}`
     (bump the pointer with `DADD @>8318,>0001` between byte reads, or `MOVE`
     the 5-byte entry head to scratch cells in one go — simpler).
   - Render `n FOR NAME`: digit = `'0'+count` (`ST` to the name-table cell),
     `TEXT ' FOR '` from GROM, then the name — VDP→VDP `MOVE` from the window
     (count from memory: `MOVE @len-cell, V@name-in-window, V@screen-cell`,
     the N=0 form, verified — but note count-from-memory reads a **word**;
     store the length byte into the low byte of a zeroed word cell first).
   - Append `{kind=GPL, entry}` to the menu table; `INC @>8312`.
   - Follow `next` until `>0000` (multi-program carts: et=7 entries!).
4. **CPU `>6000` ROM header** (33 ROM-only carts!): `MOVE >0040, @>6000,
   @>83xx`… better: `MOVE >0040, @>6000, V@>3000` (CPU→VDP verified) and parse
   the same way; pointers are plain CPU addresses — window offset = `P - >6000
   + >3000`. Kind=ML. **Never write to `>6000-7FFF`** (bank-switch hazard).
5. If count is 0, show the title's READY line and loop back to key-wait.

### A.3 Select + dispatch (V1, V2, V3, V5)

1. Key-wait (V1). Accept `'1'..='9'`, key−`'0'` ≤ count; else re-loop.
2. Fetch the chosen `{kind, entry}` from the menu table (fixed VDP address =
   `>3800 + 4*(n-1)`: compute in a word cell, read via `*V@`).
3. **Pre-launch cleanup** (V5) — copy the RECON §5 sequence.
4. `kind==ML` → `DST @>8300,entry ; XML >F0` (V2 — entry via `MOVE` into the
   two bytes… NOTE: `DST @>8300,@cell` memory form, since entry is computed).
   `kind==GPL` → `DST @>8380,@cell ; ST @>8373,>80 ; RTN` (V3).

### A.4 Gate (new test `crates/libre99-gpl/tests/menu.rs`)

- `menu_lists_and_launches_grom_cart`: mount `amazing.ctg`, boot our GROM, key
  through title, assert screen shows `2 FOR A-MAZE-ING` (name-table text per
  F14 of the review), press `2`, assert cart-GROM fetches ≥ `>6000` begin
  (grom_record) — entry `>602A`.
- `menu_lists_and_launches_rom_cart`: `centipe.ctg` → `2 FOR CENTIPEDE`,
  press `2`, assert `PC ∈ >6000..>8000` within ~120 frames.
- `menu_walks_multi_program_lists`: `VideoGames1.ctg` lists 3 entries.
- Keep `boots_to_rewrite_title_screen` green (no cart → title, READY line).
- Commit: `feat(system-roms): M2 — selection list lists and launches cartridges`.

Pitfalls: hold keys ≥2 frames and release (F14); drive time with `run_frame()`
only; copy `>8375` out immediately (V1); `bus().peek()` can't see `>6000-7FFF`
(use `cart.rom` in tests); the amazing name contains quotes (`"A-MAZE-ING"`)
— compare exact bytes from the window, don't hand-strip.

## Phase B — M3: the compatibility sweep

1. Table-driven sample test (9 carts): amazing (GROM-only), HuntTheWumpus
   (3 programs), Parsec, TI-Invaders (GROM+ROM), centipe (ROM plain), DigDug,
   MoonPatrol (ROM banked ×2), VideoGames1, et (7 programs, banked). Assert
   listed count == census count and entry 1 (or 2) launches by kind.
2. `#[ignore]`d full sweep over `cartridges/*.ctg`: parse with
   `libre99_core::cartridge`, compute expected entries (GROM `>6000` header +
   ROM bank-0 header program lists; the RECON census logic), boot, compare,
   dispatch, assert launch signal. Emit a pass/fail table with `println!`.
   Triage the 11 headerless images against the **authentic** GROM first; those
   that don't list there either are out of scope (document in STATUS.md).
3. QUIT test: launch a cart, `Fctn`+`Equals`, expect our title back in ≤60
   frames (the ISR reboots through `>0000` → our `>0020` entry).
4. Commit: `feat(system-roms): M3 — cartridge compatibility sweep`.

## Phase C — M4: TI PYTHON v0

GROM 1: header at `>2000` (`AA 02 01 00`, program list → `{0, REPL, 9,
"TI PYTHON"}` — 9 chars). REPL code + data at `>2020+`. Spec: plan §9 +
review F13 (truncating `/` and `%`, 16-bit wrap, `-32768` special-case,
16 vars × 4 bytes at `>8300-833F`, errors `SYNTAX ERROR` / `NAME ERROR` /
`ZERO DIVISION ERROR`).

Implementation notes anchored to verified semantics (V1, V6):

- **Screen-as-line-buffer**: echo accepted chars to the current row's
  name-table cells (`ST *V@cursor,@key` — wait: `ST` dst must be the VDP
  pointer: `ST *V@>83xx,@>8310`, mem-src form). ENTER → tokenize by reading
  the row back (`MOVE >0020, V@row, @>83xx`… row is 32 bytes; copy to
  `>8320-833F`? that's the var table — use a VDP scratch row instead, or
  parse in place via `*V@` pointer reads).
- Key loop per char: V1 idiom + **wait for release** (`SCAN ; CEQ @>8375,>FF ;
  BR release-loop`) before accepting the next key, or keys auto-repeat.
  Backspace = FCTN-S = code `>08`; ENTER = `>0D`.
- Recursive descent with `CALL`/`RTN` — depth-cap with a counter cell
  (`INC`/`CEQ`/`BS error`); the sub-stack has ~64 bytes (`>8380-83BF`...
  the ISR area starts `>83C0`) → cap nesting at ~8.
- Arithmetic on a small value stack in scratchpad (`>8340-834F`, word cells,
  walked with a byte pointer + `*@` — V6): `DADD/DSUB` direct; `MUL` → `DMUL`
  then take dst+2 (low word, V6); `DIV` → sign-record, `ABS` both, `DDIV`
  (quotient dst, remainder dst+2), apply sign; divisor 0 → check first with
  `DCZ`.
- Decimal print: repeated `DDIV` by 10, push remainders (`>8350+`), emit
  digits `'0'+r`; negative → emit `'-'`, negate; `-32768` → emit the literal
  `TEXT '-32768'`.
- Variables: name = first 1–2 chars (pack into a word), linear search 16
  4-byte slots via byte pointer; miss on read → `NAME ERROR`; miss on write →
  claim first empty slot.
- Gate (`tests/ti_python.rs`), driving keys and asserting screen rows: the
  plan §9 session (`2 + 3 * 4`→`14`, `x = 7`, `x * (x - 1)`→`42`, `y`→
  `NAME ERROR`, `10 / 0`→`ZERO DIVISION ERROR`) plus `10/3`→`3`,
  `-10/3`→`-3`, `-10%3`→`-1`, `32767+1`→`-32768`.
- Commit per sub-step if convenient; final:
  `feat(system-roms): M4 — TI PYTHON v0 REPL`.

## Phase D — M5/M6: polish

1. **Menu slot for TI PYTHON on the title screen**: title says
   `PRESS ANY KEY TO BEGIN` → key → menu (already lists TI PYTHON via the
   `>2000` scan).
2. Vector region `>0010-004F`: point every slot at a stub `B >0020` (safe
   reboot-to-title). If the M3 sweep shows carts fetching console-GROM
   addresses post-launch (grom_record histogram: fetches < `>6000` after
   dispatch), implement/document per address (likely empty — review F10).
3. Rebuild + commit `console-grom.bin` (`cargo run -p libre99-gpl --bin libre99gpl --
   console original-content/system-roms/grom/console-grom.bin`).
4. Docs sync: `grom/README.md` (address map, what-works), `STATUS.md`,
   repo `README.md` (a "boot the rewrite" note exists; extend if flags change),
   app help overlay if touched.
5. Optional (ask Joel first): embed the rewrite + preference toggle
   (review §10 Q1).
6. Final commit: `feat(system-roms): M5/M6 — vectors, artifact, docs`.

## If something fails

- Assembler rejects a construct → it's probably banned (RECON §7) — redesign
  with verified forms, don't force it.
- A GPL program misbehaves with no obvious cause → suspect a desynced operand
  encoding: cut the program down, verify each instruction's bytes with
  `libre99_gpl::disasm::linear`, and if needed pin semantics with a 20-line probe
  example on the real ROM (the established pattern — cheap and decisive).
- The interpreter "hangs" in a keyed test → you stopped pumping `run_frame()`
  or held a key forever (see RECON key-wait + sound-wait notes).
