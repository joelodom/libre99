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

//! # TMS9900 — the TI-99/4A's 16-bit CPU
//!
//! The TMS9900 (1976) is one of the first single-chip 16-bit microprocessors. Its
//! defining peculiarity is that **it has almost no on-chip registers**: the 16
//! general registers R0–R15 live in main memory, at consecutive words starting at
//! the address held in the on-chip **Workspace Pointer (WP)**. Register `Rn` is
//! simply the word at `WP + 2*n`. The only true on-chip state is:
//!
//! * **WP** — Workspace Pointer (points at R0 in RAM)
//! * **PC** — Program Counter
//! * **ST** — Status register (flags + interrupt mask)
//!
//! This "memory-to-memory" design makes context switches almost free: to switch
//! to a new set of registers you just change WP. The `BLWP`/`RTWP` instructions
//! and all interrupts exploit this — they load a new WP and stash the caller's
//! WP/PC/ST into three registers of the *new* workspace (R13/R14/R15), so a
//! subroutine or interrupt handler can return by reversing the process.
//!
//! ## Word orientation and byte operations
//!
//! The data bus is 16 bits and **big-endian**: word addresses are even, and the
//! high byte of a word is at the lower address. Byte instructions (`MOVB`, `AB`,
//! `CB`, …) operate on the byte selected by address bit 0 — the **high** byte at
//! an even address, the low byte at an odd address. Because registers sit at even
//! addresses (`WP + 2*n`), a byte operation on a register touches its **high**
//! byte. This module implements byte access as read-modify-write over the
//! containing word, exactly as the hardware does on its word-only bus.
//!
//! ## Status register layout (TI numbers bits with the MSB as bit 0)
//!
//! ```text
//! bit  0    1    2    3    4    5    6    7 .. 11      12 13 14 15
//!      L>   A>   EQ   C    OV   OP   X    (reserved)   interrupt mask
//!     0x8000              0x1000               0x0200  ──── 0x000F ────
//! ```
//! * **L>** logical greater-than (unsigned compare result)
//! * **A>** arithmetic greater-than (signed compare result)
//! * **EQ** equal
//! * **C**  carry
//! * **OV** overflow (signed)
//! * **OP** odd parity (set only by byte instructions)
//! * **X**  set while executing an `XOP`-vectored routine
//! * **mask** the lowest interrupt level the CPU will currently accept
//!
//! ## Reset and interrupts
//!
//! All vectors are pairs `(WP, PC)` in low memory. **Reset** vectors through
//! `>0000`. A **level-N interrupt** vectors through `>4*N` and, on acceptance,
//! lowers the mask to `N-1` so only strictly-higher-priority interrupts can nest.
//! On the bare TI-99/4A every interrupt source is wired so the CPU sees it as
//! **level 1** (vector `>0004`); software tells the VDP and peripheral interrupts
//! apart by polling the 9901 over the CRU.
//!
//! ## Timing
//!
//! Instruction cycle counts come from the TMS9900 datasheet (a base cost plus
//! per-operand addressing-mode add-ons). On the TI-99/4A most of the address
//! space is reached through an 8-bit multiplexer that adds wait states; the CPU
//! asks the [`Bus`] for those via [`Bus::wait_states`] on every access and folds
//! them into the returned cycle count. This is "cycle-aware" timing: accurate
//! enough to pace the ~60 Hz interrupt and to make slow-bus code run slower than
//! scratchpad code, which is what the firmware and games actually depend on.

use crate::bus::Bus;

// Status-register bit masks (TI bit numbering; MSB = bit 0).
/// Logical greater-than (unsigned).
pub const ST_LGT: u16 = 0x8000;
/// Arithmetic greater-than (signed).
pub const ST_AGT: u16 = 0x4000;
/// Equal.
pub const ST_EQ: u16 = 0x2000;
/// Carry.
pub const ST_C: u16 = 0x1000;
/// Overflow (signed).
pub const ST_OV: u16 = 0x0800;
/// Odd parity (byte instructions only).
pub const ST_OP: u16 = 0x0400;
/// Executing an XOP-vectored routine.
pub const ST_X: u16 = 0x0200;
/// Interrupt mask (low 4 bits).
pub const ST_MASK: u16 = 0x000F;

/// The TMS9900 processor.
///
/// Holds only the three architectural registers (WP/PC/ST) plus emulator
/// bookkeeping (a cycle total, the pending-interrupt line, and the IDLE latch).
/// Everything else — including R0–R15 — lives in memory behind the [`Bus`].
#[derive(Default)]
pub struct Cpu {
    wp: u16,
    pc: u16,
    st: u16,
    /// Total elapsed clock cycles since construction (for pacing/diagnostics).
    cycles: u64,
    /// Externally-asserted interrupt request, as a level (1 is the only level the
    /// bare console uses). `None` means no interrupt is pending. The line is
    /// level-sensitive: whoever raised it is responsible for lowering it once the
    /// handler acknowledges the source (e.g. by reading the VDP status).
    interrupt_request: Option<u8>,
    /// Set by `IDLE`; the CPU spins (consuming cycles) until an interrupt is
    /// accepted, which clears it.
    idle: bool,
    /// Scratch accumulator: cycles charged to the instruction currently
    /// executing (base + addressing add-ons + wait states). Not architectural.
    op_cycles: u32,
    /// Diagnostic count of undefined opcodes executed (see [`Cpu::illegal`]).
    /// Not architectural and not serialized — a wedged program that has run
    /// off into data is otherwise invisible from outside.
    illegal_ops: u64,
    /// Optional PC-coverage bitmap: one bit per word-aligned CPU address
    /// (32768 words = 4 KiB), set for the PC of every instruction executed.
    /// `None` (the default) disables coverage so normal runs allocate nothing
    /// and pay one branch per step. Diagnostics only — never serialized (a
    /// loaded state starts with coverage off). The GROM has the matching
    /// read-side instrument ([`crate::grom::Grom::record_coverage`]); together
    /// they answer "which firmware code ran / was fetched" — e.g. which
    /// console-ROM ranges a cartridge like Extended BASIC actually executes.
    pc_coverage: Option<Box<[u64; 512]>>,
}

