// crates/engine/src/receiver/apply.rs
#[cfg(unix)]
use nix::unistd::{Gid, Uid, chown};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Cursor, Seek, SeekFrom};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use compress::{Codec, Decompressor, ZlibX, Zstd, should_compress};

use crate::block::block_size;
use crate::cleanup::{
    TempFileGuard, atomic_rename, open_for_read, partial_paths, remove_basename_partial,
    tmp_file_path,
};
use crate::delta::{Op, Progress, apply_delta};
use crate::io::{io_context, is_device, preallocate};
use crate::{EngineError, ReadSeek, Result, ensure_max_alloc, last_good_block};
use checksums::ChecksumConfigBuilder;

use super::{Receiver, ReceiverState};

impl Receiver {
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
