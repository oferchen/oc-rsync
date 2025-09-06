// crates/filters/tests/recursive_filters.rs
use filters::{Matcher, parse};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn self_referential_filter_is_error() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".rsync-filter"), ". .rsync-filter\n").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    assert!(matcher.is_included("file").is_err());
}

#[test]
fn mutually_recursive_filters_error() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(root.join(".rsync-filter"), ". sub/.rsync-filter\n").unwrap();
    fs::write(sub.join(".rsync-filter"), ". ../.rsync-filter\n").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    assert!(matcher.is_included("sub/file").is_err());
}
