use std::collections::{HashMap, VecDeque};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;

pub use checksums::StrongHash;
use checksums::{ChecksumConfig, ChecksumConfigBuilder};
#[cfg(feature = "lz4")]
use compress::Lz4;
use compress::{available_codecs, Codec, Compressor, Decompressor, Zlib, Zstd};
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

/// Iterator over delta operations produced by [`compute_delta`].
pub struct DeltaIter<'a, R: Read + Seek> {
    cfg: &'a ChecksumConfig,
    target: &'a mut R,
    block_size: usize,
    map: HashMap<u32, Vec<(Vec<u8>, usize, usize)>>,
    lit: Vec<u8>,
    window: Vec<u8>,
    byte: [u8; 1],
    done: bool,
}

impl<'a, R: Read + Seek> Iterator for DeltaIter<'a, R> {
    type Item = Result<Op>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.window.is_empty() {
                if self.done {
                    if self.lit.is_empty() {
                        return None;
                    } else {
                        return Some(Ok(Op::Data(std::mem::take(&mut self.lit))));
                    }
                }
                while self.window.len() < self.block_size {
                    match self.target.read(&mut self.byte) {
                        Ok(0) => {
                            self.done = true;
                            break;
                        }
                        Ok(_) => self.window.push(self.byte[0]),
                        Err(e) => return Some(Err(e.into())),
                    }
                }
                if self.window.is_empty() && self.done {
                    if self.lit.is_empty() {
                        return None;
                    } else {
                        return Some(Ok(Op::Data(std::mem::take(&mut self.lit))));
                    }
                }
            }

            let len = usize::min(self.window.len(), self.block_size);
            let sum = self.cfg.checksum(&self.window[..len]);
            if let Some(candidates) = self.map.get(&sum.weak) {
                if let Some((_, off, blen)) = candidates
                    .iter()
                    .find(|(s, _, l)| *s == sum.strong && *l == len)
                {
                    if !self.lit.is_empty() {
                        return Some(Ok(Op::Data(std::mem::take(&mut self.lit))));
                    }
                    self.window.drain(..len);
                    return Some(Ok(Op::Copy {
                        offset: *off,
                        len: *blen,
                    }));
                }
            }

            // No match: emit first byte as literal and slide the window.
            self.lit.push(self.window.remove(0));
            if self.done && self.window.is_empty() {
                return Some(Ok(Op::Data(std::mem::take(&mut self.lit))));
            }
        }
    }
}

/// Compute a delta from `basis` to `target` using a simple block matching
/// algorithm driven by the checksum crate. The computation is performed using
/// streaming readers to avoid loading entire files into memory. The caller can
/// limit memory usage by constraining the number of blocks from the basis file
/// that are kept in memory at any given time via `basis_window`.
pub fn compute_delta<'a, R1: Read + Seek, R2: Read + Seek>(
    cfg: &'a ChecksumConfig,
    basis: &mut R1,
    target: &'a mut R2,
    block_size: usize,
    basis_window: usize,
) -> Result<DeltaIter<'a, R2>> {
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

    Ok(DeltaIter {
        cfg,
        target,
        block_size,
        map,
        lit: Vec::new(),
        window: Vec::new(),
        byte: [0u8; 1],
        done: false,
    })
}

/// Apply a delta to `basis` writing the reconstructed data into `out`.
fn apply_op<R: Read + Seek, W: Write + Seek>(
    basis: &mut R,
    op: Op,
    out: &mut W,
    opts: &SyncOptions,
    buf: &mut [u8],
) -> Result<()> {
    match op {
        Op::Data(d) => {
            if opts.sparse {
                let mut i = 0;
                while i < d.len() {
                    if d[i] == 0 {
                        let mut j = i;
                        while j < d.len() && d[j] == 0 {
                            j += 1;
                        }
                        out.seek(SeekFrom::Current((j - i) as i64))?;
                        i = j;
                    } else {
                        let mut j = i;
                        while j < d.len() && d[j] != 0 {
                            j += 1;
                        }
                        out.write_all(&d[i..j])?;
                        i = j;
                    }
                }
            } else {
                out.write_all(&d)?;
            }
        }
        Op::Copy { offset, len } => {
            basis.seek(SeekFrom::Start(offset as u64))?;
            let mut remaining = len;
            while remaining > 0 {
                let to_read = remaining.min(buf.len());
                basis.read_exact(&mut buf[..to_read])?;
                out.write_all(&buf[..to_read])?;
                remaining -= to_read;
            }
        }
    }
    Ok(())
}

