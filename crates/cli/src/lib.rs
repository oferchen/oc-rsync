use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use engine::{sync, EngineError, Result};
use filters::{parse as parse_filters, Matcher};
use protocol::{negotiate_version, LATEST_VERSION};

/// Command line interface for rsync-rs.
#[derive(Parser)]
#[command(name = "rsync-rs")]
#[command(about = "Minimal rsync example in Rust", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in client mode
    Client(ClientOpts),
    /// Run in daemon mode
    Daemon(DaemonOpts),
    /// Probe a remote peer
    Probe(ProbeOpts),
}

/// Arguments for the `client` subcommand.
#[derive(Parser, Debug)]
struct ClientOpts {
    /// perform a local sync
    #[arg(long)]
    local: bool,
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

/// Arguments for the `daemon` subcommand.
#[derive(Parser, Debug)]
struct DaemonOpts {
    /// module declarations of the form NAME=PATH
    #[arg(long, value_parser = parse_module, value_name = "NAME=PATH")]
    module: Vec<Module>,
}

/// Arguments for the `probe` subcommand.
#[derive(Parser, Debug)]
struct ProbeOpts {
    /// Optional address of peer in HOST:PORT form
    addr: Option<String>,
    /// version reported by peer
    #[arg(long, default_value_t = LATEST_VERSION, value_name = "VER")]
    peer_version: u32,
}

/// Execute the CLI using `std::env::args()`.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Client(opts) => run_client(opts),
        Commands::Daemon(opts) => run_daemon(opts),
        Commands::Probe(opts) => run_probe(opts),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RemoteSpec {
    Local(PathBuf),
    Remote { host: String, path: PathBuf },
}

fn parse_remote_spec(s: &str) -> Result<RemoteSpec> {
    if let Some(rest) = s.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let host = &rest[..end];
            if let Some(path) = rest[end + 1..].strip_prefix(':') {
                return Ok(RemoteSpec::Remote {
                    host: host.to_string(),
                    path: PathBuf::from(path),
                });
            }
        }
        return Ok(RemoteSpec::Local(PathBuf::from(s)));
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
                return Ok(RemoteSpec::Local(PathBuf::from(s)));
            }
        }
        let (host, path) = s.split_at(idx);
        return Ok(RemoteSpec::Remote {
            host: host.to_string(),
            path: PathBuf::from(&path[1..]),
        });
    }
    Ok(RemoteSpec::Local(PathBuf::from(s)))
}

fn run_client(opts: ClientOpts) -> Result<()> {
    let matcher = build_matcher(&opts)?;
    let src = parse_remote_spec(&opts.src)?;
    let dst = parse_remote_spec(&opts.dst)?;
    if opts.local {
        match (src, dst) {
            (RemoteSpec::Local(src), RemoteSpec::Local(dst)) => {
                sync(&src, &dst, &matcher)?;
                Ok(())
            }
            _ => Err(EngineError::Other("local sync requires local paths".into())),
        }
    } else {
        match (src, dst) {
            (RemoteSpec::Local(_), RemoteSpec::Local(_)) => Err(EngineError::Other(
                "local sync requires --local flag".into(),
            )),
            (RemoteSpec::Remote { path: src, .. }, RemoteSpec::Local(dst)) => {
                sync(&src, &dst, &matcher)?;
                Ok(())
            }
            (RemoteSpec::Local(src), RemoteSpec::Remote { path: dst, .. }) => {
                sync(&src, &dst, &matcher)?;
                Ok(())
            }
            (RemoteSpec::Remote { .. }, RemoteSpec::Remote { .. }) => Err(EngineError::Other(
                "remote to remote sync not implemented".into(),
            )),
        }
    }
}

fn build_matcher(opts: &ClientOpts) -> Result<Matcher> {
    let mut rules = Vec::new();
    for rule in &opts.filter {
        rules.extend(
            parse_filters(rule).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    for file in &opts.filter_file {
        let content = fs::read_to_string(file)?;
        rules.extend(
            parse_filters(&content).map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    Ok(Matcher::new(rules))
}

fn run_daemon(opts: DaemonOpts) -> Result<()> {
    println!("starting daemon with {} module(s)", opts.module.len());
    for m in &opts.module {
        println!("{} => {}", m.name, m.path.display());
    }
    Ok(())
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

/// Entry point for the `client` subcommand.
pub fn client() -> Result<()> {
    let args = env::args().skip(2); // skip binary and subcommand
    let opts = ClientOpts::parse_from(std::iter::once("client".to_string()).chain(args));
    run_client(opts)
}

/// Entry point for the `daemon` subcommand.
pub fn daemon() -> Result<()> {
    let args = env::args().skip(2);
    let opts = DaemonOpts::parse_from(std::iter::once("daemon".to_string()).chain(args));
    run_daemon(opts)
}

/// Entry point for the `probe` subcommand.
pub fn probe() -> Result<()> {
    let args = env::args().skip(2);
    let opts = ProbeOpts::parse_from(std::iter::once("probe".to_string()).chain(args));
    run_probe(opts)
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
}
