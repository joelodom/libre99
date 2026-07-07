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

//! Command-line argument parsing (no external arg-parsing crate).
//!
//! Flags override the preferences file for a single run. Supported:
//! `--cartridge <path>`, `--disk <path>`, `--system-rom <path>`,
//! `--system-grom <path>`, `--disk-dsr <path>`, `--scale <n>`,
//! `--fullscreen`, `--log-level <level>`, `--help`. Nothing is embedded: the
//! console boots bare unless media is given here or loaded later from the
//! in-app file browser (`F9`).

/// Parsed command line. Each `Option` is `None` when the flag was not given.
#[derive(Debug, Default, PartialEq)]
pub struct Args {
    /// `Some(path)` to mount a `.ctg` cartridge image (e.g. libre99asm output).
    pub cartridge: Option<String>,
    /// `Some(path)` to boot a system GROM image from disk in place of the default
    /// clean-room GROM (e.g. an authentic `994AGROM.Bin`).
    pub system_grom: Option<String>,
    /// `Some(path)` to boot a console ROM image from disk in place of the default
    /// clean-room ROM (e.g. an authentic `994aROM.Bin`).
    pub system_rom: Option<String>,
    /// `Some(path)` to install a disk-controller DSR ROM from disk in place of
    /// the default clean-room DSR (e.g. an authentic `Disk.Bin`).
    pub disk_dsr: Option<String>,
    /// `Some(path)` to insert a `.dsk` disk image into DSK1.
    pub disk: Option<String>,
    pub scale: Option<u32>,
    pub fullscreen: bool,
    pub log_level: Option<String>,
    pub help: bool,
}

/// One-line-per-flag usage text.
pub const USAGE: &str = "\
Usage: libre99 [options]

The console boots bare (no media). Load media from these flags or in the app
with the file browser (F9).

  --cartridge <path>   Mount a .ctg cartridge image (e.g. libre99asm output)
  --disk <path>        Insert a .dsk disk image into DSK1
  --system-grom <path> Boot a system GROM image in place of the default (clean-room) GROM
  --system-rom <path>  Boot a console ROM image in place of the default (clean-room) ROM
  --disk-dsr <path>    Install a disk DSR ROM in place of the default (clean-room) DSR
  --scale <n>          Integer window scale (1-8)
  --fullscreen         Start fullscreen
  --log-level <level>  error | warn | info | debug | trace
  --help               Show this help and exit";

impl Args {
    /// Parse arguments (excluding the program name), returning an error message
    /// suitable for printing to stderr.
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Args, String> {
        let mut out = Args::default();
        let mut it = args.into_iter();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                // `--cartridge-file` is the pre-media-rework spelling, kept as a
                // quiet alias so existing scripts and docs keep working.
                "--cartridge" | "--cartridge-file" => {
                    out.cartridge = Some(value(&mut it, "--cartridge")?)
                }
                "--system-grom" => out.system_grom = Some(value(&mut it, "--system-grom")?),
                "--system-rom" => out.system_rom = Some(value(&mut it, "--system-rom")?),
                "--disk-dsr" => out.disk_dsr = Some(value(&mut it, "--disk-dsr")?),
                "--disk" => out.disk = Some(value(&mut it, "--disk")?),
                "--scale" => {
                    let v = value(&mut it, "--scale")?;
                    out.scale = Some(v.parse().map_err(|_| format!("invalid --scale: {v}"))?);
                }
                "--fullscreen" => out.fullscreen = true,
                "--log-level" => out.log_level = Some(value(&mut it, "--log-level")?),
                "--help" | "-h" => out.help = true,
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(out)
    }
}

fn value<I: Iterator<Item = String>>(it: &mut I, flag: &str) -> Result<String, String> {
    it.next().ok_or_else(|| format!("{flag} requires a value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        Args::parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn parses_media_and_display_flags() {
        let a = parse(&["--cartridge", "path/to/game.ctg", "--disk", "vol/Foo.Dsk", "--scale", "4", "--fullscreen"])
            .unwrap();
        assert_eq!(a.cartridge.as_deref(), Some("path/to/game.ctg"));
        assert_eq!(a.disk.as_deref(), Some("vol/Foo.Dsk"));
        assert_eq!(a.scale, Some(4));
        assert!(a.fullscreen);
    }

    #[test]
    fn cartridge_file_is_a_compatible_alias() {
        let a = parse(&["--cartridge-file", "build/titris.ctg"]).unwrap();
        assert_eq!(a.cartridge.as_deref(), Some("build/titris.ctg"));
        assert!(parse(&["--cartridge-file"]).is_err());
    }

    #[test]
    fn system_firmware_flags_take_paths() {
        let a = parse(&[
            "--system-grom",
            "grom/console-grom.bin",
            "--system-rom",
            "third-party/roms/994aROM.Bin",
        ])
        .unwrap();
        assert_eq!(a.system_grom.as_deref(), Some("grom/console-grom.bin"));
        assert_eq!(a.system_rom.as_deref(), Some("third-party/roms/994aROM.Bin"));
        assert!(parse(&["--system-grom"]).is_err());
    }

    #[test]
    fn help_is_a_flag_and_removed_flags_error() {
        assert!(parse(&["--help"]).unwrap().help);
        // Embedded-media flags died with the embeds.
        assert!(parse(&["--list"]).is_err());
        assert!(parse(&["--no-cartridge"]).is_err());
    }

    #[test]
    fn missing_value_and_unknown_flag_error() {
        assert!(parse(&["--scale"]).is_err());
        assert!(parse(&["--scale", "huge"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
    }

    #[test]
    fn empty_args_are_all_defaults() {
        assert_eq!(parse(&[]).unwrap(), Args::default());
    }
}
