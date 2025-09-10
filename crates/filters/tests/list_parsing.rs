// crates/filters/tests/list_parsing.rs
#![cfg(feature = "filters_test")]
use filters::parse_list;

#[test]
fn list_file_respects_escapes_and_comments() {
    let data = b"foo\\ bar\n#comment\nbaz\\#qux\\ \n\n";
    let out = parse_list(data, false);
    assert_eq!(out, vec!["foo bar", "baz#qux "]);
}
