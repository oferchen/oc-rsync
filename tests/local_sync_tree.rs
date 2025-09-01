// tests/local_sync_tree.rs

use assert_cmd::Command;
#[cfg(unix)]
use filetime::{set_file_atime, FileTime};
#[cfg(unix)]
use meta::mkfifo;
#[cfg(unix)]
use nix::sys::stat::Mode;
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
#[cfg(unix)]
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn collect(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, root: &Path, map: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(base).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit(&path, root, map);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_path_buf();
                map.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut map = BTreeMap::new();
    visit(dir, dir, &mut map);
    map
}

#[test]
fn sync_directory_tree() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("nested")).unwrap();
    fs::write(src.join("a.txt"), b"alpha").unwrap();
    fs::write(src.join("nested/b.txt"), b"beta").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(collect(&src), collect(&dst));
}

#[cfg(unix)]
#[test]
fn sync_replaces_symlinked_dir_by_default() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    let target = tmp.path().join("target");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub/file"), b"data").unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&dst).unwrap();
    symlink(&target, dst.join("sub")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    let meta = fs::symlink_metadata(dst.join("sub")).unwrap();
    assert!(meta.file_type().is_dir());
    assert!(!meta.file_type().is_symlink());
    assert!(!target.join("file").exists());
    assert!(dst.join("sub/file").exists());
}

#[cfg(unix)]
#[test]
fn sync_keep_dirlinks_preserves_symlinked_dir() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    let target = tmp.path().join("target");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub/file"), b"data").unwrap();
    fs::create_dir_all(&target).unwrap();
    fs::create_dir_all(&dst).unwrap();
    symlink(&target, dst.join("sub")).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--keep-dirlinks",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let meta = fs::symlink_metadata(dst.join("sub")).unwrap();
    assert!(meta.file_type().is_symlink());
    assert!(target.join("file").exists());
    assert!(!dst.join("sub/file").exists());
}

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn sync_preserves_xattrs() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--xattrs", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let val = xattr::get(dst.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn sync_preserves_symlink_xattrs() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    symlink("file", src.join("link")).unwrap();
    xattr::set(src.join("link"), "user.test", b"val").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--links",
            "--xattrs",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let val = xattr::get(dst.join("link"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[cfg(all(unix, feature = "acl"))]
#[test]
fn sync_preserves_acls() {
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

    let mut dacl = PosixACL::read_default_acl(&src).unwrap();
    dacl.set(Qualifier::User(12345), ACL_READ);
    dacl.write_default_acl(&src).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--acls", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(dst.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());

    let dacl_src = PosixACL::read_default_acl(&src).unwrap();
    let dacl_dst = PosixACL::read_default_acl(&dst).unwrap();
    assert_eq!(dacl_src.entries(), dacl_dst.entries());
}

#[cfg(unix)]
#[test]
fn sync_preserves_owner_and_group() {
    use nix::errno::Errno;
    use nix::unistd::{chown, Gid, Uid};

    if !Uid::effective().is_root() {
        eprintln!("not running as root; skipping");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    if let Err(err) = chown(
        &file,
        Some(Uid::from_raw(12345)),
        Some(Gid::from_raw(54321)),
    ) {
        if err == Errno::EPERM {
            eprintln!("chown EPERM; skipping");
            return;
        }
        panic!("chown failed: {err}");
    }

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--owner",
            "--group",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let meta = fs::metadata(dst.join("file")).unwrap();
    assert_eq!(meta.uid(), 12345);
    assert_eq!(meta.gid(), 54321);
}

#[cfg(unix)]
#[test]
fn sync_applies_chmod() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--perms",
            "--chmod=g+r",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let mode = fs::metadata(dst.join("file")).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o640);
}

#[cfg(unix)]
#[test]
fn sync_preserves_executability() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--executability",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let src_mode = fs::metadata(file).unwrap().permissions().mode();
    let dst_mode = fs::metadata(dst.join("file")).unwrap().permissions().mode();
    assert_eq!(dst_mode & 0o111, src_mode & 0o111);
    assert_ne!(dst_mode & !0o111, src_mode & !0o111);
}

