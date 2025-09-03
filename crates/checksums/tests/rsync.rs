// crates/checksums/tests/rsync.rs
use checksums::{strong_digest, StrongHash};
use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn rsync_supports_checksum_seed() -> bool {
    let probe = b"seed-probe";
    let a = rsync_checksum("md5", 0, probe);
    let b = rsync_checksum("md5", 1, probe);
    a != b
}

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
    if !rsync_supports_checksum_seed() {
        eprintln!("skipping: rsync lacks --checksum-seed");
        return;
    }
    let data = b"hello world";
    let seed = 1;
    let rsync = rsync_checksum("md4", seed, data);
    let ours = strong_digest(data, StrongHash::Md4, seed);
    assert_eq!(rsync, hex::encode(ours));
}

#[test]
fn parity_with_rsync_md5() {
    if !rsync_supports_checksum_seed() {
        eprintln!("skipping: rsync lacks --checksum-seed");
        return;
    }
    let data = b"hello world";
    let seed = 1;
    let rsync = rsync_checksum("md5", seed, data);
    let ours = strong_digest(data, StrongHash::Md5, seed);
    assert_eq!(rsync, hex::encode(ours));
}

#[test]
fn parity_with_rsync_sha1() {
    if !rsync_supports_checksum_seed() {
        eprintln!("skipping: rsync lacks --checksum-seed");
        return;
    }
    let data = b"hello world";
    let seed = 1;
    let rsync = rsync_checksum("sha1", seed, data);
    let ours = strong_digest(data, StrongHash::Sha1, seed);
    assert_eq!(rsync, hex::encode(ours));
}

#[test]
fn parity_with_rsync_xxh64() {
    if !rsync_supports_checksum_seed() {
        eprintln!("skipping: rsync lacks --checksum-seed");
        return;
    }
    let data = b"hello world";
    let seed = 1;
    let rsync = rsync_checksum("xxh64", seed, data);
    let ours = strong_digest(data, StrongHash::Xxh64, seed);
    assert_eq!(rsync, hex::encode(ours));
}

#[test]
fn parity_with_rsync_xxh3() {
    if !rsync_supports_checksum_seed() {
        eprintln!("skipping: rsync lacks --checksum-seed");
        return;
    }
    let data = b"hello world";
    let seed = 1;
    let rsync = rsync_checksum("xxh3", seed, data);
    let ours = strong_digest(data, StrongHash::Xxh3, seed);
    assert_eq!(rsync, hex::encode(ours));
}

#[test]
fn parity_with_rsync_xxh128() {
    if !rsync_supports_checksum_seed() {
        eprintln!("skipping: rsync lacks --checksum-seed");
        return;
    }
    let data = b"hello world";
    let seed = 1;
    let rsync = rsync_checksum("xxh128", seed, data);
    let ours = strong_digest(data, StrongHash::Xxh128, seed);
    assert_eq!(rsync, hex::encode(ours));
}
