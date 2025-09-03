// crates/transport/tests/ssh_max_frame.rs
use protocol::{Msg, Tag, CAP_CODECS, LATEST_VERSION};
use std::io;
use transport::{
    ssh::{SshStdioTransport, MAX_FRAME_LEN},
    Transport,
};

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
    fn send(&mut self, _data: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
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
}

#[test]
fn handshake_rejects_oversized_frame() {
    let version_bytes = LATEST_VERSION.to_be_bytes().to_vec();
    let caps_bytes = CAP_CODECS.to_be_bytes().to_vec();

    let len = (MAX_FRAME_LEN + 1) as u32;
    let mut header = Vec::new();
    header.extend_from_slice(&0u16.to_be_bytes());
    header.push(Tag::Message as u8);
    header.push(Msg::Codecs as u8);
    header.extend_from_slice(&len.to_be_bytes());

    let mut transport = ChunkedTransport::new(vec![version_bytes, caps_bytes, header]);

    let res =
        SshStdioTransport::handshake(&mut transport, &[], &[], None, LATEST_VERSION, CAP_CODECS);

    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.to_string().contains("frame length exceeds maximum"),
        "unexpected error: {err}"
    );
    assert!(transport.finished());
}
