// crates/filters/src/parser/parse/util.rs

use crate::rule::{Rule, RuleData, RuleFlags};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::parser::{ParseError, compile_glob, parse_from_bytes, parse_list_file, trim_newlines};

pub(crate) fn handle_dot_prefix(
    raw_line: &str,
    r: &str,
    from0: bool,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    source: &Option<PathBuf>,
    rules: &mut Vec<Rule>,
) -> Result<(), ParseError> {
    use super::token::split_mods;
    let (m, file) = split_mods(r, "-+Cenw/!srpx");
    if file.is_empty() {
        return Err(ParseError::InvalidRule(raw_line.to_string()));
    }
    let path = PathBuf::from(file);
    if !visited.insert(path.clone()) {
        return Err(ParseError::RecursiveInclude(path));
    }
    let data = fs::read(&path).map_err(|_| ParseError::InvalidRule(raw_line.to_string()))?;
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
        super::rule::parse_with_options(&content, from0, visited, depth + 1, Some(path.clone()))?
    };
    if m.contains('s') || m.contains('r') || m.contains('p') || m.contains('x') {
        let inherited = RuleFlags::from_mods(m);
        for rule in &mut sub {
            if let Rule::Include(d) | Rule::Exclude(d) | Rule::Protect(d) | Rule::ImpliedDir(d) =
                rule
            {
                d.flags = d.flags.union(&inherited);
            }
        }
    }
    rules.extend(sub);
    if m.contains('e') {
        let pat = format!("**/{file}");
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
    Ok(())
}

pub(crate) fn handle_files_from(
    path: &Path,
    from0: bool,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    rules: &mut Vec<Rule>,
) -> Result<(), ParseError> {
    let pats = parse_list_file(path, from0)?;
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
        let parts: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
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
                source: Some(path.to_path_buf()),
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
                    source: Some(path.to_path_buf()),
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
                    source: Some(path.to_path_buf()),
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
                rules.extend(super::rule::parse_with_options(
                    &line,
                    from0,
                    visited,
                    depth + 1,
                    Some(path.to_path_buf()),
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
        rules.extend(super::rule::parse_with_options(
            &line,
            from0,
            visited,
            depth + 1,
            Some(path.to_path_buf()),
        )?);
    }
    rules.extend(super::rule::parse_with_options(
        "- /**\n",
        from0,
        visited,
        depth + 1,
        Some(path.to_path_buf()),
    )?);
    Ok(())
}
