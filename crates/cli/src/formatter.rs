// crates/cli/src/formatter.rs
use clap::Command;
use once_cell::sync::Lazy;
use regex::Regex;
use std::env;
use textwrap::{Options as WrapOptions, wrap};

use crate::branding;

const RSYNC_HELP: &str = include_str!("../resources/rsync-help-80.txt");

const UPSTREAM_HELP_PREFIX: &str = r#"rsync comes with ABSOLUTELY NO WARRANTY.  This is free software, and you
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

const UPSTREAM_HELP_SUFFIX: &str = r#"Use "rsync --daemon --help" to see the daemon-mode command-line options.
Please see the rsync(1) and rsyncd.conf(5) manpages for full documentation.
See https://rsync.samba.org/ for updates, bug reports, and answers
"#;

static RSYNC_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\brsync\b").unwrap());

static UPSTREAM_OPTS: Lazy<Vec<(String, String)>> = Lazy::new(|| {
    let mut opts = Vec::new();
    let mut in_opts = false;
    for line in RSYNC_HELP.lines() {
        if line.trim() == "Options" {
            in_opts = true;
            continue;
        }
        if !in_opts {
            continue;
        }
        if line.starts_with("Use \"rsync --daemon --help\"") {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(idx) = line.find("  ") {
            let (spec, desc) = line.split_at(idx);
            opts.push((spec.trim().to_string(), desc.trim().to_string()));
        } else if let Some((_, last)) = opts.last_mut() {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(line.trim());
        }
    }
    opts
});

pub const ARG_ORDER: &[&str] = &[
    "verbose",
    "info",
    "debug",
    "stderr",
    "quiet",
    "no_motd",
    "checksum",
    "archive",
    "recursive",
    "relative",
    "no_implied_dirs",
    "backup",
    "backup_dir",
    "suffix",
    "update",
    "inplace",
    "append",
    "append_verify",
    "dirs",
    "old_dirs",
    "mkpath",
    "links",
    "copy_links",
    "copy_unsafe_links",
    "safe_links",
    "munge_links",
    "copy_dirlinks",
    "keep_dirlinks",
    "hard_links",
    "perms",
    "executability",
    "chmod",
    "acls",
    "xattrs",
    "owner",
    "group",
    "devices",
    "copy_devices",
    "write_devices",
    "specials",
    "devices_specials",
    "no_D",
    "times",
    "atimes",
    "open_noatime",
    "crtimes",
    "omit_dir_times",
    "omit_link_times",
    "fake_super",
    "sparse",
    "preallocate",
    "dry_run",
    "whole_file",
    "checksum_choice",
    "one_file_system",
    "block_size",
    "rsh",
    "rsync_path",
    "existing",
    "ignore_existing",
    "remove_source_files",
    "delete",
    "delete_before",
    "delete_during",
    "delete_delay",
    "delete_after",
    "delete_excluded",
    "ignore_missing_args",
    "delete_missing_args",
    "ignore_errors",
    "force",
    "max_delete",
    "max_size",
    "min_size",
    "max_alloc",
    "partial",
    "partial_dir",
    "delay_updates",
    "prune_empty_dirs",
    "numeric_ids",
    "usermap",
    "groupmap",
    "chown",
    "timeout",
    "connect_timeout",
    "ignore_times",
    "size_only",
    "modify_window",
    "temp_dir",
    "fuzzy",
    "compare_dest",
    "copy_dest",
    "link_dest",
    "compress",
    "compress_choice",
    "compress_level",
    "skip_compress",
    "cvs_exclude",
    "filter",
    "filter_file",
    "filter_shorthand",
    "exclude",
    "exclude_from",
    "include",
    "include_from",
    "files_from",
    "from0",
    "old_args",
    "secluded_args",
    "trust_sender",
    "copy_as",
    "address",
    "port",
    "sockopts",
    "blocking_io",
    "outbuf",
    "stats",
    "eight_bit_output",
    "human_readable",
    "progress",
    "partial_progress",
    "itemize_changes",
    "remote_option",
    "out_format",
    "client-log-file",
    "client-log-file-format",
    "password_file",
    "early_input",
    "list_only",
    "bwlimit",
    "stop_after",
    "stop_at",
    "fsync",
    "write_batch",
    "only_write_batch",
    "read_batch",
    "protocol",
    "iconv",
    "checksum_seed",
    "ipv4",
    "ipv6",
];

fn columns() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(80)
}

pub fn apply(mut cmd: Command) -> Command {
    let width = columns();
    cmd = cmd.term_width(width);
    cmd
}

