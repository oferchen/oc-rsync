// crates/engine/src/io.rs

#![doc = include_str!("docs/io.md")]

use std::fs::File;
use std::path::Path;

use crate::EngineError;

pub fn io_context(path: &Path, err: std::io::Error) -> EngineError {
    EngineError::Io(std::io::Error::new(
        err.kind(),
        format!("{}: {}", path.display(), err),
    ))
}

pub fn is_device(file_type: &std::fs::FileType) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        file_type.is_block_device() || file_type.is_char_device()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(unix)]
#[doc = "Preallocate space for a file on supported Unix platforms.\n\n\
# Safety\n\
On macOS, this function calls `fcntl` and `ftruncate` on the provided file.\n\
The descriptor must be valid and all system calls check their return values;\n\
any errors are propagated to the caller."]
pub fn preallocate(file: &File, len: u64) -> std::io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        use nix::fcntl::{FallocateFlags, fallocate};
        fallocate(file, FallocateFlags::empty(), 0, len as i64).map_err(std::io::Error::from)
    }

    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        // SAFETY: `file` provides a valid descriptor and all libc calls check their return values.
        unsafe {
            let fd = file.as_raw_fd();
            let mut fstore = libc::fstore_t {
                fst_flags: libc::F_ALLOCATECONTIG,
                fst_posmode: libc::F_PEOFPOSMODE,
                fst_offset: 0,
                fst_length: len as libc::off_t,
                fst_bytesalloc: 0,
            };
            let ret = libc::fcntl(fd, libc::F_PREALLOCATE, &fstore);
            if ret == -1 {
                fstore.fst_flags = libc::F_ALLOCATEALL;
                if libc::fcntl(fd, libc::F_PREALLOCATE, &fstore) == -1 {
                    if libc::ftruncate(fd, len as libc::off_t) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                    return Ok(());
                }
            }
            if libc::ftruncate(fd, len as libc::off_t) == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris",
    ))]
    {
        use std::os::fd::AsRawFd;
        // SAFETY: `file` yields a valid descriptor and `posix_fallocate` is checked for errors.
        unsafe {
            let ret = libc::posix_fallocate(file.as_raw_fd(), 0, len as libc::off_t);
            if ret == 0 {
                Ok(())
            } else {
                Err(std::io::Error::from_raw_os_error(ret))
            }
        }
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris",
    )))]
    {
        file.set_len(len)
    }
}

#[cfg(not(unix))]
pub fn preallocate(_file: &File, _len: u64) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
    #[test]
    fn preallocate_failure_surfaces_error() {
        use tempfile::tempdir;

        let tmp = tempdir().unwrap();
        let path = tmp.path().join("file");
        File::create(&path).unwrap();
        let file = std::fs::OpenOptions::new().read(true).open(&path).unwrap();
        let err = preallocate(&file, 1).unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::EBADF));
    }
}
