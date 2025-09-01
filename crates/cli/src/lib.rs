// crates/cli/src/lib.rs
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

use daemon::{parse_config_file, parse_module, Module};
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::parser::ValueSource;
use clap::{ArgAction, ArgMatches, Args, CommandFactory, FromArgMatches, Parser};
use compress::{available_codecs, Codec, ModernCompress};
pub use engine::EngineError;
#[cfg(feature = "blake3")]
use engine::ModernHash;
use engine::{sync, DeleteMode, IdMapper, ModernCdc, Result, Stats, StrongHash, SyncOptions};
use filters::{default_cvs_rules, parse, Matcher, Rule};
use logging::{human_bytes, DebugFlag, InfoFlag};
use meta::{parse_chmod, parse_chown, parse_id_map};
#[cfg(unix)]
use nix::unistd::{Uid, User};
use protocol::{negotiate_version, LATEST_VERSION};
use shell_words::split as shell_split;
use transport::{
    parse_sockopts, AddressFamily, RateLimitedTransport, SockOpt, SshStdioTransport, TcpTransport,
    Transport,
};

fn parse_filters(s: &str) -> std::result::Result<Vec<Rule>, filters::ParseError> {
    let mut v = HashSet::new();
    parse(s, &mut v, 0)
}

fn parse_duration(s: &str) -> std::result::Result<Duration, std::num::ParseIntError> {
    Ok(Duration::from_secs(s.parse()?))
}

fn parse_size(s: &str) -> std::result::Result<usize, String> {
    let s = s.trim();
    if s == "0" {
        return Ok(0);
    }
    if let Some(last) = s.chars().last() {
        if last.is_ascii_alphabetic() {
            let num = s[..s.len() - 1]
                .parse::<usize>()
                .map_err(|e| e.to_string())?;
            let mult = match last.to_ascii_lowercase() {
                'k' => 1usize << 10,
                'm' => 1usize << 20,
                'g' => 1usize << 30,
                _ => return Err(format!("invalid size suffix: {last}")),
            };
            return num
                .checked_mul(mult)
                .ok_or_else(|| "size overflow".to_string());
        }
    }
    s.parse::<usize>().map_err(|e| e.to_string())
}

