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
    let cfg_md5 = ChecksumConfigBuilder::new().strong(StrongHash::Md5).build();
    let cfg_sha1 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Sha1)
        .build();
    let cfg_md4 = ChecksumConfigBuilder::new().strong(StrongHash::Md4).build();
    let cfg_blake2b = ChecksumConfigBuilder::new()
        .strong(StrongHash::Blake2b)
        .build();
    let cfg_blake2s = ChecksumConfigBuilder::new()
        .strong(StrongHash::Blake2s)
        .build();
    let cfg_xxh64 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Xxh64)
        .build();
    let cfg_xxh128 = ChecksumConfigBuilder::new()
        .strong(StrongHash::Xxh128)
        .build();
    let data = b"hello world";

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

    let cs_md4 = cfg_md4.checksum(data);
    assert_eq!(cs_md4.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_md4.strong),
        "ea91f391e02b5e19f432b43bd87a531d"
    );

    let cs_blake2b = cfg_blake2b.checksum(data);
    assert_eq!(cs_blake2b.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_blake2b.strong),
        "d32b7e7c9028b6e0b1ddd7e83799a8b857a0afcaa370985dfaa42dfa59e275097eb75b99e05bb7ef3ac5cf74c957c3b7cad1dfcbb5e3380d56b63780394af8bd",
    );

    let cs_blake2s = cfg_blake2s.checksum(data);
    assert_eq!(cs_blake2s.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_blake2s.strong),
        "a2dc531d6048af9ab7cf85108ebcf147632fce6290fbdfcd5ea789a0b31784d0",
    );

    let cs_xxh64 = cfg_xxh64.checksum(data);
    assert_eq!(cs_xxh64.weak, rolling_checksum(data));
    assert_eq!(hex::encode(cs_xxh64.strong), "648e94e9d09503e7");

    let cs_xxh128 = cfg_xxh128.checksum(data);
    assert_eq!(cs_xxh128.weak, rolling_checksum(data));
    assert_eq!(
        hex::encode(cs_xxh128.strong),
        "052acb3009ceb7609305f939f85080da",
    );

    #[cfg(feature = "blake3")]
    {
        let cfg_blake3 = ChecksumConfigBuilder::new()
            .strong(StrongHash::Blake3)
            .build();
        let cs_blake3 = cfg_blake3.checksum(data);
        assert_eq!(cs_blake3.weak, rolling_checksum(data));
        assert_eq!(
            hex::encode(cs_blake3.strong),
            "861487254e43e2e567ef5177d0c85452f1982ec89c494e8d4a957ff01dd9b421",
        );
    }
}
