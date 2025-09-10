// tests/util/mod.rs
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{TempDir, tempdir};
use walkdir::WalkDir;

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

pub fn compare_trees(expected: &Path, actual: &Path) -> bool {
    fn collect(dir: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
        let mut map = BTreeMap::new();
        for entry in WalkDir::new(dir).sort_by_file_name() {
            let entry = entry.unwrap();
            let rel = entry.path().strip_prefix(dir).unwrap();
            if rel.as_os_str().is_empty() {
                continue;
            }
            let key = rel.to_string_lossy().replace('\\', "/");
            let val = if entry.file_type().is_file() {
                Some(fs::read(entry.path()).unwrap())
            } else {
                None
            };
            map.insert(key, val);
        }
        map
    }

    collect(expected) == collect(actual)
}
