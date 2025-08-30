// crates/cli/src/lib.rs
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{ArgAction, ArgMatches, CommandFactory, FromArgMatches, Parser};
use compress::{available_codecs, Codec};
use engine::{sync, DeleteMode, EngineError, Result, Stats, StrongHash, SyncOptions};
use filters::{default_cvs_rules, parse, Matcher, Rule};
use meta::{Chmod, ChmodOp, ChmodTarget};
use protocol::{negotiate_version, LATEST_VERSION};
use shell_words::split as shell_split;
use transport::{AddressFamily, RateLimitedTransport, SshStdioTransport, TcpTransport, Transport};

fn parse_filters(s: &str) -> std::result::Result<Vec<Rule>, filters::ParseError> {
    let mut v = HashSet::new();
    parse(s, &mut v, 0)
}

fn parse_duration(s: &str) -> std::result::Result<Duration, std::num::ParseIntError> {
    Ok(Duration::from_secs(s.parse()?))
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
    #[arg(short, long, action = ArgAction::Count, help_heading = "Output")]
    verbose: u8,
    #[arg(short, long, help_heading = "Output")]
    quiet: bool,
    #[arg(long, help_heading = "Output")]
    no_motd: bool,
    #[arg(short = 'i', long = "itemize-changes", help_heading = "Output")]
    itemize_changes: bool,
    #[arg(long, help_heading = "Delete")]
    delete: bool,
    #[arg(long = "delete-before", help_heading = "Delete")]
    delete_before: bool,
    #[arg(long = "delete-during", help_heading = "Delete", alias = "del")]
    delete_during: bool,
    #[arg(long = "delete-after", help_heading = "Delete")]
    delete_after: bool,
    #[arg(long = "delete-delay", help_heading = "Delete")]
    delete_delay: bool,
    #[arg(long = "delete-excluded", help_heading = "Delete")]
    delete_excluded: bool,
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
    #[arg(long, help_heading = "Attributes")]
    perms: bool,
    #[arg(long = "chmod", value_name = "CHMOD", help_heading = "Attributes")]
    chmod: Vec<String>,
    #[arg(long, help_heading = "Attributes")]
    times: bool,
    #[arg(short = 'U', long, help_heading = "Attributes")]
    atimes: bool,
    #[arg(short = 'N', long, help_heading = "Attributes")]
    crtimes: bool,
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
    #[arg(short = 'P', help_heading = "Misc")]
    partial_progress: bool,
    #[arg(long, help_heading = "Misc")]
    append: bool,
    #[arg(long = "append-verify", help_heading = "Misc")]
    append_verify: bool,
    #[arg(short = 'I', long, help_heading = "Misc")]
    inplace: bool,
    #[arg(long = "bwlimit", value_name = "RATE", help_heading = "Misc")]
    bwlimit: Option<u64>,
    #[arg(long = "timeout", value_name = "SECONDS", value_parser = parse_duration, help_heading = "Misc")]
    timeout: Option<Duration>,
    #[arg(long = "contimeout", value_name = "SECONDS", value_parser = parse_duration, help_heading = "Misc")]
    contimeout: Option<Duration>,
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
    #[arg(short = 'e', long, value_name = "COMMAND")]
    rsh: Option<String>,
    #[arg(long, hide = true)]
    server: bool,
    #[arg(long, hide = true)]
    sender: bool,
    #[arg(long = "rsync-path", value_name = "PATH", alias = "rsync_path")]
    rsync_path: Option<String>,
    src: String,
    dst: String,
    #[arg(short = 'f', long, value_name = "RULE", help_heading = "Selection")]
    filter: Vec<String>,
    #[arg(long, value_name = "FILE", help_heading = "Selection")]
    filter_file: Vec<PathBuf>,
    #[arg(short = 'F', action = ArgAction::Count, help_heading = "Selection")]
    filter_shorthand: u8,
    #[arg(short = 'C', long = "cvs-exclude", help_heading = "Selection")]
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct Module {
    name: String,
    path: PathBuf,
}

fn parse_module(s: &str) -> std::result::Result<Module, String> {
    let mut parts = s.splitn(2, '=');
    let name = parts
        .next()
        .ok_or_else(|| "missing module name".to_string())?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| "missing module path".to_string())?;
    Ok(Module {
        name,
        path: PathBuf::from(path),
    })
}

