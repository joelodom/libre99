# Chapter 5 — The Console Memory Map: Geography Is Destiny

*In which the 64 K address space turns out to contain exactly two fast neighborhoods, and we
take a stopwatch to all the others.*

<!-- Part II — The TMS9900 and Assembly Fundamentals · target ≈20 pp -->
<!-- STATUS: DRAFTED (session 4, 2026-07-05) — pending review passes. All §5.1–5.7 + Lab measurements machine-verified on BENCH99 at commit 2da67ae; code in code/ch05/. Open item surfaced to the project: libre99-core MOV/MOVB omit the destination pre-read (all-slow MOV reads 26 vs hardware 30) — a cpu.rs fix, not a book change. -->

## The Meeting That Left No Minutes

Somewhere in Lubbock, Texas — sometime in 1978 — a group of Texas Instruments engineers and
product managers sat down in a conference room and decided how fast your programs would run.

We have to reconstruct the scene, and we should say so plainly, because no transcript of it
survives — no minutes, no memo, no whiteboard photograph; if such a record exists, it has never
surfaced. What we do know is the setting and the pressure. TI's consumer products operation ran
out of Lubbock, a plains city better known for cotton and Buddy Holly than for computers, and by
1978 it was being asked to do something nobody had quite done: build a real computer that could
sit on a department-store shelf. The Speak & Spell was in stores that year, proof that TI could
put startling silicon in a toy. The home computer was supposed to be the next act, and it had a
bill of materials that somebody had to beat into shape.

One fact towered over that bill of materials. The machine would carry the TMS9900 — a genuine
16-bit processor, born in 1976 as the heart of TI's 990 minicomputers, at a moment when the rest
of the industry's home machines were making do with 8-bit parts. Why a minicomputer chip ended up
in a kitchen-table appliance is still argued about; the story most often told — Chapter 1 told it
— involves the TMS9985, an 8-bit-bus sibling that died in development, leaving the big processor
to take a consumer job it was never costed for. However it happened, the 9900 was there, sixteen
data lines wide, and every one of those lines wanted to be connected to something.

Connected to what? Sixteen-bit-wide memory fast enough to keep up meant static RAM, and static
RAM was the expensive way to buy bits in 1978. Dynamic RAM was cheap and getting cheaper, but it
was needy — it forgets unless refresh circuitry sweeps it constantly, and that circuitry is its
own line item. The designers held one unusual card: TI's own TMS9918 video chip was already a
dynamic-RAM controller by trade, managing a private frame store as part of its day job. So the
machine's big RAM — sixteen kilobytes of it — went behind the video chip, on the video chip's
private bus, where the CPU cannot fetch a single instruction from it. And the CPU itself? It got
one narrow, byte-wide path to almost everything, with a multiplexer to fold its proud 16-bit
accesses into pairs of 8-bit ones — and, as consolation, a tiny patch of true 16-bit static RAM.
Two hundred fifty-six bytes of it.

Whoever sat in that room made those calls one line item at a time, the way such decisions are
always made: not *let's build a machine with a strange memory system*, but *this chip we already
make*, *that saves a dollar*, *this can ship in time*. Then they went to lunch, and the meeting
dissolved into the past, unrecorded.

Except it wasn't unrecorded. Every ruling from that room was written down at a resolution no memo
could match — written into sixty-five thousand five hundred thirty-six addresses, each one filed
under *fast* or *slow*, RAM or ROM or live machinery. The address map of the TI-99/4A is the
minutes of that meeting. Where the money went, the map is wide and quick. Where the money was
saved, the map narrows to a funnel. You can read the whole negotiation off it four decades later
with nothing but a stopwatch — which is convenient, because a stopwatch is exactly what Chapter 4
gave you.

This chapter reads the minutes. We will walk the map end to end, region by region; find the two
territories where the machine still runs at full width; put a cycle counter on all the others;
and turn what we measure into the first planning document of every program you will write in this
book. Geography, on this machine, is destiny.

---

## What You Will Learn

- Redraw the complete 64 K map from memory: every region's start address, size, contents, and bus
  width — and which two regions form the 16-bit, zero-wait fast island.
- Predict an instruction's cost from its placement with T = C + 4 × (accesses in the 8-bit
  domain), and verify the prediction on BENCH99.
- Build and run a timing rig that measures the real per-region access toll — and read its output
  critically, including the one case where the instrument itself is under suspicion.
- Explain partial address decoding: why mirrors exist, what unowned reads return, and why neither
  reads nor writes to "empty" space are guaranteed harmless.
- Name every memory-mapped port on the machine — sound, VDP, speech, GROM — and state which of
  them have side effects on a mere read.
- Give the scratchpad's first-order tenant map: what the console reserves, what a program may
  claim, and when.
- Choose placements — code, workspace, data — for a new program and defend each choice in
  measured cycles.
- Fill in the memory-budget worksheet this book uses at the start of every project from here to
  Ch. 43.

## The Bridge: The Last Honest Addresses

Modern memory is polite fiction stacked on polite fiction. The address you print in a debugger is
virtual — a per-process alias invented by an MMU and resolved through page tables you never see.
Beneath that, layers of cache decide, statistically, whether your load costs a nanosecond or a
hundred. You already believe placement matters — you have split hot data from cold, chased cache
lines, maybe fought a NUMA node — but you have probably never *seen* placement. You have only
seen its shadows in a profiler.

The 99/4A removes every veil. There is no MMU: `>8300` is not a name some kernel chose for a
page, it is a physical place — a specific patch of static RAM you could find with a continuity
probe. There is no cache: nothing remembers your last access or speculates about your next. There
are exactly two speeds, and the address alone decides between them, deterministically, every
time. Chapter 4 gave you the machine's clock arithmetic; this chapter gives you the terrain it
runs on. Together they make performance something almost unheard of in your world: computable in
advance.

One more mapping before we start walking. On modern hardware, memory-mapped I/O is a specialist's
business — device registers hidden behind kernel mappings, touched mostly by driver writers. Here
it is the front door: the sound chip, the video processor, the speech synthesizer, and the GROM
library all answer at fixed addresses inside this same 64 K map, and a plain MOVB is how you talk
to every one of them (§5.6). If you have written firmware for a microcontroller, you will feel at
home. If you have not, this machine is about to be the best first microcontroller you never had.

## 5.1 One Map, Sixty-Four Kilobytes

Unfold the whole territory at once. The 9900 addresses 64 K of bytes — sixty-four kilobytes, no
more, ever — and on this machine every one of them was assigned a job before you arrived. We walk
it from the bottom.

