# System-ROM rewrite — implementation status

Honest ledger of what is built and verified. **Both tracks are complete**: the
console-GROM rewrite (all planned milestones M0–M7) and the console-ROM
rewrite (M1–M5, M7, M8; M6 — TI BASIC proper — deferred indefinitely by
policy, **minus the justified XB-substrate subset that landed 2026-07-07**:
five BASIC-era ROM helpers at their authentic addresses, census-bounded in
[`XB-CENSUS.md`](./XB-CENSUS.md), with which **Extended BASIC runs end-to-end
on the clean-room pair** — L9). Since **2026-07-06 the pair boots as the
emulator's default firmware** (the authentic TI images stay bundled, selected
via `--system-rom` / `--system-grom`). **TI PYTHON grew from the v0 integer
REPL to the spec'd v1 mini-language the same day** (docs/TI-PYTHON.md;
L3 resolved). Open gaps live in [`LIMITATIONS.md`](./LIMITATIONS.md); the
interface facts everything rests on in [`RECON.md`](./RECON.md) and
[`rom/RECON.md`](./rom/RECON.md); the debugging method in
[`GROM-DEBUGGING-GUIDE.md`](./GROM-DEBUGGING-GUIDE.md) +
[`DEBUGGING.md`](./DEBUGGING.md). The executed plans, quality assessments, and
execution ledgers are archived in [`history/`](./history/).

> ✅ **The L1–L7 ledger is Resolved / deferred-by-decision** (the
> [`history/QUALITY-ASSESSMENT.md`](./history/QUALITY-ASSESSMENT.md) §7 closure plan, executed). **L1, L2,
> L4, L5, L6, L7 Resolved** (each with a commit + gate); **L3 deferred by
> decision** to the TI PYTHON track (the REPL banners `TI PYTHON 0.0.1`). Within
> L6, cassette (no emulator hardware → ROADMAP §6) and the general unshipped GROM-2
> library are **deferred-by-decision**.
>
> ⚠ **One open entry — L8 (2026-07-04).** A ship-review pass strengthened the
> coverage sweep's post-launch check from "did not reboot" into a **differential
> health panel** (is our console as alive as the authentic one after launch?). It
> found that **Video Vegas** — one of the 137 bundled carts — launches to a *dead
> console* under our GROM (display off, ISR masked), because it hard-depends on an
> unshipped GROM-2 library routine (an on-demand L6-class gap, realised by a
> bundled cart). This corrects L6's "no bundled cart needs it" by one cart. It is
> **gated** (waived by name so no *second* regression can slip in) with a scoped
> path forward. See [`LIMITATIONS.md`](./LIMITATIONS.md) L8. New reports go through
> the §8 triage policy (DEBUGGING.md protocol Step 0.5).

## Done and verified

