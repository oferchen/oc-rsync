// crates/cli/tests/golden.rs
use oc_rsync_core::fs::parse_chmod_spec;

#[test]
fn invalid_mode_operator_returns_error() {
    let err = parse_chmod_spec("u!x").unwrap_err();
    assert_eq!(err, "invalid operator '!'");
}
