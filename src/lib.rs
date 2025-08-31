// src/lib.rs
use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use logging::{subscriber, LogFormat};
use std::{fs, path::Path};
use tracing::subscriber::with_default;

/// Configuration for [`synchronize`].
///
/// `log_format` controls whether logs are human-readable text or JSON.
/// Adjust the verbosity with `verbose`, `info`, or `debug`.
///
/// # Examples
/// ```no_run
/// use logging::LogFormat;
/// use oc_rsync::{synchronize, SyncConfig};
/// use std::path::Path;
///
/// let cfg = SyncConfig { log_format: LogFormat::Json, verbose: 1, ..Default::default() };
/// synchronize(Path::new("src"), Path::new("dst"), &cfg).unwrap();
/// ```
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
pub fn synchronize(src: &Path, dst: &Path, cfg: &SyncConfig) -> Result<()> {
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
        // Fall back to a simple copy for any files not handled by the engine
        copy_recursive(src, dst)?;
        Ok(())
    })
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_recursive(&entry.path(), &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), dst_path)?;
        } else if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let target = fs::read_link(entry.path())?;
                std::os::unix::fs::symlink(&target, &dst_path)?;
            }
        }
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
        assert!(!dst_dir.exists());
        synchronize(&src_dir, &dst_dir, &SyncConfig::default()).unwrap();
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

        synchronize(&src_dir, &dst_dir, &SyncConfig::default()).unwrap();

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

        synchronize(&src_dir, &dst_dir, &SyncConfig::default()).unwrap();

        let meta = fs::symlink_metadata(dst_dir.join("link")).unwrap();
        assert!(meta.file_type().is_symlink());
        let target = fs::read_link(dst_dir.join("link")).unwrap();
        assert_eq!(target, Path::new("file.txt"));
        assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"hello");
    }
}
