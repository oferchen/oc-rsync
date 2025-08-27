use std::path::{Path, PathBuf};

/// Walk a directory tree yielding all file paths.
///
/// This is a thin wrapper around the `walkdir` crate that filters out
/// directories and returns only regular files.
pub fn walk(root: impl AsRef<Path>) -> impl Iterator<Item = PathBuf> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| {
            e.ok().and_then(|entry| {
                if entry.file_type().is_file() {
                    Some(entry.path().to_path_buf())
                } else {
                    None
                }
            })
        })
}
