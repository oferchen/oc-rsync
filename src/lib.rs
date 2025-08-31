// src/lib.rs
use compress::available_codecs;
use engine::{EngineError, Result, SyncOptions};
use filters::Matcher;
use std::{fs, io, path::Path};
use filetime::{set_file_times, set_symlink_file_times, FileTime};
#[cfg(unix)]
use nix::{
    sys::stat::{mknod, Mode, SFlag},
    unistd::mkfifo,
};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::{fs, path::Path};

/// Synchronizes the contents of the `src` directory into the `dst` directory.
///
/// The destination directory is created if it does not exist and any existing
/// files are overwritten to match the source. The rsync engine performs the
/// main transfer using default options and available compression codecs. After
/// the engine runs, any files it does not handle are copied with a simple
/// recursive copy via `copy_recursive`.
///
/// # Errors
///
/// Returns an error if the destination cannot be created, if reading from the
/// source fails, if the underlying engine encounters an error, or if copying
/// any remaining files fails.
///
/// # Examples
///
/// ```
/// use std::fs;
/// use oc_rsync::synchronize;
/// # use tempfile::tempdir;
/// # let dir = tempdir().unwrap();
/// # let src = dir.path().join("src");
/// # let dst = dir.path().join("dst");
/// # fs::create_dir(&src).unwrap();
/// # fs::write(src.join("file.txt"), b"hello").unwrap();
/// synchronize(&src, &dst).unwrap();
/// assert_eq!(fs::read(dst.join("file.txt")).unwrap(), b"hello");
/// ```
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

fn io_context(path: &Path, err: io::Error) -> EngineError {
    EngineError::Io(io::Error::new(
        err.kind(),
        format!("{}: {}", path.display(), err),
    ))
}

fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src).map_err(|e| io_context(src, e))? {
        let entry = entry.map_err(|e| io_context(src, e))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|e| io_context(&path, e))?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let meta = fs::symlink_metadata(&src_path)?;
        let file_type = meta.file_type();
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
            fs::create_dir_all(&dst_path).map_err(|e| io_context(&dst_path, e))?;
            copy_recursive(&path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&path, &dst_path).map_err(|e| io_context(&path, e))?;
            fs::create_dir_all(&dst_path)?;
            copy_recursive(&src_path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&src_path, &dst_path)?;
        } else if file_type.is_symlink() {
            let target = fs::read_link(entry.path())?;
            #[cfg(unix)]
            {
                let target = fs::read_link(&path).map_err(|e| io_context(&path, e))?;
                std::os::unix::fs::symlink(&target, &dst_path)
                    .map_err(|e| io_context(&dst_path, e))?;

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
            #[cfg(windows)]
            {
                use std::os::windows::fs::{symlink_dir, symlink_file};
                match fs::metadata(entry.path()) {
                    Ok(m) if m.is_dir() => symlink_dir(&target, &dst_path)?,
                    _ => symlink_file(&target, &dst_path)?,
                };
            }
        }

        let atime = FileTime::from_last_access_time(&meta);
        let mtime = FileTime::from_last_modification_time(&meta);
        set_file_times(&dst_path, atime, mtime)?;
        fs::set_permissions(&dst_path, meta.permissions())?;
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
        fs::create_dir_all(&dst_dir).unwrap();
        fs::File::create(src_dir.join("file.txt"))
            .unwrap()
            .write_all(b"hello world")
            .unwrap();
        assert!(!dst_dir.exists());
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

    #[cfg(any(unix, windows))]
    #[test]
    fn sync_preserves_symlinks() {
        use std::path::Path;

        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file.txt"), b"hello").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("file.txt", src_dir.join("link")).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file("file.txt", src_dir.join("link")).unwrap();

        synchronize(&src_dir, &dst_dir).unwrap();

        let meta = fs::symlink_metadata(dst_dir.join("link")).unwrap();
        assert!(meta.file_type().is_symlink());
        let target = fs::read_link(dst_dir.join("link")).unwrap();
        assert_eq!(target, Path::new("file.txt"));
        assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"hello");
    }

    #[test]
    fn engine_handles_all_files() {
    #[cfg(unix)]
    fn run_copy_unprivileged(src: &Path, dst: &Path) -> (i32, String) {
        use nix::sys::wait::{waitpid, WaitStatus};
        use nix::unistd::{fork, setuid, ForkResult, Uid};
        use std::io::{Read, Write};
        use std::os::unix::net::UnixStream;

        let (mut parent_sock, mut child_sock) = UnixStream::pair().unwrap();
        match unsafe { fork() }.expect("fork failed") {
            ForkResult::Child => {
                drop(parent_sock);
                setuid(Uid::from_raw(1)).unwrap();
                let res = copy_recursive(src, dst);
                let msg = match &res {
                    Ok(_) => "ok".to_string(),
                    Err(e) => e.to_string(),
                };
                child_sock.write_all(msg.as_bytes()).unwrap();
                std::process::exit(if res.is_err() { 0 } else { 1 });
            }
            ForkResult::Parent { child } => {
                drop(child_sock);
                let mut msg = String::new();
                parent_sock.read_to_string(&mut msg).unwrap();
                let status = waitpid(child, None).unwrap();
                let code = match status {
                    WaitStatus::Exited(_, c) => c,
                    _ => -1,
                };
                (code, msg)
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn copy_recursive_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        fs::set_permissions(&dst_dir, fs::Permissions::from_mode(0o777)).unwrap();

        let file_path = src_dir.join("file.txt");
        fs::write(&file_path, b"data").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o000)).unwrap();

        let (code, msg) = run_copy_unprivileged(&src_dir, &dst_dir);
        assert_eq!(code, 0);
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("file.txt"));
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
    fn copy_recursive_unreadable_directory() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o755)).unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        let subdir = src_dir.join("sub");
        fs::create_dir_all(&subdir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();
        fs::set_permissions(&dst_dir, fs::Permissions::from_mode(0o777)).unwrap();
        fs::set_permissions(&subdir, fs::Permissions::from_mode(0o000)).unwrap();

        let (code, msg) = run_copy_unprivileged(&src_dir, &dst_dir);
        assert_eq!(code, 0);
        assert!(msg.contains("Permission denied"));
        assert!(msg.contains("sub"));
    fn sync_preserves_fifo() {
        use filetime::{set_file_times, FileTime};
        use nix::sys::stat::Mode;
        use nix::unistd::mkfifo;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        let dst_dir = dir.path().join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("file.txt"), b"data").unwrap();

        synchronize(&src_dir, &dst_dir).unwrap();

        // copy_recursive should have nothing left to copy
        assert_eq!(copy_recursive(&src_dir, &dst_dir).unwrap(), 0);

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
