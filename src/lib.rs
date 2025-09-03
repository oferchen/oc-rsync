// src/lib.rs
use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use logging::{subscriber, DebugFlag, InfoFlag, LogFormat, SubscriberConfig};
use std::path::{Path, PathBuf};
use tracing::subscriber::with_default;

#[derive(Clone)]
pub struct SyncConfig {
    pub log_format: LogFormat,
    pub verbose: u8,
    pub info: Vec<InfoFlag>,
    pub debug: Vec<DebugFlag>,
    pub quiet: bool,
    pub log_file: Option<(PathBuf, Option<String>)>,
    pub syslog: bool,
    pub journald: bool,
    pub colored: bool,
    pub timestamps: bool,
    pub perms: bool,
    pub times: bool,
    pub atimes: bool,
    pub links: bool,
    pub devices: bool,
    pub specials: bool,
    pub owner: bool,
    pub group: bool,
    #[cfg(feature = "xattr")]
    pub xattrs: bool,
    #[cfg(feature = "acl")]
    pub acls: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Text,
            verbose: 0,
            info: Vec::new(),
            debug: Vec::new(),
            quiet: false,
            log_file: None,
            syslog: false,
            journald: false,
            colored: true,
            timestamps: false,
            perms: true,
            times: true,
            atimes: false,
            links: true,
            devices: true,
            specials: true,
            owner: true,
            group: true,
            #[cfg(feature = "xattr")]
            xattrs: false,
            #[cfg(feature = "acl")]
            acls: false,
        }
    }
}

impl SyncConfig {
    pub fn builder() -> SyncConfigBuilder {
        SyncConfigBuilder::default()
    }
}

#[derive(Default)]
#[must_use]
pub struct SyncConfigBuilder {
    cfg: SyncConfig,
}

impl SyncConfigBuilder {
    pub fn log_format(mut self, log_format: LogFormat) -> Self {
        self.cfg.log_format = log_format;
        self
    }

    pub fn verbose(mut self, verbose: u8) -> Self {
        self.cfg.verbose = verbose;
        self
    }

    pub fn info(mut self, info: Vec<InfoFlag>) -> Self {
        self.cfg.info = info;
        self
    }

    pub fn debug(mut self, debug: Vec<DebugFlag>) -> Self {
        self.cfg.debug = debug;
        self
    }

    pub fn quiet(mut self, quiet: bool) -> Self {
        self.cfg.quiet = quiet;
        self
    }

    pub fn log_file(mut self, log_file: Option<(PathBuf, Option<String>)>) -> Self {
        self.cfg.log_file = log_file;
        self
    }

    pub fn syslog(mut self, enable: bool) -> Self {
        self.cfg.syslog = enable;
        self
    }

    pub fn journald(mut self, enable: bool) -> Self {
        self.cfg.journald = enable;
        self
    }

    pub fn colored(mut self, enable: bool) -> Self {
        self.cfg.colored = enable;
        self
    }

    pub fn timestamps(mut self, enable: bool) -> Self {
        self.cfg.timestamps = enable;
        self
    }

    pub fn perms(mut self, enable: bool) -> Self {
        self.cfg.perms = enable;
        self
    }

    pub fn times(mut self, enable: bool) -> Self {
        self.cfg.times = enable;
        self
    }

    pub fn atimes(mut self, enable: bool) -> Self {
        self.cfg.atimes = enable;
        self
    }

    pub fn links(mut self, enable: bool) -> Self {
        self.cfg.links = enable;
        self
    }

    pub fn devices(mut self, enable: bool) -> Self {
        self.cfg.devices = enable;
        self
    }

    pub fn specials(mut self, enable: bool) -> Self {
        self.cfg.specials = enable;
        self
    }

    pub fn owner(mut self, enable: bool) -> Self {
        self.cfg.owner = enable;
        self
    }

    pub fn group(mut self, enable: bool) -> Self {
        self.cfg.group = enable;
        self
    }

