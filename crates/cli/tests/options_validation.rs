// crates/cli/tests/options_validation.rs
use oc_rsync_cli::{ClientOptsBuilder, cli_command, validate_paths};
use serial_test::serial;

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
    temp_env::with_var("RSYNC_PROTECT_ARGS", Some("1"), || {
        let matches = cli_command()
            .try_get_matches_from(["prog", "src", "dst"])
            .unwrap();
        let opts = ClientOptsBuilder::from_matches(&matches).build().unwrap();
        assert!(opts.secluded_args);
    });
}

#[test]
fn validate_paths_requires_dst() {
    let matches = cli_command().try_get_matches_from(["prog", "src"]).unwrap();
    let opts = ClientOptsBuilder::from_matches(&matches).build().unwrap();
    assert!(validate_paths(&opts).is_err());
}
