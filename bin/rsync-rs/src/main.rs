use anyhow::Result;
use clap::{Parser, Subcommand};

/// Minimal rsync CLI.
#[derive(Parser)]
#[command(name = "rsync-rs")]
#[command(about = "Minimal rsync example in Rust", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Supported subcommands for the rsync CLI.
#[derive(Subcommand, Debug, PartialEq)]
enum Commands {
    /// Run in client mode
    Client,
    /// Run in daemon mode
    Daemon,
    /// Probe remote server
    Probe,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Client => cli::client(),
        Commands::Daemon => cli::daemon(),
        Commands::Probe => cli::probe(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn parses_client_subcommand() {
        let cli = Cli::try_parse_from(["rsync-rs", "client"]).unwrap();
        assert!(matches!(cli.command, Commands::Client));
    }

    #[test]
    fn help_mentions_subcommands() {
        let mut cmd = Cli::command();
        let help = cmd.render_long_help().to_string();
        assert!(help.contains("client"));
        assert!(help.contains("daemon"));
        assert!(help.contains("probe"));
    }
}
