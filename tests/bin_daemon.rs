// tests/bin_daemon.rs
use assert_cmd::cargo::{cargo_bin, CommandCargoExt};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;

fn wait_for_daemon(port: u16) {
    for _ in 0..50 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        sleep(Duration::from_millis(50));
    }
    panic!("daemon did not start");
}

fn read_port(child: &mut Child) -> u16 {
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let trimmed = line.trim();
    if !trimmed.chars().all(|c| c.is_ascii_digit()) {
        let _ = child.kill();
        let mut stdout = line;
        reader.read_to_string(&mut stdout).unwrap();
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
    let mut child = Command::cargo_bin("oc-rsyncd")
        .unwrap()
        .env("OC_RSYNC_BIN", cargo_bin("oc-rsync"))
        .args([
            "--no-detach",
            "--port=0",
            "--address=127.0.0.1",
            "--module",
            &format!("data={}", tmp.path().display()),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let port = read_port(&mut child);

    wait_for_daemon(port);

    let _ = child.kill();
    let _ = child.wait();
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

    let mut child = Command::cargo_bin("oc-rsyncd")
        .unwrap()
        .env("OC_RSYNC_BIN", cargo_bin("oc-rsync"))
        .args([
            "--no-detach",
            "--port=0",
            "--address=127.0.0.1",
            "--config",
            cfg_path.to_str().unwrap(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let port = read_port(&mut child);

    wait_for_daemon(port);
    assert!(pid_file.exists());

    let _ = child.kill();
    let _ = child.wait();
}
