// crates/cli/src/branding.rs
use std::env;

pub const DEFAULT_BRAND_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_BRAND_CREDITS: &str =
    "Automatic Rust re-implementation by Ofer Chen (2025). Not affiliated with Rsync team at Samba.";
pub const DEFAULT_BRAND_URL: &str = "https://github.com/oferchen/oc-rsync";

pub const DEFAULT_TAGLINE: &str = "Pure-Rust reimplementation of rsync (protocol v32).";
pub const DEFAULT_URL: &str = DEFAULT_BRAND_URL;
pub const DEFAULT_UPSTREAM_NAME: &str = "rsync";

pub const DEFAULT_HELP_PREFIX: &str = r#"{prog} {version}
{credits}

rsync comes with ABSOLUTELY NO WARRANTY.  This is free software, and you
are welcome to redistribute it under certain conditions.  See the GNU
General Public Licence for details.

rsync is a file transfer program capable of efficient remote update
via a fast differencing algorithm.

Usage: rsync [OPTION]... SRC [SRC]... DEST
  or   rsync [OPTION]... SRC [SRC]... [USER@]HOST:DEST
  or   rsync [OPTION]... SRC [SRC]... [USER@]HOST::DEST
  or   rsync [OPTION]... SRC [SRC]... rsync://[USER@]HOST[:PORT]/DEST
  or   rsync [OPTION]... [USER@]HOST:SRC [DEST]
  or   rsync [OPTION]... [USER@]HOST::SRC [DEST]
  or   rsync [OPTION]... rsync://[USER@]HOST[:PORT]/SRC [DEST]
The ':' usages connect via remote shell, while '::' & 'rsync://' usages connect
to an rsync daemon, and require SRC or DEST to start with a module name.

Options
"#;

pub const DEFAULT_HELP_SUFFIX: &str = r#"
Use "rsync --daemon --help" to see the daemon-mode command-line options.
Please see the rsync(1) and rsyncd.conf(5) manpages for full documentation.
For project updates and documentation, visit {url}.
"#;

fn option_env_lookup(key: &str) -> Option<&'static str> {
    match key {
        "OC_RSYNC_BRAND_NAME" => option_env!("OC_RSYNC_BRAND_NAME"),
        "OC_RSYNC_UPSTREAM_NAME" => option_env!("OC_RSYNC_UPSTREAM_NAME"),
        "OC_RSYNC_VERSION_PREFIX" => option_env!("OC_RSYNC_VERSION_PREFIX"),
        "OC_RSYNC_BRAND_TAGLINE" => option_env!("OC_RSYNC_BRAND_TAGLINE"),
        "OC_RSYNC_BRAND_URL" => option_env!("OC_RSYNC_BRAND_URL"),
        "OC_RSYNC_BRAND_CREDITS" => option_env!("OC_RSYNC_BRAND_CREDITS"),
        "OC_RSYNC_BRAND_COPYRIGHT" => option_env!("OC_RSYNC_BRAND_COPYRIGHT"),
        "OC_RSYNC_HIDE_CREDITS" => option_env!("OC_RSYNC_HIDE_CREDITS"),
        "OC_RSYNC_HELP_HEADER" => option_env!("OC_RSYNC_HELP_HEADER"),
        "OC_RSYNC_BRAND_HEADER" => option_env!("OC_RSYNC_BRAND_HEADER"),
        "OC_RSYNC_HELP_FOOTER" => option_env!("OC_RSYNC_HELP_FOOTER"),
        "OC_RSYNC_BRAND_FOOTER" => option_env!("OC_RSYNC_BRAND_FOOTER"),
        "OC_RSYNC_VERSION_HEADER" => option_env!("OC_RSYNC_VERSION_HEADER"),
        "BUILD_REVISION" => option_env!("BUILD_REVISION"),
        _ => None,
    }
}

fn env_or_option(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .or_else(|| option_env_lookup(key).map(str::to_string))
}

pub fn program_name() -> String {
    env_or_option("OC_RSYNC_BRAND_NAME").unwrap_or_else(|| "oc-rsync".to_string())
}

pub fn upstream_name() -> String {
    env_or_option("OC_RSYNC_UPSTREAM_NAME").unwrap_or_else(|| DEFAULT_UPSTREAM_NAME.to_string())
}

pub fn brand_version() -> String {
    let prefix = env_or_option("OC_RSYNC_VERSION_PREFIX").unwrap_or_default();
    format!("{}{}", prefix, DEFAULT_BRAND_VERSION)
}

pub fn brand_tagline() -> String {
    env_or_option("OC_RSYNC_BRAND_TAGLINE").unwrap_or_else(|| DEFAULT_TAGLINE.to_string())
}

pub fn brand_url() -> String {
    env_or_option("OC_RSYNC_BRAND_URL").unwrap_or_else(|| DEFAULT_URL.to_string())
}

pub fn brand_credits() -> String {
    env_or_option("OC_RSYNC_BRAND_CREDITS").unwrap_or_else(|| DEFAULT_BRAND_CREDITS.to_string())
}

fn default_copyright() -> String {
    let year = option_env!("CURRENT_YEAR").unwrap_or("2025");
    format!("Copyright (C) 2024-{year} oc-rsync contributors.")
}

pub fn brand_copyright() -> String {
    env::var("OC_RSYNC_BRAND_COPYRIGHT")
        .or_else(|_| {
            option_env!("OC_RSYNC_BRAND_COPYRIGHT")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| default_copyright())
}

pub fn hide_credits() -> bool {
    env_or_option("OC_RSYNC_HIDE_CREDITS")
        .map(|v| v != "0")
        .unwrap_or(false)
}

pub fn help_prefix() -> String {
    env_or_option("OC_RSYNC_HELP_HEADER")
        .or_else(|| env_or_option("OC_RSYNC_BRAND_HEADER"))
        .unwrap_or_else(|| DEFAULT_HELP_PREFIX.to_string())
}

#[allow(clippy::let_and_return)]
pub fn help_suffix() -> String {
    let suffix = env_or_option("OC_RSYNC_HELP_FOOTER")
        .or_else(|| env_or_option("OC_RSYNC_BRAND_FOOTER"))
        .unwrap_or_else(|| DEFAULT_HELP_SUFFIX.to_string());
    suffix
}

pub fn version_header() -> Option<String> {
    env_or_option("OC_RSYNC_VERSION_HEADER")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn env_or_option_respects_precedence() {
        std::env::remove_var("BUILD_REVISION");
        assert_eq!(env_or_option("BUILD_REVISION"), Some("unknown".to_string()));

        std::env::set_var("BUILD_REVISION", "runtime");
        assert_eq!(env_or_option("BUILD_REVISION"), Some("runtime".to_string()));
        std::env::remove_var("BUILD_REVISION");

        assert_eq!(env_or_option("NON_EXISTENT_KEY"), None);
    }

    #[test]
    #[serial]
    fn program_name_defaults_when_unset() {
        std::env::remove_var("OC_RSYNC_BRAND_NAME");
        if option_env!("OC_RSYNC_BRAND_NAME").is_none() {
            assert_eq!(program_name(), "oc-rsync");
        }
    }
}
