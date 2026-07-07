# Appendix H — DSR and PAB Reference

<!-- Appendices · target ≈6 pp · companion to Ch. 30–31 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The >AA peripheral-card header (H.2), the device/subprogram chain node format and device names (H.3), the PAB field layout (H.4), the opcode table (H.5), and the calling convention (H.7) are tier-1 for the project's machine: read directly from the clean-room disk DSR original-content/system-roms/disk-dsr/disk-dsr.asm (the HDR block at >4000, the DEVCH/SUBCH chains, the PAB copy loop's scratch-cell names OPC/FLG/BUF/RCL/CNT/REC, the OPTAB dispatch, the DEVENT entry contract), which is itself differentially gated against the authentic TI Disk Controller DSR as oracle (RECON). The PAB flag-byte fields are tier-1 where verified in the handlers (mode = bits 1-2 >06; VARIABLE = bit 4 >10; error = bits 5-7 >E0) and tier-2 convention for the data-type bit (3). The error-code table (H.6) is the standard TI file-I/O convention (tier-2), annotated with the codes this DSR actually emits (tier-1). Device-name/option matter beyond DSK1-3 is tier-2, hedged (R-2). -->

A **DSR** — Device Service Routine — is the firmware a peripheral card carries in
its own ROM to make itself a named device: `DSK1`, `RS232`, `PIO`. The console
knows nothing about disks; it knows only how to hand a request to whatever card
claims the device name, through a shared data structure in VDP RAM called the
**PAB** — Peripheral Access Block. This appendix is the two cards: the DSR
header/chain structures a card exposes, and the PAB layout a caller fills in. The
teaching — how `DSRLNK` finds the card, how the file system is built on top — is
Chapters 30–32, and the project runs a full clean-room disk DSR whose source is
the tier-1 authority for everything here.

## H.1 The model

A peripheral card's ROM is paged into the CPU window **`>4000`–`>5FFF`** by
setting **CRU bit 0 at the card's base** (`SBO 0`, Appendix G); paged out by
clearing it. Only one card's ROM is visible at a time, so the console's `DSRLNK`
walks the cards — page each in, look for the device name in its chains, call the
matching routine, page out — until one handles the request or the list ends
(Ch. 30). Everything below lives inside that `>4000` window while the card is
selected.

## H.2 The `>AA` peripheral-card header (`>4000`)

Every DSR ROM opens with a fixed header the console recognizes. Tier-1, the `HDR`
block of the project's disk DSR (byte-identical in shape to the authentic card):

```text
>4000  BYTE >AA          valid-DSR marker (the console checks this first)
>4001  BYTE >02          version
>4002  BYTE >00          number of programs
>4003  BYTE >00          reserved
>4004  DATA powerup      power-up routine chain
>4006  DATA >0000        program chain (menu entries)
>4008  DATA device       device (DSR) chain  — the named devices
>400A  DATA subprog      subprogram chain    — the CALLable subprograms
>400C  DATA >0000        interrupt chain
>400E  DATA >0000        reserved
```

The `>AA` byte is the gate: no `>AA`, and the console skips the card entirely. The
four chain pointers each head a singly linked list (H.3); a `>0000` pointer means
"this card has none of that kind."

## H.3 The chains — device and subprogram nodes

Each chain is a linked list of nodes. A node is **`link, entry, name`**: a pointer
to the next node (`>0000` ends the chain), the address to `BL` into, and a
length-prefixed name or a one-byte subprogram code. Tier-1 (the `DEVCH`/`SUBCH`
blocks):

```text
device node:      DATA next       subprogram node:   DATA next
                  DATA entry                          DATA entry
                  BYTE namelen                        BYTE 1
                  TEXT 'DSK1'                          BYTE >10      (or a name)
```

The project disk DSR's **device chain** is `DSK` → `DSK1` → `DSK2` → `DSK3`
(the bare `DSK` is the volume-name form, `DSK.VOLUME.FILE`), and its
**subprogram chain** carries the standard disk subprograms:

| Code | Subprogram | Purpose |
|---|---|---|
| `>10` | `SECTOR` | absolute sector read/write |
| `>11` | `FORMAT` | initialize a disk (single-density subset) |
| `>12` | `PROTECT` | set/clear a file's protection |
| `>13` | `RENAME` | rename a file |
| `>14` | `FILEIN` | — |
| `>15` | `FILEOUT` | — |
| `>16` | `FILES` | set the number of file buffers |
| (name) | `FILES` | BASIC's named `CALL FILES(n)` |

Standard **device names** a caller uses: `DSK1.`, `DSK2.`, `DSK3.` (drive-numbered)
and `DSK.diskname.` (volume-named); a bare `DSKn.` with no file name is the
**catalog** form (H.5, opcode OPEN). Other cards define their own names — `RS232`,
`PIO`, `CS1`/`CS2` — with device-specific options (baud, data bits) appended after
a period; those are tier-2 card-manual matter (Ch. 33) and not emulated at HEAD
(R-12).

## H.4 The PAB layout card

A PAB is a block in **VDP RAM** the caller builds and points the DSR at. The
fields, tier-1 from the DSR's PAB copy loop (the scratch-cell names are this DSR's;
the offsets are the standard PAB):

