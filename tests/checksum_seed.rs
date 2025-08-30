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
