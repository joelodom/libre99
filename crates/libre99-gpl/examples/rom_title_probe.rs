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

//! Probe: how far does OUR console ROM (`libre99_asm::build_console_rom`) get
//! booting our GROM's title, vs the authentic ROM? Reports the display-enable
//! bit, the banner text, and the loud-stub breadcrumb cell `>837D` (non-zero =
//! our interpreter hit an unimplemented handler). Throwaway M1-gate scouting.

use std::sync::LazyLock;

use libre99_core::machine::Machine;

static AUTH: LazyLock<Vec<u8>> = LazyLock::new(|| {
    libre99_core::third_party::load("roms/994aROM.Bin").unwrap_or_else(|| {
        eprintln!("this probe needs third-party media (third-party/roms/994aROM.Bin)");
        std::process::exit(2)
    })
});

fn main() {
    let grom = libre99_gpl::system_grom::build_console_grom().expect("console GROM");
    let our_rom = libre99_asm::system_rom::build_console_rom().expect("console ROM");

    for (name, rom) in [("authentic", AUTH.as_slice()), ("ours     ", our_rom.as_slice())] {
        let mut m = Machine::new(rom, &grom);
        m.reset();
        m.bus_mut().grom_record(true);
        for _ in 0..40 {
            m.run_frame();
        }
        // The distinct GROM addresses fetched, as a coarse "how far did the GPL
        // get" — and the tail, to locate where our stream stops advancing.
        let log = m.bus().grom_log();
        let last: Vec<String> =
            log.iter().rev().take(12).rev().map(|(a, _)| format!("{a:04X}")).collect();
        let maxaddr = log.iter().map(|(a, _)| *a).filter(|a| *a < 0x6000).max().unwrap_or(0);
        println!(
            "  {name} end PC=>{:04X} grom-fetches={} max-grom=>{maxaddr:04X} tail=[{}]",
            m.cpu().pc(),
            log.len(),
            last.join(" ")
        );
        let base = ((m.vdp().register(2) & 0x0F) as u16) * 0x400;
        let text: String =
            (0..17).map(|i| m.vdp().vram(base + 8 * 32 + 7 + i) as char).collect();
        let press: String =
            (0..28).map(|i| m.vdp().vram(base + 16 * 32 + 2 + i) as char).collect();
        println!(
            "{name}: R1=>{:02X} (display {}) R7=>{:02X} \
             title[8,7]={text:?} press[16,2]={press:?} stub>837D=>{:02X}",
            m.vdp().register(1),
            if m.vdp().register(1) & 0x40 != 0 { "ON " } else { "off" },
            m.vdp().register(7),
            m.bus().peek(0x837D),
        );
    }
}
