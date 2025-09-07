// tests/comment_lint_spaces.rs

use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn comment_lint_handles_space_paths() {
    let worktree = tempdir().expect("tempdir");
    let worktree_path = worktree.path();

    let status = Command::new("git")
        .args([
            "worktree",
            "add",
            "--detach",
            worktree_path.to_str().unwrap(),
        ])
        .status()
        .expect("git worktree add");
    assert!(status.success());

    let file_path = worktree_path.join("space file.rs");
    fs::write(&file_path, "// space file.rs\n").expect("write file");
    let status = Command::new("git")
        .args([
            "-C",
            worktree_path.to_str().unwrap(),
            "add",
            "space file.rs",
        ])
        .status()
        .expect("git add");
    assert!(status.success());

    let status = Command::new("bash")
        .arg("tools/comment_lint.sh")
        .current_dir(worktree_path)
        .status()
        .expect("run comment_lint.sh");
    assert!(status.success());

    let _ = Command::new("git")
        .args([
            "worktree",
            "remove",
            "--force",
            worktree_path.to_str().unwrap(),
        ])
        .status();
}
