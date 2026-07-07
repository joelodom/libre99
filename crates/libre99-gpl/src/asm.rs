// Modified MIT License
//
// Copyright (c) 2026 Joel Odom
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, and sublicense copies of the
// Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// "Commons Clause" License Condition v1.0
//
// The Software is provided to you by the Licensor under the License, subject to
// the following condition.
//
// Without limiting other conditions in the License, the grant of rights under the
// License will not include, and the License does not grant to you, the right to
// Sell the Software.
//
// For purposes of the foregoing, "Sell" means practicing any or all of the rights
// granted to you under the License to provide to third parties, for a fee or other
// consideration (including without limitation fees for hosting or consulting/
// support services related to the Software), a product or service whose value
// derives, entirely or substantially, from the functionality of the Software. Any
// license notice or attribution required by the License must also include this
// Commons Clause License Condition notice.
//
// Software: Libre99
//
// License: Modified MIT
//
// Licensor: Joel Odom
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! The GPL assembler: original GPL source → a 24 KiB system-GROM image the
//! emulator boots in place of `994AGROM.Bin`.
//!
//! Two passes, mirroring `libre99-asm` (`crates/libre99-asm/src/lib.rs:90`): pass 1
//! defines every label at its GROM address and sizes each statement; pass 2
//! evaluates operands (forward references now resolvable) and emits bytes. The
//! shared E/A front end (`libre99_asm::lex`, `libre99_asm::expr`) tokenises lines and
//! evaluates operand expressions, so GPL and TMS9900 source share one syntax.
//!
//! GROM addressing is **absolute** (unlike `libre99-asm`'s forced `>6000`): a
//! `GROM >addr` / `AORG >addr` directive sets the location counter, labels
//! resolve to GROM addresses, and `B`/`CALL`/`BR`/`BS` take absolute targets.
//!
//! **v0 constraint:** a symbol used inside a *memory* (GAS) operand must be
//! defined earlier in the source (backward reference), because a GAS operand's
//! byte length depends on its value. Forward references are fine in `B`, `CALL`,
//! `BR`, `BS`, `DATA`, and immediate operands, which are all fixed-length.

use std::collections::HashMap;

use libre99_asm::front::{operands, string_operand};
use libre99_asm::lex;

use crate::encode;
use crate::isa::{self, Sig};
use crate::operand::Operand;

/// One 8 KiB GROM slot.
const GROM_SLOT: usize = 0x2000;
/// A system-GROM image is three 8 KiB slots (GROMs 0/1/2 at `>0000–5FFF`).
pub const GROM_IMAGE_LEN: usize = 3 * GROM_SLOT;

/// One assembly diagnostic tied to a source line (`0` = whole program).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub line: usize,
    pub message: String,
}

impl Diag {
    fn at(line: usize, message: impl Into<String>) -> Self {
        Diag { line, message: message.into() }
    }
}

impl std::fmt::Display for Diag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

/// A successful assembly.
#[derive(Debug, Clone)]
pub struct Assembly {
    /// The 24 KiB system-GROM image, zero-filled where unpopulated.
    pub image: Vec<u8>,
    /// Defined symbols (name → GROM address/value), sorted.
    pub symbols: Vec<(String, u16)>,
}

/// Assemble GPL `src` into a system-GROM image.
pub fn assemble(src: &str) -> Result<Assembly, Vec<Diag>> {
    let lines = lex::parse(src);
    let mut a = Asm {
        syms: HashMap::new(),
        image: vec![0u8; GROM_IMAGE_LEN],
        lc: 0,
        diags: Vec::new(),
        emit: false,
    };

    // ---- PASS 1: define symbols, size statements. ----
    for l in &lines {
        a.process(l);
    }
    if !a.diags.is_empty() {
        return Err(a.diags);
    }

    // ---- PASS 2: emit bytes. ----
    a.emit = true;
    a.lc = 0;
    for l in &lines {
        a.process(l);
    }
    if !a.diags.is_empty() {
        return Err(a.diags);
    }

    let mut symbols: Vec<(String, u16)> =
        a.syms.iter().map(|(k, v)| (k.clone(), *v as u16)).collect();
    symbols.sort();
    Ok(Assembly { image: a.image, symbols })
}

struct Asm {
    syms: HashMap<String, i64>,
    image: Vec<u8>,
    lc: u16,
    diags: Vec<Diag>,
    emit: bool,
}

