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

//! A complete two-pass TMS9900 assembler and cartridge packager for the TI-99/4A
//! emulator (Editor/Assembler-style source; the full instruction set and E/A
//! expression syntax, driven by the [`isa`] table). It assembles in one of two
//! modes:
//!
//! * **Cartridge mode** (default) — produces a bootable `.ctg`: a standard
//!   `>6000` ROM header (synthesized by default) plus the assembled machine code,
//!   packed into the `ti99sim` container via [`libre99_core::cartridge::write_v1`].
//! * **Absolute (raw-image) mode** ([`Options::absolute`]) — `AORG`-placed code
//!   and data with a per-byte overlap guard, used to build the console ROM.
//!
//! Two passes: pass 1 lays out the image, defines every label, and sizes each
//! statement; pass 2 evaluates operands (now that all symbols are known), encodes
//! instructions, and emits bytes. Forward references are therefore fine in
//! instruction and `DATA` operands.

// `expr`, `lex`, and `front` are the shared E/A front end; `libre99-gpl` reuses them
// so the two assemblers evaluate operand expressions, split source lines, and
// parse string literals identically.
pub mod disasm;
pub mod disk_dsr;
pub mod expr;
pub mod front;
pub mod isa;
pub mod lex;
pub mod system_rom;

use std::collections::{HashMap, HashSet};

use front::operands as operands_of;
use isa::Fmt;

/// One assembly diagnostic, tied to a source line.
#[derive(Debug, Clone)]
pub struct Diag {
    /// 1-based source line (`0` for whole-program diagnostics).
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
            write!(f, "{}: {}", self.line, self.message)
        }
    }
}

/// Assembly options.
pub struct Options {
    /// Cartridge menu title / `.ctg` banner (overrides `IDT`).
    pub name: Option<String>,
    /// Entry-point symbol (overrides `END`'s operand; default `START`).
    pub entry: Option<String>,
    /// Origin of the cartridge image.
    pub base: u16,
    /// Synthesize the standard `>6000` header + one menu entry (default `true`).
    pub auto_header: bool,
    /// **Absolute (raw-image) mode.** Enables `AORG` for placing code/data at
    /// fixed addresses (forward only; a backward `AORG` over already-placed
    /// bytes is a region-overlap error — this is the layout-drift guard), and
    /// zero-pads the result to [`Options::image_size`]. Used to build the
    /// console ROM (`base = 0`, `image_size = 0x2000`). Forces `auto_header`
    /// off. Default `false` (assisted-header cartridge mode).
    pub absolute: bool,
    /// In absolute mode, the final zero-padded image size (e.g. `0x2000` for
    /// the 8 KiB console ROM). `0` means "one 8 KiB bank", as in cartridge mode.
    pub image_size: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            name: None,
            entry: None,
            base: 0x6000,
            auto_header: true,
            absolute: false,
            image_size: 0,
        }
    }
}

impl Options {
    /// Options for a raw absolute image of `size` bytes originating at `>0000`
    /// (the console-ROM shape): `AORG` enabled, no header, no menu title.
    pub fn absolute_image(size: usize) -> Self {
        Options { base: 0, auto_header: false, absolute: true, image_size: size, ..Default::default() }
    }
}

/// A successful assembly.
#[derive(Debug, Clone)]
pub struct Assembly {
    /// The cartridge menu title (upper-cased).
    pub title: String,
    /// Resolved entry-point address.
    pub entry: u16,
    /// Emitted bytes from `base` (header + code), un-padded.
    pub image: Vec<u8>,
    /// `image` zero-padded to the target size (one 8 KiB bank in cartridge
    /// mode; [`Options::image_size`] in absolute mode), ready for packaging.
    pub rom: Vec<u8>,
    /// Defined symbols (name → value), sorted; predefined registers excluded.
    pub symbols: Vec<(String, u16)>,
    /// Per-source-line emission records `(address, object bytes, source text)`
    /// in program order, for [`Assembly::listing`].
    pub emitted: Vec<(u16, Vec<u8>, String)>,
}

impl Assembly {
    /// Pack this assembly into a `.ctg` cartridge image.
    pub fn ctg(&self) -> Vec<u8> {
        libre99_core::cartridge::write_v1(&self.title, 0, &self.rom, &[])
    }

    /// The resolved address of a defined symbol, if any.
    pub fn symbol(&self, name: &str) -> Option<u16> {
        self.symbols.iter().find(|(n, _)| n == name).map(|(_, v)| *v)
    }