| Milestone | State | Evidence |
|---|---|---|
| **Recon (R1, most of R2, census)** | ✅ | `RECON.md`; `crates/libre99-core/examples/recon_probe.rs` |
| **GPL toolchain (assembler, decoder, disassembler)** | ✅ | `crates/libre99-gpl/` — GAS operand model, two-pass assembler; unit + roundtrip tests |
| **M0 — our GPL runs on the real ROM** | ✅ | `crates/libre99-gpl/tests/boot_trivial.rs` (BACK reaches VDP R7) |
| **M1 — recreated master title screen** | ✅ | `crates/libre99-gpl/tests/title_screen.rs`; `grom/console.gpl`; screenshots via `examples/title_shot.rs` (authentic recon in `examples/title_recon.rs`) |
| **M2 — the selection list** | ✅ | `crates/libre99-gpl/tests/menu.rs`: lists console GROM 1 (TI PYTHON) + cartridge GROM/ROM programs, reads a digit, launches GPL carts (sub-stack trampoline) and ML carts (`XML >F0`). Needed the keyboard scan-code table (`src/keymap.rs`, RECON §9). |
| **M3 — cartridge compatibility sweep** | ✅ | `crates/libre99-gpl/tests/sweep.rs`: sample carts across every class + an ignored full sweep — **137/137** bundled images list exactly (the two far-list outliers, `starpeg`/`xb25`, now covered — L2 Resolved). |
| **M4 — TI PYTHON** (v1 since 2026-07-07) | ✅ | `crates/libre99-gpl/tests/ti_python.rs` (12 gates): the Python-like mini-language in console GROM 1 — full-size names (VRAM table), Python floor `/`·`//`·`%`, real unary minus, `print(…)`/strings/`#` comments/`exit()`, `>>> ` prompt, scrolling screen, block cursor, and the KSCAN new-key input engine. Spec of record: [`docs/TI-PYTHON.md`](../../docs/TI-PYTHON.md). |
| **ISR / interrupts (sound, sprites, QUIT)** | ✅ | `crates/libre99-gpl/tests/interrupts.rs`: `START` arms the 9901 VDP interrupt (CRU bit 2) with a GPL `IO`, so the console ROM's VBLANK ISR runs — the Tunnels of Doom splash tune plays and QUIT (`FCTN`+`=`) reboots to our title (`tests/sweep.rs::quit_returns_to_our_title`). Root-cause trace in `DEBUGGING.md` case study 1. |
| **Emulator integration (M6)** | ✅ | `--system-grom`/`--system-rom` flags; committed `grom/console-grom.bin`; `grom/README.md` |
| **M7 — console device I/O (disk loading)** | ✅ | `crates/libre99-gpl/tests/device_io.rs`: the interconnect table + an original **DSRLNK** (slot `>0010` → `>1200`) and a boot **peripheral DSR power-up scan** let a cartridge load a file from disk — Tunnels of Doom loads a QUEST scenario from `Tunnels.Dsk` and reaches `NEW DUNGEON`. Both delegate device work to the kept ROM via `XML >19/1A`; clean-room from the traced interface (`RECON.md` "Console device I/O"; `DEBUGGING.md` case study 2). Closes the disk path of `LIMITATIONS.md` L6. |
| **Console character-set loaders** | ✅ | `crates/libre99-gpl/tests/char_set.rs`: interconnect slots `>0016`/`>0018` load the console's standard/thin fonts into the VDP pattern table at the caller's `>834A` dest, so a cartridge that draws text with the console font renders it — **TI Invaders' opening screen is now pixel-identical to authentic** (was blank text, sprites-only). Clean-room from the traced interface (`RECON.md` "Console character-set loaders"; `DEBUGGING.md` case study 3). Advances `LIMITATIONS.md` L6. |
| **Hardening Chunk 1 — surface map + data homes** | ✅ | `crates/libre99-gpl/src/census.rs` + `examples/grom_census.rs` (the byte census, GROM-0 tally matches QUALITY-ASSESSMENT §3); [`grom/SURFACE-MAP.md`](./grom/SURFACE-MAP.md) classifies every GROM-0 authentic-only run; `tests/census.rs` gates byte-identity of the `DATA-MUST-MATCH` set, map completeness, and the chip-gap-zero invariant. **B1**: the fonts now ship at their authentic homes (`>04B4`/`>06B4`) and an original beep at `>0484`. **B4**: `FONT2` relocated out of the `>1800` chip gap into GROM 2 (`>4000`). QUALITY-ASSESSMENT §6 A1 + B1 + B4; execution log archived at [`history/QUALITY-ASSESSMENT-PROGRESS.md`](./history/QUALITY-ASSESSMENT-PROGRESS.md). |
| **Ship-review — differential health gate** | ✅ | `tests/coverage_sweep.rs` now launches every cart under **both** our GROM and authentic and asserts, per cart, that ours is never *less alive* (display on + ISR ticking) than authentic — the automated "play each game until it looks wrong" (QUALITY-ASSESSMENT §C2). Turns "did not reboot" into "still running." Found **L8** (Video Vegas, the one open entry); the other 136 pass, 17 arcade carts correctly classed faithful machine-takeovers. Also: per-cart tripwire attribution + a font-home interface-data safety assertion. |

