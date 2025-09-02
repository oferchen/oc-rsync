// crates/checksums/tests/golden.rs
use checksums::{rolling_checksum, ChecksumConfigBuilder, Rolling, StrongHash};

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
    let cfg_md4 = ChecksumConfigBuilder::new().build();
    let cfg_md5 = ChecksumConfigBuilder::new().strong(StrongHash::Md5).build();
    let cfg_sha1 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Sha1)
        .build();
    let data = b"hello world";

    let cs_md4 = cfg_md4.checksum(data);
    assert_eq!(cs_md4.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_md4.strong),
        "aa010fbc1d14c795d86ef98c95479d17"
    );

    let cs_md5 = cfg_md5.checksum(data);
    assert_eq!(cs_md5.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_md5.strong),
        "5eb63bbbe01eeed093cb22bb8f5acdc3"
    );

    let cs_sha1 = cfg_sha1.checksum(data);
    assert_eq!(cs_sha1.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_sha1.strong),
        "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed",
    );
}
