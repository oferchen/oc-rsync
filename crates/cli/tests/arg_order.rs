// crates/cli/tests/arg_order.rs
use oc_rsync_cli::{cli_command, ARG_ORDER};

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
fn arg_order_parity() {
    let cmd = cli_command();
    let args: Vec<_> = cmd
        .get_arguments()
        .filter(|a| !a.is_hide_set() && !a.is_positional())
        .filter(|a| !SKIP_ARGS.contains(&a.get_id().as_str()))
        .collect();

    for arg in &args {
        let id = arg.get_id().as_str();
        let count = ARG_ORDER.iter().filter(|&&v| v == id).count();
        assert_eq!(count, 1, "{id} appears {count} times in ARG_ORDER");
    }

    assert_eq!(ARG_ORDER.len(), args.len());
}
