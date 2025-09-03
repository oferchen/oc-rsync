// tests/blocking_io.rs
use assert_cmd::Command;
use std::process::Command as StdCommand;

fn sanitize(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .lines()
        .take_while(|line| !line.trim_end().is_empty())
        .filter(|line| {
            !(line.starts_with("oc-rsync")
                || line.starts_with("rsync ")
                || line.contains("official")
                || line.starts_with("Copyright")
                || line.starts_with("are welcome")
                || line.starts_with("General Public"))
        })
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn version_matches_upstream_nonblocking() {
    if StdCommand::new("which")
        .arg("rsync")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        return;
    }

    let oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .arg("--version")
        .output()
        .unwrap();
    assert!(oc_output.status.success());

    let up_output = StdCommand::new("rsync")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .arg("--version")
        .output()
        .unwrap();
    assert!(up_output.status.success());

    let ours = sanitize(&oc_output.stdout);
    if ours.is_empty() {
        return;
    }
    let upstream = sanitize(&up_output.stdout);
    assert_eq!(ours, upstream);
}

#[test]
fn version_matches_upstream_blocking() {
    if StdCommand::new("which")
        .arg("rsync")
        .output()
        .map(|o| !o.status.success())
        .unwrap_or(true)
    {
        return;
    }

    let oc_output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .args(["--blocking-io", "--version"])
        .output()
        .unwrap();
    assert!(oc_output.status.success());

    let up_output = StdCommand::new("rsync")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .env("COLUMNS", "80")
        .args(["--blocking-io", "--version"])
        .output()
        .unwrap();
    assert!(up_output.status.success());

    let ours = sanitize(&oc_output.stdout);
    if ours.is_empty() {
        return;
    }
    let upstream = sanitize(&up_output.stdout);
    assert_eq!(ours, upstream);
}
