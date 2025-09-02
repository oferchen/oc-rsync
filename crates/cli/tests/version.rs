// crates/cli/tests/version.rs
use oc_rsync_cli::version;
use protocol::SUPPORTED_PROTOCOLS;

#[test]
fn banner_is_static() {
    let expected = vec![
        format!(
            "oc-rsync {} (protocol {})",
            env!("CARGO_PKG_VERSION"),
            SUPPORTED_PROTOCOLS[0],
        ),
        format!(
            "rsync {}",
            option_env!("RSYNC_UPSTREAM_VER").unwrap_or("unknown")
        ),
        format!(
            "{} {}",
            option_env!("BUILD_REVISION").unwrap_or("unknown"),
            option_env!("OFFICIAL_BUILD").unwrap_or("unofficial"),
        ),
    ];
    assert_eq!(version::render_version_lines(), expected);
}