**`>0000`–`>1FFF` — console ROM, 8 K, sixteen bits wide.** The machine's permanent software: the
interrupt and XOP vectors at the very bottom (`>0000`–`>003F` and `>0040`–`>007F` — Ch. 4 taught
you to read them), then the console's resident operating machinery — the GPL interpreter that
runs Floor 2 of the tower (Ch. 2), the console interrupt service routine, and the floating-point
package TI BASIC leans on. All of it sits on the CPU's own 16-bit bus: full width, zero wait.
Note the irony early and keep it: the fastest memory in the machine belongs to the interpreter.

**`>2000`–`>3FFF` — low memory expansion, 8 K.** RAM if you own the 32 K expansion; nothing if
you don't. Why 32 K arrives split into an 8 K piece here and a 24 K piece at `>A000` is a
question the map itself will answer in §5.5.

**`>4000`–`>5FFF` — the peripheral window.** Device Service Routines — the driver ROMs that live
on expansion cards — take turns answering this window; which card's ROM you see depends on which
card has been switched in, a story that belongs to the CRU (Ch. 10) and to Part VII (Ch. 30).
With no card selected, nobody answers here at all — which is more interesting than it sounds
(§5.4).

**`>6000`–`>7FFF` — cartridge ROM, 8 K.** The slot on the console front lands here: whatever ROM
a cartridge carries answers in this window. (Cartridges can also carry GROM — a different beast
at a different address, Ch. 25.) Eight kilobytes is not much; Ch. 35 shows how commercial carts
banked their way past it.

**`>8000`–`>9FFF` — the control block.** The strangest 8 K on the map and the machine's real
control panel. In it: the 256-byte scratchpad RAM at `>8300`–`>83FF` (§5.2) — plus, we will
discover, its mirrors (§5.4) — and then the memory-mapped ports, marching up the block in `>0400`
strides: the sound chip's write port at `>8400`; the VDP's read pair at `>8800` (data) and
`>8802` (status) and its write pair at `>8C00` (data) and `>8C02` (address); speech at `>9000`
(read) and `>9400` (write); and the GROM ports at `>9800`/`>9802` (read data/address) and
`>9C00`/`>9C02` (write data/address). Every one of these addresses is live machinery, and several
of them react to being merely *read* (§5.6).

**`>A000`–`>FFFF` — high memory expansion, 24 K.** The larger piece of the 32 K, and the land
where most of the programs in this book will live.

Now redraw the same map along the only axis a stopwatch cares about. Two regions — console ROM
and the scratchpad — sit on the CPU's own 16-bit bus: the fast island of Chapter 2, full width,
zero wait. Everything else — both expansion RAMs, the cartridge, the peripheral window, every
port — lies on the far side of the 8-bit multiplexer, the funnel, where each 16-bit access is
folded into two 8-bit trips and taxed for the privilege. One map, two domains. The rest of this
chapter is about living with that.

Here is the whole territory in one column. The `==` edges mark the fast island — the only 16-bit,
zero-wait memory on the machine — and everything between the plain `--` rules lies across the
funnel.

```text
    >0000 +========================+   the fast island:
          |  Console ROM     8 K   |    16-bit bus, 0 wait
          |  vectors, GPL, ISR, FP |
    >1FFF +========================+
    >2000 +------------------------+
          |  Low expansion   8 K   |    RAM with 32 K  --  funnel, +4
    >3FFF +------------------------+
    >4000 +------------------------+
          |  DSR window      8 K   |    card ROMs, paged (Ch. 30)
    >5FFF +------------------------+
    >6000 +------------------------+
          |  Cartridge ROM   8 K   |    the front slot (Ch. 35)
    >7FFF +------------------------+
    >8000 +------------------------+
          |  Control block   8 K   |    scratchpad + ports (detail below)
    >9FFF +------------------------+
    >A000 +------------------------+
          |                        |
          |  High expansion 24 K   |    RAM with 32 K  --  funnel, +4
          |  (programs live here)  |
    >FFFF +------------------------+

  Inside the control block, >8000–>9FFF:
    >8300  Scratchpad RAM  256 B  ==  the island's other half: 16-bit, 0 wait
    >8400  Sound    write             \
    >8800  VDP      read data/status   |  memory-mapped ports: live machinery,
    >8C00  VDP      write data/addr    |  reached by plain MOVB, several with
    >9000  Speech   read               |  side effects on a mere read (§5.6);
    >9400  Speech   write              |  each marches up the block in >0400
    >9800  GROM     read data/addr     |  strides
    >9C00  GROM     write data/addr   /
```

And the same territory as a reference table — the column order App. C reuses unchanged:

| Region | Range | Size | Bus | Wait | Contents | First visited |
|---|---|---|---|---|---|---|
| Console ROM | `>0000`–`>1FFF` | 8 K | 16-bit | 0 | vectors, GPL interpreter, ISR, FP package | here (§5.1) |
| Low expansion | `>2000`–`>3FFF` | 8 K | 8-bit | +4 | RAM (with 32 K) | here (Lab 5) |
| DSR window | `>4000`–`>5FFF` | 8 K | 8-bit | +4 | peripheral-card ROMs, paged | Ch. 30 |
| Cartridge ROM | `>6000`–`>7FFF` | 8 K | 8-bit | +4 | the front-slot ROM | Ch. 35 |
| Scratchpad | `>8300`–`>83FF` | 256 B | 16-bit | 0 | CPU RAM; console + GPL tenants | here (§5.2) |
| Sound port | `>8400` | — | 8-bit | +4 | SN76489 write port | Ch. 19 |
| VDP ports | `>8800`/`>8802`, `>8C00`/`>8C02` | — | 8-bit | +4 | read data/status; write data/addr | Ch. 12 |
| Speech ports | `>9000`, `>9400` | — | 8-bit | +4 | read; write | Ch. 20 |
| GROM ports | `>9800`/`>9802`, `>9C00`/`>9C02` | — | 8-bit | +4 † | read data/addr; write data/addr | Ch. 25 |
| High expansion | `>A000`–`>FFFF` | 24 K | 8-bit | +4 | RAM (with 32 K) | here (Lab 5) |

† The GROM ports levy a much larger toll than the flat +4 — the chip is a slow serial device the
CPU waits on — which is why the whole GPL operating system runs so much slower than CPU-RAM code
(measured in §5.6, dissected in Ch. 25). Every boundary and width above was cross-checked against
the project's bus source (`libre99-core`, `machine.rs`) and spot-probed on the bench: a `pw` to
`>0000` is refused (the reset vector `>83E0`/`>0024` reads back unchanged — ROM), while a `pw` to
`>2000` and `>A000` sticks (RAM), and the empty cartridge window reads back `>00`.

