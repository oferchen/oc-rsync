// crates/protocol/src/demux.rs
use indexmap::IndexMap;
use std::collections::VecDeque;
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
    msg_capacity: usize,
    exit_code: Option<u8>,
    remote_error: Option<String>,
    error_xfers: VecDeque<String>,
    errors: VecDeque<String>,
    error_sockets: VecDeque<String>,
    error_utf8s: VecDeque<String>,
    successes: VecDeque<u32>,
    deletions: VecDeque<u32>,
    nosends: VecDeque<u32>,
    infos: VecDeque<String>,
    warnings: VecDeque<String>,
    logs: VecDeque<String>,
    clients: VecDeque<String>,
    progress: VecDeque<u64>,
    stats: VecDeque<Vec<u8>>,
}

pub const RSYNC_MSG_LIMIT: usize = usize::MAX;

impl Demux {
    pub fn new(timeout: Duration) -> Self {
        Self::with_capacity(timeout, RSYNC_MSG_LIMIT)
    }

    pub fn with_capacity(timeout: Duration, msg_capacity: usize) -> Self {
        Demux {
            timeout,
            channels: IndexMap::new(),
            strong_hash: StrongHash::Md4,
            compressor: Codec::Zlib,
            msg_capacity,
            exit_code: None,
            remote_error: None,
            error_xfers: VecDeque::new(),
            errors: VecDeque::new(),
            error_sockets: VecDeque::new(),
            error_utf8s: VecDeque::new(),
            successes: VecDeque::new(),
            deletions: VecDeque::new(),
            nosends: VecDeque::new(),
            infos: VecDeque::new(),
            warnings: VecDeque::new(),
            logs: VecDeque::new(),
            clients: VecDeque::new(),
            progress: VecDeque::new(),
            stats: VecDeque::new(),
        }
    }

    fn push_limited<T>(buf: &mut VecDeque<T>, val: T, cap: usize) {
        if buf.len() >= cap {
            buf.pop_front();
        }
        buf.push_back(val);
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
        let msg = Message::from_frame(frame, None)?;
        self.ingest_message(id, msg)
    }

    pub fn ingest_message(&mut self, id: u16, msg: Message) -> std::io::Result<()> {
        match &msg {
            Message::ErrorXfer(text) => {
                Self::push_limited(&mut self.error_xfers, text.clone(), self.msg_capacity);
                if self.remote_error.is_none() {
                    self.remote_error = Some(text.clone());
                }
            }
            Message::Error(text) => {
                Self::push_limited(&mut self.errors, text.clone(), self.msg_capacity);
                if self.remote_error.is_none() {
                    self.remote_error = Some(text.clone());
                }
            }
            Message::ErrorSocket(text) => {
                Self::push_limited(&mut self.error_sockets, text.clone(), self.msg_capacity);
                if self.remote_error.is_none() {
                    self.remote_error = Some(text.clone());
                }
            }
            Message::ErrorUtf8(text) => {
                Self::push_limited(&mut self.error_utf8s, text.clone(), self.msg_capacity);
                if self.remote_error.is_none() {
                    self.remote_error = Some(text.clone());
                }
            }
            _ => {}
        }

        if id == 0 {
            match &msg {
                Message::Exit(code) => {
                    self.exit_code = Some(*code);
                    if *code != 0 {
                        return Err(std::io::Error::other(format!("remote exit code {}", code)));
                    } else {
                        return Ok(());
                    }
                }
                Message::Success(idx) => {
                    Self::push_limited(&mut self.successes, *idx, self.msg_capacity);
                }
                Message::Deleted(idx) => {
                    Self::push_limited(&mut self.deletions, *idx, self.msg_capacity);
                }
                Message::NoSend(idx) => {
                    Self::push_limited(&mut self.nosends, *idx, self.msg_capacity);
                }
                Message::Info(text) => {
                    Self::push_limited(&mut self.infos, text.clone(), self.msg_capacity);
                }
                Message::Warning(text) => {
                    Self::push_limited(&mut self.warnings, text.clone(), self.msg_capacity);
                }
                Message::Log(text) => {
                    Self::push_limited(&mut self.logs, text.clone(), self.msg_capacity);
                }
                Message::Client(text) => {
                    Self::push_limited(&mut self.clients, text.clone(), self.msg_capacity);
                }
                Message::Progress(val) => {
                    Self::push_limited(&mut self.progress, *val, self.msg_capacity);
                }
                Message::Stats(data) => {
                    Self::push_limited(&mut self.stats, data.clone(), self.msg_capacity);
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

    pub fn take_error_xfers(&mut self) -> Vec<String> {
        self.error_xfers.drain(..).collect()
    }

    pub fn take_errors(&mut self) -> Vec<String> {
        self.errors.drain(..).collect()
    }

    pub fn take_error_sockets(&mut self) -> Vec<String> {
        self.error_sockets.drain(..).collect()
    }

    pub fn take_error_utf8s(&mut self) -> Vec<String> {
        self.error_utf8s.drain(..).collect()
    }

    pub fn take_successes(&mut self) -> Vec<u32> {
        self.successes.drain(..).collect()
    }

    pub fn take_deletions(&mut self) -> Vec<u32> {
        self.deletions.drain(..).collect()
    }

    pub fn take_nosends(&mut self) -> Vec<u32> {
        self.nosends.drain(..).collect()
    }

    pub fn take_infos(&mut self) -> Vec<String> {
        self.infos.drain(..).collect()
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        self.warnings.drain(..).collect()
    }

    pub fn take_logs(&mut self) -> Vec<String> {
        self.logs.drain(..).collect()
    }

    pub fn take_clients(&mut self) -> Vec<String> {
        self.clients.drain(..).collect()
    }

    pub fn take_progress(&mut self) -> Vec<u64> {
        self.progress.drain(..).collect()
    }

    pub fn take_stats(&mut self) -> Vec<Vec<u8>> {
        self.stats.drain(..).collect()
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
