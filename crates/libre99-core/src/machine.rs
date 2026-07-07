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

//! # The assembled TI-99/4A
//!
//! This module wires the chips together into a working machine. It contains two
//! things:
//!
//! * [`Tms9900Bus`] — the console's memory map and CRU routing. It owns the VDP,
//!   GROM, keyboard, and 9901, plus the console ROM and all of the RAM, and
//!   implements the [`Bus`] trait the CPU drives.
//! * [`Machine`] — the CPU plus the bus, with a frame loop ([`Machine::run_frame`])
//!   that walks the beam through one 1/60-second frame: each scanline is
//!   rendered (or its sprite flags evaluated) at the moment the beam crosses
//!   it, interleaved with that line's share of CPU cycles, and the VDP's
//!   vertical-blank interrupt rises at the end of active display. The
//!   framebuffer, audio, and keyboard are exposed to the frontend.
//!
//! ## The 8-bit multiplexer and word access
//!
//! On the real console only the system ROM and the 256-byte scratchpad RAM sit on
//! the CPU's native 16-bit bus; everything else is reached through a multiplexer
//! that performs two 8-bit transfers per 16-bit word. We model this faithfully by
//! making **byte** access the primitive ([`Tms9900Bus::read_byte`]/
//! [`write_byte`](Tms9900Bus::write_byte)) and composing word access from two byte
//! transfers. For ordinary RAM both transfers land, so a word access moves two
//! bytes. The device ports (VDP, GROM, sound) instead hang off the **high** byte
//! of the multiplexed bus and latch only the even-address transfer; the odd half
//! of a word access is discarded with no side effect. That is why a two-byte port
//! sequence (e.g. the VDP's low-then-high address setup) must be driven with
//! **byte** instructions, one transfer each — a distinction real TI software
//! relies on.

use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::cru::Tms9901;
use crate::disk::Disk;
use crate::grom::Grom;
use crate::keyboard::{Keyboard, TiKey};
use crate::psg::Psg;
use crate::state::{StateError, StateReader, StateWriter};
use crate::vdp::{Vdp, HEIGHT};

/// Clock cycles per emulated video frame: 3.0 MHz / ~60 Hz. With wait states
/// charged per access, fewer instructions actually execute per frame than this
/// implies — which is the correct, slower-than-clock behavior of the real bus.
pub const CYCLES_PER_FRAME: u64 = 50_000;

/// Scanlines per NTSC frame, matching the TMS9918A (and Classic99's
/// hardware-verified model): 262 lines of ~190.84 CPU cycles each. Lines
/// `0..192` are active display, the frame flag rises at line 192, and lines
/// `192..262` are vertical blanking — 70 lines (~13,360 cycles) for software
/// to update VRAM invisibly before the beam returns to line 0. (Classic99
/// numbers the same circle with a 27-line top border — active 27–218, flag at
/// 219; since the 9918A exposes no beam position to software, only the flag
/// period and the 70-line flag-to-active gap are observable, and both match.)
pub const LINES_PER_FRAME: u64 = 262;

/// Magic bytes at the head of every save-state file (`"TI99SAVE"`).
const SAVE_MAGIC: [u8; 8] = *b"TI99SAVE";
/// Save-state format version. Bump this when the layout changes incompatibly.
const SAVE_VERSION: u32 = 1;

/// The console memory map and CRU routing — the [`Bus`] the CPU talks to.
pub struct Tms9900Bus {
    /// Console system ROM at `>0000–1FFF` (read-only, fast 16-bit bus).
    rom: Box<[u8; 0x2000]>,
    /// 256-byte scratchpad RAM, mirrored through `>8000–83FF` (fast 16-bit bus;
    /// CPU workspaces normally live here).
    scratchpad: Box<[u8; 0x100]>,
    /// 32K expansion RAM, low part `>2000–3FFF` (8-bit bus).
    low_ram: Box<[u8; 0x2000]>,
    /// 32K expansion RAM, high part `>A000–FFFF` (8-bit bus).
    high_ram: Box<[u8; 0x6000]>,
    /// Cartridge ROM occupying the `>6000–7FFF` window (empty = no ROM cart).
    /// Bank-switched cartridges store consecutive 8 KiB banks here.
    cart_rom: Vec<u8>,
    /// Selected cartridge ROM bank (for bank-switched cartridges).
    cart_bank: usize,
    /// Number of 8 KiB banks in `cart_rom` (1 if not bank-switched, 0 if none).
    cart_banks: usize,
    /// The video chip (owns its own 16 KiB VRAM).
    pub vdp: Vdp,
    /// The GROM array (console GROMs at `>0000`, cartridge GROMs at `>6000`+).
    pub grom: Grom,
    /// The keyboard switch matrix.
    pub keyboard: Keyboard,
    /// The 9901 interface (keyboard scan select, interrupt mask, timer mode).
    pub tms9901: Tms9901,
    /// The TI Disk Controller card (FD1771 + DSR ROM at `>4000`, CRU `>1100`).
    pub disk: Disk,
    /// The SN76489 sound chip (write-only port at `>8400`).
    pub psg: Psg,
}

