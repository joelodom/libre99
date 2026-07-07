# Chapter 25 — GROM: The Strangest Memory You'll Ever Address

*A ROM with no address bus — you tell it where to go one byte at a time, and then it hands you the rest in a stream. TI's oddest, cleverest, and most consequential chip.*

<!-- Part VI — GROM, GPL, and the Operating System · target ≈16 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The GROM port protocol (address high-then-low to >9C02, the automatic prefetch, auto-increment streaming) is machine-verified on BENCH99 at commit c970196: gromlib reads the console GROM header as AA 02 00 00 00 00 00 00 13 10 13 20 (byte-identical to od roms/994AGROM.Bin), and GROMDUMP checksums the first 256 bytes to >50E7 (matching the file's own sum). Code in code/ch25/ (gromlib, gromdump). GROM is fully emulated (grom.rs), prefetch and all. -->

## A ROM You Cannot Point At

Every memory you have addressed so far works the same obvious way: you put an address on the bus, and the memory hands back the byte at that address. RAM, ROM, the scratchpad, even VRAM (through its ports) — all of them, at bottom, are "give me the byte at address A." The **GROM** — Graphics Read-Only Memory, TI's own invention — does not work that way at all, and understanding why is understanding a good half of what makes the TI-99/4A the peculiar machine it is.

A GROM has *no parallel address bus*. There is no way to say "give me the byte at address A" in one operation. Instead the chip contains its own **internal address counter**, and you interact with it through a tiny keyhole: you feed it an address one byte at a time through a port, and then you read bytes out through another port, and *the chip advances its own counter* after every read. To read a table, you tell it once where the table starts, and then you just read, and read, and read, and the bytes stream out as the counter walks forward on its own. It is less like addressing memory and more like *cueing up a tape* and pressing play. The first time you meet it, it feels backwards — why would anyone build memory you cannot point at? — and the answer, once you see it, is a small masterclass in 1970s cost engineering.

This chapter is that chip: what it physically is and why TI built such a thing, the exact port protocol (with its one genuinely tricky detail, the prefetch), the strange 64-kilobyte address space it lives in, the writable *GRAM* variants that will let us run our own code in it, and a clean `lib99` reader — verified, this chapter, reading the console's own GROM byte-for-byte. GROM is where Part VI begins, because everything else in this part — the GPL language, the operating system, TI BASIC — *lives inside GROMs*, and you cannot understand the software without first understanding the strange memory that holds it.

---

## What You Will Learn

- What a GROM physically is — a ROM with its own serial address counter — and why TI built one (density, cost, and control).
- The port protocol at `>9800`/`>9802`/`>9C02`: setting the address high-byte-first, the auto-increment, and the **prefetch** off-by-one you must know cold.
- The 64 KiB GROM space: 6 KiB devices in 8 KiB slots, the console GROMs, and cartridge GROMs.
- **GROM bases**: sixteen parallel address spaces, why they exist, and who used them.
- **GRAM**: writable GROM-space devices — the door through which we will run our own GPL.
- Reading GROM from assembly cleanly: a `gromlib` with the save/restore discipline that keeps the OS from noticing you were there.

## The Bridge: Serial Memory, Then and Now

Serial-access memory is not as alien to a modern programmer as it first looks — you use it constantly, just not as *program memory*. A file on disk is read as a stream: you `seek` to a position (set a counter) and then `read` sequentially (the position advances on its own). A network socket, a serial port, a tape — all are "set a position, then stream," and none lets you randomly address a byte without first moving to it. The GROM is exactly that model applied to a program's *code and data*: a seekable, streaming ROM. What is unusual is not the access pattern — you know it well — but that TI used it for the machine's operating system and flagship software, running an interpreter directly out of a device you can only stream.

The reason is economics, and it is the same reason streaming media exists: **cost per bit**. A serial ROM needs far fewer pins and less addressing circuitry than a parallel one, so it is cheaper to make and cheaper to wire — and in 1979, when memory was the dominant cost of a home computer, cheaper memory that held *more* per dollar was worth a great deal, even at the price of slower, sequential access. TI bet that its software could be arranged to read mostly sequentially (code runs forward; data streams) and traded random-access speed for density and cost. It is the identical trade a modern system makes choosing a cheap streaming store over expensive random-access memory — decades apart, the same engineering logic — and the GROM is where you watch it play out at the level of the individual byte.

## 25.1 What a GROM Physically Is, and Why

A GROM (TI's part number TMC0430) is a read-only memory chip with three things inside: the ROM cells themselves, a **16-bit address counter**, and the port logic that lets the CPU load the counter and read the cells. It holds **6 KiB** of data. It presents *no* address pins to the outside — the CPU never drives a GROM address bus, because there isn't one — only the four memory-mapped ports through which everything happens. The chip is, in effect, a self-contained little streaming device on the memory bus.

Why build this instead of an ordinary ROM? Three reasons, all TI's. **Density and cost**: the serial design needs fewer pins and less glue, so a GROM was cheaper to make and integrate than a parallel ROM of the same size — the decisive factor in a machine fighting a price war. **Suitability**: the console's software is an interpreter running bytecode (GPL, Chapter 26) that mostly executes forward and reads its data as tables — a workload that streams naturally, so the serial penalty is small in practice. And, less often admitted, **control**: GROM was a *TI-proprietary* part, so a cartridge that needed GROM needed TI's chip, which gave TI a licensing chokehold on its own software market (the sidebar's story, and the reason Atarisoft's cartridges pointedly used ordinary ROM instead). The GROM was cheap, it fit the software, and it locked the platform — three birds, one strange chip.

## 25.2 The Port Protocol, and the Prefetch

The CPU reaches the GROM through four byte-wide ports in the memory map, named once in `equates.inc`:

| Port | Operation |
|---|---|
| `>9800` (`GRMRD`) | read a data byte, then auto-increment the counter |
| `>9802` (`GRMRA`) | read the address counter (high byte, then low) |
| `>9C00` (`GRMWD`) | write a data byte (GRAM only; mask ROMs ignore it) |
| `>9C02` (`GRMWA`) | write the address counter (high byte, then low) |

To read from address `A`, you write `A`'s **high** byte then its **low** byte to `>9C02`, then read bytes from `>9800`; the counter advances on each read, so a whole table streams out with no further address writes. `gromlib`'s `GRDADR` is that two-byte aim, and `GRDBLK` is the stream:

```asm
GRDADR MOVB R0,@GRMWA          address HIGH byte first
       SWPB R0
       MOVB R0,@GRMWA          address LOW byte -> completes + prefetches
       ...
GRDBLK MOVB @GRMRD,*R1+        one byte -> CPU dest; GROM steps its counter
       DEC  R2
       JNE  GRDBLK
```

The one detail that trips everyone is the **prefetch**. The GROM keeps a one-byte buffer, and it fills that buffer *immediately* when you finish writing the address — automatically, before you read anything. So after you set the address to `A`, the buffer already holds `mem[A]` and the counter *already points at `A+1`*. Your first `>9800` read returns the buffered `mem[A]` (correct!) and refills the buffer from the counter; but if you read the *address* port back, you see `A+1`, not `A` — the counter is one ahead of the byte you last received. This off-by-one is not a bug and not optional: it is how the chip pipelines reads for speed, and code that reads the address back to find "where am I" must remember the counter leads the data by one. Get it wrong and your GROM offsets are all one byte off, a maddening class of bug.

We can prove the protocol reads true. `gromlib` aims the counter at address 0 and streams sixteen bytes of the console GROM, and the bench reads back exactly what the ROM file contains:

```text
m 8320 16  ->  AA 02 00 00 00 00 00 00 13 10 13 20 00 00 00 00
```

byte-identical to `od roms/994AGROM.Bin`. The whole header flowed from *one* address write — the auto-increment doing all the walking — which is the streaming model made concrete. And `GROMDUMP` checksums the first 256 bytes to `>50E7`, the same value the file itself sums to: the reader is faithful at scale, not just at the header.

## 25.3 The 64 KiB Space: Devices in Slots

The GROM address counter is 16 bits, so the GROM address space is **64 KiB** — but no single chip holds that. The space is divided into eight **8 KiB slots** (selected by the top three address bits), and each physical GROM chip, though only 6 KiB, occupies a full 8 KiB slot (the top 2 KiB of each slot is unused). So the 64 KiB space has room for eight GROM devices.

The console populates the low slots: **GROMs 0, 1, and 2** hold the console's own operating system, the master title screen and menu, TI BASIC, and the shared GPL library (Chapter 28) — the first byte of GROM 0 being the `>AA` signature `gromlib` just read. A **cartridge** plugged into the slot brings its own GROMs, **3 through 7**, appearing higher in the space; the console's boot code scans these slots looking for cartridges to add to the menu (Chapter 28's algorithm). One consequence of the slot structure matters when you program: the counter's auto-increment **wraps within the current 8 KiB slot** rather than crossing into the next, so a table cannot silently run from one GROM into another — you re-aim to cross a slot boundary. This is a helpful containment (a runaway read stays in its device) and a thing to plan around (data spanning two GROMs needs an explicit re-aim at the seam).

## 25.4 GROM Bases: Sixteen Parallel Spaces

Here the GROM gets stranger still. The console does not expose *one* set of GROM ports but **sixteen**, at `>9800`, `>9804`, `>9808`, … each four bytes apart — sixteen parallel doorways called **GROM bases**. Each base is an independent view: GROMs wired to base 0 answer at `>9800`, GROMs wired to base 1 answer at `>9804`, and so on, so the machine can hold sixteen *parallel* 64 KiB GROM spaces, addressed by which base you talk to.

Why sixteen? Expansion. The extra bases let peripheral cards carry their own GROMs without colliding with the console's — the most famous user being the **p-code card**, whose UCSD Pascal system lived in GROMs on a base of its own, invisible to software talking to base 0. In everyday cartridge programming you use base 0 (the console and cartridge GROMs), and `gromlib` targets it; but a library that must reach a peripheral's GROMs selects the base by using the corresponding port address, and the sixteen-base structure is why the GROM space is not merely large but *plural*. It is over-engineering that paid off exactly once or twice and sat mostly unused — but it is there, and knowing it exists explains addresses like `>9804` when you meet them in a peripheral's driver.

## 25.5 GRAM: The Writable Door

A GROM is read-only — `write_data` to `>9C00` does nothing on a mask ROM, as the port table notes. But the *protocol* has a write-data port, and that port is not vestigial: it is there for **GRAM**, GROM-space devices whose cells are *writable*. A GRAM chip answers the same serial protocol, occupies the same slots, streams the same way — but you can write bytes into it through `>9C00`, so it is RAM that lives in GROM space and runs like GROM.

This matters enormously to us, because GROM is where GPL code lives, and a mask GROM's code is fixed forever — but *GRAM's* is not. The community built GRAM devices for exactly this: the **GRAM Kracker** of the 1980s, and today the **FinalGROM 99** and similar carts, which present writable GROM space you can load your own GPL into and run. Without GRAM, writing GPL (Chapter 27) would be a paper exercise — you could assemble it but never execute it on hardware, because there would be nowhere writable in GROM space to put it. With GRAM, your GPL becomes a real, running program in the machine's native software language. GRAM is the door through which Chapters 26 and 27 walk: the emulator gives us writable GROM space to run our own GPL, and on hardware a FinalGROM 99 does the same. The write-data port, idle on every mask ROM, is the whole reason we can be more than readers of TI's GROMs — we can author our own.

## 25.6 Reading GROM Cleanly: `gromlib` and the Discipline

Reading GROM is `GRDADR` then `GRDBYT`/`GRDBLK` — aim, then stream. But there is a courtesy that separates a library from a hack: **save and restore the counter**. When your program has the machine to itself, you may set the GROM address freely. But when the GPL interpreter or the console's own code is alive (Chapters 22, 28), *they* own the GROM counter — the interpreter's program counter *is* a GROM address — and if you re-aim the counter and return without restoring it, you derail whatever GPL was mid-stream. So a polite GROM reader, before it aims the counter, reads the current address back (two reads of `>9802`, high then low — which, usefully, also resets the write-byte phase so a following address write is correctly sequenced), does its reading, and writes the saved address back when done, leaving the counter where it found it.

`gromlib` documents this discipline and the chapter's routines are its core; the save/restore wrapper is a few instructions around them, and the exercises build it. The principle is the same one Chapter 22 stated for the VDP and the ISR: any hardware state you share with the OS is borrowed, and a routine that borrows it must return it unchanged. The GROM counter is exactly such shared state — more so than most, because for the GPL interpreter it is not data but the *instruction pointer itself* — so a `gromlib` that saves and restores it composes safely into a running console, and one that does not corrupts the interpreter the moment GPL and your code coexist.

## Lab 25 — `GROMDUMP`

The lab is the GROM reader and a use that proves it, in `code/ch25/`.

**`gromlib` (`gromlib.inc` + `gromlib.a99`)** — the GROM reader for `lib99`: `GRDADR` (aim the counter), `GRDBYT` (read one byte), `GRDBLK` (stream a block). Build and prove it:

```sh
libre99asm code/ch25/gromlib.a99 --format bin -o build/GRLC.bin --symbols build/grl.map.json
```

The self-test reads the console GROM header — `AA 02 00 00 …` at `m 8320` — verifying the `>AA` signature and the version against the ROM file, with `R7=>02` green.

**`gromdump.a99`** — dump and checksum: stream 256 bytes of the console GROM and sum them, and confirm the checksum is `>50E7`, the value the ROM file itself sums to. It is the same reader at scale, and it is a genuinely useful tool — point it at any GROM region (console or cartridge) to fingerprint it, and you have a way to tell one console version's GROM from another (Chapter 28), or to verify a cartridge dumped correctly. The exercises extend it to browse GROM with `MONITOR99` (Chapter 13) — hex-reading TI BASIC's own bytes out of the strange memory that holds them.

> **Sidebar — The lockout that backfired.** GROM was proprietary, and TI meant it as a moat: to publish a TI-99/4A cartridge with GROM-based software, you needed TI's chip and, effectively, TI's blessing, and TI used this to control — and tax — its software market, even forbidding some third parties from writing for the machine at all. It backfired spectacularly. Locked out of GROM, third parties simply *did not use it*: Atarisoft, porting its arcade hits, put its code in ordinary parallel ROM at `>6000` (Chapter 35's cartridge space) and ignored GROM entirely — no TI chip required, no TI permission sought. The lockout that was meant to make TI indispensable instead taught the market to route around it, and the flood of ROM-only cartridges that resulted was software TI neither controlled nor profited from. It is a small, sharp lesson in platform economics: a chokehold on a component only works if the component is necessary, and by making GROM optional-in-practice, TI turned its moat into a detour. The strangest memory in the machine was also, for a while, the most political.

## Exercises

**25.1** ✦ Why does reading the GROM address port after setting the address to `A` return `A+1`? What is the one-byte buffer that causes it, and when is it filled?

**25.2** ✦ Give the exact port writes to aim the GROM counter at address `>1310`, in order. (High byte then low, to which port?)

**25.3** ✦✦ Modify `gromdump` to checksum an arbitrary GROM range (start address and length as parameters) and fingerprint three different regions of the console GROM. Confirm each is stable across runs.

**25.4** ✦✦ Write the save/restore wrapper of §25.6: read the current GROM address, do a read, and restore it. Prove (by reading the address back before and after) that the counter is left where it started.

**25.5** ✦✦ The auto-increment wraps within an 8 KiB slot. Demonstrate it: aim near the top of a slot, stream past the wrap point, and show that the reads come from the slot's start, not the next slot.

**25.6** ✦✦✦ Browse GROM with `MONITOR99` (Ch. 13): build a viewer that dumps a screenful of GROM as hex+ASCII, navigable by address, and use it to find the `>AA` header and the list pointers at the start of GROM 0. (You are reading the console's operating system in situ.)

## Further Reading

- TMC0430 GROM datasheet — the chip's serial protocol, the prefetch, and the internal counter this chapter's `gromlib` drives.
- Chapter 12 (Inside the TMS9918A) — the VDP's very similar aim-then-stream port protocol and prefetch, the GROM's sibling in design.
- Chapter 26 (The GPL Language) — the bytecode that lives in GROM and the interpreter whose program counter *is* a GROM address.
- Chapter 27 (Writing GPL Today) — running your own GPL in GRAM, the writable door of §25.5.
- Chapter 28 (The OS in GROM) — the console GROMs 0–2 and the boot scan of cartridge GROMs this chapter's header read begins.
- Chapter 35 (Cartridge Engineering) — the ROM-at-`>6000` cartridge path Atarisoft used to route around GROM (the sidebar).

## Summary

A **GROM** is TI's strangest memory: a 6 KiB read-only chip with no address bus, holding an internal 16-bit **address counter** and streaming its contents through four memory-mapped ports — `>9800` (read data, auto-increment), `>9802` (read the counter), `>9C00` (write data — GRAM only), `>9C02` (write the counter). You read by aiming the counter (its address **high byte then low** to `>9C02`) and then streaming bytes from `>9800`, the counter walking forward on its own — a seekable, streaming ROM, chosen for **density, cost, and TI's proprietary control** in 1979's price war, suited to software that reads forward. Its one tricky detail is the **prefetch**: setting the address to `A` immediately buffers `mem[A]` and advances the counter to `A+1`, so the first read returns the right byte but reading the counter back shows `A+1` — an off-by-one that is the pipeline, not a bug. The 64 KiB space is eight 8 KiB **slots** (6 KiB devices, wrapping within a slot) — console GROMs 0–2 hold the OS, menu, and TI BASIC; cartridge GROMs 3–7 are scanned at boot — and there are **sixteen parallel GROM bases** (`>9800`, `>9804`, …) for peripherals like the p-code card. **GRAM** — writable GROM-space devices (GRAM Kracker, FinalGROM 99, the emulator) — is the door through which we run our own GPL (Chapters 26–27). `gromlib` reads it cleanly with a **save/restore discipline** (the counter is shared state — for the interpreter it is the instruction pointer itself), and is verified reading the console GROM byte-for-byte (`AA 02 00 …`, checksum `>50E7`). Everything in Part VI lives in these GROMs; this chapter is the key that reads them.
