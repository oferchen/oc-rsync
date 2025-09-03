// tests/cvs_exclude.rs

use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn cvs_exclude_parity() {
    let rsync_version = StdCommand::new("rsync")
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|out| {
            out.lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(2))
                .and_then(|v| v.split('.').next())
                .and_then(|v| v.parse::<u32>().ok())
        });
    if rsync_version.is_none_or(|v| v < 3) {
        eprintln!("skipping cvs_exclude_parity test; rsync >=3 required");
        return;
    }

    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(src.join(".git")).unwrap();
    fs::write(src.join(".git/file"), "git").unwrap();
    fs::write(src.join("keep.txt"), "keep").unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("foo.o"), "obj").unwrap();
    fs::write(src.join("env_ignored"), "env").unwrap();
    fs::write(src.join("home_ignored"), "home").unwrap();
    fs::write(src.join("local_ignored"), "local").unwrap();
    fs::write(src.join(".cvsignore"), "local_ignored\n").unwrap();

    let sub = src.join("sub");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("local_ignored"), "sublocal").unwrap();
    fs::write(sub.join("env_ignored"), "env").unwrap();
    fs::write(sub.join("home_ignored"), "home").unwrap();
    fs::write(sub.join("sub_ignored"), "sub").unwrap();
    fs::write(sub.join(".cvsignore"), "sub_ignored\n").unwrap();

    let nested = sub.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("sub_ignored"), "nested").unwrap();

    let home = tempdir().unwrap();
    fs::write(home.path().join(".cvsignore"), "home_ignored\n").unwrap();

    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    let src_arg = format!("{}/", src.display());

    let mut rsync_cmd = StdCommand::new("rsync");
    rsync_cmd.env("CVSIGNORE", "env_ignored");
    rsync_cmd.env("HOME", home.path());
    rsync_cmd.args(["-r", "--quiet", "--cvs-exclude"]);
    rsync_cmd.arg(&src_arg);
    rsync_cmd.arg(&rsync_dst);
    let rsync_out = rsync_cmd.output().unwrap();
    assert!(rsync_out.status.success());
    let rsync_output = String::from_utf8_lossy(&rsync_out.stdout).to_string()
        + &String::from_utf8_lossy(&rsync_out.stderr);

    let mut ours_cmd = Command::cargo_bin("oc-rsync").unwrap();
    ours_cmd.env("CVSIGNORE", "env_ignored");
    ours_cmd.env("HOME", home.path());
    ours_cmd.args(["--recursive", "--cvs-exclude"]);
    ours_cmd.arg(&src_arg);
    ours_cmd.arg(&ours_dst);
    let ours_out = ours_cmd.output().unwrap();
    assert!(ours_out.status.success());
    let mut ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);
    ours_output = ours_output.replace("recursive mode enabled\n", "");

    assert_eq!(rsync_output, ours_output);

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(&rsync_dst)
        .arg(&ours_dst)
        .output()
        .unwrap();
    assert!(diff.status.success(), "directory trees differ");
}
