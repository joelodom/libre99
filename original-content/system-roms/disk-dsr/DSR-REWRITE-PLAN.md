# Disk Controller DSR Rewrite — exhaustive plan (Phase 3 of the system-ROM project)

A from-scratch, original-content reimplementation of the **TI Disk Controller's
DSR ROM** (`roms/Disk.Bin`, 8 KiB of TMS9900 machine code at CPU `>4000–5FFF`)
— the `>AA` peripheral header, the power-up VRAM reservation, the PAB file
system (OPEN/CLOSE/READ/WRITE/…, all file types), the low-level subprograms
(`>10`–`>16`), and the FD1771 driver — written in our own TMS9900 dialect,
assembled by our own `libre99-asm`, and proven against the genuine DSR as a
differential oracle. It is the peripheral-firmware counterpart to the completed
console GROM (Phase 1) and console ROM (Phase 2) rewrites
([`../STATUS.md`](../STATUS.md)), and it removes the **last TI firmware that
executes in the emulator's default configuration** — after this, the disk
system is fully our IP, and `Disk.Bin` moves from "required" to
"comparison-only" on the pre-public-release checklist
(`docs/DEVELOPMENT.md` §Pre-public-release, item 3, which names the disk DSR
as the remaining dependency).

Everything for this sub-project lives in this folder,
`original-content/system-roms/disk-dsr/`, mirroring [`../rom/`](../rom/) and
[`../grom/`](../grom/).

*Written 2026-07-06 (planning session; research dossiers in Appendix A).
Status: **EXECUTED — M1–M6 complete (2026-07-06); the clean-room DSR is the
emulator's default, the authentic `Disk.Bin` selectable via `--disk-dsr`.**
24 differential gates green (`crates/libre99-gpl/tests/disk_dsr.rs`); the D1
dossier is [`RECON.md`](./RECON.md) (probe-first — a recorded method
deviation); deep-tier follow-ups (fuzz, perf tripwire) and the ledger live in
[`PROGRESS.md`](./PROGRESS.md).*

Confidence tags, house-style: ✅ verified against this repo's own source or
binaries during planning · 📖 literature/Classic99-derived (consult, never
copy) — **must be re-pinned against the authentic `Disk.Bin` binary by D1
before code relies on it** · ❓ open, to be pinned by the dossier (D1/D2).

---

## 0. Scope, goals, and definition of done

> **★ OVERRIDING GOAL — functional completeness over usage evidence** (the
> Phase-2 mandate, carried over verbatim in spirit). This is a **complete**
> reimplementation of the stock TI disk DSR, not a minimal-viable one. **Every
> operation the authentic 8 KiB DSR implements — every PAB opcode across every
> legal file-type/mode combination, every subprogram, every error path — is
> implemented here and verified against the authentic oracle, whether or not
> any bundled software is known to exercise it.** Absence of usage evidence is
> never a reason to omit, stub, or leave untested. The D2 census exists to
> *order* the work and *aim* fuzz weight — never to license omission.
>
> **The recorded exceptions** (each a disposition, not a shortcut):
> 1. **Non-TI controller variants are out of contract** (§10 decision 4):
>    CorComp / Myarc / HFDC / TIPI extensions — including the subdirectory
>    subprograms (`>17+`) and Myarc CPU-RAM buffering (`>834C` bit 7) — are
>    other cards' firmware, not this card's.
> 2. **FORMAT's FD1771 mechanism is substituted, not reproduced** (§4 P3-DISK,
>    §10 decision 3): the authentic DSR formats with the FD1771 *Write Track*
>    command, which our emulated controller does not implement (✅
>    `crates/libre99-core/src/disk.rs:481-482` — `>E0`/`>F0` are no-ops). The
>    FD1771 command stream is not software-visible (only the DSR talks to the
>    chip), so our FORMAT produces the **identical observable result** — a
>    correctly initialized image — via Write Sector. The *observable* surface
>    stays complete; only the chip choreography differs, and the substitution
>    is documented + gated.
> 3. **Write durability to the host `.Dsk` file** is emulator/app work, not
>    DSR work (§9, §10 decision 1): our FD1771 model mutates only the
>    in-memory image (✅ `disk.rs:94-98`). The DSR's write behavior is complete
>    and correct either way; persistence is a companion feature.

**In scope:**

1. An **original 8 KiB DSR image** (`disk-dsr/disk-dsr.bin`, base `>4000`)
   with no TI bytes, assembled from TMS9900 source in this folder by
   `libre99-asm`.
2. The **full stock-TI operation surface**: the `>AA` header + link chains;
   the power-up VRAM reservation; device names `DSK1`/`DSK2`/`DSK3` + the
   volume-name form (`DSK.VOLNAME.FILE`); PAB opcodes 0–9 (OPEN, CLOSE, READ,
   WRITE, RESTORE/REWIND, LOAD, SAVE, DELETE, SCRATCH, STATUS) across all
   file types (DIS/FIX, DIS/VAR, INT/FIX, INT/VAR, PROGRAM) and modes
   (INPUT/OUTPUT/UPDATE/APPEND, sequential + relative); the catalog
   mechanism (OPEN `"DSKn."`); subprograms `>10`–`>16` (SECTOR, FORMAT,
   PROTECT, RENAME, FILEIN, FILEOUT, FILES); every error code byte-exact.
3. The **on-disk file system, byte-exact**: VIB, allocation bitmap, FDIR
   (sorted, binary-searched), FDRs, 3-byte cluster chains, FIXED/VARIABLE/
   PROGRAM record packing — interoperable both directions with disks the
   authentic DSR reads and writes.
4. The **FD1771 driver** against our emulated controller's exact register/CRU
   contract (§2.1), with correct error mapping.
5. **Both console firmwares run it**: our clean-room console ROM+GROM *and*
   the authentic TI console ROM+GROM (the mix-and-match requirement; both run
   the genuine DSR today, so both define the same calling convention ✅).
6. A **production-quality committed binary**: the full gate matrix green, no
   crash/hang/corruption reachable from disk user flows, perf within budget,
   docs complete, staleness gate tying `disk-dsr.bin` to its source.

**Out of scope (recorded dispositions):**

- Non-TI controllers and subdirectories (exception 1 above).
- **PC99 track-format images** — our emulated card mounts raw v9t9-style
  sector dumps only (✅ `disk.rs:79-98`); no track-level media exists to read.
- **Double-density / 80-track FORMAT** (the stock TI FD1771 card is
  single-density; we *read* any geometry the VIB declares, as the hardware
  model already does ✅) — a labeled stretch after production, never the
  baseline (§10 decision 3).
- **Blank-disk creation / import-export UI** — app-level media management
  (`docs/ROADMAP.md` §2 [later]); the DSR only needs a mounted image.
- **RS232 DSR and Speech ROM** — dormant peripherals (hardware not emulated);
  future siblings of this track, not part of it.
