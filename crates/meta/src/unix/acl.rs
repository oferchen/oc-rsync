// crates/meta/src/unix/acl.rs
use std::fs;
use std::io;
use std::path::Path;
use std::sync::OnceLock;

use crate::normalize_mode;
use crate::unix::xattr;
use std::os::unix::fs::PermissionsExt;

#[cfg(feature = "acl")]
static ACLS_SUPPORTED: OnceLock<bool> = OnceLock::new();

#[cfg(feature = "acl")]
/// Return `true` if manipulating POSIX ACLs is supported on this system.
///
/// The check attempts to write an access ACL to a temporary file and a default
/// ACL to a temporary directory. If both operations succeed, ACLs are
/// considered supported.
pub fn acls_supported() -> bool {
    use posix_acl::{ACL_READ, PosixACL, Qualifier};
    *ACLS_SUPPORTED.get_or_init(|| {
        let tmp = std::env::temp_dir();
        let file = tmp.join("oc_rsync_acl_check_file");
        let dir = tmp.join("oc_rsync_acl_check_dir");
        if fs::write(&file, b"1").is_err() || fs::create_dir(&dir).is_err() {
            return false;
        }
        let mut acl = PosixACL::new(0o644);
        acl.set(Qualifier::User(0), ACL_READ);
        let res_file = acl.write_acl(&file);
        let mut dacl = PosixACL::new(0o755);
        dacl.set(Qualifier::User(0), ACL_READ);
        let res_dir = dacl.write_default_acl(&dir);
        let _ = fs::remove_file(&file);
        let _ = fs::remove_dir(&dir);
        let supported = |res: Result<(), posix_acl::ACLError>| match res {
            Ok(_) => true,
            Err(err) => {
                let code = err.as_io_error().and_then(|e| e.raw_os_error());
                !matches!(code, Some(c) if c == libc::ENOTSUP || c == libc::EOPNOTSUPP)
            }
        };
        supported(res_file) && supported(res_dir)
    })
}

/// Convert a [`posix_acl::ACLError`] into a standard [`io::Error`].
///
/// * `err` - ACL error to convert.
pub(crate) fn acl_to_io(err: posix_acl::ACLError) -> io::Error {
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

/// Check whether an errno from an ACL operation should be ignored.
///
/// * `code` - The errno value returned from the OS.
fn should_ignore_acl_errno(code: i32) -> bool {
    matches!(
        code,
        libc::EPERM | libc::EACCES | libc::ENOSYS | libc::EINVAL | libc::ENOTSUP | libc::ENODATA
    ) || code == libc::EOPNOTSUPP
        || {
            #[cfg(any(
                target_os = "macos",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "netbsd",
                target_os = "openbsd",
            ))]
            {
                code == libc::ENOATTR
            }
            #[cfg(not(any(
                target_os = "macos",
                target_os = "freebsd",
                target_os = "dragonfly",
                target_os = "netbsd",
                target_os = "openbsd",
            )))]
            {
                false
            }
        }
}

/// Determine if an ACL error corresponds to an ignorable errno.
///
/// * `err` - The ACL error to inspect.
pub(crate) fn should_ignore_acl_error(err: &posix_acl::ACLError) -> bool {
    err.as_io_error()
        .and_then(|e| e.raw_os_error())
        .is_some_and(should_ignore_acl_errno)
}

/// Remove the default ACL from `path` if present.
///
/// * `path` - Directory from which the default ACL should be removed.
pub(crate) fn remove_default_acl(path: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    unsafe extern "C" {
        fn acl_delete_def_file(path_p: *const libc::c_char) -> libc::c_int;
    }

    let c_path = CString::new(path.as_os_str().as_bytes())?;
    let ret = unsafe { acl_delete_def_file(c_path.as_ptr()) };
    if ret == 0 {
        Ok(())
    } else {
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(code) if should_ignore_acl_errno(code) => Ok(()),
            _ => Err(err),
        }
    }
}

/// Return `true` if `entries` are equivalent to the ACL implied by `mode`.
///
/// * `entries` - ACL entries to compare.
/// * `mode` - The file mode representing the trivial ACL.
fn is_trivial_acl(entries: &[posix_acl::ACLEntry], mode: u32) -> bool {
    posix_acl::PosixACL::new(mode).entries() == entries
}

