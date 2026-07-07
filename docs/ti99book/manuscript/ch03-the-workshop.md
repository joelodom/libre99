# Chapter 3 — The Workshop: A Modern Development Environment

*Set up once, used for 42 chapters. Cross-development first; the period workflow gets its honors later (Ch. 6).*

<!-- Part I · target 20 pp · prerequisites: Ch. 1–2 · Lab: full toolchain bring-up + HELLO shipped to every target -->

## Prologue: Two Rooms, 1982

Room one is a bedroom in Ohio. The developer is fifteen; the machine is the family 99/4A, promoted to development workstation by an Editor/Assembler cartridge, a Peripheral Expansion Box bought with two summers of lawn money, and one disk drive. The workflow is a ritual: load the Editor (wait), type assembly source into a 40-column screen, save to disk (wait), quit the Editor, load the Assembler (wait), assemble — the drive chattering through two full passes (wait, minutes) — then load the object file, run it, and watch the machine lock solid because R11 got clobbered. Power off. Power on. Load the Editor (wait). Every edit-run cycle costs five to ten minutes, and every crash takes the whole development environment down with it, because the environment *is* the target.

Room two is in Lubbock, and the professionals in it are not doing any of that. TI's own programmers write much of their code at terminals wired to TI-990 minicomputers down the hall — big machines that edit with real tools, assemble in seconds, and never crash because a sprite table overflowed. The code is *cross-developed*: built on a computer that is not a 99/4A, for a computer that is, then sent down the wire to the target for the only step that truly needs the real silicon — running it. The bedroom developer and the pro are writing for the same machine; only one of them is also writing *on* it, and the difference shows up directly in what each can attempt.

This chapter builds you room two. Your minicomputer is the laptop you already own; your wire is a build script; your target is a from-scratch software 99/4A that this book's own project maintains — plus a shelf of reference emulators for second opinions, and, if you choose, a real console with a flash cartridge that updates in seconds. The bedroom ritual is honored history — Chapter 6 recreates it faithfully, once, so you know in your hands what 1982 development cost — but it is not how this book works day to day. We work like Lubbock.

---

## What You Will Learn

After this chapter you can:

- State the cross-development strategy — edit and build on a modern machine, run emulated constantly, verify on hardware occasionally — and say why each leg exists.
- Build and run the project's own toolchain: the **`libre99`** desktop machine, the **`libre99asm`** assembler, and **BENCH99**, the scriptable lab bench over the same emulator core — and place **Classic99**, **js99er.net**, and **MAME** on the reference shelf where each belongs.
- Assemble a source file into the two ship formats — a **`.ctg`** cartridge for the project emulator and a raw **`.bin`** ROM image for everything else — plus the two instrument files (listing and symbol map) every debugging session leans on.
- Explain what the console's standard cartridge header is, field by field, and how the assembler synthesizes it for you.
- Assemble, run, and deploy a complete first program to every target, including (optionally) a real console via flash cartridge.
- Use this book's six debugging instruments — pause and frame-step, the live CPU inspector, bench breakpoints and traces, memory watch-and-poke, listings read against traces, and the run log — the literacy every later lab assumes.
- Choose real hardware sensibly by budget, including the answer to "the 32 K question."

## The Bridge: You Already Know This Shape

If you have ever touched embedded development — an Arduino sketch, a microcontroller class, a Raspberry Pi Pico — you already know this chapter's architecture, because modern embedded work *is* room two: cross-compile on the host, simulate when you can, flash the target, printf-debug over a wire. The TI-99/4A in 2026 is simply an embedded target with unusually good museums. Host = your machine; toolchain = a cross-assembler that emits 9900 bytes; simulator = an emulator faithful enough to be a measuring instrument; flashing = an SD card. Even the one-command-build discipline transfers unchanged, because tools you must remember how to drive are tools you stop driving.

Two habits *don't* transfer, so install them now. First: here, **the emulator is not a compromise, and it is not a black box.** The machine you will run all book long is a from-scratch, cycle-aware software 99/4A whose source sits in the same repository as this book — when Chapter 5 measures wait states or Chapter 22 profiles an interrupt handler, you can read the very Rust that modeled them, and the project cross-checks that model against the community's reference emulators and the original datasheets. "Runs in the emulator" is evidence, not hope; real hardware is for savoring and for catching the rare analog-world issue (video timing on a real TV, a marginal power supply), not for daily truth. Second: **the machine is also a library.** Because the emulator's core is ordinary code, this book can hand you the whole console as a scriptable object — set the program counter, step one instruction, read the workspace out of RAM, count cycles — powers no 1982 developer and few modern IDEs ever get. That instrument (BENCH99, §3.8) is how this book machine-verifies its own claims, and by Chapter 11 it will be how you regression-test yours.

## 3.1 The Strategy

Stated once, followed for the rest of the book:

**Edit and assemble on the modern machine.** Your editor, your terminal, your version control. Source lives in ordinary text files; the assembler is a fast native tool; builds take milliseconds. Nothing about writing 1981 software requires suffering 1981 ergonomics.

**Run emulated constantly.** The inner loop is one build command and a keystroke, seconds long, dozens of times an hour. The project emulator is the daily driver: it boots to the master title screen in about a second, mounts your fresh cartridge from the command line, and carries its own inspectors. The reference shelf — js99er in any browser, Classic99 with its veteran debugger, MAME with its slot-level rigor — supplies second opinions and the few capabilities ours doesn't have yet (§3.2).

