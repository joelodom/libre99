# Chapter 30 — DSRs: How Peripherals Introduce Themselves

*A TI peripheral is not a dumb device the console drives. It is a device that brings its own driver — in ROM, on the card — and introduces itself to the machine with a magic byte and a linked list. Plug-and-play, in 1983.*

<!-- Part VII — Storage and Peripherals · target ≈18 pp -->
<!-- STATUS: DRAFTED (session 7, 2026-07-06) — pending review passes. The DSR mechanism is machine-verified via the Rust test harness (NOT BENCH99, which has no DSR at >4000 — a stated R-12 gap): crates/libre99-gpl/tests/device_io.rs is GREEN at HEAD (3 tests: the boot >AA scan finds the disk card, the power-up reserves the VRAM buffer, a DSK1 DSRLNK links to the disk DSR, and Tunnels of Doom loads its QUEST scenario end to end; bad devices error gracefully), and crates/libre99-core/tests/disk.rs is GREEN (14 tests: FD1771 conformance). The calling convention (>AA at >4000, device-list head at >4008, BL entry with R12=CRU base + LIMI 0, the skip-return = handled) is probe-pinned in original-content/system-roms/disk-dsr/RECON.md §1. Code: code/ch30/dsrlib.inc (our DSRLNK, assembles; libre99asm). SCOPE (R-12): the emulator hardwires >4000 to the DISK card only (no general multi-card DSR bus), so NULCARD/a custom card is a real-hardware/MAME concept — the disk DSR is the concrete verified example; the general PEB (RS-232 at >1300 etc.) is described from the hardware record. A sibling session is actively building the clean-room disk DSR firmware — this chapter cites committed HEAD only. -->

## The Device That Brings Its Own Driver

Plug a disk controller into a TI-99/4A and something quietly remarkable happens at power-on: the console *discovers* it. Nobody told the console a disk controller would be there; no driver was installed from a floppy (there is no floppy yet — the disk controller is what reads floppies). Yet a moment after boot, `DSK1.` works, the machine can catalog a disk, load a program, read a file — because the peripheral **introduced itself**. It did so by bringing its own driver, in ROM, on the card, and announcing that driver to the console with a signature byte and a linked list the console knows how to walk. This is the **Device Service Routine** — the DSR — and it is the mechanism by which every TI peripheral, from the disk controller to the RS-232 card to a modern Raspberry-Pi bridge, joins the machine.

It is, in a word, **plug-and-play** — the thing the PC world would spend the next fifteen years struggling toward — and the TI had it in 1983. The elegance is that the console contains *no* device drivers. It does not know how to talk to a floppy, a serial port, a printer; it knows only how to *find* the driver a peripheral brought and *call* it. Drivers live with their devices, paged into a shared window one card at a time, and the console's job is discovery and dispatch, not driving. Learn this mechanism and the whole peripheral world of the TI opens: storage (Chapters 31–32), the wired peripherals (Chapter 33), and the modern boards (Chapter 34) all introduce themselves the same way — the DSR is the universal handshake, and this chapter is how it works, verified against the one DSR this project emulates in full: the disk controller's.

