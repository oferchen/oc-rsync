// crates/engine/src/cdc.rs
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use blake3::Hash;
use fastcdc::v2020::StreamCDC;
use memmap2::MmapOptions;

const RSYNC_BLOCK_SIZE: usize = 700;
const RSYNC_MAX_BLOCK_SIZE: usize = 1 << 17;

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
    let chunker = StreamCDC::new(reader, min as u32, avg as u32, max as u32);
    let mut chunks = Vec::new();
    for result in chunker {
        let data = result.map_err(io::Error::other)?;
        chunks.push(Chunk {
            hash: blake3::hash(&data.data),
        });
    }
    Ok(chunks)
}

#[derive(Default)]
pub struct Manifest {
    entries: HashMap<Hash, HashSet<PathBuf>>,
    path: PathBuf,
    file: Option<fs::File>,
}

impl Manifest {
    pub fn parse(data: &[u8]) -> Vec<(Hash, PathBuf)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i + 36 <= data.len() {
            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&data[i..i + 32]);
            i += 32;
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&data[i..i + 4]);
            i += 4;
            let path_len = u32::from_le_bytes(len_bytes) as usize;
            if i + path_len > data.len() {
                break;
            }
            if let Ok(s) = std::str::from_utf8(&data[i..i + path_len]) {
                out.push((Hash::from_bytes(hash_bytes), PathBuf::from(s)));
            }
            i += path_len;
        }
        out
    }

    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
        let path = Path::new(&home).join(".oc-rsync/manifest");
        let mut entries: HashMap<Hash, HashSet<PathBuf>> = HashMap::new();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(&path);
        let mut file_opt = None;
        if let Ok(f) = file {
            if let Ok(meta) = f.metadata() {
                if meta.len() > 0 {
                    if let Ok(mmap) = unsafe { MmapOptions::new().map(&f) } {
                        for (h, p) in Self::parse(&mmap) {
                            entries.entry(h).or_default().insert(p);
                        }
                    }
                }
            }
            file_opt = Some(f);
        }
        Manifest {
            entries,
            path,
            file: file_opt,
        }
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(f) = self.file.as_mut() {
            f.sync_all()?;
        }
        Ok(())
    }

    pub fn lookup(&self, hash: &Hash, path: &Path) -> Option<PathBuf> {
        self.entries.get(hash).and_then(|set| {
            if set.contains(path) {
                Some(path.to_path_buf())
            } else {
                set.iter().next().cloned()
            }
        })
    }

    pub fn insert(&mut self, hash: &Hash, path: &Path) {
        let set = self.entries.entry(*hash).or_default();
        if set.insert(path.to_path_buf()) {
            if let Some(f) = self.file.as_mut() {
                if let Some(parent) = self.path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let p = path.to_string_lossy();
                let mut buf = Vec::with_capacity(32 + 4 + p.len());
                buf.extend_from_slice(hash.as_bytes());
                buf.extend_from_slice(&(p.len() as u32).to_le_bytes());
                buf.extend_from_slice(p.as_bytes());
                let _ = f.write_all(&buf);
            }
        }
    }
}
