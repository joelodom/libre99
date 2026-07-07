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

//! Front-end pieces shared by both assemblers (`libre99-asm` and `libre99-gpl`).
//!
//! `lex`/`expr` already back both assemblers' line tokenising and expression
//! evaluation; this module hoists the two remaining **byte-identical** helpers
//! so the GPL side reuses them rather than carrying a copy:
//!
//! * [`operands`] — split a line's operand field into individual operands.
//! * [`string_operand`] — parse a single-quoted string literal.
//!
//! Deliberately **not** hoisted, because their behaviour differs by design and
//! unifying would change observable output (P4.3/T3 was conservative here):
//!
//! * The `Diag` type — its `Display` renders `line N: msg` in `libre99-gpl` but the
//!   compiler-style `N: msg` in `libre99-asm` (whose CLI prefixes it with
//!   `file.asm:` to read as `file.asm:N: msg`). The orphan rule also forbids a
//!   shared struct carrying a per-crate `Display`, so a shared `Diag` would force
//!   one format on both.
//! * The `EQU`/`BYTE`/`DATA`/`TEXT`/`BSS` directive handlers — they hang off each
//!   assembler's own `Asm` state (different `push`/`eval` signatures, different
//!   range windows and diagnostic text), so they are not the "genuinely
//!   identical" pieces this module is for.

use crate::lex;

/// Split a line's operand field into individual operands.
pub fn operands(l: &lex::Line) -> Vec<String> {
    l.operands.as_deref().map(lex::split_operands).unwrap_or_default()
}

/// Parse a single-quoted string operand (`'…'`), unescaping doubled quotes.
pub fn string_operand(s: &str) -> Result<String, String> {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'\'' || bytes[bytes.len() - 1] != b'\'' {
        return Err(format!("expected a single-quoted string, found `{s}`"));
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                chars.next();
                out.push('\'');
            } else {
                return Err("unbalanced quote in string".into());
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_operand_unescapes_doubled_quotes() {
        assert_eq!(string_operand("'HI'").unwrap(), "HI");
        assert_eq!(string_operand("  'A''B'  ").unwrap(), "A'B");
        assert!(string_operand("HI").is_err()); // no quotes
        assert!(string_operand("'un'balanced'").is_err()); // lone inner quote
    }
}
