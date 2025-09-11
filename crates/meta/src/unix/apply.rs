// crates/meta/src/unix/apply.rs
use std::fs;
use std::io;
use std::path::Path;

use crate::{ChmodOp, ChmodTarget, Metadata, Options, normalize_mode};
#[cfg(target_os = "linux")]
use caps::{CapSet, Capability};
use filetime::{self, FileTime};
use nix::errno::Errno;
use nix::fcntl::{AT_FDCWD, AtFlags};
use nix::sys::stat::{self, FchmodatFlags, Mode};
use nix::unistd::{self, Gid, Uid};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use super::acl::write_acl;
use super::{gid_from_name, gid_to_name, nix_to_io, set_file_crtime, uid_from_name, uid_to_name};

impl Metadata {
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

            #[cfg(target_os = "linux")]
            let mut can_chown = unistd::Uid::effective().is_root();
            #[cfg(not(target_os = "linux"))]
            let can_chown = unistd::Uid::effective().is_root();
            #[cfg(target_os = "linux")]
            {
                if !can_chown {
                    can_chown = caps::has_cap(None, CapSet::Effective, Capability::CAP_CHOWN)
                        .unwrap_or(false);
                }
            }

            if can_chown {
                let res = if is_symlink {
                    match unistd::fchownat(
                        AT_FDCWD,
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
                        AtFlags::AT_SYMLINK_NOFOLLOW,
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
            } else {
                chown_failed = true;
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
            if let Err(err) = stat::fchmodat(AT_FDCWD, path, mode, FchmodatFlags::NoFollowSymlink) {
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
                        let mtime = if opts.times && !skip_mtime {
                            self.mtime
                        } else {
                            cur_mtime
                        };
                        filetime::set_symlink_file_times(path, atime, mtime)?;
                    } else if opts.times && !skip_mtime {
                        filetime::set_symlink_file_times(path, cur_atime, self.mtime)?;
                    }
                } else if opts.times && !skip_mtime {
                    filetime::set_symlink_file_times(path, cur_atime, self.mtime)?;
                }
            } else if opts.atimes {
                if let Some(atime) = self.atime {
                    if opts.times && !skip_mtime {
                        filetime::set_file_times(path, atime, self.mtime)?;
                    } else {
                        filetime::set_file_atime(path, atime)?;
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

        if opts.acl {
            let dacl = if is_dir {
                Some(self.default_acl.as_slice())
            } else {
                None
            };
            write_acl(
                path,
                &self.acl,
                dacl,
                opts.fake_super && !opts.super_user,
                opts.super_user,
            )?;
        } else {
            let dacl = if is_dir { Some(&[][..]) } else { None };
            write_acl(path, &[], dacl, false, false)?;
        }

        Ok(())
    }
}