**Verify on real hardware occasionally.** At milestones — end of a lab, end of a capstone — because CQ-82 item 9 means *the real artifact works on the real machine*, because analog output has character emulation only approximates, and frankly because dropping your own cartridge into 45-year-old silicon and seeing your name on a CRT is the paycheck this hobby runs on. The flash-cart path (§3.6) makes this a thirty-second ceremony, not a project.

One more strategic decision, made here and honored everywhere: **every target, one source tree.** Nothing we build is allowed to depend on a convenience only one loader has. The same assembled bytes ship as a `.ctg` for the project emulator and as a raw ROM image for js99er, Classic99, MAME, and the flash cart — and the build emits both, every time, so drift has nowhere to hide.

## 3.2 The Emulator on Your Bench — and the Reference Shelf

### `libre99` — the daily driver

The machine this book runs on is the **libre99 project**: a Texas Instruments 99/4A emulated from scratch in pure Rust — TMS9900 CPU with wait-state-counted timing, TMS9918A video, SN76489 sound, the GROM chips with their real prefetch quirk, the TMS9901/CRU keyboard, and the TI Disk Controller running the genuine disk DSR ROM. It does not reimplement the TI operating system; it makes the chips behave correctly and lets the **real console firmware** do what it did in 1981 — boot, draw the color-bars title, scan cartridge headers, build the master menu. Everything the machine needs — firmware, a library of 137 period cartridges, 15 disk images — is embedded in the binary, so setup is one command with nothing to find or configure:

```text
git clone <the libre99 repository>
cd libre99
cargo run --release -p libre99-app
```

A window opens at the master title screen; press a key, pick a cartridge, play. (The first build compiles dependencies and takes a few minutes; every build after is seconds. `--release` matters — it is what makes the emulation run at full speed.) The parts you will use daily: `--cartridge-file <path>` mounts a cartridge you built; `--cartridge <name>` and `--disk <name>` pick from the embedded library (`--list` prints it); **F9** opens an in-app media browser with each cartridge's title and ROM/GROM makeup; **F5** resets; **F6/F8** save and restore a whole-machine snapshot (the session also auto-saves on exit and resumes on launch); **F10/F12/Tab** are pause, frame-advance, and fast-forward; a **CPU inspector** overlays live registers and cycle counts; a keystroke saves a screenshot; and a leveled run log narrates device traffic when you ask it to (§3.8). The project's README is the emulator's manual and stays current as it grows; this book cites it rather than restating it.

Three honesty notes, because this is a living project rather than a finished museum piece. The Speech Synthesizer is not yet emulated (Chapter 20's labs will lean on the shelf or real hardware if the project hasn't grown a voice by then). Disks mean the embedded library for now — mounting arbitrary `.dsk` files from your own build is on the project roadmap, which is one reason the disk-based period workflow lives in Chapter 6 on Classic99. And interactive breakpoints in the windowed app are roadmap items too — which costs us nothing, because the same core drives BENCH99 (§3.8), where breakpoints, single-stepping, and memory pokes are a script away. When a gap matters to a lab, the lab says so and names the tool that covers it.

### The reference shelf

**Classic99** (Mike Brent — Tursi) is the community's veteran Windows emulator (happy under Wine elsewhere) and this project's own cross-checking reference: hardware-verified over decades, with the interactive debugger TI's engineers would have envied — breakpoints, watchpoints, VDP and GROM viewers, a CPU heat map that paints where time goes. Through a longstanding permission arrangement with TI's IP holders it ships with the console ROMs *and a library of original TI software* under license. It also bundles the Editor/Assembler cartridge, which is why Chapter 6's faithful recreation of the 1982 workflow — two-pass disk assembly, option 3 and option 5 loading, the tagged-object format — happens there. Keep it installed; when this book says "second opinion with a full GUI debugger," this is the shelf position it means.

**js99er.net** (Rasmus Moustgaard) is a complete expanded 4A in a browser tab — console, 32 K, disk, **speech**, even F18A modes — nothing to install, running the moment the URL loads. It opens our raw `.bin` cartridge images directly. Role: the zero-install machine (some readers will do half this course on a Chromebook), the demo machine (send someone a link and a file), and the stand-in for hardware we don't emulate yet.

**MAME**'s TI-99/4A driver (principally Michael Zapf's work) models the machine at slot-and-signal rigor, and it is the referee this project consults when a measurement is contested. It expects you to supply console ROMs yourself and has a configuration learning curve; we use it accordingly — not in the inner loop, but as the standard of accuracy behind Chapter 5's and Chapter 22's measurements. Installing it today is optional; Appendix L collects the invocations for when you want it.

**Which do I actually open every day?** The project emulator — it is the book's machine, and every lab is written against it first, with a one-line shelf equivalent noted where one matters. js99er when you're away from your own machine or need speech; Classic99 when you want its debugger's GUI or Chapter 6's period tooling; MAME when a number is worth a referee.

## 3.3 The Toolchain: `libre99asm` and Friends

The toolchain is the project's own, and it builds from the same repository in one step (the book's `setup.sh` does exactly this):

```text
cargo build --release -p libre99-asm -p libre99-app
```

The members, and their roles in this book:

