use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn include_overrides_parent_exclude() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".rsync-filter"), "- *.tmp\n").unwrap();
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".rsync-filter"), "+ keep.tmp\n").unwrap();

    let matcher = Matcher::new(Vec::new()).with_root(root);

    assert!(!matcher.is_included("other.tmp").unwrap());
    assert!(matcher.is_included("sub/keep.tmp").unwrap());
    assert!(!matcher.is_included("sub/other.tmp").unwrap());
}

#[test]
fn nested_filters_apply_in_order() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".rsync-filter"), "- *.log\n").unwrap();
    let dir = root.join("dir");
    let sub = dir.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(dir.join(".rsync-filter"), "+ debug.log\n").unwrap();
    fs::write(sub.join(".rsync-filter"), "- debug.log\n").unwrap();

    let matcher = Matcher::new(Vec::new()).with_root(root);

    assert!(matcher.is_included("dir/debug.log").unwrap());
    assert!(!matcher.is_included("dir/sub/debug.log").unwrap());
    assert!(!matcher.is_included("dir/info.log").unwrap());
}
