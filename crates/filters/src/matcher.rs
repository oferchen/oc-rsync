// crates/filters/src/matcher.rs — extracted from lib.rs to handle rule matching and caching; public API preserved via re-exports.
#![allow(clippy::collapsible_if)]

use crate::{
    parser::{MAX_PARSE_DEPTH, ParseError, parse_with_options},
    perdir::PerDir,
    rule::{Rule, RuleData},
    stats::FilterStats,
};
use logging::InfoFlag;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone)]
struct Cached {
    rules: Vec<(usize, Rule)>,
    merges: Vec<(usize, PerDir)>,
    mtime: Option<SystemTime>,
    len: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MatchResult {
    pub include: bool,
    pub descend: bool,
}

fn rule_matches(data: &RuleData, path: &Path, is_dir: bool) -> bool {
    let mut matched = data.is_match(path, is_dir);
    let pat_core = data
        .pattern
        .trim_start_matches('/')
        .trim_start_matches("./");
    if matched && pat_core.starts_with("**/") {
        let rest = &pat_core[3..];
        if rest.contains('*')
            && !rest.contains("**")
            && path.components().count() > 1
            && rest != "*"
        {
            matched = false;
        }
    } else if matched && pat_core.contains('*') && !pat_core.contains("**") {
        if data.has_slash {
            let pat_segments = pat_core
                .trim_matches('/')
                .split('/')
                .filter(|s| !s.is_empty() && *s != ".")
                .count();
            let path_segments = path.components().count();
            if path_segments != pat_segments {
                matched = false;
            }
        } else if path.components().count() > 1 && pat_core != "*" {
            matched = false;
        }
    }
    matched
}

#[derive(Clone, Default)]
pub struct Matcher {
    root: Option<PathBuf>,
    rules: Vec<(usize, Rule)>,
    per_dir: Vec<(usize, PerDir)>,
    extra_per_dir: RefCell<HashMap<PathBuf, Vec<(usize, PerDir)>>>,
    #[allow(clippy::type_complexity)]
    cached: RefCell<HashMap<(PathBuf, Option<char>, bool), Cached>>,
    existing: bool,
    prune_empty_dirs: bool,
    from0: bool,
    no_implied_dirs: bool,
    stats: RefCell<FilterStats>,
}

impl Matcher {
    pub fn new(rules: Vec<Rule>) -> Self {
        let mut per_dir = Vec::new();
        let mut global = Vec::new();
        let mut existing = false;
        let mut prune_empty_dirs = false;
        for (idx, r) in rules.into_iter().enumerate() {
            match r {
                Rule::DirMerge(p) => per_dir.push((idx, p)),
                Rule::Existing => existing = true,
                Rule::NoExisting => existing = false,
                Rule::PruneEmptyDirs => prune_empty_dirs = true,
                Rule::NoPruneEmptyDirs => prune_empty_dirs = false,
                other => global.push((idx, other)),
            }
        }
        Self {
            root: None,
            rules: global,
            per_dir,
            extra_per_dir: RefCell::new(HashMap::new()),
            cached: RefCell::new(HashMap::new()),
            existing,
            prune_empty_dirs,
            from0: false,
            no_implied_dirs: false,
            stats: RefCell::new(FilterStats::default()),
        }
    }

    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    pub fn with_existing(mut self) -> Self {
        self.existing = true;
        self
    }

    pub fn with_prune_empty_dirs(mut self) -> Self {
        self.prune_empty_dirs = true;
        self
    }

    pub fn with_from0(mut self) -> Self {
        self.from0 = true;
        self
    }

    pub fn with_no_implied_dirs(mut self) -> Self {
        self.no_implied_dirs = true;
        self
    }

