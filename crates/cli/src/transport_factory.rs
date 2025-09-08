// crates/cli/src/transport_factory.rs
use transport::TransportFactory;

use crate::options::ClientOpts;

impl From<&ClientOpts> for TransportFactory {
    fn from(opts: &ClientOpts) -> Self {
        if opts.daemon.daemon {
            TransportFactory::Tcp {
                host: "localhost".to_string(),
                port: 873,
            }
        } else if let Some(rsh) = &opts.rsh {
            let parts = rsh.cmd.clone();
            let program = parts.first().cloned().unwrap_or_else(|| "ssh".to_string());
            let args = if parts.len() > 1 {
                parts[1..].to_vec()
            } else {
                Vec::new()
            };
            TransportFactory::Ssh { program, args }
        } else {
            TransportFactory::Stdio
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::ClientOpts;
    use clap::Parser;

    #[test]
    fn selects_ssh_when_rsh_specified() {
        let opts = ClientOpts::parse_from(["prog", "--rsh", "ssh", "src", "dest"]);
        match TransportFactory::from(&opts) {
            TransportFactory::Ssh { .. } => {}
            _ => panic!("expected ssh variant"),
        }
    }

    #[test]
    fn selects_stdio_by_default() {
        let opts = ClientOpts::parse_from(["prog", "src", "dest"]);
        match TransportFactory::from(&opts) {
            TransportFactory::Stdio => {}
            _ => panic!("expected stdio variant"),
        }
    }
}
