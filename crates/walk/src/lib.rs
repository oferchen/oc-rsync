use std::fs::FileType;
use std::path::{Path, PathBuf};
use walkdir::Error;

/// Walk a directory tree yielding paths and their file types.
///
/// This is a thin wrapper around the `walkdir` crate that exposes all entries
/// (files, directories, and symlinks) along with their [`FileType`]. Unlike the
/// previous implementation, this returns [`Result`] items so callers can handle
/// traversal errors instead of silently discarding them.
pub fn walk(root: impl AsRef<Path>) -> impl Iterator<Item = Result<(PathBuf, FileType), Error>> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(|e| e.map(|entry| (entry.path().to_path_buf(), entry.file_type())))
}
