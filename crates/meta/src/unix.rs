// crates/meta/src/unix.rs
use std::fs;
use std::io;
use std::path::Path;

use filetime::{self, FileTime};
use nix::errno::Errno;
use nix::sys::stat::{self, FchmodatFlags, Mode};
use nix::unistd::{self, FchownatFlags, Gid, Uid};
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

#[cfg(target_os = "macos")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodTarget {
    All,
    File,
    Dir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodOp {
    Add,
    Remove,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chmod {
    pub target: ChmodTarget,
    pub op: ChmodOp,
    pub mask: u32,
    pub bits: u32,
    pub conditional: bool,
}

#[derive(Clone, Default)]
pub struct Options {
    pub xattrs: bool,
    pub acl: bool,
    pub chmod: Option<Vec<Chmod>>,
    pub owner: bool,
    pub group: bool,
    pub perms: bool,
    pub executability: bool,
    pub times: bool,
    pub atimes: bool,
    pub crtimes: bool,
    pub omit_dir_times: bool,
    pub omit_link_times: bool,
    pub uid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>>,
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
            .field("executability", &self.executability)
            .field("times", &self.times)
            .field("atimes", &self.atimes)
            .field("crtimes", &self.crtimes)
            .field("omit_dir_times", &self.omit_dir_times)
            .field("omit_link_times", &self.omit_link_times)
            .field("uid_map", &self.uid_map.is_some())
            .field("gid_map", &self.gid_map.is_some())
            .finish()
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
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub mtime: FileTime,
    pub atime: Option<FileTime>,
    pub crtime: Option<FileTime>,
    #[cfg(feature = "xattr")]
    pub xattrs: Vec<(OsString, Vec<u8>)>,
    #[cfg(feature = "acl")]
    pub acl: Vec<posix_acl::ACLEntry>,
}

impl Metadata {
    pub fn from_path(path: &Path, opts: Options) -> io::Result<Self> {
        // Use lstat so we capture metadata for symlinks themselves rather
        // than the file they point to.
        let st = stat::lstat(path).map_err(nix_to_io)?;
        let uid = st.st_uid;
        let gid = st.st_gid;
        let mode = (st.st_mode as u32) & 0o7777;
        let mtime = FileTime::from_unix_time(st.st_mtime, st.st_mtime_nsec as u32);

        // Mirror the lstat call above by avoiding symlink traversal when
        // collecting standard metadata.
        let std_meta = fs::symlink_metadata(path)?;
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

    pub fn apply(&self, path: &Path, opts: Options) -> io::Result<()> {
        // Obtain metadata without following symlinks so we know the type of
        // `path` itself.
        let meta = fs::symlink_metadata(path)?;
        let ft = meta.file_type();
        let is_symlink = ft.is_symlink();
        let is_dir = ft.is_dir();

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
            let res = if is_symlink {
                unistd::fchownat(
                    None,
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
                    FchownatFlags::NoFollowSymlink,
                )
            } else {
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
            };
            if let Err(err) = res {
                match err {
                    Errno::EPERM | Errno::EACCES | Errno::ENOSYS | Errno::EINVAL => {}
                    _ => return Err(nix_to_io(err)),
                }
            }
        }

        if (opts.perms || opts.chmod.is_some() || opts.executability) && !is_symlink {
            let mut mode_val = if opts.perms {
                self.mode
            } else {
                meta.permissions().mode() & 0o7777
            };
            if opts.executability && !opts.perms {
                mode_val = (mode_val & !0o111) | (self.mode & 0o111);
            }
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
            if let Err(err) = stat::fchmodat(None, path, mode, FchmodatFlags::NoFollowSymlink) {
                match err {
                    Errno::EPERM | Errno::EACCES | Errno::ENOSYS | Errno::EINVAL => {}
                    _ => return Err(nix_to_io(err)),
                }
            }
        }

        if opts.atimes || opts.times {
            let skip_mtime =
                (is_dir && opts.omit_dir_times) || (is_symlink && opts.omit_link_times);
            if is_symlink {
                let cur_mtime = FileTime::from_last_modification_time(&meta);
                let cur_atime = FileTime::from_last_access_time(&meta);
                if opts.atimes {
                    if let Some(atime) = self.atime {
                        let mtime = if skip_mtime { cur_mtime } else { self.mtime };
                        filetime::set_symlink_file_times(path, atime, mtime)?;
                    } else if opts.times && !skip_mtime {
                        filetime::set_symlink_file_times(path, cur_atime, self.mtime)?;
                    }
                } else if opts.times && !skip_mtime {
                    filetime::set_symlink_file_times(path, cur_atime, self.mtime)?;
                }
            } else {
                if opts.atimes {
                    if let Some(atime) = self.atime {
                        if skip_mtime {
                            filetime::set_file_atime(path, atime)?;
                        } else {
                            filetime::set_file_times(path, atime, self.mtime)?;
                        }
                    } else if opts.times && !skip_mtime {
                        filetime::set_file_mtime(path, self.mtime)?;
                    }
                } else if opts.times && !skip_mtime {
                    filetime::set_file_mtime(path, self.mtime)?;
                }
            }
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

#[cfg(target_os = "macos")]
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
    let secs = st.st_birthtime;
    let nsecs = st.st_birthtime_nsec;
    Ok(Some(FileTime::from_unix_time(secs, nsecs as u32)))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn get_file_crtime(_path: &Path) -> io::Result<Option<FileTime>> {
    Ok(None)
}

#[cfg(target_os = "macos")]
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

#[cfg(not(target_os = "macos"))]
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
