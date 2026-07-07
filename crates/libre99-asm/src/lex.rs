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

//! Lexing: source text → per-line `(label, mnemonic, operand-field)`, plus a
//! helper to split an operand field into individual operands.
//!
//! Source-line format (Editor/Assembler): up to four whitespace-separated fields
//! — label, mnemonic, operands, comment. A label is present only when the line
//! begins in column 1 with a non-blank, non-`*` character (a trailing `:` on the
//! label is accepted and dropped). A `*` in column 1 makes the whole line a
//! comment; a `;` (extension) starts a trailing comment anywhere outside quotes.
//! Operands carry no internal spaces except inside single-quoted constants.

/// One source line, decomposed into its fields. Blank/comment lines yield a
/// `Line` with no label/mnemonic (kept so line numbers stay accurate).
#[derive(Debug, Clone)]
pub struct Line {
    /// 1-based source line number.
    pub num: usize,
    pub label: Option<String>,
    pub mnemonic: Option<String>,
    /// The raw operand field, with the trailing comment removed.
    pub operands: Option<String>,
}

/// Decompose every source line.
pub fn parse(src: &str) -> Vec<Line> {
    src.lines()
        .enumerate()
        .map(|(i, raw)| parse_line(i + 1, raw))
        .collect()
}

fn parse_line(num: usize, raw: &str) -> Line {
    let blank = Line { num, label: None, mnemonic: None, operands: None };
    let trimmed = raw.trim_start();
    if trimmed.is_empty() || raw.starts_with('*') || trimmed.starts_with(';') {
        return blank;
    }

    let mut rest = raw;
    // A label exists iff the line does not start with whitespace.
    let label = if !raw.starts_with([' ', '\t']) {
        let (tok, after) = take_token(rest);
        rest = after;
        Some(tok.trim_end_matches(':').to_string())
    } else {
        None
    };

    rest = rest.trim_start();
    if rest.is_empty() || rest.starts_with(';') {
        return Line { num, label, mnemonic: None, operands: None };
    }

    let (mnem, after) = take_token(rest);
    rest = after.trim_start();
    let field = take_operand_field(rest);
    Line {
        num,
        label,
        mnemonic: Some(mnem.to_string()),
        operands: if field.is_empty() { None } else { Some(field) },
    }
}

/// Take a whitespace-delimited token; return `(token, remainder)`.
fn take_token(s: &str) -> (&str, &str) {
    let end = s.find([' ', '\t']).unwrap_or(s.len());
    (&s[..end], &s[end..])
}

/// Take the operand field: everything up to the first whitespace or `;` that is
/// not inside a single-quoted constant.
fn take_operand_field(s: &str) -> String {
    let mut in_quote = false;
    let mut end = s.len();
    for (i, c) in s.char_indices() {
        match c {
            '\'' => in_quote = !in_quote,
            ' ' | '\t' | ';' if !in_quote => {
                end = i;
                break;
            }
            _ => {}
        }
    }
    s[..end].trim_end().to_string()
}

/// Split an operand field on commas that are not inside a quote or parentheses.
pub fn split_operands(field: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut start = 0usize;
    for (i, c) in field.char_indices() {
        match c {
            '\'' => in_quote = !in_quote,
            '(' if !in_quote => depth += 1,
            ')' if !in_quote => depth -= 1,
            ',' if !in_quote && depth == 0 => {
                out.push(field[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(field[start..].trim().to_string());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_comments() {
        let l = &parse("START  LIMI 0           ; go")[0];
        assert_eq!(l.label.as_deref(), Some("START"));
        assert_eq!(l.mnemonic.as_deref(), Some("LIMI"));
        assert_eq!(l.operands.as_deref(), Some("0"));

        let l = &parse("       MOVB *R1+,@VDPWA")[0];
        assert_eq!(l.label, None);
        assert_eq!(l.operands.as_deref(), Some("*R1+,@VDPWA"));

        assert!(parse("* a comment")[0].mnemonic.is_none());
        assert_eq!(parse("LABEL:")[0].label.as_deref(), Some("LABEL"));
    }

    #[test]
    fn operand_split_respects_quotes_and_parens() {
        assert_eq!(split_operands(">00,>80"), vec![">00", ">80"]);
        assert_eq!(split_operands("@TAB(R1),R2"), vec!["@TAB(R1)", "R2"]);
        assert_eq!(split_operands("'A',>20"), vec!["'A'", ">20"]);
    }
}