fn parse_file_size(s: &str) -> std::result::Result<u64, String> {
    let s = s.trim();
    if s == "0" {
        return Ok(0);
    }
    if let Some(last) = s.chars().last() {
        if last.is_ascii_alphabetic() {
            let num = s[..s.len() - 1].parse::<u64>().map_err(|e| e.to_string())?;
            let mult = match last.to_ascii_lowercase() {
                'k' => 1u64 << 10,
                'm' => 1u64 << 20,
                'g' => 1u64 << 30,
                _ => return Err(format!("invalid size suffix: {last}")),
            };
            return num
                .checked_mul(mult)
                .ok_or_else(|| "size overflow".to_string());
        }
    }
    s.parse::<u64>().map_err(|e| e.to_string())
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ModernCompressArg {
    Auto,
    Zstd,
    Lz4,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ModernHashArg {
    #[cfg(feature = "blake3")]
    Blake3,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ModernCdcArg {
    Fastcdc,
    Off,
}

#[derive(Parser, Debug)]
struct ClientOpts {
    #[arg(long)]
    local: bool,
    #[arg(short = 'a', long, help_heading = "Selection")]
    archive: bool,
    #[arg(short, long, help_heading = "Selection")]
    recursive: bool,
    #[arg(short = 'd', long, help_heading = "Selection")]
    dirs: bool,
    #[arg(short = 'R', long, help_heading = "Selection")]
    relative: bool,
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
    #[arg(short = 'm', long = "prune-empty-dirs", help_heading = "Misc")]
    prune_empty_dirs: bool,
    #[arg(long = "size-only", help_heading = "Misc")]
    size_only: bool,
    #[arg(short = 'I', long = "ignore-times", help_heading = "Misc")]
    ignore_times: bool,
    #[arg(short, long, action = ArgAction::Count, help_heading = "Output")]
    verbose: u8,
    #[arg(long = "log-format", help_heading = "Output", value_parser = ["text", "json"])]
    log_format: Option<String>,
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
    #[arg(long, help_heading = "Delete")]
    delete: bool,
    #[arg(long = "delete-before", help_heading = "Delete")]
    delete_before: bool,
    #[arg(long = "delete-during", help_heading = "Delete", visible_alias = "del")]
    delete_during: bool,
    #[arg(long = "delete-after", help_heading = "Delete")]
    delete_after: bool,
    #[arg(long = "delete-delay", help_heading = "Delete")]
    delete_delay: bool,
    #[arg(long = "delete-excluded", help_heading = "Delete")]
    delete_excluded: bool,
    #[arg(long = "delete-missing-args", help_heading = "Delete")]
    delete_missing_args: bool,
    #[arg(long = "ignore-missing-args", help_heading = "Delete")]
    ignore_missing_args: bool,
    #[arg(long = "remove-source-files", help_heading = "Delete")]
    remove_source_files: bool,
    #[arg(long = "ignore-errors", help_heading = "Delete")]
    ignore_errors: bool,
    #[arg(long = "max-delete", value_name = "NUM", help_heading = "Delete")]
    max_delete: Option<usize>,
    #[arg(long = "max-alloc", value_name = "SIZE", value_parser = parse_size, help_heading = "Misc")]
    max_alloc: Option<usize>,
    #[arg(long = "max-size", value_name = "SIZE", value_parser = parse_file_size, help_heading = "Misc")]
    max_size: Option<u64>,
    #[arg(long = "min-size", value_name = "SIZE", value_parser = parse_file_size, help_heading = "Misc")]
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
    #[arg(long, help_heading = "Attributes")]
    perms: bool,
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
    #[arg(long, help_heading = "Attributes")]
    times: bool,
    #[arg(short = 'U', long, help_heading = "Attributes")]
    atimes: bool,
    #[arg(short = 'N', long, help_heading = "Attributes")]
    crtimes: bool,
    #[arg(short = 'O', long, help_heading = "Attributes")]
    omit_dir_times: bool,
    #[arg(short = 'J', long, help_heading = "Attributes")]
    omit_link_times: bool,
    #[arg(long, help_heading = "Attributes")]
    owner: bool,
    #[arg(long, help_heading = "Attributes")]
    group: bool,
    #[arg(long, help_heading = "Attributes")]
    links: bool,
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
    #[arg(long = "hard-links", help_heading = "Attributes")]
    hard_links: bool,
    #[arg(long, help_heading = "Attributes")]
    devices: bool,
    #[arg(long, help_heading = "Attributes")]
    specials: bool,
    #[cfg(feature = "xattr")]
    #[arg(long, help_heading = "Attributes")]
    xattrs: bool,
    #[cfg(feature = "acl")]
    #[arg(long, help_heading = "Attributes")]
    acls: bool,
    #[arg(long = "fake-super", help_heading = "Attributes")]
    fake_super: bool,
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

    #[arg(long, help_heading = "Compression")]
    modern: bool,
    #[arg(long = "modern-compress", value_enum, help_heading = "Compression")]
    modern_compress: Option<ModernCompressArg>,
    #[arg(long = "modern-hash", value_enum, help_heading = "Compression")]
    modern_hash: Option<ModernHashArg>,
    #[arg(long = "modern-cdc", value_enum, help_heading = "Compression")]
    modern_cdc: Option<ModernCdcArg>,
    #[arg(
        long = "modern-cdc-min",
        value_name = "BYTES",
        help_heading = "Compression"
    )]
    modern_cdc_min: Option<usize>,
    #[arg(
        long = "modern-cdc-max",
        value_name = "BYTES",
        help_heading = "Compression"
    )]
    modern_cdc_max: Option<usize>,
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
    #[arg(long = "contimeout", value_name = "SECONDS", value_parser = parse_duration, help_heading = "Misc")]
    contimeout: Option<Duration>,
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
    #[arg(
        long = "sockopts",
        value_name = "OPTIONS",
        value_delimiter = ',',
        allow_hyphen_values = true,
        help_heading = "Misc"
    )]
    sockopts: Vec<String>,
    #[arg(long = "write-batch", value_name = "FILE", help_heading = "Misc")]
    write_batch: Option<PathBuf>,
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
    #[arg(long)]
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

