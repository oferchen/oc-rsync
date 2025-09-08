// crates/transport/src/factory.rs
use std::io;

use crate::{LocalPipeTransport, SshStdioTransport, TcpTransport, Transport};


pub enum TransportFactory {
    
    Ssh { program: String, args: Vec<String> },
    
    Tcp { host: String, port: u16 },
    
    Stdio,
}

impl TransportFactory {
    
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
use std::iter;

use crate::{SshStdioTransport, TcpTransport, Transport};

pub struct TransportFactory;

impl TransportFactory {
    pub fn from_uri(uri: &str) -> io::Result<Box<dyn Transport>> {
        let (scheme, rest) = uri
            .split_once("://")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid URI"))?;
        match scheme {
            "ssh" => {
                let host = rest.split('/').next().unwrap_or("");
                if host.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing host"));
                }
                let transport = SshStdioTransport::spawn_server(
                    host,
                    iter::empty::<String>(),
                    &[],
                    None,
                    false,
                    None,
                    None,
                )?;
                Ok(Box::new(transport))
            }
            "rsync" => {
                let host_port = rest.split('/').next().unwrap_or("");
                if host_port.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "missing host"));
                }
                let mut parts = host_port.split(':');
                let host = parts.next().unwrap();
                let port = parts.next().and_then(|p| p.parse().ok()).unwrap_or(873);
                let transport = TcpTransport::connect(host, port, None, None)?;
                Ok(Box::new(transport))
            }
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unsupported scheme: {scheme}"),
            )),
        }
    }
}