- The **TI BASIC side of file I/O** (PRINT#/INPUT# tokenization etc.) — that
  is console GROM/ROM M6 territory (deferred by policy, `../LIMITATIONS.md`
  L9). TI BASIC *under authentic console firmware* is used here as a test
  *client* of our DSR, which is exactly the mix-and-match case 5 above.

**Definition of done** (from the repo root; `--disk-dsr` lands in M6):

```sh
# 1  our DSR as default: ToD loads QUEST from disk under BOTH console firmwares
cargo run -p libre99-app -- --cartridge tundoom --disk Tunnels
cargo run -p libre99-app -- --system-rom roms/994aROM.Bin --system-grom roms/994AGROM.Bin --cartridge tundoom --disk Tunnels
# 2  the authentic DSR stays selectable for comparison
cargo run -p libre99-app -- --disk-dsr roms/Disk.Bin --cartridge tundoom --disk Tunnels
```

- Tunnels of Doom loads a QUEST scenario; the 15 bundled `.Dsk` images
  catalog identically under both DSRs; TI BASIC (authentic firmware) file
  scripts — OPEN/PRINT#/INPUT#/CLOSE, SAVE/OLD, DELETE — behave identically
  under both DSRs; a disk **our** DSR writes/formats reads back correctly
  under the **authentic** DSR and vice versa (the cross-oracle, §8).
- The completeness checklist (all P9-style boxes below) is green:
  - [ ] Every PAB opcode × every legal type/mode combination gated
    differentially.
  - [ ] Every subprogram `>10`–`>16` gated (param blocks per dossier).
  - [ ] Every error code (0–7) driven and byte-exact in PAB+1 bits 5–7 /
    `>8350` per convention.
  - [ ] Catalog record stream byte-exact for every bundled disk.
  - [ ] Power-up reservation + buffer-header shape byte-exact (`>8370` =
    `>37D7`, the `>AA` header above it).
  - [ ] The on-disk structures our writes produce validate structurally and
    cross-read under the authentic DSR.
  - [ ] A coverage-completeness test cross-checks D1's element enumeration
    against the gate set and fails on any element without a gate.
- `cargo test --workspace` green including the new `dsr` gates; `cargo clippy
  --workspace` clean; docs current (this folder's README, `../STATUS.md`,
  `docs/STATUS.md`, `docs/DEVELOPMENT.md`'s checklist, `docs/KNOWN-ISSUES.md`
  if any gap ships).

---

## 1. Background — what the disk DSR actually is

The TI Disk Controller card carries a WD **FD1771** floppy controller, an
8 KiB **DSR ROM**, and glue — and **no RAM**. That single fact shapes the
whole design:

- **All bulk state lives in VDP RAM.** The DSR's power-up routine lowers the
  console's top-of-free-VRAM pointer (`>8370`) to reserve a buffer region;
  every sector buffered, every FDR cached, and the **open-file table that
  persists between DSR calls** live there. CPU-side, the DSR may use only the
  console scratchpad (`>8300–83FF`) — a stock console has no other RAM (the
  32 K expansion may be absent), and the DSR must work without it.
- **Sector bytes flow FD1771 → CPU → VDP one at a time.** The DSR reads the
  controller's data register and writes the VDP data port per byte (and the
  reverse for writes), polling the controller's Busy/DRQ status between
  bytes — on our synchronous emulated controller the polls simply always
  succeed (§2.1).
- **The DSR is discovered, not linked.** The console knows nothing about the
  card beyond the CRU scan: it enables each card's ROM (CRU bit 0), checks
  for `>AA` at `>4000`, and walks the header's link chains to find named
  device entries, numbered subprograms, and the power-up routine. **The
  console never enters the DSR at a fixed interior address** — which is why
  this rewrite's internal layout is almost entirely free (§3), unlike the
  console ROM's.
- **Two client protocols ride the same discovery walk**: file-level I/O via a
  **PAB** (Peripheral Access Block) staged in VDP RAM plus the device name
  (`DSK1.FILE`), and register-level **subprograms** (single-byte names
  `>10`–`>16`) whose parameter blocks live in scratchpad. TI BASIC's
  OPEN/PRINT#/INPUT#, the console loaders, and cartridges like Tunnels of
  Doom all reduce to these two shapes.