#[cfg(unix)]
#[test]
fn sync_preserves_crtimes() {
    use filetime::FileTime;

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let src_meta = fs::metadata(&file).unwrap();
    let src_crtime = match FileTime::from_creation_time(&src_meta) {
        Some(t) => t,
        None => return,
    };

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--crtimes", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let dst_meta = fs::metadata(dst.join("file")).unwrap();
    if let Some(t) = FileTime::from_creation_time(&dst_meta) {
        if cfg!(target_os = "linux") && t != src_crtime {
            return;
        }
        assert_eq!(src_crtime, t)
    }
}

#[cfg(unix)]
#[test]
fn sync_preserves_atimes() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    let atime = FileTime::from_unix_time(1_000_000, 123_456_789);
    set_file_atime(&file, atime).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--atimes", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let meta = fs::metadata(dst.join("file")).unwrap();
    let dst_atime = FileTime::from_last_access_time(&meta);
    assert_eq!(dst_atime, atime);
}

#[cfg(target_os = "linux")]
#[test]
fn sync_creates_device_nodes() {
    use nix::errno::Errno;
    use nix::sys::stat::{major, minor, Mode, SFlag};
    use nix::unistd::Uid;

    if !Uid::effective().is_root() {
        eprintln!("skipping: test requires root to create device nodes");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let cdev = src.join("char");
    if let Err(err) = meta::mknod(
        &cdev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 3),
    ) {
        if err.raw_os_error() == Some(Errno::EPERM as i32) {
            eprintln!("skipping: failed to create char device: {err}");
            return;
        }
        panic!("failed to create char device: {err}");
    }
    let bdev = src.join("block");
    if let Err(err) = meta::mknod(
        &bdev,
        SFlag::S_IFBLK,
        Mode::from_bits_truncate(0o600),
        meta::makedev(8, 1),
    ) {
        if err.raw_os_error() == Some(Errno::EPERM as i32) {
            eprintln!("skipping: failed to create block device: {err}");
            return;
        }
        panic!("failed to create block device: {err}");
    }

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--devices", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let meta = fs::symlink_metadata(dst.join("char")).unwrap();
    assert!(meta.file_type().is_char_device());
    assert_eq!(meta.mode() & 0o777, 0o600);
    let rdev = meta.rdev();
    assert_eq!(rdev, meta::makedev(1, 3));
    assert_eq!(major(rdev), 1);
    assert_eq!(minor(rdev), 3);

    let meta = fs::symlink_metadata(dst.join("block")).unwrap();
    assert!(meta.file_type().is_block_device());
    assert_eq!(meta.mode() & 0o777, 0o600);
    let rdev = meta.rdev();
    assert_eq!(rdev, meta::makedev(8, 1));
    assert_eq!(major(rdev), 8);
    assert_eq!(minor(rdev), 1);
}

#[cfg(target_os = "linux")]
#[test]
fn sync_copies_device_contents() {
    use nix::errno::Errno;
    use nix::sys::stat::{Mode, SFlag};
    use nix::unistd::Uid;

    if !Uid::effective().is_root() {
        eprintln!("skipping: test requires root to create device nodes");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = src.join("null");
    if let Err(err) = meta::mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 3),
    ) {
        if err.raw_os_error() == Some(Errno::EPERM as i32) {
            eprintln!("skipping: failed to create char device: {err}");
            return;
        }
        panic!("failed to create char device: {err}");
    }

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--copy-devices", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let meta = fs::symlink_metadata(dst.join("null")).unwrap();
    assert!(!meta.file_type().is_char_device());
    assert!(meta.is_file());
    assert_eq!(meta.len(), 0);
}

#[cfg(target_os = "linux")]
#[test]
fn sync_deletes_device_nodes() {
    use nix::errno::Errno;
    use nix::sys::stat::{Mode, SFlag};
    use nix::unistd::Uid;

    if !Uid::effective().is_root() {
        eprintln!("skipping: test requires root to create device nodes");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let dev = dst.join("null");
    if let Err(err) = meta::mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        meta::makedev(1, 3),
    ) {
        if err.raw_os_error() == Some(Errno::EPERM as i32) {
            eprintln!("skipping: failed to create char device: {err}");
            return;
        }
        panic!("failed to create char device: {err}");
    }

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--delete", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success();

    assert!(!dst.join("null").exists());
}

