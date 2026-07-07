# Appendix C — Memory Maps and the Scratchpad Atlas

<!-- Appendices · target ≈10 pp · companion to Ch. 5, 24 · finalized in the batch session -->
<!-- STATUS: DRAFTED (session 10, 2026-07-07) — reference appendix. The CPU memory map (C.1–C.2) is canon, established and measured in Ch. 5. The scratchpad byte table (C.3) is tier-1 for the project's default machine: the firmware-owned addresses were read from the clean-room console firmware source (original-content/system-roms/rom/console.asm), which deliberately preserves the classic Editor/Assembler scratchpad ABI — >8372 GPL data-stack pointer, >8374 KSCAN mode, >8375 key code, >8376/>8377 joystick, >8378 random result, >8379 SPEED/frame timer, >837A auto-motion sprite count, >837C GPL status, >83C0 INTWS+seed, >83C2 ISR duty bits, >83C4 user hook, >83CC/>83CE sound-list pointer/countdown, >83E0 GPLWS. The frame timer >8379 and user hook >83C4 were additionally machine-verified in boot mode (Ch. 22); the lib99 layout (>8300 workspace, stack from >8340, CURPOS/CUR40) is verified across Ch. 9–24. FAC >834A / ARG >835C are the E/A floating-point accumulators (Ch. 23). Evidence tier is noted per row. -->

Where a byte lives decides how fast it is and who else wants it. This appendix is
the machine's address space on two scales: the **whole 64 K** the CPU sees
(C.1–C.2), and then the **256 bytes** that matter most — the scratchpad, the only
fast RAM the CPU has, mapped byte by byte (C.3). The narrative, the measured
wait-state costs, and the political history of those 256 bytes are Chapters 5 and
24; this is the reference card those chapters point back to.

A note on authority. The firmware-owned scratchpad addresses below are read from
the project's own clean-room console firmware — the machine the emulator boots by
default — and are therefore tier-1 for *this* platform. The clean-room firmware
deliberately keeps the classic Editor/Assembler scratchpad layout, so these are
also, in the main, the standard TI addresses; where a program targets a different
firmware it should confirm against that firmware's own map.

## C.1 The console memory map (`>0000`–`>FFFF`)

The CPU's 64 K address space. Only the console ROM and the scratchpad are 16-bit,
zero-wait ("the fast island," Ch. 5); everything else is 8-bit expansion RAM or
memory-mapped device behind the multiplexer, and pays the wait tax on every
access.

| Range | Size | Contents | Width / speed |
|---|---|---|---|
| `>0000`–`>1FFF` | 8 K | Console ROM — GPL interpreter, ISR, floating-point package | 16-bit, 0 wait |
| `>2000`–`>3FFF` | 8 K | Low expansion RAM | 8-bit, multiplexed |
| `>4000`–`>5FFF` | 8 K | DSR window — peripheral-card ROM, paged in by a CRU bit (App G) | 8-bit, multiplexed |
| `>6000`–`>7FFF` | 8 K | Cartridge ROM (GROM-only carts leave this empty); may be bank-switched | 8-bit, multiplexed |
| `>8000`–`>83FF` | 1 K | Scratchpad RAM — 256 B at `>8300`, plus decode mirrors below it (C.2) | 16-bit, 0 wait |
| `>8400`–`>9FFF` | 7 K | The memory-mapped ports (C.2) | 8-bit, write/read-only |
| `>A000`–`>FFFF` | 24 K | High expansion RAM | 8-bit, multiplexed |

The two expansion blocks together are the "32 K memory expansion" (`>2000`–`>3FFF`
plus `>A000`–`>FFFF`); a bare console has neither, only the 256-byte scratchpad.

```text
  >0000 +===========================+  console ROM (fast island)
  >2000 |---------------------------|  low RAM  (8K expansion)
  >4000 |---------------------------|  DSR window (paged, per CRU)
  >6000 |---------------------------|  cartridge ROM (>=8K, may bank)
  >8000 |======= control block =====|  pad + mirrors, then the ports
  >A000 |---------------------------|  high RAM (24K expansion)
  >FFFF +===========================+
```

