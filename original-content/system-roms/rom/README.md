# The Libre99 Console ROM

A **from-scratch, clean-room rewrite of the TI-99/4A console ROM** — the 8 KiB
TMS9900 image at CPU `>0000-1FFF` that *is* the machine's operating system:
the GPL interpreter, the VBLANK ISR, the keyboard scanner, the device linkage
(DSR search/call), the screen-format sub-language, the cassette modem layer,
and the radix-100 floating-point package.

- **The artifact:** [`console-rom.bin`](./console-rom.bin) (8192 bytes),
  assembled from [`console.asm`](./console.asm) by our own `libre99asm`
  (`cargo run -p libre99-asm --bin libre99asm -- rom
  original-content/system-roms/rom/console-rom.bin`). Rebuild + recommit it
  whenever `console.asm` changes — a staleness gate
  (`libre99-asm/tests/committed_bin.rs`) fails the suite if the committed bytes
  lag the source.
- **Run it:** it is the emulator's **default console ROM** — plain
  `cargo run -p libre99-app` boots it (paired with the Libre99 GROM). To boot a
  user-supplied authentic TI image instead: `--system-rom
  third-party/roms/994aROM.Bin`. The flags also accept explicit paths to these
  artifacts, for pinning a specific build.

## Provenance and method

No TI bytes are copied. The authentic ROM was **disassembled only to recover
its interface contract** — addresses, table layouts, register conventions,
algorithms as behavioral specs ([`RECON.md`](./RECON.md), plan policy P5) —
and every element was reimplemented from that dossier, then **differentially
verified**: the same GPL programs run under the authentic ROM and ours from
identical machine state, and the full observable state (scratchpad, VDP
registers, VRAM, CRU) must match. Interface *data* (vectors, dispatch-table
geometry, harvested NASTY constants) is byte-identical by policy; code
expression is ours. The project's original work, this ROM included, is
licensed under the Modified MIT License with Commons Clause
([LICENSE.md](../../../LICENSE.md)).

The build was governed by nine working principles — P1 dossier-before-code,
P2 differential-first, P3 bug-for-bug behavior compatibility, P4 the authentic
performance envelope (per-byte GROM addressing), P5 clean-room IP discipline,
P6 gates-as-cargo-tests with committed artifacts, P7 one source of truth per
fact, P8 address-exact public entries (enforced by `AORG` + the frozen-layout
gate), and P9 functional completeness over observed usage as the acceptance
bar. Their full statements, the milestone ladder, the decision record, and the
bibliography are preserved in the executed plan, archived at
[`../history/ROM-REWRITE-PLAN.md`](../history/ROM-REWRITE-PLAN.md); the
increment-by-increment execution ledger is
[`../history/ROM-PROGRESS.md`](../history/ROM-PROGRESS.md).

## Status

| Milestone | Scope | State |
|---|---|---|
| M1-M3 | Kernel, GPL core, ISR, KSCAN, XML + device I/O | ✅ complete |
| M4 | Interpreter completeness (non-BASIC): FMT, every opcode, COINC/SWGR/RTGR, the IO engines, the cassette modem layer, the `>837D` character buffer | ✅ complete |
| M5 | The radix-100 floating-point + conversion package (FADD/FSUB/FMUL/FDIV/FCOMP + S-forms, ROUND/STST/OV, CSN/CSNGR/CFI) — bit-exact, 58 gates | ✅ complete |
| M6 | The TI BASIC ROM half | **deferred indefinitely by policy** for TI BASIC proper (PARSE/CONT/EXEC/RTNB, the `>1C9C` tables, the XML symbol entries — a written justification un-defers more; a tripwire test enforces the paperwork). **One justified subset is in: the XB substrate** (2026-07-07) — the five BASIC-era helpers Extended BASIC calls directly by address (`SYMSRC >15E0`, `RDCELL >187C/>1880`, `RDVAL8 >1890`, `WRWORD >18AA/>18AE`, `STKON/STKOFF >1E7A/>1E8C`, `VPOPAG >1FA8`), census-bounded and differentially gated; the justification and dossier are [`../XB-CENSUS.md`](../XB-CENSUS.md). With it, **Extended BASIC runs end-to-end on the clean-room pair** (gate: `libre99-gpl/tests/xb_smoke.rs`). |
| M7 | Hardening: the full gate set green (matrix, the 256-opcode sweep, both fuzzes + deep soaks, fetch-stream lockstep, conformance checkpoints, robustness storms, performance parity, the census tripwires). One documented residual: pinning the authentic's `>2C/>2E` garbage-corner MOVE parse (no real emitter; excluded with its own tripwire) | ✅ complete |
| M8 | Packaging: this README, the committed artifact, docs sync | ✅ complete |

