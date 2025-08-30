use filters::{parse, Matcher};
use std::collections::HashSet;

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
