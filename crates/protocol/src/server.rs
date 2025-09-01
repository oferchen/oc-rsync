// crates/protocol/src/server.rs
use std::io::{self, Read, Write};
use std::time::Duration;

use crate::{negotiate_version, Demux, Frame, Message, Mux, CAP_CODECS, CAP_ZSTD};
use compress::{decode_codecs, encode_codecs, negotiate_codec, Codec};

pub struct Server<R: Read, W: Write> {
    reader: R,
    writer: W,
    pub mux: Mux,
    pub demux: Demux,
    pub version: u32,
    pub caps: u32,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
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
            args: Vec::new(),
            env: Vec::new(),
        }
    }

    pub fn handshake(
        &mut self,
        version: u32,
        caps: u32,
        codecs: &[Codec],
    ) -> io::Result<(u32, Vec<Codec>)> {
        self.args.clear();
        self.env.clear();
        let mut b = [0u8; 1];
        let mut cur = Vec::new();
        let mut in_env = false;
        loop {
            self.reader.read_exact(&mut b)?;
            if b[0] == 0 {
                if cur.is_empty() {
                    break;
                }
                let s = String::from_utf8(cur.clone())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if !in_env && s.contains('=') && !s.starts_with('-') {
                    in_env = true;
                }
                if in_env {
                    let mut parts = s.splitn(2, '=');
                    let k = parts.next().unwrap_or_default().to_string();
                    let v = parts.next().unwrap_or_default().to_string();
                    self.env.push((k, v));
                } else {
                    self.args.push(s);
                }
                cur.clear();
            } else {
                cur.push(b[0]);
            }
        }

        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        self.writer.write_all(&version.to_be_bytes())?;
        self.writer.flush()?;
        let ver = negotiate_version(version, peer)?;
        self.version = ver;

        self.reader.read_exact(&mut buf)?;
        let peer_caps = u32::from_be_bytes(buf);
        self.writer.write_all(&caps.to_be_bytes())?;
        self.writer.flush()?;
        self.caps = peer_caps & caps;

        let mut peer_codecs = vec![Codec::Zlib];
        if self.caps & CAP_CODECS != 0 {
            match Frame::decode(&mut self.reader) {
                Ok(frame) => {
                    let msg = Message::from_frame(frame.clone(), None)?;
                    if let Message::Codecs(buf) = msg {
                        peer_codecs = decode_codecs(&buf)?;
                        let payload = encode_codecs(codecs);
                        let frame = Message::Codecs(payload).to_frame(0, None);
                        frame.encode(&mut self.writer)?;
                        self.writer.flush()?;
                    } else {
                        self.demux.ingest(frame)?;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
                Err(e) => return Err(e),
            }
        }

        let mut selected = Codec::Zlib;
        if self.caps & CAP_CODECS != 0 {
            if let Some(codec) = negotiate_codec(codecs, &peer_codecs) {
                if codec == Codec::Zstd && self.caps & CAP_ZSTD != 0 {
                    selected = Codec::Zstd;
                }
            }
        } else if self.caps & CAP_ZSTD != 0 {
            selected = Codec::Zstd;
        }
        self.mux.compressor = selected;
        self.demux.compressor = selected;

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
