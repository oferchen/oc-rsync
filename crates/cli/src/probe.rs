// crates/cli/src/probe.rs

use std::io::{Read, Write};
use std::net::TcpStream;

use oc_rsync_core::message::{SUPPORTED_PROTOCOLS, negotiate_version};
use oc_rsync_core::transfer::Result;

use crate::{EngineError, options::ProbeOpts};

pub(crate) fn run_probe(opts: ProbeOpts, quiet: bool) -> Result<()> {
    if let Some(addr) = opts.probe {
        let mut stream = TcpStream::connect(&addr)?;
        stream.write_all(&SUPPORTED_PROTOCOLS[0].to_be_bytes())?;
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(SUPPORTED_PROTOCOLS[0], peer)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    } else {
        let ver = negotiate_version(SUPPORTED_PROTOCOLS[0], opts.peer_version)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if !quiet {
            println!("negotiated version {}", ver);
        }
        Ok(())
    }
}
