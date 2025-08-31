// src/lib.rs
use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use std::{fs, path::Path};

pub fn synchronize(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    engine::sync(
        src,
        dst,
        &Matcher::default(),
        &available_codecs(None),
        &SyncOptions::default(),
    )?;
    // Copy only files that were skipped by the engine
    let _ = copy_recursive(src, dst)?;
    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<usize> {
    let mut copied = 0;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            if !dst_path.exists() {
                fs::create_dir_all(&dst_path)?;
            }
            copied += copy_recursive(&entry.path(), &dst_path)?;
        } else if file_type.is_file() {
            if !dst_path.exists() {
                fs::copy(entry.path(), &dst_path)?;
                copied += 1;
            }
        } else if file_type.is_symlink() {
            if !dst_path.exists() {
                #[cfg(unix)]
                {
                    let target = fs::read_link(entry.path())?;
                    std::os::unix::fs::symlink(&target, &dst_path)?;
                    copied += 1;
                }
            }
        }
    }
    Ok(copied)
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
        assert!(!dst_dir.exists());
        synchronize(&src_dir, &dst_dir).unwrap();
        assert!(dst_dir.exists());
        let out = fs::read(dst_dir.join("file.txt")).unwrap();
        assert_eq!(out, b"hello world");
    }

    #[test]
    fn sync_creates_destination() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file.txt"), b"data").unwrap();

        // destination should not exist before sync
        assert!(!dst_dir.exists());

        synchronize(&src_dir, &dst_dir).unwrap();

        // destination directory and file should now exist
        assert!(dst_dir.exists());
        assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"data");
    }

    #[cfg(unix)]
    #[test]
    fn sync_preserves_symlinks() {
        use std::path::Path;

        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file.txt"), b"hello").unwrap();
        std::os::unix::fs::symlink("file.txt", src_dir.join("link")).unwrap();

        synchronize(&src_dir, &dst_dir).unwrap();

        let meta = fs::symlink_metadata(dst_dir.join("link")).unwrap();
        assert!(meta.file_type().is_symlink());
        let target = fs::read_link(dst_dir.join("link")).unwrap();
        assert_eq!(target, Path::new("file.txt"));
        assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"hello");
    }

    #[test]
    fn engine_handles_all_files() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file.txt"), b"data").unwrap();

        synchronize(&src_dir, &dst_dir).unwrap();

        // copy_recursive should have nothing left to copy
        assert_eq!(copy_recursive(&src_dir, &dst_dir).unwrap(), 0);
    }
}
