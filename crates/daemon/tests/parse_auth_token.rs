// crates/daemon/tests/parse_auth_token.rs
use daemon::parse_auth_token;

#[test]
fn parse_auth_token_handles_whitespace_and_comments() {
    let contents = "\
# hash comment
   ; semicolon comment

  token1   mod1   mod2   # trailing comment
token2   mod3   ; another trailing comment
   # post comment
";

    assert_eq!(
        parse_auth_token("token1", contents),
        Some(vec!["mod1".to_string(), "mod2".to_string()])
    );
    assert_eq!(
        parse_auth_token("token2", contents),
        Some(vec!["mod3".to_string()])
    );
    assert_eq!(parse_auth_token("missing", contents), None);
}

#[test]
fn parse_auth_token_allows_comment_chars_in_quotes() {
    let contents = r#"token1 "mod#1" 'mod;2'
token2 "a#b" "c;d" ; trailing comment"#;

    assert_eq!(
        parse_auth_token("token1", contents),
        Some(vec!["mod#1".to_string(), "mod;2".to_string()])
    );
    assert_eq!(
        parse_auth_token("token2", contents),
        Some(vec!["a#b".to_string(), "c;d".to_string()])
    );
}
