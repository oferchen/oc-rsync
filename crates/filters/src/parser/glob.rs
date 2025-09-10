// crates/filters/src/parser/glob.rs

use super::ParseError;
use globset::{GlobBuilder, GlobMatcher};

pub fn compile_glob(pat: &str) -> Result<GlobMatcher, ParseError> {
    let mut translated = String::new();
    let mut chars = pat.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                translated.push('\\');
                if let Some(next) = chars.next() {
                    translated.push(next);
                }
            }
            '[' => {
                translated.push('[');
                while let Some(c) = chars.next() {
                    translated.push(c);
                    if c == '\\' {
                        if let Some(esc) = chars.next() {
                            translated.push(esc);
                        }
                        continue;
                    }
                    if c == ']' {
                        break;
                    }
                }
            }
            '*' => {
                if let Some('*') = chars.peek() {
                    chars.next();
                    translated.push_str("**");
                } else {
                    translated.push('*');
                }
            }
            _ => translated.push(ch),
        }
    }

    let translated = translated.replace("\\[", "\u{1}").replace("\\]", "\u{2}");

    let pat = expand_posix_classes(&translated);
    let pat = confine_char_classes(&pat);
    let pat = pat.replace('\u{1}', "\\[").replace('\u{2}', "\\]");

    Ok(GlobBuilder::new(&pat)
        .literal_separator(true)
        .backslash_escape(true)
        .build()?
        .compile_matcher())
}

fn expand_posix_classes(pat: &str) -> String {
    fn class(name: &str) -> Option<&'static str> {
        match name {
            "alnum" => Some("0-9A-Za-z"),
            "alpha" => Some("A-Za-z"),
            "digit" => Some("0-9"),
            "lower" => Some("a-z"),
            "upper" => Some("A-Z"),
            "xdigit" => Some("0-9A-Fa-f"),
            "space" => Some("\t\n\r\x0B\x0C "),
            "punct" => Some("!\"#$%&'()*+,\\-./:;<=>?@\\[\\\\\\]\\^_`{|}~"),
            "blank" => Some("\t "),
            "cntrl" => Some("\x00-\x1F\x7F"),
            "graph" => Some("!-~"),
            "print" => Some(" -~"),
            _ => None,
        }
    }

    let chars: Vec<char> = pat.chars().collect();
    let mut i = 0;
    let mut out = String::new();
    while i < chars.len() {
        if chars[i] == '[' {
            out.push('[');
            i += 1;
            while i < chars.len() && chars[i] != ']' {
                if chars[i] == '[' && i + 1 < chars.len() && chars[i + 1] == ':' {
                    let start = i + 2;
                    let mut j = start;
                    while j + 1 < chars.len() {
                        if chars[j] == ':' && chars[j + 1] == ']' {
                            break;
                        }
                        j += 1;
                    }
                    if j + 1 < chars.len() {
                        let name: String = chars[start..j].iter().collect();
                        if let Some(rep) = class(&name) {
                            out.push_str(rep);
                            i = j + 2;
                            continue;
                        }
                    }
                }
                out.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                out.push(']');
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn confine_char_classes(pat: &str) -> String {
    let mut out = String::new();
    let mut chars = pat.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            out.push('[');
            let mut negated = false;
            if let Some(&next) = chars.peek() {
                if next == '!' {
                    negated = true;
                    out.push('!');
                    chars.next();
                }
            }
            let mut inner = String::new();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    inner.push('\\');
                    if let Some(esc) = chars.next() {
                        inner.push(esc);
                    }
                    continue;
                }
                if c == ']' {
                    break;
                }
                inner.push(c);
            }
            if negated {
                if !inner.contains('/') {
                    inner.push('/');
                }
                out.push_str(&inner);
            } else {
                inner.retain(|c| c != '/');
                out.push_str(&inner);
            }
            out.push(']');
        } else {
            out.push(ch);
        }
    }
    out
}
