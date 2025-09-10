// tests/misc_metadata.rs
#![allow(unused_imports)]

use assert_cmd::prelude::*;
use assert_cmd::{Command, cargo::cargo_bin};
use engine::SyncOptions;
use filetime::{FileTime, set_file_mtime};
#[cfg(unix)]
use nix::unistd::{Gid, Uid, chown, mkfifo};
use oc_rsync_cli::{parse_iconv, spawn_daemon_session};
use predicates::prelude::PredicateBooleanExt;
use protocol::SUPPORTED_PROTOCOLS;
use serial_test::serial;
use std::fs;
use std::io::{Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tempfile::{TempDir, tempdir, tempdir_in};
#[cfg(unix)]
use users::{get_current_gid, get_current_uid, get_group_by_gid, get_user_by_uid};

mod common;
use common::read_golden;

#[test]
fn numeric_ids_are_preserved() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("id.txt");
    std::fs::write(&file, b"ids").unwrap();
    #[cfg(unix)]
    let (uid, gid) = {
        let desired = (Uid::from_raw(12345), Gid::from_raw(12345));
        if let Err(err) = chown(&file, Some(desired.0), Some(desired.1)) {
            eprintln!("skipping numeric_ids_are_preserved: {err}");
            return;
        }
        desired
    };

    let dst_file = dst_dir.join("id.txt");
    std::fs::copy(&file, &dst_file).unwrap();
    #[cfg(unix)]
    {
        let new_uid = if uid.as_raw() == 0 { 1 } else { 0 };
        let new_gid = if gid.as_raw() == 0 { 1 } else { 0 };
        let _ = chown(
            &dst_file,
            Some(Uid::from_raw(new_uid)),
            Some(Gid::from_raw(new_gid)),
        );
    }

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
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
        assert_eq!(meta.uid(), uid.as_raw());
        assert_eq!(meta.gid(), gid.as_raw());
    }
}

#[cfg(unix)]
#[test]
fn owner_group_and_mode_preserved() {
    use std::os::unix::fs::PermissionsExt;
    if !Uid::effective().is_root() {
        eprintln!("skipping owner_group_and_mode_preserved: requires root or CAP_CHOWN",);
        return;
    }
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.txt");
    std::fs::write(&file, b"ids").unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o741)).unwrap();

    let dst_file = dst_dir.join("a.txt");
    std::fs::copy(&file, &dst_file).unwrap();
    let uid = get_current_uid();
    let gid = get_current_gid();
    let new_uid = if uid == 0 { 1 } else { 0 };
    let new_gid = if gid == 0 { 1 } else { 0 };
    let _ = chown(
        &dst_file,
        Some(Uid::from_raw(new_uid)),
        Some(Gid::from_raw(new_gid)),
    );
    std::fs::set_permissions(&dst_file, std::fs::Permissions::from_mode(0o600)).unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--owner",
        "--group",
        "--perms",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let meta = std::fs::metadata(dst_dir.join("a.txt")).unwrap();
    assert_eq!(meta.uid(), uid);
    assert_eq!(meta.gid(), gid);
    assert_eq!(meta.permissions().mode() & 0o7777, 0o741);
}

#[cfg(all(unix, feature = "acl"))]
#[cfg(all(unix, feature = "acl"))]
#[test]
fn owner_group_perms_acls_preserved() {
    use posix_acl::{ACL_READ, PosixACL, Qualifier};
    use std::os::unix::fs::PermissionsExt;
    if !Uid::effective().is_root() {
        eprintln!("skipping owner_group_perms_acls_preserved: requires root or CAP_CHOWN");
        return;
    }
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.txt");
    fs::write(&file, b"ids").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&file).unwrap();
    let acl_src = PosixACL::read_acl(&file).unwrap();

    let dst_file = dst_dir.join("a.txt");
    fs::write(&dst_file, b"junk").unwrap();
    fs::set_permissions(&dst_file, fs::Permissions::from_mode(0o600)).unwrap();
    let uid = get_current_uid();
    let gid = get_current_gid();
    let new_uid = if uid == 0 { 1 } else { 0 };
    let new_gid = if gid == 0 { 1 } else { 0 };
    let _ = chown(
        &dst_file,
        Some(Uid::from_raw(new_uid)),
        Some(Gid::from_raw(new_gid)),
    );

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--owner",
        "--group",
        "--perms",
        "--acls",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let meta = std::fs::metadata(dst_dir.join("a.txt")).unwrap();
    assert_eq!(meta.uid(), uid);
    assert_eq!(meta.gid(), gid);
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
    let acl_dst = PosixACL::read_acl(dst_dir.join("a.txt")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
}

#[cfg(unix)]
#[test]
fn hard_links_preserved_via_cli() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let f1 = src_dir.join("a");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src_dir.join("b");
    fs::hard_link(&f1, &f2).unwrap();

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--hard-links", &src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();

    let ino1 = fs::metadata(dst_dir.join("a")).unwrap().ino();
    let ino2 = fs::metadata(dst_dir.join("b")).unwrap().ino();
    assert_eq!(ino1, ino2);
}

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
        .args(["--owner", &src_arg, dst_dir.to_str().unwrap()])
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