    /// Assert that each `(name, addr)` pair holds — the **layout-assertion
    /// gate**: a builder calls this with the frozen public entry points
    /// (vectors, KSCAN, the ISR, …) so a size regression that shifts any of
    /// them fails the build loudly. Returns every mismatch (empty = all good).
    pub fn check_layout(&self, pins: &[(&str, u16)]) -> Vec<String> {
        pins.iter()
            .filter_map(|(name, want)| match self.symbol(name) {
                Some(got) if got == *want => None,
                Some(got) => Some(format!("layout: {name} is at >{got:04X}, expected >{want:04X}")),
                None => Some(format!("layout: {name} is undefined (expected >{want:04X})")),
            })
            .collect()
    }

    /// An address / object-hex / source-text listing.
    pub fn listing(&self) -> String {
        let mut out = String::new();
        for (addr, bytes, src) in &self.emitted {
            let obj: String =
                bytes.iter().take(8).map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("");
            let more = if bytes.len() > 8 { "…" } else { "" };
            out.push_str(&format!(">{addr:04X}  {obj:<16}{more:<2}{src}\n"));
        }
        out
    }

    /// The symbol table as a small hand-rolled JSON object (zero-dep). Values
    /// are hex strings so addresses read naturally: `{"START": ">0024", …}`.
    pub fn symbols_json(&self) -> String {
        let body: Vec<String> =
            self.symbols.iter().map(|(n, v)| format!("  {n:?}: \">{v:04X}\"")).collect();
        format!("{{\n{}\n}}\n", body.join(",\n"))
    }
}

/// One 8 KiB cartridge bank.
const BANK_SIZE: usize = 0x2000;

/// Assemble `src` into a cartridge image.
pub fn assemble(src: &str, opts: &Options) -> Result<Assembly, Vec<Diag>> {
    let lines = lex::parse(src);

    // Pre-scan for the module name (IDT) and the entry symbol (END's operand).
    let mut diags = Vec::new();
    let mut idt = None;
    let mut end_entry = None;
    for l in &lines {
        match l.mnemonic.as_deref().map(str::to_uppercase).as_deref() {
            Some("IDT") => match string_operand(l.operands.as_deref()) {
                Ok(s) => idt = Some(s),
                Err(e) => diags.push(Diag::at(l.num, e)),
            },
            Some("END") => end_entry = l.operands.clone().filter(|s| !s.is_empty()),
            _ => {}
        }
    }

    let title = opts.name.clone().or(idt).unwrap_or_else(|| "CART".to_string());
    let menu = title.to_uppercase();
    let entry_name = opts
        .entry
        .clone()
        .or(end_entry)
        .unwrap_or_else(|| "START".to_string());

    // Predefined workspace registers R0–R15.
    let mut syms = HashMap::new();
    let mut reg_names = HashSet::new();
    for n in 0..16u16 {
        let name = format!("R{n}");
        syms.insert(name.clone(), n as i64);
        reg_names.insert(name);
    }

    let auto_header = opts.auto_header && !opts.absolute;

    // The auto-header prefix size: 16-byte header + entry record + EVEN pad.
    let prefix = if auto_header {
        let mut p = 16 + 5 + menu.len();
        if !(opts.base as usize + p).is_multiple_of(2) {
            p += 1;
        }
        p
    } else {
        0
    };

    let mut a = Asm {
        syms,
        image: Vec::new(),
        used: Vec::new(),
        high: 0,
        wrote: 0,
        lc: opts.base.wrapping_add(prefix as u16),
        base: opts.base,
        diags,
        emit: false,
        absolute: opts.absolute,
    };

    // ---- PASS 1: define symbols and lay out the image. ----
    for l in &lines {
        a.process(l);
    }
    // The entry symbol is only needed when synthesizing the header.
    let entry = if auto_header {
        match a.syms.get(&entry_name).copied() {
            Some(v) => v as u16,
            None => {
                a.diags
                    .push(Diag::at(0, format!("entry symbol '{entry_name}' is undefined")));
                0
            }
        }
    } else {
        a.syms.get(&entry_name).copied().unwrap_or(0) as u16
    };
    if !a.diags.is_empty() {
        return Err(a.diags);
    }

    // ---- PASS 2: emit bytes, recording each line's object span for listings. ----
    a.emit = true;
    a.lc = opts.base;
    a.image.clear();
    a.used.clear();
    a.high = 0;
    if auto_header {
        a.emit_header(&menu, entry);
    }
    debug_assert_eq!(a.lc as usize, opts.base as usize + prefix);
    let src_lines: Vec<&str> = src.lines().collect();
    let mut emitted: Vec<(u16, Vec<u8>, String)> = Vec::new();
    for l in &lines {
        let addr = a.lc;
        a.wrote = 0;
        a.process(l);
        if a.wrote > 0 {
            // The line's bytes sit contiguously at its start address (only
            // `AORG` moves the LC, and it emits nothing).
            let start = addr.wrapping_sub(a.base) as usize;
            let text = src_lines.get(l.num.wrapping_sub(1)).copied().unwrap_or("").to_string();
            emitted.push((addr, a.image[start..start + a.wrote].to_vec(), text));
        }
    }
    if !a.diags.is_empty() {
        return Err(a.diags);
    }

    let target = if opts.image_size > 0 { opts.image_size } else { BANK_SIZE };
    if a.high > target {
        let what = if opts.absolute {
            format!("the {target}-byte image window")
        } else {
            "the 8 KiB cartridge window".to_string()
        };
        return Err(vec![Diag::at(
            0,
            format!("image spans {} bytes, which exceeds {what}", a.high),
        )]);
    }
    a.image.truncate(a.high);

    let mut rom = a.image.clone();
    rom.resize(target, 0);

    let mut symbols: Vec<(String, u16)> = a
        .syms
        .iter()
        .filter(|(k, _)| !reg_names.contains(*k))
        .map(|(k, v)| (k.clone(), *v as u16))
        .collect();
    symbols.sort();

    Ok(Assembly { title: menu, entry, image: a.image, rom, symbols, emitted })
}

