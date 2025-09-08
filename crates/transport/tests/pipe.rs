// crates/transport/tests/pipe.rs
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use transport::{
    LocalPipeTransport, SshStdioTransport, TcpTransport, TimeoutTransport, Transport,
    TransportConfig, pipe,
};

fn wait_for<F: Fn() -> bool>(cond: F) {
    let start = std::time::Instant::now();
    while !cond() {
        if start.elapsed() > std::time::Duration::from_secs(1) {
            panic!("timed out waiting for condition");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn pipe_ssh_transports() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"ssh_remote").unwrap();

    let mut src_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat {}", src.display())]).unwrap();
    let mut dst_session =
        SshStdioTransport::spawn("sh", ["-c", &format!("cat > {}", dst.display())]).unwrap();

    let _ = pipe(&mut src_session, &mut dst_session).unwrap();
    drop(dst_session);
    drop(src_session);
    wait_for(|| dst.exists());
    let out = fs::read(dst).unwrap();
    assert_eq!(out, b"ssh_remote");
}

#[test]
fn pipe_tcp_transports() {
    let dir = tempdir().unwrap();
    let dst = dir.path().join("copy.txt");
    let data = b"daemon_remote".to_vec();

    let src_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let src_port = src_listener.local_addr().unwrap().port();
    let src_handle = thread::spawn(move || {
        let (mut stream, _) = src_listener.accept().unwrap();
        stream.write_all(&data).unwrap();
    });

    let dst_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let dst_port = dst_listener.local_addr().unwrap().port();
    let dst_file = dst.clone();
    let dst_handle = thread::spawn(move || {
        let (mut stream, _) = dst_listener.accept().unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        fs::write(dst_file, buf).unwrap();
    });

    let mut src_session = TcpTransport::connect("127.0.0.1", src_port, None, None).unwrap();
    let mut dst_session = TcpTransport::connect("127.0.0.1", dst_port, None, None).unwrap();

    let _ = pipe(&mut src_session, &mut dst_session).unwrap();
    drop(dst_session);
    drop(src_session);
    src_handle.join().unwrap();
    dst_handle.join().unwrap();
    let out = fs::read(dst).unwrap();
    assert_eq!(out, b"daemon_remote");
}

struct SlowReceiveTransport {
    data: Vec<u8>,
    delay: Duration,
    pos: usize,
}

impl SlowReceiveTransport {
    fn new(data: Vec<u8>, delay: Duration) -> Self {
        Self {
            data,
            delay,
            pos: 0,
        }
    }
}

impl Transport for SlowReceiveTransport {
    fn send(&mut self, _data: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        std::thread::sleep(self.delay);
        let n = std::cmp::min(buf.len(), self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }

    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct SlowSendTransport<W> {
    inner: W,
    delay: Duration,
}

impl<W> SlowSendTransport<W> {
    fn new(inner: W, delay: Duration) -> Self {
        Self { inner, delay }
    }

    fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Transport for SlowSendTransport<W> {
    fn send(&mut self, data: &[u8]) -> std::io::Result<()> {
        std::thread::sleep(self.delay);
        self.inner.write_all(data)
    }

    fn receive(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }

    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn pipe_refreshes_timeouts_on_slow_receive() {
    let src_inner = SlowReceiveTransport::new(b"x".to_vec(), Duration::from_millis(100));
    let cfg = TransportConfig::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let mut src = TimeoutTransport::new(src_inner, cfg.timeout).unwrap();
    let dst_inner = LocalPipeTransport::new(std::io::empty(), Vec::new());
    let cfg = TransportConfig::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let mut dst = TimeoutTransport::new(dst_inner, cfg.timeout).unwrap();

    let bytes = pipe(&mut src, &mut dst).unwrap();
    assert_eq!(bytes, 1);
    let (_, writer) = dst.into_inner().into_inner();
    assert_eq!(writer, b"x");
}

#[test]
fn pipe_refreshes_timeouts_on_slow_send() {
    let src_inner = LocalPipeTransport::new(&b"y"[..], std::io::sink());
    let cfg = TransportConfig::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let mut src = TimeoutTransport::new(src_inner, cfg.timeout).unwrap();
    let dst_inner = SlowSendTransport::new(Vec::new(), Duration::from_millis(100));
    let cfg = TransportConfig::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let mut dst = TimeoutTransport::new(dst_inner, cfg.timeout).unwrap();

    let bytes = pipe(&mut src, &mut dst).unwrap();
    assert_eq!(bytes, 1);
    let writer = dst.into_inner().into_inner();
    assert_eq!(writer, b"y");
}

struct InterruptReceiveTransport {
    data: Vec<u8>,
    pos: usize,
    interrupts: usize,
}

impl InterruptReceiveTransport {
    fn new(data: Vec<u8>, interrupts: usize) -> Self {
        Self {
            data,
            pos: 0,
            interrupts,
        }
    }
}

impl Transport for InterruptReceiveTransport {
    fn send(&mut self, _data: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn receive(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.interrupts > 0 {
            self.interrupts -= 1;
            return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
        }
        if self.pos >= self.data.len() {
            return Ok(0);
        }
        let n = std::cmp::min(buf.len(), self.data.len() - self.pos);
        buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }

    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct InterruptSendTransport {
    written: Vec<u8>,
    interrupts: usize,
}

impl InterruptSendTransport {
    fn new(interrupts: usize) -> Self {
        Self {
            written: Vec::new(),
            interrupts,
        }
    }
}

impl Transport for InterruptSendTransport {
    fn send(&mut self, data: &[u8]) -> std::io::Result<()> {
        if self.interrupts > 0 {
            self.interrupts -= 1;
            return Err(std::io::Error::from(std::io::ErrorKind::Interrupted));
        }
        self.written.extend_from_slice(data);
        Ok(())
    }

    fn receive(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }

    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn pipe_retries_on_interrupted() {
    let mut src = InterruptReceiveTransport::new(b"retry".to_vec(), 1);
    let mut dst = InterruptSendTransport::new(1);

    let bytes = pipe(&mut src, &mut dst).unwrap();
    assert_eq!(bytes, 5);
    assert_eq!(dst.written, b"retry");
}