#[doc(hidden)]
pub fn parse_chmod_spec(spec: &str) -> std::result::Result<Chmod, String> {
    let (target, rest) = if let Some(r) = spec.strip_prefix('D') {
        (ChmodTarget::Dir, r)
    } else if let Some(r) = spec.strip_prefix('F') {
        (ChmodTarget::File, r)
    } else {
        (ChmodTarget::All, spec)
    };

    if rest.is_empty() {
        return Err("missing mode".into());
    }

    if rest.chars().all(|c| c.is_ascii_digit()) {
        let bits = u32::from_str_radix(rest, 8).map_err(|_| "invalid octal mode")?;
        return Ok(Chmod {
            target,
            op: ChmodOp::Set,
            mask: 0o7777,
            bits,
            conditional: false,
        });
    }

    let (op_pos, op_char) = match rest.find(|c| c == '+' || c == '-' || c == '=') {
        Some(p) => (p, rest.as_bytes()[p] as char),
        None => {
            if let Some(ch) = rest.chars().find(|c| !matches!(*c, 'u' | 'g' | 'o' | 'a')) {
                return Err(format!("invalid operator '{ch}'"));
            } else {
                return Err("missing operator".into());
            }
        }
    };
    let who_part = &rest[..op_pos];
    let perm_part = &rest[op_pos + 1..];
    if perm_part.is_empty() {
        return Err("missing permissions".into());
    }

    let mut who_mask = 0u32;
    if who_part.is_empty() {
        who_mask = 0o777;
    } else {
        for ch in who_part.chars() {
            who_mask |= match ch {
                'u' => 0o700,
                'g' => 0o070,
                'o' => 0o007,
                'a' => 0o777,
                _ => return Err(format!("invalid class '{ch}'")),
            };
        }
    }

    let mut bits = 0u32;
    let mut mask = who_mask;
    let mut conditional = false;
    for ch in perm_part.chars() {
        match ch {
            'r' => bits |= 0o444 & who_mask,
            'w' => bits |= 0o222 & who_mask,
            'x' => bits |= 0o111 & who_mask,
            'X' => {
                bits |= 0o111 & who_mask;
                conditional = true;
            }
            's' => {
                if who_mask & 0o700 != 0 {
                    bits |= 0o4000;
                    mask |= 0o4000;
                }
                if who_mask & 0o070 != 0 {
                    bits |= 0o2000;
                    mask |= 0o2000;
                }
            }
            't' => {
                bits |= 0o1000;
                mask |= 0o1000;
            }
            _ => return Err(format!("invalid permission '{ch}'")),
        }
    }

    let op = match op_char {
        '+' => ChmodOp::Add,
        '-' => ChmodOp::Remove,
        '=' => ChmodOp::Set,
        other => return Err(format!("invalid operator '{other}'")),
    };

    Ok(Chmod {
        target,
        op,
        mask,
        bits,
        conditional,
    })
}

fn parse_chmod(s: &str) -> std::result::Result<Vec<Chmod>, String> {
    s.split(',').map(parse_chmod_spec).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RshCommand {
    pub env: Vec<(String, String)>,
    pub cmd: Vec<String>,
}

fn parse_env_command(parts: Vec<String>) -> RshCommand {
    let mut env = Vec::new();
    let mut iter = parts.into_iter().peekable();
    while let Some(tok) = iter.peek() {
        if let Some((k, _)) = tok.split_once('=') {
            if !k.is_empty()
                && (k.as_bytes()[0].is_ascii_alphabetic() || k.as_bytes()[0] == b'_')
                && k.as_bytes()[1..]
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_')
            {
                let tok = iter.next().unwrap();
                let (k, v) = tok.split_once('=').unwrap();
                env.push((k.to_string(), v.to_string()));
                continue;
            }
        }
        break;
    }
    let cmd: Vec<String> = iter.collect();
    RshCommand { env, cmd }
}

pub fn parse_rsh(raw: Option<String>) -> Result<RshCommand> {
    match raw {
        Some(s) => {
            let parts = shell_split(&s).map_err(|e| EngineError::Other(e.to_string()))?;
            let mut cmd = parse_env_command(parts);
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
            let cmd = parse_env_command(parts);
            if cmd.env.is_empty() && cmd.cmd.is_empty() {
                Ok(None)
            } else {
                Ok(Some(cmd))
            }
        }
        None => Ok(None),
    }
}

#[derive(Parser, Debug)]
struct DaemonOpts {
    #[arg(long)]
    daemon: bool,
    #[arg(long, value_parser = parse_module, value_name = "NAME=PATH")]
    module: Vec<Module>,
    #[arg(long)]
    address: Option<IpAddr>,
    #[arg(long, default_value_t = 873)]
    port: u16,
    #[arg(long = "timeout", value_name = "SECONDS", value_parser = parse_duration)]
    timeout: Option<Duration>,
    #[arg(long = "contimeout", value_name = "SECONDS", value_parser = parse_duration)]
    contimeout: Option<Duration>,
    #[arg(long = "bwlimit", value_name = "RATE")]
    bwlimit: Option<u64>,
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
}

#[derive(Parser, Debug)]
struct ProbeOpts {
    #[arg(long)]
    probe: bool,
    addr: Option<String>,
    #[arg(long, default_value_t = LATEST_VERSION, value_name = "VER")]
    peer_version: u32,
}

pub fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("rsync-rs {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--daemon") {
        let opts = DaemonOpts::parse_from(&args);
        run_daemon(opts)
    } else if args.iter().any(|a| a == "--probe") {
        let opts = ProbeOpts::parse_from(&args);
        run_probe(opts)
    } else if args.iter().any(|a| a == "--server") {
        run_server()
    } else {
        let cmd = ClientOpts::command();
        let matches = cmd.get_matches_from(&args);
        let opts = ClientOpts::from_arg_matches(&matches).unwrap();
        run_client(opts, &matches)
    }
}