/// Mutable assembler state, shared by both passes (`emit` selects the pass).
struct Asm {
    syms: HashMap<String, i64>,
    /// The output image, indexed by `lc - base` (random access: `AORG` may
    /// place regions in any order).
    image: Vec<u8>,
    /// Which image bytes have been written — the overlap guard: writing a byte
    /// twice means two regions collided (e.g. a routine grew past the next
    /// `AORG`), which is always a layout bug.
    used: Vec<bool>,
    /// One past the highest written image index (the image's logical length).
    high: usize,
    /// Bytes written by the current source line (for the listing records).
    wrote: usize,
    lc: u16,
    base: u16,
    diags: Vec<Diag>,
    emit: bool,
    /// Absolute mode: `AORG` is legal and the image is address-indexed from 0.
    absolute: bool,
}

impl Asm {
    fn push_u8(&mut self, v: u8) {
        if self.emit {
            let idx = self.lc.wrapping_sub(self.base) as usize;
            if idx >= self.image.len() {
                self.image.resize(idx + 1, 0);
                self.used.resize(idx + 1, false);
            }
            if self.used[idx] {
                self.diags.push(Diag::at(
                    0,
                    format!(
                        "overlap: address >{:04X} written twice — a region ran into another \
                         (check the AORG layout)",
                        self.lc
                    ),
                ));
            }
            self.image[idx] = v;
            self.used[idx] = true;
            self.high = self.high.max(idx + 1);
            self.wrote += 1;
        }
        self.lc = self.lc.wrapping_add(1);
    }

    fn push_u16(&mut self, v: u16) {
        self.push_u8((v >> 8) as u8);
        self.push_u8(v as u8);
    }

    /// Advance to a word boundary, emitting a `>00` pad byte if needed.
    fn align_word(&mut self) {
        if self.lc & 1 != 0 {
            self.push_u8(0);
        }
    }

    fn define(&mut self, name: &str, value: i64, line: usize) {
        if let Some(&existing) = self.syms.get(name) {
            // Redefining a symbol to the same value is a no-op (this is how `R3
            // EQU 3` in real E/A source stays harmless); a different value errors.
            if existing != value {
                self.diags
                    .push(Diag::at(line, format!("symbol '{name}' redefined")));
            }
            return;
        }
        self.syms.insert(name.to_string(), value);
    }

    fn eval(&self, s: &str) -> Result<i64, String> {
        expr::eval(s, &self.syms, self.lc)
    }

    /// Evaluate an operand as a workspace-register number (0–15).
    fn reg(&self, s: &str) -> Result<u16, String> {
        let v = self.eval(s)?;
        if !(0..=15).contains(&v) {
            return Err(format!("register number {v} is out of range 0..15"));
        }
        Ok(v as u16)
    }

    /// Parse a general operand into `(T, reg, optional extension word)`. The
    /// extension expression is only evaluated in the emit pass (so forward
    /// references in `@EXPR` operands are fine).
    fn general(&self, op: &str) -> Result<(u16, u16, Option<u16>), String> {
        if let Some(rest) = op.strip_prefix('*') {
            if let Some(r) = rest.strip_suffix('+') {
                Ok((3, self.reg(r)?, None)) // *Rn+
            } else {
                Ok((1, self.reg(rest)?, None)) // *Rn
            }
        } else if let Some(rest) = op.strip_prefix('@') {
            if let Some(open) = rest.find('(') {
                // @EXPR(Rn) — indexed.
                let inner = rest
                    .strip_suffix(')')
                    .ok_or("malformed indexed operand (missing ')')")?;
                let r = self.reg(&inner[open + 1..])?;
                if r == 0 {
                    return Err("@EXPR(R0) is not indexable; R0 means 'no index'".into());
                }
                let ext = if self.emit { Some(self.eval(&rest[..open])? as u16) } else { Some(0) };
                Ok((2, r, ext))
            } else {
                // @EXPR — symbolic.
                let ext = if self.emit { Some(self.eval(rest)? as u16) } else { Some(0) };
                Ok((2, 0, ext))
            }
        } else {
            Ok((0, self.reg(op)?, None)) // Rn — register direct
        }
    }

