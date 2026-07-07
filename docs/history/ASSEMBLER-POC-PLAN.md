# TI-99 Assembler POC — Implementation Plan (tracer-bullet)

Scoped, de-risked plan for the first deliverable: a real (minimal) TMS9900
assembler that turns the spec's §10.2 **HELLO** source into a bootable `.ctg`
that displays HELLO on screen. This is the executable subset of
[`ASSEMBLER.md`](../../assembler/ASSEMBLER.md) — which is now the current
guide; this plan is the historical bootstrap record.

**Decisions locked:** POC demo = **HELLO text on screen** (§10.2). Encoder scope
= **tracer-bullet** (only what this demo needs; clean path to the full
Appendix-A table later).

---

## 1. What Phase 1 already proved (don't re-litigate)

Verified empirically by hand-assembling the demos, packing `.ctg`s, and booting
them in `libre99_core::Machine` (throwaway tests, since removed; golden carts saved
at `scratchpad/poc_color.ctg` and `scratchpad/poc_hello.ctg`):

- **The `>6000` `>AA` ROM-header menu path works.** The console's GPL menu scans
  CPU `>6000`, lists the program (`2 FOR HELLO`), and transfers control to its ML
  entry. (Previously unproven — every bundled boot test uses *GROM* carts.)
- **The HELLO program is correct.** After launch: `R1=>C0` (display on, Graphics
  I), `R2=00/R3=0C/R4=01/R7=17`, name cells `@>018D = [60,61,62,62,63]`, H glyph
  `@>0B00 = [88,88,88,F8,88,88,88,00]`. PC spins at `SPIN`.
- **The `.ctg` writer logic round-trips** through `cartridge::parse` (single
  literal-run RLE per 4 KiB page).
- **Every needed encoding matches `cpu.rs`** (table in §5 below).

### ⚠ Correction to the spec's boot-test assertion

The spec (§10.2 / A5) says to "assert `'HELLO'` appears in the name table via the
`screen_text` helper." **That is wrong.** The demo uses custom glyph codes
`>60..>63`, which `screen_text` (cell-value-as-ASCII) renders as `` `abbc ``, not
`HELLO`. The visual pixels *do* spell HELLO. The boot test must assert the **raw
name-table codes** `[0x60,0x61,0x62,0x62,0x63]` at `>018D` and/or the loaded H
glyph at `>0B00` — **not** `screen_text == "HELLO"`.

---

## 2. Crate layout (tracer-bullet)

New workspace member `crates/libre99-asm` (add to root `Cargo.toml` `members`). Pure
`std`; one dependency: `libre99-core` (for `cartridge::write_v1` + the boot test).

```
crates/libre99-asm/
  Cargo.toml         # libre99-core = { path = "../libre99-core" }
  src/
    lib.rs           # assemble(source, opts) -> Result<Artifacts, Vec<Diag>>
    lex.rs           # line -> (label?, mnemonic?, operands[], comment)
    expr.rs          # constants (> hex, decimal, 'c' char), symbols, $, + - * /
    symbol.rs        # name -> value (two-pass; forward refs in insn/DATA operands)
    isa.rs           # InsnDef table + format encoders (single source of truth)
    operand.rs       # operand text -> (T, reg, Option<ext expr>)
    assemble.rs      # two-pass driver, location counter, auto-header
    cartridge.rs     # header synthesis + 8 KiB page split -> libre99_core::write_v1
    cli.rs, main.rs  > **ARCHIVED (2026-07-06).** The tracer-bullet plan that bootstrapped the
> assembler. `libre99asm` has long since outgrown it — the shipped tool is a
> complete two-pass assembler (all 69 base opcodes) with a disassembler and
> the console-ROM build mode. Kept as the record of how the toolchain was
> bootstrapped; the current guide is
> [../../assembler/ASSEMBLER.md](../../assembler/ASSEMBLER.md).

# libre99asm <in.asm> -o out.ctg --name NAME
  tests/
    encode.rs        # golden per-instruction encodings (§5 table + ASSEMBLER.md App. G)
    cartridge.rs     # assemble §10.2 -> .ctg -> mount -> boot -> assert (§6)