#[allow(dead_code)]
#[derive(Parser, Debug)]
struct DaemonOpts {
    #[arg(long)]
    daemon: bool,
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
    #[arg(long = "log-file", value_name = "FILE")]
    log_file: Option<PathBuf>,
    #[arg(long = "log-file-format", value_name = "FMT")]
    log_file_format: Option<String>,
    #[arg(long = "motd", value_name = "FILE")]
    motd: Option<PathBuf>,
    #[arg(long = "lock-file", value_name = "FILE")]
    lock_file: Option<PathBuf>,
    #[arg(long = "state-dir", value_name = "DIR")]
    state_dir: Option<PathBuf>,
}

#[allow(dead_code)]
#[derive(Parser, Debug)]
struct ProbeOpts {
    #[arg(long)]
    probe: bool,
    addr: Option<String>,
    #[arg(long, default_value_t = LATEST_VERSION, value_name = "VER")]
    peer_version: u32,
}

pub fn run(matches: &clap::ArgMatches) -> Result<()> {
    let daemon_opts =
        DaemonOpts::from_arg_matches(matches).map_err(|e| EngineError::Other(e.to_string()))?;
    if daemon_opts.daemon {
        return run_daemon(daemon_opts, matches);
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
    let cmd = DaemonOpts::augment_args(cmd);
    ProbeOpts::augment_args(cmd)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathSpec {
    path: PathBuf,
    trailing_slash: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RemoteSpec {
    Local(PathSpec),
    Remote {
        host: String,
        path: PathSpec,
        module: Option<String>,
    },
}

fn parse_remote_spec(input: &str) -> Result<RemoteSpec> {
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
                && bytes
                    .get(2)
                    .map(|c| *c == b'/' || *c == b'\\')
                    .unwrap_or(false)
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

fn pipe_transports<S, D>(src: &mut S, dst: &mut D) -> io::Result<()>
where
    S: Transport,
    D: Transport,
{
    let mut buf = [0u8; 8192];
    loop {
        let n = src.receive(&mut buf)?;
        if n == 0 {
            break;
        }
        dst.send(&buf[..n])?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_daemon_session(
    host: &str,
    module: &str,
    port: Option<u16>,
    password_file: Option<&Path>,
    no_motd: bool,
    timeout: Option<Duration>,
    contimeout: Option<Duration>,
    family: Option<AddressFamily>,
    sockopts: &[String],
    opts: &SyncOptions,
    version: u32,
    early_input: Option<&Path>,
) -> Result<TcpTransport> {
    let (host, port) = if let Some((h, p)) = host.rsplit_once(':') {
        let p = p.parse().unwrap_or(873);
        (h, p)
    } else {
        (host, port.unwrap_or(873))
    };
    let mut t = TcpTransport::connect(host, port, contimeout, family)
        .map_err(|e| EngineError::Other(e.to_string()))?;
    let parsed: Vec<SockOpt> = parse_sockopts(sockopts).map_err(EngineError::Other)?;
    t.apply_sockopts(&parsed).map_err(EngineError::from)?;
    t.set_read_timeout(timeout).map_err(EngineError::from)?;
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
                if let Ok(s) = String::from_utf8(line.clone()) {
                    if let Some(msg) = s.strip_prefix("@RSYNCD: ") {
                        print!("{msg}");
                    } else {
                        print!("{s}");
                    }
                    let _ = io::stdout().flush();
                }
            }
            line.clear();
        }
    }

    let line = format!("{module}\n");
    t.send(line.as_bytes()).map_err(EngineError::from)?;
    for opt in &opts.remote_options {
        let o = format!("{opt}\n");
        t.send(o.as_bytes()).map_err(EngineError::from)?;
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

    if let Some(cfg) = &opts.config {
        if !opts.quiet {
            println!("using config file {}", cfg.display());
        }
    }
    if opts.verbose > 0 && !opts.quiet {
        tracing::info!("verbose level set to {}", opts.verbose);
    }
    if opts.recursive && !opts.quiet {
        println!("recursive mode enabled");
    }
    if opts.dry_run && !opts.list_only {
        if !opts.quiet {
            println!("dry run: skipping synchronization");
        }
        return Ok(());
    }

    let src = parse_remote_spec(&src_arg)?;
    let mut dst = parse_remote_spec(&dst_arg)?;

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

    let modern_compress = if opts.modern || opts.modern_compress.is_some() {
        Some(
            match opts.modern_compress.unwrap_or(ModernCompressArg::Auto) {
                ModernCompressArg::Auto => ModernCompress::Auto,
                ModernCompressArg::Zstd => ModernCompress::Zstd,
                ModernCompressArg::Lz4 => ModernCompress::Lz4,
            },
        )
    } else {
        None
    };
    #[cfg(feature = "blake3")]
    let modern_hash = if opts.modern || matches!(opts.modern_hash, Some(ModernHashArg::Blake3)) {
        Some(ModernHash::Blake3)
    } else {
        None
    };
    #[cfg(not(feature = "blake3"))]
    let modern_hash = None;
    let modern_cdc_arg = if let Some(arg) = opts.modern_cdc {
        arg
    } else if opts.modern || opts.modern_cdc_min.is_some() || opts.modern_cdc_max.is_some() {
        ModernCdcArg::Fastcdc
    } else {
        ModernCdcArg::Off
    };
    let modern_cdc = match modern_cdc_arg {
        ModernCdcArg::Fastcdc => ModernCdc::Fastcdc,
        ModernCdcArg::Off => ModernCdc::Off,
    };
    let modern_enabled = modern_compress.is_some()
        || modern_hash.is_some()
        || matches!(modern_cdc, ModernCdc::Fastcdc);

    if !rsync_env.iter().any(|(k, _)| k == "RSYNC_CHECKSUM_LIST") {
        #[cfg_attr(not(feature = "blake3"), allow(unused_mut))]
        let mut list = vec!["md5", "sha1", "md4"];
        #[cfg(feature = "blake3")]
        if modern_hash.is_some() || matches!(opts.checksum_choice.as_deref(), Some("blake3")) {
            list.insert(0, "blake3");
        }
        rsync_env.push(("RSYNC_CHECKSUM_LIST".into(), list.join(",")));
    }

    if modern_enabled {
        rsync_env.push(("RSYNC_MODERN".into(), "1".into()));
    }

    let remote_bin_vec = rsync_path_cmd.as_ref().map(|c| c.cmd.clone());
    let remote_env_vec = rsync_path_cmd.as_ref().map(|c| c.env.clone());

    let strong = if let Some(choice) = opts.checksum_choice.as_deref() {
        match choice {
            "md5" => StrongHash::Md5,
            "sha1" => StrongHash::Sha1,
            "md4" => StrongHash::Md4,
            "blake2" => StrongHash::Blake2b,
            "xxh64" => StrongHash::Xxh64,
            "xxh128" => StrongHash::Xxh128,
            #[cfg(feature = "blake3")]
            "blake3" => StrongHash::Blake3,
            other => {
                return Err(EngineError::Other(format!("unknown checksum {other}")));
            }
        }
    } else if let Some(h) = modern_hash {
        match h {
            #[cfg(feature = "blake3")]
            ModernHash::Blake3 => StrongHash::Blake3,
        }
    } else if let Ok(list) = env::var("RSYNC_CHECKSUM_LIST") {
        let mut chosen = StrongHash::Md5;
        for name in list.split(',') {
            match name {
                #[cfg(feature = "blake3")]
                "blake3" if modern_enabled => {
                    chosen = StrongHash::Blake3;
                    break;
                }
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
                "blake2" => {
                    chosen = StrongHash::Blake2b;
                    break;
                }
                "xxh64" => {
                    chosen = StrongHash::Xxh64;
                    break;
                }
                "xxh128" => {
                    chosen = StrongHash::Xxh128;
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
                    "zstd" => Codec::Zstd,
                    "lz4" => Codec::Lz4,
                    other => {
                        return Err(EngineError::Other(format!("unknown codec {other}")));
                    }
                };
                if !available_codecs(Some(ModernCompress::Auto)).contains(&codec) {
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
        opts.compress
            || opts.compress_level.is_some_and(|l| l > 0)
            || compress_choice.is_some()
            || modern_compress.is_some()
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
                User::from_uid(Uid::from_raw(uid))
                    .ok()
                    .flatten()
                    .map(|u| u.gid.as_raw())
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
    let uid_map = if !opts.usermap.is_empty() {
        let spec = opts.usermap.join(",");
        Some(IdMapper(parse_id_map(&spec).map_err(EngineError::Other)?))
    } else {
        None
    };
    let gid_map = if !opts.groupmap.is_empty() {
        let spec = opts.groupmap.join(",");
        Some(IdMapper(parse_id_map(&spec).map_err(EngineError::Other)?))
    } else {
        None
    };
    let mut sync_opts = SyncOptions {
        delete: delete_mode,
        delete_excluded: opts.delete_excluded,
        ignore_missing_args: opts.ignore_missing_args,
        delete_missing_args: opts.delete_missing_args,
        remove_source_files: opts.remove_source_files,
        ignore_errors: opts.ignore_errors,
        max_delete: opts.max_delete,
        max_alloc: opts.max_alloc.unwrap_or(1usize << 30),
        max_size: opts.max_size,
        min_size: opts.min_size,
        preallocate: opts.preallocate,
        checksum: opts.checksum,
        compress,
        modern_compress,
        modern_hash,
        modern_cdc,
        modern_cdc_min: opts.modern_cdc_min.unwrap_or(2 * 1024),
        modern_cdc_max: opts.modern_cdc_max.unwrap_or(16 * 1024),
        dirs: opts.dirs,
        list_only: opts.list_only,
        update: opts.update,
        existing: opts.existing,
        ignore_existing: opts.ignore_existing,
        size_only: opts.size_only,
        ignore_times: opts.ignore_times,
        perms: opts.perms || opts.archive || {
            #[cfg(feature = "acl")]
            {
                opts.acls
            }
            #[cfg(not(feature = "acl"))]
            {
                false
            }
        },
        executability: opts.executability,
        times: opts.times || opts.archive,
        atimes: opts.atimes,
        crtimes: opts.crtimes,
        omit_dir_times: opts.omit_dir_times,
        omit_link_times: opts.omit_link_times,
        owner: opts.owner
            || opts.archive
            || chown_ids.is_some_and(|(u, _)| u.is_some())
            || uid_map.is_some(),
        group: opts.group
            || opts.archive
            || chown_ids.is_some_and(|(_, g)| g.is_some())
            || gid_map.is_some(),
        links: opts.links || opts.archive,
        copy_links: opts.copy_links,
        copy_dirlinks: opts.copy_dirlinks,
        keep_dirlinks: opts.keep_dirlinks,
        copy_unsafe_links: opts.copy_unsafe_links,
        safe_links: opts.safe_links,
        hard_links: opts.hard_links,
        devices: opts.devices || opts.archive,
        specials: opts.specials || opts.archive,
        #[cfg(feature = "xattr")]
        xattrs: opts.xattrs || opts.fake_super,
        #[cfg(feature = "acl")]
        acls: opts.acls,
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
        partial: opts.partial || opts.partial_progress,
        progress: opts.progress || opts.partial_progress,
        human_readable: opts.human_readable,
        itemize_changes: opts.itemize_changes,
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
        copy_devices: opts.copy_devices,
        write_devices: opts.write_devices,
        fake_super: opts.fake_super,
        quiet: opts.quiet,
    };
    sync_opts.prepare_remote();
    let stats = if opts.local {
        match (src, dst) {
            (RemoteSpec::Local(src), RemoteSpec::Local(dst)) => sync(
                &src.path,
                &dst.path,
                &matcher,
                &available_codecs(modern_compress),
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
                    opts.contimeout,
                    addr_family,
                    &opts.sockopts,
                    &sync_opts,
                    opts.protocol
                        .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                    opts.early_input.as_deref(),
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    &matcher,
                    &available_codecs(modern_compress),
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
                let (session, codecs, _caps) = SshStdioTransport::connect_with_rsh(
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
                    opts.contimeout,
                    addr_family,
                    modern_compress,
                    opts.protocol
                        .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                )
                .map_err(|e| EngineError::Other(e.to_string()))?;
                let (err, _) = session.stderr();
                if !err.is_empty() {
                    return Err(EngineError::Other(String::from_utf8_lossy(&err).into()));
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
                    opts.contimeout,
                    addr_family,
                    &opts.sockopts,
                    &sync_opts,
                    opts.protocol
                        .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                    opts.early_input.as_deref(),
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    &matcher,
                    &available_codecs(modern_compress),
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
                let (session, codecs, _caps) = SshStdioTransport::connect_with_rsh(
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
                    opts.contimeout,
                    addr_family,
                    modern_compress,
                    opts.protocol
                        .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                )
                .map_err(|e| EngineError::Other(e.to_string()))?;
                let (err, _) = session.stderr();
                if !err.is_empty() {
                    return Err(EngineError::Other(String::from_utf8_lossy(&err).into()));
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
                            opts.contimeout,
                            addr_family,
                        )
                        .map_err(|e| EngineError::Other(e.to_string()))?;
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
                            opts.contimeout,
                            addr_family,
                        )
                        .map_err(|e| EngineError::Other(e.to_string()))?;

                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                            let (src_err, _) = src_session.stderr();
                            if !src_err.is_empty() {
                                return Err(EngineError::Other(
                                    String::from_utf8_lossy(&src_err).into(),
                                ));
                            }
                            let dst_session = dst_session.into_inner();
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                return Err(EngineError::Other(
                                    String::from_utf8_lossy(&dst_err).into(),
                                ));
                            }
                        } else {
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                            let (src_err, _) = src_session.stderr();
                            if !src_err.is_empty() {
                                return Err(EngineError::Other(
                                    String::from_utf8_lossy(&src_err).into(),
                                ));
                            }
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                return Err(EngineError::Other(
                                    String::from_utf8_lossy(&dst_err).into(),
                                ));
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
                            opts.contimeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol
                                .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                            opts.early_input.as_deref(),
                        )?;
                        let mut src_session = spawn_daemon_session(
                            &src_host,
                            &sm,
                            opts.port,
                            opts.password_file.as_deref(),
                            opts.no_motd,
                            opts.timeout,
                            opts.contimeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol
                                .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                            opts.early_input.as_deref(),
                        )?;
                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                        } else {
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
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
                            opts.contimeout,
                            addr_family,
                        )
                        .map_err(|e| EngineError::Other(e.to_string()))?;
                        let mut src_session = spawn_daemon_session(
                            &src_host,
                            &sm,
                            opts.port,
                            opts.password_file.as_deref(),
                            opts.no_motd,
                            opts.timeout,
                            opts.contimeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol
                                .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                            opts.early_input.as_deref(),
                        )?;
                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                            let dst_session = dst_session.into_inner();
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                return Err(EngineError::Other(
                                    String::from_utf8_lossy(&dst_err).into(),
                                ));
                            }
                        } else {
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                            let (dst_err, _) = dst_session.stderr();
                            if !dst_err.is_empty() {
                                return Err(EngineError::Other(
                                    String::from_utf8_lossy(&dst_err).into(),
                                ));
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
                            opts.contimeout,
                            addr_family,
                            &opts.sockopts,
                            &sync_opts,
                            opts.protocol
                                .unwrap_or(if opts.modern { LATEST_VERSION } else { 31 }),
                            opts.early_input.as_deref(),
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
                            opts.contimeout,
                            addr_family,
                        )
                        .map_err(|e| EngineError::Other(e.to_string()))?;
                        if let Some(limit) = opts.bwlimit {
                            let mut dst_session = RateLimitedTransport::new(dst_session, limit);
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                        } else {
                            pipe_transports(&mut src_session, &mut dst_session)
                                .map_err(|e| EngineError::Other(e.to_string()))?;
                        }
                        let (src_err, _) = src_session.stderr();
                        if !src_err.is_empty() {
                            return Err(EngineError::Other(
                                String::from_utf8_lossy(&src_err).into(),
                            ));
                        }
                        Stats::default()
                    }
                }
            }
        }
    };
    if opts.stats && !opts.quiet {
        println!("files transferred: {}", stats.files_transferred);
        println!("files deleted: {}", stats.files_deleted);
        if opts.human_readable {
            println!(
                "bytes transferred: {}",
                human_bytes(stats.bytes_transferred)
            );
        } else {
            println!("bytes transferred: {}", stats.bytes_transferred);
        }
    }
    Ok(())
}

fn build_matcher(opts: &ClientOpts, matches: &ArgMatches) -> Result<Matcher> {
    fn load_patterns(path: &Path, from0: bool) -> io::Result<Vec<String>> {
        if from0 {
            let content = fs::read(path)?;
            Ok(content
                .split(|b| *b == 0)
                .filter_map(|s| {
                    if s.is_empty() {
                        return None;
                    }
                    let mut end = s.len();
                    while end > 0 && (s[end - 1] == b'\n' || s[end - 1] == b'\r') {
                        end -= 1;
                    }
                    if end == 0 {
                        return None;
                    }
                    let p = String::from_utf8_lossy(&s[..end]).to_string();
                    if p.is_empty() {
                        None
                    } else {
                        Some(p)
                    }
                })
                .collect())
        } else {
            let content = fs::read_to_string(path)?;
            Ok(content
                .lines()
                .map(|l| l.trim_end_matches('\r'))
                .map(|l| l.trim())
                .filter(|s| !s.is_empty() && !s.trim_start().starts_with('#'))
                .map(|s| s.to_string())
                .collect())
        }
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
                idx,
                parse_filters(val).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("filter_file") {
        let idxs: Vec<_> = matches
            .indices_of("filter_file")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let content = fs::read_to_string(file)?;
            add_rules(
                idx,
                parse_filters(&content).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<String>("include") {
        let idxs: Vec<_> = matches
            .indices_of("include")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, pat) in idxs.into_iter().zip(values) {
            add_rules(
                idx,
                parse_filters(&format!("+ {}", pat))
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
                idx,
                parse_filters(&format!("- {}", pat))
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("include_from") {
        let idxs: Vec<_> = matches
            .indices_of("include_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            for pat in load_patterns(file, opts.from0)? {
                add_rules(
                    idx,
                    parse_filters(&format!("+ {}", pat))
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
                );
            }
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("exclude_from") {
        let idxs: Vec<_> = matches
            .indices_of("exclude_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            for pat in load_patterns(file, opts.from0)? {
                add_rules(
                    idx,
                    parse_filters(&format!("- {}", pat))
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
                );
            }
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("files_from") {
        let idxs: Vec<_> = matches
            .indices_of("files_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            for pat in load_patterns(file, opts.from0)? {
                let anchored = if pat.starts_with('/') {
                    pat.clone()
                } else {
                    format!("/{}", pat)
                };

                add_rules(
                    idx,
                    parse_filters(&format!("+ {}", anchored))
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
                );

                let dir_pat = format!("{}/***", anchored.trim_end_matches('/'));
                add_rules(
                    idx,
                    parse_filters(&format!("+ {}", dir_pat))
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
                idx,
                parse_filters(rule_str).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if !opts.files_from.is_empty() {
        add_rules(
            usize::MAX,
            parse_filters("- *").map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    if opts.cvs_exclude {
        let mut cvs = default_cvs_rules().map_err(|e| EngineError::Other(format!("{:?}", e)))?;
        cvs.extend(parse_filters(":C\n").map_err(|e| EngineError::Other(format!("{:?}", e)))?);
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
    if opts.existing {
        matcher = matcher.with_existing();
    }
    if opts.prune_empty_dirs {
        matcher = matcher.with_prune_empty_dirs();
    }
    Ok(matcher)
}

#[allow(dead_code)]
fn run_daemon(opts: DaemonOpts, matches: &ArgMatches) -> Result<()> {
    let mut modules: HashMap<String, Module> = HashMap::new();
    let mut secrets = opts.secrets_file.clone();
    let mut hosts_allow = opts.hosts_allow.clone();
    let mut hosts_deny = opts.hosts_deny.clone();
    let mut log_file = opts.log_file.clone();
    let log_format = opts.log_file_format.clone();
    let mut motd = opts.motd.clone();
    let lock_file = opts.lock_file.clone();
    let state_dir = opts.state_dir.clone();
    let mut port = matches.get_one::<u16>("port").copied().unwrap_or(873);
    let mut address = opts.address;
    let timeout = matches.get_one::<Duration>("timeout").copied();
    let bwlimit = matches.get_one::<u64>("bwlimit").copied();
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
        if let Some(a) = cfg.address {
            address = Some(a);
        }
        if !cfg.hosts_allow.is_empty() {
            hosts_allow = cfg.hosts_allow;
        }
        if !cfg.hosts_deny.is_empty() {
            hosts_deny = cfg.hosts_deny;
        }
    }

    for m in opts.module {
        modules.insert(m.name.clone(), m);
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
        hosts_allow,
        hosts_deny,
        log_file,
        log_format,
        motd,
        lock_file,
        state_dir,
        timeout,
        bwlimit,
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

#[allow(dead_code)]
fn run_probe(opts: ProbeOpts, quiet: bool) -> Result<()> {
    if let Some(addr) = opts.addr {
        let mut stream = TcpStream::connect(&addr)?;
        stream.write_all(&LATEST_VERSION.to_be_bytes())?;
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(LATEST_VERSION, peer)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    } else {
        let ver = negotiate_version(LATEST_VERSION, opts.peer_version)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn run_server() -> Result<()> {
    use protocol::{Server, CAP_CODECS, LATEST_VERSION, SUPPORTED_CAPS};
    let stdin = io::stdin();
    let stdout = io::stdout();
    let timeout = env::var("RSYNC_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));
    let modern = env::var("RSYNC_MODERN").ok().as_deref() == Some("1");
    let mc = if modern {
        Some(ModernCompress::Auto)
    } else {
        None
    };
    let codecs = available_codecs(mc);
    let mut srv = Server::new(stdin.lock(), stdout.lock(), timeout);
    let version = if modern { LATEST_VERSION } else { 31 };
    let caps = if modern { SUPPORTED_CAPS } else { CAP_CODECS };
    let _ = srv
        .handshake(version, caps, &codecs)
        .map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(())
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
        let opts = ClientOpts::parse_from(["prog", "--checksum-choice", "md4", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("md4"));
        let opts = ClientOpts::parse_from(["prog", "--checksum-choice", "blake2", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("blake2"));
        let opts = ClientOpts::parse_from(["prog", "--checksum-choice", "xxh64", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("xxh64"));
        let opts = ClientOpts::parse_from(["prog", "--checksum-choice", "xxh128", "src", "dst"]);
        assert_eq!(opts.checksum_choice.as_deref(), Some("xxh128"));
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
            stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();

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
            stream.write_all(&LATEST_VERSION.to_be_bytes()).unwrap();
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

        let err = authenticate(&mut t, None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);

        env::set_current_dir(prev).unwrap();
        handle.join().unwrap();
    }
}
