use std::io::{self, Read, Write};

/// Trait representing a blocking transport.
pub trait Transport {
    /// Send data over the transport.
    fn send(&mut self, data: &[u8]) -> io::Result<()>;

    /// Receive data from the transport into the provided buffer.
    ///
    /// Returns the number of bytes read.
    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize>;
}

/// Transport implementation over local pipes using blocking I/O.
pub struct LocalPipeTransport<R, W> {
    reader: R,
    writer: W,
}

impl<R, W> LocalPipeTransport<R, W> {
    /// Create a new transport from the given reader and writer.
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }

    /// Consume the transport and return the underlying reader and writer.
    pub fn into_inner(self) -> (R, W) {
        (self.reader, self.writer)
    }
}

impl<R: Read, W: Write> Transport for LocalPipeTransport<R, W> {
    fn send(&mut self, data: &[u8]) -> io::Result<()> {
        self.writer.write_all(data)
    }

    fn receive(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }
}

/// Marker trait for transports carried over SSH.
pub trait SshTransport: Transport {}

/// Marker trait for transports that connect to an rsync daemon.
pub trait DaemonTransport: Transport {}
