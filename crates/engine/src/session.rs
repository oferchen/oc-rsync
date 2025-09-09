// crates/engine/src/session.rs

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime};

#[cfg(unix)]
use nix::unistd::{Gid, Uid, chown};

use compress::Codec;
use filters::Matcher;
use logging::{InfoFlag, escape_path};
use protocol::ExitCode;
use transport::{Transport, pipe};
use walk::walk;

use crate::batch::parse_batch_file;
use crate::cleanup::{atomic_rename, remove_dir_opts, remove_file_opts};
use crate::delta::{FILE_COUNTER, PROGRESS_HEADER, TOTAL_FILES};
use crate::io::io_context;
use crate::{EngineError, Receiver, Result, Sender, StrongHash};

#[derive(Clone)]
pub struct IdMapper(pub Arc<dyn Fn(u32) -> u32 + Send + Sync>);

impl std::fmt::Debug for IdMapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IdMapper")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteMode {
    Before,
    During,
    After,
}

#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub delete: Option<DeleteMode>,
    pub delete_excluded: bool,
    pub ignore_missing_args: bool,
    pub delete_missing_args: bool,
    pub remove_source_files: bool,
    pub ignore_errors: bool,
    pub force: bool,
    pub max_delete: Option<usize>,
    pub max_alloc: usize,
    pub max_size: Option<u64>,
    pub min_size: Option<u64>,
    pub preallocate: bool,
    pub checksum: bool,
    pub compress: bool,
    pub dirs_only: bool,
    pub no_implied_dirs: bool,
    pub dry_run: bool,
    pub list_only: bool,
    pub update: bool,
    pub existing: bool,
    pub ignore_existing: bool,
    pub one_file_system: bool,
    pub size_only: bool,
    pub ignore_times: bool,
    pub perms: bool,
    pub executability: bool,
    pub times: bool,
    pub atimes: bool,
    pub crtimes: bool,
    pub omit_dir_times: bool,
    pub omit_link_times: bool,
    pub owner: bool,
    pub group: bool,
    pub links: bool,
    pub copy_links: bool,
    pub copy_dirlinks: bool,
    pub keep_dirlinks: bool,
    pub copy_unsafe_links: bool,
    pub safe_links: bool,
    pub munge_links: bool,
    pub hard_links: bool,
    pub devices: bool,
    pub specials: bool,
    pub fsync: bool,
    pub fuzzy: bool,
    pub super_user: bool,
    pub fake_super: bool,
    #[cfg(feature = "xattr")]
    pub xattrs: bool,
    #[cfg(feature = "acl")]
    pub acls: bool,
    pub sparse: bool,
    pub strong: StrongHash,
    pub checksum_seed: u32,
    pub compress_level: Option<i32>,
    pub compress_choice: Option<Vec<Codec>>,
    pub whole_file: bool,
    pub skip_compress: HashSet<String>,
    pub partial: bool,
    pub progress: bool,
    pub human_readable: bool,
    pub itemize_changes: bool,
    pub out_format: Option<String>,
    pub partial_dir: Option<PathBuf>,
    pub temp_dir: Option<PathBuf>,
    pub append: bool,
    pub append_verify: bool,
    pub numeric_ids: bool,
    pub inplace: bool,
    pub delay_updates: bool,
    pub modify_window: Duration,
    pub bwlimit: Option<u64>,
    pub stop_after: Option<Duration>,
    pub stop_at: Option<SystemTime>,
    pub block_size: usize,
    pub link_dest: Option<PathBuf>,
    pub copy_dest: Option<PathBuf>,
    pub compare_dest: Option<PathBuf>,
    pub backup: bool,
    pub backup_dir: Option<PathBuf>,
    pub backup_suffix: String,
    pub chmod: Option<Vec<meta::Chmod>>,
    pub chown: Option<(Option<u32>, Option<u32>)>,
    pub copy_as: Option<(u32, Option<u32>)>,
    pub eight_bit_output: bool,
    pub blocking_io: bool,
    pub open_noatime: bool,
    pub early_input: Option<PathBuf>,
    pub secluded_args: bool,
    pub sockopts: Vec<String>,
    pub remote_options: Vec<String>,
    pub write_batch: Option<PathBuf>,
    pub only_write_batch: bool,
    pub read_batch: Option<PathBuf>,
    pub copy_devices: bool,
    pub write_devices: bool,
    pub quiet: bool,
    pub uid_map: Option<IdMapper>,
    pub gid_map: Option<IdMapper>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            delete: None,
            delete_excluded: false,
            ignore_missing_args: false,
            delete_missing_args: false,
            remove_source_files: false,
            ignore_errors: false,
            force: false,
            max_delete: None,
            max_alloc: 0,
            max_size: None,
            min_size: None,
            preallocate: false,
            checksum: false,
            compress: false,
            dirs_only: false,
            no_implied_dirs: false,
            dry_run: false,
            list_only: false,
            update: false,
            existing: false,
            ignore_existing: false,
            one_file_system: false,
            size_only: false,
            ignore_times: false,
            perms: false,
            executability: false,
            times: false,
            atimes: false,
            crtimes: false,
            omit_dir_times: false,
            omit_link_times: false,
            owner: false,
            group: false,
            links: false,
            copy_links: false,
            copy_dirlinks: false,
            keep_dirlinks: false,
            copy_unsafe_links: false,
            safe_links: false,
            munge_links: false,
            hard_links: false,
            devices: false,
            specials: false,
            fsync: false,
            fuzzy: false,
            super_user: false,
            fake_super: false,
            #[cfg(feature = "xattr")]
            xattrs: false,
            #[cfg(feature = "acl")]
            acls: false,
            sparse: false,
            strong: StrongHash::Md4,
            checksum_seed: 0,
            compress_level: None,
            compress_choice: None,
            whole_file: false,
            skip_compress: HashSet::new(),
            partial: false,
            progress: false,
            human_readable: false,
            itemize_changes: false,
            out_format: None,
            partial_dir: None,
            temp_dir: None,
            append: false,
            append_verify: false,
            numeric_ids: false,
            inplace: false,
            delay_updates: false,
            modify_window: Duration::ZERO,
            bwlimit: None,
            stop_after: None,
            stop_at: None,
            block_size: 0,
            link_dest: None,
            copy_dest: None,
            compare_dest: None,
            backup: false,
            backup_dir: None,
            backup_suffix: "~".into(),
            chmod: None,
            chown: None,
            copy_as: None,
            eight_bit_output: false,
            blocking_io: false,
            open_noatime: false,
            early_input: None,
            secluded_args: false,
            sockopts: Vec::new(),
            remote_options: Vec::new(),
            write_batch: None,
            only_write_batch: false,
            read_batch: None,
            copy_devices: false,
            write_devices: false,
            quiet: false,
            uid_map: None,
            gid_map: None,
        }
    }
}

