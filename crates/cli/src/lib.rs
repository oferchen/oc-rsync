use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser};
use compress::{available_codecs, Codec};
use engine::{sync, EngineError, Result, Stats, SyncOptions};
use filters::{parse as parse_filters, Matcher};
use protocol::{negotiate_version, LATEST_VERSION};
use transport::{SshStdioTransport, TcpTransport, Transport};

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
    /// increase logging verbosity
    #[arg(short, long, action = ArgAction::Count, help_heading = "Output")]
    verbose: u8,
    /// suppress non-error messages
    #[arg(short, long, help_heading = "Output")]
    quiet: bool,
    /// remove extraneous files from the destination
    #[arg(long, help_heading = "Delete")]
    delete: bool,
    /// use full checksums to determine file changes
    #[arg(short = 'c', long, help_heading = "Attributes")]
    checksum: bool,
    /// compress file data during the transfer
    #[arg(short = 'z', long, help_heading = "Compression")]
    compress: bool,
    /// keep partially transferred files and show progress
    #[arg(short = 'P', help_heading = "Misc")]
    partial: bool,
    /// don't map uid/gid values by user/group name
    #[arg(long, help_heading = "Attributes")]
    numeric_ids: bool,
    /// display transfer statistics on completion
    #[arg(long, help_heading = "Output")]
    stats: bool,
    /// supply a custom configuration file
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,
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
    Remote { host: String, path: PathSpec },
}

fn parse_remote_spec(input: &str) -> Result<RemoteSpec> {
    let (trailing_slash, s) = if input != "/" && input.ends_with('/') {
        (true, &input[..input.len() - 1])
    } else {
        (false, input)
    };
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

    let codecs = available_codecs();
    transport
        .send(&[codecs.len() as u8])
        .map_err(EngineError::from)?;
    for c in codecs {
        transport.send(&[*c as u8]).map_err(EngineError::from)?;
    }

    let mut len_buf = [0u8; 1];
    let mut read = 0;
    while read < 1 {
        let n = transport
            .receive(&mut len_buf[read..])
            .map_err(EngineError::from)?;
        if n == 0 {
            return Err(EngineError::Other(
                "failed to read codec list length".into(),
            ));
        }
        read += n;
    }
    let len = len_buf[0] as usize;
    let mut buf = vec![0u8; len];
    let mut off = 0;
    while off < len {
        let n = transport
            .receive(&mut buf[off..])
            .map_err(EngineError::from)?;
        if n == 0 {
            return Err(EngineError::Other("failed to read codec list".into()));
        }
        off += n;
    }

    let mut remote = Vec::new();
    for b in buf {
        let codec = match b {
            0 => Codec::Zlib,
            1 => Codec::Zstd,
            2 => Codec::Lz4,
            other => {
                return Err(EngineError::Other(format!("unknown codec {}", other)));
            }
        };
        remote.push(codec);
    }

    Ok(remote)
}