A note on ground, stated once and honored throughout. The project's emulator models the **disk controller** completely — its DSR ROM, its CRU gating, its FD1771 — and the console's discovery-and-dispatch path is machine-verified against it (the Rust test harness boots, finds the card's `>AA` header, and loads a real game from a real disk image). It does *not* model a general multi-card expansion bus: the `>4000` DSR window is wired to the disk card specifically. So this chapter teaches the DSR mechanism against the disk DSR (concrete, emulated, verified) and describes the wider peripheral bus — other cards, other CRU bases — from the hardware record, naming the shelf emulators (Classic99, MAME) that model the whole box. That is the honest shape of the ground, and it is exactly enough to learn the mechanism cold.

---

## What You Will Learn

- The **DSR idea**: every peripheral brings its driver in ROM, mapped one card at a time into `>4000`, gated by the card's CRU bit.
- The **DSR header**: the `>AA` signature and the linked lists — device names, subprograms, power-up routines, card ISRs — and how names match (`DSK1.`, `RS232`, `PIO`).
- The **CRU geography** of a loaded expansion box: the base map, and polite multi-card citizenship.
- **`DSRLNK` from the inside**: we write our own — search, page-in, call, page-out, error-return — rather than treat E/A's as a spell.
- How a DSR hooks the **interrupt chain** (the RS-232 pattern).
- Writing a **DSR of your own**: the rite of passage, and where it runs.

## The Bridge: Device Discovery, Then and Now

Every modern operating system does exactly what the TI console does at boot, at vast scale: it *enumerates* devices and *binds drivers* to them. USB is the purest example — plug in a device and it presents a **descriptor** (a vendor ID, a device class, endpoints), the host reads it, and the OS matches a driver to the class and loads it; PCI does the same with configuration space; plug-and-play was the whole promise of the 1990s. The device advertises what it is in a structured header; the host discovers it and binds a driver. You rely on this every time you plug anything in.

The DSR is that pattern, minus the operating system's help, on bare metal. A TI peripheral's `>AA` header *is* its device descriptor — "I am a valid device; here are my names and my routines" — and the console's boot scan *is* the enumeration, and `DSRLNK` *is* the driver binding, walking the descriptors to find the one that answers to `DSK1.` and calling its code. The difference is that on the TI there is no driver *database* and no drivers *in the OS at all* — the driver comes with the device, in the device's own ROM, so binding is not "find and load a driver" but "find and call the driver that is already here." That is arguably a *cleaner* model than the PC's — the device is self-describing *and* self-driving — and studying it is studying device enumeration reduced to its irreducible core: a signature, a list of names, and a call. Everything USB does, the DSR does in a few hundred bytes you can read.

## 30.1 The DSR Idea

The mechanism rests on a single shared window and a switch. Every peripheral card's DSR ROM appears at the *same* CPU addresses — **`>4000`–`>5FFF`**, the 8 KiB "DSR ROM window" in the memory map (Chapter 5) — and, because they cannot all be visible at once, each card's ROM is **paged in** by that card's own **CRU bit**: set the card's enable bit (Chapter 10's CRU) and its ROM appears at `>4000`; clear the bit and it vanishes, leaving the window for the next card. One window, many cards, multiplexed by CRU. The project's disk controller is exactly this: its DSR ROM is gated by CRU base `>1100` bit 0, and until that bit is set the `>4000` window reads as open bus (`disk.rs`).

So "using a peripheral" is a four-beat dance: **find** the card whose DSR answers to your device (scan the CRU bases, page each in, check for the signature), **page it in** (set its CRU bit), **call** its routine, and **page it out** (clear the bit) so the window is free for the next device. The console does this at boot to discover cards, and every device access repeats it. It is a deliberately frugal design — 8 KiB of address space serves an entire expansion box of cards, because no two are ever mapped at once — and it means DSR code must be **position-fixed** at `>4000` (every card's ROM is linked to that address, since that is where it always appears) and must not assume it is alone (another card's ROM will occupy the same window moments later). The CRU bit is the whole multiplexing mechanism: one bit per card, one card at a time, one shared window. Frugal, and — as §30.4 shows — simple enough to drive yourself.

## 30.2 The DSR Header: `>AA` and the Linked Lists

A paged-in DSR ROM introduces itself with a header at `>4000` that is the sibling of the cartridge header of Chapter 27 — the same `>AA` signature, the same linked-list idea — now describing a *device* rather than a program. The disk controller's header, probe-pinned in the project's reconstruction (`RECON.md`), is the concrete example:

```text
>4000  >AA            signature: a valid DSR
>4001  >02            version
>4002  >00            number of programs (0 for the disk card)
...
>4004  power-up list  -> routines run at boot (one unnamed node -> >4070)
>4008  device list    -> the named devices: DSK, DSK1, DSK2, DSK3
>400A  subprogram list-> numbered routines >10,>11,...,>16 and named FILES
```

The header holds **four linked lists**, each a chain of nodes. The **device-name list** (headed at `>4008`) names the devices this card provides — the disk card offers `DSK`, `DSK1`, `DSK2`, `DSK3` — each node being `[link][entry-address][name-length][name]`, so a walk of the list, comparing your target name to each node's name, finds the routine that serves `DSK1.`. The **subprogram list** names numbered routines (`>10` for direct sector I/O, `>16` for file enumeration, and others) plus named ones (`FILES`) — the low-level operations beneath the file abstraction (Chapter 32). The **power-up list** holds routines the console runs at boot, before the menu — the disk card's power-up node reserves its VRAM buffer and lowers the top-of-VRAM pointer (`>8370`), staking the memory it needs. And a card ISR list lets a card hook the interrupt chain (§30.5). **Name matching** is a string compare against the device-list nodes: the console parses the device name from your request (the part before the first `.`, so `DSK1.MYFILE` yields the device `DSK1`), and walks the list looking for a node whose name matches. It is the cartridge menu's scan (Chapter 28) applied to devices — a signature, a list, a name compare — and the project's boot verifies it end to end: the power-up scan finds the disk card's `>AA` header and reserves its VRAM buffer, machine-checked in `device_io.rs`.

## 30.3 CRU Geography of an Expansion Box

A loaded Peripheral Expansion Box holds several cards, and they share the CRU address space by convention, each card assigned a **CRU base** — a range of CRU bits it owns. The classic map, from the hardware record:

```text
>1100   Disk Controller        >1300   RS-232 card (first)
>1200   (reserved / other)     >1500   RS-232 card (second) / PIO
>1400   (various)              >1x00   ... one base per card, steps of >0100
```

Each card's DSR-enable bit lives at its base (bit 0), and a card's other CRU bits — motor control, drive select, interrupt masks — sit just above it (the disk card's drive-select and side bits at `>1100` +4…+7, Chapter 10). **Polite multi-card citizenship** is the discipline this geography demands: a card responds *only* to its own base, pages in *only* when its bit is set, and pages *out* the instant it is done, so the shared `>4000` window and the CRU space stay uncontended — a card that left itself paged in would collide with the next, and a card that answered outside its base would corrupt its neighbor. The `DSRLNK` scan of §30.4 relies on this: it steps through the bases (`>1000`, `>1100`, …), pages each in, and trusts that only the right card responds at each.

Here the emulator's scope (R-12) is worth stating plainly. The project models the CRU at `>1100` (the disk card) and the console's own TMS9901; other bases **idle** — there is no RS-232 card at `>1300`, no second card at `>1500`, because the `>4000` window is wired to the disk controller specifically. So the base *map* above is the real hardware's (and Classic99's, and MAME's, which model the whole box); the project verifies the *mechanism* at `>1100` and leaves the wider box to the shelf emulators. When you write DSR-scanning code (§30.4), it is correct against real hardware and the full-box emulators, and against the project it finds the one card the project models — which is exactly the disk card the file chapters need.

## 30.4 `DSRLNK` from the Inside

E/A gives you a utility called `DSRLNK` and most programmers treat it as a spell: set up a block, call `DSRLNK`, hope. We will not. `DSRLNK` is just the four-beat dance of §30.1 written as a subroutine, and writing our own — `dsrlib`'s `DSRLNK` — makes it cart-safe (no E/A vectors assumed, Chapter 23) and demystified. The skeleton is exactly the scan:

```asm
DSRLNK MOV  R11,R9           save the caller's return
       LI   R2,>1000         first CRU base
DLCARD MOV  R2,R12           this card's base
       SBO  0                page its DSR ROM in at >4000
       CB   @DSRROM,@DLAA     is >4000 = >AA (a valid DSR)?
       JNE  DLOFF            no -> page out, next base
       MOV  @DSRNL,R4        walk the device-name list at >4008
       ...                   compare our name to each node
       BL   *R6             matched: call the entry (R12 = CRU base)
       JMP  DLLINK          PLAIN return = not handled -> keep looking
*      SKIP return (handled) lands here: page out, done
```

The one genuinely subtle beat is the **skip-return**, and it is the pinned detail (`RECON.md`): a DSR that *handled* your call returns by *skipping* the instruction after the `BL` (returning to `R11+2` beyond the normal point), while a DSR that did *not* handle it returns normally — so the caller places a "keep looking" jump right after the `BL`, which a handled call skips over. It is the DSR's way of saying "yes, that was mine" versus "not me, try the next," in the return address itself. Our `DSRLNK` calls the entry with `R12` set to the card's CRU base (the DSR needs its own base to touch its CRU bits) and interrupts masked (`LIMI 0`, since a DSR touches hardware that an ISR must not race), honors the skip-return, and pages the card out before returning.

Two honesties (R-12). First, `dsrlib`'s `DSRLNK` **assembles** with `libre99asm` and shows the mechanism's bones, but a `DSRLNK` that actually *drives the disk DSR* must also stage the PAB pointer into the GPL workspace and pad cells the DSR reads (`>8355` name length, `>834A` the name, `>8356` the VDP address past the name; the PAB base is `[>8356] − devlen − 10`) — the full parameter contract, probe-pinned in `RECON.md` and machine-verified in `device_io.rs`, which Chapter 31 uses in earnest. Second, **BENCH99 cannot run this**: the bench has no card at `>4000` (its window is open bus), so the DSR path is exercised through the desktop app (`--disk`) and the Rust harness, where the console's own `DSRLNK` finds the disk card and loads a game — verified, just not from a bench transcript. The point of writing our own is not to replace that verified path but to *understand* it: `DSRLNK` is no spell, just a scan, a call, and a skip.

## 30.5 Card-Side Interrupts

A peripheral often needs the console's attention *asynchronously* — an RS-232 card with a byte waiting, a card with an event to report — and for that a DSR hooks the **interrupt chain**. Recall the interrupt of Chapter 22: the console's ISR runs on `/INT2` (the VDP's 60 Hz), but the same interrupt line is shared by expansion cards, and a card that needs servicing asserts it. The DSR's **card-ISR list** (a header link) points to a routine the console's interrupt handler calls when servicing an interrupt — so a card installs, via its header, a piece of code that runs in interrupt context to service the card. The **RS-232 pattern** is the canonical one: the serial card, receiving a byte, asserts the interrupt; the console's ISR walks the card-ISR chain; the RS-232 card's ISR routine runs, reads the byte from the TMS9902 (Chapter 33) into a buffer, and returns — so incoming serial data is buffered by interrupt without the main program polling. It is the same ISR-hook discipline Chapter 22 taught for the console's own user-ISR hook (`>83C4`), now extended to the expansion bus: a card contributes a handler to the interrupt chain, and the console's ISR calls it. The project does not emulate the RS-232 card (§30.3, Chapter 33), so this pattern is from the hardware record and the shelf emulators; but the mechanism — a DSR-installed handler in the interrupt chain — is the same one the disk card's power-up and the console's ISR (verified, Chapter 22) already showed.

## 30.6 Writing a DSR of Your Own

The rite of passage is to write a DSR — to be, for once, the *device* rather than its user. `NULCARD` is the exercise: a minimal DSR that answers to the device name `NUL.`, provides one trivial subprogram, and prints a power-up message — a card that does nothing but prove it was found, called, and dismissed. Building it is building a `>4000`-based ROM with a valid `>AA` header, a device-name node for `NUL`, an entry routine (that, say, logs the call and returns via the skip-return to signal "handled"), and a power-up node — the header choreography of §30.2 from the *producing* side. It is the DSR mirror of Chapter 27's cartridge: there you built a program the menu discovers; here you build a device the `DSRLNK` scan discovers.

Where it runs is the honest question (R-12). On **real hardware** a `NUL.` DSR needs a card to live on — a GRAM-style or RAM peripheral that presents ROM at `>4000` on a CRU base — and on the **full-box emulators** (Classic99, MAME), which model the general expansion bus, `NULCARD` can be installed at a spare CRU base and found by a scan. On **this project's emulator**, the `>4000` window is the disk card's (§30.3), so `NULCARD` is not installable as a separate card today — a real and stated limitation, and a clean roadmap item (a general DSR-card slot in the core would let the book's `NULCARD` run here, and would let BENCH99 grow a `dsr` command). So you *build* `NULCARD` (it assembles; its header is correct by construction, checkable byte-by-byte against §30.2), you *run* it on the shelf emulators or hardware that model the box, and you *understand* it as the exact inverse of the `DSRLNK` you wrote — the two halves of the handshake, author and caller, both now yours.

## Lab 30 — `dsrlib` and `NULCARD`

The lab is both ends of the DSR handshake, in `code/ch30/`.

**`dsrlib` (`dsrlib.inc` + `dsrlib.a99`)** — our own `DSRLNK`: the CRU-base scan, the `>AA` check, the device-name walk, the call with the skip-return, the page-out. Build it:

```sh
libre99asm code/ch30/dsrlib.a99 --format bin -o build/DSRL.bin
```

It assembles clean, and it is the mechanism of §30.4 made concrete — no spell, a scan. Its behavior against the *real* disk DSR is what `device_io.rs` verifies (the `>AA` scan, the `DSK1` link, the Tunnels-of-Doom QUEST load), green at HEAD; the exercises add the full PAB/pad staging (§30.4) that drives an actual device, which Chapter 31 builds on.

**`NULCARD`** — the DSR you author: a `>4000` ROM with a valid `>AA` header, a `NUL` device node, an entry, and a power-up message. Assemble it and verify its header byte-by-byte against §30.2 (the `>AA`, the list pointers, the `NUL` node); run it on a full-box emulator or hardware to see the scan find it. It is your first device — the machine discovering *your* card the way it discovered the disk controller.

> **Sidebar — Eight pounds of steel and the fire hose.** The **Peripheral Expansion Box** — "the PEB" — is where all these cards lived: a hulking beige steel enclosure, roughly the size and weight of the console itself (eight-plus pounds empty), with slots for eight cards and its own power supply and fan. It connected to the console by a broad, stiff ribbon cable that fanned out from the side of the machine — a cable so thick and unwieldy that the community christened it the **"fire hose."** The PEB was TI's answer to expansion done *right* — a proper backplane, buffered, powered, with room to grow — and it was magnificent and absurd in equal measure: to add a disk drive to your home computer you bolted on a second box heavier than the computer, joined by a cable you could have watered a garden with. It is beloved now precisely for that overbuilt seriousness — a home computer's peripherals housed like industrial equipment — and the cards this chapter describes are the cards that slotted into it, each introducing itself through the fire hose with a byte and a list. The steel outlived the company; PEBs still hum in collections today, fans turning, waiting for a card to page itself in.

## Exercises

**30.1** ✦ What are the four beats of using a peripheral through its DSR? Which one uses the CRU, and what does the CRU bit do?

**30.2** ✦ A DSR ROM begins with `>AA`. Where is the device-name list head, and what are the fields of one device-name node?

**30.3** ✦✦ Explain the skip-return: how does a DSR signal "I handled this call" versus "not mine, keep looking," and where does the caller put its "keep looking" jump?

**30.4** ✦✦ Extend `dsrlib`'s `DSRLNK` with the full parameter contract of §30.4: stage the device name and PAB pointer into the pad cells (`>8355`, `>834A`, `>8356`) the disk DSR reads. Check your staging against `RECON.md`'s pinned values.

**30.5** ✦✦ The disk card's power-up node reserves a VRAM buffer by lowering `>8370`. Why must a card claim its memory at power-up rather than when first used, and what breaks if two cards claim the same VRAM?

**30.6** ✦✦✦ Build `NULCARD`: a DSR answering to `NUL.` with a power-up message and one subprogram that returns "handled." Verify its header byte-by-byte against §30.2, and (on a full-box emulator or hardware) call it from a test program with your `DSRLNK`.

**30.7** ✦✦✦ Sketch how the RS-232 card buffers incoming bytes by interrupt: which list installs the handler, what the handler does, and why it must be fast and interrupt-safe (Chapter 22).

## Further Reading

- `original-content/system-roms/disk-dsr/RECON.md` — the project's probe-pinned reconstruction of the disk DSR: the header chains, the `DSRLNK` calling convention, the pad contract, the on-disk format (Chapter 32).
- `crates/libre99-gpl/tests/device_io.rs` and `crates/libre99-core/tests/disk.rs` — the machine-verified DSR/FD1771 path this chapter rests on (green at HEAD).
- Chapter 27 (Writing GPL Today) — the cartridge header, the DSR header's sibling; the same `>AA`-and-list pattern.
- Chapter 10 (The CRU) — the bit-serial I/O the DSR window is paged by.
- Chapter 22 (Interrupts and Time) — the interrupt chain a card ISR hooks (§30.5).
- Chapters 31–32 (File I/O; Disk Internals) — the PAB and the subprograms the disk DSR provides, driven through this chapter's `DSRLNK`.
- The Classic99 and MAME documentation — the full-box emulators that model the whole PEB and its several cards.

## Summary

A TI peripheral **brings its own driver** — a **Device Service Routine** in ROM on the card — and introduces itself to the console with an `>AA` signature and linked lists, so the machine *discovers* devices rather than containing their drivers: plug-and-play, in 1983. Every card's DSR ROM appears at the shared **`>4000`–`>5FFF`** window, **paged in one card at a time by that card's CRU bit** (the disk card at CRU base `>1100`, verified), so using a device is a four-beat dance — **find** (scan the CRU bases, check for `>AA`), **page in** (set the bit), **call**, **page out** — and DSR code is position-fixed at `>4000` and must never assume it is alone. The header holds four linked lists — **device names** (`DSK`, `DSK1`…, matched by a string compare on the part before the first `.`), **subprograms** (numbered sector/file routines), **power-up** routines (run at boot; the disk card reserves its VRAM buffer), and **card ISRs** (hooking the interrupt chain, the RS-232 buffering pattern) — the device sibling of Chapter 27's cartridge header. Cards share the CRU space by a **base map** (`>1100` disk, `>1300`/`>1500` RS-232, …) and practice polite citizenship (respond only to your base, page out when done). **`DSRLNK`** is not a spell but this scan written as a subroutine — `dsrlib`'s own version (find, `>AA`-check, name-walk, call with `R12` = the CRU base and `LIMI 0`, honor the **skip-return** by which a DSR signals "handled," page out) — and writing your own makes it cart-safe and clear. The whole mechanism is machine-verified against the project's disk DSR (the boot `>AA` scan, the power-up VRAM reservation, a `DSK1` link, and a real game loaded from a real disk — `device_io.rs`, `disk.rs`, green at HEAD; the RECON reconstruction pins the calling convention), through the Rust harness and the desktop app rather than BENCH99 (which has no card at `>4000` — a stated R-12 gap), and the emulator models the disk card specifically rather than a general expansion bus, so authoring your own card (**`NULCARD`**, answering to `NUL.`) runs on the full-box emulators (Classic99, MAME) or hardware — the two halves of the handshake, caller and device, both now yours. It is USB device enumeration reduced to its core — a signature, a list of names, a call — and every peripheral in the chapters ahead introduces itself exactly this way.
