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
    /// Per-directory merge directive (e.g., `: .rsync-filter`).
    DirMerge(PerDir),
}

impl Rule {
    fn matches<P: AsRef<Path>>(&self, path: P) -> bool {
        match self {
            Rule::Include(m) | Rule::Exclude(m) | Rule::Protect(m) => m.is_match(path),
            Rule::DirMerge(_) => false,
        }
    }

    fn is_include(&self) -> bool {
        matches!(self, Rule::Include(_))
    }
}

#[derive(Clone)]
pub struct PerDir {
    file: String,
    anchored: bool,
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
    /// Cache of parsed rules for directories already visited.
    cached: RefCell<HashMap<PathBuf, Vec<Rule>>>,
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
        let mut active = Vec::new();

        if let Some(root) = &self.root {
            let mut dirs = vec![root.clone()];

            if let Some(parent) = path.parent() {
                let mut dir = root.clone();
                for comp in parent.components() {
                    dir.push(comp.as_os_str());
                    dirs.push(dir.clone());
                }
            }

            for d in dirs.iter().rev() {
                active.extend(self.dir_rules(d)?);
            }
        }

        // Global rules have the lowest precedence.
        active.extend(self.rules.clone());

        for rule in &active {
            if rule.matches(path) {
                if let Rule::Protect(_) = rule {
                    continue;
                }
                return Ok(rule.is_include());
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

        let mut combined = Vec::new();
        for pd in &self.per_dir {
            let path = dir.join(&pd.file);
            let mut cache = self.cached.borrow_mut();
            if let Some(r) = cache.get(&path) {
                combined.extend(r.clone());
                continue;
            }
            let rel = if !pd.anchored {
                self.root
                    .as_ref()
                    .and_then(|r| dir.strip_prefix(r).ok())
                    .filter(|p| !p.as_os_str().is_empty())
            } else {
                None
            };
            let rules = match fs::read_to_string(&path) {
                Ok(content) => {
                    let adjusted = if let Some(rel) = rel {
                        let mut buf = String::new();
                        let rel_str = rel.to_string_lossy();
                        for raw_line in content.lines() {
                            let line = raw_line.trim();
                            if line.is_empty() || line.starts_with('#') {
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
                    let mut visited = HashSet::new();
                    visited.insert(path.clone());
                    parse(&adjusted, &mut visited, 0)?
                        .into_iter()
                        .filter(|r| !matches!(r, Rule::DirMerge(_)))
                        .collect()
                }
                Err(_) => Vec::new(),
            };
            cache.insert(path.clone(), rules.clone());
            combined.extend(rules);
        }
        Ok(combined)
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

        if let Some(rest) = line.strip_prefix("-F") {
            if rest.chars().all(|c| c == 'F') {
                rules.push(Rule::DirMerge(PerDir {
                    file: ".rsync-filter".to_string(),
                    anchored: false,
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