    fn emit_header(&mut self, menu: &str, entry: u16) {
        let program_list = self.base.wrapping_add(16);
        self.push_u8(0xAA); // valid flag
        self.push_u8(0x01); // version
        self.push_u8(0x01); // number of programs
        self.push_u8(0x00); // reserved
        self.push_u16(0x0000); // power-up list
        self.push_u16(program_list); // program (menu) list pointer
        self.push_u16(0x0000); // DSR list
        self.push_u16(0x0000); // subprogram list
        self.push_u16(0x0000); // interrupt/GPL link
        self.push_u16(0x0000); // unused
        // Single program-list entry.
        self.push_u16(0x0000); // next entry = none
        self.push_u16(entry); // entry address
        self.push_u8(menu.len() as u8);
        for &b in menu.as_bytes() {
            self.push_u8(b);
        }
        self.align_word();
    }

    fn process(&mut self, l: &lex::Line) {
        let mnem = l.mnemonic.as_deref().map(str::to_uppercase);

        // EQU is special: its label takes the operand's value, not the LC.
        if mnem.as_deref() == Some("EQU") {
            if !self.emit {
                match (&l.label, operands_of(l).first()) {
                    (Some(lbl), Some(op)) => match self.eval(op) {
                        Ok(v) => self.define(lbl, v, l.num),
                        Err(e) => self.diags.push(Diag::at(l.num, e)),
                    },
                    (None, _) => self.diags.push(Diag::at(l.num, "EQU requires a label")),
                    (Some(_), None) => self.diags.push(Diag::at(l.num, "EQU requires a value")),
                }
            }
            return;
        }

        // Instructions and DATA force a word boundary; align before the label so a
        // label points at the (aligned) data/instruction it names.
        let aligns = mnem.as_deref() == Some("DATA")
            || mnem.as_deref().is_some_and(|m| isa::lookup(m).is_some());
        if aligns {
            self.align_word();
        }

        if !self.emit {
            if let Some(lbl) = &l.label {
                self.define(lbl, self.lc as i64, l.num);
            }
        }

        let Some(m) = mnem.as_deref() else {
            return; // label-only or blank line
        };

        if let Some(insn) = isa::lookup(m) {
            if let Err(e) = self.encode(insn, l) {
                self.diags.push(Diag::at(l.num, e));
            }
            return;
        }

        match m {
            "IDT" | "END" => {} // consumed in the pre-scan
            "BYTE" => self.dir_byte(l),
            "DATA" => self.dir_data(l),
            "TEXT" => self.dir_text(l),
            "BSS" => self.dir_bss(l),
            "EVEN" => self.align_word(),
            "AORG" => self.dir_aorg(l),
            "COPY" => self.diags.push(Diag::at(
                l.num,
                "COPY must be expanded before assembly (call libre99_asm::expand_includes, \
                 or use the libre99asm CLI which does it for you)",
            )),
            other => self
                .diags
                .push(Diag::at(l.num, format!("unknown mnemonic or directive '{other}'"))),
        }
    }