**The embed decision (plan §9) was made 2026-07-06: this ROM, with the
Libre99 GROM, boots by default.** User-supplied authentic TI images are
selected via `--system-rom` / `--system-grom` (nothing TI ships in this
repository) — still required for **TI BASIC proper** while M6 stays deferred;
Extended BASIC no longer needs them thanks to the XB substrate
([`../LIMITATIONS.md`](../LIMITATIONS.md) L9).

What runs under this ROM today: the full boot to a pixel-identical title and
menu on our GROM, cartridge listing + launching (GPL and ML), disk device
I/O end-to-end (Tunnels of Doom loads its quest from disk), the whole
non-BASIC GPL ISA including the exhaustive 256-opcode differential sweep,
the complete floating-point package (every XML table-0/1 routine bit-exact,
garbage inputs included) — and, since 2026-07-07, **Extended BASIC**: the
XB cartridge boots, `PRINT`s, assigns floats, stores, `RUN`s and `LIST`s
programs on the all-clean-room pair (the XB substrate;
[`../XB-CENSUS.md`](../XB-CENSUS.md)).

## The test estate

- `crates/libre99-asm/tests/gpl_core.rs` — the per-element differential
  microsuite (the P9 verification engine).
- `crates/libre99-asm/tests/gpl_fmt.rs` — the FMT sub-language gates.
- `crates/libre99-asm/tests/gpl_fp.rs` — the floating-point gates + FP fuzz.
- `crates/libre99-asm/tests/gpl_opcode_sweep.rs` — all 256 opcodes, bounded and
  differential; the M6-deferral tripwire lives here.
- `crates/libre99-asm/tests/gpl_fuzz.rs` — the random-GPL differential fuzz
  (fast tier pre-commit; a 20k-program deep soak `--ignored`).
- `crates/libre99-asm/tests/committed_bin.rs` — the artifact staleness gate.
- `crates/libre99-asm/tests/entry_census.rs` — the P8 public-entry tripwire.
- `crates/libre99-gpl/tests/*` — the firmware-matrix boot flows: title, menu,
  KSCAN, ISR, sweeps, device I/O, robustness, performance parity.

## Performance parity

The perf gate (`rom_perf.rs`) asserts our-ROM boots within ×1.25 of the
authentic on frames-to-settled-title and host wall-clock per frame.
**Measured (2026-07-05):** frames-to-settled-title 22 (authentic) vs 32
(ours) — the documented fixed ~10-frame interpreter offset, frame-level
parity being the design bar; host wall-clock per emulated frame ~0.22 ms
under both, ratio 0.99-1.04 against the 1.25 budget. The per-byte GROM
addressing P4 reproduces dominates the cost, as predicted.

## Maintaining `console.asm`

Working notes distilled from the build (the full stories are in the archived
[execution ledger](../history/ROM-PROGRESS.md)):

- **⚠ GPLWS byte aliases.** The `>83E0–>83FF` cells ARE the GPL workspace
  registers (`>83E7` = R3-low, `>83EF` = R7-low, `>83F9` = R12-low, …). Two
  real bugs came from forgetting this: the SPEC dispatch read R9's stale low
  byte, and KSCAN's `CLR R3` wiped `>83E7` — the raw scan code — before use.
  Never treat a `>83Ex`/`>83Fx` cell as independent storage; comment every
  such reference with its register identity. (The authentic ROM *exploits*
  the aliasing deliberately, so reads of authentic behavior must watch for it
  too.)
