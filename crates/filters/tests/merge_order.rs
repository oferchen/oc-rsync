use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn rsync_filter_merge_order_and_wildcards() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();

    // Root excludes all log files.
    fs::write(root.join(".rsync-filter"), "- *.log\n").unwrap();

    // Subdirectory re-includes log files.
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".rsync-filter"), "+ *.log\n").unwrap();

    // Nested directory excludes a specific log again.
    let nested = sub.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join(".rsync-filter"), "- debug.log\n").unwrap();

    // Global rules mirror recorded rsync behaviour with -F.
    let mut v = HashSet::new();
    let global = parse(
        ": /.rsync-filter\n- .rsync-filter\n+ *.log\n- *\n",
        &mut v,
        0,
    )
    .unwrap();
    let matcher = Matcher::new(global).with_root(root);

    // Root rule overrides global include.
    assert!(!matcher.is_included("a.log").unwrap());
    // Sub rule overrides root exclude.
    assert!(matcher.is_included("sub/b.log").unwrap());
    // Nested rule overrides sub include.
    assert!(!matcher.is_included("sub/nested/debug.log").unwrap());
    // Sub wildcard applies to deeper paths.
    assert!(matcher.is_included("sub/nested/trace.log").unwrap());
    // Global catch-all excludes non-log files.
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