- **`libre99asm`** — the TMS9900 cross-assembler and cartridge packager, and the star of Parts II–V and VIII. Its source language is Editor/Assembler-compatible — everything we write would assemble on the real 1981 tool with at most cosmetic changes (Ch. 6 demonstrates it) — and where we lean on a nicety beyond E/A, the text flags it `[libre99asm]`, per the style guide. Two conveniences matter immediately: the register names `R0`–`R15` are predefined (the E/A "R option," permanently on), and the standard cartridge header is synthesized for you (§3.7) so a source file's first line of *code* can be its first line. One command produces a bootable cartridge:

  ```text
  libre99asm hello.a99 --name 'HELLO, 1981' -o build/hello.ctg
  ```

  Add `--format bin` for the raw ROM image, `--listing <file>` for the address-and-bytes listing, `--symbols <file>` for a JSON symbol map. It also carries a disassembler — `libre99asm dis <image> [addr]` — which Part VI turns into an archaeology tool.
- **`libre99gpl`** — the GPL assembler and disassembler, for the console's *other* language. Sleeps until Chapter 27, then becomes the second star. (It has already shipped real firmware: the project's clean-room console GROM — original title screen, menu, and all — is written in it, a story Chapter 28 tells properly.)
- **BENCH99** — the lab bench: a two-hundred-line monitor over the emulator core, living in this book's companion code and built in Lab 3. It is the debugging instrument of §3.8 and the measurement rig of Chapter 5.
- **The xdt99 suite** (Ralph Benzinger) — the community's Python cross-tools: `xas99` (9900), `xga99` (GPL), `xdm99` (disk images). Ours is the book's toolchain, but xdt99 remains the lingua franca of the wider TI development world, and two of its jobs stay on our shelf: producing the *period interchange formats* (tagged object files and EA5 images, Ch. 6) that `libre99asm` doesn't emit yet, and manipulating `.dsk` disk images (Part VII). Install it when those chapters ask; nothing before Chapter 6 needs it.

What `libre99asm` deliberately is *not* (today): a macro assembler (neither was E/A — Ch. 11 discusses what macros buy and what they hide), a relocating linker, or an EA5/tagged-object emitter (Ch. 6's formats, via the shelf). The assembler is young and grows with this book — its full specification lives in the repository (`assembler/ASSEMBLER.md`), and more than once in later parts, a chapter's needs will be the reason it grew a feature. That is what "foundation" means here: when the book and its tools disagree, one of them gets fixed, in public, in the same repository.

## 3.4 The Editor, and the Skeleton

Any programmer's editor works; what you want from it is 9900 syntax highlighting and a build keybinding. Community-made highlighters for TMS9900 assembly exist for VS Code, Vim, Emacs, and IntelliJ (search your editor's registry for "TMS9900" or "xdt99" and take the maintained one — the dialects are close enough that E/A-style highlighting fits our source unchanged). Configure two conveniences and you're done: run the build on a keystroke, and treat `.a99` as assembly.

More important than the editor is the **project skeleton**, because every lab from here to Chapter 43 is a copy of it:

```text
project/
  build.sh             the whole build, one command
  src/
    hello.a99          the program
  build/               generated artifacts land here (git-ignores itself)
```

And the build script that animates it — read it once now, use it forever:

```sh
#!/usr/bin/env sh
# build.sh — assemble one program into every artifact.  sh + cargo only.
set -e
LIBRE99ASM=../../target/release/libre99asm     # the project's assembler (setup.sh built it)
NAME=HELLO                               # TI-side name: <=10 chars, uppercase
TITLE='HELLO, 1981'                      # the console menu line

mkdir -p build
$LIBRE99ASM src/hello.a99 --name "$TITLE" -o build/$NAME.ctg \
         --listing build/$NAME.lst --symbols build/$NAME.map.json
$LIBRE99ASM src/hello.a99 --name "$TITLE" --format bin -o build/${NAME}C.bin
echo "built: $NAME.ctg  ${NAME}C.bin  (+listing, symbol map)"
```

Four lines of build, four artifacts (§3.5). Two conventions to notice: the `LIBRE99ASM` variable points at the assembler the repository built, so the script runs identically on macOS, Linux, and Windows-with-Git-Bash; and the raw image takes the community's **`C` suffix** (`HELLOC.bin`), the naming flash cartridges and other emulators use to mean *CPU ROM at `>6000`* (a `G` sibling would mean GROM — Chapter 27's world). A `run:` step is Exercise 3.6's five-minute pleasure.

## 3.5 The Build Artifacts

You will build four kinds of file all book long. Two are **ship formats** — packagings a machine can load — and two are **instruments** — files about the program, for you and your tools:

**`.ctg` cartridge** (`HELLO.ctg`). The project emulator's native cartridge container (a format inherited from the veteran `ti99sim` emulator): an 80-byte banner carrying a title, then the ROM — and, for the hybrid cartridges of Part VI, GROM — images, lightly compressed. This is what `--cartridge-file` mounts and what the F9 media browser reads titles from. One subtlety worth learning on day one: the banner title is *emulator-side* metadata; the name the **console's own menu** shows comes from bytes *inside the ROM* — the `>6000` header of §3.7. `libre99asm` sets both from `--name` so they never drift.

