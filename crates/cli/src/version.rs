// crates/cli/src/version.rs
use protocol::SUPPORTED_PROTOCOLS;

pub const RSYNC_PROTOCOL: u32 = SUPPORTED_PROTOCOLS[0];

/// Render a three-line version banner as separate lines.
///
/// Line 1: "oc-rsync <pkg-version> (protocol <RSYNC_PROTOCOL>)"
/// Line 2: "rsync <upstream-version>"
/// Line 3: "<build-revision> <official-flag>"
pub fn render_version_lines() -> Vec<String> {
    vec![
        format!(
            "oc-rsync {} (protocol {})",
            env!("CARGO_PKG_VERSION"),
            RSYNC_PROTOCOL
        ),
        format!(
            "rsync {}",
            option_env!("RSYNC_UPSTREAM_VER").unwrap_or("unknown")
        ),
        format!(
            "{} {}",
            option_env!("BUILD_REVISION").unwrap_or("unknown"),
            option_env!("OFFICIAL_BUILD").unwrap_or("unofficial")
        ),
    ]
}

#[allow(dead_code)]
pub fn version_banner() -> String {
    format!("{}\n", render_version_lines().join("\n"))
}
