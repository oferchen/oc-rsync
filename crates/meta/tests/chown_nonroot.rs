// crates/meta/tests/chown_nonroot.rs
use std::fs::{self, Permissions};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use meta::{Metadata, Options};
use nix::unistd::{setgid, setuid, Gid, Uid};
use tempfile::tempdir;
use users::get_user_by_name;

#[test]
fn chown_permission_denied() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let our_dest = dir.path().join("ours");
    fs::create_dir(&src)?;
    fs::create_dir(&our_dest)?;
    let perm = Permissions::from_mode(0o777);
    fs::set_permissions(dir.path(), perm.clone())?;
    fs::set_permissions(&src, perm.clone())?;
    fs::set_permissions(&our_dest, perm)?;

    let src_file = src.join("file");
    fs::write(&src_file, b"data")?;
    let meta = Metadata::from_path(&src_file, Options::default())?;

    if Uid::effective().is_root() {
        if let Some(nobody) = get_user_by_name("nobody") {
            setgid(Gid::from_raw(nobody.primary_group_id()))?;
            setuid(Uid::from_raw(nobody.uid()))?;
        }
    }

    let expected_uid = Uid::effective().as_raw();
    let expected_gid = Gid::effective().as_raw();

    let our_file = our_dest.join("file");
    fs::copy(&src_file, &our_file)?;
    let opts = Options {
        owner: true,
        group: true,
        ..Default::default()
    };
    meta.apply(&our_file, opts)?;
    let our_meta = fs::symlink_metadata(&our_file)?;

    assert_eq!(our_meta.uid(), expected_uid);
    assert_eq!(our_meta.gid(), expected_gid);

    Ok(())
}
