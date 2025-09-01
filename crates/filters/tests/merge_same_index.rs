// crates/filters/tests/merge_same_index.rs
use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn nested_rsync_filter_order() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join(".rsync-filter"), "+ keep\n: sub/.rsync-filter\n").unwrap();
    fs::write(root.join("sub/.rsync-filter"), "- keep\n").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- *\n", &mut v, 0).unwrap();
    let m = Matcher::new(rules).with_root(root);
    assert!(!m.is_included("sub/keep").unwrap());
}
