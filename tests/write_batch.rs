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
