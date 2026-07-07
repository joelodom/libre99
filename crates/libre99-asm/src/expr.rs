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

//! Editor/Assembler expression evaluation.
//!
//! Operands are expressions over constants (`>`hex, decimal, `'c'`/`'cc'` chars),
//! symbols, and the location counter `$`, combined with `+ - * /` (and unary
//! `±`). Evaluation is **strictly left to right with no operator precedence**, as
//! the TI E/A specifies: `2+3*4` is `20`, not `14`; `7+1/2` is `4`. Division
//! truncates toward zero. Arithmetic is done in `i64` and range-checked by the
//! caller per context (a `BYTE` must fit 8 bits, a `DATA` 16, etc.).

use std::collections::HashMap;

/// Evaluate `s` against the symbol table, with `$` resolving to `lc`.
pub fn eval(s: &str, syms: &HashMap<String, i64>, lc: u16) -> Result<i64, String> {
    let toks = tokenize(s)?;
    if toks.is_empty() {
        return Err("empty expression".into());
    }
    let mut i = 0;
    // Optional leading unary sign applies to the first term only.
    let mut acc = match toks[0] {
        Tok::Op('+') => {
            i = 1;
            term(&toks, &mut i, syms, lc)?
        }
        Tok::Op('-') => {
            i = 1;
            term(&toks, &mut i, syms, lc)?.wrapping_neg()
        }
        _ => term(&toks, &mut i, syms, lc)?,
    };
    while i < toks.len() {
        let op = match toks[i] {
            Tok::Op(c) => c,
            _ => return Err("expected an operator in expression".into()),
        };
        i += 1;
        let rhs = term(&toks, &mut i, syms, lc)?;
        acc = match op {
            '+' => acc.wrapping_add(rhs),
            '-' => acc.wrapping_sub(rhs),
            '*' => acc.wrapping_mul(rhs),
            '/' => {
                if rhs == 0 {
                    return Err("division by zero".into());
                }
                acc / rhs
            }
            other => return Err(format!("unknown operator '{other}'")),
        };
    }
    Ok(acc)
}

fn term(toks: &[Tok], i: &mut usize, syms: &HashMap<String, i64>, lc: u16) -> Result<i64, String> {
    let t = toks.get(*i).ok_or("unexpected end of expression")?;
    *i += 1;
    match t {
        Tok::Num(n) => Ok(*n),
        Tok::Dollar => Ok(lc as i64),
        Tok::Sym(name) => syms
            .get(name)
            .copied()
            .ok_or_else(|| format!("undefined symbol '{name}'")),
        Tok::Op(_) => Err("expected a value, found an operator".into()),
    }
}

enum Tok {
    Num(i64),
    Sym(String),
    Dollar,
    Op(char),
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i] as char;
        match c {
            ' ' | '\t' => i += 1,
            '+' | '-' | '*' | '/' => {
                out.push(Tok::Op(c));
                i += 1;
            }
            '$' => {
                out.push(Tok::Dollar);
                i += 1;
            }
            '>' => {
                i += 1;
                let start = i;
                while i < b.len() && (b[i] as char).is_ascii_hexdigit() {
                    i += 1;
                }
                if i == start {
                    return Err("malformed hexadecimal constant".into());
                }
                let v = i64::from_str_radix(&s[start..i], 16).map_err(|e| e.to_string())?;
                out.push(Tok::Num(v));
            }
            '\'' => {
                let (v, next) = char_const(b, i)?;
                out.push(Tok::Num(v));
                i = next;
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < b.len() && (b[i] as char).is_ascii_digit() {
                    i += 1;
                }
                let v: i64 = s[start..i].parse().map_err(|_| "number out of range".to_string())?;
                out.push(Tok::Num(v));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < b.len() && {
                    let d = b[i] as char;
                    d.is_ascii_alphanumeric() || d == '_'
                } {
                    i += 1;
                }
                out.push(Tok::Sym(s[start..i].to_string()));
            }
            other => return Err(format!("unexpected character '{other}' in expression")),
        }
    }
    Ok(out)
}

/// Parse a character constant starting at `b[i] == '\''`; return `(value,
/// next-index)`. Up to two characters, packed big-endian; a doubled `''` inside is
/// a literal quote; an empty constant `''` is `0`.
fn char_const(b: &[u8], mut i: usize) -> Result<(i64, usize), String> {
    debug_assert_eq!(b[i], b'\'');
    i += 1;
    let mut chars = Vec::new();
    loop {
        if i >= b.len() {
            return Err("unterminated character constant".into());
        }
        if b[i] == b'\'' {
            if i + 1 < b.len() && b[i + 1] == b'\'' {
                chars.push(b'\''); // doubled quote => one quote
                i += 2;
            } else {
                i += 1; // closing quote
                break;
            }
        } else {
            chars.push(b[i]);
            i += 1;
        }
    }
    if chars.len() > 2 {
        return Err("character constant longer than two characters".into());
    }
    let mut v = 0i64;
    for &c in &chars {
        v = (v << 8) | c as i64;
    }
    Ok((v, i))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn e(s: &str) -> i64 {
        eval(s, &HashMap::new(), 0).unwrap()
    }

    #[test]
    fn left_to_right_no_precedence() {
        assert_eq!(e("4+5*2"), 18);
        assert_eq!(e("7+1/2"), 4);
        assert_eq!(e("2+3*4"), 20);
        assert_eq!(e("-5+2"), -3);
    }

    #[test]
    fn constants() {
        assert_eq!(e(">37AC"), 0x37AC);
        assert_eq!(e("1000"), 1000);
        assert_eq!(e("'AB'"), 0x4142);
        assert_eq!(e("'C'"), 0x43);
        assert_eq!(e("''''"), 0x27);
        assert_eq!(e("''"), 0);
    }

    #[test]
    fn symbols_and_dollar() {
        let mut s = HashMap::new();
        s.insert("INIT".to_string(), 0x0125);
        assert_eq!(eval("INIT+3", &s, 0).unwrap(), 0x0128);
        assert_eq!(eval("$", &s, 0x6010).unwrap(), 0x6010);
        assert!(eval("NOPE", &s, 0).is_err());
    }
}
