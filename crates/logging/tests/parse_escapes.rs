// crates/logging/tests/parse_escapes.rs
use logging::parse_escapes;

#[test]
fn invalid_hex_escape_sequence_is_literal() {
    assert_eq!(parse_escapes("\\xGG"), "xGG");
}

#[test]
fn trailing_backslash_is_preserved() {
    assert_eq!(parse_escapes("\\"), "\\");
}

#[test]
fn unknown_escape_is_literal() {
    assert_eq!(parse_escapes("\\y"), "y");
}

#[test]
fn invalid_octal_escape_is_literal() {
    assert_eq!(parse_escapes("\\8"), "8");
}