fn run_client(opts: ClientOpts) -> Result<()> {
    let matcher = build_matcher(&opts)?;

    if opts.archive {
        return Err(EngineError::Other(
            "flag -a/--archive is not supported; see docs/differences.md".into(),
        ));
    }
    if opts.relative {
        return Err(EngineError::Other(
            "flag -R/--relative is not supported; see docs/differences.md".into(),
        ));
    }
    if opts.partial {
        return Err(EngineError::Other(
            "flag -P is not supported; see docs/differences.md".into(),
        ));
    }
    if opts.numeric_ids {
        return Err(EngineError::Other(
            "flag --numeric-ids is not supported; see docs/differences.md".into(),
        ));
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

    let src_trailing = match &src {
        RemoteSpec::Local(p) => p.trailing_slash,
        RemoteSpec::Remote { path, .. } => path.trailing_slash,
    };
    if !src_trailing {
        let name = match &src {
            RemoteSpec::Local(p) => p.path.file_name().map(|s| s.to_owned()),
            RemoteSpec::Remote { path, .. } => path.path.file_name().map(|s| s.to_owned()),
        }
        .ok_or_else(|| EngineError::Other("source path missing file name".into()))?;
        match &mut dst {
            RemoteSpec::Local(p) => p.path.push(&name),
            RemoteSpec::Remote { path, .. } => path.path.push(&name),
        }
    }

    let sync_opts = SyncOptions {
        delete: opts.delete,
        checksum: opts.checksum,
        compress: opts.compress,
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
            (RemoteSpec::Remote { host, path: src }, RemoteSpec::Local(dst)) => {
                let mut session = SshStdioTransport::spawn_server(&host, [src.path.as_os_str()])
                    .map_err(|e| EngineError::Other(e.to_string()))?;
                let codecs = handshake_with_peer(&mut session)?;
                let err = session.stderr();
                if !err.is_empty() {
                    return Err(EngineError::Other(String::from_utf8_lossy(&err).into()));
                }
                sync(&src.path, &dst.path, &matcher, &codecs, &sync_opts)?
            }
            (RemoteSpec::Local(src), RemoteSpec::Remote { host, path: dst }) => {
                let mut session = SshStdioTransport::spawn_server(&host, [dst.path.as_os_str()])
                    .map_err(|e| EngineError::Other(e.to_string()))?;
                let codecs = handshake_with_peer(&mut session)?;
                let err = session.stderr();
                if !err.is_empty() {
                    return Err(EngineError::Other(String::from_utf8_lossy(&err).into()));
                }
                sync(&src.path, &dst.path, &matcher, &codecs, &sync_opts)?
            }
            (
                RemoteSpec::Remote {
                    host: src_host,
                    path: src_path,
                },
                RemoteSpec::Remote {
                    host: dst_host,
                    path: dst_path,
                },
            ) => {
                if src_host.is_empty() || dst_host.is_empty() {
                    return Err(EngineError::Other("remote host missing".to_string()));
                }
                if src_path.path.as_os_str().is_empty() || dst_path.path.as_os_str().is_empty() {
                    return Err(EngineError::Other("remote path missing".to_string()));
                }

                let mut src_session =
                    SshStdioTransport::spawn_server(&src_host, [src_path.path.as_os_str()])
                        .map_err(|e| EngineError::Other(e.to_string()))?;
                let mut dst_session =
                    SshStdioTransport::spawn_server(&dst_host, [dst_path.path.as_os_str()])
                        .map_err(|e| EngineError::Other(e.to_string()))?;

                pipe_transports(&mut src_session, &mut dst_session)
                    .map_err(|e| EngineError::Other(e.to_string()))?;
                let src_err = src_session.stderr();
                if !src_err.is_empty() {
                    return Err(EngineError::Other(String::from_utf8_lossy(&src_err).into()));
                }
                let dst_err = dst_session.stderr();
                if !dst_err.is_empty() {
                    return Err(EngineError::Other(String::from_utf8_lossy(&dst_err).into()));
                }
                Stats::default()
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
    let mut rules = Vec::new();
    for rule in &opts.filter {
        rules.extend(parse_filters(rule).map_err(|e| EngineError::Other(format!("{:?}", e)))?);
    }
    for file in &opts.filter_file {
        let content = fs::read_to_string(file)?;
        rules.extend(parse_filters(&content).map_err(|e| EngineError::Other(format!("{:?}", e)))?);
    }
    Ok(Matcher::new(rules))
}

fn run_daemon(opts: DaemonOpts) -> Result<()> {
    let mut modules = HashMap::new();
    for m in opts.module {
        modules.insert(m.name, m.path);
    }

    let listener = TcpListener::bind("127.0.0.1:873")?;

    loop {
        let (stream, _) = listener.accept()?;
        let modules = modules.clone();
        std::thread::spawn(move || {
            let mut transport = TcpTransport::from_stream(stream);
            if let Err(e) = handle_connection(&mut transport, &modules) {
                eprintln!("connection error: {}", e);
            }
        });
    }
}

fn handle_connection(
    transport: &mut TcpTransport,
    modules: &HashMap<String, PathBuf>,
) -> Result<()> {
    let mut buf = [0u8; 4];
    let n = transport.receive(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let peer = u32::from_be_bytes(buf);
    transport.send(&LATEST_VERSION.to_be_bytes())?;
    negotiate_version(peer).map_err(|e| EngineError::Other(e.to_string()))?;

    let allowed = authenticate(transport).map_err(EngineError::from)?;

    let mut name_buf = [0u8; 256];
    let n = transport.receive(&mut name_buf)?;
    let name = String::from_utf8_lossy(&name_buf[..n]).trim().to_string();
    if let Some(path) = modules.get(&name) {
        if !allowed.is_empty() && !allowed.iter().any(|m| m == &name) {
            return Err(EngineError::Other("unauthorized module".into()));
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

fn authenticate(t: &mut TcpTransport) -> std::io::Result<Vec<String>> {
    let auth_path = Path::new("auth");
    if !auth_path.exists() {
        return Ok(Vec::new());
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
            RemoteSpec::Remote { host, path } => {
                assert_eq!(host, "::1");
                assert_eq!(path, PathBuf::from("/tmp"));
            }
            _ => panic!("expected remote spec"),
        }
    }

    #[test]
    fn parses_client_flags() {
        let opts = ClientOpts::parse_from([
            "-r", "-n", "-v", "--delete", "-c", "-z", "--stats", "--config", "file", "src", "dst",
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
}
