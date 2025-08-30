// crates/filters/tests/malformed.rs
use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn malformed_filter_file_returns_error() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".rsync-filter"), "+\n").unwrap();
    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);
    assert!(matcher.is_included("foo").is_err());
}