#[cfg(unix)]
#[test]
fn sync_preserves_fifos() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let fifo = src.join("fifo");
    mkfifo(&fifo, Mode::from_bits_truncate(0o600)).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--specials", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let meta = fs::symlink_metadata(dst.join("fifo")).unwrap();
    assert!(meta.file_type().is_fifo());
}

#[cfg(unix)]
#[test]
fn sync_preserves_hard_links() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let f1 = src.join("f1");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("f2");
    fs::hard_link(&f1, &f2).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--hard-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let m1 = fs::metadata(dst.join("f1")).unwrap();
    let m2 = fs::metadata(dst.join("f2")).unwrap();
    assert_eq!(m1.ino(), m2.ino());
}

#[cfg(unix)]
#[test]
fn sync_preserves_multiple_hard_links() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let f1 = src.join("f1");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("f2");
    fs::hard_link(&f1, &f2).unwrap();
    let f3 = src.join("f3");
    fs::hard_link(&f1, &f3).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--hard-links",
            "--delay-updates",
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let m1 = fs::metadata(dst.join("f1")).unwrap().ino();
    let m2 = fs::metadata(dst.join("f2")).unwrap().ino();
    let m3 = fs::metadata(dst.join("f3")).unwrap().ino();
    assert_eq!(m1, m2);
    assert_eq!(m1, m3);
}

#[cfg(unix)]
#[test]
fn sync_preserves_hard_links_with_link_dest() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let link = tmp.path().join("link");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&link).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let f1 = src.join("f1");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("f2");
    fs::hard_link(&f1, &f2).unwrap();
    fs::write(link.join("f1"), b"hi").unwrap();
    fs::write(link.join("f2"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--hard-links",
            "--link-dest",
            link.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    use std::os::unix::fs::MetadataExt;
    let link1 = fs::metadata(link.join("f1")).unwrap().ino();
    let link2 = fs::metadata(link.join("f2")).unwrap().ino();
    assert_ne!(link1, link2);
    let m1 = fs::metadata(dst.join("f1")).unwrap().ino();
    let m2 = fs::metadata(dst.join("f2")).unwrap().ino();
    assert_eq!(m1, link1);
    assert_eq!(m1, m2);
}

#[cfg(unix)]
#[test]
fn sync_preserves_hard_links_across_dirs() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(src.join("a")).unwrap();
    fs::create_dir_all(src.join("b")).unwrap();
    fs::create_dir_all(src.join("c")).unwrap();
    fs::create_dir_all(&dst).unwrap();

    let f1 = src.join("a/f1");
    fs::write(&f1, b"hi").unwrap();
    let f2 = src.join("b/f2");
    fs::hard_link(&f1, &f2).unwrap();
    let f3 = src.join("c/f3");
    fs::hard_link(&f1, &f3).unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--hard-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let ino1 = fs::metadata(dst.join("a/f1")).unwrap().ino();
    let ino2 = fs::metadata(dst.join("b/f2")).unwrap().ino();
    let ino3 = fs::metadata(dst.join("c/f3")).unwrap().ino();
    assert_eq!(ino1, ino2);
    assert_eq!(ino1, ino3);
}

#[cfg(unix)]
#[test]
fn sync_preserves_symlinks() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    let _ = symlink("file", src.join("link"));

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let meta = fs::symlink_metadata(dst.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    assert_eq!(
        fs::read_link(dst.join("link")).unwrap(),
        PathBuf::from("file")
    );
}

#[cfg(unix)]
#[test]
fn sync_follows_symlinks() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    let _ = symlink("file", src.join("link"));

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--local", "--copy-links", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert!(dst.join("link").is_file());
    assert_eq!(fs::read(dst.join("link")).unwrap(), b"hi");
}
