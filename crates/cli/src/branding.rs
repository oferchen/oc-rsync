// crates/cli/src/branding.rs
use std::env;

pub const DEFAULT_HELP_PREFIX: &str = r#"rsync comes with ABSOLUTELY NO WARRANTY.  This is free software, and you
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
See https://rsync.samba.org/ for updates, bug reports, and answers
"#;

pub fn program_name() -> String {
    env::var("PROGRAM_NAME")
        .or_else(|_| {
            option_env!("PROGRAM_NAME")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| "rsync".to_string())
}

pub fn help_prefix() -> String {
    env::var("OC_RSYNC_BRAND_HEADER")
        .or_else(|_| {
            option_env!("OC_RSYNC_BRAND_HEADER")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| DEFAULT_HELP_PREFIX.replace("rsync", &program_name()))
}

pub fn help_suffix() -> String {
    env::var("OC_RSYNC_BRAND_FOOTER")
        .or_else(|_| {
            option_env!("OC_RSYNC_BRAND_FOOTER")
                .map(str::to_string)
                .ok_or(env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| DEFAULT_HELP_SUFFIX.replace("rsync", &program_name()))
}
