// crates/engine/src/sender.rs

use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::time::Duration;

use checksums::{ChecksumConfig, ChecksumConfigBuilder};
use compress::{Codec, Compressor, Zlib, Zstd, should_compress};
use filters::Matcher;
use md4::{Digest, Md4};
use md5::Md5;
use sha1::Sha1;
use xxhash_rust::xxh64::Xxh64;

use crate::cleanup::{atomic_rename, fuzzy_match, open_for_read, partial_paths};
use crate::delta::{DEFAULT_BASIS_WINDOW, Op, compute_delta};
use crate::receiver::Receiver;
use crate::{
    EngineError, ReadSeek, Result, Stats, StrongHash, SyncOptions, block_size, ensure_max_alloc,
    io_context, is_device, last_good_block,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderState {
    Idle,
    Walking,
    Finished,
}

pub struct Sender {
    state: SenderState,
    pub(crate) cfg: ChecksumConfig,
    block_size: usize,
    _matcher: Matcher,
    codec: Option<Codec>,
    opts: SyncOptions,
}

impl Sender {
    pub fn new(
        block_size: usize,
        matcher: Matcher,
        codec: Option<Codec>,
        opts: SyncOptions,
    ) -> Self {
        Self {
            state: SenderState::Idle,
            cfg: ChecksumConfigBuilder::new()
                .strong(opts.strong)
                .seed(opts.checksum_seed)
                .build(),
            block_size,
            _matcher: matcher,
            codec,
            opts,
        }
    }

    pub(crate) fn strong_file_checksum(&self, path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path).map_err(|e| io_context(path, e))?;
        let mut reader = BufReader::new(file);
        let mut buf = [0u8; 8192];
        match self.opts.strong {
            StrongHash::Md4 => {
                let mut hasher = Md4::new();
                loop {
                    let n = reader.read(&mut buf).map_err(|e| io_context(path, e))?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                hasher.update(self.opts.checksum_seed.to_le_bytes());
                Ok(hasher.finalize().to_vec())
            }
            StrongHash::Md5 => {
                let mut hasher = Md5::new();
                loop {
                    let n = reader.read(&mut buf).map_err(|e| io_context(path, e))?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                Ok(hasher.finalize().to_vec())
            }
            StrongHash::Sha1 => {
                let mut hasher = Sha1::new();
                loop {
                    let n = reader.read(&mut buf).map_err(|e| io_context(path, e))?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                Ok(hasher.finalize().to_vec())
            }
            StrongHash::XxHash => {
                let mut hasher = Xxh64::new(self.opts.checksum_seed as u64);
                loop {
                    let n = reader.read(&mut buf).map_err(|e| io_context(path, e))?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                Ok(hasher.digest().to_le_bytes().to_vec())
            }
        }
    }

    fn metadata_unchanged(&self, path: &Path, dest: &Path) -> bool {
        if self.opts.size_only {
            if let (Ok(src_meta), Ok(dst_meta)) = (fs::metadata(path), fs::metadata(dest)) {
                return src_meta.len() == dst_meta.len();
            }
            return false;
        }
        if self.opts.ignore_times {
            return false;
        }
        if let (Ok(src_meta), Ok(dst_meta)) = (fs::metadata(path), fs::metadata(dest)) {
            if src_meta.len() == dst_meta.len() {
                if let (Ok(sm), Ok(dm)) = (src_meta.modified(), dst_meta.modified()) {
                    let diff = if sm > dm {
                        sm.duration_since(dm).unwrap_or(Duration::ZERO)
                    } else {
                        dm.duration_since(sm).unwrap_or(Duration::ZERO)
                    };
                    return diff <= self.opts.modify_window;
                }
            }
        }
        false
    }

    pub(crate) fn start(&mut self) {
        self.state = SenderState::Walking;
    }

    pub(crate) fn finish(&mut self) {
        self.state = SenderState::Finished;
    }

    pub(crate) fn process_file(
        &mut self,
        path: &Path,
        dest: &Path,
        rel: &Path,
        recv: &mut Receiver,
        stats: &mut Stats,
    ) -> Result<bool> {
        let mut dest = dest.to_path_buf();
        if dest.is_dir() {
            if let Some(name) = path.file_name() {
                dest.push(name);
            }
        }
        if self.opts.checksum {
            if let Ok(dst_sum) = self.strong_file_checksum(&dest) {
                let src_sum = self.strong_file_checksum(path)?;
                if src_sum == dst_sum {
                    recv.copy_metadata(path, &dest)?;
                    return Ok(false);
                }
            } else if self.metadata_unchanged(path, &dest) {
                recv.copy_metadata(path, &dest)?;
                return Ok(false);
            }
        } else if self.metadata_unchanged(path, &dest) {
            recv.copy_metadata(path, &dest)?;
            return Ok(false);
        }

        let meta = fs::metadata(path).map_err(|e| io_context(path, e))?;
        let src_len = meta.len();
        ensure_max_alloc(src_len, &self.opts)?;
        let block_size = if self.block_size == 0 {
            block_size(src_len)
        } else {
            self.block_size
        };
        let file_type = meta.file_type();
        let atime_guard = if self.opts.atimes {
            meta::AccessTime::new(path).ok()
        } else {
            None
        };
        let src = open_for_read(path, &self.opts).map_err(|e| io_context(path, e))?;
        let mut src_reader = BufReader::new(src);
        let file_codec = if should_compress(path, &self.opts.skip_compress) {
            self.codec
        } else {
            None
        };
        let (partial_path, basename_partial) =
            partial_paths(&dest, self.opts.partial_dir.as_deref());
        let existing_partial = if partial_path.exists() {
            Some(partial_path.clone())
        } else if let Some(bp) = basename_partial.as_ref() {
            if bp.exists() { Some(bp.clone()) } else { None }
        } else {
            None
        };
        let basis_path = if (self.opts.partial || self.opts.append || self.opts.append_verify)
            && existing_partial.is_some()
        {
            existing_partial.clone().unwrap()
        } else if self.opts.fuzzy && !dest.exists() {
            fuzzy_match(&dest).unwrap_or_else(|| dest.clone())
        } else {
            dest.clone()
        };
        let mut resume = if self.opts.partial || self.opts.append || self.opts.append_verify {
            if self.opts.append && !self.opts.append_verify {
                fs::metadata(&basis_path).map(|m| m.len()).unwrap_or(0)
            } else {
                last_good_block(&self.cfg, path, &basis_path, block_size, &self.opts)?
            }
        } else {
            0
        };
        if resume > src_len {
            resume = src_len;
        }
        let mut basis_reader: Box<dyn ReadSeek> = if self.opts.whole_file {
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
        };
        let delta: Box<dyn Iterator<Item = Result<Op>> + '_> =
            if self.opts.copy_devices && is_device(&file_type) && src_len == 0 {
                Box::new(std::iter::empty())
            } else if self.opts.whole_file {
                ensure_max_alloc(block_size.max(8192) as u64, &self.opts)?;
                let mut buf = vec![0u8; block_size.max(8192)];
                Box::new(std::iter::from_fn(move || {
                    match src_reader.read(&mut buf) {
                        Ok(0) => None,
                        Ok(n) => Some(Ok(Op::Data(buf[..n].to_vec()))),
                        Err(e) => Some(Err(e.into())),
                    }
                }))
            } else {
                Box::new(compute_delta(
                    &self.cfg,
                    &mut basis_reader,
                    &mut src_reader,
                    block_size,
                    DEFAULT_BASIS_WINDOW,
                    &self.opts,
                )?)
            };
        if self.opts.backup && dest.exists() {
            let backup_path = if let Some(ref dir) = self.opts.backup_dir {
                let mut p = dir.join(rel);
                if !self.opts.backup_suffix.is_empty() {
                    if let Some(name) = p.file_name() {
                        p = p.with_file_name(format!(
                            "{}{}",
                            name.to_string_lossy(),
                            &self.opts.backup_suffix
                        ));
                    } else {
                        p.push(&self.opts.backup_suffix);
                    }
                }
                p
            } else {
                let name = dest
                    .file_name()
                    .map(|n| format!("{}{}", n.to_string_lossy(), &self.opts.backup_suffix))
                    .unwrap_or_else(|| self.opts.backup_suffix.clone());
                dest.with_file_name(name)
            };
            if let Some(parent) = backup_path.parent() {
                fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
            }
            atomic_rename(&dest, &backup_path)?;
        }
        let mut skip = resume as u64;
        let mut literal = 0u64;
        let mut matched = 0u64;
        let adjusted = delta.filter_map(move |op_res| match op_res {
            Ok(op) => {
                if skip == 0 {
                    return Some(Ok(op));
                }
                match op {
                    Op::Data(d) => {
                        if (skip as usize) >= d.len() {
                            skip -= d.len() as u64;
                            None
                        } else {
                            let start = skip as usize;
                            skip = 0;
                            Some(Ok(Op::Data(d[start..].to_vec())))
                        }
                    }
                    Op::Copy { offset, len } => {
                        if (skip as usize) >= len {
                            skip -= len as u64;
                            None
                        } else {
                            let start = skip as usize;
                            skip = 0;
                            Some(Ok(Op::Copy {
                                offset: offset + start,
                                len: len - start,
                            }))
                        }
                    }
                }
            }
            Err(e) => Some(Err(e)),
        });
        let adjusted = adjusted.inspect(|op_res| {
            if let Ok(op) = op_res {
                match op {
                    Op::Data(d) => literal += d.len() as u64,
                    Op::Copy { len, .. } => matched += *len as u64,
                }
            }
        });
        let ops = adjusted.map(|op_res| {
            let mut op = op_res?;
            if let Some(codec) = file_codec {
                if let Op::Data(ref mut d) = op {
                    *d = match codec {
                        Codec::Zlib | Codec::ZlibX => {
                            let lvl = self.opts.compress_level.unwrap_or(6);
                            let mut out = Vec::new();
                            let mut cursor = d.as_slice();
                            Zlib::new(lvl)
                                .compress(&mut cursor, &mut out)
                                .map_err(EngineError::from)?;
                            out
                        }
                        Codec::Zstd => {
                            let lvl = self.opts.compress_level.unwrap_or(0);
                            let mut out = Vec::new();
                            let mut cursor = d.as_slice();
                            Zstd::new(lvl)
                                .compress(&mut cursor, &mut out)
                                .map_err(EngineError::from)?;
                            out
                        }
                    };
                }
            }
            Ok(op)
        });
        if !self.opts.only_write_batch {
            recv.apply(path, &dest, rel, ops)?;
            stats.literal_data += literal;
            stats.matched_data += matched;
            drop(atime_guard);
            recv.copy_metadata(path, &dest)?;
        } else {
            drop(atime_guard);
            for op in ops {
                let _ = op;
            }
            stats.literal_data += literal;
            stats.matched_data += matched;
        }
        Ok(true)
    }
}
