// tests/oc_rsyncd_start.rs
use assert_cmd::cargo::{cargo_bin, CommandCargoExt};
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::process::{Command, Stdio};
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
        .spawn()
        .unwrap();

    let port = {
        let mut line = String::new();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        reader.read_line(&mut line).unwrap();
        line.trim().parse::<u16>().unwrap()
    };

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
        .spawn()
        .unwrap();

    let port = {
        let mut line = String::new();
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        reader.read_line(&mut line).unwrap();
        line.trim().parse::<u16>().unwrap()
    };

    wait_for_daemon(port);
    assert!(pid_file.exists());

    let _ = child.kill();
    let _ = child.wait();
}
