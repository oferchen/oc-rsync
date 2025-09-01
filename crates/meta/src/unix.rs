// crates/meta/src/unix.rs
use std::fs;
use std::io;
use std::path::Path;

use crate::normalize_mode;
use filetime::{self, FileTime};
use nix::errno::Errno;
use nix::sys::stat::{self, FchmodatFlags, Mode, SFlag};
use nix::unistd::{self, FchownatFlags, Gid, Uid};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::sync::Arc;
use users::{get_group_by_gid, get_group_by_name, get_user_by_name, get_user_by_uid};

#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;

#[cfg(feature = "xattr")]
use std::ffi::OsString;

#[cfg(all(test, feature = "xattr"))]
mod xattr {
    pub use real_xattr::{get, remove, set};
    use std::ffi::OsString;
    use std::path::Path;
    use xattr as real_xattr;

    pub fn list(path: &Path) -> std::io::Result<Vec<OsString>> {
        let attrs: Vec<OsString> = real_xattr::list(path)?.collect();
        if attrs.iter().any(|a| a == "user.disappearing") {
            let _ = remove(path, "user.disappearing");
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
    pub fake_super: bool,
    pub super_user: bool,
    pub numeric_ids: bool,
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
            .field("fake_super", &self.fake_super)
            .field("super_user", &self.super_user)
            .field("numeric_ids", &self.numeric_ids)
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
    #[cfg(feature = "acl")]
    pub default_acl: Vec<posix_acl::ACLEntry>,
}

impl Metadata {
    pub fn from_path(path: &Path, opts: Options) -> io::Result<Self> {
        let meta = fs::symlink_metadata(path)?;
        let uid = meta.uid();
        let gid = meta.gid();
        let raw_mode = meta.mode();
        let mode = normalize_mode(raw_mode);
        let mtime = FileTime::from_last_modification_time(&meta);

        let atime = if opts.atimes {
            Some(FileTime::from_last_access_time(&meta))
        } else {
            None
        };
        let crtime = if opts.crtimes {
            get_file_crtime(path)?
        } else {
            None
        };

        #[cfg(feature = "xattr")]
        let (uid, gid, mode) = if opts.fake_super {
            let mut uid = uid;
            let mut gid = gid;
            let mut mode = mode;
            if let Ok(Some(val)) = xattr::get(path, "user.rsync.uid") {
                if let Ok(s) = std::str::from_utf8(&val) {
                    if let Ok(v) = s.parse::<u32>() {
                        uid = v;
                    }
                }
            }
            if let Ok(Some(val)) = xattr::get(path, "user.rsync.gid") {
                if let Ok(s) = std::str::from_utf8(&val) {
                    if let Ok(v) = s.parse::<u32>() {
                        gid = v;
                    }
                }
            }
            if let Ok(Some(val)) = xattr::get(path, "user.rsync.mode") {
                if let Ok(s) = std::str::from_utf8(&val) {
                    if let Ok(v) = s.parse::<u32>() {
                        mode = normalize_mode(v);
                    }
                }
            }
            (uid, gid, mode)
        } else {
            (uid, gid, mode)
        };

        #[cfg(feature = "xattr")]
        let xattrs = if opts.xattrs || opts.fake_super {
            let mut attrs = Vec::new();
            for attr in xattr::list(path)? {
                if let Some(name) = attr.to_str() {
                    if !opts.fake_super && name.starts_with("user.rsync.") {
                        continue;
                    }
                    if name.starts_with("security.")
                        || name == "system.posix_acl_access"
                        || name == "system.posix_acl_default"
                    {
                        continue;
                    }
                }
                if let Some(value) = xattr::get(path, &attr)? {
                    attrs.push((attr, value));
                }
            }
            attrs
        } else {
            Vec::new()
        };

        #[cfg(feature = "acl")]
        let is_dir = meta.file_type().is_dir();

        #[cfg(feature = "acl")]
        let (acl, default_acl) = if opts.acl {
            let acl_entries = match posix_acl::PosixACL::read_acl(path) {
                Ok(acl) => acl.entries(),
                Err(err) => {
                    if let Some(code) = err.as_io_error().and_then(|e| e.raw_os_error()) {
                        if matches!(code, libc::ENODATA | libc::ENOTSUP | libc::ENOSYS) {
                            Vec::new()
                        } else {
                            return Err(acl_to_io(err));
                        }
                    } else {
                        return Err(acl_to_io(err));
                    }
                }
            };
            let default_acl = if is_dir {
                match posix_acl::PosixACL::read_default_acl(path) {
                    Ok(dacl) => dacl.entries(),
                    Err(err) => {
                        if let Some(code) = err.as_io_error().and_then(|e| e.raw_os_error()) {
                            if matches!(code, libc::ENODATA | libc::ENOTSUP | libc::ENOSYS) {
                                Vec::new()
                            } else {
                                return Err(acl_to_io(err));
                            }
                        } else {
                            return Err(acl_to_io(err));
                        }
                    }
                }
            } else {
                Vec::new()
            };
            (acl_entries, default_acl)
        } else {
            (Vec::new(), Vec::new())
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
            #[cfg(feature = "acl")]
            default_acl,
        })
    }

    pub fn apply(&self, path: &Path, opts: Options) -> io::Result<()> {
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
                match unistd::fchownat(
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
                ) {
                    Err(Errno::EOPNOTSUPP) => unistd::chown(
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
                    ),
                    other => other,
                }
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
                    Errno::EPERM
                    | Errno::EACCES
                    | Errno::ENOSYS
                    | Errno::EINVAL
                    | Errno::EOPNOTSUPP
                        if !opts.super_user && !opts.numeric_ids => {}
                    _ => return Err(nix_to_io(err)),
                }
            }
        }

        if (opts.perms || opts.chmod.is_some() || opts.executability) && !is_symlink {
            let mut mode_val = if opts.perms {
                normalize_mode(self.mode)
            } else {
                normalize_mode(meta.permissions().mode())
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
            let mode_val = normalize_mode(mode_val);
            debug_assert_eq!(mode_val & !0o7777, 0);
            let mode_t: libc::mode_t = mode_val as libc::mode_t;
            let mode = Mode::from_bits_truncate(mode_t);
            if let Err(err) = stat::fchmodat(None, path, mode, FchmodatFlags::NoFollowSymlink) {
                match err {
                    Errno::EINVAL | Errno::EOPNOTSUPP => {
                        let perm = fs::Permissions::from_mode(mode_val);
                        if let Err(e) = fs::set_permissions(path, perm) {
                            if let Some(code) = e.raw_os_error() {
                                match Errno::from_i32(code) {
                                    Errno::EPERM
                                    | Errno::EACCES
                                    | Errno::ENOSYS
                                    | Errno::EINVAL
                                    | Errno::EOPNOTSUPP
                                        if !opts.super_user => {}
                                    _ => return Err(e),
                                }
                            } else {
                                return Err(e);
                            }
                        }
                    }
                    Errno::EPERM | Errno::EACCES | Errno::ENOSYS if !opts.super_user => {}
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
            } else if opts.atimes {
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

        if opts.crtimes {
            if let Some(crtime) = self.crtime {
                let _ = set_file_crtime(path, crtime);
            }
        }

        #[cfg(feature = "xattr")]
        if opts.xattrs {
            crate::apply_xattrs(path, &self.xattrs)?;
        }

        #[cfg(feature = "acl")]
        if opts.acl {
            {
                let mut acl = posix_acl::PosixACL::empty();
                for entry in &self.acl {
                    acl.set(entry.qual, entry.perm);
                }
                if let Err(err) = acl.write_acl(path) {
                    if !should_ignore_acl_error(&err) {
                        return Err(acl_to_io(err));
                    }
                }
            }
            if is_dir {
                if self.default_acl.is_empty() {
                    if let Err(err) = remove_default_acl(path) {
                        match err.raw_os_error() {
                            Some(libc::EPERM) | Some(libc::EACCES) | Some(libc::ENOSYS)
                            | Some(libc::EINVAL) | Some(libc::ENOTSUP) => {}
                            _ => return Err(err),
                        }
                    }
                } else {
                    let mut dacl = posix_acl::PosixACL::empty();
                    for entry in &self.default_acl {
                        dacl.set(entry.qual, entry.perm);
                    }
                    if let Err(err) = dacl.write_default_acl(path) {
                        if !should_ignore_acl_error(&err) {
                            return Err(acl_to_io(err));
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "xattr")]
pub fn store_fake_super(path: &Path, uid: u32, gid: u32, mode: u32) {
    let _ = xattr::set(path, "user.rsync.uid", uid.to_string().as_bytes());
    let _ = xattr::set(path, "user.rsync.gid", gid.to_string().as_bytes());
    let _ = xattr::set(path, "user.rsync.mode", mode.to_string().as_bytes());
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn mknod(path: &Path, kind: SFlag, perm: Mode, dev: u64) -> io::Result<()> {
    use nix::libc::dev_t;
    let dev: dev_t = dev as dev_t;
    stat::mknod(path, kind, perm, dev).map_err(nix_to_io)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn mkfifo(path: &Path, perm: Mode) -> io::Result<()> {
    use nix::unistd::mkfifo;
    mkfifo(path, perm).map_err(nix_to_io)
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

#[cfg(feature = "acl")]
fn should_ignore_acl_error(err: &posix_acl::ACLError) -> bool {
    if let Some(code) = err.as_io_error().and_then(|e| e.raw_os_error()) {
        matches!(
            code,
            libc::EPERM | libc::EACCES | libc::ENOSYS | libc::EINVAL | libc::ENOTSUP
        )
    } else {
        false
    }
}

#[cfg(feature = "acl")]
fn remove_default_acl(path: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    extern "C" {
        fn acl_delete_def_file(path_p: *const libc::c_char) -> libc::c_int;
    }

    let c_path = CString::new(path.as_os_str().as_bytes())?;
    let ret = unsafe { acl_delete_def_file(c_path.as_ptr()) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
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
        Ok(Some(FileTime::from_unix_time(ts.tv_sec, ts.tv_nsec)))
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

pub fn uid_from_name(name: &str) -> Option<u32> {
    get_user_by_name(name).map(|u| u.uid()).or_else(|| {
        fs::read_to_string("/etc/passwd").ok().and_then(|data| {
            data.lines().find_map(|line| {
                if line.starts_with('#') {
                    return None;
                }
                let mut parts = line.split(':');
                let user_name = parts.next()?;
                if user_name != name {
                    return None;
                }
                parts.next();
                let uid_str = parts.next()?;
                uid_str.parse().ok()
            })
        })
    })
}

pub fn gid_from_name(name: &str) -> Option<u32> {
    get_group_by_name(name).map(|g| g.gid()).or_else(|| {
        fs::read_to_string("/etc/group").ok().and_then(|data| {
            data.lines().find_map(|line| {
                if line.starts_with('#') {
                    return None;
                }
                let mut parts = line.split(':');
                let group_name = parts.next()?;
                if group_name != name {
                    return None;
                }
                parts.next();
                let gid_str = parts.next()?;
                gid_str.parse().ok()
            })
        })
    })
}

pub fn uid_from_name_or_id(spec: &str) -> Option<u32> {
    spec.parse().ok().or_else(|| uid_from_name(spec))
}

pub fn gid_from_name_or_id(spec: &str) -> Option<u32> {
    spec.parse().ok().or_else(|| gid_from_name(spec))
}

pub fn uid_to_name(uid: u32) -> Option<String> {
    get_user_by_uid(uid)
        .map(|u| u.name().to_string_lossy().into_owned())
        .or_else(|| {
            fs::read_to_string("/etc/passwd").ok().and_then(|data| {
                data.lines().find_map(|line| {
                    if line.starts_with('#') {
                        return None;
                    }
                    let mut parts = line.split(':');
                    let name = parts.next()?;
                    parts.next();
                    let uid_str = parts.next()?;
                    match uid_str.parse::<u32>() {
                        Ok(u) if u == uid => Some(name.to_string()),
                        _ => None,
                    }
                })
            })
        })
}

pub fn gid_to_name(gid: u32) -> Option<String> {
    get_group_by_gid(gid)
        .map(|g| g.name().to_string_lossy().into_owned())
        .or_else(|| {
            fs::read_to_string("/etc/group").ok().and_then(|data| {
                data.lines().find_map(|line| {
                    if line.starts_with('#') {
                        return None;
                    }
                    let mut parts = line.split(':');
                    let name = parts.next()?;
                    parts.next();
                    let gid_str = parts.next()?;
                    match gid_str.parse::<u32>() {
                        Ok(g) if g == gid => Some(name.to_string()),
                        _ => None,
                    }
                })
            })
        })
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
