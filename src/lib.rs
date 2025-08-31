// src/lib.rs
use compress::available_codecs;
use engine::{Result, SyncOptions};
use filetime::{set_file_times, set_symlink_file_times, FileTime};
use filters::Matcher;
#[cfg(unix)]
use nix::{
    sys::stat::{mknod, Mode, SFlag},
    unistd::mkfifo,
};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
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
    // Fall back to a simple copy for any files not handled by the engine
    copy_recursive(src, dst)?;
    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let meta = fs::symlink_metadata(&src_path)?;
        let file_type = meta.file_type();
        if file_type.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        } else if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let target = fs::read_link(&src_path)?;
                std::os::unix::fs::symlink(&target, &dst_path)?;
                let atime = FileTime::from_last_access_time(&meta);
                let mtime = FileTime::from_last_modification_time(&meta);
                set_symlink_file_times(&dst_path, atime, mtime)?;
            }
            #[cfg(not(unix))]
            {
                let target = fs::read_link(&src_path)?;
                std::os::windows::fs::symlink_file(&target, &dst_path)?;
            }
            continue;
        } else {
            #[cfg(unix)]
            {
                let mode = Mode::from_bits_truncate(meta.permissions().mode());
                use std::io;
                if file_type.is_fifo() {
                    mkfifo(&dst_path, mode).map_err(|e| io::Error::from_raw_os_error(e as i32))?;
                } else if file_type.is_char_device() {
                    mknod(&dst_path, SFlag::S_IFCHR, mode, meta.rdev() as u64)
                        .map_err(|e| io::Error::from_raw_os_error(e as i32))?;
                } else if file_type.is_block_device() {
                    mknod(&dst_path, SFlag::S_IFBLK, mode, meta.rdev() as u64)
                        .map_err(|e| io::Error::from_raw_os_error(e as i32))?;
                } else {
                    continue;
                }
            }
            #[cfg(not(unix))]
            {
                continue;
            }
        }

        let atime = FileTime::from_last_access_time(&meta);
        let mtime = FileTime::from_last_modification_time(&meta);
        set_file_times(&dst_path, atime, mtime)?;
        fs::set_permissions(&dst_path, meta.permissions())?;
    }
    Ok(())
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
        fs::create_dir_all(&dst_dir).unwrap();
        assert!(dst_dir.exists());
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

    #[cfg(unix)]
    #[test]
    fn sync_preserves_metadata() {
        use filetime::{set_file_times, FileTime};
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();

        let file = src_dir.join("file.txt");
        fs::write(&file, b"hello").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o744)).unwrap();
        let atime = FileTime::from_unix_time(1_000_000, 0);
        let mtime = FileTime::from_unix_time(1_000_100, 0);
        set_file_times(&file, atime, mtime).unwrap();

        synchronize(&src_dir, &dst_dir).unwrap();

        let meta = fs::metadata(dst_dir.join("file.txt")).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o744);
        let dst_atime = FileTime::from_last_access_time(&meta);
        let dst_mtime = FileTime::from_last_modification_time(&meta);
        assert_eq!(dst_atime, atime);
        assert_eq!(dst_mtime, mtime);
    }

    #[cfg(unix)]
    #[test]
    fn sync_preserves_fifo() {
        use filetime::{set_file_times, FileTime};
        use nix::sys::stat::Mode;
        use nix::unistd::mkfifo;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();

        let fifo = src_dir.join("fifo");
        mkfifo(&fifo, Mode::from_bits_truncate(0o640)).unwrap();
        let atime = FileTime::from_unix_time(2_000_000, 0);
        let mtime = FileTime::from_unix_time(2_000_100, 0);
        set_file_times(&fifo, atime, mtime).unwrap();

        synchronize(&src_dir, &dst_dir).unwrap();

        let dst_path = dst_dir.join("fifo");
        let meta = fs::metadata(&dst_path).unwrap();
        assert!(meta.file_type().is_fifo());
        assert_eq!(meta.permissions().mode() & 0o777, 0o640);
        let dst_atime = FileTime::from_last_access_time(&meta);
        let dst_mtime = FileTime::from_last_modification_time(&meta);
        assert_eq!(dst_atime, atime);
        assert_eq!(dst_mtime, mtime);
    }
}