impl Asm {
    fn push(&mut self, b: u8) {
        if self.emit {
            let i = self.lc as usize;
            if i < self.image.len() {
                self.image[i] = b;
            } else {
                self.diags.push(Diag::at(0, format!("address >{:04X} is past the 24 KiB image", self.lc)));
            }
        }
        self.lc = self.lc.wrapping_add(1);
    }

    fn define(&mut self, name: &str, value: i64, line: usize) {
        if let Some(&existing) = self.syms.get(name) {
            if existing != value && !self.emit {
                self.diags.push(Diag::at(line, format!("symbol '{name}' redefined")));
            }
            return;
        }
        self.syms.insert(name.to_string(), value);
    }

    /// Evaluate an expression. `strict` errors on an undefined symbol; non-strict
    /// (pass-1 sizing of fixed-length operands) substitutes 0.
    fn eval(&self, s: &str, strict: bool) -> Result<i64, String> {
        match libre99_asm::expr::eval(s, &self.syms, self.lc) {
            Ok(v) => Ok(v),
            Err(e) => {
                if strict {
                    Err(e)
                } else {
                    Ok(0)
                }
            }
        }
    }

    fn process(&mut self, l: &lex::Line) {
        let mnem = l.mnemonic.as_deref().map(str::to_uppercase);

        // EQU takes the operand's value, not the LC.
        if mnem.as_deref() == Some("EQU") {
            if !self.emit {
                match (&l.label, operands(l).first()) {
                    (Some(lbl), Some(op)) => match self.eval(op, true) {
                        Ok(v) => self.define(lbl, v, l.num),
                        Err(e) => self.diags.push(Diag::at(l.num, e)),
                    },
                    (None, _) => self.diags.push(Diag::at(l.num, "EQU requires a label")),
                    (Some(_), None) => self.diags.push(Diag::at(l.num, "EQU requires a value")),
                }
            }
            return;
        }

        // GROM/AORG set the location counter (before the label is placed).
        if matches!(mnem.as_deref(), Some("GROM") | Some("AORG")) {
            match operands(l).first() {
                Some(op) => match self.eval(op, true) {
                    Ok(v) => self.lc = v as u16,
                    Err(e) => self.diags.push(Diag::at(l.num, e)),
                },
                None => self.diags.push(Diag::at(l.num, "GROM/AORG requires an address")),
            }
            if let (false, Some(lbl)) = (self.emit, &l.label) {
                self.define(lbl, self.lc as i64, l.num);
            }
            return;
        }

        // Place the label at the current LC (pass 1).
        if !self.emit {
            if let Some(lbl) = &l.label {
                self.define(lbl, self.lc as i64, l.num);
            }
        }

        let Some(m) = mnem.as_deref() else {
            return; // label-only or blank
        };

        // MOVE is special: its opcode and operand layout depend on the dest kind.
        if m == "MOVE" {
            if let Err(e) = self.move_insn(l) {
                self.diags.push(Diag::at(l.num, e));
            }
            return;
        }

        if let Some(found) = isa::lookup(m) {
            let r = match found {
                isa::Lookup::Named(named) => self.encode_insn(named, l),
                isa::Lookup::Two { base_w, word } => self.two_op(base_w, word, l),
                isa::Lookup::One { op, .. } => self.one_op(op, l),
            };
            if let Err(e) = r {
                self.diags.push(Diag::at(l.num, e));
            }
            return;
        }

        match m {
            "END" => {}
            "BYTE" => self.dir_byte(l),
            "DATA" => self.dir_data(l),
            "TEXT" => self.dir_text(l),
            "BSS" => self.dir_bss(l),
            "EVEN" => {
                if self.lc & 1 != 0 {
                    self.push(0);
                }
            }
            other => self.diags.push(Diag::at(l.num, format!("unknown mnemonic or directive '{other}'"))),
        }
    }

