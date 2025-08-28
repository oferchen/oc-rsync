use std::fs;
use tempfile::tempdir;
use walk::walk;

#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(windows)]
use std::os::windows::fs::symlink_file;

#[test]
fn walk_includes_files_dirs_and_symlinks() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir(root.join("dir")).unwrap();
    fs::write(root.join("dir/file.txt"), b"hello").unwrap();
    fs::write(root.join("top.txt"), b"world").unwrap();
    let link_path = root.join("link.txt");
    #[cfg(unix)]
    symlink(root.join("top.txt"), &link_path).unwrap();
    #[cfg(windows)]
    symlink_file(root.join("top.txt"), &link_path).unwrap();

    let entries: Vec<_> = walk(root).collect::<Result<Vec<_>, _>>().unwrap();
    assert!(entries.iter().any(|(p, t)| p == &root && t.is_dir()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &root.join("dir") && t.is_dir()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &root.join("dir/file.txt") && t.is_file()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &root.join("top.txt") && t.is_file()));
    assert!(entries
        .iter()
        .any(|(p, t)| p == &link_path && t.is_symlink()));
}
