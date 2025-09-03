// crates/protocol/src/mux.rs
use indexmap::IndexMap;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};
use tracing::warn;

use crate::{ExitCode, Frame, Message};
use checksums::StrongHash;
use compress::Codec;

struct Channel {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    last_sent: Instant,
}

pub struct Mux {
    keepalive: Duration,
    channels: IndexMap<u16, Channel>,
    next: usize,
    pub strong_hash: StrongHash,
    pub compressor: Codec,
}

impl Mux {
    pub fn new(keepalive: Duration) -> Self {
        Mux {
            keepalive,
            channels: IndexMap::new(),
            next: 0,
            strong_hash: StrongHash::Md4,
            compressor: Codec::Zlib,
        }
    }

    pub fn register_channel(&mut self, id: u16) -> Sender<Message> {
        let (tx, rx) = mpsc::channel();
        let ch = Channel {
            sender: tx.clone(),
            receiver: rx,
            last_sent: Instant::now(),
        };
        self.channels.insert(id, ch);
        tx
    }

    pub fn send(&self, id: u16, msg: Message) -> Result<(), mpsc::SendError<Message>> {
        if let Some(ch) = self.channels.get(&id) {
            ch.sender.send(msg)
        } else {
            Err(mpsc::SendError(msg))
        }
    }

    pub fn send_exit_code(&self, code: ExitCode) -> Result<(), mpsc::SendError<Message>> {
        let byte: u8 = code.into();
        self.send(0, Message::Exit(byte))
    }

    pub fn send_error<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Error(text.into()))
    }

    pub fn send_error_xfer<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::ErrorXfer(text.into()))
    }

    pub fn send_info<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Info(text.into()))
    }

    pub fn send_warning<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Warning(text.into()))
    }

    pub fn send_error_socket<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::ErrorSocket(text.into()))
    }

    pub fn send_error_utf8<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::ErrorUtf8(text.into()))
    }

    pub fn send_log<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Log(text.into()))
    }

    pub fn send_client<S: Into<String>>(
        &self,
        id: u16,
        text: S,
    ) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Client(text.into()))
    }

    pub fn send_progress(&self, id: u16, val: u64) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Progress(val))
    }

    pub fn send_xattrs(&self, id: u16, data: Vec<u8>) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Xattrs(data))
    }

    pub fn send_attrs(&self, id: u16, data: Vec<u8>) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Attributes(data))
    }

    pub fn send_success(&self, id: u16, idx: u32) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Success(idx))
    }

    pub fn send_deleted(&self, id: u16, idx: u32) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Deleted(idx))
    }

    pub fn send_no_send(&self, id: u16, idx: u32) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::NoSend(idx))
    }

    pub fn send_redo(&self, id: u16, idx: u32) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Redo(idx))
    }

    pub fn send_stats(&self, id: u16, data: Vec<u8>) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::Stats(data))
    }

    pub fn send_io_error(&self, id: u16, val: u32) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::IoError(val))
    }

    pub fn send_io_timeout(&self, id: u16, val: u32) -> Result<(), mpsc::SendError<Message>> {
        self.send(id, Message::IoTimeout(val))
    }

    pub fn unregister_channel(&mut self, id: u16) {
        if self.channels.swap_remove(&id).is_some() && self.next >= self.channels.len() {
            self.next = 0;
        }
    }

    pub fn poll(&mut self) -> Option<Frame> {
        let now = Instant::now();

        if self.channels.is_empty() {
            return None;
        }

        let len = self.channels.len();
        for offset in 0..len {
            let idx = (self.next + offset) % len;
            let Some((id, ch)) = self.channels.get_index_mut(idx) else {
                warn!("channel index {idx} missing during poll");
                continue;
            };
            let id = *id;
            match ch.receiver.try_recv() {
                Ok(msg) => {
                    ch.last_sent = now;
                    self.next = (idx + 1) % len;
                    return Some(msg.into_frame(id, None));
                }
                Err(TryRecvError::Empty) => {
                    if now.duration_since(ch.last_sent) >= self.keepalive {
                        ch.last_sent = now;
                        self.next = (idx + 1) % len;
                        return Some(Message::KeepAlive.into_frame(id, None));
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    if now.duration_since(ch.last_sent) >= self.keepalive {
                        ch.last_sent = now;
                        self.next = (idx + 1) % len;
                        return Some(Message::KeepAlive.into_frame(id, None));
                    }
                }
            }
        }

        None
    }
}
