use std::io::{self, Read, Write};
use std::time::Duration;

use crate::{negotiate_version, Demux, Frame, Message, Mux, CAP_CODECS};
use compress::{available_codecs, decode_codecs, encode_codecs, Codec};

/// Server-side protocol state machine.
///
/// The server owns a [`Mux`] and [`Demux`] pair used to multiplex framed
/// messages over an underlying I/O stream. The [`handshake`] method performs
/// version negotiation as defined by the rsync v31 protocol.
pub struct Server<R: Read, W: Write> {
    reader: R,
    writer: W,
    /// Multiplexer for outbound messages.
    pub mux: Mux,
    /// Demultiplexer for inbound frames.
    pub demux: Demux,
    /// Negotiated protocol version.
    pub version: u32,
}

impl<R: Read, W: Write> Server<R, W> {
    /// Create a new server state machine with default keepalive and timeout
    /// durations of 30 seconds.
    pub fn new(reader: R, writer: W) -> Self {
        Server {
            reader,
            writer,
            mux: Mux::new(Duration::from_secs(30)),
            demux: Demux::new(Duration::from_secs(30)),
            version: 0,
        }
    }

    /// Perform the initial version negotiation handshake with a client.
    ///
    /// The client sends a 4 byte big-endian protocol version followed by a
    /// 32-bit capability bitmask. We negotiate the version, echo back the
    /// selected version and capabilities understood by both sides, and then
    /// optionally exchange codec lists if [`CAP_CODECS`] is agreed. The client's
    /// advertised codecs are returned, defaulting to `[Codec::Zlib]` when no
    /// explicit negotiation occurs.
    pub fn handshake(&mut self) -> io::Result<Vec<Codec>> {
        // Consume any environment variables sent by the client. Each entry is
        // a null-terminated string terminated by an additional null byte.
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
        // Peer protocol version
        self.reader.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(peer)?;
        self.writer.write_all(&ver.to_be_bytes())?;
        self.writer.flush()?;
        self.version = ver;

        // Peer capability bitmask
        self.reader.read_exact(&mut buf)?;
        let peer_caps = u32::from_be_bytes(buf);

        // Advertise our capabilities only if the peer signaled support.
        let mut caps = 0u32;
        if peer_caps & CAP_CODECS != 0 {
            caps |= CAP_CODECS;
        }
        self.writer.write_all(&caps.to_be_bytes())?;
        self.writer.flush()?;

        let mut peer_codecs = vec![Codec::Zlib];
        if caps & CAP_CODECS != 0 {
            match Frame::decode(&mut self.reader) {
                Ok(frame) => {
                    let msg = Message::from_frame(frame.clone())?;
                    if let Message::Codecs(buf) = msg {
                        peer_codecs = decode_codecs(&buf)?;
                        let payload = encode_codecs(available_codecs());
                        let frame = Message::Codecs(payload).to_frame(0);
                        frame.encode(&mut self.writer)?;
                        self.writer.flush()?;
                    } else {
                        // Client did not send a codecs message; queue the frame
                        // for later processing.
                        let _ = self.demux.ingest(frame);
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    // Treat missing frame as lack of codec negotiation.
                }
                Err(e) => return Err(e),
            }
        }

        Ok(peer_codecs)
    }

    /// Pump a single iteration of the multiplexed I/O machinery.
    ///
    /// Any queued outbound messages are encoded as frames and written to the
    /// underlying stream. Likewise, an inbound frame (if available) is decoded
    /// and forwarded to the [`Demux`] for delivery to registered receivers.
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
