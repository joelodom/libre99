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

//! `libre99gpl` — command-line front end for the GPL toolchain.
//!
//! ```text
//!   libre99gpl asm <src.gpl> <out.bin>   assemble GPL source to a 24 KiB GROM image
//!   libre99gpl dis <grom.bin> [addr]     disassemble from a GROM address (default >0020)
//!   libre99gpl console <out.bin>         build the rewritten console GROM (with font)
//! ```

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("asm") if args.len() == 3 => cmd_asm(&args[1], &args[2]),
        Some("dis") if args.len() >= 2 => cmd_dis(&args[1], args.get(2)),
        Some("console") if args.len() == 2 => cmd_console(&args[1]),
        _ => {
            eprintln!(
                "usage:\n  libre99gpl asm <src.gpl> <out.bin>\n  libre99gpl dis <grom.bin> [hexaddr]\n  libre99gpl console <out.bin>"
            );
            ExitCode::FAILURE
        }
    }
}

fn cmd_console(out_path: &str) -> ExitCode {
    match libre99_gpl::system_grom::build_console_grom() {
        Ok(img) => {
            if let Err(e) = std::fs::write(out_path, &img) {
                eprintln!("cannot write {out_path}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("wrote {out_path} ({} bytes)", img.len());
            ExitCode::SUCCESS
        }
        Err(diags) => {
            for d in diags {
                eprintln!("{d}");
            }
            ExitCode::FAILURE
        }
    }
}

fn cmd_asm(src_path: &str, out_path: &str) -> ExitCode {
    let src = match std::fs::read_to_string(src_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {src_path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match libre99_gpl::assemble(&src) {
        Ok(a) => {
            if let Err(e) = std::fs::write(out_path, &a.image) {
                eprintln!("cannot write {out_path}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("wrote {} ({} bytes), {} symbols", out_path, a.image.len(), a.symbols.len());
            ExitCode::SUCCESS
        }
        Err(diags) => {
            for d in diags {
                eprintln!("{d}");
            }
            ExitCode::FAILURE
        }
    }
}

fn cmd_dis(path: &str, addr: Option<&String>) -> ExitCode {
    let img = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let start = addr
        .and_then(|s| u16::from_str_radix(s.trim_start_matches(">").trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x0020);
    let (listing, tiled) = libre99_gpl::disasm::linear(&img, start as usize, start, 200);
    print!("{listing}");
    eprintln!("; tiled {tiled} bytes from >{start:04X}");
    ExitCode::SUCCESS
}
