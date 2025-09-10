// crates/cli/tests/options_validation.rs
use oc_rsync_cli::{ClientOptsBuilder, cli_command, validate_paths};
use serial_test::serial;
use std::env;

fn set_env_var(key: &str, val: &str) {
    env::set_var(key, val);
}

fn remove_env_var(key: &str) {
    env::remove_var(key);
}

#[test]
fn builder_sets_no_d_alias() {
    let matches = cli_command()
        .try_get_matches_from(["prog", "--no-D", "src", "dst"])
        .unwrap();
    let opts = ClientOptsBuilder::from_matches(&matches).build().unwrap();
    assert!(opts.no_devices);
    assert!(opts.no_specials);
}

#[test]
#[serial]
fn builder_respects_protect_args_env() {
    set_env_var("RSYNC_PROTECT_ARGS", "1");
    let matches = cli_command()
        .try_get_matches_from(["prog", "src", "dst"])
        .unwrap();
    let opts = ClientOptsBuilder::from_matches(&matches).build().unwrap();
    assert!(opts.secluded_args);
    remove_env_var("RSYNC_PROTECT_ARGS");
}

#[test]
fn validate_paths_requires_dst() {
    let matches = cli_command().try_get_matches_from(["prog", "src"]).unwrap();
    let opts = ClientOptsBuilder::from_matches(&matches).build().unwrap();
    assert!(validate_paths(&opts).is_err());
}
