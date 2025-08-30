// crates/filters/tests/anchored_wildcards.rs
use filters::{parse, Matcher};
use std::collections::HashSet;

fn p(input: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(input, &mut v, 0).unwrap()
}

#[test]
fn root_anchored_exclusion() {
    let rules = p("- /root.txt\n+ *.txt\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("root.txt").unwrap());
    assert!(matcher.is_included("dir/root.txt").unwrap());
}

#[test]
fn directory_trailing_slash() {
    let rules = p("- tmp/\n");
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("tmp").unwrap());
    assert!(!matcher.is_included("tmp/file.txt").unwrap());
    assert!(matcher.is_included("other/file.txt").unwrap());
}

#[test]
fn wildcard_question_mark() {
    let rules = p("+ file?.txt\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("file1.txt").unwrap());
    assert!(!matcher.is_included("file10.txt").unwrap());
}

#[test]
fn double_star_matches() {
    let rules = p("+ dir/**/keep.txt\n- *\n");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("dir/a/b/keep.txt").unwrap());
    assert!(!matcher.is_included("dir/a/b/drop.txt").unwrap());
}