    fn encode(&mut self, insn: &isa::Insn, l: &lex::Line) -> Result<(), String> {
        let ops = operands_of(l);
        match insn.fmt {
            Fmt::Dual => {
                if ops.len() != 2 {
                    return Err(format!("{} takes two operands", insn.name));
                }
                let (ts, s, sext) = self.general(&ops[0])?;
                let (td, d, dext) = self.general(&ops[1])?;
                self.push_u16(insn.base | (td << 10) | (d << 6) | (ts << 4) | s);
                if let Some(x) = sext {
                    self.push_u16(x);
                }
                if let Some(x) = dext {
                    self.push_u16(x);
                }
            }
            Fmt::Single => {
                if ops.len() != 1 {
                    return Err(format!("{} takes one operand", insn.name));
                }
                let (ts, s, ext) = self.general(&ops[0])?;
                self.push_u16(insn.base | (ts << 4) | s);
                if let Some(x) = ext {
                    self.push_u16(x);
                }
            }
            Fmt::Jump => {
                if ops.len() != 1 {
                    return Err(format!("{} takes one operand", insn.name));
                }
                let here = self.lc;
                if self.emit {
                    let target = self.eval(&ops[0])?;
                    let delta = target - (here as i64 + 2);
                    if delta % 2 != 0 {
                        return Err("jump target is at an odd address".into());
                    }
                    let disp = delta / 2;
                    if !(-128..=127).contains(&disp) {
                        return Err(format!(
                            "jump target out of range ({delta} bytes away; {} reaches -254..+256)",
                            insn.name
                        ));
                    }
                    self.push_u16(insn.base | ((disp as i16 as u16) & 0xFF));
                } else {
                    self.push_u16(0);
                }
            }
            Fmt::ImmReg => {
                if ops.len() != 2 {
                    return Err(format!("{} takes register,immediate", insn.name));
                }
                let r = self.reg(&ops[0])?;
                self.push_u16(insn.base | r);
                let imm = if self.emit { self.eval(&ops[1])? as u16 } else { 0 };
                self.push_u16(imm);
            }
            Fmt::Imm => {
                if ops.len() != 1 {
                    return Err(format!("{} takes an immediate", insn.name));
                }
                self.push_u16(insn.base);
                let imm = if self.emit { self.eval(&ops[0])? as u16 } else { 0 };
                self.push_u16(imm);
            }
            Fmt::Pseudo => {
                if !ops.is_empty() {
                    return Err(format!("{} takes no operands", insn.name));
                }
                self.push_u16(insn.base);
            }
            Fmt::RegSrc => {
                // COC/CZC/XOR/MPY/DIV: src,Rd ; XOP: src,n. The 4-bit field sits
                // in bits 6–9 either way.
                if ops.len() != 2 {
                    return Err(format!("{} takes source,register", insn.name));
                }
                let (ts, s, ext) = self.general(&ops[0])?;
                let field = if insn.name == "XOP" {
                    self.count(&ops[1], "XOP n")?
                } else {
                    self.reg(&ops[1])?
                };
                self.push_u16(insn.base | (field << 6) | (ts << 4) | s);
                if let Some(x) = ext {
                    self.push_u16(x);
                }
            }
            Fmt::CruMulti => {
                // LDCR/STCR: src,count (0–15; 0 means 16).
                if ops.len() != 2 {
                    return Err(format!("{} takes source,count", insn.name));
                }
                let (ts, s, ext) = self.general(&ops[0])?;
                let cnt = self.count(&ops[1], insn.name)?;
                self.push_u16(insn.base | (cnt << 6) | (ts << 4) | s);
                if let Some(x) = ext {
                    self.push_u16(x);
                }
            }
            Fmt::Shift => {
                // SLA/SRA/SRL/SRC: Wreg,count (0–15; 0 means "from R0").
                if ops.len() != 2 {
                    return Err(format!("{} takes register,count", insn.name));
                }
                let w = self.reg(&ops[0])?;
                let cnt = self.count(&ops[1], insn.name)?;
                self.push_u16(insn.base | (cnt << 4) | w);
            }
            Fmt::CruBit => {
                // SBO/SBZ/TB: the operand is the signed displacement value itself.
                if ops.len() != 1 {
                    return Err(format!("{} takes a CRU-bit displacement", insn.name));
                }
                let disp = if self.emit { self.eval(&ops[0])? } else { 0 };
                if self.emit && !(-128..=127).contains(&disp) {
                    return Err(format!(
                        "{} displacement {disp} out of range -128..127",
                        insn.name
                    ));
                }
                self.push_u16(insn.base | (disp as i16 as u16 & 0xFF));
            }
            Fmt::RegOnly => {
                if ops.len() != 1 {
                    return Err(format!("{} takes a register", insn.name));
                }
                let r = self.reg(&ops[0])?;
                self.push_u16(insn.base | r);
            }
            Fmt::Control => {
                if !ops.is_empty() {
                    return Err(format!("{} takes no operands", insn.name));
                }
                self.push_u16(insn.base);
            }
        }
        Ok(())
    }

    /// Evaluate a 0–15 count/field operand (only in the emit pass; pass 1 returns
    /// 0 since counts never affect a statement's size).
    fn count(&self, op: &str, what: &str) -> Result<u16, String> {
        if !self.emit {
            return Ok(0);
        }
        let c = self.eval(op)?;
        if !(0..=15).contains(&c) {
            return Err(format!("{what} count {c} out of range 0..15"));
        }
        Ok((c as u16) & 0xF)
    }

    fn dir_byte(&mut self, l: &lex::Line) {
        for op in operands_of(l) {
            let v = self.eval_value(&op, l.num);
            if self.emit && !(-128..=255).contains(&v) {
                self.diags
                    .push(Diag::at(l.num, format!("BYTE value {v} out of range -128..255")));
            }
            self.push_u8(v as u8);
        }
    }