impl Tms9900Bus {
    /// Build the bus with the console ROM and GROM images installed.
    pub fn new(console_rom: &[u8], console_grom: &[u8]) -> Self {
        let mut rom = Box::new([0u8; 0x2000]);
        let n = console_rom.len().min(0x2000);
        rom[..n].copy_from_slice(&console_rom[..n]);

        let mut grom = Grom::new();
        grom.load(0x0000, console_grom);

        Tms9900Bus {
            rom,
            scratchpad: Box::new([0u8; 0x100]),
            low_ram: Box::new([0u8; 0x2000]),
            high_ram: Box::new([0u8; 0x6000]),
            cart_rom: Vec::new(),
            cart_bank: 0,
            cart_banks: 0,
            vdp: Vdp::new(),
            grom,
            keyboard: Keyboard::new(),
            tms9901: Tms9901::new(),
            disk: Disk::new(),
            psg: Psg::default(),
        }
    }

    /// Install cartridge ROM banks into the `>6000–7FFF` window. `banks` is the
    /// number of 8 KiB banks present (1 for a plain ROM cartridge).
    pub fn load_cartridge_rom(&mut self, rom: Vec<u8>, banks: usize) {
        self.cart_rom = rom;
        self.cart_banks = banks;
        self.cart_bank = 0;
    }

    /// Should the CPU take its level-1 interrupt now? (The 9901 gates the VDP's
    /// vertical-blank interrupt with its enable mask.)
    pub fn interrupt_line(&self) -> Option<u8> {
        if self.tms9901.pending_interrupt(self.vdp.interrupt_pending()) {
            Some(1)
        } else {
            None
        }
    }

