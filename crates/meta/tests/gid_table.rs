use meta::{parse_chown, parse_id_map, GidTable, IdKind};

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

#[cfg(unix)]
#[test]
fn resolves_group_names_and_maps() {
    use users::{get_current_gid, get_group_by_gid};

    let gid = get_current_gid();
    let name = get_group_by_gid(gid)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let (_, g) = parse_chown(&format!(":{name}")).expect("parse_chown failed for current group");
    assert_eq!(g, Some(gid));

    let mapper =
        parse_id_map(&format!("{name}:{}", gid + 1), IdKind::Group).expect("parse_id_map failed");
    assert_eq!(mapper(gid), gid + 1);
}
