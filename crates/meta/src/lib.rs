#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::*;

// Re-export device number helpers so consumers can construct and
// deconstruct `dev_t` values without depending on `nix` directly.
#[cfg(target_os = "linux")]
pub use nix::sys::stat::{major, makedev, minor};

#[cfg(target_os = "macos")]
pub use libc::{major, makedev, minor};

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod stub;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub use stub::*;

mod parse;
pub use parse::{parse_chmod, parse_chmod_spec, parse_chown, parse_id_map};

/// Normalize a `mode_t`-style bitfield, stripping any non-permission bits.
#[inline]
pub const fn normalize_mode(mode: u32) -> u32 {
    mode & 0o7777
}

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
use std::collections::{HashMap, HashSet};
#[cfg(all(unix, feature = "xattr"))]
use std::ffi::OsString;
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

/// Table mapping group IDs to compact indexes.
///
/// This helps build the `gid` table used by the file list so that
/// repeated group IDs are transmitted only once. Calling [`push`]
/// returns the index for the provided `gid`, inserting it into the
/// table if it wasn't already present.
#[derive(Debug, Default, Clone)]
pub struct GidTable {
    map: HashMap<u32, usize>,
    table: Vec<u32>,
}

impl GidTable {
    /// Create a new, empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `gid` into the table if it is not present, returning the
    /// index associated with it.
    pub fn push(&mut self, gid: u32) -> usize {
        *self.map.entry(gid).or_insert_with(|| {
            self.table.push(gid);
            self.table.len() - 1
        })
    }

    /// Returns the group ID stored at `idx`, if any.
    pub fn gid(&self, idx: usize) -> Option<u32> {
        self.table.get(idx).copied()
    }

    /// Exposes the underlying slice of group IDs in insertion order.
    pub fn as_slice(&self) -> &[u32] {
        &self.table
    }
}

/// Apply the provided extended attributes to `path`, removing any
/// existing attributes that are not present in `xattrs`.
#[cfg(all(unix, feature = "xattr"))]
pub fn apply_xattrs(path: &Path, xattrs: &[(OsString, Vec<u8>)]) -> io::Result<()> {
    let mut existing: HashSet<OsString> = xattr::list(path)?.collect();
    for (name, value) in xattrs {
        existing.remove(name);
        xattr::set(path, name, value)?;
    }
    for name in existing {
        if let Some(s) = name.to_str() {
            if s == "system.posix_acl_access" || s == "system.posix_acl_default" {
                continue;
            }
        }
        let _ = xattr::remove(path, &name);
    }
    Ok(())
}

/// Table mapping user IDs to compact indexes.
///
/// This helps build the `uid` table used by the file list so that
/// repeated user IDs are transmitted only once. Calling [`push`]
/// returns the index for the provided `uid`, inserting it into the
/// table if it wasn't already present.
#[derive(Debug, Default, Clone)]
pub struct UidTable {
    map: HashMap<u32, usize>,
    table: Vec<u32>,
}

impl UidTable {
    /// Create a new, empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert `uid` into the table if it is not present, returning the
    /// index associated with it.
    pub fn push(&mut self, uid: u32) -> usize {
        *self.map.entry(uid).or_insert_with(|| {
            self.table.push(uid);
            self.table.len() - 1
        })
    }

    /// Returns the user ID stored at `idx`, if any.
    pub fn uid(&self, idx: usize) -> Option<u32> {
        self.table.get(idx).copied()
    }

    /// Exposes the underlying slice of user IDs in insertion order.
    pub fn as_slice(&self) -> &[u32] {
        &self.table
    }
}

#[cfg(feature = "acl")]
pub use posix_acl::{ACLEntry, PosixACL, Qualifier, ACL_EXECUTE, ACL_READ, ACL_RWX, ACL_WRITE};
