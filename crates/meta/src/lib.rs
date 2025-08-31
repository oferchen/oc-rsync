#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::*;

// Re-export device number helpers so consumers can construct and
// deconstruct `dev_t` values without depending on `nix` directly.
#[cfg(target_os = "linux")]
pub use nix::sys::stat::{makedev, major, minor};

#[cfg(target_os = "macos")]
pub use libc::{makedev, major, minor};

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod stub;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub use stub::*;

mod parse;
pub use parse::{parse_chmod, parse_chmod_spec, parse_chown, parse_id_map};

impl Options {
    pub fn needs_metadata(&self) -> bool {
        self.xattrs
            || self.acl
            || self.chmod.is_some()
            || self.owner
            || self.group
            || self.perms
            || self.executability
            || self.times
            || self.atimes
            || self.crtimes
            || self.uid_map.is_some()
            || self.gid_map.is_some()
    }
}

#[cfg(unix)]
use filetime::set_symlink_file_times;
use filetime::{set_file_times, FileTime};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Tracks a file's access time and restores it when dropped.
#[derive(Debug)]
pub struct AccessTime {
    path: PathBuf,
    atime: FileTime,
    mtime: FileTime,
    is_symlink: bool,
}

impl AccessTime {
    /// Capture the current access time of `path`.
    pub fn new(path: &Path) -> io::Result<Self> {
        let meta = fs::symlink_metadata(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            atime: FileTime::from_last_access_time(&meta),
            mtime: FileTime::from_last_modification_time(&meta),
            is_symlink: meta.file_type().is_symlink(),
        })
    }

    /// Restore the previously captured access time.
    pub fn restore(&self) -> io::Result<()> {
        if self.is_symlink {
            #[cfg(unix)]
            {
                set_symlink_file_times(&self.path, self.atime, self.mtime)?;
            }
            #[cfg(not(unix))]
            {
                set_file_times(&self.path, self.atime, self.mtime)?;
            }
        } else {
            set_file_times(&self.path, self.atime, self.mtime)?;
        }
        Ok(())
    }
}

impl Drop for AccessTime {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}
#[cfg(feature = "acl")]
pub use posix_acl::{ACLEntry, PosixACL, Qualifier, ACL_EXECUTE, ACL_READ, ACL_RWX, ACL_WRITE};
