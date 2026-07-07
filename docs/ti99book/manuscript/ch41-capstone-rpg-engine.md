# Chapter 41 — Capstone III: The Data-Driven RPG Engine

<!-- Part IX — Case Studies: Recreating the Classics · target ≈30 pp · [disk, 32K] -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — pending review passes. DUNGEONS OF FATE's engine (versioned quest-file MOUNT + validation, RLE map decode/render, table-driven combat resolver with morale, 5-bit packed-prose unpacker) MACHINE-VERIFIED on BENCH99 against toolchain commit e5c4697: dungeons.a99 -> 8,192-byte single-bank image (entry >60BE, code+data ≈ 1.6 KB), deterministic 6-part self-test GREEN (VR7=>02, FAILID=0); TBUF round-trips "FATE" (46 41 54 45 00). The disk path (loadq.a99) ASSEMBLES and is verified at the Rust-harness tier (device_io.rs), not BENCH99 (no card at >4000, R-12). Code in code/ch41/. Builds on R-19 (Ch.39); adds R-20 (data-driven artifact = the test seam). -->
<!-- SPEC: 00-master-outline.md, "### Chapter 41 —" (lines 646–655), Part IX preamble (621–623). -->

## The Game That Shipped Empty

Somewhere in 1982 a child slid a cartridge into a TI-99/4A, and the screen asked
a question no other cartridge asked: *which* adventure? Not *press fire to
start* — **insert the disk with your quest on it**. The cartridge, it turned out,
did not contain a game at all. It contained something stranger and more durable:
a machine for *playing* games, and the games themselves lived on disks you could
copy, trade, and — if you were bold — write yourself.

Pop the cartridge open in your mind and you find no dungeon, no monsters, no
map, not a single line of the story. You find a reader. It knows how to draw a
grid of tiles, how to resolve a sword-swing against a goblin's hide, how to turn
a fistful of packed bytes back into the sentence *the door is locked*. What it
does *not* know — what it politely refuses to contain — is any particular
dungeon, goblin, or door. Those it reads, at run time, from a file. Feed it one
file and it is a haunted keep; feed it another and it is a pirate's cove. The
program on the cartridge never changes. Only the data does.

This is the oldest modern idea in software, and the TI had a first-party
cartridge built on it a full generation before anyone said "data-driven" with a
straight face. The two previous capstones baked their worlds into their code:
METEOR BELT's wave script and GRIDRUNNER 99's maze were *part of the program*,
recompiled every time the design changed. That is the normal way, and for an
arcade game it is the right way. But it has a ceiling. To ship a new level you
ship a new program. To let a *player* build a level you ship them an assembler
and wish them luck.

The cartridge that shipped empty broke that ceiling. It drew a line down the
middle of the software — engine on one side, content on the other — and across
that line it passed exactly one thing: a pointer to a file. This chapter is
about drawing that line, and about the second, quieter gift it turns out to
give. Because once your engine takes its world as *data* rather than *code*, you
can hand it a world you built for the express purpose of *checking its work* —
and a program that can be handed known data and asked for known answers is a
program you can prove correct. The separation that lets a player ship a new game
is the same separation that lets us ship a green light.

---

## What You Will Learn

After this chapter you can:

- **Split an engine from its content** across a single pointer, so one program
  plays many games and no new game needs a reassembly.
- **Design a versioned binary file format** — a self-describing header of counts
  and offsets over fixed-size records — and read it position-independently.
- **Mount untrusted data safely**: validate a magic number and a version, and
  *refuse* a malformed file instead of crashing on it.