    fn encode_insn(&mut self, named: &isa::NamedOp, l: &lex::Line) -> Result<(), String> {
        let ops = operands(l);

        // Branches need the LC and slot arithmetic — handle here.
        if matches!(named.sig, Sig::Branch) {
            if ops.len() != 1 {
                return Err(format!("{} takes one target", named.name));
            }
            let target = self.eval(&ops[0], self.emit)? as u16;
            if self.emit && (target & 0xE000) != (self.lc & 0xE000) {
                return Err(format!(
                    "{} target >{target:04X} is in a different 8 KiB slot than >{:04X}",
                    named.name, self.lc
                ));
            }
            let op = named.opcode | (((target >> 8) & 0x1F) as u8);
            self.push(op);
            self.push(target as u8);
            return Ok(());
        }

        // Resolve operands per the signature, then encode.
        let resolved = self.resolve_operands(named.sig, &ops)?;
        let bytes = encode::encode(named.opcode, named.sig, &resolved)?;
        for b in bytes {
            self.push(b);
        }
        Ok(())
    }

    /// Parse+evaluate the operand strings into resolved [`Operand`]s for `sig`.
    /// GAS operands are evaluated strictly (their length depends on the value);
    /// immediate/address operands are non-strict in pass 1 (fixed length).
    fn resolve_operands(&mut self, sig: Sig, ops: &[String]) -> Result<Vec<Operand>, String> {
        let mut out = Vec::new();
        match sig {
            Sig::None => {}
            Sig::Imm8 | Sig::Addr16 => {
                let v = self.eval(&ops[0], self.emit)? as u16;
                out.push(if matches!(sig, Sig::Addr16) {
                    Operand::Grom(v)
                } else {
                    Operand::Imm8(v as u8)
                });
            }
            Sig::Gas => out.push(self.parse_gas(&ops[0])?),
            Sig::GasGas => {
                out.push(self.parse_gas(&ops[0])?);
                out.push(self.parse_gas(&ops[1])?);
            }
            Sig::GasImm8 => {
                out.push(self.parse_gas(&ops[0])?);
                out.push(Operand::Imm8(self.eval(&ops[1], self.emit)? as u8));
            }
            Sig::GasImm16 => {
                out.push(self.parse_gas(&ops[0])?);
                out.push(Operand::Imm16(self.eval(&ops[1], self.emit)? as u16));
            }
            Sig::Branch | Sig::Move | Sig::Fmt | Sig::Unknown => {
                return Err("unsupported operand signature".into())
            }
        }
        Ok(out)
    }

    /// A format-1 two-operand instruction: `OP dst, src`, byte stream
    /// `[opcode][dst GAS][src GAS or imm]`. `base_w` carries the W bit; the U
    /// (immediate-source) bit is chosen here from the source operand's shape.
    fn two_op(&mut self, base_w: u8, word: bool, l: &lex::Line) -> Result<(), String> {
        let ops = operands(l);
        if ops.len() != 2 {
            return Err("takes dst, src".into());
        }
        let dst = self.parse_gas(&ops[0])?;
        let mut bytes = Vec::new();
        if is_mem_operand(&ops[1]) {
            bytes.push(base_w);
            crate::operand::encode_gas(&dst, &mut bytes)?;
            let src = self.parse_gas(&ops[1])?;
            crate::operand::encode_gas(&src, &mut bytes)?;
        } else {
            bytes.push(base_w | 0x02);
            crate::operand::encode_gas(&dst, &mut bytes)?;
            let raw = self.eval(&ops[1], self.emit)?;
            // Range-check the immediate *before* truncating, mirroring `dir_byte`:
            // the byte form takes -128..=255, the word form -32768..=65535. Only
            // in the emit pass, so pass-1's zero-substituted forward references
            // don't false-trip. Without this, `ST @cell,>1FF` (byte form) would
            // silently truncate to >FF.
            if self.emit {
                let (lo, hi) = if word { (-32768, 65535) } else { (-128, 255) };
                if !(lo..=hi).contains(&raw) {
                    let width = if word { 16 } else { 8 };
                    return Err(format!(
                        "immediate source {raw} out of range for the {width}-bit form ({lo}..={hi})"
                    ));
                }
            }
            let v = raw as u16;
            if word {
                bytes.push((v >> 8) as u8);
            }
            bytes.push(v as u8);
        }
        for b in bytes {
            self.push(b);
        }
        Ok(())
    }

    /// A format-5 single-operand instruction: `[opcode][GAS]`.
    fn one_op(&mut self, op: u8, l: &lex::Line) -> Result<(), String> {
        let ops = operands(l);
        if ops.len() != 1 {
            return Err("takes one operand".into());
        }
        let dst = self.parse_gas(&ops[0])?;
        let mut bytes = vec![op];
        crate::operand::encode_gas(&dst, &mut bytes)?;
        for b in bytes {
            self.push(b);
        }
        Ok(())
    }