impl SyncOptions {
    pub fn prepare_remote(&mut self) {
        if self.dry_run {
            self.remote_options.push("--dry-run".into());
        }
        if self.partial {
            self.remote_options.push("--partial".into());
        }
        if self.append {
            self.remote_options.push("--append".into());
        }
        if self.append_verify {
            self.remote_options.push("--append-verify".into());
        }
        if self.inplace {
            self.remote_options.push("--inplace".into());
        }
        if self.hard_links {
            self.remote_options.push("--hard-links".into());
        }
        if let Some(dir) = &self.partial_dir {
            self.remote_options
                .push(format!("--partial-dir={}", dir.display()));
        }
        if self.one_file_system {
            self.remote_options.push("--one-file-system".into());
        }
        if self.block_size > 0 {
            self.remote_options
                .push(format!("--block-size={}", self.block_size));
        }
    }

    fn walk_links(&self) -> bool {
        self.links
            || self.copy_links
            || self.copy_dirlinks
            || self.copy_unsafe_links
            || self.safe_links
            || self.munge_links
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub files_total: usize,
    pub dirs_total: usize,
    pub files_transferred: usize,
    pub files_deleted: usize,
    pub files_created: usize,
    pub dirs_created: usize,
    pub total_file_size: u64,
    pub bytes_transferred: u64,
    pub literal_data: u64,
    pub matched_data: u64,
    pub file_list_size: u64,
    pub file_list_gen_time: Duration,
    pub file_list_transfer_time: Duration,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub start_time: Instant,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            files_total: 0,
            dirs_total: 0,
            files_transferred: 0,
            files_deleted: 0,
            files_created: 0,
            dirs_created: 0,
            total_file_size: 0,
            bytes_transferred: 0,
            literal_data: 0,
            matched_data: 0,
            file_list_size: 0,
            file_list_gen_time: Duration::default(),
            file_list_transfer_time: Duration::default(),
            bytes_sent: 0,
            bytes_received: 0,
            start_time: Instant::now(),
        }
    }
}

