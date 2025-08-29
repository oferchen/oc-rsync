use filters::{parse, Matcher};
use std::collections::HashSet;

fn p(input: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(input, &mut v, 0).unwrap()
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    // Leading comment and blank line should be ignored by the parser
    let rules = p("# initial comment\n\n+ keep.log\n- *.log\n");
    let matcher = Matcher::new(rules);

    assert!(matcher.is_included("keep.log").unwrap());
    assert!(!matcher.is_included("debug.log").unwrap());
}
