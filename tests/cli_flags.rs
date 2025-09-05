// tests/cli_flags.rs
use assert_cmd::Command;
#[cfg(unix)]
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use oc_rsync_cli::cli_command;
use predicates::str::contains;
use std::fs;
use std::net::TcpListener;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::Path;
use std::thread;
use std::time::Duration;
use tempfile::{tempdir, NamedTempFile};
use transport::tcp::TcpTransport;

#[test]
fn eight_bit_output_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--8-bit-output", "--version"])
        .assert()
        .success();
}

#[test]
fn blocking_io_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--blocking-io", "--version"])
        .assert()
        .success();
}

#[test]
fn blocking_io_nonblocking_by_default() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (_conn, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_millis(100));
    });
    let t = TcpTransport::connect(&addr.ip().to_string(), addr.port(), None, None).unwrap();
    #[cfg(unix)]
    {
        let stream = t.into_inner();
        let fd = stream.as_raw_fd();
        let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL).unwrap());
        assert!(flags.contains(OFlag::O_NONBLOCK));
    }
    #[cfg(not(unix))]
    {
        let _ = t.into_inner();
    }
    handle.join().unwrap();
}

#[test]
fn blocking_io_flag_enables_blocking_mode() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (_conn, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_millis(100));
    });
    let mut t = TcpTransport::connect(&addr.ip().to_string(), addr.port(), None, None).unwrap();
    t.set_blocking_io(true).unwrap();
    #[cfg(unix)]
    {
        let stream = t.into_inner();
        let fd = stream.as_raw_fd();
        let flags = OFlag::from_bits_truncate(fcntl(fd, FcntlArg::F_GETFL).unwrap());
        assert!(!flags.contains(OFlag::O_NONBLOCK));
    }
    #[cfg(not(unix))]
    {
        let _ = t.into_inner();
    }
    handle.join().unwrap();
}

#[test]
fn outbuf_flag_accepts_modes() {
    for mode in ["N", "L", "B"] {
        Command::cargo_bin("oc-rsync")
            .unwrap()
            .args([&format!("--outbuf={mode}"), "--version"])
            .assert()
            .success();
    }
}

#[test]
fn outbuf_flag_applies_to_stderr() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--outbuf=N", "nosuch"])
        .assert()
        .failure()
        .stderr(contains(
            "2 values required by '[SRC] [SRC]...'; only 1 was provided",
        ));
}

#[test]
fn early_input_flag_accepts_file() {
    let file = NamedTempFile::new().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--early-input", file.path().to_str().unwrap(), "--version"])
        .assert()
        .success();
}

#[test]
fn protocol_flag_accepts_version() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--protocol=31", "--version"])
        .assert()
        .success();
}

#[test]
fn log_file_flag_accepts_path() {
    let file = NamedTempFile::new().unwrap();
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--log-file", file.path().to_str().unwrap(), "--version"])
        .assert()
        .success();
}

#[test]
fn fsync_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--fsync", "--version"])
        .assert()
        .success();
}

#[test]
fn open_noatime_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--open-noatime", "--version"])
        .assert()
        .success();
}

#[test]
fn fuzzy_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--fuzzy", "--version"])
        .assert()
        .success();
}

#[test]
fn fake_super_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--fake-super", "--version"])
        .assert()
        .success();
}

#[test]
fn mkpath_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--mkpath", "--version"])
        .assert()
        .success();
}

#[test]
fn mkpath_missing_args_matches_rsync() {
    let oc = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg("--mkpath")
        .output()
        .unwrap();
    assert!(!oc.status.success());
}

#[test]
fn trust_sender_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--trust-sender", "--version"])
        .assert()
        .success();
}

#[test]
fn short_attribute_flags_are_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-p", "-o", "-g", "-t", "-l", "-D", "--version"])
        .assert()
        .success();
}

#[test]
fn remove_sent_files_alias_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--remove-sent-files", "--version"])
        .assert()
        .success();
}

#[test]
fn delete_flags_last_one_wins() {
    let matches = cli_command()
        .try_get_matches_from(["prog", "--delete-after", "--delete-before", "src", "dst"])
        .unwrap();
    assert!(matches.get_flag("delete_before"));
    assert!(!matches.get_flag("delete_after"));
}

#[test]
fn old_args_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--old-args", "--version"])
        .assert()
        .success();
}

#[test]
fn old_dirs_flag_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--old-dirs", "--version"])
        .assert()
        .success();
}

#[test]
fn old_d_alias_is_accepted() {
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["--old-d", "--version"])
        .assert()
        .success();
}

#[test]
fn old_dirs_flag_matches_rsync() {
    legacy_old_dirs_matches("--old-dirs");
}

#[test]
fn old_d_alias_matches_rsync() {
    legacy_old_dirs_matches("--old-d");
}

fn legacy_old_dirs_matches(flag: &str) {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    fs::create_dir_all(src.join("sub")).unwrap();
    fs::write(src.join("sub/file.txt"), b"data").unwrap();

    let oc_dst = tmp.path().join("oc");
    fs::create_dir_all(&oc_dst).unwrap();

    let src_arg = format!("{}/", src.display());
    let oc_dest = format!("{}/", oc_dst.display());

    let status_path = match flag {
        "--old-dirs" => "tests/golden/cli_flags/old-dirs.status",
        "--old-d" => "tests/golden/cli_flags/old-d.status",
        _ => unreachable!(),
    };
    let expected_status: i32 = fs::read_to_string(status_path)
        .unwrap()
        .trim()
        .parse()
        .unwrap();

    let output = Command::cargo_bin("oc-rsync")
        .unwrap()
        .arg(flag)
        .arg(&src_arg)
        .arg(&oc_dest)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(expected_status));

    let tree_path = match flag {
        "--old-dirs" => "tests/golden/cli_flags/old-dirs.tree",
        "--old-d" => "tests/golden/cli_flags/old-d.tree",
        _ => unreachable!(),
    };
    let expected: Vec<String> = fs::read_to_string(tree_path)
        .unwrap()
        .lines()
        .map(|s| s.to_string())
        .collect();

    fn collect_paths(root: &Path) -> Vec<String> {
        fn visit(dir: &Path, root: &Path, paths: &mut Vec<String>) {
            for entry in fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                let rel = path.strip_prefix(root).unwrap();
                paths.push(format!("/{}", rel.display()));
                if path.is_dir() {
                    visit(&path, root, paths);
                }
            }
        }
        let mut paths = vec![String::new()];
        visit(root, root, &mut paths);
        paths.sort();
        paths
    }

    let actual = collect_paths(&oc_dst);
    assert_eq!(actual, expected);
}
