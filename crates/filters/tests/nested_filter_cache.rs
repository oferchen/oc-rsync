// crates/filters/tests/nested_filter_cache.rs

use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn parent_filter_change_invalidates_cache() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("a/b")).unwrap();

    fs::write(root.join(".rsync-filter"), "- *.tmp\n").unwrap();
    fs::write(root.join("a/.rsync-filter"), "+ keep.tmp\n").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    matcher.preload_dir(root.join("a/b")).unwrap();
    assert!(matcher.is_included("a/b/keep.tmp").unwrap());

    sleep(Duration::from_secs(1));
    fs::write(root.join("a/.rsync-filter"), "- keep.tmp\n").unwrap();

    matcher.preload_dir(root.join("a/b")).unwrap();
    assert!(!matcher.is_included("a/b/keep.tmp").unwrap());
}

#[test]
fn child_filter_change_invalidates_cache() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("sub")).unwrap();

    fs::write(root.join(".rsync-filter"), "- *.log\n").unwrap();
    fs::write(root.join("sub/.rsync-filter"), "+ keep.log\n").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    matcher.preload_dir(root.join("sub")).unwrap();
    assert!(matcher.is_included("sub/keep.log").unwrap());

    sleep(Duration::from_secs(1));
    fs::write(root.join("sub/.rsync-filter"), "- keep.log\n").unwrap();

    assert!(!matcher.is_included("sub/keep.log").unwrap());
}
