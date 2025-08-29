use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn anchored_dir_merge_uses_root_file() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Root filter excludes "secret".
    fs::write(root.join(".rsync-filter"), "- secret\n").unwrap();
    // Subdirectory filter attempts to include "secret" but should be ignored.
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".rsync-filter"), "+ secret\n").unwrap();

    let mut v = HashSet::new();
    let rules = parse("dir-merge /.rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    // The anchored dir-merge directive should always load the filter from the
    // transfer root, so the subdirectory's attempt to override it is ignored.
    assert!(!matcher.is_included("secret").unwrap());
    assert!(!matcher.is_included("sub/secret").unwrap());
}
