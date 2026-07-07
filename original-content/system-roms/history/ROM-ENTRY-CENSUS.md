> **ARCHIVED (2026-07-06).** Historical record — its P8-validation finding is summarized in ../rom/SURFACE-MAP.md, and the entry_census gate keeps the guarantee live.

# ENTRY-CENSUS — D2 dynamic recon of the Console ROM (chunk R-3)

Empirical measurement of how running software actually uses the console ROM,
under the **authentic** ROM + GROM in `libre99_core::Machine`. Its job (post-P9) is
to **order** the implementation work and to **validate the frozen-address set**
(P8) — **not** to gate what gets built. Every functional element is implemented
and gated regardless of what any census counts (plan §0 / P9); this report only
confirms priorities and looks for surprises.

Reproduce: `cargo run -p libre99-asm --example rom_entry_census` (from the repo
root). The P8 invariant is also a re-runnable gate:
`cargo test -p libre99-asm --test entry_census -- --ignored`.

---

## 1. ROM entry census — the empirical public-entry set (P8 validation)

**What it measures.** Every CPU-PC transition from *outside* the ROM (cart ROM
`>6000-7FFF`, expansion RAM, the DSR window `>4000-5FFF`) *into* the ROM
(`>0000-1FFF`) — i.e. every point machine-language software branches into the
ROM. Pure GPL never appears: the interpreter always runs inside the ROM, so a
GPL cart contributes nothing here except (if it leaves interrupts enabled) the
ISR. The signal is therefore **ML carts calling the ROM's public entries**, plus
interrupts and DSR returns.

**Result (representative baseline, 2026-07-04).**

| Cart | Class | Distinct ROM entries | Addresses (count) |
|---|---|---|---|
| `centipe` (Centipede) | ROM-only ML | **1** | `>000E` KSCAN (×4614), from cart-ROM |
| `TI-Invaders` | ML | **1** | `>000E` KSCAN (×9379), from cart-ROM |
| `tunnelsofdoom` + disk | GPL + device | 0 | (see note b) |

**Finding: none.** Every observed ML→ROM entry is a **documented public
address** (the `SURFACE-MAP.md` frozen-address table). No cart branched into a
ROM *interior* address. **P8's public-entry set is validated on this sample** —
ML carts reach the ROM only through `>000E` (KSCAN), exactly as the contract
predicts.

Notes:
- (a) The ISR entry `>0900` did **not** appear — Centipede/TI Invaders run with
  interrupts masked (`LIMI 0`) and do their own timing, so the console ISR never
  fires during their gameplay. `>0900` is reached via the hardware `>0004`
  vector when an interrupt *is* enabled (a fact, not needing census proof); GPL
  carts use it internally (PC stays in-ROM, so it is invisible to this census by
  construction).
- (b) Tunnels of Doom is GPL, so its own execution is interpreter-internal. Its
  **device path** (the `>4000` disk DSR that `XML >1A` `BLWP`s into, returning to
  the ROM) would surface `>4000→ROM` entries, but reaching it needs the scripted
  "LOAD DATA FROM DSK1 → type QUEST" sequence from `libre99-gpl/tests/device_io.rs`,
  not the generic key-mash this baseline uses. **Deferred** (see §3).

**Implication for the rewrite.** The ROM's ML-facing contract that real bundled
software exercises is dominated by **`>000E` (KSCAN)** — so the KSCAN public
entry (M2) and the reset/vector/`XML >F0` set (M1/M3) are the load-bearing ML
interfaces to get exactly right. This *orders* the work; it does not shrink it —
every entry in the frozen-address table is still implemented (P8/P9).

---

## 2. Opcode / XML / KSCAN-mode usage (ordering aids) — status

Post-P9 these are pure *ordering + fuzz-weight* aids (the M4/M5 gates test
**every** opcode / XML entry / FP routine / KSCAN mode regardless — plan §6, D2
fencing). The dossier already gives the full element sets:
- **GPL opcodes**: the complete 256-entry map is `crates/libre99-gpl/src/isa.rs` /
  `../RECON.md` §8; the M4 microsuite covers all of them, census-ordered.
- **XML entries**: RECON §8 enumerates every table-0/1 entry (incl. the
  vestigial `>1C-1F`); M3/M5 gate all.
- **KSCAN modes**: RECON §5 (modes 0-5 + translation states); M2 gates all.

A dynamic usage histogram (which opcodes/XML entries/modes the corpus hammers,
to pick microsuite order and fuzz weight) is a cheap add on top of the GROM
track's `grom_log` / `coverage_sweep`, and is **deferred to M4/M5** where the
ordering is actually consumed — building it now would measure priorities we
don't yet act on. This is a deliberate scope call under P9, not an omission.

---

## 3. Deferred / concerns (come back to these)

1. **Full-137-corpus entry census.** The baseline covers 3 rich carts. The
   authoritative run launches **every** bundled cart (+ an F5 leg) and mashes
   input, reusing the GROM track's now-landed `coverage_sweep` launch driver
   (`crates/libre99-gpl/tests/coverage_sweep.rs`). Deferred until an M-milestone
   needs the full ordering; the P8 *validation* it would strengthen is already
   green on the sample and grounded in D1 + literature. **Concern, not a
   blocker.**
2. **The device-I/O entry path.** Capturing the `>4000` disk-DSR → ROM return
   entries needs the scripted ToD disk-load sequence (`device_io.rs`), not the
   generic key-mash. Fold in when M3 (device I/O) is built — M3's own
   differential probe exercises exactly this path.
3. **A CPU data-read census** (does any external code *read* ROM bytes as data,
   vs. the presumed "none" that guards SURFACE-MAP's byte-identical claim) needs
   a small bus-read instrument in `libre99-core` (the entry census watches PC, not
   operand reads). Low priority — the byte-identical set (vectors + dispatch/XML
   tables) is small and its members are addressed *as tables*, not read as
   opaque data. Fold in with M3/M6 if evidence appears.

None blocks the M1→M6 ladder: the census orders and validates, and D1
(`RECON.md` §15) is the complete, authoritative element list the gates enforce.