pub fn cli_command() -> clap::Command {
    ClientOpts::command()
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

fn spawn_daemon_session(
    host: &str,
    module: &str,
    port: Option<u16>,
    password_file: Option<&Path>,
    no_motd: bool,
    timeout: Option<Duration>,
    contimeout: Option<Duration>,
    family: Option<AddressFamily>,
) -> Result<TcpTransport> {
    let (host, port) = if let Some((h, p)) = host.rsplit_once(':') {
        let p = p.parse().unwrap_or(873);
        (h, p)
    } else {
        (host, port.unwrap_or(873))
    };
    let mut t = TcpTransport::connect(host, port, contimeout, family)
        .map_err(|e| EngineError::Other(e.to_string()))?;
    t.set_read_timeout(timeout).map_err(EngineError::from)?;
    t.send(&LATEST_VERSION.to_be_bytes())
        .map_err(EngineError::from)?;
    let mut buf = [0u8; 4];
    t.receive(&mut buf).map_err(EngineError::from)?;
    let peer = u32::from_be_bytes(buf);
    negotiate_version(peer).map_err(|e| EngineError::Other(e.to_string()))?;

    let token = password_file
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()));
    t.authenticate(token.as_deref())
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
                if let Some(s) = String::from_utf8(line.clone()).ok() {
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
    Ok(t)
}

