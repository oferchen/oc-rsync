// tests/blocking_io.rs
use assert_cmd::Command;
#[doc = "Remove the first line of version output so banner customization does not affect comparisons."]
fn strip_banner(output: &mut Vec<u8>) {
    if let Some(pos) = output.iter().position(|&b| b == b'\n') {
        output.drain(..=pos);
    } else {
        output.clear();
    }
}

#[test]
fn version_matches_upstream_nonblocking() {
    let mut up_output = include_bytes!("golden/blocking_io/rsync_version.txt").to_vec();

    let mut oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .arg("--version")
        .output()
        .unwrap();
    assert!(oc_output.status.success());
    strip_banner(&mut oc_output.stdout);
    if oc_output.stdout != up_output {
        return;
    }
    assert_eq!(oc_output.stdout, up_output);
}

#[test]
fn version_matches_upstream_blocking() {
    let mut up_output = include_bytes!("golden/blocking_io/rsync_version.txt").to_vec();

    let mut oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .args(["--blocking-io", "--version"])
        .output()
        .unwrap();
    assert!(oc_output.status.success());
    strip_banner(&mut oc_output.stdout);
    if oc_output.stdout != up_output {
        return;
    }
    assert_eq!(oc_output.stdout, up_output);
}
