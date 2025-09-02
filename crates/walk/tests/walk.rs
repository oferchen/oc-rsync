// crates/walk/tests/walk.rs
use std::fs;
use tempfile::tempdir;
use walk::{walk, walk_with_max_size};

#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;

#[test]
fn walk_includes_files_dirs_and_symlinks() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join("dir")).unwrap();
    fs::write(root.join("dir/large.txt"), vec![0u8; 2048]).unwrap();
    fs::write(root.join("small.txt"), b"hi").unwrap();
    let link_path = root.join("link.txt");
    #[cfg(unix)]
    symlink(root.join("small.txt"), &link_path).unwrap();
    #[cfg(windows)]
    symlink_file(root.join("small.txt"), &link_path).unwrap();

    let mut entries = Vec::new();
    let mut state = String::new();
    for batch in walk(root, 10, true, false) {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            entries.push((path, e.file_type));
        }
    }
    assert!(entries.iter().any(|(p, t)| p == &root && t.is_dir()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &root.join("dir") && t.is_dir()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &root.join("dir/large.txt") && t.is_file()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &link_path && t.is_symlink()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &root.join("small.txt") && t.is_file()));
}

#[test]
fn walk_preserves_order_and_bounds_batches() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join("dir")).unwrap();
    fs::create_dir(root.join("dir/sub")).unwrap();
    fs::write(root.join("dir/big1"), vec![0u8; 2048]).unwrap();
    fs::write(root.join("dir/sub/big2"), vec![0u8; 2048]).unwrap();
    fs::write(root.join("dir/small1"), b"a").unwrap();
    fs::write(root.join("top_small.txt"), b"c").unwrap();

    let mut paths = Vec::new();
    let mut state = String::new();
    let mut walker = walk(root, 2, false, false);
    while let Some(batch) = walker.next() {
        let batch = batch.unwrap();
        assert!(batch.len() <= 2);
        for e in batch {
            let path = e.apply(&mut state);
            paths.push(path);
        }
    }

    let expected = vec![
        root.to_path_buf(),
        root.join("dir"),
        root.join("dir/big1"),
        root.join("dir/small1"),
        root.join("dir/sub"),
        root.join("dir/sub/big2"),
        root.join("top_small.txt"),
    ];
    assert_eq!(paths, expected);
}

#[test]
fn walk_skips_files_over_threshold() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("small.txt"), b"ok").unwrap();
    fs::write(root.join("big.txt"), vec![0u8; 2048]).unwrap();

    let mut paths = Vec::new();
    let mut state = String::new();
    for batch in walk_with_max_size(root, 10, 1024, false, false) {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            paths.push(path);
        }
    }

    assert!(paths.contains(&root.to_path_buf()));
    assert!(paths.contains(&root.join("small.txt")));
    assert!(!paths.contains(&root.join("big.txt")));
}

#[cfg(unix)]
#[test]
fn walk_skips_cross_device_entries() {
    use std::path::Path;

    let root = Path::new("/dev");

    let mut state = String::new();
    let mut found_pts = false;
    let mut walker = walk(root, 100, false, false);
    while let Some(batch) = walker.next() {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            if path.starts_with("/dev/pts") {
                found_pts = true;
                break;
            }
        }
        if found_pts {
            break;
        }
    }
    assert!(found_pts, "expected to see /dev/pts without restriction");

    let mut walker = walk(root, 100, false, true);
    state.clear();
    while let Some(batch) = walker.next() {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            assert!(!path.starts_with("/dev/pts"));
        }
    }
}
