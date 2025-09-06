// tests/checksum_seed.rs
use checksums::{ChecksumConfigBuilder, StrongHash};

#[test]
fn checksum_seed_changes_weak_checksum() {
    let data = b"hello world";
    let cfg0 = ChecksumConfigBuilder::new().seed(0).build();
    let cfg1 = ChecksumConfigBuilder::new().seed(1).build();
    let sum0 = cfg0.checksum(data).weak;
    let sum1 = cfg1.checksum(data).weak;
    assert_eq!(sum0, 436208732);
    assert_eq!(sum1, 436929629);
    assert_ne!(sum0, sum1);
}

#[test]
fn checksum_seed_changes_strong_checksum() {
    let data = b"hello world";
    let cfg0 = ChecksumConfigBuilder::new().seed(0).build();
    let cfg1 = ChecksumConfigBuilder::new().seed(1).build();
    let strong0 = cfg0.checksum(data).strong;
    let strong1 = cfg1.checksum(data).strong;
    let hex0 = hex::encode(&strong0);
    let hex1 = hex::encode(&strong1);
    assert_eq!(hex0, "be4b47980f89d075f8f7e7a9fab84e29");
    assert_eq!(hex1, "157438ee5881306a9af554cc9b3e5974");
    assert_ne!(hex0, hex1);
}

#[test]
fn checksum_seed_changes_strong_checksum_md4() {
    let data = b"hello world";
    let cfg0 = ChecksumConfigBuilder::new()
        .seed(0)
        .strong(StrongHash::Md4)
        .build();
    let cfg1 = ChecksumConfigBuilder::new()
        .seed(1)
        .strong(StrongHash::Md4)
        .build();
    let strong0 = cfg0.checksum(data).strong;
    let strong1 = cfg1.checksum(data).strong;
    let hex0 = hex::encode(&strong0);
    let hex1 = hex::encode(&strong1);
    assert_eq!(hex0, "7ced6b52c8203ba97580659d7dc33548");
    assert_eq!(hex1, "681d333539cc115fe7b2f40bb5aa8b89");
    assert_ne!(hex0, hex1);
}

#[test]
fn checksum_seed_changes_strong_checksum_sha1() {
    let data = b"hello world";
    let cfg0 = ChecksumConfigBuilder::new()
        .seed(0)
        .strong(StrongHash::Sha1)
        .build();
    let cfg1 = ChecksumConfigBuilder::new()
        .seed(1)
        .strong(StrongHash::Sha1)
        .build();
    let strong0 = cfg0.checksum(data).strong;
    let strong1 = cfg1.checksum(data).strong;
    let hex0 = hex::encode(&strong0);
    let hex1 = hex::encode(&strong1);
    assert_eq!(hex0, "1fb6475c524899f98b088f7608bdab8f1591e078");
    assert_eq!(hex1, "076b085b6d84fa708d235291ae6ac3059b45bb37");
    assert_ne!(hex0, hex1);
}