impl Cpu {
    /// Construct a CPU in a blank state. Call [`Cpu::reset`] (or set WP/PC
    /// directly in tests) before stepping.
    pub fn new() -> Self {
        Cpu::default()
    }

    // ---- accessors -------------------------------------------------------
    pub fn wp(&self) -> u16 {
        self.wp
    }
    pub fn pc(&self) -> u16 {
        self.pc
    }
    pub fn st(&self) -> u16 {
        self.st
    }
    pub fn cycles(&self) -> u64 {
        self.cycles
    }
    /// How many undefined opcodes have been executed (diagnostics; not part
    /// of the architectural state and not saved in snapshots).
    pub fn illegal_count(&self) -> u64 {
        self.illegal_ops
    }
    /// Enable/disable the PC-coverage bitmap. Enabling installs a fresh,
    /// zeroed bitmap (each enable starts a clean census); disabling drops it,
    /// freeing the 4 KiB and making every coverage query inert.
    pub fn record_pc_coverage(&mut self, on: bool) {
        self.pc_coverage = if on { Some(Box::new([0u64; 512])) } else { None };
    }
    /// Whether an instruction has executed at (word-aligned) `addr` since
    /// coverage was enabled. Always `false` while coverage is off.
    pub fn pc_was_executed(&self, addr: u16) -> bool {
        match &self.pc_coverage {
            Some(cov) => {
                let w = (addr >> 1) as usize;
                cov[w >> 6] & (1u64 << (w & 63)) != 0
            }
            None => false,
        }
    }
    /// Every word-aligned PC executed since coverage was enabled, ascending.
    pub fn pc_coverage_addresses(&self) -> Vec<u16> {
        let Some(cov) = &self.pc_coverage else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (i, &word) in cov.iter().enumerate() {
            let mut bits = word;
            while bits != 0 {
                let b = bits.trailing_zeros() as usize;
                out.push((((i << 6) | b) << 1) as u16);
                bits &= bits - 1;
            }
        }
        out
    }
    pub fn set_wp(&mut self, v: u16) {
        self.wp = v;
    }
    pub fn set_pc(&mut self, v: u16) {
        self.pc = v;
    }
    pub fn set_st(&mut self, v: u16) {
        self.st = v;
    }

    /// Raise or lower the external interrupt request line. Pass `Some(level)` to
    /// assert an interrupt of that level, `None` to clear it. The machine wiring
    /// calls this from the 9901/VDP state each step.
    pub fn set_interrupt_request(&mut self, level: Option<u8>) {
        self.interrupt_request = level;
    }

    /// Power-on / RESET: load WP and PC from the reset vector at `>0000`/`>0002`
    /// and disable interrupts (mask = 0). The console ROM's vector is
    /// `WP=>83E0, PC=>0024`.
    pub fn reset(&mut self, bus: &mut impl Bus) {
        self.wp = bus.read_word(0x0000);
        self.pc = bus.read_word(0x0002);
        self.st = 0;
        self.idle = false;
        self.interrupt_request = None;
    }

    /// Serialize the CPU's architectural state (WP/PC/ST), the elapsed-cycle
    /// counter, and the interrupt/idle latches into a save state.
    pub(crate) fn save_state(&self, w: &mut crate::state::StateWriter) {
        w.u16(self.wp);
        w.u16(self.pc);
        w.u16(self.st);
        w.u64(self.cycles);
        w.opt_u8(self.interrupt_request);
        w.bool(self.idle);
        w.u32(self.op_cycles);
    }

    /// Restore the CPU from a save state.
    pub(crate) fn load_state(
        &mut self,
        r: &mut crate::state::StateReader<'_>,
    ) -> Result<(), crate::state::StateError> {
        self.wp = r.u16()?;
        self.pc = r.u16()?;
        self.st = r.u16()?;
        self.cycles = r.u64()?;
        self.interrupt_request = r.opt_u8()?;
        self.idle = r.bool()?;
        self.op_cycles = r.u32()?;
        // Diagnostics never persist across a load.
        self.illegal_ops = 0;
        Ok(())
    }

    /// Execute one instruction (or accept one pending interrupt) and return the
    /// number of clock cycles it took.
    pub fn step(&mut self, bus: &mut impl Bus) -> u32 {
        self.op_cycles = 0;

        // Interrupts are sampled at instruction boundaries. A request of level L
        // is accepted when L <= mask. Acceptance performs a context switch and
        // counts as this step's work.
        if let Some(level) = self.interrupt_request {
            if (level as u16) <= (self.st & ST_MASK) {
                self.accept_interrupt(bus, level);
                let c = self.op_cycles;
                self.cycles += c as u64;
                return c;
            }
        }

        // IDLE halts the processor until an interrupt is accepted (handled
        // above). While idling we still burn cycles so the rest of the machine
        // keeps advancing toward the next vblank.
        if self.idle {
            self.add_cycles(4);
            let c = self.op_cycles;
            self.cycles += c as u64;
            return c;
        }

        if let Some(cov) = self.pc_coverage.as_mut() {
            let w = (self.pc >> 1) as usize;
            cov[w >> 6] |= 1u64 << (w & 63);
        }
        let insn = self.fetch(bus);
        self.execute(bus, insn);

        let c = self.op_cycles;
        self.cycles += c as u64;
        c
    }

