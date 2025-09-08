// crates/walk/src/fs.rs
#[cfg(windows)]
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub const VERBATIM_PREFIX: &str = r"\\?\\";

#[cfg(windows)]
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let s = path.as_ref().as_os_str().to_string_lossy();
    if s.starts_with(VERBATIM_PREFIX) {
        PathBuf::from(s.into_owned())
    } else {
        PathBuf::from(format!("{}{}", VERBATIM_PREFIX, s))
    }
}

pub fn common_prefix_len(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}
