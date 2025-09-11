use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::OnceLock;

#[cfg(all(test, feature = "xattr"))]
use ::xattr as real_xattr;
#[cfg(all(test, feature = "xattr"))]
mod shim {
    use super::real_xattr;
    pub use super::real_xattr::{get, get_deref, remove, remove_deref, set};
    use std::ffi::OsString;
    use std::path::Path;

    pub fn list(path: &Path) -> std::io::Result<Vec<OsString>> {
        let attrs: Vec<OsString> = real_xattr::list(path)?.collect();
        if attrs.iter().any(|a| a == "user.disappearing") {
            let _ = remove(path, "user.disappearing");
        }
        Ok(attrs)
    }

    pub fn list_deref(path: &Path) -> std::io::Result<Vec<OsString>> {
        let attrs: Vec<OsString> = real_xattr::list_deref(path)?.collect();
        if attrs.iter().any(|a| a == "user.disappearing") {
            let _ = remove_deref(path, "user.disappearing");
        }
        Ok(attrs)
    }
}

#[cfg(not(all(test, feature = "xattr")))]
pub(crate) use ::xattr::*;
#[cfg(all(test, feature = "xattr"))]
pub(crate) use shim::*;
static XATTRS_SUPPORTED: OnceLock<bool> = OnceLock::new();

pub fn xattrs_supported() -> bool {
    *XATTRS_SUPPORTED.get_or_init(|| {
        let path = std::env::temp_dir().join("oc_rsync_xattr_check");
        if fs::write(&path, b"1").is_err() {
            return false;
        }
        let res = xattr::set(&path, "user.oc-rsync.test", b"1");
        let _ = fs::remove_file(&path);
        match res {
            Ok(_) => true,
            Err(err) => !matches!(
                err.raw_os_error(),
                Some(code) if code == libc::ENOTSUP || code == libc::EOPNOTSUPP
            ),
        }
    })
}

pub fn store_fake_super(path: &Path, uid: u32, gid: u32, mode: u32) {
    let _ = xattr::set(path, "user.rsync.uid", uid.to_string().as_bytes());
    let _ = xattr::set(path, "user.rsync.gid", gid.to_string().as_bytes());
    let _ = xattr::set(path, "user.rsync.mode", mode.to_string().as_bytes());
}

pub fn copy_xattrs(
    src: &Path,
    dest: &Path,
    include: Option<&dyn Fn(&OsStr) -> bool>,
    include_for_delete: Option<&dyn Fn(&OsStr) -> bool>,
) -> io::Result<()> {
    let mut attrs = Vec::new();
    match xattr::list_deref(src) {
        Ok(list) => {
            for attr in list {
                if let Some(name) = attr.to_str() {
                    if name.starts_with("security.")
                        || name == "system.posix_acl_access"
                        || name == "system.posix_acl_default"
                    {
                        continue;
                    }
                }
                if let Some(filter) = include {
                    if !filter(attr.as_os_str()) {
                        continue;
                    }
                }
                match xattr::get_deref(src, &attr) {
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
    crate::apply_xattrs(dest, &attrs, include, include_for_delete)
}

#[cfg(all(test, feature = "xattr"))]
mod tests {
    use super::*;
    use crate::{Metadata, Options};
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
        assert!(
            meta.xattrs
                .iter()
                .all(|(name, _)| name != "user.disappearing")
        );
        Ok(())
    }
}