- **⚠ Sequence coupling.** Single-element differential microtests are blind to
  state leaked between handlers through shared registers: the MOVE→XML
  SPEC-dispatch bug survived 89 of them and was caught only by the end-to-end
  boot probe. Every change must run the boot/menu gates, and the bounded fuzz
  covers the sequence space systematically.
- **Loud stubs.** Every unimplemented dispatch entry lands on `STUB`
  (breadcrumb `>837D` := the opcode byte, then a visible spin) — diagnosable,
  never a silent runaway. The boot/menu gates assert the breadcrumb stays
  clear.
- **`>83C6` (the KSCAN translation state) is never seeded by boot GPL** —
  authentic behavior (zeroed emulator RAM ⇒ state 0; random on real hardware;
  F5 preserves a game's leftover). Both ROMs read the same cell, so
  differential integrity holds regardless.

### Layout ledger — blocks placed outside their authentic homes

P8 pins *entries and tables*; handler *bodies* are free-placement — but several
of ours sit in regions a future milestone (in practice **M6**) would claim at
pinned addresses. **The `AORG` per-byte overlap guard makes every debt
self-enforcing** (the displacing milestone's first build fails loudly), so
nothing here can rot silently; this table exists so each displacement is
*planned*, not discovered. (Addresses measured from the symbol map,
2026-07-05, M5 slice 1 — "the great relocation.")

| Block (label) | Now at | Sits in the authentic… | Displaced by | Disposition |
|---|---|---|---|---|
| Zone A (control-flow/specials bodies) | `>0120–026F` | opcode bodies (unpinned) | — | permanent |
| Zone C (format-1 bodies) | `>0300–04B1` | KSCAN interior (unpinned; our entry trampolines out) | — | permanent |
| Zone D (format-5 + INC bodies) | `>0500–05A1` | FMT body region (entry `>04DE` pinned, body free) | — | permanent (FMT trampolines) |
| `FLDCUR`/`FSTCUR` (FMT cursor helpers) | `>05BA–05E5` | GPL-CORE interior (unpinned in ours) | — | moved here from `>15DE` by the **XB substrate** (2026-07-07) |
| COINC handler | `>05E6–066D` | GPL-CORE interior (unpinned in ours) | — | moved here from `>1620` by the XB substrate |
| Zone E (stream helpers) | `>0680–~06EB` | MOVE/COINC/GAS interior (unpinned) | — | permanent |
| `CALLI` + the loud stubs (`STUB`/`ISTUB`) | `>06EC–070F` | MOVE/COINC interior (unpinned) | — | moved here from `>1E68`/`>1E7E` by the XB substrate |
| `MDREG` (VDP-register storer) | `>0710–0733` | ditto | — | moved here from `>1F92` by the XB substrate |
| `MOVEH` entry (absolute-branch form) | `>0734–0747` | ditto | — | moved here from `>1E90` by the XB substrate; the body keeps `>1E9E+` |
| Zone G (value loads) | `>08A4–08FF` | interpreter-service interior (unpinned) | — | permanent |
| `CASW` (cassette write engine) | `>0B4A–0BBF` | DSR-LINK interior (unpinned in ours) | — | moved here from `>1872` by the XB substrate |
| `XMLH` | `>0BC0–0BDF` | ditto | — | moved twice: `>1200`→`>1FB8` (M5), `>1FB8`→here (XB substrate) |
| `IOH`/`IOCRIN`/`IOCROUT`/`IOSND` | `>0E94–~0EF4` | the FMUL-interior gap (`>0E92-0F53` unpinned) | — | permanent (M5 fit) |
| `SGROMB` + linkage constants | `>1346–~141A` | the old cassette span (free — Zone K took the engines to `>1820`) | — | permanent-ish |
| Zone I (`FMTBODY`) | `>1440–~15DD` | the cassette span + BASIC-support head | — | ends at `FEMITR`; its old tail is the substrate's `>15E0` |
| **The XB substrate** (`SYMSRC`/`RDCELL`/`RDVAL8`/`WRWORD`/`STKON`/`STKOFF`/`VPOPAG`) | **`>15E0–163D`, `>187C–18C5`, `>1E7A–1E9B`, `>1FA8–1FC7`** | **its authentic homes — these entries are address-pinned** (XB calls them by address; `XB-CENSUS.md`) | — | the 2026-07-07 M6 subset; gates in `libre99-asm/tests/xb_substrate.rs` |
| Zone J (SWGR/RTGR/CEQ + the `>837D` helpers) | `>16A8–~17D2` | BASIC-support interior | **M6** only | permanent under the deferral |
| `SNAME` | `>17D4–~17FA` | BASIC-support interior | **M6** only | permanent under the deferral |
| Zone K (the cassette engines + timer ISR, minus `CASW`) | `>1820–~1AC0` | BASIC-half interior | **M6** only | permanent under the deferral |
| `KSCANB` | `>1B00–~1CEC` | BASIC-half interior + the `>1C9C` tables span | **M6** only | permanent under the deferral |
| Zone H (stores/shifts) | `>1CF0–~1E67` | the M6 tables/support region | **M6** only | permanent under the deferral |
| `MOVEH` body (`MVCNTM`+) + loaders/storers | `>1E9E–~1F91` | ditto | **M6** only | permanent under the deferral |
| XTAB `>1C-1F` tail (harvested constants) | `>12B8–12BF` | CFI's entry (`>12B8`, pinned at M5) | resolved at M5 slice 2 | CFI's code displaced the constants; the accident reproduces structurally (RECON §8/§25) |

Free slack (re-measured after the XB-substrate dance, 2026-07-07): the
`>05BA-067F` and `>06EC-0779` and `>0B4A-0BEF` runs are now occupied (above);
still free: `>141C-143F`, `>1609-161F` (plus the vacated `>163E-16A7`),
`>17FC-181F`, `>18C6-18E7`, `>1AC0-1AFF`, `>1F92-1FA7`, `>1FC8-1FE7`, and
M5's interior gaps. **The M6 deferral is what makes this layout close** —
every remaining "M6 only" row is a displacement a future TI-BASIC milestone's
justification note must budget (the XB substrate's own displacement dance,
recorded here and in `../XB-CENSUS.md` §4, is the worked example).

## The document set

| Document | What it is |
|---|---|
| [`RECON.md`](./RECON.md) | **The interface dossier** — every empirically-pinned fact about the authentic console ROM (dispatch tables, ISR duties, FMT grammar, XML tables, FP format, the NASTY constants, per-milestone execution-pinned semantics §16–27). The permanent reference. |
| [`../XB-CENSUS.md`](../XB-CENSUS.md) | The Extended BASIC console-call census (F0) + the XB-substrate interface dossier and M6-subset justification. |
| [`KSCAN-SPEC.md`](./KSCAN-SPEC.md) | The deep single-subsystem spec for the keyboard scanner + CLEAR/BREAK test (scan-code formula, CRU-level scan loop, debounce, alpha-lock, scratchpad cells). |
| [`SURFACE-MAP.md`](./SURFACE-MAP.md) | The byte-range classification of the authentic 8 KiB and the **frozen-address table** (the P8 public contract the layout gate enforces). |
| [`../history/ROM-REWRITE-PLAN.md`](../history/ROM-REWRITE-PLAN.md) | The executed plan (principles, milestones, decisions, bibliography) — archived. |
| [`../history/ROM-PROGRESS.md`](../history/ROM-PROGRESS.md) | The increment-by-increment execution ledger with per-bug archaeology — archived. |
| [`../history/ROM-ENTRY-CENSUS.md`](../history/ROM-ENTRY-CENSUS.md) | The R-3 dynamic entry-census snapshot that validated the P8 set — archived (the live guarantee is the `entry_census` gate). |
