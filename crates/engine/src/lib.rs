// crates/engine/src/lib.rs
#![allow(clippy::collapsible_if)]
#[cfg(unix)]
use nix::unistd::{Gid, Uid, chown};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, Write};
#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "illumos",
    target_os = "solaris",
))]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime};
use transport::{Transport, pipe};

use checksums::ChecksumConfig;
pub use checksums::StrongHash;
use compress::Codec;
use filters::Matcher;
use logging::{InfoFlag, escape_path};
use protocol::ExitCode;
use thiserror::Error;
mod cleanup;
mod delta;
mod receiver;
mod sender;

pub mod flist;

pub use delta::{DeltaIter, Op, compute_delta};
pub use meta::MetaOpts;
pub use receiver::{Receiver, ReceiverState};
pub use sender::{Sender, SenderState};
pub const META_OPTS: MetaOpts = meta::META_OPTS;

use cleanup::{
    atomic_rename, files_identical, open_for_read, partial_paths, remove_dir_opts, remove_file_opts,
};
use delta::{FILE_COUNTER, PROGRESS_HEADER, TOTAL_FILES};

const RSYNC_BLOCK_SIZE: usize = 700;
const RSYNC_MAX_BLOCK_SIZE: usize = 1 << 17;
const MUNGE_PREFIX: &str = "/rsyncd-munged";

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

fn is_device(file_type: &std::fs::FileType) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        file_type.is_block_device() || file_type.is_char_device()
    }
    #[cfg(not(unix))]
    {
        false
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

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("max-alloc limit exceeded")]
    MaxAlloc,
    #[error("{1}")]
    Exit(ExitCode, String),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, EngineError>;
use walk::walk;

#[derive(Clone)]
pub struct IdMapper(pub Arc<dyn Fn(u32) -> u32 + Send + Sync>);

impl std::fmt::Debug for IdMapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("IdMapper")
    }
}

pub(crate) trait ReadSeek: Read + Seek {}
impl<T: Read + Seek> ReadSeek for T {}

fn io_context(path: &Path, err: std::io::Error) -> EngineError {
    EngineError::Io(std::io::Error::new(
        err.kind(),
        format!("{}: {}", path.display(), err),
    ))
}

fn ensure_max_alloc(len: u64, opts: &SyncOptions) -> Result<()> {
    if opts.max_alloc != 0 && len > opts.max_alloc as u64 {
        Err(EngineError::MaxAlloc)
    } else {
        Ok(())
    }
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

#[cfg(unix)]
pub fn preallocate(file: &File, len: u64) -> std::io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        use nix::fcntl::{FallocateFlags, fallocate};
        fallocate(file, FallocateFlags::empty(), 0, len as i64).map_err(std::io::Error::from)
    }

    #[cfg(target_os = "macos")]
    unsafe {
        let fd = file.as_raw_fd();
        let mut fstore = libc::fstore_t {
            fst_flags: libc::F_ALLOCATECONTIG,
            fst_posmode: libc::F_PEOFPOSMODE,
            fst_offset: 0,
            fst_length: len as libc::off_t,
            fst_bytesalloc: 0,
        };
        let ret = libc::fcntl(fd, libc::F_PREALLOCATE, &fstore);
        if ret == -1 {
            fstore.fst_flags = libc::F_ALLOCATEALL;
            if libc::fcntl(fd, libc::F_PREALLOCATE, &fstore) == -1 {
                if libc::ftruncate(fd, len as libc::off_t) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                return Ok(());
            }
        }
        if libc::ftruncate(fd, len as libc::off_t) == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    #[cfg(any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris"
    ))]
    unsafe {
        let ret = libc::posix_fallocate(file.as_raw_fd(), 0, len as libc::off_t);
        if ret == 0 {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(ret))
        }
    }

    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "netbsd",
        target_os = "openbsd",
        target_os = "illumos",
        target_os = "solaris"
    )))]
    {
        file.set_len(len)
    }
}

#[cfg(not(unix))]
pub fn preallocate(_file: &File, _len: u64) -> std::io::Result<()> {
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

fn log_name(rel: &Path, link: Option<&Path>, opts: &SyncOptions, default: String) {
    if opts.quiet {
        return;
    }
    let itemized = default.split_once(' ').map(|(i, _)| i.to_string());
    if let Some(fmt) = &opts.out_format {
        let msg =
            logging::render_out_format(fmt, rel, link, itemized.as_deref(), opts.eight_bit_output);
        tracing::info!(target: InfoFlag::Name.target(), "{}", msg);
    } else if opts.itemize_changes {
        println!("{}", default);
    }
}

fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut normalized = PathBuf::new();
    for comp in path.as_ref().components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(c) => normalized.push(c),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::Prefix(p) => normalized.push(p.as_os_str()),
        }
    }
    normalized
}

