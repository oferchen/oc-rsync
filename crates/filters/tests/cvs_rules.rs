// crates/filters/tests/cvs_rules.rs
#![forbid(unsafe_code)]
use filters::{Matcher, default_cvs_rules, parse};
use std::collections::HashSet;
use std::fs;
use temp_env::with_var;
use tempfile::tempdir;

fn p(s: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(s, &mut v, 0).unwrap()
}

#[test]
fn cvs_excludes_can_be_overridden() {
    let rules = p("+ core\n-C\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("core").unwrap());
    assert!(!matcher.is_included("foo.o").unwrap());
}

#[test]
fn filter_minus_c_ignores_local_cvsignore() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".cvsignore"), "foo\n").unwrap();
    fs::write(root.join("foo"), b"foo").unwrap();
    fs::write(root.join("core"), b"core").unwrap();

    let rules = p("-C\n");
    let matcher = Matcher::new(rules).with_root(root);

    assert!(matcher.is_included("foo").unwrap());
    assert!(!matcher.is_included("core").unwrap());
}

#[test]
fn cvsignore_is_scoped_per_directory() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".cvsignore"), "foo\n").unwrap();
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".cvsignore"), "bar\n").unwrap();

    let mut rules = default_cvs_rules().unwrap();
    rules.extend(p(":C\n"));
    let matcher = Matcher::new(rules).with_root(root);

    assert!(!matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("sub/foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("sub/bar").unwrap());
    assert!(matcher.is_included("sub/nested/bar").unwrap());
    assert!(matcher.is_included(".cvsignore").unwrap());
}

#[test]
fn home_patterns_are_global() {
    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();
    with_var("HOME", Some(home.path()), || {
        let rules = p("-C\n");
        let matcher = Matcher::new(rules);

        assert!(!matcher.is_included("home_ignored").unwrap());
        assert!(!matcher.is_included("sub/home_ignored").unwrap());
    });
}

#[test]
fn env_patterns_are_global() {
    with_var("CVSIGNORE", Some("env_ignored"), || {
        let rules = p("-C\n");
        let matcher = Matcher::new(rules);

        assert!(!matcher.is_included("env_ignored").unwrap());
        assert!(!matcher.is_included("sub/env_ignored").unwrap());
    });
}

#[test]
fn env_multiple_patterns_are_respected() {
    with_var("CVSIGNORE", Some("foo bar"), || {
        let rules = p("-C\n");
        let matcher = Matcher::new(rules);

        assert!(!matcher.is_included("foo").unwrap());
        assert!(!matcher.is_included("bar").unwrap());
    });
}

#[test]
fn env_patterns_can_be_overridden() {
    with_var("CVSIGNORE", Some("env_ignored"), || {
        let rules = p("-C\n+ env_ignored\n");
        let matcher = Matcher::new(rules);

        assert!(matcher.is_included("env_ignored").unwrap());
    });
}

#[test]
fn git_directory_is_ignored_by_default_rules() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join(".git")).unwrap();
    let rules = default_cvs_rules().unwrap();
    let matcher = Matcher::new(rules).with_root(root);
    assert!(!matcher.is_included(".git").unwrap());
}

#[test]
fn default_rules_ignore_hash_prefixed_files() {
    let rules = p("-C\n");
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("#temp").unwrap());
}

#[test]
fn env_hash_patterns_are_respected() {
    with_var("CVSIGNORE", Some("#envpat"), || {
        let rules = p("-C\n");
        let matcher = Matcher::new(rules);
        assert!(!matcher.is_included("#envpat").unwrap());
    });
}

#[test]
fn include_overrides_nested_cvsignore() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("sub/nested")).unwrap();
    fs::write(root.join("core"), b"root").unwrap();
    fs::write(root.join("keep.txt"), b"root").unwrap();
    fs::write(root.join("sub/nested/keep.txt"), b"sub").unwrap();
    fs::write(root.join("sub/nested/core"), b"sub").unwrap();
    fs::write(root.join("sub/.cvsignore"), "nested/\n").unwrap();

    let rules1 = p(":C\n+ sub/nested/\n+ sub/nested/***\n");
    let matcher1 = Matcher::new(rules1).with_root(root);
    assert!(matcher1.is_included("sub/nested/keep.txt").unwrap());
    assert!(matcher1.is_included("sub/nested/core").unwrap());

    let rules2 = p(":C\n+ sub/nested/core\n");
    let matcher2 = Matcher::new(rules2).with_root(root);
    assert!(matcher2.is_included("sub/nested/core").unwrap());
    assert!(!matcher2.is_included("sub/nested/keep.txt").unwrap());
}
