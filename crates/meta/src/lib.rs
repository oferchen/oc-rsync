use std::io;
use std::path::Path;

use filetime::{self, FileTime};
use nix::sys::stat::{self, FchmodatFlags, Mode};
use nix::unistd::{self, Gid, Uid};

#[cfg(feature = "xattr")]
use std::ffi::OsString;

#[cfg(all(test, feature = "xattr"))]
mod xattr {
    use ::xattr as real_xattr;
    pub use real_xattr::{get, set};
    use std::ffi::OsString;
    use std::path::Path;

    pub fn list(path: &Path) -> std::io::Result<Vec<OsString>> {
        let attrs: Vec<OsString> = real_xattr::list(path)?.collect();
        if attrs.iter().any(|a| a == "user.disappearing") {
            let _ = real_xattr::remove(path, "user.disappearing");
        }
        Ok(attrs)
    }
}

/// Options controlling which metadata to capture and apply.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// Include extended attributes.
    pub xattrs: bool,
    /// Include POSIX ACL entries.
    pub acl: bool,
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
            #[cfg(feature = "xattr")]
            xattrs,
            #[cfg(feature = "acl")]
            acl,
        })
    }

    /// Apply metadata to `path` using `opts`.
    pub fn apply(&self, path: &Path, _opts: Options) -> io::Result<()> {
        unistd::chown(
            path,
            Some(Uid::from_raw(self.uid)),
            Some(Gid::from_raw(self.gid)),
        )
        .map_err(nix_to_io)?;

        let mode = Mode::from_bits_truncate(self.mode);
        stat::fchmodat(None, path, mode, FchmodatFlags::NoFollowSymlink).map_err(nix_to_io)?;

        filetime::set_file_mtime(path, self.mtime)?;

        #[cfg(feature = "xattr")]
        if _opts.xattrs {
            for (name, value) in &self.xattrs {
                xattr::set(path, name, value)?;
            }
        }

        #[cfg(feature = "acl")]
        if _opts.acl {
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
        io::Error::new(io::ErrorKind::Other, err)
    }
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
