# Chapter 31 — File I/O: PABs and the Filesystem Contract

*Every file operation on the TI — open, read, write, close, catalog, load — is one block of bytes handed to a device. Learn the block, and you can read and write any file the machine ever stored.*

<!-- Part VII — Storage and Peripherals · target ≈22 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The PAB/file-I/O contract is machine-verified via the Rust harness (device_io.rs GREEN at HEAD: a real PAB drives Tunnels of Doom's QUEST scenario load off a real disk image), NOT BENCH99 (no disk — a stated R-12 gap). The PAB layout, opcodes, and error codes are probe-pinned in original-content/system-roms/disk-dsr/RECON.md §3. A real DIS/VAR 80 file was decoded live from disks/TI-Write.Dsk: FORMATDOC (flags >80 = VARIABLE, reclen >50 = 80, 25 records/25 sectors, cluster @ sector 117, first record [len=27]"     USING THE TEXT EDITOR"). Code: code/ch31/filelib.inc (PAB layout + OPEN/READ/WRITE/CLOSE wrappers, assembles; libre99asm). Depends on Ch. 30's dsrlib (DSRLNK). -->

## One Block, Every File

In most systems a file handle is an opaque token — you call `open()`, get back a number, and the operating system hides everything behind it. The TI does the opposite, and it is bracingly honest about it: a file operation is a **block of bytes you fill in yourself**, laid out in the console's memory, describing exactly what you want done — the operation, the file, the buffer, the record — which you then hand to the device that owns the file. There is no hidden handle, no kernel table; there is a **Peripheral Access Block**, a PAB, and it is all there in the open, every field visible and yours to set. Learn the PAB and you have not learned "the disk API" — you have learned *the* file API, the single contract by which the TI talks to disks, cassettes, serial ports, printers, and every modern storage device that came after, because they all speak PAB.

This is the payoff of Chapter 30. There we learned how a device *introduces* itself (the DSR) and how to *reach* it (`DSRLNK`); here we learn what to *say* once reached — and the answer is always a PAB. Open a file: a PAB. Read a record: a PAB. Catalog a disk, load a program, delete a file: a PAB, a PAB, a PAB, differing only in one opcode byte and a few fields. That uniformity is the elegance — one data structure for all I/O to all devices — and it means the file-handling code you write in this chapter (`filelib`) works unchanged whether the file lives on a 1983 floppy, a modern flash device, or a Raspberry Pi across the room (Chapter 34), because all of them answer the same block of bytes.

The ground, stated once (R-12): the project emulates the disk controller fully, and the PAB path is machine-verified against it — a real PAB, built the way this chapter builds one, drives *Tunnels of Doom* to load its game scenario off a real disk image, green in the test harness at HEAD. That verification runs through the Rust harness and the desktop app (`--disk`), not BENCH99 (which has no disk — a stated gap), and the PAB layout and error codes below are probe-pinned in the project's reconstruction. So the contract is not described from a manual alone; it is the contract the emulator's disk DSR actually honors, and one whose files we will decode byte-by-byte off a real disk before the chapter ends.

---

## What You Will Learn

- The **PAB**: every field — opcode, flags, buffer pointer, record length, count, record number, status, name — and how to lay it out.
- The **opcode set** in practice: OPEN, CLOSE, READ, WRITE, RESTORE, LOAD, SAVE, DELETE, STATUS — call sequences and error codes.
- The **file types** that matter: DIS/VAR 80 (text), DIS/FIX 80 (object), INTERNAL, PROGRAM — and interchange with modern machines.
- **VRAM etiquette** for I/O: where the PAB and buffers live, the name-pointer dance, coexisting with your screen.
- **`filelib`**: open/read/write/close wrappers with usable error surfaces — a config loader and a high-score writer.
- **Catalog reading** and **LOAD/SAVE** of program images from your own code.

## The Bridge: `struct` I/O and the `ioctl` Ancestor

A modern programmer meets this pattern more often than they think. Every time you fill in a `struct` and pass its pointer to a system call — `stat(path, &statbuf)`, an `ioctl(fd, cmd, &arg)`, a `DeviceIoControl` on Windows, a `setsockopt` with an option block — you are doing exactly what the PAB does: describing an operation in a structured block of memory and handing it to a driver. The `ioctl` interface in particular is the PAB's direct descendant in spirit: a single call that does *many* different device operations, selected by a command code, with a structure carrying the parameters. You do not have a separate system call for every device feature; you have one `ioctl` and a struct, and the driver interprets it.

The PAB is that idea as the machine's *primary* file interface, not a side channel. Where Unix eventually hid files behind `open`/`read`/`write` and pushed the struct-passing to the odd corners (`ioctl`, `fcntl`), the TI put the struct front and center: *all* file I/O is "fill in the block, call the device." Studying it is studying the structured-I/O pattern in its purest, most central form — and it demystifies `ioctl` forever, because once you have hand-built a PAB and watched a device read it, the idea of "a struct is the API" is not strange, it is obvious. The TI just never pretended otherwise.

## 31.1 The Peripheral Access Block

The PAB is a small block of bytes — living in **VRAM** (§31.4 explains why) — with fixed fields, laid out in `filelib`:

```text
PAB+0  opcode        what to do: OPEN=0, CLOSE=1, READ=2, WRITE=3, ...
PAB+1  flags         file type & mode; the DSR ORs an ERROR code into bits 5-7
PAB+2  buffer (word) VRAM address of the data buffer (where records land / come from)
PAB+4  record length logical record length (e.g. 80 for DIS/VAR 80)
PAB+5  char count    bytes actually read (after READ) or to write (before WRITE)
PAB+6  record number word; the record to access for relative (FIXED) files
PAB+8  screen offset unused for disk
PAB+9  name length   length of the device+file name that follows
PAB+10 name          the ASCII name: "DSK1.MYFILE"
```

Every field is yours to set, and each does one plain thing. The **opcode** (`PAB+0`) selects the operation. The **flags** (`PAB+1`) carry the file type and access mode going in (variable vs. fixed records, display vs. internal data, input/output/update/append) and, coming out, the **error code** — which the DSR ORs into the top three bits (5–7), where it is *sticky*: the DSR never clears it, so you check and clear it yourself (§31.2). The **buffer pointer** (`PAB+2`) names a VRAM region where a read deposits its record or a write draws its data. The **record length** and **count** meter the data; the **record number** addresses a specific record in a relative file; and the **name** (`PAB+9`/`PAB+10`) is the `DSK1.FILENAME` string the DSR matches (Chapter 30). To do a file operation you fill in these fields and hand the PAB's address to the device via `DSRLNK` — and that is the whole interface. `filelib` gives the fields names (`POPCOD`, `PFLAG`, `PBUF`, `PRECLN`, `PCOUNT`, …) so your code reads as intent rather than offsets, and a builder that stamps a PAB into VRAM from a few registers.

## 31.2 The Opcode Set in Practice

Ten opcodes cover all of file I/O, and they group naturally. The **record operations** are the daily bread: **OPEN** (`0`) associates the PAB with a file and mode (a file must be opened before reading or writing); **READ** (`2`) fetches the next record into the buffer and sets `PAB+5` to its length; **WRITE** (`3`) stores `PAB+5` bytes from the buffer as the next record; **CLOSE** (`1`) finishes and flushes. A typical text read is OPEN-for-input, then READ in a loop until end-of-file, then CLOSE — four opcodes, one PAB reused. **RESTORE** (`4`) rewinds to the start or to a record. The **whole-file operations** skip the open/read/close cycle: **LOAD** (`5`) copies a PROGRAM-image file straight into VRAM at the buffer address (§31.7), **SAVE** (`6`) writes one back, **DELETE** (`7`) removes a file, and **STATUS** (`9`) reports whether a file exists and its attributes (into `PAB+8`).

The **error codes** are where careful code earns its keep, and the project's probe-pinning makes them concrete (`RECON.md`). Errors arrive in `PAB+1` bits 5–7 and are **sticky** — the DSR never clears them, so a stale error from a previous op will fool you if you do not clear the field before each call and check it after. The codes you meet most: **error 2** — file/opcode mismatch, raised by OPEN on a missing file, by a record-length mismatch, or by asking for FIXED records on a VARIABLE file; **error 5** — past end-of-file, the natural end of a READ loop (READ one record past the last VARIABLE record returns error 5, which is how you know you are done); **error 6** — a rejected operation, which the disk DSR returns for SCRATCH (record-level delete, unsupported). A robust reader treats error 5 as "done, not failed," error 2 as "no such file, tell the user," and any other as a real fault — the error surface `filelib` exposes so a game can actually respond (§31.5). The full opcode/error matrix, with every code and the state each leaves the PAB in, is App. H; the working subset is these, and the project's `device_io.rs` exercises them through a real load (and confirms a bad device errors gracefully rather than hanging — the "error 2, told cleanly" path).

## 31.3 File Types That Matter

A TI file's *type* is the shape of its records, and four types carry essentially everything. **DISPLAY/VARIABLE 80** — "DIS/VAR 80," or DV80 — is the **text lingua franca**: display (ASCII) characters, variable-length records up to 80 bytes, and it is what nearly every text file, source listing, and document is. Its on-disk form is beautifully simple, and we will read one live (§Field Notes): each record is a length byte followed by that many characters, packed into sectors. **DISPLAY/FIXED 80** — DF80 — is the same 80-byte width but *fixed*-length records, the format of assembler **object code** (Chapter 6's tagged object files are DF80), so a loader reads object with the same READ loop as text. **INTERNAL** types (INT/VAR, INT/FIX) store data in the machine's own binary/radix-100 form rather than ASCII — compact and exact for numbers (a program's saved data, a catalog), unreadable as text. And **PROGRAM** images are unstructured byte streams — a memory image, no records — loaded and saved whole (§31.7): TI BASIC programs, memory-image games (Chapter 6's Option 5), and the game scenario *Tunnels of Doom* loads (the verified path) are PROGRAM files.

Knowing the type tells you how to read a file and how to move it between machines. **Interchange** with a modern computer is routine because the types are documented and tools speak them: `xdm99` (the xdt99 suite) extracts and inserts files into `.dsk` images by type, TIPI (Chapter 34) maps them to host files, and a DV80 file becomes a plain text file on your laptop (strip the length bytes) and back. So a level file you design as DV80, or a high-score table as INT/FIX, is not trapped on 1982 media — it round-trips to 2026 and back through documented formats. The type is the contract for interchange as much as for the READ loop, and choosing the right one (DV80 for anything a human or a tool might read; INTERNAL for compact machine data; PROGRAM for images) is a real design decision (Chapter 36).

## 31.4 VRAM Etiquette for I/O

Here is a fact that surprises every newcomer: the PAB and the file buffers live in **VRAM**, not CPU RAM. File I/O happens *behind the video chip*. The reason is the same one that put TI BASIC's variables there (Chapter 28) — VRAM's 16 KiB is where the free memory was, the scratchpad and console RAM being tiny and spoken for — and the DSR was written to read its PAB and move its data through the VDP ports (Chapter 12). So to do file I/O you carve out a region of VRAM for the PAB and its buffers, build the PAB there with VDP writes (`filelib` uses `vdplib`'s `VSBW`/`VMBW`), and the DSR reads it and deposits records there through the same ports.

This creates an **etiquette** with your screen, because your screen is *also* in VRAM. The name table, pattern table, and sprite tables (Part III) occupy the low VRAM; your PAB and file buffers must live somewhere *else* — the free VRAM above your graphics data — or a file read will scribble records over your name table and corrupt the display. So VRAM I/O and VRAM graphics must be laid out to coexist: reserve a buffer region clear of the screen tables (Chapter 5's VRAM map is the budget), and know that the console's own DSRs reserve VRAM at power-up for exactly this (Chapter 30's power-up node lowering `>8370`, the top-of-VRAM pointer — verified). There is also a small **name-pointer dance** the DSRLNK path performs — staging the device name and a pointer to it (`>8356`, the VDP address past the device name; the PAB base is computed back from it as `[>8356] − devlen − 10`, probe-pinned) — so the DSR can find both the name and the PAB from one pointer. `filelib` and `dsrlib` handle the dance; the etiquette you must keep is the layout one: your files and your screen share 16 KiB, so budget it, or they collide.

## 31.5 `filelib`: File I/O a Game Can Use

`filelib` wraps the PAB mechanics into operations with usable error surfaces, in `code/ch31/`. It lays out the PAB fields (§31.1), names the opcodes, and provides the wrappers:

```asm
FOPEN  LI  R1,OPOPEN         \
FREAD  LI  R1,OPREAD          |  each loads its opcode and falls into FDOP,
FWRITE LI  R1,OPWRIT         |   which stamps PAB+0 and links to the device
FCLOSE LI  R1,OPCLOS         /   (DSRLNK, Ch. 30), returning the error in PAB+1
```

`FDOP` writes the opcode into the PAB (`VSBW`) and calls `DSRLNK`; the wrappers just preset the opcode, so `BL @FOPEN` / `BL @FREAD` / `BL @FCLOSE` reads like the file operations it performs. Around these, the useful `filelib` clients are two a game actually needs. A **text-file reader** — a config or level loader — opens a DV80 file for input, READs records into a buffer in a loop until error 5 (end-of-file), and hands each record to the caller; it is the way a game loads a level designed as text (Chapter 36) or reads a config file a human can edit. A **text-file writer** — high-score persistence — opens a DV80 file for output, WRITEs the score records, and closes; it is how a game *remembers* between sessions. Both surface errors usefully: "no disk / no file" (error 2) becomes a message, not a hang; end-of-file (error 5) is the loop's natural exit, not an error shown to the user. This is the honest note (R-12): `filelib` **assembles** and follows the probe-pinned contract, and the *contract it speaks* is the one `device_io.rs` verifies against the real disk DSR (a real PAB, a real load), but `filelib` itself is exercised through the desktop app and hardware, not a bench transcript — the disk is not on the bench. What it gives you is real regardless: file loading and saving your game can call, with errors it can handle.

## 31.6 Reading a Catalog

A file-picker UI — "which file shall I load?" — needs to *enumerate* a disk, and the TI's way is a small, elegant trick: **the catalog is itself a file**. Open the device name with a trailing dot and nothing after — `"DSK1."` — as an **INTERNAL/FIXED 38** file for input, and READ it record by record: **record 0** is the *volume record* (the disk's name and size), and each subsequent record describes one file (its name, type, size, and record length) in the disk's directory, until end-of-file. So cataloging a disk is just opening `DSK1.` and running a READ loop — the same OPEN/READ/CLOSE you already know — and parsing each INT/FIX 38 record into a file entry for your picker. The project's reconstruction pins this exactly (`RECON.md`): the `DSK1.` catalog is INT/FIX 38, record-number-driven, record 0 the volume record. It is a lovely piece of design — enumeration reuses the file-read machinery, no special "list directory" call — and it means a file-picker (the lab's `FILER99`) is built entirely from `filelib`: open the catalog, read the entries, show them with `textlib40` (Chapter 14), and open the chosen one. The directory is a file; reading it is reading a file; the machine has no other list-files primitive, and needs none.

## 31.7 LOAD and SAVE of Program Images

Not everything is records. A **PROGRAM image** — a raw memory image, no record structure — is loaded and saved whole, with two opcodes that skip the open/read/close cycle entirely. **LOAD** (`5`) points the PAB's buffer field at a VRAM destination and names a PROGRAM file, and the DSR streams the whole file into VRAM in one operation — no OPEN, no per-record READ, just "copy this file into memory" (the probe-pinned behavior: LOAD copies the PROGRAM image to VRAM at `PAB+2`, without even reading the volume info). **SAVE** (`6`) does the reverse, writing a VRAM region out as a PROGRAM file. This is how you write a **loader** — the equivalent of E/A's Option 5, "load and run a memory-image program" (Chapter 6): LOAD the image to VRAM (or, with the buffer aimed appropriately and a copy, into CPU RAM/expansion), then jump to it. It is how *Tunnels of Doom* loads its game code and its scenario (the verified path — a PROGRAM-image load driving the real DSR), how a multi-load game pulls in its next phase (Chapter 36's overlays), and how your own loader brings a memory-image program to life from your own code. Two opcodes, whole files, no records: the PROGRAM path is the simplest file I/O of all, and the one that loads the biggest things.

## Lab 31 — `FILER99`

The lab is the chapter's most reusable artifact: a **two-pane file manager**, `FILER99`, built entirely on `filelib` and `textlib40` (Chapter 14's 40-column text). It **catalogs** a disk (§31.6) into a scrollable pane, lets you **view** a DV80 file (open, READ-loop, display each record), and **copies** and **deletes** files (READ from one PAB, WRITE to another; DELETE). It is a real tool — the thing you reach for to manage disks — and it is nothing but the chapter's pieces composed: the catalog read, the DV80 reader, the write and delete opcodes, the 40-column UI. Building it proves you have the file contract in hand, because a file manager exercises *all* of it. The honest note (R-12) holds: `FILER99` assembles and speaks the verified contract; you run it against real disks through the desktop app (`--disk`) or hardware, where the disk DSR lives, not on the bench. What you carry away is a file manager you wrote from the block up.

> **Field Notes — Reading a TI-Writer document, byte by byte.** Take a real disk — `TI-Write.Dsk`, the TI-Writer word processor — and read one of its documents with nothing but a hex dump, and the DV80 format lays itself bare. Its directory holds a file `FORMATDOC`, and its file descriptor (Chapter 32's FDR) declares it: flags `>80` (bit 7 set = **VARIABLE** records), record length `>50` (**80**), 25 records in 25 sectors, its data beginning at sector 117. Jump to sector 117 and read the bytes:
>
> ```text
> 1B 20 20 20 20 20 55 53 49 4E 47 20 54 48 45 20 54 45 58 54 20 45 44 49 54 4F 52 ...
> ^len=27  "     USING THE TEXT EDITOR"
> ```
>
> The first byte, `>1B` = 27, is a **record length**; the next 27 bytes are the record — five spaces and "USING THE TEXT EDITOR," the document's first heading. After it, the next length byte begins the next record, and so on, records packed one after another until a `>FF` marks the end of the sector's data. That is the entire DV80 format: a length byte, that many characters, repeat. A document written on a TI in 1983, still legible in a hex dump in 2026, because the format is honest all the way down — and reading it this way, byte by byte, is the surest proof that a "file" is not magic. It is length-prefixed records in sectors, and now you can read any of them.

## Exercises

**31.1** ✦ List the PAB fields in order and say what each does. Which field carries the operation, and which carries the error afterward?

**31.2** ✦ Why is the error field "sticky," and what must your code do before and after every file operation because of it?

**31.3** ✦✦ Write the READ loop for a DV80 file: OPEN for input, READ until the end, CLOSE — and say which error code is the loop's natural exit and why it is not a failure.

**31.4** ✦✦ Using `filelib`, write a high-score saver: open `DSK1.SCORES` for output, write three score records, close. Handle the "no disk" error by showing a message instead of hanging.

**31.5** ✦✦ Explain why the PAB and file buffers live in VRAM, and describe the layout discipline that keeps a file read from corrupting your screen. Where in VRAM would you put a 256-byte file buffer in a Graphics I program?

**31.6** ✦✦ Read a catalog: open `DSK1.` as INT/FIX 38, read record 0 (the volume record) and the file records, and list the file names. What makes the catalog "just a file"?

**31.7** ✦✦✦ Decode a real DV80 file (as the Field Notes did): from a `.dsk` image, find a variable-display file's descriptor, follow it to its data, and print its first few records by hand-parsing the length-prefixed format. Confirm against the file's declared record count.

**31.8** ✦✦✦ Build `FILER99`'s core: catalog a disk into a pane and view a selected DV80 file, over `textlib40`. (Run it on the desktop app with `--disk`.)

## Further Reading

- App. H — the complete opcode/flag/error-code matrix for the PAB.
- `original-content/system-roms/disk-dsr/RECON.md` §3 — the probe-pinned PAB behavior (opcodes, error codes, the name-pointer contract) this chapter rests on.
- `crates/libre99-gpl/tests/device_io.rs` — the machine-verified PAB path (a real load off a real disk), green at HEAD.
- Chapter 30 (DSRs) — `DSRLNK`, which carries the PAB to the device.
- Chapter 32 (Disk Internals) — the on-disk structures (VIB, FDR, sectors) beneath the file abstraction; where the DV80 records physically live.
- Chapter 14 (Text Mode) — `textlib40`, the UI `FILER99` is built on.
- The xdt99 `xdm99` tool and TIPI (Chapter 34) — moving files between `.dsk` images and modern hosts by type.

## Summary

All file I/O on the TI is one structure — the **Peripheral Access Block**, a block of bytes in **VRAM** you fill in and hand to a device via `DSRLNK` (Chapter 30): an **opcode** (`PAB+0`), **flags** carrying the file type/mode in and a **sticky error code** (bits 5–7) out, a **VRAM buffer pointer**, **record length** and **count**, a **record number**, and the **name** (`DSK1.FILE`). Ten **opcodes** cover everything — OPEN/READ/WRITE/CLOSE for records (open, loop reading until **error 5** = end-of-file, close), RESTORE to rewind, DELETE/STATUS, and LOAD/SAVE for whole **PROGRAM images** — with **error codes** (2 = missing/mismatch, 5 = past-EOF, 6 = rejected) that a robust program reads as conditions, not crashes. Four **file types** carry the world: **DIS/VAR 80** (text, the lingua franca — length-prefixed records in sectors, decodable by hand), **DIS/FIX 80** (object code), **INTERNAL** (compact machine data — the catalog is INT/FIX 38, opened as `"DSK1."`, record 0 the volume record), and **PROGRAM** (whole memory images, LOAD/SAVE). The PAB and buffers live **behind the VDP** (where the free memory was), so file I/O keeps a **layout etiquette** with the screen (both in VRAM) and a small name-pointer dance the DSRLNK path performs. **`filelib`** wraps this into OPEN/READ/WRITE/CLOSE with usable error surfaces — a config/level reader and a high-score writer — and **`FILER99`**, a two-pane file manager over `textlib40`, composes the lot. It is all machine-verified against the project's disk DSR (a real PAB drives *Tunnels of Doom*'s scenario load — `device_io.rs`, green at HEAD; the layout and errors probe-pinned in `RECON.md`), through the Rust harness and the desktop app rather than BENCH99 (no disk on the bench — a stated R-12 gap), and its files are real enough to read byte-by-byte off a 1983 disk (the DV80 `FORMATDOC`, decoded live). The PAB is the `ioctl` idea made central — a struct *is* the file API — and once you have built one, no file on the machine is opaque again.
