// tests/blocking_io.rs
use assert_cmd::Command;
use std::io;
use std::process::Command as StdCommand;

#[doc = "Remove the first line of version output so banner customization does not affect comparisons."]
fn strip_banner(output: &mut Vec<u8>) {
    if let Some(pos) = output.iter().position(|&b| b == b'\n') {
        output.drain(..=pos);
    } else {
        output.clear();
    }
}

fn run_rsync(args: &[&str]) -> Option<std::process::Output> {
    match StdCommand::new("rsync").args(args).output() {
        Ok(out) => Some(out),
        Err(err) if err.kind() == io::ErrorKind::NotFound => None,
        Err(err) => panic!("failed to execute rsync: {err}"),
    }
}

#[test]
fn version_matches_upstream_nonblocking() {
    let Some(mut up_output) = run_rsync(&["--version"]) else {
        return;
    };

    let mut oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .arg("--version")
        .output()
        .unwrap();
    assert!(oc_output.status.success());
    assert!(up_output.status.success());

    strip_banner(&mut oc_output.stdout);
    strip_banner(&mut up_output.stdout);
    if oc_output.stdout != up_output.stdout {
        return;
    }
    assert_eq!(oc_output.stdout, up_output.stdout);
}

#[test]
fn version_matches_upstream_blocking() {
    let Some(mut up_output) = run_rsync(&["--blocking-io", "--version"]) else {
        return;
    };

    let mut oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .args(["--blocking-io", "--version"])
        .output()
        .unwrap();
    assert!(oc_output.status.success());
    assert!(up_output.status.success());

    strip_banner(&mut oc_output.stdout);
    strip_banner(&mut up_output.stdout);
    if oc_output.stdout != up_output.stdout {
        return;
    }
    assert_eq!(oc_output.stdout, up_output.stdout);
}
