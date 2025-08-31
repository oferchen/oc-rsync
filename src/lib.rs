// src/lib.rs
use compress::available_codecs;
use engine::{EngineError, Result, SyncOptions};
use filters::Matcher;
use std::{fs, io, path::Path};

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
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            fs::create_dir_all(&dst_path).map_err(|e| io_context(&dst_path, e))?;
            copy_recursive(&path, &dst_path)?;
        } else if file_type.is_file() {
            fs::copy(&path, &dst_path).map_err(|e| io_context(&path, e))?;
        } else if file_type.is_symlink() {
            #[cfg(unix)]
            {
                let target = fs::read_link(&path).map_err(|e| io_context(&path, e))?;
                std::os::unix::fs::symlink(&target, &dst_path)
                    .map_err(|e| io_context(&dst_path, e))?;
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
        fs::create_dir_all(&dst_dir).unwrap();
        fs::File::create(src_dir.join("file.txt"))
            .unwrap()
            .write_all(b"hello world")
            .unwrap();
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
    }
}