    /// Encode the block MOVE (format 6): `MOVE count, src, dst`; byte stream
    /// `[opcode][count][dest][source]` (dest before source). Operand shapes:
    ///
    /// * count — an expression (immediate word, N=1) or `@…` (a GAS operand the
    ///   count is read from, N=0 — used for run-time lengths).
    /// * src — `G@expr` (immediate GROM address), `G*@…` (GROM address read from
    ///   the named CPU cell, C=1 — used to walk cartridge headers), or a RAM
    ///   GAS operand `@…`/`V@…` (V=1).
    /// * dst — `#expr` (starting VDP register, R=1) or a RAM GAS operand.
    ///
    /// The >31/>39 forms are execution-verified (`examples/move_probe.rs`); the
    /// bit field is Classic99 `gpl.cpp:709-729`.
    fn move_insn(&mut self, l: &lex::Line) -> Result<(), String> {
        let ops = operands(l);
        if ops.len() != 3 {
            return Err("MOVE takes count, src, dst".into());
        }

        let mut bits = isa::MoveBits {
            not_grom_dst: true,
            reg_dst: false,
            ram_src: false,
            cpu_held_grom_src: false,
            imm_count: false,
        };

        // Classify operands first (to build the opcode), then emit.
        let count_is_mem = is_mem_operand(&ops[0]);
        bits.imm_count = !count_is_mem;

        let src = ops[1].trim();
        enum Src {
            GromImm(String),
            GromViaCpu(String),
            Ram(String),
        }
        let src = if let Some(r) = src.strip_prefix("G*") {
            bits.cpu_held_grom_src = true;
            Src::GromViaCpu(r.to_string())
        } else if let Some(r) = src.strip_prefix("G@").or_else(|| src.strip_prefix("g@")) {
            Src::GromImm(r.to_string())
        } else if is_mem_operand(src) {
            bits.ram_src = true;
            Src::Ram(src.to_string())
        } else {
            return Err("MOVE source must be G@addr, G*@cell, @cpu, or V@vdp".into());
        };

        let dst = ops[2].trim();
        let reg_dst = dst.strip_prefix('#');
        if reg_dst.is_some() {
            bits.reg_dst = true;
        }

        let mut bytes = vec![bits.opcode()];
        // Count.
        if count_is_mem {
            let c = self.parse_gas(&ops[0])?;
            crate::operand::encode_gas(&c, &mut bytes)?;
        } else {
            let c = self.eval(&ops[0], self.emit)? as u16;
            bytes.push((c >> 8) as u8);
            bytes.push(c as u8);
        }
        // Destination.
        if let Some(reg_expr) = reg_dst {
            bytes.push(self.eval(reg_expr, self.emit)? as u8);
        } else {
            let d = self.parse_gas(dst)?;
            crate::operand::encode_gas(&d, &mut bytes)?;
        }
        // Source.
        match src {
            Src::GromImm(expr) => {
                let s = self.eval(&expr, self.emit)? as u16;
                bytes.push((s >> 8) as u8);
                bytes.push(s as u8);
            }
            Src::GromViaCpu(op) | Src::Ram(op) => {
                let s = self.parse_gas(&op)?;
                crate::operand::encode_gas(&s, &mut bytes)?;
            }
        }
        for b in bytes {
            self.push(b);
        }
        Ok(())
    }

    /// Parse a memory (GAS) operand: `@expr`, `V@expr`, `*@expr`, `*V@expr`, with
    /// an optional `(@ixcell)` index. The address expression is evaluated
    /// strictly (backward references only — see the module note).
    fn parse_gas(&self, s: &str) -> Result<Operand, String> {
        let s = s.trim();
        let (indirect, rest) = match s.strip_prefix('*') {
            Some(r) => (true, r),
            None => (false, s),
        };
        let (vdp, rest) = match rest.strip_prefix("V@").or_else(|| rest.strip_prefix("v@")) {
            Some(r) => (true, r),
            None => match rest.strip_prefix('@') {
                Some(r) => (false, r),
                None => return Err(format!("memory operand must start with @ or V@: `{s}`")),
            },
        };
        // Indexed addressing exists in the ISA (the X bit) but failed execution
        // verification (m4_probe: an indexed ST did not land where the docs
        // imply). Until a probe pins it, the assembler rejects it — use the
        // execution-verified indirect forms instead (`*@cell` byte pointer,
        // `*V@cell` word VDP pointer).
        if rest.contains('(') {
            return Err(
                "indexed addressing is unverified; use indirect (*@cell / *V@cell) instead".into(),
            );
        }
        let index = None;
        let addr = self.eval(rest, true)? as u16;
        Ok(if vdp {
            Operand::Vdp { addr, indirect, index }
        } else {
            Operand::Cpu { addr, indirect, index }
        })
    }

