// crates/filters/src/parser/list.rs

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::{
    ParseError, compile_glob, decode_line, parse::parse_with_options, read_path_or_stdin,
    trim_newlines,
};
use crate::rule::{Rule, RuleData, RuleFlags};

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
