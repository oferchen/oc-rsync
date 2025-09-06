// crates/filters/tests/rule_modifiers.rs
use filters::{Matcher, parse};
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

#[test]
fn sender_show_overrides_exclude() {
    let mut v = HashSet::new();
    let rules = parse("S debug.log\n- *.log\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("debug.log").unwrap());
    assert!(!matcher.is_included("info.log").unwrap());
}

#[test]
fn perishable_ignored_on_delete() {
    let mut v = HashSet::new();
    let rules = parse("-p tmp\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("tmp").unwrap());
    assert!(matcher.is_included_for_delete("tmp").unwrap());
}

#[test]
fn receiver_risk_applies_on_delete() {
    let mut v = HashSet::new();
    let rules = parse("R debug.log\n- *.log\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("debug.log").unwrap());
    assert!(matcher.is_included_for_delete("debug.log").unwrap());
}

#[test]
fn xattr_rule_only_affects_xattrs() {
    let mut v = HashSet::new();
    let rules = parse("-x user.secret\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("file").unwrap());
    assert!(matcher.is_xattr_included("user.open").unwrap());
    assert!(!matcher.is_xattr_included("user.secret").unwrap());
}

#[test]
fn per_dir_merge_precedence() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".rsync-filter"), "- *.log\n").unwrap();
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("sub/.rsync-filter"), "S *.log\n").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);

    assert!(!matcher.is_included("debug.log").unwrap());
    assert!(matcher.is_included("sub/debug.log").unwrap());
}
