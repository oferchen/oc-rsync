use filters::filters::{parse, Matcher};

#[test]
fn include_and_exclude() {
    let rules = parse("+ special.tmp\n- *.tmp\n").expect("parse");
    let matcher = Matcher::new(rules);

    assert!(matcher.is_included("special.tmp"));
    assert!(!matcher.is_included("other.tmp"));
    assert!(matcher.is_included("notes.txt"));
}
