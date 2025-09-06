// crates/filters/tests/merge.rs
use filters::{Matcher, parse};
use proptest::prelude::*;
use std::collections::HashSet;

fn p(s: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(s, &mut v, 0).unwrap()
}

#[test]
fn rsync_filter_merge() {
    let root_rules = p("- *.tmp\n");
    let mut matcher = Matcher::new(root_rules);

    assert!(matcher.is_included("notes.txt").unwrap());
    assert!(!matcher.is_included("junk.tmp").unwrap());

    let sub_rules = p("- secret\n");
    matcher.merge(sub_rules);

    assert!(!matcher.is_included("junk.tmp").unwrap());
    assert!(!matcher.is_included("secret").unwrap());
}

proptest! {
    #[test]
    fn merge_equivalent(
        first in prop::collection::vec((prop_oneof![Just("+"), Just("-")], "[a-z]{1,4}(\\.txt)?"), 1..4),
        second in prop::collection::vec((prop_oneof![Just("+"), Just("-")], "[a-z]{1,4}(\\.txt)?"), 1..4),
        path in "[a-z]{1,4}(\\.txt)?"
    ) {
        let first_str: String = first.iter().map(|(s,p)| format!("{} {}\n", s, p)).collect();
        let second_str: String = second.iter().map(|(s,p)| format!("{} {}\n", s, p)).collect();
        let mut matcher = Matcher::new(p(&first_str));
        matcher.merge(p(&second_str));
        let combined = format!("{}{}", first_str, second_str);
        let matcher_combined = Matcher::new(p(&combined));
        prop_assert_eq!(matcher.is_included(&path).unwrap(), matcher_combined.is_included(&path).unwrap());
    }
}
