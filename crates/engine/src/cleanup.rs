// crates/engine/src/cleanup.rs

use rand::{Rng, distributions::Alphanumeric};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use tempfile::Builder;

use crate::{Result, SyncOptions, io_context};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub(crate) fn atomic_rename(src: &Path, dst: &Path) -> Result<()> {
    let cross_device = {
        #[cfg(unix)]
        {
            let src_dev = fs::metadata(src).ok().map(|m| m.dev());
            let dst_parent = dst.parent().unwrap_or_else(|| Path::new("."));
            let dst_dev = fs::metadata(dst_parent).ok().map(|m| m.dev());
            src_dev.is_some() && dst_dev.is_some() && src_dev != dst_dev
        }
        #[cfg(windows)]
        {
            false
        }
        #[cfg(not(any(unix, windows)))]
        {
            false
        }
    };
    if cross_device {
        let parent = dst.parent().unwrap_or_else(|| Path::new("."));
        let base = dst.file_name().unwrap_or_default().to_string_lossy();
        let tmp = Builder::new()
            .prefix(&format!(".{}.", base))
            .rand_bytes(6)
            .tempfile_in(parent)
            .map_err(|e| io_context(parent, e))?;
        fs::copy(src, tmp.path()).map_err(|e| io_context(src, e))?;
        tmp.persist(dst).map_err(|e| io_context(dst, e.error))?;
        fs::remove_file(src).map_err(|e| io_context(src, e))?;
        Ok(())
    } else {
        match fs::rename(src, dst) {
            Ok(_) => Ok(()),
            Err(e) => {
                let cross_device_err = {
                    #[cfg(unix)]
                    {
                        e.raw_os_error() == Some(nix::errno::Errno::EXDEV as i32)
                    }
                    #[cfg(windows)]
                    {
                        matches!(e.raw_os_error(), Some(17))
                    }
                    #[cfg(not(any(unix, windows)))]
                    {
                        false
                    }
                };
                if cross_device_err {
                    let parent = dst.parent().unwrap_or_else(|| Path::new("."));
                    let base = dst.file_name().unwrap_or_default().to_string_lossy();
                    let tmp = Builder::new()
                        .prefix(&format!(".{}.", base))
                        .rand_bytes(6)
                        .tempfile_in(parent)
                        .map_err(|e| io_context(parent, e))?;
                    fs::copy(src, tmp.path()).map_err(|e| io_context(src, e))?;
                    tmp.persist(dst).map_err(|e| io_context(dst, e.error))?;
                    fs::remove_file(src).map_err(|e| io_context(src, e))?;
                    Ok(())
                } else {
                    Err(io_context(src, e))
                }
            }
        }
    }
}

pub(crate) fn partial_paths(dest: &Path, partial_dir: Option<&Path>) -> (PathBuf, Option<PathBuf>) {
    if let Some(dir) = partial_dir {
        let file = dest.file_name().unwrap_or_default();
        if let Some(parent) = dest.parent() {
            (parent.join(dir).join(file), None)
        } else {
            (dir.join(file), None)
        }
    } else {
        let mut name = dest.file_name().unwrap_or_default().to_os_string();
        name.push(".partial");
        let partial = dest.with_file_name(&name);
        let alt = dest.file_stem().map(|stem| {
            let mut n = stem.to_os_string();
            n.push(".partial");
            dest.with_file_name(n)
        });
        (partial, alt)
    }
}

pub(crate) fn remove_basename_partial(dest: &Path) {
    if let Some(stem) = dest.file_stem() {
        let mut name = stem.to_os_string();
        name.push(".partial");
        let path = dest.with_file_name(name);
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
}

pub(crate) struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn disarm(&mut self) {
        self.path.clear();
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.path.as_os_str().is_empty() {
            return;
        }
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn remove_file_opts(path: &Path, opts: &SyncOptions) -> Result<()> {
    if opts.dry_run || opts.only_write_batch {
        return Ok(());
    }
    match fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(e) => {
            let e = io_context(path, e);
            if opts.ignore_errors { Ok(()) } else { Err(e) }
        }
    }
}

pub(crate) fn tmp_file_path(dir: &Path, dest: &Path) -> PathBuf {
    let name = dest.file_name().unwrap_or_else(|| OsStr::new("tmp"));
    let rand: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();
    let mut file = OsString::from(".");
    file.push(name);
    file.push(".");
    file.push(&rand);
    dir.join(file)
}

pub(crate) fn remove_dir_opts(path: &Path, opts: &SyncOptions) -> Result<()> {
    if opts.dry_run || opts.only_write_batch {
        return Ok(());
    }
    let res = if opts.force {
        fs::remove_dir_all(path)
    } else {
        fs::remove_dir(path)
    };
    match res {
        Ok(_) => Ok(()),
        Err(e) => {
            let e = io_context(path, e);
            if opts.ignore_errors { Ok(()) } else { Err(e) }
        }
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut curr = vec![i + 1];
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { prev[j] } else { prev[j] + 1 };
            let ins = curr[j] + 1;
            let del = prev[j + 1] + 1;
            curr.push(cost.min(ins).min(del));
        }
        prev = curr;
    }
    prev[b.len()]
}

#[doc(hidden)]
pub(crate) fn fuzzy_match(dest: &Path) -> Option<PathBuf> {
    let parent = dest.parent()?;
    let stem = dest.file_stem()?.to_string_lossy().to_string();
    let mut best: Option<(usize, PathBuf)> = None;
    for entry in fs::read_dir(parent).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path == dest {
            continue;
        }
        let candidate_stem = path.file_stem()?.to_string_lossy().to_string();
        let dist = levenshtein(&stem, &candidate_stem);
        match &mut best {
            Some((d, _)) if dist < *d => best = Some((dist, path)),
            None => best = Some((dist, path)),
            _ => {}
        }
    }
    best.map(|(_, p)| p)
}

pub(crate) fn open_for_read(path: &Path, _opts: &SyncOptions) -> std::io::Result<File> {
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        if _opts.open_noatime {
            let mut o = OpenOptions::new();
            o.read(true).custom_flags(libc::O_NOATIME);
            if let Ok(f) = o.open(path) {
                return Ok(f);
            }
        }
    }
    File::open(path)
}

pub(crate) fn files_identical(a: &Path, b: &Path, opts: &SyncOptions) -> bool {
    if let (Ok(ma), Ok(mb)) = (fs::metadata(a), fs::metadata(b)) {
        if ma.len() != mb.len() {
            return false;
        }
        let mut fa = match open_for_read(a, opts) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut fb = match open_for_read(b, opts) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut ba = [0u8; 8192];
        let mut bb = [0u8; 8192];
        loop {
            match (fa.read(&mut ba), fb.read(&mut bb)) {
                (Ok(ra), Ok(rb)) => {
                    if ra != rb {
                        return false;
                    }
                    if ra == 0 {
                        break;
                    }
                    if ba[..ra] != bb[..rb] {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    } else {
        false
    }
}
