// crates/cli/tests/compress_level.rs
use clap::error::ErrorKind;
use oc_rsync_cli::cli_command;

#[test]
fn compress_level_accepts_range() {
    let mut cmd = cli_command();
    let matches = cmd
        .try_get_matches_from(["prog", "--compress-level", "0", "src", "dst"])
        .expect("valid level 0");
    assert_eq!(matches.get_one::<i32>("compress_level"), Some(&0));
    let mut cmd = cli_command();
    let matches = cmd
        .try_get_matches_from(["prog", "--compress-level", "9", "src", "dst"])
        .expect("valid level 9");
    assert_eq!(matches.get_one::<i32>("compress_level"), Some(&9));
}

#[test]
fn compress_level_rejects_out_of_range() {
    let mut cmd = cli_command();
    let err = cmd
        .try_get_matches_from(["prog", "--compress-level", "10", "src", "dst"])
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::ValueValidation);
    let mut cmd = cli_command();
    let err = cmd
        .try_get_matches_from(["prog", "--compress-level=-1", "src", "dst"])
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::ValueValidation);
}
