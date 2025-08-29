use std::io::{self, Read, Write};
use std::time::Duration;

use crate::{negotiate_version, Demux, Frame, Message, Mux};
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

    /// Perform the initial version negotiation handshake with a client. The
    /// client is expected to send a 4 byte big-endian protocol version which we
    /// negotiate against our supported range and reply with the selected
    /// version.  After version negotiation, both peers exchange lists of
    /// supported compression codecs. The client's advertised codecs are
    /// returned.
    pub fn handshake(&mut self) -> io::Result<Vec<Codec>> {
        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        let peer = u32::from_be_bytes(buf);
        let ver = negotiate_version(peer)?;
        self.writer.write_all(&ver.to_be_bytes())?;
        self.writer.flush()?;
        self.version = ver;

        // Read the client's advertised codecs message.
        let frame = Frame::decode(&mut self.reader)?;
        let msg = Message::from_frame(frame)?;
        let peer_codecs = match msg {
            Message::Codecs(buf) => decode_codecs(&buf)?,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "expected codecs message",
                ))
            }
        };

        // Respond with our own codec list via a codecs message.
        let payload = encode_codecs(available_codecs());
        let frame = Message::Codecs(payload).to_frame(0);
        frame.encode(&mut self.writer)?;
        self.writer.flush()?;

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
