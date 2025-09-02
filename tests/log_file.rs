// tests/log_file.rs
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn log_file_writes_messages() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.txt");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "-v",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("verbose level set to 1"), "{}", contents);
    assert!(!contents.contains("src"), "{}", contents);
}

#[test]
fn log_file_format_json_writes_json() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.json");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "--log-file-format=json",
            "-v",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("\"message\""), "{}", contents);
}

#[test]
fn out_format_writes_custom_message() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    fs::write(&src, b"hi").unwrap();
    let log = tmp.path().join("log.txt");
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "--log-file",
            log.to_str().unwrap(),
            "--out-format=custom:%n",
            src.to_str().unwrap(),
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("custom:src"), "{}", contents);
}

#[test]
#[cfg(unix)]
fn out_format_supports_all_escapes() {
    use std::os::unix::fs::symlink;

    let tmp = tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("f"), b"hi").unwrap();
    symlink("f", src_dir.join("ln")).unwrap();
    let dst = tmp.path().join("dst");
    fs::create_dir_all(&dst).unwrap();
    let log = tmp.path().join("log.txt");
    let fmt = "\t%o:%n%L%i%%\\\n";
    let src_arg = format!("{}/", src_dir.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--local",
            "-l",
            "--log-file",
            log.to_str().unwrap(),
            &format!("--out-format={fmt}"),
            &src_arg,
            dst.to_str().unwrap(),
        ])
        .assert()
        .success();
    let contents = fs::read_to_string(&log).unwrap();
    assert!(contents.contains("\tsend:"), "{}", contents);
    assert!(contents.contains("ln -> f"), "{}", contents);
    assert!(contents.contains(">f"), "{}", contents);
    assert!(contents.contains("%\\\n"), "{}", contents);
}