    // =====================================================================
    // Memory / register primitives (all wait-state aware)
    // =====================================================================

    fn add_cycles(&mut self, n: u32) {
        self.op_cycles += n;
    }

    /// Absolute word read, charging the region's wait states.
    fn read_word_at(&mut self, bus: &mut impl Bus, addr: u16) -> u16 {
        self.op_cycles += bus.wait_states_rw(addr, false);
        bus.read_word(addr)
    }
    /// Absolute word write, charging the region's wait states.
    fn write_word_at(&mut self, bus: &mut impl Bus, addr: u16, value: u16) {
        self.op_cycles += bus.wait_states_rw(addr, true);
        bus.write_word(addr, value);
    }

    /// Byte read through the bus (which knows the byte semantics of its device
    /// ports), charging the region's wait states once.
    fn read_byte_at(&mut self, bus: &mut impl Bus, addr: u16) -> u8 {
        self.op_cycles += bus.wait_states_rw(addr, false);
        bus.read_byte(addr)
    }
    /// Byte write through the bus, charging the region's wait states once.
    fn write_byte_at(&mut self, bus: &mut impl Bus, addr: u16, value: u8) {
        self.op_cycles += bus.wait_states_rw(addr, true);
        bus.write_byte(addr, value);
    }

    /// Address in memory of workspace register `n`.
    fn reg_addr(&self, n: u16) -> u16 {
        self.wp.wrapping_add((n & 0xF) << 1)
    }
    fn read_reg(&mut self, bus: &mut impl Bus, n: u16) -> u16 {
        let a = self.reg_addr(n);
        self.read_word_at(bus, a)
    }
    fn write_reg(&mut self, bus: &mut impl Bus, n: u16, v: u16) {
        let a = self.reg_addr(n);
        self.write_word_at(bus, a, v);
    }

    /// Fetch the word at PC and advance PC past it.
    fn fetch(&mut self, bus: &mut impl Bus) -> u16 {
        let w = self.read_word_at(bus, self.pc);
        self.pc = self.pc.wrapping_add(2);
        w
    }

    // =====================================================================
    // Status-flag helpers
    // =====================================================================

    fn set_flag(&mut self, bit: u16, on: bool) {
        if on {
            self.st |= bit;
        } else {
            self.st &= !bit;
        }
    }
    fn flag(&self, bit: u16) -> bool {
        self.st & bit != 0
    }

    /// Set L>/A>/EQ by comparing a 16-bit result to zero (the common case for
    /// result-producing instructions).
    fn set_lae(&mut self, v: u16) {
        self.set_flag(ST_LGT, v != 0);
        self.set_flag(ST_AGT, (v as i16) > 0);
        self.set_flag(ST_EQ, v == 0);
    }
    /// Set L>/A>/EQ for a byte result, treating it as an 8-bit value.
    fn set_lae_byte(&mut self, v: u8) {
        self.set_flag(ST_LGT, v != 0);
        self.set_flag(ST_AGT, (v as i8) > 0);
        self.set_flag(ST_EQ, v == 0);
    }
    /// Set L>/A>/EQ by comparing two words (for `C`, `CI`).
    fn set_compare(&mut self, a: u16, b: u16) {
        self.set_flag(ST_LGT, a > b);
        self.set_flag(ST_AGT, (a as i16) > (b as i16));
        self.set_flag(ST_EQ, a == b);
    }
    /// Set L>/A>/EQ by comparing two bytes (for `CB`).
    fn set_compare_byte(&mut self, a: u8, b: u8) {
        self.set_flag(ST_LGT, a > b);
        self.set_flag(ST_AGT, (a as i8) > (b as i8));
        self.set_flag(ST_EQ, a == b);
    }
    /// Odd-parity flag from a byte.
    fn set_parity(&mut self, v: u8) {
        self.set_flag(ST_OP, v.count_ones() & 1 == 1);
    }

    /// 16-bit add with carry/overflow and L>/A>/EQ.
    fn alu_add(&mut self, a: u16, b: u16) -> u16 {
        let (r, carry) = a.overflowing_add(b);
        self.set_flag(ST_C, carry);
        // Signed overflow: operands share a sign that differs from the result's.
        self.set_flag(ST_OV, ((a ^ r) & (b ^ r) & 0x8000) != 0);
        self.set_lae(r);
        r
    }
    /// 16-bit subtract `a - b`. Carry = "no borrow" (a >= b unsigned).
    fn alu_sub(&mut self, a: u16, b: u16) -> u16 {
        let r = a.wrapping_sub(b);
        self.set_flag(ST_C, a >= b);
        // Signed overflow on subtract: operands differ in sign and the result's
        // sign differs from the minuend's.
        self.set_flag(ST_OV, ((a ^ b) & (a ^ r) & 0x8000) != 0);
        self.set_lae(r);
        r
    }
    fn alu_add_byte(&mut self, a: u8, b: u8) -> u8 {
        let full = a as u16 + b as u16;
        let r = full as u8;
        self.set_flag(ST_C, full & 0x100 != 0);
        self.set_flag(ST_OV, ((a ^ r) & (b ^ r) & 0x80) != 0);
        self.set_lae_byte(r);
        self.set_parity(r);
        r
    }
    fn alu_sub_byte(&mut self, a: u8, b: u8) -> u8 {
        let r = a.wrapping_sub(b);
        self.set_flag(ST_C, a >= b);
        self.set_flag(ST_OV, ((a ^ b) & (a ^ r) & 0x80) != 0);
        self.set_lae_byte(r);
        self.set_parity(r);
        r
    }

