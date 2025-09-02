// bin/oc-rsync/src/version.rs
use protocol::SUPPORTED_PROTOCOLS;

/// Latest rsync protocol version supported by oc-rsync.
pub const RSYNC_PROTOCOL: u32 = SUPPORTED_PROTOCOLS[0];

/// Render a three-line version string.
///
/// Line 1: "oc-rsync <pkg-version> (protocol <RSYNC_PROTOCOL>)"
/// Line 2: "rsync <upstream-version>"
/// Line 3: "<git-hash> <official-flag>"
pub fn render_version_lines() -> String {
    format!(
        "oc-rsync {} (protocol {})\nrsync {}\n{} {}\n",
        option_env!("CARGO_PKG_VERSION").unwrap_or("unknown"),
        RSYNC_PROTOCOL,
        option_env!("OC_RSYNC_UPSTREAM").unwrap_or("unknown"),
        option_env!("OC_RSYNC_GIT").unwrap_or("unknown"),
        option_env!("OC_RSYNC_OFFICIAL").unwrap_or("unknown"),
    )
}
