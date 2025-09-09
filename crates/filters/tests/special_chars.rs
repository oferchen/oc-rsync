// crates/filters/tests/special_chars.rs
// Verify handling of negated character classes and rule modifiers without relying on rsync.
use filters::{parse, Matcher};
use std::collections::HashSet;

fn m(rules_src: &str) -> Matcher {
    let mut visited = HashSet::new();
    Matcher::new(parse(rules_src, &mut visited, 0).unwrap())
}

#[test]
fn negated_char_class_includes_only_unlisted() {
    let matcher = m("+ dir/[!ab]\n- *\n");
    assert!(matcher.is_included("dir/c").unwrap());
    assert!(!matcher.is_included("dir/a").unwrap());
    assert!(!matcher.is_included("dir/b").unwrap());
}

#[test]
fn negated_char_class_excludes_unlisted() {
    let matcher = m("- dir/[!ab]\n");
    assert!(matcher.is_included("dir/a").unwrap());
    assert!(matcher.is_included("dir/b").unwrap());
    assert!(!matcher.is_included("dir/c").unwrap());
}

#[test]
fn bang_modifier_respected() {
    let matcher = m("+! dir/[!ab]\n- *\n");
    assert!(matcher.is_included("dir/c").unwrap());
    assert!(!matcher.is_included("dir/a").unwrap());
    assert!(!matcher.is_included("dir/b").unwrap());
}
