// crates/walk/src/lib.rs
use std::collections::HashMap;
use std::fs::FileType;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const MIN_FILE_SIZE: u64 = 1024;
#[derive(Debug, Clone)]
pub struct Entry {
    pub prefix_len: usize,
    pub suffix: String,
    pub file_type: FileType,
    pub uid: usize,
    pub gid: usize,
    pub dev: usize,
    pub inode: usize,
}

impl Entry {
    pub fn apply(&self, state: &mut String) -> PathBuf {
        state.truncate(self.prefix_len);
        state.push_str(&self.suffix);
        PathBuf::from(state.clone())
    }
}

pub struct Walk {
    iter: walkdir::IntoIter,
    prev_path: String,
    batch_size: usize,
    max_file_size: Option<u64>,
    uid_map: HashMap<u32, usize>,
    uid_table: Vec<u32>,
    gid_map: HashMap<u32, usize>,
    gid_table: Vec<u32>,
    dev_map: HashMap<u64, usize>,
    dev_table: Vec<u64>,
    inode_map: HashMap<(usize, u64), usize>,
    inode_table: Vec<u64>,
}

impl Walk {
    fn new(root: PathBuf, batch_size: usize, max_file_size: Option<u64>) -> Self {
        Walk {
            iter: WalkDir::new(root)
                .sort_by(|a, b| a.file_name().cmp(b.file_name()))
                .into_iter(),
            prev_path: String::new(),
            batch_size,
            max_file_size,
            uid_map: HashMap::new(),
            uid_table: Vec::new(),
            gid_map: HashMap::new(),
            gid_table: Vec::new(),
            dev_map: HashMap::new(),
            dev_table: Vec::new(),
            inode_map: HashMap::new(),
            inode_table: Vec::new(),
        }
    }

    pub fn uids(&self) -> &[u32] {
        &self.uid_table
    }

    pub fn gids(&self) -> &[u32] {
        &self.gid_table
    }

    pub fn devs(&self) -> &[u64] {
        &self.dev_table
    }

    pub fn inodes(&self) -> &[u64] {
        &self.inode_table
    }
}

pub fn walk(root: impl AsRef<Path>, batch_size: usize) -> Walk {
    Walk::new(root.as_ref().to_path_buf(), batch_size, None)
}

pub fn walk_with_max_size(root: impl AsRef<Path>, batch_size: usize, max_file_size: u64) -> Walk {
    Walk::new(root.as_ref().to_path_buf(), batch_size, Some(max_file_size))
}

impl Iterator for Walk {
    type Item = io::Result<Vec<Entry>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = Vec::new();
        while batch.len() < self.batch_size {
            match self.iter.next() {
                Some(Ok(entry)) => {
                    let path = entry.path().to_string_lossy().into_owned();
                    let prefix = common_prefix_len(&self.prev_path, &path);
                    let suffix = path[prefix..].to_string();

                    let meta = match std::fs::symlink_metadata(entry.path()) {
                        Ok(m) => m,
                        Err(e) => return Some(Err(e)),
                    };
                    let len = meta.len();
                    if entry.file_type().is_file() && len < MIN_FILE_SIZE {
                        continue;
                    }

                    if let Some(max) = self.max_file_size {
                        if meta.is_file() && meta.len() > max {
                            continue;
                        }
                    }

                    #[cfg(unix)]
                    let (uid, gid, dev, ino) = (meta.uid(), meta.gid(), meta.dev(), meta.ino());
                    #[cfg(not(unix))]
                    let (uid, gid, dev, ino) = (0u32, 0u32, 0u64, 0u64);

                    let uid_idx = *self.uid_map.entry(uid).or_insert_with(|| {
                        self.uid_table.push(uid);
                        self.uid_table.len() - 1
                    });
                    let gid_idx = *self.gid_map.entry(gid).or_insert_with(|| {
                        self.gid_table.push(gid);
                        self.gid_table.len() - 1
                    });
                    let dev_idx = *self.dev_map.entry(dev).or_insert_with(|| {
                        self.dev_table.push(dev);
                        self.dev_table.len() - 1
                    });
                    let inode_idx = *self.inode_map.entry((dev_idx, ino)).or_insert_with(|| {
                        self.inode_table.push(ino);
                        self.inode_table.len() - 1
                    });

                    batch.push(Entry {
                        prefix_len: prefix,
                        suffix,
                        file_type: entry.file_type(),
                        uid: uid_idx,
                        gid: gid_idx,
                        dev: dev_idx,
                        inode: inode_idx,
                    });

                    self.prev_path = path;
                }
                Some(Err(err)) => {
                    let msg = err.to_string();
                    let io_err = match err.into_io_error() {
                        Some(inner) => inner,
                        None => io::Error::other(msg),
                    };
                    return Some(Err(io_err));
                }
                None => break,
            }
        }

        if batch.is_empty() {
            None
        } else {
            Some(Ok(batch))
        }
    }
}

fn common_prefix_len(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}