## C.2 The control block (`>8000`–`>9FFF`)

The strangest 8 K on the map: the scratchpad and every memory-mapped port,
marching up in `>0400` steps. GROM and VDP are reached only through these
ports — they are *not* in the CPU address space — and every port access is
side-effectful (reading VDP status clears it; the GROM/VDP address ports
auto-increment). See Appendix D (VDP), Appendix E (sound), and Chapter 25 (GROM)
for the port protocols.

| Address | Port | Direction |
|---|---|---|
| `>8300`–`>83FF` | Scratchpad RAM (256 B; mirrored down through `>8000`) | read/write |
| `>8400` | Sound chip (SN76489) | write only |
| `>8800` | VDP read — data | read |
| `>8802` | VDP read — status | read |
| `>8C00` | VDP write — data | write |
| `>8C02` | VDP write — address / register | write |
| `>9000` | Speech — read | read |
| `>9400` | Speech — write | write |
| `>9800` | GROM read — data (auto-increments) | read |
| `>9802` | GROM read — address | read |
| `>9C00` | GROM write — data | write |
| `>9C02` | GROM write — address | write |

**The mirrors.** The pad's address decoder ignores the bits that would
distinguish `>8000` from `>8300`, so the same 256 bytes answer at several
addresses across `>8000`–`>83FF` (Ch. 5 measures exactly which bits it watches).
Address the pad at `>8300` and treat the mirrors as a hardware curiosity, not a
resource.

## C.3 The scratchpad atlas (`>8300`–`>83FF`)

The 256 bytes, tenant by tenant. What you may take depends on the environment
(C.4); the addresses below are the firmware's when the firmware or GPL is alive.
"Tier" is the evidence tier (§`_stubs.md` 6): **1** = verified against the project
(core/firmware/bench); **2** = classic E/A ABI the firmware preserves.

**The program zone — the low pad, yours in a bare cartridge (C.4):**

| Address | Owner / meaning | Tier |
|---|---|---|
| `>8300`–`>831F` | Your **workspace** (R0–R15) — `lib99` puts it here; console "low scratch" when the firmware runs | 1 |
| `>8320`–`>833F` | Free lower pad — `lib99` hot variables | 1 |
| `>8340` | `lib99` **software-stack** top (R10 stack, grows down; Ch. 9) | 1 |
| `>8342` | `CURPOS` — `textlib` cursor (Ch. 13) | 1 |
| `>8346` | `CUR40` — `textlib40` cursor (Ch. 14) | 1 |

**The floating-point package (Ch. 23), active under E/A or a GPL host:**

| Address | Owner / meaning | Tier |
|---|---|---|
| `>834A`–`>8351` | **FAC** — floating-point accumulator (8-byte radix-100 real) | 2 |
| `>8354` | FP error byte | 1 |
| `>835C`–`>8363` | **ARG** — floating-point argument (8 bytes) | 2 |

**The firmware variable zone — the console's own state (clean-room firmware; classic ABI):**

