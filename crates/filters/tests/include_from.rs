// crates/filters/tests/include_from.rs
use filters::{Matcher, parse_rule_list_from_bytes};
use proptest::prelude::*;
use std::collections::HashSet;

#[test]
fn include_from_null_separated() {
    let data = b"foo\0bar\0";
    let mut v = HashSet::new();
    let rules = parse_rule_list_from_bytes(data, true, '+', &mut v, 0, None).unwrap();
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());
}

#[test]
fn exclude_from_null_separated() {
    let data = b"foo\0bar\0";
    let mut v = HashSet::new();
    let rules = parse_rule_list_from_bytes(data, true, '-', &mut v, 0, None).unwrap();
    let matcher = Matcher::new(rules);
    assert!(!matcher.is_included("foo").unwrap());
    assert!(!matcher.is_included("bar").unwrap());
    assert!(matcher.is_included("baz").unwrap());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]
    #[test]
    fn rule_list_from0_eq_newline(entries in prop::collection::vec("[^\0#\r\n]*", 0..4)) {
        let mut bytes = Vec::new();
        for e in &entries {
            if !e.is_empty() {
                bytes.extend(e.as_bytes());
            }
            bytes.push(0);
        }
        let mut v0 = HashSet::new();
        let rules0 = parse_rule_list_from_bytes(&bytes, true, '+', &mut v0, 0, None).unwrap();
        let mut v1 = HashSet::new();
        let joined = entries.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
        let rules1 = parse_rule_list_from_bytes(joined.as_bytes(), false, '+', &mut v1, 0, None).unwrap();
        let m0 = Matcher::new(rules0);
        let m1 = Matcher::new(rules1);
        for e in entries {
            prop_assert_eq!(m0.is_included(&e).unwrap(), m1.is_included(&e).unwrap());
        }
    }
}
