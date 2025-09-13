// crates/meta/src/unix/mod.rs

mod acl;
mod apply;
mod xattr;

use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::{Metadata, Options, normalize_mode};
use filetime::{self, FileTime};
use nix::sys::stat::{self, Mode, SFlag};
use users::{get_group_by_gid, get_group_by_name, get_user_by_name, get_user_by_uid};

use self::acl::read_acl_inner;

#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;

pub use acl::{acls_supported, decode_acl, encode_acl, read_acl, write_acl};
pub use xattr::{copy_xattrs, store_fake_super, xattrs_supported};

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

        let (uid, gid, mode) = if opts.fake_super {
            let mut uid = uid;
            let mut gid = gid;
            let mut mode = mode;
            if let Ok(Some(val)) = xattr::get(path, "user.rsync.uid")
                && let Ok(s) = std::str::from_utf8(&val)
                && let Ok(v) = s.parse::<u32>()
            {
                uid = v;
            }
            if let Ok(Some(val)) = xattr::get(path, "user.rsync.gid")
                && let Ok(s) = std::str::from_utf8(&val)
                && let Ok(v) = s.parse::<u32>()
            {
                gid = v;
            }
            if let Ok(Some(val)) = xattr::get(path, "user.rsync.mode")
                && let Ok(s) = std::str::from_utf8(&val)
                && let Ok(v) = s.parse::<u32>()
            {
                mode = normalize_mode(v);
            }
            (uid, gid, mode)
        } else {
            (uid, gid, mode)
        };

        #[cfg(feature = "xattr")]
        let xattrs = if opts.xattrs || opts.fake_super {
            let mut attrs = Vec::new();
            match xattr::list(path) {
                Ok(list) => {
                    for attr in list {
                        if let Some(name) = attr.to_str()
                            && ((!opts.fake_super && name.starts_with("user.rsync."))
                                || name.starts_with("security.")
                                || name == "system.posix_acl_access"
                                || name == "system.posix_acl_default")
                        {
                            continue;
                        }
                        if let Some(filter) = opts.xattr_filter.as_ref()
                            && !filter(attr.as_os_str())
                        {
                            continue;
                        }
                        match xattr::get(path, &attr) {
                            Ok(Some(value)) => attrs.push((attr, value)),
                            Ok(None) => {}
                            Err(err) => {
                                if !crate::should_ignore_xattr_error(&err) {
                                    return Err(err);
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    if !crate::should_ignore_xattr_error(&err) {
                        return Err(err);
                    }
                }
            }
            attrs
        } else {
            Vec::new()
        };

        let is_dir = meta.file_type().is_dir();

        let (acl, default_acl) = if opts.acl {
            read_acl_inner(path, is_dir, opts.fake_super, mode)?
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
            acl,
            default_acl,
        })
    }
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

pub(super) fn nix_to_io(err: nix::errno::Errno) -> io::Error {
    io::Error::from_raw_os_error(err as i32)
}
#[cfg(target_os = "linux")]
pub(super) fn get_file_crtime(path: &Path) -> io::Result<Option<FileTime>> {
    use libc::{AT_FDCWD, AT_STATX_SYNC_AS_STAT, STATX_BTIME, statx};
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())?;
    let mut stx = MaybeUninit::<libc::statx>::zeroed();
    // SAFETY: `c_path` is a valid C string and `stx` is properly allocated for `statx`.
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
    // SAFETY: `statx` initialized `stx` on success.
    let stx = unsafe { stx.assume_init() };
    if (stx.stx_mask & STATX_BTIME) == 0 {
        Ok(None)
    } else {
        let ts = stx.stx_btime;
        Ok(Some(FileTime::from_unix_time(ts.tv_sec, ts.tv_nsec)))
    }
}

#[cfg(target_os = "macos")]
pub(super) fn get_file_crtime(path: &Path) -> io::Result<Option<FileTime>> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes())?;
    let mut st = MaybeUninit::<libc::stat>::zeroed();
    // SAFETY: `c_path` is a valid C string and `st` points to enough space for `stat`.
    if unsafe { libc::stat(c_path.as_ptr(), st.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `stat` populated `st` on success.
    let st = unsafe { st.assume_init() };
    let secs = st.st_birthtime;
    let nsecs = st.st_birthtime_nsec;
    Ok(Some(FileTime::from_unix_time(secs, nsecs as u32)))
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) fn get_file_crtime(_path: &Path) -> io::Result<Option<FileTime>> {
    Ok(None)
}

#[cfg(target_os = "macos")]
pub(super) fn set_file_crtime(path: &Path, crtime: FileTime) -> io::Result<()> {
    use libc::{ATTR_BIT_MAP_COUNT, ATTR_CMN_CRTIME, attrlist, setattrlist, timespec};
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
    // SAFETY: `path` is a valid C string and the pointers to `attr` and `ts` are properly initialized.
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
pub(super) fn set_file_crtime(_path: &Path, _crtime: FileTime) -> io::Result<()> {
    Ok(())
}

pub fn uid_from_name(name: &str) -> Option<u32> {
    get_user_by_name(name).map(|u| u.uid())
}

pub fn gid_from_name(name: &str) -> Option<u32> {
    get_group_by_name(name).map(|g| g.gid())
}

pub fn uid_from_name_or_id(spec: &str) -> Option<u32> {
    let s = spec.trim();
    s.parse().ok().or_else(|| uid_from_name(s))
}

pub fn gid_from_name_or_id(spec: &str) -> Option<u32> {
    let s = spec.trim();
    s.parse().ok().or_else(|| gid_from_name(s))
}

pub fn uid_to_name(uid: u32) -> Option<String> {
    get_user_by_uid(uid).map(|u| u.name().to_string_lossy().into_owned())
}

pub fn gid_to_name(gid: u32) -> Option<String> {
    get_group_by_gid(gid).map(|g| g.name().to_string_lossy().into_owned())
}