```

**Skip for the POC** (defer to later milestones): `object.rs` (tagged `.obj`),
`listing.rs` (`.lst`/`.map.json`), banking, `DEF/REF/DXOP/COPY`, `--ea-strict`,
JSON errors. Keep diagnostics simple (`file:line:col: message`) but real.

---

## 3. Language subset the §10.2 demo needs

Implement exactly this (a clean superset is fine where it's free):

- **Instructions:** `LIMI LWPI LI MOVB SWPB DEC JNE JMP BL ORI ANDI CLR` + pseudo
  `RT`. Formats touched: I (`MOVB`), II (`JNE`/`JMP`), VI (`DEC`/`SWPB`/`CLR`/`BL`,
  and `B` underlying `RT`), VIII (`LI`/`ORI`/`ANDI`/`LWPI`/`LIMI`).
- **Addressing modes:** `Rn` (T=0), `@EXPR` (T=2, +ext word), `*Rn+` (T=3). (Add
  `*Rn` T=1 and `@EXPR(Rn)` indexed for free — same code.)
- **Directives:** `IDT 'name'`, `EQU`, `BYTE e,…`, `END [sym]`, `EVEN`, and the
  **assisted/auto header** (§4). `AORG >6000` is auto-inserted. (`DATA`/`TEXT`
  unused by §10.2 but trivial and worth adding; `DATA` must word-align — pad `>00`.)
- **Expressions:** `>`hex, decimal, symbols, predefined `R0`–`R15`. Operators
  `+ - * /` left-to-right, no precedence (§10.2 needs none, but include them).
  `$` = current LC.

---

## 4. Auto-header synthesis (the assisted default)

If the source has no `>AA` at `>6000`: insert `AORG >6000`, then emit the 16-byte
standard header + one program-list entry, then `EVEN`, then the user code. Entry
address = `END` operand (`END START`). Menu name = `--name`, else `IDT`, else file
stem. **Golden output** for name `HELLO`, `END START`:

```
>6000: AA 01 01 00 00 00 60 10 00 00 00 00 00 00 00 00   ; header (prog list ->6010)
>6010: 00 00 60 1A 05 48 45 4C 4C 4F                     ; next=0, entry=START, len=5,"HELLO"
>601A: …code…                                            ; START
```

Entry-record size = `5 + name_len`; code begins at `>6010 + 5 + name_len`,
`EVEN`-padded. (name="HELLO" → 10 → START=`>601A`, already even.) So START is not
a fixed address — the assembler computes it; `END START` resolves the field.

---

## 5. Encoding rules + golden table (verified against `cpu.rs`)

- **Format I:** `base | (Td<<10) | (D<<6) | (Ts<<4) | S`; then **source** ext word,
  then **dest** ext word (source precedes dest — `cpu.rs::resolve` order).
- **Format VI:** `base | (Ts<<4) | S`; ext word if symbolic.
- **Format VIII:** reg+imm (`LI`/`AI`/`ANDI`/`ORI`/`CI`): `base | reg`, then imm
  word. imm-only (`LWPI`/`LIMI`): `base`, then imm word.
- **Format II (jumps):** `base | (disp & 0xFF)`, `disp = (target − ($+2)) / 2`,
  range `[−128,127]`, even byte distance. (`$+2` = address of the *next* word.)
- **Modes:** `Rn`→T=0,S=n; `*Rn`→1; `@A`→2 with ext=A and reg field 0; `@A(Rn≠0)`→2
  with reg=Rn, ext=A; `*Rn+`→3. Reject `@A(R0)`.
- **Pseudos:** `RT` = `B *R11` = `04 5B`; `NOP` = `JMP $+2` = `10 00`.
- **Word-align** machine instructions and `DATA` (pad `>00` if LC odd); `BYTE`/
  `TEXT` do not pad.

Golden rows (one per distinct case in §10.2 — the encoder's test oracle, all
proven by boot):

| Source | Bytes |
|--------|-------|
| `LIMI 0` | `03 00 00 00` |
| `LWPI >8300` | `02 E0 83 00` |
| `LI R1,>0800` | `02 01 08 00` |
| `LI R2,16` | `02 02 00 10` |
| `ORI R1,>4000` | `02 61 40 00` |
| `ANDI R1,>3FFF` | `02 41 3F FF` |
| `MOVB *R1+,@>8C02` | `D8 31 8C 02` |
| `MOVB R0,@>8C00` | `D8 00 8C 00` |
| `MOVB R1,@>8C02` | `D8 01 8C 02` |
| `MOVB *R2+,@>8C00` | `D8 32 8C 00` |
| `DEC R2` | `06 02` |
| `CLR R0` | `04 C0` |
| `SWPB R1` | `06 C1` |
| `BL @SETWR` (SETWR=`>60A2`) | `06 A0 60 A2` |
| `JNE` (loop back 8 bytes) | `16 FC` |
| `JMP $` | `10 FF` |
| `RT` | `04 5B` |

---

## 6. `cartridge::write_v1` (add to `libre99-core`)

Inverse of `cartridge::parse`. Proven shape:

```rust
// banner "TI-99/4A Module - <title>\n\x1A" zero-padded to 0x50
// 0x50: 0x10 (V1)   0x51..53: cru_base big-endian
// per 8 KiB bank: region(index 6, low 4K) then region(index 7, high 4K)
//   region = index(1) #banks(1) [ banktype=2(1) RLE(4096) ]*
//   RLE single literal run = tag 0x1000 little-endian (00 10) + 4096 bytes
pub fn write_v1(title: &str, cru_base: u16, rom_banks: &[[u8; 0x2000]],
                grom_pages: &[(u16, Vec<u8>)]) -> Vec<u8>
