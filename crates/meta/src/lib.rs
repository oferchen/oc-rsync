use std::fs;
use std::io;
use std::path::Path;

use filetime::{self, FileTime};
use nix::sys::stat::{self, FchmodatFlags, Mode};
use nix::unistd::{self, Gid, Uid};

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
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Include extended attributes.
    pub xattrs: bool,
    /// Include POSIX ACL entries.
    pub acl: bool,
    /// Adjust permissions based on parsed `--chmod` rules.
    pub chmod: Option<Vec<Chmod>>,
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
    pub fn from_path(path: &Path, _opts: Options) -> io::Result<Self> {
        let st = stat::stat(path).map_err(nix_to_io)?;
        let uid = st.st_uid;
        let gid = st.st_gid;
        let mode = (st.st_mode as u32) & 0o7777;
        let mtime = FileTime::from_unix_time(st.st_mtime, st.st_mtime_nsec as u32);

        let std_meta = fs::metadata(path)?;
        let atime = FileTime::from_last_access_time(&std_meta);
        let crtime = FileTime::from_creation_time(&std_meta);

        #[cfg(feature = "xattr")]
        let xattrs = if _opts.xattrs {
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
        let acl = if _opts.acl {
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
            atime: Some(atime),
            crtime,
            #[cfg(feature = "xattr")]
            xattrs,
            #[cfg(feature = "acl")]
            acl,
        })
    }

    /// Apply metadata to `path` using `opts`.
    pub fn apply(&self, path: &Path, opts: Options) -> io::Result<()> {
        unistd::chown(
            path,
            Some(Uid::from_raw(self.uid)),
            Some(Gid::from_raw(self.gid)),
        )
        .map_err(nix_to_io)?;

        let mut mode_val = self.mode;
        if let Some(ref rules) = opts.chmod {
            let is_dir = fs::metadata(path)?.is_dir();
            let orig_mode = mode_val;
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

        if let Some(atime) = self.atime {
            filetime::set_file_times(path, atime, self.mtime)?;
        } else {
            filetime::set_file_mtime(path, self.mtime)?;
        }

        if let Some(crtime) = self.crtime {
            let _ = set_file_crtime(path, crtime);
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
            },
        )?;
        assert!(meta
            .xattrs
            .iter()
            .all(|(name, _)| name != "user.disappearing"));
        Ok(())
    }
}
