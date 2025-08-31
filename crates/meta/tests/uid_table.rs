use meta::UidTable;

#[test]
fn uid_table_deduplicates_and_indexes() {
    let mut table = UidTable::new();
    assert_eq!(table.push(1000), 0);
    assert_eq!(table.push(2000), 1);

    assert_eq!(table.push(1000), 0);
    assert_eq!(table.as_slice(), &[1000, 2000]);
    assert_eq!(table.uid(0), Some(1000));
    assert_eq!(table.uid(1), Some(2000));
    assert_eq!(table.uid(2), None);
}
