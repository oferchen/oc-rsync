// crates/filters/tests/rule_prefixes.rs
use filters::{Matcher, parse};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use tempfile::tempdir;

fn line_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        (Just("+"), "[a-z]{1,4}(\\.txt)?").prop_map(|(p, s)| format!("{} {}\n", p, s)),
        (Just("-"), "[a-z]{1,4}(\\.txt)?").prop_map(|(p, s)| format!("{} {}\n", p, s)),
        (Just("P"), "[a-z]{1,4}(\\.txt)?").prop_map(|(p, s)| format!("{} {}\n", p, s)),
        Just("!\n".to_string()),
    ]
}

proptest! {
    #[test]
    fn merge_equivalent_with_clear(
        first in prop::collection::vec(line_strategy(), 1..4),
        second in prop::collection::vec(line_strategy(), 1..4),
        path in "[a-z]{1,4}(\\.txt)?",
    ) {
        let first_str: String = first.concat();
        let second_str: String = second.concat();
        let mut v1 = HashSet::new();
        let mut matcher = Matcher::new(parse(&first_str, &mut v1, 0).unwrap());
        let mut v2 = HashSet::new();
        matcher.merge(parse(&second_str, &mut v2, 0).unwrap());
        let mut v3 = HashSet::new();
        let combined = format!("{}{}", first_str, second_str);
        let matcher_combined = Matcher::new(parse(&combined, &mut v3, 0).unwrap());
        prop_assert_eq!(matcher.is_included(&path).unwrap(), matcher_combined.is_included(&path).unwrap());
    }
}

#[test]
fn clear_resets_parent_rules() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join(".rsync-filter"), "- secret\n").unwrap();
    fs::write(root.join("secret"), "").unwrap();
    fs::write(root.join("sub/.rsync-filter"), "!\n+ secret\n").unwrap();
    fs::write(root.join("sub/secret"), "").unwrap();

    let mut v = HashSet::new();
    let rules = parse(": /.rsync-filter\n- .rsync-filter\n", &mut v, 0).unwrap();
    let matcher = Matcher::new(rules).with_root(root);
    assert!(!matcher.is_included("secret").unwrap());
    assert!(matcher.is_included("sub/secret").unwrap());
}
