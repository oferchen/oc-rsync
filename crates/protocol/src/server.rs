// crates/protocol/src/server.rs
use std::io::{self, Read, Write};
use std::time::Duration;

#[cfg(feature = "blake3")]
use crate::CAP_BLAKE3;
use crate::{
    negotiate_version, Demux, Frame, Message, Mux, CAP_BLAKE2, CAP_CDC, CAP_CODECS, CAP_LZ4,
    CAP_ZSTD,
};
use checksums::StrongHash;
use compress::{decode_codecs, encode_codecs, negotiate_codec, Codec};

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

        self.writer.write_all(&caps.to_be_bytes())?;
        self.writer.flush()?;
        self.reader.read_exact(&mut buf)?;
        let peer_caps = u32::from_be_bytes(buf);
        self.caps = peer_caps & caps;

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
                        self.demux.ingest(frame)?;
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
                Err(e) => return Err(e),
            }
        }

        #[cfg(feature = "blake3")]
        if self.caps & CAP_BLAKE3 != 0 {
            self.mux.strong_hash = StrongHash::Blake3;
            self.demux.strong_hash = StrongHash::Blake3;
        } else if self.caps & CAP_BLAKE2 != 0 {
            self.mux.strong_hash = StrongHash::Blake2b;
            self.demux.strong_hash = StrongHash::Blake2b;
        }
        #[cfg(not(feature = "blake3"))]
        if self.caps & CAP_BLAKE2 != 0 {
            self.mux.strong_hash = StrongHash::Blake2b;
            self.demux.strong_hash = StrongHash::Blake2b;
        }

        let mut selected = Codec::Zlib;
        if self.caps & CAP_CODECS != 0 {
            if let Some(codec) = negotiate_codec(codecs, &peer_codecs) {
                match codec {
                    Codec::Zstd if self.caps & CAP_ZSTD != 0 => selected = Codec::Zstd,
                    Codec::Lz4 if self.caps & CAP_LZ4 != 0 => selected = Codec::Lz4,
                    _ => {}
                }
            }
        } else if self.caps & CAP_ZSTD != 0 {
            selected = Codec::Zstd;
        } else if self.caps & CAP_LZ4 != 0 {
            selected = Codec::Lz4;
        }
        self.mux.compressor = selected;
        self.demux.compressor = selected;
        self.mux.cdc = self.caps & CAP_CDC != 0;
        self.demux.cdc = self.caps & CAP_CDC != 0;

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
