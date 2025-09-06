// crates/checksums/tests/golden.rs
use checksums::{ChecksumConfigBuilder, Rolling, StrongHash, rolling_checksum, strong_digest};

#[test]
fn rolling_golden_windows() {
    let data = b"0123456789abcdef";
    let win = 8;
    let expected = [
        118751644u32,
        121110948,
        123470252,
        128385499,
        135856650,
        145883705,
        158466664,
        173605527,
        191300294,
    ];

    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(rolling_checksum(&data[i..i + win]), exp);
    }

    let mut r = Rolling::new(&data[0..win]);
    for (i, &exp) in expected.iter().enumerate() {
        assert_eq!(r.digest(), exp);
        if i + win < data.len() {
            r.roll(data[i], data[i + win]);
        }
    }
}

#[test]
fn builder_strong_digests() {
    let cfg_default = ChecksumConfigBuilder::new().build();
    let cfg_md5 = ChecksumConfigBuilder::new().strong(StrongHash::Md5).build();
    let cfg_sha1 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Sha1)
        .build();
    let cfg_xxh = ChecksumConfigBuilder::new()
        .strong(StrongHash::XxHash)
        .build();
    let data = b"hello world";

    let cs_default = cfg_default.checksum(data);
    assert_eq!(cs_default.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_default.strong),
        "7ced6b52c8203ba97580659d7dc33548"
    );

    let cs_md5 = cfg_md5.checksum(data);
    assert_eq!(cs_md5.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_md5.strong),
        "be4b47980f89d075f8f7e7a9fab84e29"
    );

    let cs_sha1 = cfg_sha1.checksum(data);
    assert_eq!(cs_sha1.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_sha1.strong),
        "1fb6475c524899f98b088f7608bdab8f1591e078",
    );

    let cs_xxh = cfg_xxh.checksum(data);
    assert_eq!(cs_xxh.weak, rolling_checksum(data));
    assert_eq!(hex::encode(cs_xxh.strong), "68691eb23467ab45");
}

#[test]
fn negotiation_picks_common_algorithm() {
    let remote = vec![StrongHash::Md4, StrongHash::XxHash];
    let cfg = ChecksumConfigBuilder::new().negotiate(&remote).build();
    let ours = cfg.checksum(b"test");
    let expected = strong_digest(b"test", StrongHash::XxHash, 0);
    assert_eq!(ours.strong, expected);
}

#[test]
fn negotiation_defaults_to_md4_when_no_common() {
    let remote: Vec<StrongHash> = Vec::new();
    let cfg = ChecksumConfigBuilder::new().negotiate(&remote).build();
    let ours = cfg.checksum(b"test");
    let expected = strong_digest(b"test", StrongHash::Md4, 0);
    assert_eq!(ours.strong, expected);
}
