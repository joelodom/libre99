# XB-CENSUS — what Extended BASIC actually asks of the console (F0)

**The question** (owner, 2026-07-07, the TI PYTHON spec's §6 feasibility study,
[docs/TI-PYTHON.md](../../docs/TI-PYTHON.md)): what, *exactly*, does an
Extended BASIC cartridge need from the console firmware — and how much of the
feared "whole interpreter's worth of console services"
([`LIMITATIONS.md`](./LIMITATIONS.md) L9) does the clean-room rewrite really
lack?

**The answer, measured:** for the Extended BASIC in the local media set
(`xb25.ctg`, "EXTENDED BASIC V2.5"), the whole gap was **five tiny console-ROM
helpers, ~200 bytes total**, called **directly by address** from the
cartridge's own TMS9900 ROM. Everything else XB uses — the GPL interpreter,
KSCAN, the DSR/GROM search, the complete radix-100 floating-point and
conversion package — the rewrite already shipped. No console-GROM code is
used at all beyond the standard header walk. The five helpers are now
implemented (original code at the authentic pinned addresses — the
**XB substrate** section of [`rom/console.asm`](./rom/console.asm)), and the
full scripted XB session runs under the all-clean-room firmware
(`tests gate: crates/libre99-gpl/tests/xb_smoke.rs`).

This document is the census record, the interface dossier for the five
helpers, the layout-ledger delta, and the **written justification the ROM
M6-deferral policy demands** for implementing this (and only this) slice of
the BASIC-era ROM surface.

---

## 1. Method — the census instrument

`crates/libre99-gpl/examples/xb_census.rs` launches an XB cartridge from the
selection menu and drives a scripted session headlessly:

```
PRINT "HELLO"          ← immediate string output
X=1.5 / PRINT X*2      ← float assignment + arithmetic (prints 3)
10 PRINT "HI" / 20 END ← stored program entry
RUN                    ← executes (prints HI)
LIST                   ← lists both lines
```

recording, **from the launch keypress onward**:

* every **console-GROM address fetched** (`>0000–5FFF`) — the existing GROM
  read-coverage bitmap;
* every **console-ROM PC executed** (`>0000–1FFF`) — a new CPU PC-coverage
  bitmap added for this census (`Cpu::record_pc_coverage`, mirrored on
  `Machine`; unit-gated in `libre99-core/tests/cpu.rs`).

Four firmware modes isolate the layers: `authentic` (TI ROM + TI GROM),
`ours` (clean-room pair — the default boot), `ours-grom`, `ours-rom`.

## 2. Results

### 2.1 Under the authentic pair — the demand census

The session runs fully. XB's console usage, measured:

| Layer | What XB touched | Clean-room status (before this work) |
|---|---|---|
| Console **GROM 1/2** | Header bytes only (`>2000`, `>2008-2009`, `>4000`) — the standard power-up header walk | ✅ our headers serve these |
| GROM **interconnect slots** | `>0010` (DSRLNK) and `>0032` | `>0010` ✅ implemented; `>0032` stubbed (`ILRTN`) — **and XB tolerates the stub** (proven under `ours-grom`: full session works) |
| Console ROM — already shipped | GPL interpreter, KSCAN + keytab reads, ISR, SROM/SGROM device search, the FP package (`>0E8C–0FB0`), the conversions (`>11B2–12BC`) | ✅ M1–M5/M7 |
| Console ROM — **the gap** | **`>15E0-163B`**, **`>187C-188F`**, **`>18AE-18C5`**, **`>1E7A-1E9B`**, **`>1FA8-1FC7`** — five helper routines, entered by direct `BL @>xxxx` from XB's cartridge ROM (bank-switched `>6000` code, PC-verified executing) | ✗ those addresses held relocatable interior code of ours |

Decisive negative results (what XB does **not** use):

* **No GROM-side BASIC library.** The GROM-2 "shared BASIC-era GPL library"
  and the GROM-1 TI BASIC interpreter are **never fetched** — L9's model of
  the dependency was wrong for this cartridge class. XB brings its own
  interpreter (its cartridge ROM executes throughout).
