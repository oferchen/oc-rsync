use std::env;
use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use engine::sync;
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
}

/// A module exported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Module {
    name: String,
    path: PathBuf,
}

fn parse_module(s: &str) -> Result<Module, String> {
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

fn run_client(opts: ClientOpts) -> Result<()> {
    if opts.local {
        let src = PathBuf::from(&opts.src);
        let dst = PathBuf::from(&opts.dst);
        sync(&src, &dst)?;
        Ok(())
    } else {
        let src_remote = opts.src.contains(':');
        let dst_remote = opts.dst.contains(':');
        match (src_remote, dst_remote) {
            (false, false) => bail!("local sync requires --local flag"),
            (true, false) => bail!("remote source not implemented"),
            (false, true) => bail!("remote destination not implemented"),
            (true, true) => bail!("remote to remote sync not implemented"),
        }
    }
}

fn run_daemon(opts: DaemonOpts) -> Result<()> {
    println!("starting daemon with {} module(s)", opts.module.len());
    for m in &opts.module {
        println!("{} => {}", m.name, m.path.display());
    }
    Ok(())
}

fn run_probe(opts: ProbeOpts) -> Result<()> {
    let ver = negotiate_version(opts.peer_version)?;
    println!("negotiated version {}", ver);
    Ok(())
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
