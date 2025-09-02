// crates/transport/tests/pipe.rs
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use tempfile::tempdir;
use transport::{pipe, SshStdioTransport, TcpTransport};

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

    pipe(&mut src_session, &mut dst_session).unwrap();
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

    pipe(&mut src_session, &mut dst_session).unwrap();
    drop(dst_session);
    drop(src_session);
    src_handle.join().unwrap();
    dst_handle.join().unwrap();
    let out = fs::read(dst).unwrap();
    assert_eq!(out, b"daemon_remote");
}
