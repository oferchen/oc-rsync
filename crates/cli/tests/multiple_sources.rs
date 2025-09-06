// crates/cli/tests/multiple_sources.rs
use assert_cmd::Command;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn collect(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, map: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, map);
            } else {
                let rel = path.strip_prefix(base).unwrap().to_path_buf();
                let data = fs::read(&path).unwrap();
                map.insert(rel, data);
            }
        }
    }
    let mut map = BTreeMap::new();
    walk(root, root, &mut map);
    map
}

#[test]
fn multiple_source_parity() {
    let dir = tempdir().unwrap();
    let src1 = dir.path().join("src1");
    let src2 = dir.path().join("src2");
    let dst_ours = dir.path().join("dst_ours");
    fs::create_dir_all(&src1).unwrap();
    fs::create_dir_all(&src2).unwrap();
    fs::write(src1.join("a.txt"), b"a").unwrap();
    fs::write(src2.join("b.txt"), b"b").unwrap();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "-r",
            &format!("{}/", src1.display()),
            &format!("{}/", src2.display()),
            dst_ours.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut expected = BTreeMap::new();
    expected.extend(collect(&src1));
    expected.extend(collect(&src2));
    let ours = collect(&dst_ours);
    assert_eq!(expected, ours);
}
