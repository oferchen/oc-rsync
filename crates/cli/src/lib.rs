// crates/cli/src/lib.rs
use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use daemon::{parse_config_file, parse_module, Module};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::parser::ValueSource;
use clap::{ArgAction, ArgMatches, Args, CommandFactory, FromArgMatches, Parser};

mod formatter;
use compress::{available_codecs, Codec};
use encoding_rs::Encoding;
pub use engine::EngineError;
use engine::{pipe_sessions, sync, DeleteMode, IdMapper, Result, Stats, StrongHash, SyncOptions};
use filters::{default_cvs_rules, parse_with_options, Matcher, Rule};
pub use formatter::render_help;
use logging::{
    parse_escapes, progress_formatter, DebugFlag, InfoFlag, LogFormat, SubscriberConfig,
};
use meta::{parse_chmod, parse_chown, parse_id_map, IdKind};
use protocol::CharsetConv;
#[cfg(feature = "acl")]
use protocol::CAP_ACLS;
#[cfg(feature = "xattr")]
use protocol::CAP_XATTRS;
use protocol::{negotiate_version, ExitCode, CAP_CODECS, LATEST_VERSION, SUPPORTED_PROTOCOLS, V30};
use shell_words::split as shell_split;
use transport::{
    parse_sockopts, AddressFamily, RateLimitedTransport, SockOpt, SshStdioTransport, TcpTransport,
    Transport,
};
#[cfg(unix)]
use users::get_user_by_uid;

pub mod version;

pub fn print_version_if_requested<I>(args: I) -> bool
where
    I: IntoIterator<Item = OsString>,
{
    let mut show_version = false;
    let mut quiet = false;
    for arg in args {
        if arg == "--version" || arg == "-V" {
            show_version = true;
        } else if arg == "--quiet" || arg == "-q" {
            quiet = true;
        }
    }
    if show_version {
        if !quiet {
            println!("{}", version::render_version_lines().join("\n"));
        }
        true
    } else {
        false
    }
}

fn parse_filters(s: &str, from0: bool) -> std::result::Result<Vec<Rule>, filters::ParseError> {
    let mut v = HashSet::new();
    parse_with_options(s, from0, &mut v, 0, None)
}

fn parse_duration(s: &str) -> std::result::Result<Duration, std::num::ParseIntError> {
    Ok(Duration::from_secs(s.parse()?))
}

fn parse_nonzero_duration(s: &str) -> std::result::Result<Duration, String> {
    let d = parse_duration(s).map_err(|e| e.to_string())?;
    if d.as_secs() == 0 {
        Err("value must be greater than 0".into())
    } else {
        Ok(d)
    }
}

const SIZE_SUFFIXES: &[(char, u32)] = &[('k', 10), ('m', 20), ('g', 30)];

fn parse_suffixed<T>(s: &str, shifts: &[(char, u32)]) -> std::result::Result<T, String>
where
    T: TryFrom<u64>,
{
    let s = s.trim();
    if s == "0" {
        return T::try_from(0).map_err(|_| "size overflow".to_string());
    }
    if let Some(last) = s.chars().last() {
        if last.is_ascii_alphabetic() {
            let num = s[..s.len() - 1].parse::<u64>().map_err(|e| e.to_string())?;
            let shift = shifts
                .iter()
                .find(|(c, _)| last.eq_ignore_ascii_case(c))
                .map(|(_, s)| *s)
                .ok_or_else(|| format!("invalid size suffix: {last}"))?;
            let mult = 1u64 << shift;
            let val = num
                .checked_mul(mult)
                .ok_or_else(|| "size overflow".to_string())?;
            return T::try_from(val).map_err(|_| "size overflow".to_string());
        }
    }
    let val = s.parse::<u64>().map_err(|e| e.to_string())?;
    T::try_from(val).map_err(|_| "size overflow".to_string())
}

fn parse_size<T>(s: &str) -> std::result::Result<T, String>
where
    T: TryFrom<u64>,
{
    parse_suffixed(s, SIZE_SUFFIXES)
}

fn parse_dparam(s: &str) -> std::result::Result<(String, String), String> {
    let mut parts = s.splitn(2, '=');
    let name = parts
        .next()
        .ok_or_else(|| "invalid dparam".to_string())?
        .to_string();
    let value = parts
        .next()
        .ok_or_else(|| "invalid dparam".to_string())?
        .to_string();
    Ok((name, value))
}

fn parse_bool(s: &str) -> std::result::Result<bool, String> {
    if ["1", "true", "yes"]
        .iter()
        .any(|v| s.eq_ignore_ascii_case(v))
    {
        Ok(true)
    } else if ["0", "false", "no"]
        .iter()
        .any(|v| s.eq_ignore_ascii_case(v))
    {
        Ok(false)
    } else {
        Err("invalid boolean".to_string())
    }
}

pub fn parse_logging_flags(matches: &ArgMatches) -> (Vec<InfoFlag>, Vec<DebugFlag>) {
    let mut info: Vec<InfoFlag> = matches
        .get_many::<InfoFlag>("info")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    if matches.contains_id("out_format") && !info.contains(&InfoFlag::Name) {
        info.push(InfoFlag::Name);
    }
    let debug = matches
        .get_many::<DebugFlag>("debug")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    (info, debug)
}

fn init_logging(matches: &ArgMatches) {
    let verbose = matches.get_count("verbose");
    let quiet = matches.get_flag("quiet");
    let log_format = *matches
        .get_one::<LogFormat>("log_format")
        .unwrap_or(&LogFormat::Text);
    let log_file = matches.get_one::<PathBuf>("client-log-file").cloned();
    let log_file_fmt = matches.get_one::<String>("client-log-file-format").cloned();
    let syslog = matches.get_flag("syslog");
    let journald = matches.get_flag("journald");
    let (mut info, mut debug) = parse_logging_flags(matches);
    if quiet {
        info.clear();
        debug.clear();
    }
    let cfg = SubscriberConfig::builder()
        .format(log_format)
        .verbose(verbose)
        .info(info)
        .debug(debug)
        .quiet(quiet)
        .log_file(log_file.map(|p| (p, log_file_fmt)))
        .syslog(syslog)
        .journald(journald)
        .colored(true)
        .timestamps(false)
        .build();
    logging::init(cfg);
}

fn locale_charset() -> Option<String> {
    for var in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(val) = env::var(var) {
            if let Some(enc) = val.split('.').nth(1) {
                return Some(enc.to_string());
            }
        }
    }
    None
}

pub fn parse_iconv(spec: &str) -> std::result::Result<CharsetConv, String> {
    let mut parts = spec.split(',');
    let local_label = parts
        .next()
        .ok_or_else(|| "invalid iconv spec".to_string())?;
    let remote_label = parts.next().unwrap_or("UTF-8");

    let local_label = if local_label == "." {
        locale_charset().ok_or_else(|| "failed to determine locale charset".to_string())?
    } else {
        local_label.to_string()
    };
    let remote_label = if remote_label == "." {
        locale_charset().ok_or_else(|| "failed to determine locale charset".to_string())?
    } else {
        remote_label.to_string()
    };

    let local_enc = Encoding::for_label(local_label.as_bytes());
    let remote_enc = Encoding::for_label(remote_label.as_bytes());

    let local_enc = local_enc
        .ok_or_else(|| format!("iconv_open(\"{local_label}\", \"{remote_label}\") failed"))?;
    let remote_enc = remote_enc
        .ok_or_else(|| format!("iconv_open(\"{local_label}\", \"{remote_label}\") failed"))?;

    Ok(CharsetConv::new(remote_enc, local_enc))
}