| Address | Owner / meaning | Tier |
|---|---|---|
| `>8372` | GPL **data-stack** byte pointer | 1 |
| `>8374` | **KSCAN mode** — 0 full scan; 1/2 left/right split + joystick 1/2 (Ch. 21) | 1 |
| `>8375` | **KSCAN key code** returned *(also the FP sign byte — shared, see below)* | 1 |
| `>8376` | Joystick Y deflection *(also FP exponent scratch — shared)* | 1 |
| `>8377` | Joystick X deflection | 1 |
| `>8378` | **Random-number** result byte (the console's `RND`) | 1 |
| `>8379` | **SPEED / frame timer** — the ISR ticks it each frame (Ch. 22: reaches 60 after 60 frames) | 1 |
| `>837A` | Auto-motion **sprite count** (the VDP motion table; Ch. 16) | 1 |
| `>837C` | **GPL status byte** — condition flags the interpreter and services share (Ch. 26) | 1 |

> **The atlas's core lesson, in two bytes.** `>8375` and `>8376` serve KSCAN
> (key code, joystick Y) *and* the floating-point package (sign, exponent
> scratch). Nothing is wrong: KSCAN and a floating-point evaluation never run in
> the same instant, so the firmware lets them share the bytes. Two hundred
> fifty-six bytes stretch to cover a much larger machine precisely by this kind
> of time-sharing — which is also why a byte you assume is idle may be very much
> in use a microsecond later.

**The two system workspaces — the top 64 bytes, sacred while the OS runs:**

| Address | Owner / meaning | Tier |
|---|---|---|
| `>83C0`–`>83DF` | **Interrupt/utility workspace** (INTWS) — the ISR's 16 registers | 1 |
| `>83C0` | (word) the random-number **seed**, advanced by the ISR each frame | 1 |
| `>83C2` | ISR **duty/disable** bits: `>80` all · `>40` sprite motion · `>20` sound · `>10` QUIT | 1 |
| `>83C4` | **User interrupt hook** — a routine placed here runs every frame (Ch. 22) | 1 |
| `>83CC`–`>83CD` | Sound-list **pointer** (the auto-player; Ch. 19, App E) | 1 |
| `>83CE` | Sound-list frame **countdown** | 1 |
| `>83D4` / `>83D6` | Screen-timeout blank timers | 1 |
| `>83E0`–`>83FF` | **GPL workspace** (GPLWS) — the interpreter's 16 registers; R13 holds the GROM read port `>9800`, the high registers are the GPL PC / data-stack pointer / status | 1 |

## C.4 Occupancy by environment

The map *changes* with what else is running (Ch. 24, §24.3). Three environments,
three sets of what you may take:

| Region | Bare console | Under Editor/Assembler | GPL alive |
|---|---|---|---|
| `>8300`–`>836F` low pad | **yours** (nearly all) | yours, minus E/A's utility area | yours (the little you get) |
| `>8370`–`>83BF` firmware vars | free (no ISR running) | preserve KSCAN/FP bytes you use | **preserve** — in active use |
| `>83C0`–`>83DF` INTWS | free if you mask interrupts | preserve | **preserve** |
| `>83E0`–`>83FF` GPLWS | free if no GPL runs | preserve | **sacred** — never touch |

The rule that falls out (Ch. 24): **know your environment and preserve what it
owns.** A bare cartridge that has masked interrupts and runs no GPL owns almost
all 256 bytes — the freedom a self-contained game (Part IX) creates on purpose. A
program hosted by E/A or running alongside GPL owns the low pad and must save and
restore anything else it borrows. `lib99` obeys this by *naming* its few pad words
(`equates.inc`, Ch. 11) and touching no others, so its modules compose into any
environment without collision — and `PADWATCH` (Ch. 24) proves it, reporting the
exact bytes any routine changes.

## C.5 The `lib99` standard layout

Every case study in this book inherits one scratchpad template:

| Address | Use |
|---|---|
| `>8300`–`>831F` | Workspace (R0–R15) |
| `>8320`–`>833F` | Hot variables (module-named, below the stack) |
| `>8340` ↓ | Software stack top (R10 full-descending; Ch. 9) |
| `>8342`, `>8346` | `CURPOS`, `CUR40` and each module's few named words |
| `< >8370` | everything `lib99` uses stays **below** the firmware variable zone |

The template's whole virtue is that it fits under `>8370`, clear of the frame
timer, GPL status, and the workspaces above, so a program that composes several
`lib99` modules never collides with the console — and because each address is a
named constant, a conflict is a compile-time name clash rather than a runtime
ghost.

*See also:* Chapter 5 (the memory map, measured), Chapter 24 (the scratchpad
atlas and `PADWATCH`), Appendix K (the console entry points and the full
scratchpad interface-variable catalog), Appendix G (the CRU map behind the
keyboard/joystick reads), Appendix D (the VDP ports), Appendix E (the sound port
and the sound-list the `>83CC` pointer feeds).
