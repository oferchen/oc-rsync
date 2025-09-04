// crates/checksums/tests/rsync.rs
use checksums::{strong_digest, StrongHash};
use std::{fs, path::Path};

fn golden(alg: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/checksums_seed1_hello_world.txt");
    for line in fs::read_to_string(path).unwrap().lines() {
        let mut parts = line.split_whitespace();
        let a = parts.next().unwrap();
        let digest = parts.next().unwrap();
        if a == alg {
            return digest.to_string();
        }
    }
    panic!("missing golden for {alg}");
}

#[test]
fn parity_with_rsync_md4() {
    let data = b"hello world";
    let seed = 1;
    let ours = strong_digest(data, StrongHash::Md4, seed);
    assert_eq!(golden("md4"), hex::encode(ours));
}

#[test]
fn parity_with_rsync_md5() {
    let data = b"hello world";
    let seed = 1;
    let ours = strong_digest(data, StrongHash::Md5, seed);
    assert_eq!(golden("md5"), hex::encode(ours));
}

#[test]
fn parity_with_rsync_sha1() {
    let data = b"hello world";
    let seed = 1;
    let ours = strong_digest(data, StrongHash::Sha1, seed);
    assert_eq!(golden("sha1"), hex::encode(ours));
}

#[test]
fn parity_with_rsync_xxh64() {
    let data = b"hello world";
    let seed = 1;
    let ours = strong_digest(data, StrongHash::Xxh64, seed);
    assert_eq!(golden("xxh64"), hex::encode(ours));
}

#[test]
fn parity_with_rsync_xxh3() {
    let data = b"hello world";
    let seed = 1;
    let ours = strong_digest(data, StrongHash::Xxh3, seed);
    assert_eq!(golden("xxh3"), hex::encode(ours));
}

#[test]
fn parity_with_rsync_xxh128() {
    let data = b"hello world";
    let seed = 1;
    let ours = strong_digest(data, StrongHash::Xxh128, seed);
    assert_eq!(golden("xxh128"), hex::encode(ours));
}
