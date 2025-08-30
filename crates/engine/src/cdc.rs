// crates/engine/src/cdc.rs
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use blake3::Hash;
use checksums::Rolling;

const WINDOW: usize = 64;
const MIN_CHUNK: usize = 2 * 1024;
const AVG_CHUNK: usize = 8 * 1024;
const MASK: u32 = (AVG_CHUNK as u32) - 1;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub hash: Hash,
}

pub fn chunk_file(path: &Path) -> io::Result<Vec<Chunk>> {
    let data = fs::read(path)?;
    Ok(chunk_bytes(&data))
}

pub fn chunk_bytes(data: &[u8]) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    if data.is_empty() {
        return chunks;
    }
    if data.len() <= WINDOW {
        chunks.push(Chunk {
            hash: blake3::hash(data),
        });
        return chunks;
    }
    let mut start = 0usize;
    let mut roll = Rolling::new(&data[..WINDOW]);
    for i in WINDOW..data.len() {
        let out = data[i - WINDOW];
        let inp = data[i];
        roll.roll(out, inp);
        let size = i + 1 - start;
        if size >= MIN_CHUNK && (roll.digest() & MASK) == 0 {
            let chunk = &data[start..=i];
            chunks.push(Chunk {
                hash: blake3::hash(chunk),
            });
            start = i + 1;
        }
    }
    if start < data.len() {
        let chunk = &data[start..];
        chunks.push(Chunk {
            hash: blake3::hash(chunk),
        });
    }
    chunks
}

#[derive(Default)]
pub struct Manifest {
    entries: HashMap<String, PathBuf>,
    path: PathBuf,
}

impl Manifest {
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("."));
        let path = Path::new(&home).join(".rsync-rs/manifest");
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
