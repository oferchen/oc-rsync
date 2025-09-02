// crates/meta/tests/chown_nonroot.rs
use std::fs::{self, Permissions};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::process::Command;

use meta::{Metadata, Options};
use nix::unistd::{setgid, setuid, Gid, Uid};
use tempfile::tempdir;
use users::get_user_by_name;

#[test]
fn chown_permission_denied_matches_rsync() -> std::io::Result<()> {
    let dir = tempdir()?;
    let src = dir.path().join("src");
    let rsync_dest = dir.path().join("rsync");
    let our_dest = dir.path().join("ours");
    fs::create_dir(&src)?;
    fs::create_dir(&rsync_dest)?;
    fs::create_dir(&our_dest)?;
    let perm = Permissions::from_mode(0o777);
    fs::set_permissions(dir.path(), perm.clone())?;
    fs::set_permissions(&src, perm.clone())?;
    fs::set_permissions(&rsync_dest, perm.clone())?;
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

    let status = Command::new("rsync")
        .args([
            "-og",
            src_file.to_str().unwrap(),
            rsync_dest.to_str().unwrap(),
        ])
        .status()
        .expect("rsync not installed");
    assert!(status.success());
    let rsync_meta = fs::symlink_metadata(rsync_dest.join("file"))?;

    let our_file = our_dest.join("file");
    fs::copy(&src_file, &our_file)?;
    let mut opts = Options::default();
    opts.owner = true;
    opts.group = true;
    meta.apply(&our_file, opts)?;
    let our_meta = fs::symlink_metadata(&our_file)?;

    assert_eq!(our_meta.uid(), rsync_meta.uid());
    assert_eq!(our_meta.gid(), rsync_meta.gid());

    Ok(())
}