## 5.2 The Scratchpad: 256 Bytes of Sixteen-Bit Territory

Two hundred fifty-six bytes, `>8300` through `>83FF`. This is the only RAM in the machine the CPU
reaches at full width and zero wait — and say the stronger version out loud, because it sets the
whole machine's character: on an unexpanded console, this is the only CPU-addressable RAM at all.
The 16 K of video RAM belongs to the VDP, on the VDP's private bus; the CPU begs bytes from it
through the `>8800`/`>8C00` ports, one at a time (Ch. 12). A stock 99/4A is a computer with 256
bytes of memory and a very good excuse.

Chapter 4 explained why this patch, of all places, had to be fast: the 9900 keeps its registers
in memory. A workspace is 32 bytes, and wherever WP points, that memory *is* R0 through R15. Park
a workspace in the scratchpad and registers cost what registers should; park it beyond the funnel
and every register touch pays the toll — a tax not on some accesses but on nearly all of them,
because almost every instruction touches a register. The pad is where workspaces go to be worthy
of the name. Eight of them would fit exactly, and that tidy arithmetic is the seed of a real
multitasking scheme in Ch. 9.

But you are not the pad's first tenant. The console got there first. The GPL interpreter keeps
its workspace at `>83E0`–`>83FF` and its status byte at `>837C`; the console's 60 Hz interrupt
machinery keeps its working state here too — it must, since on an unexpanded console there is
nowhere else. The scratchpad is less a vacant lot than the console's own kitchen: you may cook in
it, but the staff is in and out constantly, and some shelves are simply theirs. The table below
is your first, deliberately coarse tour of who holds what — and of what a well-mannered program
may claim, and *when*. The byte-by-byte atlas waits for Ch. 24 and App. C; today you only need to
know where not to sit.

| Range | Tenant | Claimable by user code? |
|---|---|---|
| `>8300`–`>831F` | console low scratch (pointers, work bytes) | not while the console/GPL runs; free bare-metal |
| `>8320`–`>836F` | idle at rest | **yes** — the usual corner for a user workspace |
| `>8370`–`>837B` | console/GPL transient scratch | risky with interrupts on; free bare-metal |
| `>837C` | GPL status byte | **no** — the GPL interpreter reads it |
| `>837D`–`>83BF` | console/GPL transient scratch | mostly free bare-metal; guard with interrupts on |
| `>83C0`–`>83DF` | interrupt & GPL working area | **no** with interrupts enabled |
| `>83E0`–`>83FF` | GPL interpreter workspace (its R0–R15) | **no** while GPL or the console runs |

Read the room this way. On a booted console the interrupt service routine and the GPL interpreter
are in and out of the pad sixty times a second: watch it on the bench — boot the machine, dump
`>83C0`–`>83FF` twice a frame apart, and those bytes will have moved, because that is where the
staff works. The workspace pointer itself sits at `>83E0`, so `>83E0`–`>83FF` is nothing less than
the console's own register file — touch it with interrupts on and you are editing the interpreter's
registers mid-thought. The `>837C` status byte is small but load-bearing for the same reason. The
safe corner for your own workspaces, alongside the running console, is the quiet stretch around
`>8320`–`>836F` — where the labs in this book park theirs. Go bare-metal (a cartridge with
interrupts masked and no GPL underfoot) and the whole 256 bytes is yours, because you sent the
staff home. Either way, this table is deliberately coarse; the byte-by-byte atlas — every pointer,
every flag, named — is Chapter 24's work, and App. C's reference.

