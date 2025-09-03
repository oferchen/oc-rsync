// crates/cli/src/options.rs

use std::path::PathBuf;
use std::time::Duration;

pub use crate::daemon::DaemonOpts;
use crate::formatter;
use crate::utils::{parse_duration, parse_nonzero_duration, parse_size};
use clap::{ArgAction, Args, CommandFactory, Parser, ValueEnum};
use logging::{DebugFlag, InfoFlag};
use protocol::SUPPORTED_PROTOCOLS;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "UPPER")]
pub enum OutBuf {
    N,
    L,
    B,
}

#[derive(Parser, Debug)]
pub(crate) struct ClientOpts {
    #[command(flatten)]
    pub daemon: DaemonOpts,
    #[arg(short = 'a', long, help_heading = "Selection")]
    pub archive: bool,
    #[arg(short, long, help_heading = "Selection")]
    pub recursive: bool,
    #[arg(short = 'd', long, help_heading = "Selection")]
    pub dirs: bool,
    #[arg(
        long = "old-dirs",
        visible_alias = "old-d",
        help_heading = "Selection",
        help = "works like --dirs when talking to old rsync"
    )]
    pub old_dirs: bool,
    #[arg(short = 'R', long, help_heading = "Selection")]
    pub relative: bool,
    #[arg(long = "no-implied-dirs", help_heading = "Selection")]
    pub no_implied_dirs: bool,
    #[arg(short = 'n', long, help_heading = "Selection")]
    pub dry_run: bool,
    #[arg(long = "list-only", help_heading = "Output")]
    pub list_only: bool,
    #[arg(short = 'S', long, help_heading = "Selection")]
    pub sparse: bool,
    #[arg(short = 'u', long, help_heading = "Misc")]
    pub update: bool,
    #[arg(long, help_heading = "Misc")]
    pub existing: bool,
    #[arg(long, help_heading = "Misc")]
    pub ignore_existing: bool,
    #[arg(short = 'x', long = "one-file-system", help_heading = "Selection")]
    pub one_file_system: bool,
    #[arg(short = 'm', long = "prune-empty-dirs", help_heading = "Misc")]
    pub prune_empty_dirs: bool,
    #[arg(long = "size-only", help_heading = "Misc")]
    pub size_only: bool,
    #[arg(short = 'I', long = "ignore-times", help_heading = "Misc")]
    pub ignore_times: bool,
    #[arg(short, long, action = ArgAction::Count, help_heading = "Output")]
    pub verbose: u8,
    #[arg(
        long = "log-file",
        value_name = "FILE",
        help_heading = "Output",
        id = "client-log-file"
    )]
    pub log_file: Option<PathBuf>,
    #[arg(
        long = "log-file-format",
        value_name = "FMT",
        help_heading = "Output",
        id = "client-log-file-format"
    )]
    pub log_file_format: Option<String>,
    #[arg(long = "out-format", value_name = "FORMAT", help_heading = "Output")]
    pub out_format: Option<String>,
    #[arg(
        long,
        value_name = "FLAGS",
        value_delimiter = ',',
        value_enum,
        help_heading = "Output"
    )]
    pub info: Vec<InfoFlag>,
    #[arg(
        long,
        value_name = "FLAGS",
        value_delimiter = ',',
        value_enum,
        help_heading = "Output"
    )]
    pub debug: Vec<DebugFlag>,
    #[arg(long = "human-readable", help_heading = "Output")]
    pub human_readable: bool,
    #[arg(short, long, help_heading = "Output")]
    pub quiet: bool,
    #[arg(long, help_heading = "Output")]
    pub no_motd: bool,
    #[arg(short = '8', long = "8-bit-output", help_heading = "Output")]
    pub eight_bit_output: bool,
    #[arg(
        short = 'i',
        long = "itemize-changes",
        help_heading = "Output",
        help = "output a change-summary for all updates"
    )]
    pub itemize_changes: bool,
    #[arg(
        long,
        help_heading = "Delete",
        overrides_with_all = [
            "delete_before",
            "delete_during",
            "delete_after",
            "delete_delay"
        ]
    )]
    pub delete: bool,
    #[arg(
        long = "delete-before",
        help_heading = "Delete",
        overrides_with_all = ["delete", "delete_during", "delete_after", "delete_delay"]
    )]
    pub delete_before: bool,
    #[arg(
        long = "delete-during",
        help_heading = "Delete",
        visible_alias = "del",
        overrides_with_all = ["delete", "delete_before", "delete_after", "delete_delay"]
    )]
    pub delete_during: bool,
    #[arg(
        long = "delete-after",
        help_heading = "Delete",
        overrides_with_all = ["delete", "delete_before", "delete_during", "delete_delay"]
    )]
    pub delete_after: bool,
    #[arg(
        long = "delete-delay",
        help_heading = "Delete",
        overrides_with_all = ["delete", "delete_before", "delete_during", "delete_after"]
    )]
    pub delete_delay: bool,
    #[arg(long = "delete-excluded", help_heading = "Delete")]
    pub delete_excluded: bool,
    #[arg(long = "delete-missing-args", help_heading = "Delete")]
    pub delete_missing_args: bool,
    #[arg(long = "ignore-missing-args", help_heading = "Delete")]
    pub ignore_missing_args: bool,
    #[arg(
        long = "remove-source-files",
        help_heading = "Delete",
        visible_alias = "remove-sent-files"
    )]
    pub remove_source_files: bool,
    #[arg(long = "ignore-errors", help_heading = "Delete")]
    pub ignore_errors: bool,
    #[arg(
        long,
        help_heading = "Delete",
        help = "force deletion of dirs even if not empty",
        action = ArgAction::SetTrue
    )]
    pub force: bool,
    #[arg(long = "max-delete", value_name = "NUM", help_heading = "Delete")]
    pub max_delete: Option<usize>,
    #[arg(
        long = "max-alloc",
        value_name = "SIZE",
        value_parser = parse_size::<usize>,
        help_heading = "Misc"
    )]
    pub max_alloc: Option<usize>,
    #[arg(
        long = "max-size",
        value_name = "SIZE",
        value_parser = parse_size::<u64>,
        help_heading = "Misc"
    )]
    pub max_size: Option<u64>,
    #[arg(
        long = "min-size",
        value_name = "SIZE",
        value_parser = parse_size::<u64>,
        help_heading = "Misc"
    )]
    pub min_size: Option<u64>,
    #[arg(
        long,
        help_heading = "Misc",
        help = "allocate dest files before writing them"
    )]
    pub preallocate: bool,
    #[arg(short = 'b', long, help_heading = "Backup")]
    pub backup: bool,
    #[arg(long = "backup-dir", value_name = "DIR", help_heading = "Backup")]
    pub backup_dir: Option<PathBuf>,
    #[arg(
        long = "suffix",
        value_name = "SUFFIX",
        default_value = "~",
        help_heading = "Backup"
    )]
    pub suffix: String,
    #[arg(short = 'c', long, help_heading = "Attributes")]
    pub checksum: bool,
    #[arg(
        long = "checksum-choice",
        value_name = "STR",
        help_heading = "Attributes",
        visible_alias = "cc"
    )]
    pub checksum_choice: Option<String>,
    #[arg(
        long = "checksum-seed",
        value_name = "NUM",
        value_parser = clap::value_parser!(u32),
        help_heading = "Attributes",
        help = "set block/file checksum seed (advanced)"
    )]
    pub checksum_seed: Option<u32>,
    #[arg(
        short = 'p',
        long,
        help_heading = "Attributes",
        overrides_with = "no_perms"
    )]
    pub perms: bool,
    #[arg(
        long = "no-perms",
        help_heading = "Attributes",
        overrides_with = "perms"
    )]
    pub no_perms: bool,
    #[arg(short = 'E', long, help_heading = "Attributes")]
    pub executability: bool,
    #[arg(long = "chmod", value_name = "CHMOD", help_heading = "Attributes")]
    pub chmod: Vec<String>,
    #[arg(long = "chown", value_name = "USER:GROUP", help_heading = "Attributes")]
    pub chown: Option<String>,
    #[arg(
        long = "copy-as",
        value_name = "USER[:GROUP]",
        help_heading = "Attributes"
    )]
    pub copy_as: Option<String>,
    #[arg(
        long = "usermap",
        value_name = "FROM:TO",
        value_delimiter = ',',
        help_heading = "Attributes"
    )]
    pub usermap: Vec<String>,
    #[arg(
        long = "groupmap",
        value_name = "FROM:TO",
        value_delimiter = ',',
        help_heading = "Attributes"
    )]
    pub groupmap: Vec<String>,
    #[arg(
        short = 't',
        long,
        help_heading = "Attributes",
        overrides_with = "no_times"
    )]
    pub times: bool,
    #[arg(
        long = "no-times",
        help_heading = "Attributes",
        overrides_with = "times"
    )]
    pub no_times: bool,
    #[arg(short = 'U', long, help_heading = "Attributes")]
    pub atimes: bool,
    #[arg(short = 'N', long, help_heading = "Attributes")]
    pub crtimes: bool,
    #[arg(short = 'O', long, help_heading = "Attributes")]
    pub omit_dir_times: bool,
    #[arg(short = 'J', long, help_heading = "Attributes")]
    pub omit_link_times: bool,
    #[arg(
        short = 'o',
        long,
        help_heading = "Attributes",
        overrides_with = "no_owner"
    )]
    pub owner: bool,
    #[arg(
        long = "no-owner",
        alias = "no-o",
        help_heading = "Attributes",
        overrides_with = "owner",
        alias = "no-o"
    )]
    pub no_owner: bool,
    #[arg(
        short = 'g',
        long,
        help_heading = "Attributes",
        overrides_with = "no_group"
    )]
    pub group: bool,
    #[arg(
        long = "no-group",
        help_heading = "Attributes",
        overrides_with = "group",
        alias = "no-g"
    )]
    pub no_group: bool,
    #[arg(
        short = 'l',
        long,
        help_heading = "Attributes",
        overrides_with = "no_links"
    )]
    pub links: bool,
    #[arg(
        long = "no-links",
        help_heading = "Attributes",
        overrides_with = "links"
    )]
    pub no_links: bool,
    #[arg(short = 'L', long, help_heading = "Attributes")]
    pub copy_links: bool,
    #[arg(short = 'k', long, help_heading = "Attributes")]
    pub copy_dirlinks: bool,
    #[arg(short = 'K', long, help_heading = "Attributes")]
    pub keep_dirlinks: bool,
    #[arg(long, help_heading = "Attributes")]
    pub copy_unsafe_links: bool,
    #[arg(long, help_heading = "Attributes")]
    pub safe_links: bool,
    #[arg(
        long,
        help_heading = "Attributes",
        help = "munge symlinks to make them safe & unusable"
    )]
    pub munge_links: bool,
    #[arg(short = 'H', long = "hard-links", help_heading = "Attributes")]
    pub hard_links: bool,
    #[arg(long, help_heading = "Attributes", overrides_with = "no_devices")]
    pub devices: bool,
    #[arg(
        long = "no-devices",
        help_heading = "Attributes",
        overrides_with = "devices"
    )]
    pub no_devices: bool,
    #[arg(long, help_heading = "Attributes", overrides_with = "no_specials")]
    pub specials: bool,
    #[arg(
        long = "no-specials",
        help_heading = "Attributes",
        overrides_with = "specials"
    )]
    pub no_specials: bool,
    #[arg(short = 'D', help_heading = "Attributes")]
    pub devices_specials: bool,
    #[cfg(feature = "xattr")]
    #[arg(long, help_heading = "Attributes")]
    pub xattrs: bool,
    #[cfg(feature = "acl")]
    #[arg(
        short = 'A',
        long,
        help_heading = "Attributes",
        overrides_with = "no_acls"
    )]
    pub acls: bool,
    #[cfg(feature = "acl")]
    #[arg(long = "no-acls", help_heading = "Attributes", overrides_with = "acls")]
    pub no_acls: bool,
    #[arg(long = "fake-super", help_heading = "Attributes")]
    pub fake_super: bool,
    #[arg(long = "super", help_heading = "Attributes")]
    pub super_user: bool,
    #[arg(short = 'z', long, help_heading = "Compression")]
    pub compress: bool,
    #[arg(
        long = "compress-choice",
        value_name = "STR",
        help_heading = "Compression",
        visible_alias = "zc"
    )]
    pub compress_choice: Option<String>,
    #[arg(
        long = "compress-level",
        value_name = "NUM",
        help_heading = "Compression",
        visible_alias = "zl"
    )]
    pub compress_level: Option<i32>,
    #[arg(
        long = "skip-compress",
        value_name = "LIST",
        help_heading = "Compression",
        value_delimiter = ','
    )]
    pub skip_compress: Vec<String>,

    #[arg(long, help_heading = "Misc")]
    pub partial: bool,
    #[arg(long = "partial-dir", value_name = "DIR", help_heading = "Misc")]
    pub partial_dir: Option<PathBuf>,
    #[arg(
        short = 'T',
        long = "temp-dir",
        value_name = "DIR",
        help_heading = "Misc"
    )]
    pub temp_dir: Option<PathBuf>,
    #[arg(long, help_heading = "Misc")]
    pub progress: bool,
    #[arg(long, help_heading = "Misc")]
    pub blocking_io: bool,
    #[arg(
        long = "outbuf",
        value_name = "MODE",
        value_enum,
        help_heading = "Misc"
    )]
    pub outbuf: Option<OutBuf>,
    #[arg(long, help_heading = "Misc")]
    pub fsync: bool,
    #[arg(short = 'y', long = "fuzzy", help_heading = "Misc")]
    pub fuzzy: bool,
    #[arg(short = 'P', help_heading = "Misc")]
    pub partial_progress: bool,
    #[arg(long, help_heading = "Misc")]
    pub append: bool,
    #[arg(long = "append-verify", help_heading = "Misc")]
    pub append_verify: bool,
    #[arg(long, help_heading = "Misc")]
    pub inplace: bool,
    #[arg(long = "delay-updates", help_heading = "Misc")]
    pub delay_updates: bool,
    #[arg(long = "bwlimit", value_name = "RATE", help_heading = "Misc")]
    pub bwlimit: Option<u64>,
    #[arg(long = "timeout", value_name = "SECONDS", value_parser = parse_duration, help_heading = "Misc")]
    pub timeout: Option<Duration>,
    #[arg(
        long = "connect-timeout",
        alias = "contimeout",
        value_name = "SECONDS",
        value_parser = parse_nonzero_duration,
        help_heading = "Misc"
    )]
    pub connect_timeout: Option<Duration>,
    #[arg(long = "modify-window", value_name = "SECONDS", value_parser = parse_duration, help_heading = "Misc")]
    pub modify_window: Option<Duration>,
    #[arg(
        long = "protocol",
        value_name = "VER",
        value_parser = clap::value_parser!(u32),
        help_heading = "Misc",
        help = "force an older protocol version"
    )]
    pub protocol: Option<u32>,
    #[arg(long, value_name = "PORT", help_heading = "Misc")]
    pub port: Option<u16>,
    #[arg(
        short = '4',
        long = "ipv4",
        help_heading = "Misc",
        conflicts_with = "ipv6"
    )]
    pub ipv4: bool,
    #[arg(
        short = '6',
        long = "ipv6",
        help_heading = "Misc",
        conflicts_with = "ipv4"
    )]
    pub ipv6: bool,
    #[arg(
        short = 'B',
        long = "block-size",
        value_name = "SIZE",
        help_heading = "Misc"
    )]
    pub block_size: Option<usize>,
    #[arg(
        short = 'W',
        long,
        help_heading = "Misc",
        overrides_with = "no_whole_file"
    )]
    pub whole_file: bool,
    #[arg(
        long = "no-whole-file",
        help_heading = "Misc",
        overrides_with = "whole_file"
    )]
    pub no_whole_file: bool,
    #[arg(long = "link-dest", value_name = "DIR", help_heading = "Misc")]
    pub link_dest: Option<PathBuf>,
    #[arg(long = "copy-dest", value_name = "DIR", help_heading = "Misc")]
    pub copy_dest: Option<PathBuf>,
    #[arg(long = "compare-dest", value_name = "DIR", help_heading = "Misc")]
    pub compare_dest: Option<PathBuf>,
    #[arg(
        long = "mkpath",
        help_heading = "Misc",
        help = "create destination's missing path components"
    )]
    pub mkpath: bool,
    #[arg(long, help_heading = "Attributes")]
    pub numeric_ids: bool,
    #[arg(long, help_heading = "Output")]
    pub stats: bool,
    #[arg(long, value_name = "FILE")]
    pub config: Option<PathBuf>,
    #[arg(long, value_name = "FILE", env = "RSYNC_KNOWN_HOSTS")]
    pub known_hosts: Option<PathBuf>,
    #[arg(long, env = "RSYNC_NO_HOST_KEY_CHECKING")]
    pub no_host_key_checking: bool,
    #[arg(long = "password-file", value_name = "FILE")]
    pub password_file: Option<PathBuf>,
    #[arg(long = "early-input", value_name = "FILE")]
    pub early_input: Option<PathBuf>,
    #[arg(short = 'e', long, value_name = "COMMAND")]
    pub rsh: Option<String>,
    #[arg(
        short = 'M',
        long = "remote-option",
        value_name = "OPT",
        allow_hyphen_values = true,
        help = "send OPTION to the remote side only"
    )]
    pub remote_option: Vec<String>,
    #[arg(
        long = "old-args",
        help_heading = "Misc",
        help = "disable the modern arg-protection idiom",
        conflicts_with = "secluded_args"
    )]
    pub old_args: bool,
    #[arg(
        short = 's',
        long = "secluded-args",
        help_heading = "Misc",
        help = "use the protocol to safely send the args"
    )]
    pub secluded_args: bool,
    #[arg(long = "trust-sender", help_heading = "Misc")]
    pub trust_sender: bool,
    #[arg(
        long = "sockopts",
        value_name = "OPTIONS",
        value_delimiter = ',',
        allow_hyphen_values = true,
        help_heading = "Misc"
    )]
    pub sockopts: Vec<String>,
    #[arg(
        long = "iconv",
        value_name = "CONVERT_SPEC",
        help_heading = "Misc",
        help = "request charset conversion of filenames"
    )]
    pub iconv: Option<String>,
    #[arg(
        long = "write-batch",
        value_name = "FILE",
        help_heading = "Misc",
        conflicts_with = "read_batch"
    )]
    pub write_batch: Option<PathBuf>,
    #[arg(
        long = "read-batch",
        value_name = "FILE",
        help_heading = "Misc",
        help = "read a batched update from FILE",
        conflicts_with = "write_batch"
    )]
    pub read_batch: Option<PathBuf>,
    #[arg(long = "copy-devices", help_heading = "Misc")]
    pub copy_devices: bool,
    #[arg(
        long = "write-devices",
        help = "write to devices as files (implies --inplace)",
        help_heading = "Misc"
    )]
    pub write_devices: bool,
    #[arg(long, hide = true)]
    pub server: bool,
    #[arg(long, hide = true)]
    pub sender: bool,
    #[arg(long = "rsync-path", value_name = "PATH", alias = "rsync_path")]
    pub rsync_path: Option<String>,
    #[arg(value_name = "SRC", required_unless_present_any = ["daemon", "server", "probe"])]
    pub src: Option<String>,
    #[arg(value_name = "DST", required_unless_present_any = ["daemon", "server", "probe"])]
    pub dst: Option<String>,
    #[arg(short = 'f', long, value_name = "RULE", help_heading = "Selection")]
    pub filter: Vec<String>,
    #[arg(long, value_name = "FILE", help_heading = "Selection")]
    pub filter_file: Vec<PathBuf>,
    #[arg(short = 'F', action = ArgAction::Count, help_heading = "Selection")]
    pub filter_shorthand: u8,
    #[arg(
        short = 'C',
        long = "cvs-exclude",
        help_heading = "Selection",
        help = "auto-ignore files in the same way CVS does"
    )]
    pub cvs_exclude: bool,
    #[arg(long, value_name = "PATTERN")]
    pub include: Vec<String>,
    #[arg(long, value_name = "PATTERN")]
    pub exclude: Vec<String>,
    #[arg(long, value_name = "FILE")]
    pub include_from: Vec<PathBuf>,
    #[arg(long, value_name = "FILE")]
    pub exclude_from: Vec<PathBuf>,
    #[arg(long, value_name = "FILE")]
    pub files_from: Vec<PathBuf>,
    #[arg(long, short = '0')]
    pub from0: bool,
}

#[doc(hidden)]
#[derive(Parser, Debug)]
pub(crate) struct ProbeOpts {
    #[arg(long)]
    pub probe: bool,
    pub addr: Option<String>,
    #[arg(long, default_value_t = SUPPORTED_PROTOCOLS[0], value_name = "VER")]
    pub peer_version: u32,
}
pub fn cli_command() -> clap::Command {
    let cmd = ClientOpts::command();
    let cmd = ProbeOpts::augment_args(cmd);
    formatter::apply(cmd)
}