    // =====================================================================
    // Operand resolution (addressing modes)
    // =====================================================================

    /// Resolve a `Ts,S` (or `Td,D`) operand field to an **effective address** in
    /// memory, applying autoincrement and consuming any extension word. Because
    /// registers are memory, register-direct operands resolve to `WP + 2*reg`,
    /// which unifies all operand handling onto plain word/byte memory access.
    ///
    /// Addressing modes (the 2-bit `t` field):
    /// * `0` — `Rn`           : the register itself (`WP + 2*reg`)
    /// * `1` — `*Rn`          : memory at the address in `Rn`
    /// * `2` — `@A` / `@A(Rn)`: symbolic/indexed; an extension word `A` follows,
    ///   plus `Rn` (when `reg != 0`) as an index
    /// * `3` — `*Rn+`         : like `*Rn`, then `Rn += 1` (byte) or `2` (word)
    fn resolve(&mut self, bus: &mut impl Bus, t: u16, reg: u16, byte: bool) -> u16 {
        match t {
            0 => self.reg_addr(reg),
            1 => {
                self.add_cycles(4);
                self.read_reg(bus, reg)
            }
            2 => {
                self.add_cycles(8);
                let base = self.fetch(bus);
                if reg == 0 {
                    base // pure symbolic @A  (R0 is not usable as an index here)
                } else {
                    base.wrapping_add(self.read_reg(bus, reg))
                }
            }
            3 => {
                let a = self.read_reg(bus, reg);
                let inc = if byte { 1 } else { 2 };
                self.write_reg(bus, reg, a.wrapping_add(inc));
                self.add_cycles(if byte { 6 } else { 8 });
                a
            }
            _ => unreachable!("2-bit addressing field"),
        }
    }

    // =====================================================================
    // Decode and dispatch
    // =====================================================================

    /// Decode and execute a single instruction word (PC already past it). Called
    /// by [`Cpu::step`], and recursively by the `X` instruction.
    fn execute(&mut self, bus: &mut impl Bus, insn: u16) {
        let nibble = insn >> 12;
        match nibble {
            // --- Format I: two general operands (the bulk of real code) -----
            0x4..=0xF => self.exec_dual_operand(bus, insn),

            // --- 0x2000–0x3FFF: COC/CZC/XOR/XOP, LDCR/STCR, MPY/DIV ----------
            0x2 | 0x3 => self.exec_2xxx(bus, insn),

            // --- 0x1000–0x1FFF: jumps and single-bit CRU --------------------
            0x1 => self.exec_1xxx(bus, insn),

            // --- 0x0000–0x0FFF: shifts, single-operand, immediates, control -
            0x0 => self.exec_0xxx(bus, insn),

            _ => unreachable!(),
        }
    }

    /// Format I: `op Td,D,Ts,S`. The low opcode bit selects the byte variant.
    fn exec_dual_operand(&mut self, bus: &mut impl Bus, insn: u16) {
        let opc = insn >> 12; // 0x4..0xF
        let byte = opc & 1 == 1;
        let td = (insn >> 10) & 3;
        let d = (insn >> 6) & 0xF;
        let ts = (insn >> 4) & 3;
        let s = insn & 0xF;
        self.add_cycles(14);

        // Resolve the source and READ its value before touching the
        // destination — the hardware fetches the source operand completely
        // (address, then data) before it begins destination decode. The order
        // is observable: a destination autoincrement can rewrite the very
        // memory the source names (e.g. `A R3,*R3+`, whose source is R3's
        // register cell and whose destination bumps R3), and the operation
        // must use the value from BEFORE that write.
        let src_ea = self.resolve(bus, ts, s, byte);

        if byte {
            let sval = self.read_byte_at(bus, src_ea);
            let dst_ea = self.resolve(bus, td, d, byte);
            match opc {
                0x5 => {
                    // SZCB: dst &= ~src
                    let dval = self.read_byte_at(bus, dst_ea);
                    let r = dval & !sval;
                    self.set_lae_byte(r);
                    self.set_parity(r);
                    self.write_byte_at(bus, dst_ea, r);
                }
                0x7 => {
                    // SB: dst -= src
                    let dval = self.read_byte_at(bus, dst_ea);
                    let r = self.alu_sub_byte(dval, sval);
                    self.write_byte_at(bus, dst_ea, r);
                }
                0x9 => {
                    // CB: compare src to dst (no store); parity from source byte
                    let dval = self.read_byte_at(bus, dst_ea);
                    self.set_compare_byte(sval, dval);
                    self.set_parity(sval);
                }
                0xB => {
                    // AB: dst += src
                    let dval = self.read_byte_at(bus, dst_ea);
                    let r = self.alu_add_byte(dval, sval);
                    self.write_byte_at(bus, dst_ea, r);
                }
                0xD => {
                    // MOVB: dst = src
                    self.set_lae_byte(sval);
                    self.set_parity(sval);
                    self.write_byte_at(bus, dst_ea, sval);
                }
                0xF => {
                    // SOCB: dst |= src
                    let dval = self.read_byte_at(bus, dst_ea);
                    let r = dval | sval;
                    self.set_lae_byte(r);
                    self.set_parity(r);
                    self.write_byte_at(bus, dst_ea, r);
                }
                _ => unreachable!(),
            }
        } else {
            let sval = self.read_word_at(bus, src_ea);
            let dst_ea = self.resolve(bus, td, d, byte);
            match opc {
                0x4 => {
                    // SZC: dst &= ~src
                    let dval = self.read_word_at(bus, dst_ea);
                    let r = dval & !sval;
                    self.set_lae(r);
                    self.write_word_at(bus, dst_ea, r);
                }
                0x6 => {
                    // S: dst -= src
                    let dval = self.read_word_at(bus, dst_ea);
                    let r = self.alu_sub(dval, sval);
                    self.write_word_at(bus, dst_ea, r);
                }
                0x8 => {
                    // C: compare src to dst (no store)
                    let dval = self.read_word_at(bus, dst_ea);
                    self.set_compare(sval, dval);
                }
                0xA => {
                    // A: dst += src
                    let dval = self.read_word_at(bus, dst_ea);
                    let r = self.alu_add(dval, sval);
                    self.write_word_at(bus, dst_ea, r);
                }
                0xC => {
                    // MOV: dst = src
                    self.set_lae(sval);
                    self.write_word_at(bus, dst_ea, sval);
                }
                0xE => {
                    // SOC: dst |= src
                    let dval = self.read_word_at(bus, dst_ea);
                    let r = dval | sval;
                    self.set_lae(r);
                    self.write_word_at(bus, dst_ea, r);
                }
                _ => unreachable!(),
            }
        }
    }