#[derive(Parser, Debug)]
struct ClientOpts {
    #[arg(long)]
    local: bool,
    #[command(flatten)]
    daemon: DaemonOpts,
    #[arg(short = 'a', long, help_heading = "Selection")]
    archive: bool,
    #[arg(short, long, help_heading = "Selection")]
    recursive: bool,
    #[arg(short = 'd', long, help_heading = "Selection")]
    dirs: bool,
    #[arg(short = 'R', long, help_heading = "Selection")]
    relative: bool,
    #[arg(long = "no-implied-dirs", help_heading = "Selection")]
    no_implied_dirs: bool,
    #[arg(short = 'n', long, help_heading = "Selection")]
    dry_run: bool,
    #[arg(long = "list-only", help_heading = "Output")]
    list_only: bool,
    #[arg(short = 'S', long, help_heading = "Selection")]
    sparse: bool,
    #[arg(short = 'u', long, help_heading = "Misc")]
    update: bool,
    #[arg(long, help_heading = "Misc")]
    existing: bool,
    #[arg(long, help_heading = "Misc")]
    ignore_existing: bool,
    #[arg(short = 'x', long = "one-file-system", help_heading = "Selection")]
    one_file_system: bool,
    #[arg(short = 'm', long = "prune-empty-dirs", help_heading = "Misc")]
    prune_empty_dirs: bool,
    #[arg(long = "size-only", help_heading = "Misc")]
    size_only: bool,
    #[arg(short = 'I', long = "ignore-times", help_heading = "Misc")]
    ignore_times: bool,
    #[arg(short, long, action = ArgAction::Count, help_heading = "Output")]
    verbose: u8,
    #[arg(long = "log-format", help_heading = "Output", value_enum)]
    log_format: Option<LogFormat>,
    #[arg(
        long = "log-file",
        value_name = "FILE",
        help_heading = "Output",
        id = "client-log-file"
    )]
    log_file: Option<PathBuf>,
    #[arg(
        long = "log-file-format",
        value_name = "FMT",
        help_heading = "Output",
        id = "client-log-file-format"
    )]
    log_file_format: Option<String>,
    #[arg(long, help_heading = "Output", env = "OC_RSYNC_SYSLOG")]
    syslog: bool,
    #[arg(long, help_heading = "Output", env = "OC_RSYNC_JOURNALD")]
    journald: bool,
    #[arg(long = "out-format", value_name = "FORMAT", help_heading = "Output")]
    out_format: Option<String>,
    #[arg(
        long,
        value_name = "FLAGS",
        value_delimiter = ',',
        value_enum,
        help_heading = "Output"
    )]
    info: Vec<InfoFlag>,
    #[arg(
        long,
        value_name = "FLAGS",
        value_delimiter = ',',
        value_enum,
        help_heading = "Output"
    )]
    debug: Vec<DebugFlag>,
    #[arg(long = "human-readable", help_heading = "Output")]
    human_readable: bool,
    #[arg(short, long, help_heading = "Output")]
    quiet: bool,
    #[arg(long, help_heading = "Output")]
    no_motd: bool,
    #[arg(short = '8', long = "8-bit-output", help_heading = "Output")]
    eight_bit_output: bool,
    #[arg(
        short = 'i',
        long = "itemize-changes",
        help_heading = "Output",
        help = "output a change-summary for all updates"
    )]
    itemize_changes: bool,
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
    delete: bool,
    #[arg(
        long = "delete-before",
        help_heading = "Delete",
        overrides_with_all = ["delete", "delete_during", "delete_after", "delete_delay"]
    )]
    delete_before: bool,
    #[arg(
        long = "delete-during",
        help_heading = "Delete",
        visible_alias = "del",
        overrides_with_all = ["delete", "delete_before", "delete_after", "delete_delay"]
    )]
    delete_during: bool,
    #[arg(
        long = "delete-after",
        help_heading = "Delete",
        overrides_with_all = ["delete", "delete_before", "delete_during", "delete_delay"]
    )]
    delete_after: bool,
    #[arg(
        long = "delete-delay",
        help_heading = "Delete",
        overrides_with_all = ["delete", "delete_before", "delete_during", "delete_after"]
    )]
    delete_delay: bool,
    #[arg(long = "delete-excluded", help_heading = "Delete")]
    delete_excluded: bool,
    #[arg(long = "delete-missing-args", help_heading = "Delete")]
    delete_missing_args: bool,
    #[arg(long = "ignore-missing-args", help_heading = "Delete")]
    ignore_missing_args: bool,
    #[arg(
        long = "remove-source-files",
        help_heading = "Delete",
        visible_alias = "remove-sent-files"
    )]
    remove_source_files: bool,
    #[arg(long = "ignore-errors", help_heading = "Delete")]
    ignore_errors: bool,
    #[arg(
        long,
        help_heading = "Delete",
        help = "force deletion of dirs even if not empty",
        action = ArgAction::SetTrue
    )]
    force: bool,
    #[arg(long = "max-delete", value_name = "NUM", help_heading = "Delete")]
    max_delete: Option<usize>,
    #[arg(
        long = "max-alloc",
        value_name = "SIZE",
        value_parser = parse_size::<usize>,
        help_heading = "Misc"
    )]
    max_alloc: Option<usize>,
    #[arg(
        long = "max-size",
        value_name = "SIZE",
        value_parser = parse_size::<u64>,
        help_heading = "Misc"
    )]
    max_size: Option<u64>,
    #[arg(
        long = "min-size",
        value_name = "SIZE",
        value_parser = parse_size::<u64>,
        help_heading = "Misc"
    )]
    min_size: Option<u64>,
    #[arg(
        long,
        help_heading = "Misc",
        help = "allocate dest files before writing them"
    )]
    preallocate: bool,
    #[arg(short = 'b', long, help_heading = "Backup")]
    backup: bool,
    #[arg(long = "backup-dir", value_name = "DIR", help_heading = "Backup")]
    backup_dir: Option<PathBuf>,
    #[arg(
        long = "suffix",
        value_name = "SUFFIX",
        default_value = "~",
        help_heading = "Backup"
    )]
    suffix: String,
    #[arg(short = 'c', long, help_heading = "Attributes")]
    checksum: bool,
    #[arg(
        long = "checksum-choice",
        value_name = "STR",
        help_heading = "Attributes",
        visible_alias = "cc"
    )]
    checksum_choice: Option<String>,
    #[arg(
        long = "checksum-seed",
        value_name = "NUM",
        value_parser = clap::value_parser!(u32),
        help_heading = "Attributes",
        help = "set block/file checksum seed (advanced)"
    )]
    checksum_seed: Option<u32>,
    #[arg(
        short = 'p',
        long,
        help_heading = "Attributes",
        overrides_with = "no_perms"
    )]
    perms: bool,
    #[arg(
        long = "no-perms",
        help_heading = "Attributes",
        overrides_with = "perms"
    )]
    no_perms: bool,
    #[arg(short = 'E', long, help_heading = "Attributes")]
    executability: bool,
    #[arg(long = "chmod", value_name = "CHMOD", help_heading = "Attributes")]
    chmod: Vec<String>,
    #[arg(long = "chown", value_name = "USER:GROUP", help_heading = "Attributes")]
    chown: Option<String>,
    #[arg(
        long = "copy-as",
        value_name = "USER[:GROUP]",
        help_heading = "Attributes"
    )]
    copy_as: Option<String>,
    #[arg(
        long = "usermap",
        value_name = "FROM:TO",
        value_delimiter = ',',
        help_heading = "Attributes"
    )]
    usermap: Vec<String>,
    #[arg(
        long = "groupmap",
        value_name = "FROM:TO",
        value_delimiter = ',',
        help_heading = "Attributes"
    )]
    groupmap: Vec<String>,
    #[arg(
        short = 't',
        long,
        help_heading = "Attributes",
        overrides_with = "no_times"
    )]
    times: bool,
    #[arg(
        long = "no-times",
        help_heading = "Attributes",
        overrides_with = "times"
    )]
    no_times: bool,
    #[arg(short = 'U', long, help_heading = "Attributes")]
    atimes: bool,
    #[arg(short = 'N', long, help_heading = "Attributes")]
    crtimes: bool,
    #[arg(short = 'O', long, help_heading = "Attributes")]
    omit_dir_times: bool,
    #[arg(short = 'J', long, help_heading = "Attributes")]
    omit_link_times: bool,
    #[arg(
        short = 'o',
        long,
        help_heading = "Attributes",
        overrides_with = "no_owner"
    )]
    owner: bool,
    #[arg(
        long = "no-owner",
        alias = "no-o",
        help_heading = "Attributes",
        overrides_with = "owner",
        alias = "no-o"
    )]
    no_owner: bool,
    #[arg(
        short = 'g',
        long,
        help_heading = "Attributes",
        overrides_with = "no_group"
    )]
    group: bool,
    #[arg(
        long = "no-group",
        help_heading = "Attributes",
        overrides_with = "group",
        alias = "no-g"
    )]
    no_group: bool,
    #[arg(
        short = 'l',
        long,
        help_heading = "Attributes",
        overrides_with = "no_links"
    )]
    links: bool,
    #[arg(
        long = "no-links",
        help_heading = "Attributes",
        overrides_with = "links"
    )]
    no_links: bool,
    #[arg(short = 'L', long, help_heading = "Attributes")]
    copy_links: bool,
    #[arg(short = 'k', long, help_heading = "Attributes")]
    copy_dirlinks: bool,
    #[arg(short = 'K', long, help_heading = "Attributes")]
    keep_dirlinks: bool,
    #[arg(long, help_heading = "Attributes")]
    copy_unsafe_links: bool,
    #[arg(long, help_heading = "Attributes")]
    safe_links: bool,
    #[arg(
        long,
        help_heading = "Attributes",
        help = "munge symlinks to make them safe & unusable"
    )]
    munge_links: bool,
    #[arg(short = 'H', long = "hard-links", help_heading = "Attributes")]
    hard_links: bool,
    #[arg(long, help_heading = "Attributes", overrides_with = "no_devices")]
    devices: bool,
    #[arg(
        long = "no-devices",
        help_heading = "Attributes",
        overrides_with = "devices"
    )]
    no_devices: bool,
    #[arg(long, help_heading = "Attributes", overrides_with = "no_specials")]
    specials: bool,
    #[arg(
        long = "no-specials",
        help_heading = "Attributes",
        overrides_with = "specials"
    )]
    no_specials: bool,
    #[arg(short = 'D', help_heading = "Attributes")]
    devices_specials: bool,
    #[cfg(feature = "xattr")]
    #[arg(long, help_heading = "Attributes")]
    xattrs: bool,
    #[cfg(feature = "acl")]
    #[arg(
        short = 'A',
        long,
        help_heading = "Attributes",
        overrides_with = "no_acls"
    )]
    acls: bool,
    #[cfg(feature = "acl")]
    #[arg(long = "no-acls", help_heading = "Attributes", overrides_with = "acls")]
    no_acls: bool,
    #[arg(long = "fake-super", help_heading = "Attributes")]
    fake_super: bool,
    #[arg(long = "super", help_heading = "Attributes")]
    super_user: bool,
    #[arg(short = 'z', long, help_heading = "Compression")]
    compress: bool,
    #[arg(
        long = "compress-choice",
        value_name = "STR",
        help_heading = "Compression",
        visible_alias = "zc"
    )]
    compress_choice: Option<String>,
    #[arg(
        long = "compress-level",
        value_name = "NUM",
        help_heading = "Compression",
        visible_alias = "zl"
    )]
    compress_level: Option<i32>,
    #[arg(
        long = "skip-compress",
        value_name = "LIST",
        help_heading = "Compression",
        value_delimiter = ','
    )]
    skip_compress: Vec<String>,

    #[arg(long, help_heading = "Misc")]
    partial: bool,
    #[arg(long = "partial-dir", value_name = "DIR", help_heading = "Misc")]
    partial_dir: Option<PathBuf>,
    #[arg(
        short = 'T',
        long = "temp-dir",
        value_name = "DIR",
        help_heading = "Misc"
    )]
    temp_dir: Option<PathBuf>,
    #[arg(long, help_heading = "Misc")]
    progress: bool,
    #[arg(long, help_heading = "Misc")]
    blocking_io: bool,
    #[arg(long, help_heading = "Misc")]
    fsync: bool,
    #[arg(short = 'y', long = "fuzzy", help_heading = "Misc")]
    fuzzy: bool,
    #[arg(short = 'P', help_heading = "Misc")]
    partial_progress: bool,
    #[arg(long, help_heading = "Misc")]
    append: bool,
    #[arg(long = "append-verify", help_heading = "Misc")]
    append_verify: bool,
    #[arg(long, help_heading = "Misc")]
    inplace: bool,
    #[arg(long = "delay-updates", help_heading = "Misc")]
    delay_updates: bool,
    #[arg(long = "bwlimit", value_name = "RATE", help_heading = "Misc")]
    bwlimit: Option<u64>,
    #[arg(long = "timeout", value_name = "SECONDS", value_parser = parse_duration, help_heading = "Misc")]
    timeout: Option<Duration>,
    #[arg(
        long = "connect-timeout",
        alias = "contimeout",
        value_name = "SECONDS",
        value_parser = parse_nonzero_duration,
        help_heading = "Misc"
    )]
    connect_timeout: Option<Duration>,
    #[arg(long = "modify-window", value_name = "SECONDS", value_parser = parse_duration, help_heading = "Misc")]
    modify_window: Option<Duration>,
    #[arg(
        long = "protocol",
        value_name = "VER",
        value_parser = clap::value_parser!(u32),
        help_heading = "Misc",
        help = "force an older protocol version"
    )]
    protocol: Option<u32>,
    #[arg(long, value_name = "PORT", help_heading = "Misc")]
    port: Option<u16>,
    #[arg(
        short = '4',
        long = "ipv4",
        help_heading = "Misc",
        conflicts_with = "ipv6"
    )]
    ipv4: bool,
    #[arg(
        short = '6',
        long = "ipv6",
        help_heading = "Misc",
        conflicts_with = "ipv4"
    )]
    ipv6: bool,
    #[arg(
        short = 'B',
        long = "block-size",
        value_name = "SIZE",
        help_heading = "Misc"
    )]
    block_size: Option<usize>,
    #[arg(
        short = 'W',
        long,
        help_heading = "Misc",
        overrides_with = "no_whole_file"
    )]
    whole_file: bool,
    #[arg(
        long = "no-whole-file",
        help_heading = "Misc",
        overrides_with = "whole_file"
    )]
    no_whole_file: bool,
    #[arg(long = "link-dest", value_name = "DIR", help_heading = "Misc")]
    link_dest: Option<PathBuf>,
    #[arg(long = "copy-dest", value_name = "DIR", help_heading = "Misc")]
    copy_dest: Option<PathBuf>,
    #[arg(long = "compare-dest", value_name = "DIR", help_heading = "Misc")]
    compare_dest: Option<PathBuf>,
    #[arg(
        long = "mkpath",
        help_heading = "Misc",
        help = "create destination's missing path components"
    )]
    mkpath: bool,
    #[arg(long, help_heading = "Attributes")]
    numeric_ids: bool,
    #[arg(long, help_heading = "Output")]
    stats: bool,
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(long, value_name = "FILE", env = "RSYNC_KNOWN_HOSTS")]
    known_hosts: Option<PathBuf>,
    #[arg(long, env = "RSYNC_NO_HOST_KEY_CHECKING")]
    no_host_key_checking: bool,
    #[arg(long = "password-file", value_name = "FILE")]
    password_file: Option<PathBuf>,
    #[arg(long = "early-input", value_name = "FILE")]
    early_input: Option<PathBuf>,
    #[arg(short = 'e', long, value_name = "COMMAND")]
    rsh: Option<String>,
    #[arg(
        short = 'M',
        long = "remote-option",
        value_name = "OPT",
        allow_hyphen_values = true,
        help = "send OPTION to the remote side only"
    )]
    remote_option: Vec<String>,
    #[arg(
        short = 's',
        long = "secluded-args",
        help_heading = "Misc",
        help = "use the protocol to safely send the args"
    )]
    secluded_args: bool,
    #[arg(long = "trust-sender", help_heading = "Misc")]
    trust_sender: bool,
    #[arg(
        long = "sockopts",
        value_name = "OPTIONS",
        value_delimiter = ',',
        allow_hyphen_values = true,
        help_heading = "Misc"
    )]
    sockopts: Vec<String>,
    #[arg(
        long = "iconv",
        value_name = "CONVERT_SPEC",
        help_heading = "Misc",
        help = "request charset conversion of filenames"
    )]
    iconv: Option<String>,
    #[arg(
        long = "write-batch",
        value_name = "FILE",
        help_heading = "Misc",
        conflicts_with = "read_batch"
    )]
    write_batch: Option<PathBuf>,
    #[arg(
        long = "read-batch",
        value_name = "FILE",
        help_heading = "Misc",
        help = "read a batched update from FILE",
        conflicts_with = "write_batch"
    )]
    read_batch: Option<PathBuf>,
    #[arg(long = "copy-devices", help_heading = "Misc")]
    copy_devices: bool,
    #[arg(
        long = "write-devices",
        help = "write to devices as files (implies --inplace)",
        help_heading = "Misc"
    )]
    write_devices: bool,
    #[arg(long, hide = true)]
    server: bool,
    #[arg(long, hide = true)]
    sender: bool,
    #[arg(long = "rsync-path", value_name = "PATH", alias = "rsync_path")]
    rsync_path: Option<String>,
    #[arg(value_name = "SRC", required_unless_present_any = ["daemon", "server", "probe"])]
    src: Option<String>,
    #[arg(value_name = "DST", required_unless_present_any = ["daemon", "server", "probe"])]
    dst: Option<String>,
    #[arg(short = 'f', long, value_name = "RULE", help_heading = "Selection")]
    filter: Vec<String>,
    #[arg(long, value_name = "FILE", help_heading = "Selection")]
    filter_file: Vec<PathBuf>,
    #[arg(short = 'F', action = ArgAction::Count, help_heading = "Selection")]
    filter_shorthand: u8,
    #[arg(
        short = 'C',
        long = "cvs-exclude",
        help_heading = "Selection",
        help = "auto-ignore files in the same way CVS does"
    )]
    cvs_exclude: bool,
    #[arg(long, value_name = "PATTERN")]
    include: Vec<String>,
    #[arg(long, value_name = "PATTERN")]
    exclude: Vec<String>,
    #[arg(long, value_name = "FILE")]
    include_from: Vec<PathBuf>,
    #[arg(long, value_name = "FILE")]
    exclude_from: Vec<PathBuf>,
    #[arg(long, value_name = "FILE")]
    files_from: Vec<PathBuf>,
    #[arg(long, short = '0')]
    from0: bool,
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RshCommand {
    pub env: Vec<(String, String)>,
    pub cmd: Vec<String>,
}

