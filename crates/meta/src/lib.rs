// crates/meta/src/lib.rs
#![allow(clippy::collapsible_if)]
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "linux")]
pub use nix::sys::stat::{major, makedev, minor};

#[cfg(target_os = "macos")]
pub use libc::{major, makedev, minor};

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use nix::sys::stat::{Mode, SFlag};

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod stub;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub use stub::*;

mod parse;
pub use parse::{parse_chmod, parse_chmod_spec, parse_chown, parse_id_map, IdKind};

#[derive(Debug, Clone, Copy, Default)]
pub struct MetaOpts {
    pub xattrs: bool,
    pub acl: bool,
}

pub const META_OPTS: MetaOpts = MetaOpts {
    xattrs: cfg!(feature = "xattr"),
    acl: cfg!(feature = "acl"),
};

#[inline]
pub const fn normalize_mode(mode: u32) -> u32 {
    mode & 0o7777
}

#[inline]
pub fn mode_from_metadata(meta: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        normalize_mode(meta.permissions().mode())
    }
    #[cfg(windows)]
    {
        let mut mode = 0o666;
        if meta.permissions().readonly() {
            mode &= !0o222;
        }
        mode
    }
    #[cfg(not(any(unix, windows)))]
    {
        0
    }
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
            || self.fake_super
    }
}

#[cfg(unix)]
use filetime::set_symlink_file_times;
use filetime::{set_file_times, FileTime};
use std::collections::HashMap;
#[cfg(unix)]
use std::collections::HashSet;
#[cfg(unix)]
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct AccessTime {
    path: PathBuf,
    atime: FileTime,
    mtime: FileTime,
    is_symlink: bool,
}

impl AccessTime {
    pub fn new(path: &Path) -> io::Result<Self> {
        let meta = fs::symlink_metadata(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            atime: FileTime::from_last_access_time(&meta),
            mtime: FileTime::from_last_modification_time(&meta),
            is_symlink: meta.file_type().is_symlink(),
        })
    }

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

#[derive(Debug, Default, Clone)]
pub struct GidTable {
    map: HashMap<u32, usize>,
    table: Vec<u32>,
}

impl GidTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, gid: u32) -> usize {
        *self.map.entry(gid).or_insert_with(|| {
            self.table.push(gid);
            self.table.len() - 1
        })
    }

    pub fn gid(&self, idx: usize) -> Option<u32> {
        self.table.get(idx).copied()
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.table
    }
}

#[cfg(unix)]
use std::collections::hash_map::Entry;

#[cfg(unix)]
pub fn hard_link_id(dev: u64, ino: u64) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    dev.hash(&mut hasher);
    ino.hash(&mut hasher);
    hasher.finish()
}

#[cfg(unix)]
#[derive(Debug, Default)]
pub struct HardLinks {
    map: HashMap<u64, (PathBuf, Vec<PathBuf>)>,
}

#[cfg(unix)]
impl HardLinks {
    pub fn register(&mut self, id: u64, path: &Path) -> bool {
        match self.map.entry(id) {
            Entry::Occupied(mut e) => {
                let (ref first, ref mut others) = *e.get_mut();
                if path != first && !others.iter().any(|p| p == path) {
                    others.push(path.to_path_buf());
                }
                false
            }
            Entry::Vacant(v) => {
                v.insert((path.to_path_buf(), Vec::new()));
                true
            }
        }
    }

    pub fn finalize(&mut self) -> io::Result<()> {
        for (_, (first, mut others)) in std::mem::take(&mut self.map) {
            let src = if first.exists() {
                first
            } else if let Some(pos) = others.iter().position(|p| p.exists()) {
                others.remove(pos)
            } else {
                continue;
            };
            for dest in others {
                if dest.exists() {
                    fs::remove_file(&dest)?;
                }
                fs::hard_link(&src, &dest)?;
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
pub(crate) fn should_ignore_xattr_error(err: &io::Error) -> bool {
    matches!(
        err.raw_os_error(),
        Some(libc::EPERM)
            | Some(libc::EACCES)
            | Some(libc::ENOTSUP)
            | Some(libc::ENOSYS)
            | Some(libc::EINVAL)
            | Some(libc::ENODATA)
    )
}

#[cfg(unix)]
pub fn apply_xattrs(
    path: &Path,
    xattrs: &[(OsString, Vec<u8>)],
    include: Option<&dyn Fn(&OsStr) -> bool>,
    include_for_delete: Option<&dyn Fn(&OsStr) -> bool>,
) -> io::Result<()> {
    let mut existing: HashSet<OsString> = match xattr::list(path) {
        Ok(list) => list.collect(),
        Err(err) => {
            if should_ignore_xattr_error(&err) {
                return Ok(());
            }
            return Err(err);
        }
    };
    for (name, value) in xattrs {
        if let Some(filter) = include {
            if !filter(name.as_os_str()) {
                continue;
            }
        }
        existing.remove(name);
        if let Err(err) = xattr::set(path, name, value) {
            if !should_ignore_xattr_error(&err) {
                return Err(err);
            }
        }
    }
    for name in existing {
        if let Some(filter) = include_for_delete {
            if !filter(name.as_os_str()) {
                continue;
            }
        }
        if let Some(s) = name.to_str() {
            if s == "system.posix_acl_access"
                || s == "system.posix_acl_default"
                || s.starts_with("security.")
            {
                continue;
            }
        }
        if let Err(err) = xattr::remove(path, &name) {
            if !should_ignore_xattr_error(&err) {
                return Err(err);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Default, Clone)]
pub struct UidTable {
    map: HashMap<u32, usize>,
    table: Vec<u32>,
}

impl UidTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, uid: u32) -> usize {
        *self.map.entry(uid).or_insert_with(|| {
            self.table.push(uid);
            self.table.len() - 1
        })
    }

    pub fn uid(&self, idx: usize) -> Option<u32> {
        self.table.get(idx).copied()
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.table
    }
}

#[cfg(unix)]
pub use posix_acl::{ACLEntry, PosixACL, Qualifier, ACL_EXECUTE, ACL_READ, ACL_RWX, ACL_WRITE};
