// tests/sync_config.rs

use filetime::{set_file_times, FileTime};
use oc_rsync::{synchronize, synchronize_with_config, SyncConfig};
use std::{fs, path::Path};
use tempfile::{tempdir, TempDir};

fn setup_dirs() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    let dst_dir = dir.path().join("dst");
    fs::create_dir_all(&src_dir).unwrap();
    (dir, src_dir, dst_dir)
}

#[test]
fn debug_output_available() {
    let cfg = SyncConfig::builder().quiet(true).build();
    let cfg_dbg = format!("{cfg:?}");
    assert!(cfg_dbg.contains("quiet: true"));

    let builder_dbg = format!("{:?}", SyncConfig::builder());
    assert!(builder_dbg.contains("SyncConfigBuilder"));
}

#[test]
fn sync_local() {
    let (_dir, src_dir, dst_dir) = setup_dirs();
    fs::write(src_dir.join("file.txt"), b"hello world").unwrap();

    assert!(!dst_dir.exists());
    synchronize(src_dir.clone(), dst_dir.clone()).unwrap();
    assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"hello world");
}

#[test]
fn sync_creates_destination() {
    let (_dir, src_dir, dst_dir) = setup_dirs();
    fs::write(src_dir.join("file.txt"), b"data").unwrap();

    let cfg = SyncConfig::builder()
        .atimes(true)
        .perms(true)
        .times(true)
        .specials(true)
        .devices(true)
        .build();

    assert!(!dst_dir.exists());
    synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();
    assert!(dst_dir.exists());
    assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"data");
}

#[cfg(unix)]
#[test]
fn defaults_do_not_preserve_permissions_or_ownership() {
    use nix::unistd::{chown, Gid, Uid};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let (_dir, src_dir, dst_dir) = setup_dirs();
    let file = src_dir.join("file.txt");
    fs::write(&file, b"hello").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
    if Uid::effective().is_root() {
        chown(&file, Some(Uid::from_raw(1000)), Some(Gid::from_raw(1001))).unwrap();
    }

    synchronize(src_dir.clone(), dst_dir.clone()).unwrap();
    let meta = fs::metadata(dst_dir.join("file.txt")).unwrap();
    assert_ne!(meta.permissions().mode() & 0o777, 0o700);
    if Uid::effective().is_root() {
        assert_ne!(meta.uid(), 1000);
        assert_ne!(meta.gid(), 1001);
    }
}

#[cfg(unix)]
#[test]
fn defaults_skip_devices_and_specials() {
    use nix::unistd::{mkfifo, Uid};
    use oc_rsync::meta::{makedev, mknod, Mode, SFlag};
    use std::convert::TryInto;

    let (_dir, src_dir, dst_dir) = setup_dirs();

    if !Uid::effective().is_root() {
        println!("skipping: requires root privileges");
        return;
    }

    let fifo = src_dir.join("fifo");
    mkfifo(&fifo, Mode::from_bits_truncate(0o600)).unwrap();
    let dev = src_dir.join("null");
    #[allow(clippy::useless_conversion)]
    mknod(
        &dev,
        SFlag::S_IFCHR,
        Mode::from_bits_truncate(0o600),
        makedev(1, 3).try_into().unwrap(),
    )
    .unwrap();

    synchronize(src_dir.clone(), dst_dir.clone()).unwrap();

    assert!(!dst_dir.join("fifo").exists());
    assert!(!dst_dir.join("null").exists());
}

#[cfg(any(unix, windows))]
#[test]
fn sync_preserves_symlinks() {
    let (_dir, src_dir, dst_dir) = setup_dirs();
    fs::write(src_dir.join("file.txt"), b"hello").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("file.txt", src_dir.join("link")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file("file.txt", src_dir.join("link")).unwrap();

    assert!(!dst_dir.exists());
    let cfg = SyncConfig::builder().links(true).build();
    synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();

    let meta = fs::symlink_metadata(dst_dir.join("link")).unwrap();
    assert!(meta.file_type().is_symlink());
    let target = fs::read_link(dst_dir.join("link")).unwrap();
    assert_eq!(target, Path::new("file.txt"));
    assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"hello");
}

#[cfg(unix)]
#[test]
fn sync_preserves_metadata() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, src_dir, dst_dir) = setup_dirs();

    let file = src_dir.join("file.txt");
    fs::write(&file, b"hello").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o744)).unwrap();
    let mtime = FileTime::from_unix_time(1_000_100, 0);
    set_file_times(&file, FileTime::from_unix_time(1_000_000, 0), mtime).unwrap();

    assert!(!dst_dir.exists());
    let cfg = SyncConfig::builder().perms(true).times(true).build();
    synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();

    let meta = fs::metadata(dst_dir.join("file.txt")).unwrap();
    assert_eq!(meta.permissions().mode() & 0o777, 0o744);
    let dst_mtime = FileTime::from_last_modification_time(&meta);

    assert_eq!(dst_mtime, mtime);
}

#[cfg(unix)]
#[test]
fn sync_preserves_fifo() {
    use nix::sys::stat::Mode;
    use nix::unistd::mkfifo;
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};

    let (_dir, src_dir, dst_dir) = setup_dirs();
    fs::write(src_dir.join("file.txt"), b"data").unwrap();
    let fifo = src_dir.join("fifo");
    mkfifo(&fifo, Mode::from_bits_truncate(0o640)).unwrap();
    let atime = FileTime::from_unix_time(2_000_000, 0);
    let mtime = FileTime::from_unix_time(2_000_100, 0);
    set_file_times(&fifo, atime, mtime).unwrap();

    let cfg = SyncConfig::builder()
        .atimes(true)
        .perms(true)
        .times(true)
        .specials(true)
        .devices(true)
        .build();

    assert!(!dst_dir.exists());
    synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();
    assert!(dst_dir.exists());

    let dst_path = dst_dir.join("fifo");
    let meta = fs::metadata(&dst_path).unwrap();
    assert!(meta.file_type().is_fifo());
    assert_eq!(meta.permissions().mode() & 0o777, 0o640);
    let dst_atime = FileTime::from_last_access_time(&meta);
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_eq!(dst_atime, atime);
    assert_eq!(dst_mtime, mtime);
}

