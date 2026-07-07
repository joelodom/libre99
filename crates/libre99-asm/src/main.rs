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

//! `libre99asm` — command-line front end for the assembler.
//!
//! ```text
//! libre99asm [OPTIONS] <input.asm>     assemble a cartridge (default)
//! libre99asm rom <out.bin>             build the rewritten console ROM (8 KiB image)
//! libre99asm dsr <out.bin>             build the rewritten disk-controller DSR (8 KiB image)
//! libre99asm dis <file.bin> [addr]     disassemble from an address (default >0000)
//!
//! OPTIONS (cartridge mode):
//!   -o, --output <file>   Output path (default: input with .ctg/.bin extension)
//!       --format <fmt>    ctg | bin           (default: ctg)
//!       --bin             Shorthand for --format bin (raw >6000 image)
//!       --name <title>    Cartridge menu title (default: IDT, then "CART")
//!       --entry <symbol>  Entry-point symbol (default: END operand, then START)
//!       --listing <file>  Also write an address/object/source listing
//!       --symbols <file>  Also write the symbol table as JSON
//!   -h, --help            Print help and exit
//! ```
//!
//! Exit codes: `0` success, `1` assembly errors, `2` usage error, `3` I/O error.

use std::path::Path;
use std::process::ExitCode;

use libre99_asm::{assemble, disk_dsr, expand_includes, system_rom, Options};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("rom") => cmd_rom(&args[1..]),
        Some("dsr") => cmd_dsr(&args[1..]),
        Some("dis") if args.len() >= 2 => cmd_dis(&args[1], args.get(2)),
        _ => cmd_asm(&args),
    }
}

/// `libre99asm rom <out.bin>` — build the console ROM. Optional trailing
/// `--listing <f>` / `--symbols <f>` write those artifacts too.
fn cmd_rom(rest: &[String]) -> ExitCode {
    let mut out = None;
    let mut listing = None;
    let mut symbols = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--listing" => listing = it.next().cloned(),
            "--symbols" => symbols = it.next().cloned(),
            s if s.starts_with('-') => {
                eprintln!("libre99asm rom: unknown option `{s}`");
                return ExitCode::from(2);
            }
            s => out = Some(s.to_string()),
        }
    }
    let Some(out) = out else {
        eprintln!("usage: libre99asm rom <out.bin> [--listing <f>] [--symbols <f>]");
        return ExitCode::from(2);
    };

    let asm = match system_rom::assemble_console_rom() {
        Ok(a) => a,
        Err(diags) => {
            for d in &diags {
                eprintln!("console.asm:{d}");
            }
            return ExitCode::from(1);
        }
    };
    if let Err(e) = std::fs::write(&out, &asm.rom) {
        eprintln!("libre99asm: error: writing {out}: {e}");
        return ExitCode::from(3);
    }
    if let Some(path) = listing {
        if let Err(e) = std::fs::write(&path, asm.listing()) {
            eprintln!("libre99asm: error: writing {path}: {e}");
            return ExitCode::from(3);
        }
    }
    if let Some(path) = symbols {
        if let Err(e) = std::fs::write(&path, asm.symbols_json()) {
            eprintln!("libre99asm: error: writing {path}: {e}");
            return ExitCode::from(3);
        }
    }
    eprintln!("libre99asm: wrote {out} ({} bytes, {} symbols)", asm.rom.len(), asm.symbols.len());
    ExitCode::SUCCESS
}

/// `libre99asm dsr <out.bin>` — build the disk-controller DSR (8 KiB `>4000`
/// image). Optional trailing `--listing <f>` / `--symbols <f>` write those too.
fn cmd_dsr(rest: &[String]) -> ExitCode {
    let mut out = None;
    let mut listing = None;
    let mut symbols = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--listing" => listing = it.next().cloned(),
            "--symbols" => symbols = it.next().cloned(),
            s if s.starts_with('-') => {
                eprintln!("libre99asm dsr: unknown option `{s}`");
                return ExitCode::from(2);
            }
            s => out = Some(s.to_string()),
        }
    }
    let Some(out) = out else {
        eprintln!("usage: libre99asm dsr <out.bin> [--listing <f>] [--symbols <f>]");
        return ExitCode::from(2);
    };

    let asm = match disk_dsr::assemble_disk_dsr() {
        Ok(a) => a,
        Err(diags) => {
            for d in &diags {
                eprintln!("disk-dsr.asm:{d}");
            }
            return ExitCode::from(1);
        }
    };
    if let Err(e) = std::fs::write(&out, &asm.rom) {
        eprintln!("libre99asm: error: writing {out}: {e}");
        return ExitCode::from(3);
    }
    if let Some(path) = listing {
        if let Err(e) = std::fs::write(&path, asm.listing()) {
            eprintln!("libre99asm: error: writing {path}: {e}");
            return ExitCode::from(3);
        }
    }
    if let Some(path) = symbols {
        if let Err(e) = std::fs::write(&path, asm.symbols_json()) {
            eprintln!("libre99asm: error: writing {path}: {e}");
            return ExitCode::from(3);
        }
    }
    eprintln!("libre99asm: wrote {out} ({} bytes, {} symbols)", asm.rom.len(), asm.symbols.len());
    ExitCode::SUCCESS
}