fn parse_env_command(parts: Vec<String>) -> Result<RshCommand> {
    let mut env = Vec::new();
    let mut iter = parts.into_iter();
    let mut cmd = Vec::new();

    while let Some(tok) = iter.next() {
        if let Some((k, v)) = tok.split_once('=') {
            let valid = !k.is_empty()
                && (k.as_bytes()[0].is_ascii_alphabetic() || k.as_bytes()[0] == b'_')
                && k.as_bytes()[1..]
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_');
            if valid {
                env.push((k.to_string(), v.to_string()));
                continue;
            } else {
                return Err(EngineError::Other("invalid environment assignment".into()));
            }
        }
        cmd.push(tok);
        cmd.extend(iter);
        return Ok(RshCommand { env, cmd });
    }
    Ok(RshCommand { env, cmd })
}

pub fn parse_rsh(raw: Option<String>) -> Result<RshCommand> {
    match raw {
        Some(s) => {
            let parts = shell_split(&s).map_err(|e| EngineError::Other(e.to_string()))?;
            let mut cmd = parse_env_command(parts)?;
            if cmd.cmd.is_empty() {
                cmd.cmd.push("ssh".to_string());
            }
            Ok(cmd)
        }
        None => Ok(RshCommand {
            env: Vec::new(),
            cmd: vec!["ssh".to_string()],
        }),
    }
}

pub fn parse_rsync_path(raw: Option<String>) -> Result<Option<RshCommand>> {
    match raw {
        Some(s) => {
            let parts = shell_split(&s).map_err(|e| EngineError::Other(e.to_string()))?;
            let cmd = parse_env_command(parts)?;
            if cmd.env.is_empty() && cmd.cmd.is_empty() {
                Ok(None)
            } else {
                Ok(Some(cmd))
            }
        }
        None => Ok(None),
    }
}

fn parse_name_map(specs: &[String], kind: IdKind) -> Result<Option<IdMapper>> {
    if specs.is_empty() {
        Ok(None)
    } else {
        let spec = specs.join(",");
        let mapper = parse_id_map(&spec, kind).map_err(EngineError::Other)?;
        Ok(Some(IdMapper(mapper)))
    }
}

#[derive(Args, Debug)]
struct DaemonOpts {
    #[arg(long)]
    daemon: bool,
    #[arg(long = "no-detach")]
    no_detach: bool,
    #[arg(long, value_parser = parse_module, value_name = "NAME=PATH")]
    module: Vec<Module>,
    #[arg(long)]
    address: Option<IpAddr>,
    #[arg(long = "secrets-file", value_name = "FILE")]
    secrets_file: Option<PathBuf>,
    #[arg(long = "hosts-allow", value_delimiter = ',', value_name = "LIST")]
    hosts_allow: Vec<String>,
    #[arg(long = "hosts-deny", value_delimiter = ',', value_name = "LIST")]
    hosts_deny: Vec<String>,
    #[arg(long = "motd", value_name = "FILE")]
    motd: Option<PathBuf>,
    #[arg(long = "pid-file", value_name = "FILE")]
    pid_file: Option<PathBuf>,
    #[arg(long = "lock-file", value_name = "FILE")]
    lock_file: Option<PathBuf>,
    #[arg(long = "state-dir", value_name = "DIR")]
    state_dir: Option<PathBuf>,
    #[arg(long = "dparam", value_name = "NAME=VALUE", value_parser = parse_dparam)]
    dparam: Vec<(String, String)>,
}

#[derive(Parser, Debug)]
struct ProbeOpts {
    #[arg(long)]
    probe: bool,
    addr: Option<String>,
    #[arg(long, default_value_t = SUPPORTED_PROTOCOLS[0], value_name = "VER")]
    peer_version: u32,
}

pub fn run(matches: &clap::ArgMatches) -> Result<()> {
    init_logging(matches);
    let opts =
        ClientOpts::from_arg_matches(matches).map_err(|e| EngineError::Other(e.to_string()))?;
    if opts.daemon.daemon {
        return run_daemon(opts.daemon, matches);
    }
    let probe_opts =
        ProbeOpts::from_arg_matches(matches).map_err(|e| EngineError::Other(e.to_string()))?;
    if probe_opts.probe {
        return run_probe(probe_opts, matches.get_flag("quiet"));
    }
    let mut opts =
        ClientOpts::from_arg_matches(matches).map_err(|e| EngineError::Other(e.to_string()))?;
    if matches.value_source("secluded_args") != Some(ValueSource::CommandLine) {
        if let Ok(val) = env::var("RSYNC_PROTECT_ARGS") {
            if val != "0" {
                opts.secluded_args = true;
            }
        }
    }
    run_client(opts, matches)
}