    fn dir_byte(&mut self, l: &lex::Line) {
        for op in operands(l) {
            let v = self.eval_val(&op, l.num);
            if self.emit && !(-128..=255).contains(&v) {
                self.diags.push(Diag::at(l.num, format!("BYTE value {v} out of range")));
            }
            self.push(v as u8);
        }
    }

    fn dir_data(&mut self, l: &lex::Line) {
        for op in operands(l) {
            let v = self.eval_val(&op, l.num);
            self.push((v >> 8) as u8);
            self.push(v as u8);
        }
    }

    fn dir_text(&mut self, l: &lex::Line) {
        let raw = match l.operands.as_deref() {
            Some(s) => s,
            None => {
                self.diags.push(Diag::at(l.num, "TEXT requires a string"));
                return;
            }
        };
        let (negate_last, body) = match raw.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, raw),
        };
        let bytes = match string_operand(body) {
            Ok(s) => s.into_bytes(),
            Err(e) => {
                self.diags.push(Diag::at(l.num, e));
                return;
            }
        };
        let last = bytes.len().saturating_sub(1);
        for (i, &b) in bytes.iter().enumerate() {
            let v = if negate_last && i == last { b.wrapping_neg() } else { b };
            self.push(v);
        }
    }

    fn dir_bss(&mut self, l: &lex::Line) {
        let n = match operands(l).first() {
            Some(op) => self.eval(op, true).unwrap_or_else(|e| {
                self.diags.push(Diag::at(l.num, e));
                0
            }),
            None => {
                self.diags.push(Diag::at(l.num, "BSS requires a size"));
                0
            }
        };
        for _ in 0..n.max(0) {
            self.push(0);
        }
    }

    /// Evaluate a data value (BYTE/DATA), reporting errors only in the emit pass
    /// so forward references are allowed.
    fn eval_val(&mut self, op: &str, line: usize) -> i64 {
        match self.eval(op, self.emit) {
            Ok(v) => v,
            Err(e) => {
                self.diags.push(Diag::at(line, e));
                0
            }
        }
    }
}

