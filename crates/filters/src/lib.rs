use globset::{Glob, GlobMatcher};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_PARSE_DEPTH: usize = 64;

/// Additional attributes that can modify how a rule is applied.
#[derive(Clone, Default)]
pub struct RuleFlags {
    /// Rule only affects the sending side.
    sender: bool,
    /// Rule only affects the receiving side.
    receiver: bool,
    /// Rule is ignored when the parent directory is deleted.
    perishable: bool,
    /// Rule applies to xattr names, not file paths.
    xattr: bool,
}

impl RuleFlags {
    fn applies_to_sender(&self) -> bool {
        if self.receiver && !self.sender {
            return false;
        }
        true
    }
}

#[derive(Clone)]
/// Compiled matcher and attributes for a single filter rule.
///
/// This struct is public to allow library consumers to construct custom
/// [`Rule`] variants or inspect parsed rules, while keeping its fields
/// private for future flexibility. Most callers will obtain instances via
/// [`parse`] or other helpers rather than building `RuleData` manually.
pub struct RuleData {
    matcher: GlobMatcher,
    invert: bool,
    flags: RuleFlags,
}

/// A single rule compiled into a glob matcher or special directive.
#[derive(Clone)]
pub enum Rule {
    /// Include rule; `invert` flips match logic when `!` modifier is used.
    Include(RuleData),
    /// Exclude rule; `invert` flips match logic when `!` modifier is used.
    Exclude(RuleData),
    /// Protect rule used for receiver-side deletes; `invert` supported for
    /// parity with rsync's `!` modifier.
    Protect(RuleData),
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
    /// When false, rules from this merge file are not inherited by
    /// sub-directories (`n` modifier).
    inherit: bool,
}

#[derive(Clone)]
struct Cached {
    rules: Vec<(usize, Rule)>,
    merges: Vec<(usize, PerDir)>,
}

/// Matcher evaluates rules sequentially against paths.
#[derive(Clone, Default)]
pub struct Matcher {
    /// Root of the walk. Used to locate per-directory filter files.
    root: Option<PathBuf>,
    /// Global rules supplied via CLI or configuration along with their
    /// original positions.
    rules: Vec<(usize, Rule)>,
    /// Filenames that should be merged for every directory visited along
    /// with the position of the `dir-merge` directive.
    per_dir: Vec<(usize, PerDir)>,
    /// Additional per-directory merge directives discovered while walking,
    /// keyed by directory and preserving positional information.
    extra_per_dir: RefCell<HashMap<PathBuf, Vec<(usize, PerDir)>>>,
    /// Cache of parsed rules for filter files along with any nested merges.
    cached: RefCell<HashMap<PathBuf, Cached>>,
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

    /// Set the root directory used to resolve per-directory filter files.
    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    /// Determine if the provided path is included by the rules, loading any
    /// directory-specific `.rsync-filter` files along the way. Rules are
    /// evaluated in the exact order that rsync would apply them: command-line
    /// rules appear in their original sequence, and per-directory rules are
    /// injected at the position where the corresponding `dir-merge` directive
    /// was specified.
    pub fn is_included<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        let path = path.as_ref();
        let mut seq = 0usize;
        let mut active: Vec<(usize, usize, usize, Rule)> = Vec::new();

        for (idx, rule) in &self.rules {
            active.push((*idx, 0, seq, rule.clone()));
            seq += 1;
        }

        if let Some(root) = &self.root {
            let mut dirs = vec![root.clone()];
            if let Some(parent) = path.parent() {
                let mut dir = root.clone();
                for comp in parent.components() {
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
                    if !data.flags.applies_to_sender() {
                        continue;
                    }
                    let matched = data.matcher.is_match(path);
                    if (data.invert && !matched) || (!data.invert && matched) {
                        continue;
                    }
                }
                Rule::Include(data) => {
                    if !data.flags.applies_to_sender() {
                        continue;
                    }
                    let matched = data.matcher.is_match(path);
                    if (data.invert && !matched) || (!data.invert && matched) {
                        return Ok(true);
                    }
                }
                Rule::Exclude(data) => {
                    if !data.flags.applies_to_sender() {
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

    /// Merge additional global rules, as when reading a top-level filter file.
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

    /// Load rules applicable to `dir`, preserving the position of the
    /// originating `dir-merge` directive.
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
            let state = if let Some(c) = cache.get(&path) {
                c.clone()
            } else {
                let mut visited = HashSet::new();
                visited.insert(path.clone());
                let (rules, merges) =
                    self.load_merge_file(dir, &path, rel.as_deref(), &mut visited, 0, idx)?;
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
        visited: &mut HashSet<PathBuf>,
        depth: usize,
        index: usize,
    ) -> Result<(Vec<(usize, Rule)>, Vec<(usize, PerDir)>), ParseError> {
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
        };

        let mut rules = Vec::new();
        let mut merges = Vec::new();
        let parsed = parse(&adjusted, visited, depth + 1)?;

        for r in parsed {
            match r {
                Rule::DirMerge(pd) => {
                    merges.push((index, pd.clone()));
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
                        self.load_merge_file(dir, &nested, rel2, visited, depth + 1, index)?;
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

/// Parse filter rules from input with recursion tracking.
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
            if rest.chars().all(|c| c == 'F') {
                rules.push(Rule::DirMerge(PerDir {
                    file: ".rsync-filter".to_string(),
                    anchored: false,
                    root_only: false,
                    inherit: true,
                }));
                if !rest.is_empty() {
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
            (Some(RuleKind::Include), m, rest)
        } else if let Some(r) = line.strip_prefix('-') {
            let (m, rest) = split_mods(r, "/!Csrpx");
            (Some(RuleKind::Exclude), m, rest)
        } else if let Some(r) = line.strip_prefix('P').or_else(|| line.strip_prefix('p')) {
            let (m, rest) = split_mods(r, "/!Csrpx");
            (Some(RuleKind::Protect), m, rest)
        } else if let Some(r) = line.strip_prefix('.') {
            let (m, file) = split_mods(r, "-+Cenw/!srpx");
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
        } else if let Some(r) = line.strip_prefix(':') {
            let (m, file) = split_mods(r, "-+Cenw/!srpx");
            if file.is_empty() {
                return Err(ParseError::InvalidRule(raw_line.to_string()));
            }
            let anchored = file.starts_with('/') || m.contains('/');
            let fname = if anchored {
                file.trim_start_matches('/').to_string()
            } else {
                file.to_string()
            };
            rules.push(Rule::DirMerge(PerDir {
                file: fname.clone(),
                anchored,
                root_only: false,
                inherit: !m.contains('n'),
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
                "include" => (Some(RuleKind::Include), "", rest),
                "exclude" => (Some(RuleKind::Exclude), "", rest),
                "protect" => (Some(RuleKind::Protect), "", rest),
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
                    }));
                    continue;
                }
                _ => (None, "", rest),
            }
        };

        // Handle special CVS-exclude insertion when using the C modifier with no pattern.
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

const CVS_DEFAULTS: &[&str] = &[
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

fn default_cvs_rules() -> Result<Vec<Rule>, ParseError> {
    let mut out = Vec::new();
    for pat in CVS_DEFAULTS {
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
    }
    Ok(out)
}
