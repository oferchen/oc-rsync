// tests/link_copy_compare_dest.rs

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

fn load_golden(name: &str) -> BTreeMap<PathBuf, Vec<u8>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/link_copy_compare_dest")
        .join(format!("{name}.txt"));
    let mut map = BTreeMap::new();
    for line in fs::read_to_string(path).unwrap().lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, ':');
        let rel = PathBuf::from(parts.next().unwrap());
        let data = parts.next().unwrap().as_bytes().to_vec();
        map.insert(rel, data);
    }
    map
}

#[cfg(unix)]
#[test]
fn link_dest_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let link = tmp.path().join("link");
    let dst_rr = tmp.path().join("dst_rr");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&link).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    fs::write(link.join("file"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--link-dest",
            link.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(load_golden("link_dest"), collect(&dst_rr));
    use std::os::unix::fs::MetadataExt;
    let base = fs::metadata(link.join("file")).unwrap().ino();
    let rr_meta = fs::metadata(dst_rr.join("file")).unwrap().ino();
    assert_eq!(base, rr_meta);
}

#[cfg(unix)]
#[test]
fn copy_dest_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let copy = tmp.path().join("copy");
    let dst_rr = tmp.path().join("dst_rr");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&copy).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    fs::write(copy.join("file"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--copy-dest",
            copy.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(load_golden("copy_dest"), collect(&dst_rr));
    use std::os::unix::fs::MetadataExt;
    let base = fs::metadata(copy.join("file")).unwrap().ino();
    let rr_meta = fs::metadata(dst_rr.join("file")).unwrap().ino();
    assert_ne!(base, rr_meta);
}

#[test]
fn compare_dest_matches_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let cmp = tmp.path().join("cmp");
    let dst_rr = tmp.path().join("dst_rr");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&cmp).unwrap();
    fs::write(src.join("file"), b"hi").unwrap();
    fs::write(cmp.join("file"), b"hi").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--compare-dest",
            cmp.to_str().unwrap(),
            &format!("{}/", src.display()),
            dst_rr.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(load_golden("compare_dest"), collect(&dst_rr));
    assert!(!dst_rr.join("file").exists());
}
