// crates/filters/tests/rule_stats.rs
use filters::{parse_file, Matcher};
use std::collections::HashSet;
use tempfile::NamedTempFile;

#[test]
fn counts_matches_and_misses() {
    let tmp = NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), "+ foo\n- *\n").unwrap();
    let mut v = HashSet::new();
    let rules = parse_file(tmp.path(), false, &mut v, 0).unwrap();
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("foo").unwrap());
    assert!(!matcher.is_included("bar").unwrap());
    let stats = matcher.stats();
    assert_eq!(stats.matches, 2);
    assert_eq!(stats.misses, 1);
}
