// crates/filters/tests/unreadable_filter.rs
use filters::{Matcher, ParseError, parse};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn unreadable_filter_file_errors() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    // Create a directory named `.rsync-filter` so attempts to read it fail.
    fs::create_dir(root.join(".rsync-filter")).unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    let err = matcher.is_included("foo").unwrap_err();
    assert!(matches!(err, ParseError::Io(_)));
}