fn last_good_block(
    cfg: &ChecksumConfig,
    src: &Path,
    dst: &Path,
    block_size: usize,
    opts: &SyncOptions,
) -> Result<u64> {
    let block_size = block_size.max(1);
    ensure_max_alloc(block_size as u64, opts)?;
    let mut src = match open_for_read(src, opts) {
        Ok(f) => f,
        Err(_) => return Ok(0),
    };
    let mut dst = match open_for_read(dst, opts) {
        Ok(f) => f,
        Err(_) => return Ok(0),
    };
    let mut offset = 0u64;
    let mut src_buf = vec![0u8; block_size];
    let mut dst_buf = vec![0u8; block_size];
    while let Ok(rs) = src.read(&mut src_buf) {
        let rd = match dst.read(&mut dst_buf) {
            Ok(n) => n,
            Err(_) => break,
        };
        let n = rs.min(rd);
        if n == 0 {
            break;
        }
        let src_sum = cfg.checksum(&src_buf[..n]).strong;
        let dst_sum = cfg.checksum(&dst_buf[..n]).strong;
        if src_sum != dst_sum {
            break;
        }
        offset += n as u64;
        if n < block_size {
            break;
        }
    }
    Ok(offset - (offset % block_size as u64))
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
    pub dirs: bool,
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
    pub skip_compress: Vec<String>,
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
            max_alloc: 1 << 30,
            max_size: None,
            min_size: None,
            preallocate: false,
            checksum: false,
            compress: false,
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
            no_implied_dirs: false,
            dry_run: false,
            #[cfg(feature = "xattr")]
            xattrs: false,
            #[cfg(feature = "acl")]
            acls: false,
            sparse: false,
            dirs: false,
            list_only: false,
            update: false,
            existing: false,
            ignore_existing: false,
            one_file_system: false,
            size_only: false,
            ignore_times: false,
            strong: StrongHash::Md4,
            checksum_seed: 0,
            compress_level: None,
            compress_choice: None,
            whole_file: false,
            skip_compress: Vec::new(),
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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
}

fn count_entries(src_root: &Path, matcher: &Matcher, opts: &SyncOptions) -> (usize, usize, u64) {
    let mut walker = walk(src_root, 1024, opts.walk_links(), opts.one_file_system);
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
                let (included, dir_only) =
                    matcher.is_included_with_dir(rel).unwrap_or((true, false));
                if !included {
                    if dir_only || entry.file_type.is_dir() {
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                    }
                    continue;
                }
                if entry.file_type.is_file() {
                    files += 1;
                    if let Ok(meta) = fs::metadata(&path) {
                        size += meta.len();
                    }
                } else if entry.file_type.is_dir() {
                    dirs += 1;
                }
            }
        }
    }
    (files, dirs, size)
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

fn unescape_rsync(path: &str) -> String {
    let mut bytes = Vec::with_capacity(path.len());
    let mut iter = path.bytes();
    while let Some(b) = iter.next() {
        if b == b'\\' {
            let oct: Vec<u8> = iter.clone().take(3).collect();
            if oct.len() == 3 && oct.iter().all(|c| c.is_ascii_digit()) {
                let val = (oct[0] - b'0') * 64 + (oct[1] - b'0') * 8 + (oct[2] - b'0');
                bytes.push(val);
                iter.nth(2);
                continue;
            }
        }
        bytes.push(b);
    }
    String::from_utf8(bytes).unwrap_or_else(|_| path.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub flist: Vec<Vec<u8>>,
    pub checksums: Vec<Vec<u8>>,
    pub data: Vec<Vec<u8>>,
}

fn encode_section(out: &mut Vec<u8>, parts: &[Vec<u8>]) {
    out.extend((parts.len() as u32).to_le_bytes());
    for part in parts {
        out.extend((part.len() as u32).to_le_bytes());
        out.extend(part);
    }
}

pub fn encode_batch(batch: &Batch) -> Vec<u8> {
    let mut out = Vec::new();
    encode_section(&mut out, &batch.flist);
    encode_section(&mut out, &batch.checksums);
    encode_section(&mut out, &batch.data);
    out
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32> {
    if *pos + 4 > bytes.len() {
        return Err(EngineError::Other("truncated batch".into()));
    }
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[*pos..*pos + 4]);
    *pos += 4;
    Ok(u32::from_le_bytes(arr))
}

fn decode_section(bytes: &[u8], pos: &mut usize) -> Result<Vec<Vec<u8>>> {
    let count = read_u32(bytes, pos)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_u32(bytes, pos)? as usize;
        if *pos + len > bytes.len() {
            return Err(EngineError::Other("truncated batch".into()));
        }
        out.push(bytes[*pos..*pos + len].to_vec());
        *pos += len;
    }
    Ok(out)
}

