use filters::parse;
use filters::Matcher;
use std::collections::HashSet;

fn p(s: &str) -> Vec<filters::Rule> {
    let mut v = HashSet::new();
    parse(s, &mut v, 0).unwrap()
}

#[test]
fn files_from_emulation() {
    let rules = p("+ foo\n+ bar\n- *\n");
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
        rules.extend(p(&format!("+ {}\n", pat)));
    }
    rules.extend(p("- *\n"));
    let matcher = Matcher::new(rules);
    assert!(matcher.is_included("foo").unwrap());
    assert!(matcher.is_included("bar").unwrap());
    assert!(!matcher.is_included("baz").unwrap());
}
