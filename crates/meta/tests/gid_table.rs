use meta::GidTable;

#[test]
fn gid_table_deduplicates_and_indexes() {
    let mut table = GidTable::new();
    assert_eq!(table.push(100), 0);
    assert_eq!(table.push(200), 1);

    assert_eq!(table.push(100), 0);
    assert_eq!(table.as_slice(), &[100, 200]);
    assert_eq!(table.gid(0), Some(100));
    assert_eq!(table.gid(1), Some(200));
    assert_eq!(table.gid(2), None);
}
