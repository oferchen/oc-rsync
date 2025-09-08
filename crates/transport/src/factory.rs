// crates/transport/src/factory.rs
use std::io;
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
