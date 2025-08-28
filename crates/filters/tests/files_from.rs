use filters::parse;
use filters::Matcher;

#[test]
fn files_from_emulation() {
    let rules = parse("+ foo\n+ bar\n- *\n").expect("parse");
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());
}

#[test]
fn files_from_null_separated() {
    let input = b"foo\0bar\0";
    let mut rules = Vec::new();
    for part in input.split(|b| *b == 0) {
        if part.is_empty() {
            continue;
        }
        let pat = String::from_utf8_lossy(part);
        rules.extend(parse(&format!("+ {}\n", pat)).unwrap());
    }
    rules.extend(parse("- *\n").unwrap());
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());
}
