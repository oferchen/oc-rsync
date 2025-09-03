// tests/fake_super.rs
#[cfg(all(unix, feature = "xattr"))]
use assert_cmd::Command;
#[cfg(all(unix, feature = "xattr"))]
use nix::unistd::Uid;
#[cfg(all(unix, feature = "xattr"))]
use std::fs;
#[cfg(all(unix, feature = "xattr"))]
use tempfile::tempdir;

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn fake_super_stores_xattrs() {
    if Uid::effective().is_root() {
        eprintln!("skipping test as root");
        return;
    }
    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("file");
    fs::write(&file, b"hi").unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-a",
            "--fake-super",
            src_dir.to_str().unwrap(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();
    let dst_file = dst_dir.join("file");
    assert!(xattr::get(&dst_file, "user.rsync.uid").unwrap().is_some());
    assert!(xattr::get(&dst_file, "user.rsync.gid").unwrap().is_some());
    assert!(xattr::get(&dst_file, "user.rsync.mode").unwrap().is_some());
}