    fn dir_data(&mut self, l: &lex::Line) {
        // The word boundary was already forced by `process`.
        for op in operands_of(l) {
            let v = self.eval_value(&op, l.num);
            if self.emit && !(-32768..=65535).contains(&v) {
                self.diags
                    .push(Diag::at(l.num, format!("DATA value {v} out of range -32768..65535")));
            }
            self.push_u16(v as u16);
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
        // A leading '-' negates the last emitted byte (E/A idiom).
        let (negate_last, body) = match raw.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, raw),
        };
        let bytes = match string_operand(Some(body)) {
            Ok(s) => s.into_bytes(),
            Err(e) => {
                self.diags.push(Diag::at(l.num, e));
                return;
            }
        };
        let last = bytes.len().saturating_sub(1);
        for (i, &b) in bytes.iter().enumerate() {
            let v = if negate_last && i == last { b.wrapping_neg() } else { b };
            self.push_u8(v);
        }
    }

    /// `AORG >addr` — set the location counter to an absolute address (absolute
    /// mode only). Regions may be laid out in **any order**; gaps are zero-
    /// filled, and any two regions touching the same byte trip the per-byte
    /// overlap guard in `push_u8` (a routine growing past the next region's
    /// origin is always a layout bug).
    fn dir_aorg(&mut self, l: &lex::Line) {
        if !self.absolute {
            self.diags.push(Diag::at(
                l.num,
                "AORG needs absolute mode (assisted-header mode sets the origin automatically)",
            ));
            return;
        }
        let Some(op) = operands_of(l).into_iter().next() else {
            self.diags.push(Diag::at(l.num, "AORG requires an address"));
            return;
        };
        // The origin controls layout, so it must resolve in pass 1 too.
        let target = match self.eval(&op) {
            Ok(v) if (0..=0xFFFF).contains(&v) => v as u16,
            Ok(v) => {
                self.diags.push(Diag::at(l.num, format!("AORG address {v} out of range >0000..>FFFF")));
                return;
            }
            Err(e) => {
                self.diags.push(Diag::at(l.num, e));
                return;
            }
        };
        self.lc = target;
    }

    fn dir_bss(&mut self, l: &lex::Line) {
        // BSS must be well-defined in pass 1 to advance the LC.
        let n = match operands_of(l).first() {
            Some(op) => self.eval_value(op, l.num),
            None => {
                self.diags.push(Diag::at(l.num, "BSS requires a size"));
                0
            }
        };
        for _ in 0..n.max(0) {
            self.push_u8(0);
        }
    }

    /// Evaluate an operand value, recording (in the emit pass) any error and
    /// substituting 0. In pass 1 returns 0 without evaluating (sizing only needs
    /// counts, not values — except `BSS`/`EQU`, which evaluate directly).
    fn eval_value(&mut self, op: &str, line: usize) -> i64 {
        if !self.emit {
            // Pass 1: BSS needs the value (it controls layout); for BYTE/DATA the
            // value is irrelevant to size, so skip evaluation to allow forward
            // references and report errors only once, in pass 2.
            return self.eval(op).unwrap_or(0);
        }
        match self.eval(op) {
            Ok(v) => v,
            Err(e) => {
                self.diags.push(Diag::at(line, e));
                0
            }
        }
    }
}

