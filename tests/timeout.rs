// tests/timeout.rs
#![allow(clippy::err_expect)]

use std::collections::HashMap;
use std::fs;
use std::io::{self, Cursor, Read};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use daemon::{handle_connection, Handler, Module};
use engine::{EngineError, SyncOptions};
use oc_rsync_cli::spawn_daemon_session;
use predicates::str::contains;
use protocol::{Demux, ExitCode, CAP_CODECS};
use tempfile::tempdir;
use transport::{
    rate_limited, ssh::SshStdioTransport, LocalPipeTransport, TcpTransport, TimeoutTransport,
    Transport,
};

#[test]
fn connection_inactivity_timeout() {
    let reader = Cursor::new(Vec::new());
    let writer = Cursor::new(Vec::new());
    let mut t = TimeoutTransport::new(
        LocalPipeTransport::new(reader, writer),
        Some(Duration::from_millis(100)),
    )
    .unwrap();
    thread::sleep(Duration::from_millis(200));
    let err = t.send(b"ping").err().expect("error");
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn idle_inactivity_timeout() {
    let reader = Cursor::new(Vec::new());
    let writer = Cursor::new(Vec::new());
    let mut t = TimeoutTransport::new(
        LocalPipeTransport::new(reader, writer),
        Some(Duration::from_millis(100)),
    )
    .unwrap();
    t.send(b"ping").unwrap();
    thread::sleep(Duration::from_millis(200));
    let err = t.send(b"pong").err().expect("error");
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn timeout_can_be_updated() {
    let reader = Cursor::new(Vec::new());
    let writer = Cursor::new(Vec::new());
    let mut t = TimeoutTransport::new(LocalPipeTransport::new(reader, writer), None).unwrap();
    t.send(b"ping").unwrap();
    t.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    t.set_write_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    thread::sleep(Duration::from_millis(200));
    let err = t.send(b"pong").err().expect("error");
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn rate_limited_respects_timeout() {
    let reader = Cursor::new(Vec::new());
    let writer = Cursor::new(Vec::new());
    let inner = TimeoutTransport::new(
        LocalPipeTransport::new(reader, writer),
        Some(Duration::from_millis(50)),
    )
    .unwrap();
    let mut t = rate_limited(inner, 10);
    t.send(&[0]).unwrap();
    let res = t.send(&[0]);
    assert!(res.is_ok());
}

#[test]
fn tcp_read_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (_sock, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(5));
    });
    let mut t = TcpTransport::connect(&addr.ip().to_string(), addr.port(), None, None).unwrap();
    t.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut buf = [0u8; 1];
    let err = t.receive(&mut buf).err().expect("error");
    assert!(err.kind() == io::ErrorKind::WouldBlock || err.kind() == io::ErrorKind::TimedOut);
}

#[test]
fn ssh_read_timeout() {
    let mut t = SshStdioTransport::spawn("sh", ["-c", "sleep 5"]).unwrap();
    t.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let mut buf = [0u8; 1];
    let err = t.receive(&mut buf).err().expect("error");
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn tcp_handshake_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (_sock, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(5));
    });
    let timeout = Duration::from_millis(100);
    let start = Instant::now();
    let mut t =
        TcpTransport::connect(&addr.ip().to_string(), addr.port(), Some(timeout), None).unwrap();
    let remaining = timeout
        .checked_sub(start.elapsed())
        .unwrap_or_else(|| Duration::from_millis(0));
    t.set_read_timeout(Some(remaining)).unwrap();
    let mut buf = [0u8; 1];
    let err = t.receive(&mut buf).err().expect("error");
    assert!(err.kind() == io::ErrorKind::WouldBlock || err.kind() == io::ErrorKind::TimedOut);
}

#[test]
fn ssh_handshake_timeout() {
    let mut t = SshStdioTransport::spawn("sh", ["-c", "sleep 5"]).unwrap();
    t.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    t.set_write_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let err = SshStdioTransport::handshake(&mut t, &[], &[], None, 31, CAP_CODECS).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn demux_channel_timeout() {
    let mut demux = Demux::new(Duration::from_millis(100));
    demux.register_channel(0);
    thread::sleep(Duration::from_millis(200));
    let err = demux.poll().unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::TimedOut);
}

#[test]
fn daemon_handshake_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (_sock, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(5));
    });
    let res = spawn_daemon_session(
        &addr.ip().to_string(),
        "mod",
        Some(addr.port()),
        None,
        true,
        None,
        Some(Duration::from_millis(100)),
        None,
        &[],
        &SyncOptions::default(),
        31,
        None,
        None,
    );
    match res {
        Ok(_) => panic!("expected timeout"),
        Err(EngineError::Io(e)) => {
            assert!(e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::WouldBlock)
        }
        Err(other) => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn daemon_handshake_timeout_message() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let modules: HashMap<String, Module> = HashMap::new();
    let handler: Arc<Handler> = Arc::new(|_| Ok(()));
    thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let transport = TcpTransport::from_stream(stream);
        let timeout = Some(Duration::from_millis(100));
        let mut t = TimeoutTransport::new(transport, timeout).unwrap();
        let _ = handle_connection(
            &mut t,
            &modules,
            None,
            None,
            None,
            None,
            None,
            false,
            &[],
            "127.0.0.1",
            0,
            0,
            &handler,
            timeout,
        );
    });
    let mut sock = TcpStream::connect(addr).unwrap();
    let mut buf = Vec::new();
    sock.read_to_end(&mut buf).unwrap();
    let msg = String::from_utf8_lossy(&buf);
    let expected =
        fs::read_to_string("tests/golden/messages/timeout/daemon_handshake.rsync.stderr").unwrap();
    assert_eq!(msg.trim_end(), expected.trim_end());
}

#[test]
fn daemon_connection_timeout_exit_code() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (_sock, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_secs(5));
    });
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--contimeout=1",
            &format!("rsync://127.0.0.1:{}/mod/", addr.port()),
            ".",
        ])
        .assert()
        .failure()
        .code(u8::from(ExitCode::ConnTimeout) as i32)
        .stderr(contains("operation timed out"));
}

#[test]
fn ssh_connection_timeout_exit_code() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--contimeout=1", "203.0.113.1:/tmp", "."])
        .assert()
        .failure()
        .code(u8::from(ExitCode::ConnTimeout) as i32)
        .stderr(contains("failed to read version"));
}

#[test]
fn stop_at_exit_code() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file.txt"), b"data").unwrap();
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--recursive",
            "--stop-at=2000-01-01T00:00",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .code(u8::from(ExitCode::Timeout) as i32)
        .stderr(contains("operation timed out"));
}