    /// 0x2000–0x3FFF: COC, CZC, XOR, XOP (Format III/IX with a register/number in
    /// bits 9–6 and a general source) and LDCR/STCR, MPY/DIV.
    fn exec_2xxx(&mut self, bus: &mut impl Bus, insn: u16) {
        let family = insn & 0xFC00;
        let d = (insn >> 6) & 0xF;
        let ts = (insn >> 4) & 3;
        let s = insn & 0xF;
        match family {
            0x2000 => {
                // COC src,Rd : EQ if every 1 bit of src is also 1 in Rd.
                self.add_cycles(14);
                let ea = self.resolve(bus, ts, s, false);
                let sval = self.read_word_at(bus, ea);
                let dval = self.read_reg(bus, d);
                self.set_flag(ST_EQ, (sval & dval) == sval);
            }
            0x2400 => {
                // CZC src,Rd : EQ if every 1 bit of src is 0 in Rd.
                self.add_cycles(14);
                let ea = self.resolve(bus, ts, s, false);
                let sval = self.read_word_at(bus, ea);
                let dval = self.read_reg(bus, d);
                self.set_flag(ST_EQ, (sval & dval) == 0);
            }
            0x2800 => {
                // XOR src,Rd : Rd ^= src.
                self.add_cycles(14);
                let ea = self.resolve(bus, ts, s, false);
                let sval = self.read_word_at(bus, ea);
                let dval = self.read_reg(bus, d);
                let r = dval ^ sval;
                self.set_lae(r);
                self.write_reg(bus, d, r);
            }
            0x2C00 => {
                // XOP src,n : software interrupt through vector >0040 + 4n.
                self.add_cycles(36);
                let ea = self.resolve(bus, ts, s, false);
                self.do_xop(bus, ea, d);
            }
            0x3000 | 0x3400 => self.exec_cru_multi(bus, insn),
            0x3800 => {
                // MPY src,Rd : Rd:Rd+1 = Rd * src (unsigned 16x16 -> 32).
                // "Rd+1" is the next memory *address* (registers are memory),
                // so for d=15 the low word lands at WP+32, one word past the
                // workspace — no modulo-16 register wrap (Classic99
                // cpu9900.cpp op_mpy: WRWORD(D+2, ...)).
                self.add_cycles(52);
                let ea = self.resolve(bus, ts, s, false);
                let sval = self.read_word_at(bus, ea) as u32;
                let dval = self.read_reg(bus, d) as u32;
                let prod = dval.wrapping_mul(sval);
                let lo_addr = self.reg_addr(d).wrapping_add(2);
                self.write_reg(bus, d, (prod >> 16) as u16);
                self.write_word_at(bus, lo_addr, prod as u16);
            }
            0x3C00 => {
                // DIV src,Rd : (Rd:Rd+1) / src -> quotient Rd, remainder Rd+1.
                // If src <= Rd (the high word), the quotient can't fit 16 bits:
                // set OV and leave the registers untouched. As with MPY, the
                // second word lives at the next address (WP+32 for d=15), not
                // the next register number.
                let ea = self.resolve(bus, ts, s, false);
                let divisor = self.read_word_at(bus, ea);
                let hi = self.read_reg(bus, d);
                if divisor <= hi {
                    self.add_cycles(16);
                    self.set_flag(ST_OV, true);
                } else {
                    // Successful divide. Real hardware takes 92–124 cycles
                    // depending on the dividend/divisor bit patterns (the
                    // data-dependent restoring-division loop); we charge a
                    // deliberate flat 92 (the minimum) as an approximation,
                    // matching Classic99's fixed cost. Cycle-exact DIV timing is
                    // not modeled.
                    self.add_cycles(92);
                    self.set_flag(ST_OV, false);
                    let lo_addr = self.reg_addr(d).wrapping_add(2);
                    let lo = self.read_word_at(bus, lo_addr);
                    let dividend = ((hi as u32) << 16) | (lo as u32);
                    let q = dividend / divisor as u32;
                    let r = dividend % divisor as u32;
                    self.write_reg(bus, d, q as u16);
                    self.write_word_at(bus, lo_addr, r as u16);
                }
            }
            _ => self.illegal(insn),
        }
    }

