use filters::{parse, Matcher};
use proptest::prelude::*;

#[test]
fn rsync_filter_merge() {
    let root_rules = parse("- *.tmp\n").unwrap();
    let mut matcher = Matcher::new(root_rules);

    assert!(matcher.is_included("notes.txt").unwrap());
    assert!(!matcher.is_included("junk.tmp").unwrap());

    // Merge rules from a subdirectory `.rsync-filter` file
    let sub_rules = parse("- secret\n").unwrap();
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
        let mut matcher = Matcher::new(parse(&first_str).unwrap());
        matcher.merge(parse(&second_str).unwrap());
        let combined = format!("{}{}", first_str, second_str);
        let matcher_combined = Matcher::new(parse(&combined).unwrap());
        prop_assert_eq!(matcher.is_included(&path).unwrap(), matcher_combined.is_included(&path).unwrap());
    }
}
