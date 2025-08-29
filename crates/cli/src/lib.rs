use std::collections::HashMap;
use std::convert::TryFrom;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{ArgAction, Parser};
use compress::{available_codecs, Codec};
use engine::{sync, DeleteMode, EngineError, Result, Stats, StrongHash, SyncOptions};
use filters::{parse as parse_filters, Matcher};
use protocol::{negotiate_version, Frame, FrameHeader, Message, Msg, Tag, LATEST_VERSION};
use shell_words::split as shell_split;
use transport::{RateLimitedTransport, SshStdioTransport, TcpTransport, Transport};

/// Command line interface for rsync-rs.
///
/// This binary follows the flag based interface of the real `rsync` where the
/// mode of operation is selected via top level flags such as `--daemon` or
/// `--probe`.  When neither of those flags are supplied it runs in client mode
/// and expects positional `SRC` and `DST` arguments.
/// Options for client mode.
#[derive(Parser, Debug)]
struct ClientOpts {
    /// perform a local sync
    #[arg(long)]
    local: bool,
    /// archive mode
    #[arg(short = 'a', long, help_heading = "Selection")]
    archive: bool,
    /// copy directories recursively
    #[arg(short, long, help_heading = "Selection")]
    recursive: bool,
    /// use relative path names
    #[arg(short = 'R', long, help_heading = "Selection")]
    relative: bool,
    /// perform a trial run with no changes made
    #[arg(short = 'n', long, help_heading = "Selection")]
    dry_run: bool,
    /// turn sequences of nulls into sparse blocks and preserve existing holes
    /// (requires filesystem support)
    #[arg(short = 'S', long, help_heading = "Selection")]
    sparse: bool,
    /// increase logging verbosity
    #[arg(short, long, action = ArgAction::Count, help_heading = "Output")]
    verbose: u8,
    /// suppress non-error messages
    #[arg(short, long, help_heading = "Output")]
    quiet: bool,
    /// suppress daemon-mode MOTD
    #[arg(long, help_heading = "Output")]
    no_motd: bool,
    /// remove extraneous files from the destination
    #[arg(long, help_heading = "Delete")]
    delete: bool,
    /// receiver deletes before transfer, not during
    #[arg(long = "delete-before", help_heading = "Delete")]
    delete_before: bool,
    /// receiver deletes during the transfer
    #[arg(long = "delete-during", help_heading = "Delete", alias = "del")]
    delete_during: bool,
    /// receiver deletes after transfer, not during
    #[arg(long = "delete-after", help_heading = "Delete")]
    delete_after: bool,
    /// find deletions during, delete after
    #[arg(long = "delete-delay", help_heading = "Delete")]
    delete_delay: bool,
    /// also delete excluded files from destination
    #[arg(long = "delete-excluded", help_heading = "Delete")]
    delete_excluded: bool,
    /// use full checksums to determine file changes
    #[arg(short = 'c', long, help_heading = "Attributes")]
    checksum: bool,
    /// preserve permissions
    #[arg(long, help_heading = "Attributes")]
    perms: bool,
    /// preserve modification times
    #[arg(long, help_heading = "Attributes")]
    times: bool,
    /// preserve access times
    #[arg(short = 'U', long, help_heading = "Attributes")]
    atimes: bool,
    /// preserve create times
    #[arg(short = 'N', long, help_heading = "Attributes")]
    crtimes: bool,
    /// preserve owner
    #[arg(long, help_heading = "Attributes")]
    owner: bool,
    /// preserve group
    #[arg(long, help_heading = "Attributes")]
    group: bool,
    /// copy symlinks as symlinks
    #[arg(long, help_heading = "Attributes")]
    links: bool,
    /// preserve hard links
    #[arg(long = "hard-links", help_heading = "Attributes")]
    hard_links: bool,
    /// preserve device files
    #[arg(long, help_heading = "Attributes")]
    devices: bool,
    /// preserve special files
    #[arg(long, help_heading = "Attributes")]
    specials: bool,
    /// preserve extended attributes
    #[cfg(feature = "xattr")]
    #[arg(long, help_heading = "Attributes")]
    xattrs: bool,
    /// preserve ACLs
    #[cfg(feature = "acl")]
    #[arg(long, help_heading = "Attributes")]
    acls: bool,
    /// compress file data during the transfer (zlib by default, negotiates zstd when supported)
    #[arg(short = 'z', long, help_heading = "Compression")]
    compress: bool,
    /// explicitly set compression level
    #[arg(
        long = "compress-level",
        value_name = "NUM",
        help_heading = "Compression"
    )]
    compress_level: Option<i32>,
    /// enable BLAKE3 checksums (zstd is negotiated automatically)
    #[arg(long, help_heading = "Compression")]
    modern: bool,
    /// keep partially transferred files
    #[arg(long, help_heading = "Misc")]
    partial: bool,
    /// put a partially transferred file into DIR
    #[arg(long = "partial-dir", value_name = "DIR", help_heading = "Misc")]
    partial_dir: Option<PathBuf>,
    /// show progress during transfer
    #[arg(long, help_heading = "Misc")]
    progress: bool,
    /// keep partially transferred files and show progress
    #[arg(short = 'P', help_heading = "Misc")]
    partial_progress: bool,
    /// update destination files in-place
    #[arg(short = 'I', long, help_heading = "Misc")]
    inplace: bool,
    /// throttle I/O bandwidth to RATE bytes per second
    #[arg(long = "bwlimit", value_name = "RATE", help_heading = "Misc")]
    bwlimit: Option<u64>,
    /// hardlink to files in DIR when unchanged
    #[arg(long = "link-dest", value_name = "DIR", help_heading = "Misc")]
    link_dest: Option<PathBuf>,
    /// copy files from DIR when unchanged
    #[arg(long = "copy-dest", value_name = "DIR", help_heading = "Misc")]
    copy_dest: Option<PathBuf>,
    /// skip files that match in DIR
    #[arg(long = "compare-dest", value_name = "DIR", help_heading = "Misc")]
    compare_dest: Option<PathBuf>,
    /// don't map uid/gid values by user/group name
    #[arg(long, help_heading = "Attributes")]
    numeric_ids: bool,
    /// display transfer statistics on completion
    #[arg(long, help_heading = "Output")]
    stats: bool,
    /// supply a custom configuration file
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
    /// path to SSH known hosts file
    #[arg(long, value_name = "FILE", env = "RSYNC_KNOWN_HOSTS")]
    known_hosts: Option<PathBuf>,
    /// disable strict host key checking (not recommended)
    #[arg(long, env = "RSYNC_NO_HOST_KEY_CHECKING")]
    no_host_key_checking: bool,
    /// read daemon-access password from FILE
    #[arg(long = "password-file", value_name = "FILE")]
    password_file: Option<PathBuf>,
    /// specify the remote shell to use
    #[arg(short = 'e', long, value_name = "COMMAND")]
    rsh: Option<String>,
    /// run in server mode (internal use)
    #[arg(long, hide = true)]
    server: bool,
    /// run in sender mode (internal use)
    #[arg(long, hide = true)]
    sender: bool,
    /// specify the rsync to run on remote machine
    #[arg(long = "rsync-path", value_name = "PATH", alias = "rsync_path")]
    rsync_path: Option<PathBuf>,
    /// source path or HOST:PATH
    src: String,
    /// destination path or HOST:PATH
    dst: String,
    /// filter rules provided directly
    #[arg(long, value_name = "RULE")]
    filter: Vec<String>,
    /// files containing filter rules
    #[arg(long, value_name = "FILE")]
    filter_file: Vec<PathBuf>,
    /// include files matching PATTERN
    #[arg(long, value_name = "PATTERN")]
    include: Vec<String>,
    /// exclude files matching PATTERN
    #[arg(long, value_name = "PATTERN")]
    exclude: Vec<String>,
    /// read include patterns from FILE
    #[arg(long, value_name = "FILE")]
    include_from: Vec<PathBuf>,
    /// read exclude patterns from FILE
    #[arg(long, value_name = "FILE")]
    exclude_from: Vec<PathBuf>,
    /// read list of files from FILE
    #[arg(long, value_name = "FILE")]
    files_from: Vec<PathBuf>,
    /// treat file lists as null-separated
    #[arg(long)]
    from0: bool,
}

