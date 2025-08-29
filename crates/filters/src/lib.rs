use globset::{Glob, GlobMatcher};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_PARSE_DEPTH: usize = 64;

/// A single rule compiled into a glob matcher or special directive.
#[derive(Clone)]
pub enum Rule {
    Include(GlobMatcher),
    Exclude(GlobMatcher),
    Protect(GlobMatcher),
    /// Clear all previously defined rules.
    Clear,
    /// Per-directory merge directive (e.g., `: .rsync-filter`).
    DirMerge(PerDir),
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PerDir {
    file: String,
    anchored: bool,
    root_only: bool,
}

#[derive(Clone)]
struct Cached {
    rules: Vec<Rule>,
    merges: Vec<PerDir>,
}

/// Matcher evaluates rules sequentially against paths.
#[derive(Clone, Default)]
pub struct Matcher {
    /// Root of the walk. Used to locate per-directory filter files.
    root: Option<PathBuf>,
    /// Global rules supplied via CLI or configuration.
    rules: Vec<Rule>,
    /// Filenames that should be merged for every directory visited.
    per_dir: Vec<PerDir>,
    /// Additional per-directory merge directives discovered while walking.
    extra_per_dir: RefCell<HashMap<PathBuf, Vec<PerDir>>>,
    /// Cache of parsed rules for filter files along with any nested merges.
    cached: RefCell<HashMap<PathBuf, Cached>>,
}

impl Matcher {
    pub fn new(rules: Vec<Rule>) -> Self {
        let mut per_dir = Vec::new();
        let mut global = Vec::new();
        for r in rules {
            match r {
                Rule::DirMerge(p) => per_dir.push(p),
                _ => global.push(r),
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

    /// Set the root directory used to resolve per-directory filter files.
    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    /// Determine if the provided path is included by the rules, loading any
    /// directory-specific `.rsync-filter` files along the way. Rules discovered
    /// later in the walk take precedence over earlier ones, mirroring rsync's
    /// evaluation order.
    pub fn is_included<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        let path = path.as_ref();
        let mut active: Vec<Rule> = Vec::new();

        if let Some(root) = &self.root {
            let mut dirs = vec![root.clone()];

            if let Some(parent) = path.parent() {
                let mut dir = root.clone();
                for comp in parent.components() {
                    dir.push(comp.as_os_str());
                    dirs.push(dir.clone());
                }
            }

            for d in dirs {
                let mut rules = Vec::new();
                for rule in self.dir_rules(&d)? {
                    if let Rule::Clear = rule {
                        active.clear();
                        rules.clear();
                    } else {
                        rules.push(rule);
                    }
                }
                if !rules.is_empty() {
                    let mut new_rules = rules;
                    new_rules.extend(active);
                    active = new_rules;
                }
            }
        }

        active.extend(self.rules.clone());

        for rule in &active {
            match rule {
                Rule::Protect(m) => {
                    if m.is_match(path) {
                        continue;
                    }
                }
                Rule::Include(m) => {
                    if m.is_match(path) {
                        return Ok(true);
                    }
                }
                Rule::Exclude(m) => {
                    if m.is_match(path) {
                        return Ok(false);
                    }
                }
                Rule::DirMerge(_) | Rule::Clear => {}
            }
        }
        Ok(true)
    }

    /// Merge additional global rules, as when reading a top-level filter file.
    pub fn merge(&mut self, more: Vec<Rule>) {
        self.rules.extend(more);
    }

    /// Load cached rules for `dir`, reading any per-directory filter files if needed.
    fn dir_rules(&self, dir: &Path) -> Result<Vec<Rule>, ParseError> {
        if let Some(root) = &self.root {
            if !dir.starts_with(root) {
                return Ok(Vec::new());
            }
        }

        // Gather per-dir directives from globals and ancestors.
        let mut per_dirs = self.per_dir.clone();
        let mut anc = dir.parent();
        while let Some(a) = anc {
            if let Some(extra) = self.extra_per_dir.borrow().get(a) {
                per_dirs.extend(extra.clone());
            }
            anc = a.parent();
        }

        let mut combined = Vec::new();
        let mut new_merges = Vec::new();
        let mut seen_files = HashSet::new();

        for pd in per_dirs {
            let path = if pd.root_only {
                if let Some(root) = &self.root {
                    root.join(&pd.file)
                } else {
                    dir.join(&pd.file)
                }
            } else {
                dir.join(&pd.file)
            };

            if !seen_files.insert(path.clone()) {
                continue;
            }

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
            let state = if let Some(c) = cache.get(&path) {
                c.clone()
            } else {
                let mut visited = HashSet::new();
                visited.insert(path.clone());
                let (rules, merges) =
                    self.load_merge_file(dir, &path, rel.as_deref(), &mut visited, 0)?;
                let cached = Cached {
                    rules: rules.clone(),
                    merges: merges.clone(),
                };
                cache.insert(path.clone(), cached.clone());
                cached
            };

            combined.extend(state.rules.clone());
            new_merges.extend(state.merges.clone());
        }

        if !new_merges.is_empty() {
            let mut map = self.extra_per_dir.borrow_mut();
            let entry = map.entry(dir.to_path_buf()).or_default();
            for pd in new_merges {
                if !entry.contains(&pd) {
                    entry.push(pd);
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
        visited: &mut HashSet<PathBuf>,
        depth: usize,
    ) -> Result<(Vec<Rule>, Vec<PerDir>), ParseError> {
        if depth >= MAX_PARSE_DEPTH {
            return Err(ParseError::RecursionLimit);
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Ok((Vec::new(), Vec::new())),
        };

        let adjusted = if let Some(rel) = rel {
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
                } else if let Some(rest) = line.strip_prefix('P').or_else(|| line.strip_prefix('p'))
                {
                    ('P', rest.trim_start())
                } else {
                    buf.push_str(raw_line);
                    buf.push('\n');
                    continue;
                };
                let new_pat = if pat.starts_with('/') {
                    pat.to_string()
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
        };

        let mut rules = Vec::new();
        let mut merges = Vec::new();
        let parsed = parse(&adjusted, visited, depth + 1)?;

        for r in parsed {
            match r {
                Rule::DirMerge(pd) => {
                    merges.push(pd.clone());
                    let nested = if pd.root_only {
                        if let Some(root) = &self.root {
                            root.join(&pd.file)
                        } else {
                            dir.join(&pd.file)
                        }
                    } else {
                        dir.join(&pd.file)
                    };
                    if !visited.insert(nested.clone()) {
                        return Err(ParseError::RecursiveInclude(nested));
                    }
                    let rel2 = if pd.anchored { None } else { rel };
                    let (mut nr, mut nm) =
                        self.load_merge_file(dir, &nested, rel2, visited, depth + 1)?;
                    rules.append(&mut nr);
                    merges.append(&mut nm);
                }
                other => rules.push(other),
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

/// Parse filter rules from input with recursion tracking.
pub fn parse(
    input: &str,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<Vec<Rule>, ParseError> {
    if depth >= MAX_PARSE_DEPTH {
        return Err(ParseError::RecursionLimit);
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
            if rest.chars().all(|c| c == 'F') {
                rules.push(Rule::DirMerge(PerDir {
                    file: ".rsync-filter".to_string(),
                    anchored: false,
                    root_only: false,
                }));
                if !rest.is_empty() {
                    let matcher = Glob::new("**/.rsync-filter")?.compile_matcher();
                    rules.push(Rule::Exclude(matcher));
                }
                continue;
            }
        }

        let (kind, rest) = if let Some(r) = line.strip_prefix('+') {
            (Some(RuleKind::Include), r.trim_start())
        } else if let Some(r) = line.strip_prefix('-') {
            (Some(RuleKind::Exclude), r.trim_start())
        } else if let Some(r) = line.strip_prefix('P').or_else(|| line.strip_prefix('p')) {
            (Some(RuleKind::Protect), r.trim_start())
        } else if let Some(r) = line.strip_prefix('.') {
            let file = r.trim_start();
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
            let r = r.trim_start();
            let mut parts = r.splitn(2, |c: char| c.is_whitespace());
            let first = parts.next().unwrap_or("");
            let file = parts.next().unwrap_or(first).trim();
            if file.is_empty() {
                return Err(ParseError::InvalidRule(raw_line.to_string()));
            }
            let anchored = file.starts_with('/');
            let fname = if anchored {
                file.trim_start_matches('/').to_string()
            } else {
                file.to_string()
            };
            rules.push(Rule::DirMerge(PerDir {
                file: fname,
                anchored,
                root_only: false,
            }));
            continue;
        } else {
            let mut parts = line.split_whitespace();
            let token = parts.next().unwrap_or("");
            let rest = parts.next().unwrap_or("").trim();
            match token {
                "include" => (Some(RuleKind::Include), rest),
                "exclude" => (Some(RuleKind::Exclude), rest),
                "protect" => (Some(RuleKind::Protect), rest),
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
                    }));
                    continue;
                }
                _ => (None, rest),
            }
        };

        let kind = match kind {
            Some(k) => k,
            None => return Err(ParseError::InvalidRule(raw_line.to_string())),
        };
        if rest.is_empty() {
            return Err(ParseError::InvalidRule(raw_line.to_string()));
        }

        let pattern = rest;
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

        for pat in pats {
            let matcher = Glob::new(&pat)?.compile_matcher();
            match kind {
                RuleKind::Include => rules.push(Rule::Include(matcher)),
                RuleKind::Exclude => rules.push(Rule::Exclude(matcher)),
                RuleKind::Protect => rules.push(Rule::Protect(matcher)),
            }
        }
    }

    Ok(rules)
}
