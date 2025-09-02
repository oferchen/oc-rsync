// bin/oc-rsync/src/version.rs
use protocol::SUPPORTED_PROTOCOLS;

#[allow(clippy::vec_init_then_push)]
pub fn render_version_lines() -> Vec<String> {
    let mut lines = Vec::new();
    let upstream = option_env!("UPSTREAM_VERSION").unwrap_or("unknown");
    lines.push(format!(
        "oc-rsync {} (rsync {})",
        env!("CARGO_PKG_VERSION"),
        upstream,
    ));
    let protocols = SUPPORTED_PROTOCOLS
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    lines.push(format!("Protocols: {protocols}"));
    #[allow(unused_mut)]
    let mut features: Vec<&str> = Vec::new();
    #[cfg(feature = "xattr")]
    features.push("xattr");
    #[cfg(feature = "acl")]
    features.push("acl");
    let features = if features.is_empty() {
        "none".to_string()
    } else {
        features.join(", ")
    };
    lines.push(format!("Features: {features}"));
    lines
}

pub fn version_banner() -> String {
    let mut lines = render_version_lines();
    lines.push(String::new());
    lines.join("\n")
}
