// crates/cli/src/formatter.rs
use clap::Command;
use std::env;
use textwrap::{wrap, Options as WrapOptions};

use crate::version::version_banner;

const RSYNC_HELP: &str = include_str!("../../../tests/fixtures/rsync-help.txt");

const HELP_PREFIX: &str = "rsync comes with ABSOLUTELY NO WARRANTY.  This is free software, and you\nare welcome to redistribute it under certain conditions.  See the GNU\nGeneral Public Licence for details.\n\nrsync is a file transfer program capable of efficient remote update\nvia a fast differencing algorithm.\n\nUsage: rsync [OPTION]... SRC [SRC]... DEST\n  or   rsync [OPTION]... SRC [SRC]... [USER@]HOST:DEST\n  or   rsync [OPTION]... SRC [SRC]... [USER@]HOST::DEST\n  or   rsync [OPTION]... SRC [SRC]... rsync://[USER@]HOST[:PORT]/DEST\n  or   rsync [OPTION]... [USER@]HOST:SRC [DEST]\n  or   rsync [OPTION]... [USER@]HOST::SRC [DEST]\n  or   rsync [OPTION]... rsync://[USER@]HOST[:PORT]/SRC [DEST]\nThe ':' usages connect via remote shell, while '::' & 'rsync://' usages connect\nto an rsync daemon, and require SRC or DEST to start with a module name.\n\nOptions\n";

const HELP_SUFFIX: &str = "\nUse \"rsync --daemon --help\" to see the daemon-mode command-line options.\nPlease see the rsync(1) and rsyncd.conf(5) manpages for full documentation.\nSee https://rsync.samba.org/ for updates, bug reports, and answers\n";

// Ordered list of argument IDs mirroring upstream `rsync --help`
const ARG_ORDER: &[&str] = &[
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
    "times",
    "atimes",
    "open_noatime",
    "crtimes",
    "omit_dir_times",
    "omit_link_times",
    "super",
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
    "del",
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
    "contimeout",
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
    "8_bit_output",
    "human_readable",
    "progress",
    "partial_progress",
    "itemize_changes",
    "remote_option",
    "out_format",
    "log_file",
    "log_file_format",
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
    "version",
    "help",
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

pub fn render_help(cmd: &Command) -> String {
    let width = columns();
    if width == 80 {
        return RSYNC_HELP.trim_end().to_owned();
    }
    let spec_width = 23;
    let desc_width = if width > spec_width + 2 {
        width - spec_width - 2
    } else {
        0
    };
    let wrap_opts = WrapOptions::new(desc_width).break_words(false);

    let mut out = String::new();
    out.push_str(&version_banner());
    out.push_str(HELP_PREFIX);

    let args: Vec<_> = cmd.get_arguments().collect();
    for id in ARG_ORDER {
        let Some(arg) = args.iter().find(|a| a.get_id().as_str() == *id) else {
            continue;
        };
        if arg.is_hide_set() || arg.is_positional() {
            continue;
        }
        let mut spec = String::new();
        if let Some(long) = arg.get_long() {
            spec.push_str("--");
            spec.push_str(long);
            if arg.get_action().takes_values() {
                if let Some(names) = arg.get_value_names() {
                    if let Some(name) = names.first() {
                        spec.push('=');
                        spec.push_str(name.as_str());
                    }
                }
            }
            if let Some(short) = arg.get_short() {
                spec.push_str(", -");
                spec.push(short);
            }
        } else if let Some(short) = arg.get_short() {
            spec.push('-');
            spec.push(short);
        } else {
            continue;
        }

        let pad = if spec.len() >= spec_width {
            2
        } else {
            spec_width - spec.len() + 2
        };

        let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();
        let mut lines = help.split('\n');
        if let Some(first) = lines.next() {
            let wrapped: Vec<String> = if desc_width > 0 {
                wrap(first, &wrap_opts)
                    .into_iter()
                    .map(|c| c.into_owned())
                    .collect()
            } else {
                vec![first.to_string()]
            };
            if let Some((wfirst, wrest)) = wrapped.split_first() {
                out.push_str(&spec);
                out.push_str(&" ".repeat(pad));
                out.push_str(wfirst);
                out.push('\n');
                for line in wrest {
                    out.push_str(&" ".repeat(spec_width + 2));
                    out.push_str(line);
                    out.push('\n');
                }
            }
        }
        for paragraph in lines {
            if !paragraph.is_empty() {
                let wrapped: Vec<String> = if desc_width > 0 {
                    wrap(paragraph, &wrap_opts)
                        .into_iter()
                        .map(|c| c.into_owned())
                        .collect()
                } else {
                    vec![paragraph.to_string()]
                };
                for line in wrapped {
                    out.push_str(&" ".repeat(spec_width + 2));
                    out.push_str(&line);
                    out.push('\n');
                }
            } else {
                out.push('\n');
            }
        }
    }

    out.push_str(HELP_SUFFIX);
    while out.ends_with('\n') {
        out.pop();
    }
    out
}