**Raw ROM image** (`HELLOC.bin`). The universal form: the literal bytes of the `>6000`–`>7FFF` cartridge window, padded to the full 8 K slot (sockets, unlike files, don't have lengths — ours came out 8,192 bytes exactly, machine-checked). Classic99 loads it as a user cartridge, js99er opens it in the browser, MAME wraps it, EPROM programmers burn it, and the FinalGROM 99 runs it on real silicon. Multi-bank cartridges (Ch. 35) are this format repeated per bank.

**Listing** (`HELLO.lst`). Your source with the assembled truth beside it: each line's address and emitted bytes. This is the Rosetta stone between the debugger's world of addresses and your world of labels — generate it for anything you intend to debug, which is why the skeleton always does.

**Symbol map** (`HELLO.map.json`). Every label and its resolved address, as JSON. Trivial today; indispensable the moment a *tool* needs your addresses — Lab 4 feeds `START` to the bench from it, and Chapter 11's regression harness automates on it.

Two ship formats are two fewer than the platform's own count, and the difference is deliberate. The era had four native package formats — cartridge image, **tagged object file** (the E/A assembler's relocatable, symbol-bearing output), **EA5 program image** (a memory snapshot loaded from disk), and the **`.dsk` disk image** that carried the other two. The last three belong to the disk-based workflow this book reaches in Chapter 6 (where you will produce and load them with the period's own Editor/Assembler, honoring room one) and Part VII (where disks get dissected sector by sector). Until then, cartridges are not a simplification — they are how most commercial TI software actually shipped, and they run on a bare console, which no disk format can say.

> **Sidebar — The $1,500 IDE.** What did room one actually cost in 1982 dollars? Price the bedroom developer's rig from period lists: the Editor/Assembler package (cartridge, manual, disks) at around a hundred dollars; the Peripheral Expansion Box a few hundred more before it held anything; the 32 K card and a disk controller card each in the low hundreds; one drive, then the second drive everyone bought once assembling onto the same disk as the source proved suicidal; a printer for listings, because debugging by 40-column screen alone is a special punishment. Sum: on the order of **$1,500–2,000** — several times the console's own late-war price, or, run through Exercise 1.2's inflation math, roughly a mid-range gaming PC plus a very nice monitor today. And the *manual* was the crown jewel: the Editor/Assembler manual's few hundred pages were the platform's de-facto systems documentation, so complete that this book cites it constantly and the community reprints it still. The kicker worth sitting with: the teenager who fought that workflow and the professional at the 990 terminal produced *the same bytes*. The machine never knew which room you were in. Yours won't either — it will simply wonder why you iterate four hundred times faster than either of them.

## 3.6 Real Hardware on Your Desk, 2026

Nothing in this book *requires* vintage iron — every lab specifies emulator behavior — but most readers eventually want the paycheck ceremony, so here is the sane shopping guide, in tiers. (Prices float with the collector market; treat tiers, not dollars, as the advice. The community marketplaces and the faires of Ch. 45 are where this gear actually changes hands.)

**Tier 0 — $0, today.** The project emulator on the machine you already own, and js99er.net in a tab. Between them: a full standard system, a debugger, and a period software library. Several readers will finish Part IX here and owe the hobby nothing but electricity.

**Tier 1 — a console on the desk.** A 99/4A console (they made 2.8 million; working units are plentiful and cheap by retro standards), a power supply, and a composite video cable for the round DIN jack into any modern display that still speaks composite — or a cheap composite-to-HDMI box. Add the single best first purchase in the ecosystem: a **FinalGROM 99** flash cartridge, which turns an SD card into every cartridge ever made plus every cartridge *you* make — our `HELLOC.bin` runs on real silicon tonight, and Part VIII ships to it natively. This tier runs all `[console-only]` and `[cart]` material, which is most of Parts II–VI.

**Tier 2 — the standard system.** The book's baseline (console + 32 K + disk) without the furniture, via either modern sidecar: a **TIPI** in its 32 K sideport edition — expansion RAM plus a Raspberry-Pi-backed disk system whose "drives" are folders on the Pi, plus the network doorway Chapter 34 walks through — or a **nanoPEB/CF7+**, which packs 32 K and CompactFlash "floppy" volumes into one small box. Either makes file I/O and all of Part VII first-class on hardware. TIPI is the community's current default recommendation; the nanoPEB path is venerable and self-contained.

**Tier 3 — the collector's bench.** The Peripheral Expansion Box itself with period cards (for the full 1982 experience Chapter 6 honors), a **Speech Synthesizer** sidecar (often inexpensive, and Chapter 20 is twice the fun with the real voice in the room — doubly so while the project emulator is still mute), and the **F18A** VDP replacement, which requires opening the console and socketed soldering-level confidence but rewards it with crisp VGA output and the enhanced modes of Chapters 18/34/44.

Two buying notes. First, *the 32 K question* answered plainly: yes, you want it (Tier 2) — the bare console is a wonderful constraint to *program for* (Ch. 40) and a frustrating one to *develop on*. Second, console revisions: any 4A serves, but the late beige consoles include the v2.2 ROM whose third-party lockout Chapter 1 described; modern flash devices navigate it, but if you're choosing at a swap meet, the classic black-and-silver is the version with no asterisks (details in Ch. 35).

## 3.7 First Light

Time. Here is `src/hello.a99` — a complete TMS9900 program, some two dozen instructions, that clears the screen, prints its greeting, sets the backdrop dark blue, and spins forever. Type it in (typing, not pasting, is the point today), then read the stanza glosses below; *full* understanding is Chapters 4–12's job, and this listing is deliberately written so that every one of its mysteries has a scheduled appointment.

```asm
* HELLO, 1981 — first light                              (Ch. 3)
* Assumes the standard character set is already in VDP RAM —
* true when started from the master menu (as here) and under E/A.
* The honest cold-start story belongs to Ch. 35.

VDPWD  EQU  >8C00            VDP write-data port    (Ch. 2 map)
VDPWA  EQU  >8C02            VDP write-address port

START  LIMI 0                interrupts off: the machine is ours
       LWPI >8300            workspace = the fast pad (squatting; see text)

* --- clear the 768-cell screen (name table at >0000) --------
       LI   R0,>0040         VDP address >0000 + write bit, byte-swapped
       MOVB R0,@VDPWA        send low address byte   (>00)
       SWPB R0
       MOVB R0,@VDPWA        send high byte + >40: VDP poised to write at 0
       LI   R1,>2000         ASCII space, parked in the HIGH byte
       LI   R2,768           one full 32 x 24 screen
CLS    MOVB R1,@VDPWD        write a space; VDP address auto-increments
       DEC  R2
       JNE  CLS

* --- say it -------------------------------------------------
       LI   R0,>6A41         screen cell >016A (row 11, col 10) + write bit
       MOVB R0,@VDPWA
       SWPB R0
       MOVB R0,@VDPWA
       LI   R3,MSG
       LI   R2,11
PUT    MOVB *R3+,@VDPWD      one character per write, auto-incrementing
       DEC  R2
       JNE  PUT

* --- sign the frame: backdrop to dark blue ------------------
       LI   R0,>0487         value >04 into VDP register 7 (>80 + 7)
       MOVB R0,@VDPWA
       SWPB R0
       MOVB R0,@VDPWA

HERE   JMP  HERE             spin forever; admire

MSG    TEXT 'HELLO, 1981'
       END
```

The stanza glosses — enough to make the code honest, no more:

**Where's the entry point declared?** Nowhere — by convention. `libre99asm` looks for a symbol named `START` (or the operand of `END`, or `--entry`) and aims the cartridge header at it. This book's programs all enter at `START`, which is why the label is the first thing after the equates. (The E/A world declared entry names with a `DEF` directive for its linking loader — a piece of Chapter 6's world that our cartridge path simply doesn't need.)

**`LIMI 0` / `LWPI >8300`.** Interrupts silenced (no console ISR, so no screen-blank timer, no surprises — Ch. 22) and the workspace pointer aimed at scratchpad, so all sixteen "registers" this program uses are fast-island bytes (§2.3's law, obeyed in our first two instructions). We are squatting on addresses the suspended OS considers its own and never returning — the polite treaty is Chapter 24.

**The address dance.** Talking to VDP RAM means loading the VDP's internal address register by writing two bytes to `>8C02` — low byte first, then the high byte with `>40` OR'd in to mean *writing* — after which every byte moved to `>8C00` lands at that address *and advances it*. That auto-increment is why both loops contain no address arithmetic at all, and it is the platform's most-used hardware favor (Ch. 12 formalizes the whole protocol into `vdplib`).

**`MOVB` and the high byte.** The 9900 is big-endian and its byte instructions operate on a register's *most significant* byte — which is why the space character is loaded as `>2000`, why address bytes get `SWPB`-swapped between sends, and why `>6A41` reads "backwards." File under Chapter 7; for today, trust the pattern.

**The message loop.** `MOVB *R3+,@VDPWD` — move a byte from where R3 points, then advance R3 — is your first taste of the 9900's addressing modes doing a loop's work inside one instruction (Ch. 7's subject, and the seed of Ch. 13's real text engine). The screen shows ASCII directly because the standard character patterns sit at their ASCII positions — TI BASIC's famous `+>60` screen-code offset is *BASIC's* convention, not the machine's (a distinction Ch. 13 and Ch. 28 both return to).

**Build and run, every way.** `sh build.sh`. Then:

1. **The project emulator** — `cargo run --release -p libre99-app -- --cartridge-file build/HELLO.ctg` (from the repository root; or point at your own path). The console boots, you press a key, and the master menu now reads `2 FOR HELLO, 1981` — your program, enumerated by the real 1981 firmware exactly as Chapter 28 will explain. Press 2. Dark blue; your greeting; the paycheck's first installment.
2. **js99er.net** — open the site, load `build/HELLOC.bin` as a cartridge. Same menu, same greeting, zero install: this is the artifact you can send to anyone.
3. **Classic99** — load the same `.bin` as a user cartridge. (Worth doing once this week just to meet its debugger with a program you understand.)
4. **Real hardware** — copy `HELLOC.bin` to the FinalGROM's SD card, insert, power on, select. Nineteen-eighty-one silicon, twenty-twenty-six bytes, your name on glass in the next lab.

**The header you didn't write.** The console's menu found your program because the assembler placed the platform's standard cartridge header at `>6000` ahead of your code. Dump the first 32 bytes of `HELLOC.bin` and you are looking at the whole contract (Chapter 28 walks the GROM code that reads it; Chapter 35 writes fancier ones — multiple entries, power-up hooks — by hand):

```text
>6000  AA 01 01 00   >AA "valid" mark; version; program count; reserved
>6004  00 00         power-up list: none
>6006  60 10         program (menu) list lives at >6010
>6008  00 00 00 00   DSR list, subprogram list: none
>600C  00 00 00 00   interrupt hook, spare: none
>6010  00 00         menu entry: no next entry (this is the only one)
>6012  60 20         ...entry address: START landed at >6020 (see listing)
>6014  0B            ...name length, 11
>6015  48 45 4C ...  ...the name itself: 'HELLO, 1981' (+ a pad byte to stay even)
```

Sixteen bytes of pointers, one list entry, your name, your address. When the menu scan of §2.2 finds `>AA` at `>6000`, this is what it walks — and everything your eye lands on here (the byte-swapped pointers, the length-prefixed string, the word alignment) is a Part II lesson already in the wild.

## 3.8 Debugger Literacy

Every lab from Chapter 4 onward assumes you can do the following six things without thinking. Learn them now, on HELLO, while the program is small enough that nothing is mysterious *except* what you're practicing. Two of the six live in the windowed emulator; the rest belong to **BENCH99**, the lab bench you build in Lab 3 — a tiny monitor over the same emulator core, driven by typed commands (interactively, or piped from a file, which is how labs make their measurements reproducible). Where a GUI equivalent exists on the shelf, it's noted once here; later chapters just say *break at X, watch Y, step until Z* in this vocabulary.

**1. Pause time, then step it.** In the windowed emulator: **F10** freezes the machine mid-frame ("PAUSED"), **F12** advances exactly one frame, **Tab** fast-forwards while held. Boot your cartridge, pause during the title fade, and frame-step into your own launch. Time on this machine is a sequence of 60 Hz frames (Ch. 17 budgets them); these three keys make that sequence tangible.

**2. Inspect the CPU, live.** Toggle the emulator's CPU inspector overlay (see the README or the F1 help for your platform's shortcut) and watch PC, WP, ST, all sixteen workspace registers, and the elapsed-cycle counter update as the machine runs. On HELLO it tells a complete story: PC pinned at `HERE`'s address, R2 counted down to zero, R3 parked one past the message — the aftermath of every loop you wrote, legible from orbit.

**3. Break and step, instruction by instruction.** On the bench: `u 605C` runs until PC reaches an address (your listing file says `PUT` landed at `>605C` — this is why we always generate listings), then `s 3` single-steps a full loop iteration, disassembling each instruction and pricing it in cycles as it goes:

```text
bench> load build/HELLOC.bin
bench> pc 6020            # START, straight from the listing
bench> u 605C
break at >605C after 2318 instructions (47940 cycles)
bench> s 3
>605C  D833 8C00      MOVB *R3+,@>8C00        44 cycles   ST=L> A> - C - - - ...
>6060  0602           DEC R2                  14 cycles   ST=L> A> - C - - - ...
>6062  16FC           JNE >605C               14 cycles   ST=L> A> - C - - - ...
```

That is a breakpoint, a trace, and a cycle profile in three commands — Chapter 4's lab grades your paper predictions against exactly this output, and Chapter 5 turns the cycle column into a laboratory. (Those prices already whisper Chapter 5's lesson: seventy-two cycles per character, most of them paid to *addresses*, not operations.)

