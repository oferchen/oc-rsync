// crates/filters/src/matcher/merge.rs
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

use super::{
    core::{Cached, Matcher},
    path::rule_matches,
};
use crate::{
    parser::{MAX_PARSE_DEPTH, ParseError, parse_with_options},
    perdir::PerDir,
    rule::Rule,
};
use logging::InfoFlag;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

impl Matcher {
    pub(crate) fn dir_rules_at(
        &self,
        dir: &Path,
        for_delete: bool,
        xattr: bool,
    ) -> Result<Vec<(usize, Rule)>, ParseError> {
        if let Some(root) = &self.root {
            if !dir.starts_with(root) {
                return Ok(Vec::new());
            }
        }

        let mut per_dirs: Vec<(usize, PerDir)> = Vec::new();
        let mut ancestors = Vec::new();
        let mut anc = dir.parent();
        while let Some(a) = anc {
            ancestors.push(a.to_path_buf());
            anc = a.parent();
        }
        ancestors.reverse();
        for a in ancestors {
            if let Some(extra) = self.extra_per_dir.borrow().get(&a) {
                per_dirs.extend(extra.clone());
            }
        }
        per_dirs.extend(self.per_dir.clone());
        per_dirs.sort_by_key(|(idx, _)| *idx);

        let mut combined = Vec::new();
        let mut new_merges = Vec::new();

        for (idx, pd) in per_dirs {
            if !pd.flags.applies(for_delete, xattr) {
                continue;
            }
            let path = if pd.root_only {
                if let Some(root) = &self.root {
                    root.join(&pd.file)
                } else {
                    dir.join(&pd.file)
                }
            } else {
                dir.join(&pd.file)
            };

            let rel = if pd.root_only {
                None
            } else {
                self.root
                    .as_ref()
                    .and_then(|r| dir.strip_prefix(r).ok())
                    .map(|p| p.to_path_buf())
            };

            let key = (path.clone(), pd.sign, pd.word_split);
            let meta = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => {
                    continue;
                }
            };
            let mtime = meta.modified().ok();
            let len = meta.len();

            let state = {
                let cache = self.cached.borrow();
                if let Some(c) = cache.get(&key) {
                    if c.mtime == mtime && c.len == len {
                        Some(c.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            let state = if let Some(cached) = state {
                cached
            } else {
                let mut visited = HashSet::new();
                visited.insert(path.clone());
                let (rules, merges) = self.load_merge_file(
                    dir,
                    &path,
                    rel.as_deref(),
                    pd.cvs,
                    pd.word_split,
                    pd.sign,
                    &mut visited,
                    0,
                    idx,
                )?;
                let cached = Cached {
                    rules: rules.clone(),
                    merges: merges.clone(),
                    mtime,
                    len,
                };
                self.cached.borrow_mut().insert(key.clone(), cached.clone());
                cached
            };

            for (ridx, rule) in state.rules.iter() {
                let mut r = rule.clone();
                match &mut r {
                    Rule::Include(d)
                    | Rule::Exclude(d)
                    | Rule::Protect(d)
                    | Rule::ImpliedDir(d) => {
                        d.flags = d.flags.union(&pd.flags);
                    }
                    Rule::DirMerge(sub) => {
                        sub.flags = sub.flags.union(&pd.flags);
                    }
                    _ => {}
                }
                combined.push((*ridx, r));
            }
            for (midx, mut mpd) in state.merges.iter().cloned() {
                mpd.flags = mpd.flags.union(&pd.flags);
                new_merges.push((midx, mpd));
            }
        }

        if !new_merges.is_empty() {
            let mut map = self.extra_per_dir.borrow_mut();
            let entry = map.entry(dir.to_path_buf()).or_default();
            for (idx, pd) in new_merges {
                if pd.inherit {
                    if let Some(pos) = entry.iter().position(|(_, p)| p == &pd) {
                        entry.remove(pos);
                    }
                    entry.push((idx, pd));
                }
            }
        }

        Ok(combined)
    }

    pub(crate) fn load_merge_file(
        &self,
        dir: &Path,
        path: &Path,
        rel: Option<&Path>,
        cvs: bool,
        word_split: bool,
        sign: Option<char>,
        visited: &mut HashSet<PathBuf>,
        depth: usize,
        index: usize,
    ) -> Result<(Vec<(usize, Rule)>, Vec<(usize, PerDir)>), ParseError> {
        if depth >= MAX_PARSE_DEPTH {
            return Err(ParseError::RecursionLimit);
        }

        let mut content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok((Vec::new(), Vec::new()));
            }
            Err(err) => {
                tracing::warn!(
                    target: InfoFlag::Filter.target(),
                    ?path,
                    ?err,
                    "unable to open",
                );
                return Err(ParseError::Io(err));
            }
        };

        let adjusted = if cvs {
            let rel_str = rel
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let mut buf = String::new();
            for token in content.split_whitespace() {
                if token.is_empty() || token.starts_with('#') {
                    continue;
                }
                if token.starts_with('!') {
                    return Err(ParseError::InvalidRule(token.to_string()));
                }
                let tok = token.trim_start_matches('/');
                let pat = if rel_str.is_empty() {
                    format!("/{tok}")
                } else {
                    format!("/{rel_str}/{tok}")
                };
                buf.push_str("- ");
                buf.push_str(&pat);
                buf.push('\n');
            }
            buf
        } else {
            if word_split {
                let mut buf = String::new();
                if self.from0 {
                    for token in content.split('\0') {
                        if token.is_empty() {
                            continue;
                        }
                        buf.push_str(token);
                        buf.push('\n');
                    }
                } else {
                    for token in content.split_whitespace() {
                        buf.push_str(token);
                        buf.push('\n');
                    }
                }
                content = buf;
            }
            if let Some(ch) = sign {
                fn split_mods(s: &str) -> (&str, &str) {
                    let s = s.trim_start();
                    let mut idx = 0;
                    let bytes = s.as_bytes();
                    if bytes.first() == Some(&b',') {
                        idx += 1;
                    }
                    while let Some(&c) = bytes.get(idx) {
                        if b"/!Csrpx".contains(&c) {
                            idx += 1;
                        } else {
                            break;
                        }
                    }
                    let mods = &s[..idx];
                    let rest = s[idx..].trim_start();
                    (mods, rest)
                }

                let rel_str = rel.map(|p| p.to_string_lossy().to_string());
                let mut buf = String::new();
                let lines: Box<dyn Iterator<Item = &str>> = if self.from0 {
                    Box::new(content.split('\0'))
                } else {
                    Box::new(content.lines())
                };
                for raw_line in lines {
                    let line = raw_line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if line == "!" {
                        buf.push_str("!\n");
                        continue;
                    }
                    let (prefix, _rest) = match line.chars().next() {
                        Some(c @ ('+' | '-' | 'P' | 'p' | 'S' | 'H' | 'R')) => {
                            (Some(c), &line[1..])
                        }
                        _ => (None, line),
                    };
                    let rest = if matches!(
                        line.chars().next(),
                        Some('+' | '-' | 'P' | 'p' | 'S' | 'H' | 'R')
                    ) {
                        &line[1..]
                    } else {
                        line
                    };
                    let (mods, pat) = split_mods(rest);
                    let new_pat = if let Some(rel_str) = &rel_str {
                        if pat.starts_with('/') {
                            format!("/{}/{}", rel_str, pat.trim_start_matches('/'))
                        } else {
                            format!("{}/{}", rel_str, pat)
                        }
                    } else {
                        pat.to_string()
                    };
                    buf.push(prefix.unwrap_or(ch));
                    buf.push_str(mods);
                    buf.push(' ');
                    buf.push_str(&new_pat);
                    buf.push('\n');
                }
                buf
            } else if let Some(rel) = rel {
                let rel_str = rel.to_string_lossy();
                let mut buf = String::new();
                let lines: Box<dyn Iterator<Item = &str>> = if self.from0 {
                    Box::new(content.split('\0'))
                } else {
                    Box::new(content.lines())
                };
                for raw_line in lines {
                    let line = raw_line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if line == "!" {
                        buf.push_str("!\n");
                        continue;
                    }
                    let (kind, pat): (String, &str) = if let Some(rest) = line.strip_prefix('+') {
                        ("+".to_string(), rest.trim_start())
                    } else if let Some(rest) = line.strip_prefix('-') {
                        ("-".to_string(), rest.trim_start())
                    } else if let Some(rest) =
                        line.strip_prefix('P').or_else(|| line.strip_prefix('p'))
                    {
                        ("P".to_string(), rest.trim_start())
                    } else if let Some(rest) = line.strip_prefix('S') {
                        ("S".to_string(), rest.trim_start())
                    } else if let Some(rest) = line.strip_prefix('H') {
                        ("H".to_string(), rest.trim_start())
                    } else if let Some(rest) = line.strip_prefix('R') {
                        ("R".to_string(), rest.trim_start())
                    } else if let Some(rest) = line.strip_prefix("E+") {
                        ("E+".to_string(), rest.trim_start())
                    } else if let Some(rest) = line.strip_prefix("E-") {
                        ("E-".to_string(), rest.trim_start())
                    } else {
                        buf.push_str(raw_line);
                        buf.push('\n');
                        continue;
                    };
                    let new_pat = if pat.starts_with('/') {
                        format!("/{}/{}", rel_str, pat.trim_start_matches('/'))
                    } else {
                        format!("{}/{}", rel_str, pat)
                    };
                    buf.push_str(&kind);
                    buf.push(' ');
                    buf.push_str(&new_pat);
                    buf.push('\n');
                }
                buf
            } else {
                content
            }
        };

        let mut rules = Vec::new();
        let mut merges = Vec::new();

        fn is_excluded(rules: &[(usize, Rule)], path: &Path, is_dir: bool) -> bool {
            let mut state: Option<bool> = None;
            for (_, r) in rules {
                match r {
                    Rule::Clear => state = None,
                    Rule::Include(d) | Rule::Protect(d) | Rule::ImpliedDir(d) => {
                        if d.dir_only && !is_dir {
                            continue;
                        }
                        let matched = rule_matches(d, path, is_dir);
                        if (d.invert && !matched)
                            || (!d.invert && matched)
                            || (is_dir && d.may_match_descendant(path))
                        {
                            state = Some(true);
                            break;
                        }
                    }
                    Rule::Exclude(d) => {
                        if d.dir_only && !is_dir {
                            continue;
                        }
                        let matched = rule_matches(d, path, is_dir);
                        if (d.invert && !matched)
                            || (!d.invert && matched)
                            || (is_dir && d.may_match_descendant(path))
                        {
                            state = Some(false);
                            break;
                        }
                    }
                    Rule::DirMerge(_)
                    | Rule::Existing
                    | Rule::NoExisting
                    | Rule::PruneEmptyDirs
                    | Rule::NoPruneEmptyDirs => {}
                }
            }
            matches!(state, Some(false))
        }

        let parsed = parse_with_options(
            &adjusted,
            self.from0,
            visited,
            depth + 1,
            Some(path.to_path_buf()),
        )?;

        let mut next_index = index;

        for r in parsed {
            match r {
                Rule::DirMerge(pd) => {
                    let nested = if pd.root_only {
                        if let Some(root) = &self.root {
                            root.join(&pd.file)
                        } else {
                            dir.join(&pd.file)
                        }
                    } else {
                        dir.join(&pd.file)
                    };
                    let dir_for_rule = nested.parent().unwrap_or(&nested).to_path_buf();
                    if is_excluded(&rules, &dir_for_rule, true) {
                        continue;
                    }
                    merges.push((next_index, pd.clone()));
                    if !visited.insert(nested.clone()) {
                        return Err(ParseError::RecursiveInclude(nested));
                    }
                    let rel2 = if pd.root_only {
                        None
                    } else {
                        let mut base = rel.map(|p| p.to_path_buf()).unwrap_or_default();
                        if let Some(parent) = Path::new(&pd.file).parent() {
                            if !parent.as_os_str().is_empty() {
                                base = if base.as_os_str().is_empty() {
                                    parent.to_path_buf()
                                } else {
                                    base.join(parent)
                                };
                            }
                        }
                        if base.as_os_str().is_empty() {
                            None
                        } else {
                            Some(base)
                        }
                    };
                    let (mut nr, mut nm) = self.load_merge_file(
                        dir,
                        &nested,
                        rel2.as_deref(),
                        pd.cvs,
                        pd.word_split,
                        pd.sign,
                        visited,
                        depth + 1,
                        next_index,
                    )?;
                    rules.append(&mut nr);
                    merges.append(&mut nm);
                }
                other => rules.push((next_index, other)),
            }
            next_index += 1;
        }

        Ok((rules, merges))
    }
}
