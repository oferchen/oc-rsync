use filters::{parse, Matcher};
use proptest::prelude::*;

#[test]
fn include_and_exclude() {
    let rules = parse("+ special.tmp\n- *.tmp\n").expect("parse");
    let matcher = Matcher::new(rules);

    assert!(matcher.is_included("special.tmp").unwrap());
    assert!(!matcher.is_included("other.tmp").unwrap());
    assert!(matcher.is_included("notes.txt").unwrap());
}

proptest! {
    #[test]
    fn include_exclude_ordering(file in "[a-z]{1,8}\\.tmp") {
        let rules = parse(&format!("+ {}\n- *.tmp\n", file)).unwrap();
        let matcher = Matcher::new(rules);
        prop_assert!(matcher.is_included(&file).unwrap());

        let rules_rev = parse(&format!("- *.tmp\n+ {}\n", file)).unwrap();
        let matcher_rev = Matcher::new(rules_rev);
        prop_assert!(!matcher_rev.is_included(&file).unwrap());
    }
}