/// Apply a delta to `basis` writing the reconstructed data into `out`.
fn apply_delta<R: Read + Seek, W: Write + Seek, I>(
    basis: &mut R,
    ops: I,
    out: &mut W,
    opts: &SyncOptions,
) -> Result<()>
where
    I: IntoIterator<Item = Result<Op>>,
{
    let mut buf = vec![0u8; 8192];
    for op in ops {
        let op = op?;
        apply_op(basis, op, out, opts, &mut buf)?;
    }
    Ok(())
}

/// Sender responsible for walking the source tree and generating deltas.
pub struct Sender {
    state: SenderState,
    cfg: ChecksumConfig,
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
            cfg: ChecksumConfigBuilder::new().strong(opts.strong).build(),
            block_size,
            _matcher: matcher,
            codec,
            opts,
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
        if self.opts.checksum {
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
        let delta = compute_delta(
            &self.cfg,
            &mut basis_reader,
            &mut src_reader,
            self.block_size,
            DEFAULT_BASIS_WINDOW,
        )?;
        let ops = delta.map(|op_res| {
            let mut op = op_res?;
            if let Some(codec) = self.codec {
                if let Op::Data(ref mut d) = op {
                    *d = match codec {
                        Codec::Zlib => {
                            let lvl = self.opts.compress_level.unwrap_or(6);
                            Zlib::new(lvl).compress(d).map_err(EngineError::from)?
                        }
                        Codec::Zstd => {
                            let lvl = self.opts.compress_level.unwrap_or(0);
                            Zstd::new(lvl).compress(d).map_err(EngineError::from)?
                        }
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
            Ok(op)
        });
        recv.apply(path, dest, ops)?;
        Ok(true)
    }
}

/// Receiver responsible for applying deltas to the destination tree.
pub struct Receiver {
    state: ReceiverState,
    codec: Option<Codec>,
    opts: SyncOptions,
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
        }
    }

    fn apply<I>(&mut self, src: &Path, dest: &Path, delta: I) -> Result<()>
    where
        I: IntoIterator<Item = Result<Op>>,
    {
        self.state = ReceiverState::Applying;
        let mut basis: Box<dyn ReadSeek> = match File::open(dest) {
            Ok(f) => Box::new(f),
            Err(_) => Box::new(Cursor::new(Vec::new())),
        };
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = BufWriter::new(File::create(dest)?);
        let ops = delta.into_iter().map(|op_res| {
            let mut op = op_res?;
            if let Some(codec) = self.codec {
                if let Op::Data(ref mut d) = op {
                    *d = match codec {
                        Codec::Zlib => Zlib::default().decompress(d).map_err(EngineError::from)?,
                        Codec::Zstd => Zstd::default().decompress(d).map_err(EngineError::from)?,
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
            Ok(op)
        });
        apply_delta(&mut basis, ops, &mut out, &self.opts)?;
        out.flush()?;
        self.copy_metadata(src, dest)?;
        self.state = ReceiverState::Finished;
        Ok(())
    }
}

impl Receiver {
    fn copy_metadata(&self, src: &Path, dest: &Path) -> Result<()> {
        let meta = fs::symlink_metadata(src)?;
        if self.opts.perms {
            fs::set_permissions(dest, meta.permissions())?;
        }
        if self.opts.times {
            #[cfg(unix)]
            {
                use filetime::{set_file_times, FileTime};
                let mtime = FileTime::from_last_modification_time(&meta);
                let atime = FileTime::from_last_access_time(&meta);
                set_file_times(dest, atime, mtime).map_err(EngineError::from)?;
            }
        }
        #[cfg(unix)]
        {
            if self.opts.owner || self.opts.group {
                use nix::unistd::{chown, Gid, Uid};
                let uid = if self.opts.owner {
                    Some(Uid::from_raw(meta.uid()))
                } else {
                    None
                };
                let gid = if self.opts.group {
                    Some(Gid::from_raw(meta.gid()))
                } else {
                    None
                };
                chown(dest, uid, gid).map_err(|e| EngineError::Other(e.to_string()))?;
            }
            if self.opts.xattrs || self.opts.acls {
                let attrs = xattr::list(src).map_err(|e| EngineError::Other(e.to_string()))?;
                for name in attrs {
                    let name_str = name.to_string_lossy();
                    if !self.opts.acls && name_str.starts_with("system.posix_acl") {
                        continue;
                    }
                    if let Some(val) =
                        xattr::get(src, &name).map_err(|e| EngineError::Other(e.to_string()))?
                    {
                        xattr::set(dest, &name, &val)
                            .map_err(|e| EngineError::Other(e.to_string()))?;
                    }
                }
            }
        }
        Ok(())
    }
}

/// Options controlling synchronization behavior.
#[derive(Debug, Clone, Copy)]
pub struct SyncOptions {
    pub delete: bool,
    pub checksum: bool,
    pub compress: bool,
    pub perms: bool,
    pub times: bool,
    pub owner: bool,
    pub group: bool,
    pub links: bool,
    pub hard_links: bool,
    pub devices: bool,
    pub specials: bool,
    pub xattrs: bool,
    pub acls: bool,
    pub sparse: bool,
    pub strong: StrongHash,
    pub compress_level: Option<i32>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            delete: false,
            checksum: false,
            compress: false,
            perms: false,
            times: false,
            owner: false,
            group: false,
            links: false,
            hard_links: false,
            devices: false,
            specials: false,
            xattrs: false,
            acls: false,
            sparse: false,
            strong: StrongHash::Md5,
            compress_level: None,
        }
    }
}

/// Statistics produced during synchronization.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub files_transferred: usize,
    pub files_deleted: usize,
    pub bytes_transferred: u64,
}

/// Choose the compression codec to use based on local preferences and remote support.
pub fn select_codec(remote: &[Codec], opts: &SyncOptions) -> Option<Codec> {
    if !opts.compress || opts.compress_level == Some(0) {
        return None;
    }
    if remote.contains(&Codec::Zstd) && available_codecs().contains(&Codec::Zstd) {
        Some(Codec::Zstd)
    } else if remote.contains(&Codec::Zlib) && available_codecs().contains(&Codec::Zlib) {
        Some(Codec::Zlib)
    } else {
        None
    }
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
    let codec = select_codec(remote, opts);
    // Clone the matcher and attach the source root so per-directory filter files
    // can be located during the walk.
    let matcher = matcher.clone().with_root(src.to_path_buf());
    let mut sender = Sender::new(1024, matcher.clone(), codec, *opts);
    let mut receiver = Receiver::new(codec, *opts);
    let mut stats = Stats::default();
    sender.start();
    #[cfg(unix)]
    let mut hard_links: HashMap<(u64, u64), std::path::PathBuf> = HashMap::new();
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
                #[cfg(unix)]
                if opts.hard_links {
                    use std::os::unix::fs::MetadataExt;
                    let meta = fs::metadata(&path)?;
                    let key = (meta.dev(), meta.ino());
                    if let Some(existing) = hard_links.get(&key) {
                        fs::hard_link(existing, &dest_path)?;
                        continue;
                    } else {
                        hard_links.insert(key, dest_path.clone());
                    }
                }
                if sender.process_file(&path, &dest_path, &mut receiver)? {
                    stats.files_transferred += 1;
                    stats.bytes_transferred += fs::metadata(&path)?.len();
                }
            } else if file_type.is_dir() {
                fs::create_dir_all(&dest_path)?;
            } else if file_type.is_symlink() && opts.links {
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
            } else {
                #[cfg(unix)]
                {
                    if (file_type.is_char_device() || file_type.is_block_device()) && opts.devices {
                        use nix::sys::stat::{mknod, Mode, SFlag};
                        let meta = fs::symlink_metadata(&path)?;
                        let kind = if file_type.is_char_device() {
                            SFlag::S_IFCHR
                        } else {
                            SFlag::S_IFBLK
                        };
                        let perm = Mode::from_bits_truncate(meta.mode() & 0o777);
                        mknod(&dest_path, kind, perm, meta.rdev())
                            .map_err(|e| EngineError::Other(e.to_string()))?;
                    } else if file_type.is_fifo() && opts.specials {
                        use nix::sys::stat::Mode;
                        use nix::unistd::mkfifo;
                        mkfifo(&dest_path, Mode::from_bits_truncate(0o644))
                            .map_err(|e| EngineError::Other(e.to_string()))?;
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
        let mut out = Cursor::new(Vec::new());
        apply_delta(&mut basis, delta, &mut out, &SyncOptions::default()).unwrap();
        assert_eq!(out.into_inner(), b"hello brave new world");
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
        let delta: Vec<Op> =
            compute_delta(&cfg, &mut basis_reader, &mut target_reader, 3, usize::MAX)
                .unwrap()
                .collect::<Result<_>>()
                .unwrap();
        assert_eq!(
            delta,
            vec![
                Op::Copy { offset: 0, len: 3 },
                Op::Copy { offset: 3, len: 3 },
            ]
        );
        let mut basis_reader = Cursor::new(basis.clone());
        let mut out = Cursor::new(Vec::new());
        apply_delta(
            &mut basis_reader,
            delta.into_iter().map(Ok),
            &mut out,
            &SyncOptions::default(),
        )
        .unwrap();
        assert_eq!(out.into_inner(), basis);
    }

    #[test]
    fn emits_literal_for_new_data() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(Vec::new());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta: Vec<Op> = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX)
            .unwrap()
            .collect::<Result<_>>()
            .unwrap();
        assert_eq!(delta, vec![Op::Data(b"abc".to_vec())]);
    }