| Offset | Field | Meaning |
|---|---|---|
| **+0** | opcode | the operation (H.5) |
| **+1** | flag / status | mode + file type in; **error code out** (H.6) |
| **+2,+3** | buffer address | VDP address of the caller's data buffer |
| **+4** | record length | logical record length (bytes) |
| **+5** | character count | bytes to transfer; **bytes actually read** out |
| **+6,+7** | record number | for RELATIVE (fixed) files |
| **+8** | screen offset | (BASIC's screen bias; `>8356` side-effect target) |
| **+9** | name length | length of the device.file name that follows |
| **+10…** | name | the ASCII `DEVICE.FILENAME` |

### The flag / status byte (+1)

One byte carries the open mode and file type in, and the error code out
(tier-1 where marked, else tier-2 convention):

| Bits | Mask | Field | Values |
|---|---|---|---|
| 7–5 | `>E0` | **error code** (out) | 0–7 (H.6) — *tier-1* |
| 4 | `>10` | record type | 0 = FIXED, 1 = VARIABLE — *tier-1* |
| 3 | `>08` | data type | 0 = DISPLAY, 1 = INTERNAL — *tier-2* |
| 2–1 | `>06` | I/O mode | `>00` UPDATE, `>02` OUTPUT, `>04` INPUT, `>06` APPEND — *tier-1* |
| 0 | `>01` | (reserved) | — |

The DSR **OR**s the error code into bits 5–7 on return and leaves the caller's mode
and type bits intact, so the flag byte after a call reads back your request with
the error stamped on top — test `>E0` for success (Ch. 31).

## H.5 The opcode matrix

The opcode at PAB+0 selects the operation; the disk DSR dispatches through its
`OPTAB` (tier-1):

| Opcode | Operation | Effect |
|---|---|---|
| **0** | OPEN | open a file (or, empty name, the `DSKn.` **catalog**) |
| **1** | CLOSE | close and flush |
| **2** | READ | read one record into the buffer |
| **3** | WRITE | write one record from the buffer |
| **4** | RESTORE | reposition to a record (or the start) |
| **5** | LOAD | load a whole PROGRAM-image file to memory |
| **6** | SAVE | save a memory image as a PROGRAM file |
| **7** | DELETE | delete the file |
| **8** | SCRATCH | scratch a record (RELATIVE) |
| **9** | STATUS | return the file's status byte (no I/O) |

The project disk DSR implements the full set — OPEN in every mode (with
create/truncate/append), CLOSE, READ and WRITE for both FIXED and VARIABLE
records, RESTORE, LOAD, SAVE, DELETE, STATUS, and the catalog — with two authentic
edges (tier-1): **SCRATCH** returns the authentic **error 6** (the stock disk
controller does not implement record scratch), and **FORMAT** is the
single-density subset. STATUS builds the standard file-status byte (exists, EOF,
PROGRAM, INTERNAL, VARIABLE, protected) from the file's directory entry.

## H.6 The error matrix

On return, the DSR sets a 3-bit error code in **bits 5–7 of PAB+1** (value ×32).
The codes are the standard TI file-I/O convention (tier-2); the right-hand column
notes when the project disk DSR emits each (tier-1):

| Code | Meaning | The disk DSR emits it when… |
|---|---|---|
| **0** | no error / bad device name | — (an unknown device never reaches this card) |
| **1** | device write-protected | a write mode opens a **protected** file |
| **2** | bad open attribute / file-type mismatch | INPUT on a missing file; record-opening a PROGRAM file; type mismatch |
| **3** | illegal operation | an opcode the device does not support |
| **4** | out of buffer / table space | no free file buffer; `FILES(n)` out of range |
| **5** | past end of file | reading past EOF; catalog index beyond the directory |
| **6** | device (hardware) error | a sector read/write fails; an unreadable drive; **SCRATCH** |
| **7** | file error | unknown volume name; file not found on some paths |

Error 6 is "the disk didn't respond as expected"; error 7 is "the disk is fine but
the file/volume isn't there." Chapter 31's `filelib` decodes these into messages;
Chapter 32's `DISKDOC` reads the directory structures behind them.

## H.7 The calling convention

A DSR is entered by the console's `DSRLNK` with the card selected and the request
staged (tier-1, the DSR's `DEVENT` contract; RECON §1):

- **Entry:** `BL *R9` on the GPL workspace **`>83E0`**, with **`R12`** = the card
  CRU base (ROM enabled), `R13`/`R15` = the GROM/VDP ports, `LIMI 0`.
- **Inputs:** `>8355` = device-name length; `>834A…` = the staged device name;
  `>8356` = the VDP address of the character *past* the device name in the PAB
  (from which the DSR computes the PAB base = `>8356` − namelen − 10).
- **Return:** a handled request returns to **`R11+2`** (the skip return that tells
  `DSRLNK` "I took it"); an unhandled power-up node returns plainly so the scan
  continues. The disk DSR preserves `R1` and `R11`–`R15`; `R0`, `R2`–`R10` are
  scratch.

**Project scope (R-12).** The bench runs the clean-room **disk** DSR — canon, and
the tier-1 source here — over embedded disk images (DSK1–3). It is a genuine
FD1771 driver plus the full PAB file system above; it is not the stock TI byte
image but a differentially-verified reimplementation (RECON, gated against the
authentic DSR). RS-232/PIO and cassette DSRs are **not** emulated at HEAD
(Ch. 33); code for those on the shelf tools or real hardware. When you need a card
whose DSR the project does not ship, `NULCARD` (Ch. 30) is the template for
writing one.

*See also:* Chapter 30 (`dsrlib`, `DSRLNK`, and `NULCARD`), Chapter 31
(`filelib` and FILER99 — the PAB file system in use), Chapter 32 (`seclib`,
DISKDOC, and the on-disk structures — VIB/FDR, Appendix I), Appendix G (CRU card
bases and DSR paging), Appendix C (the scratchpad cells a DSR is handed),
Appendix I (the disk media and file formats the DSR reads).