#[cfg(unix)]
#[test]
fn sync_preserves_directory_metadata() {
    use std::os::unix::fs::PermissionsExt;

    let (_dir, src_dir, dst_dir) = setup_dirs();

    let subdir = src_dir.join("sub");
    fs::create_dir(&subdir).unwrap();
    fs::set_permissions(&subdir, fs::Permissions::from_mode(0o711)).unwrap();
    fs::write(subdir.join("file.txt"), b"data").unwrap();
    let mtime = FileTime::from_unix_time(3_000_100, 0);
    set_file_times(&subdir, FileTime::from_unix_time(3_000_000, 0), mtime).unwrap();

    let cfg = SyncConfig::builder().perms(true).times(true).build();
    synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();

    let meta = fs::metadata(dst_dir.join("sub")).unwrap();
    assert!(meta.is_dir());
    assert_eq!(meta.permissions().mode() & 0o777, 0o711);
    let dst_mtime = FileTime::from_last_modification_time(&meta);
    assert_eq!(dst_mtime, mtime);
}

#[cfg(unix)]
#[test]
fn sync_preserves_ownership() {
    use nix::unistd::{chown, Gid, Uid};
    use std::os::unix::fs::MetadataExt;

    if !Uid::effective().is_root() {
        println!("skipping sync_preserves_ownership: requires root privileges to change ownership");
        return;
    }

    let (_dir, src_dir, dst_dir) = setup_dirs();

    let file = src_dir.join("file.txt");
    fs::write(&file, b"hello").unwrap();
    chown(&file, Some(Uid::from_raw(1000)), Some(Gid::from_raw(1001))).unwrap();

    let cfg = SyncConfig::builder().owner(true).group(true).build();
    synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();

    let meta = fs::metadata(dst_dir.join("file.txt")).unwrap();
    assert_eq!(meta.uid(), 1000);
    assert_eq!(meta.gid(), 1001);
}

#[cfg(all(unix, feature = "xattr"))]
#[test]
fn sync_preserves_xattrs() {
    let (_dir, src_dir, dst_dir) = setup_dirs();

    let file = src_dir.join("file.txt");
    fs::write(&file, b"hello").unwrap();
    xattr::set(&file, "user.test", b"val").unwrap();

    let cfg = SyncConfig::builder().xattrs(true).build();
    synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();

    let val = xattr::get(dst_dir.join("file.txt"), "user.test")
        .unwrap()
        .unwrap();
    assert_eq!(&*val, b"val");
}

#[test]
fn custom_log_file_and_quiet_settings() {
    use std::fs;

    let (dir, src_dir, dst_dir1) = setup_dirs();
    fs::write(src_dir.join("file.txt"), b"hi").unwrap();

    let log1 = dir.path().join("log1.txt");
    let cfg = SyncConfig::builder()
        .log_file(Some((log1.clone(), None)))
        .verbose(1)
        .timestamps(true)
        .build();
    synchronize_with_config(src_dir.clone(), dst_dir1, &cfg).unwrap();
    let contents = fs::read_to_string(&log1).unwrap();
    assert!(!contents.is_empty());

    let dst_dir2 = dir.path().join("dst2");
    let log2 = dir.path().join("log2.txt");
    let quiet_cfg = SyncConfig::builder()
        .log_file(Some((log2.clone(), None)))
        .verbose(1)
        .quiet(true)
        .build();
    synchronize_with_config(src_dir, dst_dir2, &quiet_cfg).unwrap();
    let contents2 = fs::read_to_string(&log2).unwrap();
    assert!(contents2.is_empty());
}

#[cfg(unix)]
#[test]
fn custom_syslog_and_journald_settings() {
    use std::fs;
    use std::os::unix::net::UnixDatagram;

    let (dir, src_dir, dst_dir) = setup_dirs();
    fs::write(src_dir.join("file.txt"), b"hi").unwrap();

    let syslog_path = dir.path().join("syslog.sock");
    let syslog_server = UnixDatagram::bind(&syslog_path).unwrap();
    unsafe { std::env::set_var("OC_RSYNC_SYSLOG_PATH", &syslog_path) };

    let journald_path = dir.path().join("journald.sock");
    let journald_server = UnixDatagram::bind(&journald_path).unwrap();
    unsafe { std::env::set_var("OC_RSYNC_JOURNALD_PATH", &journald_path) };

    let cfg = SyncConfig::builder()
        .verbose(1)
        .syslog(true)
        .journald(true)
        .build();
    synchronize_with_config(src_dir, dst_dir, &cfg).unwrap();

    let mut buf = [0u8; 256];
    let (n, _) = syslog_server.recv_from(&mut buf).unwrap();
    let sys_msg = std::str::from_utf8(&buf[..n]).unwrap();
    assert!(sys_msg.contains("rsync"));

    let (n, _) = journald_server.recv_from(&mut buf).unwrap();
    let jour_msg = std::str::from_utf8(&buf[..n]).unwrap();
    assert!(jour_msg.contains("MESSAGE"));

    unsafe { std::env::remove_var("OC_RSYNC_SYSLOG_PATH") };
    unsafe { std::env::remove_var("OC_RSYNC_JOURNALD_PATH") };
}