    /// Decode and read one byte from the console address space. This is the bus
    /// primitive; word reads are two of these.
    fn read_byte_decode(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.rom[addr as usize],
            0x2000..=0x3FFF => self.low_ram[(addr - 0x2000) as usize],
            // DSR ROM window + FD1771 registers (the disk card, gated by CRU
            // >1100 bit 0). Reads as open bus until a controller is installed.
            0x4000..=0x5FFF => self.disk.read_byte(addr),
            0x6000..=0x7FFF => self.read_cartridge(addr),
            0x8000..=0x83FF => self.scratchpad[(addr & 0xFF) as usize],
            // Sound chip is write-only; reads return open bus.
            0x8400..=0x87FF => 0xFF,
            // VDP read ports: bit 1 selects data (>8800) vs. status (>8802). As
            // on the write side, the chip answers only at the even address; the
            // odd half of a *word* read is open bus with no side effects, so a
            // word read does not auto-increment the VRAM address twice. Matches
            // the real chip — Classic99 `rvdpbyte` returns 0 where `x & 1`.
            0x8800..=0x8BFF => {
                if addr & 1 != 0 {
                    0xFF
                } else if addr & 2 == 0 {
                    self.vdp.read_data()
                } else {
                    self.vdp.read_status()
                }
            }
            // VDP write ports — reads are open bus.
            0x8C00..=0x8FFF => 0xFF,
            // Speech (not emulated).
            0x9000..=0x97FF => 0x00,
            // GROM read ports: bit 1 selects data (>9800) vs. address (>9802).
            // Like the VDP, the GROM hangs off the high byte of the multiplexed
            // bus and answers only at the even address; the odd half of a *word*
            // read is open bus with no side effects, so a word read does not
            // auto-increment the GROM address counter twice. Matches the real chip
            // — Classic99 `rgrmbyte` returns on `x & 1`.
            0x9800..=0x9BFF => {
                if addr & 1 != 0 {
                    0xFF
                } else if addr & 2 == 0 {
                    self.grom.read_data()
                } else {
                    self.grom.read_address()
                }
            }
            // GROM write ports — reads are open bus.
            0x9C00..=0x9FFF => 0xFF,
            0xA000..=0xFFFF => self.high_ram[(addr - 0xA000) as usize],
        }
    }

    /// Decode and write one byte to the console address space.
    fn write_byte_decode(&mut self, addr: u16, value: u8) {
        match addr {
            // Console ROM is read-only.
            0x0000..=0x1FFF => {}
            0x2000..=0x3FFF => self.low_ram[(addr - 0x2000) as usize] = value,
            0x4000..=0x5FFF => self.disk.write_byte(addr, value),
            0x6000..=0x7FFF => self.write_cartridge(addr, value),
            0x8000..=0x83FF => self.scratchpad[(addr & 0xFF) as usize] = value,
            // SN76489 sound chip (write-only). Like the VDP/GROM it hangs off the
            // high byte of the multiplexed bus and latches only at the even
            // address; the odd half of a *word* write is ignored, so a word write
            // does not strobe the chip twice. Matches the real chip — Classic99
            // `wsndbyte` discards writes where `x & 1`.
            0x8400..=0x87FF => {
                if addr & 1 == 0 {
                    self.psg.write(value);
                }
            }
            // VDP read ports — writes ignored.
            0x8800..=0x8BFF => {}
            // VDP write ports: bit 1 selects data (>8C00) vs. control (>8C02).
            // The 9918A hangs off the high byte of the data bus, so it latches
            // only at the even address; the odd half of a *word* access is
            // ignored (no second latch, no second auto-increment). Matches the
            // real chip — Classic99 `wvdpbyte` discards writes where `x & 1`.
            // Without this, a word write such as the disk DSR's `CLR @>8C00`
            // VRAM-clear loop would write twice per iteration, run off the end of
            // VRAM, and wrap zeros back over the title screen.
            0x8C00..=0x8FFF => {
                if addr & 1 != 0 {
                    // odd byte of a word access — no second VDP latch
                } else if addr & 2 == 0 {
                    self.vdp.write_data(value);
                } else {
                    self.vdp.write_control(value);
                }
            }
            0x9000..=0x97FF => {} // speech
            // GROM read ports — writes ignored.
            0x9800..=0x9BFF => {}
            // GROM write ports: bit 1 selects data (>9C00) vs. address (>9C02).
            // The GROM answers only at the even address; the odd half of a *word*
            // write is ignored (no second data latch, no second address-shift), so
            // software loads a GROM address with two *byte* writes. Matches the
            // real chip — Classic99 `wgrmbyte` discards writes where `x & 1`.
            0x9C00..=0x9FFF => {
                if addr & 1 != 0 {
                    // odd byte of a word access — no second GROM latch
                } else if addr & 2 == 0 {
                    self.grom.write_data(value);
                } else {
                    self.grom.write_address(value);
                }
            }
            0xA000..=0xFFFF => self.high_ram[(addr - 0xA000) as usize] = value,
        }
    }

    /// Read the cartridge ROM window `>6000–7FFF`, honoring the selected bank.
    fn read_cartridge(&self, addr: u16) -> u8 {
        if self.cart_banks == 0 {
            return 0; // no ROM cartridge (e.g. Tunnels of Doom is GROM-only)
        }
        let offset = (addr - 0x6000) as usize;
        let base = self.cart_bank * 0x2000;
        self.cart_rom.get(base + offset).copied().unwrap_or(0)
    }

    /// A write into the cartridge ROM window selects a ROM bank: the TI scheme
    /// derives the bank from the address (`(addr >> 1)`), masked to the number of
    /// banks. Writes never change ROM contents.
    fn write_cartridge(&mut self, addr: u16, _value: u8) {
        if self.cart_banks > 1 {
            self.cart_bank = ((addr >> 1) as usize) & (self.cart_banks - 1);
        }
    }
}

