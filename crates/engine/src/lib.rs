// crates/engine/src/lib.rs
#![doc = include_str!("../../../docs/crates/engine/lib.md")]
#![allow(clippy::collapsible_if)]

use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

use checksums::ChecksumConfig;
use filters::ParseError;
use thiserror::Error;

mod cleanup;
pub use cleanup::fuzzy_match;
mod delta;
mod receiver;
pub mod remote;
mod sender;

pub mod batch;
pub mod block;
pub mod flist;
pub mod io;
pub mod session;
pub mod xattrs;

pub use batch::{Batch, decode_batch, encode_batch};
pub use block::block_size;
pub use io::{io_context, is_device, preallocate};
pub use session::{DeleteMode, IdMapper, Stats, SyncOptions, pipe_sessions, select_codec, sync};

pub use checksums::StrongHash;
pub use delta::{DeltaIter, Op, compute_delta};
pub use meta::MetaOpts;
pub use receiver::{Receiver, ReceiverState};
pub use remote::{PathSpec, RemoteSpec, is_remote_spec, parse_remote_spec};
pub use sender::{Sender, SenderState};
pub const META_OPTS: MetaOpts = meta::META_OPTS;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("max-alloc limit exceeded")]
    MaxAlloc,
    #[error("{1}")]
    Exit(protocol::ExitCode, String),
    #[error("partial file missing: {0:?}")]
    MissingPartial(PathBuf),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
impl From<ParseError> for EngineError {
    fn from(e: ParseError) -> Self {
        EngineError::Other(format!("{:?}", e))
    }
}

pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

fn ensure_max_alloc(len: u64, opts: &SyncOptions) -> Result<()> {
    if opts.max_alloc != 0 && len > opts.max_alloc as u64 {
        Err(EngineError::MaxAlloc)
    } else {
        Ok(())
    }
}

fn last_good_block(
    cfg: &ChecksumConfig,
    src: &Path,
    dst: &Path,
    block_size: usize,
    opts: &SyncOptions,
) -> Result<u64> {
    let block_size = block_size.max(1);
    ensure_max_alloc(block_size as u64, opts)?;
    let mut src = match cleanup::open_for_read(src, opts) {
        Ok(f) => f,
        Err(_) => return Ok(0),
    };
    let mut dst = match cleanup::open_for_read(dst, opts) {
        Ok(f) => f,
        Err(_) => return Ok(0),
    };
    let mut offset = 0u64;
    let mut src_buf = vec![0u8; block_size];
    let mut dst_buf = vec![0u8; block_size];
    while let Ok(rs) = src.read(&mut src_buf) {
        let rd = match dst.read(&mut dst_buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        let n = rs.min(rd);
        if n == 0 {
            break;
        }
        let src_sum = cfg.checksum(&src_buf[..n]).strong;
        let dst_sum = cfg.checksum(&dst_buf[..n]).strong;
        if src_sum != dst_sum {
            break;
        }
        offset += n as u64;
        if n < block_size {
            break;
        }
    }
    Ok(offset - (offset % block_size as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{LIT_CAP, apply_delta};
    use checksums::ChecksumConfigBuilder;
    use filters::Matcher;
    use std::io::{Cursor, Write};
    use tempfile::NamedTempFile;

    fn mem_usage_kb() -> u64 {
        let status = std::fs::read_to_string("/proc/self/status").unwrap();
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                return parts[1].parse().unwrap();
            }
        }
        0
    }

    #[test]
    fn large_file_windowed_delta_memory() {
        let cfg = ChecksumConfigBuilder::new().build();
        let block_size = 1024;
        let window = 64;
        let data = vec![42u8; block_size * 1024];
        let mut basis = Cursor::new(data.clone());
        let mut target = Cursor::new(data.clone());

        let before = mem_usage_kb();
        let delta = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            block_size,
            window,
            &SyncOptions::default(),
        )
        .unwrap();
        let after = mem_usage_kb();
        let mut basis = Cursor::new(data.clone());
        let mut out = Cursor::new(Vec::new());
        let mut progress = None;
        apply_delta(
            &mut basis,
            delta,
            &mut out,
            &SyncOptions::default(),
            0,
            &mut progress,
        )
        .unwrap();
        assert_eq!(out.into_inner(), data);
        let growth = after.saturating_sub(before);
        assert!(growth < 10 * 1024, "memory grew too much: {}KB", growth);
    }

    #[test]
    fn literal_chunks_respect_cap() {
        let cfg = ChecksumConfigBuilder::new().build();
        let data = vec![42u8; LIT_CAP * 3 + 123];
        let mut basis = Cursor::new(Vec::new());
        let mut target = Cursor::new(data.clone());
        let ops: Vec<Op> = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();
        assert!(ops.iter().all(|op| match op {
            Op::Data(d) => d.len() <= LIT_CAP,
            _ => false,
        }));
    }

    #[test]
    fn large_file_strong_checksum_matches() {
        let mut tmp = NamedTempFile::new().unwrap();
        let chunk = [0u8; 1024];
        for _ in 0..(11 * 1024) {
            tmp.write_all(&chunk).unwrap();
        }
        let path = tmp.path().to_path_buf();
        let sender = Sender::new(Matcher::default(), None, SyncOptions::default());
        let new_sum = sender.strong_file_checksum(&path).unwrap();
        let data = std::fs::read(&path).unwrap();
        let old_sum = sender.cfg.checksum(&data).strong;
        assert_eq!(new_sum, old_sum);
    }

    #[test]
    fn block_size_stats_literal_matches() {
        let cfg = ChecksumConfigBuilder::new().build();
        let block_size = 2048usize;
        let len = block_size * 4;
        let mut basis = vec![0u8; len];
        for (i, b) in basis.iter_mut().enumerate().take(len) {
            *b = (i % 256) as u8;
        }
        let mut target = basis.clone();
        let off = len / 2;
        target[off..off + block_size].fill(0xAB);

        let mut basis_f = Cursor::new(basis);
        let mut target_f = Cursor::new(target);
        let ops: Vec<Op> = compute_delta(
            &cfg,
            &mut basis_f,
            &mut target_f,
            block_size,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();

        let mut stats = Stats::default();
        for op in ops {
            match op {
                Op::Data(d) => stats.literal_data += d.len() as u64,
                Op::Copy { len, .. } => stats.matched_data += len as u64,
            }
        }

        assert_eq!(stats.literal_data, block_size as u64);
    }
}
