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
#[cfg(feature = "xattr")]
use std::rc::Rc;
use std::sync::Arc;
use users::{get_group_by_gid, get_group_by_name, get_user_by_name, get_user_by_uid};

#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;

#[cfg(feature = "xattr")]
use std::ffi::{OsStr, OsString};

#[cfg(feature = "xattr")]
type XattrFilter = Rc<dyn Fn(&OsStr) -> bool>;

#[cfg(all(test, feature = "xattr"))]
mod xattr {
    pub use real_xattr::{get, get_deref, remove, remove_deref, set};
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

    pub fn list_deref(path: &Path) -> std::io::Result<Vec<OsString>> {
        let attrs: Vec<OsString> = real_xattr::list_deref(path)?.collect();
        if attrs.iter().any(|a| a == "user.disappearing") {
            let _ = remove_deref(path, "user.disappearing");
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
    #[cfg(feature = "xattr")]
    pub xattr_filter: Option<XattrFilter>,
    #[cfg(feature = "xattr")]
    pub xattr_filter_delete: Option<XattrFilter>,
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
            .field("xattr_filter", &{
                #[cfg(feature = "xattr")]
                {
                    self.xattr_filter.is_some()
                }
                #[cfg(not(feature = "xattr"))]
                {
                    false
                }
            })
            .field("xattr_filter_delete", &{
                #[cfg(feature = "xattr")]
                {
                    self.xattr_filter_delete.is_some()
                }
                #[cfg(not(feature = "xattr"))]
                {
                    false
                }
            })
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
            match xattr::list(path) {
                Ok(list) => {
                    for attr in list {
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
                        if let Some(ref filter) = opts.xattr_filter {
                            if !filter(attr.as_os_str()) {
                                continue;
                            }
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

        #[cfg(feature = "acl")]
        let is_dir = meta.file_type().is_dir();

        #[cfg(feature = "acl")]
        let (acl, default_acl) = if opts.acl {
            read_acl_from_path(path, is_dir, opts.fake_super, mode)?
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

        let mut expected_uid = self.uid;
        let mut expected_gid = self.gid;
        let mut chown_failed = false;
        if opts.owner || opts.group {
            let uid = if let Some(ref map) = opts.uid_map {
                map(self.uid)
            } else if !opts.numeric_ids {
                if let Some(name) = uid_to_name(self.uid) {
                    uid_from_name(&name).unwrap_or(self.uid)
                } else {
                    self.uid
                }
            } else {
                self.uid
            };
            let gid = if let Some(ref map) = opts.gid_map {
                map(self.gid)
            } else if !opts.numeric_ids {
                if let Some(name) = gid_to_name(self.gid) {
                    gid_from_name(&name).unwrap_or(self.gid)
                } else {
                    self.gid
                }
            } else {
                self.gid
            };
            expected_uid = uid;
            expected_gid = gid;
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
                    Errno::EPERM | Errno::EACCES => {
                        chown_failed = true;
                        tracing::warn!(?path, ?err, "unable to change owner/group");
                    }
                    _ => return Err(nix_to_io(err)),
                }
            }
        }

        let mut need_chmod =
            (opts.perms || opts.chmod.is_some() || opts.executability || opts.acl) && !is_symlink;
        let mut mode_val = if opts.perms || opts.acl {
            normalize_mode(self.mode)
        } else {
            normalize_mode(meta.permissions().mode())
        };
        if opts.executability && !opts.perms {
            mode_val = (mode_val & !0o111) | (self.mode & 0o111);
        }
        let orig_mode = mode_val;
        if (opts.owner || opts.group) && !is_symlink && (self.mode & 0o6000) != 0 {
            need_chmod = true;
            mode_val = (mode_val & !0o6000) | (normalize_mode(self.mode) & 0o6000);
        }
        if need_chmod {
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
                        fs::set_permissions(path, perm)?;
                    }
                    _ => return Err(nix_to_io(err)),
                }
            }
            let meta_after = fs::symlink_metadata(path)?;
            if normalize_mode(meta_after.permissions().mode()) != mode_val {
                return Err(io::Error::other("failed to restore mode"));
            }
        }

        if (opts.owner || opts.group) && !chown_failed {
            let meta_after = fs::symlink_metadata(path)?;
            if opts.owner && meta_after.uid() != expected_uid {
                return Err(io::Error::other("failed to restore uid"));
            }
            if opts.group && meta_after.gid() != expected_gid {
                return Err(io::Error::other("failed to restore gid"));
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
        if opts.xattrs || opts.fake_super {
            crate::apply_xattrs(
                path,
                &self.xattrs,
                opts.xattr_filter.as_deref(),
                opts.xattr_filter_delete.as_deref(),
            )?;
        }

        #[cfg(feature = "acl")]
        if opts.acl {
            {
                let cur_mode = normalize_mode(fs::symlink_metadata(path)?.permissions().mode());
                if self.acl.is_empty() {
                    let mut acl = posix_acl::PosixACL::new(cur_mode);
                    if let Err(err) = acl.write_acl(path) {
                        if !should_ignore_acl_error(&err) {
                            return Err(acl_to_io(err));
                        }
                    }
                } else {
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

            #[cfg(all(feature = "xattr", feature = "acl"))]
            if opts.fake_super && !opts.super_user {
                store_fake_super_acl(
                    path,
                    &self.acl,
                    if is_dir { &self.default_acl } else { &[] },
                );
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

#[cfg(feature = "xattr")]
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

#[cfg(feature = "acl")]
fn is_trivial_acl(entries: &[posix_acl::ACLEntry], mode: u32) -> bool {
    posix_acl::PosixACL::new(mode).entries() == entries
}

#[cfg(feature = "acl")]
fn read_acl_from_path(
    path: &Path,
    is_dir: bool,
    fake_super: bool,
    mode: u32,
) -> io::Result<(Vec<posix_acl::ACLEntry>, Vec<posix_acl::ACLEntry>)> {
    #[cfg(all(feature = "xattr", feature = "acl"))]
    if fake_super {
        let acl = match xattr::get(path, "user.rsync.acl") {
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
        let default_acl = if is_dir {
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
    let default_acl = if is_dir {
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
    Ok((acl, default_acl))
}

#[cfg(all(feature = "xattr", feature = "acl"))]
fn encode_acl(entries: &[posix_acl::ACLEntry]) -> Vec<u8> {
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

#[cfg(all(feature = "xattr", feature = "acl"))]
fn decode_acl(data: &[u8]) -> Vec<posix_acl::ACLEntry> {
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

#[cfg(all(feature = "xattr", feature = "acl"))]
fn store_fake_super_acl(
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