/// Read access and default ACLs for a filesystem object.
///
/// * `path` - File or directory to inspect.
/// * `is_dir` - Indicates whether `path` is a directory.
/// * `fake_super` - Read ACLs from xattrs instead of the filesystem when `true`.
/// * `mode` - Mode bits used to detect trivial ACLs.
pub(crate) fn read_acl_inner(
    path: &Path,
    is_dir: bool,
    fake_super: bool,
    mode: u32,
) -> io::Result<(Vec<posix_acl::ACLEntry>, Vec<posix_acl::ACLEntry>)> {
    if fake_super {
        let mut acl = match xattr::get(path, "user.rsync.acl") {
            Ok(Some(val)) => decode_acl(&val),
            Ok(None) => Vec::new(),
            Err(err) => {
                if crate::should_ignore_xattr_error(&err) {
                    Vec::new()
                } else {
                    return Err(err);
                }
            }
        };
        let mut default_acl = if is_dir {
            match xattr::get(path, "user.rsync.dacl") {
                Ok(Some(val)) => decode_acl(&val),
                Ok(None) => Vec::new(),
                Err(err) => {
                    if crate::should_ignore_xattr_error(&err) {
                        Vec::new()
                    } else {
                        return Err(err);
                    }
                }
            }
        } else {
            Vec::new()
        };
        if is_trivial_acl(&acl, mode) {
            acl.clear();
        }
        if is_dir && is_trivial_acl(&default_acl, 0o777) {
            default_acl.clear();
        }
        if !acl.is_empty() || !default_acl.is_empty() {
            return Ok((acl, default_acl));
        }
    }

    let mut acl = match posix_acl::PosixACL::read_acl(path) {
        Ok(acl) => acl.entries(),
        Err(err) => {
            if should_ignore_acl_error(&err) {
                Vec::new()
            } else {
                return Err(acl_to_io(err));
            }
        }
    };
    if is_trivial_acl(&acl, mode) {
        acl.clear();
    }
    let mut default_acl = if is_dir {
        match posix_acl::PosixACL::read_default_acl(path) {
            Ok(dacl) => dacl.entries(),
            Err(err) => {
                if should_ignore_acl_error(&err) {
                    Vec::new()
                } else {
                    return Err(acl_to_io(err));
                }
            }
        }
    } else {
        Vec::new()
    };
    if is_dir && is_trivial_acl(&default_acl, 0o777) {
        default_acl.clear();
    }
    Ok((acl, default_acl))
}

/// Convenience wrapper around [`read_acl_inner`] that infers metadata from `path`.
///
/// * `path` - File or directory whose ACLs should be read.
/// * `fake_super` - Read ACLs from xattrs instead of the filesystem when `true`.
pub fn read_acl(
    path: &Path,
    fake_super: bool,
) -> io::Result<(Vec<posix_acl::ACLEntry>, Vec<posix_acl::ACLEntry>)> {
    let meta = fs::symlink_metadata(path)?;
    let mode = normalize_mode(meta.permissions().mode());
    let is_dir = meta.file_type().is_dir();
    read_acl_inner(path, is_dir, fake_super, mode)
}

/// Write access and default ACLs to a filesystem object and optionally store
/// them as fake-super xattrs.
///
/// * `path` - Target file or directory.
/// * `acl` - Access ACL entries to apply.
/// * `default_acl` - Optional default ACL entries for directories.
/// * `fake_super` - When `true`, store ACLs as xattrs instead of applying them.
/// * `super_user` - Indicates whether the process has super-user privileges.
pub fn write_acl(
    path: &Path,
    acl: &[posix_acl::ACLEntry],
    default_acl: Option<&[posix_acl::ACLEntry]>,
    fake_super: bool,
    super_user: bool,
) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    let is_dir = meta.file_type().is_dir();
    let cur_mode = normalize_mode(meta.permissions().mode());

    let empty: &[posix_acl::ACLEntry] = &[];
    let acl_eff = if is_trivial_acl(acl, cur_mode) {
        empty
    } else {
        acl
    };
    let dacl_eff = default_acl.map(|d| {
        if is_dir && is_trivial_acl(d, 0o777) {
            empty
        } else {
            d
        }
    });

    apply_access_acl_if_nontrivial(path, acl_eff)?;
    apply_default_acl_option(path, is_dir, dacl_eff)?;
    maybe_store_fake_super(path, is_dir, fake_super, super_user, acl_eff, dacl_eff);

    Ok(())
}

/// Apply the access ACL to `path` if the provided entries are non-empty.
///
/// * `path` - File or directory where the ACL should be written.
/// * `acl` - Access ACL entries to write.
fn apply_access_acl_if_nontrivial(path: &Path, acl: &[posix_acl::ACLEntry]) -> io::Result<()> {
    if acl.is_empty() {
        return Ok(());
    }
    let mut obj = posix_acl::PosixACL::empty();
    for e in acl {
        obj.set(e.qual, e.perm);
    }
    match obj.write_acl(path) {
        Ok(_) => Ok(()),
        Err(err) if should_ignore_acl_error(&err) => Ok(()),
        Err(err) => Err(acl_to_io(err)),
    }
}

