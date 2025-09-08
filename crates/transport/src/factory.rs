// crates/transport/src/factory.rs
use std::io;

use crate::{LocalPipeTransport, SshStdioTransport, TcpTransport, Transport};

/// `TransportFactory` builds transports based on configuration.
pub enum TransportFactory {
    /// Spawn an SSH transport using the given program and arguments.
    Ssh { program: String, args: Vec<String> },
    /// Connect to a TCP host and port.
    Tcp { host: String, port: u16 },
    /// Use the local standard input/output as a transport.
    Stdio,
}

impl TransportFactory {
    /// Build the requested transport.
    pub fn build(self) -> io::Result<Box<dyn Transport>> {
        match self {
            TransportFactory::Ssh { program, args } => {
                let session = SshStdioTransport::spawn(&program, args)?;
                Ok(Box::new(session))
            }
            TransportFactory::Tcp { host, port } => {
                let session = TcpTransport::connect(&host, port, None, None)?;
                Ok(Box::new(session))
            }
            TransportFactory::Stdio => {
                let t = LocalPipeTransport::new(io::stdin(), io::stdout());
                Ok(Box::new(t))
            }
        }
    }
}
