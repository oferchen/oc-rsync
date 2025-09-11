// crates/filters/src/matcher/path.rs
#![allow(clippy::collapsible_if)]

use super::core::{MatchResult, Matcher};
use crate::{parser::ParseError, rule::RuleData};
use std::path::Path;

pub(super) fn rule_matches(data: &RuleData, path: &Path, is_dir: bool) -> bool {
    let mut matched = data.is_match(path, is_dir);
    let pat_core = data
        .pattern
        .trim_start_matches('/')
        .trim_start_matches("./");
    if matched && pat_core.starts_with("**/") {
        let rest = &pat_core[3..];
        if (rest.contains('*') || rest.contains('?'))
            && !rest.contains("**")
            && path.components().count() > 1
            && rest != "*"
            && rest != "?"
        {
            matched = false;
        }
    } else if matched
        && (pat_core.contains('*') || pat_core.contains('?'))
        && !pat_core.contains("**")
    {
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
        } else if path.components().count() > 1 && pat_core != "*" && pat_core != "?" {
            matched = false;
        }
    }
    matched
}

impl Matcher {
    pub fn is_included<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), false, false).map(|r| r.include)
    }

    pub fn is_included_with_dir<P: AsRef<Path>>(&self, path: P) -> Result<MatchResult, ParseError> {
        self.check(path.as_ref(), false, false)
    }
}
