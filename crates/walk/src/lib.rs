use std::collections::HashMap;
use std::fs::FileType;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Entry produced by the directory walker.
///
/// Paths are delta encoded relative to the previous entry. `prefix_len`
/// indicates how many bytes from the previous path should be reused and
/// `suffix` is appended to form the current path. The `uid`, `gid` and `dev`
/// fields reference indexes into the corresponding tables accumulated by
/// [`Walk`].
#[derive(Debug, Clone)]
pub struct Entry {
    /// Number of bytes from the previous path to keep.
    pub prefix_len: usize,
    /// Remaining path bytes to append.
    pub suffix: String,
    /// File type for this entry.
    pub file_type: FileType,
    /// Index into the UID table.
    pub uid: usize,
    /// Index into the GID table.
    pub gid: usize,
    /// Index into the device table.
    pub dev: usize,
}

impl Entry {
    /// Reconstruct the full path using `state` as the previous path buffer.
    /// The buffer is updated to the current path and the full [`PathBuf`] is
    /// returned.
    pub fn apply(&self, state: &mut String) -> PathBuf {
        state.truncate(self.prefix_len);
        state.push_str(&self.suffix);
        PathBuf::from(state.clone())
    }
}

/// Generator walking a directory tree and yielding batches of delta encoded
/// [`Entry`] values. Batching bounds memory usage while preserving the
/// ordering semantics of `rsync`/`walkdir` (preorder traversal).
pub struct Walk {
    iter: walkdir::IntoIter,
    prev_path: String,
    batch_size: usize,
    uid_map: HashMap<u32, usize>,
    uid_table: Vec<u32>,
    gid_map: HashMap<u32, usize>,
    gid_table: Vec<u32>,
    dev_map: HashMap<u64, usize>,
    dev_table: Vec<u64>,
}

impl Walk {
    fn new(root: PathBuf, batch_size: usize) -> Self {
        Walk {
            iter: WalkDir::new(root)
                .sort_by(|a, b| a.file_name().cmp(b.file_name()))
                .into_iter(),
            prev_path: String::new(),
            batch_size,
            uid_map: HashMap::new(),
            uid_table: Vec::new(),
            gid_map: HashMap::new(),
            gid_table: Vec::new(),
            dev_map: HashMap::new(),
            dev_table: Vec::new(),
        }
    }

    /// Return the accumulated UID table.
    pub fn uids(&self) -> &[u32] {
        &self.uid_table
    }

    /// Return the accumulated GID table.
    pub fn gids(&self) -> &[u32] {
        &self.gid_table
    }

    /// Return the accumulated device table.
    pub fn devs(&self) -> &[u64] {
        &self.dev_table
    }
}

/// Create a new [`Walk`] generator for `root` producing batches of at most
/// `batch_size` entries.
pub fn walk(root: impl AsRef<Path>, batch_size: usize) -> Walk {
    Walk::new(root.as_ref().to_path_buf(), batch_size)
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

                    #[cfg(unix)]
                    let meta = match std::fs::symlink_metadata(entry.path()) {
                        Ok(m) => m,
                        Err(e) => return Some(Err(e)),
                    };
                    #[cfg(unix)]
                    let (uid, gid, dev) = (meta.uid(), meta.gid(), meta.dev());
                    #[cfg(not(unix))]
                    let (uid, gid, dev) = (0u32, 0u32, 0u64);

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

                    batch.push(Entry {
                        prefix_len: prefix,
                        suffix,
                        file_type: entry.file_type(),
                        uid: uid_idx,
                        gid: gid_idx,
                        dev: dev_idx,
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
