# Disk DSR — the interface dossier (D1)

Empirically-pinned facts about the authentic TI Disk Controller DSR
(`roms/Disk.Bin`), gathered by the **probe rig**
(`crates/libre99-gpl/tests/disk_dsr.rs` `probe_*`, on the shared rig in
`tests/dsr_common/mod.rs`) running scripted PAB / subprogram operations
against the genuine DSR under this emulator, plus ground truth read straight
from the TI-formatted `disks/Tunnels.Dsk`. **This is the spec our clean-room
`disk-dsr.asm` is written from** — behaviors, never TI code.

**Method note (a deliberate deviation from the console-ROM D1).** The console
ROM's dossier was disassembly-first because its contract is interior
addresses. A DSR's contract is *behavior at the header/PAB/scratchpad/disk
surfaces* — all of it drivable through the rig — so this dossier is
**probe-first**: every fact tagged with the probe that pinned it, re-runnable
forever (`cargo test -p libre99-gpl --test disk_dsr -- --ignored --nocapture`).
The differential gates then hold our DSR to these behaviors *live* against
the authentic oracle, which is stronger than any transcribed table. Facts
believed but not yet probe-pinned are tagged 📖 (seed research, plan
Appendix A) or ❓.

Evidence tags: ✅ **probe-pinned** (named probe) · 📖 seed research
(consult-never-copy) · ❓ open.

---

## 1. Discovery & calling convention (shared with the console — plan §2.2)

All ✅ (the rig itself exercises them; `probe_rig_smoke_status` end-to-end):

- `>AA` header at `>4000`; chain heads at `>4004` power-up / `>4008` device /
  `>400A` subprogram; node = `[link][entry][len][name…]`.
- Called via `BL *R9` on GPLWS, R12 = `>1100`, `LIMI 0`; **skip-return**
  (R11+2) = handled (success *or* PAB error); plain return = keep searching.
- Device inputs: `>8355` = device-name length, `>834A..` = the staged device
  name, `>8356` = VDP address of the first char *past* the device name in the
  PAB (the '.' or end); PAB base = `[>8356] − devlen − 10`.
- Subprogram inputs: `>836D = >0A`, `>8355 = 1`, `>834A` = the number byte;
  parameters in `>834C..>8351`.
- **Return side-effects** ✅ (`probe_rig_smoke_status`, `probe_write_var`):
  `>8354` ← the PAB base address (word); `>8356` ← `>37E3` — a pointer into
  the DSR's VRAM work area holding a copy of the looked-up file's FDR
  name/fields **with the first name byte zeroed**. (The interior layout of
  that record differs by op under the authentic DSR; ours writes a
  CLOSE-style record — zeroed-name + FDR fields — at the same `>37E3`. The
  differential gates compare the `>8356` *value*, not the work area's deep
  bytes.) `>83D0` is left holding the found card's CRU base (`>1100`).

## 1b. The authentic header chains (enumerated from the binary)

✅ (header walk, 2026-07-06): version `>02`, 0 programs; **power-up** chain =
one unnamed node (`len = 0` byte present) → entry `>4070`; **device** chain
in order **`DSK` (len 3), `DSK1`, `DSK2`, `DSK3`** (per-name entry stubs);
**subprogram** chain = `>10 >11 >12 >13 >14 >15 >16` (one-byte names, in
order) then a **named `FILES`** (len 5) node last. No copyright or
provenance string anywhere in the image (strings scan: only the interface
names above) — plan §10 decision 5 resolves to "nothing to replace, ship no
watermark." Our header mirrors the chain *structure and names*; the entry
addresses are ours.

## 2. Power-up & the VRAM buffer region

- ✅ (`probe_powerup_vram`) `>8370` → `>37D7`; header at `top+1..top+5` =
  **`AA 3F FF 11 03`** (marker, literal `>3FFF`, CRU-high, buffer count);
  the region above is zeroed.
- ✅ (`probe_files_sub`) `FILES(n)` (subprogram `>16`, n at `>834C`):
  `>8370` → **`>3DEF − 518·n − 6`** (n=2 ⇒ `>39DD` observed), same 5-byte
  header shape with the new count; error byte `>8350` = 0.
- Power-up runs on every boot including QUIT warm reboots → idempotent.

## 3. The PAB operations (opcodes 0–9)

PAB layout and flag bits as staged by the rig (`dsr_common::pab`): opcode,
flags, buffer, reclen, charcount, recnum, screen-offset, namelen, name.
Errors are OR'd into PAB+1 bits 5–7; **they are sticky** (the DSR never
clears them — `probe_scratch_restore` demonstrated a stale error carrying
into the next op's flag byte).

