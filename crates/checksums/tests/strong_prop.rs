// crates/checksums/tests/strong_prop.rs
use checksums::{ChecksumConfigBuilder, StrongHash};
use proptest::prelude::*;

const MD4_VECTORS: &[(&[u8], &str)] = &[
    (b"", "1b06b0037d44bcc91f0b2653e4e5ccd5"),
    (b"hello world", "7ced6b52c8203ba97580659d7dc33548"),
    (
        b"The quick brown fox jumps over the lazy dog",
        "1ddc545342d9bbb4320b2187c40b56d0",
    ),
];

const MD5_VECTORS: &[(&[u8], &str)] = &[
    (b"", "f1d3ff8443297732862df21dc4e57262"),
    (b"hello world", "be4b47980f89d075f8f7e7a9fab84e29"),
    (
        b"The quick brown fox jumps over the lazy dog",
        "579b84733e9de9dcc890a38e381966cd",
    ),
];

const SHA1_VECTORS: &[(&[u8], &str)] = &[
    (b"", "9069ca78e7450a285173431b3e52c5c25299e473"),
    (b"hello world", "1fb6475c524899f98b088f7608bdab8f1591e078"),
    (
        b"The quick brown fox jumps over the lazy dog",
        "c7f069eef2521b87552d9964cc59c3547df7bae3",
    ),
];

const XXHASH_VECTORS: &[(&[u8], &str)] = &[
    (b"", "99e9d85137db46ef"),
    (b"hello world", "68691eb23467ab45"),
    (
        b"The quick brown fox jumps over the lazy dog",
        "bc71da1f362d240b",
    ),
];

fn vectors_for(alg: StrongHash) -> &'static [(&'static [u8], &'static str)] {
    match alg {
        StrongHash::Md4 => MD4_VECTORS,
        StrongHash::Md5 => MD5_VECTORS,
        StrongHash::Sha1 => SHA1_VECTORS,
        StrongHash::XxHash => XXHASH_VECTORS,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]
    #[test]
    fn strong_hasher_matches_known_vectors(alg in prop_oneof![
        Just(StrongHash::Md4),
        Just(StrongHash::Md5),
        Just(StrongHash::Sha1),
        Just(StrongHash::XxHash),
    ], idx in 0usize..3) {
        let vectors = vectors_for(alg);
        let (data, expected) = vectors[idx];
        let mut hasher = ChecksumConfigBuilder::new().strong(alg).build().strong_hasher();
        hasher.update(data);
        let digest = hex::encode(hasher.finalize());
        prop_assert_eq!(digest, expected);
    }
}
