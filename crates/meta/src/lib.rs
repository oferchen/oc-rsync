use std::fs;
use std::io;
use std::path::Path;

use filetime::{self, FileTime};
use nix::sys::stat::{self, FchmodatFlags, Mode};
use nix::unistd::{self, Gid, Uid};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use std::os::unix::ffi::OsStrExt;

#[cfg(feature = "xattr")]
use std::ffi::OsString;

#[cfg(all(test, feature = "xattr"))]
mod xattr {
    pub use real_xattr::{get, set};
    use std::ffi::OsString;
    use std::path::Path;
    use xattr as real_xattr;

    pub fn list(path: &Path) -> std::io::Result<Vec<OsString>> {
        let attrs: Vec<OsString> = real_xattr::list(path)?.collect();
        if attrs.iter().any(|a| a == "user.disappearing") {
            let _ = real_xattr::remove(path, "user.disappearing");
        }
        Ok(attrs)
    }
}

/// Target for a mode adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodTarget {
    /// Apply to files and directories.
    All,
    /// Apply only to regular files.
    File,
    /// Apply only to directories.
    Dir,
}

/// Operation performed by a mode adjustment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodOp {
    /// Add the specified bits.
    Add,
    /// Remove the specified bits.
    Remove,
    /// Set the specified bits, clearing others within the mask.
    Set,
}

/// A single parsed `--chmod` rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chmod {
    /// Target of the rule.
    pub target: ChmodTarget,
    /// Operation to perform.
    pub op: ChmodOp,
    /// Mask of bits affected by this rule.
    pub mask: u32,
    /// Bits to set or clear depending on the operation.
    pub bits: u32,
    /// Whether execute bits are conditional (`X`).
    pub conditional: bool,
}

/// Options controlling which metadata to capture and apply.
#[derive(Clone, Default)]
pub struct Options {
    /// Include extended attributes.
    pub xattrs: bool,
    /// Include POSIX ACL entries.
    pub acl: bool,
    /// Adjust permissions based on parsed `--chmod` rules.
    pub chmod: Option<Vec<Chmod>>,
    /// Preserve file owner (`--owner`).
    pub owner: bool,
    /// Preserve file group (`--group`).
    pub group: bool,
    /// Preserve permission bits (`--perms`).
    pub perms: bool,
    /// Preserve modification times (`--times`).
    pub times: bool,
    /// Preserve access times (`--atimes`).
    pub atimes: bool,
    /// Preserve creation times (`--crtimes`).
    pub crtimes: bool,
    /// Map remote UIDs to local ones when applying metadata.
    pub uid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>>,
    /// Map remote GIDs to local ones when applying metadata.
    pub gid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>>,
}

impl std::fmt::Debug for Options {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Options")
            .field("xattrs", &self.xattrs)
            .field("acl", &self.acl)
            .field("chmod", &self.chmod)
            .field("owner", &self.owner)
            .field("group", &self.group)
            .field("perms", &self.perms)
            .field("times", &self.times)
            .field("atimes", &self.atimes)
            .field("crtimes", &self.crtimes)
            .field("uid_map", &self.uid_map.is_some())
            .field("gid_map", &self.gid_map.is_some())
            .finish()
    }
}

impl Options {
    /// Return `true` if any metadata should be processed.
    pub fn needs_metadata(&self) -> bool {
        self.xattrs
            || self.acl
            || self.chmod.is_some()
            || self.owner
            || self.group
            || self.perms
            || self.times
            || self.atimes
            || self.crtimes
    }
}

/// Serialized file metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    /// File owner user ID.
    pub uid: u32,
    /// File owner group ID.
    pub gid: u32,
    /// Permission bits (`0o7777`).
    pub mode: u32,
    /// Modification time with nanosecond precision.
    pub mtime: FileTime,
    /// Access time with nanosecond precision.
    pub atime: Option<FileTime>,
    /// Creation time with nanosecond precision when available.
    pub crtime: Option<FileTime>,
    #[cfg(feature = "xattr")]
    /// Extended attributes.
    pub xattrs: Vec<(OsString, Vec<u8>)>,
    #[cfg(feature = "acl")]
    /// POSIX ACL entries.
    pub acl: Vec<posix_acl::ACLEntry>,
}

