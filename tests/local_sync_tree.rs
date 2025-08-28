use assert_cmd::Command;
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "linux")]
use std::os::unix::fs::FileTypeExt;
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
    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args(["--local", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(collect(&src), collect(&dst));
}

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn sync_preserves_xattrs() {
    // Ensure extended attributes survive a local sync when the feature is enabled.
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dst).unwrap();
    let file = src.join("file");
    fs::write(&file, b"hi").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args(["--local", "--xattrs", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let val = xattr::get(dst.join("file"), "user.test").unwrap().unwrap();
    assert_eq!(&val[..], b"val");
}

#[cfg(all(unix, feature = "acl"))]
#[test]
fn sync_preserves_acls() {
    use posix_acl::{PosixACL, Qualifier, ACL_READ};

    // Ensure POSIX ACLs survive a local sync when the feature is enabled.

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

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args(["--local", "--acls", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let acl_src = PosixACL::read_acl(&file).unwrap();
    let acl_dst = PosixACL::read_acl(&dst.join("file")).unwrap();
    assert_eq!(acl_src.entries(), acl_dst.entries());
}

#[cfg(target_os = "linux")]
#[test]
fn sync_preserves_device_nodes() {
    use nix::sys::stat::{makedev, mknod, Mode, SFlag};

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

    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args(["--local", "--devices", &src_arg, dst.to_str().unwrap()])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let meta = fs::symlink_metadata(dst.join("null")).unwrap();
    assert!(meta.file_type().is_char_device());
    assert_eq!(meta.rdev(), makedev(1, 3));
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
    Command::cargo_bin("rsync-rs")
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
