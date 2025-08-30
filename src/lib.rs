// src/lib.rs
use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use std::path::Path;

pub fn synchronize(src: &Path, dst: &Path) -> Result<()> {
    engine::sync(
        src,
        dst,
        &Matcher::default(),
        &available_codecs(false),
        &SyncOptions::default(),
    )
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn sync_local() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::File::create(src_dir.join("file.txt"))
            .unwrap()
            .write_all(b"hello world")
            .unwrap();
        synchronize(&src_dir, &dst_dir).unwrap();
        let out = fs::read(dst_dir.join("file.txt")).unwrap();
        assert_eq!(out, b"hello world");
    }
}
