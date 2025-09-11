// crates/engine/src/receiver.rs

#[cfg(unix)]
use nix::unistd::{Gid, Uid, chown};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Cursor, Seek, SeekFrom};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
#[cfg(feature = "xattr")]
use std::rc::Rc;
use std::sync::Arc;

use compress::{Codec, Decompressor, ZlibX, Zstd, should_compress};
use filters::Matcher;

use crate::block::block_size;
use crate::cleanup::{
    TempFileGuard, atomic_rename, open_for_read, partial_paths, remove_basename_partial,
    tmp_file_path,
};
use crate::delta::{Op, Progress, apply_delta};
use crate::io::{io_context, is_device, preallocate};
use crate::{EngineError, ReadSeek, Result, SyncOptions, ensure_max_alloc, last_good_block};
use checksums::ChecksumConfigBuilder;
use logging::{NopObserver, Observer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverState {
    Idle,
    Applying,
    Finished,
}

pub struct Receiver {
    state: ReceiverState,
    codec: Option<Codec>,
    opts: SyncOptions,
    pub(crate) matcher: Matcher,
    delayed: Vec<(PathBuf, PathBuf, PathBuf)>,
    #[cfg(unix)]
    link_map: meta::HardLinks,
    progress_sink: Arc<dyn Observer>,
}

impl Default for Receiver {
    fn default() -> Self {
        Self::new(None, SyncOptions::default())
    }
}

impl Receiver {
    pub fn new(codec: Option<Codec>, opts: SyncOptions) -> Self {
        Self {
            state: ReceiverState::Idle,
            codec,
            opts,
            matcher: Matcher::default(),
            delayed: Vec::new(),
            #[cfg(unix)]
            link_map: meta::HardLinks::default(),
            progress_sink: Arc::new(NopObserver),
        }
    }

    pub fn set_progress_sink(&mut self, sink: Arc<dyn Observer>) {
        self.progress_sink = sink;
    }

    #[cfg(unix)]
    pub fn register_hard_link(&mut self, id: u64, path: &Path) -> bool {
        self.link_map.register(id, path)
    }

    pub fn apply<I>(&mut self, src: &Path, dest: &Path, _rel: &Path, delta: I) -> Result<PathBuf>
    where
        I: IntoIterator<Item = Result<Op>>,
    {
        self.state = ReceiverState::Applying;
        let mut dest = dest.to_path_buf();
        while dest
            .as_os_str()
            .to_string_lossy()
            .ends_with(std::path::MAIN_SEPARATOR)
        {
            if !dest.pop() {
                break;
            }
        }
        if dest.is_dir() {
            if _rel.as_os_str().is_empty() {
                if let Some(name) = src.file_name() {
                    dest.push(name);
                }
            } else {
                let mut rel = _rel.to_path_buf();
                while rel
                    .as_os_str()
                    .to_string_lossy()
                    .ends_with(std::path::MAIN_SEPARATOR)
                {
                    if !rel.pop() {
                        break;
                    }
                }
                dest.push(rel);
            }
        }
        let src_len = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        let (partial, basename_partial) = partial_paths(&dest, self.opts.partial_dir.as_deref());
        let mut existing_partial = if partial.exists() {
            Some(partial.clone())
        } else if let Some(bp) = basename_partial.as_ref() {
            if bp.exists() { Some(bp.clone()) } else { None }
        } else {
            None
        };
        if let Some(ref p) = existing_partial {
            let len = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            if len < src_len {
                existing_partial = None;
            }
        }
        if (self.opts.append || self.opts.append_verify)
            && existing_partial.is_none()
            && !dest.exists()
        {
            return Err(io_context(&dest, io::Error::from(io::ErrorKind::NotFound)));
        }

        let basis_path = if self.opts.inplace {
            dest.clone()
        } else if (self.opts.partial || self.opts.append || self.opts.append_verify)
            && existing_partial.is_some()
        {
            existing_partial.clone().ok_or_else(|| {
                EngineError::Other(
                    "existing partial path should exist when resuming transfers".into(),
                )
            })?
        } else {
            dest.clone()
        };
        let dest_parent = dest.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(dest_parent).map_err(|e| io_context(dest_parent, e))?;
        let mut auto_tmp = false;
        let mut tmp_dest = if self.opts.inplace {
            dest.clone()
        } else if let Some(dir) = &self.opts.temp_dir {
            #[cfg(unix)]
            let same_dev = match (fs::metadata(dest_parent), fs::metadata(dir)) {
                (Ok(d_meta), Ok(t_meta)) => d_meta.dev() == t_meta.dev(),
                _ => true,
            };
            #[cfg(not(unix))]
            let same_dev = true;
            let tmp_parent: &Path = if same_dev {
                dir.as_path()
            } else {
                auto_tmp = true;
                dest_parent
            };
            tmp_file_path(tmp_parent, &dest)
        } else if (self.opts.partial || self.opts.append || self.opts.append_verify)
            && existing_partial.is_some()
        {
            existing_partial.clone().ok_or_else(|| {
                EngineError::Other(
                    "existing partial path should exist when resuming transfers".into(),
                )
            })?
        } else if self.opts.partial {
            partial.clone()
        } else {
            dest.clone()
        };
        if !(self.opts.inplace || self.opts.partial || self.opts.append || self.opts.append_verify)
            && self.opts.temp_dir.is_none()
            && basis_path == dest
            && !self.opts.write_devices
        {
            auto_tmp = true;
            tmp_dest = tmp_file_path(dest_parent, &dest);
        }
        let mut needs_rename = !self.opts.inplace
            && ((self.opts.partial || self.opts.append || self.opts.append_verify)
                && existing_partial.is_some()
                || self.opts.temp_dir.is_some()
                || auto_tmp);
        if self.opts.delay_updates && !self.opts.inplace && !self.opts.write_devices {
            if tmp_dest == dest {
                tmp_dest = tmp_file_path(dest_parent, &dest);
            }
            needs_rename = true;
        }
        let mut tmp_guard = if needs_rename {
            Some(TempFileGuard::new(tmp_dest.clone()))
        } else {
            None
        };
        let cfg = ChecksumConfigBuilder::new()
            .strong(self.opts.strong)
            .seed(self.opts.checksum_seed)
            .build();
        let block_size = if self.opts.block_size > 0 {
            self.opts.block_size
        } else {
            block_size(src_len)
        };
        let resume_basis = existing_partial.as_ref().unwrap_or(&tmp_dest);
        let mut resume = if self.opts.partial || self.opts.append || self.opts.append_verify {
            if self.opts.append && !self.opts.append_verify {
                fs::metadata(resume_basis).map(|m| m.len()).unwrap_or(0)
            } else {
                last_good_block(&cfg, src, resume_basis, block_size, &self.opts)?
            }
        } else {
            0
        };
        if resume > src_len {
            resume = src_len;
        }
        let mut basis: Box<dyn ReadSeek> = if self.opts.copy_devices || self.opts.write_devices {
            if let Ok(meta) = fs::symlink_metadata(&basis_path) {
                let ft = meta.file_type();
                if is_device(&ft) {
                    Box::new(Cursor::new(Vec::new()))
                } else {
                    match open_for_read(&basis_path, &self.opts) {
                        Ok(f) => {
                            let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                            ensure_max_alloc(len, &self.opts)?;
                            Box::new(BufReader::new(f))
                        }
                        Err(_) => Box::new(Cursor::new(Vec::new())),
                    }
                }
            } else {
                Box::new(Cursor::new(Vec::new()))
            }
        } else {
            match open_for_read(&basis_path, &self.opts) {
                Ok(f) => {
                    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                    ensure_max_alloc(len, &self.opts)?;
                    Box::new(BufReader::new(f))
                }
                Err(_) => Box::new(Cursor::new(Vec::new())),
            }
        };
        let parent = tmp_dest.parent().unwrap_or_else(|| Path::new("."));
        let created = !parent.exists();
        fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
        #[cfg(unix)]
        if created {
            if let Some((uid, gid)) = self.opts.copy_as {
                let gid = gid.map(Gid::from_raw);
                chown(parent, Some(Uid::from_raw(uid)), gid)
                    .map_err(|e| io_context(parent, std::io::Error::from(e)))?;
            }
        }
        #[cfg(unix)]
        if !self.opts.write_devices {
            let check_path: &Path = if auto_tmp { &dest } else { &tmp_dest };
            if let Ok(meta) = fs::symlink_metadata(check_path) {
                let ft = meta.file_type();
                if ft.is_block_device() || ft.is_char_device() {
                    if self.opts.copy_devices {
                        fs::remove_file(check_path).map_err(|e| io_context(check_path, e))?;
                    } else {
                        return Err(EngineError::Other(
                            "refusing to write to device; use --write-devices".into(),
                        ));
                    }
                }
            }
        }

        let mut out = if self.opts.write_devices {
            OpenOptions::new()
                .write(true)
                .open(&tmp_dest)
                .map_err(|e| io_context(&tmp_dest, e))?
        } else if self.opts.inplace
            || self.opts.partial
            || self.opts.append
            || self.opts.append_verify
        {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_dest)
                .map_err(|e| io_context(&tmp_dest, e))?
        } else {
            File::create(&tmp_dest).map_err(|e| io_context(&tmp_dest, e))?
        };
        let file_codec = if should_compress(src, &self.opts.skip_compress) {
            self.codec
        } else {
            None
        };

        let mut ops_vec = Vec::new();
        let mut dest_len = 0u64;
        for op_res in delta {
            let mut op = op_res?;
            if let Some(codec) = file_codec {
                if let Op::Data(ref mut d) = op {
                    *d = match codec {
                        Codec::Zlib | Codec::ZlibX => {
                            let mut out = Vec::new();
                            let mut cursor = d.as_slice();
                            ZlibX::default()
                                .decompress(&mut cursor, &mut out)
                                .map_err(EngineError::from)?;
                            out
                        }
                        Codec::Zstd => {
                            let mut out = Vec::new();
                            let mut cursor = d.as_slice();
                            Zstd::default()
                                .decompress(&mut cursor, &mut out)
                                .map_err(EngineError::from)?;
                            out
                        }
                    };
                }
            }
            dest_len += match &op {
                Op::Data(d) => d.len() as u64,
                Op::Copy { len, .. } => *len as u64,
            };
            ops_vec.push(op);
        }

        if !self.opts.write_devices {
            out.set_len(resume)?;
            out.seek(SeekFrom::Start(resume))?;
            if self.opts.preallocate {
                preallocate(&out, dest_len)?;
            }
        }

        let mut progress = if self.opts.progress {
            Some(Progress::new(
                &dest,
                dest_len,
                self.opts.human_readable,
                resume,
                self.opts.quiet,
                self.progress_sink.clone(),
            ))
        } else {
            None
        };

        apply_delta(
            &mut basis,
            ops_vec.into_iter().map(Ok),
            &mut out,
            &self.opts,
            0,
            &mut progress,
        )?;
        if let Some(mut p) = progress {
            p.finish();
        }
        if !self.opts.write_devices {
            let len = out.stream_position()?;
            out.set_len(len)?;
        }
        if self.opts.fsync {
            out.sync_all().map_err(|e| io_context(&tmp_dest, e))?;
        }
        drop(out);
        if needs_rename {
            if self.opts.delay_updates {
                self.delayed
                    .push((src.to_path_buf(), tmp_dest.clone(), dest.clone()));
                if let Some(g) = tmp_guard.as_mut() {
                    g.disarm();
                }
            } else {
                atomic_rename(&tmp_dest, &dest)?;
                if let Some(g) = tmp_guard.as_mut() {
                    g.disarm();
                }
                if (self.opts.partial || self.opts.partial_dir.is_some()) && partial != tmp_dest {
                    let _ = fs::remove_file(&partial);
                    if let Some(stem) = dest.file_stem() {
                        let mut name = stem.to_os_string();
                        name.push(".partial");
                        let alt = dest.with_file_name(name);
                        if alt != partial {
                            let _ = fs::remove_file(alt);
                        }
                    }
                }
                if let Some(tmp_parent) = tmp_dest.parent() {
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
            }
            #[cfg(unix)]
            if let Some((uid, gid)) = self.opts.copy_as {
                let gid = gid.map(Gid::from_raw);
                chown(&dest, Some(Uid::from_raw(uid)), gid)
                    .map_err(|e| io_context(&dest, std::io::Error::from(e)))?;
            }
        } else {
            #[cfg(unix)]
            if let Some((uid, gid)) = self.opts.copy_as {
                let gid = gid.map(Gid::from_raw);
                chown(&dest, Some(Uid::from_raw(uid)), gid)
                    .map_err(|e| io_context(&dest, std::io::Error::from(e)))?;
            }
        }
        self.state = ReceiverState::Finished;
        Ok(if self.opts.delay_updates && needs_rename {
            tmp_dest
        } else {
            dest
        })
    }
}

impl Receiver {
    pub(crate) fn copy_metadata_now(&self, src: &Path, dest: &Path) -> Result<()> {
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
                    m1.is_xattr_included(name).unwrap_or(false)
                })),
                #[cfg(feature = "xattr")]
                xattr_filter_delete: Some(Rc::new(move |name: &std::ffi::OsStr| {
                    m2.is_xattr_included_for_delete(name).unwrap_or(false)
                })),
                acl: {
                    #[cfg(feature = "acl")]
                    {
                        self.opts.acls
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

            let meta = if meta_opts.needs_metadata() || self.opts.acls {
                Some(meta::Metadata::from_path(src, meta_opts.clone()).map_err(EngineError::from)?)
            } else {
                None
            };

            if let Some(meta) = meta {
                if meta_opts.needs_metadata() {
                    meta.apply(dest, meta_opts.clone())
                        .map_err(EngineError::from)?;
                }
                #[cfg(feature = "acl")]
                if self.opts.acls && (!meta.acl.is_empty() || !meta.default_acl.is_empty()) {
                    meta::write_acl(
                        dest,
                        &meta.acl,
                        Some(&meta.default_acl),
                        meta_opts.fake_super && !meta_opts.super_user,
                        meta_opts.super_user,
                    )
                    .map_err(EngineError::from)?;
                }
                if self.opts.fake_super && !self.opts.super_user {
                    #[cfg(feature = "xattr")]
                    {
                        meta::store_fake_super(dest, meta.uid, meta.gid, meta.mode);
                    }
                }
            } else if !self.opts.acls {
                #[cfg(feature = "acl")]
                {
                    meta::write_acl(
                        dest,
                        &[],
                        Some(&[]),
                        meta_opts.fake_super && !meta_opts.super_user,
                        meta_opts.super_user,
                    )
                    .map_err(EngineError::from)?;
                }
            }
        }
        let _ = (src, dest);
        Ok(())
    }

    pub fn copy_metadata(&mut self, src: &Path, dest: &Path) -> Result<()> {
        if self.opts.delay_updates && self.delayed.iter().any(|(_, _, d)| d == dest) {
            return Ok(());
        }
        self.copy_metadata_now(src, dest)
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
            self.copy_metadata_now(&src, &dest)?;
        }
        #[cfg(unix)]
        self.link_map.finalize()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn apply_without_existing_partial() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dest = tmp.path().join("dest.txt");
        fs::write(&src, b"hello").unwrap();

        let mut opts = SyncOptions::default();
        opts.partial = true;
        let mut recv = Receiver::new(None, opts);

        let delta = vec![Ok(Op::Data(b"hello".to_vec()))];
        recv.apply(&src, &dest, Path::new(""), delta).unwrap();

        let output = fs::read(&dest).unwrap();
        assert_eq!(output, b"hello");
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn apply_with_existing_partial() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.txt");
        let dest = tmp.path().join("dest.txt");
        fs::write(&src, b"old!").unwrap();
        let (partial, _) = partial_paths(&dest, None);
        fs::write(&partial, b"old").unwrap();

        let mut opts = SyncOptions::default();
        opts.partial = true;
        let mut recv = Receiver::new(None, opts);

        let delta = vec![
            Ok(Op::Copy { offset: 0, len: 3 }),
            Ok(Op::Data(b"!".to_vec())),
        ];
        recv.apply(&src, &dest, Path::new(""), delta).unwrap();

        let output = fs::read(&dest).unwrap();
        assert_eq!(output, b"old!");
        assert!(!partial.exists());
    }
}
