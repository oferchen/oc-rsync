// crates/filters/tests/rsync_parity.rs
use filters::{parse, Matcher};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn parity_with_stock_rsync() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("src");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join(".rsync-filter"), "- *.tmp\n").unwrap();
    fs::write(root.join("a.tmp"), "").unwrap();
    fs::write(root.join("keep.log"), "").unwrap();
    fs::write(root.join("sub/.rsync-filter"), "+ keep.tmp\n").unwrap();
    fs::write(root.join("sub/keep.tmp"), "").unwrap();
    fs::write(root.join("sub/other.tmp"), "").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(&root);

    let rsync_included = ["keep.log".to_string(), "sub/keep.tmp".to_string()];

    let files = [
        "a.tmp",
        "keep.log",
        "sub/keep.tmp",
        "sub/other.tmp",
        ".rsync-filter",
        "sub/.rsync-filter",
    ];
    for f in files {
        let ours = matcher.is_included(f).unwrap();
        let theirs = rsync_included.contains(&f.to_string());
        assert_eq!(ours, theirs, "file {}", f);
    }
}