/// Does this operand text name memory (a GAS operand) rather than an immediate?
fn is_mem_operand(s: &str) -> bool {
    let s = s.trim();
    s.starts_with('@') || s.starts_with("V@") || s.starts_with("v@") || s.starts_with('*')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asm(src: &str) -> Assembly {
        assemble(src).unwrap_or_else(|d| panic!("assembly failed: {:?}", d))
    }

    /// Read `n` bytes at GROM `addr` from an image.
    fn at(img: &[u8], addr: u16, n: usize) -> &[u8] {
        &img[addr as usize..addr as usize + n]
    }

    #[test]
    fn backdrop_at_entry() {
        let a = asm("        GROM >0020\n        BACK >17\n");
        assert_eq!(at(&a.image, 0x0020, 2), [0x04, 0x17]);
    }

    #[test]
    fn store_immediates() {
        let a = asm("        GROM >0000\n        ST @>8400,>9F\n        DST @>8372,>FF7E\n");
        assert_eq!(at(&a.image, 0x0000, 4), [0xBE, 0x81, 0x00, 0x9F]);
        assert_eq!(at(&a.image, 0x0004, 4), [0xBF, 0x72, 0xFF, 0x7E]);
    }

    #[test]
    fn branches_and_labels() {
        // BR forward to a label in the same slot.
        let a = asm("        GROM >0020\nSTART   BR   DONE\n        BACK >01\nDONE    RTN\n");
        // DONE is at >0024 (BR=2 bytes, BACK=2 bytes). BR >0024: >40|0, disp >24.
        assert_eq!(at(&a.image, 0x0020, 2), [0x40, 0x24]);
        assert_eq!(a.image[0x0024], 0x00); // RTN at DONE
    }

    #[test]
    fn absolute_branch() {
        let a = asm("        GROM >0000\n        B >4D12\n        CALL >1234\n");
        assert_eq!(at(&a.image, 0x0000, 3), [0x05, 0x4D, 0x12]);
        assert_eq!(at(&a.image, 0x0003, 3), [0x06, 0x12, 0x34]);
    }

    #[test]
    fn data_directives() {
        let a = asm("        GROM >0000\n        BYTE >AA,>02\n        DATA >1320\n        TEXT 'HI'\n");
        assert_eq!(at(&a.image, 0x0000, 2), [0xAA, 0x02]);
        assert_eq!(at(&a.image, 0x0002, 2), [0x13, 0x20]);
        assert_eq!(at(&a.image, 0x0004, 2), [0x48, 0x49]);
    }

    #[test]
    fn cross_slot_branch_is_rejected() {
        let err = assemble("        GROM >1FF0\n        BR >2005\n").unwrap_err();
        assert!(err[0].message.contains("slot"), "got {:?}", err);
    }

    #[test]
    fn move_to_registers_matches_boot_trace() {
        // RECON.md: `39 00 08 00 04 51` = move 8 bytes GROM >0451 -> VDP regs.
        let a = asm("        GROM >0000\n        MOVE >0008,G@>0451,#0\n");
        assert_eq!(at(&a.image, 0x0000, 6), [0x39, 0x00, 0x08, 0x00, 0x04, 0x51]);
    }

    #[test]
    fn move_to_vram_matches_probe() {
        // move_probe: op >31, layout count:imm16, VDP-GAS dst, GROM16 src.
        let a = asm("        GROM >0000\n        MOVE 4,G@>0100,V@>0000\n");
        assert_eq!(at(&a.image, 0x0000, 7), [0x31, 0x00, 0x04, 0xA0, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn two_op_immediate_out_of_range_is_rejected() {
        // Byte form: >1FF (511) would silently truncate to >FF — reject it.
        let err = assemble("        GROM >0000\n        ST @>8300,>1FF\n").unwrap_err();
        assert!(err[0].message.contains("out of range"), "got {:?}", err);
        assert_eq!(err[0].line, 2, "diagnostic carries the source line");
        // Byte form: one below the signed floor.
        let err = assemble("        GROM >0000\n        ST @>8300,-129\n").unwrap_err();
        assert!(err[0].message.contains("out of range"), "got {:?}", err);
        // Word form: >10000 (65536) overflows the 16-bit field.
        let err = assemble("        GROM >0000\n        DST @>8300,>10000\n").unwrap_err();
        assert!(err[0].message.contains("out of range"), "got {:?}", err);
        // Word form: one below the signed floor.
        let err = assemble("        GROM >0000\n        DST @>8300,-32769\n").unwrap_err();
        assert!(err[0].message.contains("out of range"), "got {:?}", err);
    }

    #[test]
    fn two_op_immediate_boundaries_are_accepted() {
        // Byte form accepts the full -128..=255 window (unsigned max and signed min).
        let a = asm("        GROM >0000\n        ST @>8300,>FF\n");
        assert_eq!(at(&a.image, 0x0000, 3), [0xBE, 0x00, 0xFF]);
        let a = asm("        GROM >0000\n        ST @>8300,-128\n");
        assert_eq!(at(&a.image, 0x0000, 3), [0xBE, 0x00, 0x80]);
        // Word form accepts -32768..=65535.
        let a = asm("        GROM >0000\n        DST @>8300,>FFFF\n");
        assert_eq!(at(&a.image, 0x0000, 4), [0xBF, 0x00, 0xFF, 0xFF]);
        let a = asm("        GROM >0000\n        DST @>8300,-32768\n");
        assert_eq!(at(&a.image, 0x0000, 4), [0xBF, 0x00, 0x80, 0x00]);
    }

    #[test]
    fn vdp_and_indirect_operands() {
        // VDP direct >0380 => 12-bit form with V bit: A3 80.
        let a = asm("        GROM >0000\n        ST V@>0380,>17\n");
        assert_eq!(at(&a.image, 0x0000, 4), [0xBE, 0xA3, 0x80, 0x17]);
    }
}