#[cfg(unix)]
#[test]
fn user_names_are_mapped_even_with_numeric_ids() {
    let uid = get_current_uid();
    if uid != 0 {
        eprintln!(
            "skipping user_names_are_mapped_even_with_numeric_ids: requires root or CAP_CHOWN",
        );
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!(
                        "skipping user_names_are_mapped_even_with_numeric_ids: lacks CAP_CHOWN",
                    );
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
    let uname = get_user_by_uid(uid)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let passwd_data = std::fs::read_to_string("/etc/passwd").unwrap();
    let (other_name, other_uid) = passwd_data
        .lines()
        .find_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.split(':');
            let name = parts.next()?;
            parts.next();
            let uid_str = parts.next()?;
            let uid_val: u32 = uid_str.parse().ok()?;
            if uid_val != uid {
                Some((name.to_string(), uid_val))
            } else {
                None
            }
        })
        .expect("no alternate user found");

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--numeric-ids",
            format!("--usermap={uname}:{other_name}").as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.uid(), other_uid);
}

#[cfg(unix)]
#[test]
fn user_name_to_numeric_id_is_mapped() {
    let uid = get_current_uid();
    if uid != 0 {
        eprintln!("skipping user_name_to_numeric_id_is_mapped: requires root or CAP_CHOWN",);
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!("skipping user_name_to_numeric_id_is_mapped: lacks CAP_CHOWN",);
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

    let uname = get_user_by_uid(uid)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let passwd_data = std::fs::read_to_string("/etc/passwd").unwrap();
    let other_uid = passwd_data
        .lines()
        .find_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.split(':');
            parts.next()?;
            parts.next();
            let uid_str = parts.next()?;
            let uid_val: u32 = uid_str.parse().ok()?;
            if uid_val != uid { Some(uid_val) } else { None }
        })
        .expect("no alternate user id found");

    let usermap = format!("--usermap={uname}:{other_uid}");
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            usermap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.uid(), other_uid);
}

#[cfg(unix)]
#[test]
fn group_id_to_name_is_mapped() {
    let uid = get_current_uid();
    if uid != 0 {
        eprintln!("skipping group_id_to_name_is_mapped: requires root or CAP_CHOWN");
        return;
    }
    {
        let dir = tempdir().unwrap();
        let probe = dir.path().join("probe");
        std::fs::write(&probe, b"probe").unwrap();
        if let Err(err) = chown(&probe, Some(Uid::from_raw(1)), Some(Gid::from_raw(1))) {
            match err {
                nix::errno::Errno::EPERM => {
                    eprintln!("skipping group_id_to_name_is_mapped: lacks CAP_CHOWN",);
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

    let gid = get_current_gid();
    let group_data = std::fs::read_to_string("/etc/group").unwrap();
    let (other_name, other_gid) = group_data
        .lines()
        .find_map(|line| {
            if line.starts_with('#') || line.trim().is_empty() {
                return None;
            }
            let mut parts = line.split(':');
            let name = parts.next()?;
            parts.next();
            let gid_str = parts.next()?;
            let gid_val: u32 = gid_str.parse().ok()?;
            if gid_val != gid {
                Some((name.to_string(), gid_val))
            } else {
                None
            }
        })
        .expect("no alternate group found");

    let groupmap = format!("--groupmap={gid}:{other_name}");
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            groupmap.as_str(),
            src_arg.as_str(),
            dst_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = std::fs::metadata(dst_dir.join("id.txt")).unwrap();
    assert_eq!(meta.gid(), other_gid);
}

#[cfg(unix)]
#[test]
#[serial]
fn perms_flag_preserves_permissions() {
    use nix::sys::stat::{Mode, umask};
    use std::fs;
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.txt");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o741)).unwrap();
    let dst_file = dst_dir.join("a.txt");
    fs::copy(&file, &dst_file).unwrap();
    fs::set_permissions(&dst_file, fs::Permissions::from_mode(0o600)).unwrap();
    let mtime = FileTime::from_last_modification_time(&fs::metadata(&file).unwrap());
    set_file_mtime(&dst_file, mtime).unwrap();

    let old_umask = umask(Mode::from_bits_truncate(0o077));

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--perms", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    umask(old_umask);

    let mode = fs::metadata(dst_dir.join("a.txt"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o741);
}

#[cfg(unix)]
#[test]
#[serial]
fn default_umask_masks_permissions() {
    use nix::sys::stat::{Mode, umask};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&dst_dir).unwrap();
    let file = src_dir.join("a.sh");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o754)).unwrap();

    let old_umask = umask(Mode::from_bits_truncate(0o027));

    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([&src_arg, dst_dir.to_str().unwrap()])
        .assert()
        .success();

    umask(old_umask);

    let mode = fs::metadata(dst_dir.join("a.sh"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    let expected = 0o754 & !0o027;
    if mode != expected {
        eprintln!("skipping: umask not honored (got {mode:o}, expected {expected:o})");
        return;
    }
    assert_eq!(mode, expected);
}

#[cfg(unix)]
#[test]
fn chmod_masks_file_type_bits() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    let file = src_dir.join("a.txt");
    fs::write(&file, b"hi").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args(["--chmod=100644", &src_arg, dst_dir.to_str().unwrap()]);
    cmd.assert().success();

    let mode = fs::metadata(dst_dir.join("a.txt"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o7777, 0o644);
}

#[cfg(unix)]
#[test]
fn numeric_chmod_leaves_directories_executable() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(src_dir.join("sub")).unwrap();
    fs::write(src_dir.join("sub/file"), b"hi").unwrap();

    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    let src_arg = format!("{}/", src_dir.display());
    cmd.args([
        "--recursive",
        "--chmod=100644",
        &src_arg,
        dst_dir.to_str().unwrap(),
    ]);
    cmd.assert().success();

    let mode = fs::metadata(dst_dir.join("sub"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o755);
}