* **None of the XML-dispatched symbol entries.** `XML >13–18`/`>1B`
  (SMB/SYM/ASSGNV/VPUSH/VPOP/PGMCH trampolines) are never invoked — XB reaches
  the primitives it wants by direct address instead. The XTAB loud stubs
  never fire.
* **None of the TI BASIC interpreter core.** `PARSE`/`CONT`/`EXEC`/`RTNB`
  (`>18C8–1FFB` minus the helpers above), the statement entries, the
  `>1C9C-1DE2` jump tables: zero PCs. The M6 deferral for TI BASIC proper is
  untouched by XB's needs.

### 2.2 Under the clean-room pair — the divergence (the L9 symptom, pinned)

Same script, our ROM + our GROM, **before** the substrate: XB reaches
`* ready *`, every line echoes, program lines **store**, and `LIST` works —
but `PRINT` emits nothing and `RUN` executes nothing (silently; zero illegal
opcodes — XB's direct `BL`s were landing mid-routine in whatever *our* layout
kept at those addresses, and returning harmlessly). Value flow — not line
handling — was the dead half: the five helpers are exactly the symbol-lookup
and value-stack primitives every expression needs, while `LIST` needs none of
them.

## 3. The five interfaces (the dossier)

Recovered behaviorally from the authentic ROM (disassembled-as-spec, plan
policy P5 — never copied); implemented as original code in the **XB
substrate** section of `rom/console.asm`; differentially gated by
`crates/libre99-asm/tests/xb_substrate.rs`. All run on GPLWS (`>83E0`) with
the standard port images (R13 GROM, R15 VDP-address port); "alias" means the
GPLWS low-byte cell of a register.

| Entry | Name | Contract |
|---|---|---|
| `>15E0` | **SYMSRC** | Walk the VDP-resident symbol chain headed by the word at `>833E` (0 = empty). Entry layout `+0 base · +1 len · +2,3 link (0 ends) · +4,5 text VDP addr`. Sought name: length byte at `>8359`, text at `>834A+` (FAC). Caller: `BL @>15E0` + one `DATA` word. **Miss** → control passes to the DATA word's value; **found** → resume past it with `@>834A` := entry base address. R1–R10 clobbered per the authentic register choreography (callers observe end-states). |
| `>187C` (+ `>1880`) | **RDCELL** | `BL @>187C` + `DATA cell`: read one VDP byte at the VDP address held in the named scratchpad cell → R1 high byte; resume past the DATA. `>1880` is a live secondary entry with R3 = the VDP address (no DATA word). |
| `>1890` | **RDVAL8** | Copy the 8-byte value at the VDP address in `>834E/>834F` (FAC+4) into FAC `>834A-8351` (the post-search value fetch). Clobbers R2/R3. |
| `>18AA` / `>18AE` | **WRWORD** | Write R6's word to VDP RAM at the address in R1 (`>18AA` first does R1 -= 3 — the descriptor-patch bias). R1 returns ORed with `>4000` (the write bit) — an observable end-state. |
| `>1E7A` / `>1E8C` | **STKON / STKOFF** | The bracket around the package's word pushes: STKON parks the `>8342` byte in R8 and points R9 at the GPL sub-stack top (`>8300` + the `>8373` pointer byte); callers `INCT R9 / MOV x,*R9`. STKOFF restores `>8342` and stores R9's low byte back to `>8373`. `>1E8C` is placed by STKON's fixed 18-byte encoding. |
| `>1FA8` | **VPOPAG** | Pop the top 8-byte value off the VDP value stack into ARG `>835C-8363`: `>836E` points at the top element's base — read 8 bytes there, then `>836E -= 8` (the same protocol as the S-form FP ops, `rom/RECON.md` §9). |

Interior instruction boundaries that are themselves entered (`>1880`,
`>18AE`, `>1E8C`) fall out of fixed instruction encodings and are asserted by
the gates.

