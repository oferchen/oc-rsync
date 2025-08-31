// crates/engine/src/lib.rs
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Cursor, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use checksums::{ChecksumConfig, ChecksumConfigBuilder};
pub use checksums::{ModernHash, StrongHash};
#[cfg(feature = "lz4")]
use compress::Lz4;
use compress::{
    available_codecs, should_compress, Codec, Compressor, Decompressor, ModernCompress, Zlib, Zstd,
};
use filters::Matcher;
use thiserror::Error;

mod cdc;
use cdc::{chunk_file, Manifest};

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
use walk::walk;

trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 9] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{}{}", bytes, UNITS[unit])
    } else {
        format!("{:.2}{}", size, UNITS[unit])
    }
}

fn ensure_max_alloc(len: u64, opts: &SyncOptions) -> Result<()> {
    if len > opts.max_alloc as u64 {
        Err(EngineError::Other("max-alloc limit exceeded".into()))
    } else {
        Ok(())
    }
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
                    if &ba[..ra] != &bb[..rb] {
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

fn last_good_block(cfg: &ChecksumConfig, src: &Path, dst: &Path, block_size: usize) -> u64 {
    let block_size = block_size.max(1);
    let mut src = match File::open(src) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let mut dst = match File::open(dst) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let mut offset = 0u64;
    let mut src_buf = vec![0u8; block_size];
    let mut dst_buf = vec![0u8; block_size];
    loop {
        let rs = match src.read(&mut src_buf) {
            Ok(n) => n,
            Err(_) => break,
        };
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
    offset - (offset % block_size as u64)
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
) -> Result<DeltaIter<'a, R2>> {
    let block_size = block_size.max(1);
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
            let mut j = i + 1;
            while j < data.len() && data[j] == 0 {
                j += 1;
            }
            file.seek(SeekFrom::Current((j - i) as i64))?;
            i = j;
        } else {
            let mut j = i + 1;
            while j < data.len() && data[j] != 0 {
                j += 1;
            }
            file.write_all(&data[i..j])?;
            i = j;
        }
    }
    Ok(())
}

