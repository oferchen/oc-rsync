// tests/interop/modern.rs
include!("../modern.rs");

use protocol::negotiate_version;

#[test]
fn falls_back_to_legacy_versions() {
    assert_eq!(negotiate_version(32), Ok(32));
    assert_eq!(negotiate_version(31), Ok(31));
}