impl Bus for Tms9900Bus {
    // Word access = two byte transfers (matching the 8-bit multiplexer for device
    // regions and equivalent to a native word for RAM/ROM).
    fn read_word(&mut self, addr: u16) -> u16 {
        let a = addr & 0xFFFE;
        ((self.read_byte_decode(a) as u16) << 8) | (self.read_byte_decode(a | 1) as u16)
    }
    fn write_word(&mut self, addr: u16, value: u16) {
        let a = addr & 0xFFFE;
        self.write_byte_decode(a, (value >> 8) as u8);
        self.write_byte_decode(a | 1, value as u8);
    }
    fn read_byte(&mut self, addr: u16) -> u8 {
        self.read_byte_decode(addr)
    }
    fn write_byte(&mut self, addr: u16, value: u8) {
        self.write_byte_decode(addr, value);
    }

    fn read_cru_bit(&mut self, bit_addr: u16) -> bool {
        // The hardware decodes 12 CRU address lines; higher software bit
        // addresses alias back into the 4096-bit space (as FlatRam models).
        let bit_addr = bit_addr & 0x0FFF;
        match bit_addr {
            // TMS9901 (keyboard, interrupts, timer) at CRU base 0.
            0x0000..=0x001F => {
                let vdp_int = self.vdp.interrupt_pending();
                self.tms9901.read_bit(bit_addr, &self.keyboard, vdp_int)
            }
            // TI Disk Controller card at CRU base >1100 (bit address >0880).
            0x0880..=0x088F => self.disk.read_cru(bit_addr - 0x0880),
            // Unwired cards idle high.
            _ => true,
        }
    }
    fn write_cru_bit(&mut self, bit_addr: u16, value: bool) {
        let bit_addr = bit_addr & 0x0FFF;
        match bit_addr {
            0x0000..=0x001F => self.tms9901.write_bit(bit_addr, value),
            0x0880..=0x088F => self.disk.write_cru(bit_addr - 0x0880, value),
            _ => {}
        }
    }

    fn wait_states(&self, addr: u16) -> u32 {
        match addr {
            // Console ROM and scratchpad are on the fast 16-bit bus.
            0x0000..=0x1FFF | 0x8000..=0x83FF => 0,
            // Everything else goes through the 8-bit multiplexer (~4 cycles).
            _ => 4,
        }
    }

    /// GROM accesses stall the CPU far beyond the multiplexer's 4 cycles: the
    /// chip is a slow serial device the console waits on. The stalls below are
    /// Classic99's hardware-measured values (`Tiemul.cpp`, "verified"), charged
    /// **in addition to** the base multiplexer wait — exactly as Classic99
    /// stacks them — and only when the chip actually responds (even addresses;
    /// the odd half of a word access is open bus). This is why GPL — the whole
    /// OS — runs so much slower than CPU-RAM code on real hardware.
    fn wait_states_rw(&self, addr: u16, is_write: bool) -> u32 {
        let base = self.wait_states(addr);
        if addr & 1 != 0 {
            return base;
        }
        match (addr & 0xFC00, is_write, addr & 2 != 0) {
            // >9800 read data / >9802 read address.
            (0x9800, false, false) => base + 19,
            (0x9800, false, true) => base + 13,
            // >9C00 write data.
            (0x9C00, true, false) => base + 22,
            // >9C02 write address: the second (low) byte completes the address
            // and triggers the prefetch, so it stalls longer than the first.
            (0x9C00, true, true) => {
                base + if self.grom.expecting_low_address_byte() {
                    21
                } else {
                    15
                }
            }
            _ => base,
        }
    }
}

/// A whole TI-99/4A: CPU + bus + peripherals, with a frame-paced run loop.
pub struct Machine {
    cpu: Cpu,
    bus: Tms9900Bus,
    /// Diagnostic toggle: when false, interrupts are never delivered (used to
    /// isolate interrupt-related bugs).
    interrupts_enabled: bool,
    /// Is anyone watching the picture? Off until the first
    /// [`render`](Self::render) call; while off, [`run_frame`](Self::run_frame)
    /// skips pixel work (sprite status flags are still evaluated per line, so
    /// nothing observable diverges) — headless runs pay no rasterizing cost.
    /// Not serialized: a restored machine warms the frame on its next render.
    video_live: bool,
}

