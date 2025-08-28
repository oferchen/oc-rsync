use indexmap::IndexMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use crate::{Frame, Message};

struct Channel {
    sender: Sender<Message>,
    last_recv: Instant,
}

/// Demultiplex incoming frames to per-channel message queues and monitor peer
/// liveness.
pub struct Demux {
    timeout: Duration,
    channels: IndexMap<u16, Channel>,
}

impl Demux {
    /// Create a new demultiplexer with the specified peer timeout.
    pub fn new(timeout: Duration) -> Self {
        Demux {
            timeout,
            channels: IndexMap::new(),
        }
    }

    /// Register a channel and obtain a [`Receiver`] for reading decoded
    /// messages.
    pub fn register_channel(&mut self, id: u16) -> Receiver<Message> {
        let (tx, rx) = mpsc::channel();
        let ch = Channel {
            sender: tx,
            last_recv: Instant::now(),
        };
        self.channels.insert(id, ch);
        rx
    }

    /// Unregister a previously registered channel.
    ///
    /// Dropping the channel discards any messages that have been queued but not yet
    /// received. Any subsequent frames targeting this channel will be rejected by
    /// [`Demux::ingest`].
    pub fn unregister_channel(&mut self, id: u16) {
        // `IndexMap::remove` is deprecated as it invalidates ordering; `swap_remove`
        // performs removal without shifting all elements and is sufficient here
        // since channel ordering is not significant.
        if let Some(ch) = self.channels.swap_remove(&id) {
            // Dropping the sender ensures pending messages are not delivered.
            drop(ch);
        }
    }

    /// Process an incoming [`Frame`]. Keep-alive frames simply refresh the
    /// channel's activity timestamp while other messages are forwarded to the
    /// appropriate receiver.
    pub fn ingest(&mut self, frame: Frame) -> std::io::Result<()> {
        let id = frame.header.channel;
        let msg = Message::from_frame(frame)?;

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

    /// Check for channels that have not received any frames within the timeout
    /// period. Returns an error if a timeout is detected.
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