> **Sidebar — Why 256 Bytes? The Economics of the 1979 Bill of Materials.** Every kilobyte in a
> 1979 consumer product was a knife fight. Memory came in two kinds, and the difference ran the
> industry: *static* RAM was fast and simple to attach but expensive per bit; *dynamic* RAM was
> far denser and cheaper per bit, but slower, fussier, and needy — it forgets its contents unless
> refresh circuitry sweeps it constantly, and that circuitry is its own cost. The 99/4's
> designers held one unusual card: TI's own TMS9918 video processor was a dynamic-RAM controller
> by trade, refreshing a private 16 K frame store as part of its day job. So the machine's big
> RAM went there — behind the video chip, refreshed for free, invisible to the CPU's instruction
> fetcher (reaching it is Ch. 12's whole subject). What the CPU kept was the smallest viable
> patch of the expensive kind, wired the full 16 bits wide: 256 bytes. Why is that viable at all?
> Chapter 4 again: the 9900 keeps its registers in memory. A workspace is 32 bytes, so 256 bytes
> is eight workspaces' worth of the hottest state a 9900 system has. Wire exactly that much at
> full speed and the register file runs like a register file; skimp on it and every instruction
> in the machine slows down. The scratchpad was not a luxury granted to programmers — it was the
> minimum down payment the 9900's own architecture demanded.

## 5.3 The Funnel, Measured

Chapter 4 left you holding a two-term model: **T = C + 4 × (accesses in the 8-bit domain)** — a
base cost fixed by the instruction, plus four cycles of toll for every memory access that has to
squeeze through the funnel. A model is a promissory note. This section is where we cash it.

The race is almost embarrassingly easy to set up, which is the point: on this machine you do not
*theorize* about memory speed, you clock it. We plant the same two-instruction loop at `>8300`
and at `>A000`, aim the bench at each, and let the cycle counter arbitrate. The loop does no work
at all — it just runs — so every cycle it costs is pure geography. Then we widen the race to four
lanes: the scratchpad, the low expansion at `>2000`, the cartridge slot at `>6000`, and the high
expansion at `>A000` — the four places code can plausibly live.

You have already met the appetizer numbers. `JMP $` — the tightest loop this machine can express
— costs 10 cycles per lap in the scratchpad and 14 in expansion RAM: one instruction word fetched
per lap, one toll of four. `MOV R0,R1` costs 14 cycles with everything in the pad; 18 when the
code lives in slow memory but the workspace stays in the pad; 30 when code and workspace are both
beyond the funnel. Read that progression closely, because it is the fetch-versus-operand split:
the fetch toll follows the *code's* address, the operand tolls follow the *workspace's* address,
and every access is taxed separately, at its own address, with no bulk discount. The table this
section builds completes the grid region by region — and answers a question the model is silent
on: is the toll really the same +4 in every slow neighborhood, or does some territory drive a
harder bargain?

One more thing before the table, and it is a matter of instrument honesty. Our stopwatch is
software. The emulator counts the cycles it *models*, and this book's standing rule is that body
text asserts what the *hardware* does — where emulator and hardware disagree, we say so in the
open and track the disagreement until it is fixed. One such disagreement is live in exactly this
chapter's territory: a real 9900 executing MOV reads its destination before overwriting it — a
genuine quirk of the silicon — so a MOV whose destination lies beyond the funnel pays one more
toll than a naive count suggests. All-slow `MOV R0,R1` on real iron: 30 cycles. Our core, at the
commit these measurements were taken against, skips that destination pre-read and reports **26** —
four cycles light, exactly one funnel toll. The addition `A R0,R1`, which re-reads its destination
for honest arithmetic reasons, models the pre-read and so measures the correct **30** all-slow on
the same core — so wherever the pre-read matters, our table leans on the probe the instrument gets
right, and says so in its notes. (The fix belongs in the emulator, not the book: it is logged for
the project as a `cpu.rs` correction, and until it lands, this is the one number in the chapter we
take from the datasheet rather than the stopwatch.)

Here is the grid, measured on BENCH99's bare bench — a CPU on the console bus with nothing else
running, so the program counter and workspace pointer are ours to place and every cycle counted is
pure geography. Each cell is the cost of one execution of the probe, read straight off the trace:

| Probe | island: code + WS in pad | code out, WS in pad | all out: code + WS slow |
|---|---|---|---|
| `JMP $` | 10 | 14 | 14 |
| `MOV R0,R1` | 14 | 18 | 26 * |
| `A R0,R1` | 14 | 18 | 30 |

*Probe note: `MOV R0,R1` all-out reads **26** on our core against the hardware's **30** — the
missing destination pre-read described above. `A R0,R1` re-reads its destination for legitimate
arithmetic reasons, models the pre-read, and measures the datasheet's **30**; it is the honest
instrument wherever that toll is in play. "Code out" was measured identically at `>2000`, `>6000`,
and `>A000` — the three collapse into one column because they bill the same.*

Read the table by columns first. The `JMP $` row is the purest signal on the machine: one word
fetched, nothing else, so its cost is nothing but the fetch toll. Ten cycles on the island,
fourteen everywhere else — a flat +4 — and it did not matter whether "everywhere else" was the low
expansion at `>2000`, the cartridge at `>6000`, or the high expansion at `>A000`. All three billed
the identical fourteen. That answers the question the section opened with: the funnel is one
tollbooth with one price; no slow neighborhood drives a harder bargain than another. Speed here is
binary — island or mainland — not a gradient.

Now read the `MOV` and `A` rows across, and watch the toll split into two independent line items.
All-pad, both cost the datasheet's 14: no funnel, no toll. Move only the *code* out to slow memory
and both rise to 18 — the fetch now crosses the funnel (+4), but the operands, still in the pad
workspace, ride free. Move the *workspace* out too and every access is tolled: `A` climbs to 30,
four separate crossings at four cycles each stacked onto the base 14. The fetch toll followed the
code's address; the operand tolls followed the workspace's address; and nothing was bundled — each
access paid at its own address, full price. This is the fetch-versus-operand split made numeric,
and it is the whole reason the chapters ahead obsess over where a workspace lives.

One law falls out of the grid, and the book will lean on it for four hundred pages. The same three
instructions swing from 10 to 14, from 14 to 30, and not one bit of the instruction changed —
only the addresses its bytes and operands sat at. Change nothing about the instruction; change
only where it lives, and its price changes. **Speed is a property of addresses, not instructions.**

## 5.4 Mirrors and Ghosts: Partial Address Decoding

Decoding an address costs hardware. To give every one of 64 K addresses a unique owner, the
machine would have to examine every address line at every chip select — and in a 1979 bill of
materials, each gate of that logic was a real fraction of a real dollar. So this machine, like
nearly all of its contemporaries, decodes *partially*: each region's select logic watches only as
many high address lines as it takes to claim its territory, and ignores the rest. Small chip, big
territory, money saved.

Two kinds of ghost follow from that thrift. The first is the **mirror**: if the scratchpad's
decoder ignores address lines that would distinguish `>8000` from `>8300`, then the same 256
bytes of RAM answer at more than one address, and the pad reappears — image after image — across
the block. Mirrors are not folklore to memorize; they are decode logic to *measure*, and the
experiment below settles exactly which address bits the pad's decoder examines on this machine.
The second ghost is the un-owned read: address space with no chip behind it does not *fault*.
There is no MMU, no bus error, no exception — a read of unclaimed space completes cheerfully and
returns whatever the bus happens to be holding; a write completes cheerfully and changes nothing.
The machine never slaps your hand. It just lets the wrong thing happen.

Which is why "empty" space deserves suspicion in both directions. A write to a mirage address may
land, through a mirror, on real and very live scratchpad — the console's kitchen, remember — and
a read around the `>8000` block is genuinely hazardous, because several addresses there are ports
with side effects on *read* (§5.6): a casual sweep through memory can advance the VDP's address
pointer or disturb a GROM's position mid-fetch. Old charts wrote *here be dragons* across
unexplored territory. On this map the dragons are real, and some of them wake when you look at
them.

The mirror hunt takes six bench commands. Plant a sentinel in the real scratchpad, then go looking
for it at the addresses just below:

```text
bench> pw 8300 BEEF        # plant a sentinel in the pad
bench> m 8300 2
>8300  BE EF
bench> m 8000 2
>8000  BE EF               # the same 256 bytes answer here ...
bench> m 8100 2
>8100  BE EF               # ... and here ...
bench> m 8200 2
>8200  BE EF               # ... and here
bench> pw 8000 1234        # now write through the >8000 image ...
bench> m 8300 2
>8300  12 34               # ... and the real pad at >8300 changed
```

The pad answers at `>8000`, `>8100`, `>8200`, and `>8300` — four images of the same 256 bytes,
tiling the kilobyte from `>8000` to `>83FF`. And the last two lines are the proof that these are
not four copies but one: a write aimed at the `>8000` image came back out of `>8300`. Same silicon,
four doors. What licenses the pattern is the cheap decoder: to place an access somewhere in the
256-byte pad, the logic need only examine the low byte of the address (which of 256 cells) plus
the high bits that select the `>8000`-block; the two bits that would tell `>8000` from `>8100` from
`>8200` from `>8300` — A8 and A9 in the CPU's numbering — it simply never looks at. Ignore two
address lines, save two lines' worth of gates, and the RAM reappears four times as a side effect.
That is the whole economics of §5.4, measured. It is also the hazard made concrete: a wild write
to a "mirage" address up near `>8000` does not land in the void — it lands, through a mirror, on
the console's live kitchen at `>8300`. And reads are worse than writes here, because a few
addresses in this same block answer reads with *side effects*, not data — which is the next
section's business, and the reason the proof that "reading is not looking" waits for §5.6's port
demo rather than a blind sweep of the live block.

## 5.5 Where Programs Live

So where should a program actually live? The map offers three candidacies, and the split
personality of the 32 K expansion is the place to start. Thirty-two kilobytes, but never
contiguous: 8 K at `>2000`, 24 K at `>A000`. The reason is pure geography. By the time the
expansion existed, the middle of the map was already spoken for — the peripheral window holds
`>4000`, the cartridge holds `>6000`, the control block holds `>8000` — so RAM took the only two
vacancies left. It is zoning, not whimsy, and it means any large program must either fit in 24 K
or learn to live gracefully in two houses; the program-shape conventions this book builds (Ch. 9,
Ch. 36) make the two-house life routine.

Cartridge space is the second candidacy: 8 K at `>6000`, and it is ROM. Code and constant tables
burn in beautifully; variables must live elsewhere — the pad, the expansion when present, or
across the funnel in the VDP's 16 K. That discipline of *code here, state there* is the
cartridge's whole culture, and Part VIII (Ch. 35, Ch. 36) turns it into a build style. For now it
is enough to see the constraint on the map.

The third candidacy is the sly one: run your code *in the scratchpad*. Not all of it — 256 shared
bytes will not hold a program — but the inner loop, the ten or twenty instructions where a
program spends most of its life. The oldest speed trick on this machine is exactly that: keep the
program's body in slow, roomy expansion RAM, and at startup copy the hot loop into a claimable
corner of the pad (the §5.2 table names the corners) and branch to it. Code is data; copying it
is a MOV loop like any other; and the payoff is the full funnel toll refunded on every fetch of
every lap, forever. The listing below does it end to end and hands you the measured
before-and-after.

The whole trick is `code/ch05/padrun.a99`, and its heart is these lines: a program resident in slow
cartridge ROM copies its inner loop into a claimable corner of the pad, then runs the loop a
thousand times where it sits and a thousand times from its new home.

```asm
WS     EQU  >8300            our workspace (fast pad)
HOTLOC EQU  >8360            staging area in the pad (a claimable corner, §5.2)

START  LWPI WS
       LI   R0,HOT           copy the hot loop into the pad ...
       LI   R1,HOTLOC
       LI   R2,HOTLEN
CP     MOVB *R0+,*R1+        ... one byte at a time
       DEC  R2
       JNE  CP
RUN1   LI   R5,>03E8         1000 laps, the loop where it sits (slow cartridge)
       BL   @HOT
RUN2   LI   R5,>03E8         1000 laps, the very same loop staged in the pad
       BL   @HOTLOC
STOP   JMP  STOP
HOT    DEC  R5               the hot loop: a self-relative jump and a register
       JNE  HOT              operand, so it is position-independent
       RT                    ; ... and means the same thing in either home
HOTLEN EQU  $-HOT
```

The bench brackets the two runs with breakpoints and reads the cycle counter across each. A
thousand laps from the cartridge: **28,062 cycles**. The identical thousand laps from the pad:
**20,058**. Staging the loop bought back **8,004 cycles** — almost exactly two funnel tolls per
lap (the `DEC` and the `JNE` are each fetched, each refunded its +4), plus the one-time toll on the
return fetch. That is better than a quarter of the runtime gone — a **1.40× speedup** — on a loop
we did not change by a single instruction. We only moved its bytes onto the island. This is the oldest speed trick on the machine
because it is the purest expression of the chapter's law, and Ch. 37 turns it into doctrine.

## 5.6 Peripherals Are Just Addresses

Strip the mystery from four words: *peripherals are just addresses*. On this machine the sound
generator, the video display processor, the speech synthesizer, and the GROM library do not live
in some separate I/O universe reached through special instructions. They answer at fixed byte
addresses inside the same 64 K map as your variables. (The 9900 does also have a genuinely
separate I/O system — the bit-serial CRU, which tends the keyboard, the cassette, and the card
selects; that is Ch. 10's subject. The designers put the *heavy* traffic on the memory bus.)

The consequence is a worldview, and it is worth installing consciously. A MOVB aimed at `>8400`
is not a store; it is a *command* — the byte is an order to the sound chip, and "storing" it
makes something happen in the room. A MOVB drawn from `>8802` is not a load; it is a *question*,
and asking it changes the answerer — reading the VDP's status port clears bits in it, a fact
Ch. 12 builds on. The address selects a listener; the data is a message; and the memory metaphor
you have trusted since your first pointer — reads are free, reads are repeatable, reads are
invisible — quietly stops being true on five stretches of this map. That is why §5.4 warned you
about casual memory sweeps, and why the bench grew device views (`vdp`, `screen`): so you can
inspect the machinery without addressing it.

If you have only met memory-mapped I/O as a paragraph in an operating-systems course, the
demonstration below is the magic trick with the sleeves rolled up: a handful of MOVB
instructions, no OS, no driver, no permission — and a device answers you. From here to the end of
the book, "writing a driver" means exactly this trick plus vocabulary and discipline. Chapters
12, 19, 20, and 25 are the vocabulary, one device at a time; the discipline you already started
learning in §5.4.

The listing is `code/ch05/mmio.a99`, and it has no ceremony at all — aim the GROM's address
pointer, then read it back twice:

```asm
GRMWA  EQU  >9802            GROM read-address port
GRMSA  EQU  >9C02            GROM write-address port

START  LWPI WS               a workspace of our own
       LI   R0,>6000         an address to aim the GROM at
       MOVB R0,@GRMSA        write the address, high byte first ...
       SWPB R0
       MOVB R0,@GRMSA        ... then the low byte
       MOVB @GRMWA,R1        read it back (one byte) ...
       MOVB @GRMWA,R2        ... read again: a DIFFERENT byte
HALT   JMP  HALT
```

Run it on the bare bench and read the trace:

```text
>6026  D800 9C02  MOVB R0,@>9C02    49 cycles
>602A  06C0       SWPB R0           14 cycles
>602C  D800 9C02  MOVB R0,@>9C02    55 cycles
>6030  D060 9802  MOVB @>9802,R1    47 cycles
>6034  D0A0 9802  MOVB @>9802,R2    47 cycles
   ... R1 =>6000   R2 =>0100
```

Three things are worth naming. First, the two `MOVB R0,@>9C02` instructions are *stores* by their
opcode and *commands* by their effect: they hand the GROM the two bytes of an address, high then
low (the `SWPB` between them rotates the low byte up into position — the high-byte law of Ch. 4 at
work), and afterward the chip's internal pointer is aimed. Nothing was "saved" anywhere a later
load could retrieve unchanged; the bytes went into a machine and made it move. Second — and this is
the sentence to keep — the two reads of the *same address* `>9802` return *different bytes*: `>60`
lands in R1's high byte, then `>01` lands in R2's, because each read advances the port. A read here
is a question, and asking changes the answerer. Third, mind the price: a `MOVB` to or from a port
runs 47 to 55 cycles where a register `MOVB` runs 14. Part of that is the funnel; the larger part
is that the GROM is a slow serial device the CPU physically waits on — the reason the whole GPL OS
crawls next to CPU-RAM code, and a story Ch. 25 tells in full.

You just wrote a device driver. From here to the end of the book, "writing a driver" is this trick
plus vocabulary: Ch. 12 gives the VDP its words, Ch. 19 the sound chip, Ch. 20 speech, Ch. 25 the
GROM — one device at a time.

> **Pitfalls.**
> - *Reading is not looking.* Several addresses in the `>8000` block react to reads — VDP, GROM,
>   speech. Never sweep memory "just to see" through the control block; use the bench's device
>   views instead.
> - *Word operations ignore the least-significant address bit* (A15 in TI's numbering — Ch. 4).
>   Ask for a word at `>8301` and the machine silently serves `>8300`: no fault, no warning. Near
>   the ports this matters doubly — they are byte devices, so address them with MOVB.
> - *The pad is a shared kitchen.* `>83E0`–`>83FF` is the GPL interpreter's workspace, and the
>   console's interrupt machinery keeps state in the pad too. Claim only what the §5.2 table marks
>   claimable — or mask interrupts and know exactly what you just turned off (Ch. 22).
> - *Speed tuned on the island does not travel.* A loop that just fits the frame budget (50,000
>   CPU cycles per 60 Hz frame — Ch. 4) when it runs in the scratchpad can miss it badly from
>   expansion RAM. Tune where you will ship, or budget with the §5.3 table before tuning at all.
> - *Nothing faults.* A wild pointer raises no exception on this machine — it writes through
>   mirrors onto live pad state, or hands the sound chip a command, and the machine keeps smiling.
>   Your only guards are the map in your head and the bench in your hand.

## 5.7 The Memory Budget: The Worksheet Every Project Fills First

Every project in the rest of this book — every lab, every library, every game — begins the same
way: not with code, but with a map. Before the first instruction is written, one small table gets
filled in: which regions this program claims, for what, and why *there* — each placement defended
in bytes (the region sizes of §5.1) and in cycles (the tolls of §5.3). We call it the memory
budget, and this section is where the blank form lives.

The worksheet earns its keep by forcing the three placement decisions of §5.5 explicitly and
early. Where does code live — expansion, cartridge, a hot loop staged into the pad? Where do
workspaces live — the pad if at all possible, and which claimable corner? Where does data live —
and does bulk data even belong on this side of the funnel, or across it in the VDP's 16 K? (That
last column we only *reserve* today; Ch. 12 teaches you to spend it.) Programs that skip these
questions still answer them — by accident, at four cycles per access, discovered late in Ch. 22's
profiler with a deadline looming.

Fill one out now for something tiny — the Lab's timing rig is the worked example below — and the
ritual will feel like overkill. That is fine. DODGE (Ch. 16) will fill this same form under
sprite pressure, TERRAIN (Ch. 17) under map-data pressure, and SKELETON99 (Ch. 36) will hand it
to you pre-filled at the start of every new program. By then the form is not paperwork. It is the
first draft of the program.

Here is the blank form. Every column is already familiar: the ranges and sizes are §5.1's, the
width-and-toll is §5.3's, and the rest is where you commit.

| Region | Range | Size | Width / toll | Claimed for | Used | Free | Notes |
|---|---|---|---|---|---|---|---|
| Console ROM | `>0000`–`>1FFF` | 8 K | 16-bit / 0 | — firmware | — | — | not yours |
| Low expansion | `>2000`–`>3FFF` | 8 K | 8-bit / +4 |  |  |  |  |
| DSR window | `>4000`–`>5FFF` | 8 K | 8-bit / +4 | — cards | — | — | Ch. 30 |
| Cartridge ROM | `>6000`–`>7FFF` | 8 K | 8-bit / +4 |  |  |  | code + constants |
| Scratchpad | `>8300`–`>83FF` | 256 B | 16-bit / 0 |  |  |  | mind the tenants (§5.2) |
| High expansion | `>A000`–`>FFFF` | 24 K | 8-bit / +4 |  |  |  | where programs live |
| VRAM | via VDP ports | 16 K | ports only | *reserved* | — | — | budgeted from Ch. 12 |

And here it is filled in for the smallest program we have — this chapter's timing rig — with the
byte counts taken straight from `rig.a99`'s listing and symbol map, not estimated:

| Region | Range | Size | Width / toll | Claimed for | Used | Free | Notes |
|---|---|---|---|---|---|---|---|
| Cartridge ROM | `>6000`–`>7FFF` | 8 K | 8-bit / +4 | rig code + synthesized header | 80 | 8,112 | resident probe + `PLANT` |
| Scratchpad | `>8300`–`>83FF` | 256 B | 16-bit / 0 | workspace + staged probe | 32 + 6 | 218 | WS at `>8300`, probe at `>8340` |
| Low expansion | `>2000`–`>3FFF` | 8 K | 8-bit / +4 | probe copy (race target) | 6 | 8,186 | measurement only |
| High expansion | `>A000`–`>FFFF` | 24 K | 8-bit / +4 | probe copy (race target) | 6 | 24,570 | measurement only |
| VRAM | via VDP ports | 16 K | ports only | *reserved* | — | — | the rig draws nothing |

For a program this small the form feels like overkill, and that is exactly the right first
impression: the ritual is cheap now precisely so it is habit later, when it is not. DODGE (Ch. 16)
will fill this same table under sprite pressure, TERRAIN (Ch. 17) under map-data pressure, and
SKELETON99 (Ch. 36) will hand it to you already filled at the top of every new program. By then
the worksheet is not paperwork you do before the work. It is the first draft of the program.

## Lab 5 — The Timing Rig: One Loop, Four Neighborhoods

Everything this chapter claims, you now verify with your own hands. That is the lab's whole
personality, and it will be the book's: BENCH99 will hand you cycle counts all day, but only a
rig *you assembled* makes the numbers yours. The rig is also your first taste of
position-independent thinking, because its entire trick is that identical bytes must run in four
different neighborhoods and mean the same thing in each.

The rig is one small cartridge image. It carries the probe loop — the same few
position-independent instructions — resident at `>6000`, plus a planting routine that copies that
loop to `>8300`, `>2000`, and `>A000`. The bench does the rest: aim the program counter at each
copy, step, and read per-instruction costs straight off the trace. By the end you will have
produced your own copy of the §5.3 table and checked it against the book's — and from this
chapter forward, "because I measured it" is the only performance argument this book accepts.

1. Assemble `code/ch05/rig.a99` with the canonical build line (Ch. 3's skeleton — listing and
   symbol map on).
2. Load it under BENCH99 and let it plant the copies.
3. Race all four neighborhoods; log cycles per lap in each.
4. Move the workspace between pad and expansion and race again — the fetch/operand split, now in
   your own transcript.
5. Reconcile every number against T = C + 4 × (slow accesses). Any discrepancy is a lesson;
   chase it before you read on.

Here is the whole rig. The probe is three position-independent words; `PLANT` copies those `PLEN`
bytes wherever R1 points; `START` plants three copies and spins.

```asm
WS     EQU  >8300            measurement workspace (the fast pad)
PADLP  EQU  >8340            planted copy: scratchpad    (fast island)
LOWLP  EQU  >2000            planted copy: low expansion  (funnel)
HIGHLP EQU  >A000            planted copy: high expansion (funnel)

START  LWPI WS               a workspace of our own
       LI   R1,PADLP         plant into the pad ...
       BL   @PLANT
       LI   R1,LOWLP         ... into low expansion ...
       BL   @PLANT
       LI   R1,HIGHLP        ... and into high expansion
       BL   @PLANT
DONE   JMP  DONE             planted; spin (the bench breakpoints here)

PLANT  LI   R0,PROBE         source = the resident probe
       LI   R2,PLEN          byte count
PL1    MOVB *R0+,*R1+        one byte, both pointers step
       DEC  R2
       JNE  PL1
       RT                    ; back to caller (B *R11)

PROBE  MOV  R0,R1            fetch + read source + write dest
       A    R0,R1            fetch + read source + read dest + write dest
PSPIN  JMP  PSPIN            fetch only
PLEN   EQU  $-PROBE          probe length, in bytes
       END  START
```

Build it with the canonical line (Ch. 3's skeleton — `.ctg` for the emulator, a raw `.bin` for the
bench to `load`, listing and symbol map on so you can find `START` and the planted copies):

```sh
libre99asm rig.a99 --name 'CH5 RIG' -o build/rig.ctg \
    --listing build/rig.lst --symbols build/rig.map.json
libre99asm rig.a99 --name 'CH5 RIG' --format bin -o build/RIGC.bin
```

The symbol map reports `START` at `>601C` and the probe resident at `>604A`. Now `code/ch05/race.txt`
mounts the image, plants the copies, and races each neighborhood. The opening and one lane:

```text
load build/RIGC.bin        # mount the rig ROM at >6000
pc 601C                    # aim the CPU at START
u 6038                     # run START: plant the copies, break at DONE
wp 8300                    # workspace on the island
pc 8340                    # the pad copy of the probe
s 3                        # step MOV, A, JMP and read each cost
```

Change `pc` to `>2000`, `>604A`, or `>A000` to move the *code* between neighborhoods; change `wp`
to an expansion address (the script uses `>A100`) to move the *operands*. The extremes of the
run — code and workspace both on the island, then both across the funnel — read like this:

```text
>8340  C040  MOV R0,R1   14 cycles      # all-pad
>8342  A040  A R0,R1     14 cycles
>8344  10FF  JMP >8344   10 cycles
   ...
>A000  C040  MOV R0,R1   26 cycles      # all-slow  (wp >A100)
>A002  A040  A R0,R1     30 cycles
>A004  10FF  JMP >A004   14 cycles
```

Your numbers should match §5.3's table lane for lane. If one does not, the rig is telling you
something true about your *setup*, not the machine — suspect, in order: a workspace pointer left in
the wrong region (an operand toll where you expected none, or none where you expected one);
interrupts unmasked, so the console ISR stole cycles mid-measurement (a boot-mode hazard only — the
bare bench never interrupts); or a probe straddling a region edge, so one fetch crossed the funnel
and the next did not. Chase the discrepancy before you read on: on this machine, a number you
cannot explain is a bug you have not found yet.

> **Field Notes — How Period Software Detected the 32 K.** The memory expansion was a peripheral,
> not a promise: it lived in an outboard expansion box, cost real money, and no publisher could
> assume a buyer owned one. So commercial software that could *use* 32 K had to ask the machine
> first — and the machine offers no polite inventory service. There is no configuration table, no
> firmware call to consult; there is only the map. Period programs asked the only way the
> hardware allows: empirically. Write a value where the expansion would be; read it back; if the
> neighborhood remembers, somebody lives there. Careful programs wrote *two* patterns — a value
> and then its complement — so an open bus that happened to echo the test byte back could not fool
> them; the truly careful restored whatever they had overwritten, in case the RAM was there and
> already in use. It is the same move your rig makes in this chapter's lab, dressed up as a
> product feature — the machine's geography probed at runtime, because geography was optional
> equipment. Here is that move as a subroutine (`code/ch05/det32k.a99`): `PROBE` writes a pattern
> and its complement to the same cell, and the caller trusts a region only if both survive.
>
> ```asm
> START  LWPI WS
>        CLR  R0               verdict = absent, until proven otherwise
>        LI   R1,>2000         probe the low expansion ...
>        BL   @PROBE
>        JNE  DONE             ... missing -> absent
>        LI   R1,>A000         probe the high expansion ...
>        BL   @PROBE
>        JNE  DONE             ... missing -> absent
>        SETO R0               both answered -> present (R0 = >FFFF)
> DONE   JMP  DONE
> PROBE  LI   R2,>A55A         a pattern with no accidental symmetry
>        MOV  R2,*R1           write it
>        C    R2,*R1           did it survive?
>        JNE  PBAD             no -> fail (EQ clear)
>        INV  R2               the complement, >5AA5, foils an echoing bus
>        MOV  R2,*R1
>        C    R2,*R1           did THAT survive too?
> PBAD   RT                    ; the compare's EQ flag is the verdict
> ```
>
> On the bench the verdict returns `>FFFF` — present — because the project's standard machine always
> carries the 32 K; the emulator models no unexpanded console, so the negative case cannot be run
> here (a gap logged for the project). What a real console returns from *absent* expansion space is a
> read of an open bus — no RAM drives the lines, so the value is unowned and unreliable (commonly a
> remnant of the last bus traffic), which is exactly why the two-pattern test, not a single read, is
> the honest probe. A shipping version restores the original word after testing; that polish is
> Exercise 7.

## Exercises

1. ✦ Classification drill. For each of `>0042`, `>20F0`, `>4001`, `>7FFE`, `>8377`, `>8C02`, and
   `>B000`: name the region, its bus width, its wait class, and one thing that lives there.
2. ✦ Using T = C + 4 × (slow accesses) and the §5.3 table, predict the per-lap cost of `JMP $`
   placed at `>8320` and at `>A020`, then explain in one sentence where the difference comes from.
3. ✦ The 32 K expansion arrives split 8 K + 24 K. Answer why using nothing but the map of §5.1.
4. ✦✦ A word access to an odd address: state what the 9900 does with the least-significant
   address bit, then design (and run) a two-command BENCH99 experiment — one `pw`, one `r` — that
   proves your answer.
5. ✦✦ The GPL interpreter — Floor 2 of the tower — lives in console ROM. Using the two-domain
   map, explain why TI put the interpreter there, and what would happen to every GPL program on
   the machine if it ran from `>A000` instead.
6. ✦✦ Retarget the lab rig at the peripheral window, `>4000`, with no card switched in. Predict
   the outcome from §5.4 before you run it; then run it and reconcile, using the §5.3 table for
   comparison.
7. ✦✦ Write the 32 K detector from the Field Notes as a standalone program that reports its
   verdict in R0, and demonstrate it on the bench.
8. ✦✦✦ Account for every cycle between the three placements of `MOV R0,R1` (14, 18, and 30):
   list each memory access the instruction makes and which address it hits in each placement.
   Then predict `A R0,R1` in the same three placements and check yourself against the §5.3 table.
9. ✦✦✦ From the mirror set measured in §5.4, infer which address bits the scratchpad's decoder
   actually examines, and sketch the minimal decode logic as an ASCII gate diagram.
10. ✦✦✦ The pad-copy trick of §5.5 is not free — the copy itself costs cycles. Using the §5.5
    measurements, derive the break-even lap count below which copying the loop into the pad costs
    more than it saves, and state it as a rule of thumb for later chapters.

## Further Reading

- The *Editor/Assembler* manual (Texas Instruments) — the period programmer's ground truth for
  the memory map and console conventions; we formally adopt its world in Ch. 6.
- Heiner Martin, *TI Intern* — the console ROM disassembled and the scratchpad's tenants named;
  the reference standing behind §5.2's table and Ch. 24's full atlas.
- The TMS9900 datasheet — the source of every C in the timing model; its instruction timing is
  audited against the machine in App. A.
- Classic99's source, `console/Tiemul.cpp` and `console/tivdp.cpp` (Tursi) — a hardware-verified
  emulator's view of the bus, the multiplexer, and the port semantics this chapter measures.
- The project's own source — crate `libre99-core` models the funnel this chapter clocks, and the
  repository README documents the desktop emulator and BENCH99. Read the Rust behind your
  stopwatch; that transparency is the point of building the book on it.
- *MICROpendium* (1984–99) — the community's paper of record; its hardware coverage is a period
  window onto how working programmers lived with — and around — this map.

## Summary

- The full map, south to north: `>0000` console ROM (8 K, 16-bit — vectors, GPL interpreter, ISR,
  FP package) · `>2000` low expansion (8 K) · `>4000` DSR window · `>6000` cartridge ROM (8 K) ·
  `>8000` control block (scratchpad + ports) · `>A000` high expansion (24 K). Ports: sound
  `>8400` (w); VDP `>8800`/`>8802` rd, `>8C00`/`>8C02` wr; speech `>9000` rd / `>9400` wr; GROM
  `>9800`/`>9802` rd, `>9C00`/`>9C02` wr — all side-effectful.
- Two domains: the fast island (console ROM + scratchpad, 16-bit, zero-wait) versus everything
  behind the funnel; §5.3's rig measured the toll as a flat **+4 per word crossing**, identical at
  `>2000`, `>6000`, and `>A000`, upgrading the "≈ +4 nominal" ledger row to measured; the law
  stands — speed is a property of addresses, not instructions.
- Fetch and operand tolls are separate line items: `JMP $` = 10 pad / 14 slow; `MOV R0,R1` = 14
  all-pad / 18 code-slow / 26 all-slow on our core against the datasheet's **30** — the MOV
  destination pre-read deviation, still open at commit `2da67ae`; `A R0,R1` models the pre-read and
  measures the correct 14 / 18 / **30**, so it is the honest probe where that toll matters; the
  `cpu.rs` fix was surfaced to the project, not written from the book.
- Scratchpad `>8300`–`>83FF`: the machine's only 16-bit RAM — and an unexpanded console's only
  CPU RAM at all; first coarse tenant map given (GPL WS `>83E0`, GPL status `>837C`, ISR/GPL churn
  in `>83C0`–`>83FF`, quiet claimable corner near `>8320`–`>836F`); byte-level atlas deferred to
  Ch. 24 / App. C.
- Partial decoding: the pad mirrors four times across `>8000`/`>8100`/`>8200`/`>8300` (the decoder
  ignores A8–A9), and a write through a mirror lands on the live pad; unowned space never faults;
  reading ports has side effects (two reads of GROM `>9802` returned `>60` then `>01`) — reading is
  not looking.
- The 32 K arrives split 8 + 24 because `>4000`/`>6000`/`>8000` were already spoken for; the
  hot-loop-into-pad trick was demonstrated and measured (padrun.a99): 1000 laps fell from **28,062
  to 20,058 cycles**, a **1.40× speedup**, by moving the loop's bytes onto the island.
- §5.7 established the memory-budget worksheet (blank form: `code/ch05/budget-template.md`) — every
  later project fills it first; the VRAM column is reserved until Ch. 12.
- Artifacts in `code/ch05/`: rig.a99 + race.txt (the timing rig, Lab 5), padrun.a99, mmio.a99,
  det32k.a99, budget-template.md.
