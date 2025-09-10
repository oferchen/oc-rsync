// crates/filters/src/parser/braces.rs
#![allow(clippy::while_let_on_iterator)]

use super::ParseError;

pub fn expand_braces(pat: &str) -> Result<Vec<String>, ParseError> {
    fn inner(
        prefix: &str,
        pattern: &str,
        out: &mut Vec<String>,
        count: &mut usize,
    ) -> Result<(), ParseError> {
        if pattern.is_empty() {
            out.push(prefix.to_string());
            *count += 1;
            if *count > super::MAX_BRACE_EXPANSIONS {
                return Err(ParseError::TooManyExpansions);
            }
            return Ok(());
        }
        let mut chars = pattern.chars().peekable();
        let mut i = 0;
        while let Some(ch) = chars.next() {
            match ch {
                '{' => {
                    let mut depth = 1;
                    let mut j = i + 1;
                    while let Some(c) = chars.next() {
                        j += 1;
                        if c == '{' {
                            depth += 1;
                        } else if c == '}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                    let content = &pattern[i + 1..j];
                    let mut part = String::new();
                    let mut k = 0;
                    let mut brace_depth = 0;
                    while k < content.len() {
                        let c = content.as_bytes()[k] as char;
                        if c == '{' {
                            brace_depth += 1;
                            part.push(c);
                        } else if c == '}' {
                            brace_depth -= 1;
                            part.push(c);
                        } else if c == ',' && brace_depth == 0 {
                            inner(&format!("{prefix}{part}"), &pattern[j + 1..], out, count)?;
                            part.clear();
                        } else {
                            part.push(c);
                        }
                        k += 1;
                    }
                    inner(&format!("{prefix}{part}"), &pattern[j + 1..], out, count)?;
                    return Ok(());
                }
                '\\' => {
                    if let Some(next) = chars.next() {
                        return inner(
                            &format!(
                                "{prefix}{}",
                                &pattern[i..i + ch.len_utf8() + next.len_utf8()]
                            ),
                            &pattern[i + ch.len_utf8() + next.len_utf8()..],
                            out,
                            count,
                        );
                    }
                }
                _ => {
                    i += ch.len_utf8();
                    continue;
                }
            }
            break;
        }
        out.push(format!("{prefix}{pattern}"));
        *count += 1;
        if *count > super::MAX_BRACE_EXPANSIONS {
            return Err(ParseError::TooManyExpansions);
        }
        Ok(())
    }
    let mut out = Vec::new();
    let mut count = 0;
    inner("", pat, &mut out, &mut count)?;
    Ok(out)
}
