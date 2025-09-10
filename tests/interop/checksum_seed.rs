// tests/interop/checksum_seed.rs
#![cfg(feature = "interop")]
use assert_cmd::Command;
use filetime::FileTime;
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[derive(Debug, PartialEq, Eq)]
struct Collected {
    data: Vec<u8>,
    mtime: FileTime,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    xattrs: BTreeMap<String, Vec<u8>>,
}

fn gather(path: &Path, meta: &fs::Metadata) -> Collected {
    #[cfg(unix)]
    let mode = meta.permissions().mode();
    let data = if meta.is_file() {
        fs::read(path).unwrap()
    } else if meta.file_type().is_symlink() {
        fs::read_link(path)
            .unwrap()
            .as_os_str()
            .to_string_lossy()
            .into_owned()
            .into_bytes()
    } else {
        Vec::new()
    };
    let mtime = FileTime::from_last_modification_time(meta);
    #[cfg(unix)]
    let mut xattrs = BTreeMap::new();
    #[cfg(unix)]
    if let Ok(names) = xattr::list(path) {
        for name in names {
            let key = name.to_string_lossy().into_owned();
            if let Ok(Some(val)) = xattr::get(path, &key) {
                xattrs.insert(key, val);
            }
        }
    }
    Collected {
        data,
        mtime,
        #[cfg(unix)]
        mode,
        #[cfg(unix)]
        xattrs,
    }
}

fn collect(dir: &Path) -> BTreeMap<PathBuf, Collected> {
    fn visit(base: &Path, root: &Path, map: &mut BTreeMap<PathBuf, Collected>) {
        for entry in fs::read_dir(base).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            if meta.is_dir() {
                map.insert(rel.clone(), gather(&path, &meta));
                visit(&path, root, map);
            } else {
                map.insert(rel, gather(&path, &meta));
            }
        }
    }
    let mut map = BTreeMap::new();
    visit(dir, dir, &mut map);
    map
}

fn oc_rsync() -> Command {
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.env("LC_ALL", "C").env("TZ", "UTC");
    cmd
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

    oc_rsync()
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