pub fn cli_command() -> clap::Command {
    let cmd = ClientOpts::command();
    let cmd = ProbeOpts::augment_args(cmd);
    formatter::apply(cmd)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSpec {
    path: PathBuf,
    trailing_slash: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteSpec {
    Local(PathSpec),
    Remote {
        host: String,
        path: PathSpec,
        module: Option<String>,
    },
}

pub fn parse_remote_spec(input: &str) -> Result<RemoteSpec> {
    let (trailing_slash, s) = if input != "/" && input.ends_with('/') {
        (true, &input[..input.len() - 1])
    } else {
        (false, input)
    };
    if let Some(rest) = s.strip_prefix("rsync://") {
        let mut parts = rest.splitn(2, '/');
        let host = parts.next().unwrap_or("");
        let mod_path = parts.next().unwrap_or("");
        let mut mp = mod_path.splitn(2, '/');
        let module = mp.next().unwrap_or("");
        let path = mp.next().unwrap_or("");
        return Ok(RemoteSpec::Remote {
            host: host.to_string(),
            path: PathSpec {
                path: PathBuf::from(path),
                trailing_slash,
            },
            module: Some(module.to_string()),
        });
    }
    if let Some(rest) = s.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let host = &rest[..end];
            if let Some(path) = rest[end + 1..].strip_prefix(':') {
                return Ok(RemoteSpec::Remote {
                    host: host.to_string(),
                    path: PathSpec {
                        path: PathBuf::from(path),
                        trailing_slash,
                    },
                    module: None,
                });
            }
        }
        return Ok(RemoteSpec::Local(PathSpec {
            path: PathBuf::from(input),
            trailing_slash,
        }));
    }
    if let Some(idx) = s.find("::") {
        let host = &s[..idx];
        let mut rest = s[idx + 2..].splitn(2, '/');
        let module = rest.next().unwrap_or("");
        let path = rest.next().unwrap_or("");
        return Ok(RemoteSpec::Remote {
            host: host.to_string(),
            path: PathSpec {
                path: PathBuf::from(path),
                trailing_slash,
            },
            module: Some(module.to_string()),
        });
    }
    if let Some(idx) = s.find(':') {
        if idx == 1 {
            let bytes = s.as_bytes();
            if bytes[0].is_ascii_alphabetic()
                && (bytes.len() == 2
                    || bytes
                        .get(2)
                        .map(|c| *c == b'/' || *c == b'\\')
                        .unwrap_or(false))
            {
                return Ok(RemoteSpec::Local(PathSpec {
                    path: PathBuf::from(s),
                    trailing_slash,
                }));
            }
        }
        let (host, path) = s.split_at(idx);
        return Ok(RemoteSpec::Remote {
            host: host.to_string(),
            path: PathSpec {
                path: PathBuf::from(&path[1..]),
                trailing_slash,
            },
            module: None,
        });
    }
    Ok(RemoteSpec::Local(PathSpec {
        path: PathBuf::from(s),
        trailing_slash,
    }))
}

pub fn parse_remote_specs(src: &str, dst: &str) -> Result<(RemoteSpec, RemoteSpec)> {
    let src_spec = parse_remote_spec(src)?;
    let dst_spec = parse_remote_spec(dst)?;
    if let (
        RemoteSpec::Remote {
            host: sh, path: sp, ..
        },
        RemoteSpec::Remote {
            host: dh, path: dp, ..
        },
    ) = (&src_spec, &dst_spec)
    {
        if sh.is_empty() || dh.is_empty() {
            return Err(EngineError::Other("remote host missing".into()));
        }
        if sp.path.as_os_str().is_empty() || dp.path.as_os_str().is_empty() {
            return Err(EngineError::Other("remote path missing".into()));
        }
    }
    Ok((src_spec, dst_spec))
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_daemon_session(
    host: &str,
    module: &str,
    port: Option<u16>,
    password_file: Option<&Path>,
    no_motd: bool,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    family: Option<AddressFamily>,
    sockopts: &[String],
    opts: &SyncOptions,
    version: u32,
    early_input: Option<&Path>,
    iconv: Option<&CharsetConv>,
) -> Result<TcpTransport> {
    let (host, port) = if let Some((h, p)) = host.rsplit_once(':') {
        let p = p.parse().unwrap_or(873);
        (h, p)
    } else {
        (host, port.unwrap_or(873))
    };
    let start = Instant::now();
    let mut t =
        TcpTransport::connect(host, port, connect_timeout, family).map_err(EngineError::from)?;
    let parsed: Vec<SockOpt> = parse_sockopts(sockopts).map_err(EngineError::Other)?;
    t.apply_sockopts(&parsed).map_err(EngineError::from)?;
    let handshake_timeout = connect_timeout
        .map(|dur| {
            dur.checked_sub(start.elapsed())
                .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "connection timed out"))
        })
        .transpose()
        .map_err(EngineError::from)?;
    t.set_read_timeout(handshake_timeout)
        .map_err(EngineError::from)?;
    t.set_write_timeout(handshake_timeout)
        .map_err(EngineError::from)?;
    if let Some(p) = early_input {
        if let Ok(data) = fs::read(p) {
            t.send(&data).map_err(EngineError::from)?;
        }
    }
    t.send(&version.to_be_bytes()).map_err(EngineError::from)?;
    let mut buf = [0u8; 4];
    t.receive(&mut buf).map_err(EngineError::from)?;
    let peer = u32::from_be_bytes(buf);
    negotiate_version(version, peer).map_err(|e| EngineError::Other(e.to_string()))?;

    let token = password_file
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()));
    t.authenticate(token.as_deref(), no_motd)
        .map_err(EngineError::from)?;

    let mut line = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let n = t.receive(&mut b).map_err(EngineError::from)?;
        if n == 0 {
            break;
        }
        line.push(b[0]);
        if b[0] == b'\n' {
            if line == b"@RSYNCD: OK\n" {
                break;
            }
            if !no_motd {
                let s = if let Some(cv) = iconv {
                    cv.decode_remote(&line)
                } else {
                    String::from_utf8_lossy(&line).into_owned()
                };
                if let Some(msg) = s.strip_prefix("@RSYNCD: ") {
                    print!("{msg}");
                } else {
                    print!("{s}");
                }
                let _ = io::stdout().flush();
            }
            line.clear();
        }
    }
    t.set_read_timeout(timeout).map_err(EngineError::from)?;
    t.set_write_timeout(timeout).map_err(EngineError::from)?;

    if let Some(cv) = iconv {
        let mut line = cv.encode_remote(module);
        line.push(b'\n');
        t.send(&line).map_err(EngineError::from)?;
        for opt in &opts.remote_options {
            let mut o = cv.encode_remote(opt);
            o.push(b'\n');
            t.send(&o).map_err(EngineError::from)?;
        }
    } else {
        let line = format!("{module}\n");
        t.send(line.as_bytes()).map_err(EngineError::from)?;
        for opt in &opts.remote_options {
            let o = format!("{opt}\n");
            t.send(o.as_bytes()).map_err(EngineError::from)?;
        }
    }
    t.send(b"\n").map_err(EngineError::from)?;
    Ok(t)
}

