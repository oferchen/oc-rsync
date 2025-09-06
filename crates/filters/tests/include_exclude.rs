// crates/filters/tests/include_exclude.rs
use filters::{Matcher, parse};
use proptest::prelude::*;
use std::collections::HashSet;

fn p(s: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(s, &mut v, 0).unwrap()
}

#[test]
fn include_and_exclude() {
    let rules = p("+ special.tmp\n- *.tmp\n");
    let matcher = Matcher::new(rules);

    assert!(matcher.is_included("special.tmp").unwrap());
    assert!(!matcher.is_included("other.tmp").unwrap());
    assert!(matcher.is_included("notes.txt").unwrap());
}

proptest! {
    #[test]
    fn include_exclude_ordering(file in "[a-z]{1,8}\\.tmp") {
        let rules = p(&format!("+ {}\n- *.tmp\n", file));
        let matcher = Matcher::new(rules);
        prop_assert!(matcher.is_included(&file).unwrap());

        let rules_rev = p(&format!("- *.tmp\n+ {}\n", file));
        let matcher_rev = Matcher::new(rules_rev);
        prop_assert!(!matcher_rev.is_included(&file).unwrap());
    }
}
