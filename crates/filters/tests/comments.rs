use filters::{parse, Matcher};

#[test]
fn comments_and_blank_lines_are_ignored() {
    // Leading comment and blank line should be ignored by the parser
    let rules = parse("# initial comment\n\n+ keep.log\n- *.log\n").expect("parse");
    let matcher = Matcher::new(rules);

    assert!(matcher.is_included("keep.log").unwrap());
    assert!(!matcher.is_included("debug.log").unwrap());
}
