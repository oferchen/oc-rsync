// crates/cli/tests/flag_precedence.rs
use clap::error::ErrorKind;
use oc_rsync_cli::cli_command;

#[test]
fn last_flag_wins() {
    let m = cli_command()
        .try_get_matches_from(["oc-rsync", "--perms", "--no-perms", "src", "dst"])
        .unwrap();
    assert!(!m.get_flag("perms"));
    assert!(m.get_flag("no_perms"));

    let m = cli_command()
        .try_get_matches_from(["oc-rsync", "--no-perms", "--perms", "src", "dst"])
        .unwrap();
    assert!(m.get_flag("perms"));
    assert!(!m.get_flag("no_perms"));
}

#[test]
fn copy_links_conflicts_with_safe_links() {
    let err = cli_command()
        .try_get_matches_from(["oc-rsync", "--copy-links", "--safe-links", "src", "dst"])
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
}
