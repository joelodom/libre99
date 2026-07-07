# Chapter 32 — Disk Internals: Sectors, Structures, and Controllers

*Under the file abstraction is a filesystem you can hold whole in your head — a volume record, a directory, file descriptors, a free-space map — 90 kilobytes of honest structure we will decode, byte by byte, off a real 1983 disk.*

<!-- Part VII — Storage and Peripherals · target ≈20 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The whole TI filesystem is decoded LIVE from disks/Tunnels.Dsk (verified byte-by-byte with od): VIB (sector 0) = "TUNNELS", 360 sectors (>0168), 9 spt, "DSK", 40 tracks, 1 side, density 1, allocation bitmap 0f 00 00 00 fc ff...; FDIR (sector 1) = FDR pointers to sectors 2,3; FDR PENNIES (sector 2) = PROGRAM (flags 01), 51 sectors, cluster @ sector 34; FDR QUEST (sector 3) = PROGRAM, 51 sectors, cluster @ sector 85 (the scenario device_io.rs loads). The .dsk format + FD1771 are machine-verified via crates/libre99-core/tests/disk.rs (14 tests GREEN at HEAD; disk.rs parses geometry from the VIB). Code: code/ch32/seclib.inc (SECTOR subprogram >10 frame, assembles; libre99asm). NOT bench (no disk — R-12). RS/CorComp/Myarc controllers + CF7/nanoPEB/TIPI described from the hardware record; the emulator models the TI FD1771 SSSD/DSSD. -->

## Ninety Kilobytes You Can Understand Completely

There is a rare pleasure in a system small enough to understand *completely*, and the TI's disk format is one of them. A modern filesystem — ext4, NTFS, APFS — is a research project: journals, B-trees, extents, checksums, tens of thousands of lines of kernel code, more than any one person holds in their head. The TI's filesystem fits on a napkin. A single-sided disk is **90 kilobytes** — 360 sectors of 256 bytes — and every one of its structures is a plain, fixed layout you can decode with a hex dump and a pencil: one sector says what the volume is and which sectors are free; one sector is the directory; a handful of sectors describe the files; the rest is data. That is the whole thing. No journal, no tree, no checksum — just structure, honest all the way down, and by the end of this chapter you will have read all of it off a real disk and know exactly where every byte lives.

This is the layer beneath Chapter 31. There, a file was a contract — a PAB, an opcode, records flowing through a buffer — and the disk was an abstraction that served files. Here we lift the abstraction and look at the *medium*: how 90 KB of magnetic flux is framed into tracks and sectors by the controller, and how those sectors are organized into a filesystem — a volume record, a directory, file descriptors with cluster chains, a free-space bitmap. Understanding this layer is what lets you write the tools the file abstraction cannot: a sector editor, an un-deleter, a disk mapper, a repairer — and it is what lets you *read a disk that the file layer has given up on*, byte by byte, and recover it. We will decode a real disk — `Tunnels.Dsk`, the game whose scenario Chapter 31 watched load — completely, and it will demystify disks for good.

---

## What You Will Learn

- The **single-density world**: 90 KB per side — 40 tracks × 9 sectors × 256 bytes — and how the FD1771 controller framed a floppy.
- The **on-disk anatomy**: the VIB (volume info + allocation bitmap), the directory, the FDRs (file descriptors + cluster chains) — the whole filesystem, decoded live.
- **Sector-level access**: the disk DSR's subprograms, wrapped in `seclib`.
- The **tools** this enables: disk mapper, un-deleter, catalog repairer, sector editor.
- The **controller zoo**: TI vs. CorComp vs. Myarc, the modern replacements, and capability detection.
- **Image formats** for interchange, and a word on copy protection and preservation.

## The Bridge: Every Filesystem, in Miniature

If you have ever studied how a filesystem is laid out on disk — a course, a hobby, a data-recovery afternoon — the TI's structures will feel like old friends met in childhood. Every filesystem has the same four organs, and the TI has all four, tiny and legible. A **superblock** — the volume-wide record saying how big the volume is and where things are — the TI calls the **VIB** (Volume Information Block), sector 0. An **inode table** — the per-file metadata — the TI keeps as **FDRs** (File Descriptor Records), one sector each, indexed by a directory. An **extent/block list** — where a file's data physically lives — the TI stores as **cluster chains** inside each FDR. And a **free-space map** — which blocks are used — the TI keeps as an **allocation bitmap** right inside the VIB. Superblock, inodes, extents, free-space bitmap: the four organs of ext2 and every filesystem like it, here reduced to their absolute minimum and laid out in fixed byte offsets you can read directly.

