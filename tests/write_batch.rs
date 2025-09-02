// tests/write_batch.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn creates_batch_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f"), b"data").unwrap();
    let dst = dir.path().join("dst");
    let batch = dir.path().join("batch.txt");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--write-batch",
            batch.to_str().unwrap(),
            "-r",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(batch.exists());
    let log = fs::read_to_string(batch).unwrap();
    assert!(log.contains("files_transferred=1"));
    assert!(log.contains("bytes_transferred=4"));
}

#[test]
fn replays_batch_file() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("f"), b"data").unwrap();
    let dst1 = dir.path().join("dst1");
    let batch = dir.path().join("batch.txt");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--write-batch",
            batch.to_str().unwrap(),
            "-r",
            src.to_str().unwrap(),
            dst1.to_str().unwrap(),
        ])
        .assert()
        .success();
    fs::write(src.join("g"), b"new").unwrap();
    let dst2 = dir.path().join("dst2");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--read-batch",
            batch.to_str().unwrap(),
            "-r",
            src.to_str().unwrap(),
            dst2.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(dst2.join("src/f").exists());
    assert!(!dst2.join("src/g").exists());
}
