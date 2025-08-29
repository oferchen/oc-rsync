#[cfg(unix)]
use assert_cmd::cargo::cargo_bin;
#[cfg(unix)]
use assert_cmd::Command as AssertCommand;
#[cfg(unix)]
use cli::parse_rsh;
#[cfg(unix)]
use compress::available_codecs;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::process::Command;
use tempfile::tempdir;
#[cfg(unix)]
mod remote_utils;
#[cfg(unix)]
use remote_utils::{spawn_reader, spawn_writer};
#[cfg(unix)]
use transport::ssh::SshStdioTransport;

#[cfg(unix)]
#[test]
fn rsh_remote_pair_syncs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"via rsh").unwrap();

    let src_session = spawn_reader(&format!("cat {}", src.display()));
    let dst_session = spawn_writer(&format!("cat > {}", dst.display()));
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out = fs::read(&dst).unwrap();
    assert_eq!(out, b"via rsh");
}

#[cfg(unix)]
#[test]
fn custom_rsh_matches_stock_rsync() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    fs::write(&src, b"hello shell").unwrap();

    let dst_rr = dir.path().join("dst_rr.txt");
    let dst_rsync = dir.path().join("dst_rsync.txt");

    // Create a fake remote shell that ignores the host argument and executes the rest.
    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    // Use the custom shell with our transport to copy data
    let src_session = SshStdioTransport::spawn(
        rsh.to_str().unwrap(),
        ["ignored", "cat", src.to_str().unwrap()],
    )
    .unwrap();
    let dst_session = SshStdioTransport::spawn(
        rsh.to_str().unwrap(),
        [
            "ignored",
            "sh",
            "-c",
            &format!("cat > {}", dst_rr.display()),
        ],
    )
    .unwrap();
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Use stock rsync with the same remote shell
    let dst_rsync_spec = format!("ignored:{}", dst_rsync.display());
    let output = Command::new("rsync")
        .args([
            "-e",
            rsh.to_str().unwrap(),
            src.to_str().unwrap(),
            &dst_rsync_spec,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let ours = fs::read(&dst_rr).unwrap();
    let theirs = fs::read(&dst_rsync).unwrap();
    assert_eq!(ours, theirs);
}

#[cfg(unix)]
#[test]
fn custom_rsh_negotiates_codecs() {
    let dir = tempdir().unwrap();
    let remote_bin = dir.path().join("rr-remote");
    fs::copy(cargo_bin("rsync-rs"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let rsh_cmd = vec![rsh.to_str().unwrap().to_string()];
    let rsh_env: Vec<(String, String)> = Vec::new();
    let rsync_env: Vec<(String, String)> = std::env::vars()
        .filter(|(k, _)| k.starts_with("RSYNC_"))
        .collect();
    let (session, codecs) = SshStdioTransport::connect_with_rsh(
        "ignored",
        Path::new("."),
        &rsh_cmd,
        &rsh_env,
        &rsync_env,
        Some(&remote_bin),
        None,
        true,
        None,
    )
    .unwrap();
    drop(session);
    assert_eq!(codecs, available_codecs());
}

#[cfg(unix)]
#[test]
fn rsh_parses_multi_argument_commands() {
    let dir = tempdir().unwrap();
    let script = dir.path().join("log_args.sh");
    fs::write(
        &script,
        b"#!/bin/sh\nout=\"$1\"\nshift\nprintf '%s ' \"$@\" > \"$out\"\n",
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let log = dir.path().join("log.txt");
    let cmd = format!(
        "{} {} -p 2222 -o \"foo bar\"",
        script.display(),
        log.display()
    );
    let rsh = parse_rsh(Some(cmd)).unwrap();
    let mut c = Command::new(&rsh.cmd[0]);
    c.args(&rsh.cmd[1..]);
    c.envs(rsh.env.clone());
    c.status().unwrap();
    let content = fs::read_to_string(&log).unwrap();
    assert!(content.contains("-p 2222"));
    assert!(content.contains("-o foo bar"));
}

#[cfg(unix)]
#[test]
fn rsh_environment_variables_are_propagated() {
    let dir = tempdir().unwrap();
    let out = dir.path().join("env.txt");
    let cmd = format!("FOO=bar sh -c 'echo \"$FOO\" > {}'", out.display());
    let rsh = parse_rsh(Some(cmd)).unwrap();
    let mut c = Command::new(&rsh.cmd[0]);
    c.args(&rsh.cmd[1..]);
    c.envs(rsh.env.clone());
    c.status().unwrap();
    let content = fs::read_to_string(&out).unwrap();
    assert_eq!(content.trim(), "bar");
}

#[cfg(unix)]
#[test]
fn rsync_path_respects_rsh_env_var() {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    let src_file = src_dir.join("file.txt");
    fs::write(&src_file, b"from env var").unwrap();
    let dst_dir = dir.path().join("dst");

    // Copy the client binary to act as the remote rsync executable.
    let remote_bin = dir.path().join("rr-remote");
    fs::copy(cargo_bin("rsync-rs"), &remote_bin).unwrap();
    fs::set_permissions(&remote_bin, fs::Permissions::from_mode(0o755)).unwrap();

    // Fake remote shell that ignores host argument.
    let rsh = dir.path().join("fake_rsh.sh");
    fs::write(&rsh, b"#!/bin/sh\nshift\nexec \"$@\"\n").unwrap();
    fs::set_permissions(&rsh, fs::Permissions::from_mode(0o755)).unwrap();

    let src_spec = format!("{}/", src_dir.display());
    let dst_spec = format!("ignored:{}", dst_dir.display());
    let mut cmd = AssertCommand::cargo_bin("rsync-rs").unwrap();
    cmd.env("RSYNC_RSH", rsh.to_str().unwrap());
    cmd.args([
        "--rsync-path",
        remote_bin.to_str().unwrap(),
        "-r",
        &src_spec,
        &dst_spec,
    ]);
    cmd.assert().success();

    let out = fs::read(dst_dir.join("file.txt")).unwrap();
    assert_eq!(out, b"from env var");
}
