// crates/checksums/tests/md4.rs
use checksums::{strong_digest, StrongHash};

#[test]
fn md4_seeded_digest() {
    let digest = strong_digest(b"hello world", StrongHash::Md4, 1);
    assert_eq!(hex::encode(digest), "92e5994e0babddace03f0ff88f767181");
}
