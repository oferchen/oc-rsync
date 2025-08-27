use std::path::PathBuf;

use anyhow::Result;
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
        Commands::Client { local: true, src, dst } => {
            sync(&src, &dst)?;
        }
        _ => eprintln!("Only local client mode is implemented"),
    }
    Ok(())
}
