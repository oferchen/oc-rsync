// crates/transport/tests/blocking_io.rs
#[cfg(unix)]
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::net::TcpListener;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::thread;
use transport::{ssh::SshStdioTransport, tcp::TcpTransport, Transport};

#[cfg(unix)]
#[test]
fn ssh_blocking_mode() {
    let mut t = SshStdioTransport::spawn("sh", ["-c", "cat"]).expect("spawn");
    t.set_blocking_io(true).expect("set");
    let (reader, writer) = t.into_inner().expect("inner");
    let rflags =
        OFlag::from_bits_truncate(fcntl(reader.get_ref().as_raw_fd(), FcntlArg::F_GETFL).unwrap());
    let wflags = OFlag::from_bits_truncate(fcntl(writer.as_raw_fd(), FcntlArg::F_GETFL).unwrap());
    assert!(!rflags.contains(OFlag::O_NONBLOCK));
    assert!(!wflags.contains(OFlag::O_NONBLOCK));
}

#[cfg(unix)]
#[test]
fn ssh_nonblocking_default() {
    let t = SshStdioTransport::spawn("sh", ["-c", "cat"]).expect("spawn");
    let (reader, writer) = t.into_inner().expect("inner");
    let rflags =
        OFlag::from_bits_truncate(fcntl(reader.get_ref().as_raw_fd(), FcntlArg::F_GETFL).unwrap());
    let wflags = OFlag::from_bits_truncate(fcntl(writer.as_raw_fd(), FcntlArg::F_GETFL).unwrap());
    assert!(rflags.contains(OFlag::O_NONBLOCK));
    assert!(wflags.contains(OFlag::O_NONBLOCK));
}

#[cfg(unix)]
#[test]
fn tcp_blocking_mode() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let _ = listener.accept();
    });
    let mut t =
        TcpTransport::connect(&addr.ip().to_string(), addr.port(), None, None).expect("connect");
    t.set_blocking_io(true).expect("set");
    let fd = t.into_inner().as_raw_fd();
    let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL).unwrap());
    assert!(!flags.contains(OFlag::O_NONBLOCK));
    handle.join().unwrap();
}

#[cfg(unix)]
#[test]
fn tcp_nonblocking_default() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let _ = listener.accept();
    });
    let t =
        TcpTransport::connect(&addr.ip().to_string(), addr.port(), None, None).expect("connect");
    let fd = t.into_inner().as_raw_fd();
    let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL).unwrap());
    assert!(flags.contains(OFlag::O_NONBLOCK));
    handle.join().unwrap();
}
