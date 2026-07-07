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

//! Leveled logging to both the terminal and a log file
//! (`~/.libre99/libre99.log`, appended across runs), via
//! `simplelog`.

use std::fs::OpenOptions;
use std::path::Path;

use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, SharedLogger, TerminalMode, WriteLogger};

/// Initialize logging at `level`, writing to the terminal and (if a path is
/// given and openable) to a log file. Safe to call once; later calls are no-ops.
pub fn init(level: LevelFilter, file: Option<&Path>) {
    let log_config = simplelog::Config::default();
    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![simplelog::TermLogger::new(
        level,
        log_config.clone(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )];

    if let Some(path) = file {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(handle) = OpenOptions::new().create(true).append(true).open(path) {
            loggers.push(WriteLogger::new(level, log_config, handle));
        }
    }

    let _ = CombinedLogger::init(loggers);
}

/// Map a config/CLI level string to a `LevelFilter` (`info` for anything else).
pub fn level_from_str(s: &str) -> LevelFilter {
    match s.to_ascii_lowercase().as_str() {
        "off" => LevelFilter::Off,
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    }
}