**4. Watch memory — and vandalize it.** `r` prints the registers; `m 8300 32` dumps the same bytes as raw RAM, because on this machine *registers are memory* (Chapter 4's headline). Now poke: `pw 8304 000B` rewrites R2 — by its street address — and resuming (`x`) makes the program obediently print eleven more characters than its author intended. `screen` renders the VDP name table as text, so you can watch `H`, `E`, `L` appear in video RAM as you step the loop — an inspector on the *other* chip's private memory, the superpower no 1982 developer had. (Bench peeks and pokes are side-effect-free by design — they touch RAM without tripping the MMIO ports. §2.5's warning about debuggers *reading* ports stands for tools that read through the live bus.)

**5. Read the listing against the machine.** Keep `build/HELLO.lst` open beside the bench trace until the two views feel like one document — label to address, source line to emitted bytes. For images you didn't assemble (a console ROM, somebody's cartridge), `libre99asm dis <image> <addr>` produces the machine's side of the story cold, which is how Part VI reads TI's own firmware.

**6. Make the machine narrate.** Run the windowed emulator with `--log-level debug` and watch the run log (terminal, and `~/.libre99/libre99.log`) narrate device traffic — VDP register writes, GROM address sets, disk DSR calls — per the README's logging section. HELLO's backdrop write shows up as a VDP register event with your `>04` in it. When something misbehaves at 3 a.m., this log is the first witness you interview.

The shelf, for completeness: Classic99's GUI debugger does 1–5 with breakpoints, watchpoints, a VDP RAM viewer, and its famous heat map (worth an afternoon someday); MAME's debugger does all of it with reference rigor. The skills transfer — only the keystrokes differ.

> **Pitfalls.**
> - **A comment after a no-operand instruction needs `;`.** `RT   return to caller` fails to assemble — the assembler reads `return` as an operand (`RT takes no operands`). Write `RT   ; return to caller`, or leave the line bare. Instructions *with* operands take plain trailing comments, as every listing in this book does. `[libre99asm]` sharp edge; E/A's fixed operand tables let it guess, ours refuses to.
> - **`AORG` belongs to raw images, not cartridges.** In the default cartridge mode the assembler owns the layout (header first, your code after) and rejects `AORG` with `AORG needs absolute mode`. You'll meet absolute mode when we build things that *aren't* cartridges (the project builds its clean-room console ROM with it); until then, if you're reaching for `AORG >6000`, you're hand-rolling what `--name` already does.
> - **TI filename law, previewed.** Real TI storage names are ≤10 characters, uppercase, no extensions (`HELLO`, not `hello.bin`). Nothing enforces it on your build directory today, but Chapter 6's disks and every flash cart will — our `NAME` variable exists so you pick a lawful name once. (Text files crossing to real TI disks also carry no newline bytes — a Chapter 31 story.)
> - **`>` in the shell.** TI hex notation collides with your shell's redirect. Inside source files, `>` is safe and mandatory; on a command line, quote it (`--entry '>6020'` style) or use the tools' prefix-free forms. Our scripts quote anything dangerous.
> - **ROM provenance, said plainly.** This project embeds the authentic console firmware and a library of period cartridges *for use with this emulator*; they remain their owners' copyrighted works (the repository's LICENSE and README say exactly this), and this book's position is the community's: preserve, study, don't redistribute what isn't yours. Classic99's bundle is the fully-licensed exception, negotiated with TI's IP holders — a community treasure. And the project's answer to the question behind the question is Chapter 28's showpiece: a **clean-room console GROM**, written from scratch in GPL with `libre99gpl`, that boots an original title screen and menu with *no* TI bytes at all. Preservation ethics start here and run through Chapters 32 and 45.

