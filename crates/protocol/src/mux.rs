// crates/protocol/src/mux.rs
use indexmap::IndexMap;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use crate::{Frame, Message};
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
    pub cdc: bool,
}

impl Mux {
    pub fn new(keepalive: Duration) -> Self {
        Mux {
            keepalive,
            channels: IndexMap::new(),
            next: 0,
            strong_hash: StrongHash::Md5,
            compressor: Codec::Zlib,
            cdc: false,
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

    pub fn poll(&mut self) -> Option<Frame> {
        let now = Instant::now();

        if self.channels.is_empty() {
            return None;
        }

        let len = self.channels.len();
        for offset in 0..len {
            let idx = (self.next + offset) % len;
            let (id, ch) = self.channels.get_index_mut(idx).expect("index in range");
            let id = *id;
            match ch.receiver.try_recv() {
                Ok(msg) => {
                    ch.last_sent = now;
                    self.next = (idx + 1) % len;
                    return Some(msg.into_frame(id));
                }
                Err(TryRecvError::Empty) => {
                    if now.duration_since(ch.last_sent) >= self.keepalive {
                        ch.last_sent = now;
                        self.next = (idx + 1) % len;
                        return Some(Message::KeepAlive.into_frame(id));
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    if now.duration_since(ch.last_sent) >= self.keepalive {
                        ch.last_sent = now;
                        self.next = (idx + 1) % len;
                        return Some(Message::KeepAlive.into_frame(id));
                    }
                }
            }
        }

        None
    }
}
