// crates/filters/src/matcher/evaluate.rs
#![allow(clippy::collapsible_if)]

use super::core::{MatchResult, Matcher};
use super::path::rule_matches;
use crate::{parser::ParseError, rule::Rule};
use logging::InfoFlag;
use std::{
    fs,
    path::{Path, PathBuf},
};

impl Matcher {
    pub(crate) fn check(
        &self,
        path: &Path,
        for_delete: bool,
        xattr: bool,
    ) -> Result<MatchResult, ParseError> {
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

        let per_dir_offset = self.rules.iter().map(|(i, _)| *i).max().unwrap_or(0) + 1;

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
                    let mut idx_adj = per_dir_offset + idx;
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
        let mut decided = false;
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
                        if !decided {
                            include = Some(true);
                            matched_source = data.source.clone();
                            decided = true;
                        }
                        if may_desc {
                            descend = true;
                        }
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
                        if !decided {
                            include = Some(true);
                            matched_source = data.source.clone();
                            decided = true;
                        }
                        if may_desc {
                            descend = true;
                        }
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
                            if !decided {
                                include = Some(true);
                                matched_source = data.source.clone();
                                decided = true;
                            }
                            if may_desc {
                                descend = true;
                            }
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
                        if !decided {
                            include = Some(false);
                            if !data.dir_only && !descend {
                                descend = may_desc;
                            }
                            matched_source = data.source.clone();
                            decided = true;
                        } else if may_desc {
                            descend = true;
                        }
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
            if decided && (!is_dir || descend) {
                break;
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
}
