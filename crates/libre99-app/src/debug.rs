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

//! A live CPU **inspector overlay** — a lightweight nod to Classic99's debugger.
//!
//! Unlike the modal menu/help cards, this panel is **non-modal**: it draws over a
//! corner of the picture while the machine keeps running and receiving input, so
//! you can watch the TMS9900 in real time. It is strictly read-only and uses the
//! core's existing diagnostic API ([`Machine::cpu`], [`Machine::reg`],
//! [`Tms9900Bus::peek_word`]), so it needs nothing new from `libre99-core`.

use libre99_core::machine::Machine;

use crate::text::{self, Canvas};

/// Width/height of the inspector panel in framebuffer pixels.
const PANEL_W: usize = 116;
const PANEL_H: usize = 122;

/// Draw the inspector panel (top-left) over the framebuffer.
pub fn render(canvas: &mut Canvas, machine: &Machine) {
    canvas.dim_rect(0, 0, PANEL_W, PANEL_H, 3);

    let cpu = machine.cpu();
    let pc = cpu.pc();
    let header = 0x0033_CCFF;
    let value = 0x0066_FF66;

    let mut y = 2;
    let mut line = |canvas: &mut Canvas, text: &str, color: u32| {
        canvas.draw_text(3, y, text, color, 1);
        y += text::LINE;
    };

    line(canvas, "-- TMS9900 --", header);
    line(canvas, &format!("PC {:04X}  WP {:04X}", pc, cpu.wp()), value);
    line(canvas, &format!("ST {:04X}", cpu.st()), value);
    line(canvas, &format!("CYC {:08X}", cpu.cycles() as u32), value);

    // Workspace registers R0–R15, two per row.
    for r in 0..8u16 {
        let lo = machine.reg(r);
        let hi = machine.reg(r + 8);
        line(
            canvas,
            &format!("R{r:>2}:{lo:04X}  R{:>2}:{hi:04X}", r + 8),
            0x00AA_C8FF,
        );
    }

    // A few words of memory at PC (RAM/ROM peek; device ports read as 0).
    let bus = machine.bus();
    let words: String = (0..4)
        .map(|i| format!("{:04X} ", bus.peek_word(pc.wrapping_add(i * 2))))
        .collect();
    line(canvas, "MEM @PC", header);
    line(canvas, words.trim_end(), value);
}