## Lab 3 — Bring-Up, Then Make It Yours

*Goal: the complete workshop, proven end to end — then personalized, because CQ-82's spirit starts with caring about details nobody required.*

1. **Bring-up.** Clone the libre99 repository; run the book's `setup.sh` (it builds `libre99asm`, the desktop emulator, and BENCH99, then smoke-tests the assembler). Boot the emulator bare (`cargo run --release -p libre99-app`), press around the master menu, open the F1 help and the F9 media browser, and run one period title from Chapter 1's artifact hunt. *Checkpoint: the title screen, on your machine, from a build you made.*
2. **First light, every target.** Create the skeleton, type in `hello.a99`, `sh build.sh`, and run all four paths of §3.7 (project emulator, js99er, Classic99 if installed, flash cart if you're Tier 1). *Checkpoint: a screenshot of `HELLO, 1981` from two different emulators — the emulator's screenshot key produces the clean 256×192 frame — plus, Tier 1 readers, a photo of it on glass.*
3. **Greet yourself.** Change the message to greet you by name, *keeping the print loop honest*: update the length register and re-center the text by recomputing the `>016A` cell offset for your string (the formula is in the gloss; show your arithmetic in a comment). Rebuild; confirm the menu line changed too, and say why it did (§3.5 knows). If your name pushed past column limits, welcome to 32-column thinking — solve it your way and defend the choice in a comment.
4. **Change the signature.** Pick your own backdrop color from the 9918A's sixteen (App. D previews the palette; experiment freely — this is the fastest feedback loop you'll ever have on this machine) and re-derive the `>0487`-style constant for it in a comment.
5. **Instrument drills.** Perform all six §3.8 skills on your modified program. Deliverables: your bench transcript for drills 3 and 4 (the `u`-break at PUT, one stepped iteration, the R2 poke and its visible crime scene on `screen`), and one line each for the other four (what you watched, where).
6. **✦✦✦ stretch — break it on purpose.** Predict, in writing, what happens if you delete the `SWPB` in the *message* address dance and rebuild. Then do it, and use the bench (`screen`, `vdp`) to characterize what actually happened. Explain using only §3.7's glosses. (If your prediction was wrong in an interesting way, you've just done your first real 9900 debugging — write down the corrected model.)