    /// LDCR/STCR — transfer 1..16 CRU bits between a memory operand and the CRU,
    /// LSB first, at bit address `(R12 >> 1) + i`.
    fn exec_cru_multi(&mut self, bus: &mut impl Bus, insn: u16) {
        let is_store = insn & 0xFC00 == 0x3400;
        let mut count = (insn >> 6) & 0xF;
        if count == 0 {
            count = 16; // a count field of 0 means a full 16-bit transfer
        }
        let ts = (insn >> 4) & 3;
        let s = insn & 0xF;
        // 1..=8 bits use a byte operand; 9..=16 use a word operand.
        let byte = count <= 8;
        let ea = self.resolve(bus, ts, s, byte);
        let base = self.read_reg(bus, 12) >> 1;
        // Base cycle cost differs by direction (TMS9900 data manual, verified
        // against Classic99 cpu9900.cpp `op_ldcr`/`op_stcr`):
        //   LDCR : 20 + 2*count.
        //   STCR : fixed by count — C≤7→42, C=8→44, 9≤C≤15→58, C=16→60.
        // `count` is already normalized (a 0 field means 16). Addressing-mode
        // add-ons are charged separately by `resolve` above, and per-access wait
        // states by the memory helpers, so these are the register-operand bases.
        let base_cycles = if is_store {
            match count {
                1..=7 => 42,
                8 => 44,
                9..=15 => 58,
                _ => 60, // C=16 (a count field of 0 was normalized to 16 above)
            }
        } else {
            20 + 2 * count as u32
        };
        self.add_cycles(base_cycles);

        if is_store {
            // Gather `count` bits from the CRU into a value (LSB first).
            let mut value: u16 = 0;
            for i in 0..count {
                if bus.read_cru_bit(base.wrapping_add(i)) {
                    value |= 1 << i;
                }
            }
            if byte {
                self.set_lae_byte(value as u8);
                self.set_parity(value as u8);
                self.write_byte_at(bus, ea, value as u8);
            } else {
                self.set_lae(value);
                self.write_word_at(bus, ea, value);
            }
        } else {
            // Send the low `count` bits of the operand to the CRU (LSB first).
            let value = if byte {
                self.read_byte_at(bus, ea) as u16
            } else {
                self.read_word_at(bus, ea)
            };
            for i in 0..count {
                bus.write_cru_bit(base.wrapping_add(i), (value >> i) & 1 == 1);
            }
            if byte {
                self.set_lae_byte(value as u8);
                self.set_parity(value as u8);
            } else {
                self.set_lae(value);
            }
        }
    }

    /// 0x1000–0x1FFF: conditional jumps (PC-relative) and single-bit CRU ops.
    fn exec_1xxx(&mut self, bus: &mut impl Bus, insn: u16) {
        let hi = insn >> 8; // 0x10..0x1F
        let disp = insn as u8 as i8; // signed 8-bit displacement / CRU offset
        if hi <= 0x1C {
            // Conditional jump. The displacement is in *words* relative to the
            // already-advanced PC.
            let take = match hi {
                0x10 => true,                            // JMP
                0x11 => !self.flag(ST_AGT) && !self.flag(ST_EQ), // JLT
                0x12 => !self.flag(ST_LGT) || self.flag(ST_EQ),  // JLE
                0x13 => self.flag(ST_EQ),                // JEQ
                0x14 => self.flag(ST_LGT) || self.flag(ST_EQ),   // JHE
                0x15 => self.flag(ST_AGT),               // JGT
                0x16 => !self.flag(ST_EQ),               // JNE
                0x17 => !self.flag(ST_C),                // JNC
                0x18 => self.flag(ST_C),                 // JOC
                0x19 => !self.flag(ST_OV),               // JNO
                0x1A => !self.flag(ST_LGT) && !self.flag(ST_EQ), // JL
                0x1B => self.flag(ST_LGT) && !self.flag(ST_EQ),  // JH
                0x1C => self.flag(ST_OP),                // JOP
                _ => unreachable!(),
            };
            if take {
                self.add_cycles(10);
                self.pc = self.pc.wrapping_add((disp as i16 as u16).wrapping_mul(2));
            } else {
                self.add_cycles(8);
            }
        } else {
            // Single-bit CRU. Bit address = (R12 >> 1) + signed displacement.
            self.add_cycles(12);
            let addr = (self.read_reg(bus, 12) >> 1).wrapping_add(disp as i16 as u16);
            match hi {
                0x1D => bus.write_cru_bit(addr, true), // SBO
                0x1E => bus.write_cru_bit(addr, false), // SBZ
                0x1F => {
                    // TB: copy the addressed CRU bit into EQ.
                    let b = bus.read_cru_bit(addr);
                    self.set_flag(ST_EQ, b);
                }
                _ => unreachable!(),
            }
        }
    }

    /// 0x0000–0x0FFF: shifts (0x0800–0x0BFF), single-operand (0x0400–0x07FF),
    /// immediate/control (0x0200–0x03FF), and illegal opcodes below that.
    fn exec_0xxx(&mut self, bus: &mut impl Bus, insn: u16) {
        if (0x0800..0x0C00).contains(&insn) {
            self.exec_shift(bus, insn);
        } else if (0x0400..0x0800).contains(&insn) {
            self.exec_single_operand(bus, insn);
        } else if (0x0200..0x0400).contains(&insn) {
            self.exec_immediate_or_control(bus, insn);
        } else {
            self.illegal(insn);
        }
    }