impl Machine {
    /// Build a machine from the console ROM and GROM images and reset the CPU
    /// (which vectors through `>0000` to `WP=>83E0, PC=>0024`).
    pub fn new(console_rom: &[u8], console_grom: &[u8]) -> Self {
        let mut bus = Tms9900Bus::new(console_rom, console_grom);
        let mut cpu = Cpu::new();
        cpu.reset(&mut bus);
        Machine {
            cpu,
            bus,
            interrupts_enabled: true,
            video_live: false,
        }
    }

    /// Diagnostic: enable/disable interrupt delivery.
    pub fn set_interrupts(&mut self, on: bool) {
        self.interrupts_enabled = on;
    }

    /// Execute one instruction (sampling the interrupt line first) and return the
    /// cycles it took.
    pub fn step(&mut self) -> u32 {
        let int = if self.interrupts_enabled {
            self.bus.interrupt_line()
        } else {
            None
        };
        self.cpu.set_interrupt_request(int);
        self.cpu.step(&mut self.bus)
    }

    /// Advance the machine by one video frame, walking the beam the way the
    /// hardware does: for each of the [`LINES_PER_FRAME`] scanlines, let the
    /// VDP process the line — rasterized from **live** VRAM if a frontend is
    /// watching ([`Vdp::render_line`]), sprite-flag evaluation only if not
    /// ([`Vdp::evaluate_line`]) — then run the CPU for that line's ~190.84
    /// cycles, sampling the interrupt line each instruction. The frame flag
    /// (and, when enabled, the level-1 interrupt) rises when the beam reaches
    /// line 192, the end of active display; the interrupt handler then has
    /// the real 70-line vertical-blanking window to update VRAM before line 0
    /// of the next frame is drawn. Mid-frame VRAM writes therefore appear
    /// exactly where the beam would show them — never retroactively above it.
    ///
    /// The per-line cycle targets are absolute (`start + (n+1)·50,000/262`),
    /// so there is no rounding drift, an instruction overshooting one line
    /// boundary simply shortens the next line's window, and the frame as a
    /// whole consumes exactly [`CYCLES_PER_FRAME`] — byte-identical pacing to
    /// the previous whole-slice loop.
    pub fn run_frame(&mut self) {
        let start = self.cpu.cycles();
        for line in 0..LINES_PER_FRAME {
            if line < HEIGHT as u64 {
                if self.video_live {
                    self.bus.vdp.render_line(line as usize);
                } else {
                    self.bus.vdp.evaluate_line(line as usize);
                }
            } else if line == HEIGHT as u64 {
                // End of active display: raise vertical blank (requests the
                // level-1 interrupt if enabled).
                self.bus.vdp.vblank();
            }
            let target = start + (line + 1) * CYCLES_PER_FRAME / LINES_PER_FRAME;
            while self.cpu.cycles() < target {
                self.step();
            }
        }
        debug_assert!(self.cpu.cycles() >= start + CYCLES_PER_FRAME);
    }

    /// Hand the frontend the current frame (`WIDTH*HEIGHT`, `0x00RRGGBB`).
    ///
    /// The first call renders a whole frame on the spot from the current VRAM
    /// and switches the machine to beam-accurate accumulation: from then on
    /// [`run_frame`](Self::run_frame) rasterizes each scanline at the moment
    /// the beam crosses it, and this method just copies the accumulated
    /// picture out. Machines that never render (headless tests, sweeps) skip
    /// all pixel work.
    pub fn render(&mut self, fb: &mut [u32]) {
        if self.video_live {
            self.bus.vdp.copy_frame(fb);
        } else {
            self.bus.vdp.render(fb);
            self.video_live = true;
        }
    }

    /// Press or release a key.
    pub fn set_key(&mut self, key: TiKey, down: bool) {
        self.bus.keyboard.set_key(key, down);
    }

    /// Mount a parsed cartridge: install its ROM banks into the `>6000–7FFF`
    /// window and load its GROM pages into the GROM address space. Mount before
    /// the first run, or follow with [`reset`](Self::reset) to restart the
    /// console with the new cartridge present (a "warm reset").
    pub fn mount_cartridge(&mut self, cart: &Cartridge) {
        // Replace, never layer: unmap the previous cartridge first, so a
        // GROM-only cartridge mounted after a ROM cartridge doesn't keep the
        // old ROM banks (or stale GROM pages) visible.
        self.bus.load_cartridge_rom(cart.rom.clone(), cart.rom_banks);
        self.bus.grom.clear_cartridge_space();
        for (addr, page) in &cart.grom {
            self.bus.grom.load(*addr, page);
        }
    }

