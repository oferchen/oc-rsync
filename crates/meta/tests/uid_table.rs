use meta::{parse_chown, parse_id_map, IdKind, UidTable};

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

#[cfg(unix)]
#[test]
fn resolves_user_names_and_maps() {
    use users::{get_current_uid, get_user_by_uid};

    let uid = get_current_uid();
    let name = get_user_by_uid(uid)
        .unwrap()
        .name()
        .to_string_lossy()
        .into_owned();
    let (u, _) = parse_chown(&format!("{name}:")).expect("parse_chown failed for current user");
    assert_eq!(u, Some(uid));

    let mapper =
        parse_id_map(&format!("{name}:{}", uid + 1), IdKind::User).expect("parse_id_map failed");
    assert_eq!(mapper(uid), uid + 1);
}