*Deliverables into the lab journal: screenshots/photo, your modified `hello.a99`, the bench transcripts, and the stretch write-up. The journal habit itself is part of the toolchain — Part IX's postmortems are built from yours.*

## Exercises

**3.1 ✦** Close the book: name the two ship formats and two instrument files of §3.5, who loads or reads each, and the three period formats this book defers (to where?).

**3.2 ✦** Your `build.sh` run produced `HELLOC.bin` at exactly 8,192 bytes. Where does that number come from (§2.3's map answers), why must the file be padded to it, and which two artifacts in `build/` would change if you renamed the program — and which would not?

**3.3 ✦** The message sits at cell `>016A`. Compute the cell address for row 20, column 3; for the screen's dead center; and the row/column of cell `>02C0`. (32-column arithmetic is about to be daily life.)

**3.4 ✦✦** Trace the cartridge boot with Chapter 2 in hand: from power-on to your `START`, list each hand-off (ROM reset → GPL interpreter → menu scan finds `>AA` → the header's program list → entry) and which chapter of this book owns each step. One line per hand-off. (§3.7's header dump is your map.)

**3.5 ✦✦** Two names travel with your cartridge: the `.ctg` banner title and the header's menu string. Using §3.5 and §3.7, say who reads each, prove they can disagree (how would you *make* them disagree with the tools you have?), and explain which one a real 1981 console could ever see.

**3.6 ✦✦** Add a `run` step to `build.sh` that launches the project emulator with the fresh `.ctg` (the emulator takes `--cartridge-file`; js99er and Classic99 accept the `.bin` by file-open or drop). Document the exact command in a comment. (You've just automated the inner loop's last keystroke.)

**3.7 ✦✦✦** Design the regression check §11.6 will build: a piped BENCH99 script that proves "HELLO still prints correctly" with no human watching. Which commands, asserting what? Name two things your script would *miss* (rendering? color? timing?) and which instrument from §3.8 would catch each.

**3.8 ✦✦✦** Read real shipping code: the repository's `original-content/cartridges/titris/titris.asm` is a complete falling-blocks game, built with the same assembler you used today. Without trying to understand the game logic, answer from the source: where does execution enter and how did the assembler know? Which of §3.7's patterns (the equates block, the address dance, the workspace choice) appear within the first fifty lines? Record three conventions it follows that this chapter taught — and one thing it does that nothing has taught yet (note the chapter you suspect will).

## Further Reading

- **The project README** (repository root) — the emulator's user manual: every hotkey, flag, and file location §3.2 summarized. Tonight's reading; it stays current as the emulator grows.
- **`assembler/ASSEMBLER.md`** (in the repository) — the assembler's full specification: the source language, the directive set, the `.ctg` container, and the header synthesis rules behind §3.7's dump.
- **`docs/ARCHITECTURE.md`** (in the repository) — how the emulated machine is put together; Chapter 2's block diagram, as running code.
- **The Editor/Assembler manual** — the platform's systems bible begins earning that title next chapter; today, read its introduction against this chapter's workflow. (A copy lives in the repository's `assembler/` folder.)
- **xdt99 documentation** — the community toolchain's reference; skim now, install when Chapter 6 asks.
- **Classic99's manual** — short; the debugger section is the shelf's best afternoon.
- **Thierry Nouspikel's Tech Pages** — the console and cartridge-header entries, as a preview of the depth waiting behind §3.7's dump.
- Forward: **Appendix L** (toolchain quick reference) is this chapter compressed to two pages; it exists so you never re-read this chapter to find a flag.

## Summary

- Strategy (§3.1): edit/assemble modern, run emulated constantly, verify on metal at milestones; every target, one source tree — the same bytes ship as `.ctg` and raw `.bin`, always.
- The machine (§3.2): the **libre99 project** is the book's foundation — a from-scratch pure-Rust 4A (cycle-aware CPU, VDP, PSG, GROM prefetch, disk DSR) running the **real firmware**, with 137 cartridges + 15 disks embedded; daily driver via `cargo run --release -p libre99-app` + `--cartridge-file`; F9 media browser, F5 reset, F6/F8 savestate + auto-resume, F10/F12/Tab time control, CPU inspector, screenshots, leveled logging. Known gaps stated: no speech yet, embedded-disks-only, GUI breakpoints on the roadmap. Reference shelf: **Classic99** (licensed TI bundle, GUI debugger/heat map, Ch. 6's E/A home), **js99er** (zero-install, speech, F18A), **MAME** (referee).
- Toolchain (§3.3): **`libre99asm`** — E/A-compatible source in, `.ctg` (default) or `--format bin` out, `--listing`/`--symbols` instruments, R0–R15 predefined, auto-synthesized `>6000` header (name/entry from `--name`/`START`); extensions flagged `[libre99asm]`; plus `dis` disassembler. **`libre99gpl`** sleeps till Ch. 27. **BENCH99** = the scriptable lab bench over the same core. **xdt99** stays shelved for period formats (Ch. 6) and `.dsk` work (Part VII).
- Skeleton (§3.4): `src/` + `build/` + `build.sh` (`LIBRE99ASM`, `NAME`, `TITLE` variables); artifacts (§3.5): **`.ctg`** (native container; banner title ≠ console menu name — the menu name lives in ROM) and **`HELLOC.bin`** (8,192-byte padded `>6000` image; C-suffix convention; js99er/Classic99/FinalGROM/MAME) + `.lst` and `.map.json` instruments; tagged-object/EA5/`.dsk` deferred to Ch. 6/Part VII by design.
- First light (§3.7): 25-instruction HELLO (LIMI 0, WS=`>8300`, VDP address dance via `>8C02` +`>40` write bit, auto-increment writes via `>8C00`, MOVB-is-high-byte, VR7=`>04` dark blue); **entry = `START` by convention** (no `DEF`; E/A's loader story → Ch. 6); the synthesized header dissected byte-by-byte at `>6000` (machine-verified dump: `>AA`, lists, entry `>6020`, length-11 name); menu line `2 FOR HELLO, 1981` served by the real firmware.
- Debugger literacy (§3.8): six instruments — F10/F12/Tab time control; live CPU inspector; bench `u`/`s` break-and-trace with cycle prices; `r`/`m`/`pw`/`screen` watch-and-poke (registers-are-memory: R2 edited at `>8304`); listing-vs-trace reading (+ `libre99asm dis`); `--log-level debug` narration. Shelf GUIs (Classic99/MAME) do the same jobs; vocabulary fixed for all later labs.
- Hardware tiers (§3.6): $0 tier now includes the project emulator; console+**FinalGROM 99**; 32 K answer = **TIPI(32K)** or nanoPEB; PEB/Speech/F18A bench; v2.2-console caveat flagged.
- Pitfalls logged: `;` before comments on no-operand lines `[libre99asm]`; `AORG` = absolute mode only; TI filename law previewed; `>` vs the shell; ROM provenance stated honestly (project embeds authentic firmware for its own use; Classic99's bundle is licensed; the clean-room GROM is the ethical showpiece → Ch. 28).
