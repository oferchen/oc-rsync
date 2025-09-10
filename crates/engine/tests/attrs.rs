// crates/engine/tests/attrs.rs
#![doc = "Ownership tests require `CAP_CHOWN`. Device tests require `CAP_MKNOD`. \\nXattr and ACL tests skip when unsupported."]
#![cfg(unix)]
#![allow(
    clippy::needless_return,
    clippy::single_match,
    clippy::collapsible_if,
    clippy::redundant_pattern_matching,
    clippy::needless_borrows_for_generic_args
)]

use std::convert::TryInto;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};

use compress::available_codecs;
use engine::{IdMapper, SyncOptions, sync};
use filetime::{FileTime, set_file_mtime, set_file_times, set_symlink_file_times};
use filters::Matcher;
use meta::{IdKind, parse_chmod, parse_chown, parse_id_map};
use nix::sys::stat::{Mode, SFlag, mknod};
use nix::unistd::{Gid, Uid, chown, mkfifo};
#[cfg(feature = "acl")]
use posix_acl::{ACL_READ, ACL_WRITE, PosixACL, Qualifier};
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

#[cfg(feature = "xattr")]
#[test]
fn xattrs_roundtrip() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            xattrs: true,
            ..Default::default()
        },
    )
    .unwrap();
    let val = xattr::get(dst.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[cfg(feature = "xattr")]
#[test]
fn symlink_xattrs_roundtrip() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    std::os::unix::fs::symlink("file", src.join("link")).unwrap();
    xattr::set(src.join("link"), "user.test", b"val").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            xattrs: true,
            links: true,
            ..Default::default()
        },
    )
    .unwrap();
    let val = xattr::get(dst.join("link"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[cfg(feature = "acl")]
#[test]
fn acls_roundtrip() {
    if !tests::requires_capability(tests::CapabilityCheck::Acls) {
        return;
    }
    use posix_acl::{ACL_READ, PosixACL, Qualifier};

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();

    let mut acl = match PosixACL::read_acl(&file) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    acl.set(Qualifier::User(12345), ACL_READ);
    if let Err(_) = acl.write_acl(&file) {
        eprintln!("Skipping acls_roundtrip test: ACLs not supported");
        return;
    }

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    if let Err(_) = dacl.write_default_acl(&src) {
        eprintln!("Skipping acls_roundtrip test: ACLs not supported");
        return;
    }

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            acls: true,
            ..Default::default()
        },
    )
    .unwrap();

    let acl_src = match PosixACL::read_acl(&file) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    let acl_dst = match PosixACL::read_acl(&dst.join("file")) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let dacl_src = match PosixACL::read_default_acl(&src) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    let dacl_dst = match PosixACL::read_default_acl(&dst) {
        Ok(a) => a,
        Err(_) => {
            eprintln!("Skipping acls_roundtrip test: ACLs not supported");
            return;
        }
    };
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
}

#[cfg(feature = "acl")]
#[test]
fn acls_imply_perms() {
    if !tests::requires_capability(tests::CapabilityCheck::Acls) {
        return;
    }
    use posix_acl::{ACL_READ, PosixACL, Qualifier};
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o640)).unwrap();
    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&file).unwrap();

    let dst_file = dst.join("file");
    fs::write(&dst_file, b"junk").unwrap();
    fs::set_permissions(&dst_file, fs::Permissions::from_mode(0o600)).unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            acls: true,
            perms: false,
            ..Default::default()
        },
    )
    .unwrap();

    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(dst.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
}

#[test]
fn devices_roundtrip() {
    if !tests::requires_capability(tests::CapabilityCheck::CapMknod) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("null");
    #[allow(clippy::useless_conversion)]
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 3).try_into().unwrap(),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("null")).unwrap();
    assert!(meta.file_type().is_char_device());
    assert_eq!(meta.rdev(), meta::makedev(1, 3));
}

#[test]
fn copy_devices_creates_regular_files() {
    if !tests::requires_capability(tests::CapabilityCheck::CapMknod) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("null");
    #[allow(clippy::useless_conversion)]
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 3).try_into().unwrap(),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            copy_devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("null")).unwrap();
    assert!(meta.is_file());
    assert_eq!(meta.len(), 0);
    assert!(!meta.file_type().is_char_device());
}

