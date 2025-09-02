// crates/checksums/tests/rsync.rs
use checksums::{strong_digest, StrongHash};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn rsync_checksum(alg: &str, seed: u32, data: &[u8]) -> String {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src");
    let dst = dir.path().join("dst");
    fs::write(&src, data).unwrap();
    let output = Command::new("rsync")
        .arg("-n")
        .arg("--checksum")
        .arg(format!("--checksum-choice={alg}"))
        .arg(format!("--checksum-seed={seed}"))
        .arg("--out-format=%C")
        .arg(&src)
        .arg(&dst)
        .output()
        .expect("failed to run rsync");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .last()
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn parity_with_rsync_md4() {
    let data = b"hello world";
    let seed = 0;
    let rsync = rsync_checksum("md4", seed, data);
    let ours = strong_digest(data, StrongHash::Md4, seed);
    assert_eq!(rsync, hex::encode(ours));
}

#[test]
fn parity_with_rsync_md5() {
    let data = b"hello world";
    let seed = 0;
    let rsync = rsync_checksum("md5", seed, data);
    let ours = strong_digest(data, StrongHash::Md5, seed);
    assert_eq!(rsync, hex::encode(ours));
}

#[test]
fn parity_with_rsync_sha1() {
    let data = b"hello world";
    let seed = 0;
    let rsync = rsync_checksum("sha1", seed, data);
    let ours = strong_digest(data, StrongHash::Sha1, seed);
    assert_eq!(rsync, hex::encode(ours));
}
