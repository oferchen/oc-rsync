use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn accepts_sockopts() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f"), b"data").unwrap();
    let dst = dir.path().join("dst");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--sockopts",
            "SO_KEEPALIVE",
            "-r",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
}

#[test]
fn rejects_invalid_sockopts() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    let dst = dir.path().join("dst");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--sockopts",
            "ip:bad=1",
            "-r",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .failure();
}
