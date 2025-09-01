use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn log_file_writes_messages() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.txt");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "-v",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("verbose level set to 1"), "{}", contents);
}