- **Render a world from data** — an RLE-compressed tile map (Ch. 38's decoder)
  decoded and painted to the Graphics I name table by dimensions the file itself
  supplies.
- **Write a table-driven combat resolver**: a turn engine whose to-hit, damage,
  and morale all come from the quest, so balancing is a data edit.
- **Unpack five-bit prose** back to ASCII with a bit-walking decoder, and learn
  why the carry flag is the most fragile register on the machine.
- **Load a quest off disk** with `filelib`, save the party's progress in
  versioned slots, and state honestly where the bench can and cannot follow.

## The Bridge: Every Engine Is an Interpreter

You already know this pattern; you may not know that you know it. A web browser
is an engine; the page is data. A game made in a modern toolkit ships a runtime
and a pile of *scenes*, *prefabs*, and *assets* that the runtime plays. *DOOM*
(1993) put its levels in `WAD` files precisely so that the game and its worlds
could ship, and be replaced, apart — and thirty years of fan levels followed.
Go down one more level and every programming language you have ever used is an
engine (the interpreter or CPU) playing data (your program). The line between
code and data is the most reused idea in computing.

What makes it worth a chapter is that on a 3 MHz machine with a disk drive, the
idea is not free and not automatic — you have to *build* the line, byte by byte,
and every byte you spend on the format is a byte of engine you do not get to
write. So you design the format like an engineer, not a magpie: fixed-size
records for O(1) indexing, offsets instead of pointers so the file relocates for
free, a version number so tomorrow's engine can still read today's file. And
then, because you are a programmer and not merely an author, you notice the
second gift: the very seam you built for *content* is a seam you can inject
*tests* through. Hold that thought. It is the whole reason this chapter ends in
a green border.

## 41.1 The Archaeology of an Architecture

The genre-definer here is Tunnels of Doom, TI's 1982 dungeon crawler. As with
every case study, we begin by being honest about what we can and cannot know.
The Libre99 project ships **no image** of that cartridge — `cartridges/` is
empty by design, and it will stay that way (Ch. 1). We cannot instrument the
original in the debugger, because we do not have the original. So we do not
pretend to. What we reconstruct here is not a binary; it is an **architecture**,
and an architecture, unlike a sprite's blink pattern, leaves fingerprints you
can read without the ROM in hand.

Here is the fingerprint, and it is enough. The cartridge shipped with **more
than one adventure**, on media separate from the cartridge, and the *same*
cartridge played *all* of them; new adventures reportedly circulated later that
the unchanged cartridge also played. Sit with that single observable fact and
the entire design falls out of it by necessity. If one fixed program plays many
interchangeable worlds, then the program cannot contain any world — the worlds
must be *data the program reads*. If the worlds are files, they need a *format*.
If a player is to trust an old cartridge with a new file, the format needs a
*version*. Everything else — that the monsters, the map, the items, the prose,
and the difficulty knobs must all be *in the file*, because anything left in the
program could not vary between adventures — is not a guess. It is deduction from
the one thing we can see.

> **Archaeology, honestly (R-19).** We recover the architecture, never the
> assets. Every monster, map, and line of prose in this chapter is ours,
> invented to exercise a design we inferred from the historical record — one
> cartridge, many adventures — and from the hard limits Part III taught us about
> what a 9918A can draw. Where we name the original's specifics we hedge them as
> reported; where we build, we build clean.

Contrast this with the sibling capstones and the lesson sharpens. METEOR BELT
and GRIDRUNNER 99 are *programs that are games*. DUNGEONS OF FATE is a *program
that plays games*. The difference is one pointer wide, and it changes
everything about how the thing is built, shipped, extended — and tested.

## 41.2 Specification: A Cartridge That Plays Files

Our title is **DUNGEONS OF FATE**: an engine plus a quest-file format, plus at
least one quest to prove the format carries a real game. The design goal is the
historical one, stated as an engineering requirement: *a reader can ship a new
adventure without reassembling the engine.* Everything that could differ between
two adventures is therefore data.

The heart of the chapter is the format, so we design it first and with care. A
quest file is a self-describing blob: a fixed header of counts and **offsets**,
followed by five sub-tables. Offsets — distances from the file's own base —
rather than absolute addresses, so the engine can load the file *anywhere* (into
a disk buffer, into a bench's ROM) and the internal references still resolve.
The header:

| Off | Size | Field | Meaning |
|----:|:----:|:------|:--------|
| >00 | word | `QMAG`  | magic `>DF99` — "Dungeons Of Fate," the file's fingerprint |
| >02 | word | `QVERS` | format version (this engine reads `>0001`) |
| >04 | byte | `QWID`  | map width in tiles |
| >05 | byte | `QHGT`  | map height in tiles |
| >06 | byte | `QNMON` | monster-type count |
| >07 | byte | `QNITM` | item count |
| >08 | byte | `QNTXT` | text-string count |
| >09 | byte | `QPSX`  | party start column |
| >0A | byte | `QPSY`  | party start row |
| >0C | word | `QOMAP` | offset → RLE map stream |
| >0E | word | `QOMON` | offset → monster table |
| >10 | word | `QOITM` | offset → item table |
| >12 | word | `QOTXT` | offset → text index table |
| >14 | word | `QOTUN` | offset → tuning block |

Two disciplines make this a *format* and not merely a layout. First, the **magic
and version** at the very front: they are the file saying *I am what you think I
am, and I speak your dialect.* An engine that reads them can refuse a corrupt or
future file at the door — a courtesy the CQ-82 rubric (Ch. 36) calls *zero
crashes*, achieved here by *validation* rather than luck. Second, and this is
the rule the whole chapter turns on: the format is defined **once**, in an
assembler include the engine and the file-builder both read.

```asm
QMAGIC EQU  >DF99            "Dungeons Of Fate", format id (header word 0)
QVER1  EQU  >0001            format version this engine reads
QMAG   EQU  0                >0  word  magic = QMAGIC
QVERS  EQU  2                >2  word  format version
QWID   EQU  4                >4  byte  map width  (tiles)
QHGT   EQU  5                >5  byte  map height (tiles)
*  ... counts, start tile ...
QOMAP  EQU  12               >C  word  offset -> RLE map stream
QOMON  EQU  14               >E  word  offset -> monster table
```

That file is `quest.inc`, and it is the **single source of truth** (the Ch. 38
asset-pipeline rule). The engine `COPY`s it to know where the monster table
lives; the PC quest-builder of §41.7 `COPY`s — or mirrors — the same names to
know where to *put* it. One schema, two readers, and no possibility of drift,
because a change to the layout changes both sides from one edit. This is the
same move a modern team makes when a client and a server generate their wire
types from one shared schema; we are just doing it in `EQU`s.

Where does it all live? The engine is tiny — it is only a reader — so the memory
budget is dominated by *content*, and content is transient:

| Region | Holds | Lifetime |
|:-------|:------|:---------|
| Cartridge ROM `>6000` | the engine + (on the bench) an embedded quest | fixed |
| Scratchpad `>8300` | workspace, stack, resolved table pointers, combat scalars | per-session |
| Expansion RAM `>A000` | decoded map, the mutable party, combat scratch, text buffer | per-session |
| Disk buffer (hardware) | the quest as loaded by `filelib` | per-quest |

The [32K] in this chapter's banner is not for the engine; it is for the *quest*
the engine loads and the *save* it writes. The program that plays a database can
afford to be small precisely because it holds no database.

## 41.3 Mounting a World

To *mount* a quest is to point the engine at a base address and let it resolve
the file's internal offsets into live pointers — after checking that the file is
one it can read at all. Because the quest is CPU-addressable (ROM on the bench,
a RAM buffer on hardware), the header reads are plain `MOV`s, not VDP traffic.

```asm
MOUNT  MOV  R0,R1            R1 = candidate base
       MOV  @QMAG(R1),R2
       CI   R2,QMAGIC
       JNE  MNBAD            wrong fingerprint -> refuse it
       MOV  @QVERS(R1),R2
       CI   R2,QVER1
       JNE  MNBAD            a dialect we do not speak -> refuse it
       MOV  R1,@QADDR
       MOV  @QOMAP(R1),R0
       A    R1,R0            offset + base = live pointer
       MOV  R0,@PMAP
*  ... resolve PMON, PITM, PTXT, PTUN the same way ...
       CLR  R0              status = mounted
       RT
MNBAD  LI   R0,1            status = rejected
       RT
```

`MOUNT` returns a status, and the engine *believes* it: a rejected file never
gets played. That single `CI R2,QMAGIC / JNE MNBAD` is the difference between an
RPG that greets a scrambled disk with an error and one that follows a garbage
offset into the weeds and hangs. It costs four bytes. Spend them.

With the pointers resolved, the world can be drawn. The map arrives **compressed**
— a wall-bordered room is mostly identical tiles, exactly the redundancy Ch. 38's
run-length coder was built to eat. We decode it straight into a RAM buffer and
then paint it to the name table, and here is the payoff of the data-driven stance:
the render loop does not know the map's shape. It asks the *header*.

```asm
DRAWM  DECT R10
       MOV  R11,*R10
       MOV  @PMAP,R0        the RLE stream ...
       LI   R1,MBUF         ... decoded into a RAM buffer
       BL   @RLEDEC         (rlelib, Ch. 38 — 44 bytes -> 192 tiles)
       MOV  @QADDR,R3
       MOVB @QHGT(R3),R8    height, from the file
       SRL  R8,8
       MOVB @QWID(R3),R9    width,  from the file
       SRL  R9,8
*  ... paint R8 rows of R9 tiles from MBUF to the name table ...
```

Our proving map — a 16×12 room ringed in wall — packs to forty-four bytes of RLE
and expands to a hundred and ninety-two tiles: the asset pipeline's promise,
redeemed in miniature. The party marker is stamped last, at the tile the header
names in `QPSX`/`QPSY`, so even *where the heroes begin* is content, not code.

> **Field note.** We render Graphics I tiles, not sprites, for the world — a
> dungeon is a grid, and a grid is what the name table *is*. Sprites are perfect
> garnish (a torch flicker, the party's own glyph lifted off the floor), and the
> outline reserves a pseudo-3D corridor view as a stretch build; both are left
> as exercises so the core stays a clean demonstration of *map-as-data*.

## 41.4 The Combat Resolver

An RPG is arithmetic with a story draped over it, and the arithmetic is where
"data-driven" earns its rent. Our party is not written into the engine — it is
read from the quest's **tuning block**, so the heroes a quest ships with are the
quest author's decision:

```asm
PINIT  MOV  @PTUN,R2        tuning base
       LI   R4,PARTY
       MOVB @TH0HP(R2),R1
       MOVB R1,@HHP(R4)     hero 0's HP is DATA, not a constant
       MOVB @TH0AT(R2),R1
       MOVB R1,@HATK(R4)
*  ... DEF, SPD, the alive flag, then hero 1 ...
```

Combat then runs in rounds. Every living hero swings; a **to-hit roll** off the
LFSR (Ch. 8's pseudo-random generator, the same tap the shooter used) gates the
blow against a threshold *from the tuning block*; a hit does `ATK − DEF` damage,
floored at one; and after each hit the monster checks its nerve. When its HP
falls to its **morale** line — a per-monster byte — it breaks and flees rather
than dying. Then the monster answers, striking the first hero still standing.
The resolver reads nothing but the quest and the dice:

```asm
FGHERO MOVB @HALV(R4),R6    hero alive?
       JEQ  FGHNXT
*  ... step the LFSR, roll = SEED & 7 ...
       C    R7,R9           roll vs the quest's to-hit threshold
       JHE  FGHNXT          roll >= threshold -> a miss
       MOVB @HATK(R4),R6    damage = heroATK - monDEF, floored at 1
       SRL  R6,8
       MOVB @MDEF(R2),R7
       SRL  R7,8
       S    R7,R6
*  ... apply to the monster; if HP <= morale, it flees ...
```

Because every number — hero stats, monster stats, the to-hit threshold, the
morale line — lives in the file, **re-balancing the game is a data edit**, not a
recompile. Make the goblins hit harder by changing one byte in the quest; the
engine is untouched. That is the promise of the format made concrete: the design
lives where the designer can reach it.

Now the tester's move. A resolver full of dice is exactly the kind of code that
*looks* right and *is* wrong, so we pin it down. Seed the LFSR to a fixed value,
choose stats simple enough to simulate on paper, and the fight becomes
**deterministic** — a known input with a known answer. The self-test runs two
such fights against two monsters the quest defines for the purpose:

| Monster | HP | ATK | DEF | Morale | The scripted outcome |
|:--------|---:|----:|----:|-------:|:---------------------|
| Cave rat | 8 | 3 | 2 | 0 | never flees → **killed** (HP 0); hero 0 ends at HP 8 |
| Goblin   | 8 | 3 | 2 | 5 | flees when HP ≤ 5 → **fled**, alive at HP 4 |

Both outcomes are worked out by hand in the margin of the source and then
demanded of the machine: the kill fight must end with `VICTOR` = party and the
monster at zero; the flee fight must end with `OUTCOM` = fled and the monster
alive at four. When the border comes up green, those two little dramas played
out exactly as written — which means the turn order, the damage floor, the
death test, and the morale break are all wired correctly. The story is invented;
the *proof* is real.

## 41.5 Words, Five Bits at a Time

Prose is the bulkiest thing an adventure carries and the most wasteful in ASCII,
where every letter spends eight bits to say one of about thirty things. Ch. 38's
answer was **five-bit packing**: a thirty-two-symbol alphabet — space, the
twenty-six letters, four marks, and an end sentinel — costs five bits a symbol
instead of eight, a flat 37.5% saving, and the quest stores its dialogue that
way. The engine's job is to walk the bitstream and hand back ASCII.

| Symbol | 0 | 1 | 2–27 | 28 | 29 | 30 | 31 |
|:-------|:-:|:-:|:----:|:--:|:--:|:--:|:--:|
| Means  | end | space | A–Z | `.` | `,` | `!` | `?` |

The unpacker pulls five bits at a time, most significant first, building each
symbol and looking it up in a thirty-two-byte table. It reads until the end
sentinel. Trace the word **FATE** to see the machinery turn: its symbols are
7, 2, 21, 6, then 0; laid end to end five bits each and padded to bytes, that is
`>38 >AA >60 >00` — four bytes for a four-letter word plus its terminator, and
back out the far end come `>46 >41 >54 >45` and a zero, *F A T E*.

And here the chapter earns a scar worth keeping. The obvious way to shift a bit
out of the stream and into the symbol is two shifts in a row:

```asm
       SLA  R2,1          next stream bit -> carry
       SLA  R4,1          make room in the symbol, then add the bit ...
       JNC  ...           WRONG: this tests R4's carry, not R2's
```

The first `SLA` sets the carry from the stream. The second `SLA` — the one that
makes room in the symbol — **overwrites the carry with its own shifted-out bit**
before the `JNC` ever looks at it. The unpacker read the wrong bit every time,
and the self-test caught it instantly: FAILID 6, a red border, and a `TBUF` full
of nonsense where FATE should have been. The carry flag is not a register you
own for a while; it is a single wire that the *very next* arithmetic instruction
will drive. The fix is to consult it before you clobber it:

```asm
TXHAVE SLA  R2,1          next stream bit -> carry (read it NOW)
       JOC  TXONE         a 1?  branch before anything touches carry
       SLA  R4,1          a 0: shift a zero in
       JMP  TXNOC
TXONE  SLA  R4,1          a 1: shift and set
       INC  R4
```

> **Pitfall — the carry has no memory.** Register discipline (R-16) is usually
> about R11 and the workspace. The status bits are the same lesson one level
> down: `C`, `OV`, and the rest live exactly until the next instruction that
> writes them. If you need a flag, branch on it or capture it *immediately*.
> More self-test time in this book has gone to clobbered *flags* than clobbered
> *registers*.

The unpacked line then goes to a windowed dialogue box on screen — a rectangle
of the name table the engine owns and the prose fills — but the provable part is
the bytes: unpack text[0], and the buffer must read *FATE*. It does.

## 41.6 Persistence: The Disk and the Save

On real hardware the quest is a file, and a file is read through the ceremony
Ch. 31 taught: a Peripheral Access Block in VRAM, a name, an opcode, a link to
the device. Because a quest is a single binary blob, it loads in **one**
operation — the whole-file program-image `LOAD` — rather than a record-by-record
loop:

```asm
       LI   R0,PABADR
       LI   R1,OPLOAD
       BL   @FDOP          LOAD "DSK1.QUEST" -> VRAM, one operation
*  ... then copy the blob down to CPU RAM, and:  LI R0,QBUF / BL @MOUNT
```

That is the *other half of the test seam*. The engine mounts through a pointer;
on the bench that pointer aims at an embedded blob, and here it aims at bytes
that came off a disk. Same `MOUNT`, same contract, two sources of bytes — and
the only reason the bench can prove the engine at all is that the engine never
cared where the bytes came from.

**Saving** reuses the format's own best idea. A save file is a small record with
its *own* magic and version, the party's live stats and map position, and a
checksum; it is written with `filelib`'s `WRITE` opcode and read back with the
same validation `MOUNT` uses, so a save from an old version — or a corrupt one —
is refused, not trusted. Multiple slots are just multiple filenames.
`DSK1.SAVE1`, `DSK1.SAVE2`: the disk is the slot rack.

Now the honest part, because this book does not assert what the project cannot
run (R-12). **BENCH99 has no card at `>4000`** — there is no disk DSR on the
bench, so `loadq.a99` *assembles* there but cannot *execute*. Its behavior is
proven at a different tier, the one Part VII established: the Rust test harness
`device_io.rs` drives a real PAB through a quest `LOAD` and passes green at HEAD,
and real `.dsk` images decode against the probe-pinned DSR facts. So the disk
path is machine-verified — through the harness, not the bench. And the gap
itself is a signpost: a **BENCH99 disk command**, and general **`.dsk` mounting**
in the emulator, would let a future edition drive the full load-play-save loop
under the lab bench. That is the book steering the emulator, exactly as the
outline predicts — a roadmap item, noted here for the project.

## 41.7 The Quest-Builder

The last promise is the boldest: *author a new adventure and run it, no
assembler required.* That is a tool, and it lives off the target — a small PC
program that reads a human-friendly description of an adventure (its rooms, its
bestiary, its script) and emits the binary blob the engine mounts. Its entire
correctness condition is that it writes the bytes `quest.inc` describes:

| The author writes… | The builder emits… |
|:-------------------|:-------------------|
| a map as rows of characters | the RLE stream at `QOMAP` |
| a table of monsters with stats | fixed `MSIZE` records at `QOMON` |
| items, with types and powers | `ISIZE` records at `QOITM` |
| lines of dialogue | five-bit-packed strings + the index at `QOTXT` |
| party stats and difficulty knobs | the tuning block at `QOTUN` |
| a magic number and version | the header, so the engine will mount it |

Because the builder and the engine both obey `quest.inc`, a change to the format
propagates to both, and neither can silently disagree with the other about where
the goblins are.

Where does the builder *run*? Not on this workstation. The PC toolbox for this
project is `sh` + `cargo` — no Python (R-12), the same wall Ch. 6 and Ch. 38's
image tooling met — so the quest-builder is a Python program authored and run
**Mac-side**, and what ships in the repository is its **output**: the quest blob,
committed as data the engine can mount. On the PC we reach the identical result
by the identical schema through the toolchain we *do* have: author the quest as
an assembly source — the `QUEST1` data block, laid out by hand against
`quest.inc` — and let `libre99asm` emit the bytes. The builder is a friendlier
skin over that exact process, and the format is the contract that makes the two
paths interchangeable. Specified here; run where Python lives.

## Postmortem: What the Separation Bought

The engine assembles to a single 8 K bank with room to spare — a fraction of the
[32K] the banner reserves — because it holds no content. That is the first
dividend of the line down the middle: an engine that plays a database is small,
because the database is elsewhere. The second dividend is the one this whole
book has been building toward. The seam we cut for *content* — engine here, data
there, a pointer between — is the seam we injected *tests* through. We could not
drive a disk on the bench, so we handed the engine a world through the same door
a disk would use, and asked it for answers we already knew. Data-driven design
did not just make DUNGEONS OF FATE extensible. It made it **provable**.

The scars were instructive and worth naming, because they generalize:

- **Validate at the door.** A versioned magic number turns "corrupt file" from a
  crash into a polite refusal. Four bytes buys you CQ-82's *zero crashes* against
  data you did not write.
- **The carry has no memory.** The sharpest bug here was not a clobbered
  register but a clobbered *flag* — the second `SLA` eating the first's carry.
  R-16's discipline extends to the status bits: read a flag or lose it.
- **State the gap; don't paper it.** The disk path is real and verified — at the
  harness tier, not the bench — and saying so precisely is worth more than a
  demo that pretends BENCH99 has a drive it does not.

> **Ruling R-20 — the data-driven artifact is the test seam.** When a Part IX
> capstone separates an engine from its content across a versioned binary format
> with a single-source `EQU` schema, that same seam is its verification harness:
> the deterministic self-test feeds the engine an *embedded* artifact and asserts
> known outputs, while the production path (disk, network, cartridge) supplies
> the *same* artifact through the *same* pointer contract and is verified at
> whatever tier that path allows (here: the Rust harness for `filelib`). Build
> the seam once; use it for extension and for proof.

## Lab 41 — Ship a Second Adventure

The engine is done and proven; now *be the content author*, and feel the
separation from the other side. Working only in `code/ch41/`:

1. **Reskin the world.** In `dungeons.a99`'s `QUEST1` block, change the two
   monster records — new HP, ATK, DEF, and *morale* — and rebuild. Predict, on
   paper, how the two self-test fights now resolve, then update the assertions in
   `START` to match your numbers. Green means your arithmetic and the engine's
   agree. You changed the *game* without touching a line of the *engine*.
2. **Add a line of prose.** Bump `QNTXT` to 2, add a second offset to the text
   index, and hand-pack a short word into five-bit symbols (the alphabet is in
   §41.5; check your work against the FATE trace). Call `SAY` with index 1 and
   assert the buffer. You have just used the packer's format by hand — which is
   exactly what the §41.7 tool automates.
3. **Break the file on purpose.** Point `MOUNT` at a blob with the wrong version
   number and confirm it is *refused*. Watch validation do its job.

The measure of success is not a screenshot; it is the green border reporting
that a world you authored played exactly as you predicted.

## Exercises

**✦ Warm-ups**

1. The header stores *offsets*, not absolute addresses. In one sentence, say what
   would break if it stored addresses instead, and why `MOUNT`'s `A R1,R0` is the
   line that makes offsets work.
2. `MOUNT` checks magic *and* version. Give a concrete failure each check catches
   that the other would let through.
3. The map packs 44 RLE bytes into 192 tiles. Compute the compression ratio, and
   name one map shape that would make RLE *lose* (expand rather than shrink).

**✦✦ Building**

4. Add an **item-equip** step: before the kill fight, apply the quest's first
   item (a `+2` weapon) to hero 0's ATK, and assert the monster now dies in one
   fewer swing. Items become live data, not catalog decoration.
5. Give combat **initiative**: sort the turn order by the `SPD` byte each
   combatant already carries, so a fast monster can strike first. Add a
   deterministic self-test fight that proves the order changed.
6. Implement **save and reload** against `filelib`'s contract (it need only
   assemble, per R-12): write a save record with its own magic/version/checksum,
   and a `MOUNT`-style validator that refuses a bad one. Note where in the load-
   play-save loop a real bench command would let you *run* it.
7. Replace the fixed to-hit threshold with a **DEF-relative** one (harder targets
   are harder to hit) drawn from tuning, and show the self-test still resolves
   deterministically under a fixed seed.

**✦✦✦ Reach**

8. Design a **v2** of the quest format that adds a per-tile *event* table (a tile
   can trigger prose or a fight). Keep v1 files mountable by bumping the version
   and branching in `MOUNT`. Write the format change as a `quest.inc` diff.
9. Sketch the **quest-builder** (§41.7) as a `cargo`-buildable Rust tool instead
   of Python, so it runs in this project's PC toolbox. What does it read, which
   `quest.inc` names does it share with the engine, and how would you test that
   its output round-trips through `MOUNT`?

## Further Reading

- **Chapter 38**, *Data, Compression, and the Asset Pipeline* — the RLE decoder
  and the five-bit text packing this engine consumes; the two-machine (author on
  the host, consume on the target) workflow the quest-builder extends.
- **Chapter 31**, *File I/O: PABs and the Filesystem Contract* — the `filelib`
  ceremony behind `loadq.a99`, and the Part VII verification tiers.
- **Chapter 36**, *Program Architecture in 16–48K* — SKELETON99, the chassis all
  five capstones instantiate, and the CQ-82 rubric this one meets by validation.
- **Chapter 8**, *Arithmetic, Logic, and Bits* — the LFSR the combat resolver
  rolls its dice with.
- For the modern lineage of engine/content separation, any account of *DOOM*'s
  `WAD` format repays the read: the same line down the middle, a decade later and
  a thousandfold larger.

## Summary

DUNGEONS OF FATE is a program that plays games rather than a program that is one,
and the difference is a single pointer. We built the line down the middle —
engine on one side, a versioned quest file on the other — and made the format,
not the code, the place where a monster, a map, or a mood lives. The engine
`MOUNT`s a quest by validating its magic and version and resolving its offsets;
it renders a world from an RLE-compressed map decoded by dimensions the file
supplies; it resolves combat through a turn engine whose stats, to-hit, and
morale are all quest data, so balancing is an edit and never a rebuild; and it
turns five-bit-packed bytes back into prose, a routine that taught us the carry
flag has no memory. On disk the quest arrives through `filelib` and saves return
in versioned slots — a path we verify at the harness tier and honestly cannot
run on the bench, which marks a `.dsk`-mounting roadmap item for the emulator.

The schema is defined once, in an include the engine and the PC quest-builder
both obey, so content and code can never silently disagree. And the deepest
lesson is the one the border light reports: the seam we cut so a *player* could
ship a new adventure is the seam we cut so *we* could prove the engine correct.
Hand a data-driven engine a world built to test it, ask for answers you already
know, and a green screen is not a demo — it is a proof. That is ruling R-20, and
it is why the data-driven artifact is the most testable thing in this book.

*Machine-verified against toolchain commit e5c4697: `dungeons.a99` assembles to
an 8,192-byte single bank (entry `>60BE`); the six-part deterministic self-test
paints VR7 = `>02` (GREEN) with FAILID 0, and `TBUF` round-trips* FATE *as
`46 41 54 45 00`. `loadq.a99` assembles; its disk behavior is verified in
`device_io.rs`, not BENCH99 (R-12).*
