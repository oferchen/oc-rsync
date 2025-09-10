// tests/bin_daemon.rs
use assert_cmd::cargo::{CommandCargoExt, cargo_bin};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tempfile::tempdir;

mod common;
use common::daemon::DaemonGuard;

fn wait_for_daemon(port: u16, timeout: Duration) {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start within {timeout:?}");
}

fn read_port(child: &mut Child, timeout: Duration) -> u16 {
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        tx.send(line).ok();
    });

    let line = match rx.recv_timeout(timeout) {
        Ok(line) => {
            let _ = handle.join();
            line
        }
        Err(_) => {
            let _ = child.kill();
            let _ = handle.join();
            let _ = child.wait();
            panic!("daemon did not print a port number within {timeout:?}");
        }
    };

    let trimmed = line.trim();
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        let _ = child.kill();
        let stdout = line;
        let mut stderr = String::new();
        if let Some(mut e) = child.stderr.take() {
            e.read_to_string(&mut stderr).unwrap();
        }
        let _ = child.wait();
        panic!("daemon did not print a port number\nstdout: {stdout}\nstderr: {stderr}");
    }
    trimmed.parse::<u16>().unwrap()
}

#[test]
fn starts_daemon() {
    let tmp = tempdir().unwrap();
    let mut cmd = Command::cargo_bin("oc-rsyncd").unwrap();
    cmd.env("OC_RSYNC_BIN", cargo_bin("oc-rsync"))
        .args([
            "--no-detach",
            "--port=0",
            "--address=127.0.0.1",
            "--module",
            &format!("data={}", tmp.path().display()),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = DaemonGuard::spawn(cmd);

    let port = read_port(&mut child, Duration::from_secs(5));

    wait_for_daemon(port, Duration::from_secs(5));
}

#[test]
fn accepts_config_option() {
    let tmp = tempdir().unwrap();
    let pid_file = tmp.path().join("pid");
    let cfg_path = tmp.path().join("conf");
    fs::write(
        &cfg_path,
        format!(
            "pid file = {}\n[data]\npath = {}\n",
            pid_file.display(),
            tmp.path().display()
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("oc-rsyncd").unwrap();
    cmd.env("OC_RSYNC_BIN", cargo_bin("oc-rsync"))
        .args([
            "--no-detach",
            "--port=0",
            "--address=127.0.0.1",
            "--config",
            cfg_path.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = DaemonGuard::spawn(cmd);

    let port = read_port(&mut child, Duration::from_secs(5));

    wait_for_daemon(port, Duration::from_secs(5));
    assert!(pid_file.exists());
}
