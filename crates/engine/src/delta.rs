// crates/engine/src/delta.rs

use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::Duration;

use checksums::ChecksumConfig;
use logging::{InfoFlag, progress_formatter, rate_formatter};

use crate::{EngineError, Result, SyncOptions, ensure_max_alloc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Data(Vec<u8>),
    Copy { offset: usize, len: usize },
}

pub(crate) const DEFAULT_BASIS_WINDOW: usize = 8 * 1024;
pub(crate) const LIT_CAP: usize = 1 << 20;

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
                use nix::fcntl::{FallocateFlags, fallocate};

                let offset = file.stream_position()?;
                let _ = fallocate(
                    &*file,
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

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub(crate) struct Progress {
    total: u64,
    written: u64,
    start: std::time::Instant,
    last_print: std::time::Instant,
    human_readable: bool,
    quiet: bool,
    file_idx: usize,
}

pub(crate) static TOTAL_FILES: AtomicUsize = AtomicUsize::new(0);
pub(crate) static FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);
pub(crate) static PROGRESS_HEADER: AtomicBool = AtomicBool::new(false);

const PROGRESS_UPDATE_INTERVAL: Duration = Duration::from_secs(1);

impl Progress {
    pub(crate) fn new(
        dest: &Path,
        total: u64,
        human_readable: bool,
        initial: u64,
        quiet: bool,
    ) -> Self {
        if !quiet {
            use std::io::Write as _;
            if !PROGRESS_HEADER.swap(true, Ordering::SeqCst) {
                println!("sending incremental file list");
            }
            if let Some(name) = dest.file_name() {
                println!("{}", name.to_string_lossy());
            } else {
                println!("{}", dest.display());
            }
            let _ = std::io::stdout().flush();
        }
        let now = std::time::Instant::now();
        let idx = FILE_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
        Self {
            total,
            written: initial,
            start: now,
            last_print: now - PROGRESS_UPDATE_INTERVAL,
            human_readable,
            quiet,
            file_idx: idx,
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

    pub(crate) fn finish(&mut self) {
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
        let elapsed = self.start.elapsed().as_secs_f64();
        let rate_val = if elapsed > 0.0 {
            self.written as f64 / elapsed
        } else {
            0.0
        };
        let rate = rate_formatter(rate_val);
        let secs = self.start.elapsed().as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        let time = format!("{:02}:{:02}:{:02}", h, m, s);
        let total_files = TOTAL_FILES.load(Ordering::SeqCst);
        let remaining = total_files.saturating_sub(self.file_idx);
        tracing::info!(
            target: InfoFlag::Progress.target(),
            written = self.written,
            total = self.total,
            percent,
            rate = rate.as_str()
        );
        let line = format!(
            "\r{:>15} {:>3}% {} {} (xfr#{}, to-chk={}/{})",
            bytes, percent, rate, time, self.file_idx, remaining, total_files
        );
        if done {
            println!("{line}");
        } else {
            print!("{line}");
            let _ = std::io::stdout().flush();
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

pub(crate) fn apply_delta<R: Read + Seek, W: Write + Seek + Any, I>(
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
