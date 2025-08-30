// crates/filters/src/lib.rs
use globset::{Glob, GlobMatcher};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_PARSE_DEPTH: usize = 64;

#[derive(Clone, Default)]
pub struct RuleFlags {
    sender: bool,
    receiver: bool,
    perishable: bool,
    xattr: bool,
}

impl RuleFlags {
    fn applies(&self, for_delete: bool) -> bool {
        if for_delete {
            if self.sender && !self.receiver {
                return false;
            }
        } else if self.receiver && !self.sender {
            return false;
        }
        true
    }
}

#[derive(Clone)]
pub struct RuleData {
    matcher: GlobMatcher,
    invert: bool,
    flags: RuleFlags,
}

#[derive(Clone)]
pub enum Rule {
    Include(RuleData),
    Exclude(RuleData),
    Protect(RuleData),
    Clear,
    DirMerge(PerDir),
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
}

#[derive(Clone)]
struct Cached {
    rules: Vec<(usize, Rule)>,
    merges: Vec<(usize, PerDir)>,
}

#[derive(Clone, Default)]
pub struct Matcher {
    root: Option<PathBuf>,
    rules: Vec<(usize, Rule)>,
    per_dir: Vec<(usize, PerDir)>,
    extra_per_dir: RefCell<HashMap<PathBuf, Vec<(usize, PerDir)>>>,
    cached: RefCell<HashMap<(PathBuf, Option<char>, bool), Cached>>,
}

