// crates/filters/tests/nested_merge.rs
use filters::{parse, Matcher};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn nested_dir_merge_applies() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("rules2"), "- b\n").unwrap();
    fs::write(root.join("rules1"), ": rules2\n- a\n").unwrap();
    fs::write(root.join(".rsync-filter"), ": rules1\n").unwrap();
    fs::write(root.join("a"), "").unwrap();
    fs::write(root.join("b"), "").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);
    assert!(!matcher.is_included("a").unwrap());
    assert!(!matcher.is_included("b").unwrap());
}

fn chain_strategy() -> impl Strategy<Value = (String, String)> {
    ("[a-z]{1,4}(\\.txt)?", "[a-z]{1,4}(\\.txt)?")
}

proptest! {
    #[test]
    fn nested_merge_excludes(
        (p1, p2) in chain_strategy(),
        path in "[a-z]{1,4}(\\.txt)?",
    ) {
        let tmp = tempdir().unwrap();
        let root = tmp.path();
        fs::write(root.join("f2"), format!("- {}\n", p2)).unwrap();
        fs::write(root.join("f1"), format!(": f2\n- {}\n", p1)).unwrap();
        fs::write(root.join(".rsync-filter"), ": f1\n").unwrap();

        let mut v = HashSet::new();
        let rules = parse(": /.rsync-filter\n", &mut v, 0).unwrap();
        let matcher = Matcher::new(rules).with_root(root);

        let expected = path != p1 && path != p2;
        prop_assert_eq!(matcher.is_included(&path).unwrap(), expected);
    }
}
