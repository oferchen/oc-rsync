use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn client_local_sync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello world").unwrap();

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.args([
        "client",
        "--local",
        src_dir.to_str().unwrap(),
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello world");
}