**Who calls it, in this emulator** ✅: the console ROM's `SROM` (`XML >19`,
our clean-room implementation at `console.asm:2949-2989`, spec
`../rom/RECON.md` §24) performs the scan/walk/call; the GPL side (our GROM's
DSRLNK at slot `>0010`, or the authentic GROM's) stages the scratchpad cells
and issues the `XML`. Both our console firmware and the authentic one run the
genuine `Disk.Bin` successfully today (`crates/libre99-gpl/tests/device_io.rs`
passes under both ✅), so the calling convention below is common — our DSR
honoring it works under both by construction.

**We hold a perfect oracle and the probe rig already exists** ✅:
`Machine::load_disk_controller(rom)` takes the DSR image as a parameter
(`crates/libre99-core/src/machine.rs`), `Disk::record()` captures a per-sector
read log and a full register/CRU trace (`disk.rs:301-316`), and the existing
disk test estate (`libre99-core/tests/disk.rs`, `libre99-gpl/tests/device_io.rs`)
drives real flows end-to-end. Point the DSR constant at our image and the
whole estate becomes this rewrite's regression net — the same seam trick the
console-ROM track used.

**Field-test clients available** ✅: Tunnels of Doom (the existing QUEST-load
gate); the **15 bundled `.Dsk` images** (`disks/`) as a read/catalog corpus;
**TI BASIC under the authentic console firmware** as a scriptable PAB
generator (OPEN/PRINT#/INPUT#/CLOSE/SAVE/OLD/DELETE via scripted keystrokes).
There is **no Disk Manager or Editor/Assembler cartridge in the bundle** —
FORMAT/PROTECT/RENAME are exercised through the subprogram-level PAB rig and
TI BASIC where possible, not through TI's own manager UI.

---

## 2. The compatibility contract (what MUST be reproduced)

Four surfaces. §2.1 and §2.2 are fully pinned already (our own source is the
authority); §2.3 and §2.4 are 📖-seeded (Appendix A) and D1 re-pins them
against the binary before implementation.

### 2.1 The hardware surface (client: our emulated FD1771 card) — ✅ pinned

From `crates/libre99-core/src/disk.rs` (read in full during planning):

- **Window `>4000–5FEF`**: the DSR ROM, visible only while CRU bit 0 is set.
  **`>5FF0–5FFE`**: the FD1771 registers overlay the top of the window and
  respond **regardless of the ROM-enable bit**. The top 16 bytes of our 8 KiB
  image are therefore shadow — **never place code or data at image offsets
  `>1FF0–1FFF`**.
- **Every register byte is transferred one's-complemented** (`XOR >FF`), both
  directions (`disk.rs:322-353`).
- **Registers**: read `>5FF0` Status / `>5FF2` Track / `>5FF4` Sector /
  `>5FF6` Data; write `>5FF8` Command / `>5FFA` Track / `>5FFC` Sector /
  `>5FFE` Data. Even addresses only; odd addresses inert.
- **CRU at base `>1100`** (`disk.rs:357-385`): bit 0 ROM-enable · bits 1–3
  motor/wait-states/head-load (**no-ops**) · bits 4–6 **one-hot drive select**
  → DSK1/DSK2/DSK3 · bit 7 side. Readable: **only bits 0 and 7** — everything
  else reads 0. **INTRQ/DRQ are not CRU-visible; poll the Status register's
  Busy bit (0x01).**
- **Commands implemented** (high nibble): `>0x` Restore · `>1x` Seek (target
  track taken from the Data register) · `>2x/>3x` Step (latched direction) ·
  `>4x/>5x` Step-in · `>6x/>7x` Step-out · `>8x/>9x` Read Sector · `>Ax/>Bx`
  Write Sector · `>C0` Read Address (6-byte ID: track, side, sector, size
  `>01`, `>FF >FF` CRC) · `>D0` Force Interrupt (aborts a transfer). **`>E0`
  Read Track and `>F0` Write Track are no-ops** — the FORMAT consequence is
  §0 exception 2 and risk §11.3.
- **Status bits (TI values)**: `>80` NOT_READY · `>40` WRITE_PROTECTED ·
  `>10` RECORD_NOT_FOUND · `>04` TRACK_0 · `>02` DRQ · `>01` BUSY.
- **Transfers are synchronous whole-sector**: Read Sector stages all 256
  bytes at once; each Data-register read serves the next byte and the last
  clears BUSY (writes mirror this, flushing on byte 256). Motor and
  wait-state timing do not exist — a DSR that skips spin-up waits is correct
  here.
- **LBA mapping** is the card's job, not the DSR's — the DSR just seeks
  tracks and asks for sectors; the card maps (track, side, sector) through
  the mounted image's VIB-derived geometry, side 1 in v9t9 reverse-track
  order (`disk.rs:521-532`).
- **Diagnostics** ✅: `Disk::record(true)` → `read_log()` (absolute sectors
  read) + `trace()` (every register/CRU access) — the DSR-specific probe
  instruments, already built.

### 2.2 The console→DSR calling convention (clients: both console ROMs) — ✅ pinned

From our own `console.asm:2937-2989` + `../rom/RECON.md` §24 (and matching
the authentic ROM, which the same tests pass under):

- **Discovery**: SROM scans CRU bases `>1000..>1F00` step `>0100`; `SBO 0`
  enables a card's ROM (previous card `SBZ 0`-disabled first); byte at
  `>4000` must equal `>AA`; the chain head is read from `>4000 + key`, where
  the **key at `>836D`** is `>04` power-up · `>06` program · `>08` DSR
  (device) · `>0A` subprogram. Node format: `[link word][entry word]
  [name-length byte][name…]`; zero link ends the chain.
- **Name matching**: search-name length at **`>8355`** (0 = match every node
  — the power-up case), name text staged at **`>834A`**. (⚠ house note: one
  RECON line loosely says `>8354`; the code reads `>8355` — the code wins.
  `>8354` is the word whose low byte is the length.)
- **The call**: `BL *R9` with **WP = GPLWS `>83E0`**, **R12 = the card's CRU
  base** (ROM already enabled), **R13/R15 = GROM/VDP ports intact**
  (`>9800`/`>8C02`), **interrupts masked (`LIMI 0`)** — never re-enable —
  **R9 = entry**, `>83D0`/`>83D2` = the walk cursors, **`>8356` = VDP address
  of the byte after the device name in the PAB** (the DSR's handle: the PAB
  base is `[>8356] − namelen − 10`).
- **Return protocol — the wedge hazard, get it exactly right**:
  - **Plain return** (`B *R11`): "not mine / keep walking." SROM continues
    the chain and then the scan. **Power-up entries always return plainly.**
  - **Skip return** (`INCT R11` then `B *R11`): "**I handled it**" — success
    *or* an I/O error already reported in the PAB error byte. Our SROM then
    disables the card (`SBZ 0`), **pops the GPL DSRLNK CALL frame** (the
    caller resumes past DSRLNK's error tail), and clears the condition bit
    (`console.asm:2986-2988`; the ToD-hang case study,
    `../rom/RECON.md` §26 addendum).
  - **Device not found** (no node ever skip-returns): the GPL side raises the
    error — gated by `device_io.rs::bad_device_errors_gracefully_without_hanging` ✅.
- **Error reporting**: file-level errors go in **PAB byte 1, bits 5–7**
  (written by the DSR into VDP RAM); subprogram errors in **`>8350`** 📖. The
  console ROM itself never touches `>8350` ✅.
- **Power-up** (key `>04`, walked at every console boot *including QUIT
  warm reboots* — the routine must be idempotent): reserve the VRAM buffer
  region by lowering **`>8370` from `>3FFF` to exactly `>37D7`** (the
  3-file default), lay down the disk-buffer header just above the new top
  📖 (`>AA` marker, top-of-VRAM word, CRU-high `>11`, buffer count — other
  software checks this shape), clear the reserved region via VDP writes
  (the bundled DSR does ✅ — skipping it corrupts the title screen,
  `libre99-core/tests/disk.rs` title gate), and return plainly. Gate exists ✅:
  `device_io.rs::disk_power_up_reserves_vram` (asserts `>8370 == >37D7`
  under both console ROMs).
- **Scratchpad discipline**: the DSR owns `>834A–>836D` (+ `>83DA–>83DF`) 📖
  during a call — TI's own File Management Specification treats that block as
  DSR workspace. Our DSR confines itself to those cells, the GPLWS registers
  the contract hands it, and its VRAM region. House rule inherited from the
  ROM track: **the `>83E0–83FF` cells ARE the GPLWS registers** — comment
  every such reference with its register identity.
- **Observable return side-effects** ❓: the authentic DSR reportedly leaves
  `>8354` → the PAB address and `>8356` → a pointer to the file buffer's FDR
  copy (first byte zeroed) 📖. D1 verifies on the binary; if real software
  can observe it, P3 says reproduce it.
- **Interrupts**: the DSR never uses them; the header's ISR link (`>400C`)
  ships `>0000` (our emulator raises no card interrupts ✅, and the stock
  disk card has no ISR).

### 2.3 The on-disk file-system format — byte-exact (the core deliverable) — 📖 seed, D1 pins

The interoperability surface: existing images must read identically, and
disks we write must be valid to the authentic DSR and to period tools.
Detailed field tables in **Appendix A §A.4**; headline structure:

- **Sector 0 — VIB**: volume name (10) · total sectors (BE word) ·
  sectors/track · `"DSK"` marker · protection byte · tracks/side · sides ·
  density · reserved · the **allocation bitmap at `>38`** (bit *s* = sector
  *s* allocated, LSB-first per byte; sectors 0–1 permanently allocated).
  (The geometry fields are already ✅ — our emulator parses them, verified
  against Classic99, `disk.rs:131-173`.)
- **Sector 1 — FDIR**: up to 127 big-endian pointers to FDR sectors,
  **sorted ascending by filename**, `>0000`-terminated. **The authentic DSR
  binary-searches this list** 📖⚠ — we reproduce the bisection (P3), which
  also gives a beautiful differential instrument: identical bisection ⇒
  identical FDR-probe order in the sector read log.
- **FDR** (one sector per file): name (10) · reserved · **flags** (bit 0
  PROGRAM, bit 1 INTERNAL, bit 3 PROTECTED, bit 7 VARIABLE) ·
  records/sector · sectors-allocated (BE) · EOF-offset · record length ·
  **record/sector count at `>12` stored byte-swapped (little-endian!)** ·
  reserved · the **data chain**: 3-byte clusters packing a 12-bit start
  sector + 12-bit file-relative end-offset, `>000000`-terminated.
- **Record packing**: FIXED — `256/reclen` per sector, never spanning,
  tail wasted; VARIABLE — length-prefixed records, **`>FF` length byte
  terminates a sector's records**, zero-pad; PROGRAM — raw contiguous image,
  EOF-offset in the last sector.
- **Allocation policy** ❓: Classic99 searches free sectors from `>22` (34)
  upward 📖; D1 pins the authentic DSR's actual policy (it determines where
  our writes land, hence byte-level diffability of written images).

### 2.4 The operation surface (clients: TI BASIC, cartridges, E/A-style software) — 📖 seed, D1 pins

- **PAB** (VDP RAM): opcode · flag/status · buffer addr · record length ·
  char count · record number · screen offset · name length · name. Opcodes
  0–9 = OPEN, CLOSE, READ, WRITE, RESTORE, LOAD, SAVE, DELETE, SCRATCH,
  STATUS; flag bits: `>10` VARIABLE · `>08` INTERNAL · `>01` RELATIVE · mode
  in bits 1–2 (UPDATE/OUTPUT/INPUT/APPEND). **Error codes 0–7** returned in
  PAB+1 bits 5–7 (0 doubles as "bad device name"); the **STATUS opcode's
  result byte** in PAB+8. Full semantics per opcode: Appendix A §A.3.
- **Subprograms `>10`–`>16`** (single-byte-name nodes on the `>400A` chain):
  `>10` SECTOR (raw sector read/write) · `>11` FORMAT · `>12` PROTECT ·
  `>13` RENAME · `>14` FILEIN · `>15` FILEOUT · `>16` FILES(n). Scratchpad
  parameter blocks per Appendix A §A.2 — with one flagged ambiguity D1 must
  settle by probe: **SECTOR's sector-number cell (`>8350` in vs `>834A`
  work/out)**.
- **The catalog**: OPEN of `"DSKn."` (device, no filename) as INT/FIX 38
  yields a record stream — record 0 the volume, then one record per file
  (name + type/protection + size + record length), numbers in **radix-100
  floating-point** (INTERNAL format) ❓ exact field layout D1 (probe: a TI
  BASIC catalog program under the authentic DSR, dump the records).
- **FILES(n)** re-sizing and its VRAM buffer-header shape; multiple files
  open concurrently (default 3) with state persisting across calls in the
  reserved VRAM (§1).

### 2.5 What is explicitly NOT contract

- **Internal addresses.** No external software enters the DSR except through
  the header chains (the console `BL *R9`s whatever the nodes say). D1's
  entry census verifies; unless it finds a counter-example, only `>4000`
  (`>AA`) and the header link words at `>4004`/`>4006`/`>4008`/`>400A`/
  `>400C` are pinned. **P8 is deliberately relaxed here** (§3) — the big
  structural simplification vs. the console-ROM rewrite.
- **The FD1771 command stream** — observable only by the emulated card, which
  is ours. What must match is the *result* (image bytes, PAB/scratchpad
  returns, VRAM effects). This is what licenses the FORMAT substitution.
- **Internal timing.** The call runs under `LIMI 0` — no ISR ticks during it;
  wall-clock inside the DSR is software-invisible. Faster-than-authentic
  (skipping motor waits our card doesn't model) is fine and expected; a
  frames-level tripwire keeps it *bounded* (§8), not identical.
- **TI's expression**: instruction sequences, register allocation, comments.
  Clean-room per §4 P5.

---

## 3. What we replace vs. preserve (original-content policy)

- **Replaced (everything):** all executable content is written new from the
  dossier's behavioral spec. The DSR has no creative on-screen content (no
  strings are user-visible through it ❓ — D1 scans the binary; §10
  decision 5 covers any found provenance string).
- **Preserved (the uncopyrightable interface):** the `>AA` header format and
  link-node shape; the device/subprogram names (`DSK1`… and the byte values
  `>10`–`>16`); the PAB/scratchpad conventions; the on-disk format; the
  power-up VRAM contract (`>37D7`, the buffer-header shape); error-code
  values. Functional facts required for interoperability — same policy as
  the GROM keytabs and the ROM's dispatch tables.
- **Byte-identical content:** *minimal by design* — the `>AA` byte, the six
  header words, and the on-disk *structures our writes produce*. Everything
  else in the image is original expression at addresses of our choosing.
  If D1/D2 ever observe software reading DSR ROM bytes as data or entering
  a fixed interior address, those bytes/addresses join a DATA-MUST-MATCH set
  with identity gates, GROM-track style.
- **License headers:** `disk-dsr.asm` carries the short two-line license
  pointer (firmware-source convention); new Rust files (builder, tests)
  carry the full `LICENSE.md` header (repo convention).

---

## 4. Strategy & method

Inherited from Phase 2, with disk-specific amendments:

- **P1 — Dossier before code.** No component implemented before its
  contract row in `disk-dsr/RECON.md` is ✅-tagged with disassembly/probe
  evidence. Appendix A is the *seed*, not the dossier: **every 📖 fact gets
  re-pinned against the binary** (the ROM track's documentation trap — where
  literature and the binary disagree, the binary wins).
- **P2 — Differential-first, three oracles.** The genuine `Disk.Bin` under
  our emulator is a perfect oracle for anything it can execute:
  1. **Per-op differential**: identical machine + identical image + identical
     PAB/scratchpad stimulus under `[TI_DSR, OUR_DSR]`; diff the resulting
     image bytes, the PAB/scratchpad returns, VRAM effects, and the
     `Disk::record()` read-log/trace.
  2. **Cross-oracle interop** (the automated replacement for third-party
     tools — the PC has no Python, so xdt99 can't be a cargo gate): a disk
     **our** DSR writes must read back byte-identically under the
     **authentic** DSR, and vice versa. Fully automatable in cargo, and the
     strongest possible format check.
  3. **The independent structural validator**: a small Rust FS checker
     written *from the spec* (not from the DSR code) that validates VIB/
     bitmap/FDIR/FDR/cluster invariants and extracts file contents — used
     where the authentic oracle can't run (FORMAT, §11.3) and as a
     tie-breaker.
  Third-party round-trips (xdt99/TIImageTool) remain a **manual deep-tier
  check** on the Mac at M3/M5 — documented, not gated.
- **P3 — Behavior-compatible, bug-for-bug** at every software-visible
  surface: the FDIR bisection, error-code quirks, return side-effects,
  catalog record shapes. **P3-DISK scoping:** the FD1771 command stream is
  *not* software-visible (§2.5) — chip choreography may differ where our
  hardware model requires it, with the substitution documented and the
  observable result gated.
- **P5 — Clean-room IP discipline.** Consult Classic99 + the WD1771
  datasheet + TI literature to *identify*; disassemble `Disk.Bin` (our
  `libre99asm dis`) **only to extract interface facts** into the dossier; write
  the implementation from the dossier. Never transcribe TI instruction
  sequences. The dossier is the firewall.
- **P6 — Gates are cargo tests; artifacts are committed.** Fast tier
  pre-commit; `#[ignore]`d deep tier (full corpus sweeps, fuzz, cross-oracle
  matrices). `disk-dsr.bin` rebuilt + committed whenever source changes,
  with a staleness gate tying binary to source (house pattern).
- **P7 — One source of truth per fact.** DSR facts in `disk-dsr/RECON.md`;
  shared console-side facts stay in `../rom/RECON.md` / `../RECON.md`
  (cross-referenced, never duplicated); build/run facts in this folder's
  README.
- **P8-DISK — pin only the discovery surface.** `>4000` + the header words;
  free layout everywhere else (rationale §2.5). The `check_layout` gate pins
  the header cells; D1's entry census is the tripwire that would ever extend
  the pinned set.
- **P9 — Functional completeness is the acceptance bar** (§0 ★). The D1
  element enumeration — every opcode × type × mode, every subprogram, every
  error code, the catalog, power-up — is the machine-checkable spine the
  completeness test enforces. The census orders; it never filters.

---

## 5. Milestone T1 — toolchain (small: the console-ROM work built almost all of it)

`libre99-asm` today ✅: full TMS9900 ISA (incl. `BLWP/RTWP/LDCR/STCR/TB/SBO/SBZ`
and all byte ops), two-pass, absolute `AORG` mode with per-byte overlap
guard, fixed-size zero-padded raw output, listing + symbols +
`check_layout`, `expand_includes`. Gaps to close:

1. **A `>4000`-based build entry**: `Options::absolute_image(size)` hardcodes
   `base: 0` ✅ (`lib.rs:139-141`) — add `build_disk_dsr()` in a new
   `crates/libre99-asm/src/disk_dsr.rs` (mirroring `system_rom.rs`:
   `include_str!` the source, assemble with `Options { base: 0x4000,
   absolute: true, image_size: 0x2000, auto_header: false, .. }`,
   `check_layout` the header pins, return the 8 KiB image). Optionally add
   an `Options::absolute_image_at(base, size)` helper.
2. **CLI**: `libre99asm dsr <out.bin> [--listing] [--symbols]`, mirroring
   `libre99asm rom`.
3. **Layout pins**: `>4000` = `>AA` and the header link words; an assertion
   that image offsets `>1FF0–1FFF` are all zero (the register shadow).
4. **Tracer bullet**: a hello-card DSR — header + a power-up that performs
   the full `>8370`→`>37D7` reservation + a `DSK1` device node whose handler
   plain-returns. Gate: the console boots clean with it mounted under both
   console firmwares, SROM finds and calls it, `disk_power_up_reserves_vram`
   passes, and the title screen still draws.

**Gate T1:** `cargo test -p libre99-asm` green with the new `dsr` tests (image
is 8 KiB, `rom[0] == >AA`, header words resolve, shadow region empty);
tracer bullet boots.

---

## 6. Milestones D1/D2 — the dossier (recon before code)

### D1 — the static dossier: `disk-dsr/RECON.md` + `disk-dsr/SURFACE-MAP.md`

Disassemble the authentic image with `libre99asm dis` (consult TI literature /
Classic99 to *identify*, never to copy):

- **Component map**: every byte of the 8 KiB classified (HEADER / POWER-UP /
  DSR-ENTRY / PAB-op-<name> / SUBPROGRAM-<n> / FD1771-DRIVER / FS-<structure>
  / CATALOG / DATA / STRINGS / DEAD), with the used-vs-slack tally (sizing
  input for our layout).
- **Re-pin every Appendix A 📖 fact against the binary** — the header walk,
  the PAB parse, each opcode handler's observable semantics, each
  subprogram's parameter block, the VRAM buffer layout (the per-file slot
  format and the open-file table — **the persistent-state contract**, needed
  byte-exact if authentic-opened files must survive… ❓ D1 decides whether
  cross-DSR open-file state is contract or per-boot-private), the FD1771
  choreography per operation, the error mapping (FD1771 status → codes 0–7),
  and the allocation policy for writes.
- **Settle the flagged ambiguities** (Appendix A §A.7): SECTOR's `>8350` vs
  `>834A`; the FDIR binary search (probe its FDR read order via the read
  log); the return side-effects (`>8354`/`>8356`); the catalog record
  layout; every CRU bit the DSR actually reads (our card returns 0 for bits
  1–6 ✅ — any dependence would already misbehave; enumerate anyway);
  **FORMAT's mechanism** (confirm Write Track usage, and *probe what the
  authentic FORMAT does under our emulator today* — expected: silently
  broken, which changes nothing for us but belongs in `docs/KNOWN-ISSUES.md`
  for the authentic-DSR configuration).
