use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use checksums::{ChecksumConfig, ChecksumConfigBuilder};
use walk::walk;

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

/// Compute a delta from `basis` to `target` using a simple block matching
/// algorithm driven by the checksum crate.
fn compute_delta(cfg: &ChecksumConfig, basis: &[u8], target: &[u8], block_size: usize) -> Vec<Op> {
    let mut map: HashMap<u32, (Vec<u8>, usize)> = HashMap::new();
    let mut off = 0;
    while off < basis.len() {
        let end = usize::min(off + block_size, basis.len());
        let block = &basis[off..end];
        let sum = cfg.checksum(block);
        map.insert(sum.weak, (sum.strong, off));
        off = end;
    }

    let mut ops = Vec::new();
    let mut lit = Vec::new();
    let mut i = 0;
    while i < target.len() {
        let end = usize::min(i + block_size, target.len());
        let block = &target[i..end];
        let sum = cfg.checksum(block);
        if let Some((strong, off)) = map.get(&sum.weak) {
            if &sum.strong == strong {
                if !lit.is_empty() {
                    ops.push(Op::Data(lit.clone()));
                    lit.clear();
                }
                ops.push(Op::Copy {
                    offset: *off,
                    len: block.len(),
                });
                i += block.len();
                continue;
            }
        }
        lit.push(target[i]);
        i += 1;
    }
    if !lit.is_empty() {
        ops.push(Op::Data(lit));
    }
    ops
}

/// Apply a delta to `basis` returning the reconstructed data.
fn apply_delta(basis: &[u8], ops: &[Op]) -> Vec<u8> {
    let mut out = Vec::new();
    for op in ops {
        match op {
            Op::Data(d) => out.extend_from_slice(d),
            Op::Copy { offset, len } => {
                let end = offset + len;
                if end <= basis.len() {
                    out.extend_from_slice(&basis[*offset..end]);
                }
            }
        }
    }
    out
}

/// Sender responsible for walking the source tree and generating deltas.
pub struct Sender {
    state: SenderState,
    cfg: ChecksumConfig,
    block_size: usize,
}

impl Sender {
    pub fn new(block_size: usize) -> Self {
        Self {
            state: SenderState::Idle,
            cfg: ChecksumConfigBuilder::new().build(),
            block_size,
        }
    }

    fn start(&mut self) {
        self.state = SenderState::Walking;
    }

    fn finish(&mut self) {
        self.state = SenderState::Finished;
    }

    /// Generate a delta for `path` against `dest` and ask the receiver to apply it.
    fn process_file(&mut self, path: &Path, dest: &Path, recv: &mut Receiver) -> Result<()> {
        let src_data = fs::read(path)?;
        let basis = fs::read(dest).unwrap_or_default();
        let delta = compute_delta(&self.cfg, &basis, &src_data, self.block_size);
        recv.apply(dest, &basis, delta)
    }
}

/// Receiver responsible for applying deltas to the destination tree.
pub struct Receiver {
    state: ReceiverState,
}

impl Receiver {
    pub fn new() -> Self {
        Self {
            state: ReceiverState::Idle,
        }
    }

    fn apply(&mut self, dest: &Path, basis: &[u8], delta: Vec<Op>) -> Result<()> {
        self.state = ReceiverState::Applying;
        let data = apply_delta(basis, &delta);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, data)?;
        self.state = ReceiverState::Finished;
        Ok(())
    }
}

/// Synchronize the contents of directory `src` into `dst`.
pub fn sync(src: &Path, dst: &Path) -> Result<()> {
    let mut sender = Sender::new(1024);
    let mut receiver = Receiver::new();
    sender.start();
    for path in walk(src) {
        let rel: PathBuf = path.strip_prefix(src).unwrap().to_path_buf();
        let dest_path = dst.join(rel);
        sender.process_file(&path, &dest_path, &mut receiver)?;
    }
    sender.finish();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn delta_roundtrip() {
        let cfg = ChecksumConfigBuilder::new().build();
        let basis = b"hello world";
        let target = b"hello brave new world";
        let delta = compute_delta(&cfg, basis, target, 4);
        let out = apply_delta(basis, &delta);
        assert_eq!(out, target);
    }

    #[test]
    fn sync_dir() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("a")).unwrap();
        fs::write(src.join("a/file1.txt"), b"hello").unwrap();
        fs::write(src.join("file2.txt"), b"world").unwrap();

        sync(&src, &dst).unwrap();
        assert_eq!(fs::read(dst.join("a/file1.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(dst.join("file2.txt")).unwrap(), b"world");
    }
}