/// A module exported by the daemon.
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

/// Options for daemon mode.
#[derive(Parser, Debug)]
struct DaemonOpts {
    /// run in daemon mode
    #[arg(long)]
    daemon: bool,
    /// module declarations of the form NAME=PATH
    #[arg(long, value_parser = parse_module, value_name = "NAME=PATH")]
    module: Vec<Module>,
    /// port to listen on
    #[arg(long, default_value_t = 873)]
    port: u16,
    /// path to secrets file
    #[arg(long = "secrets-file", value_name = "FILE")]
    secrets_file: Option<PathBuf>,
    /// list of hosts allowed to connect
    #[arg(long = "hosts-allow", value_delimiter = ',', value_name = "LIST")]
    hosts_allow: Vec<String>,
    /// list of hosts denied from connecting
    #[arg(long = "hosts-deny", value_delimiter = ',', value_name = "LIST")]
    hosts_deny: Vec<String>,
    /// log file path
    #[arg(long = "log-file", value_name = "FILE")]
    log_file: Option<PathBuf>,
    /// log file format (supports %h for host and %m for module)
    #[arg(long = "log-file-format", value_name = "FMT")]
    log_file_format: Option<String>,
    /// path to message of the day file
    #[arg(long = "motd", value_name = "FILE")]
    motd: Option<PathBuf>,
}

