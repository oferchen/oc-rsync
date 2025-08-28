use filters::Matcher;
use std::fs;
use tempfile::tempdir;

#[test]
fn malformed_filter_file_returns_error() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join(".rsync-filter"), "+\n").unwrap();
    let matcher = Matcher::new(Vec::new()).with_root(root);
    assert!(matcher.is_included("foo").is_err());
}
