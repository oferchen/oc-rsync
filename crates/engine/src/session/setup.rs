// crates/engine/src/session/setup.rs

use std::fs;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use compress::Codec;
use filters::Matcher;
use walk::walk;

use crate::Result;

use super::SyncOptions;

pub(crate) fn is_remote_spec(path: &OsStr) -> bool {
    if let Some(s) = path.to_str() {
        if s.starts_with("rsync://") || s.starts_with("rsync:/") {
            return true;
        }
        if s.starts_with('[') && s.contains("]:") {
            return true;
        }
        if s.contains("::") {
            return true;
        }
        if let Some(idx) = s.find(':') {
            if idx == 1 {
                let bytes = s.as_bytes();
                if bytes[0].is_ascii_alphabetic()
                    && bytes
                        .get(2)
                        .map(|c| *c == b'/' || *c == b'\\')
                        .unwrap_or(false)
                {
                    return false;
                }
            }
            return true;
        }
    }
    false
}

pub(crate) fn count_entries(
    src_root: &Path,
    matcher: &Matcher,
    opts: &SyncOptions,
) -> Result<(usize, usize, u64)> {
    let mut walker = walk(
        src_root,
        1024,
        None,
        opts.walk_links(),
        opts.one_file_system,
        &[],
    )?;
    let mut state = String::new();
    let mut files = 0usize;
    let mut dirs = 0usize;
    let mut size = 0u64;
    while let Some(batch) = walker.next() {
        let Ok(batch) = batch else { continue };
        let mut skip_dirs: Vec<PathBuf> = Vec::new();
        for entry in batch {
            let path = entry.apply(&mut state);
            if skip_dirs.iter().any(|d| path.starts_with(d)) {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(src_root) {
                let res = matcher.is_included_with_dir(rel)?;
                if !res.include {
                    if !res.descend && entry.file_type.is_dir() {
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                    }
                    continue;
                }
                if entry.file_type.is_dir() {
                    dirs += 1;
                    if !res.descend && !rel.as_os_str().is_empty() {
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                    }
                } else if entry.file_type.is_file() {
                    if opts.dirs_only {
                        continue;
                    }
                    files += 1;
                    if let Ok(meta) = fs::metadata(&path) {
                        size += meta.len();
                    }
                }
            }
        }
    }
    Ok((files, dirs, size))
}

pub fn select_codec(remote: &[Codec], opts: &SyncOptions) -> Option<Codec> {
    if !opts.compress || opts.compress_level == Some(0) {
        return None;
    }
    let choices: Vec<Codec> = opts.compress_choice.clone().unwrap_or_else(|| {
        let mut v = vec![Codec::Zstd];
        v.push(Codec::ZlibX);
        v.push(Codec::Zlib);
        v
    });
    choices.into_iter().find(|c| remote.contains(c))
}