impl Metadata {
    /// Read metadata from `path` using `opts`.
    pub fn from_path(path: &Path, opts: Options) -> io::Result<Self> {
        let st = stat::stat(path).map_err(nix_to_io)?;
        let uid = st.st_uid;
        let gid = st.st_gid;
        let mode = (st.st_mode as u32) & 0o7777;
        let mtime = FileTime::from_unix_time(st.st_mtime, st.st_mtime_nsec as u32);

        let std_meta = fs::metadata(path)?;
        let atime = if opts.atimes {
            Some(FileTime::from_last_access_time(&std_meta))
        } else {
            None
        };
        let crtime = if opts.crtimes {
            get_file_crtime(path)?
        } else {
            None
        };

        #[cfg(feature = "xattr")]
        let xattrs = if opts.xattrs {
            let mut attrs = Vec::new();
            for attr in xattr::list(path)? {
                if let Some(value) = xattr::get(path, &attr)? {
                    attrs.push((attr, value));
                }
            }
            attrs
        } else {
            Vec::new()
        };

        #[cfg(feature = "acl")]
        let acl = if opts.acl {
            let acl = posix_acl::PosixACL::read_acl(path).map_err(acl_to_io)?;
            acl.entries()
        } else {
            Vec::new()
        };

        Ok(Metadata {
            uid,
            gid,
            mode,
            mtime,
            atime,
            crtime,
            #[cfg(feature = "xattr")]
            xattrs,
            #[cfg(feature = "acl")]
            acl,
        })
    }

    /// Apply metadata to `path` using `opts`.
    pub fn apply(&self, path: &Path, opts: Options) -> io::Result<()> {
        if opts.owner || opts.group {
            let uid = if let Some(ref map) = opts.uid_map {
                map(self.uid)
            } else {
                self.uid
            };
            let gid = if let Some(ref map) = opts.gid_map {
                map(self.gid)
            } else {
                self.gid
            };
            unistd::chown(
                path,
                if opts.owner {
                    Some(Uid::from_raw(uid))
                } else {
                    None
                },
                if opts.group {
                    Some(Gid::from_raw(gid))
                } else {
                    None
                },
            )
            .map_err(nix_to_io)?;
        }

        if opts.perms || opts.chmod.is_some() {
            let meta = fs::metadata(path)?;
            let is_dir = meta.is_dir();
            let mut mode_val = if opts.perms {
                self.mode
            } else {
                meta.permissions().mode() & 0o7777
            };
            let orig_mode = mode_val;
            if let Some(ref rules) = opts.chmod {
                for rule in rules {
                    match rule.target {
                        ChmodTarget::Dir if !is_dir => continue,
                        ChmodTarget::File if is_dir => continue,
                        _ => {}
                    }
                    let mut bits = rule.bits;
                    if rule.conditional && !(is_dir || (orig_mode & 0o111) != 0) {
                        bits &= !0o111;
                    }
                    match rule.op {
                        ChmodOp::Add => mode_val |= bits,
                        ChmodOp::Remove => mode_val &= !bits,
                        ChmodOp::Set => {
                            mode_val = (mode_val & !rule.mask) | (bits & rule.mask);
                        }
                    }
                }
            }
            let mode_t: libc::mode_t = mode_val as libc::mode_t;
            let mode = Mode::from_bits_truncate(mode_t);
            stat::fchmodat(None, path, mode, FchmodatFlags::NoFollowSymlink).map_err(nix_to_io)?;
        }

        if opts.atimes {
            if let Some(atime) = self.atime {
                filetime::set_file_times(path, atime, self.mtime)?;
            } else {
                filetime::set_file_mtime(path, self.mtime)?;
            }
        } else if opts.times {
            filetime::set_file_mtime(path, self.mtime)?;
        }

        if opts.crtimes {
            if let Some(crtime) = self.crtime {
                let _ = set_file_crtime(path, crtime);
            }
        }

        #[cfg(feature = "xattr")]
        if opts.xattrs {
            for (name, value) in &self.xattrs {
                xattr::set(path, name, value)?;
            }
        }

        #[cfg(feature = "acl")]
        if opts.acl {
            let mut acl = posix_acl::PosixACL::empty();
            for entry in &self.acl {
                acl.set(entry.qual, entry.perm);
            }
            acl.write_acl(path).map_err(acl_to_io)?;
        }

        Ok(())
    }
}