/// Apply or remove a directory's default ACL based on the provided option.
///
/// * `path` - Target directory.
/// * `is_dir` - Indicates if `path` is a directory.
/// * `dacl` - Optional default ACL entries; `Some(&[])` removes the ACL.
fn apply_default_acl_option(
    path: &Path,
    is_dir: bool,
    dacl: Option<&[posix_acl::ACLEntry]>,
) -> io::Result<()> {
    if !is_dir {
        return Ok(());
    }
    match dacl {
        None => Ok(()),
        Some([]) => remove_default_acl(path),
        Some(d) => {
            let mut obj = posix_acl::PosixACL::empty();
            for e in d {
                obj.set(e.qual, e.perm);
            }
            match obj.write_default_acl(path) {
                Ok(_) => Ok(()),
                Err(err) if should_ignore_acl_error(&err) => Ok(()),
                Err(err) => Err(acl_to_io(err)),
            }
        }
    }
}

/// Store ACLs in fake-super extended attributes when required.
///
/// * `path` - File or directory to annotate.
/// * `is_dir` - Indicates whether `path` is a directory.
/// * `fake_super` - Enables fake-super behavior.
/// * `super_user` - If `false`, ACLs are stored as xattrs.
/// * `acl` - Access ACL entries.
/// * `dacl` - Default ACL entries for directories.
fn maybe_store_fake_super(
    path: &Path,
    is_dir: bool,
    fake_super: bool,
    super_user: bool,
    acl: &[posix_acl::ACLEntry],
    dacl: Option<&[posix_acl::ACLEntry]>,
) {
    if fake_super && !super_user {
        let empty: &[posix_acl::ACLEntry] = &[];
        let d = if is_dir { dacl.unwrap_or(empty) } else { empty };
        store_fake_super_acl(path, acl, d);
    }
}

/// Encode ACL entries into a compact byte representation.
///
/// * `entries` - ACL entries to encode.
pub fn encode_acl(entries: &[posix_acl::ACLEntry]) -> Vec<u8> {
    use posix_acl::Qualifier;
    let mut out = Vec::with_capacity(entries.len() * 9);
    for e in entries {
        let (tag, id) = match e.qual {
            Qualifier::UserObj => (1u8, 0u32),
            Qualifier::GroupObj => (2, 0),
            Qualifier::Other => (3, 0),
            Qualifier::User(id) => (4, id),
            Qualifier::Group(id) => (5, id),
            Qualifier::Mask => (6, 0),
            Qualifier::Undefined => (0, 0),
        };
        out.push(tag);
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&e.perm.to_le_bytes());
    }
    out
}

/// Decode ACL entries from the byte representation produced by [`encode_acl`].
///
/// * `data` - Encoded ACL bytes.
pub fn decode_acl(data: &[u8]) -> Vec<posix_acl::ACLEntry> {
    use posix_acl::Qualifier;
    let mut entries = Vec::new();
    let mut i = 0;
    while i + 9 <= data.len() {
        let tag = data[i];
        i += 1;
        let id = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        i += 4;
        let perm = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]);
        i += 4;
        let qual = match tag {
            1 => Qualifier::UserObj,
            2 => Qualifier::GroupObj,
            3 => Qualifier::Other,
            4 => Qualifier::User(id),
            5 => Qualifier::Group(id),
            6 => Qualifier::Mask,
            _ => Qualifier::Undefined,
        };
        entries.push(posix_acl::ACLEntry { qual, perm });
    }
    entries
}

/// Store ACLs as extended attributes for use in fake-super mode.
///
/// * `path` - File or directory to annotate.
/// * `acl` - Access ACL entries to store.
/// * `default_acl` - Default ACL entries to store.
pub(crate) fn store_fake_super_acl(
    path: &Path,
    acl: &[posix_acl::ACLEntry],
    default_acl: &[posix_acl::ACLEntry],
) {
    if acl.is_empty() {
        let _ = xattr::remove(path, "user.rsync.acl");
    } else {
        let data = encode_acl(acl);
        let _ = xattr::set(path, "user.rsync.acl", &data);
    }
    if default_acl.is_empty() {
        let _ = xattr::remove(path, "user.rsync.dacl");
    } else {
        let data = encode_acl(default_acl);
        let _ = xattr::set(path, "user.rsync.dacl", &data);
    }
}
