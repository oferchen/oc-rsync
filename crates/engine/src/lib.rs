// crates/engine/src/lib.rs
#[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
use nix::fcntl::{fallocate, FallocateFlags};
#[cfg(unix)]
use nix::unistd::{chown, Gid, Uid};
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
#[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tempfile::NamedTempFile;

use checksums::{ChecksumConfig, ChecksumConfigBuilder};
pub use checksums::{ModernHash, StrongHash};
#[cfg(feature = "lz4")]
use compress::Lz4;
use compress::{
    available_codecs, should_compress, Codec, Compressor, Decompressor, ModernCompress, Zlib, Zstd,
};
use filters::Matcher;
use logging::progress_formatter;
use thiserror::Error;

pub mod cdc;
use cdc::{chunk_file, Manifest};
pub mod flist;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("max-alloc limit exceeded")]
    MaxAlloc,
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
use walk::walk;

#[derive(Clone)]
pub struct IdMapper(pub Arc<dyn Fn(u32) -> u32 + Send + Sync>);

impl std::fmt::Debug for IdMapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IdMapper")
    }
}

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

fn io_context(path: &Path, err: std::io::Error) -> EngineError {
    EngineError::Io(std::io::Error::new(
        err.kind(),
        format!("{}: {}", path.display(), err),
    ))
}

fn ensure_max_alloc(len: u64, opts: &SyncOptions) -> Result<()> {
    if opts.max_alloc != 0 && len > opts.max_alloc as u64 {
        Err(EngineError::MaxAlloc)
    } else {
        Ok(())
    }
}

#[cfg(unix)]
fn preallocate(file: &File, len: u64) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe {
        let ret = libc::fallocate(file.as_raw_fd(), 0, 0, len as libc::off_t);
        if ret == 0 {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(ret))
        }
    }

    #[cfg(target_os = "macos")]
    unsafe {
        let fd = file.as_raw_fd();
        let mut fstore = libc::fstore_t {
            fst_flags: libc::F_ALLOCATECONTIG,
            fst_posmode: libc::F_PEOFPOSMODE,
            fst_offset: 0,
            fst_length: len as libc::off_t,
            fst_bytesalloc: 0,
        };
        let ret = libc::fcntl(fd, libc::F_PREALLOCATE, &fstore);
        if ret == -1 {
            fstore.fst_flags = libc::F_ALLOCATEALL;
            if libc::fcntl(fd, libc::F_PREALLOCATE, &fstore) == -1 {
                if libc::ftruncate(fd, len as libc::off_t) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                return Ok(());
            }
        }
        if libc::ftruncate(fd, len as libc::off_t) == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    unsafe {
        let ret = libc::posix_fallocate(file.as_raw_fd(), 0, len as libc::off_t);
        if ret == 0 {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(ret))
        }
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris"
    )))]
    {
        file.set_len(len)
    }
}

#[cfg(not(unix))]
fn preallocate(_file: &File, _len: u64) -> std::io::Result<()> {
    Ok(())
}

fn outside_size_bounds(len: u64, opts: &SyncOptions) -> bool {
    if let Some(min) = opts.min_size {
        if len < min {
            return true;
        }
    }
    if let Some(max) = opts.max_size {
        if len > max {
            return true;
        }
    }
    false
}

fn atomic_rename(src: &Path, dst: &Path) -> Result<()> {
    match fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(e) => {
            #[cfg(unix)]
            {
                if e.raw_os_error() == Some(nix::errno::Errno::EXDEV as i32) {
                    let parent = dst.parent().unwrap_or_else(|| Path::new("."));
                    let tmp = NamedTempFile::new_in(parent).map_err(|e| io_context(parent, e))?;
                    let tmp_path = tmp.into_temp_path();
                    fs::copy(src, &tmp_path).map_err(|e| io_context(src, e))?;
                    fs::rename(&tmp_path, dst).map_err(|e| io_context(dst, e))?;
                    fs::remove_file(src).map_err(|e| io_context(src, e))?;
                    return Ok(());
                }
            }
            Err(io_context(src, e))
        }
    }
}

