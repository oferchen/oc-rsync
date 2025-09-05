// tests/rsync_zlib.rs

use compress::Codec;
use protocol::{CAP_CODECS, LATEST_VERSION, negotiate_version};
use std::io;
use transport::Transport;

struct MockTransport {
    reads: Vec<Vec<u8>>,
    idx: usize,
}

impl Transport for MockTransport {
    fn send(&mut self, _: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let data = &self.reads[self.idx];
        self.idx += 1;
        buf[..data.len()].copy_from_slice(data);
        Ok(data.len())
    }

    fn close(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn rsync_client_falls_back_to_zlib() {
    let mut t = MockTransport {
        reads: vec![
            LATEST_VERSION.to_be_bytes().to_vec(),
            0u32.to_be_bytes().to_vec(),
        ],
        idx: 0,
    };

    t.send(&LATEST_VERSION.to_be_bytes()).unwrap();
    let mut buf = [0u8; 4];
    t.receive(&mut buf).unwrap();
    negotiate_version(LATEST_VERSION, u32::from_be_bytes(buf)).unwrap();

    t.send(&CAP_CODECS.to_be_bytes()).unwrap();
    t.receive(&mut buf).unwrap();
    let caps = u32::from_be_bytes(buf);
    assert_eq!(caps & CAP_CODECS, 0);

    let negotiated = vec![Codec::Zlib];
    assert_eq!(negotiated, vec![Codec::Zlib]);
}
