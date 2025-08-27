use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use engine::sync;

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
    Client {
        /// perform a local sync
        #[arg(long)]
        local: bool,
        /// source path
        src: PathBuf,
        /// destination path
        dst: PathBuf,
    },
}

/// Execute the CLI using `std::env::args()`.
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Client {
            local: true,
            src,
            dst,
        } => {
            sync(&src, &dst)?;
        }
        _ => anyhow::bail!("Only local client mode is implemented"),
    }
    Ok(())
}

/// Stub implementation for the `client` subcommand.
pub fn client() -> Result<()> {
    bail!("client subcommand is not implemented")
}

/// Stub implementation for the `daemon` subcommand.
pub fn daemon() -> Result<()> {
    bail!("daemon subcommand is not implemented")
}

/// Stub implementation for the `probe` subcommand.
pub fn probe() -> Result<()> {
    bail!("probe subcommand is not implemented")
}
