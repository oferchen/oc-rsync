// tests/cli/ownership.rs

use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use protocol;
use tempfile::tempdir;
#[cfg(unix)]
use nix::unistd::{chown, Gid, Uid};
#[cfg(unix)]
use users::{get_current_gid, get_current_uid};

#[cfg(unix)]
#[test]
fn numeric_ids_falls_back_when_unprivileged() {
    let dir = tempdir().unwrap();
    let probe = dir.path().join("probe");
    std::fs::write(&probe, b"probe").unwrap();
    let current_uid = get_current_uid();
    let current_gid = get_current_gid();
    if Uid::effective().is_root() {
        eprintln!("skipping numeric_ids_falls_back_when_unprivileged: requires non-root");
        return;
    }
    let target_uid = current_uid + 1;
    if chown(&probe, Some(Uid::from_raw(target_uid)), None).is_ok() {
        eprintln!("skipping numeric_ids_falls_back_when_unprivileged: has CAP_CHOWN");
        return;
    }

    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--numeric-ids",
            "--owner",
            "--group",
            &src_arg,
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let dst_file = dst_dir.join("id.txt");
    let meta = std::fs::metadata(&dst_file).unwrap();
    assert_eq!(meta.uid(), current_uid);
    assert_eq!(meta.gid(), current_gid);
}

#[cfg(unix)]
#[test]
fn owner_requires_privileges() {
    let dir = tempdir().unwrap();
    let probe = dir.path().join("probe");
    std::fs::write(&probe, b"probe").unwrap();
    let current_uid = get_current_uid();
    let target_uid = if current_uid == 0 { 1 } else { current_uid + 1 };
    if chown(&probe, Some(Uid::from_raw(target_uid)), None).is_ok() {
        eprintln!("skipping owner_requires_privileges: has CAP_CHOWN or running as root");
        return;
    }

    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--owner", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .failure()
        .code(u8::from(protocol::ExitCode::StartClient) as i32)
        .stderr(predicates::str::contains("changing ownership requires"));

    let dst_file = dst_dir.join("id.txt");
    assert!(!dst_file.exists());
}

#[cfg(unix)]
#[test]
fn user_and_group_ids_are_mapped() {
    let uid = get_current_uid();
    let _gid = get_current_gid();
    if uid != 0 {
        eprintln!("skipping user_and_group_ids_are_mapped: requires root or CAP_CHOWN");
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!("skipping user_and_group_ids_are_mapped: lacks CAP_CHOWN");
                    return;
                }
                _ => panic!("unexpected chown error: {err}"),
            }
        }
    }

    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();

    let src_arg = format!("{}/", src_dir.display());
    let uid = get_current_uid();
    let gid = get_current_gid();
    let mapped_uid = 1;
    let mapped_gid = 1;
    let usermap = format!("--usermap={uid}:{mapped_uid}");
    let groupmap = format!("--groupmap={gid}:{mapped_gid}");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            usermap.as_str(),
            groupmap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.uid(), mapped_uid);
    assert_eq!(meta.gid(), mapped_gid);
}

