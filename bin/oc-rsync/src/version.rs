// bin/oc-rsync/src/version.rs
use protocol::SUPPORTED_PROTOCOLS;

use oc_rsync_cli::branding;

pub const RSYNC_PROTOCOL: u32 = SUPPORTED_PROTOCOLS[0];

const COPYRIGHT: &str =
    "Copyright (C) 1996-2022 by Andrew Tridgell, Wayne Davison, and others.";
const WEBSITE: &str = "Web site: https://rsync.samba.org/";
const CAPABILITIES: &[&str] = &[
    "    64-bit files, 64-bit inums, 64-bit timestamps, 64-bit long ints,",
    "    socketpairs, symlinks, symtimes, hardlinks, hardlink-specials,",
    "    hardlink-symlinks, IPv6, atimes, batchfiles, inplace, append, ACLs,",
    "    xattrs, optional secluded-args, iconv, prealloc, stop-at, no crtimes",
];
const OPTIMIZATIONS: &[&str] = &[
    "    SIMD-roll, no asm-roll, openssl-crypto, no asm-MD5",
];
const CHECKSUMS: &[&str] = &[
    "    xxh128 xxh3 xxh64 (xxhash) md5 md4 sha1 none",
];
const COMPRESS: &[&str] = &[
    "    zstd lz4 zlibx zlib none",
];
const DAEMON_AUTH: &[&str] = &[
    "    sha512 sha256 sha1 md5 md4",
];

pub fn render_version_lines() -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} {} (protocol {})",
        branding::program_name(),
        env!("CARGO_PKG_VERSION"),
        RSYNC_PROTOCOL
    ));
    lines.push(format!(
        "rsync {}",
        option_env!("RSYNC_UPSTREAM_VER").unwrap_or("unknown")
    ));
    lines.push(format!(
        "{} {}",
        option_env!("BUILD_REVISION").unwrap_or("unknown"),
        option_env!("OFFICIAL_BUILD").unwrap_or("unofficial")
    ));
    lines.push(COPYRIGHT.to_string());
    lines.push(WEBSITE.to_string());
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
    lines
}

#[allow(dead_code)]
pub fn version_banner() -> String {
    format!("{}\n", render_version_lines().join("\n"))
}

