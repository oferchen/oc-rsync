// tests/files_from_dirs.rs
use filters::{Matcher, parse_with_options};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn files_from_mixed_entries_integration() {
    let tmp = tempdir().unwrap();
    let list = tmp.path().join("list");
    fs::write(&list, "foo/bar/baz\nqux/\n").unwrap();
    let filter = format!("files-from {}\n", list.display());
    let mut v = HashSet::new();
    let rules = parse_with_options(&filter, false, &mut v, 0, None).unwrap();
    let m = Matcher::new(rules);
    assert!(m.is_included("foo").unwrap());
    assert!(m.is_included("foo/bar").unwrap());
    assert!(m.is_included("foo/bar/baz").unwrap());
    assert!(!m.is_included("foo/bar/qux").unwrap());
    assert!(!m.is_included("foo/other").unwrap());
    assert!(m.is_included("qux").unwrap());
    assert!(m.is_included("qux/sub").unwrap());
    assert!(!m.is_included("other").unwrap());
    assert!(!m.is_included("qux/other").unwrap());
}
