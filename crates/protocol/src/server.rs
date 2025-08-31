// crates/protocol/src/server.rs
use std::io::{self, Read, Write};
use std::time::Duration;

use crate::{negotiate_version, Demux, Frame, Message, Mux, CAP_CODECS};
use compress::{decode_codecs, encode_codecs, Codec};

pub struct Server<R: Read, W: Write> {
    reader: R,
    writer: W,
    pub mux: Mux,
    pub demux: Demux,
    pub version: u32,
    pub caps: u32,
}

impl<R: Read, W: Write> Server<R, W> {
    pub fn new(reader: R, writer: W, timeout: Duration) -> Self {
        Server {
            reader,
            writer,
            mux: Mux::new(timeout),
            demux: Demux::new(timeout),
            version: 0,
            caps: 0,
        }
    }

    pub fn handshake(
        &mut self,
        version: u32,
        caps: u32,
        codecs: &[Codec],
    ) -> io::Result<(u32, Vec<Codec>)> {
        let mut b = [0u8; 1];
        let mut cur = Vec::new();
        loop {
            self.reader.read_exact(&mut b)?;
            if b[0] == 0 {
                if cur.is_empty() {
                    break;
                }
                cur.clear();
            } else {
                cur.push(b[0]);
            }
        }

        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(version, peer)?;
        self.writer.write_all(&ver.to_be_bytes())?;
        self.writer.flush()?;
        self.version = ver;

        self.reader.read_exact(&mut buf)?;
        let peer_caps = u32::from_be_bytes(buf);

        let negotiated = peer_caps & caps;
        self.caps = negotiated;
        self.writer.write_all(&caps.to_be_bytes())?;
        self.writer.flush()?;

        let mut peer_codecs = vec![Codec::Zlib];
        if self.caps & CAP_CODECS != 0 {
            match Frame::decode(&mut self.reader) {
                Ok(frame) => {
                    let msg = Message::from_frame(frame.clone())?;
                    if let Message::Codecs(buf) = msg {
                        peer_codecs = decode_codecs(&buf)?;
                        let payload = encode_codecs(codecs);
                        let frame = Message::Codecs(payload).to_frame(0);
                        frame.encode(&mut self.writer)?;
                        self.writer.flush()?;
                    } else {
                        let _ = self.demux.ingest(frame);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
                Err(e) => return Err(e),
            }
        }

        Ok((self.caps, peer_codecs))
    }

    pub fn pump(&mut self) -> io::Result<()> {
        if let Some(frame) = self.mux.poll() {
            frame.encode(&mut self.writer)?;
        }

        match Frame::decode(&mut self.reader) {
            Ok(frame) => self.demux.ingest(frame),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(()),
            Err(e) => Err(e),
        }
    }
}
