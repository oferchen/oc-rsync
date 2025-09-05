// crates/transport/tests/ssh_capabilities.rs
use compress::Codec;
use protocol::{CAP_CODECS, LATEST_VERSION};
use transport::{ssh::SshStdioTransport, Transport};

struct ChunkedTransport {
    chunks: Vec<Vec<u8>>,
    idx: usize,
}

impl ChunkedTransport {
    fn new(chunks: Vec<Vec<u8>>) -> Self {
        Self { chunks, idx: 0 }
    }

    fn finished(&self) -> bool {
        self.idx >= self.chunks.len()
    }
}

impl Transport for ChunkedTransport {
    fn send(&mut self, _data: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.idx >= self.chunks.len() {
            return Ok(0);
        }
        let chunk = &self.chunks[self.idx];
        let n = chunk.len().min(buf.len());
        buf[..n].copy_from_slice(&chunk[..n]);
        if n == chunk.len() {
            self.idx += 1;
        } else {
            self.chunks[self.idx] = chunk[n..].to_vec();
        }
        Ok(n)
    }

    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn handshake_reads_capabilities_in_multiple_chunks() {
    let version_bytes = LATEST_VERSION.to_be_bytes().to_vec();
    let mut transport = ChunkedTransport::new(vec![version_bytes, vec![0], vec![0, 0, 0]]);

    let (codecs, _caps) =
        SshStdioTransport::handshake(&mut transport, &[], &[], None, LATEST_VERSION, CAP_CODECS)
            .expect("handshake");

    assert_eq!(codecs, vec![Codec::Zlib]);
    assert!(transport.finished());
}
