// tests/util/mod.rs
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use tempfile::{TempDir, tempdir};

pub fn setup_files_from_env(entries: &[(&str, &[u8])]) -> (TempDir, PathBuf, PathBuf) {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    for (path, contents) in entries {
        let full = src.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(full, contents).unwrap();
    }
    (dir, src, dst)
}