```

Add a `parse(write_v1(x)) == x` round-trip test next to the existing parser tests.
(`grom_pages` empty for the POC.)

---

## 7. Frontend: mount a built cart (`--cartridge-file`)

Add to `libre99-app` (`cli.rs` + `app.rs`) a `--cartridge-file <path>` option:
`std::fs::read` → `Cartridge::parse` → mount (mirror the embedded path in
`app.rs::rebuild_machine`). Then the build-run loop is:

```
libre99asm hello.asm -o hello.ctg
cargo run -p libre99-app -- --cartridge-file hello.ctg
```

(Until then: drop the `.ctg` into `cartridges/`, rebuild, `--cartridge hello`.)

---

## 8. Tests & exit criteria

- `tests/encode.rs`: assert the §5 golden rows (+ relevant ASSEMBLER.md App. G rows).
- `tests/cartridge.rs`: assemble §10.2 → `write_v1` → `Machine::mount_cartridge` →
  `reset` → boot ~180 frames → press `Space` (advance) → press `Num2` (select) →
  run frames → assert `vdp.register(1)&0x40 != 0`, `vram(0x018D..0x0192) ==
  [60,61,62,62,63]`, `vram(0x0B00..0x0B08) == [88,88,88,F8,88,88,88,00]`. **Not**
  `screen_text == "HELLO"` (see §1).
- `cargo test -p libre99-core` and `-p libre99-asm` green; `cargo clippy` clean.
- Update `docs/STATUS.md`, `docs/ROADMAP.md`, `README.md`; link `ASSEMBLER.md`.

**Done when:** `libre99asm hello.asm -o hello.ctg` then `--cartridge-file hello.ctg`
shows HELLO on screen, and the boot test is green.

---

## 9. The demo source to ship (`assembler/examples/hello.asm`)

The assisted-header default (no explicit `AORG`/header). Verbatim from §10.2:

```asm
        IDT  'HELLO'
VDPWD   EQU  >8C00
VDPWA   EQU  >8C02
START   LIMI 0
        LWPI >8300
        LI   R1,REGTAB
        LI   R2,16
RL      MOVB *R1+,@VDPWA
        DEC  R2
        JNE  RL
        LI   R1,>0800
        BL   @SETWR
        LI   R2,2048
        CLR  R0
PCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  PCLR
        LI   R1,>0300
        BL   @SETWR
        LI   R2,32
        LI   R0,>1F00
CCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  CCLR
        LI   R1,>0000
        BL   @SETWR
        LI   R2,768
        LI   R0,>2000
NCLR    MOVB R0,@VDPWD
        DEC  R2
        JNE  NCLR
        LI   R1,>0B00
        BL   @SETWR
        LI   R2,FONT
        LI   R3,32
        BL   @VMBW
        LI   R1,>018D
        BL   @SETWR
        LI   R2,MSG
        LI   R3,5
        BL   @VMBW
SPIN    JMP  SPIN
SETWR   SWPB R1
        MOVB R1,@VDPWA
        SWPB R1
        ORI  R1,>4000
        MOVB R1,@VDPWA
        ANDI R1,>3FFF
        RT
VMBW    MOVB *R2+,@VDPWD
        DEC  R3
        JNE  VMBW
        RT
REGTAB  BYTE >00,>80,>C0,>81,>00,>82,>0C,>83
        BYTE >01,>84,>00,>85,>00,>86,>17,>87
FONT    BYTE >88,>88,>88,>F8,>88,>88,>88,>00
        BYTE >F8,>80,>80,>F0,>80,>80,>F8,>00
        BYTE >80,>80,>80,>80,>80,>80,>F8,>00
        BYTE >70,>88,>88,>88,>88,>88,>70,>00
MSG     BYTE >60,>61,>62,>62,>63
        END  START
```

Assembling this must reproduce the golden image proven in Phase 1 (labels:
`START=601A RL=602A PCLR=6040 CCLR=6058 NCLR=6070 SPIN=60A0 SETWR=60A2 VMBW=60B8
REGTAB=60C2 FONT=60D2 MSG=60F2`).