fn nix_to_io(err: nix::errno::Errno) -> io::Error {
    io::Error::from_raw_os_error(err as i32)
}

#[cfg(feature = "acl")]
fn acl_to_io(err: posix_acl::ACLError) -> io::Error {
    if let Some(e) = err.as_io_error() {
        if let Some(code) = e.raw_os_error() {
            io::Error::from_raw_os_error(code)
        } else {
            io::Error::new(e.kind(), e.to_string())
        }
    } else {
        io::Error::other(err)
    }
}

#[cfg(target_os = "linux")]
fn get_file_crtime(path: &Path) -> io::Result<Option<FileTime>> {
    use libc::{statx, AT_FDCWD, AT_STATX_SYNC_AS_STAT, STATX_BTIME};
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())?;
    let mut stx = MaybeUninit::<libc::statx>::zeroed();
    let ret = unsafe {
        statx(
            AT_FDCWD,
            c_path.as_ptr(),
            AT_STATX_SYNC_AS_STAT,
            STATX_BTIME,
            stx.as_mut_ptr(),
        )
    };
    if ret != 0 {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EINVAL) || err.raw_os_error() == Some(libc::ENOSYS) {
            return Ok(None);
        } else {
            return Err(err);
        }
    }
    let stx = unsafe { stx.assume_init() };
    if (stx.stx_mask & STATX_BTIME) == 0 {
        Ok(None)
    } else {
        let ts = stx.stx_btime;
        Ok(Some(FileTime::from_unix_time(
            ts.tv_sec as i64,
            ts.tv_nsec as u32,
        )))
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn get_file_crtime(path: &Path) -> io::Result<Option<FileTime>> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())?;
    let mut st = MaybeUninit::<libc::stat>::zeroed();
    if unsafe { libc::stat(c_path.as_ptr(), st.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let st = unsafe { st.assume_init() };
    let ts = st.st_birthtimespec;
    Ok(Some(FileTime::from_unix_time(ts.tv_sec, ts.tv_nsec as u32)))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "ios")))]
fn get_file_crtime(_path: &Path) -> io::Result<Option<FileTime>> {
    Ok(None)
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn set_file_crtime(path: &Path, crtime: FileTime) -> io::Result<()> {
    use libc::{attrlist, setattrlist, timespec, ATTR_BIT_MAP_COUNT, ATTR_CMN_CRTIME};
    use std::ffi::CString;
    use std::mem;

    let mut attr = attrlist {
        bitmapcount: ATTR_BIT_MAP_COUNT as u16,
        reserved: 0,
        commonattr: ATTR_CMN_CRTIME as u32,
        volattr: 0,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };

    let mut ts = timespec {
        tv_sec: crtime.unix_seconds(),
        tv_nsec: crtime.nanoseconds() as _,
    };

    let path = CString::new(path.as_os_str().as_bytes())?;
    let ret = unsafe {
        setattrlist(
            path.as_ptr(),
            &mut attr as *mut _ as *mut libc::c_void,
            &mut ts as *mut _ as *mut libc::c_void,
            mem::size_of::<timespec>() as libc::size_t,
            0,
        )
    };
    if ret == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "ios")))]
fn set_file_crtime(_path: &Path, _crtime: FileTime) -> io::Result<()> {
    Ok(())
}

#[cfg(all(test, feature = "xattr"))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_xattr_between_list_and_get() -> io::Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("file");
        fs::write(&path, b"hello")?;
        xattr::set(&path, "user.disappearing", b"value")?;

        let meta = Metadata::from_path(
            &path,
            Options {
                xattrs: true,
                acl: false,
                ..Default::default()
            },
        )?;
        assert!(meta
            .xattrs
            .iter()
            .all(|(name, _)| name != "user.disappearing"));
        Ok(())
    }
}