    /// Re-run the CPU reset sequence (vectors through `>0000` to `WP=>83E0,
    /// PC=>0024`) — used to restart the console after changing media.
    pub fn reset(&mut self) {
        self.cpu.reset(&mut self.bus);
    }

    /// Install the TI Disk Controller's DSR ROM (`Disk.Bin`). This makes the
    /// `>4000–5FFF` DSR window and the FD1771 registers live; the console's
    /// `DSRLNK` then finds the card and drives it.
    pub fn load_disk_controller(&mut self, dsr_rom: &[u8]) {
        self.bus.disk.load_dsr(dsr_rom);
    }

    /// Insert a raw sector-dump disk image into drive `drive` (0 = DSK1).
    pub fn mount_disk(&mut self, drive: usize, image: Vec<u8>) {
        self.bus.disk.mount(drive, image);
    }

    /// Set the host audio sample rate the SN76489 synthesises at.
    pub fn set_audio_sample_rate(&mut self, rate: u32) {
        self.bus.psg.set_sample_rate(rate);
    }

    /// Pull `buffer.len()` mono audio samples (each in `[-1.0, 1.0]`) from the
    /// sound chip's current state. The frontend calls this once per frame.
    pub fn fill_audio(&mut self, buffer: &mut [f32]) {
        self.bus.psg.fill(buffer);
    }

    /// Read-only access to the CPU (diagnostics/tests).
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
    /// Read-only access to the VDP (diagnostics/tests).
    pub fn vdp(&self) -> &Vdp {
        &self.bus.vdp
    }
    /// Mutable access to the VDP (diagnostics/tests — e.g. planting VRAM tables).
    pub fn vdp_mut(&mut self) -> &mut Vdp {
        &mut self.bus.vdp
    }
    /// Mutable access to the bus (for mounting cartridges/disks).
    pub fn bus_mut(&mut self) -> &mut Tms9900Bus {
        &mut self.bus
    }

    /// Read-only access to the bus (diagnostics).
    pub fn bus(&self) -> &Tms9900Bus {
        &self.bus
    }

    /// Read workspace register `n` without side effects (diagnostics).
    pub fn reg(&self, n: u16) -> u16 {
        self.bus.peek_word(self.cpu.wp().wrapping_add(n << 1))
    }

    /// Serialize the entire machine — CPU, all RAM, VRAM, the GROM image, the
    /// cartridge ROM, the mounted disk images (with any written-back sectors),
    /// and every chip latch — into a self-contained save-state blob. Restore it
    /// with [`load_state`](Self::load_state).
    ///
    /// Beam state is deliberately not part of the format:
    /// [`run_frame`](Self::run_frame) always completes whole frames, so every
    /// save happens at a frame boundary, and a restored machine repaints its
    /// picture in full on its next [`render`](Self::render).
    pub fn save_state(&self) -> Vec<u8> {
        let mut w = StateWriter::new();
        w.raw(&SAVE_MAGIC);
        w.u32(SAVE_VERSION);
        self.cpu.save_state(&mut w);
        self.bus.save_state(&mut w);
        w.bool(self.interrupts_enabled);
        w.into_bytes()
    }

    /// Replace this machine's entire state with the snapshot in `bytes`.
    ///
    /// On any error — a bad magic number, an unknown version, or a truncated
    /// file — `self` is left **completely untouched**: the snapshot is decoded
    /// into a staging machine that is swapped in only once the whole read
    /// succeeds.
    pub fn load_state(&mut self, bytes: &[u8]) -> Result<(), StateError> {
        let mut r = StateReader::new(bytes);
        let mut magic = [0u8; 8];
        r.fill(&mut magic)?;
        if magic != SAVE_MAGIC {
            return Err(StateError::BadMagic);
        }
        let version = r.u32()?;
        if version != SAVE_VERSION {
            return Err(StateError::UnsupportedVersion(version));
        }
        // Decode into a blank staging machine; the empty ROM/GROM it starts with
        // are immediately overwritten by the snapshot's own (self-contained)
        // images. A mid-stream error therefore cannot corrupt the live machine.
        let mut staged = Machine::new(&[], &[]);
        staged.cpu.load_state(&mut r)?;
        staged.bus.load_state(&mut r)?;
        staged.interrupts_enabled = r.bool()?;
        *self = staged;
        Ok(())
    }
}

