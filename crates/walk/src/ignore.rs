// crates/walk/src/ignore.rs
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct Ignore {
    paths: HashSet<PathBuf>,
}

impl Ignore {
    pub fn new<I, P>(paths: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let paths = paths.into_iter().map(Into::into).collect();
        Ignore { paths }
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        self.paths.iter().any(|p| path.starts_with(p))
    }
}
