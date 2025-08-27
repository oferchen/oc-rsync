use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use crate::{Frame, Message};

struct Channel {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    last_sent: Instant,
}

/// Multiplex messages from multiple channels into a single frame stream.
///
/// Each registered channel is associated with a [`Sender`] returned by
/// [`Mux::register_channel`]. Messages sent through these senders are converted
/// into [`Frame`]s when [`Mux::poll`] is invoked. If a channel is idle for
/// longer than the configured keepalive interval a [`Message::KeepAlive`] frame
/// is emitted.
pub struct Mux {
    keepalive: Duration,
    channels: HashMap<u16, Channel>,
}

impl Mux {
    /// Create a new multiplexer with the specified keepalive interval.
    pub fn new(keepalive: Duration) -> Self {
        Mux {
            keepalive,
            channels: HashMap::new(),
        }
    }

    /// Register a new channel and return a [`Sender`] for queuing messages.
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

    /// Queue a message to be sent on the given channel.
    pub fn send(&self, id: u16, msg: Message) -> Result<(), mpsc::SendError<Message>> {
        if let Some(ch) = self.channels.get(&id) {
            ch.sender.send(msg)
        } else {
            Err(mpsc::SendError(msg))
        }
    }

    /// Poll all registered channels for outgoing frames. The first available
    /// message is converted into a [`Frame`] and returned. If no messages are
    /// pending, idle channels may generate keepalive frames.
    pub fn poll(&mut self) -> Option<Frame> {
        let now = Instant::now();

        for (&id, ch) in self.channels.iter_mut() {
            match ch.receiver.try_recv() {
                Ok(msg) => {
                    ch.last_sent = now;
                    return Some(msg.into_frame(id));
                }
                Err(TryRecvError::Empty) => {
                    if now.duration_since(ch.last_sent) >= self.keepalive {
                        ch.last_sent = now;
                        return Some(Message::KeepAlive.into_frame(id));
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    // If the sender has gone away, treat as idle and emit
                    // keepalives until the channel is dropped.
                    if now.duration_since(ch.last_sent) >= self.keepalive {
                        ch.last_sent = now;
                        return Some(Message::KeepAlive.into_frame(id));
                    }
                }
            }
        }

        None
    }
}