#[test]
fn copy_devices_handles_zero() {
    if !tests::requires_capability(tests::CapabilityCheck::CapMknod) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("zero");
    #[allow(clippy::useless_conversion)]
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 5).try_into().unwrap(),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            copy_devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("zero")).unwrap();
    assert!(meta.is_file());
    assert_eq!(meta.len(), 0);
}

#[test]
fn specials_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let fifo = src.join("fifo");
    mkfifo(&fifo, Mode::from_bits_truncate(0o600)).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            specials: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("fifo")).unwrap();
    assert!(meta.file_type().is_fifo());
}

#[test]
fn sparse_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sp = src.join("sparse");
    {
        let mut f = File::create(&sp).unwrap();
        f.seek(SeekFrom::Start(1 << 20)).unwrap();
        f.write_all(b"end").unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&sp).unwrap();
    let dst_meta = fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
    assert!(src_meta.blocks() * 512 < src_meta.len());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
}

#[test]
fn sparse_creation_from_zeros() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let zs = src.join("zeros");
    {
        let mut f = File::create(&zs).unwrap();
        f.write_all(&vec![0u8; 1 << 20]).unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&zs).unwrap();
    let dst_meta = fs::metadata(dst.join("zeros")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert!(dst_meta.blocks() < src_meta.blocks());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
}

#[test]
fn sparse_middle_hole() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sp = src.join("sparse");
    {
        let mut f = File::create(&sp).unwrap();
        f.write_all(b"start").unwrap();
        f.seek(SeekFrom::Start((1 << 20) + 5)).unwrap();
        f.write_all(b"end").unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&sp).unwrap();
    if src_meta.blocks() * 512 >= src_meta.len() {
        eprintln!("skipping test: filesystem lacks sparse-file support");
        return;
    }
    let dst_meta = fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
}

#[test]
fn sparse_trailing_hole() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let sp = src.join("sparse");
    {
        let mut f = File::create(&sp).unwrap();
        f.write_all(b"start").unwrap();
        f.set_len(1 << 20).unwrap();
    }
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            sparse: true,
            ..Default::default()
        },
    )
    .unwrap();
    let src_meta = fs::metadata(&sp).unwrap();
    if src_meta.blocks() * 512 >= src_meta.len() {
        eprintln!("skipping test: filesystem lacks sparse-file support");
        return;
    }
    let dst_meta = fs::metadata(dst.join("sparse")).unwrap();
    assert_eq!(src_meta.len(), dst_meta.len());
    assert_eq!(src_meta.blocks(), dst_meta.blocks());
    assert!(dst_meta.blocks() * 512 < dst_meta.len());
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

#[cfg(feature = "xattr")]
#[test]
fn fake_super_stores_xattrs() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            perms: true,
            fake_super: true,
            ..Default::default()
        },
    )
    .unwrap();
    let dst_file = dst.join("file");
    assert!(xattr::get(&dst_file, "user.rsync.uid").unwrap().is_some());
}

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn super_overrides_fake_super() {
    if !tests::requires_capability(tests::CapabilityCheck::CapChown) {
        return;
    }
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            perms: true,
            fake_super: true,
            super_user: true,
            ..Default::default()
        },
    )
    .unwrap();
    let dst_file = dst.join("file");
    assert!(xattr::get(&dst_file, "user.rsync.uid").unwrap().is_none());
}

#[cfg(feature = "xattr")]
#[test]
fn xattrs_roundtrip_fake_super() {
    if let Err(e) = engine::xattrs::ensure_supported() {
        println!("Skipping test: {e}");
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            xattrs: true,
            ..Default::default()
        },
    )
    .unwrap();
    let val = xattr::get(dst.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[cfg(feature = "acl")]
#[test]
fn acls_roundtrip_default_acl() {
    if !tests::requires_capability(tests::CapabilityCheck::Acls) {
        return;
    }
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ | ACL_WRITE);
    acl.write_acl(&file).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        &available_codecs(),
        &SyncOptions {
            acls: true,
            ..Default::default()
        },
    )
    .unwrap();
    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(dst.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
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
