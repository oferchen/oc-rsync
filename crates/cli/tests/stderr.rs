// crates/cli/tests/stderr.rs
use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn local_sync_without_flag_emits_error_on_stderr() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    let assert = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([src_path.to_str().unwrap(), dst_path.to_str().unwrap()])
        .assert()
        .failure();
    assert!(!assert.get_output().stderr.is_empty());
}

#[test]
fn invalid_rsh_env_emits_error_on_stderr() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    let assert = Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--rsh",
            "1BAD=val ssh",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .assert()
        .failure();
    assert!(!assert.get_output().stderr.is_empty());
}
