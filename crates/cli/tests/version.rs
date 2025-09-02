// crates/cli/tests/version.rs
use oc_rsync_cli::version_banner;
use protocol::SUPPORTED_PROTOCOLS;

#[test]
fn banner_is_static() {
    let mut features = Vec::new();
    #[cfg(feature = "xattr")]
    features.push("xattr");
    #[cfg(feature = "acl")]
    features.push("acl");
    let features = if features.is_empty() {
        "none".to_string()
    } else {
        features.join(", ")
    };
    let protocols = SUPPORTED_PROTOCOLS
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let expected = format!(
        "oc-rsync {} (rsync {})\nProtocols: {}\nFeatures: {}\n",
        env!("CARGO_PKG_VERSION"),
        env!("UPSTREAM_VERSION"),
        protocols,
        features,
    );
    assert_eq!(version_banner(), expected);
}
