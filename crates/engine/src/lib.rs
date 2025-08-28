use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
use std::path::Path;

use checksums::{ChecksumConfig, ChecksumConfigBuilder};
#[cfg(feature = "lz4")]
use compress::Lz4;
use compress::{available_codecs, negotiate_codec, Codec, Compressor, Decompressor, Zlib, Zstd};
use filters::Matcher;
use thiserror::Error;

/// Error type for engine operations.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

/// Result type for engine operations.
pub type Result<T> = std::result::Result<T, EngineError>;
use walk::walk;

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

/// Sender state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderState {
    Idle,
    Walking,
    Finished,
}

/// Receiver state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverState {
    Idle,
    Applying,
    Finished,
}

/// Operation in a file delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Raw bytes to be written.
    Data(Vec<u8>),
    /// Copy bytes from an existing offset in the basis file.
    Copy { offset: usize, len: usize },
}

/// Default number of blocks from the basis file to keep indexed when
/// computing a delta. This bounds memory usage to roughly
/// `DEFAULT_BASIS_WINDOW * block_size` bytes.
const DEFAULT_BASIS_WINDOW: usize = 8 * 1024; // 8k blocks

/// Compute a delta from `basis` to `target` using a simple block matching
/// algorithm driven by the checksum crate. The computation is performed using
/// streaming readers to avoid loading entire files into memory. The caller can
/// limit memory usage by constraining the number of blocks from the basis file
/// that are kept in memory at any given time via `basis_window`.
pub fn compute_delta<R1: Read + Seek, R2: Read + Seek>(
    cfg: &ChecksumConfig,
    basis: &mut R1,
    target: &mut R2,
    block_size: usize,
    basis_window: usize,
) -> Result<Vec<Op>> {
    // Start from the beginning of both streams.
    basis.seek(SeekFrom::Start(0))?;
    target.seek(SeekFrom::Start(0))?;
    // Build a map of rolling checksum -> (strong hash, offset, len) for the
    // basis file. Only the most recent `basis_window` blocks are kept to bound
    // memory usage.
    let mut map: HashMap<u32, Vec<(Vec<u8>, usize, usize)>> = HashMap::new();
    let mut order: VecDeque<(u32, Vec<u8>, usize, usize)> = VecDeque::new();
    let mut off = 0usize;
    let mut buf = vec![0u8; block_size];
    loop {
        let n = basis.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let sum = cfg.checksum(&buf[..n]);
        map.entry(sum.weak)
            .or_default()
            .push((sum.strong.clone(), off, n));
        order.push_back((sum.weak, sum.strong, off, n));
        if order.len() > basis_window {
            if let Some((w, s, o, l)) = order.pop_front() {
                if let Some(v) = map.get_mut(&w) {
                    if let Some(pos) = v
                        .iter()
                        .position(|(ss, oo, ll)| *oo == o && *ll == l && *ss == s)
                    {
                        v.remove(pos);
                    }
                    if v.is_empty() {
                        map.remove(&w);
                    }
                }
            }
        }
        off += n;
        if n < block_size {
            break;
        }
    }

    let mut ops = Vec::new();
    let mut lit = Vec::new();

    let mut window = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        // Fill the window up to block_size bytes.
        while window.len() < block_size {
            let n = target.read(&mut byte)?;
            if n == 0 {
                break;
            }
            window.push(byte[0]);
        }
        if window.is_empty() {
            break;
        }

        let len = usize::min(window.len(), block_size);
        let sum = cfg.checksum(&window[..len]);
        if let Some(candidates) = map.get(&sum.weak) {
            if let Some((_, off, blen)) = candidates
                .iter()
                .find(|(s, _, l)| *s == sum.strong && *l == len)
            {
                if !lit.is_empty() {
                    ops.push(Op::Data(std::mem::take(&mut lit)));
                }
                ops.push(Op::Copy {
                    offset: *off,
                    len: *blen,
                });
                window.drain(..len);
                continue;
            }
        }

        // No match: emit first byte as literal and slide the window.
        lit.push(window.remove(0));
        if window.is_empty() {
            // if we've consumed everything, attempt to read more before next iteration
            continue;
        }
    }

    if !window.is_empty() {
        lit.extend(window);
    }
    if !lit.is_empty() {
        ops.push(Op::Data(lit));
    }
    Ok(ops)
}

