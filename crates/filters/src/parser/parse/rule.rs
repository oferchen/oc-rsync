// crates/filters/src/parser/parse/rule.rs

use crate::{
    parser::{
        MAX_PARSE_DEPTH, ParseError, compile_glob, decode_line, default_cvs_rules, expand_braces,
        parse_file, parse_from_bytes, parse_rule_list_file,
    },
    perdir::PerDir,
    rule::{Rule, RuleData, RuleFlags},
};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use super::token::{MergeModifier, RuleKind, split_mods};
use super::util::{handle_dot_prefix, handle_files_from};

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

        if let Some(r) = line.strip_prefix('.') {
            handle_dot_prefix(raw_line, r, from0, visited, depth, &source, &mut rules)?;
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
                    handle_files_from(&path, from0, visited, depth, &mut rules)?;
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
