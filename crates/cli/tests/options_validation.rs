// crates/cli/tests/options_validation.rs
use oc_rsync_cli::{ClientOptsBuilder, cli_command, validate_paths};

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
fn builder_respects_protect_args_env() {
    unsafe {
        std::env::set_var("RSYNC_PROTECT_ARGS", "1");
    }
    let matches = cli_command()
        .try_get_matches_from(["prog", "src", "dst"])
        .unwrap();
    let opts = ClientOptsBuilder::from_matches(&matches).build().unwrap();
    assert!(opts.secluded_args);
    unsafe {
        std::env::remove_var("RSYNC_PROTECT_ARGS");
    }
}

#[test]
fn validate_paths_requires_dst() {
    let matches = cli_command().try_get_matches_from(["prog", "src"]).unwrap();
    let opts = ClientOptsBuilder::from_matches(&matches).build().unwrap();
    assert!(validate_paths(&opts).is_err());
}
