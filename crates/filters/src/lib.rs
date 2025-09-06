// crates/filters/src/lib.rs
#![allow(clippy::collapsible_if)]

use globset::{Glob, GlobMatcher};
use logging::InfoFlag;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_PARSE_DEPTH: usize = 64;

#[derive(Clone, Default, Hash, PartialEq, Eq)]
pub struct RuleFlags {
    sender: bool,
    receiver: bool,
    perishable: bool,
    xattr: bool,
}

impl RuleFlags {
    fn applies(&self, for_delete: bool, xattr: bool) -> bool {
        if self.xattr != xattr {
            return false;
        }
        if for_delete {
            if self.sender && !self.receiver {
                return false;
            }
        } else if self.receiver && !self.sender {
            return false;
        }
        true
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            sender: self.sender || other.sender,
            receiver: self.receiver || other.receiver,
            perishable: self.perishable || other.perishable,
            xattr: self.xattr || other.xattr,
        }
    }
}

#[derive(Clone)]
pub struct RuleData {
    matcher: GlobMatcher,
    invert: bool,
    flags: RuleFlags,
    source: Option<PathBuf>,
    dir_only: bool,
}

#[derive(Clone)]
pub enum Rule {
    Include(RuleData),
    Exclude(RuleData),
    Protect(RuleData),
    Clear,
    DirMerge(PerDir),
    Existing,
    NoExisting,
    PruneEmptyDirs,
    NoPruneEmptyDirs,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PerDir {
    file: String,
    anchored: bool,
    root_only: bool,
    inherit: bool,
    cvs: bool,
    word_split: bool,
    sign: Option<char>,
    flags: RuleFlags,
}

#[derive(Clone)]
struct Cached {
    rules: Vec<(usize, Rule)>,
    merges: Vec<(usize, PerDir)>,
    mtime: Option<SystemTime>,
    len: u64,
}

#[derive(Clone, Default)]
pub struct FilterStats {
    pub matches: usize,
    pub misses: usize,
    pub last_source: Option<PathBuf>,
}

impl FilterStats {
    fn record(&mut self, source: Option<&Path>, matched: bool) {
        if matched {
            self.matches += 1;
            self.last_source = source.map(|p| p.to_path_buf());
        } else {
            self.misses += 1;
        }
    }
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

