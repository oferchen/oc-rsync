// crates/engine/src/session/mod.rs

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use compress::Codec;

use crate::StrongHash;

mod run;
mod setup;

pub use run::{pipe_sessions, sync};
pub use setup::select_codec;

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