    /// Format V shifts: `op count,W`. A count field of 0 takes the count from the
    /// low 4 bits of R0 (and 0 there means 16).
    fn exec_shift(&mut self, bus: &mut impl Bus, insn: u16) {
        let kind = (insn >> 8) & 0xF; // 8=SRA 9=SRL A=SLA B=SRC
        let reg = insn & 0xF;
        let mut count = (insn >> 4) & 0xF;
        // Base cycles: 12 + 2*count for an immediate count. When the count field
        // is 0 the count comes from R0's low nibble (0 there means 16), and that
        // extra R0 fetch costs +8 (Classic99 cpu9900.cpp `op_sra`: an
        // `AddCycleCount(8)` gated on the count-field-zero path), making the
        // count-from-R0 cost 20 + 2*count.
        if count == 0 {
            count = self.read_reg(bus, 0) & 0xF;
            if count == 0 {
                count = 16;
            }
            self.add_cycles(8);
        }
        self.add_cycles(12 + 2 * count as u32);
        let mut val = self.read_reg(bus, reg);
        let mut carry = false;
        let mut overflow = false;
        for _ in 0..count {
            match kind {
                0x8 => {
                    // SRA — arithmetic right (sign-preserving)
                    carry = val & 1 != 0;
                    val = ((val as i16) >> 1) as u16;
                }
                0x9 => {
                    // SRL — logical right (zero fill)
                    carry = val & 1 != 0;
                    val >>= 1;
                }
                0xA => {
                    // SLA — left; OV if the sign bit changes during the shift
                    carry = val & 0x8000 != 0;
                    let nv = val << 1;
                    if (nv ^ val) & 0x8000 != 0 {
                        overflow = true;
                    }
                    val = nv;
                }
                0xB => {
                    // SRC — circular right (rotate)
                    carry = val & 1 != 0;
                    val = (val >> 1) | ((val & 1) << 15);
                }
                _ => unreachable!(),
            }
        }
        self.write_reg(bus, reg, val);
        self.set_lae(val);
        self.set_flag(ST_C, carry);
        if kind == 0xA {
            self.set_flag(ST_OV, overflow);
        }
    }

    /// Format VI single-operand: opcode in bits 15–6, a general source in 5–0.
    fn exec_single_operand(&mut self, bus: &mut impl Bus, insn: u16) {
        let op = insn & 0xFFC0;
        let ts = (insn >> 4) & 3;
        let s = insn & 0xF;
        match op {
            0x0400 => {
                // BLWP @ea : vectored subroutine call with a new workspace.
                self.add_cycles(26);
                let ea = self.resolve(bus, ts, s, false);
                self.context_switch(bus, ea, false, 0);
            }
            0x0440 => {
                // B @ea : branch (PC = ea)
                self.add_cycles(8);
                let ea = self.resolve(bus, ts, s, false);
                self.pc = ea;
            }
            0x0480 => {
                // X src : execute the instruction word found at the source.
                self.add_cycles(8);
                let ea = self.resolve(bus, ts, s, false);
                let sub = self.read_word_at(bus, ea);
                self.execute(bus, sub);
            }
            0x04C0 => {
                // CLR — store 0, no flags
                self.add_cycles(10);
                let ea = self.resolve(bus, ts, s, false);
                self.write_word_at(bus, ea, 0);
            }
            0x0500 => {
                // NEG — arithmetic negate (0 - operand)
                self.add_cycles(12);
                let ea = self.resolve(bus, ts, s, false);
                let v = self.read_word_at(bus, ea);
                let r = self.alu_sub(0, v);
                self.write_word_at(bus, ea, r);
            }
            0x0540 => {
                // INV — ones complement
                self.add_cycles(10);
                let ea = self.resolve(bus, ts, s, false);
                let r = !self.read_word_at(bus, ea);
                self.set_lae(r);
                self.write_word_at(bus, ea, r);
            }
            0x0580 => self.unary_addsub(bus, ts, s, 1, true),  // INC
            0x05C0 => self.unary_addsub(bus, ts, s, 2, true),  // INCT
            0x0600 => self.unary_addsub(bus, ts, s, 1, false), // DEC
            0x0640 => self.unary_addsub(bus, ts, s, 2, false), // DECT
            0x0680 => {
                // BL — branch and link: R11 = return address, PC = ea
                self.add_cycles(12);
                let ea = self.resolve(bus, ts, s, false);
                let ret = self.pc;
                self.write_reg(bus, 11, ret);
                self.pc = ea;
            }
            0x06C0 => {
                // SWPB — swap bytes, no flags
                self.add_cycles(10);
                let ea = self.resolve(bus, ts, s, false);
                let v = self.read_word_at(bus, ea);
                self.write_word_at(bus, ea, v.rotate_left(8));
            }
            0x0700 => {
                // SETO — store all ones, no flags
                self.add_cycles(10);
                let ea = self.resolve(bus, ts, s, false);
                self.write_word_at(bus, ea, 0xFFFF);
            }
            0x0740 => {
                // ABS — absolute value. Compare flags reflect the ORIGINAL
                // operand; OV set iff operand = >8000; carry is cleared.
                let ea = self.resolve(bus, ts, s, false);
                let v = self.read_word_at(bus, ea);
                self.set_lae(v);
                self.set_flag(ST_OV, v == 0x8000);
                self.set_flag(ST_C, false);
                if v & 0x8000 != 0 {
                    self.add_cycles(14); // +2 when the operand is negative
                    let r = (!v).wrapping_add(1);
                    self.write_word_at(bus, ea, r);
                } else {
                    self.add_cycles(12);
                }
            }
            _ => self.illegal(insn),
        }
    }

    fn unary_addsub(&mut self, bus: &mut impl Bus, ts: u16, s: u16, by: u16, add: bool) {
        self.add_cycles(10);
        let ea = self.resolve(bus, ts, s, false);
        let v = self.read_word_at(bus, ea);
        let r = if add {
            self.alu_add(v, by)
        } else {
            self.alu_sub(v, by)
        };
        self.write_word_at(bus, ea, r);
    }