So this chapter is not just "the TI disk format" — it is *filesystems*, taught on the smallest complete specimen that has all the parts. What you learn decoding `Tunnels.Dsk` — find the superblock, read the free map, walk the inode to its extents, follow the data — is the exact procedure a data-recovery tool runs on a corrupted ext4 volume, minus ten thousand lines of complication. The TI filesystem is the concept made small enough to hold entire, and holding it entire is the point: you will never again wonder what a filesystem *really* is, because you will have read one, whole, in a hex dump.

## 32.1 The Single-Density World

A TI single-sided, single-density disk is **40 tracks**, each **9 sectors**, each **256 bytes** — 360 sectors, **92,160 bytes**, 90 KB — and the controller that framed it was the **FD1771** (a Western Digital floppy controller, Chapter 30's `>5FF0` registers). "Framing" is the controller's physical job: a floppy stores a spiral (well, concentric rings) of magnetic flux transitions, and the FD1771 turns them into addressable sectors — finding a track by stepping the head, locating a sector by its ID field as the disk spins, and reading or writing its 256 bytes with the encoding (FM, for single density) that packs bits into flux. Above that physical framing, the disk is simply **360 numbered sectors**, and everything else — the filesystem — is built in software on that flat array of sectors. The project's emulation models exactly this: the FD1771 registers (Chapter 30), the 256-byte sectors, and the geometry, verified by 14 conformance tests (`disk.rs`, green at HEAD) that drive the controller through seeks, reads, writes, and a real game load.

Single density's **90 KB** sounds tiny — it is a fifth of a single 1.44 MB PC floppy of a few years later — and it was tight even then: a disk held a couple of programs and their data, and serious work meant swapping disks constantly. Double-density doubled it (18 sectors per track, 180 KB; §32.5), and double-sided doubled it again. But the *structure* is the same at every size — more sectors, same organs — so we learn it at 90 KB, where it is smallest and clearest, and scale up understanding it. The number to hold: **360 sectors of 256 bytes**, a flat array the filesystem organizes.

## 32.2 On-Disk Anatomy: The Whole Filesystem, Decoded

Here is the entire TI filesystem, decoded live from `Tunnels.Dsk` with nothing but a hex dump. It is four kinds of structure, and we will read each from the real disk.

```text
Tunnels.Dsk — the whole filesystem on one diagram (decoded byte-by-byte)

 Sector 0  VIB   "TUNNELS   "  360 sectors (>0168)  9 sec/trk  "DSK"
                 40 tracks  1 side  density 1
                 allocation bitmap: 0F 00 00 00 FC FF FF FF ...
                   -> sectors 0-3 used, 4-33 free, 34-135 used
 Sector 1  FDIR  -> [00 02][00 03][00 00]   (two files: FDRs at sectors 2, 3)
 Sector 2  FDR   "PENNIES   "  flags 01 (PROGRAM)  51 sectors  cluster @ 34
 Sector 3  FDR   "QUEST     "  flags 01 (PROGRAM)  51 sectors  cluster @ 85
 Sectors 34-84   PENNIES data (the game program)
 Sectors 85-135  QUEST data   (the scenario Ch. 31 loads)
```

The **VIB** (Volume Information Block), sector 0, is the superblock: its first 10 bytes are the volume name (`TUNNELS`), then the total sector count as a big-endian word (`>0168` = 360), the sectors-per-track (`>09`), the `"DSK"` magic, and the geometry (40 tracks, 1 side, density 1). And from byte `>38` onward is the **allocation bitmap** — one bit per sector, LSB-first, `1` = used — which we read as `0F 00 00 00 FC FF FF FF …`: byte 0 = `0F` means sectors 0–3 are used (the VIB, the directory, and two FDRs), the next bytes `00` mean sectors 8–33 are free, and `FC FF…` from byte 4 means sectors 34 onward are used (the two files' data). The free-space map, in one sector, readable at a glance.

The **directory** (FDIR), sector 1, is the inode index: a list of big-endian sector pointers to FDRs, name-sorted, zero-terminated. `Tunnels.Dsk`'s reads `00 02  00 03  00 00` — two files, their FDRs at sectors 2 and 3, then the terminator. The **FDRs** (File Descriptor Records) are the inodes, one sector each: sector 2's first bytes are `PENNIES`, a flags byte (`>01` = bit 0 set = a PROGRAM file), the count of sectors allocated (`>0033` = 51), the record length, and — from byte `>1C` — the **cluster chain**, a list of 3-byte cluster descriptors packing a starting sector and an extent length (PENNIES's decodes to "start at sector 34, 51 sectors"). Sector 3 is `QUEST`, another PROGRAM of 51 sectors starting at sector 85 — the very scenario Chapter 31 watched *Tunnels of Doom* load through a PAB (`device_io.rs`, green). So the file abstraction of Chapter 31 sits directly on this: "load `QUEST`" means "find `QUEST` in the directory (sector 1 → FDR at sector 3), read its cluster chain (start 85, 51 sectors), and stream sectors 85–135." We have now read the whole path, from filename to physical sectors, off a real disk — the filesystem entire.

## 32.3 Sector-Level Access

Below the file layer, the disk DSR exposes **subprograms** reached by *number* rather than device name (Chapter 30's subprogram list), and the fundamental one is **`>10` — SECTOR** — which reads or writes one raw 256-byte sector by its absolute number. This is the primitive beneath everything in §32.2: to decode a VIB you read sector 0; to walk a directory you read sector 1; to edit a disk you read a sector, change bytes, write it back. `seclib` wraps it (`code/ch32/`), and the key thing it shows is that a subprogram is called *differently* from a named file — not a PAB, but the pinned subprogram frame:

```asm
SECIO  MOVB @x0A,@>836D      >836D = >0A   (this is a subprogram call)
       MOVB @x01,@>8355      >8355 = 1     (name length 1)
       MOVB @x10,@>834A      >834A = >10   (the SECTOR subprogram number)
       ...                   sector #, drive, buffer -> the >834C param block
       BL   @DSRLNK          link to the card that owns subprogram >10
```

`seclib`'s `SECRD`/`SECWR` stage this frame with the sector number, drive, and VDP buffer, then link — the sector read straight into VRAM, raw. The frame (`>836D`/`>8355`/`>834A`) is probe-pinned (`RECON.md`); the exact byte offsets of the SECTOR parameters within `>834C` are the disk DSR's subprogram reference (App. H), which `seclib` points to rather than assert at fragile precision (R-12). With `SECRD` you can read any sector of any disk into a buffer and decode it — which is the foundation of every tool in §32.4 — and the project's harness exercises the sector subprogram against the real DSR (`disk_dsr.rs`'s sector probe). As with all disk code, `seclib` assembles and is run through the desktop app or hardware, not the bench (no disk on the bench, R-12).

## 32.4 Tools You Can Now Write

Raw sector access plus the structure of §32.2 is the whole toolkit, and four tools fall out of it. A **disk mapper** reads the VIB's allocation bitmap and draws which sectors are used and free — a visual of the disk's occupancy, the thing you want when a disk is "full" but seems empty (fragmentation, or a lost chain). An **un-deleter** exploits the fact that DELETE (Chapter 31) removes a file's *directory entry* and frees its bitmap bits but does **not** erase its data or FDR — so if you act before those sectors are reused, you can find the orphaned FDR, re-link it into the directory, and re-mark its clusters in the bitmap, and the file returns (the same principle as undelete on any filesystem, and just as urgent about acting before overwrite). A **catalog repairer** rebuilds a directory whose pointers have gone bad by scanning for valid FDRs (a sector that looks like an FDR — a plausible name, sane flags, a cluster chain that fits) and re-threading the directory. And a **sector editor** — the lab's `DISKDOC` — reads any sector, shows it as hex with structure overlays (decode this sector *as* a VIB, *as* an FDR), lets you change bytes, and writes it back. Each is a short program over `seclib` and the structures you now know; the lab builds the sector editor, and the exercises sketch the rest. This is the payoff of understanding the medium: the file layer can only do what files allow; sector access plus structure lets you do *anything* to a disk, including fix one the file layer has broken.

## 32.5 The Controller Zoo and Its Dialects

"The disk controller" was not one thing. TI's own controller (the FD1771, single-density, 90 KB/side) was the baseline, but the third-party market built better ones, and they spoke **dialects**. **CorComp** and **Myarc** controllers added **double density** (18 sectors/track, 180 KB/side) and double-sided support, and Myarc's went further to **80-track** drives and higher capacities — same filesystem structure (§32.2), more sectors, a VIB declaring the larger geometry. The modern replacements changed the *medium* entirely while keeping the format: **CF7+/nanoPEB** put the TI filesystem on CompactFlash cards (many disk "volumes" in one card), and **TIPI** (Chapter 34) maps TI files onto a Raspberry Pi's storage — 90 KB "disks" that are really directories on a Linux box. The lesson for your code is **capability detection over assumption**: do not assume 40 tracks and single density — read the VIB and believe what it says about geometry, because the disk in the drive might be a 90 KB TI original, a 180 KB CorComp double-density, or a CF7 volume, and your sector arithmetic must follow the VIB, not a constant. The project's emulation embodies this: `disk.rs` **parses the geometry from the VIB** (total sectors, sectors/track, sides, density) and honors what it declares — trusting the VIB's numbers when they are self-consistent, falling back to 90 KB SSSD otherwise — so it reads a single-density `Tunnels.Dsk` and a double-density image alike (the repo's `FWeb501.Dsk` is a 180 KB DSSD volume). Capability detection is not just good manners; it is the only way to survive the zoo.

## 32.6 Image Formats for Interchange

To move a disk between 1982 media and a 2026 laptop, you need a **disk image** — a file that holds the whole disk — and the TI world settled on the **sector dump**: a file that is simply all the disk's sectors, in order, back to back, with no headers (256 bytes × 360 sectors = 92,160 bytes for SSSD). This is the `.dsk` format the project reads (`disk.rs`: raw 256-byte sectors in LBA order, geometry from the VIB) and the one `Tunnels.Dsk` is — which is *why* we could decode it with a plain hex dump: the image is the disk, byte for byte, so offset = sector × 256, and reading the file is reading the disk. Variants exist (the v9t9 and PC99 conventions differ in track ordering for the second side, which `disk.rs` handles), and the tooling is mature: **`xdm99`** (xdt99) and **TIImageTool** create, inspect, and edit `.dsk` images on a modern machine — extract a file, insert one, catalog, format — so the workflow is: image a real disk (with hardware or a CF7), work with it on your laptop via `xdm99`, and write it back. Getting bits between the eras is a solved problem, and the sector-dump image is why: because a TI disk *is* just its sectors, a file that is just its sectors is a perfect, lossless copy.

## 32.7 A Note on Copy Protection and Preservation

Two honest notes to close the medium. First, **copy protection**: there was barely any. Unlike the Apple II or C64 worlds, where elaborate disk-protection schemes (weird sectors, timing tricks, deliberate errors) spawned an arms race, the TI-99/4A saw very little — most software was on cartridge (Part VI), the disk market was smaller, and the schemes that did appear were mild. This is a gift to preservation: TI disks image cleanly with standard tools, no special hardware or de-protection required, because there is almost nothing fighting you. Second, **preservation ethics**, done right: imaging disks to save them — the software, the data, the documents people made — is preservation, and it matters, because magnetic media degrades and the disks are forty years old and dying. The right way honors both the work and its makers: image and archive to prevent loss, share what is freely shareable, respect the rights of software still owned, and — as this whole book models — prefer *re-implementation* (the clean-room DSR, Chapter 30; the clean-room ROM, Chapter 28) to redistribution where rights are unclear. Preservation is not piracy's excuse; it is the careful, ethical work of making sure that when the last 1983 floppy finally fails, what was on it is not lost — the bytes readable, the formats documented, the machine still able to run them.

## Lab 32 — `DISKDOC`

The lab is a **sector editor with structure overlays**, `DISKDOC`, in `code/ch32/`. It reads any sector with `seclib`'s `SECRD`, shows it as hex-and-ASCII, and — the useful part — **decodes it as a structure on demand**: view sector 0 *as a VIB* (name, size, geometry, and the allocation bitmap drawn as a used/free map), view an FDR sector *as an FDR* (name, flags, size, cluster chain), so you are not staring at raw hex but at the filesystem's organs, labeled. It lets you edit bytes and write the sector back with `SECWR`. Then the exercise that teaches the most: **deliberately corrupt a scratch disk** — zero a directory pointer, or flip an allocation-bitmap bit — watch the file layer stumble, and **repair it by hand** in `DISKDOC`, re-threading the directory or fixing the bitmap from your knowledge of §32.2. Recovering a disk you broke, with a tool you wrote, from structure you understand, is the chapter's whole thesis proven: the medium holds no mysteries once you can read its sectors. (As with all disk code, `DISKDOC` runs against real images through the desktop app `--disk` or hardware; it assembles on the bench, but the disk lives where the DSR does.)

> **Sidebar — Single density in a double-density world.** By 1983, double-density disk controllers were common in the wider microcomputer market — the technology was proven, the chips available — yet TI shipped its disk controller **single-density**, 90 KB a side, half of what the hardware era could do. Why? The usual TI blend of timing, cost, and caution: the controller was designed early, single-density was the safe, cheap, well-understood choice, and by the time double-density was obvious the company was months from leaving the home-computer business entirely (Chapter 45). So TI's own disks were half-size in a world that already knew better, and it fell to the *third parties* — CorComp, Myarc — to give the TI the double-density and double-sided drives it should have shipped, a recurring pattern in this machine's story (the community and the aftermarket finishing what TI started). The single-density baseline is why "a TI disk" means 90 KB in most people's memory, and why the format we decode is the small one — TI's caution, frozen into the medium, and generously corrected by everyone who came after.

## Exercises

**32.1** ✦ How many bytes is a single-density TI disk, and how does that break down into tracks, sectors, and sector size? What chip framed the floppy into sectors?

**32.2** ✦ Name the four organs of the TI filesystem and where each lives (which sector or structure). Which one is the free-space map, and how is it encoded?

**32.3** ✦✦ Decode a VIB by hand from a `.dsk` image: read the volume name, total sectors, geometry, and the first few bytes of the allocation bitmap, and say which sectors are used. (Use `Tunnels.Dsk` or another.)

**32.4** ✦✦ Follow a file from name to data: pick a file, find its FDR through the directory, decode its cluster chain, and list the physical sectors its data occupies. Confirm the count against the FDR's sector-allocated field.

**32.5** ✦✦ Explain how an un-deleter works: what DELETE does and does *not* erase, and the three steps to bring a just-deleted file back. Why is acting quickly essential?

**32.6** ✦✦ Why must disk code read geometry from the VIB rather than assume 40×9? Give two different real geometries your code might meet and how the VIB distinguishes them.

**32.7** ✦✦✦ Build `DISKDOC`'s core: read a sector with `seclib`, display it with a VIB or FDR overlay, edit a byte, write it back. (Run against a scratch image via the desktop app.)

**32.8** ✦✦✦ Corrupt and repair: on a scratch disk image, zero a directory pointer, observe the file layer's failure, and repair the directory by hand from the surviving FDR. Document each step.

## Further Reading

- `crates/libre99-core/src/disk.rs` and `crates/libre99-core/tests/disk.rs` — the FD1771 emulation and its 14 conformance tests (green at HEAD); the `.dsk` geometry parsing this chapter's decodes rely on.
- `original-content/system-roms/disk-dsr/RECON.md` §5 — the probe-pinned on-disk format (VIB, FDIR, FDR, cluster chains) decoded here from `Tunnels.Dsk`.
- The xdt99 `xdm99` and TIImageTool documentation — creating and editing `.dsk` images on a modern host.
- Chapter 30 (DSRs) and Chapter 31 (File I/O) — the DSR subprograms and the file abstraction that sit atop these sectors.
- Chapter 34 (Modern Peripherals) — CF7/nanoPEB and TIPI, the modern media that carry this same format.
- Any filesystem-internals reference (ext2's superblock/inode/bitmap layout) — the same four organs, at scale.

## Summary

Beneath the file abstraction (Chapter 31) is a filesystem small enough to understand **completely**: a single-density TI disk is **40 tracks × 9 sectors × 256 bytes = 90 KB**, framed into 360 numbered sectors by the **FD1771** controller (machine-verified — `disk.rs`, 14 tests green at HEAD), on which the filesystem is built in software with the **four organs every filesystem has**: a **superblock** (the **VIB**, sector 0 — volume name, size, geometry, and the **allocation bitmap** free-space map), an **inode index** (the **directory**, sector 1 — sorted sector pointers to FDRs), **inodes** (the **FDRs**, one sector each — name, flags, size, and a **cluster chain** of extents), and the data. We decoded the whole of `Tunnels.Dsk` live, byte by byte: `TUNNELS`, 360 sectors, two files — `PENNIES` and `QUEST`, both PROGRAM images of 51 sectors — with `QUEST` (sectors 85–135) the very scenario Chapter 31 watched load, so the path from filename to physical sectors is now read end to end. Beneath the file layer, the disk DSR's **`>10` SECTOR subprogram** reads and writes raw sectors (wrapped in `seclib`, whose subprogram-call frame is probe-pinned), the primitive for a **disk mapper, un-deleter, catalog repairer, and sector editor** (the lab's `DISKDOC`, which decodes sectors as VIB/FDR overlays and repairs a disk you deliberately broke). The **controller zoo** — TI's single-density baseline, CorComp/Myarc double-density and 80-track, modern CF7/nanoPEB/TIPI — demands **capability detection** (read geometry from the VIB, never assume; `disk.rs` does exactly this), and the **`.dsk` sector-dump image** (all sectors back to back, no headers — which is why a hex dump *is* the disk) makes interchange with `xdm99`/TIImageTool lossless. Copy protection was mercifully rare (clean imaging), and preservation done right images to prevent loss while preferring re-implementation to redistribution where rights are unclear. All of it is verified against the project's disk emulation (through the Rust harness and the desktop app, not BENCH99 — the R-12 gap), and all of it fits in your head: 90 KB of honest structure, and now you can read every byte.
