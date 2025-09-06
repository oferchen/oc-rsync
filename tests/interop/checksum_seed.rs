// tests/interop/checksum_seed.rs
use assert_cmd::Command;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
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
#[ignore = "requires rsync"]
fn checksum_seed_matches_upstream() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir(&src).unwrap();
    fs::write(src.join("file.txt"), b"data").unwrap();

    let oc_dst = tmp.path().join("oc");
    let rs_dst = tmp.path().join("rs");
    fs::create_dir(&oc_dst).unwrap();
    fs::create_dir(&rs_dst).unwrap();

    let src_arg = format!("{}/", src.display());

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--checksum-seed=1",
            "-r",
            &src_arg,
            oc_dst.to_str().unwrap(),
        ])
        .assert()
        .success();

    let status = StdCommand::new("rsync")
        .args([
            "--checksum-seed=1",
            "-r",
            &src_arg,
            rs_dst.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    assert_eq!(collect(&oc_dst), collect(&rs_dst));
}
