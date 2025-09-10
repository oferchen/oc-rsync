// crates/cli/tests/multiple_sources.rs
use assert_cmd::Command;
use filetime::FileTime;
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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

fn collect(root: &Path) -> BTreeMap<PathBuf, Collected> {
    fn walk(base: &Path, dir: &Path, map: &mut BTreeMap<PathBuf, Collected>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).unwrap();
            let rel = path.strip_prefix(base).unwrap().to_path_buf();
            if meta.is_dir() {
                map.insert(rel.clone(), gather(&path, &meta));
                walk(base, &path, map);
            } else {
                map.insert(rel, gather(&path, &meta));
            }
        }
    }
    let mut map = BTreeMap::new();
    walk(root, root, &mut map);
    map
}

fn oc_rsync() -> Command {
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.env("LC_ALL", "C").env("TZ", "UTC");
    cmd
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

    oc_rsync()
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