    pub fn is_included<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), false, false)
            .map(|(included, _)| included)
    }

    pub fn is_included_with_dir<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(bool, bool), ParseError> {
        self.check(path.as_ref(), false, false)
    }

    pub fn is_included_for_delete<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), true, false)
            .map(|(included, _)| included)
    }

    pub fn is_included_for_delete_with_dir<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<(bool, bool), ParseError> {
        self.check(path.as_ref(), true, false)
    }

    pub fn is_xattr_included<P: AsRef<Path>>(&self, name: P) -> Result<bool, ParseError> {
        self.check(name.as_ref(), false, true)
            .map(|(included, _)| included)
    }

    pub fn is_xattr_included_for_delete<P: AsRef<Path>>(
        &self,
        name: P,
    ) -> Result<bool, ParseError> {
        self.check(name.as_ref(), true, true)
            .map(|(included, _)| included)
    }

    pub fn is_xattr_included_for_delete_with_dir<P: AsRef<Path>>(
        &self,
        name: P,
    ) -> Result<(bool, bool), ParseError> {
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

    fn check(
        &self,
        path: &Path,
        for_delete: bool,
        xattr: bool,
    ) -> Result<(bool, bool), ParseError> {
        if self.existing {
            if let Some(root) = &self.root {
                if !root.join(path).exists() {
                    return Ok((false, false));
                }
            }
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

        let mut decision: Option<bool> = None;
        let mut matched = false;
        let mut matched_source: Option<PathBuf> = None;
        let mut dir_only_match = false;
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
                    let matched_rule = data.matcher.is_match(path);
                    let rule_match =
                        (data.invert && !matched_rule) || (!data.invert && matched_rule);
                    self.stats
                        .borrow_mut()
                        .record(data.source.as_deref(), rule_match);
                    if rule_match {
                        decision = Some(true);
                        matched = true;
                        matched_source = data.source.clone();
                        break;
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
                    let matched_rule = data.matcher.is_match(path);
                    let rule_match =
                        (data.invert && !matched_rule) || (!data.invert && matched_rule);
                    self.stats
                        .borrow_mut()
                        .record(data.source.as_deref(), rule_match);
                    if rule_match {
                        decision = Some(true);
                        matched = true;
                        matched_source = data.source.clone();
                        break;
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
                    let matched_rule = data.matcher.is_match(path);
                    let rule_match =
                        (data.invert && !matched_rule) || (!data.invert && matched_rule);
                    self.stats
                        .borrow_mut()
                        .record(data.source.as_deref(), rule_match);
                    if rule_match {
                        decision = Some(false);
                        matched = true;
                        matched_source = data.source.clone();
                        dir_only_match = data.dir_only;
                        break;
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
        let mut included = decision.unwrap_or(true);
        if included && self.prune_empty_dirs {
            if let Some(root) = &self.root {
                let full = root.join(path);
                if full.is_dir() {
                    let mut has_child = false;
                    for entry in fs::read_dir(&full)? {
                        let entry = entry?;
                        let rel = path.join(entry.file_name());
                        if self.check(&rel, for_delete, xattr)?.0 {
                            has_child = true;
                            break;
                        }
                    }
                    if !has_child {
                        included = false;
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
        Ok((included, matched && dir_only_match && !included))
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
                    .filter(|p| !p.as_os_str().is_empty())
            };

            let key = (path.clone(), pd.sign, pd.word_split);
            let meta = fs::metadata(&path).ok();
            let (mtime, len) = match &meta {
                Some(m) => (m.modified().ok(), m.len()),
                None => (None, 0),
            };

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
                    Rule::Include(d) | Rule::Exclude(d) | Rule::Protect(d) => {
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
                if pd.inherit && !entry.iter().any(|(_, p)| p == &pd) {
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
            Err(_) => return Ok((Vec::new(), Vec::new())),
        };

        let adjusted = if cvs {
            let rel_str = rel.map(|p| p.to_string_lossy().to_string());
            let mut buf = String::new();
            let iter: Box<dyn Iterator<Item = &str>> = if self.from0 {
                Box::new(content.split('\0'))
            } else {
                Box::new(content.split_whitespace())
            };
            for token in iter {
                if token.is_empty() || token.starts_with('#') {
                    continue;
                }
                let pat = if let Some(rel_str) = &rel_str {
                    if token.starts_with('/') {
                        format!("/{}/{}", rel_str, token.trim_start_matches('/'))
                    } else {
                        format!("/{}/{}", rel_str, token)
                    }
                } else if token.starts_with('/') {
                    token.to_string()
                } else {
                    format!("/{}", token)
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
                    let (kind, pat) = if let Some(rest) = line.strip_prefix('+') {
                        ('+', rest.trim_start())
                    } else if let Some(rest) = line.strip_prefix('-') {
                        ('-', rest.trim_start())
                    } else if let Some(rest) =
                        line.strip_prefix('P').or_else(|| line.strip_prefix('p'))
                    {
                        ('P', rest.trim_start())
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
                    buf.push(kind);
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

        fn is_excluded(rules: &[(usize, Rule)], path: &Path) -> bool {
            let mut state: Option<bool> = None;
            for (_, r) in rules {
                match r {
                    Rule::Clear => state = None,
                    Rule::Include(d) | Rule::Protect(d) => {
                        if d.dir_only && !path.is_dir() {
                            continue;
                        }
                        let matched = d.matcher.is_match(path);
                        if (d.invert && !matched) || (!d.invert && matched) {
                            state = Some(true);
                        }
                    }
                    Rule::Exclude(d) => {
                        if d.dir_only && !path.is_dir() {
                            continue;
                        }
                        let matched = d.matcher.is_match(path);
                        if (d.invert && !matched) || (!d.invert && matched) {
                            state = Some(false);
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
                    if is_excluded(&rules, &dir_for_rule) {
                        continue;
                    }
                    merges.push((index, pd.clone()));
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
                        index,
                    )?;
                    rules.append(&mut nr);
                    merges.append(&mut nm);
                }
                other => rules.push((index, other)),
            }
        }

        Ok((rules, merges))
    }
}

#[derive(Debug)]
pub enum ParseError {
    InvalidRule(String),
    Glob(globset::Error),
    RecursiveInclude(PathBuf),
    RecursionLimit,
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
    let mut has_data = false;
    let mut prev_space = false;
    let mut last_non_space = 0;
    for c in chars {
        if escaped {
            out.push(c);
            if !c.is_whitespace() {
                has_data = true;
                last_non_space = out.len();
                prev_space = false;
            } else {
                last_non_space = out.len();
                prev_space = true;
            }
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
                has_data = true;
                last_non_space = out.len();
                prev_space = false;
            } else {
                prev_space = true;
            }
        } else if c == '\\' {
            escaped = true;
        } else if c == '#' && prev_space && has_data {
            break;
        } else {
            out.push(c);
            if !c.is_whitespace() {
                has_data = true;
                last_non_space = out.len();
                prev_space = false;
            } else {
                prev_space = true;
            }
        }
    }
    if escaped {
        out.push('\\');
        last_non_space = out.len();
    }
    out.truncate(last_non_space);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
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

        if line == "!" {
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
                    let matcher = Glob::new("**/.rsync-filter")?.compile_matcher();
                    let data = RuleData {
                        matcher,
                        invert: false,
                        flags: RuleFlags::default(),
                        source: source.clone(),
                        dir_only: false,
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
        } else if let Some(r) = line.strip_prefix('H') {
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
                for rule in &mut sub {
                    if let Rule::Include(d) | Rule::Exclude(d) | Rule::Protect(d) = rule {
                        if m.contains('s') {
                            d.flags.sender = true;
                        }
                        if m.contains('r') {
                            d.flags.receiver = true;
                        }
                        if m.contains('p') {
                            d.flags.perishable = true;
                        }
                        if m.contains('x') {
                            d.flags.xattr = true;
                        }
                    }
                }
            }
            rules.extend(sub);
            if m.contains('e') {
                let pat = format!("**/{}", file);
                let matcher = Glob::new(&pat)?.compile_matcher();
                let data = RuleData {
                    matcher,
                    invert: false,
                    flags: RuleFlags::default(),
                    source: source.clone(),
                    dir_only: false,
                };
                rules.push(Rule::Exclude(data));
            }
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
            let sign = if m.contains('+') {
                Some('+')
            } else if m.contains('-') {
                Some('-')
            } else {
                None
            };
            rules.push(Rule::DirMerge(PerDir {
                file: fname.clone(),
                anchored,
                root_only: anchored,
                inherit: !m.contains('n'),
                cvs: m.contains('C'),
                word_split: m.contains('w'),
                sign,
                flags: {
                    let mut f = RuleFlags::default();
                    if m.contains('s') {
                        f.sender = true;
                    }
                    if m.contains('r') {
                        f.receiver = true;
                    }
                    if m.contains('p') {
                        f.perishable = true;
                    }
                    if m.contains('x') {
                        f.xattr = true;
                    }
                    f
                },
            }));
            if m.contains('e') {
                let pat = format!("**/{}", fname);
                let matcher = Glob::new(&pat)?.compile_matcher();
                let data = RuleData {
                    matcher,
                    invert: false,
                    flags: RuleFlags::default(),
                    source: source.clone(),
                    dir_only: false,
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
                    for pat in pats {
                        let anchored = if pat.starts_with('/') {
                            pat.clone()
                        } else {
                            format!("/{}", pat)
                        };
                        let dir_pat = format!("{}/***", anchored.trim_end_matches('/'));
                        let line1 = if from0 {
                            format!("+{anchored}\n")
                        } else {
                            format!("+ {anchored}\n")
                        };
                        rules.extend(parse_with_options(
                            &line1,
                            from0,
                            visited,
                            depth + 1,
                            Some(path.clone()),
                        )?);
                        let line2 = if from0 {
                            format!("+{dir_pat}\n")
                        } else {
                            format!("+ {dir_pat}\n")
                        };
                        rules.extend(parse_with_options(
                            &line2,
                            from0,
                            visited,
                            depth + 1,
                            Some(path.clone()),
                        )?);
                    }
                    rules.extend(parse_with_options(
                        "- *\n",
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
                            return Err(ParseError::InvalidRule(raw_line.to_string()))
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

        if mods.contains('C') && rest.is_empty() {
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

        let mut flags = RuleFlags::default();
        if mods.contains('s') {
            flags.sender = true;
        }
        if mods.contains('r') {
            flags.receiver = true;
        }
        if mods.contains('p') {
            flags.perishable = true;
        }
        if mods.contains('x') {
            flags.xattr = true;
        }

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
        let anchored = pattern.starts_with('/');
        let dir_all = pattern.ends_with("/***");
        let dir_only = !dir_all && pattern.ends_with('/');
        let mut base = pattern.trim_start_matches('/').to_string();
        if dir_all {
            base = base.trim_end_matches("/***").to_string();
        } else if dir_only {
            base = base.trim_end_matches('/').to_string();
        }
        let bases: Vec<String> = if !anchored && !base.contains('/') {
            vec![base.clone(), format!("**/{}", base)]
        } else {
            vec![base]
        };
        let mut pats: Vec<(String, bool)> = Vec::new();
        for b in bases {
            if dir_all {
                pats.push((b.clone(), true));
                pats.push((format!("{}/**", b), false));
            } else {
                pats.push((b, dir_only));
            }
        }

        let invert = mods.contains('!');
        for (pat, dir_only) in pats {
            let matcher = Glob::new(&pat)?.compile_matcher();
            let data = RuleData {
                matcher,
                invert,
                flags: flags.clone(),
                source: source.clone(),
                dir_only,
            };
            match kind {
                RuleKind::Include => rules.push(Rule::Include(data)),
                RuleKind::Exclude => rules.push(Rule::Exclude(data)),
                RuleKind::Protect => rules.push(Rule::Protect(data)),
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
    fn add_pat(out: &mut Vec<Rule>, pat: &str) -> Result<(), ParseError> {
        let dir_all = pat.ends_with('/') || pat.ends_with("/***");
        let mut base = pat.to_string();
        if pat.ends_with("/***") {
            base = base.trim_end_matches("/***").to_string();
        } else if pat.ends_with('/') {
            base = base.trim_end_matches('/').to_string();
        }
        let bases: Vec<String> = if !base.contains('/') {
            vec![base.clone(), format!("**/{}", base)]
        } else {
            vec![base]
        };
        let mut pats: Vec<(String, bool)> = Vec::new();
        for b in bases {
            if dir_all {
                pats.push((b.clone(), true));
                pats.push((format!("{}/**", b), false));
            } else {
                pats.push((b, false));
            }
        }
        for (p, dir_only) in pats {
            let matcher = Glob::new(&p)?.compile_matcher();
            let data = RuleData {
                matcher,
                invert: false,
                flags: RuleFlags {
                    perishable: true,
                    ..Default::default()
                },
                source: None,
                dir_only,
            };
            out.push(Rule::Exclude(data));
        }
        Ok(())
    }

    let mut out = Vec::new();
    for pat in CVS_DEFAULTS {
        add_pat(&mut out, pat)?;
    }

    if let Ok(home) = env::var("HOME") {
        let path = Path::new(&home).join(".cvsignore");
        if let Ok(content) = fs::read_to_string(path) {
            for pat in content.split_whitespace() {
                if !pat.is_empty() {
                    add_pat(&mut out, pat)?;
                }
            }
        }
    }

    if let Ok(envpats) = env::var("CVSIGNORE") {
        for pat in envpats.split_whitespace() {
            if !pat.is_empty() {
                add_pat(&mut out, pat)?;
            }
        }
    }

    Ok(out)
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
        std::io::stdin().read_to_end(&mut buf)?;
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
    parse_from_bytes(&data, from0, visited, depth, Some(path.to_path_buf()))
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
    use tempfile::{tempfile, NamedTempFile};

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