- **Strings scan** (decision 5 input): any copyright/provenance text in the
  binary.
- **The element enumeration** (the P9 spine): a machine-checkable table —
  every PAB opcode × legal type/mode, every subprogram, every error code,
  catalog, power-up — that the coverage-completeness test reads.
- **The entry census tripwire**: assert (over the D2 corpus) that the DSR is
  entered only via header-node addresses — the P8-DISK guard.

### D2 — the dynamic census (authentic DSR installed)

Instrument (diagnostics-only, `Disk::record()` + a PC-region probe if
needed) and run the corpus: the ToD QUEST load, TI BASIC file scripts under
authentic firmware, a catalog of all 15 bundled disks, subprogram probes.
Output: which entries/opcodes/subprograms real software hits (**orders** M2–
M4 work and fuzz weight; never gates inclusion), plus the golden
read-corpus expectations (per-disk catalog dumps, per-op read logs).

**Gate D1/D2:** dossier docs committed; every §2.3/§2.4 ❓ resolved or
explicitly deferred with rationale; the census re-runnable as an
`#[ignore]`d test.

---

## 7. Milestones M1–M6 — the implementation ladder

House rules per increment: implement from the dossier → differential gates →
update RECON/PROGRESS → commit. Never copy TI bytes. Rebuild + commit
`disk-dsr.bin` once it exists (M1+).

