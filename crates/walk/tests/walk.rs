// crates/walk/tests/walk.rs
use std::fs;
use tempfile::tempdir;
use walk::{walk, walk_with_max_size};

#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;
#[cfg(windows)]
use walk::normalize_path;

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
    for batch in walk(root, 10, true, false).unwrap() {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            entries.push((path, e.file_type));
        }
    }
    assert!(entries.iter().any(|(p, t)| p == root && t.is_dir()));
    assert!(
        entries
            .iter()
            .any(|(p, t)| p.as_path() == root.join("dir").as_path() && t.is_dir())
    );
    assert!(
        entries
            .iter()
            .any(|(p, t)| p.as_path() == root.join("dir/large.txt").as_path() && t.is_file())
    );
    assert!(
        entries
            .iter()
            .any(|(p, t)| p == link_path.as_path() && t.is_symlink())
    );
    assert!(
        entries
            .iter()
            .any(|(p, t)| p.as_path() == root.join("small.txt").as_path() && t.is_file())
    );
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
    for batch in walk(root, 2, false, false).unwrap() {
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
    for batch in walk_with_max_size(root, 10, 1024, false, false).unwrap() {
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
    for batch in walk(root, 100, false, false).unwrap() {
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

    state.clear();
    for batch in walk(root, 100, false, true).unwrap() {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            assert!(!path.starts_with("/dev/pts"));
        }
    }
}

#[test]
fn walk_skips_ignored_directory() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join("keep")).unwrap();
    fs::write(root.join("keep/file.txt"), b"k").unwrap();
    fs::create_dir(root.join("skip")).unwrap();
    fs::write(root.join("skip/ignored.txt"), b"i").unwrap();

    let mut walker = walk(root, 1, false, false).unwrap();
    let mut state = String::new();
    let mut seen = Vec::new();
    while let Some(batch) = walker.next() {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            if path == root.join("skip") {
                walker.skip_current_dir();
            }
            seen.push(path);
        }
    }
    assert!(seen.contains(&root.join("keep")));
    assert!(seen.contains(&root.join("keep/file.txt")));
    assert!(seen.contains(&root.join("skip")));
    assert!(!seen.contains(&root.join("skip/ignored.txt")));
}

#[cfg(unix)]
#[test]
fn walk_detects_hardlinks() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("base.txt"), b"x").unwrap();
    fs::hard_link(root.join("base.txt"), root.join("hl.txt")).unwrap();

    let mut entries = Vec::new();
    let mut walker = walk(root, 10, false, false).unwrap();
    let mut state = String::new();
    while let Some(batch) = walker.next() {
        let batch = batch.unwrap();
        for e in batch {
            let path = e.apply(&mut state);
            entries.push((path, e.dev, e.inode));
        }
    }
    let base = root.join("base.txt");
    let hl = root.join("hl.txt");
    let dev_ino_base = entries
        .iter()
        .find(|(p, _, _)| p == &base)
        .map(|(_, d, i)| (*d, *i))
        .unwrap();
    let dev_ino_hl = entries
        .iter()
        .find(|(p, _, _)| p == &hl)
        .map(|(_, d, i)| (*d, *i))
        .unwrap();
    assert_eq!(dev_ino_base, dev_ino_hl);
}

#[cfg(windows)]
#[test]
fn normalize_path_handles_verbatim_prefix() {
    let tmp = tempdir().unwrap();
    let norm = normalize_path(tmp.path());
    let s = norm.to_str().unwrap();
    assert!(s.starts_with(r"\\?\\"));
    let again = normalize_path(&norm);
    assert_eq!(norm, again);
}