/// Options for the probe mode.
#[derive(Parser, Debug)]
struct ProbeOpts {
    /// run in probe mode
    #[arg(long)]
    probe: bool,
    /// Optional address of peer in HOST:PORT form
    addr: Option<String>,
    /// version reported by peer
    #[arg(long, default_value_t = LATEST_VERSION, value_name = "VER")]
    peer_version: u32,
}

/// Execute the CLI using `std::env::args()`.
pub fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--daemon") {
        let opts = DaemonOpts::parse_from(&args);
        run_daemon(opts)
    } else if args.iter().any(|a| a == "--probe") {
        let opts = ProbeOpts::parse_from(&args);
        run_probe(opts)
    } else if args.iter().any(|a| a == "--server") {
        run_server()
    } else {
        let opts = ClientOpts::parse_from(&args);
        run_client(opts)
    }
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

fn spawn_remote_session(
    host: &str,
    path: &Path,
    rsh: &[String],
    remote_bin: Option<&Path>,
    known_hosts: Option<&Path>,
    strict_host_key_checking: bool,
) -> io::Result<SshStdioTransport> {
    if host == "sh" {
        let cmd = path
            .to_str()
            .ok_or_else(|| io::Error::other("invalid command"))?;
        let program = rsh.get(0).map(|s| s.as_str()).unwrap_or("sh");
        let mut args: Vec<String> = rsh.iter().skip(1).cloned().collect();
        args.push("-c".to_string());
        args.push(cmd.to_string());
        SshStdioTransport::spawn(program, args)
    } else {
        let program = rsh.get(0).map(|s| s.as_str()).unwrap_or("ssh");
        if program == "ssh" {
            let mut cmd = Command::new(program);
            cmd.args(&rsh[1..]);
            let known_hosts_path = known_hosts.map(Path::to_path_buf).or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|h| PathBuf::from(h).join(".ssh/known_hosts"))
            });
            let checking = if strict_host_key_checking {
                "yes"
            } else {
                "no"
            };
            cmd.arg("-o")
                .arg(format!("StrictHostKeyChecking={checking}"));
            if let Some(path) = known_hosts_path {
                cmd.arg("-o")
                    .arg(format!("UserKnownHostsFile={}", path.display()));
            }
            cmd.arg(host);
            if let Some(bin) = remote_bin {
                cmd.arg(bin);
            } else {
                cmd.arg("rsync");
            }
            cmd.arg("--server");
            cmd.arg(path.as_os_str());
            SshStdioTransport::spawn_from_command(cmd)
        } else {
            let mut args = rsh[1..].to_vec();
            args.push(host.to_string());
            if let Some(bin) = remote_bin {
                args.push(bin.to_string_lossy().into_owned());
            } else {
                args.push("rsync".to_string());
            }
            args.push("--server".to_string());
            args.push(path.to_string_lossy().into_owned());
            SshStdioTransport::spawn(program, args)
        }
    }
}

