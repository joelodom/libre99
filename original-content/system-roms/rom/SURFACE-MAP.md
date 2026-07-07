# SURFACE-MAP — the authentic Console ROM, classified (D1)

Byte-by-byte classification of the authentic 8 KiB `roms/994aROM.Bin`
(`>0000–1FFF`), plus the **frozen-address table** (the `AORG` skeleton our
source is built on, P8). Sibling of the GROM track's
[`../grom/SURFACE-MAP.md`](../grom/SURFACE-MAP.md); the behavioural detail behind
each region is [`RECON.md`](./RECON.md).

This classifies authentic content **to understand it and to bound the work** —
it records *what lives where and what it is*, not TI's instruction stream (plan
P5). (The rewrite is complete; this map remains the byte-level reference.)
Boundaries are ✅ where verified by disassembly / word dump this session,
📖 where taken from literature (TI Intern / Nouspikel) and refined as milestones
land.

**Classification legend**
- `VECTORS` — reset + interrupt vectors (`>0000–000B`).
- `ENTRY-STUB` — fixed public branch stubs external software calls.
- `KERNEL` — reset/power-up + fixed data/vector slots.
- `GPL-CORE` — the GPL interpreter: fetch/dispatch, opcode bodies, MOVE, the
  operand engine, RTN/CALL/sub-stack helpers, FMT, ALL, IO/CRU.
- `KSCAN` — keyboard/joystick scan + the CLEAR test.
- `ISR` — the level-1 VBLANK interrupt service routine.
- `DISPATCH-TABLE` / `XML-TABLE` — the interpreter dispatch tables and the XML
  master/0/1 tables (data-as-spec: address lists).