pub fn render_help(_cmd: &Command) -> String {
    let width = columns();
    let program = branding::program_name();
    let version = branding::brand_version();
    let credits = if branding::hide_credits() {
        String::new()
    } else {
        branding::brand_credits()
    };
    let url = if branding::hide_credits() {
        String::new()
    } else {
        branding::brand_url()
    };
    let upstream = branding::upstream_name();
    let mut help_prefix = branding::help_prefix();
    let mut help_suffix = branding::help_suffix();
    for s in [&mut help_prefix, &mut help_suffix] {
        let haystack = s.clone();
        *s = RSYNC_RE
            .replace_all(&haystack, |caps: &regex::Captures| {
                let m = caps.get(0).unwrap();
                let start = m.start();
                let end = m.end();
                let prev_char = haystack[..start].chars().last();
                let tail = &haystack[end..];
                if tail.starts_with("://") || tail.starts_with('/') || prev_char == Some('/') {
                    "rsync".to_string()
                } else {
                    upstream.clone()
                }
            })
            .to_string()
            .replace("{prog}", &program)
            .replace("{version}", &version)
            .replace("{credits}", &credits)
            .replace("{url}", &url);
    }
    if width == 80 {
        let prefix_end = match RSYNC_HELP.find(UPSTREAM_HELP_PREFIX) {
            Some(idx) => idx + UPSTREAM_HELP_PREFIX.len(),
            None => {
                let mut out = String::new();
                out.push_str(&help_prefix);
                out.push_str(
                    "Failed to locate upstream help prefix; displaying unmodified help text.\n\n",
                );
                out.push_str(RSYNC_HELP);
                out.push_str(&help_suffix);
                return out;
            }
        };
        let suffix_start = match RSYNC_HELP.rfind(UPSTREAM_HELP_SUFFIX) {
            Some(idx) => idx,
            None => {
                let mut out = String::new();
                out.push_str(&help_prefix);
                out.push_str(
                    "Failed to locate upstream help suffix; displaying unmodified help text.\n\n",
                );
                out.push_str(RSYNC_HELP);
                out.push_str(&help_suffix);
                return out;
            }
        };
        if prefix_end >= suffix_start {
            let mut out = String::new();
            out.push_str(&help_prefix);
            out.push_str("Upstream help markers are invalid; displaying unmodified help text.\n\n");
            out.push_str(RSYNC_HELP);
            out.push_str(&help_suffix);
            return out;
        }
        let body = &RSYNC_HELP[prefix_end..suffix_start];
        let mut out = String::new();
        out.push_str(&help_prefix);
        out.push_str(body);
        out.push_str(&help_suffix);
        return out;
    }
    let spec_width = 23;
    let desc_width = if width > spec_width + 2 {
        width - spec_width - 2
    } else {
        0
    };
    let wrap_opts =
        (desc_width > 0).then(|| WrapOptions::new(desc_width.max(1)).break_words(false));

    let mut out = String::new();
    out.push_str(&help_prefix);

    for (spec, desc) in UPSTREAM_OPTS.iter() {
        let pad = if spec.len() >= spec_width {
            2
        } else {
            spec_width - spec.len() + 2
        };
        let wrapped: Vec<String> = if let Some(opts) = &wrap_opts {
            wrap(desc, opts)
                .into_iter()
                .map(|c| c.into_owned())
                .collect()
        } else {
            vec![desc.to_string()]
        };
        if let Some((first, rest)) = wrapped.split_first() {
            out.push_str(spec);
            out.push_str(&" ".repeat(pad));
            out.push_str(first);
            out.push('\n');
            for line in rest {
                out.push_str(&" ".repeat(spec_width + 2));
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    out.push_str(&help_suffix);
    while out.ends_with('\n') {
        out.pop();
    }
    out
}

pub fn dump_help_body(cmd: &Command) -> String {
    let prev = std::env::var("COLUMNS").ok();
    unsafe {
        std::env::set_var("COLUMNS", "80");
    }
    let help = render_help(cmd);
    if let Some(v) = prev {
        unsafe {
            std::env::set_var("COLUMNS", v);
        }
    } else {
        unsafe {
            std::env::remove_var("COLUMNS");
        }
    }

    let mut out = String::new();
    let mut in_options = false;
    let stop_marker = "Use \"rsync --daemon --help\"";
    for line in help.lines() {
        if line.trim() == "Options" {
            in_options = true;
            continue;
        }
        if !in_options {
            continue;
        }
        if line.starts_with(stop_marker) {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        if let Some(idx) = line.find("  ") {
            let (spec, desc) = line.split_at(idx);
            if let Some(flag) = spec.split(',').find(|s| s.trim_start().starts_with("--")) {
                let flag = flag.trim();
                let desc = desc.trim_start();
                out.push_str(flag);
                out.push('\t');
                out.push_str(desc);
                out.push('\n');
            }
        }
    }
    out
}