    pub fn is_included<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), false, false).map(|r| r.include)
    }

    pub fn is_included_with_dir<P: AsRef<Path>>(&self, path: P) -> Result<MatchResult, ParseError> {
        self.check(path.as_ref(), false, false)
    }

    pub fn is_included_for_delete<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), true, false).map(|r| r.include)
    }

    pub fn is_included_for_delete_with_dir<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<MatchResult, ParseError> {
        self.check(path.as_ref(), true, false)
    }

    pub fn is_xattr_included<P: AsRef<Path>>(&self, name: P) -> Result<bool, ParseError> {
        self.check(name.as_ref(), false, true).map(|r| r.include)
    }

    pub fn is_xattr_included_for_delete<P: AsRef<Path>>(
        &self,
        name: P,
    ) -> Result<bool, ParseError> {
        self.check(name.as_ref(), true, true).map(|r| r.include)
    }

    pub fn is_xattr_included_for_delete_with_dir<P: AsRef<Path>>(
        &self,
        name: P,
    ) -> Result<MatchResult, ParseError> {
        self.check(name.as_ref(), true, true)
    }

    pub fn preload_dir<P: AsRef<Path>>(&self, dir: P) -> Result<(), ParseError> {
        let dir = dir.as_ref();
        if let Some(root) = &self.root {
            if dir.starts_with(root) {
                let mut current = root.to_path_buf();
                let _ = self.dir_rules_at(&current, false, false)?;
                if let Ok(rel) = dir.strip_prefix(root) {
                    for comp in rel.components() {
                        current.push(comp.as_os_str());
                        let _ = self.dir_rules_at(&current, false, false)?;
                    }
                }
                return Ok(());
            }
        }
        let _ = self.dir_rules_at(dir, false, false)?;
        Ok(())
    }

    fn check(&self, path: &Path, for_delete: bool, xattr: bool) -> Result<MatchResult, ParseError> {
        if self.existing {
            if let Some(root) = &self.root {
                if !root.join(path).exists() {
                    return Ok(MatchResult {
                        include: false,
                        descend: false,
                    });
                }
            }
        }

        if path.as_os_str().is_empty() {
            return Ok(MatchResult {
                include: true,
                descend: false,
            });
        }

        let mut is_dir = false;
        if let Some(root) = &self.root {
            is_dir = root.join(path).is_dir();
        }

        let mut seq = 0usize;
        let mut active: Vec<(usize, usize, usize, Rule)> = Vec::new();

        for (idx, rule) in &self.rules {
            active.push((*idx, 0, seq, rule.clone()));
            seq += 1;
        }

        if let Some(root) = &self.root {
            let mut dirs = vec![root.clone()];
            if let Some(parent) = path.parent() {
                let iter = match parent.strip_prefix(root) {
                    Ok(rel) => rel.components(),
                    Err(_) => parent.components(),
                };
                let mut dir = root.clone();
                for comp in iter {
                    dir.push(comp.as_os_str());
                    dirs.push(dir.clone());
                }
            }

            let fname = path.file_name().map(|f| f.to_string_lossy().to_string());
            for (depth_idx, d) in dirs.iter().enumerate() {
                let depth = depth_idx + 1;
                for (idx, rule) in self.dir_rules_at(d, for_delete, xattr)? {
                    let mut idx_adj = idx;
                    if let Some(ref f) = fname {
                        let mut is_merge = self.per_dir.iter().any(|(_, pd)| pd.file == *f);
                        if !is_merge {
                            if let Some(extra) = self.extra_per_dir.borrow().get(d) {
                                is_merge = extra.iter().any(|(_, pd)| pd.file == *f);
                            }
                        }
                        if is_merge {
                            idx_adj += 2;
                        }
                    }
                    active.push((idx_adj, depth, seq, rule));
                    seq += 1;
                }
            }
        }

        active.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| b.1.cmp(&a.1))
                .then_with(|| a.2.cmp(&b.2))
        });

        let mut ordered = Vec::new();
        for (_, _, _, rule) in active {
            if let Rule::Clear = rule {
                ordered.clear();
            } else {
                ordered.push(rule);
            }
        }

        let mut include: Option<bool> = None;
        let mut descend = false;
        let mut matched_source: Option<PathBuf> = None;
        for rule in ordered.iter() {
            match rule {
                Rule::Protect(data) => {
                    if !data.flags.applies(for_delete, xattr) {
                        continue;
                    }
                    if data.dir_only {
                        if let Some(root) = &self.root {
                            if !root.join(path).is_dir() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    if for_delete && data.flags.perishable {
                        continue;
                    }
                    let matched_rule = rule_matches(data, path, is_dir);
                    let rule_match =
                        (data.invert && !matched_rule) || (!data.invert && matched_rule);
                    let may_desc = is_dir && data.may_match_descendant(path);
                    self.stats
                        .borrow_mut()
                        .record(data.source.as_deref(), rule_match);
                    if rule_match {
                        include = Some(true);
                        if may_desc {
                            descend = true;
                        }
                        matched_source = data.source.clone();
                        break;
                    } else if may_desc {
                        descend = true;
                    }
                }
                Rule::Include(data) => {
                    if !data.flags.applies(for_delete, xattr) {
                        continue;
                    }
                    if data.dir_only {
                        if let Some(root) = &self.root {
                            if !root.join(path).is_dir() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    if for_delete && data.flags.perishable {
                        continue;
                    }
                    let matched_rule = rule_matches(data, path, is_dir);
                    let rule_match =
                        (data.invert && !matched_rule) || (!data.invert && matched_rule);
                    let may_desc = is_dir && data.may_match_descendant(path);
                    self.stats
                        .borrow_mut()
                        .record(data.source.as_deref(), rule_match);
                    if rule_match {
                        include = Some(true);
                        if may_desc {
                            descend = true;
                        }
                        matched_source = data.source.clone();
                        break;
                    } else if may_desc {
                        descend = true;
                    }
                }
                Rule::ImpliedDir(data) => {
                    if !data.flags.applies(for_delete, xattr) {
                        continue;
                    }
                    if data.dir_only {
                        if let Some(root) = &self.root {
                            if !root.join(path).is_dir() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    if for_delete && data.flags.perishable {
                        continue;
                    }
                    let matched_rule = rule_matches(data, path, is_dir);
                    let rule_match =
                        (data.invert && !matched_rule) || (!data.invert && matched_rule);
                    let may_desc = is_dir && data.may_match_descendant(path);
                    self.stats
                        .borrow_mut()
                        .record(data.source.as_deref(), rule_match);
                    if rule_match {
                        if self.no_implied_dirs {
                            include.get_or_insert(false);
                            if may_desc {
                                descend = true;
                            }
                        } else {
                            include = Some(true);
                            if may_desc {
                                descend = true;
                            }
                            matched_source = data.source.clone();
                            break;
                        }
                    } else if may_desc {
                        descend = true;
                    }
                }
                Rule::Exclude(data) => {
                    if !data.flags.applies(for_delete, xattr) {
                        continue;
                    }
                    if data.dir_only {
                        if let Some(root) = &self.root {
                            if !root.join(path).is_dir() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    if for_delete && data.flags.perishable {
                        continue;
                    }
                    let matched_rule = rule_matches(data, path, is_dir);
                    let rule_match =
                        (data.invert && !matched_rule) || (!data.invert && matched_rule);
                    let may_desc = is_dir && data.may_match_descendant(path);
                    self.stats
                        .borrow_mut()
                        .record(data.source.as_deref(), rule_match);
                    if rule_match {
                        include = Some(false);
                        if data.dir_only {
                            descend = false;
                        } else if !descend {
                            descend = may_desc;
                        }
                        matched_source = data.source.clone();
                        break;
                    } else if may_desc {
                        descend = true;
                    }
                }
                Rule::DirMerge(_)
                | Rule::Clear
                | Rule::Existing
                | Rule::NoExisting
                | Rule::PruneEmptyDirs
                | Rule::NoPruneEmptyDirs => {}
            }
        }

        let matched = include.is_some();
        let mut include_val = include.unwrap_or(true);
        if !matched {
            descend = true;
        }

        if include_val && self.prune_empty_dirs {
            if let Some(root) = &self.root {
                let full = root.join(path);
                if full.is_dir() {
                    let mut has_child = false;
                    for entry in fs::read_dir(&full)? {
                        let entry = entry?;
                        let rel = path.join(entry.file_name());
                        if self.check(&rel, for_delete, xattr)?.include {
                            has_child = true;
                            break;
                        }
                    }
                    if !has_child {
                        include_val = false;
                    }
                }
            }
        }
        let source_str = matched_source
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let stats = self.stats.borrow();
        tracing::info!(
            target: InfoFlag::Filter.target(),
            path = %path.display(),
            matched,
            matches = stats.matches,
            misses = stats.misses,
            source = %source_str,
            rules = ordered.len(),
        );
        Ok(MatchResult {
            include: include_val,
            descend,
        })
    }

    pub fn merge(&mut self, more: Vec<Rule>) {
        let mut max_idx = self
            .rules
            .iter()
            .map(|(i, _)| *i)
            .chain(self.per_dir.iter().map(|(i, _)| *i))
            .max()
            .unwrap_or(0);
        for r in more {
            max_idx += 1;
            match r {
                Rule::DirMerge(p) => self.per_dir.push((max_idx, p)),
                Rule::Existing => self.existing = true,
                Rule::NoExisting => self.existing = false,
                Rule::PruneEmptyDirs => self.prune_empty_dirs = true,
                Rule::NoPruneEmptyDirs => self.prune_empty_dirs = false,
                other => self.rules.push((max_idx, other)),
            }
        }
    }

    pub fn stats(&self) -> FilterStats {
        self.stats.borrow().clone()
    }

    pub fn report(&self) {
        let stats = self.stats();
        let source = stats
            .last_source
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        tracing::info!(
            target: InfoFlag::Filter.target(),
            matches = stats.matches,
            misses = stats.misses,
            source = %source,
        );
    }

    fn dir_rules_at(
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

    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    fn load_merge_file(
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

        let mut next_index = if cvs { 0 } else { index };

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
