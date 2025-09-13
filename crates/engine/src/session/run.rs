// crates/engine/src/session/run.rs

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Instant, SystemTime};

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
use crate::{EngineError, Receiver, Result, Sender};

use super::select_codec;
use super::setup::{count_entries, is_remote_spec};
use super::{DeleteMode, Stats, SyncOptions};

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
    let dst_is_remote = is_remote_spec(dst);
    let src_root = if src_is_remote {
        src.to_path_buf()
    } else {
        fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf())
    };
    let mut stats = Stats::default();
    let start = Instant::now();
    if !src_is_remote && !src_root.exists() {
        if opts.delete_missing_args {
            if !dst_is_remote && dst.exists() {
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
                        if !rel.as_os_str().is_empty() && !res.descend {
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
        if !dst_is_remote && opts.delete.is_some() {
            delete_extraneous(&src_root, dst, &matcher, opts, &mut stats, start)?;
        }
        return Ok(stats);
    }
    if !opts.only_write_batch && !dst_is_remote {
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
    if !dst_is_remote && matches!(opts.delete, Some(DeleteMode::Before)) {
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
                    let dest_path = dst.join(rel);
                    if rel.as_os_str().is_empty() {
                        #[cfg(feature = "acl")]
                        if opts.acls && !dst_is_remote {
                            receiver.copy_metadata_now(&path, &dest_path, None)?;
                        }
                        continue;
                    }
                    if opts.dirs_only {
                        if !dst_is_remote {
                            fs::create_dir_all(&dest_path)
                                .map_err(|e| io_context(&dest_path, e))?;
                            receiver.copy_metadata_now(&path, &dest_path, None)?;
                            stats.files_created += 1;
                            stats.dirs_created += 1;
                        }
                        continue;
                    }
                    if !res.descend {
                        if !dst_is_remote {
                            fs::create_dir_all(&dest_path)
                                .map_err(|e| io_context(&dest_path, e))?;
                            receiver.copy_metadata_now(&path, &dest_path, None)?;
                            stats.files_created += 1;
                            stats.dirs_created += 1;
                        }
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                        continue;
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
    if !dst_is_remote && matches!(opts.delete, Some(DeleteMode::During)) {
        delete_extraneous(&src_root, dst, &matcher, opts, &mut stats, start)?;
    }
    if !dst_is_remote && matches!(opts.delete, Some(DeleteMode::After)) {
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
