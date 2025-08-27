use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "rsync-rs")]
#[command(about = "Minimal rsync example in Rust", long_about = None)]
struct Cli {
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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Client {
            local: true,
            src,
            dst,
        } => {
            rsync_rs::synchronize(&src, &dst)?;
        }
        _ => {
            eprintln!("Only local client mode is implemented");
        }
    }
    Ok(())
}
