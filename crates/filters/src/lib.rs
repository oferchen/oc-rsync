use globset::{Glob, GlobMatcher};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A single include or exclude rule compiled into a glob matcher.
#[derive(Clone)]
pub enum Rule {
    Include(GlobMatcher),
    Exclude(GlobMatcher),
}

impl Rule {
    fn matches<P: AsRef<Path>>(&self, path: P) -> bool {
        match self {
            Rule::Include(m) | Rule::Exclude(m) => m.is_match(path),
        }
    }

    fn is_include(&self) -> bool {
        matches!(self, Rule::Include(_))
    }
}

/// Matcher evaluates rules sequentially against paths.
#[derive(Clone, Default)]
pub struct Matcher {
    /// Root of the walk. Used to locate `.rsync-filter` files.
    root: Option<PathBuf>,
    /// Global rules supplied via CLI or configuration.
    rules: Vec<Rule>,
    /// Cache of parsed rules for directories already visited.
    cached: RefCell<HashMap<PathBuf, Vec<Rule>>>,
}

impl Matcher {
    pub fn new(rules: Vec<Rule>) -> Self {
        Self {
            root: None,
            rules,
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
            let mut dirs = Vec::new();
            let mut dir = root.clone();
            dirs.push(dir.clone());
            if let Some(parent) = path.parent() {
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
                return Ok(rule.is_include());
            }
        }
        Ok(true)
    }

    /// Merge additional global rules, as when reading a top-level filter file.
    pub fn merge(&mut self, more: Vec<Rule>) {
        self.rules.extend(more);
    }

    /// Load cached rules for `dir`, reading its `.rsync-filter` file if needed.
    fn dir_rules(&self, dir: &Path) -> Result<Vec<Rule>, ParseError> {
        let mut cache = self.cached.borrow_mut();
        if let Some(r) = cache.get(dir) {
            return Ok(r.clone());
        }
        let file = dir.join(".rsync-filter");
        let rules = match fs::read_to_string(&file) {
            Ok(content) => {
                let rel = self
                    .root
                    .as_ref()
                    .and_then(|r| dir.strip_prefix(r).ok())
                    .filter(|p| !p.as_os_str().is_empty());
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
                        } else {
                            continue;
                        };
                        let new_pat = if pat.starts_with('/') {
                            pat.to_string()
                        } else if pat.contains('/') {
                            format!("{}/{}", rel_str, pat)
                        } else {
                            format!("{}/**/{}", rel_str, pat)
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
                parse(&adjusted)?
            }
            Err(_) => Vec::new(),
        };
        cache.insert(dir.to_path_buf(), rules.clone());
        Ok(rules)
    }
}

#[derive(Debug)]
pub enum ParseError {
    InvalidRule(String),
    Glob(globset::Error),
}

impl From<globset::Error> for ParseError {
    fn from(e: globset::Error) -> Self {
        Self::Glob(e)
    }
}

enum RuleKind {
    Include,
    Exclude,
}

/// Parse filter rules from input.
pub fn parse(input: &str) -> Result<Vec<Rule>, ParseError> {
    let mut rules = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Determine rule type based on the first character. Using
        // `strip_prefix` keeps the logic simple and avoids manual
        // character indexing.
        let (kind, pattern) = if let Some(rest) = line.strip_prefix('+') {
            (RuleKind::Include, rest.trim_start())
        } else if let Some(rest) = line.strip_prefix('-') {
            (RuleKind::Exclude, rest.trim_start())
        } else {
            return Err(ParseError::InvalidRule(raw_line.to_string()));
        };

        if pattern.is_empty() {
            return Err(ParseError::InvalidRule(raw_line.to_string()));
        }

        // Track whether the pattern is anchored to the root (starts with '/')
        // or targets a directory (ends with '/'). These affect how we
        // transform the glob to match rsync's semantics.
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
            }
        }
    }

    Ok(rules)
}
