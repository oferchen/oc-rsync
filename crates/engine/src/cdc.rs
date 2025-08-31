// crates/engine/src/cdc.rs
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use blake3::Hash;
use fastcdc::v2020::StreamCDC;

const RSYNC_BLOCK_SIZE: usize = 700;
const RSYNC_MAX_BLOCK_SIZE: usize = 1 << 17; // protocol >= 30

/// Calculate the delta block size using the same heuristics as upstream rsync.
///
/// The algorithm chooses a rounded square-root of the file length and caps the
/// result to `RSYNC_MAX_BLOCK_SIZE`.  Files smaller than `RSYNC_BLOCK_SIZE`
/// squared use the fixed `RSYNC_BLOCK_SIZE` default.  The returned value is
/// always a multiple of 8, matching rsync's behaviour.
pub fn block_size(len: u64) -> usize {
    if len <= (RSYNC_BLOCK_SIZE * RSYNC_BLOCK_SIZE) as u64 {
        return RSYNC_BLOCK_SIZE;
    }

    let mut c: usize = 1;
    let mut l = len;
    while l >> 2 > 0 {
        l >>= 2;
        c <<= 1;
    }

    if c >= RSYNC_MAX_BLOCK_SIZE || c == 0 {
        RSYNC_MAX_BLOCK_SIZE
    } else {
        let mut blength: usize = 0;
        while c >= 8 {
            blength |= c;
            if len < (blength as u64).wrapping_mul(blength as u64) {
                blength &= !c;
            }
            c >>= 1;
        }
        blength.max(RSYNC_BLOCK_SIZE)
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub hash: Hash,
}

pub fn chunk_file(path: &Path, min: usize, avg: usize, max: usize) -> io::Result<Vec<Chunk>> {
    let file = fs::File::open(path)?;
    chunk_reader(file, min, avg, max)
}

pub fn chunk_bytes<'a, I>(data: I, min: usize, avg: usize, max: usize) -> Vec<Chunk>
where
    I: IntoIterator<Item = &'a [u8]>,
{
    struct SliceReader<'a, I: Iterator<Item = &'a [u8]>> {
        iter: I,
        buf: &'a [u8],
    }

    impl<'a, I: Iterator<Item = &'a [u8]>> Read for SliceReader<'a, I> {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            let mut written = 0;
            while written < out.len() {
                if self.buf.is_empty() {
                    match self.iter.next() {
                        Some(next) => self.buf = next,
                        None => break,
                    }
                }
                let to_copy = (out.len() - written).min(self.buf.len());
                out[written..written + to_copy].copy_from_slice(&self.buf[..to_copy]);
                self.buf = &self.buf[to_copy..];
                written += to_copy;
            }
            Ok(written)
        }
    }

    let reader = SliceReader {
        iter: data.into_iter(),
        buf: &[],
    };
    chunk_reader(reader, min, avg, max).expect("SliceReader cannot fail")
}

fn chunk_reader<R: Read>(reader: R, min: usize, avg: usize, max: usize) -> io::Result<Vec<Chunk>> {
    let mut chunker = StreamCDC::new(reader, min as u32, avg as u32, max as u32);
    let mut chunks = Vec::new();
    while let Some(result) = chunker.next() {
        let data = result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        chunks.push(Chunk {
            hash: blake3::hash(&data.data),
        });
    }
    Ok(chunks)
}

#[derive(Default)]
pub struct Manifest {
    entries: HashMap<String, PathBuf>,
    path: PathBuf,
}

impl Manifest {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
        let path = Path::new(&home).join(".oc-rsync/manifest");
        let mut entries = HashMap::new();
        if let Ok(contents) = fs::read_to_string(&path) {
            for line in contents.lines() {
                let mut parts = line.splitn(2, ' ');
                if let (Some(hash), Some(p)) = (parts.next(), parts.next()) {
                    entries.insert(hash.to_string(), PathBuf::from(p));
                }
            }
        }
        Manifest { entries, path }
    }

    pub fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = String::new();
        for (hash, path) in &self.entries {
            out.push_str(hash);
            out.push(' ');
            out.push_str(&path.to_string_lossy());
            out.push('\n');
        }
        fs::write(&self.path, out)
    }

    pub fn lookup(&self, hash: &Hash) -> Option<PathBuf> {
        self.entries.get(&hash.to_hex().to_string()).cloned()
    }

    pub fn insert(&mut self, hash: &Hash, path: &Path) {
        self.entries
            .insert(hash.to_hex().to_string(), path.to_path_buf());
    }
}
