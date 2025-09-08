// crates/transport/src/temp.rs
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct TempPathGuard {
    path: PathBuf,
}

impl TempPathGuard {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }

    pub fn disarm(&mut self) {
        self.path.clear();
    }

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

pub type TempSocketGuard = TempPathGuard;

pub type TempFileGuard = TempPathGuard;