impl Stats {
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

fn is_remote_spec(path: &Path) -> bool {
    if let Some(s) = path.to_str() {
        if s.starts_with("rsync://") {
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

fn check_time_limit(start: Instant, opts: &SyncOptions) -> Result<()> {
    if let Some(limit) = opts.stop_after {
        if start.elapsed() >= limit {
            return Err(EngineError::Exit(
                ExitCode::Timeout,
                "operation timed out".into(),
            ));
        }
    }
    if let Some(limit) = opts.stop_at {
        if SystemTime::now() >= limit {
            return Err(EngineError::Exit(
                ExitCode::Timeout,
                "operation timed out".into(),
            ));
        }
    }
    Ok(())
}

fn outside_size_bounds(len: u64, opts: &SyncOptions) -> bool {
    if let Some(min) = opts.min_size {
        if len < min {
            return true;
        }
    }
    if let Some(max) = opts.max_size {
        if len > max {
            return true;
        }
    }
    false
}

fn delete_extraneous(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    opts: &SyncOptions,
    stats: &mut Stats,
    start: Instant,
) -> Result<()> {
    let mut walker = walk(dst, 1, None, opts.walk_links(), opts.one_file_system, &[])?;
    let mut state = String::new();
    let mut first_err: Option<EngineError> = None;
    while let Some(batch) = walker.next() {
        check_time_limit(start, opts)?;
        let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
        let mut skip_dirs: Vec<PathBuf> = Vec::new();
        for entry in batch {
            check_time_limit(start, opts)?;
            let path = entry.apply(&mut state);
            if skip_dirs.iter().any(|d| path.starts_with(d)) {
                continue;
            }
            let file_type = entry.file_type;
            if let Ok(rel) = path.strip_prefix(dst) {
                let res = matcher
                    .is_included_for_delete_with_dir(rel)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                let src_exists = src.join(rel).exists();
                if file_type.is_dir() {
                    if (res.include && !src_exists) || (!res.include && opts.delete_excluded) {
                        if let Some(max) = opts.max_delete {
                            if stats.files_deleted >= max {
                                return Err(EngineError::Other("max-delete limit exceeded".into()));
                            }
                        }
                        if !opts.quiet {
                            tracing::info!(
                                target: InfoFlag::Del.target(),
                                "deleting {}",
                                escape_path(rel, opts.eight_bit_output)
                            );
                        }
                        let res = if opts.dry_run || opts.only_write_batch {
                            None
                        } else if opts.backup {
                            let backup_path = if let Some(ref dir) = opts.backup_dir {
                                let mut p = dir.join(rel);
                                if !opts.backup_suffix.is_empty() {
                                    if let Some(name) = p.file_name() {
                                        p = p.with_file_name(format!(
                                            "{}{}",
                                            name.to_string_lossy(),
                                            &opts.backup_suffix
                                        ));
                                    } else {
                                        p.push(&opts.backup_suffix);
                                    }
                                }
                                p
                            } else {
                                let name = path
                                    .file_name()
                                    .map(|n| {
                                        format!("{}{}", n.to_string_lossy(), &opts.backup_suffix)
                                    })
                                    .unwrap_or_else(|| opts.backup_suffix.clone());
                                path.with_file_name(name)
                            };
                            let dir_res = if let Some(parent) = backup_path.parent() {
                                fs::create_dir_all(parent).map_err(|e| io_context(parent, e))
                            } else {
                                Ok(())
                            };
                            dir_res
                                .and_then(|_| atomic_rename(&path, &backup_path))
                                .err()
                        } else {
                            remove_dir_opts(&path, opts).err()
                        };
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                        match res {
                            None => {
                                stats.files_deleted += 1;
                            }
                            Some(e) => {
                                if first_err.is_none() {
                                    first_err = Some(e);
                                }
                            }
                        }
                    } else if !res.include {
                        walker.skip_current_dir();
                        if !res.descend || file_type.is_dir() {
                            skip_dirs.push(path.clone());
                        }
                    }
                } else if (res.include && !src_exists) || (!res.include && opts.delete_excluded) {
                    if let Some(max) = opts.max_delete {
                        if stats.files_deleted >= max {
                            return Err(EngineError::Other("max-delete limit exceeded".into()));
                        }
                    }
                    if !opts.quiet {
                        tracing::info!(
                            target: InfoFlag::Del.target(),
                            "deleting {}",
                            escape_path(rel, opts.eight_bit_output)
                        );
                    }
                    let res = if opts.dry_run || opts.only_write_batch {
                        None
                    } else if opts.backup {
                        let backup_path = if let Some(ref dir) = opts.backup_dir {
                            let mut p = dir.join(rel);
                            if !opts.backup_suffix.is_empty() {
                                if let Some(name) = p.file_name() {
                                    p = p.with_file_name(format!(
                                        "{}{}",
                                        name.to_string_lossy(),
                                        &opts.backup_suffix
                                    ));
                                } else {
                                    p.push(&opts.backup_suffix);
                                }
                            }
                            p
                        } else {
                            let name = path
                                .file_name()
                                .map(|n| format!("{}{}", n.to_string_lossy(), &opts.backup_suffix))
                                .unwrap_or_else(|| opts.backup_suffix.clone());
                            path.with_file_name(name)
                        };
                        let dir_res = if let Some(parent) = backup_path.parent() {
                            fs::create_dir_all(parent).map_err(|e| io_context(parent, e))
                        } else {
                            Ok(())
                        };
                        dir_res
                            .and_then(|_| atomic_rename(&path, &backup_path))
                            .err()
                    } else {
                        remove_file_opts(&path, opts).err()
                    };
                    match res {
                        None => {
                            stats.files_deleted += 1;
                        }
                        Some(e) => {
                            if first_err.is_none() {
                                first_err = Some(e);
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(e) = first_err {
        if opts.ignore_errors { Ok(()) } else { Err(e) }
    } else {
        Ok(())
    }
}

fn count_entries(
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
                    if opts.dirs_only && !rel.as_os_str().is_empty() {
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                        continue;
                    }
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

pub fn pipe_sessions<S, D>(src: &mut S, dst: &mut D) -> Result<Stats>
where
    S: Transport,
    D: Transport,
{
    let bytes = pipe(src, dst).map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(Stats {
        files_transferred: (bytes > 0) as usize,
        bytes_transferred: bytes,
        ..Stats::default()
    })
}

pub fn sync(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    remote: &[Codec],
    opts: &SyncOptions,
) -> Result<Stats> {
    let batch_file = opts
        .write_batch
        .as_ref()
        .and_then(|p| OpenOptions::new().create(true).append(true).open(p).ok());
    let src_is_remote = is_remote_spec(src);
    let src_root = if src_is_remote {
        src.to_path_buf()
    } else {
        fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf())
    };
    let mut stats = Stats::default();
    let start = Instant::now();
    if !src_is_remote && !src_root.exists() {
        if opts.delete_missing_args {
            if dst.exists() {
                if let Some(max) = opts.max_delete {
                    if stats.files_deleted >= max {
                        return Err(EngineError::Other("max-delete limit exceeded".into()));
                    }
                }
                let meta = fs::symlink_metadata(dst).map_err(|e| io_context(dst, e))?;
                let res = if opts.backup {
                    let backup_path = if let Some(ref dir) = opts.backup_dir {
                        let mut p = if let Some(name) = dst.file_name() {
                            dir.join(name)
                        } else {
                            dir.join(dst)
                        };
                        if !opts.backup_suffix.is_empty() {
                            if let Some(name) = p.file_name() {
                                p = p.with_file_name(format!(
                                    "{}{}",
                                    name.to_string_lossy(),
                                    &opts.backup_suffix
                                ));
                            } else {
                                p.push(&opts.backup_suffix);
                            }
                        }
                        p
                    } else {
                        let name = dst
                            .file_name()
                            .map(|n| format!("{}{}", n.to_string_lossy(), &opts.backup_suffix))
                            .unwrap_or_else(|| opts.backup_suffix.clone());
                        dst.with_file_name(name)
                    };
                    if let Some(parent) = backup_path.parent() {
                        fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
                    }
                    atomic_rename(dst, &backup_path).err()
                } else if meta.file_type().is_dir() {
                    remove_dir_opts(dst, opts).err()
                } else {
                    remove_file_opts(dst, opts).err()
                };
                match res {
                    None => stats.files_deleted += 1,
                    Some(e) => {
                        if !opts.ignore_errors {
                            return Err(e);
                        }
                    }
                }
            }
            return Ok(stats);
        } else if opts.ignore_missing_args {
            return Ok(stats);
        } else {
            let abs = if src.is_absolute() {
                src.to_path_buf()
            } else {
                std::env::current_dir()
                    .map_err(|e| EngineError::Other(e.to_string()))?
                    .join(src)
            };
            return Err(EngineError::Exit(
                ExitCode::Partial,
                format!(
                    "rsync: [sender] link_stat \"{}\" failed: No such file or directory (2)\nrsync error: some files/attrs were not transferred (see previous errors) (code 23)",
                    abs.display(),
                ),
            ));
        }
    }

    if opts.list_only {
        let mut walker = walk(
            &src_root,
            1024,
            None,
            opts.walk_links(),
            opts.one_file_system,
            &[],
        )?;
        let mut state = String::new();
        while let Some(batch) = walker.next() {
            check_time_limit(start, opts)?;
            let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
            let mut skip_dirs: Vec<PathBuf> = Vec::new();
            for entry in batch {
                let path = entry.apply(&mut state);
                if skip_dirs.iter().any(|d| path.starts_with(d)) {
                    continue;
                }
                if let Ok(rel) = path.strip_prefix(&src_root) {
                    let res = matcher
                        .is_included_with_dir(rel)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                    if !res.include {
                        if !res.descend && entry.file_type.is_dir() {
                            walker.skip_current_dir();
                            skip_dirs.push(path.clone());
                        }
                        continue;
                    }
                    if entry.file_type.is_dir() {
                        if opts.dirs_only && !rel.as_os_str().is_empty() {
                            walker.skip_current_dir();
                            skip_dirs.push(path.clone());
                        } else if !res.descend && !rel.as_os_str().is_empty() {
                            walker.skip_current_dir();
                            skip_dirs.push(path.clone());
                        }
                    } else if entry.file_type.is_file() {
                        if opts.dirs_only {
                            continue;
                        }
                        let len = fs::metadata(&path).map_err(|e| io_context(&path, e))?.len();
                        if outside_size_bounds(len, opts) {
                            continue;
                        }
                    }
                    if !opts.quiet {
                        if rel.as_os_str().is_empty() {
                            println!(".");
                        } else if entry.file_type.is_dir() {
                            println!("{}/", escape_path(rel, opts.eight_bit_output));
                        } else {
                            println!("{}", escape_path(rel, opts.eight_bit_output));
                        }
                    }
                }
            }
        }
        return Ok(stats);
    }

    let codec = select_codec(remote, opts);
    let matcher = matcher.clone().with_root(src_root.clone());
    let list_start = Instant::now();
    let (file_cnt, dir_cnt, total_size) = count_entries(&src_root, &matcher, opts)?;
    stats.file_list_gen_time = list_start.elapsed();
    stats.files_total = file_cnt;
    stats.dirs_total = dir_cnt;
    stats.total_file_size = total_size;
    if opts.progress {
        FILE_COUNTER.store(0, Ordering::SeqCst);
        PROGRESS_HEADER.store(false, Ordering::SeqCst);
        TOTAL_FILES.store(file_cnt, Ordering::SeqCst);
    }
    if opts.dry_run {
        if opts.delete.is_some() {
            delete_extraneous(&src_root, dst, &matcher, opts, &mut stats, start)?;
        }
        return Ok(stats);
    }
    if !opts.only_write_batch {
        let dir = if src_root.is_file() {
            dst.parent()
        } else if !dst.exists() {
            Some(dst)
        } else {
            None
        };

        if let Some(dir) = dir {
            if !dir.exists() {
                fs::create_dir_all(dir).map_err(|e| {
                    std::io::Error::new(
                        e.kind(),
                        format!(
                            "failed to create destination directory {}: {e}",
                            dir.display()
                        ),
                    )
                })?;
                #[cfg(unix)]
                if let Some((uid, gid)) = opts.copy_as {
                    let gid = gid.map(Gid::from_raw);
                    chown(dir, Some(Uid::from_raw(uid)), gid)
                        .map_err(|e| io_context(dir, std::io::Error::from(e)))?;
                }
                stats.files_created += 1;
                stats.dirs_created += 1;
            }
        }
    }

    let mut sender = Sender::new(matcher.clone(), codec, opts.clone());
    let mut receiver = Receiver::new(codec, opts.clone());
    receiver.matcher = matcher.clone();

    if let Some(batch_path) = &opts.read_batch {
        sender.start();
        for rel in parse_batch_file(batch_path)? {
            let path = src_root.join(&rel);
            if !path.exists() {
                continue;
            }
            let dest_path = dst.join(&rel);
            if sender.process_file(&path, &dest_path, &rel, &mut receiver, &mut stats)? {
                stats.files_transferred += 1;
                stats.bytes_transferred +=
                    fs::metadata(&path).map_err(|e| io_context(&path, e))?.len();
            }
        }
        sender.finish();
        receiver.finalize()?;
        if let Some(mut f) = batch_file {
            let _ = writeln!(
                f,
                "files_transferred={} bytes_transferred={}",
                stats.files_transferred, stats.bytes_transferred
            );
        }
        return Ok(stats);
    }
    if matches!(opts.delete, Some(DeleteMode::Before)) {
        delete_extraneous(&src_root, dst, &matcher, opts, &mut stats, start)?;
    }
    let flist_xfer_start = Instant::now();
    sender.start();
    stats.file_list_transfer_time = flist_xfer_start.elapsed();
    let mut state = String::new();
    let mut walker = walk(
        &src_root,
        1024,
        None,
        opts.walk_links(),
        opts.one_file_system,
        &[],
    )?;
    while let Some(batch) = walker.next() {
        check_time_limit(start, opts)?;
        let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
        let mut skip_dirs: Vec<PathBuf> = Vec::new();
        for entry in batch {
            check_time_limit(start, opts)?;
            let path = entry.apply(&mut state);
            if skip_dirs.iter().any(|d| path.starts_with(d)) {
                continue;
            }
            if let Ok(rel) = path.strip_prefix(&src_root) {
                let res = matcher.is_included_with_dir(rel)?;
                if !res.include {
                    if !res.descend && entry.file_type.is_dir() {
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                    }
                    continue;
                }
                if entry.file_type.is_dir() {
                    if opts.dirs_only {
                        let dest_path = dst.join(rel);
                        fs::create_dir_all(&dest_path).map_err(|e| io_context(&dest_path, e))?;
                        receiver.copy_metadata_now(&path, &dest_path)?;
                        stats.files_created += 1;
                        stats.dirs_created += 1;
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                        continue;
                    }
                    if !res.descend && !rel.as_os_str().is_empty() {
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                    }
                } else if entry.file_type.is_file() {
                    if opts.dirs_only {
                        continue;
                    }
                    let len = fs::metadata(&path).map_err(|e| io_context(&path, e))?.len();
                    if outside_size_bounds(len, opts) {
                        continue;
                    }
                    if sender.process_file(&path, &dst.join(rel), rel, &mut receiver, &mut stats)? {
                        stats.files_transferred += 1;
                        stats.bytes_transferred += len;
                    }
                }
            }
        }
    }
    sender.finish();
    receiver.finalize()?;
    if matches!(opts.delete, Some(DeleteMode::During)) {
        delete_extraneous(&src_root, dst, &matcher, opts, &mut stats, start)?;
    }
    if matches!(opts.delete, Some(DeleteMode::After)) {
        delete_extraneous(&src_root, dst, &matcher, opts, &mut stats, start)?;
    }
    if let Some(mut f) = batch_file {
        let _ = writeln!(
            f,
            "files_transferred={} bytes_transferred={}",
            stats.files_transferred, stats.bytes_transferred
        );
    }
    Ok(stats)
}
