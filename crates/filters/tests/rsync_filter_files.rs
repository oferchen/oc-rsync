use filters::{parse, Matcher};
use proptest::prelude::*;
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

    let rules = parse(": /.rsync-filter\n- .rsync-filter\n").unwrap();
    let matcher = Matcher::new(rules).with_root(root);

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

    let rules = parse(": /.rsync-filter\n- .rsync-filter\n").unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    assert!(matcher.is_included("dir/debug.log").unwrap());
    assert!(!matcher.is_included("dir/sub/debug.log").unwrap());
    assert!(!matcher.is_included("dir/info.log").unwrap());
}

proptest! {
    #[test]
    fn include_overrides_parent_exclude_prop(subdir in "[a-z]{1,8}", keep in "[a-z]{1,8}\\.tmp") {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join(".rsync-filter"), "- *.tmp\n").unwrap();
        let sub = root.join(&subdir);
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(".rsync-filter"), format!("+ {}\n", keep)).unwrap();

        let rules = parse(": /.rsync-filter\n- .rsync-filter\n").unwrap();
        let matcher = Matcher::new(rules).with_root(root);

        prop_assert!(!matcher.is_included("other.tmp").unwrap());
        let keep_path = format!("{}/{}", subdir, keep);
        prop_assert!(matcher.is_included(&keep_path).unwrap());
        let other_path = format!("{}/{}", subdir, "other.tmp");
        prop_assert!(!matcher.is_included(&other_path).unwrap());
    }
}
