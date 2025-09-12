// crates/cli/src/version.rs
use oc_rsync_core::message::SUPPORTED_PROTOCOLS as SUPPORTED_PROTOCOL_LIST;

use crate::branding;

pub const RSYNC_PROTOCOL: u32 = SUPPORTED_PROTOCOL_LIST[0];

const UPSTREAM_VERSION: &str = match option_env!("UPSTREAM_VERSION") {
    Some(v) => v,
    None => "unknown",
};
const SUPPORTED_PROTOCOLS: &str = match option_env!("SUPPORTED_PROTOCOLS") {
    Some(v) => v,
    None => "32,31,30",
};

const CAPABILITIES: &[&str] = &[
    "    64-bit files, 64-bit inums, 64-bit timestamps, 64-bit long ints,",
    "    socketpairs, symlinks, symtimes, hardlinks, hardlink-specials,",
    "    hardlink-symlinks, IPv6, atimes, batchfiles, inplace, append, ACLs,",
    "    xattrs, optional secluded-args, iconv, prealloc, stop-at, no crtimes",
];
const OPTIMIZATIONS: &[&str] = &["    SIMD-roll, no asm-roll, openssl-crypto, no asm-MD5"];
const CHECKSUMS: &[&str] = &["    md5 md4 sha1 none"];
const COMPRESS: &[&str] = &["    zstd zlibx zlib none"];
const DAEMON_AUTH: &[&str] = &["    sha512 sha256 sha1 md5 md4"];

pub fn render_version_lines() -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} {} (protocol {})",
        branding::program_name(),
        env!("CARGO_PKG_VERSION"),
        RSYNC_PROTOCOL
    ));
    let proto = SUPPORTED_PROTOCOLS
        .split(',')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(RSYNC_PROTOCOL);
    lines.push(format!(
        "compatible with rsync {} (protocol {proto})",
        UPSTREAM_VERSION
    ));
    lines.push(format!(
        "{} {}",
        option_env!("BUILD_REVISION").unwrap_or("unknown"),
        option_env!("OFFICIAL_BUILD").unwrap_or("unofficial")
    ));
    lines.push(branding::brand_copyright());
    lines.push(format!("Web site: {}", branding::brand_url()));
    lines.push("Capabilities:".to_string());
    lines.extend(CAPABILITIES.iter().map(|s| (*s).to_string()));
    lines.push("Optimizations:".to_string());
    lines.extend(OPTIMIZATIONS.iter().map(|s| (*s).to_string()));
    lines.push("Checksum list:".to_string());
    lines.extend(CHECKSUMS.iter().map(|s| (*s).to_string()));
    lines.push("Compress list:".to_string());
    lines.extend(COMPRESS.iter().map(|s| (*s).to_string()));
    lines.push("Daemon auth list:".to_string());
    lines.extend(DAEMON_AUTH.iter().map(|s| (*s).to_string()));
    lines.push(String::new());
    lines.push(format!(
        "{} comes with ABSOLUTELY NO WARRANTY.  This is free software, and you",
        branding::program_name()
    ));
    lines.push("are welcome to redistribute it under certain conditions.  See the GNU".to_string());
    lines.push("General Public Licence for details.".to_string());
    lines
}

pub fn version_banner() -> String {
    format!("{}\n", render_version_lines().join("\n"))
}