fn remove_file_opts(path: &Path, opts: &SyncOptions) -> Result<()> {
    match fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            let e = io_context(path, e);
            if opts.ignore_errors {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn remove_dir_all_opts(path: &Path, opts: &SyncOptions) -> Result<()> {
    match fs::remove_dir_all(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            let e = io_context(path, e);
            if opts.ignore_errors {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut curr = vec![i + 1];
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { prev[j] } else { prev[j] + 1 };
            let ins = curr[j] + 1;
            let del = prev[j + 1] + 1;
            curr.push(cost.min(ins).min(del));
        }
        prev = curr;
    }
    prev[b.len()]
}

fn fuzzy_match(dest: &Path) -> Option<PathBuf> {
    let parent = dest.parent()?;
    let stem = dest.file_stem()?.to_string_lossy().to_string();
    let mut best: Option<(usize, PathBuf)> = None;
    for entry in fs::read_dir(parent).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path == dest {
            continue;
        }
        let candidate_stem = path.file_stem()?.to_string_lossy().to_string();
        let dist = levenshtein(&stem, &candidate_stem);
        match &mut best {
            Some((d, _)) if dist < *d => best = Some((dist, path)),
            None => best = Some((dist, path)),
            _ => {}
        }
    }
    best.map(|(_, p)| p)
}

fn files_identical(a: &Path, b: &Path) -> bool {
    if let (Ok(ma), Ok(mb)) = (fs::metadata(a), fs::metadata(b)) {
        if ma.len() != mb.len() {
            return false;
        }
        let mut fa = match File::open(a) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut fb = match File::open(b) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut ba = [0u8; 8192];
        let mut bb = [0u8; 8192];
        loop {
            match (fa.read(&mut ba), fb.read(&mut bb)) {
                (Ok(ra), Ok(rb)) => {
                    if ra != rb {
                        return false;
                    }
                    if ra == 0 {
                        break;
                    }
                    if ba[..ra] != bb[..rb] {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    } else {
        false
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
    let mut src = match File::open(src) {
        Ok(f) => f,
        Err(_) => return Ok(0),
    };
    let mut dst = match File::open(dst) {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SenderState {
    Idle,
    Walking,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiverState {
    Idle,
    Applying,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Data(Vec<u8>),
    Copy { offset: usize, len: usize },
}

const DEFAULT_BASIS_WINDOW: usize = 8 * 1024;
const LIT_CAP: usize = 1 << 20;

pub struct DeltaIter<'a, R: Read + Seek> {
    cfg: &'a ChecksumConfig,
    target: &'a mut R,
    block_size: usize,
    map: HashMap<u32, Vec<(Vec<u8>, usize, usize)>>,
    lit: Vec<u8>,
    window: VecDeque<u8>,
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
                        Ok(_) => self.window.push_back(self.byte[0]),
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
            self.window.make_contiguous();
            let sum = self.cfg.checksum(&self.window.as_slices().0[..len]);
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

            if let Some(b) = self.window.pop_front() {
                self.lit.push(b);
                if self.lit.len() >= LIT_CAP {
                    return Some(Ok(Op::Data(std::mem::take(&mut self.lit))));
                }
            }
            if self.done && self.window.is_empty() {
                return Some(Ok(Op::Data(std::mem::take(&mut self.lit))));
            }
        }
    }
}

pub fn compute_delta<'a, R1: Read + Seek, R2: Read + Seek>(
    cfg: &'a ChecksumConfig,
    basis: &mut R1,
    target: &'a mut R2,
    block_size: usize,
    basis_window: usize,
    opts: &SyncOptions,
) -> Result<DeltaIter<'a, R2>> {
    let block_size = block_size.max(1);
    ensure_max_alloc(block_size as u64, opts)?;
    basis.seek(SeekFrom::Start(0))?;
    target.seek(SeekFrom::Start(0))?;
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
        window: VecDeque::new(),
        byte: [0u8; 1],
        done: false,
    })
}

fn write_sparse(file: &mut File, data: &[u8]) -> Result<()> {
    let mut i = 0;
    while i < data.len() {
        if data[i] == 0 {
            let start = i;
            while i < data.len() && data[i] == 0 {
                i += 1;
            }
            let len = i - start;
            #[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
            {
                let fd = file.as_raw_fd();
                let offset = file.stream_position()?;
                let _ = fallocate(
                    fd,
                    FallocateFlags::FALLOC_FL_PUNCH_HOLE | FallocateFlags::FALLOC_FL_KEEP_SIZE,
                    offset as i64,
                    len as i64,
                );
            }
            file.seek(SeekFrom::Current(len as i64))?;
        } else {
            let start = i;
            while i < data.len() && data[i] != 0 {
                i += 1;
            }
            file.write_all(&data[start..i])?;
        }
    }
    Ok(())
}

struct Progress<'a> {
    total: u64,
    written: u64,
    start: std::time::Instant,
    last_print: std::time::Instant,
    human_readable: bool,
    #[allow(dead_code)]
    dest: &'a Path,
    quiet: bool,
}

const PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_secs(1);

impl<'a> Progress<'a> {
    fn new(dest: &'a Path, total: u64, human_readable: bool, initial: u64, quiet: bool) -> Self {
        if !quiet {
            eprintln!("{}", dest.display());
        }
        let now = std::time::Instant::now();
        Self {
            total,
            written: initial,
            start: now,
            last_print: now - PROGRESS_UPDATE_INTERVAL,
            human_readable,
            dest,
            quiet,
        }
    }

    fn add(&mut self, bytes: u64) {
        self.written += bytes;
        if !self.quiet
            && self.last_print.elapsed() >= PROGRESS_UPDATE_INTERVAL
            && self.written < self.total
        {
            self.print(false);
            self.last_print = std::time::Instant::now();
        }
    }

    fn finish(&mut self) {
        if !self.quiet {
            self.print(true);
        }
    }

    fn print(&self, done: bool) {
        if self.quiet {
            return;
        }
        use std::io::Write as _;
        let bytes = progress_formatter(self.written, self.human_readable);
        let percent = if self.total == 0 {
            100
        } else {
            self.written * 100 / self.total
        };
        let elapsed = self.start.elapsed().as_secs().max(1);
        let rate = self.written / elapsed;
        let rate = format!("{}/s", progress_formatter(rate, true));
        if done {
            eprintln!("\r{:>15} {:>3}% {:>15}", bytes, percent, rate);
        } else {
            eprint!("\r{:>15} {:>3}% {:>15}", bytes, percent, rate);
            let _ = std::io::stderr().flush();
        }
    }
}

fn apply_op_plain<R: Read + Seek, W: Write + Seek>(
    basis: &mut R,
    op: Op,
    out: &mut W,
    buf: &mut [u8],
    progress: &mut Option<Progress>,
) -> Result<()> {
    match op {
        Op::Data(d) => {
            out.write_all(&d)?;
            if let Some(p) = progress {
                p.add(d.len() as u64);
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
                if let Some(p) = progress {
                    p.add(to_read as u64);
                }
            }
        }
    }
    Ok(())
}

fn apply_op_inplace<R: Read + Seek>(
    basis: &mut R,
    op: Op,
    out: &mut File,
    buf: &mut [u8],
    progress: &mut Option<Progress>,
) -> Result<()> {
    match op {
        Op::Data(d) => {
            out.write_all(&d)?;
            if let Some(p) = progress {
                p.add(d.len() as u64);
            }
        }
        Op::Copy { offset, len } => {
            let pos = out.stream_position()?;
            if offset as u64 == pos {
                out.seek(SeekFrom::Current(len as i64))?;
                if let Some(p) = progress {
                    p.add(len as u64);
                }
            } else {
                basis.seek(SeekFrom::Start(offset as u64))?;
                let mut remaining = len;
                while remaining > 0 {
                    let to_read = remaining.min(buf.len());
                    basis.read_exact(&mut buf[..to_read])?;
                    out.write_all(&buf[..to_read])?;
                    remaining -= to_read;
                    if let Some(p) = progress {
                        p.add(to_read as u64);
                    }
                }
            }
        }
    }
    Ok(())
}

fn apply_op_sparse<R: Read + Seek>(
    basis: &mut R,
    op: Op,
    out: &mut File,
    buf: &mut [u8],
    progress: &mut Option<Progress>,
) -> Result<()> {
    match op {
        Op::Data(d) => {
            write_sparse(out, &d)?;
            if let Some(p) = progress {
                p.add(d.len() as u64);
            }
        }
        Op::Copy { offset, len } => {
            basis.seek(SeekFrom::Start(offset as u64))?;
            let mut remaining = len;
            while remaining > 0 {
                let to_read = remaining.min(buf.len());
                basis.read_exact(&mut buf[..to_read])?;
                write_sparse(out, &buf[..to_read])?;
                remaining -= to_read;
                if let Some(p) = progress {
                    p.add(to_read as u64);
                }
            }
        }
    }
    Ok(())
}

fn apply_delta<R: Read + Seek, W: Write + Seek + Any, I>(
    basis: &mut R,
    ops: I,
    out: &mut W,
    opts: &SyncOptions,
    mut skip: u64,
    progress: &mut Option<Progress>,
) -> Result<()>
where
    I: IntoIterator<Item = Result<Op>>,
{
    ensure_max_alloc(8192, opts)?;
    let mut buf = vec![0u8; 8192];
    let mut adjust = |op: Op| -> Option<Op> {
        if skip == 0 {
            return Some(op);
        }
        match op {
            Op::Data(d) => {
                if (skip as usize) >= d.len() {
                    skip -= d.len() as u64;
                    None
                } else {
                    let start = skip as usize;
                    skip = 0;
                    Some(Op::Data(d[start..].to_vec()))
                }
            }
            Op::Copy { offset, len } => {
                if (skip as usize) >= len {
                    skip -= len as u64;
                    None
                } else {
                    let start = skip as usize;
                    skip = 0;
                    Some(Op::Copy {
                        offset: offset + start,
                        len: len - start,
                    })
                }
            }
        }
    };
    if opts.inplace {
        let file = (&mut *out as &mut dyn Any)
            .downcast_mut::<File>()
            .ok_or_else(|| EngineError::Other("inplace output must be a File".into()))?;
        for op in ops {
            let op = op?;
            if let Some(op) = adjust(op) {
                apply_op_inplace(basis, op, file, &mut buf, progress)?;
            }
        }
    } else if opts.sparse {
        let file = (&mut *out as &mut dyn Any)
            .downcast_mut::<File>()
            .ok_or_else(|| EngineError::Other("sparse output must be a File".into()))?;
        for op in ops {
            let op = op?;
            if let Some(op) = adjust(op) {
                apply_op_sparse(basis, op, file, &mut buf, progress)?;
            }
        }
        let pos = file.stream_position()?;
        file.set_len(pos)?;
    } else {
        for op in ops {
            let op = op?;
            if let Some(op) = adjust(op) {
                apply_op_plain(basis, op, out, &mut buf, progress)?;
            }
        }
    }
    Ok(())
}

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

    fn strong_file_checksum(&self, path: &Path) -> Result<Vec<u8>> {
        let data = fs::read(path).map_err(|e| io_context(path, e))?;
        Ok(self.cfg.checksum(&data).strong)
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

    fn start(&mut self) {
        self.state = SenderState::Walking;
    }

    fn finish(&mut self) {
        self.state = SenderState::Finished;
    }

    fn process_file(
        &mut self,
        path: &Path,
        dest: &Path,
        rel: &Path,
        recv: &mut Receiver,
    ) -> Result<bool> {
        if self.opts.checksum {
            if let Ok(dst_sum) = self.strong_file_checksum(dest) {
                let src_sum = self.strong_file_checksum(path)?;
                if src_sum == dst_sum {
                    recv.copy_metadata(path, dest)?;
                    return Ok(false);
                }
            } else if self.metadata_unchanged(path, dest) {
                recv.copy_metadata(path, dest)?;
                return Ok(false);
            }
        } else if self.metadata_unchanged(path, dest) {
            recv.copy_metadata(path, dest)?;
            return Ok(false);
        }

        let meta = fs::metadata(path).map_err(|e| io_context(path, e))?;
        let src_len = meta.len();
        ensure_max_alloc(src_len, &self.opts)?;
        let block_size = if self.block_size == 0 {
            cdc::block_size(src_len)
        } else {
            self.block_size
        };
        let file_type = meta.file_type();
        let atime_guard = if self.opts.atimes {
            meta::AccessTime::new(path).ok()
        } else {
            None
        };
        let src = File::open(path).map_err(|e| io_context(path, e))?;
        let mut src_reader = BufReader::new(src);
        let file_codec = if should_compress(path, &self.opts.skip_compress) {
            self.codec
        } else {
            None
        };
        let partial_path = if let Some(dir) = &self.opts.partial_dir {
            let file = dest.file_name().unwrap_or_default();
            if let Some(parent) = dest.parent() {
                parent.join(dir).join(file)
            } else {
                dir.join(file)
            }
        } else {
            dest.with_extension("partial")
        };
        let basis_path = if self.opts.partial && partial_path.exists() {
            partial_path.clone()
        } else if self.opts.fuzzy && !dest.exists() {
            fuzzy_match(dest).unwrap_or_else(|| dest.to_path_buf())
        } else {
            dest.to_path_buf()
        };
        let mut resume = if self.opts.partial && partial_path.exists() {
            last_good_block(&self.cfg, path, &partial_path, block_size, &self.opts)?
        } else if self.opts.append || self.opts.append_verify {
            if self.opts.append_verify {
                last_good_block(&self.cfg, path, dest, block_size, &self.opts)?
            } else {
                fs::metadata(dest).map(|m| m.len()).unwrap_or(0)
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
            match File::open(&basis_path) {
                Ok(f) => {
                    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                    ensure_max_alloc(len, &self.opts)?;
                    Box::new(BufReader::new(f))
                }
                Err(_) => Box::new(Cursor::new(Vec::new())),
            }
        };
        let delta: Box<dyn Iterator<Item = Result<Op>> + '_> = if self.opts.copy_devices
            && (file_type.is_block_device() || file_type.is_char_device())
            && src_len == 0
        {
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
                dir.join(rel)
            } else {
                let name = dest
                    .file_name()
                    .map(|n| format!("{}~", n.to_string_lossy()))
                    .unwrap_or_else(|| "~".to_string());
                dest.with_file_name(name)
            };
            if let Some(parent) = backup_path.parent() {
                fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
            }
            atomic_rename(dest, &backup_path)?;
        }
        let mut skip = resume as u64;
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
        let ops = adjusted.map(|op_res| {
            let mut op = op_res?;
            if let Some(codec) = file_codec {
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
        recv.apply(path, dest, rel, ops)?;
        drop(atime_guard);
        recv.copy_metadata(path, dest)?;
        Ok(true)
    }
}

pub struct Receiver {
    state: ReceiverState,
    codec: Option<Codec>,
    opts: SyncOptions,
    delayed: Vec<(PathBuf, PathBuf, PathBuf)>,
    #[cfg(unix)]
    link_map: HashMap<(u64, u64), PathBuf>,
    #[cfg(unix)]
    pending_links: Vec<(PathBuf, PathBuf)>,
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
            delayed: Vec::new(),
            #[cfg(unix)]
            link_map: HashMap::new(),
            #[cfg(unix)]
            pending_links: Vec::new(),
        }
    }

    #[cfg(unix)]
    fn register_hard_link(&mut self, dev: u64, inode: u64, dest: &Path) -> Result<bool> {
        if let Some(existing) = self.link_map.get(&(dev, inode)) {
            if self.opts.delay_updates || self.delayed.iter().any(|(_, _, d)| d == existing) {
                self.pending_links
                    .push((existing.clone(), dest.to_path_buf()));
            } else {
                fs::hard_link(existing, dest).map_err(|e| io_context(dest, e))?;
            }
            Ok(false)
        } else {
            self.link_map.insert((dev, inode), dest.to_path_buf());
            Ok(true)
        }
    }

    pub fn apply<I>(&mut self, src: &Path, dest: &Path, rel: &Path, delta: I) -> Result<PathBuf>
    where
        I: IntoIterator<Item = Result<Op>>,
    {
        self.state = ReceiverState::Applying;
        let partial = if let Some(dir) = &self.opts.partial_dir {
            let file = dest.file_name().unwrap_or_default();
            if let Some(parent) = dest.parent() {
                parent.join(dir).join(file)
            } else {
                dir.join(file)
            }
        } else {
            dest.with_extension("partial")
        };
        let basis_path = if self.opts.inplace {
            dest.to_path_buf()
        } else if self.opts.partial && partial.exists() {
            partial.clone()
        } else {
            dest.to_path_buf()
        };
        let mut tmp_dest = if self.opts.inplace {
            dest.to_path_buf()
        } else if let Some(dir) = &self.opts.temp_dir {
            dir.join(rel).with_extension("tmp")
        } else if self.opts.partial {
            partial.clone()
        } else {
            dest.to_path_buf()
        };
        let auto_tmp = !self.opts.inplace
            && !self.opts.partial
            && self.opts.temp_dir.is_none()
            && basis_path == dest
            && !self.opts.write_devices;
        if auto_tmp {
            tmp_dest = dest.with_extension("tmp");
        }
        let mut needs_rename =
            !self.opts.inplace && (self.opts.partial || self.opts.temp_dir.is_some() || auto_tmp);
        if self.opts.delay_updates && !self.opts.inplace && !self.opts.write_devices {
            if tmp_dest == dest {
                tmp_dest = dest.with_extension("tmp");
            }
            needs_rename = true;
        }
        let cfg = ChecksumConfigBuilder::new()
            .strong(self.opts.strong)
            .seed(self.opts.checksum_seed)
            .build();
        let src_len = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        let block_size = if self.opts.block_size == 0 {
            cdc::block_size(src_len)
        } else {
            self.opts.block_size
        };
        let mut resume = if self.opts.partial || self.opts.append || self.opts.append_verify {
            if self.opts.append && !self.opts.append_verify {
                fs::metadata(&tmp_dest).map(|m| m.len()).unwrap_or(0)
            } else {
                last_good_block(&cfg, src, &tmp_dest, block_size, &self.opts)?
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
                if ft.is_block_device() || ft.is_char_device() {
                    Box::new(Cursor::new(Vec::new()))
                } else {
                    match File::open(&basis_path) {
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
            match File::open(&basis_path) {
                Ok(f) => {
                    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                    ensure_max_alloc(len, &self.opts)?;
                    Box::new(BufReader::new(f))
                }
                Err(_) => Box::new(Cursor::new(Vec::new())),
            }
        };
        if let Some(parent) = tmp_dest.parent() {
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
        }
        #[cfg(unix)]
        if !self.opts.write_devices {
            let check_path = if auto_tmp { dest } else { &tmp_dest };
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
        if !self.opts.write_devices {
            out.set_len(resume)?;
            out.seek(SeekFrom::Start(resume))?;
            if self.opts.preallocate {
                preallocate(&out, src_len)?;
            }
        }
        let file_codec = if should_compress(src, &self.opts.skip_compress) {
            self.codec
        } else {
            None
        };
        let mut progress = if self.opts.progress {
            Some(Progress::new(
                dest,
                src_len,
                self.opts.human_readable,
                resume,
                self.opts.quiet,
            ))
        } else {
            None
        };
        let ops = delta.into_iter().map(|op_res| {
            let mut op = op_res?;
            if let Some(codec) = file_codec {
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
        apply_delta(&mut basis, ops, &mut out, &self.opts, 0, &mut progress)?;
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
                    .push((src.to_path_buf(), tmp_dest.clone(), dest.to_path_buf()));
            } else {
                atomic_rename(&tmp_dest, dest)?;
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
            }
            #[cfg(unix)]
            if let Some((uid, gid)) = self.opts.copy_as {
                let gid = gid.map(Gid::from_raw);
                chown(dest, Some(Uid::from_raw(uid)), gid)
                    .map_err(|e| io_context(dest, std::io::Error::from(e)))?;
            }
        } else {
            #[cfg(unix)]
            if let Some((uid, gid)) = self.opts.copy_as {
                let gid = gid.map(Gid::from_raw);
                chown(dest, Some(Uid::from_raw(uid)), gid)
                    .map_err(|e| io_context(dest, std::io::Error::from(e)))?;
            }
        }
        self.state = ReceiverState::Finished;
        Ok(if self.opts.delay_updates && needs_rename {
            tmp_dest
        } else {
            dest.to_path_buf()
        })
    }
}

impl Receiver {
    fn copy_metadata_now(&self, src: &Path, dest: &Path) -> Result<()> {
        #[cfg(unix)]
        if self.opts.write_devices {
            if let Ok(meta) = fs::symlink_metadata(dest) {
                let ft = meta.file_type();
                if ft.is_char_device() || ft.is_block_device() {
                    return Ok(());
                }
            }
        }

        #[cfg(unix)]
        if self.opts.perms {
            let src_meta = fs::symlink_metadata(src).map_err(|e| io_context(src, e))?;
            if !src_meta.file_type().is_symlink() {
                let mode = meta::mode_from_metadata(&src_meta);
                fs::set_permissions(dest, fs::Permissions::from_mode(mode))
                    .map_err(|e| io_context(dest, e))?;
            }
        }

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let chown_uid = self.opts.chown.and_then(|(u, _)| u);
            let chown_gid = self.opts.chown.and_then(|(_, g)| g);

            let uid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>> =
                if let Some(ref map) = self.opts.uid_map {
                    Some(map.0.clone())
                } else if let Some(uid) = chown_uid {
                    Some(Arc::new(move |_| uid))
                } else if self.opts.numeric_ids {
                    None
                } else {
                    Some(Arc::new(|uid: u32| {
                        use nix::unistd::{Uid, User};
                        if let Ok(Some(u)) = User::from_uid(Uid::from_raw(uid)) {
                            if let Ok(Some(local)) = User::from_name(&u.name) {
                                return local.uid.as_raw();
                            }
                        }
                        uid
                    }))
                };

            let gid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>> =
                if let Some(ref map) = self.opts.gid_map {
                    Some(map.0.clone())
                } else if let Some(gid) = chown_gid {
                    Some(Arc::new(move |_| gid))
                } else if self.opts.numeric_ids {
                    None
                } else {
                    Some(Arc::new(|gid: u32| {
                        use nix::unistd::{Gid, Group};
                        if let Ok(Some(g)) = Group::from_gid(Gid::from_raw(gid)) {
                            if let Ok(Some(local)) = Group::from_name(&g.name) {
                                return local.gid.as_raw();
                            }
                        }
                        gid
                    }))
                };

            let mut meta_opts = meta::Options {
                xattrs: {
                    #[cfg(feature = "xattr")]
                    {
                        self.opts.xattrs || self.opts.fake_super
                    }
                    #[cfg(not(feature = "xattr"))]
                    {
                        false
                    }
                },
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
                perms: self.opts.perms,
                executability: self.opts.executability,
                times: self.opts.times,
                atimes: self.opts.atimes,
                crtimes: self.opts.crtimes,
                omit_dir_times: self.opts.omit_dir_times,
                omit_link_times: self.opts.omit_link_times,
                uid_map,
                gid_map,
                fake_super: self.opts.fake_super,
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
                                if !meta_opts.needs_metadata() {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }

                let meta =
                    meta::Metadata::from_path(src, meta_opts.clone()).map_err(EngineError::from)?;
                meta.apply(dest, meta_opts.clone())
                    .map_err(EngineError::from)?;
                if self.opts.fake_super {
                    #[cfg(feature = "xattr")]
                    {
                        meta::store_fake_super(dest, meta.uid, meta.gid, meta.mode);
                    }
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
            #[cfg(unix)]
            if let Some((uid, gid)) = self.opts.copy_as {
                let gid = gid.map(Gid::from_raw);
                chown(&dest, Some(Uid::from_raw(uid)), gid)
                    .map_err(|e| io_context(&dest, std::io::Error::from(e)))?;
            }
            self.copy_metadata_now(&src, &dest)?;
        }
        #[cfg(unix)]
        {
            for (src, dest) in std::mem::take(&mut self.pending_links) {
                fs::hard_link(&src, &dest).map_err(|e| io_context(&dest, e))?;
            }
            self.link_map.clear();
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    Before,
    During,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModernCdc {
    Fastcdc,
    Off,
}

#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub delete: Option<DeleteMode>,
    pub delete_excluded: bool,
    pub ignore_missing_args: bool,
    pub delete_missing_args: bool,
    pub remove_source_files: bool,
    pub ignore_errors: bool,
    pub max_delete: Option<usize>,
    pub max_alloc: usize,
    pub max_size: Option<u64>,
    pub min_size: Option<u64>,
    pub preallocate: bool,
    pub checksum: bool,
    pub compress: bool,
    pub modern_compress: Option<ModernCompress>,
    pub modern_hash: Option<ModernHash>,
    pub modern_cdc: ModernCdc,
    pub modern_cdc_min: usize,
    pub modern_cdc_max: usize,
    pub dirs: bool,
    pub list_only: bool,
    pub update: bool,
    pub existing: bool,
    pub ignore_existing: bool,
    pub size_only: bool,
    pub ignore_times: bool,
    pub perms: bool,
    pub executability: bool,
    pub times: bool,
    pub atimes: bool,
    pub crtimes: bool,
    pub omit_dir_times: bool,
    pub omit_link_times: bool,
    pub owner: bool,
    pub group: bool,
    pub links: bool,
    pub copy_links: bool,
    pub copy_dirlinks: bool,
    pub keep_dirlinks: bool,
    pub copy_unsafe_links: bool,
    pub safe_links: bool,
    pub hard_links: bool,
    pub devices: bool,
    pub specials: bool,
    pub fsync: bool,
    pub fuzzy: bool,
    pub fake_super: bool,
    #[cfg(feature = "xattr")]
    pub xattrs: bool,
    #[cfg(feature = "acl")]
    pub acls: bool,
    pub sparse: bool,
    pub strong: StrongHash,

    pub checksum_seed: u32,
    pub compress_level: Option<i32>,
    pub compress_choice: Option<Vec<Codec>>,
    pub whole_file: bool,
    pub skip_compress: Vec<String>,
    pub partial: bool,
    pub progress: bool,
    pub human_readable: bool,
    pub itemize_changes: bool,
    pub partial_dir: Option<PathBuf>,
    pub temp_dir: Option<PathBuf>,
    pub append: bool,
    pub append_verify: bool,
    pub numeric_ids: bool,
    pub inplace: bool,
    pub delay_updates: bool,
    pub modify_window: Duration,
    pub bwlimit: Option<u64>,
    pub block_size: usize,
    pub link_dest: Option<PathBuf>,
    pub copy_dest: Option<PathBuf>,
    pub compare_dest: Option<PathBuf>,
    pub backup: bool,
    pub backup_dir: Option<PathBuf>,
    pub chmod: Option<Vec<meta::Chmod>>,
    pub chown: Option<(Option<u32>, Option<u32>)>,
    pub copy_as: Option<(u32, Option<u32>)>,
    pub eight_bit_output: bool,
    pub blocking_io: bool,
    pub early_input: Option<PathBuf>,
    pub secluded_args: bool,
    pub sockopts: Vec<String>,
    pub remote_options: Vec<String>,
    pub write_batch: Option<PathBuf>,
    pub copy_devices: bool,
    pub write_devices: bool,
    pub quiet: bool,
    pub uid_map: Option<IdMapper>,
    pub gid_map: Option<IdMapper>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            delete: None,
            delete_excluded: false,
            ignore_missing_args: false,
            delete_missing_args: false,
            remove_source_files: false,
            ignore_errors: false,
            max_delete: None,
            max_alloc: 1 << 30,
            max_size: None,
            min_size: None,
            preallocate: false,
            checksum: false,
            compress: false,
            modern_compress: None,
            modern_hash: None,
            modern_cdc: ModernCdc::Off,
            modern_cdc_min: 2 * 1024,
            modern_cdc_max: 16 * 1024,
            perms: false,
            executability: false,
            times: false,
            atimes: false,
            crtimes: false,
            omit_dir_times: false,
            omit_link_times: false,
            owner: false,
            group: false,
            links: false,
            copy_links: false,
            copy_dirlinks: false,
            keep_dirlinks: false,
            copy_unsafe_links: false,
            safe_links: false,
            hard_links: false,
            devices: false,
            specials: false,
            fsync: false,
            fuzzy: false,
            fake_super: false,
            #[cfg(feature = "xattr")]
            xattrs: false,
            #[cfg(feature = "acl")]
            acls: false,
            sparse: false,
            dirs: false,
            list_only: false,
            update: false,
            existing: false,
            ignore_existing: false,
            size_only: false,
            ignore_times: false,
            strong: StrongHash::Md5,
            checksum_seed: 0,
            compress_level: None,
            compress_choice: None,
            whole_file: false,
            skip_compress: Vec::new(),
            partial: false,
            progress: false,
            human_readable: false,
            itemize_changes: false,
            partial_dir: None,
            temp_dir: None,
            append: false,
            append_verify: false,
            numeric_ids: false,
            inplace: false,
            delay_updates: false,
            modify_window: Duration::ZERO,
            bwlimit: None,
            block_size: 0,
            link_dest: None,
            copy_dest: None,
            compare_dest: None,
            backup: false,
            backup_dir: None,
            chmod: None,
            chown: None,
            copy_as: None,
            eight_bit_output: false,
            blocking_io: false,
            early_input: None,
            secluded_args: false,
            sockopts: Vec::new(),
            remote_options: Vec::new(),
            write_batch: None,
            copy_devices: false,
            write_devices: false,
            quiet: false,
            uid_map: None,
            gid_map: None,
        }
    }
}

impl SyncOptions {
    pub fn prepare_remote(&mut self) {
        if let Some(dir) = &self.partial_dir {
            self.remote_options
                .push(format!("--partial-dir={}", dir.display()));
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub files_transferred: usize,
    pub files_deleted: usize,
    pub bytes_transferred: u64,
}

fn cpu_prefers_lz4() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        !std::arch::is_x86_feature_detected!("sse4.2")
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        false
    }
}

pub fn select_codec(remote: &[Codec], opts: &SyncOptions) -> Option<Codec> {
    if !opts.compress || opts.compress_level == Some(0) {
        return None;
    }
    if let Some(choice) = &opts.compress_choice {
        return choice.iter().copied().find(|c| remote.contains(c));
    }
    let modern = match opts.modern_compress {
        Some(m) => m,
        None => return remote.contains(&Codec::Zlib).then_some(Codec::Zlib),
    };
    let local = available_codecs(Some(modern));
    match modern {
        ModernCompress::Auto => {
            let prefer_lz4 = cpu_prefers_lz4();
            if prefer_lz4 && local.contains(&Codec::Lz4) && remote.contains(&Codec::Lz4) {
                Some(Codec::Lz4)
            } else if local.contains(&Codec::Zstd) && remote.contains(&Codec::Zstd) {
                Some(Codec::Zstd)
            } else if local.contains(&Codec::Lz4) && remote.contains(&Codec::Lz4) {
                Some(Codec::Lz4)
            } else if remote.contains(&Codec::Zlib) {
                Some(Codec::Zlib)
            } else {
                None
            }
        }
        ModernCompress::Zstd => {
            if remote.contains(&Codec::Zstd) {
                Some(Codec::Zstd)
            } else if remote.contains(&Codec::Zlib) {
                Some(Codec::Zlib)
            } else {
                None
            }
        }
        ModernCompress::Lz4 => {
            if remote.contains(&Codec::Lz4) {
                Some(Codec::Lz4)
            } else if remote.contains(&Codec::Zstd) {
                Some(Codec::Zstd)
            } else if remote.contains(&Codec::Zlib) {
                Some(Codec::Zlib)
            } else {
                None
            }
        }
    }
}

fn delete_extraneous(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    opts: &SyncOptions,
    stats: &mut Stats,
) -> Result<()> {
    let walker = walk(dst, 1024, opts.links);
    let mut state = String::new();

    let mut first_err: Option<EngineError> = None;
    for batch in walker {
        let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
        for entry in batch {
            let path = entry.apply(&mut state);
            let file_type = entry.file_type;
            if let Ok(rel) = path.strip_prefix(dst) {
                let included = matcher
                    .is_included_for_delete(rel)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                let src_exists = src.join(rel).exists();
                if (included && !src_exists) || (!included && opts.delete_excluded) {
                    if let Some(max) = opts.max_delete {
                        if stats.files_deleted >= max {
                            return Err(EngineError::Other("max-delete limit exceeded".into()));
                        }
                    }
                    let res = if opts.backup {
                        let backup_path = if let Some(ref dir) = opts.backup_dir {
                            dir.join(rel)
                        } else {
                            let name = path
                                .file_name()
                                .map(|n| format!("{}~", n.to_string_lossy()))
                                .unwrap_or_else(|| "~".to_string());
                            path.with_file_name(name)
                        };
                        let dir_res = if let Some(parent) = backup_path.parent() {
                            fs::create_dir_all(parent).map_err(|e| io_context(parent, e))
                        } else {
                            Ok(())
                        };
                        dir_res
                            .and_then(|_| atomic_rename(&path, &backup_path))
                            .err()
                    } else if file_type.is_dir() {
                        remove_dir_all_opts(&path, opts).err()
                    } else {
                        remove_file_opts(&path, opts).err()
                    };
                    match res {
                        None => {
                            stats.files_deleted += 1;
                        }
                        Some(e) => {
                            if first_err.is_none() {
                                first_err = Some(e);
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(e) = first_err {
        if opts.ignore_errors {
            Ok(())
        } else {
            Err(e)
        }
    } else {
        Ok(())
    }
}

pub fn sync(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    remote: &[Codec],
    opts: &SyncOptions,
) -> Result<Stats> {
    let mut batch_file = opts
        .write_batch
        .as_ref()
        .and_then(|p| OpenOptions::new().create(true).append(true).open(p).ok());
    let src_root = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    let mut stats = Stats::default();
    if !src_root.exists() {
        if opts.delete_missing_args {
            if dst.exists() {
                if let Some(max) = opts.max_delete {
                    if stats.files_deleted >= max {
                        return Err(EngineError::Other("max-delete limit exceeded".into()));
                    }
                }
                let meta = fs::symlink_metadata(dst).map_err(|e| io_context(dst, e))?;
                let res = if opts.backup {
                    let backup_path = if let Some(ref dir) = opts.backup_dir {
                        if let Some(name) = dst.file_name() {
                            dir.join(name)
                        } else {
                            dir.join(dst)
                        }
                    } else {
                        let name = dst
                            .file_name()
                            .map(|n| format!("{}~", n.to_string_lossy()))
                            .unwrap_or_else(|| "~".to_string());
                        dst.with_file_name(name)
                    };
                    if let Some(parent) = backup_path.parent() {
                        fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
                    }
                    atomic_rename(dst, &backup_path).err()
                } else if meta.file_type().is_dir() {
                    remove_dir_all_opts(dst, opts).err()
                } else {
                    remove_file_opts(dst, opts).err()
                };
                match res {
                    None => stats.files_deleted += 1,
                    Some(e) => {
                        if !opts.ignore_errors {
                            return Err(e);
                        }
                    }
                }
            }
            return Ok(stats);
        } else if opts.ignore_missing_args {
            return Ok(stats);
        } else {
            return Err(EngineError::Other(format!(
                "source path missing: {}",
                src.display()
            )));
        }
    }
    if opts.list_only {
        let matcher = matcher.clone().with_root(src_root.clone());
        let walker = walk(&src_root, 1024, opts.links);
        let mut state = String::new();
        for batch in walker {
            let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
            for entry in batch {
                let path = entry.apply(&mut state);
                if let Ok(rel) = path.strip_prefix(&src_root) {
                    if !matcher
                        .is_included(rel)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?
                    {
                        continue;
                    }
                    if entry.file_type.is_dir() {
                        matcher
                            .preload_dir(&path)
                            .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                    }
                    if opts.dirs && !entry.file_type.is_dir() {
                        continue;
                    }
                    if entry.file_type.is_file() {
                        let len = fs::metadata(&path).map_err(|e| io_context(&path, e))?.len();
                        if outside_size_bounds(len, opts) {
                            continue;
                        }
                    }
                    if !opts.quiet {
                        if rel.as_os_str().is_empty() {
                            println!(".");
                        } else if entry.file_type.is_dir() {
                            println!("{}/", rel.display());
                        } else {
                            println!("{}", rel.display());
                        }
                    }
                }
            }
        }
        return Ok(stats);
    }

    if !dst.exists() {
        fs::create_dir_all(dst).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!(
                    "failed to create destination directory {}: {e}",
                    dst.display()
                ),
            )
        })?;
        #[cfg(unix)]
        if let Some((uid, gid)) = opts.copy_as {
            let gid = gid.map(Gid::from_raw);
            chown(dst, Some(Uid::from_raw(uid)), gid)
                .map_err(|e| io_context(dst, std::io::Error::from(e)))?;
        }
    }

    let codec = select_codec(remote, opts);
    let matcher = matcher.clone().with_root(src_root.clone());
    let mut sender = Sender::new(opts.block_size, matcher.clone(), codec, opts.clone());
    let mut receiver = Receiver::new(codec, opts.clone());
    let mut manifest = if matches!(opts.modern_cdc, ModernCdc::Fastcdc) {
        Manifest::load()
    } else {
        Manifest::default()
    };
    let mut dir_meta: Vec<(PathBuf, PathBuf)> = Vec::new();
    if matches!(opts.delete, Some(DeleteMode::Before)) {
        delete_extraneous(&src_root, dst, &matcher, opts, &mut stats)?;
    }
    sender.start();
    let mut state = String::new();
    let mut walker = walk(&src_root, 1024, opts.links);
    while let Some(batch) = walker.next() {
        let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
        for entry in batch {
            let path = entry.apply(&mut state);
            let file_type = entry.file_type;
            if let Ok(rel) = path.strip_prefix(&src_root) {
                if !matcher
                    .is_included(rel)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?
                {
                    continue;
                }
                let dest_path = dst.join(rel);
                if opts.dirs && !file_type.is_dir() {
                    continue;
                }
                if file_type.is_file()
                    || (opts.copy_devices
                        && (file_type.is_char_device() || file_type.is_block_device()))
                {
                    let src_meta = fs::metadata(&path).map_err(|e| io_context(&path, e))?;
                    if outside_size_bounds(src_meta.len(), opts) {
                        continue;
                    }
                    if opts.ignore_existing && dest_path.exists() {
                        continue;
                    }
                    if opts.update && dest_path.exists() {
                        if let Ok(dst_meta) = fs::metadata(&dest_path) {
                            if let (Ok(src_mtime), Ok(dst_mtime)) =
                                (src_meta.modified(), dst_meta.modified())
                            {
                                if dst_mtime > src_mtime
                                    && dst_mtime
                                        .duration_since(src_mtime)
                                        .unwrap_or(Duration::ZERO)
                                        > opts.modify_window
                                {
                                    continue;
                                }
                            }
                        }
                    }
                    #[cfg(unix)]
                    if opts.hard_links
                        && !receiver.register_hard_link(
                            walker.devs()[entry.dev],
                            walker.inodes()[entry.inode],
                            &dest_path,
                        )?
                    {
                        continue;
                    }
                    let partial_exists = if opts.partial {
                        let partial_path = if let Some(ref dir) = opts.partial_dir {
                            let file = dest_path.file_name().unwrap_or_default();
                            if let Some(parent) = dest_path.parent() {
                                parent.join(dir).join(file)
                            } else {
                                dir.join(file)
                            }
                        } else {
                            dest_path.with_extension("partial")
                        };
                        partial_path.exists()
                    } else {
                        false
                    };
                    if opts.existing && !dest_path.exists() && !partial_exists {
                        continue;
                    }
                    if opts.update && !dest_path.exists() && !partial_exists {
                        continue;
                    }
                    if !dest_path.exists() && !partial_exists {
                        if matches!(opts.modern_cdc, ModernCdc::Fastcdc) {
                            if let Ok(chunks) = chunk_file(
                                &path,
                                opts.modern_cdc_min,
                                (opts.modern_cdc_min + opts.modern_cdc_max) / 2,
                                opts.modern_cdc_max,
                            ) {
                                if !chunks.is_empty() {
                                    if let Some(existing) =
                                        manifest.lookup(&chunks[0].hash, &dest_path)
                                    {
                                        let all = chunks.iter().all(|c| {
                                            manifest.lookup(&c.hash, &dest_path).is_some()
                                        });
                                        if all {
                                            if let Some(parent) = dest_path.parent() {
                                                fs::create_dir_all(parent)
                                                    .map_err(|e| io_context(parent, e))?;
                                            }
                                            fs::copy(&existing, &dest_path)
                                                .map_err(|e| io_context(&dest_path, e))?;
                                            stats.files_transferred += 1;
                                            receiver.copy_metadata(&path, &dest_path)?;
                                            if let Some(f) = batch_file.as_mut() {
                                                let _ = writeln!(f, "{}", rel.display());
                                            }
                                            if opts.itemize_changes && !opts.quiet {
                                                println!(">f+++++++++ {}", rel.display());
                                            }
                                            for c in &chunks {
                                                manifest.insert(&c.hash, &dest_path);
                                            }
                                            if opts.remove_source_files {
                                                remove_file_opts(&path, opts)?;
                                            }
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(ref link_dir) = opts.link_dest {
                            let link_path = link_dir.join(rel);
                            if files_identical(&path, &link_path) {
                                if let Some(parent) = dest_path.parent() {
                                    fs::create_dir_all(parent)
                                        .map_err(|e| io_context(parent, e))?;
                                }
                                fs::hard_link(&link_path, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                                receiver.copy_metadata(&path, &dest_path)?;
                                if opts.remove_source_files {
                                    remove_file_opts(&path, opts)?;
                                }
                                continue;
                            }
                        }
                        if let Some(ref copy_dir) = opts.copy_dest {
                            let copy_path = copy_dir.join(rel);
                            if files_identical(&path, &copy_path) {
                                if let Some(parent) = dest_path.parent() {
                                    fs::create_dir_all(parent)
                                        .map_err(|e| io_context(parent, e))?;
                                }
                                fs::copy(&copy_path, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                                receiver.copy_metadata(&path, &dest_path)?;
                                if opts.remove_source_files {
                                    remove_file_opts(&path, opts)?;
                                }
                                continue;
                            }
                        }
                        if let Some(ref compare_dir) = opts.compare_dest {
                            let comp_path = compare_dir.join(rel);
                            if files_identical(&path, &comp_path) {
                                if opts.remove_source_files {
                                    remove_file_opts(&path, opts)?;
                                }
                                continue;
                            }
                        }
                    }
                    if sender.process_file(&path, &dest_path, rel, &mut receiver)? {
                        stats.files_transferred += 1;
                        stats.bytes_transferred +=
                            fs::metadata(&path).map_err(|e| io_context(&path, e))?.len();
                        if let Some(f) = batch_file.as_mut() {
                            let _ = writeln!(f, "{}", rel.display());
                        }
                        if opts.itemize_changes && !opts.quiet {
                            println!(">f+++++++++ {}", rel.display());
                        }
                        if matches!(opts.modern_cdc, ModernCdc::Fastcdc) {
                            if let Ok(chunks) = chunk_file(
                                &dest_path,
                                opts.modern_cdc_min,
                                (opts.modern_cdc_min + opts.modern_cdc_max) / 2,
                                opts.modern_cdc_max,
                            ) {
                                for c in &chunks {
                                    manifest.insert(&c.hash, &dest_path);
                                }
                            }
                        }
                    }
                    if opts.remove_source_files {
                        remove_file_opts(&path, opts)?;
                    }
                } else if file_type.is_dir() {
                    matcher
                        .preload_dir(&path)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                    let dest_meta = fs::symlink_metadata(&dest_path).ok();
                    let dest_is_symlink = dest_meta
                        .as_ref()
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false);
                    let mut created = dest_meta.is_none();
                    if dest_is_symlink {
                        if opts.keep_dirlinks {
                            created = false;
                        } else {
                            remove_file_opts(&dest_path, opts)?;
                            fs::create_dir_all(&dest_path)
                                .map_err(|e| io_context(&dest_path, e))?;
                            created = true;
                        }
                    } else {
                        fs::create_dir_all(&dest_path).map_err(|e| io_context(&dest_path, e))?;
                    }
                    if created {
                        #[cfg(unix)]
                        if let Some((uid, gid)) = opts.copy_as {
                            let gid = gid.map(Gid::from_raw);
                            chown(&dest_path, Some(Uid::from_raw(uid)), gid)
                                .map_err(|e| io_context(&dest_path, std::io::Error::from(e)))?;
                        }
                        if opts.itemize_changes && !opts.quiet {
                            println!("cd+++++++++ {}/", rel.display());
                        }
                    }
                    if !(dest_is_symlink && opts.keep_dirlinks) {
                        dir_meta.push((path.clone(), dest_path.clone()));
                    }
                } else if file_type.is_symlink() {
                    let target = fs::read_link(&path).map_err(|e| io_context(&path, e))?;
                    let target_path = if target.is_absolute() {
                        target.clone()
                    } else {
                        path.parent().unwrap_or(Path::new("")).join(&target)
                    };
                    let is_unsafe = match fs::canonicalize(&target_path) {
                        Ok(p) => !p.starts_with(&src_root),
                        Err(_) => true,
                    };
                    if opts.safe_links && is_unsafe {
                        continue;
                    }
                    let meta = fs::metadata(&target_path).ok();
                    if opts.copy_links
                        || (opts.copy_dirlinks
                            && meta.as_ref().map(|m| m.is_dir()).unwrap_or(false))
                        || (opts.copy_unsafe_links && is_unsafe)
                    {
                        if let Some(meta) = meta {
                            if meta.is_dir() {
                                if let Some(parent) = dest_path.parent() {
                                    fs::create_dir_all(parent)
                                        .map_err(|e| io_context(parent, e))?;
                                }
                                let sub = sync(&target_path, &dest_path, &matcher, remote, opts)?;
                                stats.files_transferred += sub.files_transferred;
                                stats.files_deleted += sub.files_deleted;
                                stats.bytes_transferred += sub.bytes_transferred;
                            } else if meta.is_file()
                                && sender.process_file(
                                    &target_path,
                                    &dest_path,
                                    rel,
                                    &mut receiver,
                                )?
                            {
                                stats.files_transferred += 1;
                                stats.bytes_transferred += meta.len();
                                if let Some(f) = batch_file.as_mut() {
                                    let _ = writeln!(f, "{}", rel.display());
                                }
                            }
                            if opts.remove_source_files {
                                remove_file_opts(&path, opts)?;
                            }
                        }
                    } else if opts.links {
                        let created = fs::symlink_metadata(&dest_path).is_err();
                        if let Some(parent) = dest_path.parent() {
                            fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
                        }
                        if let Ok(meta_dest) = fs::symlink_metadata(&dest_path) {
                            if meta_dest.is_dir() {
                                remove_dir_all_opts(&dest_path, opts)?;
                            } else {
                                remove_file_opts(&dest_path, opts)?;
                            }
                        }
                        #[cfg(unix)]
                        std::os::unix::fs::symlink(&target, &dest_path)
                            .map_err(|e| io_context(&dest_path, e))?;
                        #[cfg(windows)]
                        {
                            if meta.as_ref().map_or(false, |m| m.is_dir()) {
                                std::os::windows::fs::symlink_dir(&target, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                            } else {
                                std::os::windows::fs::symlink_file(&target, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                            }
                        }
                        receiver.copy_metadata(&path, &dest_path)?;
                        if created {
                            stats.files_transferred += 1;
                            if opts.itemize_changes && !opts.quiet {
                                println!("cL+++++++++ {} -> {}", rel.display(), target.display());
                            }
                        }
                        if opts.remove_source_files {
                            remove_file_opts(&path, opts)?;
                        }
                    }
                } else {
                    #[cfg(unix)]
                    {
                        if (file_type.is_char_device() || file_type.is_block_device())
                            && opts.devices
                            && !opts.copy_devices
                        {
                            use nix::sys::stat::{Mode, SFlag};
                            let created = fs::symlink_metadata(&dest_path).is_err();
                            if !created {
                                remove_file_opts(&dest_path, opts)?;
                            }
                            if let Some(parent) = dest_path.parent() {
                                fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
                            }
                            let meta =
                                fs::symlink_metadata(&path).map_err(|e| io_context(&path, e))?;
                            let kind = if file_type.is_char_device() {
                                SFlag::S_IFCHR
                            } else {
                                SFlag::S_IFBLK
                            };
                            let perm =
                                Mode::from_bits_truncate((meta.mode() & 0o777) as libc::mode_t);
                            meta::mknod(&dest_path, kind, perm, meta.rdev())
                                .map_err(|e| io_context(&dest_path, e))?;
                            receiver.copy_metadata(&path, &dest_path)?;
                            if created {
                                stats.files_transferred += 1;
                                if opts.itemize_changes && !opts.quiet {
                                    println!("cD+++++++++ {}", rel.display());
                                }
                            }
                        } else if file_type.is_fifo() && opts.specials {
                            use nix::sys::stat::Mode;
                            meta::mkfifo(&dest_path, Mode::from_bits_truncate(0o644))
                                .map_err(|e| io_context(&dest_path, e))?;
                            receiver.copy_metadata(&path, &dest_path)?;
                        }
                    }
                    if opts.remove_source_files {
                        remove_file_opts(&path, opts)?;
                    }
                }
            } else {
                continue;
            }
        }
    }
    sender.finish();
    receiver.finalize()?;
    if matches!(
        opts.delete,
        Some(DeleteMode::After) | Some(DeleteMode::During)
    ) {
        delete_extraneous(&src_root, dst, &matcher, opts, &mut stats)?;
    }
    for (src_dir, dest_dir) in dir_meta.into_iter().rev() {
        receiver.copy_metadata(&src_dir, &dest_dir)?;
    }
    if matches!(opts.modern_cdc, ModernCdc::Fastcdc) {
        manifest.save()?;
    }
    if let Some(mut f) = batch_file {
        let _ = writeln!(
            f,
            "files_transferred={} bytes_transferred={}",
            stats.files_transferred, stats.bytes_transferred
        );
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
        let delta = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap();
        let mut basis = Cursor::new(b"hello world".to_vec());
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
        let delta: Vec<Op> = compute_delta(
            &cfg,
            &mut basis_reader,
            &mut target_reader,
            3,
            usize::MAX,
            &SyncOptions::default(),
        )
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
        let mut progress = None;
        apply_delta(
            &mut basis_reader,
            delta.into_iter().map(Ok),
            &mut out,
            &SyncOptions::default(),
            0,
            &mut progress,
        )
        .unwrap();
        assert_eq!(out.into_inner(), basis);
    }

    #[test]
    fn emits_literal_for_new_data() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(Vec::new());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta: Vec<Op> = compute_delta(
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
        assert_eq!(delta, vec![Op::Data(b"abc".to_vec())]);
    }

    #[test]
    fn empty_target_yields_no_ops() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(Vec::new());
        let delta: Vec<Op> = compute_delta(
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
        assert!(delta.is_empty());
    }

    #[test]
    fn small_basis_matches() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"abc".to_vec());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta: Vec<Op> = compute_delta(
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
        assert_eq!(delta, vec![Op::Copy { offset: 0, len: 3 }]);
    }

    #[test]
    fn matches_partial_blocks() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(b"hello".to_vec());
        let delta: Vec<Op> = compute_delta(
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
        assert_eq!(
            delta,
            vec![
                Op::Copy { offset: 0, len: 4 },
                Op::Copy { offset: 4, len: 1 },
            ]
        );
    }

    #[test]
    fn last_good_block_detects_prefix() {
        let cfg = ChecksumConfigBuilder::new().build();
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");
        fs::write(&src, b"abcd1234").unwrap();
        fs::write(&dst, b"abcdxxxx").unwrap();
        assert_eq!(
            last_good_block(&cfg, &src, &dst, 4, &SyncOptions::default()).unwrap(),
            4
        );
    }

    #[test]
    fn sync_dir() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("a")).unwrap();

        fs::write(src.join("a/file1.txt"), vec![b'h'; 2048]).unwrap();
        fs::write(src.join("file2.txt"), vec![b'w'; 2048]).unwrap();

        sync(
            &src,
            &dst,
            &Matcher::default(),
            &available_codecs(None),
            &SyncOptions::default(),
        )
        .unwrap();
        assert_eq!(fs::read(dst.join("a/file1.txt")).unwrap(), vec![b'h'; 2048]);
        assert_eq!(fs::read(dst.join("file2.txt")).unwrap(), vec![b'w'; 2048]);
    }

    #[test]
    fn sync_skips_outside_paths() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("inside.txt"), b"inside").unwrap();

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
                    .process_file(&path, &dest_path, rel, &mut receiver)
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
}
