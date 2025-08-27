use std::fs::FileType;
use std::path::{Path, PathBuf};

/// Walk a directory tree yielding paths and their file types.
///
/// This is a thin wrapper around the `walkdir` crate that exposes all entries
/// (files, directories, and symlinks) along with their [`FileType`].
pub fn walk(root: impl AsRef<Path>) -> impl Iterator<Item = (PathBuf, FileType)> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok().map(|entry| (entry.path().to_path_buf(), entry.file_type())))
}