fn spawn_daemon_session(
    host: &str,
    module: &str,
    password_file: Option<&Path>,
    no_motd: bool,
) -> Result<TcpTransport> {
    let addr = if host.contains(':') {
        host.to_string()
    } else {
        format!("{host}:873")
    };
    let mut t = TcpTransport::connect(&addr).map_err(|e| EngineError::Other(e.to_string()))?;
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

    // consume daemon greeting until OK
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

fn handshake_with_peer<T: Transport>(transport: &mut T) -> Result<Vec<Codec>> {
    transport
        .send(&LATEST_VERSION.to_be_bytes())
        .map_err(EngineError::from)?;

    let mut ver_buf = [0u8; 4];
    let mut read = 0;
    while read < ver_buf.len() {
        let n = transport
            .receive(&mut ver_buf[read..])
            .map_err(EngineError::from)?;
        if n == 0 {
            return Err(EngineError::Other("failed to read version".into()));
        }
        read += n;
    }
    let peer = u32::from_be_bytes(ver_buf);
    negotiate_version(peer).map_err(|e| EngineError::Other(e.to_string()))?;

    // Advertise our supported codecs via a codecs message frame.
    let payload = compress::encode_codecs(available_codecs());
    let frame = Message::Codecs(payload).to_frame(0);
    let mut buf = Vec::new();
    frame.encode(&mut buf).map_err(EngineError::from)?;
    transport.send(&buf).map_err(EngineError::from)?;

    // Read the peer's codecs message frame.
    let mut hdr = [0u8; 8];
    let mut read = 0;
    while read < hdr.len() {
        let n = transport
            .receive(&mut hdr[read..])
            .map_err(EngineError::from)?;
        if n == 0 {
            return Err(EngineError::Other("failed to read frame header".into()));
        }
        read += n;
    }
    let channel = u16::from_be_bytes([hdr[0], hdr[1]]);
    let tag = Tag::try_from(hdr[2]).map_err(|e| EngineError::Other(e.to_string()))?;
    let msg = Msg::try_from(hdr[3]).map_err(|e| EngineError::Other(e.to_string()))?;
    let len = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    let mut payload = vec![0u8; len];
    let mut off = 0;
    while off < len {
        let n = transport
            .receive(&mut payload[off..])
            .map_err(EngineError::from)?;
        if n == 0 {
            return Err(EngineError::Other("failed to read frame payload".into()));
        }
        off += n;
    }
    let frame = Frame {
        header: FrameHeader {
            channel,
            tag,
            msg,
            len: len as u32,
        },
        payload,
    };
    let msg = Message::from_frame(frame).map_err(EngineError::from)?;
    match msg {
        Message::Codecs(data) => compress::decode_codecs(&data).map_err(EngineError::from),
        _ => Err(EngineError::Other("expected codecs message".into())),
    }
}

fn run_client(opts: ClientOpts) -> Result<()> {
    let matcher = build_matcher(&opts)?;

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
    if opts.dry_run {
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
    let rsh_cmd = match rsh_raw {
        Some(cmd) => shell_split(&cmd).map_err(|e| EngineError::Other(e.to_string()))?,
        None => vec!["ssh".to_string()],
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

    let compress = opts.modern || opts.compress || opts.compress_level.map_or(false, |l| l > 0);
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
    let sync_opts = SyncOptions {
        delete: delete_mode,
        delete_excluded: opts.delete_excluded,
        checksum: opts.checksum,
        compress,
        perms: opts.perms || opts.archive,
        times: opts.times || opts.archive,
        atimes: opts.atimes,
        crtimes: opts.crtimes,
        owner: opts.owner || opts.archive,
        group: opts.group || opts.archive,
        links: opts.links || opts.archive,
        hard_links: opts.hard_links,
        devices: opts.devices || opts.archive,
        specials: opts.specials || opts.archive,
        #[cfg(feature = "xattr")]
        xattrs: opts.xattrs,
        #[cfg(feature = "acl")]
        acls: opts.acls,
        sparse: opts.sparse,
        strong: if opts.modern {
            StrongHash::Blake3
        } else {
            StrongHash::Md5
        },
        compress_level: opts.compress_level,
        partial: opts.partial || opts.partial_progress,
        progress: opts.progress || opts.partial_progress,
        partial_dir: opts.partial_dir.clone(),
        numeric_ids: opts.numeric_ids,
        inplace: opts.inplace,
        bwlimit: opts.bwlimit,
        link_dest: opts.link_dest.clone(),
        copy_dest: opts.copy_dest.clone(),
        compare_dest: opts.compare_dest.clone(),
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
                    opts.password_file.as_deref(),
                    opts.no_motd,
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
                let mut session = spawn_remote_session(
                    &host,
                    &src.path,
                    &rsh_cmd,
                    opts.rsync_path.as_deref(),
                    known_hosts.as_deref(),
                    strict_host_key_checking,
                )
                .map_err(|e| EngineError::Other(e.to_string()))?;
                let codecs = handshake_with_peer(&mut session)?;
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
                    opts.password_file.as_deref(),
                    opts.no_motd,
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
                let mut session = spawn_remote_session(
                    &host,
                    &dst.path,
                    &rsh_cmd,
                    opts.rsync_path.as_deref(),
                    known_hosts.as_deref(),
                    strict_host_key_checking,
                )
                .map_err(|e| EngineError::Other(e.to_string()))?;
                let codecs = handshake_with_peer(&mut session)?;
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
                        let mut dst_session = spawn_remote_session(
                            &dst_host,
                            &dst_path.path,
                            &rsh_cmd,
                            opts.rsync_path.as_deref(),
                            known_hosts.as_deref(),
                            strict_host_key_checking,
                        )
                        .map_err(|e| EngineError::Other(e.to_string()))?;
                        let mut src_session = spawn_remote_session(
                            &src_host,
                            &src_path.path,
                            &rsh_cmd,
                            opts.rsync_path.as_deref(),
                            known_hosts.as_deref(),
                            strict_host_key_checking,
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
                            opts.password_file.as_deref(),
                            opts.no_motd,
                        )?;
                        let mut src_session = spawn_daemon_session(
                            &src_host,
                            &sm,
                            opts.password_file.as_deref(),
                            opts.no_motd,
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
                        let mut dst_session = spawn_remote_session(
                            &dst_host,
                            &dst_path.path,
                            &rsh_cmd,
                            opts.rsync_path.as_deref(),
                            known_hosts.as_deref(),
                            strict_host_key_checking,
                        )
                        .map_err(|e| EngineError::Other(e.to_string()))?;
                        let mut src_session = spawn_daemon_session(
                            &src_host,
                            &sm,
                            opts.password_file.as_deref(),
                            opts.no_motd,
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
                            opts.password_file.as_deref(),
                            opts.no_motd,
                        )?;
                        let mut src_session = spawn_remote_session(
                            &src_host,
                            &src_path.path,
                            &rsh_cmd,
                            opts.rsync_path.as_deref(),
                            known_hosts.as_deref(),
                            strict_host_key_checking,
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

fn build_matcher(opts: &ClientOpts) -> Result<Matcher> {
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

    let mut rules = Vec::new();
    for rule in &opts.filter {
        rules.extend(parse_filters(rule).map_err(|e| EngineError::Other(format!("{:?}", e)))?);
    }
    for file in &opts.filter_file {
        let content = fs::read_to_string(file)?;
        rules.extend(parse_filters(&content).map_err(|e| EngineError::Other(format!("{:?}", e)))?);
    }
    for pat in &opts.include {
        rules.extend(
            parse_filters(&format!("+ {}", pat))
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    for pat in &opts.exclude {
        rules.extend(
            parse_filters(&format!("- {}", pat))
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    for file in &opts.include_from {
        for pat in load_patterns(file, opts.from0)? {
            rules.extend(
                parse_filters(&format!("+ {}", pat))
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    for file in &opts.exclude_from {
        for pat in load_patterns(file, opts.from0)? {
            rules.extend(
                parse_filters(&format!("- {}", pat))
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    for file in &opts.files_from {
        for pat in load_patterns(file, opts.from0)? {
            rules.extend(
                parse_filters(&format!("+ {}", pat))
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if !opts.files_from.is_empty() {
        rules.extend(parse_filters("- *").map_err(|e| EngineError::Other(format!("{:?}", e)))?);
    }
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

    let listener = TcpListener::bind(("127.0.0.1", opts.port))?;

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

fn handle_connection(
    transport: &mut TcpTransport,
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

fn authenticate(t: &mut TcpTransport, path: Option<&Path>) -> std::io::Result<Vec<String>> {
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
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
        if let Some(tok) = parts.next() {
            if tok == token {
                return Ok(parts.map(|s| s.to_string()).collect());
            }
        }
    }
    let _ = t.send(b"@ERROR: access denied");
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "unauthorized",
    ))
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
    let mut srv = Server::new(stdin.lock(), stdout.lock());
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
    fn parses_rsh_flag_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--rsh", "ssh", "src", "dst"]);
        assert_eq!(opts.rsh.as_deref(), Some("ssh"));
        let opts = ClientOpts::parse_from(["prog", "-e", "ssh", "src", "dst"]);
        assert_eq!(opts.rsh.as_deref(), Some("ssh"));
    }

    #[test]
    fn parses_rsync_path_and_alias() {
        let opts = ClientOpts::parse_from(["prog", "--rsync-path", "/bin/rsync", "src", "dst"]);
        assert_eq!(opts.rsync_path, Some(PathBuf::from("/bin/rsync")));
        let opts = ClientOpts::parse_from(["prog", "--rsync_path", "/bin/rsync", "src", "dst"]);
        assert_eq!(opts.rsync_path, Some(PathBuf::from("/bin/rsync")));
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
