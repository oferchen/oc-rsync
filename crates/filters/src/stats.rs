// crates/filters/src/stats.rs

use std::path::{Path, PathBuf};

#[derive(Clone, Default)]
pub struct FilterStats {
    pub matches: usize,
    pub misses: usize,
    pub last_source: Option<PathBuf>,
}

impl FilterStats {
    pub(crate) fn record(&mut self, source: Option<&Path>, matched: bool) {
        if matched {
            self.matches += 1;
            self.last_source = source.map(|p| p.to_path_buf());
        } else {
            self.misses += 1;
        }
    }
}
