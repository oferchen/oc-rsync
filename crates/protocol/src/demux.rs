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
    exit_code: Option<u8>,
    remote_error: Option<String>,
    successes: Vec<u32>,
    deletions: Vec<u32>,
    nosends: Vec<u32>,
    infos: Vec<String>,
    warnings: Vec<String>,
    logs: Vec<String>,
    clients: Vec<String>,
}

impl Demux {
    pub fn new(timeout: Duration) -> Self {
        Demux {
            timeout,
            channels: IndexMap::new(),
            strong_hash: StrongHash::Md4,
            compressor: Codec::Zlib,
            exit_code: None,
            remote_error: None,
            successes: Vec::new(),
            deletions: Vec::new(),
            nosends: Vec::new(),
            infos: Vec::new(),
            warnings: Vec::new(),
            logs: Vec::new(),
            clients: Vec::new(),
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
        let msg = Message::from_frame(frame.clone(), None)?;

        if let Some(text) = msg.error_text() {
            if let Some(ch) = self.channels.get_mut(&id) {
                ch.last_recv = Instant::now();
                let _ = ch.sender.send(msg.clone());
            }
            let err = text.to_string();
            self.remote_error = Some(err.clone());
            return Err(std::io::Error::other(err));
        }

        if id == 0 {
            match &msg {
                Message::Data(payload) if payload.len() == 1 => {
                    let code = payload[0];
                    self.exit_code = Some(code);
                    if code != 0 {
                        return Err(std::io::Error::other(format!("remote exit code {}", code)));
                    } else {
                        return Ok(());
                    }
                }
                Message::ErrorExit(code) => {
                    self.exit_code = Some(*code);
                    return Err(std::io::Error::other(format!("remote exit code {}", code)));
                }
                Message::Success(idx) => {
                    self.successes.push(*idx);
                }
                Message::Deleted(idx) => {
                    self.deletions.push(*idx);
                }
                Message::NoSend(idx) => {
                    self.nosends.push(*idx);
                }
                Message::Info(text) => {
                    self.infos.push(text.clone());
                }
                Message::Warning(text) => {
                    self.warnings.push(text.clone());
                }
                Message::Log(text) => {
                    self.logs.push(text.clone());
                }
                Message::Client(text) => {
                    self.clients.push(text.clone());
                }
                _ => {}
            }
        }

        if let Some(ch) = self.channels.get_mut(&id) {
            ch.last_recv = Instant::now();
            if msg != Message::KeepAlive && msg != Message::Noop {
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

    pub fn take_successes(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.successes)
    }

    pub fn take_deletions(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.deletions)
    }

    pub fn take_nosends(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.nosends)
    }

    pub fn take_infos(&mut self) -> Vec<String> {
        std::mem::take(&mut self.infos)
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        std::mem::take(&mut self.logs)
    }

    pub fn take_clients(&mut self) -> Vec<String> {
        std::mem::take(&mut self.clients)
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
