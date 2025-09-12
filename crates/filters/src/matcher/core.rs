// crates/filters/src/matcher/core.rs

use crate::{parser::ParseError, perdir::PerDir, rule::Rule, stats::FilterStats};
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone)]
pub(super) struct Cached {
    pub(super) rules: Vec<(usize, Rule)>,
    pub(super) merges: Vec<(usize, PerDir)>,
    pub(super) mtime: Option<SystemTime>,
    pub(super) len: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MatchResult {
    pub include: bool,
    pub descend: bool,
}

#[derive(Clone, Default)]
pub struct Matcher {
    pub(super) root: Option<PathBuf>,
    pub(super) rules: Vec<(usize, Rule)>,
    pub(super) per_dir: Vec<(usize, PerDir)>,
    pub(super) extra_per_dir: RefCell<HashMap<PathBuf, Vec<(usize, PerDir)>>>,
    #[allow(clippy::type_complexity)]
    pub(super) cached: RefCell<HashMap<(PathBuf, Option<char>, bool), Cached>>,
    pub(super) existing: bool,
    pub(super) prune_empty_dirs: bool,
    pub(super) from0: bool,
    pub(super) no_implied_dirs: bool,
    pub(super) stats: RefCell<FilterStats>,
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
            target: "filter",
            matches = stats.matches,
            misses = stats.misses,
            source = %source,
        );
    }
}