| Behavior | Pinned by | Result |
|---|---|---|
| OPEN INPUT, missing file | ✅ `probe_errors` | **error 2** (`>14`→`>54`) |
| OPEN, record-length mismatch (40 vs 80) | ✅ `probe_errors` | **error 2** |
| OPEN FIX flags on a VAR file | ✅ `probe_errors` | **error 2** |
| OPEN with PAB reclen = 0 | ✅ `probe_errors` | error 0; **PAB+4 filled** with the file's reclen |
| OPEN OUTPUT on a `'P'`-protected VIB | ✅ `probe_errors` | **error 0** (VIB protection not enforced at OPEN) |
| READ (VAR sequential) | ✅ `probe_read_builder_files` | record → buffer, PAB+5 = record length; **PAB+6/7 unchanged** for VAR |
| READ past the last VAR record | ✅ same | **error 5**, buffer untouched |
| READ (FIX relative, `F_REL`) | ✅ same | record `PAB+6/7` served, PAB+5 = reclen, **PAB+6/7 incremented** |
| WRITE (VAR sequential) | ✅ `probe_write_var` | appends length-prefixed records |
| WRITE (FIX sequential) | ✅ `probe_write_fix` | PAB+6/7 increments per record |
| RESTORE (op 4) with a record # | ✅ `probe_scratch_restore` | repositions (following READ served that record) |
| LOAD (op 5) | ✅ `probe_load_program` | PROGRAM file → VDP at PAB+2; PAB+6/7 (max length) **unchanged** on return; no VIB read (read log `[1, fdr, data…]`) |
| SAVE (op 6) | ✅ `probe_save_program` | creates a PROGRAM file; PAB+6/7 = byte length (input, unchanged) |
| DELETE (op 7) | ✅ `probe_delete` | frees the FDR + data bits in the bitmap, **compacts the FDIR** |
| SCRATCH (op 8) | ✅ `probe_scratch_restore` | **error 6** (rejected by the disk DSR) |
| STATUS (op 9) | ✅ `probe_rig_smoke_status` | PAB+8: `>08` program-exists; `>80` no such file; error bits 0 either way |
| Catalog: OPEN `"DSK1."` | ✅ `probe_catalog` | opens (INT/FIX 38); **READ is record-number-driven** (PAB+6/7), PAB+5 = 38, PAB+6/7 increments; record 0 = volume record |
| Volume record content | ✅ `probe_catalog` | INTERNAL fields: string = volume name **trailing-space-trimmed**; numbers (radix-100, 8-byte, len-prefixed 8): `0`, `total−2` (358 for SSSD), `free` (unallocated count) |
| File catalog records (r ≥ 1) | 📖 + differential | (name trimmed, ±type 1–5 — negative = protected, sectors+1, reclen); the live differential gates pin the exact bytes |
| Volume-name device form `DSK.VOL.FILE` | ✅ `probe_volume_form` | resolves the drive by VIB volume name; unknown volume = **error 7** (still handled/skip-return) |

Radix-100 integers (catalog): `0` = 8 zero bytes; `1..99` = `>40, n`;
`100..9999` = `>41, n/100, n%100`; negative = two's-complement the first
word. ✅ (`probe_catalog`: 358 = `41 03 3A`, 254 = `41 02 36`).

## 4. Subprograms

- ✅ (`probe_sector_sub`) **`>10` SECTOR**: unit `>834C`, read≠0/write=0 at
  `>834D`, VDP buffer `>834E/F`, **sector number = the word at `>8350`**
  (the Appendix-A ambiguity, settled: input is `>8350`); returns the sector
  number in `>834A` (word) and the error in the `>8350` byte; parameter
  cells otherwise preserved.
- ✅ (`probe_files_sub`) **`>16` FILES(n)**: §2 above.
- 📖 `>11` FORMAT / `>12` PROTECT / `>13` RENAME / `>14` FILEIN /
  `>15` FILEOUT: parameter blocks per plan Appendix A §A.2; pinned live by
  the differential gates as each is implemented. FORMAT under this emulator:
  see §7.

## 5. The on-disk format (ground truth: `Tunnels.Dsk` + write probes)

All ✅ (`probe_tunnels_layout`, `probe_write_var`, `probe_write_fix`,
`probe_save_program`, `probe_load_program`, `probe_delete`):

- **VIB**: name(10) · total(BE word `>0A`) · spt `>0C` · `"DSK"` · prot
  `>10` (`' '`/`'P'`) · tracks `>11` · sides `>12` · density `>13` ·
  `>14..>37` zero · bitmap `>38..` LSB-first per byte; sectors 0–1 always
  set; **bitmap bytes past the disk's total = `>FF`** (45 data bytes for
  360 sectors, `>FF` from offset `>65`).
- **FDIR** (sector 1): ≤127 BE pointers, sorted by name, zero-terminated,
  compacted on delete.
