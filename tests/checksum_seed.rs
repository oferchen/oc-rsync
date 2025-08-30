// tests/checksum_seed.rs
use checksums::ChecksumConfigBuilder;

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
    let hex0: String = strong0.iter().map(|b| format!("{:02x}", b)).collect();
    let hex1: String = strong1.iter().map(|b| format!("{:02x}", b)).collect();
    assert_eq!(hex0, "be4b47980f89d075f8f7e7a9fab84e29");
    assert_eq!(hex1, "157438ee5881306a9af554cc9b3e5974");
    assert_ne!(hex0, hex1);
}