impl Tms9900Bus {
    /// Peek a RAM/ROM byte without side effects (diagnostics; device ports read
    /// as 0 here so this never perturbs them).
    pub fn peek(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.rom[addr as usize],
            0x2000..=0x3FFF => self.low_ram[(addr - 0x2000) as usize],
            0x8000..=0x83FF => self.scratchpad[(addr & 0xFF) as usize],
            0xA000..=0xFFFF => self.high_ram[(addr - 0xA000) as usize],
            _ => 0,
        }
    }
    /// Peek a RAM/ROM word without side effects (diagnostics).
    pub fn peek_word(&self, addr: u16) -> u16 {
        ((self.peek(addr & 0xFFFE) as u16) << 8) | self.peek((addr & 0xFFFE) | 1) as u16
    }
    /// Poke a RAM byte (diagnostics/tests). Writes only the writable RAM regions;
    /// ROM and device ports are left untouched, mirroring `peek`.
    pub fn poke(&mut self, addr: u16, value: u8) {
        match addr {
            0x2000..=0x3FFF => self.low_ram[(addr - 0x2000) as usize] = value,
            0x8000..=0x83FF => self.scratchpad[(addr & 0xFF) as usize] = value,
            0xA000..=0xFFFF => self.high_ram[(addr - 0xA000) as usize] = value,
            _ => {}
        }
    }
    /// Poke a RAM word, big-endian (diagnostics/tests).
    pub fn poke_word(&mut self, addr: u16, value: u16) {
        self.poke(addr & 0xFFFE, (value >> 8) as u8);
        self.poke((addr & 0xFFFE) | 1, value as u8);
    }
    /// The GROM address counter (diagnostics).
    pub fn grom_address(&self) -> u16 {
        self.grom.address()
    }
    /// Enable/disable the GROM read log (diagnostics).
    pub fn grom_record(&mut self, on: bool) {
        self.grom.record(on);
    }
    /// The recorded GROM reads (diagnostics).
    pub fn grom_log(&self) -> &[(u16, u8)] {
        &self.grom.log
    }
    /// Enable/disable the GROM read-coverage bitmap (diagnostics — the cartridge
    /// coverage sweep).
    pub fn grom_record_coverage(&mut self, on: bool) {
        self.grom.record_coverage(on);
    }
    /// Whether GROM address `addr`'s byte has been read since coverage was enabled.
    pub fn grom_was_read(&self, addr: u16) -> bool {
        self.grom.was_read(addr)
    }
    /// Every GROM address read since coverage was enabled, ascending.
    pub fn grom_coverage_addresses(&self) -> Vec<u16> {
        self.grom.read_addresses()
    }

    /// Serialize the whole bus — console ROM, all RAM, the cartridge ROM and
    /// selected bank, and every owned chip — into a save state.
    pub(crate) fn save_state(&self, w: &mut StateWriter) {
        w.raw(&self.rom[..]);
        w.raw(&self.scratchpad[..]);
        w.raw(&self.low_ram[..]);
        w.raw(&self.high_ram[..]);
        w.blob(&self.cart_rom);
        w.usize(self.cart_bank);
        w.usize(self.cart_banks);
        self.vdp.save_state(w);
        self.grom.save_state(w);
        self.keyboard.save_state(w);
        self.tms9901.save_state(w);
        self.disk.save_state(w);
        self.psg.save_state(w);
    }

    /// Restore the whole bus from a save state.
    pub(crate) fn load_state(&mut self, r: &mut StateReader<'_>) -> Result<(), StateError> {
        r.fill(&mut self.rom[..])?;
        r.fill(&mut self.scratchpad[..])?;
        r.fill(&mut self.low_ram[..])?;
        r.fill(&mut self.high_ram[..])?;
        self.cart_rom = r.blob()?;
        self.cart_bank = r.usize()?;
        self.cart_banks = r.usize()?;
        self.vdp.load_state(r)?;
        self.grom.load_state(r)?;
        self.keyboard.load_state(r)?;
        self.tms9901.load_state(r)?;
        self.disk.load_state(r)?;
        self.psg.load_state(r)?;
        Ok(())
    }
}