- **FDR**: name(10) · `>0A/B` zero · flags `>0C` (bit0 PROGRAM, bit1
  INTERNAL, bit3 PROTECTED, bit7 VARIABLE) · records/sector `>0D` ·
  sectors-allocated BE `>0E/F` · **EOF offset `>10`** (PROGRAM: `len%256`;
  VAR: bytes-used-in-last-sector **excluding** the `>FF` terminator — 14
  observed for 5+1+7+1; FIX: **0**) · reclen `>11` · **`>12/13`
  little-endian count** (FIX: records; VAR: sectors; PROGRAM: 0) ·
  `>14..>1B` zero · 3-byte clusters from `>1C` (`start[7:0]`,
  `start[11:8]|endoff[3:0]<<4`, `endoff[11:4]`; QUEST: `55 20 03` =
  start 85, end-offset 50).
- **Record packing**: FIX — `256/reclen` per sector, no spanning, zero
  tails; VAR — `[len][bytes]…` then `>FF`, zero pad; PROGRAM — raw, zero
  pad in the final sector.
- **Allocation policy**: FDR sector = **first free bit from 0** (lands at
  2, 3, … on a fresh disk); data sectors = **first free from `>22`** (34)
  — both observed. (Fallback when the region is exhausted: ❓ — the fuzz
  differential is the tripwire; our DSR wraps to the low region.)
- **Geometry for sector I/O**: the authentic DSR does **no VIB read** on
  lookups (`probe_load_program` read log) — track math is the stock card's
  hardcoded **9 sectors/track, 40 tracks**; side 1 = absolute ≥ 360. Our
  DSR matches (differential read-log parity); DD images are out of contract
  (stock card, plan §10 decision 4).

## 6. The FDIR search — the bisect, decoded

✅ (`probe_fdir_search_order`, 5 files at FDR sectors 2,3,4,5,6):

Observed FDR probe orders: `GGG → [5]` · `AAA → [5,3,2]` · missing
`ZZZ → [5,6]`. Unique consistent algorithm: **binary search over the full
127-slot pointer table by byte offset** — `lo=0, hi=254`;
`mid = ((lo+hi)/2) & ~1`; an **empty (zero) slot compares "high"** (files
sort to the front), no disk read; a non-empty slot reads that FDR and
compares the 10-byte name; equal → found, greater → `hi=mid`, less →
`lo=mid+2`. (126→62→30→14→6 are empty on a 5-file disk — zero reads — then
6→GGG explains `[5]`; the rest follow exactly.) Our DSR implements the same
shape so the FDR-probe order in the sector read log is differential-clean.

## 7. FORMAT under this emulator (the oracle gap)

- Our FD1771 model no-ops Write Track (`disk.rs:481-482`) — the authentic
  DSR's FORMAT cannot lay sectors here, so **FORMAT has no differential
  oracle** (plan §11.3). Ours re-initializes the mounted image via Write
  Sector (observable result identical; chip choreography is not contract,
  plan §2.5), gated by the structural validator + cross-oracle read-back.
  Scope rides on plan §10 decision 3 (DEFERRED).

## 7b. Write-side pins (added during M3–M5 execution, all ✅ via the
## differential gates)

- READ/WRITE/RESTORE on an **unopened PAB**: error **7**.
- A **protected file** refuses write-mode OPEN (UPDATE/OUTPUT/APPEND) and
  DELETE with error **1**; INPUT opens fine. (The VIB `'P'` byte is *not*
  enforced at OPEN-create — §3.)
- **UPDATE on a missing file creates it** — with a **name-only FDR** (flags,
  recs/sector, reclen all zero; the attributes appear on later use).
- **DELETE on a missing file succeeds silently.**
- **SAVE writes its final partial sector as a full 256 bytes straight from
  the VDP source** — bytes past the program length are whatever follows in
  VRAM, never zero-padded.
- An **unreadable drive** (no disk / no FDIR): OPEN = error **6**.
- A closed empty VAR file (no records written) allocates **no** data sector.
- The `>8350` word after a successful RENAME is an internal artifact
  (observed `>20xx`) — the on-disk result is the contract, not that cell.
- **FILEIN/FILEOUT block placement**: the caller-chosen `>8300+x` block must
  not overlap the DSR-owned `>834A..>836D` workspace — a colliding block is
  undefined under either DSR (the gates use `>8326`).

## 8. Notes for the implementation

- Errors are OR'd into PAB+1 bits 5–7 and never cleared by the DSR.
- STATUS/LOAD/SAVE/DELETE work without an OPEN; the open-file table lives in
  the reserved VRAM region and is keyed by the PAB address (the PAB **is**
  the file handle).
- The DSR owns `>834A..>836D` + `>83DA..>83DF` during a call; `>83E0..>83FF`
  are the GPLWS registers (preserve R1/R9 and R11–R15 across the call —
  our SROM reloads its cursor from `>83D2` but R1 is its found count).
- Every FD1771 register byte is inverted (`XOR >FF`); DRQ/INTRQ are not
  CRU-readable — the synchronous card serves bytes immediately.