/// Apply a delta to `basis` writing the reconstructed data into `out`.
fn apply_delta<R: Read + Seek, W: Write>(basis: &mut R, ops: &[Op], out: &mut W) -> Result<()> {
    let mut buf = vec![0u8; 8192];
    for op in ops {
        match op {
            Op::Data(d) => out.write_all(d)?,
            Op::Copy { offset, len } => {
                basis.seek(SeekFrom::Start(*offset as u64))?;
                let mut remaining = *len;
                while remaining > 0 {
                    let to_read = remaining.min(buf.len());
                    basis.read_exact(&mut buf[..to_read])?;
                    out.write_all(&buf[..to_read])?;
                    remaining -= to_read;
                }
            }
        }
    }
    Ok(())
}

/// Sender responsible for walking the source tree and generating deltas.
pub struct Sender {
    state: SenderState,
    cfg: ChecksumConfig,
    block_size: usize,
    matcher: Matcher,
    codec: Option<Codec>,
    checksum: bool,
}

impl Sender {
    pub fn new(block_size: usize, matcher: Matcher, codec: Option<Codec>, checksum: bool) -> Self {
        Self {
            state: SenderState::Idle,
            cfg: ChecksumConfigBuilder::new().build(),
            block_size,
            matcher,
            codec,
            checksum,
        }
    }

    fn start(&mut self) {
        self.state = SenderState::Walking;
    }

    fn finish(&mut self) {
        self.state = SenderState::Finished;
    }

    /// Generate a delta for `path` against `dest` and ask the receiver to apply it.
    /// Returns `true` if the destination file was updated.
    fn process_file(&mut self, path: &Path, dest: &Path, recv: &mut Receiver) -> Result<bool> {
        if self.checksum {
            if let Ok(dst_bytes) = fs::read(dest) {
                let src_bytes = fs::read(path)?;
                if self.cfg.checksum(&src_bytes).strong == self.cfg.checksum(&dst_bytes).strong {
                    return Ok(false);
                }
            }
        } else if let (Ok(src_meta), Ok(dst_meta)) = (fs::metadata(path), fs::metadata(dest)) {
            if src_meta.len() == dst_meta.len() {
                if let (Ok(sm), Ok(dm)) = (src_meta.modified(), dst_meta.modified()) {
                    if sm == dm {
                        return Ok(false);
                    }
                }
            }
        }

        let src = File::open(path)?;
        let mut src_reader = BufReader::new(src);
        let mut basis_reader: Box<dyn ReadSeek> = match File::open(dest) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(_) => Box::new(Cursor::new(Vec::new())),
        };
        let mut delta = compute_delta(
            &self.cfg,
            &mut basis_reader,
            &mut src_reader,
            self.block_size,
            DEFAULT_BASIS_WINDOW,
        )?;
        if let Some(codec) = self.codec {
            for op in &mut delta {
                if let Op::Data(ref mut d) = op {
                    *d = match codec {
                        Codec::Zlib => Zlib.compress(d).map_err(EngineError::from)?,
                        Codec::Zstd => Zstd.compress(d).map_err(EngineError::from)?,
                        Codec::Lz4 => {
                            #[cfg(feature = "lz4")]
                            {
                                Lz4.compress(d).map_err(EngineError::from)?
                            }
                            #[cfg(not(feature = "lz4"))]
                            {
                                return Err(EngineError::Other("LZ4 feature not enabled".into()));
                            }
                        }
                    };
                }
            }
        }
        recv.apply(dest, delta)?;
        Ok(true)
    }
}

/// Receiver responsible for applying deltas to the destination tree.
pub struct Receiver {
    state: ReceiverState,
    codec: Option<Codec>,
}

impl Default for Receiver {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Receiver {
    pub fn new(codec: Option<Codec>) -> Self {
        Self {
            state: ReceiverState::Idle,
            codec,
        }
    }