fn run_client(mut opts: ClientOpts, matches: &ArgMatches) -> Result<()> {
    let src_arg = opts
        .src
        .take()
        .ok_or_else(|| EngineError::Other("missing SRC".into()))?;
    let dst_arg = opts
        .dst
        .take()
        .ok_or_else(|| EngineError::Other("missing DST".into()))?;
    if opts.archive {
        opts.recursive = true;
        if !opts.no_links {
            opts.links = true;
        }
        if !opts.no_perms {
            opts.perms = true;
        }
        if !opts.no_times {
            opts.times = true;
        }
        if !opts.no_group {
            opts.group = true;
        }
        if !opts.no_owner {
            opts.owner = true;
        }
        if !opts.no_devices {
            opts.devices = true;
        }
        if !opts.no_specials {
            opts.specials = true;
        }
    }
    let matcher = build_matcher(&opts, matches)?;
    let addr_family = if opts.ipv4 {
        Some(AddressFamily::V4)
    } else if opts.ipv6 {
        Some(AddressFamily::V6)
    } else {
        None
    };

    parse_sockopts(&opts.sockopts).map_err(EngineError::Other)?;

    #[cfg(feature = "acl")]
    let acls = opts.acls && !opts.no_acls;

    #[cfg(unix)]
    {
        let need_owner = if opts.no_owner {
            false
        } else {
            opts.owner || opts.archive
        };
        let need_group = if opts.no_group {
            false
        } else {
            opts.group || opts.archive
        };
        let needs_privs = need_owner
            || need_group
            || opts.chown.is_some()
            || !opts.usermap.is_empty()
            || !opts.groupmap.is_empty();
        let numeric_fallback = opts.numeric_ids
            && opts.chown.is_none()
            && opts.usermap.is_empty()
            && opts.groupmap.is_empty();
        if needs_privs && !numeric_fallback {
            use nix::unistd::Uid;
            if !Uid::effective().is_root() {
                #[cfg(target_os = "linux")]
                {
                    use caps::{CapSet, Capability};
                    if !caps::has_cap(None, CapSet::Effective, Capability::CAP_CHOWN)
                        .unwrap_or(false)
                    {
                        return Err(EngineError::Exit(
                            ExitCode::StartClient,
                            "changing ownership requires root or CAP_CHOWN".into(),
                        ));
                    }
                }
                #[cfg(not(target_os = "linux"))]
                {
                    return Err(EngineError::Exit(
                        ExitCode::StartClient,
                        "changing ownership requires root".into(),
                    ));
                }
            }
        }
    }

    let iconv = if let Some(spec) = &opts.iconv {
        Some(parse_iconv(spec).map_err(EngineError::Other)?)
    } else {
        None
    };

    if let Some(pf) = &opts.password_file {
        #[cfg(unix)]
        {
            let mode = fs::metadata(pf)?.permissions().mode();
            if mode & 0o077 != 0 {
                return Err(EngineError::Other(
                    "password file permissions are too open".into(),
                ));
            }
        }
        let _ = fs::read_to_string(pf).map_err(|e| EngineError::Other(e.to_string()))?;
    }

    let mut remote_opts = opts.remote_option.clone();
    if opts.secluded_args {
        remote_opts.push("--secluded-args".into());
    }
    if opts.trust_sender {
        remote_opts.push("--trust-sender".into());
    }
    if let Some(spec) = &opts.iconv {
        remote_opts.push(format!("--iconv={spec}"));
    }
    #[cfg(feature = "xattr")]
    if opts.xattrs {
        remote_opts.push("--xattrs".into());
    }
    #[cfg(feature = "acl")]
    if acls {
        remote_opts.push("--acls".into());
    }

    if let Some(cfg) = &opts.config {
        if !opts.quiet {
            println!("using config file {}", cfg.display());
        }
    }
    if opts.verbose > 0 && !opts.quiet {
        tracing::info!(
            target: InfoFlag::Misc.target(),
            "verbose level set to {}",
            opts.verbose
        );
    }
    if opts.recursive && !opts.quiet {
        println!("recursive mode enabled");
    }
    let (src, mut dst) = parse_remote_specs(&src_arg, &dst_arg)?;
    if opts.mkpath {
        match &dst {
            RemoteSpec::Local(ps) => {
                let target = if ps.trailing_slash {
                    &ps.path
                } else {
                    ps.path.parent().unwrap_or(&ps.path)
                };
                fs::create_dir_all(target).map_err(|e| EngineError::Other(e.to_string()))?;
            }
            RemoteSpec::Remote { .. } => {
                remote_opts.push("--mkpath".into());
            }
        }
    }

    let known_hosts = opts.known_hosts.clone();
    let strict_host_key_checking = !opts.no_host_key_checking;
    let rsh_raw = opts.rsh.clone().or_else(|| env::var("RSYNC_RSH").ok());
    let rsh_cmd = parse_rsh(rsh_raw)?;
    let rsync_path_cmd = parse_rsync_path(opts.rsync_path.clone())?;
    let mut rsync_env: Vec<(String, String)> = env::vars()
        .filter(|(k, _)| k.starts_with("RSYNC_"))
        .collect();
    rsync_env.extend(
        rsh_cmd
            .env
            .iter()
            .filter(|(k, _)| k.starts_with("RSYNC_"))
            .cloned(),
    );
    if let Some(cmd) = &rsync_path_cmd {
        rsync_env.extend(
            cmd.env
                .iter()
                .filter(|(k, _)| k.starts_with("RSYNC_"))
                .cloned(),
        );
    }
    if let Some(to) = opts.timeout {
        rsync_env.push(("RSYNC_TIMEOUT".into(), to.as_secs().to_string()));
    }

    let proto = opts.protocol.unwrap_or(LATEST_VERSION);
    if !rsync_env.iter().any(|(k, _)| k == "RSYNC_CHECKSUM_LIST") {
        let list = if proto < V30 {
            ["md4", "md5", "sha1"]
        } else {
            ["md5", "md4", "sha1"]
        };
        rsync_env.push(("RSYNC_CHECKSUM_LIST".into(), list.join(",")));
    }

    let remote_bin_vec = rsync_path_cmd.as_ref().map(|c| c.cmd.clone());
    let remote_env_vec = rsync_path_cmd.as_ref().map(|c| c.env.clone());

    let strong = if proto < V30 {
        StrongHash::Md4
    } else if let Some(choice) = opts.checksum_choice.as_deref() {
        match choice {
            "md4" => StrongHash::Md4,
            "md5" => StrongHash::Md5,
            "sha1" => StrongHash::Sha1,
            other => {
                return Err(EngineError::Other(format!("unknown checksum {other}")));
            }
        }
    } else if let Ok(list) = env::var("RSYNC_CHECKSUM_LIST") {
        let mut chosen = if proto < V30 {
            StrongHash::Md4
        } else {
            StrongHash::Md5
        };
        for name in list.split(',') {
            match name {
                "sha1" => {
                    chosen = StrongHash::Sha1;
                    break;
                }
                "md5" => {
                    chosen = StrongHash::Md5;
                    break;
                }
                "md4" => {
                    chosen = StrongHash::Md4;
                    break;
                }
                _ => {}
            }
        }
        chosen
    } else {
        StrongHash::Md5
    };

    let src_trailing = match &src {
        RemoteSpec::Local(p) => p.trailing_slash,
        RemoteSpec::Remote { path, .. } => path.trailing_slash,
    };
    let src_path = match &src {
        RemoteSpec::Local(p) => &p.path,
        RemoteSpec::Remote { path, .. } => &path.path,
    };
    if opts.relative {
        let rel = if src_path.is_absolute() {
            src_path.strip_prefix(Path::new("/")).unwrap_or(src_path)
        } else {
            src_path
        };
        match &mut dst {
            RemoteSpec::Local(p) => p.path.push(rel),
            RemoteSpec::Remote { path, .. } => path.path.push(rel),
        }
    } else if !src_trailing {
        let name = src_path
            .file_name()
            .map(|s| s.to_owned())
            .ok_or_else(|| EngineError::Other("source path missing file name".into()))?;
        match &mut dst {
            RemoteSpec::Local(p) => p.path.push(&name),
            RemoteSpec::Remote { path, .. } => path.path.push(&name),
        }
    }

    let compress_choice = match opts.compress_choice.as_deref() {
        Some("none") => None,
        Some(s) => {
            let mut list = Vec::new();
            for name in s.split(',') {
                let codec = match name {
                    "zlib" => Codec::Zlib,
                    "zlibx" => Codec::Zlibx,
                    "zstd" => Codec::Zstd,
                    "lz4" => Codec::Lz4,
                    other => {
                        return Err(EngineError::Other(format!("unknown codec {other}")));
                    }
                };
                if !available_codecs().contains(&codec) {
                    return Err(EngineError::Other(format!(
                        "codec {name} not supported by this build"
                    )));
                }
                list.push(codec);
            }
            if list.is_empty() {
                None
            } else {
                Some(list)
            }
        }
        None => None,
    };
    let compress = if opts.compress_choice.as_deref() == Some("none") {
        false
    } else {
        opts.compress || opts.compress_level.is_some_and(|l| l > 0) || compress_choice.is_some()
    };
    let mut delete_mode = if opts.delete_before {
        Some(DeleteMode::Before)
    } else if opts.delete_after || opts.delete_delay {
        Some(DeleteMode::After)
    } else if opts.delete_during || opts.delete {
        Some(DeleteMode::During)
    } else {
        None
    };
    if delete_mode.is_none() && opts.delete_excluded {
        delete_mode = Some(DeleteMode::During);
    }
    let block_size = opts.block_size.unwrap_or(0);
    let mut chmod_rules = Vec::new();
    for spec in &opts.chmod {
        chmod_rules.extend(parse_chmod(spec).map_err(EngineError::Other)?);
    }
    let chown_ids = if let Some(ref spec) = opts.chown {
        Some(parse_chown(spec).map_err(EngineError::Other)?)
    } else {
        None
    };
    let copy_as = if let Some(ref spec) = opts.copy_as {
        let (uid_opt, gid_opt) = parse_chown(spec).map_err(EngineError::Other)?;
        let uid = uid_opt.ok_or_else(|| EngineError::Other("--copy-as requires a user".into()))?;
        let gid = if let Some(g) = gid_opt {
            Some(g)
        } else {
            #[cfg(unix)]
            {
                get_user_by_uid(uid).map(|u| u.primary_group_id())
            }
            #[cfg(not(unix))]
            {
                None
            }
        };
        Some((uid, gid))
    } else {
        None
    };
    let uid_map = parse_name_map(&opts.usermap, IdKind::User)?;
    let gid_map = parse_name_map(&opts.groupmap, IdKind::Group)?;
    let mut sync_opts = SyncOptions {
        delete: delete_mode,
        delete_excluded: opts.delete_excluded,
        ignore_missing_args: opts.ignore_missing_args,
        delete_missing_args: opts.delete_missing_args,
        remove_source_files: opts.remove_source_files,
        ignore_errors: opts.ignore_errors,
        force: opts.force,
        max_delete: opts.max_delete,
        max_alloc: opts.max_alloc.unwrap_or(1usize << 30),
        max_size: opts.max_size,
        min_size: opts.min_size,
        preallocate: opts.preallocate,
        checksum: opts.checksum,
        compress,
        dirs: opts.dirs,
        no_implied_dirs: opts.no_implied_dirs,
        dry_run: opts.dry_run,
        list_only: opts.list_only,
        update: opts.update,
        existing: opts.existing,
        ignore_existing: opts.ignore_existing,
        one_file_system: opts.one_file_system,
        size_only: opts.size_only,
        ignore_times: opts.ignore_times,
        perms: if opts.no_perms {
            false
        } else {
            opts.perms || opts.archive || {
                #[cfg(feature = "acl")]
                {
                    acls
                }
                #[cfg(not(feature = "acl"))]
                {
                    false
                }
            }
        },
        executability: opts.executability,
        times: if opts.no_times {
            false
        } else {
            opts.times || opts.archive
        },
        atimes: opts.atimes,
        crtimes: opts.crtimes,
        omit_dir_times: opts.omit_dir_times,
        omit_link_times: opts.omit_link_times,
        owner: if opts.no_owner {
            false
        } else {
            opts.owner
                || opts.archive
                || chown_ids.is_some_and(|(u, _)| u.is_some())
                || uid_map.is_some()
        },
        group: if opts.no_group {
            false
        } else {
            opts.group
                || opts.archive
                || chown_ids.is_some_and(|(_, g)| g.is_some())
                || gid_map.is_some()
        },
        links: if opts.no_links {
            false
        } else {
            opts.links || opts.archive
        },
        copy_links: opts.copy_links,
        copy_dirlinks: opts.copy_dirlinks,
        keep_dirlinks: opts.keep_dirlinks,
        copy_unsafe_links: opts.copy_unsafe_links,
        safe_links: opts.safe_links,
        munge_links: opts.munge_links,
        hard_links: opts.hard_links,
        devices: if opts.no_devices {
            false
        } else {
            opts.devices || opts.archive || opts.devices_specials
        },
        specials: if opts.no_specials {
            false
        } else {
            opts.specials || opts.archive || opts.devices_specials
        },
        #[cfg(feature = "xattr")]
        xattrs: opts.xattrs || (opts.fake_super && !opts.super_user),
        #[cfg(feature = "acl")]
        acls,
        sparse: opts.sparse,
        strong,
        checksum_seed: opts.checksum_seed.unwrap_or(0),
        compress_level: opts.compress_level,
        compress_choice,
        whole_file: if opts.no_whole_file {
            false
        } else {
            opts.whole_file
        },
        skip_compress: opts.skip_compress.clone(),
        partial: opts.partial || opts.partial_progress || opts.partial_dir.is_some(),
        progress: opts.progress || opts.partial_progress,
        human_readable: opts.human_readable,
        itemize_changes: opts.itemize_changes,
        out_format: opts.out_format.as_ref().map(|s| parse_escapes(s)),
        partial_dir: opts.partial_dir.clone(),
        temp_dir: opts.temp_dir.clone(),
        append: opts.append,
        append_verify: opts.append_verify,
        numeric_ids: opts.numeric_ids,
        inplace: opts.inplace || opts.write_devices,
        delay_updates: opts.delay_updates,
        modify_window: opts.modify_window.unwrap_or(Duration::ZERO),
        bwlimit: opts.bwlimit,
        block_size,
        link_dest: opts.link_dest.clone(),
        copy_dest: opts.copy_dest.clone(),
        compare_dest: opts.compare_dest.clone(),
        backup: opts.backup || opts.backup_dir.is_some(),
        backup_dir: opts.backup_dir.clone(),
        backup_suffix: opts.suffix.clone(),
        chmod: if chmod_rules.is_empty() {
            None
        } else {
            Some(chmod_rules)
        },
        chown: chown_ids,
        copy_as,
        uid_map,
        gid_map,
        eight_bit_output: opts.eight_bit_output,
        blocking_io: opts.blocking_io,
        early_input: opts.early_input.clone(),
        secluded_args: opts.secluded_args,
        sockopts: opts.sockopts.clone(),
        remote_options: remote_opts.clone(),
        write_batch: opts.write_batch.clone(),
        read_batch: opts.read_batch.clone(),
        copy_devices: opts.copy_devices,
        write_devices: opts.write_devices,
        fsync: opts.fsync,
        fuzzy: opts.fuzzy,
        super_user: opts.super_user,
        fake_super: opts.fake_super && !opts.super_user,
        quiet: opts.quiet,
    };
    sync_opts.prepare_remote();
    let stats = if opts.local {
        match (src, dst) {
            (RemoteSpec::Local(src), RemoteSpec::Local(dst)) => sync(
                &src.path,
                &dst.path,
                &matcher,
                &available_codecs(),
                &sync_opts,
            )?,
            _ => return Err(EngineError::Other("local sync requires local paths".into())),
        }
    } else {
        match (src, dst) {
            (RemoteSpec::Local(_), RemoteSpec::Local(_)) => {
                return Err(EngineError::Other(
                    "local sync requires --local flag".into(),
                ))
            }
            (
                RemoteSpec::Remote {
                    host,
                    path: src,
                    module: Some(module),
                },
                RemoteSpec::Local(dst),
            ) => {
                let mut _session = spawn_daemon_session(
                    &host,
                    &module,
                    opts.port,
                    opts.password_file.as_deref(),
                    opts.no_motd,
                    opts.timeout,
                    opts.connect_timeout,
                    addr_family,
                    &opts.sockopts,
                    &sync_opts,
                    opts.protocol.unwrap_or(31),
                    opts.early_input.as_deref(),
                    iconv.as_ref(),
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    &matcher,
                    &available_codecs(),
                    &sync_opts,
                )?
            }
            (
                RemoteSpec::Remote {
                    host,
                    path: src,
                    module: None,
                },
                RemoteSpec::Local(dst),
            ) => {
                let connect_timeout = opts.connect_timeout;
                let caps_send = CAP_CODECS
                    | {
                        #[cfg(feature = "acl")]
                        {
                            if sync_opts.acls {
                                CAP_ACLS
                            } else {
                                0
                            }
                        }
                        #[cfg(not(feature = "acl"))]
                        {
                            0
                        }
                    }
                    | {
                        #[cfg(feature = "xattr")]
                        {
                            if sync_opts.xattrs {
                                CAP_XATTRS
                            } else {
                                0
                            }
                        }
                        #[cfg(not(feature = "xattr"))]
                        {
                            0
                        }
                    };
                let (session, codecs, caps) = SshStdioTransport::connect_with_rsh(
                    &host,
                    &src.path,
                    &rsh_cmd.cmd,
                    &rsh_cmd.env,
                    &rsync_env,
                    remote_bin_vec.as_deref(),
                    remote_env_vec.as_deref().unwrap_or(&[]),
                    &sync_opts.remote_options,
                    known_hosts.as_deref(),
                    strict_host_key_checking,
                    opts.port,
                    connect_timeout,
                    addr_family,
                    opts.protocol.unwrap_or(31),
                    caps_send,
                    None,
                )
                .map_err(EngineError::from)?;
                #[cfg(not(any(feature = "xattr", feature = "acl")))]
                let _ = caps;
                #[cfg(feature = "xattr")]
                if sync_opts.xattrs && caps & CAP_XATTRS == 0 {
                    sync_opts.xattrs = false;
                }
                #[cfg(feature = "acl")]
                if sync_opts.acls && caps & CAP_ACLS == 0 {
                    sync_opts.acls = false;
                }
                let (err, _) = session.stderr();
                if !err.is_empty() {
                    let msg = if let Some(cv) = iconv.as_ref() {
                        cv.decode_remote(&err)
                    } else {
                        String::from_utf8_lossy(&err).into_owned()
                    };
                    return Err(EngineError::Other(msg));
                }
                sync(&src.path, &dst.path, &matcher, &codecs, &sync_opts)?
            }
            (
                RemoteSpec::Local(src),
                RemoteSpec::Remote {
                    host,
                    path: dst,
                    module: Some(module),
                },
            ) => {
                let mut _session = spawn_daemon_session(
                    &host,
                    &module,
                    opts.port,
                    opts.password_file.as_deref(),
                    opts.no_motd,
                    opts.timeout,
                    opts.connect_timeout,
                    addr_family,
                    &opts.sockopts,
                    &sync_opts,
                    opts.protocol.unwrap_or(31),
                    opts.early_input.as_deref(),
                    iconv.as_ref(),
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    &matcher,
                    &available_codecs(),
                    &sync_opts,
                )?
            }
            (
                RemoteSpec::Local(src),
                RemoteSpec::Remote {
                    host,
                    path: dst,
                    module: None,
                },
            ) => {
                let connect_timeout = opts.connect_timeout;
                let caps_send = CAP_CODECS
                    | {
                        #[cfg(feature = "acl")]
                        {
                            if sync_opts.acls {
                                CAP_ACLS
                            } else {
                                0
                            }
                        }
                        #[cfg(not(feature = "acl"))]
                        {
                            0
                        }
                    }
                    | {
                        #[cfg(feature = "xattr")]
                        {
                            if sync_opts.xattrs {
                                CAP_XATTRS
                            } else {
                                0
                            }
                        }
                        #[cfg(not(feature = "xattr"))]
                        {
                            0
                        }
                    };
                let (session, codecs, caps) = SshStdioTransport::connect_with_rsh(
                    &host,
                    &dst.path,
                    &rsh_cmd.cmd,
                    &rsh_cmd.env,
                    &rsync_env,
                    remote_bin_vec.as_deref(),
                    remote_env_vec.as_deref().unwrap_or(&[]),
                    &sync_opts.remote_options,
                    known_hosts.as_deref(),
                    strict_host_key_checking,
                    opts.port,
                    connect_timeout,
                    addr_family,
                    opts.protocol.unwrap_or(31),
                    caps_send,
                    None,
                )
                .map_err(EngineError::from)?;
                #[cfg(not(any(feature = "xattr", feature = "acl")))]
                let _ = caps;
                #[cfg(feature = "xattr")]
                if sync_opts.xattrs && caps & CAP_XATTRS == 0 {
                    sync_opts.xattrs = false;
                }
                #[cfg(feature = "acl")]
                if sync_opts.acls && caps & CAP_ACLS == 0 {
                    sync_opts.acls = false;
                }
                let (err, _) = session.stderr();
                if !err.is_empty() {
                    let msg = if let Some(cv) = iconv.as_ref() {
                        cv.decode_remote(&err)
                    } else {
                        String::from_utf8_lossy(&err).into_owned()
                    };
                    return Err(EngineError::Other(msg));
                }
                sync(&src.path, &dst.path, &matcher, &codecs, &sync_opts)?
            }
            (
                RemoteSpec::Remote {
                    host: src_host,
                    path: src_path,
                    module: src_mod,
                },
                RemoteSpec::Remote {
                    host: dst_host,
                    path: dst_path,
                    module: dst_mod,
                },
            ) => {
                if src_host.is_empty() || dst_host.is_empty() {
                    return Err(EngineError::Other("remote host missing".to_string()));
                }
                if (src_mod.is_none() && src_path.path.as_os_str().is_empty())
                    || (dst_mod.is_none() && dst_path.path.as_os_str().is_empty())
                {
                    return Err(EngineError::Other("remote path missing".to_string()));
                }

                match (src_mod, dst_mod) {
                    (None, None) => {
                        let connect_timeout = opts.connect_timeout;
                        let mut dst_session = SshStdioTransport::spawn_with_rsh(
                            &dst_host,
                            &dst_path.path,
                            &rsh_cmd.cmd,
                            &rsh_cmd.env,
                            remote_bin_vec.as_deref(),
                            remote_env_vec.as_deref().unwrap_or(&[]),
                            &sync_opts.remote_options,
                            known_hosts.as_deref(),
                            strict_host_key_checking,
                            opts.port,
                            connect_timeout,
                            addr_family,
                        )
                        .map_err(EngineError::from)?;
                        let mut src_session = SshStdioTransport::spawn_with_rsh(
                            &src_host,
                            &src_path.path,
                            &rsh_cmd.cmd,
                            &rsh_cmd.env,
                            remote_bin_vec.as_deref(),
                            remote_env_vec.as_deref().unwrap_or(&[]),
                            &sync_opts.remote_options,
                            known_hosts.as_deref(),
                            strict_host_key_checking,
                            opts.port,
                            connect_timeout,
                            addr_family,
                        )
                        .map_err(EngineError::from)?;

                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                            let (src_err, _) = src_session.stderr();
                            if !src_err.is_empty() {
                                let msg = if let Some(cv) = iconv.as_ref() {
                                    cv.decode_remote(&src_err)
                                } else {
                                    String::from_utf8_lossy(&src_err).into_owned()
                                };
                                return Err(EngineError::Other(msg));
                            }
                            let dst_session = dst_session.into_inner();
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                let msg = if let Some(cv) = iconv.as_ref() {
                                    cv.decode_remote(&dst_err)
                                } else {
                                    String::from_utf8_lossy(&dst_err).into_owned()
                                };
                                return Err(EngineError::Other(msg));
                            }
                        } else {
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                            let (src_err, _) = src_session.stderr();
                            if !src_err.is_empty() {
                                let msg = if let Some(cv) = iconv.as_ref() {
                                    cv.decode_remote(&src_err)
                                } else {
                                    String::from_utf8_lossy(&src_err).into_owned()
                                };
                                return Err(EngineError::Other(msg));
                            }
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                let msg = if let Some(cv) = iconv.as_ref() {
                                    cv.decode_remote(&dst_err)
                                } else {
                                    String::from_utf8_lossy(&dst_err).into_owned()
                                };
                                return Err(EngineError::Other(msg));
                            }
                        }
                        Stats::default()
                    }
                    (Some(sm), Some(dm)) => {
                        let mut dst_session = spawn_daemon_session(
                            &dst_host,
                            &dm,
                            opts.port,
                            opts.password_file.as_deref(),
                            opts.no_motd,
                            opts.timeout,
                            opts.connect_timeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol.unwrap_or(31),
                            opts.early_input.as_deref(),
                            iconv.as_ref(),
                        )?;
                        let mut src_session = spawn_daemon_session(
                            &src_host,
                            &sm,
                            opts.port,
                            opts.password_file.as_deref(),
                            opts.no_motd,
                            opts.timeout,
                            opts.connect_timeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol.unwrap_or(31),
                            opts.early_input.as_deref(),
                            iconv.as_ref(),
                        )?;
                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                        } else {
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                        }
                        Stats::default()
                    }
                    (Some(sm), None) => {
                        let mut dst_session = SshStdioTransport::spawn_with_rsh(
                            &dst_host,
                            &dst_path.path,
                            &rsh_cmd.cmd,
                            &rsh_cmd.env,
                            remote_bin_vec.as_deref(),
                            remote_env_vec.as_deref().unwrap_or(&[]),
                            &sync_opts.remote_options,
                            known_hosts.as_deref(),
                            strict_host_key_checking,
                            opts.port,
                            opts.connect_timeout,
                            addr_family,
                        )
                        .map_err(EngineError::from)?;
                        let mut src_session = spawn_daemon_session(
                            &src_host,
                            &sm,
                            opts.port,
                            opts.password_file.as_deref(),
                            opts.no_motd,
                            opts.timeout,
                            opts.connect_timeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol.unwrap_or(31),
                            opts.early_input.as_deref(),
                            iconv.as_ref(),
                        )?;
                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                            let dst_session = dst_session.into_inner();
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                let msg = if let Some(cv) = iconv.as_ref() {
                                    cv.decode_remote(&dst_err)
                                } else {
                                    String::from_utf8_lossy(&dst_err).into_owned()
                                };
                                return Err(EngineError::Other(msg));
                            }
                        } else {
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                let msg = if let Some(cv) = iconv.as_ref() {
                                    cv.decode_remote(&dst_err)
                                } else {
                                    String::from_utf8_lossy(&dst_err).into_owned()
                                };
                                return Err(EngineError::Other(msg));
                            }
                        }
                        Stats::default()
                    }
                    (None, Some(dm)) => {
                        let mut dst_session = spawn_daemon_session(
                            &dst_host,
                            &dm,
                            opts.port,
                            opts.password_file.as_deref(),
                            opts.no_motd,
                            opts.timeout,
                            opts.connect_timeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol.unwrap_or(31),
                            opts.early_input.as_deref(),
                            iconv.as_ref(),
                        )?;
                        let mut src_session = SshStdioTransport::spawn_with_rsh(
                            &src_host,
                            &src_path.path,
                            &rsh_cmd.cmd,
                            &rsh_cmd.env,
                            remote_bin_vec.as_deref(),
                            remote_env_vec.as_deref().unwrap_or(&[]),
                            &sync_opts.remote_options,
                            known_hosts.as_deref(),
                            strict_host_key_checking,
                            opts.port,
                            opts.connect_timeout,
                            addr_family,
                        )
                        .map_err(EngineError::from)?;
                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                        } else {
                            pipe_sessions(&mut src_session, &mut dst_session)?;
                        }
                        let (src_err, _) = src_session.stderr();
                        if !src_err.is_empty() {
                            let msg = if let Some(cv) = iconv.as_ref() {
                                cv.decode_remote(&src_err)
                            } else {
                                String::from_utf8_lossy(&src_err).into_owned()
                            };
                            return Err(EngineError::Other(msg));
                        }
                        Stats::default()
                    }
                }
            }
        }
    };
    if opts.stats && !opts.quiet {
        println!(
            "Number of regular files transferred: {}",
            stats.files_transferred
        );
        println!("Number of deleted files: {}", stats.files_deleted);
        let bytes = progress_formatter(stats.bytes_transferred, opts.human_readable);
        println!("Total transferred file size: {} bytes", bytes);
        tracing::info!(
            target: InfoFlag::Stats.target(),
            files_transferred = stats.files_transferred,
            files_deleted = stats.files_deleted,
            bytes = stats.bytes_transferred
        );
    }
    Ok(())
}