    #[cfg(feature = "xattr")]
    pub fn xattrs(mut self, enable: bool) -> Self {
        self.cfg.xattrs = enable;
        self
    }

    #[cfg(feature = "acl")]
    pub fn acls(mut self, enable: bool) -> Self {
        self.cfg.acls = enable;
        self
    }

    pub fn build(self) -> SyncConfig {
        self.cfg
    }
}

pub fn synchronize_with_config<P: AsRef<Path>>(src: P, dst: P, cfg: &SyncConfig) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let sub_cfg = SubscriberConfig::builder()
        .format(cfg.log_format)
        .verbose(cfg.verbose)
        .info(cfg.info.clone())
        .debug(cfg.debug.clone())
        .quiet(cfg.quiet)
        .log_file(cfg.log_file.clone())
        .syslog(cfg.syslog)
        .journald(cfg.journald)
        .colored(cfg.colored)
        .timestamps(cfg.timestamps)
        .build();
    let sub = subscriber(sub_cfg);
    with_default(sub, || -> Result<()> {
        let opts = SyncOptions {
            perms: cfg.perms,
            times: cfg.times,
            atimes: cfg.atimes,
            links: cfg.links,
            devices: cfg.devices,
            specials: cfg.specials,
            owner: cfg.owner,
            group: cfg.group,
            quiet: cfg.quiet,
            #[cfg(feature = "xattr")]
            xattrs: cfg.xattrs,
            #[cfg(feature = "acl")]
            acls: cfg.acls,
            ..SyncOptions::default()
        };
        engine::sync(src, dst, &Matcher::default(), &available_codecs(), &opts)?;
        Ok(())
    })
}

pub fn synchronize<P: AsRef<Path>>(src: P, dst: P) -> Result<()> {
    synchronize_with_config(src, dst, &SyncConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{set_file_times, FileTime};
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

        let cfg = SyncConfig::builder().atimes(true).build();

        assert!(!dst_dir.exists());
        synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();
        assert!(dst_dir.exists());
        assert_eq!(fs::read(dst_dir.join("file.txt")).unwrap(), b"data");
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
        synchronize(src_dir.clone(), dst_dir.clone()).unwrap();

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
        synchronize(src_dir.clone(), dst_dir.clone()).unwrap();

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

        let cfg = SyncConfig::builder().atimes(true).build();

        assert!(!dst_dir.exists());
        synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();
        assert!(dst_dir.exists());

        let fifo = src_dir.join("fifo");
        mkfifo(&fifo, Mode::from_bits_truncate(0o640)).unwrap();
        let atime = FileTime::from_unix_time(2_000_000, 0);
        let mtime = FileTime::from_unix_time(2_000_100, 0);
        set_file_times(&fifo, atime, mtime).unwrap();

        synchronize_with_config(src_dir.clone(), dst_dir.clone(), &cfg).unwrap();

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

        synchronize(src_dir.clone(), dst_dir.clone()).unwrap();

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
            println!(
                "skipping sync_preserves_ownership: requires root privileges to change ownership"
            );
            return;
        }

        let (_dir, src_dir, dst_dir) = setup_dirs();

        let file = src_dir.join("file.txt");
        fs::write(&file, b"hello").unwrap();
        chown(&file, Some(Uid::from_raw(1000)), Some(Gid::from_raw(1001))).unwrap();

        synchronize(src_dir.clone(), dst_dir.clone()).unwrap();

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
        std::env::set_var("OC_RSYNC_SYSLOG_PATH", &syslog_path);

        let journald_path = dir.path().join("journald.sock");
        let journald_server = UnixDatagram::bind(&journald_path).unwrap();
        std::env::set_var("OC_RSYNC_JOURNALD_PATH", &journald_path);

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

        std::env::remove_var("OC_RSYNC_SYSLOG_PATH");
        std::env::remove_var("OC_RSYNC_JOURNALD_PATH");
    }
}
