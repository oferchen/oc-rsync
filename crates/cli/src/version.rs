// crates/cli/src/version.rs
use protocol::SUPPORTED_PROTOCOLS;

/// Latest rsync protocol version supported by oc-rsync.
pub const RSYNC_PROTOCOL: u32 = SUPPORTED_PROTOCOLS[0];

/// Render a three-line version banner as separate lines.
///
/// Line 1: "oc-rsync <pkg-version> (protocol <RSYNC_PROTOCOL>)"
/// Line 2: "rsync <upstream-version>"
/// Line 3: "<git-hash> <official-flag>"
pub fn render_version_lines() -> Vec<String> {
    vec![
        format!(
            "oc-rsync {} (protocol {})",
            env!("CARGO_PKG_VERSION"),
            RSYNC_PROTOCOL
        ),
        format!(
            "rsync {}",
            option_env!("OC_RSYNC_UPSTREAM").unwrap_or("unknown")
        ),
        format!(
            "{} {}",
            option_env!("OC_RSYNC_GIT").unwrap_or("unknown"),
            option_env!("OC_RSYNC_OFFICIAL").unwrap_or("unofficial")
        ),
    ]
}

/// Render the version banner as a single string ending with a newline.
#[allow(dead_code)]
pub fn version_banner() -> String {
    format!("{}\n", render_version_lines().join("\n"))
}