The rewrite boots the genuine console ROM to an **original recreation of the
master title screen**: the authentic layout (colour bars top and bottom, the
`TEXAS INSTRUMENTS` / `HOME COMPUTER` banner, `READY-PRESS ANY KEY TO BEGIN`,
and the power-on beep) drawn in the genuine 8×8 character set — but with TI's
logo and copyright replaced: the "TI" logo by an **original Texas + 99 emblem**
and `© 1981 TEXAS INSTRUMENTS` by `© 2026 JOEL ODOM`. The master selection
screen matches the same authentic layout. It is the emulator's **default
boot** — run it with plain:

```sh
cargo run -p libre99-app -- --no-cartridge
```

(To boot the authentic firmware for comparison:
`--system-rom roms/994aROM.Bin --system-grom roms/994AGROM.Bin`.)

**Performance parity (P1, gate `tests/perf_parity.rs`).** Booting the console ROM
on our GROM vs the authentic one, with a cart mounted, our rewrite reaches both
usable screens **sooner**: frames-to-title **11 vs 41** (the authentic boot spends
its time on a ROM/GROM checksum + full charset copy our rewrite skips),
frames-to-menu (reset → cart listed) **24 vs 47**. The isolated menu-*build*
segment is the one place we cost more (SPACE → cart listed: **10 vs 3** — our
visible `SCANNING` pass), but the faster title more than covers it. Numbers are
from the emulator (which runs GPL far faster than 1981 silicon), so they are a
relative parity tripwire, not wall-clock; the gate asserts ours ≤ authentic ×1.25
on both from-reset metrics.

## Verified GPL instructions (execution-anchored)

The assembler implements the **complete GPL ISA** from Classic99's
authoritative 256-entry table (`../classic99/addons/gpl.cpp`, consulted never
copied), organised as format-1 two-operand families (`ADD SUB MUL DIV AND OR
XOR ST EX CH CHE CGT CGE CEQ CLOG SRA SLL SRL SRC` — `D` prefix for word
forms, immediate/memory source auto-selected), format-5 single-operand
families (`ABS NEG INV CLR FETCH CASE PUSH CZ INC DEC INCT DECT`), `MOVE` with
its exact `001GRVCN` bit field, the named ops (`RTN RTNC RAND SCAN BACK B CALL
ALL EXIT PARSE XML BR BS …`), and the GAS operand grammar (short/12-bit/16-bit
CPU-biased forms, CPU byte-pointer and VDP word-pointer indirection). Key
members are execution-verified on the real ROM (`examples/*_probe.rs`,
RECON §§1–8); the decoder tiles the authentic firmware's own instruction
streams byte-exactly. Indexed addressing and MOVE's C=1 form failed
verification for **our emitter** and are rejected/banned in our GPL source
(RECON §7) — a *what-we-write* rule; the rewritten console ROM's interpreter
still **executes** those forms correctly for foreign carts that use them
(rom/RECON §15/§25, M4 slice 2).

## What comes next

- **Phase 3 — the Disk Controller DSR rewrite — COMPLETE (2026-07-06,
  M1–M6).** The clean-room replacement for `roms/Disk.Bin` — the `>AA`
  header + power-up VRAM reservation, the FD1771 driver, subprograms
  `>10`–`>16` (incl. stock single-density FORMAT), the full PAB file system
  in every mode, and the byte-exact on-disk format — **now installs by
  default** (`--disk-dsr roms/Disk.Bin` selects the authentic image, which
  stays embedded for comparison). 24 differential gates
  (`crates/libre99-gpl/tests/disk_dsr.rs`) hold it to the genuine DSR:
  image-level byte-identity on the write flows, cross-oracle interop both
  directions, ToD's disk load under both console firmwares, robustness, and
  a staleness gate on the committed `disk-dsr.bin`. With this, **no TI
  firmware executes anywhere in the default configuration.** Dossier:
  [`disk-dsr/RECON.md`](./disk-dsr/RECON.md); ledger + deep-tier follow-ups
  (fuzz, perf tripwire): [`disk-dsr/PROGRESS.md`](./disk-dsr/PROGRESS.md).
