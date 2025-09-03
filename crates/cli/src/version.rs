// crates/cli/src/version.rs
use protocol::SUPPORTED_PROTOCOLS;

use crate::branding;

pub const RSYNC_PROTOCOL: u32 = SUPPORTED_PROTOCOLS[0];

pub fn render_version_lines() -> Vec<String> {
    vec![
        format!(
            "{} {} (protocol {})",
            branding::program_name(),
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