pub fn decode_batch(bytes: &[u8]) -> Result<Batch> {
    let mut pos = 0;
    let flist = decode_section(bytes, &mut pos)?;
    let checksums = decode_section(bytes, &mut pos)?;
    let data = decode_section(bytes, &mut pos)?;
    Ok(Batch {
        flist,
        checksums,
        data,
    })
}

fn parse_batch_file(batch_path: &Path) -> Result<Vec<PathBuf>> {
    let content = fs::read_to_string(batch_path).map_err(|e| EngineError::Other(e.to_string()))?;
    let mut paths = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.contains('=') {
            continue;
        }
        paths.push(PathBuf::from(unescape_rsync(trimmed)));
    }
    Ok(paths)
}

fn delete_extraneous(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    opts: &SyncOptions,
    stats: &mut Stats,
    start: Instant,
) -> Result<()> {
    let mut walker = walk(dst, 1, opts.walk_links(), opts.one_file_system);
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
                let (included, dir_only) = matcher
                    .is_included_for_delete_with_dir(rel)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                let src_exists = src.join(rel).exists();
                if file_type.is_dir() {
                    if (included && !src_exists) || (!included && opts.delete_excluded) {
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
                    } else if !included {
                        walker.skip_current_dir();
                        if dir_only || file_type.is_dir() {
                            skip_dirs.push(path.clone());
                        }
                    }
                } else if (included && !src_exists) || (!included && opts.delete_excluded) {
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

pub fn sync(
    src: &Path,
    dst: &Path,
    matcher: &Matcher,
    remote: &[Codec],
    opts: &SyncOptions,
) -> Result<Stats> {
    let mut batch_file = opts
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
        let matcher = matcher.clone().with_root(src_root.clone());
        let mut walker = walk(&src_root, 1024, opts.walk_links(), opts.one_file_system);
        let mut state = String::new();
        while let Some(batch) = walker.next() {
            let batch = batch.map_err(|e| EngineError::Other(e.to_string()))?;
            let mut skip_dirs: Vec<PathBuf> = Vec::new();
            for entry in batch {
                let path = entry.apply(&mut state);
                if skip_dirs.iter().any(|d| path.starts_with(d)) {
                    continue;
                }
                if let Ok(rel) = path.strip_prefix(&src_root) {
                    let (included, dir_only) = matcher
                        .is_included_with_dir(rel)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                    if !included {
                        if dir_only || entry.file_type.is_dir() {
                            walker.skip_current_dir();
                            skip_dirs.push(path.clone());
                        }
                        continue;
                    }
                    if entry.file_type.is_dir() {
                        matcher
                            .preload_dir(&path)
                            .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                    }
                    if opts.dirs && !entry.file_type.is_dir() {
                        continue;
                    }
                    if entry.file_type.is_file() {
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
    let (file_cnt, dir_cnt, total_size) = count_entries(&src_root, &matcher, opts);
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

    let mut sender = Sender::new(opts.block_size, matcher.clone(), codec, opts.clone());
    let mut receiver = Receiver::new(codec, opts.clone());
    receiver.matcher = matcher.clone();
    let mut dir_meta: Vec<(PathBuf, PathBuf)> = Vec::new();

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
    let mut walker = walk(&src_root, 1024, opts.walk_links(), opts.one_file_system);
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
            if let Ok(rel) = path.strip_prefix(&src_root) {
                let (included, dir_only) = matcher
                    .is_included_with_dir(rel)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                if !included {
                    if dir_only || file_type.is_dir() {
                        walker.skip_current_dir();
                        skip_dirs.push(path.clone());
                    }
                    continue;
                }
                let mut dest_path = dst.join(rel);
                if file_type.is_file() && dest_path.is_dir() {
                    if let Some(name) = path.file_name() {
                        dest_path.push(name);
                    }
                }
                if opts.dirs && !file_type.is_dir() {
                    continue;
                }
                if file_type.is_file() || (opts.copy_devices && is_device(&file_type)) {
                    let src_meta = fs::metadata(&path).map_err(|e| io_context(&path, e))?;
                    if outside_size_bounds(src_meta.len(), opts) {
                        continue;
                    }
                    if opts.ignore_existing && dest_path.exists() {
                        continue;
                    }
                    if opts.update && dest_path.exists() {
                        if let Ok(dst_meta) = fs::metadata(&dest_path) {
                            if let (Ok(src_mtime), Ok(dst_mtime)) =
                                (src_meta.modified(), dst_meta.modified())
                            {
                                if dst_mtime > src_mtime
                                    && dst_mtime
                                        .duration_since(src_mtime)
                                        .unwrap_or(Duration::ZERO)
                                        > opts.modify_window
                                {
                                    continue;
                                }
                            }
                        }
                    }
                    let partial_exists = if opts.partial || opts.append || opts.append_verify {
                        let (partial_path, basename_partial) =
                            partial_paths(&dest_path, opts.partial_dir.as_deref());
                        partial_path.exists()
                            || basename_partial.map(|p| p.exists()).unwrap_or(false)
                    } else {
                        false
                    };
                    if opts.existing && !dest_path.exists() && !partial_exists {
                        continue;
                    }
                    if opts.update && !dest_path.exists() && !partial_exists {
                        continue;
                    }
                    #[cfg(unix)]
                    if opts.hard_links && src_meta.nlink() > 1 {
                        let dev = walker.devs()[entry.dev];
                        let ino = walker.inodes()[entry.inode];
                        let group = meta::hard_link_id(dev, ino);
                        if !receiver.register_hard_link(group, &dest_path) {
                            if let Some(parent) = dest_path.parent() {
                                fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
                            }
                            continue;
                        }
                    }
                    if !dest_path.exists() && !partial_exists {
                        if let Some(ref link_dir) = opts.link_dest {
                            let link_path = link_dir.join(rel);
                            if files_identical(&path, &link_path, opts) {
                                if let Some(parent) = dest_path.parent() {
                                    fs::create_dir_all(parent)
                                        .map_err(|e| io_context(parent, e))?;
                                }
                                fs::hard_link(&link_path, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                                receiver.copy_metadata(&path, &dest_path)?;
                                if opts.remove_source_files {
                                    remove_file_opts(&path, opts)?;
                                }
                                continue;
                            }
                        }
                        if let Some(ref copy_dir) = opts.copy_dest {
                            let copy_path = copy_dir.join(rel);
                            if files_identical(&path, &copy_path, opts) {
                                if let Some(parent) = dest_path.parent() {
                                    fs::create_dir_all(parent)
                                        .map_err(|e| io_context(parent, e))?;
                                }
                                fs::copy(&copy_path, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                                receiver.copy_metadata(&path, &dest_path)?;
                                if opts.remove_source_files {
                                    remove_file_opts(&path, opts)?;
                                }
                                continue;
                            }
                        }
                        if let Some(ref compare_dir) = opts.compare_dest {
                            let comp_path = compare_dir.join(rel);
                            if files_identical(&path, &comp_path, opts) {
                                if opts.remove_source_files {
                                    remove_file_opts(&path, opts)?;
                                }
                                continue;
                            }
                        }
                    }
                    let dest_missing = !dest_path.exists() && !partial_exists;
                    if sender.process_file(&path, &dest_path, rel, &mut receiver, &mut stats)? {
                        stats.files_transferred += 1;
                        stats.bytes_transferred +=
                            fs::metadata(&path).map_err(|e| io_context(&path, e))?.len();
                        if let Some(f) = batch_file.as_mut() {
                            let _ = writeln!(f, "{}", rel.display());
                        }
                        if (opts.out_format.is_some() || opts.itemize_changes) && !opts.quiet {
                            let name = escape_path(rel, opts.eight_bit_output);
                            log_name(rel, None, opts, format!(">f+++++++++ {}", name));
                        }
                    }
                    if dest_missing {
                        stats.files_created += 1;
                    }
                    if opts.remove_source_files {
                        remove_file_opts(&path, opts)?;
                    }
                } else if file_type.is_dir() {
                    matcher
                        .preload_dir(&path)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
                    let dest_meta = fs::symlink_metadata(&dest_path).ok();
                    let dest_is_symlink = dest_meta
                        .as_ref()
                        .map(|m| m.file_type().is_symlink())
                        .unwrap_or(false);
                    if opts.no_implied_dirs {
                        if dest_meta.is_none() {
                            fs::create_dir_all(&dest_path)
                                .map_err(|e| io_context(&dest_path, e))?;
                        }
                        continue;
                    }
                    let mut created = dest_meta.is_none();
                    if dest_is_symlink {
                        if opts.keep_dirlinks {
                            let link_target =
                                fs::read_link(&dest_path).map_err(|e| io_context(&dest_path, e))?;
                            let target_path = if link_target.is_absolute() {
                                normalize_path(&link_target)
                            } else if let Some(parent) = dest_path.parent() {
                                normalize_path(parent.join(&link_target))
                            } else {
                                normalize_path(&link_target)
                            };
                            if !target_path.exists() {
                                created = true;
                            }
                            fs::create_dir_all(&target_path)
                                .map_err(|e| io_context(&target_path, e))?;
                        } else {
                            remove_file_opts(&dest_path, opts)?;
                            fs::create_dir_all(&dest_path)
                                .map_err(|e| io_context(&dest_path, e))?;
                            created = true;
                        }
                    } else {
                        if !dest_path.exists() {
                            created = true;
                        }
                        fs::create_dir_all(&dest_path).map_err(|e| io_context(&dest_path, e))?;
                    }
                    if created {
                        stats.files_created += 1;
                        stats.dirs_created += 1;
                        #[cfg(unix)]
                        if let Some((uid, gid)) = opts.copy_as {
                            let gid = gid.map(Gid::from_raw);
                            chown(&dest_path, Some(Uid::from_raw(uid)), gid)
                                .map_err(|e| io_context(&dest_path, std::io::Error::from(e)))?;
                        }
                        if (opts.out_format.is_some() || opts.itemize_changes) && !opts.quiet {
                            let name = escape_path(rel, opts.eight_bit_output);
                            log_name(rel, None, opts, format!("cd+++++++++ {}/", name));
                        }
                    }
                    dir_meta.push((path.clone(), dest_path.clone()));
                } else if file_type.is_symlink() {
                    if !(opts.links
                        || opts.copy_links
                        || opts.copy_dirlinks
                        || opts.copy_unsafe_links)
                    {
                        continue;
                    }
                    let mut target = fs::read_link(&path).map_err(|e| io_context(&path, e))?;
                    if opts.munge_links {
                        if let Ok(stripped) = target.strip_prefix(MUNGE_PREFIX) {
                            target = stripped.to_path_buf();
                        }
                    }
                    let target_path = if target.is_absolute() {
                        normalize_path(&target)
                    } else if let Some(parent) = path.parent() {
                        normalize_path(parent.join(&target))
                    } else {
                        normalize_path(src_root.join(&target))
                    };
                    let is_unsafe = target.is_absolute() || !target_path.starts_with(&src_root);
                    if opts.safe_links && is_unsafe {
                        continue;
                    }
                    if opts.ignore_existing && dest_path.exists() {
                        continue;
                    }
                    let meta = fs::metadata(&target_path).ok();
                    if (opts.copy_dirlinks && meta.as_ref().map(|m| m.is_dir()).unwrap_or(false))
                        || opts.copy_links
                        || (opts.copy_unsafe_links && is_unsafe)
                    {
                        if let Some(meta) = meta {
                            if meta.is_dir() {
                                if let Some(parent) = dest_path.parent() {
                                    fs::create_dir_all(parent)
                                        .map_err(|e| io_context(parent, e))?;
                                }
                                let sub = sync(&target_path, &dest_path, &matcher, remote, opts)?;
                                stats.files_total += sub.files_total;
                                stats.dirs_total += sub.dirs_total;
                                stats.files_transferred += sub.files_transferred;
                                stats.files_deleted += sub.files_deleted;
                                stats.total_file_size += sub.total_file_size;
                                stats.bytes_transferred += sub.bytes_transferred;
                                stats.literal_data += sub.literal_data;
                                stats.matched_data += sub.matched_data;
                                stats.bytes_sent += sub.bytes_sent;
                                stats.bytes_received += sub.bytes_received;
                            } else if meta.is_file() {
                                let dest_missing = !dest_path.exists();
                                if sender.process_file(
                                    &target_path,
                                    &dest_path,
                                    rel,
                                    &mut receiver,
                                    &mut stats,
                                )? {
                                    stats.files_transferred += 1;
                                    stats.bytes_transferred += meta.len();
                                    if let Some(f) = batch_file.as_mut() {
                                        let _ = writeln!(f, "{}", rel.display());
                                    }
                                }
                                if dest_missing {
                                    stats.files_created += 1;
                                }
                            }
                            if opts.remove_source_files {
                                remove_file_opts(&path, opts)?;
                            }
                        } else {
                            return Err(EngineError::Other(format!(
                                "symlink has no referent: {}",
                                path.display()
                            )));
                        }
                    } else if opts.links {
                        if opts.munge_links {
                            if let Ok(stripped) = target.strip_prefix("/") {
                                target = PathBuf::from(MUNGE_PREFIX).join(stripped);
                            } else {
                                target = PathBuf::from(MUNGE_PREFIX).join(&target);
                            }
                        }
                        let created = fs::symlink_metadata(&dest_path).is_err();
                        if let Some(parent) = dest_path.parent() {
                            fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
                        }
                        if let Ok(meta_dest) = fs::symlink_metadata(&dest_path) {
                            if meta_dest.is_dir() {
                                remove_dir_opts(&dest_path, opts)?;
                            } else {
                                remove_file_opts(&dest_path, opts)?;
                            }
                        }
                        #[cfg(unix)]
                        std::os::unix::fs::symlink(&target, &dest_path)
                            .map_err(|e| io_context(&dest_path, e))?;
                        #[cfg(windows)]
                        {
                            if meta.as_ref().map_or(false, |m| m.is_dir()) {
                                std::os::windows::fs::symlink_dir(&target, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                            } else {
                                std::os::windows::fs::symlink_file(&target, &dest_path)
                                    .map_err(|e| io_context(&dest_path, e))?;
                            }
                        }
                        receiver.copy_metadata(&path, &dest_path)?;
                        if created {
                            stats.files_created += 1;
                            stats.files_transferred += 1;
                            if (opts.out_format.is_some() || opts.itemize_changes) && !opts.quiet {
                                let name = escape_path(rel, opts.eight_bit_output);
                                let target_name = escape_path(&target, opts.eight_bit_output);
                                log_name(
                                    rel,
                                    Some(&target),
                                    opts,
                                    format!("cL+++++++++ {} -> {}", name, target_name),
                                );
                            }
                        }
                        if opts.remove_source_files {
                            remove_file_opts(&path, opts)?;
                        }
                    }
                } else {
                    #[cfg(unix)]
                    {
                        if (file_type.is_char_device() || file_type.is_block_device())
                            && opts.devices
                            && !opts.copy_devices
                        {
                            use nix::sys::stat::{Mode, SFlag};
                            let created = fs::symlink_metadata(&dest_path).is_err();
                            if !created {
                                remove_file_opts(&dest_path, opts)?;
                            }
                            if let Some(parent) = dest_path.parent() {
                                fs::create_dir_all(parent).map_err(|e| io_context(parent, e))?;
                            }
                            let meta =
                                fs::symlink_metadata(&path).map_err(|e| io_context(&path, e))?;
                            let kind = if file_type.is_char_device() {
                                SFlag::S_IFCHR
                            } else {
                                SFlag::S_IFBLK
                            };
                            let perm =
                                Mode::from_bits_truncate((meta.mode() & 0o777) as libc::mode_t);
                            meta::mknod(&dest_path, kind, perm, meta.rdev())
                                .map_err(|e| io_context(&dest_path, e))?;
                            receiver.copy_metadata_now(&path, &dest_path)?;
                            if created {
                                stats.files_created += 1;
                                stats.files_transferred += 1;
                                if (opts.out_format.is_some() || opts.itemize_changes)
                                    && !opts.quiet
                                {
                                    let name = escape_path(rel, opts.eight_bit_output);
                                    log_name(rel, None, opts, format!("cD+++++++++ {}", name));
                                }
                            }
                        } else if file_type.is_fifo() && opts.specials {
                            use nix::sys::stat::Mode;
                            meta::mkfifo(&dest_path, Mode::from_bits_truncate(0o644))
                                .map_err(|e| io_context(&dest_path, e))?;
                            receiver.copy_metadata(&path, &dest_path)?;
                        }
                    }
                    if opts.remove_source_files {
                        remove_file_opts(&path, opts)?;
                    }
                }
            } else {
                continue;
            }
        }
    }
    sender.finish();
    receiver.finalize()?;
    if matches!(
        opts.delete,
        Some(DeleteMode::After) | Some(DeleteMode::During)
    ) {
        delete_extraneous(&src_root, dst, &matcher, opts, &mut stats, start)?;
    }
    for (src_dir, dest_dir) in dir_meta.into_iter().rev() {
        receiver.copy_metadata(&src_dir, &dest_dir)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delta::{LIT_CAP, apply_delta};
    use checksums::{ChecksumConfigBuilder, rolling_checksum};
    use compress::available_codecs;
    use filters::Matcher;
    use std::fs;
    use std::io::Write;
    use tempfile::{NamedTempFile, tempdir};

    #[test]
    fn delta_roundtrip() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello world".to_vec());
        let mut target = Cursor::new(b"hello brave new world".to_vec());
        let delta = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap();
        let mut basis = Cursor::new(b"hello world".to_vec());
        let mut out = Cursor::new(Vec::new());
        let mut progress = None;
        apply_delta(
            &mut basis,
            delta,
            &mut out,
            &SyncOptions::default(),
            0,
            &mut progress,
        )
        .unwrap();
        assert_eq!(out.into_inner(), b"hello brave new world");
    }

    #[test]
    fn weak_checksum_collision() {
        let cfg = ChecksumConfigBuilder::new().build();
        let block1 = b"\x01\x00\x01";
        let block2 = b"\x00\x02\x00";
        assert_eq!(rolling_checksum(block1), rolling_checksum(block2));
        let basis: Vec<u8> = [block1.as_ref(), block2.as_ref()].concat();
        let mut basis_reader = Cursor::new(basis.clone());
        let mut target_reader = Cursor::new(basis.clone());
        let delta: Vec<Op> = compute_delta(
            &cfg,
            &mut basis_reader,
            &mut target_reader,
            3,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();
        assert_eq!(
            delta,
            vec![
                Op::Copy { offset: 0, len: 3 },
                Op::Copy { offset: 3, len: 3 },
            ]
        );
        let mut basis_reader = Cursor::new(basis.clone());
        let mut out = Cursor::new(Vec::new());
        let mut progress = None;
        apply_delta(
            &mut basis_reader,
            delta.into_iter().map(Ok),
            &mut out,
            &SyncOptions::default(),
            0,
            &mut progress,
        )
        .unwrap();
        assert_eq!(out.into_inner(), basis);
    }

    #[test]
    fn emits_literal_for_new_data() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(Vec::new());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta: Vec<Op> = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();
        assert_eq!(delta, vec![Op::Data(b"abc".to_vec())]);
    }

    #[test]
    fn empty_target_yields_no_ops() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(Vec::new());
        let delta: Vec<Op> = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();
        assert!(delta.is_empty());
    }

    #[test]
    fn small_basis_matches() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"abc".to_vec());
        let mut target = Cursor::new(b"abc".to_vec());
        let delta: Vec<Op> = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();
        assert_eq!(delta, vec![Op::Copy { offset: 0, len: 3 }]);
    }

    #[test]
    fn matches_partial_blocks() {
        let cfg = ChecksumConfigBuilder::new().build();
        let mut basis = Cursor::new(b"hello".to_vec());
        let mut target = Cursor::new(b"hello".to_vec());
        let delta: Vec<Op> = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();
        assert_eq!(
            delta,
            vec![
                Op::Copy { offset: 0, len: 4 },
                Op::Copy { offset: 4, len: 1 },
            ]
        );
    }

    #[test]
    fn last_good_block_detects_prefix() {
        let cfg = ChecksumConfigBuilder::new().build();
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src.bin");
        let dst = tmp.path().join("dst.bin");
        fs::write(&src, b"abcd1234").unwrap();
        fs::write(&dst, b"abcdxxxx").unwrap();
        assert_eq!(
            last_good_block(&cfg, &src, &dst, 4, &SyncOptions::default()).unwrap(),
            4
        );
    }

    #[test]
    fn sync_dir() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("a")).unwrap();

        fs::write(src.join("a/file1.txt"), vec![b'h'; 2048]).unwrap();
        fs::write(src.join("file2.txt"), vec![b'w'; 2048]).unwrap();

        sync(
            &src,
            &dst,
            &Matcher::default(),
            &available_codecs(),
            &SyncOptions::default(),
        )
        .unwrap();
        assert_eq!(fs::read(dst.join("a/file1.txt")).unwrap(), vec![b'h'; 2048]);
        assert_eq!(fs::read(dst.join("file2.txt")).unwrap(), vec![b'w'; 2048]);
    }

    #[test]
    fn sync_skips_outside_paths() {
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("inside.txt"), b"inside").unwrap();

        let outside = tmp.path().join("outside.txt");
        fs::write(&outside, b"outside").unwrap();

        let mut sender = Sender::new(
            1024,
            Matcher::default(),
            Some(Codec::Zlib),
            SyncOptions::default(),
        );
        let mut receiver = Receiver::new(Some(Codec::Zlib), SyncOptions::default());
        let mut stats = Stats::default();
        sender.start();
        for path in [src.join("inside.txt"), outside.clone()] {
            if let Ok(rel) = path.strip_prefix(&src) {
                let dest_path = dst.join(rel);
                sender
                    .process_file(&path, &dest_path, rel, &mut receiver, &mut stats)
                    .unwrap();
            }
        }
        sender.finish();

        assert_eq!(fs::read(dst.join("inside.txt")).unwrap(), b"inside");
        assert!(!dst.join("outside.txt").exists());
    }

    fn mem_usage_kb() -> u64 {
        use std::fs;
        let status = fs::read_to_string("/proc/self/status").unwrap();
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                return parts[1].parse().unwrap();
            }
        }
        0
    }

    #[test]
    fn large_file_windowed_delta_memory() {
        let cfg = ChecksumConfigBuilder::new().build();
        let block_size = 1024;
        let window = 64;
        let data = vec![42u8; block_size * 1024];
        let mut basis = Cursor::new(data.clone());
        let mut target = Cursor::new(data.clone());

        let before = mem_usage_kb();
        let delta = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            block_size,
            window,
            &SyncOptions::default(),
        )
        .unwrap();
        let after = mem_usage_kb();
        let mut basis = Cursor::new(data.clone());
        let mut out = Cursor::new(Vec::new());
        let mut progress = None;
        apply_delta(
            &mut basis,
            delta,
            &mut out,
            &SyncOptions::default(),
            0,
            &mut progress,
        )
        .unwrap();
        assert_eq!(out.into_inner(), data);
        let growth = after.saturating_sub(before);
        assert!(growth < 10 * 1024, "memory grew too much: {}KB", growth);
    }

    #[test]
    fn literal_chunks_respect_cap() {
        let cfg = ChecksumConfigBuilder::new().build();
        let data = vec![42u8; LIT_CAP * 3 + 123];
        let mut basis = Cursor::new(Vec::new());
        let mut target = Cursor::new(data.clone());
        let ops: Vec<Op> = compute_delta(
            &cfg,
            &mut basis,
            &mut target,
            4,
            usize::MAX,
            &SyncOptions::default(),
        )
        .unwrap()
        .collect::<Result<_>>()
        .unwrap();
        assert!(ops.iter().all(|op| match op {
            Op::Data(d) => d.len() <= LIT_CAP,
            _ => false,
        }));
    }

    #[cfg(all(unix, any(target_os = "linux", target_os = "android")))]
    #[test]
    fn preallocate_failure_surfaces_error() {
        let tmp = tempdir().unwrap();
        let path = tmp.path().join("file");
        File::create(&path).unwrap();
        let file = std::fs::OpenOptions::new().read(true).open(&path).unwrap();
        let err = preallocate(&file, 1).unwrap_err();
        assert_eq!(err.raw_os_error(), Some(libc::EBADF));
    }

    #[test]
    fn large_file_strong_checksum_matches() {
        let mut tmp = NamedTempFile::new().unwrap();
        let chunk = [0u8; 1024];
        for _ in 0..(11 * 1024) {
            tmp.write_all(&chunk).unwrap();
        }
        let path = tmp.path().to_path_buf();
        let sender = Sender::new(
            RSYNC_BLOCK_SIZE,
            Matcher::default(),
            None,
            SyncOptions::default(),
        );
        let new_sum = sender.strong_file_checksum(&path).unwrap();
        let data = fs::read(&path).unwrap();
        let old_sum = sender.cfg.checksum(&data).strong;
        assert_eq!(new_sum, old_sum);
    }
}
