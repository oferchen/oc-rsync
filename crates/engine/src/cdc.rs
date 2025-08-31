// crates/engine/src/cdc.rs
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use blake3::Hash;
use fastcdc::v2020::FastCDC;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub hash: Hash,
}

pub fn chunk_file(path: &Path, min: usize, avg: usize, max: usize) -> io::Result<Vec<Chunk>> {
    let data = fs::read(path)?;
    Ok(chunk_bytes(&data, min, avg, max))
}

pub fn chunk_bytes(data: &[u8], min: usize, avg: usize, max: usize) -> Vec<Chunk> {
    if data.is_empty() {
        return Vec::new();
    }
    FastCDC::new(data, min as u32, avg as u32, max as u32)
        .into_iter()
        .map(|e| {
            let chunk = &data[e.offset..e.offset + e.length];
            Chunk {
                hash: blake3::hash(chunk),
            }
        })
        .collect()
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
        if !path.exists() {
            let legacy = Path::new(&home).join(".rsync-rs/manifest");
            if legacy.exists() {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::rename(&legacy, &path).is_err() {
                    if fs::copy(&legacy, &path).is_ok() {
                        let _ = fs::remove_file(&legacy);
                    }
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migrates_legacy_manifest() {
        let home = tempdir().unwrap();
        std::env::set_var("HOME", home.path());

        let legacy_dir = home.path().join(".rsync-rs");
        fs::create_dir_all(&legacy_dir).unwrap();
        let legacy_manifest = legacy_dir.join("manifest");

        let hash = blake3::hash(b"hello");
        fs::write(
            &legacy_manifest,
            format!("{} {}\n", hash.to_hex(), "/some/path"),
        )
        .unwrap();

        let manifest = Manifest::load();

        assert_eq!(manifest.lookup(&hash), Some(PathBuf::from("/some/path")));
        assert!(home.path().join(".oc-rsync/manifest").exists());
    }
}
