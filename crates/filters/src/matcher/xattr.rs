// crates/filters/src/matcher/xattr.rs
use super::core::{MatchResult, Matcher};
use crate::parser::ParseError;
use std::path::Path;

impl Matcher {
    pub fn is_xattr_included<P: AsRef<Path>>(&self, name: P) -> Result<bool, ParseError> {
        self.check(name.as_ref(), false, true).map(|r| r.include)
    }

    pub fn is_xattr_included_for_delete<P: AsRef<Path>>(
        &self,
        name: P,
    ) -> Result<bool, ParseError> {
        self.check(name.as_ref(), true, true).map(|r| r.include)
    }

    pub fn is_xattr_included_for_delete_with_dir<P: AsRef<Path>>(
        &self,
        name: P,
    ) -> Result<MatchResult, ParseError> {
        self.check(name.as_ref(), true, true)
    }
}
