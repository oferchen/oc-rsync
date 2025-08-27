use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn client_local_sync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("a.txt");
    let dst = dir.path().join("b.txt");
    std::fs::write(&src, b"hello world").unwrap();

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args(["client", "--local", src.to_str().unwrap(), dst.to_str().unwrap()]);
    cmd.assert().success();

    let out = std::fs::read(&dst).unwrap();
    assert_eq!(out, b"hello world");
}