fn apply_op_plain<R: Read + Seek, W: Write + Seek>(
    basis: &mut R,
    op: Op,
    out: &mut W,
    buf: &mut [u8],
) -> Result<()> {
    match op {
        Op::Data(d) => {
            out.write_all(&d)?;
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

fn apply_op_inplace<R: Read + Seek>(
    basis: &mut R,
    op: Op,
    out: &mut File,
    buf: &mut [u8],
) -> Result<()> {
    match op {
        Op::Data(d) => {
            out.write_all(&d)?;
        }
        Op::Copy { offset, len } => {
            let pos = out.stream_position()?;
            if offset as u64 == pos {
                out.seek(SeekFrom::Current(len as i64))?;
            } else {
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
    }
    Ok(())
}

fn apply_op_sparse<R: Read + Seek>(
    basis: &mut R,
    op: Op,
    out: &mut File,
    buf: &mut [u8],
) -> Result<()> {
    match op {
        Op::Data(d) => {
            write_sparse(out, &d)?;
        }
        Op::Copy { offset, len } => {
            basis.seek(SeekFrom::Start(offset as u64))?;
            let mut remaining = len;
            while remaining > 0 {
                let to_read = remaining.min(buf.len());
                basis.read_exact(&mut buf[..to_read])?;
                write_sparse(out, &buf[..to_read])?;
                remaining -= to_read;
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
) -> Result<()>
where
    I: IntoIterator<Item = Result<Op>>,
{
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
                apply_op_inplace(basis, op, file, &mut buf)?;
            }
        }
    } else if opts.sparse {
        let file = (&mut *out as &mut dyn Any)
            .downcast_mut::<File>()
            .ok_or_else(|| EngineError::Other("sparse output must be a File".into()))?;
        for op in ops {
            let op = op?;
            if let Some(op) = adjust(op) {
                apply_op_sparse(basis, op, file, &mut buf)?;
            }
        }
        let pos = file.stream_position()?;
        file.set_len(pos)?;
    } else {
        for op in ops {
            let op = op?;
            if let Some(op) = adjust(op) {
                apply_op_plain(basis, op, out, &mut buf)?;
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
        let block_size = block_size.max(1);
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
        let data = fs::read(path)?;
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
                    return sm == dm;
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

        let src_len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        ensure_max_alloc(src_len, &self.opts)?;
        let src = File::open(path)?;
        let mut src_reader = BufReader::new(src);
        let file_codec = if should_compress(path, &self.opts.skip_compress) {
            self.codec
        } else {
            None
        };
        let file_name = dest
            .file_name()
            .ok_or_else(|| EngineError::Other("destination has no file name".into()))?;
        let partial_path = if let Some(dir) = &self.opts.partial_dir {
            dir.join(file_name).with_extension("partial")
        } else {
            dest.with_extension("partial")
        };
        let basis_path = if self.opts.partial && partial_path.exists() {
            partial_path.clone()
        } else {
            dest.to_path_buf()
        };
        let mut resume = if self.opts.partial && partial_path.exists() {
            last_good_block(&self.cfg, path, &partial_path, self.block_size)
        } else if self.opts.append || self.opts.append_verify {
            if self.opts.append_verify {
                last_good_block(&self.cfg, path, dest, self.block_size)
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
        let mut buf: Vec<u8> = Vec::new();
        let delta: Box<dyn Iterator<Item = Result<Op>> + '_> = if self.opts.whole_file {
            let len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            ensure_max_alloc(len, &self.opts)?;
            src_reader.read_to_end(&mut buf)?;
            Box::new(std::iter::once(Ok(Op::Data(buf))))
        } else {
            Box::new(compute_delta(
                &self.cfg,
                &mut basis_reader,
                &mut src_reader,
                self.block_size,
                DEFAULT_BASIS_WINDOW,
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
                fs::create_dir_all(parent)?;
            }
            fs::rename(dest, &backup_path)?;
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
        recv.apply(path, dest, ops)?;
        Ok(true)
    }
}

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
        let file_name = dest
            .file_name()
            .ok_or_else(|| EngineError::Other("destination has no file name".into()))?;
        let partial = if let Some(dir) = &self.opts.partial_dir {
            dir.join(file_name).with_extension("partial")
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
        let tmp_dest = if self.opts.inplace {
            dest.to_path_buf()
        } else if let Some(dir) = &self.opts.temp_dir {
            dir.join(file_name).with_extension("tmp")
        } else if self.opts.partial {
            partial.clone()
        } else {
            dest.to_path_buf()
        };
        let cfg = ChecksumConfigBuilder::new()
            .strong(self.opts.strong)
            .seed(self.opts.checksum_seed)
            .build();
        let mut resume = if self.opts.partial || self.opts.append || self.opts.append_verify {
            if self.opts.append && !self.opts.append_verify {
                fs::metadata(&tmp_dest).map(|m| m.len()).unwrap_or(0)
            } else {
                last_good_block(&cfg, src, &tmp_dest, self.opts.block_size)
            }
        } else {
            0
        };
        let src_len = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        if resume > src_len {
            resume = src_len;
        }
        let mut basis: Box<dyn ReadSeek> = match File::open(&basis_path) {
            Ok(mut f) => {
                let len = f.metadata().map(|m| m.len()).unwrap_or(0);
                ensure_max_alloc(len, &self.opts)?;
                let mut buf = Vec::with_capacity(len as usize);
                f.read_to_end(&mut buf)?;
                Box::new(Cursor::new(buf))
            }
            Err(_) => Box::new(Cursor::new(Vec::new())),
        };
        if let Some(parent) = tmp_dest.parent() {
            fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        if !self.opts.write_devices {
            if let Ok(meta) = fs::symlink_metadata(&tmp_dest) {
                let ft = meta.file_type();
                if ft.is_block_device() || ft.is_char_device() {
                    if self.opts.copy_devices {
                        fs::remove_file(&tmp_dest)?;
                    } else {
                        return Err(EngineError::Other(
                            "refusing to write to device; use --write-devices".into(),
                        ));
                    }
                }
            }
        }
        let mut out = if self.opts.inplace
        let cfg = ChecksumConfigBuilder::new()
            .strong(self.opts.strong)
            .seed(self.opts.checksum_seed)
            .build();
        let mut resume = if self.opts.partial || self.opts.append || self.opts.append_verify {
            if self.opts.append && !self.opts.append_verify {
                fs::metadata(&tmp_dest).map(|m| m.len()).unwrap_or(0)
            } else {
                last_good_block(&cfg, src, &tmp_dest, self.opts.block_size)
            }
        } else {
            0
        };
        let src_len = fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        if resume > src_len {
            resume = src_len;
        }
        let mut out = if self.opts.write_devices {
            OpenOptions::new().write(true).open(&tmp_dest)?
        } else if self.opts.inplace
            || self.opts.partial
            || self.opts.append
            || self.opts.append_verify
        {
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&tmp_dest)?
        } else {
            File::create(&tmp_dest)?
        };
        if !self.opts.write_devices {
            out.set_len(resume)?;
            out.seek(SeekFrom::Start(resume))?;
        }
        let file_codec = if should_compress(src, &self.opts.skip_compress) {
            self.codec
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
        apply_delta(&mut basis, ops, &mut out, &self.opts, 0)?;
        if !self.opts.write_devices {
            let len = out.seek(SeekFrom::Current(0))?;
            out.set_len(len)?;
        }
        if !self.opts.inplace && (self.opts.partial || self.opts.temp_dir.is_some()) {
            fs::rename(&tmp_dest, dest)?;
        }
        self.copy_metadata(src, dest)?;
        if self.opts.progress {
            let len = fs::metadata(dest)?.len();
            if self.opts.human_readable {
                eprintln!("{}: {}", dest.display(), human_bytes(len));
            } else {
                eprintln!("{}: {} bytes", dest.display(), len);
            }
        }
        self.state = ReceiverState::Finished;
        Ok(())
    }
}

impl Receiver {
    fn copy_metadata(&self, src: &Path, dest: &Path) -> Result<()> {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            let chown_uid = self.opts.chown.map(|(u, _)| u).flatten();
            let chown_gid = self.opts.chown.map(|(_, g)| g).flatten();

            let uid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>> =
                if let Some(uid) = chown_uid {
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
                if let Some(gid) = chown_gid {
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

            let meta_opts = meta::Options {
                xattrs: {
                    #[cfg(feature = "xattr")]
                    {
                        self.opts.xattrs
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
            };

            if meta_opts.needs_metadata() {
                let meta =
                    meta::Metadata::from_path(src, meta_opts.clone()).map_err(EngineError::from)?;
                meta.apply(dest, meta_opts).map_err(EngineError::from)?;
            }
        }
        let _ = (src, dest);
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
    pub max_delete: Option<usize>,
    pub max_alloc: usize,
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
    pub copy_unsafe_links: bool,
    pub safe_links: bool,
    pub hard_links: bool,
    pub devices: bool,
    pub specials: bool,
    #[cfg(feature = "xattr")]
    pub xattrs: bool,
    #[cfg(feature = "acl")]
    pub acls: bool,
    pub sparse: bool,
    pub strong: StrongHash,
    /// Seed value mixed into rolling and strong checksums.
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
    pub bwlimit: Option<u64>,
    pub block_size: usize,
    pub link_dest: Option<PathBuf>,
    pub copy_dest: Option<PathBuf>,
    pub compare_dest: Option<PathBuf>,
    pub backup: bool,
    pub backup_dir: Option<PathBuf>,
    pub chmod: Option<Vec<meta::Chmod>>,
    pub chown: Option<(Option<u32>, Option<u32>)>,
    pub eight_bit_output: bool,
    pub blocking_io: bool,
    pub early_input: Option<PathBuf>,
    pub secluded_args: bool,
    pub sockopts: Vec<String>,
    pub write_batch: Option<PathBuf>,
    pub copy_devices: bool,
    pub write_devices: bool,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            delete: None,
            delete_excluded: false,
            max_delete: None,
            max_alloc: 1 << 30,
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
            copy_unsafe_links: false,
            safe_links: false,
            hard_links: false,
            devices: false,
            specials: false,
            #[cfg(feature = "xattr")]
            xattrs: false,
            #[cfg(feature = "acl")]
            acls: false,
            sparse: false,
            dirs: false,
            list_only: false,
            update: false,
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
            bwlimit: None,
            block_size: 1024,
            link_dest: None,
            copy_dest: None,
            compare_dest: None,
            backup: false,
            backup_dir: None,
            chmod: None,
            chown: None,
            eight_bit_output: false,
            blocking_io: false,
            early_input: None,
            secluded_args: false,
            sockopts: Vec::new(),
            write_batch: None,
            copy_devices: false,
            write_devices: false,
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
    let mut walker = walk(dst, 1024);
    let mut state = String::new();
    while let Some(batch) = walker.next() {
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
                    if opts.backup {
                        let backup_path = if let Some(ref dir) = opts.backup_dir {
                            dir.join(rel)
                        } else {
                            let name = path
                                .file_name()
                                .map(|n| format!("{}~", n.to_string_lossy()))
                                .unwrap_or_else(|| "~".to_string());
                            path.with_file_name(name)
                        };
                        if let Some(parent) = backup_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        fs::rename(&path, &backup_path)?;
                    } else if file_type.is_dir() {
                        fs::remove_dir_all(&path)?;
                    } else {
                        fs::remove_file(&path)?;
                    }
                    stats.files_deleted += 1;
                }
            }
        }
    }
    Ok(())
}

pub fn sync(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    remote: &[Codec],
    opts: &SyncOptions,
) -> Result<Stats> {
    if opts.list_only {
        let matcher = matcher.clone().with_root(src.to_path_buf());
        let mut walker = walk(src, 1024);
        let mut state = String::new();
        while let Some(batch) = walker.next() {
            let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
            for entry in batch {
                let path = entry.apply(&mut state);
                if let Ok(rel) = path.strip_prefix(src) {
                    if !matcher
                        .is_included(rel)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?
                    {
                        continue;
                    }
                    if opts.dirs && !entry.file_type.is_dir() {
                        continue;
                    }
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
        return Ok(Stats::default());
    }

    let codec = select_codec(remote, opts);
    let matcher = matcher.clone().with_root(src.to_path_buf());
    let mut sender = Sender::new(opts.block_size, matcher.clone(), codec, opts.clone());
    let mut receiver = Receiver::new(codec, opts.clone());
    let mut stats = Stats::default();
    let mut manifest = if matches!(opts.modern_cdc, ModernCdc::Fastcdc) {
        Manifest::load()
    } else {
        Manifest::default()
    };
    if matches!(opts.delete, Some(DeleteMode::Before)) {
        delete_extraneous(src, dst, &matcher, opts, &mut stats)?;
    }
    sender.start();
    #[cfg(unix)]
    let mut hard_links: HashMap<(u64, u64), std::path::PathBuf> = HashMap::new();
    let mut walker = walk(src, 1024);
    let mut state = String::new();
    while let Some(batch) = walker.next() {
        let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
        for entry in batch {
            let path = entry.apply(&mut state);
            let file_type = entry.file_type;
            if let Ok(rel) = path.strip_prefix(src) {
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
                    if opts.ignore_existing && dest_path.exists() {
                        continue;
                    }
                    if opts.update && dest_path.exists() {
                        if let (Ok(src_meta), Ok(dst_meta)) =
                            (fs::metadata(&path), fs::metadata(&dest_path))
                        {
                            if let (Ok(src_mtime), Ok(dst_mtime)) =
                                (src_meta.modified(), dst_meta.modified())
                            {
                                if dst_mtime > src_mtime {
                                    continue;
                                }
                            }
                        }
                    }
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
                    let partial_exists = if opts.partial {
                        let partial_path = if let Some(ref dir) = opts.partial_dir {
                            dir.join(rel).with_extension("partial")
                        } else {
                            dest_path.with_extension("partial")
                        };
                        partial_path.exists()
                    } else {
                        false
                    };
                    if !dest_path.exists() && !partial_exists {
                        if matches!(opts.modern_cdc, ModernCdc::Fastcdc) {
                            if let Ok(chunks) = chunk_file(
                                &path,
                                opts.modern_cdc_min,
                                (opts.modern_cdc_min + opts.modern_cdc_max) / 2,
                                opts.modern_cdc_max,
                            ) {
                                if !chunks.is_empty() {
                                    if let Some(existing) = manifest.lookup(&chunks[0].hash) {
                                        let all = chunks
                                            .iter()
                                            .all(|c| manifest.lookup(&c.hash).is_some());
                                        if all {
                                            if let Some(parent) = dest_path.parent() {
                                                fs::create_dir_all(parent)?;
                                            }
                                            fs::copy(&existing, &dest_path)?;
                                            stats.files_transferred += 1;
                                            if opts.itemize_changes {
                                                println!(">f+++++++++ {}", rel.display());
                                            }
                                            for c in &chunks {
                                                manifest.insert(&c.hash, &dest_path);
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
                                    fs::create_dir_all(parent)?;
                                }
                                fs::hard_link(&link_path, &dest_path)?;
                                continue;
                            }
                        }
                        if let Some(ref copy_dir) = opts.copy_dest {
                            let copy_path = copy_dir.join(rel);
                            if files_identical(&path, &copy_path) {
                                if let Some(parent) = dest_path.parent() {
                                    fs::create_dir_all(parent)?;
                                }
                                fs::copy(&copy_path, &dest_path)?;
                                continue;
                            }
                        }
                        if let Some(ref compare_dir) = opts.compare_dest {
                            let comp_path = compare_dir.join(rel);
                            if files_identical(&path, &comp_path) {
                                continue;
                            }
                        }
                    }
                    if sender.process_file(&path, &dest_path, rel, &mut receiver)? {
                        stats.files_transferred += 1;
                        stats.bytes_transferred += fs::metadata(&path)?.len();
                        if opts.itemize_changes {
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
                } else if file_type.is_dir() {
                    let created = !dest_path.exists();
                    fs::create_dir_all(&dest_path)?;
                    if created && opts.itemize_changes {
                        println!("cd+++++++++ {}/", rel.display());
                    }
                    receiver.copy_metadata(&path, &dest_path)?;
                } else if file_type.is_symlink() {
                    let created = !dest_path.exists();
                    let target = fs::read_link(&path)?;
                    let target_path = if target.is_absolute() {
                        target.clone()
                    } else {
                        path.parent().unwrap_or(Path::new("")).join(&target)
                    };
                    let is_unsafe = match fs::canonicalize(&target_path) {
                        Ok(p) => !p.starts_with(src),
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
                                    fs::create_dir_all(parent)?;
                                }
                                let sub = sync(&target_path, &dest_path, &matcher, remote, opts)?;
                                stats.files_transferred += sub.files_transferred;
                                stats.files_deleted += sub.files_deleted;
                                stats.bytes_transferred += sub.bytes_transferred;
                            } else if meta.is_file() {
                                if sender.process_file(
                                    &target_path,
                                    &dest_path,
                                    rel,
                                    &mut receiver,
                                )? {
                                    stats.files_transferred += 1;
                                    stats.bytes_transferred += meta.len();
                                }
                            }
                        }
                    } else if opts.links {
                        if let Some(parent) = dest_path.parent() {
                            fs::create_dir_all(parent)?;
                        }
                        #[cfg(unix)]
                        std::os::unix::fs::symlink(&target, &dest_path)?;
                        #[cfg(windows)]
                        {
                            if meta.map_or(false, |m| m.is_dir()) {
                                std::os::windows::fs::symlink_dir(&target, &dest_path)?;
                            } else {
                                std::os::windows::fs::symlink_file(&target, &dest_path)?;
                            }
                        }
                        receiver.copy_metadata(&path, &dest_path)?;
                        if created {
                            stats.files_transferred += 1;
                            if opts.itemize_changes {
                                println!("cL+++++++++ {} -> {}", rel.display(), target.display());
                            }
                        }
                    }
                } else {
                    #[cfg(unix)]
                    {
                        if (file_type.is_char_device() || file_type.is_block_device())
                            && opts.devices
                            && !opts.copy_devices
                        {
                            use nix::sys::stat::{mknod, Mode, SFlag};
                            let meta = fs::symlink_metadata(&path)?;
                            let kind = if file_type.is_char_device() {
                                SFlag::S_IFCHR
                            } else {
                                SFlag::S_IFBLK
                            };
                            use nix::libc::{dev_t, mode_t};

                            let perm_bits: mode_t = mode_t::try_from(meta.mode() & 0o777)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                            let perm = Mode::from_bits_truncate(perm_bits);
                            let rdev: dev_t = dev_t::try_from(meta.rdev())
                                .map_err(|e| EngineError::Other(e.to_string()))?;

                            mknod(&dest_path, kind, perm, rdev)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                            receiver.copy_metadata(&path, &dest_path)?;
                        } else if file_type.is_fifo() && opts.specials {
                            use nix::sys::stat::Mode;
                            use nix::unistd::mkfifo;
                            mkfifo(&dest_path, Mode::from_bits_truncate(0o644))
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                            receiver.copy_metadata(&path, &dest_path)?;
                        }
                    }
                }
            } else {
                continue;
            }
        }
    }
    sender.finish();
    if matches!(
        opts.delete,
        Some(DeleteMode::After) | Some(DeleteMode::During)
    ) {
        delete_extraneous(src, dst, &matcher, opts, &mut stats)?;
    }
    if matches!(opts.modern_cdc, ModernCdc::Fastcdc) {
        manifest.save()?;
    }
    if let Some(batch) = &opts.write_batch {
        if let Ok(mut f) = File::create(batch) {
            let _ = writeln!(
                f,
                "files_transferred={} bytes_transferred={}",
                stats.files_transferred, stats.bytes_transferred
            );
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
        apply_delta(&mut basis, delta, &mut out, &SyncOptions::default(), 0).unwrap();
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
            0,
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
    fn last_good_block_detects_prefix() {
        let cfg = ChecksumConfigBuilder::new().build();
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");
        fs::write(&src, b"abcd1234").unwrap();
        fs::write(&dst, b"abcdxxxx").unwrap();
        assert_eq!(last_good_block(&cfg, &src, &dst, 4), 4);
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
            &available_codecs(None),
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
        let delta = compute_delta(&cfg, &mut basis, &mut target, block_size, window).unwrap();
        let after = mem_usage_kb();
        let mut basis = Cursor::new(data.clone());
        let mut out = Cursor::new(Vec::new());
        apply_delta(&mut basis, delta, &mut out, &SyncOptions::default(), 0).unwrap();
        assert_eq!(out.into_inner(), data);
        let growth = after.saturating_sub(before);
        assert!(growth < 10 * 1024, "memory grew too much: {}KB", growth);
    }
}
