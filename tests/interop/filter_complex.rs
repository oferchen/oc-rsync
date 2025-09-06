// tests/interop/filter_complex.rs

use assert_cmd::Command;
use std::fs;
use std::process::Command as StdCommand;
use tempfile::tempdir;

#[test]
#[ignore = "requires rsync"]
fn complex_filter_cases_match_rsync() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let rsync_dst = tmp.path().join("rsync");
    let ours_dst = tmp.path().join("ours");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    fs::create_dir_all(src.join("keep/inner")).unwrap();
    fs::create_dir_all(src.join("skip")).unwrap();
    fs::write(src.join("keep/file.txt"), "keep").unwrap();
    fs::write(src.join("keep/inner/special.log"), "special").unwrap();
    fs::write(src.join("keep/inner/data.txt"), "data").unwrap();
    fs::write(src.join("keep/inner/data.tmp"), "tmpdata").unwrap();
    fs::write(src.join("skip/file.log"), "skip").unwrap();
    fs::write(src.join("tmp.tmp"), "tmp").unwrap();
    fs::write(src.join("top.log"), "top").unwrap();
    fs::write(src.join("keep/inner/.rsync-filter"), "+ special.log\n- *\n").unwrap();

    let src_arg = format!("{}/", src.display());
    let rules = [
        "--filter=+ tmp.tmp",
        "--filter=- *.tmp",
        "--filter=- skip/",
        "--filter=- *.log",
        "--filter=:- .rsync-filter",
    ];

    let mut rsync_ok = false;
    let rsync_output = match StdCommand::new("rsync")
        .args(["-r", "--quiet"])
        .args(&rules)
        .arg(&src_arg)
        .arg(&rsync_dst)
        .output()
    {
        Ok(out) if out.status.success() => {
            rsync_ok = true;
            String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr)
        }
        _ => fs::read_to_string(
            "tests/golden/filter_complex/complex_filter_cases_match_rsync.stdout",
        )
        .unwrap(),
    };

    let mut ours_cmd = Command::cargo_bin("oc-rsync").unwrap();
    ours_cmd.args(["--recursive"]);
    ours_cmd.args(&rules);
    ours_cmd.arg(&src_arg);
    ours_cmd.arg(&ours_dst);
    let ours_out = ours_cmd.output().unwrap();
    assert!(ours_out.status.success());
    let mut ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
        + &String::from_utf8_lossy(&ours_out.stderr);
    ours_output = ours_output.replace("recursive mode enabled\n", "");
    assert_eq!(rsync_output, ours_output);

    let diff_target: &std::path::Path = if rsync_ok {
        rsync_dst.as_path()
    } else {
        std::path::Path::new("tests/golden/filter_complex/expected")
    };
    let diff = StdCommand::new("diff")
        .arg("-r")
        .arg(diff_target)
        .arg(&ours_dst)
        .output()
        .unwrap();
    assert!(diff.status.success(), "directory trees differ");
}
