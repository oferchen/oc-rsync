// src/lib.rs
use compress::available_codecs;
use engine::{EngineError, Result, SyncOptions};
use filetime::{set_file_times, set_symlink_file_times, FileTime};
use filters::Matcher;
use logging::{subscriber, LogFormat};
#[cfg(unix)]
use nix::{
    sys::stat::{dev_t, mknod, Mode, SFlag},
    unistd::mkfifo,
};
use std::convert::TryInto;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::{fs, io, path::Path};
use tracing::subscriber::with_default;

#[derive(Clone)]
pub struct SyncConfig {
    pub log_format: LogFormat,
    pub verbose: u8,
    pub info: bool,
    pub debug: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Text,
            verbose: 0,
            info: false,
            debug: false,
        }
    }
}

pub fn synchronize_with_config(src: &Path, dst: &Path, cfg: &SyncConfig) -> Result<()> {
    let sub = subscriber(cfg.log_format, cfg.verbose, cfg.info, cfg.debug);
    with_default(sub, || -> Result<()> {
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

        let _ = copy_recursive(src, dst)?;
        Ok(())
    })
}

pub fn synchronize(src: &Path, dst: &Path) -> Result<()> {
    synchronize_with_config(src, dst, &SyncConfig::default())
}

fn io_context(path: &Path, err: io::Error) -> EngineError {
    EngineError::Io(io::Error::new(
        err.kind(),
        format!("{}: {}", path.display(), err),
    ))
}

fn apply_metadata(dst: &Path, meta: &fs::Metadata) -> Result<()> {
    let atime = FileTime::from_last_access_time(meta);
    let mtime = FileTime::from_last_modification_time(meta);

    fs::set_permissions(dst, meta.permissions()).map_err(|e| io_context(dst, e))?;
    set_file_times(dst, atime, mtime).map_err(|e| io_context(dst, e))?;
    Ok(())
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<usize> {
    let mut copied = 0;

    for entry in fs::read_dir(src).map_err(|e| io_context(src, e))? {
        let entry = entry.map_err(|e| io_context(src, e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let meta = fs::symlink_metadata(&src_path).map_err(|e| io_context(&src_path, e))?;
        let file_type = meta.file_type();

        if file_type.is_dir() {
            fs::create_dir_all(&dst_path).map_err(|e| io_context(&dst_path, e))?;
            copied += copy_recursive(&src_path, &dst_path)?;
            apply_metadata(&dst_path, &meta)?;
            continue;
        }

        if file_type.is_file() {
            if !dst_path.exists() {
                fs::copy(&src_path, &dst_path).map_err(|e| io_context(&src_path, e))?;
                copied += 1;
            }
            apply_metadata(&dst_path, &meta)?;
            continue;
        }

        #[cfg(unix)]
        {
            if file_type.is_symlink() {
                if !dst_path.exists() {
                    let target = fs::read_link(&src_path).map_err(|e| io_context(&src_path, e))?;
                    std::os::unix::fs::symlink(&target, &dst_path)
                        .map_err(|e| io_context(&dst_path, e))?;
                    let atime = FileTime::from_last_access_time(&meta);
                    let mtime = FileTime::from_last_modification_time(&meta);
                    set_symlink_file_times(&dst_path, atime, mtime)
                        .map_err(|e| io_context(&dst_path, e))?;
                    copied += 1;
                }
                continue;
            }

            let raw_mode: u16 = meta.permissions().mode().try_into().unwrap();
            let mode = Mode::from_bits_truncate(raw_mode.into());
            use std::io as stdio;
            if file_type.is_fifo() {
                mkfifo(&dst_path, mode)
                    .map_err(|e| stdio::Error::from_raw_os_error(e as i32))
                    .map_err(|e| io_context(&dst_path, e))?;
            } else if file_type.is_char_device() {
                let dev: dev_t = meta.rdev().try_into().unwrap();
                mknod(&dst_path, SFlag::S_IFCHR, mode, dev)
                    .map_err(|e| stdio::Error::from_raw_os_error(e as i32))
                    .map_err(|e| io_context(&dst_path, e))?;
            } else if file_type.is_block_device() {
                let dev: dev_t = meta.rdev().try_into().unwrap();
                mknod(&dst_path, SFlag::S_IFBLK, mode, dev)
                    .map_err(|e| stdio::Error::from_raw_os_error(e as i32))
                    .map_err(|e| io_context(&dst_path, e))?;
            } else {
                continue;
            }
            copied += 1;
            apply_metadata(&dst_path, &meta)?;
            continue;
        }

        #[cfg(windows)]
        {
            if file_type.is_symlink() {
                if !dst_path.exists() {
                    let target = fs::read_link(&src_path).map_err(|e| io_context(&src_path, e))?;
                    match fs::metadata(&src_path) {
                        Ok(m) if m.is_dir() => symlink_dir(&target, &dst_path),
                        _ => symlink_file(&target, &dst_path),
                    }
                    .map_err(|e| io_context(&dst_path, e))?;
                    copied += 1;
                }
                continue;
            }
            continue;
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    fn setup_dirs() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        (dir, src_dir, dst_dir)
    }

    fn assert_no_remaining_copy(src: &Path, dst: &Path) {
        assert_eq!(copy_recursive(src, dst).unwrap(), 0);
    }

    #[test]
    fn sync_local() {
        let (_dir, src_dir, dst_dir) = setup_dirs();
        fs::write(src_dir.join("file.txt"), b"hello world").unwrap();

        assert!(!dst_dir.exists());
        synchronize(&src_dir, &dst_dir).unwrap();
        assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"hello world");
    }

    #[test]
    fn sync_creates_destination() {
        let (_dir, src_dir, dst_dir) = setup_dirs();
        fs::write(src_dir.join("file.txt"), b"data").unwrap();

        assert!(!dst_dir.exists());
        synchronize(&src_dir, &dst_dir).unwrap();
        assert!(dst_dir.exists());
        assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"data");
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn sync_preserves_symlinks() {
        let (_dir, src_dir, dst_dir) = setup_dirs();
        fs::write(src_dir.join("file.txt"), b"hello").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("file.txt", src_dir.join("link")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file("file.txt", src_dir.join("link")).unwrap();

        assert!(!dst_dir.exists());
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
        use std::os::unix::fs::PermissionsExt;

        let (_dir, src_dir, dst_dir) = setup_dirs();

        let file = src_dir.join("file.txt");
        fs::write(&file, b"hello").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o744)).unwrap();
        let atime = FileTime::from_unix_time(1_000_000, 0);
        let mtime = FileTime::from_unix_time(1_000_100, 0);
        set_file_times(&file, atime, mtime).unwrap();

        assert!(!dst_dir.exists());
        synchronize(&src_dir, &dst_dir).unwrap();

        let meta = fs::metadata(dst_dir.join("file.txt")).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o744);
        let dst_mtime = FileTime::from_last_modification_time(&meta);

        assert_eq!(dst_mtime, mtime);
    }

    #[cfg(unix)]
    #[test]
    fn sync_preserves_fifo() {
        use nix::sys::stat::Mode;
        use nix::unistd::mkfifo;
        use std::os::unix::fs::PermissionsExt;

        let (_dir, src_dir, dst_dir) = setup_dirs();
        fs::write(src_dir.join("file.txt"), b"data").unwrap();

        assert!(!dst_dir.exists());
        synchronize(&src_dir, &dst_dir).unwrap();
        assert!(dst_dir.exists());

        assert_no_remaining_copy(&src_dir, &dst_dir);

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
