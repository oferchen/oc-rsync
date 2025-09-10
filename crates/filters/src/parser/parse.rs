// crates/filters/src/parser/parse.rs

use crate::{
    perdir::PerDir,
    rule::{Rule, RuleData, RuleFlags},
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    MAX_PARSE_DEPTH, ParseError, compile_glob, decode_line, default_cvs_rules, expand_braces,
    parse_file, parse_from_bytes, parse_list_file, parse_rule_list_file, trim_newlines,
};

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

enum RuleKind {
    Include,
    Exclude,
    Protect,
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