## 4. The layout dance (ledger delta)

Claiming the five authentic windows displaced interior code the
[`rom/README.md`](./rom/README.md) ledger had marked "*permanent under the
deferral / displaced by M6*" — the planned dance, executed:

| Displaced (was) | Now at | Notes |
|---|---|---|
| `FLDCUR`/`FSTCUR` (`>15DE`) | `>05BA` | FMT cursor helpers |
| COINC handler (`>1620`) | `>05E6` | SWGR/RTGR keep `>16A8+` (re-pinned) |
| `CASW` write engine (`>1872`) | `>0B4A` | `CASSU`/`CASBIT` keep their homes (`>18E8` re-pinned) |
| `CALLI` + loud stubs (`>1E68`/`>1E7E`) | `>06EC`/`>0702` | |
| `MOVEH` entry (`>1E90`) | `>0734` | entry rewritten with absolute branches; body keeps `>1E9E+` |
| `MDREG` storer (`>1F92`) | `>0710` | |
| `XMLH` (`>1FB8`) | `>0BC0` | second relocation (first was M5) |

All references are label-based (dispatch tables, `BL`/`B`); the AORG overlap
guard, the P8 `entry_census` tripwire, the 256-opcode sweep, and the fuzzes
gate the dance.

## 5. Policy — what this un-defers, and what it does not

ROM **M6 stays deferred** for TI BASIC proper: `PARSE`/`CONT`/`EXEC`/`RTNB`
and the `XML >13-18`/`>1B` symbol entries remain loud stubs; running **TI
BASIC** (the console GROM 1/2 interpreter) on the rewrite is as far away as
before. What this work implements is the **XB-observed five-helper subset**,
justified by: (a) the owner's 2026-07-07 instruction to make the firmware an
Extended BASIC substrate and test it against the local XB cartridge; (b) this
census bounding the cost to ~200 bytes at five pinned addresses; (c) the
differential gates holding it to the authentic behavior. This section is the
deferral policy's required written justification.

## 6. Scope, honestly

* **Measured against `xb25.ctg`** (Extended BASIC V2.5, 52 KiB — the XB in the
  local `third-party/` set, the same set the old `ti-99-emulator` checkout
  carries; never committed to this repository). Other XB-family or
  BASIC-coupled cartridges may lean on more of the BASIC half — candidates:
  `PGMCH` (`>1868`, whose core `>1F7E` we do not ship), the `SMB`/`SYM`
  symbol XMLs, `VPUSH` (`>1E9C/>1EAA`) with its string-GC tail. The census
  instrument makes each new demand a one-command measurement; implement on
  demand, this document's pattern.
* The scripted session covers immediate statements, float arithmetic, program
  storage, `RUN`, `LIST`. A broader script (arrays, strings ops, `FOR`,
  `GOSUB`, `DEF`, file I/O to disk) may surface additional direct-entry
  helpers; run the census before assuming.
* The GROM slot `>0032` remains an `ILRTN` stub — XB calls it and tolerates
  the no-op (differentially verified end-to-end). Video Vegas (L8), the other
  known caller, **stopped wedging** once the substrate populated
  formerly-zero ROM addresses on its data-driven launch path — the 137-cart
  health panel now passes with an empty waiver list (see L8's update note);
  the slot's routine itself is still unimplemented.

## 7. Reproduce

```sh
cargo run -q -p libre99-gpl --example xb_census                # authentic pair
cargo run -q -p libre99-gpl --example xb_census -- ours       # clean-room pair
cargo run -q -p libre99-gpl --example xb_census -- ours-grom  # isolate the GROM
cargo run -q -p libre99-gpl --example xb_census -- authentic cartridges/sxba.ctg
```

Gates: `cargo test -p libre99-asm --test xb_substrate` (per-helper
differential microtests) and `cargo test -p libre99-gpl --test xb_smoke`
(the end-to-end session under the clean-room pair; skips green without
third-party media, like the rest of the estate).