/// `libre99asm dis <file.bin> [addr]` — disassemble a raw image.
fn cmd_dis(path: &str, addr: Option<&String>) -> ExitCode {
    let img = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("libre99asm: error: reading {path}: {e}");
            return ExitCode::from(3);
        }
    };
    let start = addr
        .and_then(|s| u16::from_str_radix(s.trim_start_matches('>').trim_start_matches("0x"), 16).ok())
        .unwrap_or(0x0000);
    let (listing, tiled) = libre99_asm::disasm::linear(&img, start as usize, start, 4096);
    print!("{listing}");
    eprintln!("; tiled {tiled} bytes from >{start:04X}");
    ExitCode::SUCCESS
}

/// The default cartridge-assembly command (`libre99asm [OPTIONS] input.asm`).
fn cmd_asm(args: &[String]) -> ExitCode {
    let mut input = None;
    let mut output = None;
    let mut name = None;
    let mut entry = None;
    let mut format = String::from("ctg");
    let mut listing = None;
    let mut symbols = None;

    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" | "--output" => output = it.next().cloned(),
            "--name" => name = it.next().cloned(),
            "--entry" => entry = it.next().cloned(),
            "--format" => format = it.next().cloned().unwrap_or_default(),
            "--bin" => format = String::from("bin"),
            "--listing" => listing = it.next().cloned(),
            "--symbols" => symbols = it.next().cloned(),
            "-h" | "--help" => {
                print_help();
                return ExitCode::SUCCESS;
            }
            s if s.starts_with('-') => {
                eprintln!("libre99asm: error: unknown option `{s}` (try --help)");
                return ExitCode::from(2);
            }
            s => input = Some(s.to_string()),
        }
    }

    let Some(input) = input else {
        eprintln!("libre99asm: error: no input file (try --help)");
        return ExitCode::from(2);
    };

    let raw = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("libre99asm: error: reading {input}: {e}");
            return ExitCode::from(3);
        }
    };
    // Resolve `COPY '<file>'` includes relative to the input's directory.
    let dir = Path::new(&input).parent().map(Path::to_path_buf).unwrap_or_default();
    let resolve = |p: &str| std::fs::read_to_string(dir.join(p)).map_err(|e| e.to_string());
    let src = match expand_includes(&raw, &resolve) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("libre99asm: error: {e}");
            return ExitCode::from(1);
        }
    };

    let opts = Options { name, entry, ..Default::default() };
    let asm = match assemble(&src, &opts) {
        Ok(a) => a,
        Err(diags) => {
            for d in &diags {
                eprintln!("{input}:{d}");
            }
            return ExitCode::from(1);
        }
    };

    let bytes = match format.as_str() {
        "ctg" => asm.ctg(),
        "bin" => asm.rom.clone(),
        other => {
            eprintln!("libre99asm: error: unknown format `{other}` (use ctg or bin)");
            return ExitCode::from(2);
        }
    };

    let out_path = output.unwrap_or_else(|| default_output(&input, &format));
    if let Err(e) = std::fs::write(&out_path, &bytes) {
        eprintln!("libre99asm: error: writing {out_path}: {e}");
        return ExitCode::from(3);
    }
    if let Some(path) = listing {
        if let Err(e) = std::fs::write(&path, asm.listing()) {
            eprintln!("libre99asm: error: writing {path}: {e}");
            return ExitCode::from(3);
        }
    }
    if let Some(path) = symbols {
        if let Err(e) = std::fs::write(&path, asm.symbols_json()) {
            eprintln!("libre99asm: error: writing {path}: {e}");
            return ExitCode::from(3);
        }
    }

    eprintln!(
        "libre99asm: wrote {out_path} ({} bytes); title {:?}, entry >{:04X}",
        bytes.len(),
        asm.title,
        asm.entry
    );
    ExitCode::SUCCESS
}

fn default_output(input: &str, format: &str) -> String {
    let p = Path::new(input);
    let stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());
    let dir = p.parent().unwrap_or_else(|| Path::new(""));
    dir.join(format!("{stem}.{format}")).to_string_lossy().into_owned()
}

fn print_help() {
    print!(
        "libre99asm — TMS9900 assembler / cartridge packager\n\n\
         USAGE:\n\
         \x20   libre99asm [OPTIONS] <input.asm>     assemble a cartridge (default)\n\
         \x20   libre99asm rom <out.bin>             build the rewritten console ROM\n\
         \x20   libre99asm dsr <out.bin>             build the rewritten disk-controller DSR\n\
         \x20   libre99asm dis <file.bin> [addr]     disassemble from an address\n\n\
         OPTIONS (cartridge mode):\n\
         \x20   -o, --output <file>   Output path (default: input with .ctg/.bin extension)\n\
         \x20       --format <fmt>    ctg | bin            (default: ctg)\n\
         \x20       --bin             Shorthand for --format bin (raw >6000 image)\n\
         \x20       --name <title>    Cartridge menu title (default: IDT, then \"CART\")\n\
         \x20       --entry <symbol>  Entry-point symbol (default: END operand, then START)\n\
         \x20       --listing <file>  Also write an address/object/source listing\n\
         \x20       --symbols <file>  Also write the symbol table as JSON\n\
         \x20   -h, --help            Print this help\n"
    );
}
