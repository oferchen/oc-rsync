// crates/filters/tests/merge_order.rs
use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn rsync_filter_merge_order_and_wildcards() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    fs::write(root.join(".rsync-filter"), "- *.log\n").unwrap();

    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".rsync-filter"), "+ *.log\n").unwrap();

    let nested = sub.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join(".rsync-filter"), "- debug.log\n").unwrap();

    let mut v = HashSet::new();
    let global = parse(
        ": /.rsync-filter\n- .rsync-filter\n+ *.log\n- *\n",
        &mut v,
        0,
    )
    .unwrap();
    let matcher = Matcher::new(global).with_root(root);

    assert!(!matcher.is_included("a.log").unwrap());
    assert!(matcher.is_included("sub/b.log").unwrap());
    assert!(!matcher.is_included("sub/nested/debug.log").unwrap());
    assert!(matcher.is_included("sub/nested/trace.log").unwrap());
    assert!(!matcher.is_included("sub/nested/file.txt").unwrap());
}

#[test]
fn recorded_selection_parity() {
    let mut v = HashSet::new();
    let rules = parse("+ *.txt\n- *\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules);

    assert!(matcher.is_included("a.txt").unwrap());
    assert!(!matcher.is_included("b.log").unwrap());
}
