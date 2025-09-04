// crates/cli/src/branding.rs
use std::env;

pub const DEFAULT_TAGLINE: &str =
    "Automatic Rust re-implementation of rsync semantics by Ofer Chen (2025). Not affiliated with Samba.";
pub const DEFAULT_URL: &str = "https://github.com/oc-rsync/oc-rsync";
pub const DEFAULT_COPYRIGHT: &str = "Copyright (C) 2024-2025 oc-rsync contributors.";

pub const DEFAULT_HELP_PREFIX: &str = r#"{prog} {version}{tagline}

Usage: {prog} [OPTION]... SRC [SRC]... DEST
  or   {prog} [OPTION]... SRC [SRC]... [USER@]HOST:DEST
  or   {prog} [OPTION]... SRC [SRC]... [USER@]HOST::DEST
  or   {prog} [OPTION]... SRC [SRC]... {prog}://[USER@]HOST[:PORT]/DEST
  or   {prog} [OPTION]... [USER@]HOST:SRC [DEST]
  or   {prog} [OPTION]... [USER@]HOST::SRC [DEST]
  or   {prog} [OPTION]... {prog}://[USER@]HOST[:PORT]/SRC [DEST]
Options
"#;

pub const DEFAULT_HELP_SUFFIX: &str = r#"Use "{prog} --daemon --help" to see the daemon-mode command-line options.
Please see the {prog}(1) and {prog}d.conf(5) manpages for full documentation.{url_line}
"#;

pub fn program_name() -> String {
    env::var("OC_RSYNC_BRAND_NAME")
        .or_else(|_| {
            option_env!("OC_RSYNC_BRAND_NAME")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| "oc-rsync".to_string())
}

pub fn brand_version() -> String {
    let prefix = env::var("OC_RSYNC_VERSION_PREFIX")
        .or_else(|_| {
            option_env!("OC_RSYNC_VERSION_PREFIX")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_default();
    format!("{}{}", prefix, env!("CARGO_PKG_VERSION"))
}

pub fn brand_tagline() -> String {
    env::var("OC_RSYNC_BRAND_TAGLINE")
        .or_else(|_| {
            option_env!("OC_RSYNC_BRAND_TAGLINE")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| DEFAULT_TAGLINE.to_string())
}

pub fn brand_url() -> String {
    env::var("OC_RSYNC_BRAND_URL")
        .or_else(|_| {
            option_env!("OC_RSYNC_BRAND_URL")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| DEFAULT_URL.to_string())
}

pub fn brand_copyright() -> String {
    env::var("OC_RSYNC_BRAND_COPYRIGHT")
        .or_else(|_| {
            option_env!("OC_RSYNC_BRAND_COPYRIGHT")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| DEFAULT_COPYRIGHT.to_string())
}

pub fn hide_credits() -> bool {
    env::var("OC_RSYNC_HIDE_CREDITS")
        .or_else(|_| {
            option_env!("OC_RSYNC_HIDE_CREDITS")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .map(|v| v != "0")
        .unwrap_or(false)
}

pub fn help_prefix() -> String {
    env::var("OC_RSYNC_HELP_HEADER")
        .or_else(|_| {
            option_env!("OC_RSYNC_HELP_HEADER")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| DEFAULT_HELP_PREFIX.to_string())
}

pub fn help_suffix() -> String {
    DEFAULT_HELP_SUFFIX.to_string()
}

pub fn version_header() -> Option<String> {
    env::var("OC_RSYNC_VERSION_HEADER")
        .or_else(|_| {
            option_env!("OC_RSYNC_VERSION_HEADER")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .ok()
}