fn run_client(opts: ClientOpts, matches: &ArgMatches) -> Result<()> {
    let matcher = build_matcher(&opts, matches)?;
    let addr_family = if opts.ipv4 {
        Some(AddressFamily::V4)
    } else if opts.ipv6 {
        Some(AddressFamily::V6)
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

    if let Some(cfg) = &opts.config {
        if !opts.quiet {
            println!("using config file {}", cfg.display());
        }
    }
    if opts.verbose > 0 && !opts.quiet {
        println!("verbose level set to {}", opts.verbose);
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

    let src = parse_remote_spec(&opts.src)?;
    let mut dst = parse_remote_spec(&opts.dst)?;

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

    if !rsync_env.iter().any(|(k, _)| k == "RSYNC_CHECKSUM_LIST") {
        let mut list = vec!["md5", "sha1"];
        if opts.modern || matches!(opts.checksum_choice.as_deref(), Some("blake3")) {
            list.insert(0, "blake3");
        }
        rsync_env.push(("RSYNC_CHECKSUM_LIST".into(), list.join(",")));
    }

    let remote_bin_vec = rsync_path_cmd.as_ref().map(|c| c.cmd.clone());
    let remote_env_vec = rsync_path_cmd.as_ref().map(|c| c.env.clone());

    let strong = if let Some(choice) = opts.checksum_choice.as_deref() {
        match choice {
            "md5" => StrongHash::Md5,
            "sha1" => StrongHash::Sha1,
            "blake3" => StrongHash::Blake3,
            other => {
                return Err(EngineError::Other(format!("unknown checksum {other}")));
            }
        }
    } else if let Ok(list) = env::var("RSYNC_CHECKSUM_LIST") {
        let mut chosen = StrongHash::Md5;
        for name in list.split(',') {
            match name {
                "blake3" if opts.modern => {
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
        opts.modern
            || opts.compress
            || opts.compress_level.map_or(false, |l| l > 0)
            || compress_choice.is_some()
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
    let block_size = opts.block_size.unwrap_or(1024);
    let mut chmod_rules = Vec::new();
    for spec in &opts.chmod {
        chmod_rules.extend(parse_chmod(spec).map_err(|e| EngineError::Other(e))?);
    }
    let sync_opts = SyncOptions {
        delete: delete_mode,
        delete_excluded: opts.delete_excluded,
        checksum: opts.checksum,
        compress,
        dirs: opts.dirs,
        list_only: opts.list_only,
        update: opts.update,
        perms: opts.perms || opts.archive,
        times: opts.times || opts.archive,
        atimes: opts.atimes,
        crtimes: opts.crtimes,
        owner: opts.owner || opts.archive,
        group: opts.group || opts.archive,
        links: opts.links || opts.archive,
        copy_links: opts.copy_links,
        copy_dirlinks: opts.copy_dirlinks,
        copy_unsafe_links: opts.copy_unsafe_links,
        safe_links: opts.safe_links,
        hard_links: opts.hard_links,
        devices: opts.devices || opts.archive,
        specials: opts.specials || opts.archive,
        #[cfg(feature = "xattr")]
        xattrs: opts.xattrs,
        #[cfg(feature = "acl")]
        acls: opts.acls,
        sparse: opts.sparse,
        strong,
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
        itemize_changes: opts.itemize_changes,
        partial_dir: opts.partial_dir.clone(),
        temp_dir: opts.temp_dir.clone(),
        append: opts.append,
        append_verify: opts.append_verify,
        numeric_ids: opts.numeric_ids,
        inplace: opts.inplace,
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
    };
    let stats = if opts.local {
        match (src, dst) {
            (RemoteSpec::Local(src), RemoteSpec::Local(dst)) => sync(
                &src.path,
                &dst.path,
                &matcher,
                available_codecs(),
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
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    &matcher,
                    available_codecs(),
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
                let (session, codecs) = SshStdioTransport::connect_with_rsh(
                    &host,
                    &src.path,
                    &rsh_cmd.cmd,
                    &rsh_cmd.env,
                    &rsync_env,
                    remote_bin_vec.as_deref(),
                    remote_env_vec.as_deref().unwrap_or(&[]),
                    known_hosts.as_deref(),
                    strict_host_key_checking,
                    opts.port,
                    opts.contimeout,
                    addr_family,
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
                )?;
                sync(
                    &src.path,
                    &dst.path,
                    &matcher,
                    available_codecs(),
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
                let (session, codecs) = SshStdioTransport::connect_with_rsh(
                    &host,
                    &dst.path,
                    &rsh_cmd.cmd,
                    &rsh_cmd.env,
                    &rsync_env,
                    remote_bin_vec.as_deref(),
                    remote_env_vec.as_deref().unwrap_or(&[]),
                    known_hosts.as_deref(),
                    strict_host_key_checking,
                    opts.port,
                    opts.contimeout,
                    addr_family,
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
                        )?;
                        let mut src_session = SshStdioTransport::spawn_with_rsh(
                            &src_host,
                            &src_path.path,
                            &rsh_cmd.cmd,
                            &rsh_cmd.env,
                            remote_bin_vec.as_deref(),
                            remote_env_vec.as_deref().unwrap_or(&[]),
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
        println!("bytes transferred: {}", stats.bytes_transferred);
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
                    let p = String::from_utf8_lossy(s).trim().to_string();
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
                .map(|l| l.trim().to_string())
                .filter(|s| !s.is_empty())
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
        let idxs: Vec<_> = matches.indices_of("filter").unwrap().collect();
        for (idx, val) in idxs.into_iter().zip(values) {
            add_rules(
                idx,
                parse_filters(val).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("filter_file") {
        let idxs: Vec<_> = matches.indices_of("filter_file").unwrap().collect();
        for (idx, file) in idxs.into_iter().zip(values) {
            let content = fs::read_to_string(file)?;
            add_rules(
                idx,
                parse_filters(&content).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<String>("include") {
        let idxs: Vec<_> = matches.indices_of("include").unwrap().collect();
        for (idx, pat) in idxs.into_iter().zip(values) {
            add_rules(
                idx,
                parse_filters(&format!("+ {}", pat))
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<String>("exclude") {
        let idxs: Vec<_> = matches.indices_of("exclude").unwrap().collect();
        for (idx, pat) in idxs.into_iter().zip(values) {
            add_rules(
                idx,
                parse_filters(&format!("- {}", pat))
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("include_from") {
        let idxs: Vec<_> = matches.indices_of("include_from").unwrap().collect();
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
        let idxs: Vec<_> = matches.indices_of("exclude_from").unwrap().collect();
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
        let idxs: Vec<_> = matches.indices_of("files_from").unwrap().collect();
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
    if matches.contains_id("filter_shorthand") {
        if let Some(idx) = matches.index_of("filter_shorthand") {
            let count = matches.get_count("filter_shorthand");
            if count >= 1 {
                add_rules(
                    idx,
                    parse_filters("-F").map_err(|e| EngineError::Other(format!("{:?}", e)))?,
                );
            }
            if count >= 2 {
                add_rules(
                    idx,
                    parse_filters("- .rsync-filter")
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
                );
            }
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
        cvs.extend(parse_filters(":C").map_err(|e| EngineError::Other(format!("{:?}", e)))?);
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
    Ok(Matcher::new(rules))
}

fn run_daemon(opts: DaemonOpts) -> Result<()> {
    let mut modules = HashMap::new();
    for m in opts.module {
        modules.insert(m.name, m.path);
    }

    let secrets = opts.secrets_file.clone();
    let hosts_allow = opts.hosts_allow.clone();
    let hosts_deny = opts.hosts_deny.clone();
    let log_file = opts.log_file.clone();
    let log_format = opts.log_file_format.clone();
    let motd = opts.motd.clone();
    let timeout = opts.timeout;
    let bwlimit = opts.bwlimit;

    let (listener, port) = TcpTransport::listen(opts.address, opts.port)?;
    if opts.port == 0 {
        println!("{}", port);
        let _ = io::stdout().flush();
    }

    loop {
        let (stream, addr) = listener.accept()?;
        let ip = addr.ip();
        if !host_allowed(&ip, &hosts_allow, &hosts_deny) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            continue;
        }
        let peer = ip.to_string();
        let modules = modules.clone();
        let secrets = secrets.clone();
        let log_file = log_file.clone();
        let log_format = log_format.clone();
        let motd = motd.clone();
        std::thread::spawn(move || {
            let mut transport = TcpTransport::from_stream(stream);
            let _ = transport.set_read_timeout(timeout);
            if let Some(limit) = bwlimit {
                let mut transport = RateLimitedTransport::new(transport, limit);
                if let Err(e) = handle_connection(
                    &mut transport,
                    &modules,
                    secrets.as_deref(),
                    log_file.as_deref(),
                    log_format.as_deref(),
                    motd.as_deref(),
                    &peer,
                ) {
                    eprintln!("connection error: {}", e);
                }
            } else if let Err(e) = handle_connection(
                &mut transport,
                &modules,
                secrets.as_deref(),
                log_file.as_deref(),
                log_format.as_deref(),
                motd.as_deref(),
                &peer,
            ) {
                eprintln!("connection error: {}", e);
            }
        });
    }
}

fn host_matches(ip: &IpAddr, pat: &str) -> bool {
    if pat == "*" {
        return true;
    }
    pat.parse::<IpAddr>().map_or(false, |p| &p == ip)
}

fn host_allowed(ip: &IpAddr, allow: &[String], deny: &[String]) -> bool {
    if !allow.is_empty() && !allow.iter().any(|p| host_matches(ip, p)) {
        return false;
    }
    if deny.iter().any(|p| host_matches(ip, p)) {
        return false;
    }
    true
}

fn handle_connection<T: Transport>(
    transport: &mut T,
    modules: &HashMap<String, PathBuf>,
    secrets: Option<&Path>,
    log_file: Option<&Path>,
    log_format: Option<&str>,
    motd: Option<&Path>,
    peer: &str,
) -> Result<()> {
    let mut buf = [0u8; 4];
    let n = transport.receive(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let peer_ver = u32::from_be_bytes(buf);
    transport.send(&LATEST_VERSION.to_be_bytes())?;
    negotiate_version(peer_ver).map_err(|e| EngineError::Other(e.to_string()))?;

    let allowed = authenticate(transport, secrets).map_err(EngineError::from)?;

    if let Some(mpath) = motd {
        if let Ok(content) = fs::read_to_string(mpath) {
            for line in content.lines() {
                let msg = format!("@RSYNCD: {line}\n");
                transport.send(msg.as_bytes())?;
            }
            transport.send(b"@RSYNCD: OK\n")?;
        }
    }

    let mut name_buf = [0u8; 256];
    let n = transport.receive(&mut name_buf)?;
    let name = String::from_utf8_lossy(&name_buf[..n]).trim().to_string();
    if let Some(path) = modules.get(&name) {
        if !allowed.is_empty() && !allowed.iter().any(|m| m == &name) {
            return Err(EngineError::Other("unauthorized module".into()));
        }
        if let Some(path) = log_file {
            let fmt = log_format.unwrap_or("%h %m");
            let line = fmt.replace("%h", peer).replace("%m", &name);
            let mut f = OpenOptions::new().create(true).append(true).open(path)?;
            writeln!(f, "{}", line)?;
            f.flush()?;
        }
        #[cfg(unix)]
        {
            use nix::unistd::{chdir, chroot, setgid, setuid, Gid, Uid};
            chroot(path).map_err(|e| EngineError::Other(e.to_string()))?;
            chdir("/").map_err(|e| EngineError::Other(e.to_string()))?;
            setgid(Gid::from_raw(65534)).map_err(|e| EngineError::Other(e.to_string()))?;
            setuid(Uid::from_raw(65534)).map_err(|e| EngineError::Other(e.to_string()))?;
        }
        sync(
            Path::new("."),
            Path::new("."),
            &Matcher::default(),
            available_codecs(),
            &SyncOptions::default(),
        )?;
    }
    Ok(())
}

pub fn parse_auth_token(token: &str, contents: &str) -> Option<Vec<String>> {
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
        if let Some(tok) = parts.next() {
            if tok == token {
                return Some(parts.map(|s| s.to_string()).collect());
            }
        }
    }
    None
}

fn authenticate<T: Transport>(t: &mut T, path: Option<&Path>) -> std::io::Result<Vec<String>> {
    let auth_path = path.unwrap_or(Path::new("auth"));
    if !auth_path.exists() {
        return Ok(Vec::new());
    }
    #[cfg(unix)]
    {
        let mode = fs::metadata(auth_path)?.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "auth file permissions are too open",
            ));
        }
    }
    let contents = fs::read_to_string(auth_path)?;
    let mut buf = [0u8; 256];
    let n = t.receive(&mut buf)?;
    if n == 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "missing token",
        ));
    }
    let token = String::from_utf8_lossy(&buf[..n]).trim().to_string();
    if let Some(allowed) = parse_auth_token(&token, &contents) {
        Ok(allowed)
    } else {
        let _ = t.send(b"@ERROR: access denied");
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "unauthorized",
        ))
    }
}

fn run_probe(opts: ProbeOpts) -> Result<()> {
    if let Some(addr) = opts.addr {
        let mut stream = TcpStream::connect(&addr)?;
        stream.write_all(&LATEST_VERSION.to_be_bytes())?;
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(peer).map_err(|e| EngineError::Other(e.to_string()))?;
        println!("negotiated version {}", ver);
        Ok(())
    } else {
        let ver =
            negotiate_version(opts.peer_version).map_err(|e| EngineError::Other(e.to_string()))?;
        println!("negotiated version {}", ver);
        Ok(())
    }
}

fn run_server() -> Result<()> {
    use protocol::Server;
    let stdin = io::stdin();
    let stdout = io::stdout();
    let timeout = env::var("RSYNC_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));
    let mut srv = Server::new(stdin.lock(), stdout.lock(), timeout);
    let _ = srv
        .handshake()
        .map_err(|e| EngineError::Other(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "prog", "-r", "-n", "-v", "--delete", "-c", "-z", "--stats", "--config", "file", "src",
            "dst",
        ]);
        assert!(opts.recursive);
        assert!(opts.dry_run);
        assert_eq!(opts.verbose, 1);
        assert!(opts.delete);
        assert!(opts.checksum);
        assert!(opts.compress);
        assert!(opts.stats);
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
    fn parses_internal_server_sender_flags() {
        let opts = ClientOpts::parse_from(["prog", "--server", "--sender", "src", "dst"]);
        assert!(opts.server);
        assert!(opts.sender);
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