    fn apply(&mut self, dest: &Path, mut delta: Vec<Op>) -> Result<()> {
        self.state = ReceiverState::Applying;
        let mut basis: Box<dyn ReadSeek> = match File::open(dest) {
            Ok(f) => Box::new(f),
            Err(_) => Box::new(Cursor::new(Vec::new())),
        };
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = BufWriter::new(File::create(dest)?);
        if let Some(codec) = self.codec {
            for op in &mut delta {
                if let Op::Data(ref mut d) = op {
                    *d = match codec {
                        Codec::Zlib => Zlib.decompress(d).map_err(EngineError::from)?,
                        Codec::Zstd => Zstd.decompress(d).map_err(EngineError::from)?,
                        Codec::Lz4 => {
                            #[cfg(feature = "lz4")]
                            {
                                Lz4.decompress(d).map_err(EngineError::from)?
                            }
                            #[cfg(not(feature = "lz4"))]
                            {
                                return Err(EngineError::Other("LZ4 feature not enabled".into()));
                            }
                        }
                    };
                }
            }
        }
        apply_delta(&mut basis, &delta, &mut out)?;
        out.flush()?;
        self.state = ReceiverState::Finished;
        Ok(())
    }
}

/// Options controlling synchronization behavior.
#[derive(Debug, Clone, Copy, Default)]
pub struct SyncOptions {
    pub delete: bool,
    pub checksum: bool,
    pub compress: bool,
}

/// Statistics produced during synchronization.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub files_transferred: usize,
    pub files_deleted: usize,
    pub bytes_transferred: u64,
}