fn build_matcher(opts: &ClientOpts, matches: &ArgMatches) -> Result<Matcher> {
    fn load_patterns(path: &Path, from0: bool) -> io::Result<Vec<String>> {
        filters::parse_list_file(path, from0).map_err(|e| io::Error::other(format!("{:?}", e)))
    }

    let mut entries: Vec<(usize, usize, Rule)> = Vec::new();
    let mut seq = 0;
    let mut add_rules = |idx: usize, rs: Vec<Rule>| {
        for r in rs {
            entries.push((idx, seq, r));
            seq += 1;
        }
    };

    if let Some(values) = matches.get_many::<String>("filter") {
        let idxs: Vec<_> = matches
            .indices_of("filter")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, val) in idxs.into_iter().zip(values) {
            add_rules(
                idx + 1,
                parse_filters(val, opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("filter_file") {
        let idxs: Vec<_> = matches
            .indices_of("filter_file")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let rs = filters::parse_file(file, opts.from0, &mut HashSet::new(), 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<String>("include") {
        let idxs: Vec<_> = matches
            .indices_of("include")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, pat) in idxs.into_iter().zip(values) {
            add_rules(
                idx + 1,
                parse_filters(&format!("+ {}", pat), opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<String>("exclude") {
        let idxs: Vec<_> = matches
            .indices_of("exclude")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, pat) in idxs.into_iter().zip(values) {
            add_rules(
                idx + 1,
                parse_filters(&format!("- {}", pat), opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("include_from") {
        let idxs: Vec<_> = matches
            .indices_of("include_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filters::parse_rule_list_file(file, opts.from0, '+', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("exclude_from") {
        let idxs: Vec<_> = matches
            .indices_of("exclude_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filters::parse_rule_list_file(file, opts.from0, '-', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("files_from") {
        for file in values {
            for pat in load_patterns(file, opts.from0)? {
                let anchored = if pat.starts_with('/') {
                    pat.clone()
                } else {
                    format!("/{}", pat)
                };

                let rule1 = if opts.from0 {
                    format!("+{}", anchored)
                } else {
                    format!("+ {}", anchored)
                };
                add_rules(
                    usize::MAX - 1,
                    parse_filters(&rule1, opts.from0)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
                );

                let dir_pat = format!("{}/***", anchored.trim_end_matches('/'));
                let rule2 = if opts.from0 {
                    format!("+{}", dir_pat)
                } else {
                    format!("+ {}", dir_pat)
                };
                add_rules(
                    usize::MAX - 1,
                    parse_filters(&rule2, opts.from0)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
                );
            }
        }
    }
    if matches.contains_id("filter_shorthand") {
        if let Some(idx) = matches.index_of("filter_shorthand") {
            let count = matches.get_count("filter_shorthand");
            let rule_str = if count >= 2 { "-FF" } else { "-F" };
            add_rules(
                idx + 1,
                parse_filters(rule_str, opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if !opts.files_from.is_empty() {
        add_rules(
            usize::MAX,
            parse_filters("- *", opts.from0).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    if opts.cvs_exclude {
        let mut cvs = default_cvs_rules().map_err(|e| EngineError::Other(format!("{:?}", e)))?;
        cvs.extend(
            parse_filters(":C\n", opts.from0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
        add_rules(usize::MAX, cvs);
    }

    entries.sort_by(|a, b| {
        if a.0 == b.0 {
            a.1.cmp(&b.1)
        } else {
            a.0.cmp(&b.0)
        }
    });
    let rules: Vec<Rule> = entries.into_iter().map(|(_, _, r)| r).collect();
    let mut matcher = Matcher::new(rules);
    if opts.from0 {
        matcher = matcher.with_from0();
    }
    if opts.existing {
        matcher = matcher.with_existing();
    }
    if opts.prune_empty_dirs {
        matcher = matcher.with_prune_empty_dirs();
    }
    Ok(matcher)
}

fn run_daemon(opts: DaemonOpts, matches: &ArgMatches) -> Result<()> {
    let mut modules: HashMap<String, Module> = HashMap::new();
    let mut secrets = opts.secrets_file.clone();
    let password = matches
        .get_one::<PathBuf>("password_file")
        .cloned()
        .map(|pf| -> Result<String> {
            #[cfg(unix)]
            {
                let mode = fs::metadata(&pf)?.permissions().mode();
                if mode & 0o077 != 0 {
                    return Err(EngineError::Other(
                        "password file permissions are too open".into(),
                    ));
                }
            }
            let data = fs::read_to_string(&pf)?;
            Ok(data.lines().next().unwrap_or_default().trim().to_string())
        })
        .transpose()?;
    let mut hosts_allow = opts.hosts_allow.clone();
    let mut hosts_deny = opts.hosts_deny.clone();
    let mut log_file = matches.get_one::<PathBuf>("client-log-file").cloned();
    let log_format = matches
        .get_one::<String>("client-log-file-format")
        .map(|s| parse_escapes(s));
    let mut motd = opts.motd.clone();
    let mut pid_file = opts.pid_file.clone();
    let mut lock_file = opts.lock_file.clone();
    let mut state_dir = opts.state_dir.clone();
    let mut port = matches.get_one::<u16>("port").copied().unwrap_or(873);
    let mut address = opts.address;
    let timeout = matches.get_one::<Duration>("timeout").copied();
    let bwlimit = matches.get_one::<u64>("bwlimit").copied();
    let numeric_ids_flag = matches.get_flag("numeric_ids");
    let mut list = true;
    let mut refuse = Vec::new();
    let mut max_conn = None;
    let mut read_only = None;
    if let Some(cfg_path) = matches.get_one::<PathBuf>("config") {
        let cfg = parse_config_file(cfg_path).map_err(|e| EngineError::Other(e.to_string()))?;
        for m in cfg.modules {
            modules.insert(m.name.clone(), m);
        }
        if let Some(p) = cfg.port {
            port = p;
        }
        if let Some(m) = cfg.motd_file {
            motd = Some(m);
        }
        if let Some(l) = cfg.log_file {
            log_file = Some(l);
        }
        if let Some(s) = cfg.secrets_file {
            secrets = Some(s);
        }
        if let Some(p) = cfg.pid_file {
            pid_file = Some(p);
        }
        if let Some(l) = cfg.lock_file {
            lock_file = Some(l);
        }
        if let Some(a) = cfg.address {
            address = Some(a);
        }
        if !cfg.hosts_allow.is_empty() {
            hosts_allow = cfg.hosts_allow;
        }
        if !cfg.hosts_deny.is_empty() {
            hosts_deny = cfg.hosts_deny;
        }
        if let Some(val) = cfg.numeric_ids {
            for m in modules.values_mut() {
                m.numeric_ids = val;
            }
        }
        if let Some(val) = cfg.read_only {
            read_only = Some(val);
        }
        if let Some(val) = cfg.list {
            list = val;
        }
        if let Some(val) = cfg.max_connections {
            max_conn = Some(val);
        }
        if !cfg.refuse_options.is_empty() {
            refuse = cfg.refuse_options;
        }
    }

    for m in opts.module {
        modules.insert(m.name.clone(), m);
    }

    for (name, value) in opts.dparam {
        match name.as_str() {
            "motdfile" => motd = Some(PathBuf::from(value)),
            "pidfile" => pid_file = Some(PathBuf::from(value)),
            "logfile" => log_file = Some(PathBuf::from(value)),
            "lockfile" => lock_file = Some(PathBuf::from(value)),
            "statedir" => state_dir = Some(PathBuf::from(value)),
            "secretsfile" => secrets = Some(PathBuf::from(value)),
            "address" => {
                address = Some(
                    value
                        .parse::<IpAddr>()
                        .map_err(|e| EngineError::Other(e.to_string()))?,
                )
            }
            "port" => {
                port = value
                    .parse::<u16>()
                    .map_err(|e| EngineError::Other(e.to_string()))?
            }
            "numericids" => {
                let val = parse_bool(&value).map_err(EngineError::Other)?;
                for m in modules.values_mut() {
                    m.numeric_ids = val;
                }
            }
            "read only" | "read_only" => {
                let val = parse_bool(&value).map_err(EngineError::Other)?;
                for m in modules.values_mut() {
                    m.read_only = val;
                }
            }
            "list" => {
                list = parse_bool(&value).map_err(EngineError::Other)?;
            }
            "max connections" | "maxconnections" => {
                max_conn = Some(
                    value
                        .parse::<usize>()
                        .map_err(|e| EngineError::Other(e.to_string()))?,
                );
            }
            "hosts allow" | "hostsallow" => {
                hosts_allow = value.split_whitespace().map(|s| s.to_string()).collect();
            }
            "hosts deny" | "hostsdeny" => {
                hosts_deny = value.split_whitespace().map(|s| s.to_string()).collect();
            }
            "refuse options" | "refuseoptions" => {
                refuse = value.split_whitespace().map(|s| s.to_string()).collect();
            }
            _ => {
                return Err(EngineError::Other(format!(
                    "unknown daemon parameter: {name}"
                )));
            }
        }
    }

    if numeric_ids_flag {
        for m in modules.values_mut() {
            m.numeric_ids = true;
        }
    }
    if let Some(val) = read_only {
        for m in modules.values_mut() {
            m.read_only = val;
        }
    }
    if !refuse.is_empty() {
        for m in modules.values_mut() {
            m.refuse_options = refuse.clone();
        }
    }

    let addr_family = if matches.get_flag("ipv4") {
        Some(AddressFamily::V4)
    } else if matches.get_flag("ipv6") {
        Some(AddressFamily::V6)
    } else {
        None
    };

    let handler: Arc<daemon::Handler> = Arc::new(|_| Ok(()));
    let quiet = matches.get_flag("quiet");

    daemon::run_daemon(
        modules,
        secrets,
        password,
        hosts_allow,
        hosts_deny,
        log_file,
        log_format,
        motd,
        pid_file,
        lock_file,
        state_dir,
        timeout,
        bwlimit,
        max_conn,
        refuse,
        list,
        port,
        address,
        addr_family,
        65534,
        65534,
        handler,
        quiet,
    )
    .map_err(|e| EngineError::Other(format!("daemon failed to bind to port {port}: {e}")))
}

fn run_probe(opts: ProbeOpts, quiet: bool) -> Result<()> {
    if let Some(addr) = opts.addr {
        let mut stream = TcpStream::connect(&addr)?;
        stream.write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())?;
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(SUPPORTED_PROTOCOLS[0], peer)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    } else {
        let ver = negotiate_version(SUPPORTED_PROTOCOLS[0], opts.peer_version)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use daemon::authenticate;
    use engine::SyncOptions;

    #[test]
    fn windows_paths_are_local() {
        let spec = parse_remote_spec("C:/tmp/foo").unwrap();
        assert!(matches!(spec, RemoteSpec::Local(_)));
    }

    #[test]
    fn parse_bool_is_case_insensitive() {
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("Yes").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(!parse_bool("No").unwrap());
    }

    #[test]
    fn ipv6_specs_are_remote() {
        let spec = parse_remote_spec("[::1]:/tmp").unwrap();
        match spec {
            RemoteSpec::Remote { host, path, module } => {
                assert_eq!(host, "::1");
                assert!(module.is_none());
                assert_eq!(path.path, PathBuf::from("/tmp"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn rsync_url_specs_are_remote() {
        let spec = parse_remote_spec("rsync://host/mod/path").unwrap();
        match spec {
            RemoteSpec::Remote { host, module, path } => {
                assert_eq!(host, "host");
                assert_eq!(module.as_deref(), Some("mod"));
                assert_eq!(path.path, PathBuf::from("path"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn daemon_double_colon_specs_are_remote() {
        let spec = parse_remote_spec("host::mod/path").unwrap();
        match spec {
            RemoteSpec::Remote { host, module, path } => {
                assert_eq!(host, "host");
                assert_eq!(module.as_deref(), Some("mod"));
                assert_eq!(path.path, PathBuf::from("path"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn host_path_specs_are_remote() {
        let spec = parse_remote_spec("host:/tmp").unwrap();
        match spec {
            RemoteSpec::Remote { host, module, path } => {
                assert_eq!(host, "host");
                assert!(module.is_none());
                assert_eq!(path.path, PathBuf::from("/tmp"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn parses_client_flags() {
        let opts = ClientOpts::parse_from([
            "prog",
            "-r",
            "-n",
            "-v",
            "--delete",
            "-c",
            "-z",
            "--stats",
            "--executability",
            "--config",
            "file",
            "src",
            "dst",
        ]);
        assert!(opts.recursive);
        assert!(opts.dry_run);
        assert_eq!(opts.verbose, 1);
        assert!(opts.delete);
        assert!(opts.checksum);
        assert!(opts.compress);
        assert!(opts.stats);
        assert!(opts.executability);
        assert_eq!(opts.config, Some(PathBuf::from("file")));
    }

    #[test]
    fn parses_checksum_choice_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--checksum-choice", "sha1", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("sha1"));
        let opts = ClientOpts::parse_from(["prog", "--cc", "md5", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("md5"));
    }

    #[test]
    fn parses_rsh_flag_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--rsh", "ssh", "src", "dst"]);
        assert_eq!(opts.rsh.as_deref(), Some("ssh"));
        let opts = ClientOpts::parse_from(["prog", "-e", "ssh", "src", "dst"]);
        assert_eq!(opts.rsh.as_deref(), Some("ssh"));
    }

    #[test]
    fn parses_rsync_path_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--rsync-path", "/bin/rsync", "src", "dst"]);
        assert_eq!(opts.rsync_path.as_deref(), Some("/bin/rsync"));
        let opts = ClientOpts::parse_from(["prog", "--rsync_path", "/bin/rsync", "src", "dst"]);
        assert_eq!(opts.rsync_path.as_deref(), Some("/bin/rsync"));
    }

    #[test]
    fn parses_skip_compress_list() {
        let opts = ClientOpts::parse_from(["prog", "--skip-compress=gz,zip", "src", "dst"]);
        assert_eq!(opts.skip_compress, vec!["gz", "zip"]);
    }

    #[test]
    fn parses_skip_flags() {
        let opts = ClientOpts::parse_from([
            "prog",
            "--ignore-existing",
            "--existing",
            "--prune-empty-dirs",
            "--size-only",
            "--ignore-times",
            "src",
            "dst",
        ]);
        assert!(opts.ignore_existing);
        assert!(opts.existing);
        assert!(opts.prune_empty_dirs);
        assert!(opts.size_only);
        assert!(opts.ignore_times);
    }

    #[test]
    fn parses_protocol_version() {
        let opts = ClientOpts::parse_from(["prog", "--protocol", "30", "src", "dst"]);
        assert_eq!(opts.protocol, Some(30));
    }

    #[test]
    fn parses_8_bit_output() {
        let opts = ClientOpts::parse_from(["prog", "-8", "src", "dst"]);
        assert!(opts.eight_bit_output);
    }

    #[test]
    fn parses_blocking_io() {
        let opts = ClientOpts::parse_from(["prog", "--blocking-io", "src", "dst"]);
        assert!(opts.blocking_io);
    }

    #[test]
    fn parses_early_input() {
        let opts = ClientOpts::parse_from(["prog", "--early-input", "file", "src", "dst"]);
        assert_eq!(opts.early_input.as_deref(), Some(Path::new("file")));
    }

    #[test]
    fn protocol_override_sent_to_server() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(u32::from_be_bytes(buf), 30);
            stream
                .write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())
                .unwrap();

            let mut b = [0u8; 1];
            while stream.read(&mut b).unwrap() > 0 {
                if b[0] == b'\n' {
                    break;
                }
            }

            stream.write_all(b"@RSYNCD: OK\n").unwrap();

            let mut m = Vec::new();
            loop {
                stream.read_exact(&mut b).unwrap();
                if b[0] == b'\n' {
                    break;
                }
                m.push(b[0]);
            }
            assert_eq!(m, b"mod".to_vec());
        });

        let _t = spawn_daemon_session(
            "127.0.0.1",
            "mod",
            Some(port),
            None,
            true,
            None,
            None,
            None,
            &[],
            &SyncOptions::default(),
            30,
            None,
            None,
        )
        .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn sends_early_input_to_daemon() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let path = dir.path().join("input.txt");
        fs::write(&path, b"hello\n").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 6];
            stream.read_exact(&mut buf).unwrap();
            assert_eq!(&buf, b"hello\n");
            let mut ver = [0u8; 4];
            stream.read_exact(&mut ver).unwrap();
            assert_eq!(u32::from_be_bytes(ver), 30);
            stream
                .write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())
                .unwrap();
            let mut b = [0u8; 1];
            while stream.read(&mut b).unwrap() > 0 {
                if b[0] == b'\n' {
                    break;
                }
            }
            stream.write_all(b"@RSYNCD: OK\n").unwrap();
            let mut m = Vec::new();
            loop {
                stream.read_exact(&mut b).unwrap();
                if b[0] == b'\n' {
                    break;
                }
                m.push(b[0]);
            }
            assert_eq!(m, b"mod".to_vec());
        });

        let _t = spawn_daemon_session(
            "127.0.0.1",
            "mod",
            Some(port),
            None,
            true,
            None,
            None,
            None,
            &[],
            &SyncOptions::default(),
            30,
            Some(&path),
            None,
        )
        .unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn parses_internal_server_sender_flags() {
        let opts = ClientOpts::parse_from(["prog", "--server", "--sender", "src", "dst"]);
        assert!(opts.server);
        assert!(opts.sender);
    }

    #[test]
    fn rejects_invalid_env_assignment() {
        let err = parse_rsh(Some("1BAD=val ssh".into())).unwrap_err();
        assert!(matches!(err, EngineError::Other(_)));
    }

    #[test]
    #[cfg(unix)]
    fn rejects_insecure_auth_file() {
        use std::net::{TcpListener, TcpStream};
        use std::os::unix::fs::PermissionsExt;
        use std::{env, fs};
        use tempfile::tempdir;
        use transport::TcpTransport;

        let dir = tempdir().unwrap();
        let auth_path = dir.path().join("auth");
        fs::write(&auth_path, "tok user").unwrap();
        fs::set_permissions(&auth_path, fs::Permissions::from_mode(0o644)).unwrap();

        let prev = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (_s, _) = listener.accept().unwrap();
        });
        let stream = TcpStream::connect(addr).unwrap();
        let mut t = TcpTransport::from_stream(stream);

        let err = authenticate(&mut t, None, None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

        env::set_current_dir(prev).unwrap();
        handle.join().unwrap();
    }
}