/// Recursively expand `COPY '<file>'` directives by inlining the referenced
/// source, resolving each path through `resolve` (so callers control the search
/// root and this stays filesystem-agnostic and unit-testable). Include cycles
/// and runaway nesting are rejected.
///
/// The console-ROM source splits into component files (`kernel.asm`, `isr.asm`,
/// …) `COPY`'d from a top-level `console.asm`; this stitches them before the
/// two-pass assembler runs. Diagnostics after expansion count lines in the
/// combined text (single-file source is unaffected — a per-file line map is a
/// later refinement, only worth it once the split lands).
pub fn expand_includes(
    src: &str,
    resolve: &impl Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    fn go(
        src: &str,
        resolve: &impl Fn(&str) -> Result<String, String>,
        depth: usize,
    ) -> Result<String, String> {
        if depth > 32 {
            return Err("COPY nesting deeper than 32 — include cycle?".into());
        }
        let mut out = String::new();
        for line in src.lines() {
            if let Some(path) = copy_target(line) {
                let included =
                    resolve(&path).map_err(|e| format!("COPY '{path}': {e}"))?;
                out.push_str(&go(&included, resolve, depth + 1)?);
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        Ok(out)
    }
    go(src, resolve, 0)
}

/// If `line` is a `COPY '<file>'` statement, return the quoted path.
fn copy_target(line: &str) -> Option<String> {
    let parsed = lex::parse(line);
    let l = parsed.first()?;
    if l.mnemonic.as_deref().map(str::to_uppercase).as_deref() != Some("COPY") {
        return None;
    }
    string_operand(l.operands.as_deref()).ok()
}

/// `Option`-taking wrapper over [`front::string_operand`]: the `IDT` pre-scan and
/// `COPY` pass a line's optional operand field, where `None` means "no operand at
/// all" (distinct from a malformed string).
fn string_operand(s: Option<&str>) -> Result<String, String> {
    front::string_operand(s.ok_or("expected a quoted string")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assemble a snippet with no auto-header and return the emitted bytes.
    fn enc(src: &str) -> Vec<u8> {
        let opts = Options { auto_header: false, ..Default::default() };
        assemble(src, &opts).expect("assembles").image
    }

    #[test]
    fn dual_operand_modes() {
        assert_eq!(enc("   MOV R1,R2"), [0xC0, 0x81]);
        assert_eq!(enc("   MOV *R1+,R2"), [0xC0, 0xB1]);
        assert_eq!(enc("   MOVB *R1+,@>8C02"), [0xD8, 0x31, 0x8C, 0x02]);
    }

    #[test]
    fn source_extension_precedes_destination() {
        assert_eq!(
            enc("INIT EQU >0125\n   MOV @INIT+3,@3"),
            [0xC8, 0x20, 0x01, 0x28, 0x00, 0x03]
        );
    }

    #[test]
    fn immediate_and_single() {
        assert_eq!(enc("   LI R0,>1234"), [0x02, 0x00, 0x12, 0x34]);
        assert_eq!(enc("   LIMI 0"), [0x03, 0x00, 0x00, 0x00]);
        assert_eq!(enc("   DEC R2"), [0x06, 0x02]);
        assert_eq!(enc("   CLR R0"), [0x04, 0xC0]);
    }

    #[test]
    fn jumps_and_pseudos() {
        assert_eq!(enc("   JMP $"), [0x10, 0xFF]);
        assert_eq!(enc("   JMP $+2"), [0x10, 0x00]);
        assert_eq!(enc("   NOP"), [0x10, 0x00]);
        assert_eq!(enc("   RT"), [0x04, 0x5B]);
    }

    #[test]
    fn data_directives_and_alignment() {
        assert_eq!(enc("   DATA 4+5*2"), [0x00, 0x12]); // = 18, left to right
        assert_eq!(enc("   DATA >37AC"), [0x37, 0xAC]);
        assert_eq!(enc("   DATA -32768"), [0x80, 0x00]);
        assert_eq!(enc("   BYTE 'C'"), [0x43]);
        assert_eq!(enc("   TEXT 'EXAMPLE'"), [0x45, 0x58, 0x41, 0x4D, 0x50, 0x4C, 0x45]);
        assert_eq!(enc("   TEXT -'AB'"), [0x41, 0xBE]);
        // DATA word-aligns after an odd-length BYTE; BYTE does not pad.
        assert_eq!(enc("   BYTE 1\n   DATA >1234"), [0x01, 0x00, 0x12, 0x34]);
        assert_eq!(enc("   BYTE 1\n   BYTE 2"), [0x01, 0x02]);
    }

    #[test]
    fn undefined_symbol_is_reported() {
        let opts = Options { auto_header: false, ..Default::default() };
        let err = assemble("   LI R0,NOPE", &opts).unwrap_err();
        assert!(err[0].message.contains("NOPE"), "got {:?}", err);
    }

    #[test]
    fn extended_instruction_set() {
        // Oracle: the opcodes asserted in libre99-core's tests/cpu.rs.
        assert_eq!(enc("   XOR R1,R2"), [0x28, 0x81]);
        assert_eq!(enc("   COC R1,R2"), [0x20, 0x81]);
        assert_eq!(enc("   CZC R1,R2"), [0x24, 0x81]);
        assert_eq!(enc("   MPY R1,R2"), [0x38, 0x81]);
        assert_eq!(enc("   DIV R1,R2"), [0x3C, 0x81]);
        assert_eq!(enc("   LDCR R0,3"), [0x30, 0xC0]);
        assert_eq!(enc("   STCR R1,8"), [0x36, 0x01]);
        assert_eq!(enc("   LDCR R0,8"), [0x32, 0x00]); // count 8 -> byte transfer
        assert_eq!(enc("   SLA R1,1"), [0x0A, 0x11]);
        assert_eq!(enc("   SRL R1,1"), [0x09, 0x11]);
        assert_eq!(enc("   SRA R1,1"), [0x08, 0x11]);
        assert_eq!(enc("   SRC R1,4"), [0x0B, 0x41]);
        assert_eq!(enc("   SLA R1,0"), [0x0A, 0x01]); // count 0 -> from R0
        assert_eq!(enc("   SBO 1"), [0x1D, 0x01]);
        assert_eq!(enc("   SBZ 1"), [0x1E, 0x01]);
        assert_eq!(enc("   TB 1"), [0x1F, 0x01]);
        assert_eq!(enc("   STWP R0"), [0x02, 0xA0]);
        assert_eq!(enc("   STST R0"), [0x02, 0xC0]);
        assert_eq!(enc("   RTWP"), [0x03, 0x80]);
        // XOP with a symbolic source: ext word follows; n in bits 6–9.
        assert_eq!(enc("X EQU >0100\n   XOP @X,1"), [0x2C, 0x60, 0x01, 0x00]);
    }

    #[test]
    fn indexed_addressing() {
        // @EXPR(Rn): T=2, the index register in the S field, EXPR as the ext word.
        assert_eq!(enc("   MOV @4(R1),R2"), [0xC0, 0xA1, 0x00, 0x04]);
        assert_eq!(enc("TAB EQU >6100\n   MOV @TAB(R3),R0"), [0xC0, 0x23, 0x61, 0x00]);
        // @EXPR(R0) is rejected — R0 in the index field means "no index".
        let opts = Options { auto_header: false, ..Default::default() };
        assert!(assemble("   MOV @4(R0),R1", &opts).is_err());
    }

    /// Assemble in absolute (raw-image) mode and return the padded image.
    fn abs(src: &str, size: usize) -> Vec<u8> {
        assemble(src, &Options::absolute_image(size)).expect("assembles").rom
    }

    #[test]
    fn aorg_places_at_absolute_addresses_with_zero_fill() {
        // Vector at >0000, code at >0024 — the console-ROM shape. Bytes between
        // are zero-filled; the image is exactly `size` long.
        let src = "\
             \x20   DATA >83E0\n\
             \x20   DATA START\n\
             \x20   AORG >0024\n\
             START LI R0,>1700\n";
        let rom = abs(src, 0x2000);
        assert_eq!(rom.len(), 0x2000);
        assert_eq!(&rom[0..4], [0x83, 0xE0, 0x00, 0x24]); // WP, PC=>0024
        assert_eq!(&rom[4..0x24], &[0u8; 0x20]); // gap is zero
        assert_eq!(&rom[0x24..0x28], [0x02, 0x00, 0x17, 0x00]); // LI R0,>1700
    }

    #[test]
    fn aorg_backward_over_content_is_an_overlap_error() {
        // A region at >0030 that runs 4 bytes, then an AORG back to >0032 — the
        // second region would overlap the first. This is the layout-drift guard.
        let src = "\
             \x20   AORG >0030\n\
             \x20   DATA >1111,>2222\n\
             \x20   AORG >0032\n\
             \x20   DATA >3333\n";
        let err = assemble(src, &Options::absolute_image(0x2000)).unwrap_err();
        assert!(err.iter().any(|d| d.message.contains("overlap")), "got {err:?}");
    }

    #[test]
    fn aorg_regions_may_be_out_of_order() {
        // Pinned islands can appear in any source order; gaps zero-fill.
        let src = "\
             \x20   AORG >0100\n\
             \x20   DATA >AAAA\n\
             \x20   AORG >0010\n\
             \x20   DATA >BBBB\n\
             \x20   AORG >0102\n\
             \x20   DATA >CCCC\n";
        let rom = abs(src, 0x2000);
        assert_eq!(&rom[0x10..0x12], [0xBB, 0xBB]);
        assert_eq!(&rom[0x100..0x104], [0xAA, 0xAA, 0xCC, 0xCC]);
        assert_eq!(rom[0x50], 0x00);
    }

    #[test]
    fn aorg_needs_absolute_mode() {
        let err = assemble("   AORG >0024", &Options { auto_header: false, ..Default::default() })
            .unwrap_err();
        assert!(err[0].message.contains("absolute mode"), "got {:?}", err);
    }

    #[test]
    fn absolute_image_overflow_is_reported() {
        // Placing a word past the image window overflows.
        let err = assemble("   AORG >1FFF\n   DATA >1234", &Options::absolute_image(0x2000))
            .unwrap_err();
        assert!(err[0].message.contains("exceeds"), "got {:?}", err);
    }

    #[test]
    fn layout_check_catches_a_shifted_entry() {
        let src = "\x20   AORG >0024\nSTART LI R0,>1700\n";
        let a = assemble(src, &Options::absolute_image(0x2000)).unwrap();
        assert!(a.check_layout(&[("START", 0x0024)]).is_empty());
        let bad = a.check_layout(&[("START", 0x0070)]);
        assert_eq!(bad.len(), 1);
        assert!(bad[0].contains("START"));
    }

    #[test]
    fn expand_includes_inlines_and_detects_cycles() {
        use std::collections::HashMap;
        let mut files = HashMap::new();
        files.insert("body.asm".to_string(), "   LI R0,1\n".to_string());
        files.insert("loop.asm".to_string(), "   COPY 'loop.asm'\n".to_string());
        let resolve = |p: &str| {
            files.get(p).cloned().ok_or_else(|| format!("no such file {p}"))
        };
        let out =
            expand_includes("   NOP\n   COPY 'body.asm'\n   RT\n", &resolve).unwrap();
        assert!(out.contains("NOP") && out.contains("LI R0,1") && out.contains("RT"));
        // A self-referential COPY is caught, not looped forever.
        let err = expand_includes("   COPY 'loop.asm'\n", &resolve).unwrap_err();
        assert!(err.contains("cycle") || err.contains("deep"), "got {err}");
    }
}
