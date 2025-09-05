// tests/cvs_exclude.rs

use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
fn cvs_exclude_parity() {
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

    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&ours_dst).unwrap();

    let src_arg = format!("{}/", src.display());

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

    assert!(ours_output.is_empty());

    assert!(ours_dst.join("sub/local_ignored").exists());
    assert!(ours_dst.join("sub/nested/sub_ignored").exists());

    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg("tests/golden/cvs_exclude/expected")
        .arg(&ours_dst)
        .output()
        .unwrap();
    assert!(diff.status.success(), "directory trees differ");
}
