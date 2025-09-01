// src/lib.rs
use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use logging::{subscriber, DebugFlag, InfoFlag, LogFormat};
use std::path::Path;
use tracing::subscriber::with_default;

#[derive(Clone)]
pub struct SyncConfig {
    pub log_format: LogFormat,
    pub verbose: u8,
    pub info: Vec<InfoFlag>,
    pub debug: Vec<DebugFlag>,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Text,
            verbose: 0,
            info: Vec::new(),
            debug: Vec::new(),
        }
    }
}

pub fn synchronize_with_config(src: &Path, dst: &Path, cfg: &SyncConfig) -> Result<()> {
    let sub = subscriber(
        cfg.log_format,
        cfg.verbose,
        &cfg.info,
        &cfg.debug,
        false,
        None,
    );
    with_default(sub, || -> Result<()> {
        let opts = SyncOptions {
            perms: true,
            times: true,
            atimes: true,
            links: true,
            devices: true,
            specials: true,
            ..SyncOptions::default()
        };
        engine::sync(src, dst, &Matcher::default(), &available_codecs(), &opts)?;
        Ok(())
    })
}

pub fn synchronize(src: &Path, dst: &Path) -> Result<()> {
    synchronize_with_config(src, dst, &SyncConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{set_file_times, FileTime};
    use std::{fs, path::Path};
    use tempfile::{tempdir, TempDir};

    fn setup_dirs() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        (dir, src_dir, dst_dir)
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
        use std::os::unix::fs::{FileTypeExt, PermissionsExt};

        let (_dir, src_dir, dst_dir) = setup_dirs();
        fs::write(src_dir.join("file.txt"), b"data").unwrap();

        assert!(!dst_dir.exists());
        synchronize(&src_dir, &dst_dir).unwrap();
        assert!(dst_dir.exists());

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

    #[cfg(unix)]
    #[test]
    fn sync_preserves_directory_metadata() {
        use std::os::unix::fs::PermissionsExt;

        let (_dir, src_dir, dst_dir) = setup_dirs();

        let subdir = src_dir.join("sub");
        fs::create_dir(&subdir).unwrap();
        fs::set_permissions(&subdir, fs::Permissions::from_mode(0o711)).unwrap();
        fs::write(subdir.join("file.txt"), b"data").unwrap();
        let atime = FileTime::from_unix_time(3_000_000, 0);
        let mtime = FileTime::from_unix_time(3_000_100, 0);
        set_file_times(&subdir, atime, mtime).unwrap();

        synchronize(&src_dir, &dst_dir).unwrap();

        let meta = fs::metadata(dst_dir.join("sub")).unwrap();
        assert!(meta.is_dir());
        assert_eq!(meta.permissions().mode() & 0o777, 0o711);
        let dst_mtime = FileTime::from_last_modification_time(&meta);
        assert_eq!(dst_mtime, mtime);
    }
}