    #[test]
    fn empty_target_yields_no_ops() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(Vec::new());
        let delta: Vec<Op> = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX)
            .unwrap()
            .collect::<Result<_>>()
            .unwrap();
        assert!(delta.is_empty());
    }

    #[test]
    fn small_basis_matches() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"abc".to_vec());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta: Vec<Op> = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX)
            .unwrap()
            .collect::<Result<_>>()
            .unwrap();
        assert_eq!(delta, vec![Op::Copy { offset: 0, len: 3 }]);
    }

    #[test]
    fn matches_partial_blocks() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(b"hello".to_vec());
        let delta: Vec<Op> = compute_delta(&cfg, &mut basis, &mut target, 4, usize::MAX)
            .unwrap()
            .collect::<Result<_>>()
            .unwrap();
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

        let mut sender = Sender::new(
            1024,
            Matcher::default(),
            Some(Codec::Zlib),
            SyncOptions::default(),
        );
        let mut receiver = Receiver::new(Some(Codec::Zlib), SyncOptions::default());
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
        let mut out = Cursor::new(Vec::new());
        apply_delta(&mut basis, delta, &mut out, &SyncOptions::default()).unwrap();
        assert_eq!(out.into_inner(), data);
        // Memory usage should stay under ~10MB
        assert!(
            after - before < 10 * 1024,
            "memory grew too much: {}KB",
            after - before
        );
    }
}
