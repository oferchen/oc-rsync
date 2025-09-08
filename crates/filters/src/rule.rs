// crates/filters/src/rule.rs
use globset::{GlobBuilder, GlobMatcher};
use std::path::{Path, PathBuf};

#[derive(Clone, Default, Hash, PartialEq, Eq)]
pub struct RuleFlags {
    pub(crate) sender: bool,
    pub(crate) receiver: bool,
    pub(crate) perishable: bool,
    pub(crate) xattr: bool,
}

impl RuleFlags {
    pub(crate) fn from_mods(mods: &str) -> Self {
        let mut flags = Self::default();
        for ch in mods.chars() {
            match ch {
                's' => flags.sender = true,
                'r' => flags.receiver = true,
                'p' => flags.perishable = true,
                'x' => flags.xattr = true,
                _ => {}
            }
        }
        flags
    }

    pub(crate) fn applies(&self, for_delete: bool, xattr: bool) -> bool {
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

    pub(crate) fn union(&self, other: &Self) -> Self {
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
    pub(crate) matcher: GlobMatcher,
    pub(crate) invert: bool,
    pub(crate) flags: RuleFlags,
    pub(crate) source: Option<PathBuf>,
    pub(crate) dir_only: bool,
    pub(crate) has_slash: bool,
    pub(crate) pattern: String,
}

impl RuleData {
    pub(crate) fn is_match(&self, path: &Path, _is_dir: bool) -> bool {
        let full = format!("/{}", path.to_string_lossy());
        let full_match = self.matcher.is_match(&full);
        if self.has_slash {
            full_match
        } else {
            let file_match = path
                .file_name()
                .map(|name| self.matcher.is_match(Path::new(name)))
                .unwrap_or(false);
            full_match || file_match
        }
    }

    pub(crate) fn may_match_descendant(&self, path: &Path) -> bool {
        if !self.has_slash {
            return false;
        }

        let pat_trim = self.pattern.trim_end_matches('/');
        let pat_segments: Vec<&str> = pat_trim.split('/').filter(|s| !s.is_empty()).collect();
        let path_segments: Vec<String> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();

        let mut idx = 0;
        while idx < path_segments.len() {
            if idx >= pat_segments.len() {
                return false;
            }
            let pat_seg = pat_segments[idx];
            if pat_seg == "**" {
                return true;
            }
            if pat_seg.contains('*') || pat_seg.contains('?') || pat_seg.contains('[') {
                let m = match GlobBuilder::new(pat_seg)
                    .literal_separator(true)
                    .backslash_escape(true)
                    .build()
                {
                    Ok(g) => g.compile_matcher(),
                    Err(_) => return false,
                };
                if !m.is_match(&path_segments[idx]) {
                    return false;
                }
            } else if pat_seg != path_segments[idx] {
                return false;
            }
            idx += 1;
        }

        idx < pat_segments.len()
    }
}

use crate::perdir::PerDir;

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
