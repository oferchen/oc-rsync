// crates/transport/src/temp.rs
use std::fs;
use std::path::{Path, PathBuf};

/// Removes the path when dropped.
#[derive(Debug)]
pub struct TempPathGuard {
    path: PathBuf,
}

impl TempPathGuard {
    /// Create a new guard for the given path.
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }

    /// Prevent removal of the path on drop.
    pub fn disarm(&mut self) {
        self.path.clear();
    }

    /// Get the guarded path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        if self.path.as_os_str().is_empty() {
            return;
        }
        let _ = fs::remove_file(&self.path);
    }
}

/// Guard for temporary sockets.
pub type TempSocketGuard = TempPathGuard;

/// Guard for temporary files.
pub type TempFileGuard = TempPathGuard;
