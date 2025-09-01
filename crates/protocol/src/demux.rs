// crates/protocol/src/demux.rs
use indexmap::IndexMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use crate::{ExitCode, Frame, Message, UnknownExit};
use checksums::StrongHash;
use compress::Codec;

struct Channel {
    sender: Sender<Message>,
    last_recv: Instant,
}

pub struct Demux {
    timeout: Duration,
    channels: IndexMap<u16, Channel>,
    pub strong_hash: StrongHash,
    pub compressor: Codec,
    pub cdc: bool,
    exit_code: Option<u8>,
    remote_error: Option<String>,
}

impl Demux {
    pub fn new(timeout: Duration) -> Self {
        Demux {
            timeout,
            channels: IndexMap::new(),
            strong_hash: StrongHash::Md5,
            compressor: Codec::Zlib,
            cdc: false,
            exit_code: None,
            remote_error: None,
        }
    }

    pub fn register_channel(&mut self, id: u16) -> Receiver<Message> {
        let (tx, rx) = mpsc::channel();
        let ch = Channel {
            sender: tx,
            last_recv: Instant::now(),
        };
        self.channels.insert(id, ch);
        rx
    }

    pub fn unregister_channel(&mut self, id: u16) {
        if let Some(ch) = self.channels.swap_remove(&id) {
            drop(ch);
        }
    }

    pub fn ingest(&mut self, frame: Frame) -> std::io::Result<()> {
        let id = frame.header.channel;
        let msg = Message::from_frame(frame.clone())?;

        if let Message::Error(text) = &msg {
            if let Some(ch) = self.channels.get_mut(&id) {
                ch.last_recv = Instant::now();
                let _ = ch.sender.send(msg.clone());
            }
            self.remote_error = Some(text.clone());
            return Err(std::io::Error::other(text.clone()));
        }

        if id == 0 {
            if let Message::Data(payload) = &msg {
                if payload.len() == 1 {
                    let code = payload[0];
                    self.exit_code = Some(code);
                    if code != 0 {
                        return Err(std::io::Error::other(format!("remote exit code {}", code)));
                    } else {
                        return Ok(());
                    }
                }
            }
        }

        if let Some(ch) = self.channels.get_mut(&id) {
            ch.last_recv = Instant::now();
            if msg != Message::KeepAlive {
                ch.sender.send(msg).map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::BrokenPipe, "channel closed")
                })?;
            }
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "unknown channel",
            ))
        }
    }

    pub fn take_exit_code(&mut self) -> Option<Result<ExitCode, UnknownExit>> {
        self.exit_code.take().map(ExitCode::try_from)
    }

    pub fn take_remote_error(&mut self) -> Option<String> {
        self.remote_error.take()
    }

    pub fn poll(&mut self) -> std::io::Result<()> {
        let now = Instant::now();
        for (&id, ch) in self.channels.iter() {
            if now.duration_since(ch.last_recv) > self.timeout {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("channel {} timed out", id),
                ));
            }
        }
        Ok(())
    }
}
