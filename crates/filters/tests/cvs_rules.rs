// crates/filters/tests/cvs_rules.rs
use filters::{parse, Matcher};
use std::collections::HashSet;
use std::env;
use std::fs;
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
fn cvsignore_is_scoped_per_directory() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".cvsignore"), "foo\n").unwrap();
    let sub = root.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join(".cvsignore"), "bar\n").unwrap();

    let rules = p(":C\n");
    let matcher = Matcher::new(rules).with_root(root);

    assert!(!matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("sub/foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("sub/bar").unwrap());
    assert!(matcher.is_included("sub/nested/bar").unwrap());
}

#[test]
fn home_and_env_patterns_are_global() {
    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();
    env::set_var("HOME", home.path());
    env::set_var("CVSIGNORE", "env_ignored");

    let rules = p("-C\n");
    let matcher = Matcher::new(rules);

    assert!(!matcher.is_included("home_ignored").unwrap());
    assert!(!matcher.is_included("sub/home_ignored").unwrap());
    assert!(!matcher.is_included("env_ignored").unwrap());
    assert!(!matcher.is_included("sub/env_ignored").unwrap());

    env::remove_var("HOME");
    env::remove_var("CVSIGNORE");
}
