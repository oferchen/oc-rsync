// crates/walk/src/lib.rs
//! Filesystem traversal utilities for building file lists.
#![deny(unsafe_op_in_unsafe_fn, rust_2018_idioms)]
#![deny(warnings)]
#![warn(missing_docs)]
#![allow(clippy::collapsible_if)]

use std::collections::HashMap;
use std::fs::FileType;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[cfg(windows)]
const VERBATIM_PREFIX: &str = r"\\?\\";
#[cfg(windows)]
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let s = path.as_ref().as_os_str().to_string_lossy();
    if s.starts_with(VERBATIM_PREFIX) {
        PathBuf::from(s.into_owned())
    } else {
        PathBuf::from(format!("{}{}", VERBATIM_PREFIX, s))
    }
}

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
    include_links: bool,
    one_file_system: bool,
    root_dev: u64,
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
    fn new(
        root: PathBuf,
        batch_size: usize,
        max_file_size: Option<u64>,
        include_links: bool,
        one_file_system: bool,
    ) -> std::io::Result<Self> {
        #[cfg(windows)]
        let walk_root = normalize_path(&root);
        #[cfg(windows)]
        let iter = WalkDir::new(walk_root)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter();
        #[cfg(windows)]
        let root_dev = 0;

        #[cfg(unix)]
        let root_dev = if one_file_system {
            std::fs::symlink_metadata(&root)?.dev()
        } else {
            0
        };
        #[cfg(unix)]
        let iter = WalkDir::new(root)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter();

        #[cfg(not(any(unix, windows)))]
        let root_dev = 0;
        #[cfg(not(any(unix, windows)))]
        let iter = WalkDir::new(root)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter();

        Ok(Walk {
            iter,
            prev_path: String::new(),
            batch_size,
            max_file_size,
            include_links,
            one_file_system,
            root_dev,
            uid_map: HashMap::new(),
            uid_table: Vec::new(),
            gid_map: HashMap::new(),
            gid_table: Vec::new(),
            dev_map: HashMap::new(),
            dev_table: Vec::new(),
            inode_map: HashMap::new(),
            inode_table: Vec::new(),
        })
    }

    pub fn skip_current_dir(&mut self) {
        self.iter.skip_current_dir();
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

pub fn walk(
    root: impl AsRef<Path>,
    batch_size: usize,
    include_links: bool,
    one_file_system: bool,
) -> std::io::Result<Walk> {
    Walk::new(
        root.as_ref().to_path_buf(),
        batch_size,
        None,
        include_links,
        one_file_system,
    )
}

pub fn walk_with_max_size(
    root: impl AsRef<Path>,
    batch_size: usize,
    max_file_size: u64,
    include_links: bool,
    one_file_system: bool,
) -> std::io::Result<Walk> {
    Walk::new(
        root.as_ref().to_path_buf(),
        batch_size,
        Some(max_file_size),
        include_links,
        one_file_system,
    )
}

impl Iterator for Walk {
    type Item = std::io::Result<Vec<Entry>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut batch = Vec::new();
        while batch.len() < self.batch_size {
            match self.iter.next() {
                Some(Ok(entry)) => {
                    let file_type = entry.file_type();
                    if file_type.is_symlink() && !self.include_links {
                        continue;
                    }
                    let path = {
                        #[cfg(windows)]
                        let mut p = entry.path().to_string_lossy().into_owned();
                        #[cfg(not(windows))]
                        let p = entry.path().to_string_lossy().into_owned();
                        #[cfg(windows)]
                        {
                            if let Some(stripped) = p.strip_prefix(VERBATIM_PREFIX) {
                                p = stripped.to_string();
                            }
                        }
                        p
                    };
                    let prefix = common_prefix_len(&self.prev_path, &path);
                    let suffix = path[prefix..].to_string();

                    let meta = match std::fs::symlink_metadata(entry.path()) {
                        Ok(m) => m,
                        Err(e) => return Some(Err(e)),
                    };
                    if let Some(max) = self.max_file_size {
                        if meta.is_file() && meta.len() > max {
                            continue;
                        }
                    }

                    #[cfg(unix)]
                    let (uid, gid, dev, ino) = (meta.uid(), meta.gid(), meta.dev(), meta.ino());
                    #[cfg(not(unix))]
                    let (uid, gid, dev, ino) = (0u32, 0u32, 0u64, 0u64);

                    if self.one_file_system && dev != self.root_dev {
                        if file_type.is_dir() {
                            self.iter.skip_current_dir();
                        }
                        continue;
                    }

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
                        file_type,
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
                        None => std::io::Error::other(msg),
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