    /// 0x0200–0x03FF: immediate-operand instructions and the no-operand control
    /// instructions.
    fn exec_immediate_or_control(&mut self, bus: &mut impl Bus, insn: u16) {
        let family = insn & 0xFFE0;
        let reg = insn & 0xF;
        match family {
            0x0200 => {
                // LI Rn,imm
                self.add_cycles(12);
                let imm = self.fetch(bus);
                self.write_reg(bus, reg, imm);
                self.set_lae(imm);
            }
            0x0220 => {
                // AI Rn,imm
                self.add_cycles(14);
                let imm = self.fetch(bus);
                let v = self.read_reg(bus, reg);
                let r = self.alu_add(v, imm);
                self.write_reg(bus, reg, r);
            }
            0x0240 => {
                // ANDI Rn,imm
                self.add_cycles(14);
                let imm = self.fetch(bus);
                let r = self.read_reg(bus, reg) & imm;
                self.set_lae(r);
                self.write_reg(bus, reg, r);
            }
            0x0260 => {
                // ORI Rn,imm
                self.add_cycles(14);
                let imm = self.fetch(bus);
                let r = self.read_reg(bus, reg) | imm;
                self.set_lae(r);
                self.write_reg(bus, reg, r);
            }
            0x0280 => {
                // CI Rn,imm — compare, no store
                self.add_cycles(14);
                let imm = self.fetch(bus);
                let v = self.read_reg(bus, reg);
                self.set_compare(v, imm);
            }
            0x02A0 => {
                // STWP Rn — store Workspace Pointer
                self.add_cycles(8);
                let wp = self.wp;
                self.write_reg(bus, reg, wp);
            }
            0x02C0 => {
                // STST Rn — store Status register
                self.add_cycles(8);
                let st = self.st;
                self.write_reg(bus, reg, st);
            }
            0x02E0 => {
                // LWPI imm — load Workspace Pointer immediate
                self.add_cycles(10);
                self.wp = self.fetch(bus);
            }
            0x0300 => {
                // LIMI imm — load interrupt mask (low 4 bits of the immediate)
                self.add_cycles(16);
                let imm = self.fetch(bus);
                self.st = (self.st & !ST_MASK) | (imm & ST_MASK);
            }
            0x0340 => {
                // IDLE — halt until an interrupt is accepted
                self.add_cycles(12);
                self.idle = true;
            }
            0x0360 => {
                // RSET — reset I/O; mask cleared. (External device reset is a
                // no-op here.)
                self.add_cycles(12);
                self.st &= !ST_MASK;
            }
            0x0380 => {
                // RTWP — return: WP/PC/ST <- R13/R14/R15
                self.add_cycles(14);
                let r13 = self.read_reg(bus, 13);
                let r14 = self.read_reg(bus, 14);
                let r15 = self.read_reg(bus, 15);
                self.wp = r13;
                self.pc = r14;
                self.st = r15;
            }
            // CKON/CKOF/LREX (0x03A0/0x03C0/0x03E0): external clock & restart
            // lines, unused on the bare console — treated as no-ops.
            0x03A0 | 0x03C0 | 0x03E0 => {
                self.add_cycles(12);
            }
            _ => self.illegal(insn),
        }
    }

    // =====================================================================
    // Context switches (BLWP / interrupt / XOP) and traps
    // =====================================================================

    /// Perform a BLWP-style context switch through the vector at `ea`
    /// (`ea`→new WP, `ea+2`→new PC), saving the caller's WP/PC/ST into the new
    /// workspace's R13/R14/R15. When `set_mask` is true the interrupt mask is set
    /// to `new_mask` (used by interrupt acceptance); BLWP itself leaves the mask
    /// alone.
    fn context_switch(&mut self, bus: &mut impl Bus, ea: u16, set_mask: bool, new_mask: u16) {
        let new_wp = self.read_word_at(bus, ea);
        let new_pc = self.read_word_at(bus, ea.wrapping_add(2));
        let old_wp = self.wp;
        let old_pc = self.pc;
        let old_st = self.st;
        self.wp = new_wp;
        self.write_reg(bus, 13, old_wp);
        self.write_reg(bus, 14, old_pc);
        self.write_reg(bus, 15, old_st);
        self.pc = new_pc;
        if set_mask {
            self.st = (self.st & !ST_MASK) | (new_mask & ST_MASK);
        }
    }

    /// Accept an interrupt of `level`: vector through `>4*level`, save context,
    /// and lower the mask to `level - 1` so only higher-priority interrupts nest.
    fn accept_interrupt(&mut self, bus: &mut impl Bus, level: u8) {
        self.idle = false;
        let vector = (level as u16) * 4;
        let new_mask = (level as u16).saturating_sub(1);
        self.add_cycles(22);
        self.context_switch(bus, vector, true, new_mask);
    }

    /// XOP src,n — software interrupt: vector through `>0040 + 4n`, save context,
    /// put the source operand's effective address in the new R11, and set ST's X
    /// bit.
    fn do_xop(&mut self, bus: &mut impl Bus, src_ea: u16, n: u16) {
        let vector = 0x0040u16.wrapping_add(n.wrapping_mul(4));
        self.context_switch(bus, vector, false, 0);
        self.write_reg(bus, 11, src_ea);
        self.st |= ST_X;
    }

    /// An undefined opcode. On real silicon the effect is unpredictable; we leave
    /// state unchanged and charge a nominal cost. (The console firmware never
    /// executes these; cartridges that do are buggy.)
    fn illegal(&mut self, _insn: u16) {
        self.illegal_ops += 1;
        self.add_cycles(6);
    }
}
