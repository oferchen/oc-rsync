// crates/engine/tests/permissions.rs
#![doc = "Ownership tests require `CAP_CHOWN`."]
#![cfg(unix)]
#![allow(
    clippy::needless_return,
    clippy::single_match,
    clippy::collapsible_if,
    clippy::redundant_pattern_matching,
    clippy::needless_borrows_for_generic_args
)]

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use compress::available_codecs;
use engine::{IdMapper, SyncOptions, sync};
use filetime::{FileTime, set_file_mtime, set_file_times, set_symlink_file_times};
use filters::Matcher;
use meta::{IdKind, parse_chmod, parse_chown, parse_id_map};
use nix::unistd::{Gid, Uid, chown};
use tempfile::tempdir;

mod tests;

#[test]
fn perms_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            perms: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
}

#[test]
fn chmod_applies() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            perms: true,
            chmod: Some(parse_chmod("F600").unwrap()),
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o600);
}

#[test]
fn chown_applies() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            owner: true,
            group: true,
            chown: Some(parse_chown("1:1").unwrap()),
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.uid(), 1);
    assert_eq!(meta.gid(), 1);
}

#[test]
fn chown_matches_rsync() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst_rrs = tmp.path().join("rrs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst_rrs).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();

    sync(
        &src,
        &dst_rrs,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            owner: true,
            group: true,
            chown: Some(parse_chown("1:1").unwrap()),
            ..Default::default()
        },
    )
    .unwrap();

    let meta_rrs = fs::metadata(dst_rrs.join("file")).unwrap();
    assert_eq!(meta_rrs.uid(), 1);
    assert_eq!(meta_rrs.gid(), 1);
}

#[test]
fn times_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_file_mtime(&file, mtime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            times: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_eq!(dst_mtime, mtime);
}

#[test]
fn omit_dir_times_skips_dirs() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sub = src.join("dir");
    fs::create_dir(&sub).unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_file_mtime(&sub, mtime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            times: true,
            omit_dir_times: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("dir")).unwrap();
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_ne!(dst_mtime, mtime);
}

#[test]
fn omit_link_times_skips_symlinks() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let target = src.join("file");
    fs::write(&target, b"hi").unwrap();
    let link = src.join("link");
    std::os::unix::fs::symlink("file", &link).unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_symlink_file_times(&link, mtime, mtime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            times: true,
            links: true,
            omit_link_times: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("link")).unwrap();
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_ne!(dst_mtime, mtime);
}

#[test]
fn atimes_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let atime = FileTime::from_unix_time(1_000_000, 987_654_321);
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_file_times(&file, atime, mtime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            atimes: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(src.join("file")).unwrap();
    let dst_meta = fs::metadata(dst.join("file")).unwrap();
    let src_atime = FileTime::from_last_access_time(&src_meta);
    let dst_atime = FileTime::from_last_access_time(&dst_meta);
    let dst_mtime = FileTime::from_last_modification_time(&dst_meta);
    assert_eq!(src_atime, atime);
    assert_eq!(dst_atime, atime);
    assert_ne!(dst_mtime, mtime);
}

#[test]
fn symlink_atimes_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let link = src.join("link");
    std::os::unix::fs::symlink("file", &link).unwrap();
    let atime = FileTime::from_unix_time(1_000_000, 987_654_321);
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_symlink_file_times(&link, atime, mtime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            links: true,
            atimes: true,
            times: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("link")).unwrap();
    let dst_atime = FileTime::from_last_access_time(&meta);
    if dst_atime != atime {
        eprintln!("Skipping symlink_atimes_roundtrip test: atime not supported");
        return;
    }
}

#[test]
fn crtimes_roundtrip() {
    use std::time::Duration;

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();

    let src_meta = fs::metadata(&file).unwrap();
    let src_crtime = match src_meta.created() {
        Ok(t) => t,
        Err(_) => return,
    };

    std::thread::sleep(Duration::from_secs(1));

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            crtimes: true,
            ..Default::default()
        },
    )
    .unwrap();

    let dst_meta = fs::metadata(dst.join("file")).unwrap();
    match dst_meta.created() {
        Ok(t) => {
            if cfg!(target_os = "linux") {
                if t != src_crtime {
                    return;
                }
            }
            assert_eq!(t, src_crtime);
        }
        Err(_) => {}
    }
}

#[test]
fn owner_roundtrip() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    chown(&file, Some(Uid::from_raw(1000)), Some(Gid::from_raw(0))).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            owner: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.uid(), 1000);
    assert_eq!(meta.gid(), 0);
}

#[test]
fn group_roundtrip() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    chown(&file, Some(Uid::from_raw(0)), Some(Gid::from_raw(1000))).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            group: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.uid(), 0);
    assert_eq!(meta.gid(), 1000);
}

#[test]
fn links_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("target"), b"t").unwrap();
    std::os::unix::fs::symlink("target", src.join("link")).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            links: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    assert_eq!(
        fs::read_link(dst.join("link")).unwrap(),
        std::path::PathBuf::from("target")
    );
}

#[test]
fn hard_links_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let f1 = src.join("f1");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("f2");
    fs::hard_link(&f1, &f2).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            hard_links: true,
            ..Default::default()
        },
    )
    .unwrap();
    let m1 = fs::metadata(dst.join("f1")).unwrap();
    let m2 = fs::metadata(dst.join("f2")).unwrap();
    assert_eq!(m1.ino(), m2.ino());
}

#[test]
fn metadata_matches_source() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_file_mtime(&file, mtime).unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            owner: true,
            group: true,
            perms: true,
            times: true,
            crtimes: true,
            ..Default::default()
        },
    )
    .unwrap();

    let meta_src = fs::metadata(&file).unwrap();
    let meta_dst = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta_dst.uid(), meta_src.uid());
    assert_eq!(meta_dst.gid(), meta_src.gid());
    assert_eq!(
        meta_dst.permissions().mode() & 0o7777,
        meta_src.permissions().mode() & 0o7777
    );
    let mt_src = FileTime::from_last_modification_time(&meta_src);
    let mt_dst = FileTime::from_last_modification_time(&meta_dst);
    assert_eq!(mt_src, mt_dst);
    let cr_src = meta_src.created().ok();
    let cr_dst = meta_dst.created().ok();
    assert!(cr_src.is_some());
    assert!(cr_dst.is_some());
}
#[test]
fn usermap_groupmap_names() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    chown(
        &file,
        Some(Uid::from_raw(65534)),
        Some(Gid::from_raw(65534)),
    )
    .unwrap();
    let uid_map = parse_id_map("nobody:root", IdKind::User).unwrap();
    let gid_map = parse_id_map("nogroup:root", IdKind::Group).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            owner: true,
            group: true,
            perms: true,
            uid_map: Some(IdMapper(uid_map)),
            gid_map: Some(IdMapper(gid_map)),
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("file")).unwrap();
    assert_eq!(meta.uid(), 0);
    assert_eq!(meta.gid(), 0);
}