- `FP` — the floating-point package + numeric conversions.
- `DSR-LINK` — SROM/SGROM device+GROM search (`XML >19/>1A`).
- `CASSETTE` — the cassette bit engines + cassette-timer ISR (hardware-gated).
- `BASIC` — the TI BASIC ROM half (co-designed with BASIC's GROMs).
- `VESTIGIAL` — content targeting hardware that never shipped / unreachable, but
  present and reproduced behavior-faithfully (P9, not skipped).
- `DATA` — trailing/uncategorised data words.

Every region is **in scope for a complete rewrite (P9)** — none is "skip."
Classification drives *which milestone owns it and how it's placed*, never
whether it is implemented.

---

## Region map (`>0000–1FFF`)

| Range | Class | Contents (spec in RECON.md §) | Milestone |
|---|---|---|---|
| `>0000–000B` | VECTORS ✅ | reset `83E0 0024`, L1 int `83C0 0900`, L2 `83C0 0A92` (§2) | M1 |
| `>000C–000D` | DATA ✅ | clock byte `>30` + header marker `>AA` (§2, §13) | M1 |
| `>000E–0023` | ENTRY-STUB ✅ | KSCAN `>000E`, interp entries `>0016/001C`, CLEAR `>0020`, `SBZ 0` prologues `>0014/001A`, harvested `>0012` (§2) | M1 |
| `>0024–0035` | KERNEL ✅ | reset/EXIT routine: GPLWS setup, GROM strobe+addr `>0020`, status clear, enter `>0070` (§1) | M1 |
| `>0036–003F` | VESTIGIAL ✅ | extended-GPL-card return stub (`SBZ 0/LWPI >280A/RTWP`) | M4 |
| `>0040–004B` | KERNEL/VESTIGIAL ✅ | XOP 0/1/2 vectors (`>0040`→ext-GPL card; `>0044/0048` user) (§2) | M1/M4 |
| `>004C–004D` | DATA ✅ | QUIT row mask `>1100` (§2, §13) | M2 |
| `>004E–006F` | GPL-CORE ✅ | `SWGR` handler + interpreter prologue → `>0070` | M4/M1 |
| `>0070–00CB` | GPL-CORE ✅ | **main loop** (`>0070`), fetch/dispatch, status-bit ops (`H/GT/CARRY/OVF`→`>00F4`) (§3) | M1 |
| `>00CC–0103` | GPL-CORE ✅ | compare handlers `CGE/CH/CHE/CGT/CLOG/CZ/CEQ` | M1 |
| `>0104–0135` | GPL-CORE ✅ | branches `B >0104 / BS >010E / BR >011A` | M1 |
| `>0136–0161` | GPL-CORE ✅ | unary `ABS/NEG/CLR/INV/FETCH` | M1 |
| `>0162–01CD` | GPL-CORE ✅ | `CASE/PUSH`, arith `ADD/SUB`, logic `AND/OR/XOR/ST/EX`, shifts | M1 |
| `>01CE–026F` | GPL-CORE ✅ | `MPY`/`DIV` (long mul/div) | M1 |
| `>0270–02AD` | GPL-CORE ✅ | specials 2nd-level dispatch (`>0270`), `RAND >027A`, `BACK >029E` | M1/M4 |
| `>02AE–04B1` | KSCAN ✅ | `SCAN` shim (`>02AE`) + **KSCAN `>02B2`** (§5) | M2 |
| `>04B2–04DD` | KSCAN ✅ | CLEAR (FCTN-4) test | M2 |
| `>04DE–05A1` | GPL-CORE (FMT) ✅ | **FMT sub-interpreter** (§7) | M4 |
| `>05A2–05C7` | GPL-CORE ✅ | `ALL` (fill screen) | M1 |
| `>05C8–061D` | GPL-CORE ✅ | `IO` dispatch (`>05C8`), sound-list setup (`>05D6`), CRU in/out, **`XML` dispatch `>0608`** (§3, §8) | M2/M3 |
| `>061E–06D1` | GPL-CORE ✅ | **`MOVE`** (all src/dst handlers) (§3) | M1 |
| `>06D2–08FF` | GPL-CORE ✅ | `COINC`, the GAS operand engine (`>077A`), `RTN/RTNC/CALL/RTGR`, sub-stack + GROM/VDP-reg helpers (`>0864/089A`) | M1 |
| `>0900–0ABF` | ISR ✅ | **level-1 VBLANK ISR** + screen-blank (`>0A92` = L2 target) (§6) | M2 |
| `>0AC0–0C0B` | DSR-LINK 📖✅home | **SROM `>0AC0`** (`XML >19`), **SGROM `>0B24`** (`XML >1A`) (§10) | M3 |
| `>0C0C–0C35` | VESTIGIAL ✅ | extended-GPL trampolines (`>0C0C/0C14`), XOP-0 target `>0C1C`→CRU `>1B00` | M4 |
| `>0C36–0CF9` | DISPATCH-TABLE ✅ | the six dispatch tables (first-nibble/special/two-op/MOVE/FMT/IO) (§3) | M1/M4 |
| `>0CFA–0D19` | XML-TABLE ✅ | XML **master table** (16 pointers) (§8) | M3 |
| `>0D1A–0D39` | XML-TABLE ✅ | **FLTAB** (table 0, FP dispatch) (§8) | M5 |
| `>0D3A–11A1` | FP 📖✅tables | floating-point package (add/sub/mul/div/compare, rounding, overflow) (§9) | M5 |
| `>11A2–129F` | FP 📖✅home | `CSNGR >11A2`, `CSN >11AE` (string→real) (§9) | M5 |
| `>12A0–12BF` | XML-TABLE ✅ | **XTAB** (table 1) — 12 live + `>1C–1F` vestigial (§8) | M3/M5 |
| `>12C0–1345` | FP 📖✅home | `CFI >12B8` (real→integer) (§9) | M5 |
| `>1346–15D5` | CASSETTE 📖 | bit engines (write/read/verify) + cassette-timer ISR `>1404` (§11) | M4 (hardware-gated) |
| `>15D6–18C7` | BASIC 📖 | BASIC-support XMLs (symbol table, PGMCH, trampolines) (§12) | M6 |
| `>18C8–1FFB` | BASIC 📖 | BASIC interpreter core: PARSE/CONT/EXEC/RTNB, statement entries, jump tables `>1C9C`, VPUSH/VPOP (§12) | M6 |
| `>1FFC–1FFF` | DATA ✅ | `2A61 A38A` (📖 checksum; ❓ any reader — D2). Ours ships zero. | M6 |

Byte accounting (approx, 📖 boundaries refined per milestone): interpreter +
KSCAN + ISR + dispatch tables + DSR-link ≈ `>0000–0C0B` ≈ 3.1 K; XML tables +
FP + conversions ≈ `>0C36–1345` ≈ 1.8 K; cassette ≈ 0.65 K; BASIC half ≈ 1.8 K.
Comfortable in 8 KiB (the authentic image fills it); cassette-transport being
hardware-gated is our only slack, and P8 pins entries so each routine has an
authentic-sized budget with the trampoline escape hatch.

---

## Frozen-address table (the `AORG` skeleton, P8)

**Hard public contract** — externally entered; these addresses are frozen for
interoperability and are the layout-assertion gate's core input:

| Symbol | Addr | Entered by |
|---|---|---|
| reset vector | `>0000` | CPU reset, `BLWP @>0000` |
| L1 int vector | `>0004` | 9901 interrupt |
| L2 int vector | `>0008` | (present; unreachable) |
| header marker | `>000D` | ISR / SROM read `@>000D` |
| `KSCAN` | `>000E` | `BL @>000E` (E/A, cartridges) |
| interp entry (R9) | `>0016` | `B @>0016` |
| interp entry (fetch) | `>001C` | `B @>001C` |
| `CLEAR` test | `>0020` | `BL @>0020` (break-key poll) |
| reset/`EXIT` | `>0024` | reset PC, GPL `EXIT`, QUIT |
| interp soft entry | `>006A` | `B @>006A` (GPLLNK-from-asm) |
| interp main loop | `>0070` | interpreter re-entry |
| QUIT mask | `>004C` | ISR `CZC @>004C` |
| `XML >F0` vector | `>8300` | ML dispatch (RAM, not ROM) |
| `SYMSRC` | `>15E0` | `BL @>15E0` (Extended BASIC cart ROM — the F0 census, `../XB-CENSUS.md`) |
| `RDCELL` (+`>1880`) | `>187C` | `BL @>187C` / `BL @>1880` (XB) |
| `RDVAL8` | `>1890` | `BL @>1890` (XB) |
| `WRWORD` (±3 entry) | `>18AA`/`>18AE` | `BL` (XB) |
| `STKON`/`STKOFF` | `>1E7A`/`>1E8C` | `BL` (XB) |
| `VPOPAG` | `>1FA8` | `BL @>1FA8` (XB) |

**P8-frozen interior entries** — not externally entered, but pinned so the
dispatch tables carry identical values and differential traces stay aligned (XB
also enters some interiors directly, 📖 TI Intern): **every dispatch-table target
in RECON §3** (opcode handlers `>0104…>06D2`, MOVE handlers `>0660…0698`, FMT
sub-ops `>0502…056C`, IO handlers `>05D6…1426`), **every XML-table target**
(FLTAB `>0D3A…0FF8`, XTAB `>0AC0…1868`), KSCAN `>02B2`, the CLEAR test `>04B2`,
the ISR `>0900`, SROM `>0AC0` / SGROM `>0B24`, the FMT interpreter `>04DE`, and
the BASIC entries `>18C8/1920/1968/19F0`. The complete list is derived from the
RECON §3/§8 tables; the M-milestones `AORG` each routine at its authentic
address and the layout-assertion gate (`Assembly::check_layout`, built in R-1)
fails on any drift.

**NASTY harvested constants** (RECON §13) — placed as explicit `DATA`/`EQU` at
their exact addresses: `>0012`, `>0032`, `>0036`, `>0072`, `>0074`, `>004C`,
`>000D`, `>011B` (+ the full set from the M-milestone disassembly).

**Validated empirically.** The R-3 dynamic entry census (2026-07-04, archived at
[`../history/ROM-ENTRY-CENSUS.md`](../history/ROM-ENTRY-CENSUS.md)) observed
every ML→ROM entry landing on a documented public address — `>000E` (KSCAN)
dominating, no cartridge entering a ROM interior — validating this table's
public-entry set; since M7 the `entry_census` test keeps it pinned permanently.

---

## What changes vs. authentic (the intended diff)

Per RECON's clean-room policy and P8 (as **scoped 2026-07-04** — plan P8):
- **Byte-identical by construction** (uncopyrightable interface data): the
  vectors, the fixed data words (`>000C`, `>004C`, `>0010–0015`), the **XML
  master table**, and the NASTY constants. Enumerated here; each gets an
  identity gate.
- **Structure-identical, our values**: the six dispatch tables and XML tables
  0/1 keep the authentic location/size/indexing, but entries carry **our**
  handler addresses wherever the handler body is not itself pinned — TI packed
  bodies with shared-tail tricks into 4-byte slots, so pinning bodies would
  coerce TI's structure (vs P5) or burn trampolines. No software enters
  handler interiors (the R-3 entry census: ML enters only `>000E`); the
  `entry_census` tripwire re-pins any address ever observed entered.
- **Original code at pinned entries**: public entries + literature-documented
  routine entries (the frozen table above) sit at authentic addresses; interior
  instruction sequences are ours; observable behaviour matches.
- **All-zero tail** `>1FF0–1FFF` (no watermark, §10.5) — unless D2 finds a
  reader of the `>1FFC` words, in which case they join the byte-identical set.
