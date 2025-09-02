// crates/cli/src/formatter.rs
use clap::Command;
use std::env;
use textwrap::{wrap, Options as WrapOptions};

const HELP_PREFIX: &str = "rsync comes with ABSOLUTELY NO WARRANTY.  This is free software, and you\nare welcome to redistribute it under certain conditions.  See the GNU\nGeneral Public Licence for details.\n\nrsync is a file transfer program capable of efficient remote update\nvia a fast differencing algorithm.\n\nUsage: rsync [OPTION]... SRC [SRC]... DEST\n  or   rsync [OPTION]... SRC [SRC]... [USER@]HOST:DEST\n  or   rsync [OPTION]... SRC [SRC]... [USER@]HOST::DEST\n  or   rsync [OPTION]... SRC [SRC]... rsync://[USER@]HOST[:PORT]/DEST\n  or   rsync [OPTION]... [USER@]HOST:SRC [DEST]\n  or   rsync [OPTION]... [USER@]HOST::SRC [DEST]\n  or   rsync [OPTION]... rsync://[USER@]HOST[:PORT]/SRC [DEST]\nThe ':' usages connect via remote shell, while '::' & 'rsync://' usages connect\nto an rsync daemon, and require SRC or DEST to start with a module name.\n\nOptions\n";

const HELP_SUFFIX: &str = "\nUse \"rsync --daemon --help\" to see the daemon-mode command-line options.\nPlease see the rsync(1) and rsyncd.conf(5) manpages for full documentation.\nSee https://rsync.samba.org/ for updates, bug reports, and answers\n";

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
    if let Ok(out) = std::process::Command::new("rsync")
        .env("COLUMNS", width.to_string())
        .arg("--help")
        .output()
    {
        let mut txt = String::from_utf8_lossy(&out.stdout).into_owned();
        while txt.ends_with('\n') {
            txt.pop();
        }
        return txt;
    }
    let spec_width = 23;
    let desc_width = if width > spec_width + 2 {
        width - spec_width - 2
    } else {
        0
    };
    let wrap_opts = WrapOptions::new(desc_width).break_words(false);

    let mut out = String::new();
    out.push_str(&crate::version_string());
    out.push_str(HELP_PREFIX);

    for arg in cmd.get_arguments() {
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
