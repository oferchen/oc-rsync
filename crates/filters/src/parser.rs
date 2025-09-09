// crates/filters/src/parser.rs
#![allow(clippy::collapsible_if)]

use crate::{
    perdir::PerDir,
    rule::{Rule, RuleData, RuleFlags},
};
use globset::{GlobBuilder, GlobMatcher};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub const MAX_PARSE_DEPTH: usize = 64;
pub const MAX_BRACE_EXPANSIONS: usize = 10_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MergeModifier {
    IncludeOnly,
    ExcludeOnly,
    NoInherit,
    WordSplit,
    CvsMode,
}

impl MergeModifier {
    fn apply(self, pd: &mut PerDir) {
        match self {
            MergeModifier::IncludeOnly => pd.sign = Some('+'),
            MergeModifier::ExcludeOnly => pd.sign = Some('-'),
            MergeModifier::NoInherit => pd.inherit = false,
            MergeModifier::WordSplit => pd.word_split = true,
            MergeModifier::CvsMode => pd.cvs = true,
        }
    }
}

fn compile_glob(pat: &str) -> Result<GlobMatcher, ParseError> {
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

fn expand_braces(pat: &str) -> Result<Vec<String>, ParseError> {
    fn inner(
        prefix: &str,
        pattern: &str,
        out: &mut Vec<String>,
        count: &mut usize,
    ) -> Result<(), ParseError> {
        if let Some(start) = pattern.find('{') {
            let mut depth = 0;
            let mut end = start;
            let chars: Vec<char> = pattern.chars().collect();
            for (idx, ch) in chars.iter().enumerate().skip(start) {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = idx;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth != 0 {
                let final_str = format!("{}{}", prefix, pattern);
                *count += 1;
                if *count > MAX_BRACE_EXPANSIONS {
                    return Err(ParseError::TooManyExpansions);
                }
                out.push(final_str);
                return Ok(());
            }
            let pre = &pattern[..start];
            let suffix = &pattern[end + 1..];
            let body = &pattern[start + 1..end];
            let mut parts = Vec::new();
            let mut part = String::new();
            let mut d = 0;
            for ch in body.chars() {
                match ch {
                    '{' => {
                        d += 1;
                        part.push(ch);
                    }
                    '}' => {
                        d -= 1;
                        part.push(ch);
                    }
                    ',' if d == 0 => {
                        parts.push(part);
                        part = String::new();
                    }
                    _ => part.push(ch),
                }
            }
            parts.push(part);
            let mut expanded_parts = Vec::new();
            for p in parts {
                if let Some(range) = expand_range(&p)? {
                    expanded_parts.extend(range);
                } else {
                    expanded_parts.push(p);
                }
            }
            for part in expanded_parts {
                let new_prefix = format!("{}{}{}", prefix, pre, part);
                inner(&new_prefix, suffix, out, count)?;
                if *count > MAX_BRACE_EXPANSIONS {
                    return Err(ParseError::TooManyExpansions);
                }
            }
            Ok(())
        } else {
            let final_str = format!("{}{}", prefix, pattern);
            *count += 1;
            if *count > MAX_BRACE_EXPANSIONS {
                return Err(ParseError::TooManyExpansions);
            }
            out.push(final_str);
            Ok(())
        }
    }

    fn expand_range(part: &str) -> Result<Option<Vec<String>>, ParseError> {
        let parts: Vec<&str> = part.split("..").collect();
        if parts.len() < 2 || parts.len() > 3 {
            return Ok(None);
        }
        let start = parts[0];
        let end = parts[1];
        let step = parts.get(2).copied().unwrap_or("1");
        if let (Ok(a), Ok(b), Ok(s)) = (
            start.parse::<i64>(),
            end.parse::<i64>(),
            step.parse::<i64>(),
        ) {
            if s == 0 {
                return Ok(None);
            }
            let width = start.len().max(end.len());
            let mut out = Vec::new();
            if (a <= b && s > 0) || (a >= b && s < 0) {
                let mut i = a;
                if s > 0 {
                    while i <= b {
                        if width > 1 {
                            out.push(format!("{:0width$}", i, width = width));
                        } else {
                            out.push(i.to_string());
                        }
                        i += s;
                        if out.len() > MAX_BRACE_EXPANSIONS {
                            return Err(ParseError::TooManyExpansions);
                        }
                    }
                } else {
                    while i >= b {
                        if width > 1 {
                            out.push(format!("{:0width$}", i, width = width));
                        } else {
                            out.push(i.to_string());
                        }
                        i += s;
                        if out.len() > MAX_BRACE_EXPANSIONS {
                            return Err(ParseError::TooManyExpansions);
                        }
                    }
                }
                return Ok(Some(out));
            }
        } else if start.len() == 1 && end.len() == 1 {
            let a = start.chars().next().unwrap() as u32;
            let b = end.chars().next().unwrap() as u32;
            let s = parts
                .get(2)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1);
            if a <= b && s > 0 {
                let out: Vec<String> = (a..=b)
                    .step_by(s as usize)
                    .map(|c| char::from_u32(c).unwrap().to_string())
                    .collect();
                if out.len() > MAX_BRACE_EXPANSIONS {
                    return Err(ParseError::TooManyExpansions);
                }
                return Ok(Some(out));
            }
        }
        Ok(None)
    }

    let mut out = Vec::new();
    let mut count = 0;
    inner("", pat, &mut out, &mut count)?;
    Ok(out)
}

#[derive(Debug)]
pub enum ParseError {
    InvalidRule(String),
    Glob(globset::Error),
    RecursiveInclude(PathBuf),
    RecursionLimit,
    TooManyExpansions,
    Io(std::io::Error),
}

impl From<globset::Error> for ParseError {
    fn from(e: globset::Error) -> Self {
        Self::Glob(e)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

enum RuleKind {
    Include,
    Exclude,
    Protect,
}

fn decode_line(raw: &str) -> Option<String> {
    let mut out = String::new();
    let chars = raw.trim_end_matches('\r').chars();
    let mut escaped = false;
    let mut started = false;
    let mut last_non_space = 0;
    for c in chars {
        if escaped {
            if c.is_whitespace() || c == '#' || c == '\\' {
                out.push(c);
            } else {
                out.push('\\');
                out.push(c);
            }
            last_non_space = out.len();
            escaped = false;
            started = true;
            continue;
        }
        if !started {
            if c.is_whitespace() {
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '#' {
                return None;
            }
            started = true;
            out.push(c);
            if !c.is_whitespace() {
                last_non_space = out.len();
            }
        } else if c == '\\' {
            escaped = true;
        } else {
            out.push(c);
            if !c.is_whitespace() {
                last_non_space = out.len();
            }
        }
    }
    if escaped {
        out.push('\\');
        last_non_space = out.len();
    }
    out.truncate(last_non_space);
    if out.is_empty() { None } else { Some(out) }
}

pub fn parse_with_options(
    input: &str,
    from0: bool,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    source: Option<PathBuf>,
) -> Result<Vec<Rule>, ParseError> {
    if depth >= MAX_PARSE_DEPTH {
        return Err(ParseError::RecursionLimit);
    }
    fn split_mods<'a>(s: &'a str, allowed: &str) -> (&'a str, &'a str) {
        if let Some(ch) = s.chars().next() {
            if ch.is_whitespace() {
                let rest = &s[ch.len_utf8()..];
                return ("", rest);
            }
        } else {
            return ("", "");
        }
        let mut idx = 0;
        let bytes = s.as_bytes();
        if bytes.first() == Some(&b',') {
            idx += 1;
        }
        while let Some(&c) = bytes.get(idx) {
            if allowed.as_bytes().contains(&c) {
                idx += 1;
            } else {
                break;
            }
        }
        let rest = &s[idx..];
        let rest = if rest.starts_with(' ') || rest.starts_with('\t') {
            &rest[1..]
        } else {
            rest
        };
        (&s[..idx], rest)
    }

    let mut rules = Vec::new();

    for raw_line in input.lines() {
        let line = match decode_line(raw_line) {
            Some(l) => l,
            None => continue,
        };

        if line == "!" || line == "c" {
            rules.push(Rule::Clear);
            continue;
        }
        if line == "existing" {
            rules.push(Rule::Existing);
            continue;
        }
        if line == "no-existing" {
            rules.push(Rule::NoExisting);
            continue;
        }
        if line == "prune-empty-dirs" {
            rules.push(Rule::PruneEmptyDirs);
            continue;
        }
        if line == "no-prune-empty-dirs" {
            rules.push(Rule::NoPruneEmptyDirs);
            continue;
        }

        if let Some(rest) = line.strip_prefix("-F") {
            let count = rest.chars().take_while(|c| *c == 'F').count();
            if count == rest.len() {
                rules.push(Rule::DirMerge(PerDir {
                    file: ".rsync-filter".to_string(),
                    anchored: true,
                    root_only: false,
                    inherit: true,
                    cvs: false,
                    word_split: false,
                    sign: None,
                    flags: RuleFlags::default(),
                }));
                if count > 0 {
                    let pat = "**/.rsync-filter";
                    let matcher = compile_glob(pat)?;
                    let data = RuleData {
                        matcher,
                        invert: false,
                        flags: RuleFlags::default(),
                        source: source.clone(),
                        dir_only: false,
                        has_slash: pat.contains('/'),
                        pattern: pat.to_string(),
                    };
                    rules.push(Rule::Exclude(data));
                }
                continue;
            }
        }

        let (kind, mods, rest) = if let Some(r) = line.strip_prefix('+') {
            let (m, rest) = split_mods(r, "/!Csrpx");
            (Some(RuleKind::Include), m.to_string(), rest)
        } else if let Some(r) = line.strip_prefix('-') {
            let (m, rest) = split_mods(r, "/!Csrpx");
            (Some(RuleKind::Exclude), m.to_string(), rest)
        } else if let Some(r) = line.strip_prefix('P').or_else(|| line.strip_prefix('p')) {
            let (m, rest) = split_mods(r, "/!Csrpx");
            let mut mods = m.to_string();
            if !mods.contains('r') {
                mods.push('r');
            }
            (Some(RuleKind::Protect), mods, rest)
        } else if let Some(r) = line.strip_prefix('S') {
            let (m, rest) = split_mods(r, "/!Csrpx");
            let mut mods = m.to_string();
            if !mods.contains('s') {
                mods.push('s');
            }
            (Some(RuleKind::Include), mods, rest)
        } else if let Some(r) = line.strip_prefix('H').or_else(|| line.strip_prefix('h')) {
            let (m, rest) = split_mods(r, "/!Csrpx");
            let mut mods = m.to_string();
            if !mods.contains('s') {
                mods.push('s');
            }
            (Some(RuleKind::Exclude), mods, rest)
        } else if let Some(r) = line.strip_prefix('R') {
            let (m, rest) = split_mods(r, "/!Csrpx");
            let mut mods = m.to_string();
            if !mods.contains('r') {
                mods.push('r');
            }
            (Some(RuleKind::Include), mods, rest)
        } else if let Some(r) = line.strip_prefix('.') {
            let (m, file) = split_mods(r, "-+Cenw/!srpx");
            if file.is_empty() {
                return Err(ParseError::InvalidRule(raw_line.to_string()));
            }
            let path = PathBuf::from(file);
            if !visited.insert(path.clone()) {
                return Err(ParseError::RecursiveInclude(path));
            }
            let data =
                fs::read(&path).map_err(|_| ParseError::InvalidRule(raw_line.to_string()))?;
            let mut sub = if from0 {
                if m.contains('C') {
                    let mut buf = Vec::new();
                    for token in data.split(|b| *b == 0) {
                        let token = trim_newlines(token);
                        if token.is_empty() || token.starts_with(b"#") {
                            continue;
                        }
                        buf.extend_from_slice(b"- ");
                        buf.extend_from_slice(token);
                        buf.push(b'\n');
                    }
                    parse_from_bytes(&buf, false, visited, depth + 1, Some(path.clone()))?
                } else if m.contains('+') || m.contains('-') {
                    let sign = if m.contains('+') { b'+' } else { b'-' };
                    let mut buf = Vec::new();
                    for token in data.split(|b| *b == 0) {
                        let token = trim_newlines(token);
                        if token.is_empty() || token.starts_with(b"#") {
                            continue;
                        }
                        buf.push(sign);
                        buf.push(b' ');
                        buf.extend_from_slice(token);
                        buf.push(b'\n');
                    }
                    parse_from_bytes(&buf, false, visited, depth + 1, Some(path.clone()))?
                } else {
                    parse_from_bytes(&data, true, visited, depth + 1, Some(path.clone()))?
                }
            } else {
                let mut content = String::from_utf8_lossy(&data).to_string();
                if m.contains('C') {
                    let mut buf = String::new();
                    for token in content.split_whitespace() {
                        if token.starts_with('#') {
                            continue;
                        }
                        buf.push_str("- ");
                        buf.push_str(token);
                        buf.push('\n');
                    }
                    content = buf;
                } else {
                    if m.contains('w') {
                        let mut buf = String::new();
                        for token in content.split_whitespace() {
                            buf.push_str(token);
                            buf.push('\n');
                        }
                        content = buf;
                    }
                    if m.contains('+') || m.contains('-') {
                        let sign = if m.contains('+') { '+' } else { '-' };
                        let mut buf = String::new();
                        for raw in content.lines() {
                            let line = raw.trim();
                            if line.is_empty() || line.starts_with('#') {
                                continue;
                            }
                            buf.push(sign);
                            buf.push(' ');
                            buf.push_str(line);
                            buf.push('\n');
                        }
                        content = buf;
                    }
                }
                parse_with_options(&content, from0, visited, depth + 1, Some(path.clone()))?
            };
            if m.contains('s') || m.contains('r') || m.contains('p') || m.contains('x') {
                let inherited = RuleFlags::from_mods(m);
                for rule in &mut sub {
                    if let Rule::Include(d)
                    | Rule::Exclude(d)
                    | Rule::Protect(d)
                    | Rule::ImpliedDir(d) = rule
                    {
                        d.flags = d.flags.union(&inherited);
                    }
                }
            }
            rules.extend(sub);
            if m.contains('e') {
                let pat = format!("**/{}", file);
                let matcher = compile_glob(&pat)?;
                let data = RuleData {
                    matcher,
                    invert: false,
                    flags: RuleFlags::default(),
                    source: source.clone(),
                    dir_only: false,
                    has_slash: pat.contains('/'),
                    pattern: pat.clone(),
                };
                rules.push(Rule::Exclude(data));
            }
            continue;
        } else if let Some(r) = line.strip_prefix('d') {
            let rewritten = format!(":{}\n", r);
            let sub = parse_with_options(&rewritten, from0, visited, depth + 1, source.clone())?;
            rules.extend(sub);
            continue;
        } else if let Some(rest) = line.strip_prefix(":include-merge") {
            let file = rest.trim();
            if file.is_empty() {
                return Err(ParseError::InvalidRule(raw_line.to_string()));
            }
            let path = PathBuf::from(file);
            if !visited.insert(path.clone()) {
                return Err(ParseError::RecursiveInclude(path));
            }
            let data =
                fs::read(&path).map_err(|_| ParseError::InvalidRule(raw_line.to_string()))?;
            let sub = parse_from_bytes(&data, from0, visited, depth + 1, Some(path.clone()))?;
            rules.extend(sub);
            continue;
        } else if let Some(r) = line.strip_prefix(':') {
            let (m, file) = split_mods(r, "-+Cenw/!srpx");
            if file.is_empty() {
                if m.contains('C') {
                    rules.push(Rule::DirMerge(PerDir {
                        file: ".cvsignore".into(),
                        anchored: false,
                        root_only: false,
                        inherit: true,
                        cvs: true,
                        word_split: false,
                        sign: None,
                        flags: RuleFlags::default(),
                    }));
                    continue;
                } else {
                    return Err(ParseError::InvalidRule(raw_line.to_string()));
                }
            }
            let anchored = file.starts_with('/') || m.contains('/');
            let fname = if anchored {
                file.trim_start_matches('/').to_string()
            } else {
                file.to_string()
            };
            let mut pd = PerDir {
                file: fname.clone(),
                anchored,
                root_only: anchored,
                inherit: true,
                cvs: false,
                word_split: false,
                sign: None,
                flags: RuleFlags::from_mods(m),
            };
            let mut exclude_self = false;
            for ch in m.chars() {
                match ch {
                    '+' => MergeModifier::IncludeOnly.apply(&mut pd),
                    '-' => MergeModifier::ExcludeOnly.apply(&mut pd),
                    'n' => MergeModifier::NoInherit.apply(&mut pd),
                    'w' => MergeModifier::WordSplit.apply(&mut pd),
                    'C' => MergeModifier::CvsMode.apply(&mut pd),
                    'e' => exclude_self = true,
                    _ => {}
                }
            }
            rules.push(Rule::DirMerge(pd));
            if exclude_self {
                let pat = format!("**/{}", fname);
                let matcher = compile_glob(&pat)?;
                let data = RuleData {
                    matcher,
                    invert: false,
                    flags: RuleFlags::default(),
                    source: source.clone(),
                    dir_only: false,
                    has_slash: pat.contains('/'),
                    pattern: pat.clone(),
                };
                rules.push(Rule::Exclude(data));
            }
            continue;
        } else {
            let mut parts = line.split_whitespace();
            let token = parts.next().unwrap_or("");
            let rest = parts.next().unwrap_or("").trim();
            match token {
                "include" => (Some(RuleKind::Include), String::new(), rest),
                "exclude" => (Some(RuleKind::Exclude), String::new(), rest),
                "show" => (Some(RuleKind::Include), "s".to_string(), rest),
                "hide" => (Some(RuleKind::Exclude), "s".to_string(), rest),
                "protect" => (Some(RuleKind::Protect), "r".to_string(), rest),
                "risk" => (Some(RuleKind::Include), "r".to_string(), rest),
                "include-from" => {
                    let path = PathBuf::from(rest);
                    if !visited.insert(path.clone()) {
                        return Err(ParseError::RecursiveInclude(path));
                    }
                    let sub = parse_rule_list_file(&path, from0, '+', visited, depth + 1)?;
                    rules.extend(sub);
                    continue;
                }
                "exclude-from" => {
                    let path = PathBuf::from(rest);
                    if !visited.insert(path.clone()) {
                        return Err(ParseError::RecursiveInclude(path));
                    }
                    let sub = parse_rule_list_file(&path, from0, '-', visited, depth + 1)?;
                    rules.extend(sub);
                    continue;
                }
                "files-from" => {
                    let path = PathBuf::from(rest);
                    if !visited.insert(path.clone()) {
                        return Err(ParseError::RecursiveInclude(path));
                    }
                    let pats = parse_list_file(&path, from0)?;
                    let mut dirs: Vec<String> = Vec::new();
                    for pat in pats {
                        let anchored = if pat.starts_with('/') {
                            pat.clone()
                        } else {
                            format!("/{}", pat)
                        };
                        let trimmed = anchored.trim_end_matches('/');
                        let list_parent = path.parent().unwrap_or_else(|| Path::new("."));
                        let fs_path = if Path::new(&pat).is_absolute() {
                            PathBuf::from(pat.trim_end_matches('/'))
                        } else {
                            list_parent.join(pat.trim_end_matches('/'))
                        };
                        let is_dir = pat.ends_with('/') || fs_path.is_dir();
                        let parts: Vec<&str> =
                            trimmed.split('/').filter(|s| !s.is_empty()).collect();
                        let mut prefix = String::new();
                        let mut ancestors: Vec<String> = Vec::new();
                        for part in parts.iter() {
                            prefix.push('/');
                            prefix.push_str(part);
                            ancestors.push(prefix.clone());
                        }
                        let dir_count = ancestors.len().saturating_sub(1);
                        for anc in ancestors.iter().take(dir_count) {
                            let matcher = compile_glob(anc)?;
                            let data = RuleData {
                                matcher,
                                invert: false,
                                flags: RuleFlags::default(),
                                source: Some(path.clone()),
                                dir_only: true,
                                has_slash: true,
                                pattern: anc.clone(),
                            };
                            rules.push(Rule::ImpliedDir(data));
                            if !dirs.contains(anc) {
                                dirs.push(anc.clone());
                            }
                        }
                        if let Some(last) = ancestors.last() {
                            if is_dir {
                                let glob = format!("{last}/**");
                                let matcher = compile_glob(&glob)?;
                                let data = RuleData {
                                    matcher,
                                    invert: false,
                                    flags: RuleFlags::default(),
                                    source: Some(path.clone()),
                                    dir_only: false,
                                    has_slash: true,
                                    pattern: glob.clone(),
                                };
                                rules.push(Rule::Include(data));

                                let matcher = compile_glob(last)?;
                                let data = RuleData {
                                    matcher,
                                    invert: false,
                                    flags: RuleFlags::default(),
                                    source: Some(path.clone()),
                                    dir_only: true,
                                    has_slash: true,
                                    pattern: last.to_string(),
                                };
                                rules.push(Rule::Include(data));
                                if !dirs.contains(last) {
                                    dirs.push(last.clone());
                                }
                            } else {
                                let line = if from0 {
                                    format!("+{last}\n")
                                } else {
                                    format!("+ {last}\n")
                                };
                                rules.extend(parse_with_options(
                                    &line,
                                    from0,
                                    visited,
                                    depth + 1,
                                    Some(path.clone()),
                                )?);
                            }
                        }
                    }
                    for d in dirs {
                        let line = if from0 {
                            format!("-{d}/*\n")
                        } else {
                            format!("- {d}/*\n")
                        };
                        rules.extend(parse_with_options(
                            &line,
                            from0,
                            visited,
                            depth + 1,
                            Some(path.clone()),
                        )?);
                    }
                    rules.extend(parse_with_options(
                        "- /**\n",
                        from0,
                        visited,
                        depth + 1,
                        Some(path.clone()),
                    )?);
                    continue;
                }
                "merge" => {
                    let path = PathBuf::from(rest);
                    if !visited.insert(path.clone()) {
                        return Err(ParseError::RecursiveInclude(path));
                    }
                    let sub = match parse_file(&path, from0, visited, depth + 1) {
                        Ok(r) => r,
                        Err(ParseError::Io(_)) => {
                            return Err(ParseError::InvalidRule(raw_line.to_string()));
                        }
                        Err(e) => return Err(e),
                    };
                    rules.extend(sub);
                    continue;
                }
                "dir-merge" => {
                    let anchored = rest.starts_with('/');
                    let fname = if anchored {
                        rest.trim_start_matches('/').to_string()
                    } else {
                        rest.to_string()
                    };
                    rules.push(Rule::DirMerge(PerDir {
                        file: fname,
                        anchored,
                        root_only: anchored,
                        inherit: true,
                        cvs: false,
                        word_split: false,
                        sign: None,
                        flags: RuleFlags::default(),
                    }));
                    continue;
                }
                _ => (None, String::new(), rest),
            }
        };

        if matches!(kind, Some(RuleKind::Exclude)) && mods.contains('C') && rest.is_empty() {
            rules.extend(default_cvs_rules()?);
            continue;
        }

        let kind = match kind {
            Some(k) => k,
            None => return Err(ParseError::InvalidRule(raw_line.to_string())),
        };
        if rest.is_empty() {
            return Err(ParseError::InvalidRule(raw_line.to_string()));
        }

        let flags = RuleFlags::from_mods(&mods);

        let mut pattern = rest.to_string();
        let mut has_anchor = false;
        while pattern.starts_with("./") {
            has_anchor = true;
            pattern = pattern[2..].to_string();
        }
        if mods.contains('/') && !pattern.starts_with('/') {
            pattern = format!("/{}", pattern);
        }
        if has_anchor && !pattern.starts_with('/') {
            pattern = format!("/{}", pattern);
        }
        if pattern.contains('/') && !pattern.starts_with('/') {
            pattern.insert(0, '/');
        }
        let anchored = pattern.starts_with('/') || pattern.contains('/');
        let dir_all = pattern.ends_with("/***");
        let dir_only = !dir_all && pattern.ends_with('/');
        let mut base = pattern.trim_start_matches('/').to_string();
        if dir_all {
            base = base.trim_end_matches("/***").to_string();
        } else if dir_only {
            base = base.trim_end_matches('/').to_string();
        }
        let base_for_ancestors = base.clone();
        let bases: Vec<String> = if !anchored && !base.starts_with("**/") && base != "**" {
            vec![base.clone(), format!("**/{}", base)]
        } else {
            vec![base.clone()]
        };

        if matches!(kind, RuleKind::Include) {
            if base_for_ancestors.contains('/')
                && !base_for_ancestors.contains('*')
                && !base_for_ancestors.contains('?')
                && !base_for_ancestors.contains('[')
                && !base_for_ancestors.contains('{')
            {
                let parts: Vec<&str> = base_for_ancestors.split('/').collect();
                for i in 1..parts.len() {
                    let ancestor = parts[..i].join("/");
                    let ancestor_bases: Vec<String> = if !anchored && ancestor != "**" {
                        vec![ancestor.clone(), format!("**/{}", ancestor)]
                    } else {
                        vec![ancestor]
                    };
                    for pat in ancestor_bases {
                        for exp in expand_braces(&pat)? {
                            let matcher = compile_glob(&exp)?;
                            let data = RuleData {
                                matcher,
                                invert: mods.contains('!'),
                                flags: flags.clone(),
                                source: source.clone(),
                                dir_only: true,
                                has_slash: exp.contains('/'),
                                pattern: exp.clone(),
                            };
                            rules.push(Rule::Include(data));
                        }
                    }
                }
            }
        }

        let mut pats: Vec<(String, bool)> = Vec::new();
        for b in bases {
            let b = if anchored { format!("/{}", b) } else { b };
            if dir_all || dir_only {
                pats.push((b.clone(), false));
                pats.push((format!("{}/**", b), false));
            } else {
                pats.push((b, false));
            }
        }

        let invert = mods.contains('!');
        for (pat, dir_only) in pats {
            for exp in expand_braces(&pat)? {
                let matcher = compile_glob(&exp)?;
                let data = RuleData {
                    matcher,
                    invert,
                    flags: flags.clone(),
                    source: source.clone(),
                    dir_only,
                    has_slash: exp.contains('/'),
                    pattern: exp.clone(),
                };
                match kind {
                    RuleKind::Include => rules.push(Rule::Include(data)),
                    RuleKind::Exclude => rules.push(Rule::Exclude(data)),
                    RuleKind::Protect => rules.push(Rule::Protect(data)),
                }
            }
        }
    }

    Ok(rules)
}

pub fn parse(
    input: &str,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<Vec<Rule>, ParseError> {
    parse_with_options(input, false, visited, depth, None)
}

pub const CVS_DEFAULTS: &[&str] = &[
    "RCS",
    "SCCS",
    "CVS",
    "CVS.adm",
    "RCSLOG",
    "cvslog.*",
    "tags",
    "TAGS",
    ".make.state",
    ".nse_depinfo",
    "*~",
    "#*",
    ".#*",
    ",*",
    "_$*",
    "*$",
    "*.old",
    "*.bak",
    "*.BAK",
    "*.orig",
    "*.rej",
    ".del-*",
    "*.a",
    "*.olb",
    "*.o",
    "*.obj",
    "*.so",
    "*.exe",
    "*.Z",
    "*.elc",
    "*.ln",
    "core",
    ".svn/",
    ".git/",
    ".hg/",
    ".bzr/",
];

pub fn default_cvs_rules() -> Result<Vec<Rule>, ParseError> {
    fn append_rule(buf: &mut String, tok: &str) {
        if tok.is_empty() {
            return;
        }
        let mut chars = tok.chars();
        if let Some(first) = chars.next() {
            if matches!(first, '+' | '-' | 'P' | 'p' | 'S' | 'H' | 'R' | '!') {
                let rest = chars.as_str();
                let mods_len = rest.chars().take_while(|c| "!/Csrpx".contains(*c)).count();
                let (mods, pat) = rest.split_at(mods_len);
                let mut mods = mods.to_string();
                if !mods.contains('p') {
                    mods.push('p');
                }
                if pat.is_empty() {
                    buf.push(first);
                    buf.push_str(&mods);
                } else {
                    buf.push(first);
                    buf.push_str(&mods);
                    buf.push(' ');
                    buf.push_str(pat);
                }
            } else {
                buf.push_str("-p ");
                buf.push_str(tok);
            }
            buf.push('\n');
        }
    }

    let mut buf = String::new();
    for pat in CVS_DEFAULTS {
        append_rule(&mut buf, pat);
    }
    let mut rules = parse(&buf, &mut HashSet::new(), 0)?;
    for rule in &mut rules {
        match rule {
            Rule::Include(d) | Rule::Exclude(d) | Rule::Protect(d) | Rule::ImpliedDir(d) => {
                d.flags.perishable = true
            }
            Rule::DirMerge(pd) => pd.flags.perishable = true,
            _ => {}
        }
    }

    if let Ok(home) = env::var("HOME") {
        let path = Path::new(&home).join(".cvsignore");
        if let Ok(content) = fs::read_to_string(path) {
            let mut buf = String::new();
            for tok in content.split_whitespace() {
                append_rule(&mut buf, tok);
            }
            let mut v = parse(&buf, &mut HashSet::new(), 0)?;
            for rule in &mut v {
                match rule {
                    Rule::Include(d)
                    | Rule::Exclude(d)
                    | Rule::Protect(d)
                    | Rule::ImpliedDir(d) => {
                        d.flags.perishable = true;
                    }
                    Rule::DirMerge(pd) => pd.flags.perishable = true,
                    _ => {}
                }
            }
            rules.append(&mut v);
        }
    }

    if let Ok(envpats) = env::var("CVSIGNORE") {
        let mut buf = String::new();
        for tok in envpats.split_whitespace() {
            append_rule(&mut buf, tok);
        }
        let mut v = parse(&buf, &mut HashSet::new(), 0)?;
        for rule in &mut v {
            match rule {
                Rule::Include(d) | Rule::Exclude(d) | Rule::Protect(d) | Rule::ImpliedDir(d) => {
                    d.flags.perishable = true;
                }
                Rule::DirMerge(pd) => pd.flags.perishable = true,
                _ => {}
            }
        }
        rules.append(&mut v);
    }

    rules.push(Rule::DirMerge(PerDir {
        file: ".cvsignore".into(),
        anchored: true,
        root_only: false,
        inherit: false,
        cvs: true,
        word_split: false,
        sign: None,
        flags: RuleFlags {
            perishable: true,
            ..RuleFlags::default()
        },
    }));

    Ok(rules)
}

fn trim_newlines(mut s: &[u8]) -> &[u8] {
    while let Some((&last, rest)) = s.split_last() {
        if last == b'\n' || last == b'\r' {
            s = rest;
        } else {
            break;
        }
    }
    s
}

pub fn parse_list(input: &[u8], from0: bool) -> Vec<String> {
    if from0 {
        input
            .split(|b| *b == 0)
            .filter_map(|s| {
                let s = trim_newlines(s);
                if s.is_empty() {
                    return None;
                }
                Some(String::from_utf8_lossy(s).to_string())
            })
            .collect()
    } else {
        let s = String::from_utf8_lossy(input);
        s.lines().filter_map(decode_line).collect()
    }
}

fn read_path_or_stdin(path: &Path) -> io::Result<Vec<u8>> {
    if path == Path::new("-") {
        let mut buf = Vec::new();
        std::io::stdin().lock().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        fs::read(path)
    }
}

pub fn parse_list_file(path: &Path, from0: bool) -> Result<Vec<String>, ParseError> {
    let data = read_path_or_stdin(path)?;
    Ok(parse_list(&data, from0))
}

pub fn parse_from_bytes(
    input: &[u8],
    from0: bool,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    source: Option<PathBuf>,
) -> Result<Vec<Rule>, ParseError> {
    if from0 {
        let mut rules = Vec::new();
        for part in input.split(|b| *b == 0) {
            let part = trim_newlines(part);
            if part.is_empty() {
                continue;
            }
            let line = String::from_utf8_lossy(part).to_string();
            if line.is_empty() {
                continue;
            }
            let mut buf = line;
            buf.push('\n');
            rules.extend(parse_with_options(
                &buf,
                from0,
                visited,
                depth,
                source.clone(),
            )?);
        }
        Ok(rules)
    } else {
        let s = String::from_utf8_lossy(input);
        parse_with_options(&s, from0, visited, depth, source)
    }
}

pub fn parse_file(
    path: &Path,
    from0: bool,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<Vec<Rule>, ParseError> {
    let data = read_path_or_stdin(path)?;
    let from0 = from0 || path == Path::new("-");
    let source = if path == Path::new("-") {
        None
    } else {
        Some(path.to_path_buf())
    };
    parse_from_bytes(&data, from0, visited, depth, source)
}

pub fn rooted_and_parents(pat: &str) -> (String, Vec<String>) {
    let rooted = pat.trim_start_matches('/').to_string();
    let trimmed = rooted.trim_end_matches('/');
    let mut base = String::new();
    let mut dirs = Vec::new();
    let mut finished = true;
    for part in trimmed.split('/') {
        if part == "**" {
            finished = false;
            break;
        }
        if !base.is_empty() {
            base.push('/');
        }
        base.push_str(part);
        let has_glob = part.contains(['*', '?', '[', ']']);
        dirs.push(format!("{base}/"));
        if has_glob {
            finished = false;
            break;
        }
    }
    if !trimmed.contains('/') {
        dirs.clear();
    } else if finished {
        dirs.pop();
    }
    (rooted, dirs)
}

pub fn parse_rule_list_from_bytes(
    input: &[u8],
    from0: bool,
    sign: char,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    source: Option<PathBuf>,
) -> Result<Vec<Rule>, ParseError> {
    let pats = parse_list(input, from0);
    let mut rules = Vec::new();
    for pat in pats {
        if pat.is_empty() {
            continue;
        }
        if sign == '+' && !pat.starts_with(['+', '-', ':']) {
            let (rooted, parents) = rooted_and_parents(&pat);
            let has_globstar = rooted.contains("**");

            let line = if from0 {
                format!("+./{rooted}\n")
            } else {
                format!("+ ./{rooted}\n")
            };
            rules.extend(parse_with_options(
                &line,
                from0,
                visited,
                depth,
                source.clone(),
            )?);

            let skip_exclude = rooted
                .rsplit('/')
                .next()
                .is_some_and(|seg| seg.chars().all(|c| c == '*'));
            for dir in &parents {
                let glob = format!("/{}", dir.trim_end_matches('/'));
                let matcher = compile_glob(&glob)?;
                let data = RuleData {
                    matcher,
                    invert: false,
                    flags: RuleFlags::default(),
                    source: source.clone(),
                    dir_only: false,
                    has_slash: true,
                    pattern: glob.clone(),
                };
                rules.push(Rule::ImpliedDir(data));
            }

            if !skip_exclude && !has_globstar {
                for dir in &parents {
                    let exc_pat = format!("/{}*", dir);
                    let matcher = compile_glob(&exc_pat)?;
                    let data = RuleData {
                        matcher,
                        invert: false,
                        flags: RuleFlags::default(),
                        source: source.clone(),
                        dir_only: false,
                        has_slash: true,
                        pattern: exc_pat.clone(),
                    };
                    rules.push(Rule::Exclude(data));
                }
            }
        } else {
            let line = if from0 {
                format!("{sign}{pat}\n")
            } else {
                format!("{sign} {pat}\n")
            };
            rules.extend(parse_with_options(
                &line,
                from0,
                visited,
                depth,
                source.clone(),
            )?);
        }
    }
    Ok(rules)
}

pub fn parse_rule_list_file(
    path: &Path,
    from0: bool,
    sign: char,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<Vec<Rule>, ParseError> {
    let data = read_path_or_stdin(path)?;
    parse_rule_list_from_bytes(&data, from0, sign, visited, depth, Some(path.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::read_path_or_stdin;
    use std::io::{Seek, SeekFrom, Write};
    #[cfg(unix)]
    use std::os::unix::io::IntoRawFd;
    use std::path::Path;
    use tempfile::{NamedTempFile, tempfile};

    #[test]
    fn reads_from_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "hello world").unwrap();
        let data = read_path_or_stdin(tmp.path()).unwrap();
        assert_eq!(data, b"hello world");
    }

    #[cfg(unix)]
    #[test]
    fn reads_from_stdin() {
        let mut file = tempfile().unwrap();
        write!(file, "stdin data").unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();

        let stdin_fd = unsafe { libc::dup(0) };
        assert!(stdin_fd >= 0);

        let file_fd = file.into_raw_fd();
        assert!(unsafe { libc::dup2(file_fd, 0) } >= 0);
        unsafe { libc::close(file_fd) };

        let data = read_path_or_stdin(Path::new("-")).unwrap();

        assert!(unsafe { libc::dup2(stdin_fd, 0) } >= 0);
        unsafe { libc::close(stdin_fd) };

        assert_eq!(data, b"stdin data");
    }
}
