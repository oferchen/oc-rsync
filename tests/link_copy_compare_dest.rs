use assert_cmd::Command;
use std::collections::BTreeMap;
use std::fs;
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

#[cfg(unix)]
#[test]
fn link_dest_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let link = tmp.path().join("link");
    let dst_rr = tmp.path().join("dst_rr");
    let dst_rsync = tmp.path().join("dst_rsync");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&link).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    fs::write(link.join("file"), b"hi").unwrap();

    Command::new("rsync")
        .args([
            "-a",
            "--link-dest",
            link.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rsync.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--local",
            "--link-dest",
            link.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(collect(&dst_rsync), collect(&dst_rr));
    use std::os::unix::fs::MetadataExt;
    let base = fs::metadata(link.join("file")).unwrap().ino();
    let rsync_meta = fs::metadata(dst_rsync.join("file")).unwrap().ino();
    let rr_meta = fs::metadata(dst_rr.join("file")).unwrap().ino();
    assert_eq!(base, rsync_meta);
    assert_eq!(base, rr_meta);
}

#[cfg(unix)]
#[test]
fn copy_dest_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let copy = tmp.path().join("copy");
    let dst_rr = tmp.path().join("dst_rr");
    let dst_rsync = tmp.path().join("dst_rsync");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&copy).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    fs::write(copy.join("file"), b"hi").unwrap();

    Command::new("rsync")
        .args([
            "-a",
            "--copy-dest",
            copy.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rsync.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--local",
            "--copy-dest",
            copy.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(collect(&dst_rsync), collect(&dst_rr));
    use std::os::unix::fs::MetadataExt;
    let base = fs::metadata(copy.join("file")).unwrap().ino();
    let rsync_meta = fs::metadata(dst_rsync.join("file")).unwrap().ino();
    let rr_meta = fs::metadata(dst_rr.join("file")).unwrap().ino();
    assert_ne!(base, rsync_meta);
    assert_ne!(base, rr_meta);
}

#[test]
fn compare_dest_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let cmp = tmp.path().join("cmp");
    let dst_rr = tmp.path().join("dst_rr");
    let dst_rsync = tmp.path().join("dst_rsync");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&cmp).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    fs::write(cmp.join("file"), b"hi").unwrap();

    Command::new("rsync")
        .args([
            "-a",
            "--compare-dest",
            cmp.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rsync.to_str().unwrap(),
        ])
        .assert()
        .success();

    Command::cargo_bin("rsync-rs")
        .unwrap()
        .args([
            "--local",
            "--compare-dest",
            cmp.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(collect(&dst_rsync), collect(&dst_rr));
    assert!(!dst_rsync.join("file").exists());
    assert!(!dst_rr.join("file").exists());
}
