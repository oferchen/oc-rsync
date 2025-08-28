use assert_cmd::Command;
#[cfg(unix)]
use nix::unistd::{chown, Gid, Uid};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use tempfile::tempdir;

#[test]
fn client_local_sync() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello world").unwrap();

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success().stdout("").stderr("");

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello world");
}

#[test]
fn local_sync_without_flag_fails() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([&src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().failure();
}

#[test]
#[ignore]
fn remote_destination_syncs() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("remote_dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("file.txt"), b"hello").unwrap();

    let dst_spec = format!("remote:{}", dst_dir.to_str().unwrap());

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([&src_arg, &dst_spec]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("file.txt")).unwrap();
    assert_eq!(out, b"hello");
}

#[test]
#[ignore]
fn remote_destination_ipv6_syncs() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("remote_dst_v6");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("file.txt"), b"hello").unwrap();

    let dst_spec = format!("[::1]:{}", dst_dir.to_str().unwrap());

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([&src_arg, &dst_spec]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("file.txt")).unwrap();
    assert_eq!(out, b"hello");
}

#[test]
fn relative_preserves_ancestors() {
    let dir = tempdir().unwrap();
    let src_root = dir.path().join("src");
    std::fs::create_dir_all(src_root.join("a/b")).unwrap();
    std::fs::write(src_root.join("a/b/file.txt"), b"hi").unwrap();
    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    cmd.current_dir(dir.path());
    cmd.args(["--local", "-R", "src/a/b/", "dst"]);
    cmd.assert().success();

    let out = std::fs::read(dir.path().join("dst/src/a/b/file.txt")).unwrap();
    assert_eq!(out, b"hi");
}

#[test]
fn progress_flag_shows_output() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello").unwrap();

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--local", "--progress", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert()
        .success()
        .stderr(predicates::str::contains("a.txt"));
}

#[test]
fn resumes_from_partial_dir() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    let partial_dir = dir.path().join("partial");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("a.txt"), b"hello").unwrap();
    std::fs::create_dir_all(&partial_dir).unwrap();
    std::fs::write(partial_dir.join("a.partial"), b"he").unwrap();

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--local",
        "--partial",
        "--partial-dir",
        partial_dir.to_str().unwrap(),
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let out = std::fs::read(dst_dir.join("a.txt")).unwrap();
    assert_eq!(out, b"hello");
    assert!(!partial_dir.join("a.partial").exists());
}

#[test]
fn numeric_ids_are_preserved() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();
    #[cfg(unix)]
    {
        chown(
            &file,
            Some(Uid::from_raw(12345)),
            Some(Gid::from_raw(12345)),
        )
        .unwrap();
    }

    let mut cmd = Command::cargo_bin("rsync-rs").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--local",
        "--numeric-ids",
        "--owner",
        "--group",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    #[cfg(unix)]
    {
        let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
        assert_eq!(meta.uid(), 12345);
        assert_eq!(meta.gid(), 12345);
    }
}
