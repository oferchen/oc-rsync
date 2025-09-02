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
    assert_eq!(hex0, "ea91f391e02b5e19f432b43bd87a531d");
    assert_eq!(hex1, "92e5994e0babddace03f0ff88f767181");
    assert_ne!(hex0, hex1);
}