/// Synchronize the contents of directory `src` into `dst`.
pub fn sync(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    remote: &[Codec],
    opts: &SyncOptions,
) -> Result<Stats> {
    // Determine the codec to use by negotiating with the remote peer.
    let codec = if opts.compress {
        negotiate_codec(available_codecs(), remote)
    } else {
        None
    };
    // Clone the matcher and attach the source root so per-directory filter files
    // can be located during the walk.
    let matcher = matcher.clone().with_root(src.to_path_buf());
    let mut sender = Sender::new(1024, matcher.clone(), codec, opts.checksum);
    let mut receiver = Receiver::new(codec);
    let mut stats = Stats::default();
    sender.start();
    for entry in walk(src) {
        let (path, file_type) = entry.map_err(|e| EngineError::Other(e.to_string()))?;
        if let Ok(rel) = path.strip_prefix(src) {
            if !matcher
                .is_included(rel)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?
            {
                continue;
            }
            let dest_path = dst.join(rel);
            if file_type.is_file() {
                if sender.process_file(&path, &dest_path, &mut receiver)? {
                    stats.files_transferred += 1;
                    stats.bytes_transferred += fs::metadata(&path)?.len() as u64;
                }
            } else if file_type.is_dir() {
                fs::create_dir_all(&dest_path)?;
            } else if file_type.is_symlink() {
                let target = fs::read_link(&path)?;
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                #[cfg(unix)]
                std::os::unix::fs::symlink(&target, &dest_path)?;
                #[cfg(windows)]
                {
                    if target.is_dir() {
                        std::os::windows::fs::symlink_dir(&target, &dest_path)?;
                    } else {
                        std::os::windows::fs::symlink_file(&target, &dest_path)?;
                    }
                }
            }
        } else {
            // Path lies outside of the source directory, skip it.
            continue;
        }
    }
    sender.finish();
    if opts.delete {
        for entry in walk(dst) {
            let (path, file_type) = entry.map_err(|e| EngineError::Other(e.to_string()))?;
            if let Ok(rel) = path.strip_prefix(dst) {
                if !matcher
                    .is_included(rel)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?
                {
                    continue;
                }
                if !src.join(rel).exists() {
                    if file_type.is_dir() {
                        fs::remove_dir_all(&path)?;
                    } else {
                        fs::remove_file(&path)?;
                    }
                    stats.files_deleted += 1;
                }
            }
        }
    }
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use checksums::rolling_checksum;
    use tempfile::tempdir;

    #[test]
    fn delta_roundtrip() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello world".to_vec());
        let mut target = Cursor::new(b"hello brave new world".to_vec());
        let delta = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX).unwrap();
        let mut basis = Cursor::new(b"hello world".to_vec());
        let mut out = Vec::new();
        apply_delta(&mut basis, &delta, &mut out).unwrap();
        assert_eq!(out, b"hello brave new world");
    }

    #[test]
    fn weak_checksum_collision() {
        let cfg = ChecksumConfigBuilder::new().build();
        let block1 = b"\x01\x00\x01";
        let block2 = b"\x00\x02\x00";
        assert_eq!(rolling_checksum(block1), rolling_checksum(block2));
        let basis: Vec<u8> = [block1.as_ref(), block2.as_ref()].concat();
        let mut basis_reader = Cursor::new(basis.clone());
        let mut target_reader = Cursor::new(basis.clone());
        let delta =
            compute_delta(&cfg, &mut basis_reader, &mut target_reader, 3, usize::MAX).unwrap();
        assert_eq!(
            delta,
            vec![
                Op::Copy { offset: 0, len: 3 },
                Op::Copy { offset: 3, len: 3 },
            ]
        );
        let mut basis_reader = Cursor::new(basis.clone());
        let mut out = Vec::new();
        apply_delta(&mut basis_reader, &delta, &mut out).unwrap();
        assert_eq!(out, basis);
    }

    #[test]
    fn emits_literal_for_new_data() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(Vec::new());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX).unwrap();
        assert_eq!(delta, vec![Op::Data(b"abc".to_vec())]);
    }

    #[test]
    fn empty_target_yields_no_ops() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(Vec::new());
        let delta = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX).unwrap();
        assert!(delta.is_empty());
    }

    #[test]
    fn small_basis_matches() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"abc".to_vec());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX).unwrap();
        assert_eq!(delta, vec![Op::Copy { offset: 0, len: 3 }]);
    }

    #[test]
    fn matches_partial_blocks() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(b"hello".to_vec());
        let delta = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX).unwrap();
        assert_eq!(
            delta,
            vec![
                Op::Copy { offset: 0, len: 4 },
                Op::Copy { offset: 4, len: 1 },
            ]
        );
    }

    #[test]
    fn sync_dir() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("a")).unwrap();
        fs::write(src.join("a/file1.txt"), b"hello").unwrap();
        fs::write(src.join("file2.txt"), b"world").unwrap();

        sync(
            &src,
            &dst,
            &Matcher::default(),
            available_codecs(),
            &SyncOptions::default(),
        )
        .unwrap();
        assert_eq!(fs::read(dst.join("a/file1.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(dst.join("file2.txt")).unwrap(), b"world");
    }

    #[test]
    fn sync_skips_outside_paths() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("inside.txt"), b"inside").unwrap();

        // Create a file outside the source tree.
        let outside = tmp.path().join("outside.txt");
        fs::write(&outside, b"outside").unwrap();

        let mut sender = Sender::new(1024, Matcher::default(), Some(Codec::Zlib), false);
        let mut receiver = Receiver::new(Some(Codec::Zlib));
        sender.start();
        for path in [src.join("inside.txt"), outside.clone()] {
            if let Some(rel) = path.strip_prefix(&src).ok() {
                let dest_path = dst.join(rel);
                sender
                    .process_file(&path, &dest_path, &mut receiver)
                    .unwrap();
            }
        }
        sender.finish();

        assert_eq!(fs::read(dst.join("inside.txt")).unwrap(), b"inside");
        assert!(!dst.join("outside.txt").exists());
    }

    fn mem_usage_kb() -> u64 {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status").unwrap();
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
        let window = 64; // keep at most 64 blocks in memory
        let data = vec![42u8; block_size * 1024]; // 1 MiB file
        let mut basis = Cursor::new(data.clone());
        let mut target = Cursor::new(data.clone());

        let before = mem_usage_kb();
        let delta = compute_delta(&cfg, &mut basis, &mut target, block_size, window).unwrap();
        let after = mem_usage_kb();
        // delta should reconstruct target
        let mut basis = Cursor::new(data.clone());
        let mut out = Vec::new();
        apply_delta(&mut basis, &delta, &mut out).unwrap();
        assert_eq!(out, data);
        // Memory usage should stay under ~10MB
        assert!(
            after - before < 10 * 1024,
            "memory grew too much: {}KB",
            after - before
        );
    }
}