- **L8 — the unshipped GROM-2 library (the one open ledger entry).** Two threads,
  both detailed in [`LIMITATIONS.md`](./LIMITATIONS.md) L8: (a) **fix Video Vegas** —
  implement the GROM-2 routine that interconnect slots `>002C`/`>0032` target so it
  launches instead of wedging (sized at one routine; repro
  `examples/isr_regression_probe.rs`); (b) **enumerate all dependents** — a static
  call-scan of every cart for control-transfers into the stubbed `>0010-005F`
  entries (L8 records the feasibility + difficulty + the over/under-approximation
  caveats so the analysis is not lost). Gated meanwhile (differential health panel +
  named waiver).
- **The L1–L7 ledger closure ([`history/QUALITY-ASSESSMENT.md`](./history/QUALITY-ASSESSMENT.md) §7) is
  complete** — all six hand-off chunks landed (2026-07-02 → -07-04):
  **Chunk 1** surface map + authentic data homes (census gate, fonts at
  `>04B4`/`>06B4`); **Chunk 3** console robustness (reset sound mute — Joel's F5
  "no fun beep", case study 7; menu 9-cap; DSRLNK bad-device waived by execution);
  **Chunk 4** L7 reject-beep + **L2** far-list carts (137/137 via `SCANW`/`SFAR`
  8 KiB re-copy) + **L5** `SCANNING` cue; **Chunk 2** the differential harness
  (structure-handoff audit → RECON; loud-stub grid; the GROM read-coverage
  instrument; the all-cart `coverage_sweep` → `grom/COVERAGE-REPORT.md`; the
  `conformance` state-contract harness incl. the F5 reset-drift guard);
  **Chunk 5** L6 service-surface closure (Resolved); **Chunk 6** close-out
  (perf-parity gate, §8 triage → DEBUGGING Step 0.5, the two-tier gate in
  `grom/README.md`, this §7.6 walk). Two field bugs were fixed en route — the F5
  launch half (`>8305`, case study 9) and a loud-stub reboot regression the
  coverage sweep caught (Parsec, case study 10). Decisions recorded: TI PYTHON
  deferred (L3, banner `0.0.1`); cassette → emulator ROADMAP §6. New defects now
  go through the §8 triage policy, worked with
  [`GROM-DEBUGGING-GUIDE.md`](./GROM-DEBUGGING-GUIDE.md) +
  [`DEBUGGING.md`](./DEBUGGING.md).
- **Phase 2 — the console-ROM rewrite — COMPLETE** (2026-07-05; a separate
  track with its own front door, [`rom/README.md`](./rom/README.md)). The goal
  — an original 8 KiB console ROM so **no TI bytes remain in the boot path**,
  with both GROMs running on it — is met: **M1–M5, M7, and M8 are done and
  differentially verified** (kernel + the complete non-BASIC GPL interpreter,
  ISR, KSCAN in all modes, XML + device linkage, FMT, the cassette modem
  layer, and the bit-exact radix-100 floating-point package; boots are
  pixel-identical to the authentic ROM two screens deep with no loud-stub
  breadcrumb; ~130 differential microtests + the 256-opcode sweep + fuzz
  soaks + the firmware matrix). **M6 — the TI BASIC ROM half — is deferred
  indefinitely by policy**, with a tripwire test enforcing the written-
  justification rule — **one justified subset is in (2026-07-07): the XB
  substrate**, five BASIC-era helpers at their authentic addresses that make
  **Extended BASIC run end-to-end on the clean-room pair** (census, dossier
  and justification: [`XB-CENSUS.md`](./XB-CENSUS.md); gates:
  `libre99-asm/tests/xb_substrate.rs` + `libre99-gpl/tests/xb_smoke.rs`;
  [`LIMITATIONS.md`](./LIMITATIONS.md) L9). The committed
  artifact `rom/console-rom.bin` **boots by default** paired with our GROM
  (decision 2026-07-06). Status detail, the test estate, the maintenance
  layout ledger, and the house rules: [`rom/README.md`](./rom/README.md);
  the interface dossier: [`rom/RECON.md`](./rom/RECON.md); the executed plan
  and execution ledger: [`history/ROM-REWRITE-PLAN.md`](./history/ROM-REWRITE-PLAN.md)
  + [`history/ROM-PROGRESS.md`](./history/ROM-PROGRESS.md).
