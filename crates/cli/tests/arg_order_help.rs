// crates/cli/tests/arg_order_help.rs
use oc_rsync_cli::{ARG_ORDER, cli_command};
use std::collections::{HashMap, HashSet};

const HELP_TEXT: &str = include_str!("../resources/rsync-help-80.txt");

const SKIP_ARGS: &[&str] = &[
    "config",
    "daemon",
    "dparam",
    "hosts_allow",
    "hosts_deny",
    "known_hosts",
    "lock_file",
    "module",
    "motd",
    "no_detach",
    "no_devices",
    "no_group",
    "no_host_key_checking",
    "no_links",
    "no_owner",
    "no_perms",
    "no_specials",
    "no_times",
    "no_whole_file",
    "peer_version",
    "pid_file",
    "probe",
    "secrets_file",
    "state_dir",
    "syslog",
    "journald",
    "no_acls",
    "super_user",
];

#[test]
fn arg_order_matches_upstream_help() {
    let cmd = cli_command();
    let implemented: HashSet<String> = cmd
        .get_arguments()
        .filter(|a| !a.is_hide_set() && !a.is_positional())
        .map(|a| a.get_id().to_string())
        .filter(|id| !SKIP_ARGS.contains(&id.as_str()))
        .collect();

    let renames: HashMap<&str, &str> = [
        ("8-bit-output", "eight_bit_output"),
        ("log-file", "client-log-file"),
        ("log-file-format", "client-log-file-format"),
        ("contimeout", "connect_timeout"),
    ]
    .into();

    let mut derived = Vec::new();
    for line in HELP_TEXT.lines() {
        let Some(rest) = line.strip_prefix("--") else {
            continue;
        };
        let token = rest.split_whitespace().next().unwrap_or("");
        let token = token.trim_end_matches(',');
        let name = token.split('=').next().unwrap();

        let id = if let Some(&mapped) = renames.get(name) {
            mapped.to_string()
        } else {
            name.replace('-', "_")
        };

        if implemented.contains(&id) {
            derived.push(id);
        }
    }

    let derived_refs: Vec<&str> = derived.iter().map(|s| s.as_str()).collect();
    let derived_set: HashSet<&str> = derived_refs.iter().copied().collect();
    let arg_order_filtered: Vec<&str> = ARG_ORDER
        .iter()
        .copied()
        .filter(|id| derived_set.contains(id))
        .collect();
    assert_eq!(derived_refs, arg_order_filtered);
}
