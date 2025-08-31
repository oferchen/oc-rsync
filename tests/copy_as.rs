// tests/copy_as.rs
use assert_cmd::Command;
#[cfg(unix)]
use nix::unistd::{chown, Gid, Uid, User};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use tempfile::tempdir;
#[cfg(unix)]
use users::get_current_uid;

#[cfg(unix)]
fn can_chown() -> bool {
    if get_current_uid() != 0 {
        eprintln!("skipping copy_as test: requires root or CAP_CHOWN");
        return false;
    }
    let dir = tempdir().unwrap();
    let probe = dir.path().join("probe");
    std::fs::write(&probe, b"probe").unwrap();
    match chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
        Ok(_) => true,
        Err(nix::errno::Errno::EPERM) => {
            eprintln!("skipping copy_as test: lacks CAP_CHOWN");
            false
        }
        Err(err) => panic!("unexpected chown error: {err}"),
    }
}

#[cfg(unix)]
#[test]
fn copy_as_sets_owner_and_group() {
    if !can_chown() {
        return;
    }
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(src_dir.join("file.txt"), b"hi").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--copy-as=1:1",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("file.txt")).unwrap();
    assert_eq!(meta.uid(), 1);
    assert_eq!(meta.gid(), 1);
}

#[cfg(unix)]
#[test]
fn copy_as_uses_default_group() {
    if !can_chown() {
        return;
    }
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    std::fs::write(src_dir.join("file.txt"), b"hi").unwrap();
    let default_gid = User::from_uid(Uid::from_raw(1))
        .unwrap()
        .unwrap()
        .gid
        .as_raw();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--copy-as=1",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("file.txt")).unwrap();
    assert_eq!(meta.uid(), 1);
    assert_eq!(meta.gid(), default_gid);
}