| # | Milestone | Contents | Gate (all `cargo test`-runnable) |
|---|---|---|---|
| **M1** | **Card plumbing + the sector primitive** | The `>AA` header + all four chains (power-up, program=empty, device, subprogram); power-up (reserve to `>37D7`, buffer header, clear, idempotent); device-name resolution `DSK1/2/3` (+ `DSK.` volume form parse); the dispatch skeleton with **correct skip-return discipline**; the FD1771 driver (CRU select/side, seek+verify, read/write sector, un-invert, BUSY poll, error mapping); **SECTOR (`>10`)** complete; the **PAB rig** (the test harness that stages PAB/scratchpad and drives the DSR through the console's own DSRLNK path — foundational instrument, built here) | found + called by SROM under **both console ROMs**; `disk_power_up_reserves_vram` + title-screen gates parameterized over `[TI_DSR, OUR_DSR]`; SECTOR read/write differential-clean (image + scratchpad + read-log) |
| **M2** | **The read side** | VIB parse; **FDIR binary search** (bisection per dossier); FDR interpretation; cluster-chain traversal; OPEN(INPUT) for all five types; READ (sequential + relative); CLOSE; RESTORE; STATUS; **LOAD** (program image); the **catalog** stream; the FILES/VRAM buffer model needed to hold open files | **ToD loads QUEST** under both console firmwares × our DSR (the existing `device_io.rs`/`disk.rs` gates re-pointed); **catalog of all 15 bundled disks byte-identical** to authentic; golden read-corpus per file type; FDR-probe-order lockstep (the bisection instrument) |
| **M3** | **The write side + FORMAT** | Bitmap allocate/free per the pinned policy; FDR create/update; FDIR sorted insert/remove; OPEN(OUTPUT/UPDATE/APPEND); WRITE (fixed + variable incl. the `>FF` terminator discipline); SAVE; DELETE; SCRATCH; disk/file protection enforcement; **FORMAT (`>11`)** re-initializing the mounted image in place via Write Sector (§0 exception 2; geometry per §10 decision 3) | per-op differential where the oracle runs; **the cross-oracle**: ours-writes→authentic-reads and authentic-writes→ours-reads, byte-level content + catalog equality; the structural validator green on everything we write; TI BASIC scripted SAVE/OLD/PRINT#/INPUT# identical under both DSRs |
| **M4** | **The rest + completeness** | PROTECT, RENAME, FILEIN, FILEOUT, FILES(n) re-size; the volume-name (`DSK.VOL.FILE`) path end-to-end; concurrent open files (interleaved ops across ≥2 files and drives); every error path driven (write-protected, bad attribute, illegal op, disk/FDIR full, past-EOF, device error, file-not-found; error-0 dual meaning); the return side-effects if pinned observable | the **complete per-element differential microsuite** + the completeness assertion (gate set ⊇ D1 enumeration); the entry-census tripwire green |
| **M5** | **Hardening** | Random-PAB differential **fuzz** (seeded legal op sequences over synthetic disks; diff final image + returns vs authentic; validator + cross-oracle where the oracle can't run); **robustness** (corrupt VIB/FDIR/FDR/chains, full disk, 127-file FDIR, fragmentation, unformatted image, missing disk, wrong drive, protected everything — never hang, never corrupt scratchpad outside the owned block); the perf tripwire (ToD-load frames ≤ authentic ×1.25 — expected *faster*); the deep-tier all-disks × both-DSRs × both-console-firmwares matrix; manual third-party round-trip check recorded | deep tier green over committed seeds; robustness suite green |
| **M6** | **Packaging + integration** | Committed `disk-dsr.bin` + staleness gate; `disk-dsr/README.md` (front door: build/run, address map, method, perf numbers, test estate); app integration per §10 decisions 1–2 (`--disk-dsr` flag, default selection, assets embed alongside the kept `Disk.Bin`); docs sync (`../STATUS.md`, `../README.md`, `docs/STATUS.md`, `docs/DEVELOPMENT.md` checklist item 3, `docs/KNOWN-ISSUES.md`, in-app F1 if user-visible); archive this plan to `../history/` with a banner naming `PROGRESS.md`/`README.md` as successors | §0 definition-of-done checklist fully checked |

Sizing: the authentic image's used/slack split is a D1 deliverable; our
budget is 8 KiB minus the 16-byte register shadow, single bank, no
address-pinned interiors — fit risk is low (the console ROM packed a full
GPL interpreter + FP package into the same 8 KiB *with* pinning).

---

## 8. Testing strategy

- **The DSR matrix.** Parameterize the existing estate over
  `[TI_DSR, OUR_DSR]`: `crates/libre99-core/tests/disk.rs` (register-level +
  ToD end-to-end + title screen) and `crates/libre99-gpl/tests/device_io.rs`
  (both console ROMs; QUEST load, bad-device, power-up reservation). The
  console-firmware dimension already exists there — the full matrix is
  `[TI_ROM, OUR_ROM] × [TI_DSR, OUR_DSR]` where flows warrant it.
- **The per-element differential microsuite** (new,
  `crates/libre99-asm/tests/dsr_*.rs`, mirroring the `gpl_*.rs` pattern):
  build our DSR in-memory via `build_disk_dsr()`, drive one element per test
  through the PAB rig from a controlled machine + synthetic disk, diff
  everything observable. Fast tier, pre-commit.
- **The PAB rig** (M1): stages a PAB + name in VDP, the scratchpad cells,
  and invokes the console's own GPL DSRLNK path (a tiny GPL driver assembled
  in-memory by `libre99-gpl`, the `device_io.rs` precedent) — so every test
  exercises the *real* discovery walk, both console firmwares selectable.
- **Golden corpus, generated not committed**: at test time, script the
  *authentic* DSR (under the emulator) to author per-type fixture disks
  deterministically; our DSR must read them identically. No new binary
  fixtures in the repo, no provenance questions.
- **The cross-oracle interop gates** (M3+, the format's heart): write under
  ours → read/catalog under authentic (and reverse), byte-level equality on
  file contents + catalog records.
- **The independent structural validator** (test-support Rust, written from
  the spec): VIB/bitmap/FDIR-sort/FDR/cluster invariants + file extraction —
  the FORMAT gate and the fuzz's corruption detector.
- **Instruments**: `Disk::record()` read-log + register/CRU trace ✅; the
  read-log doubles as the bisection-order lockstep check; loud-stub
  discipline inside our DSR during bring-up (a breadcrumb cell in our owned
  scratchpad block, asserted clear by every boot gate — house pattern).
- **Fuzz**: seeded random legal PAB/subprogram sequences over synthetic
  disks, deep tier, committed seeds (house SplitMix64 pattern).
- **Two-tier gating**, house pattern: fast suite pre-commit; `#[ignore]`d
  deep tier (corpus sweeps, fuzz, matrices) before demos/after multi-fix
  sessions.

---

## 9. Emulator & app integration

- **Selection seam** (M6, per §10 decision 2): a `--disk-dsr <file>` flag
  mirroring `--system-rom`/`--system-grom`; `assets.rs` gains
  `DEFAULT_DISK_DSR` (our committed artifact) alongside the kept `Disk.Bin`
  in the ROMS table; `main.rs` wires the override; the system-information
  screen names the active DSR if cheap (polish, optional).
- **Write-back durability** (§10 decision 1): a *separate, small* emulator/
  app feature — flush mounted-image mutations back to the host `.Dsk`
  (explicitly, on unmount/exit, or write-through; design owned by whoever
  executes it). The DSR is correct without it (writes persist in-session and
  through save-states ✅); with it, writes survive restarts. Recommended to
  land alongside M3 so write testing exercises real user value.
- **Instruments stay diagnostics-only** — `Disk::record()` is already gated
  behind an explicit call; keep it that way.
- **No hardware changes required** otherwise: the DSR targets the card as it
  is. (If §10 decision 3 were ever answered "authentic Write Track," the
  card grows that command first — not the recommendation.)

---

## 10. Decision record — DECIDED / DEFERRED by Joel, 2026-07-06

Executing sessions: the **DECIDED** rows are settled — do not re-litigate. The
**DEFERRED** rows do not block their dependent milestone's *start*; each names
where it must be resolved before that milestone's write path lands.

| # | Decision | Verdict (Joel, 2026-07-06) | Affects |
|---|---|---|---|
| **1** | **Write durability** — should the emulator flush disk writes back to the host `.Dsk` file so they survive a restart? | **DEFERRED / TABLED.** The model (write-through / explicit-commit / session-only) isn't settled, and **it does not affect the DSR implementation** — the DSR's write behavior is correct regardless; durability is an emulator/app companion feature decided later. | nothing in the DSR; a future `disk.rs`/app slice only |
| **2** | **Selection & default** — add `--disk-dsr`, make our DSR the default? | **DECIDED: our DSR becomes the default _after_ it is complete and tested** (the M5 matrix green). Ship the `--disk-dsr` flag with the authentic `Disk.Bin` kept embedded and selectable for comparison / differential tests; until the flip, authentic stays the default. | M6 |
| **3** | **FORMAT scope** — stock single-density only, or extend to DD/80-track? | **DEFERRED.** Revisit before M3's FORMAT slice lands. D1 still pins the authentic FORMAT behavior; only the FORMAT *code* (chunk DSR-5) waits on this. M1–M4 are independent of it. | M3 / DSR-5 (the FORMAT slice only) |
| **4** | **Controller variant** — stock TI FD1771 card only? | **DECIDED: stock TI FD1771 card only for now.** Device names DSK1–3, subprograms `>10`–`>16`, the flat (no-subdirectory) file system; CorComp/Myarc/HFDC/TIPI extensions (incl. `>17+` subprograms and Myarc CPU-RAM buffering) are out of contract. | scoping (settled) |
| **5** | **Provenance string** — watermark ours or leave blank? | **DECIDED: no watermark** (the console-ROM precedent). Whatever TI text D1 finds is dropped; we add none — authorship lives in the repo, the license headers, and git history. | M1 strings |

Settled by architecture (not decisions, recorded for clarity): P8-DISK
relaxation (§2.5/§3); the VRAM-resident state model (§1); the FORMAT mechanism
substitution (§0 exception 2 — its final geometry scope rides on decision 3,
but the Write-Track-unavailable constraint holds regardless).

---

## 11. Risks & mitigations

1. **On-disk format byte-exactness** — the 3-byte cluster packing, the
   byte-swapped `>12` count, bitmap edge cases, VARIABLE's `>FF` terminator,
   FDIR sort order + bisection, the allocation policy (which determines
   *where* our writes land). → the three oracles (§4 P2); the fuzz hammers
   allocation/fragmentation; the bisection lockstep instrument.
2. **Skip-return vs. plain-return discipline** — backwards wedges the
   console (the documented ToD hang). → pinned in M1's first gate; the
   existing `device_io.rs` gates catch it instantly.
3. **The FORMAT oracle gap** — the authentic DSR's FORMAT can't run on our
   card (Write Track no-op), so FORMAT can't be differentially gated. → the
   structural validator + cross-oracle read-back are the gate; D1 probes and
   documents the authentic-DSR-under-our-emulator status in KNOWN-ISSUES.
4. **The VRAM buffer contract** — the header shape above `>8370` is checked
   by other software 📖, and the open-file table must keep working across
   interleaved calls; cross-DSR expectations (❓ D1) could extend the
   contract. → D1 pins the layout; concurrency gates in M4.
5. **The catalog's INTERNAL-format records** — radix-100 numbers inside
   file-like records; fiddly. → probe-derived spec (D1), byte-diff gates over
   all 15 bundled disks (M2).
6. **Scratchpad trampling** — the DSR shares GPLWS with the interpreter;
   one stray cell corrupts the console (the ROM track's alias bugs). → the
   owned-block discipline (§2.2) + a conformance check that non-owned
   scratchpad is bit-identical across the call.
7. **Ambiguity debt from the seed dossier** — Appendix A is
   Classic99-derived; Classic99 *emulates around* the real DSR in places
   (linear FDIR scan, hooked entry points). → P1: nothing implemented before
   D1 re-pins it on the binary; the flagged list (§A.7) is the worked
   checklist.
8. **Two console firmwares** — ours and TI's must both drive our DSR. → both
   already drive the genuine DSR through the same traced convention ✅; the
   matrix keeps both green continuously.

---

## 12. The `disk-dsr/` folder (target layout)

```
original-content/system-roms/disk-dsr/
├─ DSR-REWRITE-PLAN.md   this document (archive to ../history/ at M6)
├─ PROGRESS.md           execution ledger + resume point (live)
├─ README.md             front door: status, build/run, doc map (grows at M6)
├─ RECON.md              the DSR interface dossier (D1/D2; ✅-tagged facts)
├─ SURFACE-MAP.md        authentic-image classification + pinned-cell table
├─ disk-dsr.asm          the rewritten DSR source (TMS9900, our dialect)
└─ disk-dsr.bin          the committed 8 KiB artifact (build: libre99asm dsr)
```

## 13. Work packaging — hand-off chunks for executing sessions

**Ground rules (every chunk).** Read `../README.md` → `../STATUS.md` → this
plan → [`PROGRESS.md`](./PROGRESS.md) → `RECON.md` (once it exists) → your
chunk. Before debugging: `../GROM-DEBUGGING-GUIDE.md` + `../DEBUGGING.md`
(the method transfers; the instruments here are `Disk::record()` and the PAB
rig). Probes before fixes; a regression test per fix; consult-never-copy;
rebuild + commit `disk-dsr.bin` when source changes (M1+); update
RECON/PROGRESS/STATUS as you go; commit per completed increment to `main`
(check for sibling-session work first). Fast gate: `cargo test -p libre99-asm
-p libre99-core -p libre99-gpl`; deep gate where the chunk says so. **Do not edit
`../rom/**` or `../grom/**` from DSR sessions** beyond agreed
cross-references.

| Chunk | Contents | Exit criterion |
|---|---|---|
| **DSR-1** | **T1 toolchain + D1 static dossier** (pair well: `dis` output feeds both) | T1 gate green; tracer bullet boots; RECON/SURFACE-MAP committed, every ❓ resolved or deferred-with-rationale |
| **DSR-2** | **D2 dynamic census** (may fold into DSR-1) | census re-runnable + report committed |
| **DSR-3** | **M1 plumbing + sector primitive + the PAB rig** | M1 gates green both console ROMs |
| **DSR-4** | **M2 read side** | ToD + 15-disk catalog + golden reads green |
| **DSR-5** | **M3 write side + FORMAT** (needs §10 decision 3) | cross-oracle + validator + TI BASIC scripts green |
| **DSR-6** | **M4 completeness** | microsuite + completeness assertion green |
| **DSR-7** | **M5 hardening + M6 packaging** (needs §10 decisions 1–2) | deep tier green; definition-of-done checked; plan archived |

Dependencies: DSR-1 → DSR-3 → DSR-4 → DSR-5 → DSR-6 → DSR-7 strictly;
DSR-2 interleaves anywhere after DSR-1's instruments exist. DSR-4 and DSR-5
are the fat ones (the file system); everything else ≈ one session each at
the established track pace.

## 14. References

| Topic | Location |
|---|---|
| The emulated card (this plan §2.1's authority) | `crates/libre99-core/src/disk.rs` (whole file read during planning) |
| The console-side calling convention (§2.2's authority) | `original-content/system-roms/rom/console.asm:2937-3027` (SROM/SNAME); `../rom/RECON.md` §24, §26 addendum |
| Existing gates that become the matrix | `crates/libre99-core/tests/disk.rs`; `crates/libre99-gpl/tests/device_io.rs` |
| The DSR seam | `crates/libre99-core/src/machine.rs` `load_disk_controller`; `crates/libre99-app/src/assets.rs` |
| Toolchain (raw-image mode, `check_layout`, the `system_rom.rs` template) | `crates/libre99-asm/src/lib.rs:101-142,323-338,374-397,763-788`; `crates/libre99-asm/src/system_rom.rs` |
| Read/catalog corpus | `disks/*.Dsk` (15 images); `cartridges/tundoom.ctg` |
| The pre-public-release dependency this closes | `docs/DEVELOPMENT.md` §Pre-public-release item 3 |
| Classic99 (consult, never copy) — the seed dossier's sources | `C:\ClaudeShared\classic99\disk\{TICCDisk.cpp,diskclass.h,disk.cpp,ImageDisk.cpp,FiadDisk.cpp,diskclass.cpp}` |
| Literature 📖 | WD1771 datasheet (Type II/III command choreography); Nouspikel Tech Pages (disks.htm, fileformats.htm, headers.htm); TI's File Management Specification (via Classic99's citations) |
| House method | `../GROM-DEBUGGING-GUIDE.md`, `../DEBUGGING.md`, the archived plans in `../history/` |

---

# Appendix A — planning-research dossier (the D1 seed)

Collected 2026-07-06 from Classic99's disk subsystem and this repo's own
source. **Status: 📖 unless marked ✅.** Every 📖 row must be re-verified
against `roms/Disk.Bin` (disassembly and/or probe) before code relies on it
— record the verdicts in `RECON.md`, not here. Where Classic99 and the
binary disagree, the binary wins.

## A.1 The `>AA` peripheral header at `>4000`

| Offset | Field | Notes |
|---|---|---|
| `>4000` | `>AA` signature | checked by SROM against `@>000D` ✅ |
| `>4001` | version byte | |
| `>4002` | number of programs | `>00` for the disk card ⚠ unread by Classic99 |
| `>4003` | reserved | |
| `>4004` | **power-up chain head** | key `>04`; walked at every boot ✅ (our SROM) |
| `>4006` | program chain head | empty (`>0000`) for the disk card |
| `>4008` | **device (DSR) chain head** | key `>08` |
| `>400A` | **subprogram chain head** | key `>0A` |
| `>400C` | ISR chain head | ship `>0000` (stock card has no ISR; emulator raises no card ints ✅) |
| `>400E` | reserved | |

**Node format** (device + subprogram chains) ✅ (pinned from our own SROM
walk, which runs the genuine DSR): `[link word][entry word][name-length
byte][name bytes…]`; link `>0000` ends a chain. Power-up nodes: `[link]
[entry]`, no name ⚠ (Classic99 honors only the first power-up node — don't
chain ours). Authentic `Disk.Bin` reference points (that ROM's own layout,
NOT spec): power-up entry `>4070`; Classic99 hooks `>4676` (sector-op
return) and `>42A0` (error path).

## A.2 Subprograms (single-byte-name nodes on the `>400A` chain)

All entered with drive/unit in `>834C`; error code returned in `>8350`
(universal for subprograms). Numbers `>17+` are other controllers' — out of
contract (§10 decision 4).

| # | Name | Function | Parameter block 📖 |
|---|---|---|---|
| `>10` | SECTOR | absolute sector read/write | `>834C` drive; `>834D` 0=write/≠0=read; `>834E` VDP buffer; sector number: caller sets `>8350` (word), ROM works via `>834A` — **⚠ ambiguity, D1 settles by probe**; returns sector in `>834A`, error `>8350` |
| `>11` | FORMAT | initialize a disk | `>834D` tracks (35/40); `>834E` VDP buffer; `>8350` density; `>8351` sides; returns total sectors `>834A`, errors `>8350`+`>8351` |
| `>12` | PROTECT | file protect flag | `>834D` `>00` clear / `>FF` set; `>834E` VDP addr of 10-byte name |
| `>13` | RENAME | rename file | `>834E` VDP addr new name; `>8350` VDP addr old name |
| `>14` | FILEIN | read N sectors / file info | `>834D` sector count (0 ⇒ info request); `>834E` VDP name addr; `>8350` low byte of the scratchpad info-block addr (`>8300+x`) |
| `>15` | FILEOUT | write N sectors / create | mirror of FILEIN |
| `>16` | FILES | (re)allocate VDP buffers | `>834C` = n (1–9? bounds ❓); reshapes `>8370` + the buffer header |

**FILEIN/FILEOUT info block** (at `>8300 + [>8350]`): `+0` VDP data buffer
(word) · `+2` first sector (word; on info request returns the count) · `+4`
type/status flags · `+5` records/sector · `+6` EOF offset · `+7` record
length · `+8` #records (word).

**FILES(n) VRAM shape** 📖: new top-of-VRAM = `>3DEF − (256+256+6)·n − 6`
(= `>37D7` at n=3 ✅ gate-verified); immediately above it the 6-byte
header `>AA / >3F / >FF / >11 / n / …` (valid marker, top-of-VRAM word,
CRU-high, count) — other software (P-Code) checks this shape. n=0 = "CS1
mode": top `>3FFF`, no buffers.

## A.3 The PAB and the file-level opcodes

**PAB layout** (VDP RAM, base = `[>8356] − namelen − 10`) 📖:

```
+0 opcode        +1 flag/status   +2..3 data buffer (VDP addr)
+4 record length +5 char count    +6..7 record number (FIXED only)
+8 screen offset / STATUS result  +9 name length   +10… "DSKn.NAME"
```

Flag byte: `>10` VARIABLE · `>08` INTERNAL · `>01` RELATIVE · mode bits 1–2:
`>00` UPDATE / `>02` OUTPUT / `>04` INPUT / `>06` APPEND. High-ASCII (bit-7)
bytes in names act as `.` separators ⚠.

**Opcodes** 📖: 0 OPEN (verify vs FDR, create on OUTPUT, implicit rewind,
fill PAB+4 if caller passed 0) · 1 CLOSE (flush + release) · 2 READ
(char-count → PAB+5; relative via PAB+6/7) · 3 WRITE · 4 RESTORE (rewind /
seek record) · 5 LOAD (program image → VDP, PAB+6/7 = max length) · 6 SAVE
(VDP → program image, PAB+6/7 = length) · 7 DELETE · 8 SCRATCH (delete
record, relative files) ⚠ unimplemented in Classic99's image path — D1 pins
from the binary · 9 STATUS (result byte → PAB+8).

**Error codes** (PAB+1 bits 5–7; DSR ORs `code<<5`): 0 none/bad-device ·
1 write-protected · 2 bad open attribute · 3 illegal operation · 4 buffer/
table/disk full · 5 past EOF · 6 device error · 7 file error. **STATUS
result bits**: `>80` no file · `>40` protected · `>10` internal · `>08`
program · `>04` variable · `>02` disk full ⚠ · `>01` EOF.

**Device names**: `DSK1/DSK2/DSK3` (digit selects the drive), `DSK` +
volume-name form (match against the VIB name across mounted drives).
Catalog: OPEN `"DSKn."` INT/FIX 38 — record 0 = volume (name + sizes),
then per-file records (name, type ±(1..5, negative = protected), sectors,
record length) with numbers in radix-100 FP ❓ exact layout by probe.

## A.4 The on-disk format

**VIB (sector 0)** — geometry fields ✅ (our `disk.rs:131-173`, verified
against Classic99), the rest 📖:

```
>00 volume name (10, space-padded)   >0A total sectors (BE word)
>0C sectors/track                    >0D "DSK" marker
>10 protection (' '/'P')             >11 tracks/side
>12 sides                            >13 density (1 SD / 2 DD)
>14–>37 reserved (zero on stock TI)  >38–>FF allocation bitmap
```

Bitmap: bit for sector *s* = `byte[>38 + s/8] >> (s%8) & 1`; sectors 0–1
always allocated; stock TI uses `>38–>EB` (1440 bits); free-sector search
starts at sector `>22` (34) 📖 ❓ (pin the authentic policy — it determines
write placement).

**FDIR (sector 1)**: ≤127 BE words → FDR sectors, **sorted ascending by
name**, `>0000`-terminated. **The authentic DSR bisects** (`p = p1 +
(((p2−p1)/2) & ~1)` shape) 📖⚠ — Classic99 linear-scans instead; reproduce
the bisection (P3) and lockstep-gate its FDR read order.

**FDR**:

```
>00 name (10)      >0A reserved/ext-reclen (0 stock)   >0C flags
>0D recs/sector    >0E sectors allocated (BE)          >10 EOF offset
>11 record length  >12 #records (LITTLE-ENDIAN swap!)  >14–>1B reserved
>1C–>FF cluster chain (3-byte entries, >000000-terminated)
```

Flags `>0C`: bit0 PROGRAM · bit1 INTERNAL · bit3 PROTECTED · bit7 VARIABLE.
`>12` counts records (FIXED) / **sectors** (VARIABLE) / 0 (PROGRAM).

**Cluster packing** (both directions):

```
byte0 = start[7:0]
byte1 = start[11:8] | (end_offset[3:0] << 4)
byte2 = end_offset[11:4]
```

`start` = 12-bit absolute first sector of a contiguous run; `end_offset` =
12-bit file-relative index of the run's **last** sector (0-based,
cumulative across the chain).

**Record packing**: FIXED — `256/reclen` per sector, no spanning, tail
wasted; VARIABLE — `[len][data]…`, a `>FF` length byte ends the sector's
records, zero-pad, `recs/sector = 256/(reclen+1)`; PROGRAM — raw bytes
across the chain, EOF-offset governs the last sector (0 ⇒ full 256).

## A.5 FD1771 choreography (per operation)

✅ for what our card implements (§2.1); 📖 (datasheet) for the authentic
sequence shape: select drive/side via CRU → seek (Restore `>0x` / Seek
`>1x` with target in Data / Steps) → optionally Read Address `>C0` to
verify the track → set Sector register → Read `>8x` / Write `>Ax` → move
256 bytes through Data, polling status (DRQ/BUSY; on our synchronous card
the poll always succeeds) → check final status → map errors
(RECORD_NOT_FOUND → 6/7-class, WRITE_PROTECTED → 1, NOT_READY → 6). All
register bytes inverted (`XOR >FF`). FORMAT: authentic uses Write Track
`>F0` per track 📖 — **substituted** on our card (§0 exception 2). Motor
(`bit 1` strobe) and head-load are no-ops here; timing loops may be
dropped (§2.5).

## A.6 Scratchpad cells (the DSR-visible set)

| Cell | Role |
|---|---|
| `>834A–8351` | subprogram parameter block / staged search name ✅(name)📖(params) |
| `>834C` | drive/unit (subprograms); bit 7 = Myarc CPU-buffer — out of contract |
| `>8350` | subprogram error return; SECTOR input word ⚠ |
| `>8354/>8355` | word: (high) …/(low) **search-name length** ✅; return side-effect → PAB addr 📖❓ |
| `>8356` | VDP addr past the device name (PAB handle) ✅; return side-effect → FDR copy 📖❓ |
| `>836D` | the chain key: `>04/>06/>08/>0A` ✅ |
| `>8370` | top of free VRAM — power-up lowers to `>37D7` ✅ |
| `>83D0/>83D2` | SROM walk cursors (card base / node) ✅ |
| `>834A–>836D`, `>83DA–>83DF` | the DSR-owned workspace block 📖 (File Mgmt Spec via Classic99) |
| `>83E0–>83FF` | GPLWS — the registers themselves; honor the §2.2 invariants ✅ |

## A.7 The flagged-ambiguity worklist for D1

1. SECTOR's sector-number cell (`>8350` in vs `>834A` working/out) — probe.
2. The FDIR bisection — confirm + pin the probe order.
3. Return side-effects `>8354`→PAB / `>8356`→FDR-copy (first byte zeroed) —
   verify on the binary; decide observable-contract status.
4. The catalog record layout (INTERNAL-format fields) — probe via TI BASIC.
5. Every CRU bit the DSR actually *reads* (our card answers only 0 and 7).
6. FORMAT's mechanism + the authentic-FORMAT-under-our-emulator probe
   (expected broken today → KNOWN-ISSUES note for the authentic config).
7. The VRAM buffer/open-file-table layout — the persistent-state contract.
8. The allocation policy (first-free from `>22`? exact scan shape).
9. FILES(n) bounds (1–9? 16?) and the exact header/top math at each n.
10. SCRATCH (opcode 8) semantics — Classic99 stubs it; the binary decides.
11. Strings in the ROM (decision 5 input).
12. `>8359` version-cell / any DSR-side scratch the console expects ❓.
