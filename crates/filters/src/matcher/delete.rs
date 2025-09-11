// crates/filters/src/matcher/delete.rs
use super::core::{MatchResult, Matcher};
use crate::parser::ParseError;
use std::path::Path;

impl Matcher {
    pub fn is_included_for_delete<P: AsRef<Path>>(&self, path: P) -> Result<bool, ParseError> {
        self.check(path.as_ref(), true, false).map(|r| r.include)
    }

    pub fn is_included_for_delete_with_dir<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<MatchResult, ParseError> {
        self.check(path.as_ref(), true, false)
    }
}
