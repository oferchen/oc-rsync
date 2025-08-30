// tests/filter_corpus.rs
use assert_cmd::Command;
use shell_words::split;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::tempdir;

fn setup_basic(src: &Path) {
    fs::create_dir_all(src.join("keep/sub")).unwrap();
    fs::create_dir_all(src.join("keep/tmp")).unwrap();
    fs::create_dir_all(src.join("skip")).unwrap();
    fs::write(src.join("keep/file.txt"), "keep").unwrap();
    fs::write(src.join("keep/sub/file.md"), "sub").unwrap();
    fs::write(src.join("keep/tmp/file.tmp"), "tmp").unwrap();
    fs::write(src.join("skip/file.txt"), "skip").unwrap();
    fs::write(src.join("root.tmp"), "root").unwrap();
    fs::write(src.join("note.md"), "note").unwrap();
    fs::write(src.join("core"), "core").unwrap();
    fs::write(src.join("foo.o"), "obj").unwrap();
}

fn setup_perdir(src: &Path) {
    fs::create_dir_all(src.join("sub/nested")).unwrap();
    fs::write(src.join(".rsync-filter"), "- *.tmp\n").unwrap();
    fs::write(
        src.join("sub/.rsync-filter"),
        "+ nested/\n+ keep.tmp\n- *\n",
    )
    .unwrap();
    fs::write(src.join("sub/keep.tmp"), "keep").unwrap();
    fs::write(src.join("sub/other.tmp"), "other").unwrap();
    fs::write(src.join("sub/other.txt"), "other").unwrap();
    fs::write(src.join("sub/nested/.rsync-filter"), "+ keep.tmp\n- *\n").unwrap();
    fs::write(src.join("sub/nested/keep.tmp"), "nested").unwrap();
    fs::write(src.join("sub/nested/other.tmp"), "other").unwrap();
}

fn setup_edge(src: &Path) {
    fs::create_dir_all(src.join("dir")).unwrap();
    fs::create_dir_all(src.join("tmp")).unwrap();
    fs::write(src.join("root.txt"), "root").unwrap();
    fs::write(src.join("dir/root.txt"), "sub").unwrap();
    fs::write(src.join("tmp/file.txt"), "junk").unwrap();
}

#[test]
fn filter_corpus_parity() {
    let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/filter_corpus");
    for entry in fs::read_dir(&fixture_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rules") {
            continue;
        }
        let rules_line = fs::read_to_string(&path).unwrap();
        let args = split(rules_line.trim()).unwrap();
        let tmp = tempdir().unwrap();
        let src = tmp.path().join("src");
        let rsync_dst = tmp.path().join("rsync");
        let ours_dst = tmp.path().join("ours");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&rsync_dst).unwrap();
        fs::create_dir_all(&ours_dst).unwrap();
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap();
        if stem.starts_with("perdir") {
            setup_perdir(&src);
        } else if stem == "edge" {
            setup_edge(&src);
        } else {
            setup_basic(&src);
        }
        let src_arg = format!("{}/", src.display());

        let mut rsync_cmd = StdCommand::new("rsync");
        rsync_cmd.args(["-r", "--quiet"]);
        rsync_cmd.args(&args);
        rsync_cmd.arg(&src_arg);
        rsync_cmd.arg(&rsync_dst);
        let rsync_out = rsync_cmd.output().unwrap();
        assert!(rsync_out.status.success());
        let rsync_output = String::from_utf8_lossy(&rsync_out.stdout).to_string()
            + &String::from_utf8_lossy(&rsync_out.stderr);

        let mut ours_cmd = Command::cargo_bin("oc-rsync").unwrap();
        ours_cmd.args(["--local", "--recursive"]);
        ours_cmd.args(&args);
        ours_cmd.arg(&src_arg);
        ours_cmd.arg(&ours_dst);
        let ours_out = ours_cmd.output().unwrap();
        assert!(ours_out.status.success());
        let mut ours_output = String::from_utf8_lossy(&ours_out.stdout).to_string()
            + &String::from_utf8_lossy(&ours_out.stderr);
        ours_output = ours_output.replace("recursive mode enabled\n", "");
        assert_eq!(rsync_output, ours_output, "output mismatch for {:?}", path);

        let diff = StdCommand::new("diff")
            .arg("-r")
            .arg(&rsync_dst)
            .arg(&ours_dst)
            .output()
            .unwrap();
        assert!(
            diff.status.success(),
            "directory trees differ for {:?}",
            path
        );
    }
}

#[test]
fn ignores_parent_rsync_filter_with_ff() {
    let tmp = tempdir().unwrap();
    let parent = tmp.path();
    fs::write(parent.join(".rsync-filter"), "- file.txt\n").unwrap();

    let src = parent.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("file.txt"), "data").unwrap();

    let rsync_dst = parent.join("rsync");
    let ours_dst = parent.join("ours");
    fs::create_dir_all(&rsync_dst).unwrap();
    fs::create_dir_all(&ours_dst).unwrap();

    let src_arg = format!("{}/", src.display());

    let mut rsync_cmd = StdCommand::new("rsync");
    rsync_cmd.args([
        "-r",
        "--quiet",
        "--filter=: .rsync-filter",
        "--filter=- .rsync-filter",
    ]);
    rsync_cmd.arg(&src_arg);
    rsync_cmd.arg(&rsync_dst);
    let rsync_out = rsync_cmd.output().unwrap();
    assert!(rsync_out.status.success());
    let rsync_output = String::from_utf8_lossy(&rsync_out.stdout).to_string()
        + &String::from_utf8_lossy(&rsync_out.stderr);

    let mut ours_cmd = Command::cargo_bin("oc-rsync").unwrap();
    ours_cmd.args([
        "--local",
        "--recursive",
        "--filter=: .rsync-filter",
        "--filter=- .rsync-filter",
    ]);
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
