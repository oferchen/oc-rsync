// crates/protocol/src/server.rs
use std::io::{self, Read, Write};
use std::time::Duration;

use checksums::{strong_digest, StrongHash};
use subtle::ConstantTimeEq;

use crate::{
    negotiate_caps, negotiate_version, Demux, ExitCode, Frame, Message, Mux, UnknownExit,
    CAP_CODECS, CAP_ZSTD, V30,
};
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
    pub max_args: usize,
    pub max_arg_len: usize,
    pub max_env_vars: usize,
    pub max_env_len: usize,
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
            max_args: 1024,
            max_arg_len: 16 * 1024,
            max_env_vars: 1024,
            max_env_len: 16 * 1024,
        }
    }

    pub fn handshake(
        &mut self,
        version: u32,
        caps: u32,
        codecs: &[Codec],
        token: Option<&str>,
    ) -> io::Result<(u32, Vec<Codec>)> {
        self.args.clear();
        self.env.clear();
        let mut b = [0u8; 1];
        let mut cur = Vec::new();
        let mut saw_nonopt = false;

        loop {
            self.reader.read_exact(&mut b)?;
            if b[0] == 0 {
                if cur.is_empty() {
                    break;
                }
                if cur.len() > self.max_arg_len {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "argument too long",
                    ));
                }
                let s = String::from_utf8(cur.clone())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if s.starts_with('-') {
                    if saw_nonopt {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "option after argument",
                        ));
                    }
                } else {
                    saw_nonopt = true;
                }
                self.args.push(s);
                if self.args.len() > self.max_args {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "too many arguments",
                    ));
                }
                cur.clear();
            } else {
                cur.push(b[0]);
                if cur.len() > self.max_arg_len {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "argument too long",
                    ));
                }
            }
        }

        loop {
            self.reader.read_exact(&mut b)?;
            if b[0] == 0 {
                if cur.is_empty() {
                    break;
                }
                if cur.len() > self.max_env_len {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "environment variable too long",
                    ));
                }
                let s = String::from_utf8(cur.clone())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                if !s.contains('=') || s.starts_with('=') {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "invalid environment variable",
                    ));
                }
                let mut parts = s.splitn(2, '=');
                let k = parts.next().unwrap_or_default().to_string();
                let v = parts.next().unwrap_or_default().to_string();
                self.env.push((k, v));
                if self.env.len() > self.max_env_vars {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "too many environment variables",
                    ));
                }
                cur.clear();
            } else {
                cur.push(b[0]);
                if cur.len() > self.max_env_len {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "environment variable too long",
                    ));
                }
            }
        }

        if let Some(tok) = token {
            const CHALLENGE: &[u8; 16] = b"0123456789abcdef";
            self.writer.write_all(CHALLENGE)?;
            self.writer.flush()?;
            let mut resp = [0u8; 16];
            self.reader.read_exact(&mut resp)?;
            let mut buf = Vec::new();
            buf.extend_from_slice(CHALLENGE);
            buf.extend_from_slice(tok.as_bytes());
            let expected = strong_digest(&buf, StrongHash::Md5, 0);
            let expected_arr: [u8; 16] =
                expected[..16].try_into().expect("MD5 digests are 16 bytes");
            if expected_arr.ct_eq(&resp).unwrap_u8() != 1 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "invalid challenge response",
                ));
            }
        }

        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(version, peer)?;
        self.version = ver;
        self.writer.write_all(&ver.to_be_bytes())?;
        self.writer.flush()?;

        self.reader.read_exact(&mut buf)?;
        let peer_caps = u32::from_be_bytes(buf);
        self.writer.write_all(&caps.to_be_bytes())?;
        self.writer.flush()?;
        self.caps = negotiate_caps(caps, peer_caps);

        let mut peer_codecs = vec![Codec::Zlib];
        if self.caps & CAP_CODECS != 0 {
            match Frame::decode(&mut self.reader) {
                Ok(frame) => {
                    let id = frame.header.channel;
                    let msg = Message::from_frame(frame, None)?;
                    if let Message::Codecs(buf) = msg {
                        peer_codecs = decode_codecs(&buf)?;
                        let payload = encode_codecs(codecs);
                        let frame = Message::Codecs(payload).to_frame(0, None);
                        frame.encode(&mut self.writer)?;
                        self.writer.flush()?;
                    } else {
                        self.demux.ingest_message(id, msg)?;
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
                } else {
                    selected = codec;
                }
            }
        } else if self.caps & CAP_ZSTD != 0 {
            selected = Codec::Zstd;
        }
        self.mux.compressor = selected;
        self.demux.compressor = selected;

        let strong =
            if let Some((_, list)) = self.env.iter().find(|(k, _)| k == "RSYNC_CHECKSUM_LIST") {
                let mut chosen = if self.version < V30 {
                    StrongHash::Md4
                } else {
                    StrongHash::Md5
                };
                for name in list.split(',') {
                    match name {
                        "sha1" => {
                            chosen = StrongHash::Sha1;
                            break;
                        }
                        "md5" => {
                            chosen = StrongHash::Md5;
                            break;
                        }
                        "md4" => {
                            chosen = StrongHash::Md4;
                            break;
                        }
                        _ => {}
                    }
                }
                chosen
            } else if self.version < V30 {
                StrongHash::Md4
            } else {
                StrongHash::Md5
            };
        self.mux.strong_hash = strong;
        self.demux.strong_hash = strong;

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

    pub fn take_exit_code(&mut self) -> Option<Result<ExitCode, UnknownExit>> {
        self.demux.take_exit_code()
    }

    pub fn take_remote_error(&mut self) -> Option<String> {
        self.demux.take_remote_error()
    }

    pub fn take_error_xfers(&mut self) -> Vec<String> {
        self.demux.take_error_xfers()
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        self.demux.take_errors()
    }

    pub fn take_error_sockets(&mut self) -> Vec<String> {
        self.demux.take_error_sockets()
    }

    pub fn take_error_utf8s(&mut self) -> Vec<String> {
        self.demux.take_error_utf8s()
    }

    pub fn take_successes(&mut self) -> Vec<u32> {
        self.demux.take_successes()
    }

    pub fn take_deletions(&mut self) -> Vec<u32> {
        self.demux.take_deletions()
    }

    pub fn take_nosends(&mut self) -> Vec<u32> {
        self.demux.take_nosends()
    }

    pub fn take_infos(&mut self) -> Vec<String> {
        self.demux.take_infos()
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        self.demux.take_warnings()
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        self.demux.take_logs()
    }

    pub fn take_clients(&mut self) -> Vec<String> {
        self.demux.take_clients()
    }

    pub fn take_progress(&mut self) -> Vec<u64> {
        self.demux.take_progress()
    }

    pub fn take_stats(&mut self) -> Vec<Vec<u8>> {
        self.demux.take_stats()
    }
}
