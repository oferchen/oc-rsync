#![cfg(unix)]

use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::process::Command;

use compress::available_codecs;
use engine::{sync, SyncOptions};
use filetime::{set_file_atime, set_file_mtime, FileTime};
use filters::Matcher;
use nix::sys::stat::{makedev, mknod, Mode, SFlag};
use nix::unistd::{chown, mkfifo, Gid, Uid};
use tempfile::tempdir;

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
        available_codecs(),
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
fn times_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 0);
    set_file_mtime(&file, mtime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
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
fn atimes_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let atime = FileTime::from_unix_time(1_000_000, 0);
    set_file_atime(&file, atime).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
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
    assert_eq!(dst_atime, src_atime);
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
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
        Err(_) => return, // platform does not support creation time
    };

    std::thread::sleep(Duration::from_secs(1));

    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            crtimes: true,
            ..Default::default()
        },
    )
    .unwrap();

    let dst_meta = fs::metadata(dst.join("file")).unwrap();
    match dst_meta.created() {
        Ok(t) => assert_eq!(t, src_crtime),
        Err(_) => return,
    }
}

#[test]
fn owner_group_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    chown(&file, Some(Uid::from_raw(1000)), Some(Gid::from_raw(1000))).unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            owner: true,
            group: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.uid(), 1000);
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
        available_codecs(),
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
        available_codecs(),
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
        available_codecs(),
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
fn acls_roundtrip() {
    use posix_acl::{PosixACL, Qualifier, ACL_READ};

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();

    let mut acl = PosixACL::read_acl(&file).unwrap();
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.write_acl(&file).unwrap();

    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            acls: true,
            ..Default::default()
        },
    )
    .unwrap();

    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(&dst.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
}

#[test]
fn devices_roundtrip() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("null");
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        makedev(1, 3),
    )
    .unwrap();
    sync(
        &src,
        &dst,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            devices: true,
            ..Default::default()
        },
    )
    .unwrap();
    let meta = fs::symlink_metadata(dst.join("null")).unwrap();
    assert!(meta.file_type().is_char_device());
    assert_eq!(meta.rdev(), makedev(1, 3));
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
        available_codecs(),
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
        available_codecs(),
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
        available_codecs(),
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
fn metadata_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst_rrs = tmp.path().join("rrs");
    let dst_rsync = tmp.path().join("rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst_rrs).unwrap();
    fs::create_dir_all(&dst_rsync).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let mtime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_file_mtime(&file, mtime).unwrap();

    sync(
        &src,
        &dst_rrs,
        &Matcher::default(),
        available_codecs(),
        &SyncOptions {
            owner: true,
            group: true,
            times: true,
            crtimes: true,
            ..Default::default()
        },
    )
    .unwrap();

    let mut cmd = Command::new("rsync");
    cmd.arg("-a");
    let ver = Command::new("rsync").arg("--version").output().unwrap();
    let ver_str = String::from_utf8_lossy(&ver.stdout);
    let cr_supported = !ver_str.contains("no crtimes");
    if cr_supported {
        cmd.arg("--crtimes");
    }
    cmd.arg(format!("{}/", src.display()));
    cmd.arg(dst_rsync.to_str().unwrap());
    assert!(cmd.status().unwrap().success());

    let meta_rrs = fs::metadata(dst_rrs.join("file")).unwrap();
    let meta_rsync = fs::metadata(dst_rsync.join("file")).unwrap();
    assert_eq!(meta_rrs.uid(), meta_rsync.uid());
    assert_eq!(meta_rrs.gid(), meta_rsync.gid());
    let mt_rrs = FileTime::from_last_modification_time(&meta_rrs);
    let mt_rsync = FileTime::from_last_modification_time(&meta_rsync);
    assert_eq!(mt_rrs, mt_rsync);
    if cr_supported {
        let cr_rrs = meta_rrs.created().ok();
        let cr_rsync = meta_rsync.created().ok();
        if cr_rrs.is_some() && cr_rsync.is_some() {
            assert_eq!(cr_rrs, cr_rsync);
        }
    }
}