impl Matcher {
    pub fn new(rules: Vec<Rule>) -> Self {
        let mut per_dir = Vec::new();
        let mut global = Vec::new();
        for (idx, r) in rules.into_iter().enumerate() {
            match r {
                Rule::DirMerge(p) => per_dir.push((idx, p)),
                other => global.push((idx, other)),
            }
        }
        Self {
            root: None,
            rules: global,
            per_dir,
            extra_per_dir: RefCell::new(HashMap::new()),
            cached: RefCell::new(HashMap::new()),
        }
    }

    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    pub fn is_included<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), false)
    }

    pub fn is_included_for_delete<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), true)
    }

    fn check(&self, path: &Path, for_delete: bool) -> Result<bool, ParseError> {
        let path = path;
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

            for (depth, d) in dirs.iter().enumerate() {
                let depth = depth + 1;
                for (idx, rule) in self.dir_rules_at(d)? {
                    active.push((idx, depth, seq, rule));
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

        for rule in &ordered {
            match rule {
                Rule::Protect(data) => {
                    if !data.flags.applies(for_delete) {
                        continue;
                    }
                    if for_delete && data.flags.perishable {
                        continue;
                    }
                    let matched = data.matcher.is_match(path);
                    if (data.invert && !matched) || (!data.invert && matched) {
                        return Ok(true);
                    }
                }
                Rule::Include(data) => {
                    if !data.flags.applies(for_delete) {
                        continue;
                    }
                    if for_delete && data.flags.perishable {
                        continue;
                    }
                    let matched = data.matcher.is_match(path);
                    if (data.invert && !matched) || (!data.invert && matched) {
                        return Ok(true);
                    }
                }
                Rule::Exclude(data) => {
                    if !data.flags.applies(for_delete) {
                        continue;
                    }
                    if for_delete && data.flags.perishable {
                        continue;
                    }
                    let matched = data.matcher.is_match(path);
                    if (data.invert && !matched) || (!data.invert && matched) {
                        return Ok(false);
                    }
                }
                Rule::DirMerge(_) | Rule::Clear => {}
            }
        }
        Ok(true)
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
                other => self.rules.push((max_idx, other)),
            }
        }
    }

    fn dir_rules_at(&self, dir: &Path) -> Result<Vec<(usize, Rule)>, ParseError> {
        if let Some(root) = &self.root {
            if !dir.starts_with(root) {
                return Ok(Vec::new());
            }
        }

        let mut per_dirs: Vec<(usize, PerDir)> = Vec::new();
        let mut anc = dir.parent();
        while let Some(a) = anc {
            if let Some(extra) = self.extra_per_dir.borrow().get(a) {
                per_dirs.extend(extra.clone());
            }
            anc = a.parent();
        }
        per_dirs.extend(self.per_dir.clone());
        per_dirs.sort_by_key(|(idx, _)| *idx);

        let mut combined = Vec::new();
        let mut new_merges = Vec::new();

        for (idx, pd) in per_dirs {
            let path = if pd.root_only {
                if let Some(root) = &self.root {
                    root.join(&pd.file)
                } else {
                    dir.join(&pd.file)
                }
            } else {
                dir.join(&pd.file)
            };

            let rel = if pd.anchored {
                None
            } else {
                self.root
                    .as_ref()
                    .and_then(|r| dir.strip_prefix(r).ok())
                    .map(|p| p.to_path_buf())
                    .filter(|p| !p.as_os_str().is_empty())
            };

            let mut cache = self.cached.borrow_mut();
            let key = (path.clone(), pd.sign, pd.word_split);
            let state = if let Some(c) = cache.get(&key) {
                c.clone()
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
                };
                cache.insert(key, cached.clone());
                cached
            };

            combined.extend(state.rules.clone());
            new_merges.extend(state.merges.clone());
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

        let mut adjusted = if cvs || path.file_name().map_or(false, |f| f == ".cvsignore") {
            let rel_str = rel.map(|p| p.to_string_lossy().to_string());
            let mut buf = String::new();
            for token in content.split_whitespace() {
                if token.starts_with('#') {
                    continue;
                }
                let pat = if let Some(rel_str) = &rel_str {
                    if token.starts_with('/') {
                        format!("/{}/{}", rel_str, token.trim_start_matches('/'))
                    } else {
                        format!("{}/{}", rel_str, token)
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
                for token in content.split_whitespace() {
                    buf.push_str(token);
                    buf.push('\n');
                }
                content = buf;
            }
            if let Some(rel) = rel {
                let rel_str = rel.to_string_lossy();
                let mut buf = String::new();
                for raw_line in content.lines() {
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

        if !cvs {
            if let Some(ch) = sign {
                let mut buf = String::new();
                for raw in adjusted.lines() {
                    let line = raw.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    buf.push(ch);
                    buf.push(' ');
                    buf.push_str(line);
                    buf.push('\n');
                }
                adjusted = buf;
            }
        }

        let mut rules = Vec::new();
        let mut merges = Vec::new();

        fn is_excluded(rules: &[(usize, Rule)], path: &Path) -> bool {
            let mut state: Option<bool> = None;
            for (_, r) in rules {
                match r {
                    Rule::Clear => state = None,
                    Rule::Include(d) | Rule::Protect(d) => {
                        let matched = d.matcher.is_match(path);
                        if (d.invert && !matched) || (!d.invert && matched) {
                            state = Some(true);
                        }
                    }
                    Rule::Exclude(d) => {
                        let matched = d.matcher.is_match(path);
                        if (d.invert && !matched) || (!d.invert && matched) {
                            state = Some(false);
                        }
                    }
                    Rule::DirMerge(_) => {}
                }
            }
            matches!(state, Some(false))
        }

        let parsed = parse(&adjusted, visited, depth + 1)?;

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
                    let rel2 = if pd.anchored {
                        None
                    } else {
                        let mut base = rel.map(|p| p.to_path_buf()).unwrap_or_else(PathBuf::new);
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
}

impl From<globset::Error> for ParseError {
    fn from(e: globset::Error) -> Self {
        Self::Glob(e)
    }
}

enum RuleKind {
    Include,
    Exclude,
    Protect,
}

pub fn parse(
    input: &str,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<Vec<Rule>, ParseError> {
    if depth >= MAX_PARSE_DEPTH {
        return Err(ParseError::RecursionLimit);
    }
    fn split_mods<'a>(s: &'a str, allowed: &str) -> (&'a str, &'a str) {
        if s.chars().next().map(|c| c.is_whitespace()).unwrap_or(true) {
            return ("", s.trim_start());
        }
        let s = s.trim_start();
        let mut idx = 0;
        let bytes = s.as_bytes();
        if bytes.get(0) == Some(&b',') {
            idx += 1;
        }
        while let Some(&c) = bytes.get(idx) {
            if allowed.as_bytes().contains(&c) {
                idx += 1;
            } else {
                break;
            }
        }
        let mods = &s[..idx];
        let rest = s[idx..].trim_start();
        (mods, rest)
    }

    let mut rules = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line == "!" {
            rules.push(Rule::Clear);
            continue;
        }

        if let Some(rest) = line.strip_prefix("-F") {
            let count = rest.chars().take_while(|c| *c == 'F').count();
            if count == rest.len() {
                rules.push(Rule::DirMerge(PerDir {
                    file: ".rsync-filter".to_string(),
                    anchored: false,
                    root_only: false,
                    inherit: true,
                    cvs: false,
                    word_split: false,
                    sign: None,
                }));
                if count > 0 {
                    let matcher = Glob::new("**/.rsync-filter")?.compile_matcher();
                    let data = RuleData {
                        matcher,
                        invert: false,
                        flags: RuleFlags::default(),
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
            let mut content = fs::read_to_string(&path)
                .map_err(|_| ParseError::InvalidRule(raw_line.to_string()))?;
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
            let mut sub = parse(&content, visited, depth + 1)?;
            if m.contains('s') || m.contains('r') || m.contains('p') || m.contains('x') {
                for rule in &mut sub {
                    if let Rule::Include(ref mut d)
                    | Rule::Exclude(ref mut d)
                    | Rule::Protect(ref mut d) = rule
                    {
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
            let content = fs::read_to_string(&path)
                .map_err(|_| ParseError::InvalidRule(raw_line.to_string()))?;
            rules.extend(parse(&content, visited, depth + 1)?);
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
                root_only: false,
                inherit: !m.contains('n'),
                cvs: m.contains('C'),
                word_split: m.contains('w'),
                sign,
            }));
            if m.contains('e') {
                let pat = format!("**/{}", fname);
                let matcher = Glob::new(&pat)?.compile_matcher();
                let data = RuleData {
                    matcher,
                    invert: false,
                    flags: RuleFlags::default(),
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
                "merge" => {
                    let path = PathBuf::from(rest);
                    if !visited.insert(path.clone()) {
                        return Err(ParseError::RecursiveInclude(path));
                    }
                    let content = fs::read_to_string(&path)
                        .map_err(|_| ParseError::InvalidRule(raw_line.to_string()))?;
                    rules.extend(parse(&content, visited, depth + 1)?);
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
        if mods.contains('/') && !pattern.starts_with('/') {
            pattern = format!("/{}", pattern);
        }
        let anchored = pattern.starts_with('/');
        let dir_only = pattern.ends_with('/');
        let mut base = pattern.trim_start_matches('/').to_string();
        if dir_only {
            base = base.trim_end_matches('/').to_string();
        }
        if !anchored && !base.contains('/') {
            base = format!("**/{}", base);
        }
        let pats = if dir_only {
            vec![base.clone(), format!("{}/**", base)]
        } else {
            vec![base]
        };

        let invert = mods.contains('!');
        for pat in pats {
            let matcher = Glob::new(&pat)?.compile_matcher();
            let data = RuleData {
                matcher,
                invert,
                flags: flags.clone(),
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
        let dir_only = pat.ends_with('/');
        let mut base = pat.trim_end_matches('/').to_string();
        if !base.contains('/') {
            base = format!("**/{}", base);
        }
        let pats = if dir_only {
            vec![base.clone(), format!("{}/**", base)]
        } else {
            vec![base]
        };
        for p in pats {
            let matcher = Glob::new(&p)?.compile_matcher();
            let data = RuleData {
                matcher,
                invert: false,
                flags: RuleFlags {
                    perishable: true,
                    ..Default::default()
                },
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
