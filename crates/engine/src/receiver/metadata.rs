// crates/engine/src/receiver/metadata.rs
#[cfg(unix)]
use nix::unistd::{Gid, Uid, chown};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::Path;
#[cfg(feature = "xattr")]
use std::rc::Rc;
use std::sync::Arc;

use crate::cleanup::{atomic_rename, remove_basename_partial};
use crate::io::io_context;
use crate::{EngineError, Result};

use filelist::Entry;
#[cfg(feature = "acl")]
use meta::decode_acl;

use super::Receiver;

impl Receiver {
    pub(crate) fn copy_metadata_now(
        &mut self,
        src: &Path,
        dest: &Path,
        entry: Option<&Entry>,
    ) -> Result<()> {
        #[cfg(unix)]
        if self.opts.hard_links {
            if let Some(entry) = entry {
                if let Some(id) = entry.hardlink {
                    if !self.register_hard_link(id as u64, dest) {
                        return Ok(());
                    }
                }
            }
        }
        #[cfg(unix)]
        if self.opts.write_devices && !self.opts.devices {
            if let Ok(meta) = fs::symlink_metadata(dest) {
                let ft = meta.file_type();
                if ft.is_char_device() || ft.is_block_device() {
                    return Ok(());
                }
            }
        }

        #[cfg(unix)]
        {
            if self.opts.perms {
                let src_meta = fs::symlink_metadata(src).map_err(|e| io_context(src, e))?;
                if !src_meta.file_type().is_symlink() {
                    let mode = meta::mode_from_metadata(&src_meta);
                    fs::set_permissions(dest, fs::Permissions::from_mode(mode))
                        .map_err(|e| io_context(dest, e))?;
                }
            }
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let chown_uid = self.opts.chown.and_then(|(u, _)| u);
            let chown_gid = self.opts.chown.and_then(|(_, g)| g);

            let uid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>> = if self.opts.owner {
                if let Some(ref map) = self.opts.uid_map {
                    Some(map.0.clone())
                } else if let Some(uid) = chown_uid {
                    Some(Arc::new(move |_| uid))
                } else {
                    None
                }
            } else {
                None
            };

            let gid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>> = if self.opts.group {
                if let Some(ref map) = self.opts.gid_map {
                    Some(map.0.clone())
                } else if let Some(gid) = chown_gid {
                    Some(Arc::new(move |_| gid))
                } else {
                    None
                }
            } else {
                None
            };

            #[cfg(feature = "xattr")]
            let m1 = self.matcher.clone();
            #[cfg(feature = "xattr")]
            let m2 = self.matcher.clone();

            let mut meta_opts = meta::Options {
                xattrs: {
                    #[cfg(feature = "xattr")]
                    {
                        self.opts.xattrs || (self.opts.fake_super && !self.opts.super_user)
                    }
                    #[cfg(not(feature = "xattr"))]
                    {
                        false
                    }
                },
                #[cfg(feature = "xattr")]
                xattr_filter: Some(Rc::new(move |name: &std::ffi::OsStr| {
                    m1.is_xattr_included(name).unwrap_or(true)
                })),
                #[cfg(feature = "xattr")]
                xattr_filter_delete: Some(Rc::new(move |name: &std::ffi::OsStr| {
                    m2.is_xattr_included_for_delete(name).unwrap_or(true)
                })),
                acl: {
                    #[cfg(feature = "acl")]
                    {
                        self.opts.acls && entry.is_none()
                    }
                    #[cfg(not(feature = "acl"))]
                    {
                        false
                    }
                },
                chmod: self.opts.chmod.clone(),
                owner: self.opts.owner,
                group: self.opts.group,
                perms: self.opts.perms || {
                    #[cfg(feature = "acl")]
                    {
                        self.opts.acls
                    }
                    #[cfg(not(feature = "acl"))]
                    {
                        false
                    }
                },
                executability: self.opts.executability,
                times: self.opts.times,
                atimes: self.opts.atimes,
                crtimes: self.opts.crtimes,
                omit_dir_times: self.opts.omit_dir_times,
                omit_link_times: self.opts.omit_link_times,
                uid_map,
                gid_map,
                fake_super: self.opts.fake_super && !self.opts.super_user,
                super_user: self.opts.super_user,
                numeric_ids: self.opts.numeric_ids,
            };

            if meta_opts.needs_metadata() {
                if let Ok(src_meta) = fs::symlink_metadata(src) {
                    if src_meta.file_type().is_dir() {
                        if let Some(ref rules) = meta_opts.chmod {
                            if rules
                                .iter()
                                .all(|r| matches!(r.target, meta::ChmodTarget::File))
                            {
                                meta_opts.chmod = None;
                                if !meta_opts.needs_metadata() && !self.opts.acls {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }

            let meta = if meta_opts.needs_metadata() || (self.opts.acls && entry.is_none()) {
                Some(meta::Metadata::from_path(src, meta_opts.clone()).map_err(EngineError::from)?)
            } else {
                None
            };

            if let Some(ref meta) = meta {
                if meta_opts.needs_metadata() {
                    meta.apply(dest, meta_opts.clone())
                        .map_err(EngineError::from)?;
                }
                if self.opts.fake_super && !self.opts.super_user {
                    #[cfg(feature = "xattr")]
                    {
                        meta::store_fake_super(dest, meta.uid, meta.gid, meta.mode);
                    }
                }
            }

            #[cfg(feature = "acl")]
            if self.opts.acls {
                let (acl, default_acl) = if let Some(entry) = entry {
                    (decode_acl(&entry.acl), decode_acl(&entry.default_acl))
                } else if let Some(ref meta) = meta {
                    (meta.acl.clone(), meta.default_acl.clone())
                } else {
                    (Vec::new(), Vec::new())
                };
                let default_acl = if default_acl.is_empty() {
                    &[] as &[meta::ACLEntry]
                } else {
                    &default_acl[..]
                };
                let acl = if entry.is_none() && dest.is_dir() {
                    &[] as &[meta::ACLEntry]
                } else {
                    &acl[..]
                };
                meta::write_acl(
                    dest,
                    acl,
                    Some(default_acl),
                    meta_opts.fake_super && !meta_opts.super_user,
                    meta_opts.super_user,
                )
                .map_err(EngineError::from)?;
            }
        }
        #[cfg(not(unix))]
        let _ = entry;
        let _ = (src, dest);
        Ok(())
    }

    pub fn copy_metadata(
        &mut self,
        src: &Path,
        dest: &Path,
        entry: Option<&Entry>,
    ) -> Result<()> {
        if self.opts.delay_updates && self.delayed.iter().any(|(_, _, d)| d == dest) {
            #[cfg(unix)]
            if self.opts.hard_links {
                if let Some(entry) = entry {
                    if let Some(id) = entry.hardlink {
                        self.register_hard_link(id as u64, dest);
                    }
                }
            }
            return Ok(());
        }
        self.copy_metadata_now(src, dest, entry)
    }

    pub fn finalize(&mut self) -> Result<()> {
        for (src, tmp, dest) in std::mem::take(&mut self.delayed) {
            atomic_rename(&tmp, &dest)?;
            if let Some(tmp_parent) = tmp.parent() {
                if dest.parent() != Some(tmp_parent)
                    && tmp_parent
                        .read_dir()
                        .map(|mut i| i.next().is_none())
                        .unwrap_or(false)
                {
                    let _ = fs::remove_dir(tmp_parent);
                }
            }
            remove_basename_partial(&dest);
            #[cfg(unix)]
            if let Some((uid, gid)) = self.opts.copy_as {
                let gid = gid.map(Gid::from_raw);
                chown(&dest, Some(Uid::from_raw(uid)), gid)
                    .map_err(|e| io_context(&dest, std::io::Error::from(e)))?;
            }
            self.copy_metadata_now(&src, &dest, None)?;
        }
        #[cfg(unix)]
        self.link_map.finalize()?;
        Ok(())
    }
}
