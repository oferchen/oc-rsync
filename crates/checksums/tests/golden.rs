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
    let cfg_xxh64 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Xxh64)
        .build();
    let cfg_xxh3 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Xxh3)
        .build();
    let cfg_xxh128 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Xxh128)
        .build();
    let data = b"hello world";

    let cs_md4 = cfg_md4.checksum(data);
    assert_eq!(cs_md4.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_md4.strong),
        "ea91f391e02b5e19f432b43bd87a531d"
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

    let cs_xxh64 = cfg_xxh64.checksum(data);
    assert_eq!(cs_xxh64.weak, rolling_checksum(data));
    assert_eq!(hex::encode(&cs_xxh64.strong), "68691eb23467ab45");
    let mut be64 = cs_xxh64.strong.clone();
    be64.reverse();
    assert_eq!(hex::encode(be64), "45ab6734b21e6968");

    let cs_xxh3 = cfg_xxh3.checksum(data);
    assert_eq!(cs_xxh3.weak, rolling_checksum(data));
    assert_eq!(hex::encode(&cs_xxh3.strong), "8b98e640eab147d4");
    let mut be3 = cs_xxh3.strong.clone();
    be3.reverse();
    assert_eq!(hex::encode(be3), "d447b1ea40e6988b");

    let cs_xxh128 = cfg_xxh128.checksum(data);
    assert_eq!(cs_xxh128.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(&cs_xxh128.strong),
        "c7b615cc75879ba90049873fe9098ddf",
    );
    let mut be128 = cs_xxh128.strong.clone();
    be128.reverse();
    assert_eq!(hex::encode(be128), "df8d09e93f874900a99b8775cc15b6c7",);
}
